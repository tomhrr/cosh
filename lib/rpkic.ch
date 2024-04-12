# rpki-client functions.

: rpkic.init
    dup; tals get; tals var; tals !;
    dup; name get; name var; name !;
         exec get; exec var; exec !;

    rpkic state get-storage-dir; / ++; name @; ++; dup; mkdir;
    state-dir var; state-dir !;

    state-dir @; /tals ++; dup; mkdir; tals-dir var; tals-dir !;
    tals @; [tals-dir @; cp] for;

    state-dir @; /cache ++; mkdir;
    state-dir @; /output ++; mkdir;
    exec @; state-dir @; /rpki-client ++; link;
    ,,

: rpkic.clear
    rpkic state get-storage-dir; / ++; swap; ++; rmrf;
    ,,

: rpkic.cd
    rpkic state get-storage-dir; / ++; swap; ++; cd;
    ,,

: rpkic.run
    cwd; cwd var; cwd !;
    rpkic state get-storage-dir; / ++; swap; ++; cd;
    tals ls; [-t{} fmt] map; ' ' join;
    {./rpki-client {} -d ./cache -c ./output}/c;
    r; dup; clone;
    [0 get; 1 =] grep; [1 get] map; last-stdout f>;
    [0 get; 2 =] grep; [1 get] map; last-stderr f>;
    cwd @; cd;
    ,,

: rpkic.last-stdout
    rpkic state get-storage-dir; / ++; swap; ++; /last-stdout ++; f<;
    ,,

: rpkic.last-stderr
    rpkic state get-storage-dir; / ++; swap; ++; /last-stderr ++; f<;
    ,,

: rpkic.vrps-raw
    rpkic state get-storage-dir; / ++; swap; ++; /output/csv ++; f<;
    dup; shift; drop;
    [chomp; , split] map;
    ,,

: rpkic.vrps
    rpkic.vrps-raw;
    [dup; 0 get; AS '' s;    0 swap; set;
     dup; 1 get; ip;         1 swap; set;
     dup; 4 get; from-epoch; 4 swap; set;] map;
    ,,

: rpkic.rov
    name var; name !;
    asn var; asn !;
    pfx var; ips; pfx !;
    pfx @; clone; 0 get; ip.len; pfl var; pfl !;

    name @;
    rpkic.vrps;
    [1 get; ips; dup; clone; pfx @; clone; union; =] grep; r;
    dup; len; 0 =; if;
        drop;
        unknown
    else;
        [0 get; asn @; =] grep;
        [2 get; pfl @; >=] grep;
        [1 get; ip.len; pfl @; <=] grep;
        len; 0 >; if;
            valid
        else;
            invalid
        then;
    then;
    ,,

: rpkic.file-raw
    cwd; cwd var; cwd !;
    name var; name !;
    rpkic state get-storage-dir; / ++; name @; ++; cd;
    tals ls; [-t{} fmt] map; ' ' join;
    {./rpki-client {} -d ./cache -f {} -j}/o;
    from-json;
    ,,

: rpkic.file
    cwd; cwd var; cwd !;
    name var; name !;
    rpkic state get-storage-dir; / ++; name @; ++; cd;
    tals ls; [-t{} fmt] map; ' ' join;
    {./rpki-client {} -d ./cache -f {} -j}/o;
    from-json;
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