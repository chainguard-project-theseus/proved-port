#!/bin/sh
# kit/skip-summary-test.sh: tests of kit/make/skip-summary.sh (the
# skip-summary-test check). Each case writes a test log and a manifest,
# runs the script and checks its exit status and its output, which the
# cases below spell out by hand from challoc's ci/skip-summary.sh's
# format (none is taken from a run of the script under test): the
# table, the verdict lines, a new skip, a gone one, a missing manifest,
# a missing or empty log, a log or manifest it cannot read or a tool
# that fails on one, CI_SKIP_UPDATE=1, the manifest's comments and
# blanks, the keys it drops, a key that starts with `-`, the bytewise
# order, its temporary files, and usage. Run it from the repository root. It needs only a POSIX shell
# and BusyBox's tools.
set -eu

script=$(pwd)/kit/make/skip-summary.sh
[ -f "$script" ] || { echo "FAIL: run it from the repository root (no kit/make/skip-summary.sh)" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"
unset CI_SKIP_UPDATE || true

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# run ARGS...: the script with ARGS; its output (stdout and stderr) in
# out, its status in rc.
run() {
  if sh "$script" "$@" > out 2>&1; then rc=0; else rc=$?; fi
}

# want WHAT RC: the run exited RC and printed exactly the file want.
want() {
  if [ "$rc" -ne "$2" ] || ! cmp -s want out; then
    printf 'got (exit %s):\n' "$rc" >&2
    cat out >&2
    printf 'want (exit %s):\n' "$2" >&2
    cat want >&2
    fail "$1"
  fi
  echo "ok $1"
}

# --- every skip expected ------------------------------------------------------
mkdir ci
printf '%s\n' 'running 4 tests' 'SKIP c_mseal no mseal syscall' 'test a ... ok' \
  'SKIP thp/auto THP is off' 'SKIP c_mseal again, from another test' > test.log
printf '%s\n' c_mseal thp/auto > ci/x86_64-glibc.txt
run test.log ci/x86_64-glibc.txt
cat > want <<'EOF'
### Skips: x86_64-glibc

2 distinct skips.

| skip | status |
|---|---|
| `c_mseal` | expected |
| `thp/auto` | expected |
skips match ci/x86_64-glibc.txt
EOF
want "every skip expected passes, one row per key" 0
# Its temporary files are removed (mktemp follows TMPDIR).
mkdir td
if TMPDIR=$tmp/td sh "$script" test.log ci/x86_64-glibc.txt > out 2>&1; then rc=0; else rc=$?; fi
[ "$rc" -eq 0 ] && [ -z "$(ls -A td)" ] || { ls -A td >&2; fail "temporary files: got = exit $rc and these left, want = exit 0 and none"; }
echo "ok its temporary files are removed"

# --- a new skip, a gone one and an expected one --------------------------------
printf '%s\n' 'SKIP c_mseal no mseal' 'SKIP ctest-cc-race: no /dev/shm' > test.log
printf '%s\n' c_mseal gone_test > ci/x86_64-glibc.txt
run test.log ci/x86_64-glibc.txt
cat > want <<'EOF'
### Skips: x86_64-glibc

2 distinct skips.

| skip | status |
|---|---|
| `c_mseal` | expected |
| `ctest-cc-race:` | **NEW** |
| `gone_test` | no longer skipped |
notice: no longer skipped, drop it from ci/x86_64-glibc.txt: gone_test
error: new skip (lost coverage) not in ci/x86_64-glibc.txt: ctest-cc-race:
observed skip keys (the manifest if all are expected):
c_mseal
ctest-cc-race:
EOF
want "a new skip fails, naming it, and a gone one is a notice" 1

# --- a gone skip alone passes ---------------------------------------------------
printf '%s\n' 'SKIP c_mseal no mseal' > test.log
printf '%s\n' c_mseal gone_test > ci/x86_64-glibc.txt
run test.log ci/x86_64-glibc.txt
cat > want <<'EOF'
### Skips: x86_64-glibc

1 distinct skips.

| skip | status |
|---|---|
| `c_mseal` | expected |
| `gone_test` | no longer skipped |
notice: no longer skipped, drop it from ci/x86_64-glibc.txt: gone_test
skips match ci/x86_64-glibc.txt
EOF
want "a gone skip alone passes with a notice" 0

# --- no skips, an empty manifest --------------------------------------------------
printf '%s\n' 'running 1 test' 'test a ... ok' > test.log
: > ci/empty.txt
run test.log ci/empty.txt
cat > want <<'EOF'
### Skips: empty

0 distinct skips.

skips match ci/empty.txt
EOF
want "no skips and an empty manifest pass with no table" 0
# No skips, but a manifest key: the table still shows it as gone.
printf '%s\n' gone_test > ci/gone.txt
run test.log ci/gone.txt
cat > want <<'EOF'
### Skips: gone

0 distinct skips.

| skip | status |
|---|---|
| `gone_test` | no longer skipped |
notice: no longer skipped, drop it from ci/gone.txt: gone_test
skips match ci/gone.txt
EOF
want "no skips and a manifest key: a table with the gone row, and a pass" 0

# --- the manifest's comments, blanks and trailing blanks ------------------------
printf '%s\n' 'SKIP c_mseal x' 'SKIP thp/auto y' > test.log
printf '%s\n' '# kernel-gated tests' '' 'c_mseal   ' 'thp/auto # THP' '   ' > ci/commented.txt
run test.log ci/commented.txt
cat > want <<'EOF'
### Skips: commented

2 distinct skips.

| skip | status |
|---|---|
| `c_mseal` | expected |
| `thp/auto` | expected |
skips match ci/commented.txt
EOF
want "comments, blank lines and trailing blanks in the manifest count for nothing" 0

# A line with leading blanks is a key that never matches; it is
# reported without them, as challoc's script does.
printf '%s\n' 'SKIP c_mseal x' > test.log
printf '%s\n' '  c_mseal' > ci/indented.txt
run test.log ci/indented.txt
cat > want <<'EOF'
### Skips: indented

1 distinct skips.

| skip | status |
|---|---|
| `c_mseal` | **NEW** |
| `c_mseal` | no longer skipped |
notice: no longer skipped, drop it from ci/indented.txt: c_mseal
error: new skip (lost coverage) not in ci/indented.txt: c_mseal
observed skip keys (the manifest if all are expected):
c_mseal
EOF
want "a manifest line with leading blanks never matches" 1

# --- the keys it drops ---------------------------------------------------------------
# A lone test or ok (a libtest line cut by another test's output), a
# SKIP with no key, and SKIP anywhere but at the start of a line.
printf '%s\n' 'SKIP test' 'SKIP ok' 'SKIP' 'SKIP  ' ' SKIP indented' 'xSKIP joined' 'SKIPPED c_x' > test.log
run test.log ci/empty.txt
cat > want <<'EOF'
### Skips: empty

0 distinct skips.

skips match ci/empty.txt
EOF
want "lone test/ok keys, an empty key and SKIP not at a line's start are no skips" 0

# --- bytewise order, and a key that starts with - ----------------------------------
printf '%s\n' 'SKIP alpha a' 'SKIP Zed z' 'SKIP -dash d' 'SKIP _under u' > test.log
printf '%s\n' alpha -dash Zed _under > ci/order.txt
run test.log ci/order.txt
cat > want <<'EOF'
### Skips: order

4 distinct skips.

| skip | status |
|---|---|
| `-dash` | expected |
| `Zed` | expected |
| `_under` | expected |
| `alpha` | expected |
skips match ci/order.txt
EOF
want "keys sort bytewise, and a key that starts with - is matched" 0

# --- a missing manifest --------------------------------------------------------------
printf '%s\n' 'SKIP thp/auto x' 'SKIP c_mseal y' > test.log
run test.log ci/none.txt
cat > want <<'EOF'
### Skips: none

2 distinct skips.

| skip | status |
|---|---|
| `c_mseal` | **NEW** |
| `thp/auto` | **NEW** |
error: no skip manifest ci/none.txt; commit this one:
c_mseal
thp/auto
EOF
want "a missing manifest fails and prints the one to commit" 1
[ ! -e ci/none.txt ] || fail "a missing manifest: the run wrote it"

# --- CI_SKIP_UPDATE=1 ------------------------------------------------------------------
if CI_SKIP_UPDATE=1 sh "$script" test.log new/cell.txt > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<'EOF'
### Skips: cell

2 distinct skips.

| skip | status |
|---|---|
| `c_mseal` | **NEW** |
| `thp/auto` | **NEW** |
wrote new/cell.txt
EOF
want "CI_SKIP_UPDATE=1 writes a missing manifest, its directory too, and passes" 0
printf '%s\n' c_mseal thp/auto > want
cmp -s want new/cell.txt || { cat new/cell.txt >&2; fail "CI_SKIP_UPDATE=1: the manifest is not the observed keys"; }
echo "ok CI_SKIP_UPDATE=1: the manifest written is the observed keys, sorted"
# Over an existing manifest with a new skip and a gone one: it passes
# and the manifest becomes the observed keys.
printf '%s\n' '# old' c_mseal gone_test > ci/x86_64-glibc.txt
if CI_SKIP_UPDATE=1 sh "$script" test.log ci/x86_64-glibc.txt > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<'EOF'
### Skips: x86_64-glibc

2 distinct skips.

| skip | status |
|---|---|
| `c_mseal` | expected |
| `thp/auto` | **NEW** |
| `gone_test` | no longer skipped |
wrote ci/x86_64-glibc.txt
EOF
want "CI_SKIP_UPDATE=1 over a manifest with a new and a gone skip passes" 0
printf '%s\n' c_mseal thp/auto > want
cmp -s want ci/x86_64-glibc.txt || { cat ci/x86_64-glibc.txt >&2; fail "CI_SKIP_UPDATE=1: the manifest was not rewritten to the observed keys"; }
echo "ok CI_SKIP_UPDATE=1: an existing manifest is rewritten to the observed keys"
# Any other value is no update.
printf '%s\n' c_mseal > ci/x86_64-glibc.txt
if CI_SKIP_UPDATE=yes sh "$script" test.log ci/x86_64-glibc.txt > out 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 1 ] || ! grep -qxF 'error: new skip (lost coverage) not in ci/x86_64-glibc.txt: thp/auto' out; then
  cat out >&2
  fail "CI_SKIP_UPDATE=yes: got = exit $rc, want = exit 1 naming the new skip"
fi
printf '%s\n' c_mseal > want
cmp -s want ci/x86_64-glibc.txt || fail "CI_SKIP_UPDATE=yes: the manifest was rewritten"
echo "ok CI_SKIP_UPDATE=yes is no update"

# --- a missing log, and usage -----------------------------------------------------------
run no.log ci/x86_64-glibc.txt
printf '%s\n' 'error: no.log missing: the test step did not run' > want
want "a missing log fails: the test step did not run" 1

# --- a log or manifest it cannot read ---------------------------------------------------
# A stub first on PATH that exits 2 stands in for a read error (chmod
# 000 does not stop root, which a check guest runs as).
mkdir stub-grep stub-sed
# shellcheck disable=SC2016 # $0 is the stub's, expanded when it runs
printf '%s\n' '#!/bin/sh' 'echo "$0: read error" >&2' 'exit 2' > stub-grep/grep
cp stub-grep/grep stub-sed/sed
chmod +x stub-grep/grep stub-sed/sed
printf '%s\n' c_mseal > ci/x86_64-glibc.txt
printf '%s\n' 'SKIP newkey x' > test.log
if PATH=$tmp/stub-grep:$PATH sh "$script" test.log ci/x86_64-glibc.txt > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' "$tmp/stub-grep/grep: read error" 'error: cannot read test.log (grep exit 2)' > want
want "a log grep cannot read fails, not a pass" 1
if PATH=$tmp/stub-sed:$PATH sh "$script" test.log ci/x86_64-glibc.txt > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' "$tmp/stub-sed/sed: read error" 'error: cannot read ci/x86_64-glibc.txt' > want
want "a manifest sed cannot read fails, not a pass" 1
# An awk that fails: on the log's SKIP lines (every call), and on the
# manifest alone (its blank-line filter; other calls run the real awk).
mkdir stub-awk stub-awk-manifest
cp stub-grep/grep stub-awk/awk
cat > stub-awk-manifest/awk <<'EOF'
#!/bin/sh
if [ "$1" = 'length($0) > 0' ]; then echo "$0: failed" >&2; exit 2; fi
PATH=$REAL_PATH exec awk "$@"
EOF
chmod +x stub-awk/awk stub-awk-manifest/awk
if PATH=$tmp/stub-awk:$PATH sh "$script" test.log ci/x86_64-glibc.txt > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' "$tmp/stub-awk/awk: read error" 'error: awk failed on the SKIP lines of test.log' > want
want "an awk that fails on the log's SKIP lines fails, not a pass" 1
REAL_PATH=$PATH
export REAL_PATH
if PATH=$tmp/stub-awk-manifest:$PATH sh "$script" test.log ci/x86_64-glibc.txt > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' "$tmp/stub-awk-manifest/awk: failed" 'error: awk failed on ci/x86_64-glibc.txt' > want
want "an awk that fails on the manifest fails, not a pass" 1
# An empty log: the test step printed nothing.
: > empty.log
: > ci/empty.txt
run empty.log ci/empty.txt
printf '%s\n' 'error: empty.log is empty: the test step printed nothing' > want
want "an empty log fails: the test step printed nothing" 1

run test.log
printf '%s\n' 'usage: skip-summary.sh LOG MANIFEST' > want
want "one argument is a usage error" 2
run test.log ci/x86_64-glibc.txt extra
want "three arguments are a usage error" 2

echo "PASS"
