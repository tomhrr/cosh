# Common functions and variables.

: and if; if; .t else; .f then; else; drop; .f then; ,,
: or if; drop; .t else; if; .t else; .f then; then; ,,
: not if; .f else; .t then; ,,

: nip   swap; drop; ,,

: <= over; over; <; rot; rot; =; or; ,,
: >= over; over; >; rot; rot; =; or; ,,

: no-upwards dup; "." =; swap; ".." =; or; not; ,,

: id ,,

: get-dir-for-ls
    swap; cwd-param var!;
    1 =; if; . then;
    dirname var!;

    cwd; cwd-param @; =; not; if;
        dirname @; . =; if;
            cwd-param @;
        else;
            cwd-param @; / ++; dirname @; ++;
        then;
        dirname !;
    then;
    dirname @;
    ,,

:~ _lsh 2 1
    get-dir-for-ls;
    dirname var!;
    dirname @;

    opendir;
    dh var; dh !;
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

: lsh
    cwd; _lsh; ,,

: ls-filter-path
    expand-tilde; "/*$" "" s; "^{}/\." fmt; ,,

:~ _ls 2 1
    get-dir-for-ls;
    dirname var!;
    dirname @;

    dup; ls-filter-path; myre var; myre !;
    lsh; lsv var; lsv !;
    begin;
        lsv @; shift; dup;
        is-null; if;
            drop;
            leave;
        else;
            dup; myre @; m; not; if;
                yield;
            else;
                drop;
            then;
        then;
        .f until; ,,

: ls
    cwd; _ls; ,,

:~ _lshr 2 1
    get-dir-for-ls;
    dirname var!;
    dirname @;

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

: lshr
    cwd; _lshr; ,,

:~ _lsr 2 1
    get-dir-for-ls;
    dirname var!;
    dirname @;

    dup; ls-filter-path; myre var; myre !;
    lshr; lsv var; lsv !;
    begin;
        lsv @; shift; dup;
        is-null; if;
            drop;
            leave;
        else;
            dup; myre @; m; not; if;
                yield;
            else;
                drop;
            then;
        then;
        .f until; ,,

: lsr
    cwd; _lsr; ,,

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

:~ beforei 2 2
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
            yield;
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

:~ afteri 2 2
    drop;
    fn var; to-function; fn !;
    lst var; lst !;

    begin;
        lst @; shift;
        dup; is-null; if;
            leave;
        then;
        dup; fn @; funcall; if;
            yield;
            begin;
                lst @; shift;
                dup; is-null; if;
                    leave;
                then;
                yield;
                .f until;
            leave;
        else;
            drop;
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

: avg
    gen var; gen !;
    0 total var; total !;
    0 count var; count !;
    begin;
        gen @; shift; dup; is-null; if;
            drop;
            leave;
        then;
        total @; +; total !;
        count @; 1 +; count !;
        0 until;
    count @; 0 =; if;
        0
    else;
        total @; 0.0 +; count @; /;
    then;
    ,,

: mlist
    depth; 1 <; if;
        "mlist requires at least one argument" error;
    then;
    n var; n !;
    ()
    begin;
        n @; 0 <=; if;
            leave;
        then;
        n @; 1 -; n !;
        swap; push;
        .f until;
    reverse;
    ,,

: mset
    depth; 1 <; if;
        "mset requires at least one argument" error;
    then;
    mlist; s() swap; push for;
    ,,

: mhash
    depth; 1 <; if;
        "mhash requires at least one argument" error;
    then;
    2 *; mlist; lst var; lst !;
    h()
    begin;
        lst @; len; 0 =; if;
            leave;
        then;
        lst @; dup; shift; swap; shift; set;
        .f until;
    ,,

: shift-all
    depth; 1 <; if;
        "shift-all requires one argument" error;
    then;
    obj var; obj !;
    begin;
        obj @; shift; dup; is-null; if;
            drop;
            leave;
        then;
        .f until;
    ,,

: pforn
    pc var; pc !;
    fn var; fn !;
    [ fn @; funcall; .t ] pc @; pmapn; r; drop;
    ,,

: pfor 4 pforn; ,,

:~ pgrepn 3 3
    drop;
    pc var; pc !;
    fn var; fn !;
    res var;
    [ dup; clone; fn @; funcall; 2 mlist ] pc @; pmapn; res !;
    begin;
        res @; shift; dup; is-null; if;
            drop;
            leave;
        else;
            shift-all; if;
                yield;
            else;
                drop;
            then;
        then;
        .f until;
    ,,

: pgrep 4 pgrepn; ,,

: basename \/$ '' s; .*\/ '' s; ,,

: dirname
    dup; / =; if;
        return;
    then;
    \/$ '' s; "(.*)/" c;
    dup; len; 0 =; if;
        drop;
        .
    else;
        dup; 0 get; / =; if;
            0 get;
        else;
            1 get;
        then;
    then;
    ,,

: pse /proc/{} fmt; is-dir; ,,

: joinr
    sep var; sep !;
    gen var; gen !;
    "" res var; res !;
    begin;
        gen @; shift;
        dup; is-null; if;
            drop;
            res @;
            leave;
        else;
            res @; swap; sep @; ++; swap; ++; res !;
        then;
        0 until;
        ,,

: lr
    pst-index var; pst-index !;
    fn var; fn !;
    pre-index var; pre-index !;
    lst var; lst !;

    lst @; pre-index @; get;
    fn @; funcall;
    lst @; pst-index @; rot; set;
    ,,

: hr
    pst-index var; pst-index !;
    fn var; fn !;
    pre-index var; pre-index !;
    hsh var; hsh !;

    hsh @; pre-index @; get;
    fn @; funcall;
    hsh @; pst-index @; rot; set;
    ,,

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
    ["icmp_seq=(\d+) ttl=(\d+) time=(\d+\.?\d+?) (.*)" c;
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

# Common commands and aliases.
: vim depth; 0 =; if; vim exec; else; "vim {}" exec; then; drop; ,,
: ssh "ssh {}" fmtq; exec; drop; ,,
: scp swap; "scp {} {}" fmtq; exec; drop; ,,

: git.clone      "git clone {}"     fmtq; exec; drop; ,,
: git.add        "git add {}"       fmtq; exec; drop; ,,
: git.mv swap;   "git mv {} {}"     fmtq; exec; drop; ,,
: git.rm         "git rm {}"        fmtq; exec; drop; ,,
: git.diff swap; "git diff {} {}"   fmtq; exec; drop; ,,
: git.commit     "git commit -m {}" fmtq; exec; drop; ,,

: git.show
    depth; 0 =; if;
        "git show ." exec; drop;
    else;
        "git show {}" fmtq; exec; drop;
    then; ,,

: git.checkout
    depth; 0 =; if;
        "git checkout ." exec; drop;
    else;
        "git checkout {}" fmtq; exec; drop;
    then; ,,

: git.init
    depth; 0 =; if;
        "git init ." exec; drop;
    else;
        "git init {}" fmtq; exec; drop;
    then; ,,

: git.pull "git pull" exec; drop; ,,
: git.push "git push" exec; drop; ,,

:~ git.log 1 0
    0 =; if;
        .
    then;
    {git log --format=%H,"%P","%an","%ae",%at,"%cn","%ce",%ct,"%s" {}}/c;
    _rt.combined-to-lists;
    dup; len; 0 =; if;
        drop;
        gl var; gl !;
        begin;
            gl @; shift; dup; is-null; if;
                drop;
                leave;
            else;
                h() res var; res !;
                dup; '"$' m; not; if; '"' ++; then;
                chomp; , split; reverse; entry var; entry !;
                (hash parents
                 author-name author-email author-time
                 committer-name committer-email committer-time
                 subject)
                [entry @; pop; res @; rot; rot; set; drop] for;
                res @;
                parents [\s+ splitr] parents hr;
                author-time from-epoch author-time hr;
                committer-time from-epoch committer-time hr;
                yield;
            then;
            0 until;
    else;
        swap; drop; "" join; chomp; error;
    then;
        ,,

: git.status
    depth; 0 =; if;
        .
    then;
    {git status --porcelain {}};
    [h() res var; res !;
     "(.)(.)\s(.*)" c; dup; shift; drop;
     reverse;
     dup; pop; res @; swap; state1 swap; set; drop;
     dup; pop; res @; swap; state2 swap; set; drop;
          pop; dup; \s+ m; if;
              " ->" '' s;
              \s+ splitr; reverse;
              dup; pop; res @; swap; from-path swap; set; drop;
                   pop; res @; swap; path swap; set;
          else;
              res @; swap; path swap; set;
          then; ] map;
    ,,

: zathura     "zathura {}"     fmtq; exec; drop; ,,
: libreoffice "libreoffice {}" fmtq; exec; drop; ,,
: nano        "nano {}"        fmtq; exec; drop; ,,

: gr {grep -Zri "{}" .}; [0 chr; split; 1 chomp 1 lr] map; ,,

: _rt.combined-to-lists
    () stdout var; stdout !;
    () stderr var; stderr !;
    [dup; 1 get; swap; 0 get; 1 =; if;
        stdout @;
     else;
        stderr @;
     then;
     swap; push; drop] for;
    stdout @; stderr @; ,,

: _rt.docker.created-at-parse
    '\.\d+' '' s;
    ' [A-Z]*$' '' s;
    '%F %T %z' strptime;
    ,,
: _rt.docker.created-at-map
    [CreatedAt _rt.docker.created-at-parse CreatedAt hr] map;
    ,,
: docker.cp swap; "docker cp {} {}" fmtq; exec; drop; ,,
: docker.ps
    {docker ps --no-trunc --format '\{\{json .\}\}'}/c;
    _rt.combined-to-lists;
    dup; len; 0 =; if;
        drop;
        from-json map; _rt.docker.created-at-map;
    else;
        swap; drop; "" join; chomp; error;
    then;
    ,,
: docker.psa
    {docker ps -a --no-trunc --format '\{\{json .\}\}'}/c;
    _rt.combined-to-lists;
    dup; len; 0 =; if;
        drop;
        from-json map; _rt.docker.created-at-map;
    else;
        swap; drop; "" join; chomp; error;
    then;
    ,,
: docker.images
    {docker images --no-trunc --format '\{\{json .\}\}'}/c;
    _rt.combined-to-lists;
    dup; len; 0 =; if;
        drop;
        from-json map; _rt.docker.created-at-map;
    else;
        swap; drop; "" join; chomp; error;
    then;
    ,,
: docker.volume
    {docker volume ls --format '\{\{json .\}\}'}/c;
    _rt.combined-to-lists;
    dup; len; 0 =; if;
        drop;
        from-json map;
    else;
        swap; drop; "" join; chomp; error;
    then;
    ,,
: docker.rm    "docker rm    {}" fmtq; exec; drop; ,,
: docker.kill  "docker kill  {}" fmtq; exec; drop; ,,
: docker.rmi   "docker rmi   {}" fmtq; exec; drop; ,,
: docker.start "docker start {}" fmtq; exec; drop; ,,
: docker.stop  "docker stop  {}" fmtq; exec; drop; ,,

: docker.volume.rm "docker volume rm {}" fmtq; exec; drop; ,,
: docker.volume.prune "docker volume prune" exec; drop; ,,
: docker.volume.inspect
    {docker volume inspect {}}/c;
    _rt.combined-to-lists;
    dup; len; 0 =; if;
        drop;
        '' join; from-json;
        [CreatedAt ['%FT%T%z' strptime] CreatedAt hr] map;
    else;
        swap; drop; "" join; chomp; error;
    then;
    ,,

: docker.logs
    {docker logs {}}/c;
    _rt.combined-to-lists;
    dup; len; 0 =; if;
        drop;
    else;
        swap; drop; "" join; chomp; error;
    then;
    ,,

: docker.network
    {docker network ls --no-trunc --format '\{\{json .\}\}'}/c;
    _rt.combined-to-lists;
    dup; len; 0 =; if;
        drop;
        from-json map; _rt.docker.created-at-map;
    else;
        swap; drop; "" join; chomp; error;
    then;
    ,,

: docker.network.rm "docker network rm {}" fmtq; exec; drop; ,,
: docker.network.prune "docker network prune" exec; drop; ,,
: docker.network.inspect
    {docker network inspect {}}/c;
    _rt.combined-to-lists;
    dup; len; 0 =; if;
        drop;
        '' join; from-json;
        [Created ['\.\d+' '' s;
                  '%FT%T%z' strptime] Created hr] map;
    else;
        swap; drop; "" join; chomp; error;
    then;
    ,,

# Storage-related functions for libraries.
: make-xdg-env-var
    XDG_ swap; uc; ++; _HOME ++; ,,

xdg-types var; h(data   .local/share
                 config .config
                 state  .local/state
                 cache  .cache) xdg-types !;

: get-storage-dir
    type var; type !;
    lib var; lib !;

    xdg-types @; type @; get; dup; is-null; if;
        drop;
        "storage type is invalid" error;
    then;
    path-segment var; path-segment !;

    type @; make-xdg-env-var; getenv;
    dup; is-null; if;
        drop;
        HOME getenv;
        dup; is-null; if;
            drop;
            "no home directory set" error;
        then;
        / ++;
        path-segment @; ++;
    then;
    /cosh/ ++;
    lib @; ++;
    dup; "mkdir -p {}" exec; drop; ,,

: ip.is-unspecified
    ip.addr-int; 0 =;
    ,,

127.0.0.0/8 ips; _rt.ipv4.loopback var; _rt.ipv4.loopback !;
::/1 ip;         _rt.ipv6.loopback var; _rt.ipv6.loopback !;

: ip.is-loopback
    dup; ip.version; 4 =; if;
        ips; dup; _rt.ipv4.loopback @; isect; =;
    else;
        _rt.ipv6.loopback @; =;
    then;
    ,,

(10.0.0.0/8 172.16.0.0/12 192.168.0.0/16) ips;
_rt.ipv4.private var; _rt.ipv4.private !;

: ip.is-private
    dup; ip.version; 4 =; if;
        ips; dup; _rt.ipv4.private @; isect; =;
    else;
        .f
    then;
    ,,

fc00::/7 ips; _rt.ipv6.unique-local var; _rt.ipv6.unique-local !;

: ip.is-unique-local
    dup; ip.version; 6 =; if;
        ips; dup; _rt.ipv6.unique-local @; isect; =;
    else;
        .f
    then;
    ,,

169.254.0.0/16 ips; _rt.ipv4.link-local var; _rt.ipv4.link-local !;
fe80::/10      ips; _rt.ipv6.link-local var; _rt.ipv6.link-local !;

: ip.is-link-local
    dup; ip.version; 4 =; if;
        _rt.ipv4.link-local @;
    else;
        _rt.ipv6.link-local @;
    then;
    swap; ips; dup; rot; isect; =;
    ,,

224.0.0.0/4 ips; _rt.ipv4.multicast var; _rt.ipv4.multicast !;
ff00::/8    ips; _rt.ipv6.multicast var; _rt.ipv6.multicast !;

: ip.is-multicast
    dup; ip.version; 4 =; if;
        _rt.ipv4.multicast @;
    else;
        _rt.ipv6.multicast @;
    then;
    swap; ips; dup; rot; isect; =;
    ,,

(192.0.2.0/24 198.51.100.0/24 203.0.113.0/24) ips;
_rt.ipv4.documentation var; _rt.ipv4.documentation !;
2001:db8::/32 ips;
_rt.ipv6.documentation var; _rt.ipv6.documentation !;

: ip.is-documentation
    dup; ip.version; 4 =; if;
        _rt.ipv4.documentation @;
    else;
        _rt.ipv6.documentation @;
    then;
    swap; ips; dup; rot; isect; =;
    ,,

2000::/3 ips; _rt.ipv6.global var; _rt.ipv6.global !;

(0.0.0.0/8 240.0.0.0/4) ips;
_rt.ipv4.reserved var; _rt.ipv4.reserved !;
::/0 ips;
    _rt.ipv6.global @; diff;
    _rt.ipv6.unique-local @; diff;
    _rt.ipv6.link-local @; diff;
    _rt.ipv6.multicast @; diff;
_rt.ipv6.reserved var; _rt.ipv6.reserved !;

: ip.is-reserved
    dup; ip.version; 4 =; if;
        _rt.ipv4.reserved @;
    else;
        _rt.ipv6.reserved @;
    then;
    swap; ips; dup; rot; isect; =;
    ,,

0.0.0.0/0 ips;
    _rt.ipv4.reserved @; diff;
    _rt.ipv4.multicast @; diff;
    _rt.ipv4.documentation @; diff;
    _rt.ipv4.link-local @; diff;
    _rt.ipv4.loopback @; diff;
    _rt.ipv4.private @; diff;
_rt.ipv4.global var; _rt.ipv4.global !;

: ip.is-global
    dup; ip.version; 4 =; if;
        _rt.ipv4.global @;
    else;
        _rt.ipv6.global @;
    then;
    swap; ips; dup; rot; isect; =;
    ,,

: ephemeral-ports
    {sysctl net.ipv4.ip_local_port_range};
    shift;
    chomp;
    '.* = ' '' s;
    \t splitr;
    shift-all;
    ,,

: is-ephemeral-port
    port var; port !;
    ephemeral-ports;
    port @; >=;
    swap;
    port @; <=;
    and;
    ,,
