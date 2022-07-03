#!/usr/bin/perl

use warnings;
use strict;

my $i = 0;
my $gen = sub {
    if ($i < 10000) {
        return $i++;
    } else {
        return undef;
    }
};
while (defined (my $n = $gen->())) {
    print "$n\n";
}
