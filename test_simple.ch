# Simple test for RPSL parsing functionality
# Copy the functions inline to avoid import issues

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

# Test data generators for both cases

# Input WITH trailing blank line (should work)
:~ input_with_blank 0 0
    field1: value1 yield;
    field2: value2 yield;
    "" yield;
    field3: value3 yield;
    field4: value4 yield;
    "" yield;
    ,,

# Input WITHOUT trailing blank line (currently causes error)
:~ input_without_blank 0 0
    field1: value1 yield;
    field2: value2 yield;
    "" yield;
    field3: value3 yield;
    field4: value4 yield;
    ,,

"Testing WITH blank line:" println;
input_with_blank; rpsl.parsem; shift-all;

println;

"Testing WITHOUT blank line:" println;
input_without_blank; rpsl.parsem; shift-all;