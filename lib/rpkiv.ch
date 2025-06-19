# RPKI validator functions.

: rpkiv._gsd
    rpkiv state get-storage-dir; / ++;
    ,,

: rpkiv.init
    dup; tals get; tals var; tals !;
    dup; type get; type var; type !;
    dup; name get; name var; name !;
         exec get; exec var; exec !;

    rpkiv._gsd; name @; ++; dup; mkdir;
    state-dir var; state-dir !;

    state-dir @; /tals ++; dup; mkdir; tals-dir var; tals-dir !;
    tals @; [tals-dir @; cp] for;

    state-dir @; /cache ++; mkdir;
    state-dir @; /output ++; mkdir;
    type @; state-dir @; /type ++; f>;
    exec @; state-dir @; /rpki-validator ++; link;
    ,,

: rpkiv.instances
    rpkiv._gsd; gsd var; gsd !;
    gsd @; ls; is-dir grep; [gsd @; '' s] map;
    ,,

: rpkiv.type
    rpkiv._gsd; swap; ++; /type ++; f<; shift; chomp;
    ,,

: rpkiv.clear
    rpkiv._gsd; swap; ++; rmrf;
    ,,

: rpkiv.cd
    rpkiv._gsd; swap; ++; cd;
    ,,

: _rpkiv.run.rpki-client
    cwd; cwd var; cwd !;
    rpkiv._gsd; swap; ++; cd;
    tals ls; [-t{} fmt] map; ' ' join;

    "./rpki-validator {} -d ./cache -c ./output" fmtq; '"/g' '' s;
    cmd/c;
    r; dup; clone;
    [0 get; 1 =] grep; [1 get] map; last-stdout f>;
    [0 get; 2 =] grep; [1 get] map; last-stderr f>;
    cwd @; cd;
    ,,

: _rpkiv.run.routinator
    cwd; cwd var; cwd !;
    rpkiv._gsd; swap; ++; cd;

    "./rpki-validator --no-rir-tals --extra-tals-dir ./tals -r ./cache vrps -o output/csv"
    cmd/c;
    r; dup; clone;
    [0 get; 1 =] grep; [1 get] map; last-stdout f>;
    [0 get; 2 =] grep; [1 get] map; last-stderr f>;
    cwd @; cd;
    ,,

: _rpkiv.run.fort
    cwd; cwd var; cwd !;
    rpkiv._gsd; swap; ++; cd;

    "./rpki-validator --tal=./tals --local-repository=./cache --mode=standalone --daemon=false --validation-log.enabled=true --validation-log.output=console --output.roa=output/csv --output.format=csv"
    cmd/c;
    r; dup; clone;
    [0 get; 1 =] grep; [1 get] map; last-stdout f>;
    [0 get; 2 =] grep; [1 get] map; last-stderr f>;
    cwd @; cd;
    ,,

: rpkiv.run
    name var; name !;
    cwd; cwd var; cwd !;
    rpkiv._gsd; name @; ++; /type ++; f<; shift; chomp;
    dup; rpki-client =; if;
        drop;
        cwd @; cd;
        name @; _rpkiv.run.rpki-client;
    else; dup; routinator =; if;
        drop;
        cwd @; cd;
        name @; _rpkiv.run.routinator;
    else; fort =; if;
        cwd @; cd;
        name @; _rpkiv.run.fort;
    else;
        "invalid RPKI validator type" error;
    then; then; then;
    ,,

: rpkiv.last-stdout
    rpkiv._gsd; swap; ++; /last-stdout ++; f<;
    ,,

: rpkiv.last-stderr
    rpkiv._gsd; swap; ++; /last-stderr ++; f<;
    ,,

: rpkiv.vrps-raw
    rpkiv._gsd; swap; ++; /output/csv ++; f<;
    dup; shift; drop;
    [chomp; , split] map;
    ,,

: _rpkiv.vrps.common
    [dup; 0 get; AS '' s;    0 swap; set;
     dup; 1 get; ip;         1 swap; set] map;
    ,,

: _rpkiv.vrps.rpki-client
    _rpkiv.vrps.common;
    [dup; 4 get; from-epoch; 4 swap; set;] map;
    ,,

: rpkiv.vrps
    name var; name !;
    name @; rpkiv.type; rpki-client =; if;
        name @; rpkiv.vrps-raw;
        _rpkiv.vrps.rpki-client;
    else;
        name @; rpkiv.vrps-raw;
        _rpkiv.vrps.common;
    then;
    ,,

# : _rpkiv.vrps.group-by-asn
#     h() vrp-index var; vrp-index !;
#     begin;
#         shift; dup; is-null; if;
#             drop; leave;
#         then;
#         dup; 0 get; asn-key var; asn-key !;
#         vrp-index @; asn-key @; get; dup; is-null; if;
#             drop; () asn-list var; asn-list !;
#         else;
#             asn-list var; asn-list !;
#         then;
#         asn-list @; rot; push; asn-list !;
#         vrp-index @; asn-key @; asn-list @; set; vrp-index !;
#         .f until;
#     vrp-index @;
#     ,,

# : rpkiv.vrps-indexed
#     name var; name !;
#     name @; rpkiv.vrps;
#     _rpkiv.vrps.group-by-asn;
#     ,,

: rpkiv.rov
    name var; name !;
    asn var; asn !;
    pfx var; ips; pfx !;
    pfx @; 0 get; ip.len; pfl var; pfl !;

    name @;
    rpkiv.vrps;
    # Filter by ASN first (most selective filter)
    [0 get; asn @; =] grep;
    dup; len; 0 =; if;
        drop;
        unknown
    else;
        # Now filter by prefix intersection on smaller subset
        [1 get; ips; dup; pfx @; union; =] grep; r;
        dup; len; 0 =; if;
            drop;
            unknown
        else;
            # Filter by prefix length constraints
            [2 get; pfl @; >=] grep;
            [1 get; ip.len; pfl @; <=] grep;
            len; 0 >; if;
                valid
            else;
                invalid
            then;
        then;
    then;
    ,,

: rpkiv.file-raw
    cwd; cwd var; cwd !;
    name var; name !;
    rpkiv._gsd; name @; ++; cd;
    type f<; shift; chomp; rpki-client =; not; if;
        "rpkiv.file-raw only available for rpki-client" error;
    then;
    tals ls; [-t{} fmt] map; ' ' join;
    {./rpki-client {} -d ./cache -f {} -j}/o;
    from-json;
    ,,

: _rpkiv.file-annotate
    dup; valid_since exists; if;
        dup; valid_since get; from-epoch; valid_since swap; set;
    then;
    dup; valid_until exists; if;
        dup; valid_until get; from-epoch; valid_until swap; set;
    then;
    dup; expires exists; if;
        dup; expires get; from-epoch; expires swap; set;
    then;
    dup; signing_time exists; if;
        dup; signing_time get; from-epoch; signing_time swap; set;
    then;
    dup; vrps exists; if;
        dup; vrps get;
             [dup; prefix get; ip; prefix swap; set] map;
        vrps swap; set;
    then;
    dup; subordinate_resources exists; if;
        asres var;
        ipres var;
        dup; subordinate_resources get;
        dup; clone; [ip_prefix exists] grep;
                    [ip_prefix get] map; ips; ipres !;
        dup; clone; [ip_range exists]  grep;
                    [ip_range get; (min max) get; shift-all; - swap; ++; ++] map;
                    ips;
             ipres @; union; ipres !;
        dup; clone; [asid exists] grep;
                    [asid get] map; r; asres !;
             clone; [asrange exists] grep;
                    [asrange get; (min max) get] map; r;
             asres @; swap; ++; asres !;
        h() asns asres @; r; set;
            ips  ipres @; r; set;
        subordinate_resources swap; set;
    then;
    dup; revoked_certs exists; if;
        dup; revoked_certs get;
             [dup; date get; "%a %d %b %Y %T %z" strptime; date swap; set] map;
        revoked_certs swap; set;
    then;
    ,,

:~ rpkiv.files 2 2
    drop;
    cwd; cwd var; cwd !;
    name var; name !;
    files var; files !;
    rpkiv._gsd; name @; ++; rsv var; rsv !;
    rsv @; cd;
    type f<; shift; chomp; rpki-client =; not; if;
        "rpkiv.files only available for rpki-client" error;
    then;
    tals ls; [-t{} fmt] map; ' ' join; talstr var; talstr !;
    begin;
        files @; 100 take; r;
        dup; len; 0 =; if;
            drop;
            leave;
        else;
            dup; len; range; [drop; "-f {}"] map; ' ' joinr;
            talstr @;
            rsv @; cd;
            "./rpki-validator {} -d ./cache {} -j" fmt; cmdstr var; cmdstr !;
            shift-all; cmdstr @; fmt;
            cmd; res var; res !;
            cwd @; cd;
            begin;
                res @;
                ["^}\n" m] before; r;
                dup; len; 0 =; if;
                    drop;
                    leave;
                else;
                    "}\n" push; '' join;
                    from-json;
                    _rpkiv.file-annotate;
                    yield;
                then;
                0 until;
        then;
        0 until;
    cwd @; cd;
    ,,

: rpkiv.file
    swap; 1 mlist; swap; rpkiv.files;
    ,,
