# dsgit

[![CI](https://github.com/sott0n/dsgit/actions/workflows/ci.yml/badge.svg)](https://github.com/sott0n/dsgit/actions/workflows/ci.yml)

A toy version management system written in Rust.

## How to build

```
❯ 🍻 cargo build --release
```

## How to run

```
❯ 🍻 ./target/release/dsgit --help
dsgit: A toy version management system written in Rust.

USAGE:
    dsgit [COMMANDS]

COMMANDS:
    --help | -h                   : Show this help.
    init                          : Initialize dsgit, creating `.dsgit` directory.
    hash-object [FILE NAME]       : Given file, calculate hash object.
    cat-object [FILE NAME]        : Given object id, display object's contents.
    read-tree [OID]               : Read a tree objects from specified tree oid.
    write-tree                    : Write a tree objects structure into .dsgit.
    commit [MESSAGE]              : Record changes to the repository.
    switch [COMMIT]               : Switch branch or restore working tree's files.
    tag [TAG NAME] [COMMIT]       : Set a mark to commit hash.
    branch [BRANCH NAME] [COMMIT] : Diverge from the main line of development and continue to do work without messing with that main line.
    status                        : Display a current status of version management.
    reset [COMMIT]                : Reset to HEAD from specified commit hash.
    show [OID]                    : Display a commit object's contents.
```

## How to test

```
❯ 🍻 make test
```

## Acknowledgements
This toy project based on [ugit](https://www.leshenko.net/p/ugit/).
