# RDAP client functions.

: rdap.fetch-file
    filename var; filename !;
    rdap state get-storage-dir;
    state-dir var; state-dir !;

    https://data.iana.org/rdap/ filename @; ++; http.get;
    to-json; 1 mlist;
    state-dir @; / ++; filename @; ++; f>; ,,

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

    =; ,,

: rdap.fetch-file-if-not-current
    dup; rdap.file-is-current; not; if;
        rdap.fetch-file;
    else;
        drop;
    then; ,,

: rdap.init
    (dns.json ipv4.json ipv6.json asn.json)
        rdap.fetch-file pmap; r; drop; ,,

: rdap.refetch
    (dns.json ipv4.json ipv6.json asn.json)
        [ rdap.fetch-file-if-not-current; 1 ] pmap; r; ,,

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
        rdap.domain;
    then; then; then; ,,
