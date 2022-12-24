#!/usr/bin/perl

use warnings;
use strict;

for (1..3) {
    print STDOUT "standard output $_\n";
    print STDERR "standard error $_\n";
}

1;
