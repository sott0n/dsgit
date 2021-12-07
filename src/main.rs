pub mod base;
pub mod data;

use anyhow::{anyhow, Result};
use std::env;
use std::fs;
use std::path::Path;
use std::process::exit;

enum Commands {
    Help,
    Init,
    WriteTree,
    Log,
    Cat(String),
    HashObject(String),
    ReadTree(String),
    Commit(String),
}

fn arg_parse() -> Result<Commands> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let cmd: Commands = match args[1].as_str() {
            "--help" | "-h" => Commands::Help,
            "init" => Commands::Init,
            "write-tree" => Commands::WriteTree,
            "log" => Commands::Log,
            "cat-object" => {
                let f: String = args[2].to_owned();
                Commands::Cat(f)
            }
            "hash-object" => {
                let f: String = args[2].to_owned();
                Commands::HashObject(f)
            }
            "read-tree" => {
                let f: String = args[2].to_owned();
                Commands::ReadTree(f)
            }
            "commit" | "-m" => match args[2].as_str() {
                "-m" | "--message" => {
                    let msg: String = args[3].to_owned();
                    Commands::Commit(msg)
                }
                _ => return Err(anyhow!("dsgit: commit required '-m' or '--message'.")),
            },
            _ => {
                return Err(anyhow!(
                    "dsgit: '{}' is not a dsgit command. See 'dsgit --help'.",
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
    let path = env::current_dir().unwrap();
    println!(
        "Initialized empty DSGit repository in {}/.dsgit",
        path.display()
    );
}

fn log() {
    if let Some(mut oid) = data::get_head().unwrap() {
        loop {
            let commit = base::Commit::get_commit(&oid).unwrap();
            println!("commit {:#}", &oid);
            println!("tree   {:#}", &commit.tree);
            if let Some(parent_oid) = &commit.parent {
                println!("parent {:#}", parent_oid);
            }
            println!("\n{:ident$}{:#}", "", &commit.message, ident = 4);
            println!();

            oid = match commit.parent {
                Some(oid) => oid,
                None => break,
            }
        }
    }
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

fn read_tree(oid: &str, ignore_files: Vec<String>) {
    base::read_tree(oid, &ignore_files);
}

fn write_tree(ignore_files: Vec<String>) {
    let oid = base::write_tree(".", &ignore_files).unwrap();
    println!("{:#}", oid);
}

fn read_ignore_file() -> Vec<String> {
    let ignore_file_path: &str = ".dsgitignore";
    let mut ignore_files = vec![];

    if Path::new(ignore_file_path).exists() {
        let contents = fs::read_to_string(ignore_file_path).unwrap();
        for file_name in contents.lines() {
            ignore_files.push(file_name.to_string());
        }
    }
    ignore_files
}

fn commit(msg: &str, ignore_files: Vec<String>) {
    let oid = base::commit(msg, &ignore_files).unwrap();
    println!("{:#}", oid);
}

fn help() {
    println!(
        "\
dsgit: Version management system for dataset written in Rust.

USAGE:
    dsgit [COMMANDS]

COMMANDS:
    --help | -h             : Show this help
    init                    : Initialize dsgit
    hash-object [FILE NAME] : Given file, calculate hash object.
    cat-object [FILE NAME]  : Given object id, display object's contents.
    read-tree  [OID]        : Read a tree objects from specified tree oid.
    write-tree              : Write a tree objects structure into .dsgit.
    commit                  : Commit version.
"
    );
    exit(0);
}

fn main() {
    match arg_parse().unwrap() {
        Commands::Help => help(),
        Commands::Init => init(),
        Commands::Log => log(),
        Commands::Cat(file) => cat_object(&file),
        Commands::HashObject(file) => hash_object(&file),
        Commands::ReadTree(oid) => {
            let ignore_files = read_ignore_file();
            read_tree(&oid, ignore_files);
        }
        Commands::WriteTree => {
            let ignore_files = read_ignore_file();
            write_tree(ignore_files);
        }
        Commands::Commit(msg) => {
            let ignore_files = read_ignore_file();
            commit(&msg, ignore_files);
        }
    }
}
