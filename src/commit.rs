use crate::data::{get_object, hash_object, TypeObject};
use crate::entry::Tree;
use crate::reference::RefValue;
use anyhow::{anyhow, Result};

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

        if let Some(ref_value) = RefValue::get_ref("HEAD", true)? {
            commit = commit + "parent " + &ref_value.value + "\n"
        }

        commit = commit + "\n" + message + "\n";
        let commit_oid = hash_object(&commit, TypeObject::Commit)?;
        let ref_value = RefValue::new(Some(&commit_oid), false, &commit_oid);
        RefValue::update_ref("HEAD", &ref_value, true)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::data::init;
    use serial_test::serial;
    use std::fs;

    const DSGIT_DIR: &str = ".dsgit";
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
        let obj: String = get_object(&got_first_oid, TypeObject::Commit).unwrap();
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
        let obj: String = get_object(&got_second_oid, TypeObject::Commit).unwrap();
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
}
