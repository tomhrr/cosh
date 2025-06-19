# Test the exact issue described by the user

# Copy the RPSL functions locally
: rpsl.parse
    type var;
    attrs var; () attrs !;
    gen var; gen !;
    
    # Skip initial blank lines
    begin;
        gen @; shift;
        dup; is-null; if;
            return;  # No more input, return null
        then;
        dup; ^\s+$ m; not; if;
            leave;  # Found non-blank line, exit loop
        then;
        drop;  # Drop blank line and continue
        0 until;
    
    # Now we have a non-blank line on the stack
    # Process field:value pairs until blank line or end of input
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
        
        # Try to get the next line
        gen @; shift;
        dup; is-null; if;
            drop;  # End of input, exit loop
            leave;
        then;
        dup; ^\s+$ m; if;
            drop;  # Blank line, exit loop  
            leave;
        then;
        # Continue with this line in next iteration
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