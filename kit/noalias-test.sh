#!/bin/sh
# kit/noalias-test.sh: tests of kit/make/noalias.sh with cargo (the
# noalias-test check). A fixture library with no dependencies has a type
# `heap::Heap` with two `&mut self` methods (push writes, peek only
# reads), a `&self` one and one that takes `self` by value and writes
# it, each kept out of line, and a release profile with fat LTO, as
# challoc's has. Then:
#   - with the type `Unpin`, the run fails and names both `&mut self`
#     methods (rustc marks them noalias; after LLVM passes peek would be
#     `noalias readonly` too, so this holds the IR to before them), and
#     lists the `&self` one and the by-value one (`dead_on_return
#     noalias`), which do not fail it;
#   - the same with the release profile at opt-level 0, where rustc
#     would mark nothing noalias but for noalias.sh's -C opt-level=3;
#     with -C opt-level=0 in RUSTFLAGS, which wins, the run fails as
#     unoptimized;
#   - with a stub cargo, two targets in one build are each scanned on
#     their own and must each pass: one whole-crate .ll and one per-unit
#     (one target fails), and one optimized and one not (the unoptimized
#     one fails);
#   - a build with a target triple (--target, IR under TRIPLE/), and its
#     rerun;
#   - with a `PhantomPinned` field (`!Unpin`), the run passes;
#   - two builds, one with a feature that pins the type, each in its own
#     target directory: one verdict per build, and the run fails;
#   - a release profile with several codegen units (one .ll file per
#     unit) gives the same verdicts, and an earlier run's single .ll in
#     the same target directory is not scanned;
#   - a rerun in the same target directory builds the library again and
#     gives the same verdict;
#   - NOALIAS_CARGO_FLAGS reach cargo: from the fixture's parent, with
#     --manifest-path, the same verdict;
#   - a library that builds an rlib only (IR named CRATE-HASH.ll);
#   - a build script that writes CRATE.ll in its OUT_DIR (with cargo
#     1.100's build directory layout, a sibling of the library's out
#     directory): the file is neither scanned nor deleted;
#   - a NOALIAS_TYPE the crate does not have, or one with only `&self`
#     methods, fails, and a NOALIAS_CRATE that is not the library's fails,
#     naming no IR;
#   - a type at the crate root, with its impl there and in a module, and
#     a type of the same name in a module, which is not it.
# Run it from the repository root. It needs cargo and no network.
set -eu

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
script=$(pwd)/kit/make/noalias.sh
[ -f "$script" ] || { echo "FAIL: run it from the repository root (no kit/make/noalias.sh)" >&2; exit 1; }
unset NOALIAS_CRATE NOALIAS_TYPE NOALIAS_BUILDS NOALIAS_CARGO_FLAGS NOALIAS_TARGET_DIR CARGO_TARGET_DIR \
  CARGO_PROFILE_RELEASE_CODEGEN_UNITS RUSTFLAGS || true

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

fx=$tmp/fixture
mkdir -p "$fx/src"
# manifest TYPES: the fixture's Cargo.toml, its library's crate-type
# TYPES (a TOML list's items).
manifest() {
  cat > "$fx/Cargo.toml" <<EOF
[package]
name = "noalias-fx"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = [$1]

[features]
pinned = []

# As challoc's: noalias.sh turns fat LTO off for its builds.
[profile.release]
lto = true
EOF
}
manifest '"rlib", "cdylib"'
# lib PINNED [ro]: the fixture's src/lib.rs, with a PhantomPinned field
# in Heap when PINNED is yes, or under the feature `pinned` when it is
# feature; with ro, Heap has its `&self` method only. Each method is
# #[inline(never)], so rustc's own inliner leaves it a `define`.
lib() {
  case $1 in
    yes) pin='pub _pin: core::marker::PhantomPinned,' init='_pin: core::marker::PhantomPinned,' ;;
    feature) pin='#[cfg(feature = "pinned")] pub _pin: core::marker::PhantomPinned,' \
      init='#[cfg(feature = "pinned")] _pin: core::marker::PhantomPinned,' ;;
    *) pin='' init='' ;;
  esac
  muts='        #[inline(never)]
        pub fn push(&mut self, x: u64) {
            self.slots[self.n % 8] = x;
            self.n += 1;
        }
        #[inline(never)]
        pub fn peek(&mut self) -> u64 {
            self.slots[1]
        }'
  calls='    h.push(x);
    let y = h.peek();'
  bind='let mut h'
  if [ "${2-}" = ro ]; then muts='' calls='    let y = 0;' bind='let h'; fi
  cat > "$fx/src/lib.rs" <<EOF
pub mod heap {
    pub struct Heap {
        pub slots: [u64; 8],
        pub n: usize,
        $pin
    }
    impl Heap {
        pub fn new() -> Self {
            Heap { slots: [0; 8], n: 0, $init }
        }
$muts
        #[inline(never)]
        pub fn first(&self) -> u64 {
            self.slots[0]
        }
        #[inline(never)]
        pub fn into_n(mut self) -> usize {
            self.n += 1;
            self.slots[self.n % 8] = 7;
            self.n + self.slots[3] as usize
        }
    }
}

#[no_mangle]
pub extern "C" fn noalias_fx(x: u64) -> u64 {
    $bind = heap::Heap::new();
$calls
    let f = h.first();
    f + x + y + h.into_n() as u64
}
EOF
}

# run NAME WANT [VAR=VALUE...]: noalias.sh over the fixture with the
# settings, from the fixture (or from DIR, when RUN_DIR=DIR is first);
# it must exit WANT (0 or 1), its output to $tmp/NAME.
run() {
  _name=$1 _want=$2 _dir=$fx
  shift 2
  case ${1-} in
    RUN_DIR=*)
      _dir=${1#RUN_DIR=}
      shift
      ;;
  esac
  if (cd "$_dir" && env NOALIAS_CRATE=noalias-fx NOALIAS_TYPE=heap::Heap "$@" sh "$script") > "$tmp/$_name" 2>&1; then
    _rc=0
  else
    _rc=$?
  fi
  [ "$_rc" -eq "$_want" ] || { cat "$tmp/$_name" >&2; fail "$_name: got = exit $_rc, want = exit $_want"; }
}

# has NAME TEXT: the output of run NAME has a line TEXT.
has() {
  grep -qxF -- "$2" "$tmp/$1" || { cat "$tmp/$1" >&2; fail "$1: no line '$2'"; }
}

# names NAME: the output of run NAME flags push and peek (and nothing
# else), lists first as the one &self and into_n as the one by value.
names() {
  _sym='  _RNvMNtCs[0-9A-Za-z]*_10noalias_fx4heapNtB2_4Heap'
  sed -n '/take a noalias &mut self:/,/^[^ ]/p' "$tmp/$1" | grep '^  ' > "$tmp/$1.flagged" || true
  sed -n '/take a noalias readonly &self/,/^[^ ]/p' "$tmp/$1" | grep '^  ' > "$tmp/$1.ro" || true
  sed -n '/take self by value/,/^[^ ]/p' "$tmp/$1" | grep '^  ' > "$tmp/$1.bv" || true
  grep -qxE "${_sym}4push" "$tmp/$1.flagged" && grep -qxE "${_sym}4peek" "$tmp/$1.flagged" \
    && [ "$(wc -l < "$tmp/$1.flagged")" -eq 2 ] \
    || { cat "$tmp/$1" >&2; fail "$1: got = another flagged list, want = push and peek"; }
  grep -qxE "${_sym}5first" "$tmp/$1.ro" && [ "$(wc -l < "$tmp/$1.ro")" -eq 1 ] \
    || { cat "$tmp/$1" >&2; fail "$1: got = another &self list, want = first alone"; }
  grep -qxE "${_sym}6into_n" "$tmp/$1.bv" && [ "$(wc -l < "$tmp/$1.bv")" -eq 1 ] \
    || { cat "$tmp/$1" >&2; fail "$1: got = another by-value list, want = into_n alone"; }
}

# ir REL NAME: some file NAME (a glob) of the library's IR is under REL
# (TARGET_DIR/release or TARGET_DIR/TRIPLE/release): in REL/deps (a
# cargo before 1.100) or in the library's REL/build/PACKAGE/HASH/out
# (cargo 1.100's build directory layout).
ir() {
  # NAME is left unquoted so that it globs; it holds no blank.
  # shellcheck disable=SC2231
  for _f in "$1"/deps/$2 "$1"/build/noalias-fx/*/out/$2; do
    case $_f in
      */deps/*) ;;
      *) [ -e "${_f%/out/*}/fingerprint/lib-noalias_fx" ] || continue ;;
    esac
    [ ! -e "$_f" ] || return 0
  done
  return 1
}

flagged='default: FAIL 2 of 4 Heap methods take a noalias &mut self:'
clean='default: ok, 4 Heap methods take self, no &mut self is noalias'

lib no
(cd "$fx" && cargo generate-lockfile --offline -q) || fail "cargo generate-lockfile in the fixture"
run unpin 1
has unpin "$flagged"
has unpin "1 take a noalias readonly &self (allowed):"
has unpin "1 take self by value (allowed):"
names unpin
grep -c '^self-test: ok ' "$tmp/unpin" | grep -qx 5 || fail "unpin: the self-test did not run first"
echo "ok an Unpin type's &mut self is noalias, and fails, a &mut self that only reads included; a self by value is not"

run unpin-again 1
has unpin-again "$flagged"
echo "ok a rerun in the same target directory gives the same verdict"

run opt0 1 CARGO_PROFILE_RELEASE_OPT_LEVEL=0
has opt0 "$flagged"
names opt0
echo "ok a release profile at opt-level 0 still shows the noalias &mut self"

# RUSTFLAGS come after `cargo rustc --`'s arguments, so their opt-level
# wins: with nothing noundef, the run fails rather than pass.
run rustflags-opt0 1 RUSTFLAGS='-C opt-level=0'
grep -q "^default: FAIL no Heap method's self is noundef" "$tmp/rustflags-opt0" \
  || { cat "$tmp/rustflags-opt0" >&2; fail "rustflags-opt0: no failure for an unoptimized build"; }
echo "ok an opt-level 0 in RUSTFLAGS fails the run"

# Two targets in one build, written by a stub cargo; each is scanned on
# its own and must pass on its own. STUB=units: one has a whole-crate .ll
# (clean), the other per-unit files (a noalias &mut self). STUB=opt: one
# is optimized and clean, the other unoptimized (no noundef).
mkdir -p "$tmp/stub"
cat > "$tmp/stub/cargo" <<'EOF'
#!/bin/sh
a=$CARGO_TARGET_DIR/aarch64-unknown-linux-gnu/release/deps
b=$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/release/deps
s='define void @_RNvMNtCsabc_10noalias_fx4heapNtB2_4Heap4push'
mkdir -p "$a" "$b"
printf '%s\n' "$s(ptr noundef nonnull align 8 %self) {" > "$a/noalias_fx.ll"
if [ "$STUB" = units ]; then
  printf '%s\n' "$s(ptr noalias noundef align 8 %self) {" > "$b/noalias_fx.noalias_fx.abc-cgu.0.rcgu.ll"
  printf '%s\n' 'define i64 @_RNvMNtCsabc_10noalias_fx4heapNtB2_4Heap5first(ptr noalias noundef readonly align 8 %self) {' \
    > "$b/noalias_fx.noalias_fx.abc-cgu.1.rcgu.ll"
else
  printf '%s\n' "$s(ptr align 8 %self) {" > "$b/noalias_fx.ll"
fi
EOF
chmod +x "$tmp/stub/cargo"
run two-targets 1 PATH="$tmp/stub:$PATH" STUB=units NOALIAS_TARGET_DIR="$tmp/stub-units-target"
has two-targets "default (aarch64-unknown-linux-gnu): ok, 1 Heap methods take self, no &mut self is noalias"
has two-targets "default (x86_64-unknown-linux-gnu): FAIL 1 of 2 Heap methods take a noalias &mut self:"
run two-opt 1 PATH="$tmp/stub:$PATH" STUB=opt NOALIAS_TARGET_DIR="$tmp/stub-opt-target"
has two-opt "default (aarch64-unknown-linux-gnu): ok, 1 Heap methods take self, no &mut self is noalias"
grep -q "^default (x86_64-unknown-linux-gnu): FAIL no Heap method's self is noundef" "$tmp/two-opt" \
  || { cat "$tmp/two-opt" >&2; fail "two-opt: the unoptimized target did not fail"; }
echo "ok two targets in one build are each scanned and must each pass (whole and per-unit IR; optimized and not)"

run cargo-flags 1 RUN_DIR="$tmp" NOALIAS_CARGO_FLAGS='--manifest-path fixture/Cargo.toml'
has cargo-flags "$flagged"
[ -d "$tmp/target/noalias-default" ] || fail "cargo-flags: no target directory under the directory it ran from"
echo "ok NOALIAS_CARGO_FLAGS reach cargo (--manifest-path from the fixture's parent)"

host=$(rustc -vV | sed -n 's/^host: //p')
run triple 1 NOALIAS_CARGO_FLAGS="--target $host"
has triple "$flagged"
ir "$fx/target/noalias-default/$host/release" noalias_fx.ll \
  || fail "triple: no IR under $host/release, so the case tested nothing"
run triple-again 1 NOALIAS_CARGO_FLAGS="--target $host"
has triple-again "$flagged"
echo "ok a build with a target triple ($host), and its rerun"

lib yes
run pinned 0
has pinned "$clean"
echo "ok a PhantomPinned type's &mut self is not noalias, and passes"

lib feature
run builds 1 NOALIAS_BUILDS='plain pin' NOALIAS_FLAGS_pin='--features pinned'
has builds "plain: ${flagged#default: }"
has builds "pin: ${clean#default: }"
[ -d "$fx/target/noalias-plain" ] && [ -d "$fx/target/noalias-pin" ] || fail "builds: not one target directory per build"
echo "ok two builds, each its own verdict and target directory"

run cgus 1 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=4 NOALIAS_BUILDS='plain pin' NOALIAS_FLAGS_pin='--features pinned' \
  NOALIAS_TARGET_DIR="$tmp/cgu"
has cgus "plain: ${flagged#default: }"
has cgus "pin: ${clean#default: }"
if ir "$tmp/cgu/noalias-plain/release" noalias_fx.ll \
  || ! ir "$tmp/cgu/noalias-plain/release" 'noalias_fx.*.rcgu.ll'; then
  fail "cgus: got = one .ll file, want = one for each codegen unit"
fi
echo "ok several codegen units give the same verdicts"

# The builds' target directories still hold one .ll each, for an Unpin
# type in plain. Pinned now, and with several codegen units, plain must
# pass: the earlier run's noalias_fx.ll is gone, not scanned.
lib yes
run stale 0 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=4 NOALIAS_BUILDS='plain pin' NOALIAS_FLAGS_pin='--features pinned'
has stale "plain: ${clean#default: }"
! ir "$fx/target/noalias-plain/release" noalias_fx.ll || fail "stale: an earlier run's noalias_fx.ll is still there"
echo "ok an earlier run's IR is not scanned"

# An rlib alone: cargo names the IR noalias_fx-HASH.ll.
lib no
manifest '"rlib"'
run rlib 1
has rlib "$flagged"
ir "$fx/target/noalias-default/release" 'noalias_fx-*.ll' \
  || fail "rlib: no noalias_fx-HASH.ll, so the case tested nothing"
run rlib-again 1
has rlib-again "$flagged"
echo "ok a library that builds an rlib only (CRATE-HASH.ll), and its rerun"
manifest '"rlib", "cdylib"'

# A build script that writes CRATE.ll, with a noalias &mut self, in its
# OUT_DIR (from cargo 1.100 on, release/build/PACKAGE/HASH/out, as the
# library's outputs are): with the type pinned, the run passes, so that
# file is not scanned, and the file is still there after a rerun, so it
# is not deleted either (cargo, which takes the build script's run as
# fresh, would not write it again).
lib yes
cat > "$fx/build.rs" <<'EOF'
fn main() {
    let out = std::env::var("OUT_DIR").unwrap();
    std::fs::write(
        format!("{out}/noalias_fx.ll"),
        "; written by the fixture's build script\ndefine void @_RNvMNtCsabc_10noalias_fx4heapNtB2_4Heap4push(ptr noalias noundef align 8 %self) {\n",
    )
    .unwrap();
}
EOF
run build-script 0
has build-script "$clean"
run build-script-again 0
has build-script-again "$clean"
grep -rlF "written by the fixture's build script" "$fx/target/noalias-default" > /dev/null \
  || fail "build-script: the build script's noalias_fx.ll in its OUT_DIR is gone"
rm "$fx/build.rs"
lib no
echo "ok a build script's CRATE.ll in its OUT_DIR is neither scanned nor deleted"

run other-type 1 NOALIAS_TYPE=heap::Arena
grep -q '^default: FAIL no Arena method with a self parameter' "$tmp/other-type" \
  || { cat "$tmp/other-type" >&2; fail "other-type: no failure naming Arena"; }
echo "ok a type the crate does not have fails"

run wrong-crate 1 NOALIAS_CRATE=other
grep -q '^default: no LLVM IR for other' "$tmp/wrong-crate" \
  || { cat "$tmp/wrong-crate" >&2; fail "wrong-crate: no line naming the missing IR"; }
echo "ok a crate name that is not the library's fails"

lib no ro
run ro-only 1
grep -q '^default: FAIL no Heap method with a &mut self' "$tmp/ro-only" \
  || { cat "$tmp/ro-only" >&2; fail "ro-only: no failure for a type with only &self"; }
echo "ok a type with only &self methods fails"

# A type at the crate root, its impl at the root and in a module, and a
# pinned type of the same name in a module, whose &mut self would be a
# witness if it were taken for the root's.
cat > "$fx/src/lib.rs" <<'EOF'
pub struct Heap {
    pub slots: [u64; 8],
    pub n: usize,
}
impl Heap {
    #[inline(never)]
    pub fn push(&mut self, x: u64) {
        self.slots[self.n % 8] = x;
        self.n += 1;
    }
}
pub mod more {
    impl crate::Heap {
        #[inline(never)]
        pub fn pop(&mut self) -> u64 {
            self.n -= 1;
            self.slots[self.n % 8]
        }
    }
    pub struct Heap {
        pub n: usize,
        pub _pin: core::marker::PhantomPinned,
    }
    impl Heap {
        #[inline(never)]
        pub fn bump(&mut self) {
            self.n += 1;
        }
    }
}

#[no_mangle]
pub extern "C" fn noalias_fx(x: u64) -> u64 {
    let mut h = Heap { slots: [0; 8], n: 0 };
    h.push(x);
    let mut o = more::Heap { n: 0, _pin: core::marker::PhantomPinned };
    o.bump();
    h.pop() + o.n as u64
}
EOF
run root 1 NOALIAS_TYPE=Heap
has root "default: FAIL 2 of 2 Heap methods take a noalias &mut self:"
run root-more 0 NOALIAS_TYPE=more::Heap
has root-more "default: ok, 1 Heap methods take self, no &mut self is noalias"
echo "ok a type at the crate root, with impls at the root and in a module, and another of its name in a module"
echo PASS
