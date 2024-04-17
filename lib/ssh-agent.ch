# ssh-agent functions.

HOME getenv; "/.ssh/environment" ++;
ssh-env-path var; ssh-env-path !;

: ssh-agent.get-vars
    ssh-env-path @; f<;
    [^echo m; not] grep;
    ["(.*?)=(.*?);\s*" c; (1 2) get] map;
    [0 get; is-null; not] grep;
    ,,

: ssh-agent.set-vars
    ssh-agent.get-vars;
    [shift-all; setenv] for;
    ,,

: ssh-agent.start
    {/usr/bin/ssh-agent}; [^echo #echo s] map; ssh-env-path @; f>;
    ssh-env-path @; 600 oct; chmod;
    ssh-agent.set-vars;
    /usr/bin/ssh-add exec;
    ,,

: ssh-agent.start-if-required
    ssh-env-path @; is-file; if;
        ssh-agent.set-vars;
        ps;
        [pid get; SSH_AGENT_PID getenv; =] grep;
        [cmd get; ssh-agent$ m] grep;
        len; 0 =; if;
            ssh-agent.start;
        then;
    else;
        ssh-agent.start;
    then;
    ,,
