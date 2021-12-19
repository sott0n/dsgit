use crate::commit::Commit;
use crate::data::get_oid;
use crate::entry::Tree;
use anyhow::{anyhow, Context, Result};
use std::fs::{create_dir_all, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::str;

const DSGIT_DIR: &str = ".dsgit";

#[derive(Debug)]
pub struct RefValue {
    pub ref_oid: Option<String>,
    pub symbolic: bool,
    pub value: String,
}

impl RefValue {
    pub fn new(ref_oid: Option<&str>, symbolic: bool, value: &str) -> Self {
        RefValue {
            ref_oid: ref_oid.map(|oid| oid.to_owned()),
            symbolic,
            value: value.to_owned(),
        }
    }

    pub fn update_ref<'a>(refs: &'a str, ref_value: &'a RefValue, deref: bool) -> Result<String> {
        let refs = match RefValue::get_ref_internal(refs, deref)? {
            Some(ref_value) => ref_value.ref_oid.unwrap(),
            // At first commit case, this returns None.
            None => refs.to_owned(),
        };

        assert!(!ref_value.value.is_empty());
        let value: String = if ref_value.symbolic {
            String::from("ref:") + &ref_value.value
        } else {
            ref_value.value.to_owned()
        };

        let ref_path: String = format!("{}/{}", DSGIT_DIR, refs);
        let parent_path = Path::new(&ref_path).parent().unwrap();

        create_dir_all(parent_path)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&ref_path)
            .with_context(|| format!("Failed to open object file: {}", ref_path))?;

        file.write_all(value.as_bytes()).unwrap();
        file.flush().unwrap();
        Ok(value)
    }

    pub fn get_ref(refs: &str, deref: bool) -> Result<Option<RefValue>> {
        RefValue::get_ref_internal(refs, deref)
    }

    fn get_ref_internal(refs: &str, deref: bool) -> Result<Option<RefValue>> {
        let ref_path = &format!("{}/{}", DSGIT_DIR, refs);
        if Path::new(ref_path).is_file() {
            let mut file = OpenOptions::new()
                .read(true)
                .open(ref_path)
                .with_context(|| format!("Failed to open file: {}", ref_path))?;

            let mut value = String::from("");
            file.read_to_string(&mut value)?;

            // This means a symbolic reference.
            let symbolic = !value.is_empty() && value.starts_with("ref:");
            if symbolic {
                value = value.split(':').collect::<Vec<&str>>()[1].to_string();
                if deref {
                    return RefValue::get_ref_internal(&value, true);
                }
            };
            Ok(Some(RefValue::new(Some(refs), symbolic, &value)))
        } else {
            Ok(None)
        }
    }
}

pub fn switch(name: &str, ignore_options: &[String]) {
    let oid = get_oid(name).unwrap();
    let commit = Commit::get_commit(&oid).unwrap();
    Tree::read_tree(&commit.tree, ignore_options);

    let head_ref = if is_branch(name) {
        let value = String::from("refs/heads/") + name;
        RefValue::new(Some(&oid), true, &value)
    } else {
        RefValue::new(Some(&oid), false, &oid)
    };

    RefValue::update_ref("HEAD", &head_ref, false).unwrap();
}

fn is_branch(name: &str) -> bool {
    let p = String::from("refs/heads/") + name;
    RefValue::get_ref(&p, true).unwrap().is_some()
}

pub fn get_branch_name() -> Result<Option<String>> {
    let head_ref = match RefValue::get_ref("HEAD", false)? {
        Some(head_ref) => head_ref,
        None => return Err(anyhow!("A `HEAD` file is not found.")),
    };

    if !head_ref.symbolic {
        return Ok(None);
    };

    assert!(head_ref.value.starts_with("refs/heads/"));
    let head_path = Path::new(&head_ref.value);
    let base_path = Path::new("refs/heads");
    let rel_path = head_path
        .strip_prefix(base_path)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    Ok(Some(rel_path))
}

pub fn create_tag(tag: &str, oid: &str) {
    let ref_value = RefValue::new(Some(oid), false, oid);
    RefValue::update_ref(&format!("refs/tags/{}", tag), &ref_value, true).unwrap();
}

pub fn create_branch(name: &str, oid: &str) {
    let ref_name = String::from("refs/heads/") + name;
    let ref_value = RefValue::new(Some(oid), false, oid);
    RefValue::update_ref(&ref_name, &ref_value, true).unwrap();
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::commit::Commit;
    use crate::data::init;
    use serial_test::serial;
    use std::fs;
    use std::io;
    use std::io::BufRead;

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

    fn assert_file_contents(path: &str, expects: Vec<String>) {
        let f1 = fs::File::open(path).unwrap();
        let f1_contents = io::BufReader::new(f1);
        for (got, expect) in f1_contents.lines().zip(expects) {
            assert_eq!(got.unwrap(), expect);
        }
    }

    #[test]
    #[serial]
    fn test_switch() {
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

        // Switch `1st commit` hash.
        switch(&oid1, &ignore_files);
        assert_number_files("./tests", 4);

        // Switch branch.
        create_branch("branch1", &oid1);
        switch("branch1", &ignore_files);
        let head_path = format!("{}/HEAD", DSGIT_DIR);
        let expect_val = "ref:refs/heads/branch1".to_string();
        assert_file_contents(&head_path, vec![expect_val]);
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
