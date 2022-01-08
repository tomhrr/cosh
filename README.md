## cosh

[![stability-beta](https://img.shields.io/badge/stability-beta-33bbff.svg)](https://github.com/mkenney/software-guides/blob/master/STABILITY-BADGES.md#beta)

cosh is a concatenative command-line shell.

### Why?

 - The use of real values, rather than text streams, avoids the need
   to remember considerations like null-termination of strings (per
   `find ... -print0` and `xargs ... -0`).
 - A smaller set of more versatile primitives means that less needs to
   be remembered, as compared with the various flags for `cut(1)`,
   `sort(1)`, and so on.
 - Arithmetical operators and XML/JSON/CSV encoding/decoding reduce
   the number of times that it becomes necessary to use a 'real'
   programming language.

### Install

    make
    make test
    sudo make install

### Documentation

[As single page](./doc/all.md)

### Examples

```
$ 1 2 +
3
$ 10 3 -
7
$ 1 2 swap
2 1
$ : add-1 1 +; ::
$ 10 add-1
11
$ "1,2,3,4" , split
(
    1
    2
    3
    4
)
$ "1,2,3,4" , split; add-1 map
(
    2
    3
    4
    5
)
$ "1,2,3,4" , split; , join
1,2,3,4
$ test-data/csv f<;
(
    "1,2,3,4\n"
    "5,6,7,8\n"
    "9,10,11,12\n"
)
$ test-data/csv f<; chomp map; [, split] map
(
    (
        1
        2
        3
        4
    )
    (
        5
        6
        7
        8
    )
    (
        9
        10
        11
        12
    )
)
$ : sum 0 + reduce; ::
$ test-data/csv f<; chomp map; [, split] map; sum map
(
    10
    26
    42
)
$ test-data/csv f<; chomp map; [, split] map; sum map; [\n append] map; new-data swap; f>
$ test-data ls
(
    test-data/readfile
    test-data/csv
    test-data/split
)
$ test-data ls; [dup; readfile m; swap; csv m; or] grep;
(
    test-data/readfile
    test-data/csv
)
$ test-data ls; [dup; println; stat; size at; println] for;
test-data/readfile
10
test-data/csv
27
test-data/split
46
$ {ls test-data}; {sort -r} |;
(
    "split\n"
    "readfile\n"
    "csv\n"
)
```

### Licence

See [LICENCE](./LICENCE).
