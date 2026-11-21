#!/bin/sh
# gen.sh: regenerate testdata/golden from testdata/reps.tsv and
# testdata/pull with the reference awk in this directory (not with
# stats), so the goldens do not come from the code under test. Run from
# kit/perf/stats. The comparisons are the ones CMP_CASES in main_test.go
# lists.
set -eu
cd testdata || exit 1
mkdir -p golden
awk -f ref/med.awk reps.tsv | LC_ALL=C sort > golden/med.tsv
for p in A0:C0 B0:C0 D0:C0 A0:W0 B1:C1 A1:C1 D1:C1 B1:W1 C1:W0; do
	t=${p%:*} r=${p#*:}
	awk -v T="$t" -v R="$r" -f ref/cmp.awk reps.tsv | LC_ALL=C sort > "golden/cmp-$t-$r.tsv"
done
awk -f ref/ahead.awk golden/med.tsv | LC_ALL=C sort > golden/ahead.tsv
# The pull's checks, on the fixtures in pull/. relabel keeps file order,
# so its golden is not sorted.
awk -v MAP=pull/map.tsv -f ref/relabel.awk pull/raw.tsv > golden/relabel.tsv
awk -v SETS="A0,B0,C0,D0 A1,B1,C1,D1" -f ref/check.awk pull/check.tsv | LC_ALL=C sort > golden/check.tsv
awk -v SETS="A1,B1,C1,D1 W1" -f ref/check.awk pull/check.tsv | LC_ALL=C sort > golden/check-w1.tsv
sh ref/md5.sh pull/before.log pull/after.log | LC_ALL=C sort > golden/md5.tsv
