# NRTM functions.

rpsl import;

h(afrinic h(host whois.afrinic.net
            port 43003
            type ripe)
  altdb   h(host whois.altdb.net
            port 43
            type irrd)
  apnic   h(host nrtm.apnic.net
            port 43003
            type ripe)
  arin    h(host rr.arin.net
            port 43
            type irrd)
  bell    h(host whois.in.bell.ca
            port 43
            type irrd)
  bboi    h(host irr.bboi.net
            port 43
            type irrd)
  canarie h(host whois.canarie.ca
            port 43
            type irrd)
  idnic   h(host irr.idnic.net
            port 43
            type irrd)
  jpirr   h(host jpirr.nic.ad.jp
            port 43
            type irrd)
  lacnic  h(host irr.lacnic.net
            port 43
            type irrd)
  level3  h(host rr.level3.net
            port 43
            type irrd)
  nestegg h(host whois.nestegg.net
            port 43
            type irrd)
  nttcom  h(host rr.ntt.net
            port 43
            type irrd)
  panix   h(host rrdb.access.net
            port 43
            type irrd)
  radb    h(host nrtm.radb.net
            port 43
            type irrd)
  reach   h(host rr.telstraglobal.net
            port 43
            type irrd)
  ripe    h(host whois.ripe.net
            port 4444
            type ripe)
  tc      h(host irr.bgp.net.br
            port 43
            type irrd))
nrtm._servers var; nrtm._servers !;

: nrtm.servers nrtm._servers @; clone; ,,

: nrtm._sources-ripe
    chomp map;
    [^% m; not] grep;
    ["" =; not] grep;
    [":3:" ":" s;
     ":" splitr;
     2 [- split] 2 lr;
     3 take; shift-all;
     serials var!;
     status var!;
     name var!;
     h(first     serials @; 0 get;
       last      serials @; 1 get;
       name      name @;
       available status @; X =; not;)] map;
    ,,

: nrtm._sources-irrd
    chomp map;
    [^% m; not] grep;
    ["" =; not] grep;
    [^A\d*$ m; not] grep;
    [C =; not] grep;
    [":" splitr;
     2 [- split] 2 lr;
     3 take; shift-all;
     serials var!;
     status var!;
     name var!;
     h(first     serials @; 0 get;
       last      serials @; 1 get;
       name      name @;
       available status @; X =; not;)] map;
    ,,

: nrtm.sources
    nrtm._servers @; swap; get; dup; is-null; if;
        "invalid NRTM server name" error;
    else;
    r; (host port type) get; shift-all;
    irrd =; is-irrd var!;
    port var!;
    server var!;
    is-irrd @; if;
        ("!j-*\n")
    else;
        ("-q sources\n")
    then;
    server @; port @; nc;
    is-irrd @; if;
        nrtm._sources-irrd;
    else;
        nrtm._sources-ripe;
    then;
    ,,

:~ nrtm._query-gen 1 1
    drop;
    gen var!;
    begin;
        gen @; 
        ["^(ADD|DEL) \d+" m] beforei; r; pop;
        dup; is-null; if;
            drop;
            leave;
        then;
        chomp; dup; "" =; if;
            drop;
            leave;
        then;
        "^(ADD|DEL) (\d+)" c; shift-all;
        serial var!;
        type var!;
        drop;
        gen @; rpsl.parse; ro var!;
        ro @; is-null; if;
            leave;
        then;
        h(serial serial @;
          type   type   @;
          object ro @;)
        yield;
        0 until;
    ,,

: nrtm.query
    nrtm._servers @; swap; get; dup; is-null; if;
        "invalid NRTM server name" error;
    else;
    r; (host port type) get; shift-all;
    irrd =; is-irrd var!;
    port var!;
    server var!;
    swap; rot;
    "-g {}:3:{}-{}\n" fmt; 1 mlist;
    server @; port @; nc;
    gen var!;
    gen @; ["^%START " m] beforei; r; prelude var!;
    prelude @; pop; chomp;
    "START Version: \d+ .+? \d+-\d+$" m; if;
        gen @;
        nrtm._query-gen;
    else;
        prelude @; [^% m] first;
        dup; is-null; if;
            drop;
            "did not see response header" error;
        then;
        chomp;
        "did not see response header: got '{}' instead" fmt; error;
    then;
    ,,

