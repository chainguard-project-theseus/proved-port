#!/bin/sh
# kit/codegen-test.sh: tests of kit/make/codegen.sh (the codegen-test
# check). Run it from the repository root, in a git checkout. It needs
# cargo, go (it builds kit/perf/codegen/fncmp), binutils and git, and no
# network.
#   - notices on challoc's saved compare reports
#     (kit/perf/codegen/testdata/notices): report.txt must give want.txt
#     exactly, and report-big.txt (more than 40 changed functions, and an
#     fncmp failure) want-big.txt;
#   - base with a stand-in capture and compare (CODEGEN_DIFF) exiting 0
#     and then 2: base exits the same and ends with want.txt's notices;
#   - settings: no CODEGEN_LIB, a CODEGEN_LIB with a directory, a build
#     name that is not [A-Za-z0-9_], and base with no base revision are
#     refused (exit 2) before any build;
#   - a fixture cdylib with no dependencies, in a git repository of its
#     own, built twice (builds `a`, and `b` with a feature), through base
#     with fncmp built here:
#       - with no change, each build's .text is IDENTICAL, exit 0;
#       - with a static added before a table one function reads, the
#         .text differs but no function changed (some reloc-only: one
#         on x86_64, more on aarch64, where other addresses move too;
#         `changed 0`), exit 0;
#       - with a function's body changed, it is `changed 1` in both builds,
#         its diff is printed, the notices name it, exit 2;
#       - with the feature's function changed, only build b changes;
#       - with a missing comparer (CODEGEN_FNCMP), exit 1 before any
#         build, on an unchanged tree;
#       - with a comparer that writes a `changed 0` report and exits 1,
#         `fncmp failed (exit 1)` and exit 2;
#       - with a base revision git cannot resolve, exit 1 before the
#         capture;
#       - with a capture and a compare run as two commands (make
#         codegen-capture, then make codegen-compare), the same verdict;
#   - the paths that fail closed, each exit 1 with no verdict: an objcopy
#     or a sha256sum that fails (stubs on PATH), a directory with no
#     capture, a capture that lacks a build CODEGEN_BUILDS names, a
#     CODEGEN_LIB cargo does not write, and a build that fails (cargo's
#     output, and no library left from the last good build);
#   - a build with --target: the library under TRIPLE/release is
#     compared, not the one a native capture left in release/.
set -eu

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
root=$(pwd)
script=$root/kit/make/codegen.sh
n=$root/kit/perf/codegen/testdata/notices
[ -f "$script" ] || { echo "FAIL: run it from the repository root (no kit/make/codegen.sh)" >&2; exit 1; }
unset CODEGEN_LIB CODEGEN_BUILDS CODEGEN_CARGO_FLAGS CODEGEN_TARGET_DIR CODEGEN_FNCMP CODEGEN_DIFF \
  CODEGEN_BASE CHAINGUARD_BASE_REVISION CARGO_TARGET_DIR RUSTFLAGS || true

fail() {
  echo "FAIL: $*" >&2
  exit 1
}
ok() { echo "ok   $*"; }

# notices on the saved reports.
for c in "x86_64 report.txt want.txt" "aarch64 report-big.txt want-big.txt"; do
  # shellcheck disable=SC2086 # three words
  set -- $c
  sh "$script" notices "$1" "$n/$2" > "$tmp/got"
  cmp -s "$tmp/got" "$n/$3" || { cat "$tmp/got"; fail "notices $2: not $3"; }
  ok "notices $2"
done

# base with the stand-in: compare's exit status is base's, and the
# report's notices end the output.
arch=$(uname -m)
for want in 0 2; do
  got=0
  CHAINGUARD_BASE_REVISION=HEAD CODEGEN_DIFF=$n/standin.sh STANDIN_RC=$want \
    sh "$script" base > "$tmp/out" 2>&1 || got=$?
  tail -n "$(wc -l < "$n/want.txt")" "$tmp/out" > "$tmp/tail"
  awk -v a="$arch" '{ sub(/ x86_64 /, " " a " "); print }' "$n/want.txt" > "$tmp/want"
  if [ "$got" != "$want" ] || ! cmp -s "$tmp/want" "$tmp/tail"; then
    cat "$tmp/out"
    fail "base with compare exiting $want: exit $got"
  fi
  ok "base with compare exiting $want"
done

# Settings refused before any build.
refused() { # refused WHAT WANT-TEXT ENV... -- ARGS...
  what=$1 text=$2
  shift 2
  got=0
  env "$@" > "$tmp/out" 2>&1 || got=$?
  [ "$got" = 2 ] || { cat "$tmp/out"; fail "$what: exit $got, want 2"; }
  grep -qF "$text" "$tmp/out" || { cat "$tmp/out"; fail "$what: no \"$text\""; }
  ok "$what refused"
}
refused "no CODEGEN_LIB" "CODEGEN_LIB is empty" sh "$script" capture "$tmp/c"
refused "a CODEGEN_LIB with a directory" "a file name, with no directory" \
  CODEGEN_LIB=target/libx.so sh "$script" capture "$tmp/c"
refused "a build name with a dash" "CODEGEN_BUILDS names q-8" \
  CODEGEN_LIB=libx.so CODEGEN_BUILDS="q16 q-8" sh "$script" compare "$tmp/c"
refused "base with no base revision" "no base revision" sh "$script" base
[ ! -e "$tmp/c" ] || fail "a refused run made $tmp/c"

# The fixture: one cdylib, with no dependencies, in its own repository.
(cd "$root/kit/perf/codegen/fncmp" && GOTOOLCHAIN=local GOFLAGS=-mod=mod go build -o "$tmp/fncmp" .)
fx=$tmp/fixture
mkdir -p "$fx/src"
cat > "$fx/Cargo.toml" <<'EOF'
[package]
name = "codegen-fx"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[features]
wide = []
EOF
# lib MIX SUM WIDE [pad]: the library's source, with the bodies of
# fx_mix, fx_sum, and fx_wide, which only build b has; fx_pick reads a
# table in .rodata, and with `pad` an exported static goes before it,
# which moves the table and so changes fx_pick's PC-relative offset and
# nothing else.
lib() {
  pad=
  if [ "${4-}" = pad ]; then pad='#[no_mangle]
pub static FX_PAD: [u64; 32] = [1; 32];'; fi
  cat > "$fx/src/lib.rs" <<EOF
$pad
static TABLE: [u64; 4] = [3, 5, 7, 11];

#[no_mangle]
pub extern "C" fn fx_pick(i: usize) -> u64 { TABLE[i & 3] }

#[no_mangle]
pub extern "C" fn fx_mix(a: u64, b: u64) -> u64 { $1 }

#[no_mangle]
pub extern "C" fn fx_sum(p: *const u64, n: usize) -> u64 { $2 }

#[cfg(feature = "wide")]
#[no_mangle]
pub extern "C" fn fx_wide(a: u64) -> u64 { $3 }
EOF
}
mix='a.rotate_left(7) ^ b.wrapping_mul(0x9e37_79b9_7f4a_7c15)'
sum='let s = unsafe { core::slice::from_raw_parts(p, n) }; s.iter().fold(0, |x, y| x.wrapping_add(*y))'
wide='a.wrapping_mul(3)'
lib "$mix" "$sum" "$wide"
g() { git -C "$fx" -c user.name=t -c user.email=t@t -c commit.gpgsign=false "$@"; }
g init -q
printf 'target\n' > "$fx/.gitignore"
g add .
g commit -q -m base
export CODEGEN_LIB=libcodegen_fx.so CODEGEN_BUILDS="a b" CODEGEN_FLAGS_b="--features wide" \
  CODEGEN_FNCMP="$tmp/fncmp" CODEGEN_BASE=HEAD CODEGEN_TARGET_DIR="$tmp/target"

# run NAME WANT [VAR=VALUE...]: base over the fixture, with the
# assignments, which must exit WANT; the output is $tmp/NAME.out.
run() {
  name=$1 want=$2
  shift 2
  got=0
  (cd "$fx" && env "$@" sh "$script" base) > "$tmp/$name.out" 2>&1 || got=$?
  [ "$got" = "$want" ] || { cat "$tmp/$name.out"; fail "$name: exit $got, want $want"; }
}
has() { # has NAME TEXT: the output of run NAME has the line TEXT
  grep -qxF "$2" "$tmp/$1.out" || { cat "$tmp/$1.out"; fail "$1: no line \"$2\""; }
}
hasnt() { # hasnt NAME PATTERN: no line of the output matches PATTERN
  if grep -q "$2" "$tmp/$1.out"; then cat "$tmp/$1.out"; fail "$1: a line matches $2"; fi
}

run same 0
for b in a b; do
  has same "::notice title=codegen $arch $b::.text identical: byte-identical machine code"
done
hasnt same 'differs$'
ok "no change: .text identical in both builds"

lib "$mix" "$sum" "$wide" pad
run moved 0
grep -c '^   \.text hash .* differs$' "$tmp/moved.out" | grep -qx 2 || { cat "$tmp/moved.out"; fail "moved: the .text did not change in both builds, so this tests nothing"; }
grep -c '^   functions old .*: .*reloc-only [1-9][0-9]*, .*, changed 0$' "$tmp/moved.out" | grep -qx 2 || { cat "$tmp/moved.out"; fail "moved: not reloc-only and changed 0 in both builds"; }
grep -c '^::notice title=codegen .*::functions old .*, changed 0$' "$tmp/moved.out" | grep -qx 2 || { cat "$tmp/moved.out"; fail "moved: no changed 0 notice for both builds"; }
hasnt moved 'changed [1-9]'
ok "a table moved in .rodata: .text differs, reloc-only, changed 0"

lib "$mix" 'unsafe { *p.add(n / 2) }' "$wide"
run changed 2
grep -c '^   functions old .*, changed 1$' "$tmp/changed.out" | grep -qx 2 || { cat "$tmp/changed.out"; fail "changed: not changed 1 in both builds"; }
grep -q '^   ### changed: fx_sum ' "$tmp/changed.out" || { cat "$tmp/changed.out"; fail "changed: fx_sum is not named"; }
grep -q '^   --- diff of fx_sum, immediates masked$' "$tmp/changed.out" || { cat "$tmp/changed.out"; fail "changed: no diff of fx_sum"; }
for b in a b; do
  grep -q "^::notice title=codegen $arch $b::changed (1): fx_sum (" "$tmp/changed.out" || { cat "$tmp/changed.out"; fail "changed: no notice for build $b"; }
done
ok "a function changed: changed 1 in both builds, exit 2"

lib "$mix" "$sum" 'a.wrapping_mul(5).rotate_right(3)'
run feature 2
has feature "::notice title=codegen $arch a::.text identical: byte-identical machine code"
grep -q "^::notice title=codegen $arch b::changed (1): fx_wide (" "$tmp/feature.out" || { cat "$tmp/feature.out"; fail "feature: build b names no fx_wide"; }
ok "the feature's function changed: only build b"

# A missing comparer fails before any build, on a tree whose machine
# code did not change too, not only on the first run that needs it.
lib "$mix" "$sum" "$wide"
run nofncmp 1 CODEGEN_FNCMP="$tmp/no-such-fncmp"
grep -qF "the comparer $tmp/no-such-fncmp (CODEGEN_FNCMP) is not a command" "$tmp/nofncmp.out" || { cat "$tmp/nofncmp.out"; fail "nofncmp: no message"; }
if grep -q '^captured at \|^== \|^::notice' "$tmp/nofncmp.out"; then cat "$tmp/nofncmp.out"; fail "nofncmp: it built or gave a verdict first"; fi
ok "a missing comparer: exit 1 before any build"

# A comparer that fails after writing a report that says `changed 0`
# (a failed write, say): the run says so and exits 2.
mkdir -p "$tmp/badcmp"
printf '#!/bin/sh\necho "functions old 1 new 1: exact 1, reloc-only 0, padding-only 0, erratum-stub 0, changed 0"\nexit 1\n' > "$tmp/badcmp/fncmp"
chmod +x "$tmp/badcmp/fncmp"
lib "$mix" 'unsafe { *p.add(n / 2) }' "$wide"
run badcmp 2 CODEGEN_FNCMP="$tmp/badcmp/fncmp"
grep -c '^   fncmp failed (exit 1)$' "$tmp/badcmp.out" | grep -qx 2 || { cat "$tmp/badcmp.out"; fail "badcmp: no fncmp failure line for both builds"; }
grep -q '^::notice title=codegen .*::fncmp failed (exit 1)$' "$tmp/badcmp.out" || { cat "$tmp/badcmp.out"; fail "badcmp: no notice of the failure"; }
ok "a comparer that fails after a changed 0 report: exit 2"

# A base revision git cannot resolve fails before the capture, with no
# verdict.
run nobase 1 CODEGEN_BASE=no-such-rev
grep -qF "the base revision no-such-rev is not a commit here" "$tmp/nobase.out" || { cat "$tmp/nobase.out"; fail "nobase: no message"; }
if grep -q '^captured at \|^::notice' "$tmp/nobase.out"; then cat "$tmp/nobase.out"; fail "nobase: it captured or gave a verdict"; fi
ok "a base revision that does not resolve: exit 1 before the capture"

# capture at the base, then compare the change, as two commands.
g checkout -q HEAD -- src
(cd "$fx" && sh "$script" capture "$tmp/two") > "$tmp/capture.out" 2>&1 || { cat "$tmp/capture.out"; fail "capture failed"; }
grep -q '^captured at [0-9a-f]\{40\}$' "$tmp/capture.out" || { cat "$tmp/capture.out"; fail "capture: no commit"; }
lib "$mix" 'unsafe { *p.add(n / 2) }' "$wide"
got=0
(cd "$fx" && sh "$script" compare "$tmp/two") > "$tmp/two.out" 2>&1 || got=$?
[ "$got" = 2 ] || { cat "$tmp/two.out"; fail "two commands: exit $got, want 2"; }
grep -q '^baseline [0-9a-f]\{40\}, tree [0-9a-f]\{40\} + uncommitted changes$' "$tmp/two.out" || { cat "$tmp/two.out"; fail "two commands: no baseline line"; }
grep -c '^   functions old .*, changed 1$' "$tmp/two.out" | grep -qx 2 || { cat "$tmp/two.out"; fail "two commands: not changed 1 in both builds"; }
ok "capture, then compare: the same verdict"

# The paths that must fail closed, each with exit 1 and its message.
# fails NAME TEXT MODE DIR [VAR=VALUE...]: codegen.sh MODE DIR in the
# fixture, with the assignments, exits 1 and prints TEXT.
fails() {
  what=$1 text=$2 mode=$3 dir=$4
  shift 4
  got=0
  (cd "$fx" && env "$@" sh "$script" "$mode" "$dir") > "$tmp/fail.out" 2>&1 || got=$?
  [ "$got" = 1 ] || { cat "$tmp/fail.out"; fail "$what: exit $got, want 1"; }
  grep -qF "$text" "$tmp/fail.out" || { cat "$tmp/fail.out"; fail "$what: no \"$text\""; }
  if grep -q 'IDENTICAL' "$tmp/fail.out"; then cat "$tmp/fail.out"; fail "$what: a verdict was printed"; fi
  ok "$what: exit 1"
}
g checkout -q HEAD -- src
(cd "$fx" && sh "$script" capture "$tmp/good") > "$tmp/capture.out" 2>&1 || { cat "$tmp/capture.out"; fail "capture failed"; }
# A tool that fails gives no hash, rather than an empty one on both
# sides, which would compare equal.
mkdir -p "$tmp/stub-objcopy" "$tmp/stub-sha"
printf '#!/bin/sh\nexit 1\n' > "$tmp/stub-objcopy/objcopy"
printf '#!/bin/sh\nexit 1\n' > "$tmp/stub-sha/sha256sum"
chmod +x "$tmp/stub-objcopy/objcopy" "$tmp/stub-sha/sha256sum"
fails "capture with an objcopy that fails" "objcopy could not copy" capture "$tmp/c-objcopy" PATH="$tmp/stub-objcopy:$PATH"
fails "compare with an objcopy that fails" "objcopy could not copy" compare "$tmp/good" PATH="$tmp/stub-objcopy:$PATH"
fails "compare with a sha256sum that fails" "sha256sum failed" compare "$tmp/good" PATH="$tmp/stub-sha:$PATH"
# A sha256sum that succeeds and prints no digest is refused too.
mkdir -p "$tmp/stub-sha-empty"
printf '#!/bin/sh\nexit 0\n' > "$tmp/stub-sha-empty/sha256sum"
chmod +x "$tmp/stub-sha-empty/sha256sum"
fails "compare with a sha256sum that prints no digest" "gave no digest" compare "$tmp/good" PATH="$tmp/stub-sha-empty:$PATH"
# A build that writes the library in more than one place (several
# --target flags, say) is refused, rather than one picked: a cargo stub
# writes it in release/ and under a triple.
mkdir -p "$tmp/stub-cargo"
# shellcheck disable=SC2016 # the stub expands these when cargo runs
printf '#!/bin/sh\nfor d in "$CARGO_TARGET_DIR/release" "$CARGO_TARGET_DIR/x/release"; do mkdir -p "$d" && : > "$d/$CODEGEN_LIB"; done\n' > "$tmp/stub-cargo/cargo"
chmod +x "$tmp/stub-cargo/cargo"
fails "a build that writes the library twice" "in more than one of" capture "$tmp/c-twice" PATH="$tmp/stub-cargo:$PATH"
# Captures that are not there.
mkdir -p "$tmp/empty"
fails "compare with no capture" "holds no capture (commit.txt)" compare "$tmp/empty"
(cd "$fx" && CODEGEN_BUILDS=a sh "$script" capture "$tmp/only-a") > "$tmp/capture.out" 2>&1 || { cat "$tmp/capture.out"; fail "capture of build a failed"; }
fails "compare of a build the capture lacks" "has no capture of build b" compare "$tmp/only-a"
# A library cargo does not write.
fails "a CODEGEN_LIB cargo does not write" "build a wrote no libnope.so" capture "$tmp/c-nope" CODEGEN_LIB=libnope.so
# A build that fails: cargo's output, and no library left from the last
# good build in its target directory.
printf 'pub fn broken( {\n' > "$fx/src/lib.rs"
fails "a build that fails" "build a failed" compare "$tmp/good"
grep -q 'error' "$tmp/fail.out" || { cat "$tmp/fail.out"; fail "a build that fails: no compiler output"; }
# The build's own failure stops the run, not a later step's.
if grep -q 'wrote no' "$tmp/fail.out"; then cat "$tmp/fail.out"; fail "a build that fails: the run went on past it"; fi
[ ! -e "$tmp/target/codegen-a/release/libcodegen_fx.so" ] || fail "a build that fails: the last good library is still there"
ok "a build that fails leaves no library to compare"
# A target triple: a build with --target writes only TRIPLE/release, so
# the native library an earlier run left in release/ must not be the
# one compared. Captured natively, then compared with --target and a
# changed function: the change is seen.
g checkout -q HEAD -- src
host=$(rustc -vV | sed -n 's/^host: //p')
[ -n "$host" ] || fail "rustc -vV names no host"
(cd "$fx" && CODEGEN_BUILDS=a sh "$script" capture "$tmp/native") > "$tmp/capture.out" 2>&1 || { cat "$tmp/capture.out"; fail "native capture failed"; }
[ -f "$tmp/target/codegen-a/release/libcodegen_fx.so" ] || fail "the native capture left no library in release/, so this tests nothing"
lib "$mix" 'unsafe { *p.add(n / 2) }' "$wide"
got=0
(cd "$fx" && CODEGEN_BUILDS=a CODEGEN_CARGO_FLAGS="--target $host" sh "$script" compare "$tmp/native") > "$tmp/triple.out" 2>&1 || got=$?
[ "$got" = 2 ] || { cat "$tmp/triple.out"; fail "a target triple: exit $got, want 2"; }
grep -q '^   functions old .*, changed 1$' "$tmp/triple.out" || { cat "$tmp/triple.out"; fail "a target triple: the change is not seen"; }
[ ! -e "$tmp/target/codegen-a/release/libcodegen_fx.so" ] || fail "a target triple: the native library is still there"
ok "a target triple: the triple's library is compared, not the native one left in release/"

echo PASS
