use crate::data::{get_object, TypeObject};
use crate::entry::Tree;

use std::collections::{HashMap, HashSet};
use std::fmt;

use anyhow::Result;
use console::{style, Style};
use similar::{ChangeTag, TextDiff};

pub fn diff_trees(from: Tree, to: Tree) -> Result<Vec<String>> {
    fn convert_dict(tree: Tree) -> HashMap<String, String> {
        let mut tree_dict: HashMap<String, String> = HashMap::new();
        for entry in tree.entries.iter() {
            tree_dict.insert(entry.path.to_owned(), entry.oid.to_owned());
        }
        tree_dict
    }

    let from_tree = convert_dict(from);
    let to_tree = convert_dict(to);

    // Extract unique paths vector from current tree and parent tree.
    let mut paths = from_tree.keys().cloned().collect::<Vec<String>>();
    let mut to_paths = to_tree.keys().cloned().collect::<Vec<String>>();
    paths.append(&mut to_paths);
    let uniq_paths: HashSet<String> = paths.iter().cloned().collect();

    let mut diff_entries: Vec<String> = vec![];
    for path in uniq_paths.iter() {
        match &from_tree.get(path) {
            Some(from_oid) => match &to_tree.get(path) {
                Some(to_oid) => {
                    if from_oid != to_oid {
                        println!("Changed: {}", path);
                        display_diff_file(Some(from_oid), Some(to_oid))?;
                        diff_entries.push(path.to_owned());
                    }
                }
                None => {
                    println!("Removed: {}", path);
                    display_diff_file(Some(from_oid), None)?;
                    diff_entries.push(path.to_owned());
                }
            },
            None => match &to_tree.get(path) {
                Some(to_oid) => {
                    println!("Created: {}", path);
                    display_diff_file(None, Some(to_oid))?;
                    diff_entries.push(path.to_owned());
                }
                None => continue,
            },
        }
    }

    Ok(diff_entries)
}

struct Line(Option<usize>);

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            None => write!(f, "    "),
            Some(idx) => write!(f, "{:<4}", idx + 1),
        }
    }
}

fn display_diff_file(old_oid: Option<&str>, new_oid: Option<&str>) -> Result<()> {
    let old_contents = match old_oid {
        Some(oid) => get_object(oid, TypeObject::Blob)?,
        None => String::from(""),
    };
    let new_contents = match new_oid {
        Some(oid) => get_object(oid, TypeObject::Blob)?,
        None => String::from(""),
    };

    let diff = TextDiff::from_lines(&old_contents, &new_contents);
    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            println!("{:-^1$}", "-", 80);
        }
        for op in group {
            for change in diff.iter_inline_changes(op) {
                let (sign, s) = match change.tag() {
                    ChangeTag::Delete => ("-", Style::new().red()),
                    ChangeTag::Insert => ("+", Style::new().green()),
                    ChangeTag::Equal => (" ", Style::new().dim()),
                };
                print!(
                    "{}{} |{}",
                    style(Line(change.old_index())).dim(),
                    style(Line(change.new_index())).dim(),
                    s.apply_to(sign).bold(),
                );
                for (emphasized, value) in change.iter_strings_lossy() {
                    if emphasized {
                        print!("{}", s.apply_to(value).underlined().on_black());
                    } else {
                        print!("{}", s.apply_to(value));
                    }
                }
                if change.missing_newline() {
                    println!();
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::data::TypeObject;
    use crate::entry::{Entry, Tree};

    #[test]
    fn test_diff_trees() {
        let from_tree = Tree {
            entries: vec![
                Entry {
                    path: "test_a".to_string(),
                    oid: "1111".to_string(),
                    obj_type: TypeObject::Blob,
                },
                Entry {
                    path: "test_b".to_string(),
                    oid: "2222".to_string(),
                    obj_type: TypeObject::Blob,
                },
                Entry {
                    path: "test_c".to_string(),
                    oid: "3333".to_string(),
                    obj_type: TypeObject::Blob,
                },
            ],
        };
        let to_tree = Tree {
            entries: vec![
                Entry {
                    path: "test_a".to_string(),
                    oid: "1111".to_string(),
                    obj_type: TypeObject::Blob,
                },
                Entry {
                    path: "test_d".to_string(),
                    oid: "4444".to_string(),
                    obj_type: TypeObject::Blob,
                },
                Entry {
                    path: "test_c".to_string(),
                    oid: "3333".to_string(),
                    obj_type: TypeObject::Blob,
                },
            ],
        };

        let mut trees = diff_trees(from_tree, to_tree);
        trees.sort();
        assert_eq!(trees, vec!["test_b", "test_d"]);
    }
}
