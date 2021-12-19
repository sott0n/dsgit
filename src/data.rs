use crate::reference::RefValue;
use anyhow::{anyhow, Context, Result};
use hex;
use sha1::{Digest, Sha1};
use std::fmt;
use std::fs::{create_dir, File, OpenOptions};
use std::io::{Read, Write};
use std::str::FromStr;

const DSGIT_DIR: &str = ".dsgit";

pub fn init() -> Result<()> {
    create_dir(DSGIT_DIR)?;
    create_dir(format!("{}/objects", DSGIT_DIR))
        .with_context(|| format!("Failed to create a directory: {}/objects", DSGIT_DIR))?;

    RefValue::update_ref("HEAD", &RefValue::new(None, true, "refs/heads/main"), true)?;
    Ok(())
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum TypeObject {
    Blob,
    Tree,
    Commit,
}

impl FromStr for TypeObject {
    // TODO Define this error
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blob" => Ok(TypeObject::Blob),
            "tree" => Ok(TypeObject::Tree),
            "commit" => Ok(TypeObject::Commit),
            _ => Err(()),
        }
    }
}

impl fmt::Display for TypeObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TypeObject::Blob => write!(f, "blob"),
            TypeObject::Tree => write!(f, "tree"),
            TypeObject::Commit => write!(f, "commit"),
        }
    }
}

pub fn sha1_hash(data: impl AsRef<[u8]>, out: &mut [u8]) {
    let mut hasher = Sha1::new();
    hasher.update(data);
    out.copy_from_slice(&hasher.finalize())
}

pub fn hash_object(data: &str, type_obj: TypeObject) -> Result<String> {
    let obj = match type_obj {
        TypeObject::Blob => "blob".to_owned() + "\x00" + data,
        TypeObject::Tree => "tree".to_owned() + "\x00" + data,
        TypeObject::Commit => "commit".to_owned() + "\x00" + data,
    };

    let mut hash = [0u8; 20];
    sha1_hash(&obj.as_bytes(), &mut hash);
    let oid = hex::encode(&hash);

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(format!("{}/objects/{}", DSGIT_DIR, oid))
        .with_context(|| format!("Failed to open object file: objects/{}", oid))?;

    file.write_all(obj.as_bytes()).unwrap();
    Ok(oid)
}

pub fn get_object(oid: &str, expected_type: TypeObject) -> Result<String> {
    let mut file = File::open(format!("{}/objects/{}", DSGIT_DIR, oid))
        .with_context(|| format!("Failed to open object file: objects/{}", oid))?;

    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    let objs: Vec<&str> = buf.split('\x00').collect();
    if objs.len() != 2 {
        anyhow!("dsgit object must be obj type and contents");
    }

    let type_obj = TypeObject::from_str(objs[0]).unwrap();
    if type_obj != expected_type {
        anyhow!(
            "Missing object type, expected: {}, but got {}",
            expected_type,
            type_obj,
        );
    }

    Ok(objs[1].to_owned())
}

pub fn get_oid(name: &str) -> Result<String> {
    let refs_walk = [
        name.to_string(),
        format!("refs/{}", name),
        format!("refs/tags/{}", name),
        format!("refs/heads/{}", name),
    ];
    for path in refs_walk.iter() {
        match RefValue::get_ref(path, false)? {
            Some(ref_value) => return Ok(ref_value.value),
            None => continue,
        };
    }

    // Check a given name is hash value.
    let is_hex = name
        .chars()
        .collect::<Vec<char>>()
        .iter()
        .all(|c| c.is_ascii_hexdigit());
    if name.len() == 40 && is_hex {
        return Ok(name.to_string());
    }

    Err(anyhow!(format!(
        "Unknown name and not hash value: {}",
        name
    )))
}

#[cfg(test)]
mod test {
    use super::*;
    use serial_test::serial;
    use std::assert;
    use std::fs;
    use std::io;
    use std::io::BufRead;
    use std::path::Path;

    const TEST_DATA: [(&str, &str, &str, &str, &str); 3] = [
        (
            "./tests/hello.txt",
            "4963f4ed0612f7242d9d92bf59b4fb8ac8d29ec2", // Linux and MacOS
            "Hello World!\n",
            "f0981ab57ce65e2716df953d09c80478fd7dec1c", // Windows
            "Hello World!\r\n",
        ),
        (
            "./tests/cat.txt",
            "38d458fa6e384e24e7f15c5d17be0e9cee67f823", // Linux and MacOS
            "cat cat\n",
            "738355a2d1dda0b9f26feb6bb8e2de8f735bcd19", // Windows
            "cat cat\r\n",
        ),
        (
            "./tests/dogs.txt",
            "bdb10d71fac51e4952b37042faa62640cd7847db", // Linux and MacOS
            "dog dog dog\n",
            "45ce866627173403d0a0406d7c3f4cb54708ec1c", // Windows
            "dog dog dog\r\n",
        ),
    ];

    fn setup() {
        let _ = fs::remove_dir_all(DSGIT_DIR);
        init().unwrap();
    }

    fn assert_file_contents(path: &str, expects: Vec<String>) {
        let f1 = fs::File::open(path).unwrap();
        let f1_contents = io::BufReader::new(f1);
        for (got, expect) in f1_contents.lines().zip(expects) {
            assert_eq!(got.unwrap(), expect);
        }
    }

    #[test]
    #[serial]
    fn test_init() {
        // Remove `.dsgit` to pass init before this test.
        if Path::new(DSGIT_DIR).exists() {
            fs::remove_dir_all(DSGIT_DIR).unwrap();
        }

        init().unwrap();
        let head_path = format!("{}/HEAD", DSGIT_DIR);
        let expect_val = "ref:refs/heads/main".to_string();
        assert_file_contents(&head_path, vec![expect_val]);
    }

    #[test]
    #[serial]
    fn test_hash_object() {
        setup();
        for f in TEST_DATA.iter() {
            let contents = fs::read_to_string(f.0).unwrap();
            let hash = hash_object(&contents, TypeObject::Blob).unwrap();

            if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
                assert_eq!(hash, f.1);
                assert!(Path::new(&format!("{}/objects/{}", DSGIT_DIR, f.1)).exists());
            }
            if cfg!(target_os = "windows") {
                assert_eq!(hash, f.3);
                assert!(Path::new(&format!("{}/objects/{}", DSGIT_DIR, f.3)).exists());
            }
        }
    }

    #[test]
    #[serial]
    fn test_get_object() {
        setup();
        for f in TEST_DATA.iter() {
            let contents = fs::read_to_string(f.0).unwrap();
            let hash = hash_object(&contents, TypeObject::Blob).unwrap();
            let obj = get_object(&hash, TypeObject::Blob).unwrap();

            if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
                assert_eq!(obj, f.2);
            }
            if cfg!(target_os = "windows") {
                assert_eq!(obj, f.4);
            }
        }
    }
}
