## cosh

[![Build Status](https://github.com/tomhrr/cosh/workflows/build/badge.svg?branch=main)](https://github.com/tomhrr/cosh/actions)
[![stability-beta](https://img.shields.io/badge/stability-beta-33bbff.svg)](https://github.com/mkenney/software-guides/blob/master/STABILITY-BADGES.md#beta)

cosh is a concatenative command-line shell.

 * [Why?](#why)
 * [Install](#install)
    * [Dependencies](#dependencies)
    * [Supported systems](#supported-systems)
    * [Building](#building)
    * [Running](#running)
 * [Examples](#examples)
 * [Documentation](#documentation)
 * [Licence](#licence)

### Why?

Basic shell operations like `ls`, `ps`, `stat`, and so on are
implemented as functions that return first-class values, as opposed to
relying on executables that return text streams.  This makes working
with the results simpler:

- Find files matching a path, and search them for data:
  - **sh**:&nbsp;&nbsp;&nbsp;&nbsp; `find . -iname '*test*' -print0 | xargs -0 grep data`
  - **cosh**: `lsr; [test m] grep; [f<; [data m] grep] map`

- Find the total size of all files in the current directory:
  - **sh**:&nbsp;&nbsp;&nbsp;&nbsp; `ls | xargs stat -c %s | awk '{s+=$1} END {print s}' -`
  - **cosh**: `ls; [is-dir; not] grep; [stat; size get] map; sum`

A small set of versatile primitives means that less needs to be
remembered when compared with typical shells (see e.g. the various
flags for `cut(1)`), though some commands may be longer as a result:

- Get the second and third columns from each row of a CSV file:
  - **sh**:&nbsp;&nbsp;&nbsp;&nbsp; `cut -d, -f2,3 test-data/csv`
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
  - **sh**:&nbsp;&nbsp;&nbsp;&nbsp; `jq .zxcv[0] test-data/json2`
  - **cosh**: `test-data/json2 f<; from-json; zxcv get; 0 get`

It also integrates with external executable calls, where that is
necessary:

- Print certificate data:
  - **bash**: ``for i in `find . -iname '*.pem'`; do openssl x509 -in $i -text -noout; done``
  - **cosh**: `lsr; [pem$ m] grep; [{openssl x509 -in {} -text -noout}] map;`

See the full [documentation](./doc/all.md) for more details.

### Install

#### Dependencies

 - [Rust](https://github.com/rust-lang/rust)

#### Supported systems

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

### Examples

Each example starts from the repository clone directory.

List files in a specified directory:

    cosh$ test-data ls
    v[gen (
        0: test-data/json-bigint
        1: test-data/json2
        2: test-data/json1
        3: test-data/readfile
        4: test-data/csv
        5: test-data/split
    )]
    cosh$

Sort files alphabetically in a specified directory:

    cosh$ test-data ls; sort
    (
        0: test-data/csv
        1: test-data/json-bigint
        2: test-data/json1
        3: test-data/json2
        4: test-data/readfile
        5: test-data/split
    )
    cosh$

An external command can be run by prefixing the command with a space:

    cosh$  vim test-data/csv
    ...

Read a file into memory:

    cosh$ test-data/csv f<;
    v[gen (
        0: "1,2,3,4\n"
        1: "5,6,7,8\n"
        2: "9,10,11,12\n"
    )]
    cosh$

For each line of a CSV file, remove the newline and split on commas:

    cosh$ test-data/csv f<; [chomp; , split] map;
    v[gen (
        0: (
            0: 1
            1: 2
            2: 3
            3: 4
        )
        1: (
            0: 5
            1: 6
            2: 7
            3: 8
        )
        2: (
            0: 9
            1: 10
            2: 11
            3: 12
        )
    )]
    cosh$

Read a JSON file into memory:

    cosh$ test-data/json2 f<; from-json;
    h(
        "asdf": 1
        "qwer": 2
        "tyui": h(
            "asdf": 5
        )
        "zxcv": (
            0: 3
            1: 4
        )
    )
    cosh$

Get the field names from the JSON file, and print them to standard
output:

    cosh$ test-data/json2 f<; from-json; keys; println for;
    asdf
    qwer
    tyui
    zxcv
    cosh$

Find the field names that match a given regex:

    cosh$ test-data/json2 f<; from-json; keys; [.{4} m] grep;
    v[gen (
        0: asdf
        1: qwer
        2: tyui
        3: zxcv
    )]
    cosh$

    cosh$ test-data/json2 f<; from-json; keys; [a..f m] grep;
    v[gen (
        0: asdf
    )]
    cosh$

Define and use a new function:

    cosh$ : add-5 5 +; ,,
    cosh$ (1 2 3) add-5 map;
    (
        0: 6
        1: 7
        2: 8
    )
    cosh$

Capture a value using a regex:

    cosh$ test-data ls;
    v[gen (
        0: test-data/json-bigint
        1: test-data/json2
        2: test-data/json1
        3: test-data/readfile
        4: test-data/csv
        5: test-data/split
    )]
    cosh$ test-data ls; ["(/.*)" c; shift] map;
    v[gen (
        0: /json-bigint
        1: /json2
        2: /json1
        3: /readfile
        4: /csv
        5: /split
    )]
    cosh$

Print a path's modification time in a specific format:

    cosh$ test-data stat; mtime get; from-epoch; %F strftime;
    2023-01-20
    cosh$

### Documentation

[Documentation](./doc/all.md)

### Licence

See [LICENCE](./LICENCE).
