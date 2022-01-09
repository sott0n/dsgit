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
    let pre_oid = Tree::write_tree(".", &[]).unwrap();

    fs::remove_file("./cat.txt").unwrap();
    fs::write("./dragon.txt", "Ryuu").unwrap();
    fs::write("./tiger.txt", "ToraTora").unwrap();
    let mut f = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open("./dogs.txt")
        .unwrap();
    f.write_all(b"wan wan wan").unwrap();
    f.flush().unwrap();

    let to_tree = Tree::new(".", &[]).unwrap();
    let mut diffs = diff_trees(from_tree, to_tree).unwrap();
    diffs.0.sort();
    diffs.1.sort();
    diffs.2.sort();

    if cfg!(target_os = "windows") {
        assert_eq!(diffs.0, vec![".\\dogs.txt"]);
        assert_eq!(diffs.1, vec![".\\dragon.txt", ".\\tiger.txt"]);
        assert_eq!(diffs.2, vec![".\\cat.txt"]);
    } else {
        assert_eq!(diffs.0, vec!["./dogs.txt"]);
        assert_eq!(diffs.1, vec!["./dragon.txt", "./tiger.txt"]);
        assert_eq!(diffs.2, vec!["./cat.txt"]);
    }

    // Teardown, restore removed file.
    Tree::read_tree(&pre_oid, &[]);
}
