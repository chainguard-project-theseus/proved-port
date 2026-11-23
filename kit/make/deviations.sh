#!/bin/sh
# deviations.sh [--update] RESULTS LIST LEDGER: the deviations ratchet
# (`make deviations`, proved-port.mk), which the theseus-proved-port
# package installs in /usr/share/proved-port/make/. The per-block
# comparison of the PCRE2 port's conformance check, in POSIX sh: the
# port's run (upstream's test data through upstream's driver, split
# into blocks) stays in the port and writes RESULTS; this holds RESULTS
# to LIST.
#
# RESULTS has one line per key of the run, `PASS KEY` or `FAIL KEY`
# (the PCRE2 port: `FAIL testinput2:1234`, a pattern block at a line of
# a test input). A key is one word and appears once. A line of any
# other form, a blank line, a key twice, or no line at all fails: the
# run did not write what it should. Nothing is checked, or updated, then.
# That RESULTS holds the whole run is the port's to prove: this cannot
# tell a run cut short (its first block written, the rest lost) from a
# small one, so the port's run must fail on its own when it does not
# finish (check each step's status, no pipeline that drops it), and the
# port calls this only after it succeeded.
#
# LIST is the deviations list: `KEY ABI-nnn: reason` lines, one per
# failing key the port accepts, in any order. A line whose first
# non-blank is `#` is a comment; blank lines count for nothing. LEDGER
# is the port's ABI ledger, where each deliberate difference is an
# entry: a directory of `ABI-nnn.md` entry files (the kit template's
# docs/abi/) or one file of entries (the PCRE2 port's
# docs/abi-changes.md). ledger-active.sh, installed beside this script,
# reads it: its header says what an entry is, when one is active, and
# what makes a ledger malformed. A malformed ledger fails before
# anything is checked or updated.
#
# Strict both ways (exit 1):
#
#   - a failing key that is not listed is a regression;
#   - a listed key that passes, or is not a key of this run, is stale;
#   - a listed key without a reason, listed twice, whose reason does not
#     start with `ABI-nnn:` or says nothing after it, or whose ABI-nnn
#     is not an active entry of LEDGER, is unexplained.
#
# --update first drops from LIST the listed keys that pass or are not
# keys of this run, keeping comments, blank lines and every other line
# as they are, then checks. It never adds a line, so a regression still
# fails.
#
# Each kind prints at most 50 lines. With DEVIATIONS_OUT set to a
# directory, the full lists are written there, one key per line, in
# RESULTS order (new-failures, fixed) or LIST order (stale,
# unexplained, the latter as `KEY: why`), so the port can show what a
# new failure's block printed (the PCRE2 port shows the first five).
#
# Exit 0: the failing keys are exactly the listed ones, each explained;
# 1: they are not, or RESULTS, LIST or LEDGER is missing, unreadable or
# malformed; 2: usage.
set -eu
usage() { echo "usage: deviations.sh [--update] RESULTS LIST LEDGER" >&2; exit 2; }
update=no
if [ "${1:-}" = --update ]; then update=yes; shift; fi
[ $# -eq 3 ] || usage
res=$1
list=$2
ledger=$3
LC_ALL=C
export LC_ALL
[ -f "$res" ] || { echo "error: no results file $res"; exit 1; }
[ -f "$list" ] || { echo "error: no deviations list $list"; exit 1; }
[ -e "$ledger" ] || { echo "error: no ABI ledger $ledger"; exit 1; }
# The names awk and the tools read: awk takes a relative operand with
# `=` in it (deviations=amd64.txt) as a variable assignment, not a file,
# and would read nothing; a ./ prefix keeps each a file. Messages keep
# the names as given.
op() { case $1 in /*) printf '%s' "$1" ;; *) printf './%s' "$1" ;; esac; }
res_f=$(op "$res")
list_f=$(op "$list")

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
out=${DEVIATIONS_OUT:-$tmp/out}
mkdir -p "$out" || { echo "error: cannot make $out"; exit 1; }
# Its files are awk operands too (show below): o=1/new-failures would
# be an assignment.
out=$(op "$out")

# RESULTS: every line is PASS or FAIL and one key, each key once.
RES=$res awk '
  BEGIN { r = ENVIRON["RES"] }
  NF == 2 && ($1 == "PASS" || $1 == "FAIL") {
    if ($2 in seen) { print "error: " r ":" FNR ": " $2 " appears twice (first at line " seen[$2] ")"; bad = 1; next }
    seen[$2] = FNR; next
  }
  NF == 0 { print "error: " r ":" FNR ": a blank line, not `PASS KEY` or `FAIL KEY`"; bad = 1; next }
  { print "error: " r ":" FNR ": not a `PASS KEY` or `FAIL KEY` line: " $0; bad = 1 }
  END {
    if (NR == 0) { print "error: " r " is empty: the run wrote no results"; bad = 1 }
    exit bad
  }' "$res_f" > "$tmp/malformed" || {
  rc=$?
  if [ -s "$tmp/malformed" ]; then head -50 "$tmp/malformed"; else echo "error: cannot read $res (awk exit $rc)"; fi
  exit 1
}

# The ledger's active entries, one ID per line, read by ledger-active.sh
# beside this script (its header says how it reads a ledger and what
# makes one malformed).
here=$(dirname "$0")
if sh "$here/ledger-active.sh" "$ledger" > "$tmp/active"; then :; else
  rc=$?
  if [ -s "$tmp/active" ]; then cat "$tmp/active"; else echo "error: $here/ledger-active.sh failed on $ledger (exit $rc)"; fi
  [ "$rc" -ne 1 ] || echo "error: the ABI ledger $ledger is malformed; nothing was checked or updated"
  exit 1
fi

# Inputs read; only now does --update touch LIST.
if [ "$update" = yes ]; then
  # Keep comments, blank lines and the listed keys that still fail.
  awk 'FILENAME == ARGV[1] { if ($1 == "FAIL") failing[$2] = 1; next }
       NF == 0 || $1 ~ /^#/ || ($1 in failing) { print }' \
    "$res_f" "$list_f" > "$tmp/updated" || { echo "error: cannot read $list"; exit 1; }
  # cat, not mv: LIST keeps its mode and stays the file it was.
  cat "$tmp/updated" > "$list" || { echo "error: cannot write $list"; exit 1; }
  echo "updated $list"
fi

# The listed lines, comments and blank lines dropped.
awk 'NF > 0 && $1 !~ /^#/' "$list_f" > "$tmp/listed" || { echo "error: cannot read $list"; exit 1; }

# Compare: new failures and fixed keys in RESULTS order, stale ones in
# LIST order. Each list starts empty: DEVIATIONS_OUT may hold an
# earlier run's.
: > "$out/new-failures"
: > "$out/fixed"
: > "$out/stale"
# Paths through ENVIRON, not awk -v, which would read backslashes as
# escapes. FILENAME, not NR == FNR: the list may be empty.
OUT=$out awk '
  BEGIN { new = ENVIRON["OUT"] "/new-failures"; fixed = ENVIRON["OUT"] "/fixed"; stale = ENVIRON["OUT"] "/stale" }
  FILENAME == ARGV[1] { listed[$1] = 1; order[++n] = $1; next }
  $1 == "FAIL" && !($2 in listed) { print $2 > new }
  $1 == "PASS" && ($2 in listed) { print $2 > fixed }
  { seen[$2] = 1 }
  END {
    for (i = 1; i <= n; i++) if (!(order[i] in seen) && !(order[i] in done)) { print order[i] > stale; done[order[i]] = 1 }
  }' "$tmp/listed" "$res_f" || { echo "error: awk failed comparing $res with $list"; exit 1; }

# Every listed key needs a reason, listed once, that starts with an
# active entry of LEDGER (`ABI-nnn:`) and says something after it.
LEDGER=$ledger awk '
  FILENAME == ARGV[1] { active[$1] = 1; next }
  NF < 2 { print $1 ": no reason"; next }
  seen[$1]++ { print $1 ": listed twice"; next }
  $2 !~ /^ABI-[0-9]+:$/ { print $1 ": the reason does not start with ABI-nnn:"; next }
  NF < 3 { print $1 ": the reason says nothing after " $2; next }
  {
    id = $2; sub(/:$/, "", id)
    if (!(id in active)) print $1 ": " id " is not an active entry of " ENVIRON["LEDGER"]
  }' "$tmp/active" "$tmp/listed" > "$out/unexplained" \
  || { echo "error: awk failed on $list"; exit 1; }

total=$(wc -l < "$res" | tr -d ' ')
# Counted as the results check reads a line (blank- or tab-separated).
nfail=$(awk '$1 == "FAIL" { n++ } END { print n + 0 }' "$res_f") || { echo "error: cannot read $res"; exit 1; }
echo "total: $((total - nfail))/$total pass"

# show FILE BEFORE AFTER: FILE's first 50 lines as `FAIL: BEFORE<line>AFTER`,
# then a count of the rest. awk, not sed: LIST's name may hold any byte.
show() {
  BEFORE=$2 AFTER=$3 awk '
    NR <= 50 { print "FAIL: " ENVIRON["BEFORE"] $0 ENVIRON["AFTER"] }
    END { if (NR > 50) print "FAIL: ... and " NR - 50 " more" }' "$1" \
    || { echo "error: awk failed on $1"; exit 1; }
}
bad=no
show "$out/fixed" "listed in $list but passes: " ""
show "$out/stale" "listed in $list but not a key of this run: " ""
show "$out/unexplained" "unexplained deviation: " ""
nfixed=$(wc -l < "$out/fixed" | tr -d ' ')
nstale=$(wc -l < "$out/stale" | tr -d ' ')
if [ "$nfixed" != 0 ] || [ "$nstale" != 0 ]; then
  echo "FAIL: $nfixed listed keys pass and $nstale are not keys of this run;" \
    "run the ratchet with --update to drop them from $list"
  bad=yes
fi
[ ! -s "$out/unexplained" ] || bad=yes
nnew=$(wc -l < "$out/new-failures" | tr -d ' ')
if [ "$nnew" != 0 ]; then
  show "$out/new-failures" "" " fails and is not in $list"
  echo "FAIL: $nnew failing keys are not in $list (a regression: fix it, do not list it)"
  bad=yes
fi
if [ "$bad" = yes ]; then exit 1; fi
echo "PASS: the failing keys are exactly the $(wc -l < "$tmp/listed" | tr -d ' ') listed in $list, each with its reason"
