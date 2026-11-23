#!/bin/sh
# md5.sh LOG...: the reference md5 check (stats md5). Port A's pull
# kept the md5sum lines of its logs and printed each file with more than
# one md5; this prints the file, a tab and its md5s, sorted. Unsorted
# across files; gen.sh sorts.
cat "$@" | grep -E '^[0-9a-f]{32}[ 	]' | awk '{print $1, $2}' | sort -u |
	awk '{s[$2] = n[$2]++ ? s[$2] " " $1 : $1} END {for (f in n) if (n[f] > 1) print f "\t" s[f]}'
