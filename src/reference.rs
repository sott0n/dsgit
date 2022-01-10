use crate::commit::Commit;
use crate::data::get_oid;
use crate::entry::Tree;
use anyhow::{anyhow, Context, Result};
use std::fs::{create_dir_all, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::str;
use walkdir::WalkDir;

const DSGIT_DIR: &str = ".dsgit";

#[derive(Debug)]
pub struct RefValue {
    pub ref_oid: Option<String>,
    pub symbolic: bool,
    pub value: String,
}

impl RefValue {
    pub fn new(ref_oid: Option<&str>, symbolic: bool, value: &str) -> Self {
        RefValue {
            ref_oid: ref_oid.map(|oid| oid.to_owned()),
            symbolic,
            value: value.to_owned(),
        }
    }

    pub fn update_ref<'a>(refs: &'a str, ref_value: &'a RefValue, deref: bool) -> Result<String> {
        let refs = match RefValue::get_ref_internal(refs, deref)? {
            Some(ref_value) => ref_value.ref_oid.unwrap(),
            // At first commit case, this returns None.
            None => refs.to_owned(),
        };

        assert!(!ref_value.value.is_empty());
        let value: String = if ref_value.symbolic {
            String::from("ref:") + &ref_value.value
        } else {
            ref_value.value.to_owned()
        };

        let ref_path: String = format!("{}/{}", DSGIT_DIR, refs);
        let parent_path = Path::new(&ref_path).parent().unwrap();

        create_dir_all(parent_path)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&ref_path)
            .with_context(|| format!("Failed to open object file: {}", ref_path))?;

        file.write_all(value.as_bytes()).unwrap();
        file.flush().unwrap();
        Ok(value)
    }

    pub fn get_ref(refs: &str, deref: bool) -> Result<Option<RefValue>> {
        RefValue::get_ref_internal(refs, deref)
    }

    fn get_ref_internal(refs: &str, deref: bool) -> Result<Option<RefValue>> {
        let ref_path = &format!("{}/{}", DSGIT_DIR, refs);
        if Path::new(ref_path).is_file() {
            let mut file = OpenOptions::new()
                .read(true)
                .open(ref_path)
                .with_context(|| format!("Failed to open file: {}", ref_path))?;

            let mut value = String::from("");
            file.read_to_string(&mut value)?;

            // This means a symbolic reference.
            let symbolic = !value.is_empty() && value.starts_with("ref:");
            if symbolic {
                value = value.split(':').collect::<Vec<&str>>()[1].to_string();
                if deref {
                    return RefValue::get_ref_internal(&value, true);
                }
            };
            Ok(Some(RefValue::new(Some(refs), symbolic, &value)))
        } else {
            Ok(None)
        }
    }

    pub fn get_refs(prefix: Option<&str>, rel_path: &str) -> Result<Vec<String>> {
        let mut refs = vec![String::from("HEAD")];
        let prefix_root = format!("{}/{}", DSGIT_DIR, rel_path);
        let prefix_rel_path = Path::new(&prefix_root);
        for entry in WalkDir::new(format!("{}/refs/", DSGIT_DIR))
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir())
        {
            if let Some(p) = prefix {
                let file_name = entry.file_name().to_str().unwrap();
                if file_name.starts_with(p) {
                    continue;
                }
            }
            let ref_path = entry.path().strip_prefix(prefix_rel_path)?;
            refs.push(ref_path.to_str().unwrap().to_owned());
        }

        Ok(refs)
    }

    pub fn switch(name: &str, ignore_options: &[String]) -> Result<()> {
        let oid = get_oid(name).unwrap();
        let commit = Commit::get_commit(&oid).unwrap();
        Tree::read_tree(&commit.tree, ignore_options)?;

        let head_ref = if RefValue::is_branch(name) {
            let value = String::from("refs/heads/") + name;
            RefValue::new(Some(&oid), true, &value)
        } else {
            RefValue::new(Some(&oid), false, &oid)
        };

        RefValue::update_ref("HEAD", &head_ref, false).unwrap();
        Ok(())
    }

    fn is_branch(name: &str) -> bool {
        let p = String::from("refs/heads/") + name;
        RefValue::get_ref(&p, true).unwrap().is_some()
    }

    pub fn get_branch_name() -> Result<Option<String>> {
        let head_ref = match RefValue::get_ref("HEAD", false)? {
            Some(head_ref) => head_ref,
            None => return Err(anyhow!("A `HEAD` file is not found.")),
        };

        if !head_ref.symbolic {
            return Ok(None);
        };

        assert!(head_ref.value.starts_with("refs/heads/"));
        let head_path = Path::new(&head_ref.value);
        let base_path = Path::new("refs/heads");
        let rel_path = head_path
            .strip_prefix(base_path)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        Ok(Some(rel_path))
    }
}

pub fn create_tag(tag: &str, oid: &str) {
    let ref_value = RefValue::new(Some(oid), false, oid);
    RefValue::update_ref(&format!("refs/tags/{}", tag), &ref_value, true).unwrap();
}

pub fn create_branch(name: &str, oid: &str) {
    let ref_name = String::from("refs/heads/") + name;
    let ref_value = RefValue::new(Some(oid), false, oid);
    RefValue::update_ref(&ref_name, &ref_value, true).unwrap();
}

pub fn reset(commit: &str) {
    let ref_value = RefValue::new(Some(commit), false, commit);
    RefValue::update_ref("HEAD", &ref_value, true).unwrap();
}
