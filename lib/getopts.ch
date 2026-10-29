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
    options @@;
    [dup; short get; swap; by-short @; rot; rot; set; drop] for;

    h() by-long var!;
    options @@;
    [dup; long get; swap; by-long @; rot; rot; set; drop] for;

    h() results var!;
    by-long @; each;
    [1 get; default exists] grep;
    [shift-all; default get; results @; rot; rot; set; drop] for;

    0 i var!;
    myargs @; clone; len; 0 =; not; if;
        begin;
            myargs @; i @; get; myarg var!;
            i @; 1 +; i !;
            arg var;
            myarg @; ^- '' s; by-short @; swap; get; dup; is-null; not; if;
                arg !;
            else; drop; myarg @; ^-- '' s; by-long @; swap; get; dup; is-null; not; if;
                arg !;
            else;
                free @; myarg @; push; drop;
            then; then;
            arg @; is-null; not; if;
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
            then;
            i @; 1 +; myargs @; clone; len; >; until;
    then;

    options @@;
    [required exists] grep;
    [required get] grep;
    [results @; swap; long get; exists; not] first;
    dup; is-null; not; if;
        long get;
        "--{} is a required argument" fmt; options @; swap; usage;
    else;
        drop;
    then;

    results @; free @;
    ,,
