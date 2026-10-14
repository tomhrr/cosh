#!/bin/sh
i=1
while [ "$i" -le 25 ]; do
    echo "standard output $i"
    echo "standard error $i" >&2
    i=$((i + 1))
done
