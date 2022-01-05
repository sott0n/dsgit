mod common;

use serial_test::serial;

use common::setup;
use dsgit::commit::Commit;
use dsgit::data::{get_object, TypeObject};

#[test]
#[serial]
fn commit() {
    setup();
    // First commit, not include parent hash.
    let got_first_oid: String = Commit::commit("test", &[]).unwrap().to_string();

    if cfg!(target_os = "windows") {
        assert_eq!(&got_first_oid, "5444358a6d8d39c780af4b0cb7bbaeebeab42bfe");
    } else {
        assert_eq!(&got_first_oid, "0c641ad2b7a880c5f4a391562edc5dd1d8ebf82f");
    }
    let obj: String = get_object(&got_first_oid, TypeObject::Commit).unwrap();
    let contents: Vec<&str> = obj.lines().collect();
    assert_eq!(contents[2], "test");

    // Second commit, include parent hash.
    let got_second_oid: String = Commit::commit("second commit", &[]).unwrap().to_string();

    if cfg!(target_os = "windows") {
        assert_eq!(&got_second_oid, "2b539f4ff7b42e6f8dcceea4e7f99f739d379660");
    } else {
        assert_eq!(&got_second_oid, "0d26aafe9054ffd3625978ab302e74752f78f3be");
    }
    let obj: String = get_object(&got_second_oid, TypeObject::Commit).unwrap();
    let contents: Vec<&str> = obj.lines().collect();
    assert!(contents[0].contains("tree"));
    assert!(contents[1].contains("parent"));
    assert_eq!(contents[3], "second commit");
}

#[test]
#[serial]
fn get_commit() {
    setup();
    let oid1 = Commit::commit("test", &[]).unwrap().to_string();
    let oid2 = Commit::commit("second commit", &[]).unwrap().to_string();

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
