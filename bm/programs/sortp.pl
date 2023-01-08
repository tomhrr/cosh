#!/usr/bin/perl

use warnings;
use strict;

my @data;
open my $fh, '<', 'bm-file' or die $!;
while (my $line = <$fh>) {
    push @data, $line;
}

@data = sort { $a <=> $b } @data;

1;
