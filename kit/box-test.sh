#!/bin/sh
# kit/box-test.sh: tests of kit/perf/box (the box-test check), with no
# box: stand-in perf and taskset scripts on PATH. The stand-in perf runs
# the command and writes a CSV with, for the k-th event of -e, the count
# ROUND*100+k, after decoy lines for the event without its modifier and
# with a letter after it, and before a second line for it with another
# count, except the events
# gone:u (no line) and nc:u ("<not counted>"); it exits with the
# command's status, as perf stat does. Every expected line below is
# written by hand. It checks:
#   - -n's order for three arms over four rounds against the rotation
#     worked out by hand, rows in file order within each arm;
#   - a run's raw lines: row, arm, round, the counts (exact event match,
#     first line), the output with tabs and newlines as spaces, ARM and
#     ROUND in the command's environment, and the output cut to -w;
#   - an event with no line is an empty field, "<not counted>" is kept,
#     and stats check flags both;
#   - -c runs the command under taskset -c CORE, and no -c runs no
#     taskset;
#   - a command that exits non-zero keeps its line, is named on stderr,
#     and makes the run exit 1 after the last run;
#   - a row that reads stdin leaves the later rows to run, and a run
#     whose perf writes no CSV (STUB_NOCSV=ARM:ROUND) gets empty counts,
#     not the previous run's;
#   - the raw lines read through stats relabel and stats check (clean);
#   - each refused setting exits 2 and writes no raw file;
#   - md5.sh: a before-only log that stats md5 refuses, an unchanged
#     bracket it passes, and a changed file it finds; a missing file, a directory and a file md5sum fails on (a
#     stand-in md5sum) each give exit 1, an md5sum error line that stats
#     md5 refuses, and the rest written; and its
#     refusals (exit 2, nothing written).
# Run it from the repository root. It needs go (to build stats).
set -eu
root=$(pwd)
rounds=$root/kit/perf/box/rounds.sh
md5=$root/kit/perf/box/md5.sh
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }
tab=$(printf '\t')

(cd kit/perf/stats && GOTOOLCHAIN=local GOFLAGS=-mod=mod go build -o "$tmp/stats" .)
stats=$tmp/stats

mkdir "$tmp/bin"
cat > "$tmp/bin/perf" <<'EOF'
#!/bin/sh
# Stand-in: perf stat -x, -e EVENTS -o CSV -- COMMAND...
[ "$1" = stat ] && [ "$2" = -x, ] && [ "$3" = -e ] && [ "$5" = -o ] && [ "$7" = -- ] || { echo "stand-in perf: bad args $*" >&2; exit 99; }
ev=$4 csv=$6
shift 7
rc=0
"$@" || rc=$?
# STUB_NOCSV=ARM:ROUND: that run writes no CSV at all.
[ "${STUB_NOCSV:-}" != "$ARM:$ROUND" ] || exit $rc
k=0
: > "$csv"
for e in $(echo "$ev" | tr , ' '); do
	k=$((k + 1))
	echo "999,,${e%%:*},1,100.00,," >> "$csv"
	echo "998,,${e}x,1,100.00,," >> "$csv"
	case $e in
	gone:u) ;;
	nc:u) echo "<not counted>,,$e,0,0.00,," >> "$csv" ;;
	*) echo "$((ROUND * 100 + k)),,$e,1,100.00,," >> "$csv"; echo "7,,$e,1,100.00,," >> "$csv" ;;
	esac
done
exit $rc
EOF
cat > "$tmp/bin/taskset" <<'EOF'
#!/bin/sh
# Stand-in: taskset -c CORE COMMAND...; logs CORE.
[ "$1" = -c ] || exit 98
echo "$2" >> "$TASKSET_LOG"
shift 2
exec "$@"
EOF
chmod +x "$tmp/bin/perf" "$tmp/bin/taskset"
PATH=$tmp/bin:$PATH
export TASKSET_LOG="$tmp/taskset.log"
: > "$TASKSET_LOG"

# The -n order: three arms, four rounds, two rows. Round r runs arm
# (i + r) mod 3 + 1 at position i: r=1 y z x, r=2 z x y, r=3 x y z,
# r=4 y z x.
printf 'x\ny\nz\n' > "$tmp/arms3"
printf 'p\techo p\nq\techo q\n' > "$tmp/rows2"
"$rounds" -n -r 4 "$tmp/arms3" "$tmp/rows2" > "$tmp/dry"
: > "$tmp/dry.want"
for ra in 1:y 1:z 1:x 2:z 2:x 2:y 3:x 3:y 3:z 4:y 4:z 4:x; do
	for row in p q; do printf '%s\t%s\t%s\n' "${ra%:*}" "${ra#*:}" "$row" >> "$tmp/dry.want"; done
done
cmp "$tmp/dry" "$tmp/dry.want" || { diff "$tmp/dry.want" "$tmp/dry" >&2; fail "-n order"; }
[ ! -s "$TASKSET_LOG" ] || fail "-n ran something"

# A run: two arms, two rounds (r=1 b a, r=2 a b).
printf 'a\nb\n' > "$tmp/arms2"
# The rows file in a quoted here-document, with | for the tab, so the
# command text is exactly what the shell sees.
tr '|' '\t' > "$tmp/rows" <<'EOF'
p|echo "out $ARM $ROUND"
q|printf 'x\ty\nz\n'
EOF
"$rounds" -o "$tmp/raw" -r 2 "$tmp/arms2" "$tmp/rows"
# The outputs' last newline is a space too (sp).
sp=' '
cat > "$tmp/raw.want" <<EOF
p${tab}b${tab}1${tab}101${tab}102${tab}out b 1${sp}
q${tab}b${tab}1${tab}101${tab}102${tab}x y z${sp}
p${tab}a${tab}1${tab}101${tab}102${tab}out a 1${sp}
q${tab}a${tab}1${tab}101${tab}102${tab}x y z${sp}
p${tab}a${tab}2${tab}201${tab}202${tab}out a 2${sp}
q${tab}a${tab}2${tab}201${tab}202${tab}x y z${sp}
p${tab}b${tab}2${tab}201${tab}202${tab}out b 2${sp}
q${tab}b${tab}2${tab}201${tab}202${tab}x y z${sp}
EOF
cmp "$tmp/raw" "$tmp/raw.want" || { diff "$tmp/raw.want" "$tmp/raw" >&2; fail "raw lines"; }
[ ! -s "$TASKSET_LOG" ] || fail "no -c ran taskset"
# It appends: a second run with -w 3 and other events adds lines. An
# empty or not-counted count is recorded as it is (stats check flags
# it), so the run itself passes.
printf 'b\n' > "$tmp/arm1"
"$rounds" -o "$tmp/raw" -r 1 -w 3 -e gone:u,cycles:u,nc:u "$tmp/arm1" "$tmp/rows" 2> "$tmp/err"
[ ! -s "$tmp/err" ] || fail "stderr on a passing run: $(cat "$tmp/err")"
[ "$(wc -l < "$tmp/raw")" -eq 10 ] || fail "append: $(wc -l < "$tmp/raw") lines"
tail -n 2 "$tmp/raw" > "$tmp/tail"
printf 'p\tb\t1\t\t102\t<not counted>\tout\nq\tb\t1\t\t102\t<not counted>\tx y\n' > "$tmp/tail.want"
cmp "$tmp/tail" "$tmp/tail.want" || { diff "$tmp/tail.want" "$tmp/tail" >&2; fail "empty, not counted and -w"; }

# With -g the lines are stats's layout: relabelled, check passes with
# no set; a set over both arms flags the p rows, whose output names the
# arm and round (file order b1, a1, a2, b2: three differ from b1); and
# an empty counter is flagged.
"$rounds" -o "$tmp/rawg" -g g1 -r 2 "$tmp/arms2" "$tmp/rows"
sed 's/^/g1\t/' "$tmp/raw.want" | cmp - "$tmp/rawg" || fail "-g lines: $(cat "$tmp/rawg")"
printf 'a\tA0\nb\tB0\n' > "$tmp/map"
"$stats" relabel -map "$tmp/map" "$tmp/rawg" > "$tmp/reps"
"$stats" check "$tmp/reps" || fail "stats check on a clean run"
if "$stats" check -same A0,B0 "$tmp/reps" > "$tmp/found" 2> /dev/null; then fail "stats check -same passed differing outputs"; fi
printf 'output-diff\tg1\tp\tA0\t1\tB0\t1\noutput-diff\tg1\tp\tA0\t2\tB0\t1\noutput-diff\tg1\tp\tB0\t2\tB0\t1\n' |
	cmp - "$tmp/found" || fail "stats check -same findings: $(cat "$tmp/found")"
"$rounds" -o "$tmp/rawe" -g g1 -r 1 -e gone:u,cycles:u "$tmp/arm1" "$tmp/rows"
if "$stats" check "$tmp/rawe" > "$tmp/found" 2> /dev/null; then fail "stats check passed an empty counter"; fi
printf 'counter\tg1\tp\tb\t1\tinstructions\t\ncounter\tg1\tq\tb\t1\tinstructions\t\n' |
	cmp - "$tmp/found" || fail "stats check counter findings: $(cat "$tmp/found")"

# -c: under taskset with the core, the same lines.
"$rounds" -o "$tmp/rawc" -r 2 -c 3 "$tmp/arms2" "$tmp/rows"
if [ "$(grep -cx 3 "$TASKSET_LOG")" -ne 8 ] || [ "$(wc -l < "$TASKSET_LOG")" -ne 8 ]; then
	fail "-c 3: taskset log $(cat "$TASKSET_LOG")"
fi
cmp "$tmp/raw.want" "$tmp/rawc" || fail "-c lines: $(cat "$tmp/rawc")"

# A failing command keeps its line and fails the run at the end, with
# and without -c; stderr is in the output.
printf 'f\techo bad; exit 3\ng\techo good; echo err >&2\n' > "$tmp/rowsf"
for c in '' 0-1; do
	rm -f "$tmp/rawf"
	if "$rounds" -o "$tmp/rawf" ${c:+-c "$c"} -r 1 "$tmp/arm1" "$tmp/rowsf" 2> "$tmp/err"; then fail "a failing command passed (-c '$c')"; fi
	printf 'f\tb\t1\t101\t102\tbad \ng\tb\t1\t101\t102\tgood err \n' | cmp - "$tmp/rawf" || fail "failing run lines (-c '$c'): $(cat "$tmp/rawf")"
	[ "$(grep -c . "$tmp/err")" -eq 2 ] || fail "failing run stderr lines (-c '$c'): $(cat "$tmp/err")"
	grep -q 'round 1 arm b row f exited 3' "$tmp/err" || fail "failing run stderr (-c '$c'): $(cat "$tmp/err")"
	grep -q '1 runs exited non-zero' "$tmp/err" || fail "failing run count (-c '$c'): $(cat "$tmp/err")"
done
[ "$(tail -n 2 "$TASKSET_LOG")" = "0-1
0-1" ] || fail "-c 0-1: taskset log $(cat "$TASKSET_LOG")"

# A row that reads stdin does not eat the rows after it (each run's
# stdin is /dev/null), and a run whose perf writes no CSV gets empty
# counts, not the previous run's: STUB_NOCSV=b:2 drops both round 2
# CSVs, and round 2's first run follows round 1's last.
printf 'p\tcat > /dev/null; echo p\nq\techo q\n' > "$tmp/rowsin"
for c in '' 0; do
	rm -f "$tmp/rawin"
	STUB_NOCSV=b:2 "$rounds" -o "$tmp/rawin" ${c:+-c "$c"} -r 2 "$tmp/arm1" "$tmp/rowsin"
	printf 'p\tb\t1\t101\t102\tp \nq\tb\t1\t101\t102\tq \np\tb\t2\t\t\tp \nq\tb\t2\t\t\tq \n' |
		cmp - "$tmp/rawin" || fail "stdin and no-CSV runs (-c '$c'): $(cat "$tmp/rawin")"
done

# Refused settings: exit 2, no raw file.
refuse() {
	want=$1
	shift
	rm -f "$tmp/rawr"
	rc=0
	"$rounds" "$@" > /dev/null 2> "$tmp/err" || rc=$?
	[ $rc -eq 2 ] || fail "refuse ($want): exit $rc"
	grep -q -- "$want" "$tmp/err" || fail "refuse ($want): stderr $(cat "$tmp/err")"
	[ ! -e "$tmp/rawr" ] || fail "refuse ($want): wrote a raw file"
}
printf 'a\n\nb\n' > "$tmp/arms-empty"
printf 'a b\n' > "$tmp/arms-space"
printf 'a\nb\na\n' > "$tmp/arms-twice"
: > "$tmp/none"
printf 'p echo\n' > "$tmp/rows-notab"
printf '\techo\n' > "$tmp/rows-norow"
printf 'p\t\n' > "$tmp/rows-nocmd"
printf 'p\techo\np\techo 2\n' > "$tmp/rows-twice"
refuse "-o is required" "$tmp/arms2" "$tmp/rows"
refuse "want ARMS and ROWS" -o "$tmp/rawr" "$tmp/arms2"
refuse "-r must be a number" -o "$tmp/rawr" -r x "$tmp/arms2" "$tmp/rows"
refuse "-r must be positive" -o "$tmp/rawr" -r 0 "$tmp/arms2" "$tmp/rows"
refuse "-w must be a number" -o "$tmp/rawr" -w -1 "$tmp/arms2" "$tmp/rows"
refuse "-w must be positive" -o "$tmp/rawr" -w 0 "$tmp/arms2" "$tmp/rows"
refuse "-c must be a CPU list" -o "$tmp/rawr" -c 'a' "$tmp/arms2" "$tmp/rows"
refuse "-g GROUP is one non-empty word" -o "$tmp/rawr" -g '' "$tmp/arms2" "$tmp/rows"
refuse "-g GROUP is one non-empty word" -o "$tmp/rawr" -g 'a b' "$tmp/arms2" "$tmp/rows"
refuse "empty event" -o "$tmp/rawr" -e 'instructions:u,' "$tmp/arms2" "$tmp/rows"
refuse "empty event" -o "$tmp/rawr" -e '' "$tmp/arms2" "$tmp/rows"
refuse "unknown option" -o "$tmp/rawr" -z "$tmp/arms2" "$tmp/rows"
refuse "rounds.sh: no file $tmp/nope" -o "$tmp/rawr" "$tmp/nope" "$tmp/rows"
refuse "rounds.sh: no file $tmp/nope" -o "$tmp/rawr" "$tmp/arms2" "$tmp/nope"
refuse "arms-empty:2: an arm is one non-empty word" -o "$tmp/rawr" "$tmp/arms-empty" "$tmp/rows"
refuse "arms-space:1: an arm is one non-empty word" -o "$tmp/rawr" "$tmp/arms-space" "$tmp/rows"
refuse "arms-twice:3: arm 'a' named twice" -o "$tmp/rawr" "$tmp/arms-twice" "$tmp/rows"
refuse "has no arms" -o "$tmp/rawr" "$tmp/none" "$tmp/rows"
refuse "rows-notab:1: want row<TAB>command" -o "$tmp/rawr" "$tmp/arms2" "$tmp/rows-notab"
refuse "rows-norow:1: empty row or command" -o "$tmp/rawr" "$tmp/arms2" "$tmp/rows-norow"
refuse "rows-nocmd:1: empty row or command" -o "$tmp/rawr" "$tmp/arms2" "$tmp/rows-nocmd"
refuse "rows-twice:2: row 'p' named twice" -o "$tmp/rawr" "$tmp/arms2" "$tmp/rows-twice"
refuse "has no rows" -o "$tmp/rawr" "$tmp/arms2" "$tmp/none"
[ "$(wc -l < "$TASKSET_LOG")" -eq 14 ] || fail "a refused run ran something"

# md5.sh: a bracket around a change.
mkdir "$tmp/in"
echo one > "$tmp/in/a"
echo two > "$tmp/in/b c"
printf '%s\n' "$tmp/in/a" "$tmp/in/b c" > "$tmp/list"
"$md5" before "$tmp/list" "$tmp/md5.log"
# A run that stopped after its before side: stats md5 refuses the log.
rc=0
"$stats" md5 "$tmp/md5.log" > /dev/null 2> "$tmp/err" || rc=$?
[ $rc -eq 1 ] || fail "stats md5 on a before-only bracket: exit $rc"
grep -q '1 "== LABEL" sides; want before-and-after pairs' "$tmp/err" || fail "stats md5 on a before-only bracket: $(cat "$tmp/err")"
# Both sides, nothing changed: clean.
cp "$tmp/md5.log" "$tmp/md5same.log"
"$md5" after "$tmp/list" "$tmp/md5same.log"
"$stats" md5 "$tmp/md5same.log" || fail "stats md5 on an unchanged bracket"
echo changed > "$tmp/in/b c"
"$md5" after "$tmp/list" "$tmp/md5.log"
[ "$(grep -c '^== ' "$tmp/md5.log")" -eq 2 ] || fail "md5 labels"
head -n 1 "$tmp/md5.log" | grep -qx '== before' || fail "md5 first label"
if "$stats" md5 "$tmp/md5.log" > "$tmp/found" 2> /dev/null; then fail "stats md5 missed a change"; fi
[ "$(cut -f1 "$tmp/found")" = "$tmp/in/b c" ] || fail "stats md5 found $(cat "$tmp/found")"
# A missing file, a directory, and a file md5sum fails on (the md5sum
# stand-in fails for STUB_MD5FAIL and runs the real one otherwise; a
# mode-000 file would not fail for root): each gets an md5sum error line
# in LOG and a stderr line, the files after them are still written, and
# md5.sh exits 1.
real=$(PATH=${PATH#"$tmp/bin:"} command -v md5sum)
cat > "$tmp/bin/md5sum" <<EOF
#!/bin/sh
if [ "\$1" = "\${STUB_MD5FAIL:-}" ]; then echo "md5sum: \$1: Permission denied" >&2; exit 1; fi
exec $real "\$@"
EOF
chmod +x "$tmp/bin/md5sum"
echo locked > "$tmp/in/locked"
printf '%s\n' "$tmp/in/gone" "$tmp/in" "$tmp/in/locked" "$tmp/in/a" > "$tmp/list2"
rc=0
STUB_MD5FAIL=$tmp/in/locked "$md5" after2 "$tmp/list2" "$tmp/md5b.log" 2> "$tmp/err" || rc=$?
[ $rc -eq 1 ] || fail "md5 failures: exit $rc"
{
	echo "== after2"
	echo "md5sum: $tmp/in/gone: missing or not a file"
	echo "md5sum: $tmp/in: missing or not a file"
	echo "md5sum: $tmp/in/locked: Permission denied"
} > "$tmp/md5b.want"
head -n 4 "$tmp/md5b.log" | cmp - "$tmp/md5b.want" || fail "md5 failure lines: $(cat "$tmp/md5b.log")"
[ "$(wc -l < "$tmp/md5b.log")" -eq 5 ] || fail "md5 failure log length: $(cat "$tmp/md5b.log")"
tail -n 1 "$tmp/md5b.log" | grep -q "  $tmp/in/a\$" || fail "md5 after the failures"
{
	echo "md5.sh: missing or not a file: $tmp/in/gone"
	echo "md5.sh: missing or not a file: $tmp/in"
	echo "md5.sh: md5sum failed on $tmp/in/locked"
} | cmp - "$tmp/err" || fail "md5 failure stderr: $(cat "$tmp/err")"
# stats md5 refuses the log for them.
rc=0
"$stats" md5 "$tmp/md5b.log" > /dev/null 2> "$tmp/err" || rc=$?
[ $rc -eq 1 ] || fail "stats md5 on a failed bracket's log: exit $rc"
grep -q "md5b.log:2: md5sum could not read a file" "$tmp/err" || fail "stats md5 on a failed bracket's log: $(cat "$tmp/err")"
# Only a missing file: exit 1 for it alone.
printf '%s\n' "$tmp/in/a" "$tmp/in/gone" > "$tmp/list4"
rc=0
"$md5" after4 "$tmp/list4" "$tmp/md5d.log" 2> /dev/null || rc=$?
[ $rc -eq 1 ] || fail "md5 one missing file: exit $rc"
# Only the locked file failing: still refused, at its own line.
printf '%s\n' "$tmp/in/a" "$tmp/in/locked" > "$tmp/list3"
rc=0
STUB_MD5FAIL=$tmp/in/locked "$md5" after3 "$tmp/list3" "$tmp/md5c.log" 2> /dev/null || rc=$?
[ $rc -eq 1 ] || fail "md5 one failure: exit $rc"
rc=0
"$stats" md5 "$tmp/md5c.log" > /dev/null 2> "$tmp/err" || rc=$?
[ $rc -eq 1 ] || fail "stats md5 on one failure: exit $rc"
grep -q "md5c.log:3: md5sum could not read a file" "$tmp/err" || fail "stats md5 on one failure: $(cat "$tmp/err")"
# Refusals write nothing.
md5refuse() {
	want=$1
	shift
	rm -f "$tmp/md5r.log"
	rc=0
	"$md5" "$@" 2> "$tmp/err" || rc=$?
	[ $rc -eq 2 ] || fail "md5 refuse ($want): exit $rc"
	grep -q -- "$want" "$tmp/err" || fail "md5 refuse ($want): stderr $(cat "$tmp/err")"
	[ ! -e "$tmp/md5r.log" ] || fail "md5 refuse ($want): wrote the log"
}
printf '%s\n\n' "$tmp/in/a" > "$tmp/list-empty"
md5refuse "usage" before "$tmp/list"
md5refuse "empty LABEL" "" "$tmp/list" "$tmp/md5r.log"
md5refuse "md5.sh: no file $tmp/nope" before "$tmp/nope" "$tmp/md5r.log"
md5refuse "md5.sh: no file $tmp/in" before "$tmp/in" "$tmp/md5r.log"
md5refuse "list-empty:2: empty line" before "$tmp/list-empty" "$tmp/md5r.log"
md5refuse "has no files" before "$tmp/none" "$tmp/md5r.log"
echo "box-test: ok"
