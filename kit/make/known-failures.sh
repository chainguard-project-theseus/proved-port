#!/bin/sh
# known-failures.sh LOG LIST [EXTRA]: after a failed test run, pass only
# if every failure in LOG is listed in LIST (`make known-failures`,
# proved-port.mk), which the theseus-proved-port package installs in
# /usr/share/proved-port/make/. challoc's ci/known-failures.sh, in POSIX
# sh, with its port-specific failure source moved out to EXTRA.
#
# A failure is a libtest line `test NAME ... FAILED` at the start of a
# line, listed by NAME: everything between `test ` and ` ... FAILED`,
# blanks included (api::tests::some_test; a should_panic test's
# `tests::boom - should panic`; a doctest's `src/lib.rs - parse (line
# 12)`), or a name in the last `failures:` block before a `test result:
# FAILED.` line (four blanks in), which also holds a test whose FAILED
# landed on a line of its own; a block that names fewer tests than its
# result line counts as failed fails the run. Or a line the port's
# command EXTRA prints. EXTRA, when given, runs as
# `sh EXTRA LOG` and prints one failure key per line for the port's own
# sources (challoc: its cgroup programs' `OUTCOME` lines, as
# `cgroup-tests KIND ALLOC`); a line of blanks alone is no key, and a
# key's leading and trailing blanks are dropped. Then:
#
#   - every failure listed passes (exit 0), each as a warning line, so a
#     known failure stays visible on a green run;
#   - an unlisted failure fails (exit 1), naming it;
#   - no failure found at all fails: the run failed in a build or
#     another step (see the log);
#   - a missing LOG or LIST, a LOG or LIST it cannot read, a tool that
#     fails on one, or an EXTRA that fails, fails.
#
# LIST's lines are entries; `#` starts a comment, and blank lines and
# blanks around an entry count for nothing. An entry ending in ` @GLOB`
# applies only when the cell's name in CI_CELL (challoc:
# hosted-amd64-privileged) matches the shell pattern GLOB; with CI_CELL
# unset or empty only a GLOB that matches the empty name applies (`*`).
# Names compare bytewise (LC_ALL=C).
#
# The caller runs it only after the test step failed; it does not check
# that the tests ran or finished. A test group that never started, and
# a test binary that died partway (a signal or an abort: libtest prints
# no FAILED line for the test that crashed, and cargo only `process
# didn't exit successfully`), leave no failure it reads, so beside a
# listed failure the run passes. A port checks that every group
# finished itself, as challoc's ci/test-cell.sh does (or prints a key
# for such a crash from EXTRA).
#
# Two more things a list cannot tell apart, as with challoc's script: a
# key is the test's name alone, not its test binary (two integration
# test files with `fn roundtrip()` both print `test roundtrip ...
# FAILED`; listing one lists both), and a listed test that fails
# for another reason than the one its entry records passes as known. An
# entry in scope for the cell that did not fail is printed as a notice
# (the exit status is the failures' alone), so a fixed test's entry can
# be dropped before it hides a later failure. The log must be
# uncoloured: a `test result:` line that is neither `ok.` nor `FAILED.`
# (libtest's colours, from `-- --color=always` or a terminal) fails the
# run, since its FAILED lines cannot be read either.
#
# Exit 0: every failure is listed; 1: an unlisted failure, none found,
# or an input that is missing or cannot be read; 2: usage.
set -eu
[ $# -eq 2 ] || [ $# -eq 3 ] || { echo "usage: known-failures.sh LOG LIST [EXTRA]" >&2; exit 2; }
log=$1
list=$2
extra=${3:-}
cell=${CI_CELL:-}
LC_ALL=C
export LC_ALL
[ -f "$log" ] || { echo "error: $log missing: the test step did not run"; exit 1; }
[ -f "$list" ] || { echo "error: no known-failures list $list"; exit 1; }

known=$(mktemp)
failed=$(mktemp)
hits=$(mktemp)
keys=$(mktemp)
blocks=$(mktemp)
trap 'rm -f "$known" "$failed" "$hits" "$keys" "$blocks"' EXIT
# No step is on the left of a pipe, where its status would be lost
# (there is no pipefail): each writes a file and set -e or its own
# check sees it fail. grep exits 1 for no match and 2 or more when it
# cannot read: only the first is no failures.

# The entries: comments and trailing blanks stripped, blank lines
# dropped, cell-scoped lines reduced to their name when CI_CELL matches
# and dropped when it does not.
sed -e 's/#.*//' -e 's/[[:space:]]*$//' "$list" > "$hits" || { echo "error: cannot read $list"; exit 1; }
# `|| [ -n "$line" ]`: a last line with no newline is an entry too.
while read -r line || [ -n "$line" ]; do
  [ -n "$line" ] || continue
  case $line in
    *" @"*)
      # shellcheck disable=SC2254 # the pattern is the point
      case $cell in ${line##* @}) printf '%s\n' "${line% @*}" ;; esac ;;
    *) printf '%s\n' "$line" ;;
  esac
done < "$hits" > "$known"

# The failures: libtest's, then EXTRA's.
rc=0
grep -aE '^test .+ \.\.\. FAILED' "$log" > "$hits" || rc=$?
[ "$rc" -le 1 ] || { echo "error: cannot read $log (grep exit $rc)"; exit 1; }
# The name: from after `test ` to the first ` ... FAILED` (words after
# it, from another test's output on the same line, are dropped).
awk '{ s = substr($0, 6); print substr(s, 1, index(s, " ... FAILED") - 1) }' "$hits" > "$keys" || { echo "error: awk failed on the FAILED lines of $log"; exit 1; }
# libtest also names each failed test, four blanks in, in the last
# `failures:` block before its `test result: FAILED. N passed; M
# failed` line. A FAILED that is not on its test's line (one thread:
# libtest writes `test NAME ... ` before the test runs, and output it
# does not capture, from C code or a child, can land between) is read
# from there. Each block's names are counted against its M: a run with
# fewer names than failures (`short`) fails. The block gives a test's
# bare name, without the mode its FAILED line adds (`tests::boom`, where
# the FAILED line has `tests::boom - should panic`): a block name that a
# FAILED-line key of the same test binary already gives, as itself or
# followed by ` - `, adds nothing, so a list keeps the whole-name keys.
# A binary's section runs from the end of the one before it (its `test
# result:` line) to its own `test result:` line, so another binary's
# key covers nothing here. A test named only by its block (its FAILED on
# a line of its own) is keyed by its bare name.
awk '
  /^test .+ \.\.\. FAILED/ {
    s = substr($0, 6); have[substr(s, 1, index(s, " ... FAILED") - 1)] = 1
  }
  function covered(s,   k) {
    if (s in have) return 1
    for (k in have) if (index(k, s " - ") == 1) return 1
    return 0
  }
  /^failures:$/ { n = 0; inblk = 1; next }
  inblk && /^    [^ ]/ { name[++n] = substr($0, 5); next }
  /^test result: FAILED\. / {
    m = $0; sub(/.* passed; /, "", m); sub(/ failed.*/, "", m)
    for (i = 1; i <= n; i++) if (!covered(name[i])) print name[i]
    if (m + 0 > n) short += m - n
    n = 0; inblk = 0
  }
  /^test result: / { for (k in have) delete have[k] }
  # A result line that is neither `ok.` nor `FAILED.` (coloured output:
  # `-- --color=always`, or a terminal) cannot be read, nor can its
  # FAILED lines: counted, so the run fails rather than pass on them.
  /^test result: / && !/^test result: (ok|FAILED)\.( |$)/ { bad++ }
  END { print "short " short + 0 " " bad + 0 > "/dev/stderr" }
' "$log" > "$blocks" 2> "$hits" || { echo "error: awk failed on the failures blocks of $log"; exit 1; }
cat "$blocks" >> "$keys"
read -r _ short bad < "$hits"
if [ -n "$extra" ]; then
  rc=0
  sh "$extra" "$log" >> "$keys" || rc=$?
  [ "$rc" -eq 0 ] || { echo "error: $extra failed on $log (exit $rc)"; exit 1; }
fi
sort -u "$keys" > "$hits"
awk 'NF > 0' "$hits" > "$failed" || { echo "error: awk failed on the failures of $log"; exit 1; }

if [ ! -s "$failed" ]; then
  echo "error: the run failed, but no test did (a build or other step failed; see the log)"
  exit 1
fi
rc=0
# read drops a key's leading and trailing blanks, as it does an entry's.
# The keys, so read, are kept for the notices below.
: > "$hits"
while read -r t; do
  printf '%s\n' "$t" >> "$hits"
  if grep -qxF -- "$t" "$known"; then
    echo "warning: known failure ($list): $t"
  else
    echo "error: failed: $t"
    rc=1
  fi
done < "$failed"
# An entry in scope for this cell that did not fail is only reported (a
# notice): its test passes now, or did not run; drop it once it is fixed,
# so a later failure of that test is not taken as known.
sort -u "$known" > "$keys"
while read -r e; do
  grep -qxF -- "$e" "$hits" || echo "notice: listed but did not fail, drop it from $list once fixed: $e"
done < "$keys"
if [ "$bad" != 0 ]; then
  echo "error: $log has $bad test result lines that are neither ok. nor FAILED. (coloured output?); run the tests without --color"
  rc=1
fi
if [ "$short" != 0 ]; then
  echo "error: $log's test result lines count $short more failures than its failures blocks name (a test the run does not name failed)"
  rc=1
fi
exit $rc
