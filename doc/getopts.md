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
        - `type`: the type of the option
        - `default`: the default value for the option (optional)
        - `required`: whether the option is required (defaults to false)
        - `doc`: documentation text, for displaying usage
      Returns a list of free arguments, followed by a hash containing
      the option details.  If the arguments did not meet the
      requirements from the options, then this will instead print
      usage details and exit.
