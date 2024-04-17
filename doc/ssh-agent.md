## ssh-agent

Utility functions for initialising `ssh-agent`.

### Usage

    # In cosh.conf:
    ssh-agent import;
    ssh-agent.start-if-required;

### Functions

 - `ssh-agent.start-if-required`
    - If `ssh-agent` is not running, it is started, and `ssh-add` is
      called so as to make identities available to it.
