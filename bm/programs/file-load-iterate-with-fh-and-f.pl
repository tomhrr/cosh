#!/usr/bin/perl

use warnings;
use strict;

sub dd { 1; }
open my $fh, '<', 'bm-file.txt' or die $!;
while (my $line = <$fh>) {
    dd($line);
}

1;
