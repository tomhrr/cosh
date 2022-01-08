: myfn
    bm-file.txt r open;
    begin;
        dup;
        readline;
        dup; is-null; if;
            drop;
            leave;
        then;
        drop;
        0 until; ::
myfn; drop;
