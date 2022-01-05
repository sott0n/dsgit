use crate::data::{get_object, hash_object, TypeObject};
use anyhow::{anyhow, Context, Result};
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Entry {
    pub path: String,
    pub oid: String,
    pub obj_type: TypeObject,
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
            panic!(
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

#[derive(Debug)]
pub struct Tree {
    pub entries: Vec<Entry>,
}

impl Tree {
    pub fn new(target_path: &str, ignore_options: &[String]) -> Result<Self> {
        let mut entries: Vec<Entry> = vec![];
        for entry in fs::read_dir(target_path)
            .with_context(|| format!("Failed to read directory: {}", target_path))?
        {
            let path = entry.unwrap().path();
            if Tree::is_ignored(path.to_str().unwrap(), ignore_options) {
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
                let mut tmp_tree = Tree::new(path.to_str().unwrap(), ignore_options)?;
                entries.append(&mut tmp_tree.entries);
            }
        }

        Ok(Tree { entries })
    }

    pub fn write_tree(target_path: &str, ignore_options: &[String]) -> Result<String> {
        let mut entries: Vec<Entry> = vec![];
        for entry in fs::read_dir(target_path)
            .with_context(|| format!("Failed to read directory: {}", target_path))?
        {
            let path = entry.unwrap().path();
            if Tree::is_ignored(path.to_str().unwrap(), ignore_options) {
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
                let oid = Tree::write_tree(path.to_str().unwrap(), ignore_options).unwrap();
                entries.push(Entry {
                    path: path.to_str().unwrap().to_string(),
                    oid: oid.to_string(),
                    obj_type: TypeObject::Tree,
                })
            }
        }

        entries.sort();
        let mut tree_contents = String::new();
        for entry in entries.iter() {
            tree_contents = tree_contents + &entry.to_string();
        }

        let hash_tree = hash_object(&tree_contents, TypeObject::Tree).unwrap();
        Ok(hash_tree)
    }

    fn clear_current_directory(ignore_options: &[String]) {
        for entry in fs::read_dir(".").unwrap() {
            let path = entry.unwrap().path();
            if Tree::is_ignored(path.to_str().unwrap(), ignore_options) {
                continue;
            }
            let metadata = fs::symlink_metadata(&path).unwrap();

            if metadata.is_file() {
                fs::remove_file(&path).unwrap();
            }
            if metadata.is_dir() {
                fs::remove_dir_all(&path).unwrap();
            }
        }
    }

    pub fn read_tree(oid: &str, ignore_options: &[String]) {
        Tree::clear_current_directory(ignore_options);
        let tree_contents = get_object(oid, TypeObject::Tree).unwrap();
        let tree = &Tree::get_tree(&tree_contents).unwrap();

        for entry in tree.entries.iter() {
            let path = Path::new(&entry.path);
            let prefix = path.parent().unwrap();
            if !prefix.exists() {
                fs::create_dir_all(prefix).unwrap();
            }
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&entry.path)
                .with_context(|| format!("Failed to read tree: {}", &entry.path))
                .unwrap();

            file.write_all(get_object(&entry.oid, TypeObject::Blob).unwrap().as_bytes())
                .unwrap();
        }
    }

    pub fn get_tree(tree: &str) -> Result<Self> {
        let mut entries = vec![];

        for line in tree.lines() {
            let entry = Entry::from(line);
            match entry.obj_type {
                TypeObject::Blob => {
                    entries.push(entry);
                }
                TypeObject::Tree => {
                    let tmp_tree = get_object(&entry.oid, TypeObject::Tree)?;
                    let mut tmp_tree = Tree::get_tree(&tmp_tree)?;
                    entries.append(&mut tmp_tree.entries);
                }
                _ => return Err(anyhow!("Unknown tree entry.")),
            }
        }
        Ok(Tree { entries })
    }

    pub fn get_working_tree(ignore_options: &[String]) -> Result<Tree> {
        Tree::new(".", ignore_options)
    }

    fn is_ignored(path: &str, ignore_options: &[String]) -> bool {
        let path = path.to_string();
        if path.contains(".dsgit")
            || path.contains(".dsgitignore")
            || path.contains(".git")
            || path.contains(".gitignore")
            || path.contains(".github")
        {
            return true;
        }
        for ignore_path in ignore_options.iter() {
            if path.contains(ignore_path) {
                return true;
            }
        }
        false
    }
}
