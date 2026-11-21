#!/bin/sh
# gen.sh: regenerate testdata/golden from testdata/reps.tsv with the
# reference awk in this directory (not with stats), so the goldens do
# not come from the code under test. Run from kit/perf/stats. The
# comparisons are the ones CMP_CASES in main_test.go lists.
set -eu
cd testdata || exit 1
mkdir -p golden
awk -f ref/med.awk reps.tsv | LC_ALL=C sort > golden/med.tsv
for p in A0:C0 B0:C0 D0:C0 A0:W0 B1:C1 A1:C1 D1:C1 B1:W1 C1:W0; do
	t=${p%:*} r=${p#*:}
	awk -v T="$t" -v R="$r" -f ref/cmp.awk reps.tsv | LC_ALL=C sort > "golden/cmp-$t-$r.tsv"
done
awk -f ref/ahead.awk golden/med.tsv | LC_ALL=C sort > golden/ahead.tsv
