# Test with ORIGINAL rpsl.parse function to reproduce the issue

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

# Test the exact case described by user
"Testing input WITH blank line:" println;
test_input_with_blank.txt f<; rpsl.parsem; shift-all;

"Testing input WITHOUT blank line:" println;
test_input_without_blank.txt f<; rpsl.parsem; shift-all;