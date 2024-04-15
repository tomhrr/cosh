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
        dup; ^\s+$ m; not; if;
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
            leave;
        then;
        dup; ^\s+$ m; if;
            drop;
            leave;
        then;
        0 until;
    attrs @;
    ,,
