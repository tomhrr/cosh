#!/usr/bin/perl

use warnings;
use strict;

my @data;
open my $fh, '<', 'bm-file' or die $!;
while (my $line = <$fh>) {
    push @data, $line;
}

@data = sort @data;

1;
