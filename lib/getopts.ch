# Command line option parser.

: usage
    errormsg var!;
    options var!;
    errormsg @; eprintln;
    "Usage:" eprintln;
    options @;
    [(doc long short) get; shift-all;
     "  -{}/--{}	{}" fmt; eprintln] for;
    10 exit;
    ,,

: getopts
    options var!;
    depth; mlist; myargs var!;
    () free var!;

    h() by-short var!;
    options @; clone;
    [dup; short get; swap; by-short @; rot; rot; set; drop] for;

    h() by-long var!;
    options @; clone;
    [dup; long get; swap; by-long @; rot; rot; set; drop] for;

    h() results var!;
    by-long @; each;
    [shift-all; default get; results @; rot; rot; set; drop] for;

    0 i var!;
    myargs @; clone; len; 0 =; not; if;
        begin;
            myargs @; i @; get; myarg var!;
            i @; 1 +; i !;
            myarg @; ^- '' s; by-short @; swap; get; dup; is-null; not; if;
                arg var!;
                results @;
                arg @; long get;
                arg @; type get; bool =; if;
                    .t set; drop;
                else;
                    myargs @; i @; get; dup; is-null; if;
                        options @; "incorrect number of arguments" usage;
                    then;
                    set; drop;
                    i @; 1 +; i !;
                then;
            else; drop; myarg @; ^-- '' s; by-long @; swap; get; dup; is-null; not; if;
                arg var!;
                results @;
                arg @; long get;
                arg @; type get; bool =; if;
                    .t set; drop;
                else;
                    myargs @; i @; get; dup; is-null; if;
                        options @; "incorrect number of arguments" usage;
                    then;
                    set; drop;
                    i @; 1 +; i !;
                then;
            else;
                free @; myarg @; push; drop;
            then; then;
            i @; 1 +; myargs @; clone; len; >; until;
    then;

    results @; free @;
    ,,
