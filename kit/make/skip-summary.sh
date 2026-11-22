#!/bin/sh
# skip-summary.sh LOG MANIFEST: the skips a test run printed against the
# expected-skips manifest (`make skip-summary`, proved-port.mk), which
# the theseus-proved-port package installs in /usr/share/proved-port/make/.
# challoc's ci/skip-summary.sh, in POSIX sh.
#
# A port whose kernel- or privilege-gated tests print `SKIP <key>
# <reason>` and pass when they cannot run (challoc's
# CHALLOC_TEST_ALLOW_SKIP=1) keeps those skips from hiding lost coverage
# with this script. Each skip is keyed by its first word after SKIP: the
# test (`SKIP c_mseal ...`) or a program's claim name (`SKIP thp/auto
# ...`). Only the first word: parallel tests write to the same stream and
# another test's output can land inside a line after it (a lone `test` or
# `ok` key, from a libtest line cut in two the same way, is dropped).
# The reverse cut counts too: a `SKIP` after a libtest `test NAME ... `
# prefix on the same line (`test foo ... SKIP c_new reason`, the skip
# landing before foo's result) is a skip, keyed the same way (#195).
# Then:
#
#   - a key not in MANIFEST fails (exit 1): coverage was lost;
#   - a MANIFEST key that did not appear is only reported (a notice):
#     coverage was gained, drop the line from MANIFEST;
#   - a missing MANIFEST fails and prints the one to commit;
#   - a missing or empty LOG fails: the test step did not run, or
#     printed nothing;
#   - a LOG or MANIFEST it cannot read, or a tool that fails on it,
#     fails.
#
# What it does not check, as challoc's script does not: that the tests
# ran or passed (a cut-short log with no SKIP lines passes; the caller
# fails on the test step's own status, as challoc's ci/cell.sh does);
# anything but key names (no libtest totals; an `ignored` test is no
# skip); a skip whose `SKIP` starts neither its line nor follows a
# libtest `test NAME ... ` prefix (other output landed before it on the
# same line), which is not counted; and a key that stopped appearing, which is only a notice, so leaving it in
# MANIFEST lets that test go back to skipping later without a failure.
#
# A markdown table of the skips goes to stdout, headed with MANIFEST's
# name less its .txt. MANIFEST's lines are keys; `#` starts a comment,
# and blank lines and trailing blanks count for nothing (a line with
# leading blanks is a key that never matches, reported as no longer
# skipped without them, as challoc's script does). CI_SKIP_UPDATE=1
# writes the observed keys to MANIFEST (creating its directory) and
# passes, to start a new manifest. Keys sort and compare bytewise
# (LC_ALL=C).
#
# Exit 0: the skips match MANIFEST (or CI_SKIP_UPDATE=1 wrote it); 1: a
# new skip, no MANIFEST, no LOG or an empty one, or one it cannot read;
# 2: usage.
set -eu
[ $# -eq 2 ] || { echo "usage: skip-summary.sh LOG MANIFEST" >&2; exit 2; }
log=$1
manifest=$2
LC_ALL=C
export LC_ALL
[ -f "$log" ] || { echo "error: $log missing: the test step did not run"; exit 1; }

[ -s "$log" ] || { echo "error: $log is empty: the test step printed nothing"; exit 1; }

seen=$(mktemp)
want=$(mktemp)
hits=$(mktemp)
keys=$(mktemp)
trap 'rm -f "$seen" "$want" "$hits" "$keys"' EXIT
# No step is on the left of a pipe, where its status would be lost
# (there is no pipefail): each writes a file and set -e or its own
# check sees it fail. grep exits 1 for no match and 2 or more when it
# cannot read the log: only the first is no skips.
rc=0
grep -aE '^(test .+ \.\.\. )?SKIP ' "$log" > "$hits" || rc=$?
[ "$rc" -le 1 ] || { echo "error: cannot read $log (grep exit $rc)"; exit 1; }
# A SKIP after a libtest `test NAME ... ` prefix (the skip landed before
# that test's result) is read from the first ` ... SKIP ` on.
awk '{
  s = $0
  if (substr(s, 1, 5) != "SKIP ") s = substr(s, index(s, " ... SKIP ") + 5)
  split(s, w, " "); k = w[2]
  if (k != "" && k != "test" && k != "ok") print k
}' "$hits" > "$keys" \
  || { echo "error: awk failed on the SKIP lines of $log"; exit 1; }
sort -u "$keys" > "$seen"
if [ -f "$manifest" ]; then
  sed -e 's/#.*//' -e 's/[[:space:]]*$//' "$manifest" > "$hits" || { echo "error: cannot read $manifest"; exit 1; }
  awk 'length($0) > 0' "$hits" > "$keys" || { echo "error: awk failed on $manifest"; exit 1; }
  sort -u "$keys" > "$want"
fi

new=$(comm -23 "$seen" "$want")
gone=$(comm -13 "$seen" "$want")
echo "### Skips: $(basename "$manifest" .txt)"
echo
echo "$(wc -l < "$seen" | tr -d ' ') distinct skips."
echo
if [ -s "$seen" ] || [ -n "$gone" ]; then
  echo '| skip | status |'
  echo '|---|---|'
  while IFS= read -r k; do
    if grep -qxF -- "$k" "$want"; then s=expected; else s='**NEW**'; fi
    echo "| \`$k\` | $s |"
  done < "$seen"
  if [ -n "$gone" ]; then
    printf '%s\n' "$gone" | while read -r k; do echo "| \`$k\` | no longer skipped |"; done
  fi
fi

if [ "${CI_SKIP_UPDATE:-}" = 1 ]; then
  mkdir -p "$(dirname "$manifest")"
  cp "$seen" "$manifest"
  echo "wrote $manifest"
  exit 0
fi
if [ ! -f "$manifest" ]; then
  echo "error: no skip manifest $manifest; commit this one:"
  cat "$seen"
  exit 1
fi
if [ -n "$gone" ]; then
  printf '%s\n' "$gone" | while read -r k; do echo "notice: no longer skipped, drop it from $manifest: $k"; done
fi
if [ -n "$new" ]; then
  printf '%s\n' "$new" | while IFS= read -r k; do echo "error: new skip (lost coverage) not in $manifest: $k"; done
  echo "observed skip keys (the manifest if all are expected):"
  cat "$seen"
  exit 1
fi
echo "skips match $manifest"
