#!/bin/sh
# codegen.sh MODE ...: whether a change changed the machine code of the
# port's release library (`make codegen`, `make codegen-capture`, `make
# codegen-compare`, proved-port.mk), which the theseus-proved-port
# package installs in /usr/share/proved-port/make/. It is challoc's
# scripts/codegen-diff.sh, ci/codegen.sh and ci/codegen-notices.sh in one
# script, with the library, the builds and the comparer as settings, and
# the comparer the kit's fncmp (kit/perf/codegen/fncmp) in place of
# challoc's scripts/codegen-fncmp.py. Run it from the port's root.
#
#   codegen.sh capture DIR   build each build CODEGEN_BUILDS names, in
#                            release, and save its library, the library's
#                            `objdump -d --no-show-raw-insn` dump and its
#                            section sizes under DIR
#   codegen.sh compare DIR   build them again and compare each with DIR's
#   codegen.sh base          capture the base revision
#                            (CHAINGUARD_BASE_REVISION, else CODEGEN_BASE)
#                            in a git worktree, compare this tree with it,
#                            then print the report's verdicts as
#                            `::notice` lines (a port's codegen checks)
#   codegen.sh notices ARCH REPORT
#                            print a compare report's verdicts as
#                            `::notice` lines (below)
#
# compare prints, per build (`== NAME`):
#   .text hash          identical => byte-identical machine code, done
# and when the hash differs, fncmp's report: each function paired by
# name with its baseline function is one of
#   exact               instruction text matches once addresses are
#                       stripped
#   reloc-only          matches once immediates are masked too (moving
#                       code between files shifts .rodata and every
#                       PC-relative offset with it)
#   padding-only        matches once trailing alignment nops are dropped
#                       too (the function moved, and the gap to the next
#                       aligned symbol changed)
#   erratum-stub        GNU ld's Cortex-A53 erratum 843419 workaround (an
#                       `adrp` at page offset 0xff8/0xffc became `adr`,
#                       or its load a branch to a stub after the body)
#   changed             anything else, with its full unified diff, plus
#                       every baseline function left with no counterpart
# A new name pairs with a leftover baseline function whose name differs
# only in the crate hash or impl disambiguator, else with one of the same
# instructions, as a rename (fncmp's header says how). A refactor that
# moves code without changing it shows `changed 0`. Anything changed is
# a real code change.
#
# Exit status: 0 when no build has a changed function; 2 when one has,
# or fncmp fails on one (its report then says `fncmp failed (exit N)`);
# 1 when a build fails or a tool is missing; 2 for a setting missing or
# malformed, before any build. base exits as its compare does.
#
# Each build NAME is `cargo build --release --lib` with
# CODEGEN_CARGO_FLAGS and CODEGEN_FLAGS_<NAME>, in its own target
# directory, CODEGEN_TARGET_DIR/codegen-NAME; the library is
# release/CODEGEN_LIB there, or TRIPLE/release/CODEGEN_LIB when the
# flags name one target. challoc's two quanta are
#   CODEGEN_BUILDS=q16 q8
#   CODEGEN_FLAGS_q8=--no-default-features --features quantum8
#
# The environment; empty is the same as unset:
#
#   CODEGEN_LIB          the library's file name, as cargo writes it
#                        (libchalloc.so) (required)
#   CODEGEN_BUILDS       the builds, by name ([A-Za-z0-9_], by default
#                        one, `default`)
#   CODEGEN_FLAGS_<NAME> cargo flags for that build only
#   CODEGEN_CARGO_FLAGS  cargo flags for every build (--manifest-path,
#                        -p for a library in a virtual workspace)
#   CODEGEN_TARGET_DIR   by default target, from the port's root; base's
#                        capture builds in a target directory of its own,
#                        removed after the run
#   CODEGEN_FNCMP        the comparer: fncmp built from the kit's
#                        kit/perf/codegen/fncmp (by default `fncmp`,
#                        from PATH); compare and base look it up before
#                        any build, and fail (exit 1) without it
#   CODEGEN_DIFF         base only: the script base runs for its capture
#                        and compare in place of this one (a test's
#                        stand-in)
#
# It needs cargo, objdump, objcopy, readelf, sha256sum and git; fncmp
# uses c++filt, when it is on PATH, for its demangled rename key.
#
# notices: for each build (`== NAME`) of a compare report, at most two
# `::notice title=codegen ARCH NAME::...` lines, which Checks lists at
# the top of a check run's output text (it shows only the first 20):
# the build's `functions old ..` line, or `.text identical` when the hash
# matched, or the fncmp failure; then, when functions changed, one
# `changed (N): ..` line listing them, each as its demangled name (the
# report's mangled one when it gives none) without crate hashes, then
# `(old -> new)` instruction counts (`(old -> new; baseline M)` when the
# baseline is another function), `(new, N)` for a function with no
# baseline, or `(removed, N)` for a baseline function with no
# counterpart. At most 40 names, then a count of the rest.
set -eu

usage() {
  echo "usage: codegen.sh capture DIR | compare DIR | base | notices ARCH REPORT" >&2
  exit 2
}
# The container runs as root over a tree another user owns, which git
# refuses as "dubious ownership" without safe.directory.
git="git -c safe.directory=*"

notices() { # notices ARCH REPORT
  awk -v arch="$1" '
    function note(msg) {
      gsub(/%/, "%25", msg)
      print "::notice title=codegen " arch " " v "::" msg
    }
    function count(f) { sub(/^\(/, "", f); return f }
    # fncmp`s key for a mangled name: no crate hash or impl
    # disambiguator, and every backref spelled B_.
    function key(s) {
      gsub(/Cs[0-9A-Za-z]+_|Ms[0-9A-Za-z]*_/, "", s)
      gsub(/B[0-9A-Za-z]*_/, "B_", s)
      return s
    }
    # A changed function waits for the `= demangled` line that may follow.
    function put() {
      if (name == "") return
      gsub(/\[[0-9a-f]+\]/, "", name)
      if (++n <= 40) list = list (n > 1 ? "; " : "") name " (" insns ")"
      name = ""
    }
    function flush() {
      put()
      if (n > 0) note("changed (" n "): " list (n > 40 ? "; and " (n - 40) " more (see the log)" : ""))
      n = 0; list = ""
    }
    /^== / { flush(); v = $2; orphan = 0; next }
    v == "" { next }
    name != "" && /^       = / { sub(/^       = /, ""); name = $0; put(); next }
    { put() }
    /^   \.text hash .* IDENTICAL$/ { note(".text identical: byte-identical machine code"); next }
    /^   functions old / { sub(/^   /, ""); note($0); next }
    /^   fncmp failed/ { sub(/^   /, ""); note($0); next }
    # ### changed: NAME (A insns) vs baseline M (B insns)
    # ### changed: NAME (A insns) vs no baseline counterpart
    # When M is another function (fncmp pairs a leftover by instruction
    # similarity), the old count is M`s, so M is named too:
    # `(B -> A; baseline M)`. M under a new crate hash or impl
    # disambiguator, by fncmp`s key, is the same function.
    /^   ### changed: / {
      name = $3
      if ($0 ~ /vs no baseline counterpart$/) insns = "new, " count($4)
      else {
        insns = count($(NF - 1)) " -> " count($4)
        if (key($8) != key($3)) insns = insns "; baseline " $8
      }
      next
    }
    /^   baseline functions with no counterpart/ { orphan = 1; next }
    # NAME (B insns), or NAME = DEMANGLED (B insns)
    orphan && /^      [^ ]/ {
      name = $1
      if ($2 == "=") { name = $3; for (i = 4; i <= NF - 2; i++) name = name " " $i }
      insns = "removed, " count($(NF - 1))
      put(); next
    }
    { orphan = 0 }
    END { flush() }
  ' "$2"
}

# The settings capture and compare read.
settings() {
  lib=${CODEGEN_LIB-}
  builds=${CODEGEN_BUILDS:-default}
  tdir=${CODEGEN_TARGET_DIR:-target}
  fncmp=${CODEGEN_FNCMP:-fncmp}
  case $lib in
    '') echo "codegen.sh: CODEGEN_LIB is empty: name the library's file (libchalloc.so)" >&2; exit 2 ;;
    */*) echo "codegen.sh: CODEGEN_LIB is $lib: a file name, with no directory" >&2; exit 2 ;;
  esac
  for b in $builds; do
    case $b in
      *[!A-Za-z0-9_]*) echo "codegen.sh: CODEGEN_BUILDS names $b: a build name is [A-Za-z0-9_]" >&2; exit 2 ;;
    esac
  done
  for t in cargo objdump objcopy readelf sha256sum; do
    command -v "$t" >/dev/null || { echo "codegen.sh: $t is not on PATH" >&2; exit 1; }
  done
}
# need_fncmp: compare (and base, before its capture) needs the comparer,
# looked up before any build, so a missing one fails every run, not only
# the first whose machine code changed.
need_fncmp() {
  fncmp=${CODEGEN_FNCMP:-fncmp}
  command -v "$fncmp" >/dev/null || {
    echo "codegen.sh: the comparer $fncmp (CODEGEN_FNCMP) is not a command: build the kit's kit/perf/codegen/fncmp and name it" >&2
    exit 1
  }
}

build() { # build NAME: builds it and prints its library's path
  name=$1
  bdir=$tdir/codegen-$name
  bflags=
  eval "bflags=\${CODEGEN_FLAGS_$name-}"
  log=$out/.build-$name.log
  # An earlier run's library, in release/ or under a target triple, goes
  # first: the one found after the build is then the one it wrote, not
  # one left from a build with other flags (`--target` writes only
  # TRIPLE/release, and a failed build writes nothing).
  rm -f "$bdir/release/$lib" "$bdir"/*/release/"$lib"
  # shellcheck disable=SC2086 # the flags are words
  if ! CARGO_TARGET_DIR=$bdir cargo build --release --lib -q ${CODEGEN_CARGO_FLAGS-} $bflags > "$log" 2>&1; then
    cat "$log" >&2
    echo "codegen.sh: build $name failed" >&2
    exit 1
  fi
  rm -f "$log"
  found=
  for so in "$bdir/release/$lib" "$bdir"/*/release/"$lib"; do
    [ -f "$so" ] || continue
    if [ -n "$found" ]; then
      echo "codegen.sh: build $name wrote $lib in more than one of $bdir/release and $bdir/TRIPLE/release: give it one target" >&2
      exit 1
    fi
    found=$so
  done
  if [ -z "$found" ]; then
    echo "codegen.sh: build $name wrote no $lib in $bdir/release or $bdir/TRIPLE/release" >&2
    exit 1
  fi
  echo "$found"
}
# text_hash LIB: the first 16 hex digits of the SHA-256 of LIB's .text,
# failing (and printing nothing) when a tool fails or gives no digest,
# so a hash never compares equal by being empty on both sides.
text_hash() {
  if ! objcopy -O binary --only-section=.text "$1" "$out/.text.bin"; then
    echo "codegen.sh: objcopy could not copy the .text of $1" >&2
    rm -f "$out/.text.bin"
    return 1
  fi
  if ! sum=$(sha256sum "$out/.text.bin"); then
    echo "codegen.sh: sha256sum failed on the .text of $1" >&2
    rm -f "$out/.text.bin"
    return 1
  fi
  rm -f "$out/.text.bin"
  sum=${sum%% *}
  case $sum in
    [0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]*) ;;
    *)
      echo "codegen.sh: sha256sum gave no digest for the .text of $1" >&2
      return 1
      ;;
  esac
  printf '%s\n' "$sum" | cut -c1-16
}
sections() { readelf -S -W "$1" | awk '$2 ~ /^\.(text|rodata|data\.rel\.ro|data|bss)$/ {printf "%s=%s ", $2, $6}'; }
disasm() { objdump -d --no-show-raw-insn "$1"; }

case ${1-} in
  capture)
    [ $# -eq 2 ] || usage
    settings
    out=$2
    mkdir -p "$out"
    $git rev-parse HEAD > "$out/commit.txt"
    for b in $builds; do
      so=$(build "$b") || exit 1
      h=$(text_hash "$so") || exit 1
      cp "$so" "$out/lib-$b.so"
      disasm "$so" > "$out/disasm-$b.txt"
      echo "$b: .text $h $(sections "$so")functions $(grep -c '^[0-9a-f]* <.*>:$' "$out/disasm-$b.txt" || true)"
    done
    echo "captured at $(cat "$out/commit.txt")"
    ;;
  compare)
    [ $# -eq 2 ] || usage
    settings
    need_fncmp
    out=$2
    [ -f "$out/commit.txt" ] || { echo "codegen.sh: $out holds no capture (commit.txt)" >&2; exit 1; }
    # Every build's capture is there before any build runs, so no
    # verdict is printed for a run that then fails.
    for b in $builds; do
      [ -f "$out/lib-$b.so" ] && [ -f "$out/disasm-$b.txt" ] ||
        { echo "codegen.sh: $out has no capture of build $b" >&2; exit 1; }
    done
    echo "baseline $(cat "$out/commit.txt"), tree $($git rev-parse HEAD)$($git diff --quiet HEAD || echo ' + uncommitted changes')"
    rc=0
    for b in $builds; do
      so=$(build "$b") || exit 1
      oh=$(text_hash "$out/lib-$b.so") || exit 1
      nh=$(text_hash "$so") || exit 1
      disasm "$so" > "$out/disasm-$b-new.txt"
      echo "== $b"
      echo "   .text hash   old $oh  new $nh $([ "$oh" = "$nh" ] && echo IDENTICAL || echo differs)"
      echo "   sections old: $(sections "$out/lib-$b.so")"
      echo "   sections new: $(sections "$so")"
      if [ "$oh" != "$nh" ]; then
        frc=0
        "$fncmp" "$out/disasm-$b.txt" "$out/disasm-$b-new.txt" > "$out/report-$b.txt" || frc=$?
        if [ "$frc" -ne 0 ]; then
          echo "   fncmp failed (exit $frc)"
          rc=2
        fi
        sed 's/^/   /' "$out/report-$b.txt"
        grep -q 'changed 0$' "$out/report-$b.txt" || rc=2
      fi
    done
    exit $rc
    ;;
  base)
    [ $# -eq 1 ] || usage
    base=${CHAINGUARD_BASE_REVISION:-${CODEGEN_BASE-}}
    [ -n "$base" ] || { echo "codegen.sh: no base revision: set CHAINGUARD_BASE_REVISION or CODEGEN_BASE" >&2; exit 2; }
    self=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")
    diff=${CODEGEN_DIFF:-$self}
    # The base resolves, and the compare's tools are there, before the
    # capture spends a build on the base.
    $git rev-parse -q --verify "$base^{commit}" >/dev/null ||
      { echo "codegen.sh: the base revision $base is not a commit here" >&2; exit 1; }
    if [ -z "${CODEGEN_DIFF-}" ]; then
      settings
      need_fncmp
    fi
    dir=$(mktemp -d)
    trap '$git worktree remove --force "$dir/base" >/dev/null 2>&1 || true; rm -rf "$dir"' EXIT
    $git worktree add --detach "$dir/base" "$base" >/dev/null
    # The capture builds in a target directory of its own. Sharing the
    # tree's (an absolute CODEGEN_TARGET_DIR) would share cargo's
    # fingerprints: the base's build, newer than the tree's last edit,
    # would leave the tree's build fresh, and compare would compare the
    # base's library with itself.
    (cd "$dir/base" && CODEGEN_TARGET_DIR=$dir/target sh "$diff" capture "$dir/baseline")
    # The report streams to the log; a copy feeds the notices, and
    # compare's own exit status stays base's.
    { rc=0; sh "$diff" compare "$dir/baseline" || rc=$?; echo "$rc" > "$dir/rc"; } |
      tee "$dir/report.txt"
    notices "$(uname -m)" "$dir/report.txt"
    exit "$(cat "$dir/rc")"
    ;;
  notices)
    [ $# -eq 3 ] || usage
    notices "$2" "$3"
    ;;
  *) usage ;;
esac
