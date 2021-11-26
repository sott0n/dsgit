use sha1::{Digest, Sha1};
use std::fs::{create_dir, File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::str;

const DSGIT_DIR: &str = ".dsgit";

pub fn init() -> io::Result<()> {
    create_dir(DSGIT_DIR)?;
    create_dir(format!("{}/objects", DSGIT_DIR))?;
    Ok(())
}

pub fn sha1_hash(data: impl AsRef<[u8]>, out: &mut [u8]) {
    let mut hasher = Sha1::new();
    hasher.update(data);
    out.copy_from_slice(&hasher.finalize())
}

pub fn hash_object(data: &str) -> io::Result<()> {
    let mut hash = [0u8; 20];
    sha1_hash(&data.as_bytes(), &mut hash);
    let oid = hex::encode(&hash);

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(format!("{}/objects/{}", DSGIT_DIR, oid))
        .unwrap();

    write!(file, "{}", &data).unwrap();
    Ok(())
}

pub fn get_object(oid: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = File::open(format!("{}/objects/{}", DSGIT_DIR, oid)).expect("file not found");
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    Ok(buf)
}
