use crate::data;
use crate::data::{get_object, hash_object, TypeObject};
use anyhow::{anyhow, Context, Result};
use std::fmt;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::str::FromStr;

#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub struct Entry {
    path: String,
    oid: String,
    obj_type: TypeObject,
}

impl fmt::Display for Entry {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        writeln!(fmt, "{} {} {}", self.obj_type, self.oid, self.path)
    }
}

impl From<&str> for Entry {
    fn from(item: &str) -> Entry {
        let entry: Vec<&str> = item.split(' ').collect();
        if entry.len() != 3 {
            anyhow!(
                "Entry must be length == 3, but this length got {}",
                entry.len()
            );
        }

        Entry {
            path: entry[2].to_owned(),
            oid: entry[1].to_owned(),
            obj_type: TypeObject::from_str(entry[0]).unwrap(),
        }
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
                path: path.to_str().unwrap().to_string(),
                oid: oid.to_string(),
                obj_type: TypeObject::Blob,
            })
        }
        if metadata.is_dir() {
            let oid = write_tree(path.to_str().unwrap()).unwrap();
            entries.push(Entry {
                path: path.to_str().unwrap().to_string(),
                oid: oid.to_string(),
                obj_type: TypeObject::Tree,
            })
        }
    }

    let mut tree = String::new();
    entries.sort();
    for entry in entries.iter() {
        tree = tree + &entry.to_string();
    }

    let hash_tree = hash_object(&tree, TypeObject::Tree).unwrap();
    Ok(hash_tree)
}

pub fn read_tree(oid: &str) {
    let tree = get_object(oid, TypeObject::Tree).unwrap();
    let entries = &get_tree(&tree).unwrap();

    for entry in entries.iter() {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&entry.path)
            .with_context(|| format!("Failed to read tree: {}", &entry.path))
            .unwrap();

        file.write_all(
            data::get_object(&entry.oid, TypeObject::Blob)
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
    }
}

fn get_tree(tree: &str) -> Result<Vec<Entry>> {
    let mut entries = vec![];

    for line in tree.lines() {
        let entry = Entry::from(line);
        match entry.obj_type {
            TypeObject::Blob => {
                entries.push(entry);
            }
            TypeObject::Tree => {
                let tmp_tree = get_object(&entry.oid, TypeObject::Tree).unwrap();
                let mut tmp_entries = get_tree(&tmp_tree).unwrap();
                entries.append(&mut tmp_entries);
            }
        }
    }

    Ok(entries)
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
