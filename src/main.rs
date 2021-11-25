use anyhow::Result;
use std::env;
use std::fs::create_dir;
use std::process::exit;

enum Commands {
    Help,
    Init,
}

fn arg_parse() -> Commands {
    for arg in env::args().skip(1) {
        if arg == "init" {
            return Commands::Init;
        }
    }
    Commands::Help
}

const DSGIT_DIR: &str = ".dsgit";

fn init() -> std::io::Result<()> {
    create_dir(DSGIT_DIR)?;
    println!("Initialized dsgit");
    Ok(())
}

fn help() {
    println!(
        "\
dsgit: Version management system for dataset written in Rust.

USAGE:
    dsgit [COMMANDS]

COMMANDS:
    init        : Initialize dsgit
    --help | -h : Show this help"
    );
    exit(0);
}

fn main() {
    let cmd = arg_parse();
    match cmd {
        Commands::Help => help(),
        Commands::Init => init().unwrap(),
    }
}
