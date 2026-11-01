## getopts

Utility functions for dealing with command-line arguments in
standalone scripts.

### Usage

    $ double.ch f<; print for
    getopts import;

    (h(short "h"
       long "help"
       type "bool"
       default .f
       doc "Display help text.")
     h(short "n"
       long "num"
       type "int"
       required .t
       doc "Number to double")) getopts;

    drop;
    num get; 2 *; println;
    $  cosh double.ch -n 10
    20

### Functions

 - `getopts`
    - Takes an arbitrary number of arguments, followed by a list of
      hashes defining the options.  The option hash has the following
      entries:
        - `short`: the single letter option specifier
        - `long`: the long option specifier
        - `name`: the key to use in the results (optional)
        - `type`: the type of the option
        - `is-list`: whether the option can be provided multiple times
        - `default`: the default value for the option (optional)
        - `required`: whether the option is required (defaults to false)
        - `doc`: documentation text, for displaying usage
      At least one of `short` and `long` must be provided.  `name`
      defaults to the value of `long`, unless `long` is not present,
      in which case it defaults to the value of `short`.  If `is-list`
      is set, the result for the option will be a list.
      Returns a list of free arguments, followed by a hash containing
      the option details.  If the arguments did not meet the
      requirements from the options, then this will instead print
      usage details and exit.
