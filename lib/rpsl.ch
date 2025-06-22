# RPSL functions.

: rpsl.parse
    type var;
    attrs var; () attrs !;
    gen var; gen !;
    begin;
        gen @; shift;
        dup; is-null; if;
            return;
        then;
        dup; ^\s*$ m; not; if;
            leave;
        then;
        drop;
        0 until;
    begin;
        dup; "(.*?):\s+(.*)" c; dup;
        len; 0 =; if;
            drop;
            ^\s* '' s;
            attrs @; pop;
            dup; pop; \n ++; rot; ++; chomp; push;
            attrs @; swap; push;
            drop;
        else;
            (1 2) get; attrs @; swap; push; attrs !;
            drop;
        then;
        gen @; shift;
        dup; is-null; if;
            drop;
            leave;
        then;
        dup; ^\s*$ m; if;
            drop;
            leave;
        then;
        0 until;
    attrs @;
    ,,

: rpsl.str
    # Takes a list of [key, value] pairs and converts to RPSL text format
    input var; input !;
    result var; "" result !;
    input @; len; n var; n !;
    0 i var; i !;
    begin;
        i @; n @; =; if;
            leave;
        then;
        # Get the [key, value] pair at index i
        input @; i @; get;
        # Extract key and value
        dup; 0 get; str;
        ": " ++;
        swap; 1 get; str;
        ++;
        # Add to result
        result @; swap; ++;
        # Add newline if not last element
        i @; n @; 1 -; =; not; if;
            "\n" ++;
        then;
        result !;
        # Increment i
        i @; 1 +; i !;
        0 until;
    result @;
    ,,

:~ rpsl.parsem 1 1
    drop;
    [^#|% m; not] grep;
    gen var; gen !;
    begin;
        gen @;
        rpsl.parse;
        dup; is-null; if;
            drop;
            leave;
        else;
            yield;
        then;
        0 until;
        ,,

h(afrinic whois.afrinic.net
  altdb   whois.altdb.net
  apnic   whois.apnic.net
  arin    rr.arin.net
  bell    whois.in.bell.ca
  bboi    irr.bboi.net
  canarie whois.canarie.ca
  idnic   irr.idnic.net
  jpirr   jpirr.nic.ad.jp
  lacnic  irr.lacnic.net
  level3  rr.level3.net
  nestegg whois.nestegg.net
  nttcom  rr.ntt.net
  panix   rrdb.access.net
  radb    whois.radb.net
  reach   rr.telstraglobal.net
  ripe    whois.ripe.net
  tc      irr.bgp.net.br)
servers var; servers !;

: rpsl.servers servers @; clone; ,,

: rpsl.query-raw
    dup; servers @; swap; get; dup; is-null; if;
        drop;
    else;
        swap;
        drop;
    then;
    43 socket; tsw var; tsw !; tsr var; tsr !;
    dup; is-shiftable; not; if;
        1 mlist;
    then;
    [str; chomp; \n ++; tsw @; swap; writeline] for;
    null tsw !;
    tsr @;
    rpsl.parsem;
    ,,

: rpsl.query
    rpsl.query-raw;
    [[row var; row !;
      row @@; shift-all;
      swap;
      dup; last-modified =; if;
          drop;
          %FT%TZ strptime;
          row @; swap; 1 swap; set; row !;
      else; dup; ^inetnum$|^inet6num$|^route$|^route6$ m; if;
          drop;
          ip;
          row @; swap; 1 swap; set; row !;
      else;
          drop;
          drop;
      then; then;
      row @] map] map;
    ,,
