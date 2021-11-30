pub mod base;
pub mod data;

use anyhow::{anyhow, Result};
use std::env;
use std::fs;
use std::process::exit;

enum Commands {
    Help,
    Init,
    Tree,
    Cat(String),
    HashObject(String),
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
            "write-tree" => Commands::Tree,
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

fn write_tree() {
    // TODO Test directory, it will be removed in future.
    let target_path = "./tests";
    let oid = base::write_tree(target_path).unwrap();
    println!("{:#}", oid);
}

fn help() {
    println!(
        "\
dsgit: Version management system for dataset written in Rust.

USAGE:
    dsgit [COMMANDS]

COMMANDS:
    init        : Initialize dsgit
    hash-object : Given file, calculate hash object.
    cat-object  : Given object id, display object's contents.
    --help | -h : Show this help"
    );
    exit(0);
}

fn main() {
    match arg_parse().unwrap() {
        Commands::Help => help(),
        Commands::Init => init(),
        Commands::Cat(file) => cat_object(&file),
        Commands::HashObject(file) => hash_object(&file),
        Commands::Tree => write_tree(),
    }
}
