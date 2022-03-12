: myfn
    1000000
    begin;
        ()
        1 push;
        1 push;
        1 push;
        1 push;
        1 push;
        dup; pop; drop;
        dup; pop; drop;
        dup; pop; drop;
        dup; pop; drop;
        dup; pop; drop;
        drop;
        1 -;
        dup; 0 =; until; ::
myfn; drop;
