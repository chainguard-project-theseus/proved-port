#!/bin/sh
# noalias.sh [--self-test] [PORT_ROOT]: no inherent method of the type
# NOALIAS_TYPE takes a `noalias` `&mut self` in the LLVM IR rustc emits
# for the port's release library (`make noalias`, proved-port.mk), which
# the theseus-proved-port package installs in
# /usr/share/proved-port/make/. PORT_ROOT is the port's root (by default
# the current directory). It is challoc's ci/heap-noalias.sh (its
# heap-noalias check), with the crate, the type and the builds as
# settings.
#
# A port whose type's methods write through a raw pointer while a
# `&mut` to the type is live (challoc's `Heap` writes its home cache
# headers that way) is sound only while rustc marks no `&mut` of the
# type `noalias`, which it does not for a `!Unpin` type (a
# `PhantomPinned` field). This checks what rustc emitted. For each build
# NOALIAS_BUILDS names, one `cargo rustc --release --lib` that also
# writes the crate's own IR as rustc's code generation left it, at
# `-C opt-level=3` after the profile's (rustc marks parameters
# `noalias` only when it optimizes, so at opt-level 0 the run would show
# nothing; an opt-level in RUSTFLAGS or a config's rustflags still comes
# after it and wins, so a build in which no method's `self` is `noundef`,
# which rustc adds only when it optimizes, fails), before any LLVM pass (`-C no-prepopulate-passes`) and
# without fat LTO
# (`CARGO_PROFILE_RELEASE_LTO=false`; with LTO, `--emit=llvm-ir` writes
# the merged, optimized module, where inlined methods have no `define`
# and their `noalias` is `!noalias` metadata in the caller). On rustc
# 1.99.1 the IR before LLVM passes is the same with fat LTO on, so no
# test shows LTO's effect; the setting stays, as challoc has it, for a
# rustc whose IR is not. Before LLVM passes matters: they add
# `readonly` to a `&mut self` that only reads, which the scan would then
# list as a `&self` (noalias-test shows it). The IR is
# NOALIAS_CRATE[-HASH].ll (no hash for a cdylib or dylib), or one
# NOALIAS_CRATE[-HASH].*.rcgu.ll file for each codegen unit when the
# release profile has more than one, in release/deps, or in
# TRIPLE/release/deps for each target the build names (from cargo 1.100
# on, in the out directory of the library's unit,
# release/build/PACKAGE/HASH/out, or under TRIPLE/); each target's IR
# is scanned on its own and must pass on its own (its lines then name
# it). Each inherent method of the type
# that rustc emits out of line then has its own `define` (methods, below,
# reads the v0 symbols, and the type's path in them must be
# NOALIAS_TYPE's exactly), and a `%self` parameter that carries
# `noalias` but not `readonly` fails: a `&mut` receiver. A `&self`
# receiver of a `Freeze` type is `noalias readonly` whatever `Unpin`
# says; those are listed, not failed. So is a `self` passed by value in
# memory, the caller's copy, which rustc marks `dead_on_return noalias`
# (a rustc that does not mark it `dead_on_return` would have it
# flagged). A build fails when no method of the type with a `self`
# parameter is found, or only `&self` and by-value ones (no `&mut self`
# witness), so a symbol scheme the reader does not know
# cannot pass unseen. What that guard cannot tell: a `&self` of a type
# that is not `Freeze` (it holds a `Cell` or an atomic) is neither
# `noalias` nor `readonly`, as a `&mut self` of a `!Unpin` type is, and
# the IR's attributes do not tell the two apart, so such a `&self` also
# counts as the witness.
#
# Every run first scans saved `define` lines (the fixtures below,
# challoc's scripts/testdata/heap-noalias; no build), so a matcher that
# stops seeing a `noalias` self, or tools here that read the lines
# differently, turn the check red: each fixture's verdict must be the
# one its `; want:` lines give (red fails, clean passes, none has no
# method of the type and fails, ro-only has only a `&self` and fails).
# With --self-test it does only that, and needs no settings.
#
# The environment; empty is the same as unset:
#
#   NOALIAS_CRATE        the library's crate name, as rustc names it
#                        (`-` is read as `_`) (required)
#   NOALIAS_TYPE         the type: its module path in the crate and its
#                        name, `heap::Heap`, or `Heap` at the crate
#                        root; each part an ASCII identifier that starts
#                        with a letter (required)
#   NOALIAS_BUILDS       the builds, by name ([A-Za-z0-9_], by default
#                        one, `default`), each in its own target
#                        directory, NOALIAS_TARGET_DIR/noalias-NAME
#   NOALIAS_FLAGS_<NAME> cargo flags for that build only (challoc's q8:
#                        --no-default-features --features quantum8)
#   NOALIAS_CARGO_FLAGS  cargo flags for every build (--manifest-path,
#                        -p for a library in a virtual workspace)
#   NOALIAS_TARGET_DIR   by default target, from PORT_ROOT
#
# A setting missing or malformed refuses the run (exit 2) before any
# build.
set -eu

me=noalias.sh
self_test_only=
if [ "${1-}" = --self-test ]; then
  self_test_only=1
  shift
fi
[ $# -le 1 ] || { echo "usage: $me [--self-test] [PORT_ROOT]" >&2; exit 2; }
root=${1:-.}

refuse() {
  echo "$me: $*" >&2
  exit 2
}

# pattern CRATE TYPE: the path methods asks for, the crate and then
# TYPE's modules and name, separated by spaces (`challoc heap Heap`).
pattern() {
  printf '%s\n' "$1::$2" | sed 's/::/ /g'
}

# methods PATH: the `define` lines on standard input that are inherent
# methods of the type at PATH (pattern's), as v0 mangles them:
# `_R[I]NvM[s<dis>_]<impl path><type path><method>`. The type's path is
# read whole, through its backrefs (`B<base62>_`, an offset into the
# symbol after `_R`), and must be PATH exactly: a type of the same name
# in another module, or an impl of another type, is not one. A path is
# `C<crate>`, `N<namespace><path><identifier>` (only `t`, the type
# namespace, can be PATH's) or a backref; an identifier is an optional
# `s<base62>_` disambiguator, then its length in decimal, an optional
# `_`, and its bytes. A symbol in another shape (a punycode name, a
# generic type's arguments the path does not end at) is not counted, and
# when that leaves no `&mut self`, the run fails rather than passes.
methods() {
  awk -v want="$1" '
    function b62(p,   v, c) {
      if (substr(s, p, 1) == "_") { end = p + 1; return 0 }
      v = 0
      while ((c = index(digits, substr(s, p, 1))) > 0) { v = v * 62 + c - 1; p++ }
      if (substr(s, p, 1) != "_") { bad = 1; end = p; return 0 }
      end = p + 1
      return v + 1
    }
    function ident(p,   n) {
      if (substr(s, p, 1) == "s") { b62(p + 1); p = end }
      if (substr(s, p, 1) !~ /[0-9]/) { bad = 1; return p }
      n = 0
      while (substr(s, p, 1) ~ /[0-9]/) { n = n * 10 + substr(s, p, 1); p++ }
      if (substr(s, p, 1) == "_") p++
      id = substr(s, p, n)
      return p + n
    }
    function path(p, depth,   c, ns, save, q) {
      if (bad || depth > 64) { bad = 1; return p }
      c = substr(s, p, 1)
      if (c == "C") { p = ident(p + 1); ids = id; return p }
      if (c == "N") {
        ns = substr(s, p + 1, 1)
        p = path(p + 2, depth + 1)
        save = ids
        p = ident(p)
        ids = save " " (ns == "t" ? "" : "?") id
        return p
      }
      if (c == "B") {
        # A backref points before itself.
        q = b62(p + 1) + 1
        if (q >= p) { bad = 1; return end }
        p = end
        path(q, depth + 1)
        return p
      }
      bad = 1
      return p
    }
    BEGIN { digits = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ" }
    /^define / {
      if (!match($0, /@_R[0-9A-Za-z_]*\(/)) next
      s = substr($0, RSTART + 3, RLENGTH - 4)
      bad = 0
      p = 1
      if (substr(s, p, 1) == "I") p++
      if (substr(s, p, 3) != "NvM") next
      p += 3
      if (substr(s, p, 1) == "s") { b62(p + 1); p = end }
      p = path(p, 0)
      if (substr(s, p, 1) == "I") p++
      ids = ""
      path(p, 0)
      if (!bad && ids == want) print
    }'
}

# scan LL OUT TNAME PATTERN: the verdict for the IR files LL (one, or a
# space-separated list), with the flagged methods and then the `&self`
# ones after it; returns 1 on a failure. TNAME is the type's name for
# the messages, PATTERN pattern's. Scratch files go to OUT.
scan() {
  defs=$2/noalias-methods.txt
  bad=$2/noalias-flagged.txt
  ro=$2/noalias-readonly.txt
  bv=$2/noalias-byvalue.txt
  # shellcheck disable=SC2086 # $1 is a list of paths with no spaces
  grep -h '^define ' $1 | methods "$4" >"$defs"
  # The `self` parameter: from the `(` or `, ` before its `ptr` to
  # `%self` (attributes carry no `%`; `dereferenceable(N)` and
  # `captures(..)` carry parentheses and commas).
  n=0
  # Selves that carry noundef: rustc adds noundef, nonnull, noalias and
  # readonly to parameters only when it optimizes.
  nu=0
  : >"$bad"
  : >"$ro"
  : >"$bv"
  while IFS= read -r d; do
    p=$(printf '%s\n' "$d" | grep -oE '(\(|, )ptr [^%]*%self[,)]' || true)
    [ -n "$p" ] || continue
    n=$((n + 1))
    case $p in *' noundef '*) nu=$((nu + 1)) ;; esac
    name=$(printf '%s\n' "$d" | sed -E 's/^[^@]*@([^(]*)\(.*/  \1/')
    case $p in
      # A `self` passed by value in memory: the caller's copy, which
      # rustc marks dead_on_return (and noalias), not a borrow.
      *' dead_on_return '*) printf '%s\n' "$name" >>"$bv" ;;
      *' noalias '*' readonly '* | *' readonly '*' noalias '*) printf '%s\n' "$name" >>"$ro" ;;
      *' noalias '*) printf '%s\n' "$name" >>"$bad" ;;
    esac
  done <"$defs"
  st=0
  if [ "$n" -eq 0 ]; then
    echo "FAIL no $3 method with a self parameter in $1 (symbol pattern out of date?)"
    return 1
  elif [ "$nu" -eq 0 ]; then
    echo "FAIL no $3 method's self is noundef in $1: rustc did not optimize, so it marked nothing noalias (an opt-level in RUSTFLAGS or a config's rustflags?)"
    return 1
  elif [ "$n" -eq "$(($(wc -l <"$ro") + $(wc -l <"$bv")))" ]; then
    echo "FAIL no $3 method with a &mut self in $1, only &self or self by value (symbol pattern out of date?)"
    st=1
  elif [ -s "$bad" ]; then
    echo "FAIL $(wc -l <"$bad") of $n $3 methods take a noalias &mut self:"
    cat "$bad"
    st=1
  else
    echo "ok, $n $3 methods take self, no &mut self is noalias"
  fi
  if [ -s "$ro" ]; then
    echo "$(wc -l <"$ro") take a noalias readonly &self (allowed):"
    cat "$ro"
  fi
  if [ -s "$bv" ]; then
    echo "$(wc -l <"$bv") take self by value (allowed):"
    cat "$bv"
  fi
  return $st
}

# want LL TNAME: the verdict scan must give for the fixture LL, from its
# `; want:` lines (each names what the next `define` must count as).
want() {
  w=$(grep -A1 '^; want: ' "$1")
  names() { printf '%s\n' "$w" | grep -A1 "^; want: $1\$" | sed -nE 's/^define [^@]*@([^(]*)\(.*/  \1/p'; }
  f=$(names flagged)
  r=$(names readonly)
  v=$(names byvalue)
  n=$(grep -cE '^; want: (flagged|ok|readonly|byvalue)$' "$1" || true)
  m=$(grep -cE '^; want: (flagged|ok)$' "$1" || true)
  if [ "$n" -eq 0 ]; then
    echo "FAIL no $2 method with a self parameter in $1 (symbol pattern out of date?)"
    return
  elif grep -q '^; unoptimized$' "$1"; then
    echo "FAIL no $2 method's self is noundef in $1: rustc did not optimize, so it marked nothing noalias (an opt-level in RUSTFLAGS or a config's rustflags?)"
    return
  elif [ "$m" -eq 0 ]; then
    echo "FAIL no $2 method with a &mut self in $1, only &self or self by value (symbol pattern out of date?)"
  elif [ -n "$f" ]; then
    echo "FAIL $(printf '%s\n' "$f" | wc -l) of $n $2 methods take a noalias &mut self:"
    printf '%s\n' "$f"
  else
    echo "ok, $n $2 methods take self, no &mut self is noalias"
  fi
  [ -z "$r" ] || {
    echo "$(printf '%s\n' "$r" | wc -l) take a noalias readonly &self (allowed):"
    printf '%s\n' "$r"
  }
  [ -z "$v" ] || {
    echo "$(printf '%s\n' "$v" | wc -l) take self by value (allowed):"
    printf '%s\n' "$v"
  }
}

# fixtures DIR: writes the saved `define` lines, challoc's
# scripts/testdata/heap-noalias, as DIR/red.ll, clean.ll, none.ll and
# ro-only.ll. They are from challoc's release IR (optimized, and as
# rustc emits it before LLVM passes) and made up where marked, for the
# type `heap::Heap` in the crate `challoc`. The kit adds five made-up
# lines: a `Heap` below heap, one at the crate root and one inside a
# function named heap (none of them the type), an impl in a module that
# is not below heap (the type), and a `self` by value. Each `define` is
# preceded by what the scan must make of it: `flagged` (a method whose
# self is noalias and not readonly, a `&mut`), `readonly` (a noalias
# readonly self, a `&self`: listed, not failed), `byvalue` (a
# dead_on_return self, passed by value: listed, not failed), `ok` (a
# method whose self is not noalias), `skipped` (a method with no self
# parameter) or `unmatched` (not an inherent method of the type).
fixtures() {
  # Lines more than one fixture has.
  flush_into='; Made up: self after a noalias parameter.
; want: ok
define internal fastcc void @_RNvMNtNtCsiUoM9lG5jwN_7challoc4heap5pathsNtB4_4Heap10flush_into(ptr noalias nofree noundef nonnull align 8 dereferenceable(16) %out, ptr noundef nonnull align 64 %self) unnamed_addr #10 {'
  stats='; want: readonly
define internal { i64, i64 } @_RNvMsj_NtCseBiCXK8p50Z_7challoc4heapNtB5_4Heap5stats(ptr noalias nofree noundef readonly align 64 captures(address, read_provenance) dereferenceable(3456) %self) unnamed_addr #1 !dbg !33784 {'
  into='; Made up: a self passed by value, which writes its copy.
; want: byvalue
define internal fastcc i64 @_RNvMNtNtCsiUoM9lG5jwN_7challoc4heap5flushNtB4_4Heap10into_stats(ptr dead_on_return noalias nofree noundef align 64 captures(none) dereferenceable(3456) %self) unnamed_addr #10 {'
  capacity='; Made up: no self.
; want: skipped
define internal fastcc noundef i64 @_RNvMNtNtCsiUoM9lG5jwN_7challoc4heap5flushNtB4_4Heap8capacity(i64 noundef range(i64 0, 1024) %class) unnamed_addr #10 {'
  others='; Made up: another type.
; want: unmatched
define internal fastcc void @_RNvMNtCsiUoM9lG5jwN_7challoc5arenaNtB2_5Arena4init(ptr noalias nofree noundef nonnull align 128 dereferenceable(64) %self) unnamed_addr #10 {
; Made up: a type of the same name in a module below heap.
; want: unmatched
define internal void @_RNvMNtNtCsiUoM9lG5jwN_7challoc4heap3subNtB2_4Heap4push(ptr noundef nonnull align 8 %self) unnamed_addr #10 {
; Made up: a type of the same name at the crate root.
; want: unmatched
define internal void @_RNvMNtNtCsiUoM9lG5jwN_7challoc4heap5flushNtB6_4Heap4push(ptr noundef nonnull align 8 %self) unnamed_addr #10 {
; Made up: a type of the same name inside a function named heap.
; want: unmatched
define internal void @_RNvMNvCsiUoM9lG5jwN_7challoc4heapNtB2_4Heap4push(ptr noundef nonnull align 8 %self) unnamed_addr #10 {
; want: unmatched
define internal fastcc void @_RINvNtNtNtCslSNmEl5Cpmc_4core5slice4sort8unstable7ipnsortOhNCINvMB6_SBT_20sort_unstable_by_keyjNCNvMNtNtCsiUoM9lG5jwN_7challoc4heap5flushNtB1C_4Heap5flushs_0E0EB1E_(ptr noalias nofree noundef nonnull align 8 %v.0, i64 noundef range(i64 21, 5368709118) %v.1, ptr noalias nofree noundef nonnull align 8 dereferenceable(8) %is_less) unnamed_addr #3 personality ptr @rust_eh_personality !dbg !5682 {
; want: unmatched
define internal void @_RINvMNtCslSNmEl5Cpmc_4core5sliceSOh20sort_unstable_by_keyjNCNvMNtNtCseBiCXK8p50Z_7challoc4heap5flushNtB11_4Heap5flushs_0EB13_(ptr noalias nofree noundef nonnull align 8 %self.0, i64 noundef range(i64 0, 1152921504606846976) %self.1, ptr noalias nofree noundef readonly align 8 captures(address, read_provenance) dereferenceable(8) %0) unnamed_addr #0 personality ptr @rust_eh_personality !dbg !8 {'
  cat >"$1/red.ll" <<EOF
; red.ll: must fail, flagging exactly the \`flagged\` methods.
; want: flagged
define internal fastcc noundef range(i32 0, -2147483648) i32 @_RNvMNtNtCsiUoM9lG5jwN_7challoc4heap5flushNtB4_4Heap6refill(ptr noalias nofree noundef nonnull align 64 dereferenceable(3456) %self, ptr noundef nonnull align 128 %shared, ptr noundef nonnull align 128 %arena, ptr noundef nonnull captures(address, read_provenance) %cache, i64 noundef range(i64 0, 1024) %class) unnamed_addr #0 personality ptr @rust_eh_personality !dbg !11556 {
; want: flagged
define internal noundef i32 @_RNvMNtNtCseBiCXK8p50Z_7challoc4heap5flushNtB4_4Heap6refill(ptr noalias nofree noundef align 64 dereferenceable(3456) %self, ptr noundef nonnull align 128 %shared, ptr noundef nonnull align 128 %arena, ptr noundef %cache, i64 noundef %class) unnamed_addr #1 !dbg !16838 {
; Made up: a noalias self whose attributes carry a comma.
; want: flagged
define internal fastcc { ptr, i1 } @_RNvMNtNtCsiUoM9lG5jwN_7challoc4heap5pathsNtB4_4Heap10alloc_huge(ptr noalias noundef nonnull align 64 captures(address, read_provenance) dereferenceable(3456) %self, i64 noundef %size) unnamed_addr #10 {
; Made up: an impl in a module that is not below heap, whose type path
; is not a backref to the impl's own path.
; want: flagged
define internal fastcc void @_RNvMNtCsiUoM9lG5jwN_7challoc5arenaNtNtB4_4heap4Heap5reset(ptr noalias noundef nonnull align 64 %self) unnamed_addr #10 {
$flush_into
$stats
$into
$capacity
$others
EOF
  cat >"$1/clean.ll" <<EOF
; clean.ll: must pass.
; want: ok
define internal fastcc noundef range(i32 0, -2147483648) i32 @_RNvMNtNtCsiUoM9lG5jwN_7challoc4heap5flushNtB4_4Heap6refill(ptr noundef nonnull align 64 %self, ptr noundef nonnull align 128 %shared, ptr noundef nonnull align 128 %arena, ptr noundef nonnull captures(address, read_provenance) %cache, i64 noundef range(i64 0, 1024) %class) unnamed_addr #0 personality ptr @rust_eh_personality !dbg !11556 {
; want: ok
define internal noundef i32 @_RNvMNtNtCseBiCXK8p50Z_7challoc4heap5flushNtB4_4Heap6refill(ptr noundef nonnull align 64 %self, ptr noundef nonnull align 128 %shared, ptr noundef nonnull align 128 %arena, ptr noundef %cache, i64 noundef %class) unnamed_addr #1 !dbg !16838 {
$flush_into
$stats
$into
$capacity
$others
EOF
  cat >"$1/none.ll" <<EOF
; none.ll: no inherent Heap method, so it must fail.
$others
EOF
  cat >"$1/ro-only.ll" <<EOF
; ro-only.ll: only a \`&self\` receiver, so it must fail.
$stats
EOF
  cat >"$1/opt0.ll" <<'EOF'
; opt0.ll: made up, as rustc emits the methods at opt-level 0, with no
; parameter attributes; it must fail, though nothing is noalias.
; unoptimized
; want: ok
define internal i32 @_RNvMNtNtCsiUoM9lG5jwN_7challoc4heap5flushNtB4_4Heap6refill(ptr align 64 %self, ptr align 128 %shared) unnamed_addr #1 {
; want: ok
define internal { i64, i64 } @_RNvMsj_NtCseBiCXK8p50Z_7challoc4heapNtB5_4Heap5stats(ptr align 64 %self) unnamed_addr #1 {
EOF
}

# self_test: scan each fixture; its verdict, and whether it fails, must
# be the wanted ones.
self_test() {
  t=$work/fixtures
  mkdir -p "$t"
  fixtures "$t"
  pat=$(pattern challoc heap::Heap)
  st=0
  for f in red.ll:fail clean.ll:pass none.ll:fail ro-only.ll:fail opt0.ll:fail; do
    ll=$t/${f%:*}
    if got=$(scan "$ll" "$work" Heap "$pat"); then res=pass; else res=fail; fi
    exp=$(want "$ll" Heap)
    if [ "$res" = "${f#*:}" ] && [ "$got" = "$exp" ]; then
      echo "self-test: ok   ${f%:*} (${f#*:}): $(printf '%s\n' "$got" | head -1)"
    else
      printf 'self-test: FAIL %s: got = (%s)\n%s\nwant = (%s)\n%s\n' "${f%:*}" "$res" "$got" "${f#*:}" "$exp"
      st=1
    fi
  done
  return $st
}

# The settings, checked before anything runs.
ident='^[A-Za-z][A-Za-z0-9_]*$'
if [ -z "$self_test_only" ]; then
  crate=$(printf '%s' "${NOALIAS_CRATE-}" | tr - _)
  [ -n "$crate" ] || refuse "NOALIAS_CRATE is empty: the port names its library's crate"
  printf '%s\n' "$crate" | grep -qE "$ident" || refuse "NOALIAS_CRATE is $crate, not an identifier that starts with a letter"
  ty=${NOALIAS_TYPE-}
  [ -n "$ty" ] || refuse "NOALIAS_TYPE is empty: the port names the type, with its module path (heap::Heap)"
  printf '%s\n' "$ty" | tr -s : '\n' | grep -vqE "$ident" && refuse "NOALIAS_TYPE is $ty, not a module path and a name, each an identifier that starts with a letter"
  case $ty in
    :* | *: | *:::* | *[!:]:[!:]*) refuse "NOALIAS_TYPE is $ty, not a module path and a name joined by ::" ;;
  esac
  builds=${NOALIAS_BUILDS:-default}
  for b in $builds; do
    printf '%s\n' "$b" | grep -qE '^[A-Za-z0-9_]+$' || refuse "NOALIAS_BUILDS names $b, not a build name ([A-Za-z0-9_])"
  done
  [ -d "$root" ] || refuse "no port root $root"
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
rc=0
self_test || rc=1
[ -z "$self_test_only" ] || exit $rc

cd "$root"
tname=${ty##*::}
pat=$(pattern "$crate" "$ty")
base=${NOALIAS_TARGET_DIR:-target}
# libdir DIR: DIR may hold the library's outputs: a deps directory (a
# cargo before 1.100), or, from cargo 1.100 on, the out directory of a
# unit of the library (build/PACKAGE/HASH/out, whose
# HASH/fingerprint has a lib-CRATE file), not that of a build script or
# its run.
libdir() {
  [ -d "$1" ] || return 1
  case $1 in
    */deps) return 0 ;;
  esac
  [ -e "${1%/out}/fingerprint/lib-$crate" ]
}
for b in $builds; do
  flags=
  eval "flags=\${NOALIAS_FLAGS_$b-}"
  target=$base/noalias-$b
  # The library's outputs: in release/deps, or TRIPLE/release/deps when
  # the build names a target (--target, CARGO_BUILD_TARGET, a [build]
  # target), with a cargo before 1.100; from cargo 1.100 on (its build
  # directory layout), in release/build/PACKAGE/HASH/out (or under
  # TRIPLE/), one HASH directory per unit. Of those, only the library's
  # units hold its outputs: the ones whose fingerprint directory has a
  # lib-CRATE file; another HASH's out is a build script's OUT_DIR, whose
  # files stay.
  # Those of an earlier run go first, its IR with them: cargo then
  # compiles the library again rather than take it as fresh (it does not
  # know the IR is an output, and would write none), and a codegen unit
  # this build does not write is not scanned. Its dependencies stay
  # built. cargo names them CRATE-HASH, or CRATE alone for a library
  # that builds a cdylib or dylib.
  for deps in "$target/release/deps" "$target"/*/release/deps "$target/release/build"/*/*/out "$target"/*/release/build/*/*/out; do
    libdir "$deps" || continue
    rm -f "$deps/$crate".* "$deps/$crate"-* "$deps/lib$crate".* "$deps/lib$crate"-*
  done
  # -C opt-level=3, after the profile's: rustc marks parameters noalias
  # and readonly only when it optimizes, so with opt-level 0 (a profile,
  # or CARGO_PROFILE_RELEASE_OPT_LEVEL) no &mut self would be noalias,
  # and the run would pass on IR that shows nothing.
  # shellcheck disable=SC2086 # the flags are word lists
  CARGO_PROFILE_RELEASE_LTO=false CARGO_TARGET_DIR=$target cargo rustc --release --locked --lib -q ${NOALIAS_CARGO_FLAGS-} $flags -- --emit=llvm-ir -C no-prepopulate-passes -C opt-level=3
  # The IR, for each target the build names (release, or
  # TRIPLE/release), in its deps directory or its library's out
  # directories: CRATE[-HASH].ll, or with several codegen units one
  # CRATE[-HASH].*.rcgu.ll each, chosen for each target on its own,
  # since two targets may differ in codegen units. Only this build's are
  # left.
  # Each target is scanned on its own and must pass on its own: its own
  # &mut self witness, its own noundef, since targets may differ in
  # rustflags and cfg. One line of $work/targets per target with IR: its
  # name (host, or the triple) and its files.
  : >"$work/targets"
  for rel in "$target/release" "$target"/*/release; do
    whole=
    units=
    for deps in "$rel/deps" "$rel/build"/*/*/out; do
      libdir "$deps" || continue
      for f in "$deps/$crate".ll "$deps/$crate"-*.ll "$deps/$crate".*.rcgu.ll; do
        [ -s "$f" ] || continue
        case ${f##*/} in
          *.rcgu.ll) units=$units${units:+ }$f ;;
          *) whole=$whole${whole:+ }$f ;;
        esac
      done
    done
    f=${whole:-$units}
    t=${rel#"$target"/}
    t=${t%/release}
    [ "$t" != release ] || t=host
    [ -z "$f" ] || printf '%s %s\n' "$t" "$f" >>"$work/targets"
  done
  [ -s "$work/targets" ] || {
    echo "$b: no LLVM IR for $crate in $target/release/deps or $target/*/release/deps, or in the library's $target/release/build/*/*/out or $target/*/release/build/*/*/out (is NOALIAS_CRATE the library's crate name?)" >&2
    exit 1
  }
  label=$b
  while read -r t ll; do
    [ "$(wc -l <"$work/targets")" -eq 1 ] || label="$b ($t)"
    if v=$(scan "$ll" "$work" "$tname" "$pat"); then echo "$label: $v"; else
      echo "$label: $v"
      rc=1
    fi
  done <"$work/targets"
done
exit $rc
