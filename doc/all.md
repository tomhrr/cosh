## cosh

cosh is a concatenative command-line shell.

 * [Usage](#usage)
 * [Core](#core)
    * [Types](#types)
    * [Functions](#functions)
    * [Variables](#variables)
    * [Conditionals](#conditionals)
    * [Loops](#loops)
    * [Scoping](#scoping)
    * [Anonymous functions](#anonymous-functions)
    * [Generators](#generators)
 * [Built-in functions](#built-in-functions)
    * [Stack functions](#stack-functions)
    * [Boolean functions](#boolean-functions)
    * [Arithmetic and relations](#arithmetic-and-relations)
    * [String functions](#string-functions)
       * [Regular expressions](#regular-expressions)
    * [List functions](#list-functions)
    * [Set functions](#set-functions)
    * [Hash functions](#hash-functions)
    * [Higher-order functions (map, grep, for, etc.)](#higher-order-functions-map-grep-for-etc)
    * [Parallel processing](#parallel-processing)
    * [Sorting](#sorting)
    * [Filesystem operations](#filesystem-operations)
    * [Environment variables](#environment-variables)
    * [JSON/XML/YAML Parsing](#jsonxmlyaml-parsing)
    * [Datetimes](#datetimes)
    * [IP addresses](#ip-addresses)
    * [Networking](#networking)
    * [HTTP](#http)
    * [SQL databases](#sql-databases)
    * [Miscellaneous functions](#miscellaneous-functions)
 * [External program execution](#external-program-execution)
    * [Default command aliases](#default-command-aliases)
 * [Libraries](#libraries)
    * [rdap](#rdap)
    * [rpkic](#rpkic)
    * [rpsl](#rpsl)
    * [ssh-agent](#ssh-agent)
 * [Miscellaneous](#miscellaneous)
    * [Caveats and pitfalls](#caveats-and-pitfalls)
    * [Development](#development)

### Usage

The `cosh` executable will start an interactive shell when it is run
without arguments:

    user@host:/$ cosh
    /$

(The remaining examples will omit the directory from the beginning of
the prompt.)

Ctrl-R can be used to search through history, which is written to the
`.cosh_history` file in the user's home directory.  Double-tabbing
shows autocomplete options: red strings are built-in functions, blue
strings are user-defined functions, and black strings are filenames.
Ctrl-D can be used to exit the shell.

To compile a library:

    user@host:/$ cat test.ch
    : test-fn hello println; ,,
    user@host:/$ cosh -c test.ch -o test.chc
    user@host:/$

To import and use a library:

    $ test.chc import
    $ test-fn
    hello
    $

Libraries are searched for first within the current directory,
followed by the compile-time `libdir` directory (which defaults to the
`lib` directory within the install prefix) with `/cosh` appended to
it.  The unqualified filename is checked first, followed by
`{filename}.chc`, then `{filename}.ch`.

To run a script:

    user@host:/$ cat script.ch
    1 1 +;
    user@host:/$ cosh script.ch
    2
    user@host:/$ cat script2.ch
    #!/usr/local/bin/cosh
    1 1 +;
    user@host:/$ ./script2.ch
    2

Scripts may accept command-line arguments:

    user@host:/$ cat script.ch
    +;
    user@host:/$ cosh script.ch 1 1
    2

To source a script (run the script within the existing context):

    user@host:/$ cat test.ch
    1 +;
    user@host:/$ cosh
    $ 1 test.ch source;
    2

### Core

#### Types

The shell language is dynamically-typed.  The basic primitive types
are:

  * `bool`: boolean value
  * `byte`: unsigned 8-bit integer
  * `int`: signed 32-bit integer
  * `bigint`: arbitrary-precision integer
  * `float`: double-width floating-point number
  * `string`: a string encoded as UTF-8

The basic composite types are:

  * `list`: a list of values
  * `set`: a set of values
  * `hash`: a hash map of values

There is also a `Null` type, with an associated null value.

Interpretation is like so:

  * The tokens `.t` and `.f` are interpreted as true and false boolean
    values respectively.
  * The token `null` is interpreted as the null value.
  * A number with a fractional component is interpreted as a `float`.
  * A number without a fractional component is interpreted as an
    `int`, if it fits within a 32-bit signed integer, and as a
    `bigint` otherwise.
  * A series of tokens enclosed within parentheses (`(...)`) is
    interpreted as a `list`.
  * A series of tokens enclosed within parentheses and preceded by an
    `s` character (`s(...)`) is interpreted as a `set`.
  * A series of tokens enclosed within parentheses and preceded by an
    `h` character (`h(...)`) is interpreted as a `hash`.
  * All other tokens are interpreted as `string`s.  (To construct a
    string that contains whitespace, use double-quote characters to
    delimit the string.)

The forms `bool`, `byte`, `str`, `int`, `bigint`, and `float` can be
used to convert primitive values of one type to another type.  If the
conversion is not supported, then the null value will be returned.
`int` will convert a value to a `bigint` if required.  `str` can also
be used to convert a list of bytes into a string.

There are type predicates for each of the basic types, as well as the
null value:

  * `is-bool`
  * `is-byte`
  * `is-int`
  * `is-bigint`
  * `is-float`
  * `is-str`
  * `is-list`
  * `is-set`
  * `is-hash`
  * `is-null`

`is-callable` returns a boolean indicating whether the argument can be
called like a function.

The primitive types have value semantics, whereas the composite types
have reference semantics.  Memory is handled via reference counting.

#### Functions

All `string`s are quoted by default.  The semicolon character will
cause the definition associated with the last string on the stack to be
executed:

    $ 1 2 + ;
    3

A semicolon may be appended to the `string`:

    $ 1 2 +;
    3

A newline acts as an implicit semicolon, when the last `string` on the
stack has an associated definition:

    $ 1 2 + ; 1 2 + ; +
    6

(When running interactively, trailing whitespace is also ignored for
the purposes of implicit execution.)

Functions are defined like so:

    $ : add-1 1 + ; ,,
    $ 1 add-1
    2

#### Variables

Variable definition and load/store operations are like so:

    $ x var;
    $ 0 x !;
    $ x @
    0
    $ x @; 1 +; x !; x @
    1
    $ x @; 1 +; x !; x @
    2

At the top level, variables can also be defined using the form `varm`
('var multiple').  With `var`, only one definition operation for a
given name may appear at a given scope, whereas with `varm`,
redefinition is permitted.  This is to facilitate the interactive use
of variables, by avoiding the need for a user to define a variable in
one command and then to make use of that variable in another command
(or commands).

#### Conditionals

Conditional execution is handled by `if`.  False boolean values, zero
numeric values and the strings "", "0", and "0.0" evaluate to false,
while all other values evaluate to true.

    $ 100 if; 1 else; 2 then;
    1
    $ "0" if; 1 else; 2 then;
    2

#### Loops

Loops can be constructed using `begin` and `until` (`.s` prints the
stack to standard output):

    $ x var; 0 x !;
    $ begin; x @; 1 +; x !; x @; .s; 4 >; until
    1
    2
    3
    4
    5

`leave` can be used to exit a loop early.

#### Scoping

Scoping is lexical.  Variable definitions within functions may shadow
global definitions:

    $ x var; 10 x !; x @;
    10
    $ : mf1 x var; 20 x !; x @; .s; drop; ,, mf1 ;
    20
    $ x @;
    10
    $ : mf2 20 x !; x @; .s; drop; ,, mf2 ;
    20
    $ x @;
    20

Function definitions may be nested.  Nested functions have access to
the surrounding environment when they are executed, but they do not
close over that environment.

#### Anonymous functions

An anonymous function is defined by way of a list that contains the
function's tokens.  It is executed using `funcall`:

    $ [1 2 +] funcall ;
    3

The last token in the list is treated as a function implicitly, if it is
not followed by a semicolon.

Anonymous functions close over their environment.  However, changes to
value types within the scope of the anonymous function will only
persist for calls to that anonymous function (i.e. variables with
value types are effectively copied into the state of the anonymous
function).  Changes to composite and other reference types will affect
both the anonymous function state and the original state, though.

#### Generators

A function may be defined as a generator function.  When such a
function is called, it returns a generator object, which can be
iterated by way of `shift`.  On iteration, function execution
continues until a `yield` statement is reached, at which point control
returns to the caller.  When the generator object is next iterated,
execution resumes from the point after the `yield` statement.  For
example:

    $ :~ gen 0 0 drop; n var; 0 n !; begin; n @; yield; n @; 1 +; n !; n @; 3 >; until; ,,
    $ : iter dup; shift; println; ,,
    $ gen; iter; iter; iter; iter; drop;
    0
    1
    2
    3

The first two arguments after the generator name are the maximum
argument count and the required argument count for the generator:
these arguments are collected and stored when the generator is
instantiated (where `gen;` appears in the above).  When the generator
is first iterated, the number of arguments that have been passed in is
provided as the top value on the stack.

Generators close over their environment, so it is possible e.g. to
have a function which defines local functions/variables, followed by a
generator making use of those, with the function returning an instance
of that generator when called.

`take` can be used to return a certain number of elements from a
generator object as a list:

    $ gen; 2 take;
    (
        0: 0
        1: 1
    )

(When lists are printed in the shell, each entry in the list includes
the list index.)

`take-all` can be used to return all the elements from a generator
object:

    $ gen; take-all;
    (
        0: 0
        1: 1
        2: 2
        3: 3
    )

By default, each generator object that remains on the stack after
command execution is finished will be iterated such that its results
are displayed:

    $ gen;
    v[gen(
        0: 0
        1: 1
        2: 2
        3: 3
    )]

The output in this case is slightly different from previously, because
`take` and `take-all` convert the generator into a list, whereas in
the above the generator is being printed as-is, which is why it is
wrapped in `v[gen(...)]`.  In general, built-in composite types other
than lists, sets, and hashes will be displayed using this syntax.

`shift`, `take`, and `take-all` also work in the same way on lists and
sets.  In general, any built-in form that works on a list will also
work on a generator, and if it operates as a transformation, then its
result will also be a generator.  `is-shiftable` is an additional type
predicate that returns a boolean indicating whether `shift` can be
called on the argument.

There is also a general reification function named `r`, which will
convert any generators in the argument value into lists, recursively,
while leaving other values unchanged.

#### Error handling

Whenever an error occurs, an error message is displayed and control is
returned to the user at the shell.  There are no facilities for
catching errors or resuming processing at the point where an error
occurred.  (This only affects internal calls, though: if a call to an
external program fails, that will not of itself cause control to be
returned to the user.)

To cause an error to occur manually, use the `error` form:

    $ "an error message" error
    1:20: an error message

The `lib/rt.ch` library contains various example uses of this
function.

### Built-in functions

#### Boolean functions

 - `and`: the conjunction function, taking two values and returning a
   boolean indicating whether both values evaluate to true.
 - `or`: the disjunction function, taking two values and returning a
   boolean indicating whether at least one value evaluates to true.
 - `not`: the negation function, taking a value and returning a
   boolean indicating whether that value evaluates to false.

Both `and` and `or` evaluate each of their expressions.  `if` can be
used to avoid this behaviour, if necessary.

#### Arithmetic and relations

`+`, `-`, `*`, `/`, `=`, `<`, and `>` are defined over the numeric
types.  `=`, `<`, and `>` are also defined over `string`s.

`<=>` returns -1 if the first argument is less than the second
argument, 0 if the two arguments are equal, and 1 if the first
argument is greater than the second argument.  It is defined over the
numeric types, as well as `string`s.

`sqrt` and `abs` are defined over the numeric types  `**`
(exponentation) is defined over the numeric types for the base, and
over `int` and `float` for the exponent.

`<<` (logical left shift) and `>>` (logical right shift) are defined
over the integral types for the operand and `int`s for the number of
bit positions.

`&` (bitwise and), `||` (bitwise or), `^` (bitwise xor) and `%`
(remainder) are defined over the integral types.

#### Stack functions

Some of the more commonly-used stack functions from Forth are defined:

    $ 1 clear
    $ 1 dup
    1
    1
    $ 1 2 swap
    2
    1
    $ 1 2 drop
    1
    $ 1 2 nip
    2
    $ 1 2 3 rot
    2
    3
    1
    $ 1 2 over
    1
    2
    1
    $ 1 1 depth
    1
    1
    2

#### String functions

`++` appends one string to another:

    $ asdf qwer ++
    "asdfqwer"

`++` also works for lists and hashes, as well as generators.

`chomp` removes the final newline from the end of a string, if the
string ends in a newline:

    $ "asdf\n" chomp
    "asdf"
    $ "asdf" chomp
    "asdf"

`chr` takes an integer or a bigint and returns the character
associated with that input.  `ord` takes a character and returns the
integer or bigint associated with that character.

`hex` takes an integer or bigint and returns the number as a
hexadecimal string, with `unhex` performing the reverse operation.
`oct` and `unoct` work in the same way, but for octal strings.  `hex`
also supports conversion of a byte list into a hexadecimal string.

`lc` takes a string, converts all characters to lowercase, and returns
the updated string.  `lcfirst` takes a string, converts the first
character to lowercase, and returns the updated string.  `uc` and
`ucfirst` operate similarly, except they convert to uppercase.

`reverse` reverses a string.  It also works on lists.

##### Regular expressions

The two basic regular expression forms are `m` and `c`.  The `m` form
returns a boolean indicating whether a string matches against a
regular expression.  The `c` form is similar, except that it results
in a list containing the captures from the expression (if any):

    $ asdf asdf m
    .t
    $ asdf asdf2 m
    .f
    $ asdf ".." c
    (
        0: as
    )
    $ asdf ".(.)" c
    (
        0: as
        1: s
    )

The `s` form handles search and replace:

    $ asdf as qw s;
    qwdf

For the supported syntax, see the Rust
[https://docs.rs/regex/1.3.9/regex/index.html#syntax](regex) crate.
It is close to that of PCRE, except that lookahead and backreferences
are not supported.

The following flags are supported:

 - `i`: case-insensitive matching;
 - `g`: global matching;
 - `m`: multi-line matching (`^` and `$` match against the lines
   within the string); and
 - `s`: single-line matching (`.` matches against any character,
   including newline).

Flags are attached to regular expressions like so:

    $ AsDf asdf/i m
    .t
    $ AsAs as/ig DF s
    DFDF

Because flags are separated from the regular expression by a forward
slash, other forward slash characters that appear within the
expression must be escaped with a backslash.

If the global flag is attached to a capturing regular expression, then
the result is a list of lists:

    $ asdf "../g" c
    (
        0: (
            0: as
        )
        1: (
            0: df
        )
    )
    $ asdf ".(.)/g" c
    (
        0: (
            0: as
            1: s
        )
        1: (
            0: df
            1: f
        )
    )

#### List functions

When called with a list argument, `shift` removes one element from the
beginning of the list and places it on the stack:

    $ (1 2 3) shift;
    1

`shift-all` removes all elements from the list and places them onto
the stack:

    $ (1 2 3) shift-all;
    1
    2
    3

`unshift` takes a list and an element and places it at the beginning
of the list:

    $ (2 3 4) 1 unshift;
    (
        0: 1
        1: 2
        2: 3
        3: 4
    )

`pop` removes one element from the end of the list and places it on
the stack:

    $ (1 2 3) pop;
    3

`push` takes a list and an element and places it at the end of the
list:

    $ (1 2 3) 4 push;
    (
        0: 1
        1: 2
        2: 3
        3: 4
    )

`get` returns a specific element from a list:

    $ (1 2 3) 1 get
    2

`get` can also return multiple elements:

    $ (1 2 3 4) (0 2) get;
    (
        0: 1
        1: 3
    )

`set` updates a specific element in a list:

    $ (1 2 3 4) 2 10 set;
    (
        0: 1
        1: 2
        2: 10
        3: 4
    )

`split` splits a string based on a delimiter string:

    $ asdf,asdf , split
    (
        0: asdf
        1: asdf
    )

`join` joins a list of strings together using a delimiter string:

    $ asdf,asdf , split; , join
    asdf,asdf

Both `split` and `join` handle quoting of values that contain either
the delimiter, or a quotation mark.

`splitr` splits a string based on a delimiter regex.  It does not
handle quoting of values, though.  `joinr` joins a list of strings
together using a delimiter string, also without handling quoting of
values.

`len` returns the length of a string.  This function also works for
sets, hashes, strings, and generators.

`empty` returns a boolean indicating whether the length of the string
is zero.  This function also works for sets, hashes, strings, and
generators.  (In the case of a generator, it will exhaust the generator
even though that's not strictly necessary for determining whether the
generator is empty, because having it shift a single element from the
generator each time it is called could be confusing.)

`mlist` takes an integer argument, removes that number of elements
from the stack, and returns a list containing those elements.

`lr` takes a list, an initial index argument, a function, and a final
index argument, and takes the value at the initial index from the
list, calls the function on that value, and sets the result as the
value of the final index in the list.  The list is left on the stack
at the end of the call.

#### Set functions

When called with a set argument, `shift` removes one element from the
beginning of the set and places it on the stack:

    $ s(1 2 3) shift;
    1

`exists` checks whether a key is present in a set:

    $ s(1 2 3) 2 exists;
    .t

`get` retrieves an element by index:

    $ s(1 2 3) 2 get;
    3

`union` combines two sets:

    $ s(1 2 3) s(2 3 4) union;
    s(
        1
        2
        3
        4
    )

`isect` returns the intersection of two sets:

    $ s(1 2 3) s(2 3 4) isect;
    s(
        2
        3
    )

`diff` subtracts one set from another:

    $ s(1 2 3) s(2 3 4) diff;
    s(
        1
    )

`symdiff` returns the symmetric difference of two sets:

    $ s(1 2 3) s(2 3 4) symdiff;
    s(
        1
        4
    )

`mset` takes an integer argument, removes that number of elements
from the stack, and returns a set containing those elements.

#### Hash functions

`get` returns a value from a hash:

    $ h(a 1 b 2) b get;
    1

`get` can also return multiple elements:

    $ h(a 1 b 2) (a b) get;
    (
        0: 1
        1: 2
    )

and chained accessors can be used as well:

    $ h(a 1 b (0 1 h(c 5 e 7))) b.2.e get;
    7

`set` is used to update a value in a hash:

    $ h(a 1 b 2) c 3 set; c get;
    3

`delete` removes a key-value pair from a hash:

    $ h(a 1 b 2) dup; a delete;
    h(
        "b": 2
    )

`exists` checks whether a key is present in a hash:

    $ h(a 1 b 2) a exists;
    .t

`keys` returns a generator over the hash's keys:

    $ h(a 1 b 2) c 3 set; keys;
    v[keys-gen (
        0: b
        1: a
        2: c
    )]

`values` returns a generator over the hash's values:

    $ h(a 1 b 2) c 3 set; values;
    v[values-gen (
        0: 2
        1: 1
        2: 3
    )]

`each` returns a generator over the key-value pairs from the hash:

    $ h(a 1 b 2) c 3 set; each; take-all;
    v[each-gen (
        0: (
            0: b
            1: 2
        )
        1: (
            0: a
            1: 1
        )
        2: (
            0: c
            1: 3
        )
    )]

(There are separate generator types for the generators created by way
of `keys`, `values`, and `each`, but their behaviour is as per
the previous generator discussion in this document.)

`mhash` takes an integer argument, removes that number of element
pairs from the stack, and returns a hash containing each of those
pairs, where the first element is the key and the second is the value.

`hr` takes a hash, an initial index argument, a function, and a final
index argument, and takes the value at the initial index from the
hash, calls the function on that value, and sets the result as the
value of the final index in the hash.  The hash is left on the stack
at the end of the call.

#### Higher-order functions (map, grep, for, etc.)

`map` iterates over a list, applying a function to each
element and collecting the results into a new generator:

    $ : add-1 1 + ; ,,
    $ (1 2 3 4) add-1 map
    v[gen (
        0: 2
        1: 3
        2: 4
        3: 5
    )]

`grep` iterates over a list, applying a predicate to each element and
collecting the values for which the predicate is true into a new
generator:

    $ : <4 4 < ; ,,
    $ (1 2 3 4) add-1 map; <4 grep;
    v[gen (
        0: 2
        1: 3
    )]

`for` is the same as map, except that it does not collect the results
into a new generator (i.e. the function is executed only for its
side effects).

`foldl` takes a list, a seed, and a function, applies the function to
the seed and the first element from the list to produce a value, and
then continues applying the function to the resulting value and the
next element from the list until the list is exhausted:

    $ (1 2 3) 0 + foldl
    6

Each of the above functions can accept a generator or a set as an
argument, instead of a list.

Anonymous functions can be used inline in these calls:

    $ (1 2 3 4) [1 +] map
    (
        0: 2
        1: 3
        2: 4
        3: 5
    )

Other higher-order functions:

 - `any`: takes a list and a function, and returns a boolean
   indicating whether the function returns true for any element of the
   list.
 - `all`: takes a list and a function, and returns a boolean indicating
   whether the function returns true for all of the elements of the
   list.
 - `none`: like `all`, except it returns a boolean indicating
   whether the function returns false for all of the elements of the
   list.
 - `notall`: like `any`, except it returns a boolean indicating
   whether the function returns false for any of the elements of the
   list.
 - `first`: takes a list and a function, and returns the first element
   for which the function returns true.
 - `uniq`: takes a list, and returns a generator over the unique
   elements from that list (uniqueness is determined by converting
   each value to a string and comparing the strings).
 - `min`: takes a list and returns the smallest element of that list.
 - `max`: takes a list and returns the largest element of that list.
 - `shuffle`: takes a list and moves each element to a random location
   in the list.
 - `product`: multiplies all of the elements of the list together and
   returns the result.
 - `pairwise`: takes two lists and a function, and on each iteration,
   shifts one element from each of the lists and calls the function on
   those elements.  The result is a generator over the results from
   the function calls.
 - `slide`: takes a list and a function, and calls the function for
   sliding pairs from the list.  For example, the first call is for
   elements 0 and 1, the next call is for elements 1 and 2, and so on.
   The result is a generator over the results from the function calls.
 - `before`: takes a list and a function, and calls the function on
   each element from the list, returning elements up until the
   function call returns a value that evaluates to true, at which
   point it returns no more elements.
 - `after`: works similarly to `before`, save that it returns the
   elements from after the point where the function returns a true
   value.
 - `apply`: like `map`, but it works on the stack, rather than on a
   list.  Takes a function and the number of stack elements to which
   the function should be applied.

Each of the above, except for `apply`, can also accept a set or
generator in place of a list argument.

#### Parallel processing

`pmap` operates similarly to `map`, except that it distributes the
work across four processes, and the resulting generator returns
results as they are received from those processes (i.e. in
indeterminate order):

    $ 5 range; [1 rand; sleep] pmap;
    v[channel-gen (
        0: 2
        1: 0
        2: 1
        3: 3
        4: 4
    )]

State changes in the forked processes (e.g. new function definitions,
variable modifications) will not take effect in the original process.

`pmapn` operates in the same way as `pmap`, except that it takes an
additional process count argument:

    $ : dt date; to-epoch; ,,
    $ dt; 10 range; [1 sleep] 10 pmapn; r; drop; dt; swap; -
    1
    $ dt; 10 range; [1 sleep] 2  pmapn; r; drop; dt; swap; -
    5

`pgrep`, `pgrepn`, `pfor`, and `pforn` work similarly for the `grep`
and `for` functions.

#### Sorting

`sort` sorts a list or generator, where the values in the list are of
primitive types:

    $ (1 3 5 4 2 1) sort
    (
        0: 1
        1: 1
        2: 2
        3: 3
        4: 4
        5: 5
    )

`sortp` accepts an additional predicate argument, being a function
that operates like `<=>` (i.e. -1 for less-than, 0 for equal, 1 for
greater-than):

    $ (1 3 5 4 2 1) [<=>; -1 *] sortp
    (
        0: 5
        1: 4
        2: 3
        3: 2
        4: 1
        5: 1
    )

#### Filesystem operations

`ls` takes a directory name as its argument and returns a generator
object over the files in that directory:

    $ . ls
    v[gen (
        0: ./Cargo.toml
        ...
    )]

`lsr` does the same thing, but includes all files within nested
directories as well.  If the stack is empty when either of these
functions is called, then they will act as if they were called on the
current working directory.

`lsh` and `lshr` operate in the same way as `ls` and `lsr`, except
that hidden files/directories are included in the output.

`f<` takes a filename as its argument and returns a generator over the
lines in that file:

    $ README.md f<; 2 take;
    (
        0: "## cosh\n"
        1: "\n"
    )

`f>` takes a list of strings (or a single string) and a filename as
its arguments and writes the strings (or string) to that file:

    $ ("asdf\n" "qwer\n" "zxcv\n") asdf f>;
    $ asdf f<;
    v[gen (
        0: "asdf\n"
        1: "qwer\n"
        2: "zxcv\n"
    )]

`b<` and `b>` operate in the same way as `f<` and `f>`, except that
they produce and consume lists where each element of the list is a
list of bytes.  This make them suitable for handling binary data.

Other operations:

 - `cd`: changes the current working directory.
 - `cwd`: returns the current working directory.
 - `is-dir`: returns a boolean indicating whether the argument is a
   directory.
 - `is-file`: returns a boolean indicating whether the argument is a
   plain file.
 - `is-link`: returns a boolean indicating whether the argument is a
   link.
 - `rm`: removes the argument file.
 - `rmf`: removes the argument file, without reporting an error in the
   event that the file doesn't exist.
 - `rmrf`: removes the argument path, including its contents if it's a
   directory, without reporting an error in the event that the path
   doesn't exist.
 - `touch`: if the argument file doesn't exist, creates an empty file
   with the given name, otherwise updates the modification time of the
   existing file to be the current time.
 - `cp`: copies the file/directory at the first path to the second
   path.  If the second path is a directory, the source is copied into
   that directory.
 - `mv`: moves the file/directory at the first path to the second
   path.  If the second path is a directory, the source is moved into
   that directory.
 - `rename`: rename the file at the first path such that its path is
   the second path.
 - `stat`: returns a hash containing metadata about the argument file.
 - `lstat`: like stat, but if the argument is a symbolic link, returns
   metadata about the link itself, instead of its target.
 - `readlink`: takes a symbolic link path, and returns the link
   target.
 - `ps`: returns a list containing details on the currently-running
   processes, where each process has a separate hash containing the
   PID, UID, username, GID, process name, full command, memory usage
   (in bytes), virtual memory usage (in bytes), current CPU usage (as
   a percentage of the total number of available CPUs), the time the
   process was started, the number of seconds for which the process
   has been running, and the process's status.
 - `pss`: takes a PID and returns the associated process details,
   using the same hash structure as in `ps`.
 - `pse`: takes a PID and returns a boolean indicating whether the
   specified process exists.
 - `kill`: takes a PID and a signal name ("hup", "int", "term",
   "kill", "usr1", "usr2", "cont", or "stop"), and sends the specified
   signal to the process.
 - `chmod`: takes a path and a numeric mode, and updates the path's
   mode accordingly.  (`oct` may be useful for mode conversions.)
 - `chown`: takes a path, a user name, and a group name, and updates
   the path's ownership accordingly.
 - `mkdir`: takes a path and creates a directory at that path.
 - `rmdir`: takes a path and removes the directory at that path
   (directory must be empty).
 - `link`: takes two paths, and creates a symbolic link at the second
   path that targets the first path.
 - `tempfile`: returns a file writer and a path string for a new
   temporary file.  This file is not cleaned up automatically on
   program exit or similar.
 - `tempdir`: returns a path string for a new temporary directory.
   This directory is not cleaned up automatically on program exit or
   similar.
 - `opendir`: takes a directory path, and put a directory handle
   object onto the stack.
 - `readdir`: reads the next entry for a directory handle object.
 - `no-upwards`: takes a directory name as its argument and returns a
   boolean indicating whether that name is not either "." or "..".
 - `basename`: takes a file path and returns the file's name.
 - `dirname`: takes a file path and returns the file's directory.
 - `is-r`: takes a file path and returns a boolean indicating whether
   the file is readable by the current user.
 - `is-w`: takes a file path and returns a boolean indicating whether
   the file is writable by the current user.
 - `is-x`: takes a file path and returns a boolean indicating whether
   the file is executable by the current user.

Core input/output operations:

 - `print`: takes a value and prints it to standard output.
 - `println`: takes a value and prints it to standard output, followed
   by a newline.
 - `open`: takes a file path and a mode string (either 'r' or 'w'),
   and puts a file reader or a file writer object onto the stack.
 - `readline`: read a line from a file reader object.
 - `writeline`: write a line to a file writer object.
 - `close`: close a file reader or file writer object.

#### Environment variables

`env` returns a hash containing the current set of environment
variables.

`getenv` takes an environment variable name and returns the value for
that variable, or null if the variable does not exist.

`setenv` takes an environment variable name and a value, and set that
environment variable as having that value.

#### JSON/XML/YAML Parsing

JSON, XML, and YAML can be serialised and deserialised using the
`from-json`, `to-json`, `from-xml`, `to-xml`, `from-yaml`, and
`to-yaml` functions.

#### Datetimes

 - `now`: returns the current time as a DateTime object, offset at
   UTC.
 - `date`: returns the current time as a DateTime object, offset at
   the local time zone.
 - `from-epoch`: takes the epoch time (i.e. the number of seconds that
   have elapsed since 1970-01-01 00:00:00 UTC) and returns a DateTime
   object (offset at UTC) that corresponds to that time.
 - `to-epoch`: takes a DateTime object and returns the epoch time that
   corresponds to that object.
 - `set-tz`: takes a DateTime object and a named timezone (per the tz
   database) and returns a new DateTime object offset at that
   timezone.
 - `+time`: takes a DateTime object, a period (one of years, months,
   days, minutes, hours, or seconds) and a count as its arguments.
   Adds the specified number of periods to the DateTime object and
   returns the result as a new DateTime object.
 - `-time`: the reverse of `+time`.
 - `strftime`: takes a DateTime object and a strftime pattern as its
   arguments.  Returns the stringification of the date per the
   pattern.
 - `strptime`: takes a datetime string and a strftime pattern as its
   arguments.  Returns the parsed datetime string as a DateTime
   object.
 - `strptimez`: takes a datetime string, a strftime pattern, and a
   named timezone (per the tz database) as its arguments.  Returns the
   parsed datetime string as a DateTime object.

The `strptime` and `strptimez` functions do not require that any
particular specifiers be used in the pattern.  By default, the
DateTime object result is `1970-01-01 00:00:00 +0000`, with values
parsed by way of the pattern being applied on top of that initial
result.

#### IP addresses

 - `ip`: takes a single IP address or range as a string, and returns
   an IP object for that address or range.
 - `ip.from-int`: takes an IP address as an integer and an IP version
   (either 4 or 6) and returns an IP object for the address.
 - `ip.len`: takes an IP object and returns the prefix length of the
   range.
 - `ip.addr`: takes an IP object and returns the first address from
   the range as a string.
 - `ip.addr-int`: takes an IP object and returns the first address
   from the range as an integer.
 - `ip.last-addr`: takes an IP object and returns the last address
   from the range as a string.
 - `ip.last-addr-int`: takes an IP object and returns the last address
   from the range as an integer.
 - `ip.size`: takes an IP object and returns the number of hosts it
   covers.
 - `ip.version`: takes an IP object and returns the version of that
   object (either 4 or 6).
 - `ip.prefixes`: takes an IP object and returns a list comprising the
   prefixes (as IP objects) that make up the object.  (The main use of
   this is for converting a range into a set of prefixes, if
   necessary.)

There is also a separate IP set object, for storing multiple IP
address ranges in a single type.  The `ips` function takes a single IP
address or range as a string or a list of IP address objects or IP
address/range strings as its single argument, and returns an IP set
object for those addresses/ranges.  This object supports all the same
functions as a standard set, but it will additionally simplify the set
after each call to the minimum set of prefixes required to cover the
address space in the set.  Finally, `=` is also defined for IP sets,
and `str` is defined for both IP objects and IP sets.

#### Networking

 - `ping`: takes a single IP address or hostname as a string, and
   returns a boolean indicating whether the host is able to respond to
   a ping within five seconds.
 - `pingn`: takes a single IP address or hostname, along with a ping
   count, and returns a generator over a set of ping results for the
   host.  Each ping result is a hash comprising the ICMP sequence
   number of the ping attempt, the TTL of the response packet, and the
   time it took to receive the response.  The generator will return
   results as they are received.

 - `dns`: takes a DNS name and a record type, and returns the DNS
   response for that query.  The result from the call is a hash,
   including separate entries for the header, question, answer,
   authority, and additional sections from the DNS response.
 - `dnsat`: takes a nameserver IP address, a DNS name, and a DNS
   record type.  Queries the nameserver and returns a response
   with the same format as `dns`.

 - `socket`: takes a server name/address and a port, and returns
   socket reader and socket writer objects.
 - `nc`: takes a string or a list of strings, a server name/address,
   and a port, and returns a generator over the response from the
   server.

 - `ifconfig`: returns a list containing the network interface details
   for the current host.  Each list entry is a hash comprising the
   name, hardware address, IP address and network, and flags for the
   interface.

 - `netstat`: returns a list containing network connection
   information.  Eac list entry is a hash comprising the port type
   (either "tcp" or "udp"), the local address and port, the remote
   address and port (for TCP), inode, UID, username, associated PIDs,
   and connection state (for TCP).

#### HTTP

 - `http.get`: takes a URL, makes a GET request to that URL, and
   returns the response body as a string (or a list of bytes, for
   binary data).  If the response has a JSON or XML content type, then
   the returned value will be a data structure per
   `from-json`/`from-xml`.
 - `http`: takes a single hash argument, containing entries for the
   `url`, `method`, `headers` (hash), and `body` for the request.
   Submits the request and returns the response body as a string or a
   list of bytes.  As with `http.get`, for JSON and XML content type
   responses, the returned value will be a data structure per
   `from-json`/`from-xml`.  If the request content type is a JSON or
   XML content type, then `to-json`/`to-xml` will be used to convert
   the `body` into a string for the request.
    - If the hash argument contains a `raw` entry with a true value,
      then the response becomes a hash with keys for `code`, `headers`
      (hash) and `body` (returned as-is).

#### SQL databases

 - `db.conn`: takes a username, a password, a database name, a
   database server hostname, and a database type (one of "mysql",
   "postgresql", or "sqlite"), and returns a database connection
   object.  (If "sqlite" is used, the database server hostname is the
   path to the database file, and the remaining parameters are
   omitted.)
 - `db.prep`: takes a database connection object and an SQL statement,
   and returns a database statement handle.
 - `db.exec`: takes a database statement handle and a list of
   parameters (possibly empty), and returns a generator containing a
   hash for each of the records retrieved from the database.

For PostgreSQL, not all field types are supported.  The issue for
tracking this problem is https://github.com/tomhrr/cosh/issues/123.

#### Miscellaneous functions

`rand` takes a floating-point value and returns a random value between
zero and that floating-point value (excluding the floating-point value
itself).

`sleep` takes a floating-point value and pauses execution for that
number of seconds.

`md5`, `sha1`, `sha256` and `sha512` each take a single string
argument and return the corresponding cryptographic hash for that
input as a list of bytes.  (See `hex` for conversion of that output
into a hexadecimal string.)

`range` takes an integer and returns a generator over the integers
from zero to that integer, minus one.

`avg` takes a generator and returns the average of the values in that
generator.

`to-function` takes a callable string (e.g. a function name) and
converts it into a function object.  Using `funcall` on the function
object will then be quicker than using it on the original string.

`id` is a no-op function.

`exit` takes an exit code as its single argument, and exits the
program/shell.

`.ss`. is similar to `.s`, except that it prints only the top element
from the stack.

When running interactively, `last` takes the previous stack (i.e. as
at the conclusion of the last line that was executed) and adds those
elements on to the current stack.  Any generators that were on the
previous stack will be converted into lists.

### External program execution

A command that begins with `$` will be treated as an external call:

    $ $ls
    bin     eg      LICENSE     ...

When using the shell interactively, a line that begins with a space
character will also be treated as an external call:

    $  ls
    bin     eg      LICENSE     ...

It's also possible to treat a string as an external call:

    $ ls exec
    bin     eg      LICENSE     ...

For `exec`, the exit status for the command will be placed on the
stack after completion.

A form wrapped in braces is handled similarly, except that the result
is a generator:

    $ {ls}; take-all;
    (
        0: "bin\n"
        ...
    )

As with `exec`, strings can be treated as external calls with
generator output:

    $ ls cmd; take-all;
    (
        "bin\n"
        ...
    )

For braces and `cmd`, the exit status can be got by calling `status`
with the resulting generator as its single argument.  `status` will
also kill any associated processes, if they are still running.

Values can be substituted into external program calls, either by
popping values from the stack, or by indexing into the stack (0 is the
element most recently pushed onto the stack):

    $ {ls}; [{stat -c "%s" {}}; shift; chomp] map;
    v[gen (
        0: 4096
        1: 4096
        ...
    )]

    $ 1 2 3 {dc -e "{0} {1} + {2} + p"}; shift; chomp
    1
    2
    3
    6

This behaviour is available for standard strings as well, by way of
the `fmtq` function.  There is also a `fmt` function that works
similarly, except that it doesn't surround values that contain
whitespace with quotation marks.

The output of a generator can also be piped to a command:

    $ {ls}; {sort -r} |; take-all;
    (
        0: "tests\n"
        1: "test-data\n"
        ...
    )

By default, the generator for a command will return the standard
output stream of the command.  Flags can be added to the command in
order to get the generator to return the standard error stream:

    $ {ls asdf}/e;
    v[command-gen (
        0: "ls: cannot access \'asdf\': No such file or directory\n"
    )]

or both combined:

    $ {ls Cargo.toml asdf}/oe;
    v[command-gen (
        0: "ls: cannot access \'asdf\': No such file or directory\n"
        1: "Cargo.toml\n"
    )]

or both combined, with a number indicating the stream for the line (1
for standard output, and 2 for standard error):

    $ {ls Cargo.toml asdf}/c;
    v[command-gen (
        0: (
            0: 2
            1: "ls: cannot access \'asdf\': No such file or directory\n"
        )
        1: (
            0: 1
            1: "Cargo.toml\n"
        )
    )

The `/b` flag can be used to produce a generator over lists of bytes
from standard output, in instances where binary data is being dealt
with.

Environment variables can also be set for commands, in the same way as
for a standard shell:

    $ {TZ=Europe/London date};
    v[command-gen (
        0: "Mon 26 Dec 2022 11:43:19 GMT\n"
    )]

External calls that do not produce generators support `&&` for
sequencing multiple commands, `>` for redirecting standard output, and
`2>` for redirecting standard error, in the same way as for a standard
shell.

The `jobs` form can be used to get information about background
processes.  When it is called, it will also check for jobs that are
complete, and remove them from the job list, as with a standard shell:

    $ {sleep 1}; m varm; m !; jobs
    (
        0: h(
            "pid":      924429
            "desc":     "sleep 1"
            "complete": .f
        )
    )
    $ 1 sleep
    $ jobs
    (
        0: h(
            "pid":      924429
            "desc":     "sleep 1"
            "complete": .t
        )
    )
    $ jobs
    ()
    $

Each [parallel processing](#parallel-processing) operation involves a
co-ordinator process that manages the remaining processes that
actually do the work.  That co-ordinator process will also be included
in the jobs list.

#### Default command aliases

Aliases are defined for the following commands by default:

 - `vim`
 - `nano`
 - `ssh`
 - `scp`
 - `git`
 - `docker`
 - `zathura`
 - `libreoffice`
 - `nano`

In each case, the alias takes 0-2 arguments and passes them to the
relevant executable as-is.  See
https://github.com/tomhrr/cosh/blob/main/lib/rt.ch#L809 for the
definitions.  There are some additional custom aliases:

 - `gr`: maps to "grep -ri {} .".

### Libraries

#### [rdap](rdap.md)

A library for working with Registration Data Access Protocol
([RDAP](https://about.rdap.org)) queries.

#### [rpkic](rpkic.md)

A library that acts as a wrapper around
[rpki-client](https://www.rpki-client.org).

#### [rpsl](rpsl.md)

A library for working with Routing Policy Specification Language
(RPSL) objects.

#### [ssh-agent](ssh-agent.md)

Utility functions for initialising `ssh-agent`.

### Miscellaneous

Comments can be added by prefixing the comment line with `#`.

The `clone` form can be used to get a deep copy of certain types of
values: lists, hashes, sets, and the generators returned by the
`keys`, `values`, and `each` calls.  For all other value types,
`clone` has the same effect as `dup`: this is fine in most cases, but
it's important to be aware that `shift` on a shallow copy of a
generator affects the original generator value as well.

`@@` works in the same way as `@`, except that it also `clone`s the
variable's value.  In conjunction with reification (via `r`), this can
be useful when there's a need to generate, store, and refer to a value
repeatedly, and generation of that value is time-consuming.

On starting the shell for interactive use or when running a script,
the `cosh.conf` file in the current user's configuration directory
(typically `~/.config`) will be run.  The `--no-cosh-conf` option can
be used to skip loading that file.

When running interactively, the `history` function will return a
generator over the shell's history.  When not running interactively,
the `history` function will return an empty generator.

A `sudo` form is available when running interactively.  When that form
appears as the final form in the input, then a separate instance of
`cosh` will be run using `sudo`, with the remainder of the input being
passed in as the command to that instance.  (This does mean that
variables and related state are not available in the separate
instance, and state changes that happen in the separate instance do
not register in the original instance.)

A `time` form is available when running interactively.  When that form
appears as the final form in the input, then real and CPU time will be
tracked for the input, and printed to standard error once processing
is complete.

By default, the shell starts in 'transient' mode, unless it is being
used to run a script, in which case it starts in 'persistent' mode.
In 'transient' mode, after each command is submitted, each generator
in the stack is converted into a list, and each element on the stack
is printed.  In 'persistent' mode, this doesn't happen.  The mode can
be switched using `toggle-mode`:

    $ toggle-mode
    $ 1 2
    $ + ;
    $ .s
    3
    $ .s
    3

All conversions to string (e.g. `str`, `readline`, `f<`) will replace
invalid UTF-8 sequences with `U+FFFD REPLACEMENT CHARACTER`.

For using as a shell within Vim, the following configuration options
should help:

    set shellcmdflag=-e
    set noshelltemp

#### Caveats and pitfalls

Opening a file and using regular expression matching to find a
particular line will be considerably slower than relying on `grep(1)`,
because that executable typically implements various optimisations
past simple line-based matching.  There are likely to be other cases
where calling out to an executable will lead to faster processing when
compared with relying on the shell language alone.

Floating-point numbers are parsed as strings, so that chained
accessors function as expected with `get`.  Strings will be converted
to floating-point numbers as required anyway, but `float` can be used
to convert strings to actual floating-point numbers if that is
necessary for some reason.

There are likely to be many bugs and problems with the implementation
here, and the performance isn't spectacular.  The code could do with
some tidying, too.

There are not currently any guarantees around stability of the
language, or of the underlying bytecode schema.

#### Acknowledgments

 - [Crafting Interpreters](https://craftinginterpreters.com)
    - Much of the structure of the compiler, the operations it
      supports, etc., is based on the text of this (very good) book.
