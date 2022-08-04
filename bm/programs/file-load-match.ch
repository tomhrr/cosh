// Open a file and iterate over its contents, checking for a string
// match on each line.
: myfn
    bm-file.txt r open;
    begin;
        dup;
        readline;
        dup; is-null; if;
            drop;
            leave;
        then;
        e m;
        drop;
        0 until; ::
myfn; drop;
