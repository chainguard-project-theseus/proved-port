# check.awk -v SETS="A0,B0 A1,B1" reps.tsv: the reference findings
# (stats check). Port A's pull printed lines whose counters were empty
# or "<not counted>", repetitions with an empty output, and outputs that
# differ from the first of their (group, row, arm set) in file order;
# this prints the same, as stats's tab-separated findings. A counter is
# bad when it is not digits with an optional point and digits. Valid
# input only (no short lines or repeated keys). Unsorted; gen.sh sorts.
BEGIN {
	FS = OFS = "\t"
	ns = split(SETS, s, " ")
	for (i = 1; i <= ns; i++) { n = split(s[i], a, ","); for (j = 1; j <= n; j++) set[a[j]] = i }
}
{
	id = $1 OFS $2 OFS $3 OFS $4
	if ($5 !~ /^[0-9]+(\.[0-9]+)?$/) print "counter", id, "instructions", $5
	if ($6 !~ /^[0-9]+(\.[0-9]+)?$/) print "counter", id, "cycles", $6
	o = ""; for (i = 7; i <= NF; i++) o = o (i > 7 ? OFS : "") $i
	t = o; gsub(/\t/, "", t)
	if (t == "") print "empty-output", id
	if ($3 in set) {
		k = set[$3] SUBSEP $1 SUBSEP $2
		if (!(k in first)) { first[k] = o; fa[k] = $3; fr[k] = $4 }
		else if (first[k] != o) print "output-diff", id, fa[k], fr[k]
	}
}
