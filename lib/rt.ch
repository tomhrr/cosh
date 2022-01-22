: and if; if; 1 else; 0 then; else; drop; 0 then; ::
: or if; drop; 1 else; if; 1 else; 0 then; then; ::
: not if; 0 else; 1 then; ::

: nip   swap; drop; ::
: 2over over; over; ::
: 2rot  rot;  rot;  ::

: <= 2over; <; 2rot; =; or; ::
: >= 2over; >; 2rot; =; or; ::

: no-upwards dup; "." =; swap; ".." =; or; not; ::

: is-integer ^\d+ m; ::

:~ ls 1 0
    0 =; if; . then;
    opendir;
    dh var;
    dh !;
    begin;
	dh @;
	readdir;
	dup;
	is-null;
	if;
	    leave;
	then;
	yield;
	0 until;
    drop; ::

:~ lsr 1 0
    0 =; if; . then;
    "/" append;
    dirname var;
    dup;
    dirname !;
    opendir;
    dh var;
    dh !;
    dhs var;
    () dhs !;
    finished var;
    0 finished !;
    begin;
        dh @; readdir;
        dup; is-null;
        if;
            drop;
            dhs @; len; 0 =; if; leave; then;
            dhs @; dup; shift; nip; opendir; dh !;
        else;
            dup; dup; is-dir; swap; no-upwards; and;
            if;
                dup;
                dhs @; swap; push; drop;
            then;
            dup; no-upwards;
            if;
                yield;
            else;
                drop;
            then;
        then;
	finished @; 1 =; until; ::

:~ f< 1 1
    drop;
    r open;
    fh var;
    fh !;
    begin;
        fh @;
        readline;
        dup; is-null; if;
            drop;
            leave;
        then;
        yield;
        0 until; ::

: for
    depth; 2 <; if;
        "for requires two arguments" error;
    then;
    dup; is-callable; not; if;
        "second for argument must be callable" error;
    then;
    fn var;
    fn !;
    dup; is-shiftable; not; if;
        "first for argument must be shiftable" error;
    then;
    lst var;
    lst !;
    begin;
        lst @; dup; shift; nip;
        dup; is-null; if;
            drop;
            leave;
        then;
        fn @; funcall;
        0 until; ::

: f>
    depth; 2 <; if;
        "f> requires two arguments" error;
    then;
    swap; w open; fh var; fh !;
    fh @; drop;
    [ fh @; swap; writeline; ] for;
    fh @; close; ::

: take
    depth; 2 <; if;
        "take requires two arguments" error;
    then;
    dup; is-integer; not; if;
        "second take argument must be integer" error;
    then;
    dup; 0 =; if;
        drop;
        drop;
        ()
    else;
        () lst var; lst !;
        begin;
            swap; dup; shift;
            dup; is-null; if;
                drop;
                leave;
            then;
            lst @; swap; push; drop;
            swap;
            1 -;
            dup; 0 =; until;
        drop;
        drop;
        lst @;
    then; ::

: take-all
    depth; 1 <; if;
        "take-all requires one argument" error;
    then;
    () lst var; lst !;
    begin;
        dup; shift;
        dup; is-null; if;
            drop;
            drop;
            leave;
        then;
        lst @; swap; push; drop;
        0 until;
    lst @; ::

:~ grep-generator 2 2
    drop;
    dup; is-callable; not; if;
        "second grep argument must be callable" error;
    then;
    fn var; fn !;
    dup; is-shiftable; not; if;
        "first grep argument must be shiftable" error;
    then;
    lst var; lst !;
    begin;
        lst @;
        dup;
        shift;
        nip;
        dup; is-null; if;
            leave;
        then;
        dup; fn @; funcall; if;
            yield;
        else;
            drop;
        then;
        0 until; ::

: grep-list
    dup; is-callable; not; if;
        "second grep argument must be callable" error;
    then;
    fn var; fn !;
    dup; is-shiftable; not; if;
        "first grep argument must be shiftable" error;
    then;
    lst var; lst !;
    () reslst var; reslst !;
    begin;
        lst @;
        dup;
        shift;
        nip;
        dup; is-null; if;
            drop;
            leave;
        then;
        dup; fn @; funcall; if;
            reslst @; swap; push; drop;
        else;
            drop;
        then;
        0 until;
    reslst @; ::

: grep
    depth; 2 <; if;
        "grep requires two arguments" error;
    then;
    swap; dup; is-list; if;
        swap;
        grep-list;
    else;
        swap;
        grep-generator;
    then; ::

:~ map-generator 2 2
    drop;
    dup; is-callable; not; if;
        "second map argument must be callable" error;
    then;
    fn var; fn !;
    dup; is-shiftable; not; if;
        "first map argument must be shiftable" error;
    then;
    lst var; lst !;
    begin;
        lst @;
        dup;
        shift;
        nip;
        dup; is-null; if;
            leave;
        then;
        fn @; funcall; yield;
        0 until; ::

: map-list
    dup; is-callable; not; if;
        "second map argument must be callable" error;
    then;
    fn var; fn !;
    dup; is-shiftable; not; if;
        "first map argument must be shiftable" error;
    then;
    lst var; lst !;
    () reslst var; reslst !;
    begin;
        lst @;
        dup;
        shift;
        nip;
        dup; is-null; if;
            drop;
            leave;
        then;
        fn @; funcall;
        reslst @; swap; push; drop;
        0 until;
    reslst @; ::

: map
    depth; 2 <; if;
        "map requires two arguments" error;
    then;
    swap; dup; is-list; if;
        swap;
        map-list;
    else;
        swap;
        map-generator;
    then; ::

:~ range 1 1
    drop;
    dup; is-integer; not; if;
        "range argument must be integer" error;
    then;
    limit var; limit !;
    0 i var; i !;
    begin;
        i @; yield;
        i @; 1 +; i !;
        i @; limit @; >=; until; ::

: foldl
    rot;
    dup; is-shiftable; not; if;
        "first foldl argument must be shiftable" error;
    then;
    lst var; lst !;
    dup; is-callable; not; if;
        "second foldl argument must be callable" error;
    then;
    fn var; fn !;
    begin;
        lst @; dup; shift; nip;
        dup; is-null; if;
            drop;
            leave;
        then;
        fn @; funcall;
        0 until; ::

: partition
    high var; high !;
    low var; low !;
    lst var; lst !;

    lst @; high @; nth;
    pivot var; pivot !;

    low @; 1 -;
    i var; i !;

    low @;
    j var; j !;

    swap var;

    begin;
        lst @; j @; nth; pivot @; <; if;
            i @; 1 +; i !;
            lst @; i @; nth; swap !;
            lst @; i @; lst @; j @; nth; nth!; drop;
            lst @; j @; swap @; nth!; drop;
        then;
        j @; 1 +; j !;
        j @; high @; >=; until;

    i @; 1 +; i !;
    lst @; i @; nth; swap !;
    lst @; i @; lst @; high @; nth; nth!; drop;
    lst @; high @; swap @; nth!; drop;

    i @; ::

: sort-internal
    high var; high !;
    low var; low !;
    lst var; lst !;

    low @; high @; >=; if;
        return;
    then;
    low @; 0 <; if;
        return;
    then;

    lst @; low @; high @; partition;
    p var; p !;

    lst @; low @; p @; 1 -;  sort-internal;
    lst @; p @; 1 +; high @; sort-internal; ::

: sort
    take-all;
    lst var; lst !;
    lst @; dup; 0 swap; len; 1 -; sort-internal; lst @; ::

: partitionp
    fn var; fn !;
    high var; high !;
    low var; low !;
    lst var; lst !;

    lst @; high @; nth;
    pivot var; pivot !;

    low @; 1 -;
    i var; i !;

    low @;
    j var; j !;

    swap var;

    begin;
        lst @; j @; nth; pivot @; fn @; funcall; if;
            i @; 1 +; i !;
            lst @; i @; nth; swap !;
            lst @; i @; lst @; j @; nth; nth!; drop;
            lst @; j @; swap @; nth!; drop;
        then;
        j @; 1 +; j !;
        j @; high @; >=; until;

    i @; 1 +; i !;
    lst @; i @; nth; swap !;
    lst @; i @; lst @; high @; nth; nth!; drop;
    lst @; high @; swap @; nth!; drop;

    i @; ::

: sort-internalp
    fn var; fn !;
    high var; high !;
    low var; low !;
    lst var; lst !;

    low @; high @; >=; if;
        return;
    then;
    low @; 0 <; if;
        return;
    then;

    lst @; low @; high @; fn @; partitionp;
    p var; p !;

    lst @; low @; p @; 1 -;  fn @; sort-internalp;
    lst @; p @; 1 +; high @; fn @; sort-internalp; ::

: sortp
    fn var; fn !;
    take-all;
    lst var; lst !;
    lst @; dup; 0 swap; len; 1 -; fn @; sort-internalp; lst @; ::

: chomp "\n$" "" s; ::

: sum 0 + foldl; ::

: any
    depth; 2 <; if;
        "any requires two arguments" error;
    then;
    dup; is-callable; not; if;
        "second any argument must be callable" error;
    then;
    fn var; fn !;
    dup; is-shiftable; not; if;
        "first any argument must be shiftable" error;
    then;
    lst var; lst !;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            drop;
            0 leave;
        then;
        fn @; funcall; if;
            1 leave;
        then;
        0 until; ::

: all
    depth; 2 <; if;
        "all requires two arguments" error;
    then;
    dup; is-callable; not; if;
        "second all argument must be callable" error;
    then;
    fn var; fn !;
    dup; is-shiftable; not; if;
        "first all argument must be shiftable" error;
    then;
    lst var; lst !;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            drop;
            1 leave;
        then;
        fn @; funcall; not; if;
            0 leave;
        then;
        0 until; ::

: none
    depth; 2 <; if;
        "none requires two arguments" error;
    then;
    dup; is-callable; not; if;
        "second none argument must be callable" error;
    then;
    fn var; fn !;
    dup; is-shiftable; not; if;
        "first none argument must be shiftable" error;
    then;
    lst var; lst !;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            drop;
            1 leave;
        then;
        fn @; funcall; if;
            0 leave;
        then;
        0 until; ::

: notall none; ::

: first
    depth; 2 <; if;
        "first requires two arguments" error;
    then;
    dup; is-callable; not; if;
        "second first argument must be callable" error;
    then;
    fn var; fn !;
    dup; is-shiftable; not; if;
        "first first argument must be shiftable" error;
    then;
    lst var; lst !;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            leave;
        then;
        dup; fn @; funcall; if;
            leave;
        then;
        drop;
        0 until; ::

: min
    depth; 1 <; if;
        "min requires one argument" error;
    then;
    dup; is-shiftable; not; if;
        "min argument must be shiftable" error;
    then;
    lst var; lst !;
    cmin var;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            drop;
            cmin @;
            leave;
        then;
        dup; cmin @; <; if;
            cmin !;
        else;
            drop;
        then;
        0 until; ::

: max
    depth; 1 <; if;
        "max requires one argument" error;
    then;
    dup; is-shiftable; not; if;
        "max argument must be shiftable" error;
    then;
    lst var; lst !;
    cmax var;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            drop;
            cmax @;
            leave;
        then;
        dup; cmax @; >; if;
            cmax !;
        else;
            drop;
        then;
        0 until; ::

: product
    depth; 1 <; if;
        "product requires one argument" error;
    then;
    1 * foldl; ::
