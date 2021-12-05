use crate::data;
use crate::data::{get_object, hash_object, TypeObject};
use anyhow::{anyhow, Context, Result};
use std::fmt;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::str::FromStr;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Entry {
    path: String,
    oid: String,
    obj_type: TypeObject,
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
            anyhow!(
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

pub fn write_tree(target_path: &str, ignore_options: &[String]) -> Result<String> {
    let mut entries: Vec<Entry> = vec![];
    for entry in fs::read_dir(target_path)
        .with_context(|| format!("Failed to read directory: {}", target_path))?
    {
        let path = entry.unwrap().path();

        if is_ignored(path.to_str().unwrap(), ignore_options) {
            println!("{}", &path.display());
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
            let oid = write_tree(path.to_str().unwrap(), ignore_options).unwrap();
            entries.push(Entry {
                path: path.to_str().unwrap().to_string(),
                oid: oid.to_string(),
                obj_type: TypeObject::Tree,
            })
        }
    }

    let mut tree = String::new();
    entries.sort();
    for entry in entries.iter() {
        tree = tree + &entry.to_string();
    }

    let hash_tree = hash_object(&tree, TypeObject::Tree).unwrap();
    Ok(hash_tree)
}

pub fn read_tree(oid: &str) {
    let tree = get_object(oid, TypeObject::Tree).unwrap();
    let entries = &get_tree(&tree).unwrap();

    for entry in entries.iter() {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&entry.path)
            .with_context(|| format!("Failed to read tree: {}", &entry.path))
            .unwrap();

        file.write_all(
            data::get_object(&entry.oid, TypeObject::Blob)
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
    }
}

fn get_tree(tree: &str) -> Result<Vec<Entry>> {
    let mut entries = vec![];

    for line in tree.lines() {
        let entry = Entry::from(line);
        match entry.obj_type {
            TypeObject::Blob => {
                entries.push(entry);
            }
            TypeObject::Tree => {
                let tmp_tree = get_object(&entry.oid, TypeObject::Tree).unwrap();
                let mut tmp_entries = get_tree(&tmp_tree).unwrap();
                entries.append(&mut tmp_entries);
            }
        }
    }

    Ok(entries)
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

#[cfg(test)]
mod test {
    use crate::read_tree;

    use super::*;
    use data::{get_object, init};
    use serial_test::serial;
    use std::collections::HashSet;
    use std::fs;
    use std::hash::Hash;
    use std::io;
    use std::path::PathBuf;

    const DSGIT_DIR: &str = ".dsgit";
    const TARGET_PATH: &str = "./tests";
    const IGNORE_FILES: [&str; 4] = ["target/", "Cargo.toml", "Cargo.lock", "src/"];

    fn setup() {
        let _ = fs::remove_dir_all(DSGIT_DIR);
        let _ = init();
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

    #[test]
    #[serial]
    fn test_write_tree() {
        setup();
        let ignore_files: &[String] = &IGNORE_FILES.map(|f| f.to_string());
        if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            let expect_result = test_data(""); // Not need spefify os in linux or macos case.
            let oid = write_tree(TARGET_PATH, ignore_files).unwrap();
            assert_eq!(oid, "758e8e0c0eae49610531a22c1778e10baece4415");

            let obj = get_object(&oid, TypeObject::Tree).unwrap();
            for (i, line) in obj.lines().enumerate() {
                let entry = Entry::from(line);
                assert_eq!(entry, expect_result[i]);
            }
        }
        if cfg!(target_os = "windows") {
            let expect_result = test_data("windows");
            let oid = write_tree(TARGET_PATH, ignore_files).unwrap();
            assert_eq!(oid, "e64d4dd00d39e3f8f76337cbe3bab51a48d70708");

            let obj = get_object(&oid, TypeObject::Tree).unwrap();
            for (i, line) in obj.lines().enumerate() {
                let entry = Entry::from(line);
                assert_eq!(entry, expect_result[i]);
            }
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

    #[test]
    #[serial]
    fn test_read_tree() {
        fn assert_read_tree(expect_oid: &str, expect_paths: &[PathBuf; 4]) {
            let ignore_files: &[String] = &IGNORE_FILES.map(|f| f.to_string());
            let oid = write_tree(TARGET_PATH, ignore_files).unwrap();
            assert_eq!(oid, expect_oid);

            fs::remove_file("./tests/cat.txt").unwrap();
            let paths = fs::read_dir("./tests").unwrap();
            assert_eq!(paths.count(), 3);

            read_tree(&oid);
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
        if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            assert_read_tree("758e8e0c0eae49610531a22c1778e10baece4415", &expect_paths);
        }
        if cfg!(target_os = "windows") {
            assert_read_tree("e64d4dd00d39e3f8f76337cbe3bab51a48d70708", &expect_paths);
        }
    }
}
