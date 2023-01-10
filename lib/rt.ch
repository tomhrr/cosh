: and if; if; .t else; .f then; else; drop; .f then; ,,
: or if; drop; .t else; if; .t else; .f then; then; ,,
: not if; .f else; .t then; ,,

: nip   swap; drop; ,,
: 2over over; over; ,,
: 2rot  rot;  rot;  ,,

: <= 2over; <; 2rot; =; or; ,,
: >= 2over; >; 2rot; =; or; ,,

: no-upwards dup; "." =; swap; ".." =; or; not; ,,

: id ,,

:~ lsh 1 0
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
	.f until;
    drop; ,,

: ls
    depth;
    0 =; if; . then;
    lsh; [ "/\." m; not; ] grep; ,,

:~ lshr 1 0
    0 =; if; . then;
    "/" ++;
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
            dhs @; shift; opendir; dh !;
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
	finished @; 1 =; until; ,,

: lsr
    depth;
    0 =; if; . then;
    lshr; [ "/\." m; not; ] grep; ,,

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
        .f until; ,,

: for
    depth; 2 <; if;
        "for requires two arguments" error;
    then;
    dup; is-callable; not; if;
        "second for argument must be callable" error;
    then;
    fn var;
    to-function;
    fn !;
    dup; is-shiftable; not; if;
        "first for argument must be shiftable" error;
    then;
    lst var;
    lst !;
    begin;
        lst @; shift;
        dup; is-null; if;
            drop;
            leave;
        then;
        fn @; funcall;
        .f until; ,,

: f>
    depth; 2 <; if;
        "f> requires two arguments" error;
    then;
    w open; fh var; fh !;
    dup; is-str; if;
        fh @; swap; writeline;
        fh @; close;
    else;
        begin;
            dup; shift;
            dup; is-null; if;
                drop;
                drop;
                leave;
            then;
            fh @; swap; writeline;
            .f until;
        fh @; close;
    then; ,,

: take
    depth; 2 <; if;
        "take requires two arguments" error;
    then;
    dup; int; is-null; if;
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
    then; ,,

: take-all
    depth; 1 <; if;
        "take-all requires one argument" error;
    then;
    dup; is-list; if;
        return;
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
        .f until;
    lst @; ,,

:~ grep-generator 2 2
    drop;
    dup; is-callable; not; if;
        "second grep argument must be callable" error;
    then;
    fn var; to-function; fn !;
    dup; is-shiftable; not; if;
        "first grep argument must be shiftable" error;
    then;
    lst var; lst !;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            leave;
        then;
        dup; fn @; funcall; if;
            yield;
        else;
            drop;
        then;
        .f until; ,,

: grep-list
    dup; is-callable; not; if;
        "second grep argument must be callable" error;
    then;
    fn var; to-function; fn !;
    dup; is-shiftable; not; if;
        "first grep argument must be shiftable" error;
    then;
    lst var; lst !;
    () reslst var; reslst !;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            drop;
            leave;
        then;
        dup; fn @; funcall; if;
            reslst @; swap; push; drop;
        else;
            drop;
        then;
        .f until;
    reslst @; ,,

: is-list-or-set
    dup; is-list; swap; is-set; or; ,,

: grep
    depth; 2 <; if;
        "grep requires two arguments" error;
    then;
    swap; dup; is-list-or-set; if;
        swap;
        grep-list;
    else;
        swap;
        grep-generator;
    then; ,,

:~ map-generator 2 2
    drop;
    dup; is-callable; not; if;
        "second map argument must be callable" error;
    then;
    fn var; to-function; fn !;
    dup; is-shiftable; not; if;
        "first map argument must be shiftable" error;
    then;
    lst var; lst !;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            leave;
        then;
        fn @; funcall; yield;
        .f until; ,,

: map-list
    dup; is-callable; not; if;
        "second map argument must be callable" error;
    then;
    fn var; to-function; fn !;
    dup; is-shiftable; not; if;
        "first map argument must be shiftable" error;
    then;
    lst var; lst !;
    () reslst var; reslst !;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            drop;
            leave;
        then;
        fn @; funcall;
        reslst @; swap; push; drop;
        .f until;
    reslst @; ,,

: map
    depth; 2 <; if;
        "map requires two arguments" error;
    then;
    swap; dup; is-list-or-set; if;
        swap;
        map-list;
    else;
        swap;
        map-generator;
    then; ,,

:~ range 1 1
    drop;
    dup; int; is-null; if;
        "range argument must be integer" error;
    then;
    limit var; limit !;
    0 i var; i !;
    begin;
        i @; yield;
        i @; 1 +; i !;
        i @; limit @; >=; until; ,,

: foldl
    rot;
    dup; is-shiftable; not; if;
        "first foldl argument must be shiftable" error;
    then;
    lst var; lst !;
    dup; is-callable; not; if;
        "second foldl argument must be callable" error;
    then;
    fn var; to-function; fn !;
    begin;
        lst @; shift;
        dup; is-null; if;
            drop;
            leave;
        then;
        fn @; funcall;
        .f until; ,,

: chomp "\n$" "" s; ,,

: sum 0 + foldl; ,,

: any
    depth; 2 <; if;
        "any requires two arguments" error;
    then;
    dup; is-callable; not; if;
        "second any argument must be callable" error;
    then;
    fn var; to-function; fn !;
    dup; is-shiftable; not; if;
        "first any argument must be shiftable" error;
    then;
    lst var; lst !;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            drop;
            .f leave;
        then;
        fn @; funcall; if;
            .t leave;
        then;
        .f until; ,,

: all
    depth; 2 <; if;
        "all requires two arguments" error;
    then;
    dup; is-callable; not; if;
        "second all argument must be callable" error;
    then;
    fn var; to-function; fn !;
    dup; is-shiftable; not; if;
        "first all argument must be shiftable" error;
    then;
    lst var; lst !;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            drop;
            .t leave;
        then;
        fn @; funcall; not; if;
            .f leave;
        then;
        .f until; ,,

: none
    depth; 2 <; if;
        "none requires two arguments" error;
    then;
    dup; is-callable; not; if;
        "second none argument must be callable" error;
    then;
    fn var; to-function; fn !;
    dup; is-shiftable; not; if;
        "first none argument must be shiftable" error;
    then;
    lst var; lst !;
    begin;
        lst @;
        shift;
        dup; is-null; if;
            drop;
            .t leave;
        then;
        fn @; funcall; if;
            .f leave;
        then;
        .f until; ,,

: notall none; ,,

: first
    depth; 2 <; if;
        "first requires two arguments" error;
    then;
    dup; is-callable; not; if;
        "second first argument must be callable" error;
    then;
    fn var; to-function; fn !;
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
        .f until; ,,

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
        .f until; ,,

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
        .f until; ,,

: product
    depth; 1 <; if;
        "product requires one argument" error;
    then;
    1 * foldl; ,,

: shuffle
    depth; 1 <; if;
        "shuffle requires one argument" error;
    then;
    take-all;
    lst var; lst !;
    lst @; len; lstlen var; lstlen !;
    i var; 0 i !;
    begin;
        i @; lstlen @; >=; if;
            lst @;
            leave;
        then;
        rand-index var;
        lstlen @; rand; int; rand-index !;
        temp var;
        lst @; i @; get; temp !;
        lst @; i @; lst @; rand-index @; get; set;
        rand-index @; temp @; set;
        drop;
        i @; 1 +; i !;
        .f until; ,,

:~ uniq 1 1
    drop;
    depth; 1 <; if;
        "uniq requires one argument" error;
    then;
    lst var; lst !;
    seen var; h() seen !;
    begin;
        lst @; shift;
        dup; is-null; if;
            leave;
        then;
        dup; str; seen @; swap; get; is-null; if;
            dup; str; seen @; swap; 1 set; drop;
            yield;
        else;
            drop;
        then;
        .f until; ,,

:~ pairwise 3 3
    drop;
    fn var; to-function; fn !;
    lst2 var; lst2 !;
    lst1 var; lst1 !;
    begin;
        lst1 @; shift;
        dup; is-null; if;
            leave;
        then;
        lst2 @; shift;
        dup; is-null; if;
            leave;
        then;
        fn @; funcall; yield;
        .f until; ,,

:~ slide 2 2
    drop;
    fn var; to-function; fn !;
    lst var; lst !;
    last var;

    lst @; shift;
    dup; is-null; if;
        return;
    then;
    lst @; shift;
    dup; is-null; if;
        return;
    then;
    dup; last !;

    fn @; funcall; yield;

    begin;
        last @;
        lst @; shift;
        dup; is-null; if;
            drop;
            drop;
            leave;
        then;
        dup; last !;

        fn @; funcall; yield;
        .f until; ,,

:~ before 2 2
    drop;
    fn var; to-function; fn !;
    lst var; lst !;

    begin;
        lst @; shift;
        dup; is-null; if;
            leave;
        then;
        dup;
        fn @; funcall; not; if;
            yield;
        else;
            drop;
            leave;
        then;
        .f until; ,,

:~ after 2 2
    drop;
    fn var; to-function; fn !;
    lst var; lst !;

    begin;
        lst @; shift;
        dup; is-null; if;
            leave;
        then;
        fn @; funcall; if;
            begin;
                lst @; shift;
                dup; is-null; if;
                    leave;
                then;
                yield;
                .f until;
            leave;
        then;
        .f until; ,,
