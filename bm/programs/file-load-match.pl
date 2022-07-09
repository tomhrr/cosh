#!/usr/bin/perl

use warnings;
use strict;

open my $fh, '<', 'bm-file' or die $!;
while (my $line = <$fh>) {
    my $res = ($line =~ /e/);
}

1;
