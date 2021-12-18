use crate::data;
use crate::data::{get_object, hash_object, TypeObject};
use anyhow::{anyhow, Context, Result};
use std::fmt;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
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

#[derive(Debug)]
pub struct Tree {
    entries: Vec<Entry>,
}

impl Tree {
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

        let mut tree = String::new();
        entries.sort();
        for entry in entries.iter() {
            tree = tree + &entry.to_string();
        }

        let hash_tree = hash_object(&tree, TypeObject::Tree).unwrap();
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
            let mut file = OpenOptions::new()
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

    fn get_tree(tree: &str) -> Result<Self> {
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

#[derive(Debug, PartialEq)]
pub struct Commit {
    pub tree: String,
    pub parent: Option<String>,
    pub message: String,
}

impl Commit {
    pub fn get_commit(oid: &str) -> Result<Self> {
        let commit_obj = get_object(oid, TypeObject::Commit)?;
        let lines: Vec<&str> = commit_obj.lines().collect::<Vec<&str>>();

        // Parse each line from below commit format:
        //   tree [commit hash]
        //   parent [commit hash] // if first commit, this line is nothing.
        //
        //   [commit message]
        //
        // Parse a tree line as line0.
        let line0: Vec<&str> = lines[0].split(' ').collect();
        let tree = if line0[0] == "tree" {
            line0[1].to_string()
        } else {
            return Err(anyhow!(
                "Commit object expected including tree object, but got {}",
                line0[0]
            ));
        };

        // Parse a parent line as line0,
        // this line may None in this case of first commit.
        let line1: Vec<&str> = lines[1].split(' ').collect();
        let parent = if line1[0] == "parent" {
            Some(line1[1].to_string())
        } else {
            None
        };

        // Parse a commit message at last line.
        let message = String::from("") + lines.last().unwrap();

        Ok(Commit {
            tree,
            parent,
            message,
        })
    }

    pub fn commit(message: &str, ignore_options: &[String]) -> Result<String> {
        let oid = Tree::write_tree(".", ignore_options)?;
        let mut commit = String::from("tree ") + &oid + "\n";

        if let Some(head) = data::get_ref("HEAD")? {
            commit = commit + "parent " + &head + "\n"
        }

        commit = commit + "\n" + message + "\n";
        let commit_oid = data::hash_object(&commit, data::TypeObject::Commit)?;
        Ok(data::update_ref("HEAD", &commit_oid)?.to_owned())
    }
}

pub fn checkout(oid: &str, ignore_options: &[String]) {
    let commit = Commit::get_commit(oid).unwrap();
    Tree::read_tree(&commit.tree, ignore_options);
    let _ = data::update_ref("HEAD", oid);
}

pub fn create_tag(tag: &str, oid: &str) {
    let _ = data::update_ref(&format!("refs/tags/{}", tag), oid);
}

pub fn get_oid(name: &str) -> Result<String> {
    let refs_walk = [
        name.to_string(),
        format!("refs/{}", name),
        format!("refs/tags/{}", name),
        format!("refs/heads/{}", name),
    ];
    for path in refs_walk.iter() {
        match data::get_ref(path)? {
            Some(oid) => return Ok(oid),
            None => continue,
        };
    }

    // Check a given name is hash value.
    let is_hex = name
        .chars()
        .collect::<Vec<char>>()
        .iter()
        .all(|c| c.is_ascii_hexdigit());
    if name.len() == 40 && is_hex {
        return Ok(name.to_string());
    }

    Err(anyhow!(format!(
        "Unknown name and not hash value: {}",
        name
    )))
}

pub fn create_branch(name: &str, oid: &str) {
    let ref_name = String::from("refs/heads/") + name;
    let _ = data::update_ref(&ref_name, oid);
}

#[cfg(test)]
mod test {
    use super::*;
    use data::{get_object, init};
    use serial_test::serial;
    use std::collections::HashSet;
    use std::fs;
    use std::hash::Hash;
    use std::io;
    use std::io::BufRead;
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
            let oid = Tree::write_tree(TARGET_PATH, ignore_files).unwrap();
            assert_eq!(oid, "758e8e0c0eae49610531a22c1778e10baece4415");

            let obj = get_object(&oid, TypeObject::Tree).unwrap();
            for (i, line) in obj.lines().enumerate() {
                let entry = Entry::from(line);
                assert_eq!(entry, expect_result[i]);
            }
        }
        if cfg!(target_os = "windows") {
            let expect_result = test_data("windows");
            let oid = Tree::write_tree(TARGET_PATH, ignore_files).unwrap();
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
        if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            assert_read_tree("758e8e0c0eae49610531a22c1778e10baece4415", &expect_paths);
        }
        if cfg!(target_os = "windows") {
            assert_read_tree("e64d4dd00d39e3f8f76337cbe3bab51a48d70708", &expect_paths);
        }
    }

    #[test]
    #[serial]
    fn test_commit() {
        setup();
        let ignore_files: &[String] = &IGNORE_FILES.map(|f| f.to_string());

        // First commit, not include parent hash.
        let got_first_oid: String = Commit::commit("test", &ignore_files).unwrap().to_string();

        if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            assert_eq!(&got_first_oid, "7679dce8118ba45c0e0698845d71db172b350852");
        }
        if cfg!(target_os = "windows") {
            assert_eq!(&got_first_oid, "aaad0be6d821d5398420eaad5f385892d6727df2");
        }
        let obj: String = get_object(&got_first_oid, data::TypeObject::Commit).unwrap();
        let contents: Vec<&str> = obj.lines().collect();
        assert_eq!(contents[2], "test");

        // Second commit, include parent hash.
        let got_second_oid: String = Commit::commit("second commit", &ignore_files)
            .unwrap()
            .to_string();

        if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
            assert_eq!(&got_second_oid, "6605e946039f5c20e9af1972736a3ecd012fe095");
        }
        if cfg!(target_os = "windows") {
            assert_eq!(&got_second_oid, "36c824b68b40b8c054d9d82c424db2e37ff8c628");
        }
        let obj: String = get_object(&got_second_oid, data::TypeObject::Commit).unwrap();
        let contents: Vec<&str> = obj.lines().collect();
        assert!(contents[0].contains("tree"));
        assert!(contents[1].contains("parent"));
        assert_eq!(contents[3], "second commit");
    }

    #[test]
    #[serial]
    fn test_get_commit() {
        setup();
        let ignore_files: &[String] = &IGNORE_FILES.map(|f| f.to_string());

        let oid1 = Commit::commit("test", &ignore_files).unwrap().to_string();
        let oid2 = Commit::commit("second commit", &ignore_files)
            .unwrap()
            .to_string();

        let commit1 = Commit::get_commit(&oid1).unwrap();
        assert!(matches!(commit1, Commit { parent: None, .. }));
        assert_eq!(commit1.message, "test".to_string());

        let commit2 = Commit::get_commit(&oid2).unwrap();
        assert!(matches!(
            commit2,
            Commit {
                parent: Some(..),
                ..
            }
        ));
        assert_eq!(commit2.message, "second commit".to_string());
    }

    #[test]
    #[serial]
    fn test_checkout() {
        fn assert_number_files(path: &str, expected: usize) {
            let files = fs::read_dir(path)
                .unwrap()
                .map(|res| res.map(|e| e.path()))
                .collect::<Result<Vec<_>, io::Error>>()
                .unwrap();
            assert_eq!(files.len(), expected);
        }
        setup();
        let ignore_files: &[String] = &IGNORE_FILES.map(|f| f.to_string());

        let oid1 = Commit::commit("1st commit", &ignore_files).unwrap();
        assert_number_files("./tests", 4);

        // Create a new file.
        fs::write("./tests/foo.txt", "foo bar").unwrap();
        Commit::commit("2nd commit", &ignore_files).unwrap();
        assert_number_files("./tests", 5);

        // Checkout `1st commit` hash.
        checkout(&oid1, &ignore_files);
        assert_number_files("./tests", 4);
    }

    fn assert_file_contents(path: &str, expects: Vec<String>) {
        let f1 = fs::File::open(path).unwrap();
        let f1_contents = io::BufReader::new(f1);
        for (got, expect) in f1_contents.lines().zip(expects) {
            assert_eq!(got.unwrap(), expect);
        }
    }

    #[test]
    #[serial]
    fn test_create_tag() {
        setup();
        let ignore_files: &[String] = &IGNORE_FILES.map(|f| f.to_string());
        let oid1 = Commit::commit("1st commit", &ignore_files).unwrap();
        let oid2 = Commit::commit("2nd commit", &ignore_files).unwrap();

        create_tag("tag1", &oid1);
        let f1_path = format!("{}/refs/tags/tag1", DSGIT_DIR);
        assert!(Path::new(&f1_path).exists());
        assert_file_contents(&f1_path, vec![oid1]);

        create_tag("tag2", &oid2);
        let f2_path = format!("{}/refs/tags/tag2", DSGIT_DIR);
        assert!(Path::new(&f2_path).exists());
        assert_file_contents(&f2_path, vec![oid2]);
    }

    #[test]
    #[serial]
    fn test_create_branch() {
        setup();
        let ignore_files: &[String] = &IGNORE_FILES.map(|f| f.to_string());
        let oid1 = Commit::commit("1st commit", &ignore_files).unwrap();
        let oid2 = Commit::commit("2nd commit", &ignore_files).unwrap();

        create_branch("branch1", &oid1);
        let b1_path = format!("{}/refs/heads/branch1", DSGIT_DIR);
        assert!(Path::new(&b1_path).exists());
        assert_file_contents(&b1_path, vec![oid1]);

        create_branch("branch2", &oid2);
        let b2_path = format!("{}/refs/heads/branch2", DSGIT_DIR);
        assert!(Path::new(&b2_path).exists());
        assert_file_contents(&b2_path, vec![oid2]);
    }
}
