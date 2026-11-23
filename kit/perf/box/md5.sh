#!/bin/sh
# kit/perf/box/md5.sh LABEL LIST LOG: one side of a bench run's md5
# bracket (PLAN step 6). A port's box run took the md5s of its inputs
# (libraries, scripts, workloads) before the first measured row and
# after the last, into one log; `stats md5 LOG` then names any file
# whose md5 changed.
#
# Appends "== LABEL" to LOG, then md5sum's line for each file LIST names
# (one path per line), in LIST order. A path that is missing or not a
# regular file gets an "md5sum: PATH: missing or not a file" line in
# LOG; a file md5sum fails on (unreadable, say) gets md5sum's own error
# line in LOG. Each also gets a line on stderr and makes md5.sh exit 1
# once the rest are written. stats md5 refuses a log with an "md5sum: "
# line, so a later read of the bracket reports them too. Refused (exit
# 2, writing nothing): a wrong operand count, an empty LABEL, a LIST
# that is not a file, an empty LIST line, and a LIST with no lines.
set -eu
if [ $# -ne 3 ]; then
	echo "usage: md5.sh LABEL LIST LOG" >&2
	exit 2
fi
label=$1 list=$2 log=$3
if [ -z "$label" ]; then
	echo "md5.sh: empty LABEL" >&2
	exit 2
fi
if [ ! -f "$list" ]; then
	echo "md5.sh: no file $list" >&2
	exit 2
fi
n=0
while IFS= read -r p || [ -n "$p" ]; do
	n=$((n + 1))
	if [ -z "$p" ]; then
		echo "md5.sh: $list:$n: empty line" >&2
		exit 2
	fi
done < "$list"
if [ "$n" -eq 0 ]; then
	echo "md5.sh: $list has no files" >&2
	exit 2
fi
echo "== $label" >> "$log"
failed=0
while IFS= read -r p || [ -n "$p" ]; do
	if [ ! -f "$p" ]; then
		echo "md5sum: $p: missing or not a file" >> "$log"
		echo "md5.sh: missing or not a file: $p" >&2
		failed=$((failed + 1))
	elif ! md5sum "$p" >> "$log" 2>&1; then
		echo "md5.sh: md5sum failed on $p" >&2
		failed=$((failed + 1))
	fi
done < "$list"
if [ "$failed" -gt 0 ]; then
	exit 1
fi
