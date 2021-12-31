use crate::entry::Tree;
use std::collections::{HashMap, HashSet};

pub fn diff_trees(from: Tree, to: Tree) -> Vec<String> {
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
                    if to_oid != from_oid {
                        diff_entries.push(path.to_owned());
                    }
                }
                None => diff_entries.push(path.to_owned()),
            },
            None => match &to_tree.get(path) {
                Some(_) => diff_entries.push(path.to_owned()),
                None => continue,
            },
        }
    }

    diff_entries
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
