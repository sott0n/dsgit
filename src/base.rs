use crate::data::{hash_object, TypeObject};
use anyhow::{Context, Result};
use std::fs;

#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub struct Entry {
    name: String,
    oid: String,
    obj_type: TypeObject,
}

impl Entry {
    fn to_string_object(&self) -> String {
        format!("{} {} {}\n", self.obj_type, self.oid, self.name)
    }
}

pub fn write_tree(target_path: &str) -> Result<String> {
    let mut entries: Vec<Entry> = vec![];
    for entry in fs::read_dir(target_path)
        .with_context(|| format!("Failed to read directory: {}", target_path))?
    {
        let path = entry.unwrap().path();

        // TODO setting ignore options for rust
        let ignore_options = vec!["target"];
        if is_ignored(path.to_str().unwrap(), ignore_options) {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).unwrap();

        if metadata.is_file() {
            let contents = fs::read_to_string(&path).unwrap();
            let oid = hash_object(&contents, TypeObject::Blob).unwrap();
            entries.push(Entry {
                name: path.to_str().unwrap().to_string(),
                oid: oid.to_string(),
                obj_type: TypeObject::Blob,
            })
        }
        if metadata.is_dir() {
            let oid = &write_tree(path.to_str().unwrap()).unwrap();
            entries.push(Entry {
                name: path.to_str().unwrap().to_string(),
                oid: oid.to_string(),
                obj_type: TypeObject::Tree,
            })
        }
    }

    let mut tree = String::new();
    entries.sort();
    for entry in entries.iter() {
        tree = tree + &entry.to_string_object();
    }

    let hash_tree = hash_object(&tree, TypeObject::Tree).unwrap();
    Ok(hash_tree)
}

fn is_ignored(path: &str, ignore_options: Vec<&str>) -> bool {
    let path = path.to_string();
    if path.contains(".dsgit") || path.contains(".git") {
        return true;
    }
    for ignore_path in ignore_options.iter() {
        if path.contains(ignore_path) {
            return true;
        }
    }
    false
}
