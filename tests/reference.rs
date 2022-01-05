mod common;

use dsgit::commit::Commit;
use serial_test::serial;
use std::fs;
use std::io;
use std::path::Path;

use common::{assert_file_contents, setup, DSGIT_DIR};
use dsgit::reference;
use dsgit::reference::RefValue;

#[test]
#[serial]
fn switch() {
    fn assert_number_files(expected: usize) {
        let files = fs::read_dir(".")
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()
            .unwrap();
        assert_eq!(files.len(), expected);
    }
    setup();
    let oid1 = Commit::commit("1st commit", &[]).unwrap();
    assert_number_files(5);

    // Create a new file.
    fs::write("./foo.txt", "foo bar").unwrap();
    Commit::commit("2nd commit", &[]).unwrap();
    assert_number_files(6);

    // Switch `1st commit` hash.
    RefValue::switch(&oid1, &[]);
    assert_number_files(5);

    // Switch branch.
    reference::create_branch("branch1", &oid1);
    RefValue::switch("branch1", &[]);
    let head_path = format!("{}/HEAD", DSGIT_DIR);
    let expect_val = "ref:refs/heads/branch1".to_string();
    assert_file_contents(&head_path, vec![expect_val]);
}

#[test]
#[serial]
fn create_tag() {
    setup();
    let oid1 = Commit::commit("1st commit", &[]).unwrap();
    let oid2 = Commit::commit("2nd commit", &[]).unwrap();

    reference::create_tag("tag1", &oid1);
    let f1_path = format!("{}/refs/tags/tag1", DSGIT_DIR);
    assert!(Path::new(&f1_path).exists());
    assert_file_contents(&f1_path, vec![oid1]);

    reference::create_tag("tag2", &oid2);
    let f2_path = format!("{}/refs/tags/tag2", DSGIT_DIR);
    assert!(Path::new(&f2_path).exists());
    assert_file_contents(&f2_path, vec![oid2]);
}

#[test]
#[serial]
fn create_branch() {
    setup();
    let oid1 = Commit::commit("1st commit", &[]).unwrap();
    let oid2 = Commit::commit("2nd commit", &[]).unwrap();

    reference::create_branch("branch1", &oid1);
    let b1_path = format!("{}/refs/heads/branch1", DSGIT_DIR);
    assert!(Path::new(&b1_path).exists());
    assert_file_contents(&b1_path, vec![oid1]);

    reference::create_branch("branch2", &oid2);
    let b2_path = format!("{}/refs/heads/branch2", DSGIT_DIR);
    assert!(Path::new(&b2_path).exists());
    assert_file_contents(&b2_path, vec![oid2]);
}

#[test]
#[serial]
fn get_all_branches() {
    setup();
    let oid1 = Commit::commit("1st commit", &[]).unwrap();
    let oid2 = Commit::commit("2nd commit", &[]).unwrap();
    reference::create_branch("branch1", &oid1);
    reference::create_branch("branch2", &oid2);

    let mut branches = RefValue::get_refs(Some("."), "refs/heads").unwrap();
    branches.sort();
    assert_eq!(branches, vec!["HEAD", "branch1", "branch2"]);
}

#[test]
#[serial]
fn reset() {
    setup();
    let oid1 = Commit::commit("1st commit", &[]).unwrap();
    let _ = Commit::commit("2nd commit", &[]).unwrap();

    let head_path = format!("{}/HEAD", DSGIT_DIR);

    reference::reset(&oid1);
    if cfg!(target_os = "windows") {
        let expect_val = "49db91fcd51c7f6e04916cf6679a2055882c5c7d".to_owned();
        assert_file_contents(&head_path, vec![expect_val]);
    } else {
        let expect_val = "924a1ce93c755545d46c95bb2ae8e3ea15367587".to_owned();
        assert_file_contents(&head_path, vec![expect_val]);
    }
}
