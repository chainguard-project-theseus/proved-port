#!/bin/sh
# kit/miri-test.sh: tests of the Miri template (kit/templates/ci/miri.sh,
# the miri-template check). Fills the template into a scratch port with a
# stub cargo and a stub ci/lib.sh, so no toolchain or network is needed,
# and shows which runs pass and which fail: the known list's rules, the
# thread tests' guard, a thread test the Miri build leaves out, and an
# unfilled setting. The check runs it with BusyBox's tools, as the ports'
# miri checks run the script. It also shows that kit/templates/README.md's
# placeholder pattern finds every placeholder the fill replaces. Run it
# from the repository root.
set -eu

templates=$(pwd)/kit/templates
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
port=$tmp/port
log=$tmp/out

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# The placeholder pattern kit/templates/README.md gives for these files,
# from its `grep -nE '...' ci/miri.sh ...` line.
pattern=$(sed -n "s/^grep -nE '\(.*\)' ci\/miri\.sh .*/\1/p" "$templates/README.md")
[ -n "$pattern" ] || fail "no ci/miri.sh placeholder grep in kit/templates/README.md"
miri_files="ci/miri.sh ci/lib.sh ci/miri-known.txt .chainguard/ci/miri.yaml"
for p in CAPI_CRATE CAPI_CRATE_DIR KIT_VERSION KIT_SHA MIRI_NIGHTLY MIRI_MANIFEST_SHA256 \
  MIRI_NO_RUN MIRI_THREAD_TESTS MIRI_FLAGS RUSTUP_RELEASE \
  RUSTUP_INIT_SHA256_X86_64 RUSTUP_INIT_SHA256_AARCH64; do
  found=
  for f in $miri_files; do
    if grep -oE "$pattern" "$templates/$f" | grep -q "{${p}[}:]"; then found=1; fi
  done
  [ -n "$found" ] || fail "the README's pattern does not find {$p}"
done
echo "ok the README's pattern finds every placeholder"

# fill THREAD_TESTS: the template into $port/ci/miri.sh, with
# thread_tests=THREAD_TESTS and nomiri=clink.
fill() {
  sed -e 's/{MIRI_NIGHTLY}/2026-10-27/g' -e 's/{MIRI_MANIFEST_SHA256}/0/' \
    -e 's|{CAPI_CRATE_DIR}|crates/foo-capi|' -e 's/{CAPI_CRATE}/foo-capi/g' \
    -e 's/{KIT_VERSION}/0.0.0/' -e 's/{KIT_SHA}/0000000/' -e 's/{MIRI_NO_RUN:[^}]*}/clink/' \
    -e "s/{MIRI_THREAD_TESTS:[^}]*}/$1/" -e 's/{MIRI_FLAGS:[^}]*}//' \
    "$templates/ci/miri.sh" >"$port/ci/miri.sh"
  if grep -nE "$pattern" "$port/ci/miri.sh" >&2; then
    fail "placeholders left in the filled ci/miri.sh"
  fi
}

mkdir -p "$port/ci" "$port/crates/foo-capi/src" "$port/crates/foo-capi/tests" "$tmp/bin"
cat >"$port/ci/lib.sh" <<'EOF'
# shellcheck shell=sh
rust_host() { echo stub-host; }
rustup_pinned() { :; }
EOF
cat >"$port/crates/foo-capi/src/lib.rs" <<'EOF'
#[cfg(test)]
mod tests {
    #[test]
    fn plain() {}

    #[test]
    fn spawns() {
        std::thread::spawn(|| {}).join().unwrap();
    }

    #[test]
    fn known_ub() {}

    #[cfg(not(miri))]
    #[test]
    fn gated_stress() {
        std::thread::scope(|_| {});
    }
}
EOF
cat >"$port/crates/foo-capi/tests/api.rs" <<'EOF'
#[test]
fn api_call() {}
EOF
cat >"$port/crates/foo-capi/tests/clink.rs" <<'EOF'
#[test]
fn links_c() {
    std::thread::spawn(|| {}).join().unwrap();
}
EOF

# The stub cargo: `miri setup` and `--no-run` succeed; `--list` prints
# the binary's tests as MOCK_LIB and MOCK_API name them; a run prints
# each test ok, or Miri's undefined behaviour line for one MOCK_UB names.
# A run of the clink binary fails, since nomiri leaves it out.
cat >"$tmp/bin/cargo" <<'EOF'
#!/bin/sh
bin= list= exact= names=
for a in "$@"; do
  case $a in
    setup | --no-run) exit 0 ;;
    --lib) bin=lib ;;
    --test) bin=next ;;
    --list) list=1 ;;
    --exact) exact=1 ;;
    -*) ;;
    *)
      if [ "$bin" = next ]; then bin=$a
      elif [ -n "$exact" ]; then names="$names $a"
      fi ;;
  esac
done
case $bin in
  lib) tests=$MOCK_LIB ;;
  api) tests=$MOCK_API ;;
  *) echo "stub cargo: binary $bin should not run" >&2; exit 9 ;;
esac
if [ -n "$list" ]; then
  for t in $tests; do echo "$t: test"; done
  exit 0
fi
rc=0
for n in $names; do
  case " $MOCK_UB " in
    *" $n "*) echo "error: Undefined Behavior: stub $n"; rc=1 ;;
    *) echo "test $n ... ok" ;;
  esac
done
exit "$rc"
EOF
chmod +x "$tmp/bin/cargo"

# miri EXPECT WHAT [VAR=VALUE...]: run the filled script in $port with
# the stub cargo; EXPECT is pass or fail.
miri() {
  expect=$1 what=$2
  shift 2
  if (cd "$port" && env PATH="$tmp/bin:$PATH" MIRI_JOBS=2 \
    MOCK_LIB="tests::plain tests::spawns tests::known_ub" MOCK_API=api_call \
    MOCK_UB=tests::known_ub "$@" sh ci/miri.sh) >"$log" 2>&1; then
    got=pass
  else
    got=fail
  fi
  if [ "$got" != "$expect" ]; then
    cat "$log" >&2
    fail "$what: got = $got, want = $expect"
  fi
  echo "ok $what: $got"
}

# said TEXT: the last run's output holds TEXT, so it failed for the
# reason the case is about. wc -l pads its count on some systems, so the
# run's spaces are squeezed first.
said() {
  tr -s ' ' <"$log" | grep -qF "$1" || { cat "$log" >&2; fail "the run did not say: $1"; }
}

known_row() {
  printf '# rows\n\ntests::known_ub #12 an aliasing violation\n' >"$port/ci/miri-known.txt"
}
empty_known() {
  cp "$templates/ci/miri-known.txt" "$port/ci/miri-known.txt"
}

fill yes
known_row
miri pass "a known test that still fails"
grep -q '^miri: thread tests:' "$log" || fail "no thread tests reported"
grep -q '^  lib tests::spawns$' "$log" || fail "tests::spawns is not a thread test"
grep -q 'gated_stress is not in the Miri build' "$log" || fail "gated_stress not reported as left out"
if grep -q links_c "$log"; then fail "the nomiri binary's thread test was selected"; fi
grep -q 'tests::known_ub still fails (#12)' "$log" || fail "the known test was not run alone"
miri fail "a known test that passes" MOCK_UB=
said "delete its ci/miri-known.txt row"
miri fail "new undefined behaviour" MOCK_UB="tests::known_ub tests::plain"
said "stacked borrows failed"
printf 'tests::gone #1 gone\n' >"$port/ci/miri-known.txt"
miri fail "a row naming no listed test"
said "tests::gone matches 0 listed tests"
printf 'tests::plain no-issue\n' >"$port/ci/miri-known.txt"
miri fail "a row citing no issue"
said "tests::plain cites no issue"

# The template's own known list has no rows: the thread tests must be
# found with it, and their guard must hold (BusyBox grep -v -x -F -f
# with an empty file selects no line).
empty_known
miri pass "thread_tests=yes with an empty known list" MOCK_UB=
grep -q '^  lib tests::spawns$' "$log" || fail "an empty known list hid tests::spawns"
fill no
miri fail "thread_tests=no with a thread test and an empty known list" MOCK_UB=
said "set thread_tests=yes"
miri pass "thread_tests=no with no thread test" MOCK_UB= MOCK_LIB="tests::plain tests::known_ub"
sed -e 's/^thread_tests="no"/thread_tests="{MIRI_THREAD_TESTS: yes | no}"/' "$port/ci/miri.sh" >"$tmp/unfilled"
cp "$tmp/unfilled" "$port/ci/miri.sh"
miri fail "an unfilled thread_tests" MOCK_UB=
said "not yes or no"

echo "PASS"
