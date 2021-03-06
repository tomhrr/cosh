## cosh

cosh is a concatenative command-line shell.

### Types

The shell language is dynamically-typed.  The basic types are:

  * `Int`: signed integer (32-bit)
  * `BigInt`: arbitrary-precision integer
  * `Float`: double-width floating-point number
  * `String`: a string
  * `List`: a list of values
  * `Hash`: a hash map of values

A number with a fractional component is interpreted as a `Float`.  A
number without a fractional component is interpreted as an `Int`, if
it fits within a 32-bit signed integer, and as a `BigInt` otherwise.
A series of tokens enclosed within parentheses (`(...)`) is
interpreted as a `List`, and a series of tokens enclosed within
parentheses and preceded by an `h` character (`h(...)`) is interpreted
as a `Hash`.  All other tokens are interpreted as `Strings`s.  (To
construct a string that contains whitespace, use double-quote
characters to delimit the string.)

The forms `str`, `int`, and `flt` can be used to convert primitive
values of one type to another type.  `int` will convert a value to a
`BigInt` if required.

### Basic usage

All `String`s are quoted by default.  The semicolon character will
cause the definition associated with the last string on the stack to be
executed:

    $ 1 2 + ;
    3

A semicolon may instead be appended to the `String`:

    $ 1 2 +;
    3

A newline acts as an implicit semicolon, when the last `String` on the
stack has an associated definition:

    $ 1 2 + ; 1 2 + ; +
    6

`Function`s are defined like so:

    $ : add-1 1 + ; ::
    $ 1 add-1
    2

Conditional execution is handled by `if`, which will evaluate to true
for any non-zero value:

    $ : to-bool if; 1 else; 0 then; ::
    $ 100 to-bool
    1
    $ 0 to-bool
    0

Variable definition and load/store operations are like so:

    $ x var;
    $ 0 x !;
    $ x @
    0
    $ x @; 1 +; x !; x @
    1
    $ x @; 1 +; x !; x @
    2

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

### Stack operators

Some of the more commonly-used stack operators from Forth are defined:

    $ 1 clear
    $ 1 dup
    1
    1
    $ 1 2 swap
    2
    1
    $ 1 1 drop
    1
    $ 1 2 3 rot
    2
    3
    1
    $ 1 2 3 depth
    1
    2
    3
    3

### Type predicates

`is-null` returns a boolean indicating whether the argument is null.

`is-list` returns a boolean indicating whether the argument is a list.

`is-callable` returns a boolean indicating whether the argument is
callable.

### Boolean operators

`and` is the conjunction function, taking two values and returning a
boolean indicating whether both values evaluate to true.

`or` is the disjunction function, taking two values and returning a
boolean indicating whether at least one value evaluates to true.

`not` is the negation function, taking a value and returning a boolean
indicating whether that value evaluates to false.

Both `and` and `or` evaluate each of their expressions.  `if` can be
used to avoid this behaviour, if necessary.

### Arithmetic and relations

`+`, `-`, `*`, `/`, `=`, `<`, and `>` are defined over `Int`s,
`BigInt`s, and `Float`s.  `=`, `<`, and `>` are also defined over
`String`s.

### Anonymous functions

An anonymous function is defined by way of a list that contains the
function's tokens.  It is executed using `funcall`:

    $ [1 2 +] funcall ;
    3

The last token in the list is treated as a function implicitly, if it is
not followed by a semicolon.

### Scoping

Scoping is lexical.  Variable definitions within functions may shadow
global definitions:

    $ x var; 10 x !; x @;
    10
    $ : mf1 x var; 20 x !; x @; .s; drop; :: mf1 ;
    20
    $ x @;
    10
    $ : mf2 20 x !; x @; .s; drop; :: mf2 ;
    20
    $ x @;
    20

Function definitions may be nested.  Nested functions have access
to the surrounding environment when they are executed, but they do not
close over that environment.

### Generators

A function may be defined as a generator function.  When such a
function is called, it returns a generator object, which can be
iterated by way of `shift`.  On iteration, function execution
continues until a `yield` statement is reached, at which point control
returns to the caller.  When the generator object is next iterated,
execution resumes from the point after the `yield` statement.  For
example:

    $ :~ gen 0 0 drop; n var; 0 n !; begin; n @; yield; n @; 1 +; n !; n @; 3 >; until; ::
    $ : iter dup; shift; println; ::
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

`take` can be used to return a certain number of elements from a
generator object:

    $ gen; 2 take;
    (
        0
        1
    )

`take-all` can be used to return all the elements from a generator
object:

    $ gen; take-all;
    (
        0
        1
        2
    )

In 'immediate' mode, each generator object that remains on the stack
after command execution is finished will be replaced with the result
of calling `take-all` on that object before the stack is printed:

    $ gen;
    0
    1
    2
    3

`shift`, `take`, and `take-all` also work in the same way on lists.
In general, any built-in form that works on a list will also work on a
generator, and if it operates as a transformation, then its result
will also be a generator.

### map, grep, for, foldl

`map` iterates over a list, applying a function to each element and
collecting the results into a new list:

    $ : add-1 1 + ; ::
    $ (1 2 3 4) add-1 map
    (
        2
        3
        4
        5
    )

`grep` iterates over a list, applying a predicate to each element and
collecting the values for which the predicate is true into a new list:

    $ : <4 4 < ; ::
    $ (1 2 3 4) add-1 map; <4 grep;
    (
        2
        3
    )

`for` is the same as map, except that it does not collect the results
into a new list (i.e. the function is executed only for its
side effects).

`foldl` takes a list, a seed, and a function, applies the function to
the seed and the first element from the list to produce a value, and
then continues applying the function to the resulting value and the
next element from the list until the list is exhausted:

    $ (1 2 3) 0 + foldl
    6

Anonymous functions can be used inline in these calls:

    $ (1 2 3 4) [1 +] map
    (
	2
	3
	4
	5
    )

#### Miscellaneous list functions

`any` takes a list and a function, and returns a boolean indicating
whether the function returns true for any element of the list.

`all` takes a list and a function, and returns a boolean indicating whether
the function returns true for all of the elements of the list.

`none`/`notall` is like `all`, except it returns a boolean indicating
whether the function returns false for all of the elements of the
list.

`first` takes a list and a function, and returns the first
element for which the function returns true.

`uniq` takes a list, and returns a generator over the unique elements
from that list (uniqueness is determined by converting each value to a
string and comparing the strings).

`min` takes a list and returns the smallest element of that list, and
`max` takes a list and returns the largest element of that list.

`shuffle` takes a list and moves each element to a random location in
the list.

`product` multiplies all of the elements of the list together and
returns the result.

### sort, sortp

`sort` sorts a list or generator, where the values in the list are of
primitive types:

    $ (1 3 5 4 2 1) sort
    (
	1
	1
	2
	3
	4
	5
    )

`sortp` accepts an additional predicate argument:

    $ (1 3 5 4 2 1) > sortp
    (
	5
	4
	3
	2
	1
	1
    )

### Filesystem operations

`ls` takes a directory name as its argument and returns a generator
object over the files in that directory:

    $ . ls
    (
        ./Cargo.toml
        ...
    )

`lsr` does the same thing, but includes all files within nested
directories as well.  If the stack is empty when either of these
functions is called, then they will act as if they were called on the
current working directory.

`f<` takes a filename as its argument and returns a generator over the
lines in that file:

    $ README.md f<; 3 take;
    (
	"## cosh\n"
	"\n"
	"cosh is a concatenative command-line shell.\n"
    )

`f>` takes a filename and a list of strings as its arguments and
writes the strings to that file:

    $ asdf ("asdf\n" "qwer\n" "zxcv\n") f>;
    $ asdf f<;
    (
	"asdf\n"
	"qwer\n"
	"zxcv\n"
    )

Other operations:

 - `cd`: changes the current working directory;
 - `pwd`: returns the current working directory;
 - `is-dir`: returns a boolean indicating whether the argument is a
   directory;
 - `rm`: removes the argument file;
 - `touch`: if the argument file doesn't exist, creates an empty file with the
   given name, otherwise updates the modification time of the existing
   file to be the current time;
 - `stat`: returns a hash containing metadata about the argument file;
 - `ps`: returns a list containing details on the currently-running
   processes, where each current process has a separate hash
   containing the PID, UID, and the process name; and
 - `kill`: takes a PID and a signal name (e.g. "term", "kill"), and
   sends the specified signal to the process.

### Regular expressions

The two basic regular expression forms are `m` and `c`.  The `m` form
returns a boolean indicating whether a string matches against a
regular expression.  The `c` form is similar, except that it results
in a list containing the captures from the expression (if any):

    $ asdf asdf m
    1
    $ asdf asdf2 m
    0
    $ asdf "(..)" c
    (
        as
        df
    )

The `s` form handles search and replace:

    $ asdf as qw s;
    qwdf

For the supported syntax, see the Rust
[https://docs.rs/regex/1.3.9/regex/index.html#syntax](regex) crate.
It is close to that of PCRE, except that lookahead and backreferences
are not supported.

### List operators

`nth` returns a specific element from a list:

    $ (1 2 3) 1 nth
    2

`nth!` updates a specific element in a list:

    $ (1 2 3 4) 2 10 nth!;
    (
	1
	2
	10
	4
    )

`gnth` is the name of the form that does the above for generators.  It
has a different name, because it involves reading elements from the
generator until the specified element is reached, so its semantics are
different from `nth`, which does not alter the argument list.

`split` splits a string based on a delimiter string:

    $ asdf,asdf , split
    (
        asdf
        asdf
    )

`join` joins a list of strings together using a delimiter string:

    $ asdf,asdf , split; , join
    "asdf,asdf"

Both `split` and `join` handle quoting of values that contain either
the delimiter, or a quotation mark.

`splitr` splits a string based on a delimiter regex.  It does not
handle quoting of values, though.

### String-handling functions

`append` appends one string to another:

    $ asdf qwer append
    "asdfqwer"

`chomp` removes the final newline from the end of a string, if the
string ends in a newline:

    $ "asdf\n" chomp
    "asdf"
    $ "asdf" chomp
    "asdf"

### External program execution

A command that begins with `$` will be treated as an external call:

    $ $ls
    bin     eg      LICENSE     ...

When using the REPL, a line that begins with a space character will
also be treated as an external call.

A form wrapped in braces operates in the same way, except that
the result is a generator:

    $ {ls}; take-all;
    (
        "bin\n"
        ...
    )

Values can be substituted into external program calls, either by
popping values from the stack, or by indexing into the stack (0 is the
element most recently pushed onto the stack):

    $ {ls}; [{stat -c "%s" {}}; shift; chomp] map;
    (
	4096
	4096
        ...
    )

    {ls}; [{stat -c "%s" {}}; shift; chomp] map;
    (
        4096
        4096
        22813
        ...
    )
    $ 1 2 3 {dc -e "{0} {1} + {2} + p"} shift; chomp
    1
    2
    3
    6

The output of a generator can also be piped to a command:

    $ {ls}; {sort -r} |; take-all;
    (
        "tests\n"
        "test-data\n"
        ...
    )

### Hashes

Hashes support `at` for retrieving a value, `at!` for
updating a value, `keys` for getting a generator over the hash's keys,
and `values` for getting a generator over the hash's values:

    $ h(a 1 b 2) dup; a at; swap; b at;
    1
    2
    $ h(a 1 b 2) c 3 at!; c at;
    3
    $ h(a 1 b 2) c 3 at!; keys; take-all;
    (
        b
        a
        c
    )
    $ h(a 1 b 2) c 3 at!; values; take-all;
    (
        2
        1
        3
    )
    $ h(a 1 b 2) c 3 at!; each; take-all;
    (
	(
	    b
	    2
	)
	(
	    a
	    1
	)
	(
	    c
	    3
	)
    )

### Parsing

JSON and XML can be serialised and deserialised using the
`from-json`, `to-json`, `from-xml` and `to-xml` functions.

### Miscellaneous functions

`rand` takes a floating-point value and returns a random value between
zero and that floating-point value (excluding the floating-point value
itself).

### Miscellaneous

On starting the shell for interactive use, the `.coshrc` file in the
current user's home directory will be run.
