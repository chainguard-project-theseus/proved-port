#!/bin/sh
# ci/miri.sh: {CAPI_CRATE}'s tests under Miri (the miri and miri-tree
# checks, .chainguard/ci/miri.yaml). Miri interprets the tests and reports
# undefined behaviour in the crate's unsafe code: an out-of-bounds or
# misaligned access, a use after free, a leak, or a pointer used against
# the aliasing model. The other crates forbid unsafe code, so only
# {CAPI_CRATE} runs: its library's unit tests and one binary per file
# under its tests/ (cargo's autotests), so a new test file is picked up,
# less the files `nomiri` names (a test that links C, which Miri cannot
# run). A binary whose listing fails or lists no tests fails the script.
#
# Miri needs nightly, which Wolfi does not package, so this installs a
# dated nightly with rustup under target/miri (ci/lib.sh rustup_pinned):
# rustup-init by version and SHA-256, and the toolchain by date and the
# SHA-256 of its channel manifest.
#
# ci/miri-known.txt lists the tests that hit an open, Miri-confirmed bug,
# each with its issue. They are left out of the runs below, and the
# Stacked Borrows pass then runs each of them alone and requires it to
# fail still, with undefined behaviour: one that passes fails this script,
# so the fix that makes it pass deletes its row, and so does one that fails
# another way, so its row is updated. A row that names no listed test, or
# cites no issue, fails it too.
#
# A report only Tree Borrows makes (the test passes under Stacked
# Borrows) has a row whose third word is `tree-borrows`. Its test runs in
# the Stacked Borrows passes like any other, is left out of the Tree
# Borrows pass, and that pass requires it to fail still, under Tree
# Borrows, the same way.
#
# Runs, with the tests split round-robin into MIRI_JOBS shards (default
# nproc) that run at once: every other test under Stacked Borrows, Miri's
# default; with `thread_tests=yes`, the tests that start threads (found as
# described below) again over the scheduler seeds MIRI_SEEDS (default
# 0..4, -Zmiri-many-seeds, which runs the seeds in parallel); then every
# other test under Tree Borrows. MIRI_STACKED_BORROWS=0 skips the first
# two, MIRI_TREE_BORROWS=0 the last: the miri check runs the first two,
# the on-request miri-tree check the last. A test that cannot run under
# Miri, or runs fewer rounds there, says so with cfg(miri). `flags` holds
# the flags the crate needs on every run, each with its reason; MIRIFLAGS
# adds flags to every run.
set -eu

nightly="{MIRI_NIGHTLY}"
toolchain=nightly-$nightly
# sha256 of https://static.rust-lang.org/dist/{MIRI_NIGHTLY}/channel-rust-nightly.toml
manifest_sum="{MIRI_MANIFEST_SHA256}"
crate="{CAPI_CRATE}"
capi="{CAPI_CRATE_DIR}"
# The files under $capi/tests that Miri cannot run, by name without .rs,
# split by spaces, each with its reason here.
nomiri="{MIRI_NO_RUN: the test files that link C, such as c_link_test, or nothing}"
# yes when a test of the crate starts threads, so the seeds pass runs and
# at least one thread test must be found; no when none does, so the first
# one fails the script until this says yes.
thread_tests="{MIRI_THREAD_TESTS: yes | no}"
# The flags every run needs, each with its reason here.
flags="{MIRI_FLAGS: such as -Zmiri-permissive-provenance for an integer-to-pointer cast the crate keeps, or nothing}"
seeds=${MIRI_SEEDS:-0..4}

# shellcheck source=ci/lib.sh
. "$(dirname "$0")/lib.sh"
rust_host miri >/dev/null

case $thread_tests in
  yes | no) ;;
  *) echo "miri: thread_tests is $thread_tests, not yes or no" >&2; exit 1 ;;
esac

top=$(pwd)
known=$top/ci/miri-known.txt
home=$top/target/miri
export RUSTUP_HOME="$home/rustup" CARGO_HOME="$home/cargo"
export CARGO_TARGET_DIR="$home/target" RUSTUP_INIT_SKIP_PATH_CHECK=yes
# cargo miri setup builds its sysroot under the cache directory.
export XDG_CACHE_HOME="$home/cache"
export PATH="$CARGO_HOME/bin:$PATH"
mkdir -p "$home"
start=$(date +%s)

rustup_pinned miri "$home" "https://static.rust-lang.org/dist/$nightly/channel-rust-nightly.toml" \
  "$manifest_sum" "$toolchain" --component miri,rust-src
cargo miri setup

# The test binaries: lib, and test:NAME for each file under tests/ but
# those nomiri names, each of which must exist.
for n in $nomiri; do
  if [ ! -f "$capi/tests/$n.rs" ] && [ ! -f "$capi/tests/$n/main.rs" ]; then
    echo "miri: nomiri names $n, which is not a test file under $capi/tests" >&2
    exit 1
  fi
done
bins=lib
for f in "$capi"/tests/*.rs "$capi"/tests/*/main.rs; do
  [ -f "$f" ] || continue
  case $f in
    */main.rs) t=${f%/main.rs} ;;
    *) t=${f%.rs} ;;
  esac
  t=${t##*/}
  case " $nomiri " in
    *" $t "*) continue ;;
  esac
  bins="$bins test:$t"
done
# binargs BIN: cargo's arguments for BIN (lib, or test:NAME).
binargs() {
  case $1 in
    lib) echo --lib ;;
    *) echo "--test ${1#test:}" ;;
  esac
}

# Every test as BIN NAME.
all=$home/tests.txt
: >"$all"
for bin in $bins; do
  # shellcheck disable=SC2046 # the binary's arguments are two words
  cargo miri test --locked -p "$crate" $(binargs "$bin") --no-run
  # shellcheck disable=SC2046
  if ! cargo miri test --locked -q -p "$crate" $(binargs "$bin") -- --list --format terse \
    >"$home/listing.txt" 2>"$home/listing.err"; then
    cat "$home/listing.err" >&2
    echo "miri: listing the tests of $(binargs "$bin") failed" >&2
    exit 1
  fi
  n=$(sed -n "s/: test\$//p" "$home/listing.txt" | sed "s|^|$bin |" | tee -a "$all" | wc -l)
  if [ "$n" -eq 0 ]; then
    cat "$home/listing.txt" "$home/listing.err" >&2
    echo "miri: $(binargs "$bin") lists no tests" >&2
    exit 1
  fi
  echo "miri: $(binargs "$bin"): $n tests"
done

# The known rows: TEST #ISSUE [tree-borrows] REASON, '#' comments and
# blank lines aside. Each must name exactly one listed test and cite an
# issue. rows holds the Stacked Borrows rows, tree_rows the Tree Borrows
# ones; skip the tests the Stacked Borrows passes leave out, tree_skip
# those the Tree Borrows pass does (both kinds).
rows=$home/rows.txt
tree_rows=$home/tree-rows.txt
awk '!/^[ \t]*(#|$)/' "$known" >"$home/all-rows.txt"
: >"$rows"
: >"$tree_rows"
skip=$home/known.txt
tree_skip=$home/tree-known.txt
: >"$skip"
: >"$tree_skip"
bad=0
while read -r name issue model _; do
  case $issue in
    \#[0-9]*) ;;
    *) echo "miri: ci/miri-known.txt: $name cites no issue (#n)" >&2; bad=1 ;;
  esac
  awk -v t="$name" '$2 == t' "$all" >"$home/match.txt"
  c=$(wc -l <"$home/match.txt")
  if [ "$c" -ne 1 ]; then
    echo "miri: ci/miri-known.txt: $name matches $c listed tests, not one" >&2
    bad=1
  fi
  if [ "$model" = tree-borrows ]; then
    echo "$name $issue" >>"$tree_rows"
  else
    echo "$name $issue" >>"$rows"
    cat "$home/match.txt" >>"$skip"
  fi
  cat "$home/match.txt" >>"$tree_skip"
done <"$home/all-rows.txt"
[ "$bad" = 0 ] || exit 1
list=$home/run.txt
tree_list=$home/tree-run.txt
sort "$all" >"$home/all.sorted"
sort "$skip" | comm -23 "$home/all.sorted" - >"$list"
sort "$tree_skip" | comm -23 "$home/all.sorted" - >"$tree_list"
echo "miri: $(wc -l <"$skip") known (ci/miri-known.txt), $(wc -l <"$list") to run"
echo "miri: $(wc -l <"$tree_rows") known under Tree Borrows only, $(wc -l <"$tree_list") to run there"

# The tests that start threads: every #[test] fn of the binaries above
# whose body calls thread::spawn or thread::scope, or carries a
# "miri: threads" comment (for one that starts threads through a helper),
# less those whose body carries a "miri: no-seeds" comment saying why and
# the plain known ones (a `tree-borrows` row's test runs over the seeds,
# which use Stacked Borrows). The scan reads source text, so it also selects a test
# that cfg(not(miri)) or a feature leaves out of the Miri build: a name
# the listing above does not hold is skipped as not built under Miri. A
# name that matches more than one listed test is an error.
#
# nomiri_file PATH: whether PATH belongs to a test binary nomiri names.
nomiri_file() {
  rel=${1#"$capi"/tests/}
  [ "$rel" != "$1" ] || return 1
  b=${rel%%/*}
  b=${b%.rs}
  case " $nomiri " in
    *" $b "*) return 0 ;;
  esac
  return 1
}
srcs=$(find "$capi/src" "$capi/tests" -name '*.rs' 2>/dev/null | while read -r f; do
  nomiri_file "$f" || echo "$f"
done)
[ -n "$srcs" ] || { echo "miri: no sources under $capi/src" >&2; exit 1; }
# shellcheck disable=SC2086 # the paths have no spaces
awk '
  FNR == 1 { cur = ""; pending = 0 }
  /#\[test\]/ { pending = 1 }
  match($0, /^[ \t]*(pub(\([a-z]+\))? )?fn [A-Za-z0-9_]+/) {
    name = substr($0, RSTART, RLENGTH); sub(/.*fn /, "", name)
    indent = match($0, /[^ \t]/)
    if (pending) { cur = name; depth = indent; pending = 0 }
    else if (indent <= depth) cur = ""
  }
  cur != "" && /thread::(spawn|scope)|miri: threads/ { thr[cur] = 1 }
  cur != "" && /miri: no-seeds/ { skip[cur] = 1 }
  END { for (t in thr) if (!(t in skip)) print t }
' $srcs | sort -u >"$home/thread-names.txt"
threads=$home/threads.txt
: >"$threads"
while IFS= read -r name; do
  awk -v t="$name" '{ n = $NF } n == t || substr(n, length(n) - length(t) - 1) == "::" t' \
    "$all" >"$home/match.txt"
  c=$(wc -l <"$home/match.txt")
  if [ "$c" -eq 0 ]; then
    echo "miri: thread test $name is not in the Miri build (cfg(not(miri)) or a feature); no seeds pass for it"
    continue
  fi
  if [ "$c" -ne 1 ]; then
    echo "miri: thread test $name matches $c listed tests, not one" >&2
    exit 1
  fi
  # BusyBox grep with -f and an empty file selects no line under -v -F
  # rather than every line, so an empty known list skips the grep.
  if [ -s "$skip" ]; then
    grep -vxFf "$skip" "$home/match.txt" >>"$threads" || :
  else
    cat "$home/match.txt" >>"$threads"
  fi
done <"$home/thread-names.txt"
if [ "$thread_tests" = yes ]; then
  [ -s "$threads" ] || { echo "miri: no thread tests found, but thread_tests=yes" >&2; exit 1; }
  echo "miri: thread tests:"
  sed 's/^/  /' "$threads"
elif [ -s "$threads" ]; then
  sed 's/^/  /' "$threads" >&2
  echo "miri: these tests start threads: set thread_tests=yes" >&2
  exit 1
fi

# run LABEL FLAGS LIST SHARDS: the tests in LIST under MIRIFLAGS=FLAGS,
# round-robin into SHARDS shards; fails if any shard fails. Each shard
# runs its tests of each binary in one Miri process, which stops at the
# first undefined behaviour: a failing shard's later tests did not run.
run() {
  label=$1
  shards=$4
  t=$(date +%s)
  echo "miri: $label: $(wc -l <"$3") tests in $shards shards"
  i=0
  pids=
  while [ "$i" -lt "$shards" ]; do
    awk -v n="$shards" -v i="$i" 'NR % n == i' "$3" >"$home/shard$i"
    (
      rc=0
      for bin in $bins; do
        names=$(awk -v b="$bin" '$1 == b { print $2 }' "$home/shard$i")
        [ -n "$names" ] || continue
        # shellcheck disable=SC2046,SC2086 # the binary is two words, names one each
        MIRIFLAGS=$2 cargo miri test --locked -q -p "$crate" $(binargs "$bin") -- --exact $names </dev/null || rc=1
      done
      exit "$rc"
    ) >"$home/shard$i.log" 2>&1 &
    pids="$pids $!"
    i=$((i + 1))
  done
  failed=
  i=0
  for pid in $pids; do
    if ! wait "$pid"; then failed="$failed $i"; fi
    i=$((i + 1))
  done
  i=0
  while [ "$i" -lt "$shards" ]; do
    echo "--- shard $i"
    grep -E '^test |^test result|^error|^warning: unused|panicked' "$home/shard$i.log" || :
    i=$((i + 1))
  done
  if [ -n "$failed" ]; then
    for i in $failed; do
      echo "--- shard $i failed:"
      cat "$home/shard$i.log"
    done
    echo "miri: $label failed in $(($(date +%s) - t))s" >&2
    exit 1
  fi
  echo "miri: $label passed in $(($(date +%s) - t))s"
}

# still_fail FLAGS ROWS: each known test of ROWS alone under
# MIRIFLAGS=FLAGS must still fail with undefined behaviour, the failure
# its row cites. Passing, or failing any other way (an unsupported
# operation, a leak, a panic), fails the check, so the row is updated or
# deleted.
still_fail() {
  stale=
  while read -r name issue _; do
    bin=$(awk -v t="$name" '$2 == t { print $1 }' "$all")
    # shellcheck disable=SC2046 # the binary's arguments are two words
    if MIRIFLAGS=$1 cargo miri test --locked -q -p "$crate" $(binargs "$bin") -- --exact "$name" \
      >"$home/known.log" 2>&1 </dev/null; then
      stale="$stale $name"
      echo "miri: $name passes; $issue may be fixed: delete its ci/miri-known.txt row" >&2
    elif grep -q '^error: Undefined Behavior' "$home/known.log"; then
      echo "miri: $name still fails ($issue): $(grep -m1 '^error: Undefined Behavior' "$home/known.log")"
    else
      stale="$stale $name"
      echo "miri: $name fails without undefined behaviour; its $issue row no longer describes it" >&2
      cat "$home/known.log" >&2
    fi
  done <"$2"
  [ -z "$stale" ]
}

base="$flags ${MIRIFLAGS:-}"
jobs=${MIRI_JOBS:-$(nproc)}
if [ "${MIRI_STACKED_BORROWS:-1}" = 1 ]; then
  run "stacked borrows" "$base" "$list" "$jobs"
  if [ "$thread_tests" = yes ]; then
    # One shard: -Zmiri-many-seeds runs the seeds in parallel itself.
    run "threads, seeds $seeds" "$base -Zmiri-many-seeds=$seeds" "$threads" 1
  fi
  still_fail "$base" "$rows"
fi
if [ "${MIRI_TREE_BORROWS:-1}" = 1 ]; then
  run "tree borrows" "$base -Zmiri-tree-borrows" "$tree_list" "$jobs"
  still_fail "$base -Zmiri-tree-borrows" "$tree_rows"
fi
echo "miri: passed in $(($(date +%s) - start))s"
