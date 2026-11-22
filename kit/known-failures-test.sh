#!/bin/sh
# kit/known-failures-test.sh: tests of kit/make/known-failures.sh (the
# known-failures-test check). Each case writes a test log and a list,
# runs the script and checks its exit status and its output, which the
# cases below spell out by hand from challoc's ci/known-failures.sh's
# format (none is taken from a run of the script under test): every
# failure known, an unlisted one, none at all, the lines that are not
# failures, the list's comments and blanks, cell-scoped entries, an
# EXTRA command (its keys, its blank lines, its failure), a missing log
# or list, a log or list it cannot read or a tool that fails on one, its
# temporary files, and usage. Run it from the repository root. It needs
# only a POSIX shell and BusyBox's tools.
set -eu

script=$(pwd)/kit/make/known-failures.sh
[ -f "$script" ] || { echo "FAIL: run it from the repository root (no kit/make/known-failures.sh)" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"
unset CI_CELL || true

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

# --- every failure known, and an unlisted one -------------------------------------
printf '%s\n' 'running 3 tests' 'test api::tests::big ... FAILED' 'test small ... ok' \
  'test tls::race ... FAILED' '' 'failures:' '' '---- tls::race stdout ----' '    indented output' '' \
  'failures:' '    api::tests::big' '    tls::race' '' \
  'test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s' > test.log
printf '%s\n' tls::race api::tests::big > known.txt
run test.log known.txt
cat > want <<'EOF'
warning: known failure (known.txt): api::tests::big
warning: known failure (known.txt): tls::race
EOF
want "every failure known passes, each a warning, in bytewise order" 0
# Its temporary files are removed (mktemp follows TMPDIR).
mkdir td
if TMPDIR=$tmp/td sh "$script" test.log known.txt > out 2>&1; then rc=0; else rc=$?; fi
[ "$rc" -eq 0 ] && [ -z "$(ls -A td)" ] || { ls -A td >&2; fail "temporary files: got = exit $rc and these left, want = exit 0 and none"; }
echo "ok its temporary files are removed"

printf '%s\n' tls::race > known.txt
run test.log known.txt
cat > want <<'EOF'
error: failed: api::tests::big
warning: known failure (known.txt): tls::race
EOF
want "an unlisted failure fails, naming it, beside the known one" 1

# --- no failure at all, and lines that are not failures --------------------------------
printf '%s\n' 'error[E0425]: cannot find value' ' test indented ... FAILED' 'xtest joined ... FAILED' \
  'test b ...  FAILED' 'test c ... ok' 'test d - should panic ... ok' > test.log
run test.log known.txt
printf '%s\n' 'error: the run failed, but no test did (a build or other step failed; see the log)' > want
want "a build error, an indented or joined line, two blanks before FAILED and ok lines are no failures" 1

# --- names with blanks: should_panic tests and doctests ----------------------------------
# libtest writes `test NAME - MODE ... FAILED` for these; the whole name
# is the key, so an unlisted one beside a listed failure fails.
printf '%s\n' 'test api::known ... FAILED' 'test tests::boom - should panic ... FAILED' \
  'test src/lib.rs - parse (line 12) ... FAILED' > test.log
printf '%s\n' api::known > known.txt
run test.log known.txt
cat > want <<'EOF'
warning: known failure (known.txt): api::known
error: failed: src/lib.rs - parse (line 12)
error: failed: tests::boom - should panic
EOF
want "an unlisted should_panic test and doctest beside a listed failure fail, named whole" 1
printf '%s\n' api::known 'tests::boom - should panic' 'src/lib.rs - parse (line 12) @*' > known.txt
run test.log known.txt
cat > want <<'EOF'
warning: known failure (known.txt): api::known
warning: known failure (known.txt): src/lib.rs - parse (line 12)
warning: known failure (known.txt): tests::boom - should panic
EOF
want "a should_panic test and a doctest listed by their whole names (one cell-scoped) pass" 0
printf '%s\n' 'tests::boom' 'src/lib.rs' > known.txt
run test.log known.txt
cat > want <<'EOF'
error: failed: api::known
error: failed: src/lib.rs - parse (line 12)
error: failed: tests::boom - should panic
notice: listed but did not fail, drop it from known.txt once fixed: src/lib.rs
notice: listed but did not fail, drop it from known.txt once fixed: tests::boom
EOF
want "a name's first word does not list it" 1

# --- the failures blocks: a FAILED not on its test's line, and counts --------------------
# On one thread libtest writes `test NAME ... ` before the test runs;
# output it does not capture (C code over FFI, a child) can land before
# FAILED, which then sits on a line of its own. The last failures block
# before `test result:` names it.
printf '%s\n' 'running 2 tests' 'test api::known ... FAILED' 'test ffi::parse ... c_parse: bad header' 'FAILED' \
  '' 'failures:' '' '---- api::known stdout ----' 'boom' '' \
  'failures:' '    api::known' '    ffi::parse' '' \
  'test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s' > test.log
printf '%s\n' api::known > known.txt
run test.log known.txt
cat > want <<'EOF'
warning: known failure (known.txt): api::known
error: failed: ffi::parse
EOF
want "a FAILED on a line of its own is read from the failures block, and an unlisted one fails" 1
printf '%s\n' api::known ffi::parse > known.txt
run test.log known.txt
cat > want <<'EOF'
warning: known failure (known.txt): api::known
warning: known failure (known.txt): ffi::parse
EOF
want "listed, a failure named only in the failures block passes" 0
# A block naming fewer tests than its result line counts fails, beside
# known failures; each binary's block is counted against its own line.
printf '%s\n' 'failures:' '    api::known' '' 'test result: FAILED. 3 passed; 1 failed; 0 ignored' \
  'failures:' '    ffi::parse' '' 'test result: FAILED. 0 passed; 2 failed; 0 ignored' \
  'test result: ok. 4 passed; 0 failed; 0 ignored' > test.log
run test.log known.txt
cat > want <<'EOF'
warning: known failure (known.txt): api::known
warning: known failure (known.txt): ffi::parse
error: test.log's test result lines count 1 more failures than its failures blocks name (a test the run does not name failed)
EOF
want "a block naming fewer tests than its result line counts fails" 1
# A result line with no block of its own takes no names from the block
# before it.
printf '%s\n' 'failures:' '    api::known' '' 'test result: FAILED. 0 passed; 1 failed; 0 ignored' \
  'test result: FAILED. 0 passed; 1 failed; 0 ignored' > test.log
run test.log known.txt
cat > want <<'EOF'
warning: known failure (known.txt): api::known
notice: listed but did not fail, drop it from known.txt once fixed: ffi::parse
error: test.log's test result lines count 1 more failures than its failures blocks name (a test the run does not name failed)
EOF
want "a result line with no block of its own counts as unnamed" 1
# A doctest binary's block names whole names, blanks included.
printf '%s\n' 'test src/lib.rs - parse (line 12) ... ' 'stray' 'FAILED' 'failures:' '    src/lib.rs - parse (line 12)' '' \
  'test result: FAILED. 0 passed; 1 failed; 0 ignored' > test.log
printf '%s\n' 'src/lib.rs - parse (line 12)' > known.txt
run test.log known.txt
printf '%s\n' 'warning: known failure (known.txt): src/lib.rs - parse (line 12)' > want
want "a doctest named in a failures block, blanks included, is read whole" 0
# The block gives a should_panic test's bare name; its FAILED line's
# whole name (with ` - should panic`) is the key, and the bare name adds
# nothing.
printf '%s\n' 'test tests::boom - should panic ... FAILED' '' 'failures:' '    tests::boom' '' \
  'test result: FAILED. 0 passed; 1 failed; 0 ignored' > test.log
printf '%s\n' 'tests::boom - should panic' > known.txt
run test.log known.txt
printf '%s\n' 'warning: known failure (known.txt): tests::boom - should panic' > want
want "a should_panic test's bare block name adds nothing beside its whole FAILED-line name" 0
printf '%s\n' 'tests::boom' > known.txt
run test.log known.txt
printf '%s\n' 'error: failed: tests::boom - should panic' \
  'notice: listed but did not fail, drop it from known.txt once fixed: tests::boom' > want
want "the bare name does not list a should_panic test" 1
# A doctest run in a mode: its FAILED line adds ` - compile fail` to the
# name its block gives; one list entry, the whole name, passes.
printf '%s\n' 'test src/lib.rs - parse (line 12) - compile fail ... FAILED' '' 'failures:' \
  '    src/lib.rs - parse (line 12)' '' 'test result: FAILED. 0 passed; 1 failed; 0 ignored' > test.log
printf '%s\n' 'src/lib.rs - parse (line 12) - compile fail' > known.txt
run test.log known.txt
printf '%s\n' 'warning: known failure (known.txt): src/lib.rs - parse (line 12) - compile fail' > want
want "a doctest in a mode passes with its whole FAILED-line name alone" 0
# A run whose only failure has its FAILED on a line of its own: the
# block names it (no FAILED-line keys at all).
printf '%s\n' 'test ffi::parse ... c_parse: bad header' 'FAILED' '' 'failures:' '    ffi::parse' '' \
  'test result: FAILED. 0 passed; 1 failed; 0 ignored' > test.log
printf '%s\n' ffi::parse > known.txt
run test.log known.txt
printf '%s\n' 'warning: known failure (known.txt): ffi::parse' > want
want "with no FAILED-line keys, the block alone names the failure" 0
# A block name is covered only by a key that is it or it then ` - `: a
# split `api::x` beside a FAILED line for `api::xy` is a failure of its
# own.
printf '%s\n' 'test api::xy ... FAILED' 'test api::x ... out' 'FAILED' '' 'failures:' '    api::x' '    api::xy' '' \
  'test result: FAILED. 0 passed; 2 failed; 0 ignored' > test.log
printf '%s\n' api::xy > known.txt
run test.log known.txt
cat > want <<'EOF'
error: failed: api::x
warning: known failure (known.txt): api::xy
EOF
want "a key that only starts with a block name does not cover it" 1
# Coverage is per test binary: binary A's listed `tests::boom - should
# panic` does not cover binary B's split `tests::boom`, a different test.
printf '%s\n' 'running 1 test' 'test tests::boom - should panic ... FAILED' '' 'failures:' '    tests::boom' '' \
  'test result: FAILED. 0 passed; 1 failed; 0 ignored' '' \
  'running 1 test' 'test tests::boom ... out of C' 'FAILED' '' 'failures:' '    tests::boom' '' \
  'test result: FAILED. 0 passed; 1 failed; 0 ignored' > test.log
printf '%s\n' 'tests::boom - should panic' > known.txt
run test.log known.txt
cat > want <<'EOF'
error: failed: tests::boom
warning: known failure (known.txt): tests::boom - should panic
EOF
want "a key from another test binary does not cover a block name" 1
# Within one binary, before its result line, the FAILED-line key still
# covers its own bare block name after an `ok` binary's result line.
printf '%s\n' 'test result: ok. 3 passed; 0 failed; 0 ignored' \
  'test tests::boom - should panic ... FAILED' 'failures:' '    tests::boom' '' \
  'test result: FAILED. 0 passed; 1 failed; 0 ignored' > test.log
run test.log known.txt
printf '%s\n' 'warning: known failure (known.txt): tests::boom - should panic' > want
want "the same binary's key covers its bare block name" 0
# A FAILED line with words after it is still one (the match is a prefix),
# and a name seen twice counts once.
printf '%s\n' 'test a ... FAILED trailing' 'test a ... FAILED' > test.log
printf '%s\n' a > known.txt
run test.log known.txt
printf '%s\n' 'warning: known failure (known.txt): a' > want
want "words after FAILED still count, and a name seen twice counts once" 0

# --- the list's comments and blanks ------------------------------------------------------
printf '%s\n' 'test api::x ... FAILED' 'test api::y ... FAILED' > test.log
printf '%s\n' '# TODO: api::x races (issue 12)' '' 'api::x   ' '  api::y # flaky' '   ' > known.txt
run test.log known.txt
cat > want <<'EOF'
warning: known failure (known.txt): api::x
warning: known failure (known.txt): api::y
EOF
want "comments, blank lines and blanks around an entry count for nothing" 0
printf '%s\n' '# api::x' 'api::y' > known.txt
run test.log known.txt
cat > want <<'EOF'
error: failed: api::x
warning: known failure (known.txt): api::y
EOF
want "a commented-out entry is no entry" 1
: > known.txt
run test.log known.txt
cat > want <<'EOF'
error: failed: api::x
error: failed: api::y
EOF
want "an empty list knows nothing" 1
# A last entry with no newline is an entry (with trailing blanks too).
printf '%s' 'api::x' > known.txt
run test.log known.txt
cat > want <<'EOF'
warning: known failure (known.txt): api::x
error: failed: api::y
EOF
want "a last entry with no newline is an entry" 1
printf '%s\n%s' 'api::x' 'api::y  ' > known.txt
run test.log known.txt
cat > want <<'EOF'
warning: known failure (known.txt): api::x
warning: known failure (known.txt): api::y
EOF
want "a last entry with no newline and trailing blanks is an entry" 0
# An entry in scope that did not fail is a notice, in list order; one
# scoped to another cell is out of scope, so no notice. The exit status
# is the failures' alone.
printf '%s\n' 'test api::x ... FAILED' > test.log
printf '%s\n' api::x 'api::gone' 'api::arm @hosted-arm64' 'api::amd @hosted-amd64' > known.txt
if CI_CELL=hosted-amd64 sh "$script" test.log known.txt > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<'EOF'
warning: known failure (known.txt): api::x
notice: listed but did not fail, drop it from known.txt once fixed: api::amd
notice: listed but did not fail, drop it from known.txt once fixed: api::gone
EOF
want "an in-scope entry that did not fail is a notice; one out of scope is not; the exit is the failures'" 0

# --- cell-scoped entries ------------------------------------------------------------------
printf '%s\n' 'test api::x ... FAILED' > test.log
printf '%s\n' 'api::x @hosted-*-privileged' > known.txt
if CI_CELL=hosted-amd64-privileged sh "$script" test.log known.txt > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' 'warning: known failure (known.txt): api::x' > want
want "an entry scoped to a matching cell applies" 0
if CI_CELL=hosted-amd64 sh "$script" test.log known.txt > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' 'error: failed: api::x' > want
want "an entry scoped to another cell does not" 1
run test.log known.txt
want "a scoped entry does not apply with CI_CELL unset" 1
printf '%s\n' 'api::x @*' > known.txt
run test.log known.txt
printf '%s\n' 'warning: known failure (known.txt): api::x' > want
want "an entry scoped to * applies with CI_CELL unset" 0
printf '%s\n' 'api::x @hosted-arm64 # arm64 only' 'api::x @hosted-amd64' > known.txt
if CI_CELL=hosted-amd64 sh "$script" test.log known.txt > out 2>&1; then rc=0; else rc=$?; fi
want "the second of two scoped lines applies, after a comment is stripped" 0

# --- EXTRA ----------------------------------------------------------------------------------
cat > extra.sh <<'EOF'
# challoc's cgroup verdicts: OUTCOME KIND key=value...; not ok is a failure.
awk '/^OUTCOME / { v = ""; a = ""; for (i = 3; i <= NF; i++) { if ($i ~ /^verdict=/) v = substr($i, 9); if ($i ~ /^alloc=/) a = substr($i, 7) } if (v != "ok") print "cgroup-tests " $2 " " a }' "$1"
echo
EOF
printf '%s\n' 'test api::x ... FAILED' 'OUTCOME lazy alloc=challoc-eager verdict=oom' \
  'OUTCOME eager alloc=glibc verdict=ok' 'OUTCOME -dash alloc=x verdict=bad' > test.log
printf '%s\n' api::x 'cgroup-tests lazy challoc-eager' > known.txt
run test.log known.txt extra.sh
cat > want <<'EOF'
warning: known failure (known.txt): api::x
error: failed: cgroup-tests -dash x
warning: known failure (known.txt): cgroup-tests lazy challoc-eager
EOF
want "EXTRA's keys are failures too, its blank lines are none, and a key may start with -" 1
printf '%s\n' 'OUTCOME lazy alloc=challoc-eager verdict=oom' > test.log
run test.log known.txt extra.sh
printf '%s\n' 'warning: known failure (known.txt): cgroup-tests lazy challoc-eager' \
  'notice: listed but did not fail, drop it from known.txt once fixed: api::x' > want
want "EXTRA's failures alone, all known, pass; the entry that did not fail is a notice" 0
# An OUTCOME line with no alloc= gives a key with a trailing blank,
# which is dropped, as an entry's is.
printf '%s\n' 'OUTCOME odd verdict=bad' > test.log
printf '%s\n' 'cgroup-tests odd' > odd.txt
run test.log odd.txt extra.sh
printf '%s\n' 'warning: known failure (odd.txt): cgroup-tests odd' > want
want "a key's trailing blank is dropped, as an entry's is" 0
# Keys from EXTRA: a line of blanks alone is none; an @ with no blank
# before it is part of the name, not a scope; a listed key that starts
# with - is matched; and a match is the whole entry, not a part of it.
cat > keys.sh <<'EOF'
printf '%s\n' '   ' 'user@host' '-dash' 'api::xy'
EOF
printf '%s\n' 'user@host' '-dash' 'api::xyz' > keys.txt
run test.log keys.txt keys.sh
cat > want <<'EOF'
warning: known failure (keys.txt): -dash
error: failed: api::xy
warning: known failure (keys.txt): user@host
notice: listed but did not fail, drop it from keys.txt once fixed: api::xyz
EOF
want "blanks alone are no key, @ without a blank is a name, -dash matches, api::xyz does not list api::xy" 1
run test.log known.txt
printf '%s\n' 'error: the run failed, but no test did (a build or other step failed; see the log)' > want
want "without EXTRA its lines are not read" 1
printf '%s\n' 'echo "cgroup-tests half"' 'exit 3' > bad-extra.sh
run test.log known.txt bad-extra.sh
printf '%s\n' 'error: bad-extra.sh failed on test.log (exit 3)' > want
want "an EXTRA that fails fails, whatever it printed" 1
run test.log known.txt ''
printf '%s\n' 'error: the run failed, but no test did (a build or other step failed; see the log)' > want
want "an empty EXTRA is none" 1
# A coloured log (`-- --color=always`): neither its FAILED line nor its
# result line reads, so a listed EXTRA key beside it must not pass.
esc=$(printf '\033')
printf '%s\n' "test api::x ... ${esc}[31mFAILED${esc}[0m" 'failures:' '    api::x' '' \
  "test result: ${esc}[31mFAILED${esc}[0m. 0 passed; 1 failed; 0 ignored" \
  'OUTCOME lazy alloc=challoc-eager verdict=oom' > test.log
printf '%s\n' 'cgroup-tests lazy challoc-eager' > known.txt
run test.log known.txt extra.sh
cat > want <<'EOF'
warning: known failure (known.txt): cgroup-tests lazy challoc-eager
error: test.log has 1 test result lines that are neither ok. nor FAILED. (coloured output?); run the tests without --color
EOF
want "a coloured result line fails the run, beside a listed EXTRA key" 1

# --- missing, unreadable, failing tools ----------------------------------------------------
printf '%s\n' 'test api::x ... FAILED' > test.log
printf '%s\n' api::x > known.txt
run no.log known.txt
printf '%s\n' 'error: no.log missing: the test step did not run' > want
want "a missing log fails" 1
run test.log no.txt
printf '%s\n' 'error: no known-failures list no.txt' > want
want "a missing list fails" 1
# A stub first on PATH that exits 2 stands in for a read error (chmod
# 000 does not stop root, which a check guest runs as). The stub grep
# fails only the failures grep (-aE); the stub awk fails every call.
mkdir stub-grep stub-sed stub-awk
cat > stub-grep/grep <<'EOF'
#!/bin/sh
if [ "$1" = -aE ]; then echo "$0: read error" >&2; exit 2; fi
PATH=$REAL_PATH exec grep "$@"
EOF
# shellcheck disable=SC2016 # $0 is the stub's, expanded when it runs
printf '%s\n' '#!/bin/sh' 'echo "$0: read error" >&2' 'exit 2' > stub-sed/sed
cp stub-sed/sed stub-awk/awk
chmod +x stub-grep/grep stub-sed/sed stub-awk/awk
REAL_PATH=$PATH
export REAL_PATH
if PATH=$tmp/stub-grep:$PATH sh "$script" test.log known.txt > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' "$tmp/stub-grep/grep: read error" 'error: cannot read test.log (grep exit 2)' > want
want "a log grep cannot read fails" 1
if PATH=$tmp/stub-sed:$PATH sh "$script" test.log known.txt > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' "$tmp/stub-sed/sed: read error" 'error: cannot read known.txt' > want
want "a list sed cannot read fails" 1
if PATH=$tmp/stub-awk:$PATH sh "$script" test.log known.txt > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' "$tmp/stub-awk/awk: read error" 'error: awk failed on the FAILED lines of test.log' > want
want "an awk that fails on the FAILED lines fails" 1

# --- usage -------------------------------------------------------------------------------------
run test.log
printf '%s\n' 'usage: known-failures.sh LOG LIST [EXTRA]' > want
want "one argument is a usage error" 2
run test.log known.txt extra.sh more
want "four arguments are a usage error" 2

echo "PASS"
