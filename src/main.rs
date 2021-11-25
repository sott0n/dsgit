pub mod data;

use std::env;
use std::fs;
use std::process::exit;

enum Commands {
    Help,
    Init,
    HashObject(String),
}

fn arg_parse() -> Commands {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        if args[1] == "init" {
            return Commands::Init;
        }

        if args[1] == "hash-object" {
            let f: String = args[2].to_string();
            return Commands::HashObject(f);
        }
    }

    Commands::Help
}

fn init() {
    data::init().unwrap();
    println!("Initialized dsgit");
}

fn hash_object(file: &str) {
    let contents = fs::read_to_string(file).unwrap();
    data::hash_object(contents.as_bytes()).unwrap();
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
    --help | -h : Show this help"
    );
    exit(0);
}

fn main() {
    let cmd = arg_parse();
    match cmd {
        Commands::Help => help(),
        Commands::Init => init(),
        Commands::HashObject(file) => hash_object(&file),
    }
}
