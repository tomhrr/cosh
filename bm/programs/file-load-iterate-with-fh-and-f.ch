// Open a file and iterate over its contents, storing the filehandle
// in a variable and using a separate wrapper function for the drop.
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
