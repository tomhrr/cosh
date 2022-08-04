// Open a file and iterate over its contents, storing the filehandle
// in a variable.
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
