# RDAP client functions.

: rdap.fetch-file
    filename var; filename !;
    rdap state get-storage-dir;
    state-dir var; state-dir !;

    https://data.iana.org/rdap/ filename @; ++; http.get;
    to-json; 1 mlist;
    state-dir @; / ++; filename @; ++; f>; .t
    ,,

: rdap.file-is-current
    filename var; filename !;
    rdap state get-storage-dir; cd;

    filename @; f<; from-json; publication get;
    %FT%TZ strptime;

    h(url    https://data.iana.org/rdap/ filename @; ++;
      method head
      raw    .t) http;
    headers get; last-modified get;
    "%a, %d %b %Y %T %Z" strptime;

    =;
    ,,

: rdap.fetch-file-if-not-current
    dup; rdap.file-is-current; not; if;
        rdap.fetch-file;
    else;
        drop; .t
    then;
    ,,

: rdap.init
    (dns.json ipv4.json ipv6.json asn.json object-tags.json)
        rdap.fetch-file pfor; ,,

: rdap.refetch
    (dns.json ipv4.json ipv6.json asn.json object-tags.json)
        rdap.fetch-file-if-not-current pfor; ,,

: rdap.ip
    swap; iparg var; iparg !;
    rdap state get-storage-dir; / ++; swap; ++; f<; from-json;
    services get;
    [ 0 get; ips; iparg @; ips; isect; len; 0 =; not ] first;
    dup; is-null; if;
    else;
        1 get; 0 get; ip/ ++; iparg @; ++; http.get;
    then;
    ,,

: rdap.ipv4 ipv4.json rdap.ip; ,,
: rdap.ipv6 ipv6.json rdap.ip; ,,

: rdap.in-range
    needle var; needle !;
    dup; - m; if;
        - splitr; shift-all;
        needle @; >=;
        swap;
        needle @; <=;
        and;
    else;
        needle @; =;
    then;
    ,,

: rdap.asn
    asnarg var; asnarg !;
    rdap state get-storage-dir; /asn.json ++; f<; from-json;
    services get;
    [ 0 get; clone; [ asnarg @; rdap.in-range ] first; is-null; not ] first;
    dup; is-null; if;
    else;
        1 get; 0 get; autnum/ ++; asnarg @; ++; http.get;
    then;
    ,,

: rdap.domain-match
    needle var; needle !;
    domain var; domain !;
    needle @; domain @; "$" ++; m; if;
        domain @; "\./g" c; len; 1 +;
    else;
        0
    then;
    ,,

: rdap.domain-service-match
    dmnarg var; dmnarg !;
    service var; service !;
    service @; 1 get; 0 get; server var; server !;
    service @; 0 get; clone; [ dmnarg @; rdap.domain-match ] map;
    [ 0 > ] grep;
    [ service @; 1 get; 0 get; 2 mlist ] map; r;
    ,,

: rdap.domain
    dmnarg var; dmnarg !;
    rdap state get-storage-dir; /dns.json ++; f<; from-json;
    services get;
    [ dmnarg @; rdap.domain-service-match ] map;
    flatten;
    [ [ 0 get ] 2 apply; <=> ] sortp; r; pop;
    dup; is-null; if;
    else;
        1 get; domain/ ++; dmnarg @; ++; http.get;
    then;
    ,,

: rdap.object-tag-match
    entarg var; entarg !;
    service var; service !;
    service @; 1 get;
    [ - swap; ++; "$" ++; entarg @; uc; swap; m ] grep; len; 0 >;
    ,,

: rdap.entity
    entarg var; entarg !;
    rdap state get-storage-dir; /object-tags.json ++; f<; from-json;
    services get;
    [ entarg @; rdap.object-tag-match ] first;
    dup; is-null; if;
    else;
        2 get; 0 get; entity/ ++; entarg @; ++; http.get;
    then;
    ,,

: rdap
    dup; "^[0-9]+$" m; if;
        rdap.asn;
    else;
    dup; "^[0-9\./]+$" m; if;
        rdap.ipv4;
    else;
    dup; "^[0-9a-fA-F:/]+$" m; if;
        rdap.ipv6;
    else;
    dup; "-[a-zA-Z]+$" m; if;
        # TLD labels do not contain dashes.
        rdap.entity;
    else;
        rdap.domain;
    then; then; then; then;
    ,,
