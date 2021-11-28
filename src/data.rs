use anyhow::{anyhow, Context, Result};
use hex;
use sha1::{Digest, Sha1};
use std::fmt;
use std::fs::{create_dir, File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::str;
use std::str::FromStr;

const DSGIT_DIR: &str = ".dsgit";

pub fn init() -> io::Result<()> {
    create_dir(DSGIT_DIR)?;
    create_dir(format!("{}/objects", DSGIT_DIR))?;
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum TypeObject {
    Blob,
}

impl FromStr for TypeObject {
    // TODO Define this error
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blob" => Ok(TypeObject::Blob),
            _ => Err(()),
        }
    }
}

impl fmt::Display for TypeObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TypeObject::Blob => write!(f, "TypeObject::Blob"),
        }
    }
}

pub fn sha1_hash(data: impl AsRef<[u8]>, out: &mut [u8]) {
    let mut hasher = Sha1::new();
    hasher.update(data);
    out.copy_from_slice(&hasher.finalize())
}

pub fn hash_object(data: &str, type_obj: TypeObject) -> Result<()> {
    let obj = match type_obj {
        TypeObject::Blob => "blob".to_owned() + "\x00" + data,
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
    Ok(())
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
