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
