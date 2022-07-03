#!/usr/bin/perl

use warnings;
use strict;

open my $fh, '<', 'bm-file' or die $!;
open my $fh2, '>', 'bm-file-copy' or die $!;
while (my $line = <$fh>) {
    print $fh2 $line;
}
close $fh;
close $fh2;

1;
