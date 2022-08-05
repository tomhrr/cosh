# Decrement a number until zero is reached.
: myfn
    20000000
    begin;
        1 -;
        dup;
        0 =; until; ::
myfn; drop;
