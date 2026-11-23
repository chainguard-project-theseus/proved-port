#!/bin/sh
# kit/perf-budget-test.sh: tests of kit/make/perf-budget.sh with cargo
# (the perf-budget-test check). A fixture library with no dependencies
# (kit/perf/budget/fixture, copied to a scratch directory) exports
# fx_pick and fx_load, each a fast path that returns and a slow path that
# tail-calls `slow`, behind a flag byte the table names as a cell; the
# `wide` feature gives fx_pick another fast path. The counts depend on
# the compiler, so the test pins none: `print` measures them into the
# table, and the rest checks against that. Then:
#   - the settings it refuses, each with exit 2 before any build: no
#     PERF_BUDGET_LIB, one with a directory, a malformed build name, a
#     build with no table or a table that is not a file, a second
#     argument, and an unknown mode;
#   - print over a table with placeholder counts exits 0, prints a
#     `paths` and a `budget` line per budget line, and says the table
#     differs;
#   - check over the measured table passes, after a self-test that
#     rejects all 8 mutations of fx_pick and the table;
#   - a count off by one in the table fails the check (exit 1);
#   - PERF_BUDGET_SELFTEST_EXPORT=fx_set, whose body has no register
#     move to mutate, fails the self-test (exit 1) saying so;
#   - a table whose cell no budget line names skips the cell mutation,
#     saying so, and passes;
#   - two builds, one with `wide`, each in its own target directory and
#     each against its own table: both pass; with the tables swapped the
#     run fails;
#   - a failed build (an unknown feature) and a library name the build
#     does not write each fail (exit 1);
#   - after a native run, a run with --target HOST checks
#     HOST/release's library and removes the native one first;
#   - with a stub cargo, a build that leaves the library in release/ and
#     TRIPLE/release fails, and so does one that writes none though an
#     earlier run's library is in release/;
#   - a blank PERF_BUDGET_BUILDS is refused (exit 2), and print fails
#     (exit 1) on a checker error, a cell naming a static the library
#     lacks;
#   - the frozen case: the checker over kit/perf/budget/testdata's
#     objdump output for the fixture, through its objdump.sh stand-in,
#     against table.txt's hand-worked counts: it passes, and each of the
#     five counts off by one fails, whatever this compiler emits.
# Run it from the repository root. It needs cargo, rustc, objdump and
# no network, on x86_64.
set -eu
root=$(pwd)
script=$root/kit/make/perf-budget.sh
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }
case $(uname -m) in
  x86_64) ;;
  *) fail "perf-budget-test needs x86_64 (the checker reads x86_64 code only); this is $(uname -m)" ;;
esac
cp -R kit/perf/budget/fixture "$tmp/fx"
cd "$tmp/fx"
export PERF_BUDGET_TARGET_DIR="$tmp/target"

# run EXPECTED-EXIT NAME ARGS [VAR=VALUE...]: runs perf-budget.sh ARGS
# (words) with the settings, output in $tmp/out, and fails unless it
# exits EXPECTED.
run() {
  want=$1 name=$2 args=$3
  shift 3
  # shellcheck disable=SC2086 # ARGS are words
  if env "$@" sh "$script" $args > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
  [ "$rc" -eq "$want" ] || { cat "$tmp/out" >&2; fail "$name: got = exit $rc, want = exit $want"; }
}
has() { # has NAME TEXT: $tmp/out holds TEXT
  grep -qF -- "$2" "$tmp/out" || { cat "$tmp/out" >&2; fail "$1: got = no '$2'"; }
}

# The table's head: the cell, the exports and placeholder budgets.
table_head() { # table_head yes|no: whether a budget line names the cell
  cat <<'TABLE'
# kit/perf-budget-test.sh's table for kit/perf/budget/fixture.
cell flags pbfixture::FLAGS 0 1
export fx_pick pbfixture::slow
export fx_load pbfixture::slow
TABLE
  echo "budget fx_pick - 0 0 0 0 0"
  if [ "$1" = yes ]; then
    echo "budget fx_pick flags=0 0 0 0 0 0"
  fi
  echo "budget fx_load - 0 0 0 0 0"
}
lib=PERF_BUDGET_LIB=libpbfixture.so

# --- settings ---------------------------------------------------------------
table_head yes > "$tmp/placeholder.txt"
ph=PERF_BUDGET_TABLE_default=$tmp/placeholder.txt
run 2 "no PERF_BUDGET_LIB" check
has "no PERF_BUDGET_LIB" "PERF_BUDGET_LIB is empty"
run 2 "PERF_BUDGET_LIB with a directory" check PERF_BUDGET_LIB=release/libpbfixture.so "$ph"
has "PERF_BUDGET_LIB with a directory" "a file name, with no directory"
run 2 "a malformed build name" check "$lib" PERF_BUDGET_BUILDS=q-8
has "a malformed build name" "a build name is [A-Za-z0-9_]"
run 2 "a blank PERF_BUDGET_BUILDS" check "$lib" "PERF_BUDGET_BUILDS= "
has "a blank PERF_BUDGET_BUILDS" "PERF_BUDGET_BUILDS is blank"
run 2 "a build with no table" check "$lib"
has "a build with no table" "PERF_BUDGET_TABLE_default is empty"
run 2 "a table that is not a file" check "$lib" PERF_BUDGET_TABLE_default="$tmp/none.txt"
has "a table that is not a file" "which is not a file"
run 2 "a second argument" "check extra" "$lib" "$ph"
has "a second argument" "usage: perf-budget.sh"
run 2 "an unknown mode" verify "$lib" "$ph"
has "an unknown mode" "usage: perf-budget.sh"
[ ! -d "$tmp/target" ] || fail "a refused setting: got = a build in $tmp/target, want = none"
echo "ok perf-budget.sh refuses each malformed setting before any build"

# --- print, then check ------------------------------------------------------
# measure NAME yes|no TABLE [VAR=VALUE...]: print with table_head's
# placeholders, and write TABLE: the head's comment, cell and export
# lines, then the measured lines.
measure() {
  what="print, $1" cells=$2 table=$3
  shift 3
  table_head "$cells" > "$tmp/head.txt"
  run 0 "$what" print "$lib" PERF_BUDGET_TABLE_default="$tmp/head.txt" "$@"
  has "$what" "differs from"
  grep -v '^budget ' "$tmp/head.txt" > "$table"
  grep -E '^(paths|budget) ' "$tmp/out" >> "$table" || true
  n=$(grep -c '^budget ' "$table" || true)
  p=$(grep -c '^paths ' "$table" || true)
  lines=$(grep -c '^budget ' "$tmp/head.txt")
  [ "$n" -eq "$lines" ] && [ "$p" -eq "$lines" ] \
    || { cat "$tmp/out" >&2; fail "$what: got = $n budget and $p paths lines, want = $lines of each"; }
}
measure "one build" yes "$tmp/t.txt"
echo "ok print measures a paths and a budget line for each budget line"
# A checker error (here a cell naming a static the library lacks) fails
# print, though a mere difference from the table does not.
sed 's/pbfixture::FLAGS/pbfixture::NOPE/' "$tmp/placeholder.txt" > "$tmp/nope.txt"
run 1 "print, a cell with no static" print "$lib" PERF_BUDGET_TABLE_default="$tmp/nope.txt"
has "print, a cell with no static" "no static pbfixture::NOPE"
echo "ok print fails on a checker error"
t=PERF_BUDGET_TABLE_default=$tmp/t.txt
run 0 "check" check "$lib" "$t"
has "check" "self-test: 8 mutations of fx_pick"
has "check" "perf-budget: OK (3 budgets)"
echo "ok check passes the measured table after rejecting 8 mutations"

# A count off by one: the first budget line's instructions.
awk '!done && $1 == "budget" { $4 = $4 + 1; done = 1 } { print }' "$tmp/t.txt" > "$tmp/off.txt"
run 1 "a count off by one" check "$lib" PERF_BUDGET_TABLE_default="$tmp/off.txt"
has "a count off by one" "max insns shrank"
echo "ok a count off by one fails the check"

run 1 "a self-test export with no register move" check "$lib" "$t" PERF_BUDGET_SELFTEST_EXPORT=fx_set
has "a self-test export with no register move" "mutation 'load' does not apply to fx_set's body"
echo "ok a self-test mutation that does not apply fails the run"

measure "no cell named" no "$tmp/nocell.txt"
run 0 "no cell named" check "$lib" PERF_BUDGET_TABLE_default="$tmp/nocell.txt"
has "no cell named" "the cell mutation is skipped"
has "no cell named" "self-test: 7 mutations of fx_pick"
echo "ok with no cell a budget names, the cell mutation is skipped"

# --- two builds -------------------------------------------------------------
measure "wide" yes "$tmp/wide.txt" PERF_BUDGET_FLAGS_default="--features wide"
if cmp -s "$tmp/t.txt" "$tmp/wide.txt"; then
  fail "the wide build: got = the default build's table, want = another fx_pick"
fi
run 0 "two builds" check "$lib" PERF_BUDGET_BUILDS="narrow wide" PERF_BUDGET_TABLE_narrow="$tmp/t.txt" \
  PERF_BUDGET_TABLE_wide="$tmp/wide.txt" PERF_BUDGET_FLAGS_wide="--features wide"
has "two builds" "== narrow: $tmp/target/perf-budget-narrow/release/libpbfixture.so"
has "two builds" "== wide: $tmp/target/perf-budget-wide/release/libpbfixture.so"
[ "$(grep -c 'perf-budget: OK' "$tmp/out")" -eq 2 ] || { cat "$tmp/out" >&2; fail "two builds: got = not two OKs"; }
run 1 "two builds, swapped tables" check "$lib" PERF_BUDGET_BUILDS="narrow wide" PERF_BUDGET_TABLE_narrow="$tmp/wide.txt" \
  PERF_BUDGET_TABLE_wide="$tmp/t.txt" PERF_BUDGET_FLAGS_wide="--features wide"
has "two builds, swapped tables" "== narrow"
echo "ok two builds each pass their own table, and fail with the tables swapped"

# --- failures ---------------------------------------------------------------
run 1 "a failed build" check "$lib" "$t" PERF_BUDGET_FLAGS_default="--features nope"
has "a failed build" "perf-budget.sh: build default failed"
run 1 "a library the build does not write" check PERF_BUDGET_LIB=libnope.so "$t"
has "a library the build does not write" "wrote no libnope.so"
echo "ok a failed build and an unwritten library fail the run"

# --- where the library is ---------------------------------------------------
# After a native run left release/libpbfixture.so, a run with --target
# HOST checks HOST/release's library: the native one goes first.
host=$(rustc -vV | sed -n 's/^host: //p')
[ -n "$host" ] || fail "rustc -vV: got = no host line"
bdir=$tmp/target/perf-budget-default
run 0 "native" check "$lib" "$t"
[ -f "$bdir/release/libpbfixture.so" ] || fail "native: got = no $bdir/release/libpbfixture.so"
run 0 "--target $host" check "$lib" "$t" PERF_BUDGET_FLAGS_default="--target $host"
has "--target $host" "== default: $bdir/$host/release/libpbfixture.so"
[ ! -f "$bdir/release/libpbfixture.so" ] || fail "--target $host: got = the native library left in release/, want = removed"
echo "ok a --target build checks TRIPLE/release's library, and the native one goes first"

# A stub cargo (first on PATH) that copies the built library where
# STUB_CARGO says, or writes nothing: the run refuses a build that leaves
# the library in two places, and one that writes none, though an earlier
# run's library was in release/.
real=$tmp/real.so
cp "$bdir/$host/release/libpbfixture.so" "$real"
mkdir "$tmp/stub"
cat > "$tmp/stub/cargo" <<STUB
#!/bin/sh
case \$STUB_CARGO in
  two) mkdir -p "\$CARGO_TARGET_DIR/release" "\$CARGO_TARGET_DIR/$host/release"
       cp "$real" "\$CARGO_TARGET_DIR/release/libpbfixture.so"
       cp "$real" "\$CARGO_TARGET_DIR/$host/release/libpbfixture.so" ;;
esac
STUB
chmod +x "$tmp/stub/cargo"
run 1 "a library in two places" check "$lib" "$t" PATH="$tmp/stub:$PATH" STUB_CARGO=two
has "a library in two places" "wrote libpbfixture.so in more than one of"
mkdir -p "$bdir/release"
cp "$real" "$bdir/release/libpbfixture.so"
run 1 "an earlier run's library, none written" check "$lib" "$t" PATH="$tmp/stub:$PATH" STUB_CARGO=none
has "an earlier run's library, none written" "wrote no libpbfixture.so"
echo "ok a library in two places, and an earlier run's library with none written, fail the run"

# --- the frozen case --------------------------------------------------------
# The checker over kit/perf/budget/testdata's dumps of the fixture (its
# objdump.sh stands in for objdump), against table.txt's counts, worked
# out by hand: they pass, and each count off by one fails, so a checker
# that miscounts (cache lines included) fails here whatever code this
# compiler emits.
data=$root/kit/perf/budget/testdata
rustc -O --edition 2021 "$root/kit/make/perf-budget.rs" -o "$tmp/perf-budget"
frozen() { # frozen TABLE: the checker over the dumps, output in $tmp/out
  if PERF_BUDGET_OBJDUMP=$data/objdump.sh "$tmp/perf-budget" libpbfixture.so "$1" > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
}
frozen "$data/table.txt"
[ "$rc" -eq 0 ] || { cat "$tmp/out" >&2; fail "the frozen case: got = exit $rc, want = exit 0"; }
has "the frozen case" "perf-budget: OK (3 budgets)"
i=0
for metric in insns loads stores branches lines; do
  # Field 4 + i of fx_load's budget line, and its paths count, one more.
  awk -v f=$((4 + i)) -v k=$((i + 1)) '
    $1 == "budget" && $2 == "fx_load" { $f = $f + 1 }
    $1 == "paths" && $2 == "fx_load" { split($4, c, "/"); c[k]++; $4 = c[1] "/" c[2] "/" c[3] "/" c[4] "/" c[5] }
    { print }' "$data/table.txt" > "$tmp/frozen.txt"
  frozen "$tmp/frozen.txt"
  [ "$rc" -eq 1 ] || { cat "$tmp/out" >&2; fail "the frozen case, $metric off by one: got = exit $rc, want = exit 1"; }
  has "the frozen case, $metric off by one" "max $metric shrank"
  i=$((i + 1))
done
echo "ok the checker's counts over the frozen dumps are table.txt's, and each off by one fails"
echo PASS
