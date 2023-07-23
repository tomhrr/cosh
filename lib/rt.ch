: and if; if; .t else; .f then; else; drop; .f then; ,,
: or if; drop; .t else; if; .t else; .f then; then; ,,
: not if; .f else; .t then; ,,

: nip   swap; drop; ,,

: <= over; over; <; rot; rot; =; or; ,,
: >= over; over; >; rot; rot; =; or; ,,

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

:~ b< 1 1
    drop;
    r open;
    fh var;
    fh !;
    begin;
        fh @;
        1024 read;
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

: b>
    depth; 2 <; if;
        "b> requires two arguments" error;
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
            fh @; swap; write;
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

:~ grep 2 2
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

: is-list-or-set
    dup; is-list; swap; is-set; or; ,,

:~ map 2 2
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

: flatten () ++ foldl; ,,

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

: apply
    n var; n !;
    fn var; fn !;
    lst var; () lst !;

    begin;
        n @;
        dup; 0 =; if;
            drop;
            leave;
        then;
        1 -; n !;
        fn @; funcall;
        lst @; swap; unshift; drop;
        .f until;

    begin;
        lst @;
        shift;
        dup; is-null; if;
            drop;
            leave;
        then;
        .f until; ,,

# ping and pingn are implemented by calling ping(1), to avoid needing
# root privileges in the shell.
: ping
    depth; 1 <; if;
        "ping requires one argument" error;
    then;
    {ping -c 1 -W 5 {}}/oe; r;
    ["1 received" m] first;
    is-null; not;
    ,,

: pingn
    depth; 2 <; if;
        "pingn requires two arguments" error;
    then;
    {ping -c {} -W 5 {}};
    ["bytes from" m] grep;
    ["icmp_seq=(\d+) ttl=(\d+) time=(\d+\.\d+) (.*)" c;
     dup; shift; drop; results var; results !;
     h() res var; res !;
     res @; "icmp_seq" results @; shift; set;
            "ttl"      results @; shift; set;
            "time_ms"  results @; shift; set; drop;
     results @; shift; s =; if;
        res @; time [1000 *] time hm; drop;
     then;
     res @] map;
    ,,

: _list-to-hash
    input var; input !;
    h() result var; result !;
    begin;
        input @; len; 0 =; if;
            leave;
        then;
        result @; input @; 2 take;
        dup; shift; swap; shift; set; drop;
        .f until;
    result @;
    ,,

: _dig-header
    response var; response !;
    results var; results !;

    results @; ["<<>>" m] first;
    dup; is-null; if;
        "no top section in response" error;
    then;
    top var; top !;
    top @; "->>HEADER<<- (.*)\n" c; 1 get;
    dup; is-null; if;
        "no header section in response" error;
    then;
    ,\s* splitr; [\s*:\s* splitr] map; flatten; r; _list-to-hash;
    response @; header rot; set; drop;
    ,,

: _dig-questions
    response var; response !;
    results var; results !;

    results @; ["QUESTION SECTION" m] first;
    dup; is-null; not; if;
        "QUESTION SECTION:\n;(.*)" c; 1 get; \s+ splitr;
        qlist var; qlist !;
        h() question var; question !;
        question @; name  qlist @; shift; set;
                    type  qlist @; shift; set;
                    class qlist @; shift; set;
        response @; question rot; set; drop;
    else;
        drop;
    then;
    ,,

: _dig-rrs
    response-name var; response-name !;
    response var; response !;
    elist var; elist !;

    () entries var; entries !;
    begin;
        elist @; len; 0 =; if;
            leave;
        then;
        elist @; shift; \s+ splitr; e var; e !;
        h() entry var; entry !;
        entry @; name  e @; shift; set;
                 ttl   e @; shift; set;
                 class e @; shift; set;
                 type  e @; shift; set;
                 rdata e @;
                    dup; clone;
                    rdata var; rdata !;
                    " " join; set;
        entries @; swap; push; drop;

        h() sdata var; sdata !;
        entry @; type get;
        dup; is-null; not; if;
            dup; A =; if;
                sdata @; address  rdata @; 0 get; set; drop;
            else; dup; AAAA =; if;
                sdata @; address  rdata @; 0 get; set; drop;
            else; dup; NS =; if;
                sdata @; nsdname  rdata @; 0 get; set; drop;
            else; dup; CNAME =; if;
                sdata @; cname    rdata @; 0 get; set; drop;
            else; dup; PTR =; if;
                sdata @; ptrdname rdata @; 0 get; set; drop;
            else; dup; TXT =; if;
                sdata @; txtdata  rdata @; 0 get; set; drop;
            else; dup; MX =; if;
                sdata @; preference rdata @; 0 get; set;
                         exchange   rdata @; 1 get; set; drop;
            else; dup; SOA =; if;
                sdata @; mname   rdata @; 0 get; set;
                         rname   rdata @; 1 get; set;
                         serial  rdata @; 2 get; set;
                         refresh rdata @; 3 get; set;
                         retry   rdata @; 4 get; set;
                         expire  rdata @; 5 get; set;
                         minimum rdata @; 6 get; set; drop;
            else; dup; DS =; if;
                sdata @; keytag    rdata @; shift; set;
                         algorithm rdata @; shift; set;
                         digtype   rdata @; shift; set;
                         digest    rdata @; " " join; set; drop
            else; dup; DNSKEY =; if;
                sdata @; flags     rdata @; shift; set;
                         protocol  rdata @; shift; set;
                         algorithm rdata @; shift; set;
                         keybin    rdata @; " " join; set; drop
            then; then; then; then; then; then; then; then; then; then;
            drop;
        else;
            drop;
        then;

        sdata @; len; 0 =; not; if;
            entry @; sdata sdata @; set; drop;
        then;

        .f until;
    response @; response-name @; entries @; set; drop;
    response @;
    ,,

: _dig-rrs-section
    section-name var; section-name !;
    response-name var; response-name !;
    response var; response !;
    results var; results !;

    results @; [section-name @; " SECTION" ++; m] first;
    dup; is-null; not; if;
        ".*" section-name @; ++; " SECTION:\n" ++; '' s; \n splitr;
        response @;
        response-name @;
        _dig-rrs;
    then;
    drop;
    ,,

: dig
    depth; 2 <; if;
        "dig requires two arguments" error;
    then;
    {dig {} {}}; r;
    "" join;
    "\n\n" split;
    [^\s* '' s; \s*$ '' s] map; r;
    results var; results !;

    h() response var; response !;

    results @; clone; response @;
    _dig-header;

    results @; clone; response @;
    _dig-questions;

    results @; clone; response @; answer     ANSWER     _dig-rrs-section;
    results @; clone; response @; additional ADDITIONAL _dig-rrs-section;
    results @; clone; response @; authority  AUTHORITY  _dig-rrs-section;

    response @;
    ,,

:~ dig/t 2 2
    drop;
    depth; 2 <; if;
        "dig/t requires two arguments" error;
    then;
    {dig +trace {} {}};
    results var; results !;
    results @; shift; drop;
    results @; shift; drop;
    results @; shift; drop;
    begin;
        results @; [\n =] before;
        ["^;;" m; not] grep; r;
        dup; clone; len; 0 =; if;
            drop;
            null yield;
        then;
        h() answer _dig-rrs;
        yield;
        .f until;
    ,,

: digat
    depth; 3 <; if;
        "digat requires three arguments" error;
    then;
    {dig @{} {} {}}; r;
    "" join;
    "\n\n" split;
    [^\s* '' s; \s*$ '' s] map; r;
    results var; results !;

    h() response var; response !;

    results @; clone; response @;
    _dig-header;

    results @; clone; response @;
    _dig-questions;

    results @; clone; response @; answer     ANSWER     _dig-rrs-section;
    results @; clone; response @; additional ADDITIONAL _dig-rrs-section;
    results @; clone; response @; authority  AUTHORITY  _dig-rrs-section;

    response @;
    ,,
