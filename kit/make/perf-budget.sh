#!/bin/sh
# perf-budget.sh [check|print]: the hot exports' fast-path cost bounds on
# an x86_64 release build of the port's library (`make perf-budget`,
# `make perf-budget-print`, proved-port.mk), which the theseus-proved-port
# package installs in /usr/share/proved-port/make/ beside the checker,
# perf-budget.rs. It is challoc's scripts/perf-budget.sh with the library,
# the builds and their tables as settings. Run it from the port's root.
#
#   perf-budget.sh check   per build PERF_BUDGET_BUILDS names: build the
#                          release library, run the self-test (below) and
#                          check the library against the build's table
#                          (the default)
#   perf-budget.sh print   per build: build the release library and print
#                          the measured `paths` and `budget` lines (the
#                          checker's --print), to update the table after
#                          a deliberate change; it exits 0 whether or not
#                          they match the table, and prints the checker's
#                          FAIL lines when they do not
#
# The checker (perf-budget.rs, std only, built here with `rustc -O`)
# reads `objdump` output and, for every `export` line of the table, fails
# on an atomic read-modify-write, a fence, a call, a syscall, an indirect
# jump or an unparsed instruction in the export's body (P2); for every
# `budget` line it walks every path from the entry and fails on a back
# edge (P3) and on any change to the counts of instructions, loads,
# stores, conditional branches and cache lines of every fast path (a path
# to a `ret`). Its header documents the table format. The counts depend
# on the compiler, so pin the rust stream of the check that runs this,
# and update the table and the pin together.
#
# The self-test runs the checker on mutated input, and fails unless each
# mutation is rejected with its own message:
#   load     the first `mov R64,R64` in the self-test export's body reads
#            memory instead (max loads GREW: the mov must be on a fast
#            path one of the export's budget lines walks)
#   call     its first `ret` is a call to the export (P2)
#   loop     its first `ret` is a jump to the export's entry (P3)
#   unknown  its first `ret` is an instruction the checker does not know
#   gap      its second instruction is missing (a path falls out of the
#            body)
#   shrank   the table's first `budget` line has 999 instructions
#   cell     the first `cell` a budget line's configuration names is 1 MiB
#            further into its static (stale); skipped, with a note, when
#            no budget line names a cell
#   path     the table's first `paths` line has a different first count
# A mutation that leaves objdump's output or the table unchanged fails
# the run too: name another export in PERF_BUDGET_SELFTEST_EXPORT.
#
# Exit status: 0 when every build passes; 1 when a check, the self-test,
# a build or a tool fails; 2 for a setting missing or malformed, before
# any build.
#
# Each build NAME is `cargo build --release --lib` with
# PERF_BUDGET_CARGO_FLAGS and PERF_BUDGET_FLAGS_<NAME>, in its own target
# directory, PERF_BUDGET_TARGET_DIR/perf-budget-NAME; the library is
# release/PERF_BUDGET_LIB there, or TRIPLE/release/PERF_BUDGET_LIB when
# the flags name one target. challoc's two quanta are
#   PERF_BUDGET_BUILDS=q16 q8
#   PERF_BUDGET_TABLE_q16=verus/perf-budget.txt
#   PERF_BUDGET_TABLE_q8=verus/perf-budget-q8.txt
#   PERF_BUDGET_FLAGS_q8=--no-default-features --features quantum8
#
# The environment; empty is the same as unset:
#
#   PERF_BUDGET_LIB           the library's file name, as cargo writes it
#                             (libchalloc.so) (required)
#   PERF_BUDGET_BUILDS        the builds, by name ([A-Za-z0-9_], by
#                             default one, `default`)
#   PERF_BUDGET_TABLE_<NAME>  that build's table, from the port's root
#                             (required for each build)
#   PERF_BUDGET_FLAGS_<NAME>  cargo flags for that build only
#   PERF_BUDGET_CARGO_FLAGS   cargo flags for every build
#                             (--manifest-path, -p for a library in a
#                             virtual workspace, --locked)
#   PERF_BUDGET_TARGET_DIR    by default target, from the port's root
#   PERF_BUDGET_SELFTEST_EXPORT
#                             the export the self-test mutates (by
#                             default the first `export` line's)
#   PERF_BUDGET_RUSTC         the compiler for the checker (by default
#                             rustc, from PATH)
#
# It needs cargo, rustc, objdump (GNU binutils), awk and sed, on x86_64:
# the checker refuses a library for another architecture.
set -eu

usage() {
  echo "usage: perf-budget.sh [check|print]" >&2
  exit 2
}
mode=${1:-check}
[ $# -le 1 ] || usage
case $mode in check | print) ;; *) usage ;; esac

here=$(cd "$(dirname "$0")" && pwd)
lib=${PERF_BUDGET_LIB-}
builds=${PERF_BUDGET_BUILDS:-default}
tdir=${PERF_BUDGET_TARGET_DIR:-target}
rustc=${PERF_BUDGET_RUSTC:-rustc}
case $lib in
  '') echo "perf-budget.sh: PERF_BUDGET_LIB is empty: name the library's file (libchalloc.so)" >&2; exit 2 ;;
  */*) echo "perf-budget.sh: PERF_BUDGET_LIB is $lib: a file name, with no directory" >&2; exit 2 ;;
esac
nbuilds=0
for b in $builds; do
  nbuilds=$((nbuilds + 1))
  case $b in
    *[!A-Za-z0-9_]*) echo "perf-budget.sh: PERF_BUDGET_BUILDS names $b: a build name is [A-Za-z0-9_]" >&2; exit 2 ;;
  esac
  table=
  eval "table=\${PERF_BUDGET_TABLE_$b-}"
  if [ -z "$table" ]; then
    echo "perf-budget.sh: PERF_BUDGET_TABLE_$b is empty: name build $b's table" >&2
    exit 2
  fi
  if [ ! -f "$table" ]; then
    echo "perf-budget.sh: PERF_BUDGET_TABLE_$b is $table, which is not a file" >&2
    exit 2
  fi
done
if [ "$nbuilds" -eq 0 ]; then
  echo "perf-budget.sh: PERF_BUDGET_BUILDS is blank: name the builds (or leave it unset for one, default)" >&2
  exit 2
fi
for t in cargo "$rustc" objdump awk sed; do
  command -v "$t" >/dev/null || { echo "perf-budget.sh: $t is not on PATH" >&2; exit 1; }
done
[ -f "$here/perf-budget.rs" ] || { echo "perf-budget.sh: no checker $here/perf-budget.rs" >&2; exit 1; }

out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
trap 'exit 130' INT TERM
"$rustc" --version
"$rustc" -O --edition 2021 "$here/perf-budget.rs" -o "$out/perf-budget"

build() { # build NAME: builds it and prints its library's path
  name=$1
  bdir=$tdir/perf-budget-$name
  bflags=
  eval "bflags=\${PERF_BUDGET_FLAGS_$name-}"
  log=$out/build-$name.log
  # An earlier run's library goes first: the one found after the build is
  # then the one it wrote (as codegen.sh's build).
  rm -f "$bdir/release/$lib" "$bdir"/*/release/"$lib"
  # shellcheck disable=SC2086 # the flags are words
  if ! CARGO_TARGET_DIR=$bdir cargo build --release --lib -q ${PERF_BUDGET_CARGO_FLAGS-} $bflags > "$log" 2>&1; then
    cat "$log" >&2
    echo "perf-budget.sh: build $name failed" >&2
    exit 1
  fi
  found=
  for so in "$bdir/release/$lib" "$bdir"/*/release/"$lib"; do
    [ -f "$so" ] || continue
    if [ -n "$found" ]; then
      echo "perf-budget.sh: build $name wrote $lib in more than one of $bdir/release and $bdir/TRIPLE/release: give it one target" >&2
      exit 1
    fi
    found=$so
  done
  if [ -z "$found" ]; then
    echo "perf-budget.sh: build $name wrote no $lib in $bdir/release or $bdir/TRIPLE/release" >&2
    exit 1
  fi
  echo "$found"
}

# The stand-in objdump the self-test runs: `-d` output with MUT applied to
# SYM's body, everything else the real objdump's.
cat > "$out/objdump" <<'EOF'
#!/bin/sh
case "$*" in
  *-d*) objdump "$@" | awk -v m="$MUT" -v sym="$SYM" '
    $0 == "" { inm = 0 }
    $2 == "<" sym ">:" && $1 ~ /^[0-9a-f]+$/ { inm = 1; entry = $1; sub(/^0+/, "", entry) }
    inm && !done && m == "load" && /\tmov +r[a-z0-9]+,r[a-z0-9]+$/ {
      sub(/\tmov .*/, "\tmov    rax,QWORD PTR [rdi]"); done = 1 }
    inm && !done && m == "call" && /\tret/ { sub(/\tret.*/, "\tcall   " entry " <" sym ">"); done = 1 }
    inm && !done && m == "loop" && /\tret/ { sub(/\tret.*/, "\tjmp    " entry " <" sym ">"); done = 1 }
    inm && !done && m == "unknown" && /\tret/ { sub(/\tret.*/, "\tvzeroupper"); done = 1 }
    # The second instruction after the entry, deleted: a gap.
    inm && m == "gap" && /^ *[0-9a-f]+:\t/ { n++; if (n == 2) next }
    { print }';;
  *) exec objdump "$@";;
esac
EOF
chmod +x "$out/objdump"

selftest() { # selftest LIB TABLE
  so=$1 table=$2
  sym=${PERF_BUDGET_SELFTEST_EXPORT:-$(awk '$1 == "export" { print $2; exit }' "$table")}
  if [ -z "$sym" ]; then
    echo "perf-budget.sh: $table has no export line for the self-test" >&2
    exit 1
  fi
  objdump -d -M intel -w "$so" > "$out/dump"
  n=0
  for mut in "load:max loads GREW" "call:P2 $sym: call" "loop:P3: cycle" "unknown:unclassified instruction" "gap:fell out of the body"; do
    m=${mut%%:*} want=${mut#*:}
    MUT=$m SYM=$sym "$out/objdump" -d -M intel -w "$so" > "$out/mutated"
    if cmp -s "$out/dump" "$out/mutated"; then
      echo "perf-budget self-test: mutation '$m' does not apply to $sym's body: name another export in PERF_BUDGET_SELFTEST_EXPORT" >&2
      exit 1
    fi
    if MUT=$m SYM=$sym PERF_BUDGET_OBJDUMP="$out/objdump" "$out/perf-budget" "$so" "$table" > "$out/log" 2>&1; then
      echo "perf-budget self-test: mutation '$m' of $sym was not rejected" >&2
      exit 1
    fi
    grep -qF -- "$want" "$out/log" || {
      echo "perf-budget self-test: mutation '$m' of $sym failed without '$want':" >&2
      cat "$out/log" >&2
      exit 1
    }
    n=$((n + 1))
  done
  # The table's mutations.
  cell=$(awk '
    $1 == "budget" && $3 != "-" { k = split($3, kv, ","); for (i = 1; i <= k; i++) { split(kv[i], p, "="); named[p[1]] = 1 } }
    $1 == "cell" { cells[++nc] = $2 }
    END { for (i = 1; i <= nc; i++) if (cells[i] in named) { print cells[i]; exit } }
  ' "$table")
  for mut in "shrank:max insns shrank" "cell:is stale" "path:the fast paths' counts"; do
    m=${mut%%:*} want=${mut#*:}
    if [ "$m" = cell ] && [ -z "$cell" ]; then
      echo "self-test: no budget line names a cell, so the cell mutation is skipped"
      continue
    fi
    awk -v m="$m" -v cell="$cell" '
      function hex(s,   v, i, c) {
        if (s !~ /^0x/) return s + 0
        v = 0
        for (i = 3; i <= length(s); i++) { c = index("0123456789abcdef", tolower(substr(s, i, 1))) - 1; v = v * 16 + c }
        return v
      }
      !done && m == "shrank" && $1 == "budget" { $4 = ($4 == 999 ? 998 : 999); done = 1 }
      !done && m == "cell" && $1 == "cell" && $2 == cell { $4 = sprintf("0x%x", hex($4) + 1048576); done = 1 }
      !done && m == "path" && $1 == "paths" { $4 = ($4 == "0/0/0/0/0" ? "1/0/0/0/0" : "0/0/0/0/0"); done = 1 }
      { print }
    ' "$table" > "$out/table"
    if cmp -s "$table" "$out/table"; then
      echo "perf-budget self-test: table mutation '$m' does not apply to $table" >&2
      exit 1
    fi
    if "$out/perf-budget" "$so" "$out/table" > "$out/log" 2>&1; then
      echo "perf-budget self-test: table mutation '$m' was not rejected" >&2
      exit 1
    fi
    grep -qF -- "$want" "$out/log" || {
      echo "perf-budget self-test: table mutation '$m' failed without '$want':" >&2
      cat "$out/log" >&2
      exit 1
    }
    n=$((n + 1))
  done
  echo "self-test: $n mutations of $sym and $table rejected"
}

for b in $builds; do
  eval "table=\${PERF_BUDGET_TABLE_$b}"
  so=$(build "$b")
  echo "== $b: $so ($table)"
  if [ "$mode" = print ]; then
    # The checker prints the measured lines, then exits 1 when they
    # differ from the table's (the reason to print them); 2 is an error.
    rc=0
    "$out/perf-budget" --print "$so" "$table" || rc=$?
    case $rc in
      0) ;;
      1) echo "perf-budget.sh: build $b differs from $table (the FAIL lines above); the lines above are the measured ones" >&2 ;;
      *) exit 1 ;;
    esac
    continue
  fi
  selftest "$so" "$table"
  "$out/perf-budget" "$so" "$table"
done
