pub mod base;
pub mod data;

use anyhow::{anyhow, Result};
use std::env;
use std::fs;
use std::process::exit;

const TARGET_PATH: &str = "./tests";

enum Commands {
    Help,
    Init,
    WriteTree,
    Cat(String),
    HashObject(String),
    ReadTree(String),
}

fn arg_parse() -> Result<Commands> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let cmd: Commands = match args[1].as_str() {
            "--help" | "-h" => Commands::Help,
            "init" => Commands::Init,
            "hash-object" => {
                let f: String = args[2].to_owned();
                Commands::HashObject(f)
            }
            "cat-object" => {
                let f: String = args[2].to_owned();
                Commands::Cat(f)
            }
            "read-tree" => {
                let f: String = args[2].to_owned();
                Commands::ReadTree(f)
            }
            "write-tree" => Commands::WriteTree,
            _ => {
                return Err(anyhow!(
                    "tgit: '{}' is not a dsgit command. See 'dsgit --help'.",
                    args[1]
                ))
            }
        };
        Ok(cmd)
    } else {
        // Not given command pattern.
        Ok(Commands::Help)
    }
}

fn init() {
    data::init().unwrap();
    println!("Initialized dsgit with creating '.dsgit'.");
}

fn hash_object(file: &str) {
    let contents = fs::read_to_string(file).unwrap();
    let hash = data::hash_object(&contents, data::TypeObject::Blob).unwrap();
    println!("{:#}", hash);
}

fn cat_object(file: &str) {
    let contents = data::get_object(file, data::TypeObject::Blob).unwrap();
    print!("{}", contents);
}

fn read_tree(oid: &str) {
    base::read_tree(oid);
}

fn write_tree() {
    let oid = base::write_tree(TARGET_PATH).unwrap();
    println!("{:#}", oid);
}

fn help() {
    println!(
        "\
dsgit: Version management system for dataset written in Rust.

USAGE:
    dsgit [COMMANDS]

COMMANDS:
    init                    : Initialize dsgit
    hash-object [FILE NAME] : Given file, calculate hash object.
    cat-object [FILE NAME]  : Given object id, display object's contents.
    read-tree  [OID]        : Read a tree objects from specified tree oid.
    write-tree              : Write a tree objects structure into .dsgit.
    --help | -h             : Show this help"
    );
    exit(0);
}

fn main() {
    match arg_parse().unwrap() {
        Commands::Help => help(),
        Commands::Init => init(),
        Commands::Cat(file) => cat_object(&file),
        Commands::HashObject(file) => hash_object(&file),
        Commands::ReadTree(oid) => read_tree(&oid),
        Commands::WriteTree => write_tree(),
    }
}
