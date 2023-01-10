## cosh

[![Build Status](https://github.com/tomhrr/cosh/workflows/build/badge.svg?branch=main)](https://github.com/tomhrr/cosh/actions)
[![stability-beta](https://img.shields.io/badge/stability-beta-33bbff.svg)](https://github.com/mkenney/software-guides/blob/master/STABILITY-BADGES.md#beta)

cosh is a concatenative command-line shell.

### Why?

Basic shell operations like `ls`, `ps`, `stat`, and so on are
implemented as functions that return first-class values, as opposed to
relying on executables that return text streams.  This makes working
with the results simpler:

- Find files matching a path, and search them for data:
  - **sh**:&nbsp;&nbsp;&nbsp;&nbsp; `find . -print0 | xargs -0 grep data`
  - **cosh**: `lsr; [f<; [data m] grep] map`

- Find the total size of all files in the current directory:
  - **sh**:&nbsp;&nbsp;&nbsp;&nbsp; `ls | xargs stat -c %s | awk '{s+=$1} END {print s}' -`
  - **cosh**: `ls; [stat; size get] map; sum`

A small set of versatile primitives means that less needs to be
remembered when compared with typical shells (see e.g. the various
flags for `cut(1)`), though some commands may be longer as a result:

- Get the second and third columns from each row of a CSV file:
  - **sh**:&nbsp;&nbsp;&nbsp;&nbsp; `cat test-data/csv | cut -d, -f2,3`
  - **cosh**: `test-data/csv f<; [chomp; , split; (1 2) get] map`

- Sort files by modification time:
  - **sh**:&nbsp;&nbsp;&nbsp;&nbsp; `ls -tr`
  - **cosh**: `ls; [[stat; mtime get] 2 apply; <=>] sortp`

Arithmetical operators and XML/JSON/CSV encoding/decoding functions
reduce the number of times that it becomes necessary to use a more
full-featured programming language or a third-party executable:

- Increment floating-point numbers in file:
  - **sh**:&nbsp;&nbsp;&nbsp;&nbsp; `sed 's/$/+10/' nums | bc`
  - **cosh**: `nums f<; [chomp; 10 +] map;`

- Get the first value from the "zxcv" array member of a JSON file:
  - **sh**:&nbsp;&nbsp;&nbsp;&nbsp; `cat test-data/json2 | jq .zxcv[0]`
  - **cosh**: `test-data/json2 f<; from-json; zxcv get; 0 get`

It also integrates with external executable calls, where that is
necessary:

- Print certificate data:
  - **bash**:&nbsp;&nbsp;&nbsp;&nbsp; `for i in ``find . -iname '*.pem'``; do openssl x509 -in $i -text -noout; done`
  - **cosh**: `lsr; [pem$ m] grep; [{openssl x509 -in {} -text -noout}] map;`

### Install

#### Dependencies

 - [Rust](https://github.com/rust-lang/rust)

### Supported systems

This has been tested on Linux (Debian 12), but should work on any
Linux/macOS/BSD system where Rust can be built.

#### Building

    make
    make test
    sudo make install

Apart from the core `cosh` executable, this will also install a
compiled library of core functions (`rt.chc`).

#### Running

    user@host:/$ cosh
    /$ hello println;
    hello

### Documentation

[Documentation](./doc/all.md)

### Licence

See [LICENCE](./LICENCE).
