: myfn
    bm-file.txt r open;
    fh var;
    fh !;
    begin;
        fh @;
        readline;
        dup; is-null; if;
            drop;
            leave;
        then;
        drop;
        0 until; ::
myfn;
