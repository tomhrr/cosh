## cosh

[![Build Status](https://github.com/tomhrr/cosh/workflows/build/badge.svg?branch=main)](https://github.com/tomhrr/cosh/actions)
[![stability-beta](https://img.shields.io/badge/stability-beta-33bbff.svg)](https://github.com/mkenney/software-guides/blob/master/STABILITY-BADGES.md#beta)

cosh is a concatenative command-line shell.

### Why?

 - Basic shell operations like `ls`, `ps`, and so on are implemented
   as functions that return first-class values, as opposed to relying
   on executables that return text streams.  This makes working with
   the results simpler:

```
/test$ ls;
(
    0: ./file-0
    1: ./file-1
    2: ./file-2
)
/test$ ls; shift;
file-0
/test$ ls; shift; stat; size get
1024
```

 - A small set of versatile primitives means that less needs to be
   remembered when compared with typical shells (see e.g. the various
   flags for `cut(1)`):

```
/test$ file-0 f<;
v[gen (
    0: "1,2,3\n"
    1: "4,5,6\n"
)]
/test$ file-0 f<; [chomp; , split] map;
v[gen (
    0: (
        0: 1
        1: 2
        2: 3
    )
    1: (
        0: 4
        1: 5
        2: 6
    )
)]
/test$ file-0 f<; [chomp; , split] map; [2 nth] map; sum
9
```

 - Arithmetical operators and XML/JSON/CSV encoding/decoding
   functions reduce the number of times that it becomes
   necessary to use a more full-featured programming language:

```
/test$ file-1 f<; print for;
{"asdf":1,"qwer":2,"zxcv":3}
/test$ file-1 f<; from-json;
h(
    "asdf": 1
    "qwer": 2
    "zxcv": 3
)
/test$ file-1 f<; from-json; each; [dup; pop; 3 *; push; , join; \n ++] map;
v[gen (
    0: "asdf,3\n"
    1: "qwer,6\n"
    2: "zxcv,9\n"
)]
/test$ file-1 f<; from-json; each; [dup; pop; 3 *; push; , join; \n ++] map; file-1-csv f>;
/test$
```

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
