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
    Log(Option<String>),
    Cat(String),
    HashObject(String),
    ReadTree(String),
    Commit(String),
    Checkout(String),
    Tag((String, Option<String>)),
    Branch((String, Option<String>)),
}

fn check_args(args: &[String], expect_length: usize, err_msg: &'static str) -> Result<()> {
    if args.len() != expect_length {
        return Err(anyhow!(err_msg));
    }
    Ok(())
}

fn arg_parse() -> Result<Commands> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let cmd: Commands = match args[1].as_str() {
            "--help" | "-h" => Commands::Help,
            "init" => Commands::Init,
            "write-tree" => Commands::WriteTree,
            "log" => {
                if args.len() > 2 {
                    let oid: String = args[2].to_owned();
                    Commands::Log(Some(oid))
                } else {
                    Commands::Log(None)
                }
            }
            "cat-object" => {
                let err_msg = "dsgit: `cat-object` required commit-hash.";
                check_args(&args, 3, err_msg)?;
                let f: String = args[2].to_owned();
                Commands::Cat(f)
            }
            "hash-object" => {
                let err_msg = "dsgit: `hash-object` required commit-hash.";
                check_args(&args, 3, err_msg)?;
                let f: String = args[2].to_owned();
                Commands::HashObject(f)
            }
            "read-tree" => {
                let err_msg = "dsgit: `read-tree` required commit-hash.";
                check_args(&args, 3, err_msg)?;
                let f: String = args[2].to_owned();
                Commands::ReadTree(f)
            }
            "commit" | "-m" => {
                let err_msg = "dsgit: `commit` required '-m' or '--message' and message.";
                check_args(&args, 4, err_msg)?;
                match args[2].as_str() {
                    "-m" | "--message" => {
                        let msg: String = args[3].to_owned();
                        Commands::Commit(msg)
                    }
                    _ => return Err(anyhow!(err_msg)),
                }
            }
            "branch" => {
                if args.len() == 3 {
                    let branch_name: String = args[2].to_owned();
                    Commands::Branch((branch_name, None))
                } else {
                    let err_msg = "dsgit: `branch` required branch-name, and (option) commit-hash.";
                    check_args(&args, 4, err_msg)?;

                    let branch_name: String = args[2].to_owned();
                    let oid: String = args[3].to_owned();
                    Commands::Branch((branch_name, Some(oid)))
                }
            }
            "checkout" => {
                let err_msg = "dsgit: `checkout` required branch-name or commit-hash.";
                check_args(&args, 3, err_msg)?;
                let oid: String = args[2].to_owned();
                Commands::Checkout(oid)
            }
            "tag" => {
                if args.len() < 3 {
                    return Err(anyhow!(
                        "dsgit: `tag` required tag-name, and (option) commit-hash."
                    ));
                } else {
                    let tag: String = args[2].to_owned();
                    if args.len() == 3 {
                        Commands::Tag((tag, None))
                    } else {
                        let oid: String = args[3].to_owned();
                        Commands::Tag((tag, Some(oid)))
                    }
                }
            }
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

fn log(tag_or_oid: Option<String>) {
    let mut oid = match tag_or_oid {
        Some(tag_or_oid) => base::get_oid(&tag_or_oid).unwrap(),
        None => match data::RefValue::get_ref("HEAD").unwrap() {
            Some(ref_value) => ref_value.value,
            None => return,
        },
    };

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

fn hash_object(file: &str) {
    let contents = fs::read_to_string(file).unwrap();
    let hash = data::hash_object(&contents, data::TypeObject::Blob).unwrap();
    println!("{:#}", hash);
}

fn cat_object(tag_or_oid: &str) {
    let oid = base::get_oid(tag_or_oid).unwrap();
    let contents = data::get_object(&oid, data::TypeObject::Blob).unwrap();
    print!("{}", contents);
}

fn read_tree(tag_or_oid: &str, ignore_files: Vec<String>) {
    let oid = base::get_oid(tag_or_oid).unwrap();
    base::Tree::read_tree(&oid, &ignore_files);
}

fn write_tree(ignore_files: Vec<String>) {
    let oid = base::Tree::write_tree(".", &ignore_files).unwrap();
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
    let oid = base::Commit::commit(msg, &ignore_files).unwrap();
    println!("{:#}", oid);
}

fn checkout(tag_or_oid: &str, ignore_files: Vec<String>) {
    let oid = base::get_oid(tag_or_oid).unwrap();
    base::checkout(&oid, &ignore_files);
}

fn create_tag(tag: &str, tag_or_oid: &str) {
    let oid = base::get_oid(tag_or_oid).unwrap();
    base::create_tag(tag, &oid);
}

fn branch(name: &str, oid: &str) {
    base::create_branch(name, oid);
    println!("Created a branch: {} at {}", name, oid);
}

fn help() {
    println!(
        "\
dsgit: A toy version management system written in Rust.

USAGE:
    dsgit [COMMANDS]

COMMANDS:
    --help | -h                : Show this help.
    init                       : Initialize dsgit, creating `.dsgit` directory.
    hash-object [FILE NAME]    : Given file, calculate hash object.
    cat-object [FILE NAME]     : Given object id, display object's contents.
    read-tree [OID]            : Read a tree objects from specified tree oid.
    write-tree                 : Write a tree objects structure into .dsgit.
    commit [MESSAGE]           : Record changes to the repository.
    checkout [OID]             : Switch branch or restore working tree's files.
    tag [TAG NAME] [OID]       : Set a mark to commit hash.
    branch [BRANCH NAME] [OID] : Diverge from the main line of development and \
continue to do work without messing with that main line.
"
    );
    exit(0);
}

fn main() {
    match arg_parse().unwrap() {
        Commands::Help => help(),
        Commands::Init => init(),
        Commands::Log(oid) => log(oid),
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
        Commands::Checkout(oid) => {
            let ignore_files = read_ignore_file();
            checkout(&oid, ignore_files);
        }
        Commands::Tag((tag, oid_or_none)) => {
            let oid = match oid_or_none {
                Some(oid) => oid,
                None => data::RefValue::get_ref("HEAD").unwrap().unwrap().value,
            };
            create_tag(&tag, &oid);
        }
        Commands::Branch((name, oid_or_none)) => {
            let oid = match oid_or_none {
                Some(oid) => oid,
                None => data::RefValue::get_ref("HEAD").unwrap().unwrap().value,
            };
            branch(&name, &oid);
        }
    }
}
