mod common;

use serial_test::serial;
use std::collections::HashSet;
use std::fs;
use std::hash::Hash;
use std::io;
use std::path::PathBuf;

use common::setup;
use dsgit::data::{get_object, TypeObject};
use dsgit::entry;
use dsgit::entry::{Entry, Tree};

fn test_data(target_os: &str) -> [entry::Entry; 4] {
    match target_os {
        "windows" => [
            Entry {
                path: ".\\cat.txt".to_string(),
                oid: "738355a2d1dda0b9f26feb6bb8e2de8f735bcd19".to_string(),
                obj_type: TypeObject::Blob,
            },
            Entry {
                path: ".\\dogs.txt".to_string(),
                oid: "45ce866627173403d0a0406d7c3f4cb54708ec1c".to_string(),
                obj_type: TypeObject::Blob,
            },
            Entry {
                path: ".\\hello.txt".to_string(),
                oid: "f0981ab57ce65e2716df953d09c80478fd7dec1c".to_string(),
                obj_type: TypeObject::Blob,
            },
            Entry {
                path: ".\\other".to_string(),
                oid: "7716369a392b192a80b7766c99f2d310d056a807".to_string(),
                obj_type: TypeObject::Tree,
            },
        ],
        // Linux or MacOS
        _ => [
            Entry {
                path: "./cat.txt".to_string(),
                oid: "38d458fa6e384e24e7f15c5d17be0e9cee67f823".to_string(),
                obj_type: TypeObject::Blob,
            },
            Entry {
                path: "./dogs.txt".to_string(),
                oid: "bdb10d71fac51e4952b37042faa62640cd7847db".to_string(),
                obj_type: TypeObject::Blob,
            },
            Entry {
                path: "./hello.txt".to_string(),
                oid: "4963f4ed0612f7242d9d92bf59b4fb8ac8d29ec2".to_string(),
                obj_type: TypeObject::Blob,
            },
            Entry {
                path: "./other".to_string(),
                oid: "b19cdcfa8e09aed887a25d11d73fbe68261dbfc3".to_string(),
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
fn write_tree() {
    setup();
    if cfg!(target_os = "windows") {
        let expect_result = test_data("windows");
        let oid = Tree::write_tree(".", &[]).unwrap();
        assert_eq!(oid, "c98d27e4286eaa1a0a2fe8b809bb16a598bf0638");

        let obj = get_object(&oid, TypeObject::Tree).unwrap();
        for (i, line) in obj.lines().enumerate() {
            let entry = Entry::from(line);
            assert_eq!(entry, expect_result[i]);
        }
    } else {
        let expect_result = test_data(""); // Not need spefify os in linux or macos case.
        let oid = Tree::write_tree(".", &[]).unwrap();
        assert_eq!(oid, "cfafd0b3d132774e6c44b39d2e2bfc3635ec49ef");

        let obj = get_object(&oid, TypeObject::Tree).unwrap();
        for (i, line) in obj.lines().enumerate() {
            let entry = Entry::from(line);
            assert_eq!(entry, expect_result[i]);
        }
    }
}

#[test]
#[serial]
fn read_tree() {
    fn assert_read_tree(expect_oid: &str, expect_paths: &[PathBuf; 5]) {
        let oid = Tree::write_tree(".", &[]).unwrap();
        assert_eq!(oid, expect_oid);
        fs::remove_file("./cat.txt").unwrap();
        let paths = fs::read_dir(".").unwrap();
        assert_eq!(paths.count(), 4);

        Tree::read_tree(&oid, &[]).unwrap();
        let paths = fs::read_dir(".").unwrap();
        let got_paths = paths
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, io::Error>>()
            .unwrap();

        assert_eq!(&got_paths.len(), &expect_paths.len());
        assert_list(&got_paths, expect_paths);
    }
    let expect_paths = [
        PathBuf::from("./cat.txt"),
        PathBuf::from("./dogs.txt"),
        PathBuf::from("./hello.txt"),
        PathBuf::from("./other"),
        PathBuf::from("./.dsgit"),
    ];
    setup();
    if cfg!(target_os = "windows") {
        assert_read_tree("c98d27e4286eaa1a0a2fe8b809bb16a598bf0638", &expect_paths);
    } else {
        assert_read_tree("cfafd0b3d132774e6c44b39d2e2bfc3635ec49ef", &expect_paths);
    }
}
