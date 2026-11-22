#!/bin/sh
# kit/installed-test.sh: tests of kit/make as the theseus-proved-port
# package installs it (the count-installed check). It installs the kit
# into a scratch root with kit/install.sh, the way the package's build
# does: the counter built from kit/count as usr/bin/verus-count,
# kit/make/* (mode 0644) in usr/share/proved-port/make/, all of
# kit/templates in usr/share/proved-port/templates/, kit/CHANGELOG.md as
# usr/share/proved-port/CHANGELOG.md with porting notes for the release,
# and usr/share/proved-port/KIT with the commit and tag. It holds the script's
# refusals (an untagged commit, a changed kit/, but with --dev) and a
# reinstall over an earlier install (a failed one leaves no KIT; one that
# succeeds replaces make/ and templates/ whole). Then a fixture port whose Makefile includes
# $(PROVED_PORT_HOME)/make/proved-port.mk runs:
#   - make verus-count runs the installed counter, builds nothing, and
#     prints what the counter run by hand prints;
#   - `make -n` shows that it runs verus-count from PATH and builds
#     nothing, even with a verus/count/Cargo.toml in the port;
#   - PROVED_PORT_VERSION: a version KIT does not record, none, and a
#     missing KIT fail every kit target before it runs anything; another
#     verus-count first on PATH fails verus and verus-count; the
#     recorded one passes;
#   - make verus-headers holds the port's Verus files to the installed
#     templates, reads no verus/make or verus/count the port still has,
#     and needs no `# kit:` line.
# Run it from the repository root. It builds the counter (cargo).
set -eu

export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0='*'

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# expect WHAT GOT WANT: GOT and WANT are the same lines.
expect() {
  if [ "$2" != "$3" ]; then
    printf 'got:\n%s\nwant:\n%s\n' "$2" "$3" >&2
    fail "$1"
  fi
  echo "ok $1"
}

other=0.0.1-other
target=${CARGO_TARGET_DIR:-$tmp/target}

# --- the scratch kit: this tree's kit/, committed -------------------------
kitrepo=$tmp/kit
mkdir -p "$kitrepo/kit"
cp -R kit/make kit/templates kit/install.sh kit/CHANGELOG.md "$kitrepo/kit/"
mkdir "$kitrepo/kit/count"
for f in Cargo.toml Cargo.lock src; do
  cp -R "kit/count/$f" "$kitrepo/kit/count/"
done
git -C "$kitrepo" init -q
git -C "$kitrepo" add .
git -C "$kitrepo" -c user.name=installed-test -c user.email=installed-test@example.com \
  -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -qm kit
sha=$(git -C "$kitrepo" rev-parse HEAD)

# --- the install, as the package builds it --------------------------------
# kit/install.sh lays the kit out under PREFIX as the package does under
# /usr. It refuses a checkout KIT would misdescribe (not the release's
# tag, or with a change under kit/) unless --dev, which says so.
# install PREFIX [--dev]: kit/install.sh from the scratch kit, its stderr
# in $tmp/install.err; its exit status.
install() {
  CARGO_TARGET_DIR=$target sh "$kitrepo/kit/install.sh" ${2:+"$2"} "$1" 2> "$tmp/install.err"
}
if install "$tmp/untagged"; then
  fail "kit/install.sh at an untagged commit: got = success, want = refused"
fi
grep -q "is not tag v.*pass --dev" "$tmp/install.err" \
  || { cat "$tmp/install.err" >&2; fail "kit/install.sh at an untagged commit: the refusal does not say why"; }
[ ! -e "$tmp/untagged/share/proved-port/KIT" ] || fail "kit/install.sh at an untagged commit: got = a KIT, want = none"
install "$tmp/dev" --dev || { cat "$tmp/install.err" >&2; fail "kit/install.sh --dev at an untagged commit failed"; }
grep -q -- "--dev: .* is not tag v" "$tmp/install.err" \
  || { cat "$tmp/install.err" >&2; fail "kit/install.sh --dev: got = no note, want = one"; }
# The package's version is its tag's, which is the counter's --version.
version=$("$tmp/dev/bin/verus-count" --version | sed 's/^verus-count //')
[ -n "$version" ] || fail "verus-count --version printed no version"
# At the tag, as a port's pin names it: installed, with nothing to note.
git -C "$kitrepo" update-ref "refs/tags/v$version" HEAD
root=$tmp/root
home=$root/usr/share/proved-port
install "$root/usr" || { cat "$tmp/install.err" >&2; fail "kit/install.sh at the tag failed"; }
! grep -q "kit/install.sh: .*tag v" "$tmp/install.err" \
  || { cat "$tmp/install.err" >&2; fail "kit/install.sh at the tag: got = a note, want = none"; }
expect "kit/install.sh records the commit and the counter's release in KIT" \
  "$(sed 1d "$home/KIT")" "commit $sha
tag v$version"
# CHANGELOG.md beside KIT, as kit/ has it, with the porting notes of the
# release KIT records.
cmp -s kit/CHANGELOG.md "$home/CHANGELOG.md" \
  || fail "kit/install.sh: got = no CHANGELOG.md or another one, want = kit/CHANGELOG.md in $home"
expect "the installed CHANGELOG.md is mode 0644" "$(find "$home/CHANGELOG.md" -perm 644)" "$home/CHANGELOG.md"
expect "the installed CHANGELOG.md has porting notes for v$version" \
  "$(awk -v v="## $version" '$0 == v { s = 1; next } /^## / { s = 0 } s && $0 == "### Porting notes" { print "yes"; exit }' "$home/CHANGELOG.md")" "yes"
# At the tag with a change under kit/: refused.
: > "$kitrepo/kit/stray"
if install "$tmp/changed"; then
  fail "kit/install.sh with a change under kit/: got = success, want = refused"
fi
grep -q "kit/ has changes not in tag v" "$tmp/install.err" \
  || { cat "$tmp/install.err" >&2; fail "kit/install.sh with a change under kit/: the refusal does not say why"; }
rm "$kitrepo/kit/stray"
echo "ok kit/install.sh installs the release's tag, and refuses another commit or a changed kit/ but with --dev"

# Over the --dev install, at the tag. An install that fails part way (here
# at its bin/, a file) leaves no KIT, which every kit target refuses, not
# the --dev install's KIT over a mix of files.
devhome=$tmp/dev/share/proved-port
[ -f "$devhome/KIT" ] || fail "the --dev install has no KIT"
mv "$tmp/dev/bin" "$tmp/dev-bin.saved"
: > "$tmp/dev/bin"
if install "$tmp/dev"; then
  fail "kit/install.sh, a reinstall that cannot write bin/: got = success, want = failure"
fi
[ ! -e "$devhome/KIT" ] || fail "kit/install.sh, a failed reinstall: got = the earlier KIT, want = none"
rm "$tmp/dev/bin"
mv "$tmp/dev-bin.saved" "$tmp/dev/bin"
# A reinstall that succeeds replaces make/ and templates/ whole: a file
# the earlier install had and this tree does not is gone.
: > "$devhome/make/stale.mk"
: > "$devhome/templates/stale"
install "$tmp/dev" || { cat "$tmp/install.err" >&2; fail "kit/install.sh over the --dev install, at the tag, failed"; }
[ ! -e "$devhome/make/stale.mk" ] && [ ! -e "$devhome/templates/stale" ] \
  || fail "kit/install.sh over an earlier install: got = its stale make/ and templates/ files, want = none"
expect "kit/install.sh over the --dev install records the tag in KIT" \
  "$(sed 1d "$devhome/KIT")" "commit $sha
tag v$version"
echo "ok kit/install.sh over an earlier install drops its KIT first and replaces its files whole"
PATH=$root/usr/bin:$PATH
export PATH

# --- the fixture port -----------------------------------------------------
# fixture DIR: a port with one crate, verified and unverified functions,
# FLOOR at 0.0 and a count.toml with [floor].
fixture() {
  mkdir -p "$1/crates/a/src" "$1/verus"
  printf '[package]\nname = "a"\nversion = "0.0.0"\nedition = "2021"\n\n[package.metadata.verus]\nverify = true\n' > "$1/crates/a/Cargo.toml"
  printf 'pub fn f() {}\n\nverus! {\npub fn g() {}\n\npub fn h(x: u8) -> u8 {\n    x\n}\n}\n\npub fn i() {}\n' > "$1/crates/a/src/lib.rs"
  printf '[crates]\ndir = "crates"\n\n[floor]\npath = "verus/FLOOR"\n' > "$1/verus/count.toml"
  printf 'functions 0.0\nlines 0.0\nchecked-functions 0.0\nchecked-lines 0.0\n' > "$1/verus/FLOOR"
}

installed=$tmp/installed
fixture "$installed"
cat > "$installed/Makefile" <<EOF
VERUS_CRATES = a
PROVED_PORT_VERSION = $version
PROVED_PORT_HOME ?= $home
include \$(PROVED_PORT_HOME)/make/proved-port.mk
EOF

# A counter of the installed release at another path, for VERUS_COUNT.
mkdir "$tmp/opt"
printf '#!/bin/sh\nexec %s "$@"\n' "$root/usr/bin/verus-count" > "$tmp/opt/verus-count"
chmod +x "$tmp/opt/verus-count"

# shows PORT TARGET [VAR=VALUE...]: `make -n TARGET` in PORT, each run of
# spaces one space.
shows() {
  _port=$1
  shift
  # CARGO_TARGET_DIR= keeps the build cache this script may set (above)
  # out of the commands shown.
  make -s -n -C "$_port" CARGO_TARGET_DIR= "$@" | tr -s ' ' | sed 's/ $//'
}

# --- the counter runs, unbuilt -------------------------------------------
make -s -C "$installed" verus-count VERUS_BASE=none VERUS_RLIMIT= VERUS_COUNT_FLAGS=--list > "$tmp/installed.out" 2> "$tmp/installed.err" \
  || { cat "$tmp/installed.err" >&2; fail "make verus-count"; }
grep -q '^shipped' "$tmp/installed.out" || { cat "$tmp/installed.out" >&2; fail "make verus-count printed no shipped row"; }
[ ! -s "$tmp/installed.err" ] || { cat "$tmp/installed.err" >&2; fail "make verus-count: got = output on stderr (a build?), want = none"; }
(cd "$installed" && "$root/usr/bin/verus-count" --floor verus/FLOOR --no-base --list .) > "$tmp/direct.out" \
  || fail "the installed verus-count run by hand"
expect "make verus-count prints what the installed counter run by hand prints" \
  "$(cat "$tmp/installed.out")" "$(cat "$tmp/direct.out")"
echo "ok make verus-count builds nothing"

# --- make -n ----------------------------------------------------------------
expect "make -n verus: verus-count from PATH, no build" \
  "$(shows "$installed" verus VERUS_BASE=abc123)" \
  "[ -f target/CACHEDIR.TAG ] || { mkdir -p target && printf '%s\\n' 'Signature: 8a477f597d28d172789f06886806bc55' '# This file is a cache directory tag created by cargo.' '# For information about cache directory tags see https://bford.info/cachedir/' > target/CACHEDIR.TAG; }
mkdir -p target/
cargo clean --locked -p a --target-dir target/verus-partial
cargo verus focus --locked -p a -j 1 -- --time-expanded --output-json > target/verus-stats.json
verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --base-rev abc123 ."
mkdir -p "$installed/verus/count"
echo '[package]' > "$installed/verus/count/Cargo.toml"
expect "make -n verus-count, with a verus/count/Cargo.toml in the port" \
  "$(shows "$installed" verus-count VERUS_BASE=none)" \
  "verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --no-base ."
rm -r "$installed/verus/count"
expect "make -n verus-count, VERUS_COUNT given" \
  "$(shows "$installed" verus-count VERUS_BASE=none VERUS_COUNT="$tmp/opt/verus-count")" \
  "$tmp/opt/verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --no-base ."
expect "make verus-count-test runs no cargo test" \
  "$(make -s -C "$installed" verus-count-test)" \
  "verus-count-test: verus-count is not built here; its package tests it"
expect "make -n kani-list: the package's kani.sh" \
  "$(shows "$installed" kani-list)" "sh $home/make/kani.sh --list"
expect "make -n lint-unsafe: the package's lint-unsafe.sh" \
  "$(shows "$installed" lint-unsafe UNSAFE_CRATES=c)" \
  "UNSAFE_CRATES='c' UNSAFE_WORKSPACE_LINTS='yes' UNSAFE_MANIFEST='' UNSAFE_CHECK_FLAGS='' sh $home/make/lint-unsafe.sh ."
if [ ! -f "$home/make/lint-unsafe.sh" ] || ! sh "$home/make/lint-unsafe.sh" --self-test > "$tmp/out"; then
  cat "$tmp/out" >&2 || true
  fail "the installed lint-unsafe.sh: got = missing or a failing self-test, want = its self-test passes"
fi
echo "ok the installed lint-unsafe.sh passes its self-test"
expect "make -n noalias: the package's noalias.sh" \
  "$(shows "$installed" noalias NOALIAS_CRATE=c NOALIAS_TYPE=T)" \
  "NOALIAS_CRATE='c' NOALIAS_TYPE='T' NOALIAS_BUILDS='default' NOALIAS_FLAGS_default='' NOALIAS_CARGO_FLAGS='' NOALIAS_TARGET_DIR='target' sh $home/make/noalias.sh ."
if [ ! -f "$home/make/noalias.sh" ] || ! sh "$home/make/noalias.sh" --self-test > "$tmp/out"; then
  cat "$tmp/out" >&2 || true
  fail "the installed noalias.sh: got = missing or a failing self-test, want = its self-test passes"
fi
echo "ok the installed noalias.sh passes its self-test"
expect "make -n codegen: the package's codegen.sh" \
  "$(shows "$installed" codegen CODEGEN_LIB=libc.so)" \
  "CODEGEN_LIB='libc.so' CODEGEN_BUILDS='default' CODEGEN_FLAGS_default='' CODEGEN_CARGO_FLAGS='' CODEGEN_TARGET_DIR='target' CODEGEN_FNCMP='fncmp' CODEGEN_BASE='' sh $home/make/codegen.sh base"
# codegen.sh refuses a run with no CODEGEN_LIB before it builds anything.
if [ ! -f "$home/make/codegen.sh" ]; then
  fail "the installed codegen.sh: got = missing, want = installed"
fi
if sh "$home/make/codegen.sh" capture "$tmp/codegen" > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 2 ] || ! grep -qF 'CODEGEN_LIB is empty' "$tmp/out"; then
  cat "$tmp/out" >&2
  fail "the installed codegen.sh, no CODEGEN_LIB: got = exit $rc, want = exit 2 naming CODEGEN_LIB"
fi
echo "ok the installed codegen.sh refuses a run with no CODEGEN_LIB"
expect "make -n perf-budget: the package's perf-budget.sh" \
  "$(shows "$installed" perf-budget PERF_BUDGET_LIB=libc.so PERF_BUDGET_TABLE_default=t.txt)" \
  "PERF_BUDGET_LIB='libc.so' PERF_BUDGET_BUILDS='default' PERF_BUDGET_TABLE_default='t.txt' PERF_BUDGET_FLAGS_default='' PERF_BUDGET_CARGO_FLAGS='' PERF_BUDGET_TARGET_DIR='target' PERF_BUDGET_SELFTEST_EXPORT='' sh $home/make/perf-budget.sh check"
# perf-budget.sh builds its checker from the perf-budget.rs installed
# beside it, and refuses a run with no PERF_BUDGET_LIB before any build.
for f in perf-budget.sh perf-budget.rs; do
  [ -f "$home/make/$f" ] || fail "the installed $f: got = missing, want = installed"
done
if sh "$home/make/perf-budget.sh" check > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 2 ] || ! grep -qF 'PERF_BUDGET_LIB is empty' "$tmp/out"; then
  cat "$tmp/out" >&2
  fail "the installed perf-budget.sh, no PERF_BUDGET_LIB: got = exit $rc, want = exit 2 naming PERF_BUDGET_LIB"
fi
echo "ok the installed perf-budget.sh and perf-budget.rs; the script refuses a run with no PERF_BUDGET_LIB"
expect "make -n skip-summary: the package's skip-summary.sh" \
  "$(shows "$installed" skip-summary SKIP_LOG=test.log SKIP_MANIFEST=skips.txt)" \
  "sh $home/make/skip-summary.sh 'test.log' 'skips.txt'"
# The installed script runs: a log whose one skip the manifest expects.
[ -f "$home/make/skip-summary.sh" ] || fail "the installed skip-summary.sh: got = missing, want = installed"
printf 'SKIP c_mseal no mseal\n' > "$tmp/skip.log"
printf 'c_mseal\n' > "$tmp/skips.txt"
if sh "$home/make/skip-summary.sh" "$tmp/skip.log" "$tmp/skips.txt" > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 0 ] || ! grep -qF "skips match $tmp/skips.txt" "$tmp/out"; then
  cat "$tmp/out" >&2
  fail "the installed skip-summary.sh, an expected skip: got = exit $rc, want = exit 0 with skips match"
fi
echo "ok the installed skip-summary.sh passes an expected skip"
expect "make -n glibc-max: the package's glibc-max.sh" \
  "$(shows "$installed" glibc-max GLIBC_MAX_LIB=libc.so GLIBC_MAX=2.44)" \
  "sh $home/make/glibc-max.sh 'libc.so' '2.44'"
# The installed script runs: a missing library fails, naming it (the
# check's environment may have no readelf, so it reads no ELF here).
[ -f "$home/make/glibc-max.sh" ] || fail "the installed glibc-max.sh: got = missing, want = installed"
if sh "$home/make/glibc-max.sh" "$tmp/no.so" 2.44 > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 1 ] || ! grep -qxF "error: $tmp/no.so missing" "$tmp/out"; then
  cat "$tmp/out" >&2
  fail "the installed glibc-max.sh, a missing library: got = exit $rc, want = exit 1 naming it"
fi
echo "ok the installed glibc-max.sh fails a missing library"
expect "make -n known-failures: the package's known-failures.sh" \
  "$(shows "$installed" known-failures KNOWN_FAILURES_LOG=test.log KNOWN_FAILURES_LIST=known.txt)" \
  "sh $home/make/known-failures.sh 'test.log' 'known.txt'"
# The installed script runs: a log whose one failure the list knows.
[ -f "$home/make/known-failures.sh" ] || fail "the installed known-failures.sh: got = missing, want = installed"
printf 'test api::x ... FAILED\n' > "$tmp/kf.log"
printf 'api::x\n' > "$tmp/known.txt"
if sh "$home/make/known-failures.sh" "$tmp/kf.log" "$tmp/known.txt" > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 0 ] || ! grep -qxF "warning: known failure ($tmp/known.txt): api::x" "$tmp/out"; then
  cat "$tmp/out" >&2
  fail "the installed known-failures.sh, a known failure: got = exit $rc, want = exit 0 with its warning"
fi
echo "ok the installed known-failures.sh passes a known failure"
expect "make -n header-check: the package's header-check.sh" \
  "$(shows "$installed" header-check HEADER_CHECK_HEADERS=include/foo.h HEADER_CHECK_PREFIXES=FOO_)" \
  "sh $home/make/header-check.sh '/usr/include' 'FOO_' 'include/foo.h'"
# The installed script runs: a header whose one definition is upstream's.
[ -f "$home/make/header-check.sh" ] || fail "the installed header-check.sh: got = missing, want = installed"
mkdir -p "$tmp/hc/up" "$tmp/hc/ours"
printf '#define FOO_X 1\n' > "$tmp/hc/up/foo.h"
printf '#define  FOO_X 1 /* ours */\n' > "$tmp/hc/ours/foo.h"
if sh "$home/make/header-check.sh" "$tmp/hc/up" FOO_ "$tmp/hc/ours/foo.h" > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 0 ] || ! grep -qxF "ok   foo.h: 1 definitions match $tmp/hc/up/foo.h" "$tmp/out"; then
  cat "$tmp/out" >&2
  fail "the installed header-check.sh, a matching header: got = exit $rc, want = exit 0 with its ok line"
fi
echo "ok the installed header-check.sh passes a matching header"
expect "make -n abi-elf: the package's abi-elf.sh" \
  "$(shows "$installed" abi-elf ABI_ELF_LIB=libfoo.so.1 ABI_ELF_MAP=foo.map ABI_ELF_SONAME=libfoo.so.1)" \
  "sh $home/make/abi-elf.sh 'libfoo.so.1' 'foo.map' 'libfoo.so.1' 'libc.so.6 ld-linux-x86-64.so.2 ld-linux-aarch64.so.1'"
# The installed script runs: a missing library fails naming it, before
# readelf runs (kit/abi-elf-test.sh runs it on built libraries).
[ -f "$home/make/abi-elf.sh" ] || fail "the installed abi-elf.sh: got = missing, want = installed"
if sh "$home/make/abi-elf.sh" "$tmp/no.so" "$tmp/no.map" libfoo.so.1 libc.so.6 > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 1 ] || ! grep -qxF "error: $tmp/no.so missing" "$tmp/out"; then
  cat "$tmp/out" >&2
  fail "the installed abi-elf.sh, a missing library: got = exit $rc, want = exit 1 naming it"
fi
echo "ok the installed abi-elf.sh fails a missing library"
expect "make -n kani-quotes: the package's kani-quotes.sh" \
  "$(shows "$installed" kani-quotes)" \
  "sh $home/make/kani-quotes.sh 'verus/seams' '.'"
# The installed script runs: a record whose one quoted line is its
# source's.
[ -f "$home/make/kani-quotes.sh" ] || fail "the installed kani-quotes.sh: got = missing, want = installed"
mkdir -p "$tmp/kq/seams" "$tmp/kq/src"
printf 'pub fn get(v: &Vec<u8>) -> (r: u8)\n' > "$tmp/kq/src/a.rs"
# shellcheck disable=SC2016 # the record's Markdown backquotes are literal text
printf '# `get`\n\nSeam S1: `get` (`src/a.rs`).\n\n## 1. Contract\n\n```rust\npub fn get(v: &Vec<u8>) -> (r: u8)\n```\n' > "$tmp/kq/seams/kani-s1.md"
if (cd "$tmp/kq" && sh "$home/make/kani-quotes.sh" seams .) > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 0 ] || ! grep -qxF "ok   kani-s1.md: 1 chunks match ./src/a.rs" "$tmp/out"; then
  cat "$tmp/out" >&2
  fail "the installed kani-quotes.sh, a matching record: got = exit $rc, want = exit 0 with its ok line"
fi
echo "ok the installed kani-quotes.sh passes a matching record"
expect "make -n deviations: the package's deviations.sh" \
  "$(shows "$installed" deviations DEVIATIONS_RESULTS=res DEVIATIONS_LIST=dev.txt)" \
  "sh $home/make/deviations.sh 'res' 'dev.txt' 'docs/abi'"
# The installed script runs: one listed failure citing an active entry.
[ -f "$home/make/deviations.sh" ] || fail "the installed deviations.sh: got = missing, want = installed"
mkdir -p "$tmp/dv/abi"
printf 'PASS a\nFAIL b\n' > "$tmp/dv/res"
printf 'b ABI-001: a known difference\n' > "$tmp/dv/dev.txt"
printf '### ABI-001: x\n\n- **Status:** active\n' > "$tmp/dv/abi/ABI-001.md"
if (cd "$tmp/dv" && sh "$home/make/deviations.sh" res dev.txt abi) > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 0 ] || ! grep -qxF "PASS: the failing keys are exactly the 1 listed in dev.txt, each with its reason" "$tmp/out"; then
  cat "$tmp/out" >&2
  fail "the installed deviations.sh, a listed failure: got = exit $rc, want = exit 0 with its PASS line"
fi
echo "ok the installed deviations.sh passes a listed failure"
expect "make -n known-cites: the package's known-cites.sh" \
  "$(shows "$installed" known-cites KNOWN_CITES_FILE=known.c KNOWN_CITES_REPO=o/port)" \
  "KNOWN_CITES_OFFLINE='' KNOWN_CITES_MACRO='' sh $home/make/known-cites.sh 'known.c' 'docs/abi' 'o/port'"
# The installed script runs, reading the ledger with the installed
# ledger-active.sh beside it: one row citing an active entry.
[ -f "$home/make/known-cites.sh" ] || fail "the installed known-cites.sh: got = missing, want = installed"
[ -f "$home/make/ledger-active.sh" ] || fail "the installed ledger-active.sh: got = missing, want = installed"
mkdir -p "$tmp/kc/abi"
printf '#define KNOWN(c, p) {c, p}\n    KNOWN("ABI-001", a),\n' > "$tmp/kc/known.c"
printf '### ABI-001: x\n\n- **Status:** active\n' > "$tmp/kc/abi/ABI-001.md"
if (cd "$tmp/kc" && sh "$home/make/known-cites.sh" known.c abi o/port) > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 0 ] || ! grep -qxF "PASS: 1 rows of known.c, each citing an active entry of abi or an open issue" "$tmp/out"; then
  cat "$tmp/out" >&2
  fail "the installed known-cites.sh, a cited row: got = exit $rc, want = exit 0 with its PASS line"
fi
echo "ok the installed known-cites.sh passes a cited row"
# Through make, with blank KNOWN_CITES_MACRO and KNOWN_CITES_OFFLINE on
# the command line (which make exports to the recipe as given): the
# script gets them stripped, reads them as unset, and passes.
if make -s -C "$installed" known-cites KNOWN_CITES_FILE="$tmp/kc/known.c" KNOWN_CITES_LEDGER="$tmp/kc/abi" KNOWN_CITES_REPO=o/port KNOWN_CITES_MACRO=' ' KNOWN_CITES_OFFLINE=' ' PROVED_PORT_VERSION="$version" > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 0 ] || ! grep -qxF "PASS: 1 rows of $tmp/kc/known.c, each citing an active entry of $tmp/kc/abi or an open issue" "$tmp/out"; then
  cat "$tmp/out" >&2
  fail "make known-cites with blank KNOWN_CITES_MACRO and KNOWN_CITES_OFFLINE: got = exit $rc, want = exit 0 with its PASS line"
fi
echo "ok make known-cites with blank KNOWN_CITES_MACRO and KNOWN_CITES_OFFLINE"

# --- PROVED_PORT_VERSION -------------------------------------------------
for t in verus verus-proofs verus-count verus-count-test verus-headers kani kani-list kani-regular lint-unsafe noalias codegen codegen-capture codegen-compare perf-budget perf-budget-print skip-summary glibc-max known-failures header-check abi-elf kani-quotes deviations known-cites verify; do
  make -s -n -C "$installed" "$t" VERUS_BASE=none UNSAFE_CRATES=c NOALIAS_CRATE=c NOALIAS_TYPE=T CODEGEN_LIB=libc.so PERF_BUDGET_LIB=libc.so SKIP_LOG=test.log SKIP_MANIFEST=skips.txt GLIBC_MAX_LIB=libc.so GLIBC_MAX=2.44 KNOWN_FAILURES_LOG=test.log KNOWN_FAILURES_LIST=known.txt HEADER_CHECK_HEADERS=include/foo.h HEADER_CHECK_PREFIXES=FOO_ ABI_ELF_LIB=libc.so ABI_ELF_MAP=c.map ABI_ELF_SONAME=libc.so DEVIATIONS_RESULTS=res DEVIATIONS_LIST=dev.txt KNOWN_CITES_FILE=known.c KNOWN_CITES_REPO=o/port PROVED_PORT_VERSION="$version" > /dev/null 2> "$tmp/err" \
    || { cat "$tmp/err" >&2; fail "make -n $t, PROVED_PORT_VERSION=$version (KIT's): got = failure, want = success"; }
  if make -s -n -C "$installed" "$t" VERUS_BASE=none PROVED_PORT_VERSION="$other" > "$tmp/out" 2> "$tmp/err"; then
    fail "make -n $t, PROVED_PORT_VERSION=$other: got = success, want = failure"
  fi
  grep -q "PROVED_PORT_VERSION is $other, but $home/KIT records tag v$version" "$tmp/err" \
    || { cat "$tmp/err" >&2; fail "make -n $t: the mismatch message does not name both versions"; }
  [ ! -s "$tmp/out" ] || { cat "$tmp/out" >&2; fail "make -n $t, a mismatched version: got = commands, want = none"; }
  # No pin, as when a Makefile drops or misspells the line.
  if make -s -n -C "$installed" "$t" VERUS_BASE=none PROVED_PORT_VERSION= > "$tmp/out" 2> "$tmp/err"; then
    fail "make -n $t, PROVED_PORT_VERSION empty: got = success, want = failure"
  fi
  grep -q "PROVED_PORT_VERSION is not set" "$tmp/err" \
    || { cat "$tmp/err" >&2; fail "make -n $t, no pin: the message does not say so"; }
  [ ! -s "$tmp/out" ] || { cat "$tmp/out" >&2; fail "make -n $t, no pin: got = commands, want = none"; }
  echo "ok make -n $t: PROVED_PORT_VERSION"
done
sed 's/^PROVED_PORT_VERSION = /PROVED_PORT_VERISON = /' "$installed/Makefile" > "$tmp/misspelt"
cp "$installed/Makefile" "$tmp/makefile.saved"
cp "$tmp/misspelt" "$installed/Makefile"
if make -s -n -C "$installed" verus-count VERUS_BASE=none > /dev/null 2> "$tmp/err"; then
  fail "make -n verus-count, a misspelt PROVED_PORT_VERSION line: got = success, want = failure"
fi
grep -q "PROVED_PORT_VERSION is not set" "$tmp/err" || { cat "$tmp/err" >&2; fail "a misspelt pin: the message does not say it is not set"; }
cp "$tmp/makefile.saved" "$installed/Makefile"
echo "ok make -n verus-count fails a misspelt pin line"
mv "$home/KIT" "$tmp/kit-file.saved"
for pin in "PROVED_PORT_VERSION=$version" "PROVED_PORT_VERSION="; do
  if make -s -n -C "$installed" verus-count VERUS_BASE=none "$pin" > /dev/null 2> "$tmp/err"; then
    fail "make -n verus-count, no KIT, $pin: got = success, want = failure"
  fi
  grep -q "KIT records no tag" "$tmp/err" || { cat "$tmp/err" >&2; fail "no KIT, $pin: the message does not say so"; }
done
mv "$tmp/kit-file.saved" "$home/KIT"
echo "ok a missing KIT fails verus-count, pin or no pin"

# Another counter first on PATH, of another version: the targets that run
# the counter fail on its --version; the others pass on KIT alone.
mkdir "$tmp/stale"
# Another release, and one whose version only starts with the pinned one.
for stale in "$other" "${version}1"; do
  printf '#!/bin/sh\necho "verus-count %s"\n' "$stale" > "$tmp/stale/verus-count"
  chmod +x "$tmp/stale/verus-count"
  for t in verus verus-count; do
    if PATH=$tmp/stale:$PATH make -s -n -C "$installed" "$t" VERUS_BASE=none PROVED_PORT_VERSION="$version" > "$tmp/out" 2> "$tmp/err"; then
      fail "make -n $t, verus-count $stale first on PATH: got = success, want = failure"
    fi
    grep -q "$home/KIT records tag v$version, but \`verus-count --version\` prints verus-count $stale" "$tmp/err" \
      || { cat "$tmp/err" >&2; fail "make -n $t, verus-count $stale: the message does not name its version"; }
    [ ! -s "$tmp/out" ] || { cat "$tmp/out" >&2; fail "make -n $t, verus-count $stale: got = commands, want = none"; }
    echo "ok make -n $t fails verus-count $stale first on PATH"
  done
done
PATH=$tmp/stale:$PATH make -s -n -C "$installed" kani-list PROVED_PORT_VERSION="$version" > /dev/null \
  || fail "make -n kani-list, a stale counter: got = failure, want = success (it runs no counter)"

# --- make verus-headers --------------------------------------------------
# held PORT: PORT's copy of each Verus file VERUS-FILES names, taken from
# the installed templates, as an installed port has it.
held() {
  grep -v -e '^#' -e '^[[:space:]]*$' "$home/make/VERUS-FILES" | while IFS= read -r f; do
    mkdir -p "$1/$(dirname "$f")"
    cp "$home/templates/$f" "$1/$f"
  done
}
hdr_installed=$tmp/hdr-installed
mkdir "$hdr_installed"
cp "$installed/Makefile" "$hdr_installed/Makefile"
held "$hdr_installed"
! grep -rq '^# kit:' "$hdr_installed/verus" "$hdr_installed/.chainguard" \
  || fail "the installed fixture has a # kit: line: got = an installed template with one, want = none"

# headers PORT [VAR=VALUE...]: make verus-headers in PORT, output in
# $tmp/out (both streams), status in $hdr_rc.
headers() {
  _port=$1
  shift
  hdr_rc=0
  make -s -C "$_port" verus-headers "$@" > "$tmp/out" 2>&1 || hdr_rc=$?
}
headers "$hdr_installed" PROVED_PORT_VERSION="$version"
[ "$hdr_rc" -eq 0 ] || { cat "$tmp/out" >&2; fail "make verus-headers: got = failure, want = success"; }
[ "$(grep -c '^ok ' "$tmp/out")" -eq "$(grep -cv -e '^#' -e '^[[:space:]]*$' "$home/make/VERUS-FILES")" ] \
  || { cat "$tmp/out" >&2; fail "make verus-headers: got = not every listed file ok, want = all"; }

# The port's own old verus/make/templates and verus/count/KIT are not read.
mkdir -p "$hdr_installed/verus/make/templates/verus" "$hdr_installed/verus/count"
echo '# something the kit never said' > "$hdr_installed/verus/make/templates/verus/FLOOR.tmpl"
printf 'commit 0000000000000000000000000000000000000000\ntag v0.0.0\n' > "$hdr_installed/verus/count/KIT"
headers "$hdr_installed"
[ "$hdr_rc" -eq 0 ] || { cat "$tmp/out" >&2; fail "make verus-headers, a stale vendored copy in the port: got = failure, want = success (it reads the installed kit)"; }
rm -r "$hdr_installed/verus/make" "$hdr_installed/verus/count"
echo "ok make verus-headers reads the installed templates and KIT"

# A file the installed directory holds but VERUS-FILES does not name is
# not held, however the port has reshaped it.
[ -f "$home/templates/.chainguard/deps.yaml" ] || fail "the install has no .chainguard/deps.yaml template"
mkdir -p "$hdr_installed/.chainguard"
echo '# the port reshaped this' > "$hdr_installed/.chainguard/deps.yaml"
headers "$hdr_installed"
[ "$hdr_rc" -eq 0 ] || { cat "$tmp/out" >&2; fail "make verus-headers, a reshaped deps.yaml: got = failure, want = success"; }
! grep -q 'deps.yaml' "$tmp/out" || { cat "$tmp/out" >&2; fail "make verus-headers: it held deps.yaml"; }
echo "ok make verus-headers holds only what VERUS-FILES names"

# Drift in the port's file fails, naming the installed template.
cp "$hdr_installed/verus/FLOOR" "$tmp/floor.saved"
awk 'NR == 2 { print "# something the kit never said"; next } { print }' "$tmp/floor.saved" > "$hdr_installed/verus/FLOOR"
headers "$hdr_installed"
[ "$hdr_rc" -ne 0 ] || fail "make verus-headers, a drifted verus/FLOOR: got = success, want = failure"
grep -q "headers: verus/FLOOR: the header drifted from the kit's ($home/templates/verus/FLOOR)" "$tmp/out" \
  || { cat "$tmp/out" >&2; fail "make verus-headers, drift: the message does not name the installed template"; }
cp "$tmp/floor.saved" "$hdr_installed/verus/FLOOR"
echo "ok make verus-headers fails a drifted header"

# A `# kit:` line a port still carries ends its header and is
# not read, whatever commit it names.
awk -v line="# kit: proved-port v0.0.0 (1234567890abcdef)" '!done && !/^#/ { print line; done = 1 } { print }' \
  "$tmp/floor.saved" > "$hdr_installed/verus/FLOOR"
grep -q '^# kit: proved-port v0.0.0 (1234567890abcdef)$' "$hdr_installed/verus/FLOOR" || fail "the stale kit line was not planted"
headers "$hdr_installed"
[ "$hdr_rc" -eq 0 ] || { cat "$tmp/out" >&2; fail "make verus-headers, a stale kit line: got = failure, want = success (not read)"; }
cp "$tmp/floor.saved" "$hdr_installed/verus/FLOOR"
echo "ok make verus-headers needs no kit line and reads none"

# A port moving from a vendored copy, with its own comment below the kit
# line: replacing the line with a blank line keeps that comment out of
# the header and passes; deleting it outright puts the comment in the
# header, which then drifts from the template.
awk -v kit="# kit: proved-port v0.0.0 (1234567890abcdef)" \
  '!done && !/^#/ { print kit; print "#"; print "# The port'"'"'s own note."; done = 1 } { print }' \
  "$tmp/floor.saved" > "$tmp/floor.vendored"
grep -q "^# The port's own note.$" "$tmp/floor.vendored" || fail "the port's own note was not planted"
awk '/^# kit: proved-port / { print ""; next } { print }' "$tmp/floor.vendored" > "$hdr_installed/verus/FLOOR"
headers "$hdr_installed"
[ "$hdr_rc" -eq 0 ] || { cat "$tmp/out" >&2; fail "make verus-headers, installed, kit line replaced by a blank line: got = failure, want = success"; }
sed '/^# kit: proved-port /d' "$tmp/floor.vendored" > "$hdr_installed/verus/FLOOR"
headers "$hdr_installed"
[ "$hdr_rc" -ne 0 ] || fail "make verus-headers, installed, kit line deleted with the port's comment below it: got = success, want = failure"
grep -q "headers: verus/FLOOR: the header drifted from the kit's" "$tmp/out" \
  || { cat "$tmp/out" >&2; fail "make verus-headers, installed, port comment in the header: the message does not say it drifted"; }
cp "$tmp/floor.saved" "$hdr_installed/verus/FLOOR"
echo "ok make verus-headers, installed, a kit line becomes a blank line where the port's comments follow it"

# A template VERUS-FILES names that is not installed fails.
mv "$home/templates/verus/RLIMIT" "$tmp/rlimit.saved"
headers "$hdr_installed"
[ "$hdr_rc" -ne 0 ] || fail "make verus-headers, a listed template missing: got = success, want = failure"
grep -q "names verus/RLIMIT, but $home/templates/verus/RLIMIT is not installed" "$tmp/out" \
  || { cat "$tmp/out" >&2; fail "make verus-headers, a missing template: the message does not say so"; }
mv "$tmp/rlimit.saved" "$home/templates/verus/RLIMIT"
echo "ok make verus-headers fails a listed template not installed"

echo "ok kit $sha installed as the package installs it"
echo PASS
