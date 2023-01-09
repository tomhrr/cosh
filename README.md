## cosh

[![Build Status](https://github.com/tomhrr/cosh/workflows/build/badge.svg?branch=main)](https://github.com/tomhrr/cosh/actions)
[![stability-beta](https://img.shields.io/badge/stability-beta-33bbff.svg)](https://github.com/mkenney/software-guides/blob/master/STABILITY-BADGES.md#beta)

cosh is a concatenative command-line shell.

### Why?

Basic shell operations like `ls`, `ps`, `stat`, and so on are
implemented as functions that return first-class values, as opposed to
relying on executables that return text streams.  This makes working
with the results simpler.  For example, to find the size of all files
in the current directory:

<table>
    <tr>
        <th><b>sh</b></th>
        <td><code>ls | xargs stat -c %s | awk '{s+=$1} END {print s}' -</code></td>
    </tr>
    <tr>
        <th><b>cosh</b></th>
        <td><code>ls; [stat; size get] map; sum</code></td>
    </tr>
</table>

A small set of versatile primitives means that less needs to be
remembered when compared with typical shells (see e.g. the various
flags for `cut(1)`).  For example, to get the second column from each
row of a CSV file:

<table>
    <tr>
        <th><b>sh</b></th>
        <td><code>cat test-data/csv | cut -d, -f2</code></td>
    </tr>
    <tr>
        <th><b>cosh</b></th>
        <td><code>test-data/csv f<; [chomp; , split; 1 nth] map;</code></td>
    </tr>
</table>

Arithmetical operators and XML/JSON/CSV encoding/decoding functions
reduce the number of times that it becomes necessary to use a more
full-featured programming language or a third-party executable.  For
example, to get the first value from the "zxcv" array member of a JSON
file:

<table>
    <tr>
        <th><b>sh</b></th>
        <td><code>cat test-data/json2 | jq .zxcv[0]</code></td>
    </tr>
    <tr>
        <th><b>cosh</b></th>
        <td><code>test-data/json2 f<; from-json; zxcv get; 0 nth</code></td>
    </tr>
</table>

### Install

#### Dependencies

 - [Rust](https://github.com/rust-lang/rust)

#### Building

    make
    make test
    sudo make install

#### Running

    user@host:/$ cosh
    /$ hello println;
    hello

### Documentation

[As single page](./doc/all.md)

### Licence

See [LICENCE](./LICENCE).
