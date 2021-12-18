use anyhow::{anyhow, Context, Result};
use hex;
use sha1::{Digest, Sha1};
use std::fs::{create_dir, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::str;
use std::str::FromStr;
use std::{fmt, fs};

const DSGIT_DIR: &str = ".dsgit";

pub fn init() -> Result<()> {
    create_dir(DSGIT_DIR)?;
    create_dir(format!("{}/objects", DSGIT_DIR))
        .with_context(|| format!("Failed to create a directory: {}/objects", DSGIT_DIR))?;
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

pub struct RefValue {
    pub ref_oid: String,
    pub symbolic: bool,
    pub value: String,
}

impl RefValue {
    pub fn new(ref_oid: &str, symbolic: bool, value: &str) -> Self {
        RefValue {
            ref_oid: ref_oid.to_owned(),
            symbolic,
            value: value.to_owned(),
        }
    }

    pub fn update_ref<'a>(refs: &'a str, ref_value: &'a RefValue) -> Result<&'a str> {
        assert!(!ref_value.symbolic);
        let refs = match RefValue::get_ref_internal(refs)? {
            Some(ref_value) => ref_value.ref_oid,
            // At first commit case, this returns None.
            None => refs.to_owned(),
        };
        let ref_path: String = format!("{}/{}", DSGIT_DIR, refs);
        let parent_path = Path::new(&ref_path).parent().unwrap();

        fs::create_dir_all(parent_path)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&ref_path)
            .with_context(|| format!("Failed to open object file: {}", ref_path))?;

        file.write_all(ref_value.value.as_bytes()).unwrap();
        Ok(&ref_value.value)
    }

    pub fn get_ref(refs: &str) -> Result<Option<RefValue>> {
        RefValue::get_ref_internal(refs)
    }

    fn get_ref_internal(refs: &str) -> Result<Option<RefValue>> {
        let ref_path = &format!("{}/{}", DSGIT_DIR, refs);
        if Path::new(ref_path).is_file() {
            let mut file = OpenOptions::new()
                .read(true)
                .open(ref_path)
                .with_context(|| format!("Failed to open file: {}", ref_path))?;

            let mut buf = String::from("");
            file.read_to_string(&mut buf)?;

            // This means a symbolic reference.
            if buf.is_empty() && buf.starts_with("ref:") {
                let refs = buf.split(':').collect::<Vec<&str>>()[1];
                return RefValue::get_ref_internal(refs);
            }

            Ok(Some(RefValue::new(refs, false, &buf)))
        } else {
            Ok(None)
        }
    }
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

#[cfg(test)]
mod test {
    use super::*;
    use serial_test::serial;
    use std::assert;
    use std::fs;
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
        let _ = init();
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
