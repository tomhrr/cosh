whois.ripe.net 43 socket; tsw varm; tsw !; tsr varm; tsr !;
tsw @; -k\n writeline;
tsr @; ["^%.*was served by the" m] before; r; drop;
tsw @; 193.0.11.51\n writeline;
tsr @; ["^%.*was served by the" m] before;
