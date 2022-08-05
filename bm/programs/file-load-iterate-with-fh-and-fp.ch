# Open a file and iterate over its contents, storing the filehandle
# in a variable and using a pointer to a separate wrapper function for the
# drop.
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
