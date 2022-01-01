use crate::data::{get_object, hash_object, TypeObject};
use anyhow::{anyhow, Context, Result};
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Entry {
    pub path: String,
    pub oid: String,
    pub obj_type: TypeObject,
}

impl fmt::Display for Entry {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        writeln!(fmt, "{} {} {}", self.obj_type, self.oid, self.path)
    }
}

impl From<&str> for Entry {
    fn from(item: &str) -> Entry {
        let entry: Vec<&str> = item.split(' ').collect();
        if entry.len() != 3 {
            panic!(
                "Entry must be length == 3, but this length got {}",
                entry.len()
            );
        }
        Entry {
            path: entry[2].to_owned(),
            oid: entry[1].to_owned(),
            obj_type: TypeObject::from_str(entry[0]).unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Tree {
    pub entries: Vec<Entry>,
}

impl Tree {
    pub fn new(target_path: &str, ignore_options: &[String]) -> Result<Self> {
        let mut entries: Vec<Entry> = vec![];
        for entry in fs::read_dir(target_path)
            .with_context(|| format!("Failed to read directory: {}", target_path))?
        {
            let path = entry.unwrap().path();
            if Tree::is_ignored(path.to_str().unwrap(), ignore_options) {
                continue;
            }

            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_file() {
                let contents = fs::read_to_string(&path).unwrap();
                let oid = hash_object(&contents, TypeObject::Blob).unwrap();
                entries.push(Entry {
                    path: path.to_str().unwrap().to_string(),
                    oid: oid.to_string(),
                    obj_type: TypeObject::Blob,
                })
            }
            if metadata.is_dir() {
                let mut tmp_tree = Tree::new(path.to_str().unwrap(), ignore_options)?;
                entries.append(&mut tmp_tree.entries);
            }
        }

        Ok(Tree { entries })
    }

    pub fn write_tree(target_path: &str, ignore_options: &[String]) -> Result<String> {
        let mut entries: Vec<Entry> = vec![];
        for entry in fs::read_dir(target_path)
            .with_context(|| format!("Failed to read directory: {}", target_path))?
        {
            let path = entry.unwrap().path();
            if Tree::is_ignored(path.to_str().unwrap(), ignore_options) {
                continue;
            }

            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_file() {
                let contents = fs::read_to_string(&path).unwrap();
                let oid = hash_object(&contents, TypeObject::Blob).unwrap();
                entries.push(Entry {
                    path: path.to_str().unwrap().to_string(),
                    oid: oid.to_string(),
                    obj_type: TypeObject::Blob,
                })
            }
            if metadata.is_dir() {
                let oid = Tree::write_tree(path.to_str().unwrap(), ignore_options).unwrap();
                entries.push(Entry {
                    path: path.to_str().unwrap().to_string(),
                    oid: oid.to_string(),
                    obj_type: TypeObject::Tree,
                })
            }
        }

        entries.sort();
        let mut tree_contents = String::new();
        for entry in entries.iter() {
            tree_contents = tree_contents + &entry.to_string();
        }

        let hash_tree = hash_object(&tree_contents, TypeObject::Tree).unwrap();
        Ok(hash_tree)
    }

    fn clear_current_directory(ignore_options: &[String]) {
        for entry in fs::read_dir(".").unwrap() {
            let path = entry.unwrap().path();
            if Tree::is_ignored(path.to_str().unwrap(), ignore_options) {
                continue;
            }
            let metadata = fs::symlink_metadata(&path).unwrap();

            if metadata.is_file() {
                fs::remove_file(&path).unwrap();
            }
            if metadata.is_dir() {
                fs::remove_dir_all(&path).unwrap();
            }
        }
    }

    pub fn read_tree(oid: &str, ignore_options: &[String]) {
        Tree::clear_current_directory(ignore_options);
        let tree_contents = get_object(oid, TypeObject::Tree).unwrap();
        let tree = &Tree::get_tree(&tree_contents).unwrap();

        for entry in tree.entries.iter() {
            let path = Path::new(&entry.path);
            let prefix = path.parent().unwrap();
            if !prefix.exists() {
                fs::create_dir_all(prefix).unwrap();
            }
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&entry.path)
                .with_context(|| format!("Failed to read tree: {}", &entry.path))
                .unwrap();

            file.write_all(get_object(&entry.oid, TypeObject::Blob).unwrap().as_bytes())
                .unwrap();
        }
    }

    pub fn get_tree(tree: &str) -> Result<Self> {
        let mut entries = vec![];

        for line in tree.lines() {
            let entry = Entry::from(line);
            match entry.obj_type {
                TypeObject::Blob => {
                    entries.push(entry);
                }
                TypeObject::Tree => {
                    let tmp_tree = get_object(&entry.oid, TypeObject::Tree)?;
                    let mut tmp_tree = Tree::get_tree(&tmp_tree)?;
                    entries.append(&mut tmp_tree.entries);
                }
                _ => return Err(anyhow!("Unknown tree entry.")),
            }
        }
        Ok(Tree { entries })
    }

    pub fn get_working_tree(ignore_options: &[String]) -> Result<Tree> {
        Tree::new(".", ignore_options)
    }

    fn is_ignored(path: &str, ignore_options: &[String]) -> bool {
        let path = path.to_string();
        if path.contains(".dsgit")
            || path.contains(".dsgitignore")
            || path.contains(".git")
            || path.contains(".gitignore")
            || path.contains(".github")
        {
            return true;
        }
        for ignore_path in ignore_options.iter() {
            if path.contains(ignore_path) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::data::init;
    use serial_test::serial;
    use std::collections::HashSet;
    use std::hash::Hash;
    use std::io;
    use std::path::PathBuf;

    const DSGIT_DIR: &str = ".dsgit";
    const TARGET_PATH: &str = "./tests";
    const IGNORE_FILES: [&str; 7] = [
        "target",
        "Cargo.toml",
        "Cargo.lock",
        "src",
        "LICENSE",
        "README.md",
        "Makefile",
    ];

    fn setup() {
        let _ = fs::remove_dir_all(DSGIT_DIR);
        init().unwrap();
    }

    fn test_data(target_os: &str) -> [Entry; 4] {
        match target_os {
            "windows" => [
                Entry {
                    path: "./tests\\cat.txt".to_string(),
                    oid: "738355a2d1dda0b9f26feb6bb8e2de8f735bcd19".to_string(),
                    obj_type: TypeObject::Blob,
                },
                Entry {
                    path: "./tests\\dogs.txt".to_string(),
                    oid: "45ce866627173403d0a0406d7c3f4cb54708ec1c".to_string(),
                    obj_type: TypeObject::Blob,
                },
                Entry {
                    path: "./tests\\hello.txt".to_string(),
                    oid: "f0981ab57ce65e2716df953d09c80478fd7dec1c".to_string(),
                    obj_type: TypeObject::Blob,
                },
                Entry {
                    path: "./tests\\other".to_string(),
                    oid: "431e2e4cda7956920d1c070ad178a4b348d2c360".to_string(),
                    obj_type: TypeObject::Tree,
                },
            ],
            // Linux or MacOS
            _ => [
                Entry {
                    path: "./tests/cat.txt".to_string(),
                    oid: "38d458fa6e384e24e7f15c5d17be0e9cee67f823".to_string(),
                    obj_type: TypeObject::Blob,
                },
                Entry {
                    path: "./tests/dogs.txt".to_string(),
                    oid: "bdb10d71fac51e4952b37042faa62640cd7847db".to_string(),
                    obj_type: TypeObject::Blob,
                },
                Entry {
                    path: "./tests/hello.txt".to_string(),
                    oid: "4963f4ed0612f7242d9d92bf59b4fb8ac8d29ec2".to_string(),
                    obj_type: TypeObject::Blob,
                },
                Entry {
                    path: "./tests/other".to_string(),
                    oid: "0e1ca2a42b0b2934261acd1e9c056b71f7ce405f".to_string(),
                    obj_type: TypeObject::Tree,
                },
            ],
        }
    }

    fn assert_list<T>(a: &[T], b: &[T])
    where
        T: Eq + Hash,
    {
        let a: HashSet<_> = a.iter().collect();
        let b: HashSet<_> = b.iter().collect();

        assert!(a == b);
    }

    #[serial]
    #[test]
    fn test_write_tree() {
        setup();
        let ignore_files: &[String] = &IGNORE_FILES.map(|f| f.to_string());
        if cfg!(target_os = "windows") {
            let expect_result = test_data("windows");
            let oid = Tree::write_tree(TARGET_PATH, ignore_files).unwrap();
            assert_eq!(oid, "e64d4dd00d39e3f8f76337cbe3bab51a48d70708");

            let obj = get_object(&oid, TypeObject::Tree).unwrap();
            for (i, line) in obj.lines().enumerate() {
                let entry = Entry::from(line);
                assert_eq!(entry, expect_result[i]);
            }
        } else {
            let expect_result = test_data(""); // Not need spefify os in linux or macos case.
            let oid = Tree::write_tree(TARGET_PATH, ignore_files).unwrap();
            assert_eq!(oid, "758e8e0c0eae49610531a22c1778e10baece4415");

            let obj = get_object(&oid, TypeObject::Tree).unwrap();
            for (i, line) in obj.lines().enumerate() {
                let entry = Entry::from(line);
                assert_eq!(entry, expect_result[i]);
            }
        }
    }

    #[test]
    #[serial]
    fn test_read_tree() {
        fn assert_read_tree(expect_oid: &str, expect_paths: &[PathBuf; 4]) {
            let ignore_files: &[String] = &IGNORE_FILES.map(|f| f.to_string());
            let oid = Tree::write_tree(TARGET_PATH, ignore_files).unwrap();
            assert_eq!(oid, expect_oid);
            fs::remove_file("./tests/cat.txt").unwrap();
            let paths = fs::read_dir("./tests").unwrap();
            assert_eq!(paths.count(), 3);

            Tree::read_tree(&oid, ignore_files);
            let paths = fs::read_dir("./tests").unwrap();
            let got_paths = paths
                .map(|res| res.map(|e| e.path()))
                .collect::<Result<Vec<_>, io::Error>>()
                .unwrap();

            assert_eq!(&got_paths.len(), &expect_paths.len());
            assert_list(&got_paths, expect_paths);
        }
        let expect_paths = [
            PathBuf::from("./tests/cat.txt"),
            PathBuf::from("./tests/dogs.txt"),
            PathBuf::from("./tests/hello.txt"),
            PathBuf::from("./tests/other"),
        ];
        setup();
        if cfg!(target_os = "windows") {
            assert_read_tree("e64d4dd00d39e3f8f76337cbe3bab51a48d70708", &expect_paths);
        } else {
            assert_read_tree("758e8e0c0eae49610531a22c1778e10baece4415", &expect_paths);
        }
    }
}
