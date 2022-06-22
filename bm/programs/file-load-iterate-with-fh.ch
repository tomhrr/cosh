: myfn
    bm-file.txt r open;
    fh var;
    fh !;
    begin;
        fh @;
        readline;
        is-null; if;
            leave;
        then;
        0 until; ::
myfn;
