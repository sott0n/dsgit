mod common;

use serial_test::serial;
use std::assert;
use std::fs;
use std::path::Path;

use common::{assert_file_contents, setup, DSGIT_DIR};
use dsgit::data;

const TEST_DATA: [(&str, &str, &str, &str, &str); 3] = [
    (
        "./hello.txt",
        "4963f4ed0612f7242d9d92bf59b4fb8ac8d29ec2", // Linux and MacOS
        "Hello World!\n",
        "f0981ab57ce65e2716df953d09c80478fd7dec1c", // Windows
        "Hello World!\r\n",
    ),
    (
        "./cat.txt",
        "38d458fa6e384e24e7f15c5d17be0e9cee67f823", // Linux and MacOS
        "cat cat\n",
        "738355a2d1dda0b9f26feb6bb8e2de8f735bcd19", // Windows
        "cat cat\r\n",
    ),
    (
        "./dogs.txt",
        "bdb10d71fac51e4952b37042faa62640cd7847db", // Linux and MacOS
        "dog dog dog\n",
        "45ce866627173403d0a0406d7c3f4cb54708ec1c", // Windows
        "dog dog dog\r\n",
    ),
];

#[test]
#[serial]
fn init() {
    // Remove `.dsgit` to pass init before this test.
    if Path::new(DSGIT_DIR).exists() {
        fs::remove_dir_all(DSGIT_DIR).unwrap();
    }

    data::init().unwrap();
    let head_path = format!("{}/HEAD", DSGIT_DIR);
    let expect_val = "ref:refs/heads/main".to_string();
    assert_file_contents(&head_path, vec![expect_val]);
}

#[test]
#[serial]
fn hash_object() {
    setup();
    for f in TEST_DATA.iter() {
        let contents = fs::read_to_string(f.0).unwrap();
        let hash = data::hash_object(&contents, data::TypeObject::Blob).unwrap();

        if cfg!(target_os = "windows") {
            assert_eq!(hash, f.3);
            assert!(Path::new(&format!("{}/objects/{}", DSGIT_DIR, f.3)).exists());
        } else {
            assert_eq!(hash, f.1);
            assert!(Path::new(&format!("{}/objects/{}", DSGIT_DIR, f.1)).exists());
        }
    }
}

#[test]
#[serial]
fn get_object() {
    setup();
    for f in TEST_DATA.iter() {
        let contents = fs::read_to_string(f.0).unwrap();
        let hash = data::hash_object(&contents, data::TypeObject::Blob).unwrap();
        let obj = data::get_object(&hash, data::TypeObject::Blob).unwrap();

        if cfg!(target_os = "windows") {
            assert_eq!(obj, f.4);
        } else {
            assert_eq!(obj, f.2);
        }
    }
}
