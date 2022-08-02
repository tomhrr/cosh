: mfn
    bm-file.txt r open;
    : dd drop; ::
    fn var;
    dd to-function; fn !;
    fh var;
    fh !;
    begin;
        fh @;
        readline;
        dup; is-null; if;
            drop;
            leave;
        then;
        fn @; funcall;
        0 until; ::
mfn;
