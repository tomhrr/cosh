: mfn
    bm-file.txt r open;
    : dd drop; ::
    fh var;
    fh !;
    begin;
        fh @;
        readline;
        dup; is-null; if;
            drop;
            leave;
        then;
        dd;
        0 until; ::
mfn;
