#!/bin/sh
# bench.sh [SUITE] [ARGS...]: the bench runner (`make bench-run`,
# proved-port.mk), which the theseus-proved-port package installs in
# /usr/share/proved-port/make/. The generic parts of the PCRE2 port's
# ci/bench.sh: the runner over a reference and a port library, the
# agreement check on the program's counts, the rustc flag check, the
# suite mode and the environment report. The program, its workloads, the
# library's link step and the session wrapper stay in the port. Run it
# from the repository root. UNVALIDATED: no box run has compared it with
# the port's own runner yet (#262).
#
# The build. It cleans and rebuilds the cargo package BENCH_PACKAGE
# (`cargo clean --release -p` then `cargo build --locked --release -p
# ... -v`, the log in WORK/build.log), and requires every rustc command
# cargo printed for the crate BENCH_CRATE (at least one) (default: BENCH_PACKAGE with
# `-` as `_`) to carry each word of BENCH_FLAGS as a codegen option,
# `-C FLAG` or `-CFLAG`, whole: cargo takes rustflags from one source
# only (CARGO_ENCODED_RUSTFLAGS, RUSTFLAGS, a target rustflags entry,
# then build.rustflags), so a library built from another source does not
# carry the port's pinned code layout and does not compare with its
# main. Then it runs BENCH_LINK with sh -c when set (the PCRE2 port's
# `sh scripts/link-so.sh release`), copies the files the patterns of
# BENCH_SO match (links as links) into WORK/lib, which holds nothing
# else, and builds the program: BENCH_CC (default cc) BENCH_CFLAGS
# (default -O2 -std=c99 -Wall -Wextra -Werror) -o WORK/bench BENCH_SRC
# BENCH_LDLIBS. The program links the reference library as the system
# has it; LD_LIBRARY_PATH=WORK/lib swaps in the port's.
#
# The program's protocol:
#
#   PROG --version
#     one line: the version of the library it loaded.
#   PROG --lib SIDE --mode MODE [SIDE_ARGS...] [ARGS...]
#     one tab-separated line per workload on stdout:
#     SIDE MODE WORKLOAD VALUE UNIT COUNT. SIDE and MODE are the ones it
#     was given. A UNIT of MB/s is a rate (higher is faster); any other
#     is a time (lower is faster). COUNT is the workload's result, the
#     same in every run, or rc=N where the workload ends with a return
#     code that may differ (the PCRE2 port's catastrophic patterns).
#
# The run (no SUITE). For each MODE in BENCH_MODES (default `default`),
# the reference side BENCH_REF and then, for each MODE again, the port
# side BENCH_PORT (on WORK/lib): PROG --lib SIDE --mode MODE, then
# BENCH_REF_ARGS or BENCH_PORT_ARGS, then ARGS. A run that fails, that
# prints no line, or that prints a line with other than six fields, with
# another SIDE or MODE, with an empty WORKLOAD, UNIT or COUNT, with a
# VALUE that is not a decimal number, or with a WORKLOAD it printed
# before, fails the run. It prints the environment and a table, one row a
# workload: each arm's value, the port's first mode over each reference
# arm (>1 is faster), and the count. A workload fails (MISMATCH, exit 1)
# when its counts differ among the arms that give counts, when some arms
# give a count and some a return code, or when one side has it in a mode
# and the other does not; return codes that differ are noted, allowed.
# It writes OUT/BENCH_PORT-bench-ARCH.tsv (the program's lines) and .txt
# (what it printed).
#
# The suite (SUITE, a word of lowercase letters, digits and `-` not
# starting with `-`). After the build and the environment report it runs
# BENCH_SUITES/SUITE/run.sh ARGS with sh, with, in its environment, P
# the prefix BENCH_PREFIX (default PORT; capitals, digits and `_`, so
# the PCRE2 port's suites keep CG_PCRE2_*):
#
#   P_LIB        WORK/lib, the port's libraries only
#   P_ROOT       the repository root (the current directory)
#   P_BENCH      the built program
#   P_BENCH_OUT  OUT/SUITE, the suite's output directory
#
# and its exit status is the script's.
#
# Environment (each word list is split on blanks, with no quoting and no
# globbing; an empty value is unset): BENCH_SRC, BENCH_PACKAGE, BENCH_REF
# and BENCH_PORT (required), BENCH_CRATE, BENCH_FLAGS, BENCH_LINK,
# BENCH_SO (required: word patterns, each matching at least one file),
# BENCH_CC, BENCH_CFLAGS, BENCH_LDLIBS, BENCH_MODES, BENCH_REF_ARGS,
# BENCH_PORT_ARGS, BENCH_NEEDS (files that must exist before anything
# runs, e.g. the reference library's header), BENCH_REF_FILE (the
# reference library's file, to name its package in the report with apk
# info -W), BENCH_PREFIX, BENCH_SUITES (default bench), BENCH_OUT
# (default target/remote-out/bench, which the PCRE2 port's session
# wrapper brings back) and BENCH_WORK (default target/bench). Side and
# mode names are lowercase letters, digits and `-`, and distinct.
#
# Exit status: 0 pass, 1 fail (a build, a run or a MISMATCH), 2 usage.
# Linux (glibc) only; needs cargo, a C compiler, awk and sed.
set -eu

die() { echo "bench: $*" >&2; exit 1; }
usage() { echo "bench: $*" >&2; exit 2; }

[ -n "${BENCH_SRC:-}" ] || usage "BENCH_SRC is empty: name the bench program's C source"
[ -n "${BENCH_PACKAGE:-}" ] || usage "BENCH_PACKAGE is empty: name the cargo package of the port's library"
[ -n "${BENCH_REF:-}" ] || usage "BENCH_REF is empty: name the reference side (the C library)"
[ -n "${BENCH_PORT:-}" ] || usage "BENCH_PORT is empty: name the port side"
[ -n "${BENCH_SO:-}" ] || usage "BENCH_SO is empty: name the port's library files to copy (patterns)"
crate=${BENCH_CRATE:-$(printf '%s' "$BENCH_PACKAGE" | tr - _)}
modes=${BENCH_MODES:-default}
prefix=${BENCH_PREFIX:-PORT}
cc=${BENCH_CC:-cc}
cflags=${BENCH_CFLAGS:--O2 -std=c99 -Wall -Wextra -Werror}
root=$(pwd)
suites=${BENCH_SUITES:-bench}
out=${BENCH_OUT:-target/remote-out/bench}
work=${BENCH_WORK:-target/bench}
case $suites in /*) ;; *) suites=$root/$suites ;; esac
case $out in /*) ;; *) out=$root/$out ;; esac
case $work in /*) ;; *) work=$root/$work ;; esac

name_ok() {
  case $1 in
    "" | -* | *[!a-z0-9-]*) return 1 ;;
  esac
}
for n in "$BENCH_REF" "$BENCH_PORT"; do
  name_ok "$n" || usage "bad side name '$n' (lowercase letters, digits and -, not starting with -)"
done
[ "$BENCH_REF" != "$BENCH_PORT" ] || usage "BENCH_REF and BENCH_PORT are both '$BENCH_REF'"
set -f
seen=" "
# shellcheck disable=SC2086 # a word list
for m in $modes; do
  name_ok "$m" || usage "bad mode name '$m' (lowercase letters, digits and -, not starting with -)"
  case $seen in *" $m "*) usage "mode '$m' is named twice in BENCH_MODES" ;; esac
  seen="$seen$m "
done
set +f
case $prefix in
  [A-Z]*) case $prefix in *[!A-Z0-9_]*) usage "bad BENCH_PREFIX '$prefix' (capitals, digits and _)" ;; esac ;;
  *) usage "bad BENCH_PREFIX '$prefix' (capitals, digits and _)" ;;
esac

suite=
case ${1:-} in
  "" | -*) ;;
  *) suite=$1; shift ;;
esac
if [ -n "$suite" ]; then
  name_ok "$suite" || usage "bad suite name '$suite' (lowercase letters, digits and -)"
  if [ ! -f "$suites/$suite/run.sh" ]; then
    known=$(for r in "$suites"/*/run.sh; do
      [ -f "$r" ] || continue
      d=${r%/run.sh}
      printf ' %s' "${d##*/}"
    done)
    usage "unknown suite '$suite': no $suites/$suite/run.sh (suites:${known:- none})"
  fi
fi

set -f
# shellcheck disable=SC2086 # a word list
for f in ${BENCH_NEEDS:-}; do
  [ -e "$f" ] || die "needs $f (BENCH_NEEDS)"
done
set +f
[ -f "$BENCH_SRC" ] || die "no bench program source $BENCH_SRC (BENCH_SRC)"

# ------------------------------------------------------------------ build
mkdir -p "$work"
cargo clean --release -p "$BENCH_PACKAGE" > "$work/clean.log" 2>&1 || {
  cat "$work/clean.log" >&2
  die "cargo clean failed"
}
cargo build --locked --release -p "$BENCH_PACKAGE" -v > "$work/build.log" 2>&1 || {
  cat "$work/build.log" >&2
  die "cargo build failed"
}
# -a: the log is text even with a NUL or a byte the locale cannot decode.
# The crate's own command: `--crate-name CRATE` followed by a blank.
LC_ALL=C grep -aF 'Running' "$work/build.log" | LC_ALL=C grep -aF -e "--crate-name $crate " > "$work/rustc.lines" || :
[ -s "$work/rustc.lines" ] || die "no rustc command for $crate in $work/build.log"
# has_flag FLAG: the line has `-C FLAG` or `-CFLAG` as whole words.
has_flag() {
  set -f
  # shellcheck disable=SC2086 # the words of the line
  set -- "$1" $rustc_line
  set +f
  want=$1
  shift
  prev=
  for w in "$@"; do
    [ "$prev" = -C ] && [ "$w" = "$want" ] && return 0
    [ "$w" = "-C$want" ] && return 0
    prev=$w
  done
  return 1
}
# Each of the crate's commands (a build can run more than one) carries
# every flag. Cargo prints `Running \`COMMAND\``: the words between the
# backticks, so that a flag written last is not read with the closing
# one.
while IFS= read -r rustc_line; do
  rustc_line=${rustc_line#*\`}
  rustc_line=${rustc_line%\`*}
  set -f
  # shellcheck disable=SC2086 # a word list
  for f in ${BENCH_FLAGS:-}; do
    has_flag "$f" ||
      die "$crate was built without -C $f: CARGO_ENCODED_RUSTFLAGS, RUSTFLAGS or a target rustflags entry replaced the port's build.rustflags; include it there (BENCH_FLAGS)"
  done
  set +f
done < "$work/rustc.lines"
if [ -n "${BENCH_LINK:-}" ]; then
  sh -c "$BENCH_LINK" || die "BENCH_LINK failed: $BENCH_LINK"
fi
lib=$work/lib
rm -rf "${lib:?}"
mkdir -p "$lib" "$out"
# shellcheck disable=SC2086 # patterns, globbed here
for p in $BENCH_SO; do
  n=0
  for f in $p; do
    [ -e "$f" ] || [ -L "$f" ] || continue
    cp -P "$f" "$lib/"
    n=$((n + 1))
  done
  [ "$n" -gt 0 ] || die "BENCH_SO pattern $p matches no file"
done
prog=$work/bench
set -f
# shellcheck disable=SC2086 # word lists
"$cc" $cflags -o "$prog" "$BENCH_SRC" ${BENCH_LDLIBS:-} || die "the bench program did not build"
set +f

# ------------------------------------------------------------ environment
# The source: the commit when .git is usable, else the git tree id of the
# files, which equals `git rev-parse HEAD^{tree}` for a clean tree.
source_id() {
  if c=$(git -c safe.directory='*' rev-parse --verify -q HEAD 2> /dev/null); then
    if [ -n "$(git -c safe.directory='*' status --porcelain 2> /dev/null)" ]; then
      echo "commit $c, with local changes"
    else
      echo "commit $c"
    fi
    return
  fi
  g=$(mktemp -d)
  if git init -q "$g" 2> /dev/null &&
    GIT_DIR=$g/.git GIT_WORK_TREE=$root git -c safe.directory='*' add -A 2> /dev/null &&
    t=$(GIT_DIR=$g/.git git write-tree 2> /dev/null); then
    echo "tree $t (no usable .git: compare with git rev-parse HEAD^{tree})"
  else
    echo "unknown (no git)"
  fi
  rm -rf "${g:?}"
}

cpu_model() {
  m=$(sed -n 's/^model name[[:space:]]*: //p' /proc/cpuinfo 2> /dev/null | head -n 1)
  if [ -z "$m" ]; then
    impl=$(sed -n 's/^CPU implementer[[:space:]]*: //p' /proc/cpuinfo 2> /dev/null | head -n 1)
    part=$(sed -n 's/^CPU part[[:space:]]*: //p' /proc/cpuinfo 2> /dev/null | head -n 1)
    case $impl/$part in
      0x41/0xd0c) m="Arm Neoverse-N1" ;;
      0x41/0xd40) m="Arm Neoverse-V1" ;;
      0x41/0xd49) m="Arm Neoverse-N2" ;;
      0x41/0xd4f) m="Arm Neoverse-V2" ;;
      0x41/0xd8e) m="Arm Neoverse-N3" ;;
      0x41/0xd84) m="Arm Neoverse-V3" ;;
      0x61/*) m="Apple" ;;
      *) m="unknown" ;;
    esac
    m="$m (CPU implementer $impl, part $part)"
  fi
  echo "$m"
}

arch=$(uname -m)
ref_version=$("$prog" --version) || die "$prog --version failed"
port_version=$(LD_LIBRARY_PATH=$lib "$prog" --version) || die "$prog --version failed on $lib"
env_report() {
  echo "source:    $(source_id)"
  echo "date:      $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "machine:   $arch, $(cpu_model), nproc $(nproc)"
  echo "kernel:    $(uname -r)"
  pkg=
  if [ -n "${BENCH_REF_FILE:-}" ]; then
    pkg=$(apk info -W "$BENCH_REF_FILE" 2> /dev/null | sed -n 's/.* is owned by //p') || pkg=
  fi
  echo "$BENCH_REF: $ref_version (${pkg:-package unknown})" | tr '\t' ' '
  echo "$BENCH_PORT: $port_version" | tr '\t' ' '
  echo "cc:        $("$cc" --version | head -n 1)"
  echo "rustc:     $(rustc --version)"
}

# ------------------------------------------------------------------ suite
if [ -n "$suite" ]; then
  env_report
  mkdir -p "$out/$suite"
  echo "== suite $suite: $suites/$suite/run.sh $*"
  env "${prefix}_LIB=$lib" "${prefix}_ROOT=$root" "${prefix}_BENCH=$prog" \
    "${prefix}_BENCH_OUT=$out/$suite" sh "$suites/$suite/run.sh" "$@"
  exit
fi

# -------------------------------------------------------------------- run
tsv=$out/$BENCH_PORT-bench-$arch.tsv
txt=$out/$BENCH_PORT-bench-$arch.txt
env_report | tee "$txt"
echo "args:      ${*:-(defaults)}" | tee -a "$txt"
: > "$tsv"
arms=
for side in "$BENCH_REF" "$BENCH_PORT"; do
  set -f
  # shellcheck disable=SC2086 # a word list
  for mode in $modes; do
    set +f
    arms="$arms $side/$mode"
    echo "== $side $mode" >&2
    set -f
    if [ "$side" = "$BENCH_PORT" ]; then
      # shellcheck disable=SC2086 # a word list
      LD_LIBRARY_PATH=$lib "$prog" --lib "$side" --mode "$mode" ${BENCH_PORT_ARGS:-} "$@" > "$work/arm.tsv" ||
        die "the bench program failed for $side $mode"
    else
      # shellcheck disable=SC2086 # a word list
      "$prog" --lib "$side" --mode "$mode" ${BENCH_REF_ARGS:-} "$@" > "$work/arm.tsv" ||
        die "the bench program failed for $side $mode"
    fi
    set +f
    awk -F '\t' -v s="$side" -v m="$mode" '
      NF != 6 { printf "line %d has %d fields, want 6: %s\n", NR, NF, $0; exit 1 }
      $1 != s || $2 != m { printf "line %d is for %s %s, want %s %s: %s\n", NR, $1, $2, s, m, $0; exit 1 }
      $3 == "" || $5 == "" || $6 == "" { printf "line %d has an empty workload, unit or count: %s\n", NR, $0; exit 1 }
      $4 !~ /^[0-9]+(\.[0-9]+)?([eE][-+]?[0-9]+)?$/ { printf "line %d has value %s, want a number: %s\n", NR, $4, $0; exit 1 }
      $3 in seen { printf "line %d repeats workload %s\n", NR, $3; exit 1 }
      { seen[$3] = 1 }
      END { if (NR == 0) { print "no workload lines"; exit 1 } }
    ' "$work/arm.tsv" > "$work/arm.err" || die "the bench program's output for $side $mode: $(cat "$work/arm.err")"
    cat "$work/arm.tsv" >> "$tsv"
  done
  set +f
done

# The comparison: one row per workload, each arm's value, the port's
# first mode over each reference arm (>1: the port is faster; MB/s values
# are rates, the others times) and the count, after checking the counts.
status=0
awk -F '\t' -v arms="$arms" -v ref="$BENCH_REF" -v port="$BENCH_PORT" -v modes="$modes" '
  BEGIN {
    nk = split(arms, keys, " ")
    nm = split(modes, mode, " ")
    lead = port "/" mode[1]
  }
  {
    w = $3; k = $1 "/" $2
    if (!(w in unit)) { order[++nw] = w; unit[w] = $5 }
    val[w, k] = $4; cnt[w, k] = $6; has[w, k] = 1
  }
  function ratio(w, base,   a, b, t) {
    if (!has[w, base] || !has[w, lead]) return "-"
    a = val[w, lead] + 0; b = val[w, base] + 0
    if (unit[w] != "MB/s") { t = a; a = b; b = t }
    if (b <= 0 || a <= 0) return "-"
    return sprintf(a / b >= 100 ? "%.0fx" : "%.2fx", a / b)
  }
  function num(v) {
    v += 0
    if (v >= 1000) return sprintf("%.0f", v)
    if (v >= 10) return sprintf("%.1f", v)
    if (v >= 0.01) return sprintf("%.3f", v)
    return sprintf("%.2e", v)
  }
  function show(w, k) { return has[w, k] ? num(val[w, k]) : "-" }
  END {
    line = sprintf("%-26s %-5s", "workload", "unit")
    for (j = 1; j <= nk; j++) line = line sprintf(" %14s", keys[j])
    for (j = 1; j <= nm; j++) line = line sprintf(" %14s", "vs " ref "/" mode[j])
    print line "  count"
    bad = 0
    for (i = 1; i <= nw; i++) {
      w = order[i]; r = ""; nrc = 0; n = 0; mism = ""
      for (j = 1; j <= nk; j++) {
        k = keys[j]
        if (!has[w, k]) continue
        n++
        if (cnt[w, k] ~ /^rc=/) { nrc++; continue }
        if (r == "") r = cnt[w, k]
        else if (cnt[w, k] != r) mism = "counts differ"
      }
      if (nrc > 0 && nrc < n) mism = "some runs give a return code, some a count"
      for (j = 1; j <= nm; j++)
        if (has[w, ref "/" mode[j]] != has[w, port "/" mode[j]])
          mism = "missing from one library"
      rcs = ""; rcdiff = 0
      for (j = 1; j <= nk; j++) {
        k = keys[j]
        if (!has[w, k] || cnt[w, k] !~ /^rc=/) continue
        if (rcs == "") rcs = cnt[w, k]
        else if (cnt[w, k] != rcs) rcdiff = 1
      }
      count = r
      if (nrc) count = rcdiff ? "rc differs (allowed)" : rcs
      line = sprintf("%-26s %-5s", w, unit[w])
      for (j = 1; j <= nk; j++) line = line sprintf(" %14s", show(w, keys[j]))
      for (j = 1; j <= nm; j++) line = line sprintf(" %14s", ratio(w, ref "/" mode[j]))
      print line "  " count
      detail = ""
      for (j = 1; j <= nk; j++)
        if (has[w, keys[j]]) detail = detail " " keys[j] "=" cnt[w, keys[j]]
      if (mism != "") { bad = 1; notes = notes "MISMATCH " w ": " mism ":" detail "\n" }
      else if (rcdiff) notes = notes "differs (allowed): " w ":" detail "\n"
    }
    printf "%s", notes
    printf "vs %s/MODE: %s over %s in that mode, >1 is faster\n", ref, lead, ref
    exit bad
  }
' "$tsv" > "$work/table" || status=$?
tee -a "$txt" < "$work/table"
echo "results: $tsv, $txt"
if [ "$status" != 0 ]; then
  echo "bench: MISMATCH: the libraries disagree on a count" >&2
  exit 1
fi
