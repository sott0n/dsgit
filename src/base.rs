use anyhow::{Context, Result};
use std::fs;

pub fn write_tree(target_path: &str) -> Result<()> {
    for entry in fs::read_dir(target_path)
        .with_context(|| format!("Failed to read directory: {}", target_path))?
    {
        let path = entry.unwrap().path();
        if is_ignored(&path.to_str().unwrap()) {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).unwrap();

        if metadata.is_file() {
            // TODO write the file to object store.
            println!("{:?}", path);
        }
        if metadata.is_dir() {
            let _ = write_tree(path.to_str().unwrap());
        }
    }

    Ok(())
}

fn is_ignored(path: &str) -> bool {
    let path = path.to_string();
    path.contains(".dsgit") || path.contains(".git")
}
