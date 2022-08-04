// Increment a number until a large number is reached.
: myfn
    0
    begin;
        1 +;
        dup;
        20000000 =; until; ::
myfn; drop;
