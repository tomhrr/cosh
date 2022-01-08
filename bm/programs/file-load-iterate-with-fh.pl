#!/usr/bin/perl

use warnings;
use strict;

open my $fh, '<', 'bm-file.txt' or die $!;
while (my $line = <$fh>) {
}

1;
