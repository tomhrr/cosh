# Command line option parser.

: getopts._usage-text
    [(doc long short) get; shift-all;
     "  -{}/--{}	{}" fmt] map; r;
    "Usage:" unshift;
    \n join;
    ,,

: usage
    eprintln;
    getopts._usage-text;
    eprintln;
    10 exit;
    ,,

: getopts
    options var!;
    depth; mlist; myargs var!;
    () free var!;

    0 i var!;
    begin;
        options @; i @; get;
        dup; name exists; not; if;
            dup; long exists; if;
                dup; long get; name swap; set; drop;
            else;
                dup; short get; name swap; set; drop;
            then;
        else;
            drop;
        then;
        i @; 1 +; i !;
        i @; options @@; len; >=; until;

    h() by-short var!;
    options @@;
    [short exists] grep;
    [dup; short get; swap; by-short @; rot; rot; set; drop] for;

    h() by-long var!;
    options @@;
    [long exists] grep;
    [dup; long get; swap; by-long @; rot; rot; set; drop] for;

    h() by-name var!;
    options @@;
    [dup; name get; swap; by-name @; rot; rot; set; drop] for;

    h() results var!;
    by-name @; each;
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
                arg @; name get;
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
    [results @; swap; name get; exists; not] first;
    dup; is-null; not; if;
        dup; long exists; if;
            name get;
            "--{} is a required argument" fmt; options @; swap; usage;
        else;
            name get;
            "-{} is a required argument" fmt; options @; swap; usage;
        then;
    else;
        drop;
    then;

    results @; free @;
    ,,
