use std::env::set_current_dir;
use std::fs;
use std::io;
use std::io::BufRead;

use dsgit::data::init;

pub const DSGIT_DIR: &str = ".dsgit";

pub fn setup() {
    let _ = set_current_dir("./tests/test_files");
    let _ = fs::remove_dir_all(DSGIT_DIR);
    init().unwrap();
}

#[allow(dead_code)]
pub fn assert_file_contents(path: &str, expects: Vec<String>) {
    let f1 = fs::File::open(path).unwrap();
    let f1_contents = io::BufReader::new(f1);
    for (got, expect) in f1_contents.lines().zip(expects) {
        assert_eq!(got.unwrap(), expect);
    }
}
