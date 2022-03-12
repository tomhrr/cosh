#!/usr/bin/perl

use warnings;
use strict;

for (1..1000000) {
    my @list;
    push @list, 1;
    push @list, 1;
    push @list, 1;
    push @list, 1;
    push @list, 1;
    pop @list;
    pop @list;
    pop @list;
    pop @list;
    pop @list;
}
