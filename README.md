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

- Find file paths matching a string, and search those files for data

  <div>
    <table>
      <tr>
        <td><b>sh</b></td>
        <td><tt>find . -iname '*test*' -print0 | xargs -0 grep data</tt></td>
      </tr>
      <tr>
        <td><b>cosh</b></td>
        <td><tt>lsr; [test m] grep; [f<; [data m] grep] map</tt></td>
      </tr>
    </table>
  </div>

- Find all processes using more than 500M of memory:

  <div>
    <table>
      <tr>
        <td><b>sh</b></td>
        <td><tt>ps --no-headers aux | awk '$6>500000'</tt></td>
      </tr>
      <tr>
        <td><b>cosh</b></td>
        <td><tt>ps; [mem get; 1000 1000 *; 500 *; &gt;] grep</tt></td>
      </tr>
    </table>
  </div>

A small set of versatile primitives means that less needs to be
remembered when compared with typical shells (see e.g. the various
flags for `cut(1)`), though some commands may be longer as a result:

- Get the second and third columns from each row of a CSV file:

  <div>
    <table>
      <tr>
        <td><b>sh</b></td>
        <td><tt>cut -d, -f2,3 test-data/csv</tt></td>
      </tr>
      <tr>
        <td><b>cosh</b></td>
        <td><tt>test-data/csv f<; [chomp; , split; (1 2) get] map</tt></td>
      </tr>
    </table>
  </div>

- Sort files by modification time:

  <div>
    <table>
      <tr>
        <td><b>sh</b></td>
        <td><tt>ls -tr</tt></td>
      </tr>
      <tr>
        <td><b>cosh</b></td>
        <td><tt>ls; [[stat; mtime get] 2 apply; <=>] sortp</tt></td>
      </tr>
    </table>
  </div>

Arithmetical operators and XML/JSON/YAML/CSV encoding/decoding
functions reduce the number of times that it becomes necessary to use
a more full-featured programming language or a third-party executable:

- Increment floating-point numbers in file:

  <div>
    <table>
      <tr>
        <td><b>sh</b></td>
        <td><tt>sed 's/$/+10/' nums | bc</tt></td>
      </tr>
      <tr>
        <td><b>cosh</b></td>
        <td><tt>nums f<; [chomp; 10 +] map</tt></td>
      </tr>
    </table>
  </div>

- Get the first value from the "zxcv" array member of a JSON file:

  <div>
    <table>
      <tr>
        <td><b>sh</b></td>
        <td><tt>jq .zxcv[0] test-data/json2</tt></td>
      </tr>
      <tr>
        <td><b>cosh</b></td>
        <td><tt>test-data/json2 f<; from-json; zxcv get; 0 get</tt></td>
      </tr>
    </table>
  </div>

It also integrates with external executable calls, where that is
necessary:

- Print certificate data:

  <div>
    <table>
      <tr>
        <td><b>bash</b></td>
        <td><tt>for i in `find . -iname '*.pem'`; do openssl x509 -in $i -text -noout; done</tt></td>
      </tr>
      <tr>
        <td><b>cosh</b></td>
        <td><tt>lsr; [pem$ m] grep; [{openssl x509 -in {} -text -noout}] map;</tt></td>
      </tr>
    </table>
  </div>

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
        0: test-data/cert.der
        1: test-data/json-bigint
        2: test-data/json2
        3: test-data/json1
        4: test-data/readfile
        5: test-data/yaml1.yml
        6: test-data/test.ch
        7: test-data/csv
        8: test-data/split
        9: test-data/readlines
    )]
    cosh$

Sort files alphabetically in a specified directory:

    cosh$ test-data ls; sort
    (
        0: test-data/cert.der
        1: test-data/csv
        2: test-data/json-bigint
        3: test-data/json1
        4: test-data/json2
        5: test-data/readfile
        6: test-data/readlines
        7: test-data/split
        8: test-data/test.ch
        9: test-data/yaml1.yml
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
    cosh$ test-data ls; ["/.*" c; shift] map;
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

Find the ping times for a series of domain names, in parallel:

    cosh$ (sourcehut.org github.com gitlab.com) [dup; A dig; answer.0.sdata.address get; 1 pingn; 0 get; 2 mlist] pmap;
    v[channel-gen (
        0: (
            0: gitlab.com
            1: h(
                "icmp_seq": 1
                "ttl":      58
                "time_ms":  11.6
            )
        )
        1: (
            0: github.com
            1: h(
                "icmp_seq": 1
                "ttl":      115
                "time_ms":  28.4
            )
        )
        2: (
            0: sourcehut.org
            1: h(
                "icmp_seq": 1
                "ttl":      52
                "time_ms":  346
            )
        )
    )]
    cosh$

Get the total number of hosts in a set of IP address ranges:

    cosh$ (1.0.0.0/24 2.0.0.0/14 3.0.0.0/8) [ip; ip.size] map; sum
    17039616
    cosh$

Create a new SQLite database, add a table to the database, and add a
record to the table:

    cosh$ mydb touch
    cosh$ mydb sqlite db.conn; c var; c !
    cosh$ c @; "create table test (id, num)" db.prep; () db.exec
    ()
    cosh$ c @; "insert into test values (?, ?)" db.prep; (1 2) db.exec
    ()
    cosh$ c @; "select * from test" db.prep; () db.exec
    (
        0: h(
            "id":  1
            "num": 2
        )
    )
    cosh$

### Documentation

[Documentation](./doc/all.md)

### Licence

See [LICENCE](./LICENCE).
