#!/bin/sh
# kit/bench-test.sh: tests of kit/make/bench.sh (the bench-test check).
# It builds a reference library libfoo.so.1 with cc, a stub cargo whose
# build writes the port's libfoo.so.1 and prints the rustc command a
# test gives it, and a bench program that prints the rows a rows file
# lists for the library it loaded, then checks the script's exit status
# and output, spelled out by hand from the PCRE2 port's ci/bench.sh's
# format (none is taken from a run of the script under test):
#   - a clean run: the swap (each side's version line), the arguments
#     each side gets, the table with rates, times and return codes (a
#     differing one allowed), the result files, the work lib holding
#     only BENCH_SO's files, BENCH_LINK run;
#   - MISMATCH: counts that differ, a count beside a return code, a
#     workload one side lacks in a mode;
#   - the program's lines: other than six fields, another side or mode,
#     an empty field, a value that is no number, a repeated workload,
#     no line, a failing program;
#   - the rustc flag check: a missing flag, the -CFLAG form, a longer
#     flag, a flag only on another crate's command, no command, a failed
#     build;
#   - suites: the environment under a prefix, the arguments, the exit
#     status, unknown and bad names;
#   - usage and set-up refusals: required variables, side, mode and
#     prefix names, BENCH_SO, BENCH_NEEDS, BENCH_LINK.
# Run it from the repository root. It needs a POSIX shell, BusyBox's
# tools and a C compiler with glibc.
set -eu

script=$(pwd)/kit/make/bench.sh
[ -f "$script" ] || { echo "FAIL: run it from the repository root (no kit/make/bench.sh)" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"
tmp=$(pwd)

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# ------------------------------------------------------------ the fixtures
mkdir ref bin port port/bench
cat > foo.c << 'EOF'
#ifndef SIDE
#error SIDE
#endif
const char *foo_side(void) { return SIDE; }
const char *foo_version(void) { return "foo " SIDE " 1.0"; }
EOF
cc -shared -fPIC -DSIDE='"ref"' -Wl,-soname,libfoo.so.1 -o ref/libfoo.so.1 foo.c
ln -s libfoo.so.1 ref/libfoo.so
# The program: --version prints the loaded library's version; --lib S
# --mode M prints, tab-separated, each row of $BENCH_TEST_ROWS whose
# first word is the loaded library's side and whose second is M, with
# the first word replaced by S (by $BENCH_TEST_SIDE when set), and its
# other arguments on stderr. --fail exits 3.
cat > port/bench/prog.c << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
const char *foo_side(void);
const char *foo_version(void);
int main(int argc, char **argv) {
  const char *lib = 0, *mode = 0, *rows, *side;
  char line[512];
  FILE *f;
  int i;
  if (argc == 2 && strcmp(argv[1], "--version") == 0) { puts(foo_version()); return 0; }
  fprintf(stderr, "prog: args:");
  for (i = 1; i < argc; i++) {
    if (strcmp(argv[i], "--lib") == 0 && i + 1 < argc) { lib = argv[++i]; continue; }
    if (strcmp(argv[i], "--mode") == 0 && i + 1 < argc) { mode = argv[++i]; continue; }
    if (strcmp(argv[i], "--fail") == 0) { fprintf(stderr, " --fail\n"); return 3; }
    fprintf(stderr, " %s", argv[i]);
  }
  fprintf(stderr, "\n");
  if (!lib || !mode || !(rows = getenv("BENCH_TEST_ROWS")) || !(f = fopen(rows, "r"))) return 2;
  side = getenv("BENCH_TEST_SIDE");
  while (fgets(line, sizeof line, f)) {
    char *w[8], *p;
    int n = 0;
    for (p = strtok(line, " \n"); p && n < 8; p = strtok(0, " \n")) w[n++] = p;
    if (n < 2 || strcmp(w[0], foo_side()) != 0 || strcmp(w[1], mode) != 0) continue;
    printf("%s", side ? side : lib);
    for (i = 1; i < n; i++) printf("\t%s", strcmp(w[i], "-") == 0 ? "" : w[i]);
    printf("\n");
  }
  return 0;
}
EOF
# The stub cargo: clean and build log their arguments; build writes the
# port's library into target/release (and a stale libbar there) and
# prints $BENCH_TEST_RUSTC, one command per line, or exits 101 when
# BENCH_TEST_BUILD_FAIL is set.
cat > bin/cargo << 'EOF'
#!/bin/sh
echo "cargo $*" >> "$BENCH_TEST_LOG"
case $1 in
  clean) exit 0 ;;
  build)
    if [ -n "${BENCH_TEST_BUILD_FAIL:-}" ]; then echo "error: could not compile"; exit 101; fi
    mkdir -p target/release
    cc -shared -fPIC -DSIDE='"port"' -Wl,-soname,libfoo.so.1 -o target/release/libfoo.so.1 "$BENCH_TEST_FOO"
    printf '%s\n' "$BENCH_TEST_RUSTC" >&2
    exit 0 ;;
esac
exit 1
EOF
printf '#!/bin/sh\necho "rustc 1.99.0 (stub)"\n' > bin/rustc
chmod +x bin/cargo bin/rustc
PATH=$tmp/bin:$PATH
BENCH_TEST_LOG=$tmp/cargo.log
BENCH_TEST_FOO=$tmp/foo.c
export PATH BENCH_TEST_LOG BENCH_TEST_FOO

align='-C llvm-args=-align-all-functions=6 -C llvm-args=-align-loops=64'
good="     Running \`rustc --crate-name foo_capi --edition=2021 capi/src/lib.rs --crate-type cdylib -C opt-level=3 $align --cfg x\`"
other="     Running \`rustc --crate-name foo_core --edition=2021 core/src/lib.rs -C opt-level=3 $align\`"

# The rows: rates (MB/s), times (ns), equal return codes, and return
# codes that differ (allowed).
cat > rows << 'EOF'
ref interp rate 100 MB/s 7
ref jit rate 400 MB/s 7
port interp rate 200 MB/s 7
port jit rate 200 MB/s 7
ref interp time 50 ns 3
ref jit time 25 ns 3
port interp time 10 ns 3
port jit time 10 ns 3
ref interp cata 0.005 s rc=-47
ref jit cata 0.005 s rc=-47
port interp cata 0.005 s rc=-47
port jit cata 0.005 s rc=-47
ref interp cata-d 1 ms rc=-47
ref jit cata-d 1 ms rc=-46
port interp cata-d 1 ms rc=-47
port jit cata-d 1 ms rc=-47
EOF

# setup: the default environment of a run, from the port's root.
setup() {
  BENCH_SRC=bench/prog.c
  BENCH_PACKAGE=foo-capi
  BENCH_REF=foo
  BENCH_PORT=cg-foo
  BENCH_MODES='interp jit'
  BENCH_SO='target/release/libfoo.so*'
  BENCH_FLAGS='llvm-args=-align-all-functions=6 llvm-args=-align-loops=64'
  BENCH_LDLIBS="-L$tmp/ref -Wl,--enable-new-dtags -Wl,-rpath,$tmp/ref -lfoo"
  BENCH_REF_ARGS='--corpus a'
  BENCH_PORT_ARGS='--corpus b'
  BENCH_TEST_ROWS=$tmp/rows
  BENCH_TEST_RUSTC="$other
$good"
  export BENCH_SRC BENCH_PACKAGE BENCH_REF BENCH_PORT BENCH_MODES BENCH_SO BENCH_FLAGS \
    BENCH_LDLIBS BENCH_REF_ARGS BENCH_PORT_ARGS BENCH_TEST_ROWS BENCH_TEST_RUSTC
  unset BENCH_CRATE BENCH_LINK BENCH_NEEDS BENCH_PREFIX BENCH_SUITES BENCH_OUT BENCH_WORK \
    BENCH_REF_FILE BENCH_TEST_SIDE BENCH_TEST_BUILD_FAIL BENCH_CC BENCH_CFLAGS 2> /dev/null || true
  : > "$BENCH_TEST_LOG"
}

# go [ARGS...]: the script with ARGS from the port's root; its output
# (stdout and stderr) in out, its status in rc.
go() {
  if (cd port && sh "$script" "$@") > out 2>&1; then rc=0; else rc=$?; fi
}

# want_rc WHAT RC TEXT: the run exited RC and printed the line TEXT.
want_rc() {
  if [ "$rc" -ne "$2" ] || ! grep -qxF -- "$3" out; then
    printf 'got (exit %s):\n' "$rc" >&2
    cat out >&2
    printf 'want (exit %s) a line: %s\n' "$2" "$3" >&2
    fail "$1"
  fi
  echo "ok $1"
}

# want_table WHAT: the result file's table (after its args line) is
# exactly the file want.
want_table() {
  sed -n '/^args:/,$p' "port/target/remote-out/bench/cg-foo-bench-$(uname -m).txt" | sed 1d > got
  if ! cmp -s want got; then
    echo "got:" >&2
    cat got >&2
    echo "want:" >&2
    cat want >&2
    fail "$1"
  fi
  echo "ok $1"
}

# row FIELD...: one table row of two modes (four arms, two ratios).
row() { printf '%-26s %-5s %14s %14s %14s %14s %14s %14s  %s\n' "$@"; }

# ------------------------------------------------------------ a clean run
setup
mkdir -p port/target/bench/lib
: > port/target/bench/lib/stale.so
BENCH_LINK='touch target/linked'
export BENCH_LINK
go --size 1K
want_rc "a clean run passes" 0 "results: $tmp/port/target/remote-out/bench/cg-foo-bench-$(uname -m).tsv, $tmp/port/target/remote-out/bench/cg-foo-bench-$(uname -m).txt"
{
  row workload unit foo/interp foo/jit cg-foo/interp cg-foo/jit "vs foo/interp" "vs foo/jit" count
  row rate MB/s 100.0 400.0 200.0 200.0 2.00x 0.50x 7
  row time ns 50.0 25.0 10.0 10.0 5.00x 2.50x 3
  row cata s 5.00e-03 5.00e-03 5.00e-03 5.00e-03 1.00x 1.00x rc=-47
  row cata-d ms 1.000 1.000 1.000 1.000 1.00x 1.00x "rc differs (allowed)"
  echo "differs (allowed): cata-d: foo/interp=rc=-47 foo/jit=rc=-46 cg-foo/interp=rc=-47 cg-foo/jit=rc=-47"
  echo "vs foo/MODE: cg-foo/interp over foo in that mode, >1 is faster"
} > want
want_table "a clean run's table"
want_rc "the reference side's version line" 0 "foo: foo ref 1.0 (package unknown)"
want_rc "the port side's version line: the swapped library" 0 "cg-foo: foo port 1.0"
want_rc "the args line" 0 "args:      --size 1K"
want_rc "the reference side's arguments" 0 "prog: args: --corpus a --size 1K"
want_rc "the port side's arguments" 0 "prog: args: --corpus b --size 1K"
[ "$(wc -l < "port/target/remote-out/bench/cg-foo-bench-$(uname -m).tsv")" -eq 16 ] || fail "a clean run's tsv: got $(wc -l < "port/target/remote-out/bench/cg-foo-bench-$(uname -m).tsv") lines, want 16"
printf 'cargo clean --release -p foo-capi\ncargo build --locked --release -p foo-capi -v\n' > want
cmp -s want "$BENCH_TEST_LOG" || { cat "$BENCH_TEST_LOG" >&2; fail "cargo's arguments"; }
echo "ok cargo cleans and builds BENCH_PACKAGE"
got=
for e in port/target/bench/lib/* port/target/bench/lib/.[!.]*; do
  [ -e "$e" ] || [ -L "$e" ] || continue
  got="$got ${e##*/}"
done
[ "$got" = " libfoo.so.1" ] || fail "the work lib: got$got, want libfoo.so.1 only"
echo "ok the work lib holds only BENCH_SO's files"
[ -f port/target/linked ] || fail "BENCH_LINK did not run"
echo "ok BENCH_LINK runs"

# One mode, the default (`default`), and no per-side arguments.
setup
unset BENCH_MODES BENCH_REF_ARGS BENCH_PORT_ARGS
printf 'ref default time 40 ns 1\nport default time 20 ns 1\n' > rows1
BENCH_TEST_ROWS=$tmp/rows1
go
want_rc "one default mode passes" 0 "args:      (defaults)"
{
  printf '%-26s %-5s %14s %14s %14s  %s\n' workload unit foo/default cg-foo/default "vs foo/default" count
  printf '%-26s %-5s %14s %14s %14s  %s\n' time ns 40.0 20.0 2.00x 1
  echo "vs foo/MODE: cg-foo/default over foo in that mode, >1 is faster"
} > want
want_table "one default mode's table"

# ------------------------------------------------------------ MISMATCH
# mismatch WHAT ROWS LINE: a run over ROWS (the clean rows' first four
# lines plus ROWS) fails with the MISMATCH line LINE.
mismatch() {
  setup
  { head -n 4 rows; printf '%s\n' "$2"; } > rowsm
  BENCH_TEST_ROWS=$tmp/rowsm
  go
  want_rc "$1" 1 "$3"
  grep -qxF "bench: MISMATCH: the libraries disagree on a count" out || fail "$1: no closing MISMATCH line"
}
mismatch "counts that differ" "ref interp w 1 ns 5
ref jit w 1 ns 5
port interp w 1 ns 6
port jit w 1 ns 5" "MISMATCH w: counts differ: foo/interp=5 foo/jit=5 cg-foo/interp=6 cg-foo/jit=5"
mismatch "a count beside a return code" "ref interp w 1 ns 5
ref jit w 1 ns 5
port interp w 1 ns rc=-1
port jit w 1 ns 5" "MISMATCH w: some runs give a return code, some a count: foo/interp=5 foo/jit=5 cg-foo/interp=rc=-1 cg-foo/jit=5"
mismatch "a workload the port lacks in one mode" "ref interp w 1 ns 5
ref jit w 1 ns 5
port interp w 1 ns 5" "MISMATCH w: missing from one library: foo/interp=5 foo/jit=5 cg-foo/interp=5"
mismatch "a workload the reference lacks in one mode" "ref jit w 1 ns 5
port interp w 1 ns 5
port jit w 1 ns 5" "MISMATCH w: missing from one library: foo/jit=5 cg-foo/interp=5 cg-foo/jit=5"

# ------------------------------------------------------------ the program's lines
# badline WHAT ROW LINE: a run where the port's interp mode also prints
# ROW fails with LINE.
badline() {
  setup
  { cat rows; printf '%s\n' "$2"; } > rowsb
  BENCH_TEST_ROWS=$tmp/rowsb
  go
  want_rc "$1" 1 "$3"
}
tab=$(printf '\t')
badline "a line with five fields" "port interp w 1 ns" \
  "bench: the bench program's output for cg-foo interp: line 5 has 5 fields, want 6: cg-foo${tab}interp${tab}w${tab}1${tab}ns"
badline "a line with seven fields" "port interp w 1 ns 1 x" \
  "bench: the bench program's output for cg-foo interp: line 5 has 7 fields, want 6: cg-foo${tab}interp${tab}w${tab}1${tab}ns${tab}1${tab}x"
badline "an empty count" "port interp w 1 ns -" \
  "bench: the bench program's output for cg-foo interp: line 5 has an empty workload, unit or count: cg-foo${tab}interp${tab}w${tab}1${tab}ns${tab}"
badline "a value that is no number" "port interp w 1.5x ns 1" \
  "bench: the bench program's output for cg-foo interp: line 5 has value 1.5x, want a number: cg-foo${tab}interp${tab}w${tab}1.5x${tab}ns${tab}1"
badline "a repeated workload" "port interp time 9 ns 3" \
  "bench: the bench program's output for cg-foo interp: line 5 repeats workload time"
setup
BENCH_TEST_SIDE=foo
export BENCH_TEST_SIDE
go
want_rc "a line for another side" 1 "bench: the bench program's output for cg-foo interp: line 1 is for foo interp, want cg-foo interp: foo${tab}interp${tab}rate${tab}200${tab}MB/s${tab}7"
setup
grep -v '^port jit' rows > rowsn
BENCH_TEST_ROWS=$tmp/rowsn
go
want_rc "an arm with no line" 1 "bench: the bench program's output for cg-foo jit: no workload lines"
setup
go --fail
want_rc "a failing program" 1 "bench: the bench program failed for foo interp"

# ------------------------------------------------------------ the rustc flags
setup
BENCH_TEST_RUSTC="     Running \`rustc --crate-name foo_capi -C llvm-args=-align-all-functions=6\`"
go
want_rc "a missing flag" 1 "bench: foo_capi was built without -C llvm-args=-align-loops=64: CARGO_ENCODED_RUSTFLAGS, RUSTFLAGS or a target rustflags entry replaced the port's build.rustflags; include it there (BENCH_FLAGS)"
setup
BENCH_TEST_RUSTC="     Running \`rustc --crate-name foo_capi -Cllvm-args=-align-all-functions=6 -Cllvm-args=-align-loops=64\`"
go
want_rc "the -CFLAG form passes" 0 "cg-foo: foo port 1.0"
setup
BENCH_TEST_RUSTC="     Running \`rustc --crate-name foo_capi -C llvm-args=-align-all-functions=60 -C llvm-args=-align-loops=64\`"
go
want_rc "a longer flag is not the flag" 1 "bench: foo_capi was built without -C llvm-args=-align-all-functions=6: CARGO_ENCODED_RUSTFLAGS, RUSTFLAGS or a target rustflags entry replaced the port's build.rustflags; include it there (BENCH_FLAGS)"
setup
BENCH_TEST_RUSTC="$other
     Running \`rustc --crate-name foo_capi --edition=2021\`"
go
want_rc "a flag only on another crate's command" 1 "bench: foo_capi was built without -C llvm-args=-align-all-functions=6: CARGO_ENCODED_RUSTFLAGS, RUSTFLAGS or a target rustflags entry replaced the port's build.rustflags; include it there (BENCH_FLAGS)"
setup
BENCH_TEST_RUSTC="$good
     Running \`rustc --crate-name foo_capi --crate-type rlib -C llvm-args=-align-loops=64\`"
go
want_rc "a flag missing from the crate's second command" 1 "bench: foo_capi was built without -C llvm-args=-align-all-functions=6: CARGO_ENCODED_RUSTFLAGS, RUSTFLAGS or a target rustflags entry replaced the port's build.rustflags; include it there (BENCH_FLAGS)"
setup
BENCH_TEST_RUSTC="$good
$good"
go
want_rc "two commands that carry every flag pass" 0 "cg-foo: foo port 1.0"
setup
BENCH_TEST_RUSTC=$other
go
want_rc "no command for the crate" 1 "bench: no rustc command for foo_capi in $tmp/port/target/bench/build.log"
setup
BENCH_CRATE=foo_core
export BENCH_CRATE
BENCH_TEST_RUSTC=$other
go
want_rc "BENCH_CRATE names the crate" 0 "cg-foo: foo port 1.0"
setup
BENCH_TEST_BUILD_FAIL=1
export BENCH_TEST_BUILD_FAIL
go
want_rc "a failed build prints its log" 1 "error: could not compile"
want_rc "a failed build fails" 1 "bench: cargo build failed"

# ------------------------------------------------------------ suites
mkdir -p port/bench/demo port/bench/bad
cat > port/bench/demo/run.sh << 'EOF'
set -eu
[ -f "$CG_FOO_LIB/libfoo.so.1" ] && [ -x "$CG_FOO_BENCH" ] && [ -d "$CG_FOO_BENCH_OUT" ]
[ "$CG_FOO_ROOT" = "$(pwd)" ]
echo "demo: $(LD_LIBRARY_PATH=$CG_FOO_LIB "$CG_FOO_BENCH" --version) out=${CG_FOO_BENCH_OUT##*/} args: $*"
EOF
printf 'exit 3\n' > port/bench/bad/run.sh
setup
BENCH_PREFIX=CG_FOO
export BENCH_PREFIX
go demo --x 'y z'
want_rc "a suite runs with its environment under the prefix" 0 "demo: foo port 1.0 out=demo args: --x y z"
want_rc "a suite prints its line" 0 "== suite demo: $tmp/port/bench/demo/run.sh --x y z"
setup
BENCH_PREFIX=CG_FOO
export BENCH_PREFIX
go bad
[ "$rc" -eq 3 ] || { cat out >&2; fail "a suite's exit status: got $rc, want 3"; }
echo "ok a suite's exit status is the script's"
setup
go nope
want_rc "an unknown suite" 2 "bench: unknown suite 'nope': no $tmp/port/bench/nope/run.sh (suites: bad demo)"
[ ! -s "$BENCH_TEST_LOG" ] || fail "an unknown suite ran cargo"
go Demo
want_rc "a bad suite name" 2 "bench: bad suite name 'Demo' (lowercase letters, digits and -)"

# ------------------------------------------------------------ usage and set-up
# refuses WHAT RC LINE VAR=VALUE...: the run with VAR set to VALUE (an
# empty VALUE unsets it) exits RC with LINE, and runs no cargo when RC
# is 2.
refuses() {
  what=$1 code=$2 line=$3
  shift 3
  setup
  for a in "$@"; do
    v=${a%%=*}
    # shellcheck disable=SC2163 # a is NAME=VALUE: export sets NAME
    if [ -n "${a#*=}" ]; then export "$a"; else unset "$v"; fi
  done
  go
  want_rc "$what" "$code" "$line"
  if [ "$code" -eq 2 ] && [ -s "$BENCH_TEST_LOG" ]; then fail "$what: cargo ran"; fi
  return 0
}
refuses "no BENCH_SRC" 2 "bench: BENCH_SRC is empty: name the bench program's C source" BENCH_SRC=
refuses "no BENCH_PACKAGE" 2 "bench: BENCH_PACKAGE is empty: name the cargo package of the port's library" BENCH_PACKAGE=
refuses "no BENCH_REF" 2 "bench: BENCH_REF is empty: name the reference side (the C library)" BENCH_REF=
refuses "no BENCH_PORT" 2 "bench: BENCH_PORT is empty: name the port side" BENCH_PORT=
refuses "no BENCH_SO" 2 "bench: BENCH_SO is empty: name the port's library files to copy (patterns)" BENCH_SO=
refuses "a bad side name" 2 "bench: bad side name 'Foo' (lowercase letters, digits and -, not starting with -)" BENCH_REF=Foo
refuses "the same side twice" 2 "bench: BENCH_REF and BENCH_PORT are both 'cg-foo'" BENCH_REF=cg-foo
refuses "a bad mode name" 2 "bench: bad mode name 'a/b' (lowercase letters, digits and -, not starting with -)" "BENCH_MODES=jit a/b"
refuses "a mode named twice" 2 "bench: mode 'jit' is named twice in BENCH_MODES" "BENCH_MODES=jit interp jit"
refuses "a bad prefix" 2 "bench: bad BENCH_PREFIX 'cg' (capitals, digits and _)" BENCH_PREFIX=cg
refuses "a prefix with a dash" 2 "bench: bad BENCH_PREFIX 'CG-FOO' (capitals, digits and _)" BENCH_PREFIX=CG-FOO
refuses "a missing BENCH_NEEDS file" 1 "bench: needs /no/such/pcre2.h (BENCH_NEEDS)" "BENCH_NEEDS=$tmp/rows /no/such/pcre2.h"
refuses "a missing source" 1 "bench: no bench program source bench/none.c (BENCH_SRC)" BENCH_SRC=bench/none.c
refuses "a BENCH_SO pattern that matches nothing" 1 "bench: BENCH_SO pattern target/release/libbar.so* matches no file" "BENCH_SO=target/release/libfoo.so* target/release/libbar.so*"
refuses "a failing BENCH_LINK" 1 "bench: BENCH_LINK failed: exit 4" "BENCH_LINK=exit 4"
refuses "a program that does not build" 1 "bench: the bench program did not build" "BENCH_CFLAGS=-DNOPE -include /no/such.h"

echo "bench-test: ok"
