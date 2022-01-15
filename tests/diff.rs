mod common;

use std::fs;
use std::io::Write;

use common::setup;
use dsgit::diff::diff_trees;
use dsgit::entry::Tree;

#[test]
fn test_diff_trees() {
    setup();
    let from_tree = Tree::new(".", &[]).unwrap();
    let f_oid = Tree::write_tree(".", &[]).unwrap();

    // This diff pattern: a removed file and two new files.
    fs::remove_file("./cat.txt").unwrap();
    fs::write("./dragon.txt", "Ryuu").unwrap();
    fs::write("./tiger.txt", "ToraTora").unwrap();
    let to_tree = Tree::new(".", &[]).unwrap();

    let mut diffs = diff_trees(from_tree, to_tree, false).unwrap();
    diffs.1.sort();

    if cfg!(target_os = "windows") {
        assert!(diffs.0.len() == 0);
        assert_eq!(diffs.1, vec![".\\dragon.txt", ".\\tiger.txt"]);
        assert_eq!(diffs.2, vec![".\\cat.txt"]);
    } else {
        assert!(diffs.0.len() == 0);
        assert_eq!(diffs.1, vec!["./dragon.txt", "./tiger.txt"]);
        assert_eq!(diffs.2, vec!["./cat.txt"]);
    }

    // This diff pattern: a update file.
    let from_tree = Tree::new(".", &[]).unwrap();
    let mut f = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open("./tiger.txt")
        .unwrap();
    f.write_all(b"gaoo").unwrap();
    f.flush().unwrap();
    let to_tree = Tree::new(".", &[]).unwrap();
    let diffs = diff_trees(from_tree, to_tree, false).unwrap();

    if cfg!(target_os = "windows") {
        assert_eq!(diffs.0, vec![".\\tiger.txt"]);
        assert!(diffs.1.len() == 0);
        assert!(diffs.2.len() == 0);
    } else {
        assert_eq!(diffs.0, vec!["./tiger.txt"]);
        assert!(diffs.1.len() == 0);
        assert!(diffs.2.len() == 0);
    }

    // Teardown, restore removed file.
    Tree::read_tree(&f_oid, &[]).unwrap();
}
