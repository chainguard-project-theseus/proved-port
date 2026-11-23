#!/bin/sh
# kit/lint-unsafe-test.sh: tests of kit/make/lint-unsafe.sh with cargo
# (the lint-unsafe-test check). A fixture workspace with no dependencies
# has two crates allowed unsafe (capi, which depends on safe, and cmem,
# which safe depends on, so a fence crate need not be a leaf) and two
# that forbid it (safe, with the plain attribute; vpair, with the Verus
# pair, a binary, a test and a build script). The fixture passes; then
# each case plants one change in a fresh copy and checks the verdict and
# the line that names it:
#   - unsafe code in a module, a test, a build script and an example, and
#     an allow(unsafe_code) that lowers the workspace lint in a test,
#     which only the command-line forbid catches;
#   - a root without the forbid, half the Verus pair, a binary's root;
#   - roots from cargo metadata: a member with no lib or bin (benches,
#     tests, an example) read through those, a bin-only member, a lib
#     or bin at a path of its own, and a src/lib.rs or src/main.rs
#     cargo does not build (autolib, autobins = false), which is none;
#   - a misspelt UNSAFE_CRATES name, an empty one, a bad
#     UNSAFE_WORKSPACE_LINTS;
#   - no [workspace.lints.rust], a member that does not inherit it, and
#     both passing with UNSAFE_WORKSPACE_LINTS=no;
#   - a workspace below the root (UNSAFE_MANIFEST), the caller's
#     RUSTC_WORKSPACE_WRAPPER (still run), the caller's RUSTFLAGS
#     allowing unsafe_code (the forbid still wins), UNSAFE_CHECK_FLAGS;
#   - --cap-lints and --force-warn in the caller's flags or an argument
#     file, refused (but cargo 1.100's
#     --force-warn=unused_crate_dependencies), and a member's arguments
#     ending with a bare --force-warn, refused by the wrapper.
# Run it from the repository root. It needs cargo and no network.
set -eu

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
lint=$(pwd)/kit/make/lint-unsafe.sh
[ -f "$lint" ] || { echo "FAIL: run it from the repository root (no kit/make/lint-unsafe.sh)" >&2; exit 1; }
CARGO_TARGET_DIR=$tmp/target
export CARGO_TARGET_DIR
unset RUSTC_WORKSPACE_WRAPPER RUSTFLAGS UNSAFE_CRATES UNSAFE_WORKSPACE_LINTS UNSAFE_MANIFEST UNSAFE_CHECK_FLAGS || true

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# --- the fixture -------------------------------------------------------------
fx=$tmp/fixture
mkdir -p "$fx/crates/safe/src" "$fx/crates/cmem/src" "$fx/crates/capi/src" \
  "$fx/crates/vpair/src/bin" "$fx/crates/vpair/tests" "$fx/crates/vpair/examples"
cat > "$fx/Cargo.toml" <<'EOF'
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.lints.rust]
unsafe_code = "deny"
EOF
# member NAME [DEPENDENCY...]: crates/NAME's manifest.
member() {
  _m=$fx/crates/$1/Cargo.toml
  printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2021"\n\n[lints]\nworkspace = true\n\n[dependencies]\n' "$1" > "$_m"
  shift
  for _d in "$@"; do
    printf '%s = { path = "../%s" }\n' "$_d" "$_d" >> "$_m"
  done
}
member cmem
member safe cmem
member capi safe cmem
member vpair
cat > "$fx/crates/cmem/src/lib.rs" <<'EOF'
#![allow(unsafe_code)]
/// # Safety
/// `p` points to a readable byte.
pub unsafe fn read(p: *const u8) -> u8 {
    // SAFETY: the caller's contract.
    unsafe { *p }
}
EOF
cat > "$fx/crates/safe/src/lib.rs" <<'EOF'
//! The safe crate.
#![forbid(missing_docs, unsafe_code)]
mod inner;
/// One.
pub fn one() -> u8 {
    inner::one()
}
EOF
printf 'pub(crate) fn one() -> u8 {\n    1\n}\n' > "$fx/crates/safe/src/inner.rs"
cat > "$fx/crates/capi/src/lib.rs" <<'EOF'
#![allow(unsafe_code)]
/// # Safety
/// `p` points to a readable byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn capi_read(p: *const u8) -> u8 {
    // SAFETY: the caller's contract.
    unsafe { cmem::read(p) }.wrapping_add(safe::one())
}
EOF
printf '%s\n' '#![cfg_attr(not(verus_keep_ghost), forbid(unsafe_code))]' \
  '#![cfg_attr(verus_keep_ghost, deny(unsafe_code))]' 'pub fn two() -> u8 {' '    2' '}' > "$fx/crates/vpair/src/lib.rs"
printf '#![forbid(unsafe_code)]\nfn main() {\n    println!("{}", vpair::two());\n}\n' > "$fx/crates/vpair/src/bin/tool.rs"
printf '#[test]\nfn two() {\n    assert_eq!(vpair::two(), 2);\n}\n' > "$fx/crates/vpair/tests/t.rs"
printf 'fn main() {}\n' > "$fx/crates/vpair/build.rs"
printf 'fn main() {}\n' > "$fx/crates/vpair/examples/ex.rs"
(cd "$fx" && cargo generate-lockfile --offline -q) || fail "cargo generate-lockfile in the fixture"

# copy: a fresh copy of the fixture as $tmp/case, printed.
copy() {
  rm -rf "$tmp/case"
  cp -a "$fx" "$tmp/case"
  echo "$tmp/case"
}

# run DIR [VAR=VALUE...]: lint-unsafe.sh over DIR with UNSAFE_CRATES="capi
# cmem" and the settings given; its output in $tmp/out, its status in rc.
run() {
  _dir=$1
  shift
  if (cd "$_dir" && env UNSAFE_CRATES="capi cmem" "$@" sh "$lint" .) > "$tmp/out" 2>&1; then rc=0; else rc=$?; fi
}

# verdict WHAT WANT_RC TEXT: the run exited WANT_RC and printed TEXT.
verdict() {
  if [ "$rc" -ne "$2" ] || ! grep -qF -- "$3" "$tmp/out"; then
    cat "$tmp/out" >&2
    fail "$1: got = exit $rc, want = exit $2 with a line holding: $3"
  fi
  echo "ok $1"
}

# --- the fixture passes --------------------------------------------------------
run "$fx"
verdict "the fixture passes" 0 "lint-unsafe: PASS"
verdict "the fixture: unsafe_code is forbidden in safe and vpair, not in capi and cmem" 0 \
  "every target but capi cmem's checks with unsafe_code forbidden (members checked: 2, targets: 6)"
grep -qx 'ok   crates/vpair/src/bin/tool.rs' "$tmp/out" || { cat "$tmp/out" >&2; fail "the fixture: a binary's root is not read"; }
if grep -q 'crates/c\(api\|mem\)/src' "$tmp/out"; then cat "$tmp/out" >&2; fail "the fixture: a fence crate's root is read"; fi
echo "ok the fixture: the roots of safe and vpair, binary too, are read, and no fence crate's"

# --- unsafe code where it is not allowed --------------------------------------
d=$(copy)
printf 'pub(crate) fn one() -> u8 {\n    let x = 1u8;\n    unsafe { *(&raw const x) }\n}\n' > "$d/crates/safe/src/inner.rs"
run "$d"
verdict "an unsafe block in a module of safe fails the cargo check" 1 "a crate other than capi cmem uses unsafe code"

d=$(copy)
printf '#![allow(unsafe_code)]\npub(crate) fn one() -> u8 {\n    1\n}\n' > "$d/crates/safe/src/inner.rs"
run "$d"
verdict "a module's allow(unsafe_code) under safe's forbid fails" 1 "a crate other than capi cmem uses unsafe code"

# The test and build-script crates carry no root attribute: the
# workspace lint denies unsafe code there, an allow lowers that, and only
# the command-line forbid still catches it.
d=$(copy)
printf '#![allow(unsafe_code)]\n#[test]\nfn two() {\n    let x = 2u8;\n    assert_eq!(unsafe { *(&raw const x) }, vpair::two());\n}\n' > "$d/crates/vpair/tests/t.rs"
run "$d"
verdict "an allow(unsafe_code) and an unsafe block in a test fail" 1 "a crate other than capi cmem uses unsafe code"
grep -q 'E0453\|forbid' "$tmp/out" || { cat "$tmp/out" >&2; fail "the test case: rustc did not name the forbid"; }

d=$(copy)
printf '#![allow(unsafe_code)]\nfn main() {\n    let x = 0u8;\n    let _ = unsafe { *(&raw const x) };\n}\n' > "$d/crates/vpair/build.rs"
run "$d"
verdict "an unsafe block in a build script fails" 1 "a crate other than capi cmem uses unsafe code"

d=$(copy)
printf 'fn main() {\n    let x = 0u8;\n    let _ = unsafe { *(&raw const x) };\n}\n' > "$d/crates/vpair/examples/ex.rs"
run "$d"
verdict "an unsafe block in an example fails" 1 "a crate other than capi cmem uses unsafe code"

# The caller's RUSTFLAGS allow unsafe code: the wrapper's forbid comes after.
d=$(copy)
printf '#![allow(unsafe_code)]\nfn main() {\n    let x = 0u8;\n    let _ = unsafe { *(&raw const x) };\n}\n' > "$d/crates/vpair/build.rs"
run "$d" RUSTFLAGS=-Aunsafe_code
verdict "RUSTFLAGS=-Aunsafe_code does not lower the forbid" 1 "a crate other than capi cmem uses unsafe code"

# --cap-lints and --force-warn outrank a command-line forbid, wherever
# the caller's flags come from: each is refused, here with the unsafe
# example planted (which they would otherwise let pass) and, for one,
# on the clean fixture.
d=$(copy)
printf 'fn main() {\n    let x = 0u8;\n    let _ = unsafe { *(&raw const x) };\n}\n' > "$d/crates/vpair/examples/ex.rs"
for f in --cap-lints=allow '--cap-lints warn' '--force-warn unsafe_code' --force-warn=unsafe-code \
  '--force-warn=unused_crate_dependencies --force-warn unsafe_code' \
  '--force-warn unused_crate_dependencies --force-warn=unsafe_code'; do
  run "$d" RUSTFLAGS="$f"
  verdict "RUSTFLAGS='$f' is refused" 1 "a member's rustc gets a flag that outranks -Funsafe_code"
done
# The --force-warn cargo 1.100 and later pass every rustc themselves,
# --force-warn=unused_crate_dependencies, leaves unsafe_code alone: the
# clean fixture passes with it in RUSTFLAGS too, in either form, and the
# planted example still fails as unsafe code, not as a refused flag.
for f in '--force-warn unused_crate_dependencies' --force-warn=unused-crate-dependencies; do
  run "$fx" RUSTFLAGS="$f"
  verdict "RUSTFLAGS='$f' on the clean fixture passes" 0 "lint-unsafe: PASS"
  run "$d" RUSTFLAGS="$f"
  verdict "RUSTFLAGS='$f' does not lower the forbid" 1 "a crate other than capi cmem uses unsafe code"
done
# CARGO_ENCODED_RUSTFLAGS separates flags with 0x1f.
run "$d" CARGO_ENCODED_RUSTFLAGS="$(printf -- '--cap-lints\037allow')"
verdict "CARGO_ENCODED_RUSTFLAGS with --cap-lints allow is refused" 1 "a member's rustc gets a flag that outranks -Funsafe_code"
mkdir -p "$d/.cargo"
printf '[build]\nrustflags = ["--cap-lints", "allow"]\n' > "$d/.cargo/config.toml"
run "$d"
verdict "a cargo configuration's --cap-lints is refused" 1 "a member's rustc gets a flag that outranks -Funsafe_code"
cp -a "$d/.cargo" "$tmp/cargo-config"
d=$(copy)
cp -a "$tmp/cargo-config" "$d/.cargo"
run "$d"
verdict "a cargo configuration's --cap-lints is refused with no unsafe code too" 1 " --cap-lints): take it out"
# An @FILE argument holds arguments rustc reads as its own.
d=$(copy)
printf 'fn main() {\n    let x = 0u8;\n    let _ = unsafe { *(&raw const x) };\n}\n' > "$d/crates/vpair/examples/ex.rs"
printf -- '--edition=2021\n--cap-lints=allow\n' > "$tmp/flags"
run "$d" RUSTFLAGS="@$tmp/flags"
verdict "an argument file holding --cap-lints=allow is refused" 1 " --cap-lints=allow): take it out"
# rustc drops the CR of a CRLF line, so the check must too.
printf -- '--cap-lints\r\nallow\r\n' > "$tmp/flags"
run "$d" RUSTFLAGS="@$tmp/flags"
verdict "a CRLF argument file holding --cap-lints is refused" 1 " --cap-lints): take it out"
# A --force-warn on one line and its lint on the next.
printf -- '--force-warn\nunsafe_code\n' > "$tmp/flags"
run "$d" RUSTFLAGS="@$tmp/flags"
verdict "an argument file holding --force-warn, then unsafe_code, is refused" 1 " --force-warn unsafe_code): take it out"
# A --force-warn that ends the caller's flags (here an argument file's
# last line) fails the run before any member compiles: cargo's probe of
# the target passes the flags before its own --crate-type, which
# --force-warn takes as its lint, and rustc refuses the rest.
printf -- '--force-warn\n' > "$tmp/flags"
run "$d" RUSTFLAGS="@$tmp/flags"
verdict "an argument file ending with --force-warn fails the run" 1 "does not build (cargo check above)"
# A member's rustc whose arguments still end with a bare --force-warn
# (here added by the caller's RUSTC_WRAPPER, which cargo runs before the
# workspace wrapper, for safe and vpair alone, so the probe never sees
# it) is refused by the wrapper itself, on the clean fixture: rustc
# would take the -Funsafe_code appended after it as --force-warn's lint,
# and no forbid would apply.
cat > "$tmp/trail" << 'EOF'
#!/bin/sh
w=$1
shift
case ${CARGO_PKG_NAME-} in
  safe | vpair) exec "$w" "$@" --force-warn ;;
esac
exec "$w" "$@"
EOF
chmod +x "$tmp/trail"
run "$fx" RUSTC_WRAPPER="$tmp/trail"
verdict "a member's rustc whose arguments end with --force-warn is refused" 1 " --force-warn): take it out"
# A missing one fails rustc itself (cargo's probe of it, before any
# member), which fails the run.
run "$d" RUSTFLAGS="@$tmp/no-such-flags"
verdict "a missing argument file fails the run" 1 "does not build (cargo check above)"

# --- crate roots -----------------------------------------------------------------
d=$(copy)
printf '//! The safe crate.\n#![forbid(missing_docs)]\nmod inner;\n/// One.\npub fn one() -> u8 {\n    inner::one()\n}\n' > "$d/crates/safe/src/lib.rs"
run "$d"
verdict "a root without the forbid fails" 1 "FAIL crates/safe/src/lib.rs lacks #![forbid(unsafe_code)]"

d=$(copy)
printf '%s\n' '#![cfg_attr(not(verus_keep_ghost), forbid(unsafe_code))]' 'pub fn two() -> u8 {' '    2' '}' > "$d/crates/vpair/src/lib.rs"
run "$d"
verdict "half the Verus pair fails" 1 "FAIL crates/vpair/src/lib.rs lacks #![forbid(unsafe_code)]"

d=$(copy)
printf 'fn main() {\n    println!("{}", vpair::two());\n}\n' > "$d/crates/vpair/src/bin/tool.rs"
run "$d"
verdict "a binary's root without the forbid fails" 1 "FAIL crates/vpair/src/bin/tool.rs lacks"

# A member with no lib and no bin target (#186): its test, bench and
# example targets are its roots, each read from cargo metadata.
# bonly D: add crates/bonly to the copy D, with a bench and a test that
# forbid unsafe code, a build script and no lib or bin, and relock D.
bonly() {
  mkdir -p "$1/crates/bonly/benches" "$1/crates/bonly/tests"
  printf '[package]\nname = "bonly"\nversion = "0.1.0"\nedition = "2021"\n\n[lints]\nworkspace = true\n\n[[bench]]\nname = "b"\nharness = false\n' > "$1/crates/bonly/Cargo.toml"
  printf '#![forbid(unsafe_code)]\nfn main() {}\n' > "$1/crates/bonly/benches/b.rs"
  printf '#![forbid(unsafe_code)]\n#[test]\nfn t() {}\n' > "$1/crates/bonly/tests/t.rs"
  # A build script is no lib or bin target.
  printf 'fn main() {}\n' > "$1/crates/bonly/build.rs"
  (cd "$1" && cargo generate-lockfile --offline -q) || fail "cargo generate-lockfile in $1"
}
d=$(copy)
bonly "$d"
run "$d"
verdict "a member of a bench and a test alone passes" 0 "lint-unsafe: PASS"
for r in crates/bonly/benches/b.rs crates/bonly/tests/t.rs; do
  grep -qx "ok   $r" "$tmp/out" || { cat "$tmp/out" >&2; fail "a bench-and-test member: its root $r is not read"; }
done
echo "ok a bench-and-test member: both its roots are read"

d=$(copy)
bonly "$d"
printf '#[test]\nfn t() {}\n' > "$d/crates/bonly/tests/t.rs"
run "$d"
verdict "a bench-and-test member's test root without the forbid fails" 1 "FAIL crates/bonly/tests/t.rs lacks #![forbid(unsafe_code)]"

d=$(copy)
bonly "$d"
rm "$d/crates/bonly/tests/t.rs"
printf '#![forbid(missing_docs)]\nfn main() {}\n' > "$d/crates/bonly/benches/b.rs"
run "$d"
verdict "a bench-only member's root without the forbid fails" 1 "FAIL crates/bonly/benches/b.rs lacks #![forbid(unsafe_code)]"

# A bench at a path of its own is read where cargo metadata puts it.
d=$(copy)
bonly "$d"
rm -r "$d/crates/bonly/benches" "$d/crates/bonly/tests"
mkdir -p "$d/crates/bonly/perf"
printf '#![forbid(unsafe_code)]\nfn main() {}\n' > "$d/crates/bonly/perf/run.rs"
printf 'path = "perf/run.rs"\n' >> "$d/crates/bonly/Cargo.toml"
run "$d"
verdict "a bench-only member's bench at a path of its own passes" 0 "ok   crates/bonly/perf/run.rs"
verdict "a bench-only member's bench at a path of its own: the run passes" 0 "lint-unsafe: PASS"
printf '#![allow(dead_code)]\nfn main() {}\n' > "$d/crates/bonly/perf/run.rs"
run "$d"
verdict "a bench at a path of its own without the forbid fails" 1 "FAIL crates/bonly/perf/run.rs lacks"

# An example alone is a root too.
d=$(copy)
bonly "$d"
rm -r "$d/crates/bonly/benches" "$d/crates/bonly/tests"
printf '[package]\nname = "bonly"\nversion = "0.1.0"\nedition = "2021"\n\n[lints]\nworkspace = true\n' > "$d/crates/bonly/Cargo.toml"
mkdir -p "$d/crates/bonly/examples"
printf 'fn main() {}\n' > "$d/crates/bonly/examples/e.rs"
run "$d"
verdict "an example-only member's root without the forbid fails" 1 "FAIL crates/bonly/examples/e.rs lacks"

# A member whose one lib or bin target is a bin (src/main.rs) is read
# through it, not through its bench and test.
d=$(copy)
bonly "$d"
mkdir -p "$d/crates/bonly/src"
printf 'fn main() {}\n' > "$d/crates/bonly/src/main.rs"
run "$d"
verdict "a bin-only member's src/main.rs without the forbid fails" 1 "FAIL crates/bonly/src/main.rs lacks #![forbid(unsafe_code)]"
printf '#![forbid(unsafe_code)]\nfn main() {}\n' > "$d/crates/bonly/src/main.rs"
run "$d"
verdict "a bin-only member's src/main.rs with the forbid passes" 0 "ok   crates/bonly/src/main.rs"
verdict "a bin-only member with the forbid: the run passes" 0 "lint-unsafe: PASS"
if grep -q 'ok   crates/bonly/\(benches\|tests\)/' "$tmp/out"; then cat "$tmp/out" >&2; fail "a bin-only member: its bench or test was read as a root"; fi
echo "ok a bin-only member: its bench and test are not read as roots"

# A src/lib.rs cargo does not build (autolib = false) is no root: the
# member has no lib target, so its test and bench are its roots.
d=$(copy)
bonly "$d"
mkdir -p "$d/crates/bonly/src"
printf '#![forbid(unsafe_code)]\n' > "$d/crates/bonly/src/lib.rs"
printf '[package]\nname = "bonly"\nversion = "0.1.0"\nedition = "2021"\nautolib = false\n\n[lints]\nworkspace = true\n\n[[bench]]\nname = "b"\nharness = false\n' > "$d/crates/bonly/Cargo.toml"
run "$d"
verdict "an autolib = false member is read through its test and bench" 0 "ok   crates/bonly/tests/t.rs"
if grep -q 'crates/bonly/src/lib.rs' "$tmp/out"; then cat "$tmp/out" >&2; fail "autolib = false: the unbuilt src/lib.rs was read as a root"; fi
echo "ok autolib = false: the unbuilt src/lib.rs is not read"
printf '#[test]\nfn t() {}\n' > "$d/crates/bonly/tests/t.rs"
run "$d"
verdict "an autolib = false member's test root without the forbid fails" 1 "FAIL crates/bonly/tests/t.rs lacks"

# A member's lib and bin roots are where cargo metadata puts them: a
# lib at a path of its own is read, and its bench and test are not.
d=$(copy)
bonly "$d"
printf '#![forbid(unsafe_code)]\n' > "$d/crates/bonly/lib.rs"
printf '\n[lib]\npath = "lib.rs"\n' >> "$d/crates/bonly/Cargo.toml"
run "$d"
verdict "a lib at a path of its own is read" 0 "ok   crates/bonly/lib.rs"
verdict "a lib at a path of its own: the run passes" 0 "lint-unsafe: PASS"
if grep -q 'ok   crates/bonly/\(benches\|tests\)/' "$tmp/out"; then cat "$tmp/out" >&2; fail "a member with a lib: its bench or test was read as a root"; fi
echo "ok a member with a lib: its bench and test are not read as roots"
printf 'pub fn f() {}\n' > "$d/crates/bonly/lib.rs"
run "$d"
verdict "a lib at a path of its own without the forbid fails" 1 "FAIL crates/bonly/lib.rs lacks"
# A bin at a path of its own beside a lib at src/lib.rs: both are read.
d=$(copy)
mkdir -p "$d/crates/vpair/cli"
printf 'fn main() {}\n' > "$d/crates/vpair/cli/main.rs"
printf '\n[[bin]]\nname = "cli"\npath = "cli/main.rs"\n' >> "$d/crates/vpair/Cargo.toml"
run "$d"
verdict "a bin at a path of its own beside a default lib, without the forbid, fails" 1 "FAIL crates/vpair/cli/main.rs lacks"
grep -qx 'ok   crates/vpair/src/lib.rs' "$tmp/out" || { cat "$tmp/out" >&2; fail "a bin at a path of its own: the lib beside it is not read"; }
echo "ok a bin at a path of its own: the lib beside it is read too"
# And a src/main.rs cargo does not build (autobins = false) beside a
# lib is not read.
d=$(copy)
printf 'fn main() {}\n' > "$d/crates/vpair/src/main.rs"
awk '{ print } /^edition = / { print "autobins = false" }' "$d/crates/vpair/Cargo.toml" > "$tmp/m" && cp "$tmp/m" "$d/crates/vpair/Cargo.toml"
run "$d"
verdict "an unbuilt src/main.rs (autobins = false) is no root" 0 "lint-unsafe: PASS"
if grep -q 'crates/vpair/src/main.rs' "$tmp/out"; then cat "$tmp/out" >&2; fail "autobins = false: the unbuilt src/main.rs was read"; fi
echo "ok autobins = false: the unbuilt src/main.rs is not read"

# --- settings --------------------------------------------------------------------
run "$fx" UNSAFE_CRATES="capi cmme"
verdict "a misspelt UNSAFE_CRATES name fails" 1 "FAIL cmme (UNSAFE_CRATES) is not a workspace member"
run "$fx" UNSAFE_CRATES=
verdict "an empty UNSAFE_CRATES refuses the run" 2 "UNSAFE_CRATES is empty"
run "$fx" UNSAFE_WORKSPACE_LINTS=maybe
verdict "UNSAFE_WORKSPACE_LINTS=maybe refuses the run" 2 "UNSAFE_WORKSPACE_LINTS is maybe: yes or no"

# --- workspace lints ---------------------------------------------------------------
d=$(copy)
printf '[workspace]\nmembers = ["crates/*"]\nresolver = "2"\n' > "$d/Cargo.toml"
for c in cmem safe capi vpair; do
  awk '!/^\[lints\]$/ && !/^workspace = true$/' "$d/crates/$c/Cargo.toml" > "$tmp/m" && cp "$tmp/m" "$d/crates/$c/Cargo.toml"
done
run "$d"
verdict "no [workspace.lints.rust] fails" 1 "FAIL Cargo.toml lacks unsafe_code = \"deny\""
verdict "no [workspace.lints.rust]: a member that does not inherit it fails too" 1 "FAIL crates/safe/Cargo.toml lacks [lints] workspace = true"
if grep -q 'crates/c\(api\|mem\)/Cargo.toml' "$tmp/out"; then cat "$tmp/out" >&2; fail "no workspace lints: a fence crate's manifest is held to them"; fi
run "$d" UNSAFE_WORKSPACE_LINTS=no
verdict "no workspace lints pass with UNSAFE_WORKSPACE_LINTS=no" 0 "lint-unsafe: PASS"

d=$(copy)
printf '[workspace]\nmembers = ["crates/*"]\nresolver = "2"\n\n[workspace.lints.rust]\nunsafe_code = { level = "forbid", priority = 1 }\n' > "$d/Cargo.toml"
# With forbid, which the fence crates' allow cannot lower, they do not
# inherit the workspace lints (cg-libarchive).
for c in cmem capi; do
  awk '!/^\[lints\]$/ && !/^workspace = true$/' "$d/crates/$c/Cargo.toml" > "$tmp/m" && cp "$tmp/m" "$d/crates/$c/Cargo.toml"
done
run "$d"
verdict "a forbid level in a table, the fence crates not inheriting it, passes" 0 "ok   Cargo.toml denies unsafe_code for the workspace"
verdict "a forbid level in a table: the run passes" 0 "lint-unsafe: PASS"

d=$(copy)
awk '!/^\[lints\]$/ && !/^workspace = true$/' "$d/crates/vpair/Cargo.toml" > "$tmp/m" && cp "$tmp/m" "$d/crates/vpair/Cargo.toml"
run "$d"
verdict "one member that does not inherit the workspace lints fails" 1 "FAIL crates/vpair/Cargo.toml lacks [lints] workspace = true"

# --- where the workspace is, and the caller's wrapper ------------------------
d=$tmp/below
rm -rf "$d"
mkdir -p "$d"
cp -a "$fx" "$d/sub"
run "$d" UNSAFE_MANIFEST=sub/Cargo.toml
verdict "a workspace below the root, through UNSAFE_MANIFEST" 0 "lint-unsafe: PASS"
grep -qx 'ok   sub/crates/safe/src/lib.rs' "$tmp/out" || { cat "$tmp/out" >&2; fail "UNSAFE_MANIFEST: the roots are not named from the port's root"; }
run "$d"
verdict "no workspace manifest at the root refuses the run" 2 "lint-unsafe: no workspace manifest"

# shellcheck disable=SC2016  # the stub's own variables
printf '#!/bin/sh\necho "$CARGO_PKG_NAME" >> %s\nexec "$@"\n' "$tmp/outer.log" > "$tmp/outer"
chmod +x "$tmp/outer"
: > "$tmp/outer.log"
run "$fx" RUSTC_WORKSPACE_WRAPPER="$tmp/outer"
verdict "the caller's RUSTC_WORKSPACE_WRAPPER" 0 "lint-unsafe: PASS"
# A call with no package (cargo's probe of rustc) logs an empty line.
[ "$(sed '/^$/d' "$tmp/outer.log" | sort -u | tr '\n' ' ')" = "capi cmem safe vpair " ] \
  || { cat "$tmp/outer.log" >&2; fail "the caller's RUSTC_WORKSPACE_WRAPPER: got = these packages, want = every member"; }
echo "ok the caller's RUSTC_WORKSPACE_WRAPPER runs for every member"

d=$(copy)
printf '[features]\nloud = []\n' >> "$d/crates/vpair/Cargo.toml"
printf '#[cfg(feature = "loud")]\n#[allow(unsafe_code)]\npub fn loud() -> u8 {\n    let x = 0u8;\n    unsafe { *(&raw const x) }\n}\n' >> "$d/crates/vpair/src/lib.rs"
run "$d"
verdict "unsafe code behind a feature passes without it" 0 "lint-unsafe: PASS"
run "$d" UNSAFE_CHECK_FLAGS="--features vpair/loud"
verdict "unsafe code behind a feature fails with UNSAFE_CHECK_FLAGS naming it" 1 "a crate other than capi cmem uses unsafe code"

# A target with required-features is skipped by --all-targets unless
# they are on: in a non-fence member it fails the run without
# --all-features, and is checked with it; in a fence crate it is no
# matter.
d=$(copy)
printf '[features]\nloud = []\n\n[[example]]\nname = "gated"\nrequired-features = ["loud"]\n' >> "$d/crates/vpair/Cargo.toml"
printf '#![allow(unsafe_code)]\nfn main() {\n    let x = 0u8;\n    let _ = unsafe { *(&raw const x) };\n}\n' > "$d/crates/vpair/examples/gated.rs"
run "$d"
verdict "a non-fence member's target with required-features fails without --all-features" 1 \
  "FAIL crates/vpair/examples/gated.rs (vpair) has required-features"
run "$d" UNSAFE_CHECK_FLAGS=--all-features
verdict "with --all-features the gated target is checked, and its unsafe code fails" 1 "a crate other than capi cmem uses unsafe code"
d=$(copy)
printf '[features]\nloud = []\n\n[[example]]\nname = "gated"\nrequired-features = ["loud"]\n' >> "$d/crates/capi/Cargo.toml"
mkdir -p "$d/crates/capi/examples"
printf 'fn main() {}\n' > "$d/crates/capi/examples/gated.rs"
run "$d"
verdict "a fence crate's target with required-features passes" 0 "lint-unsafe: PASS"

# --all-targets skips a test that says test = false and a bench that
# says bench = false; each target's source must reach rustc.
d=$(copy)
printf '\n[[test]]\nname = "t"\ntest = false\n' >> "$d/crates/vpair/Cargo.toml"
printf '#![allow(unsafe_code)]\n#[test]\nfn two() {\n    let x = 2u8;\n    assert_eq!(unsafe { *(&raw const x) }, vpair::two());\n}\n' > "$d/crates/vpair/tests/t.rs"
run "$d"
verdict "a test with test = false fails the run" 1 "FAIL crates/vpair/tests/t.rs (vpair, target t): cargo check --all-targets did not compile this target"
# The same skipped test, its file also an example's: the example's
# compile (no cfg(test)) must not count for the test.
printf '\n[[example]]\nname = "shared"\npath = "tests/t.rs"\ntest = false\nbench = false\n' >> "$d/crates/vpair/Cargo.toml"
printf '#![cfg_attr(test, allow(unsafe_code))]\n#[test]\nfn two() {\n    let x = 2u8;\n    assert_eq!(unsafe { *(&raw const x) }, vpair::two());\n}\nfn main() {}\n' > "$d/crates/vpair/tests/t.rs"
run "$d"
verdict "a skipped test whose file an example compiles fails the run" 1 "FAIL crates/vpair/tests/t.rs (vpair, target t): cargo check --all-targets did not compile this target"
verdict "a file two targets share fails the run" 1 "FAIL crates/vpair/tests/t.rs (vpair) is the source of more than one of its targets"
d=$(copy)
mkdir -p "$d/crates/vpair/benches"
printf '#![allow(unsafe_code)]\nfn main() {\n    let x = 0u8;\n    let _ = unsafe { *(&raw const x) };\n}\n' > "$d/crates/vpair/benches/b.rs"
printf '\n[[bench]]\nname = "b"\nharness = false\nbench = false\n' >> "$d/crates/vpair/Cargo.toml"
run "$d"
verdict "a bench with bench = false fails the run" 1 "FAIL crates/vpair/benches/b.rs (vpair, target b): cargo check --all-targets did not compile this target"
# And a bench that is benched is checked: its unsafe code fails.
d=$(copy)
mkdir -p "$d/crates/vpair/benches"
printf '#![allow(unsafe_code)]\nfn main() {\n    let x = 0u8;\n    let _ = unsafe { *(&raw const x) };\n}\n' > "$d/crates/vpair/benches/b.rs"
printf '\n[[bench]]\nname = "b"\nharness = false\n' >> "$d/crates/vpair/Cargo.toml"
run "$d"
verdict "a benched bench's unsafe code fails the cargo check" 1 "a crate other than capi cmem uses unsafe code"

# A target belongs to the package that declares it, wherever its file
# is: outside every member's directory (an absolute path), or inside a
# fence crate's.
d=$(copy)
mkdir -p "$tmp/outside"
printf '#![allow(unsafe_code)]\nfn main() {\n    let x = 0u8;\n    let _ = unsafe { *(&raw const x) };\n}\n' > "$tmp/outside/ext.rs"
printf '[features]\nloud = []\n\n[[example]]\nname = "ext"\npath = "%s"\nrequired-features = ["loud"]\n' "$tmp/outside/ext.rs" >> "$d/crates/vpair/Cargo.toml"
run "$d"
verdict "a gated example of vpair's outside every member's directory fails" 1 "$tmp/outside/ext.rs (vpair) has required-features"
run "$d" UNSAFE_CHECK_FLAGS=--all-features
verdict "with --all-features, the outside example's unsafe code fails the cargo check" 1 "a crate other than capi cmem uses unsafe code"
d=$(copy)
mkdir -p "$d/crates/capi/extra"
printf '#![allow(unsafe_code)]\n#[test]\nfn x() {\n    let x = 0u8;\n    let _ = unsafe { *(&raw const x) };\n}\n' > "$d/crates/capi/extra/x.rs"
printf '\n[[test]]\nname = "x"\npath = "../capi/extra/x.rs"\ntest = false\n' >> "$d/crates/vpair/Cargo.toml"
run "$d"
verdict "a skipped test of vpair's in the fence crate's directory fails" 1 "FAIL crates/vpair/../capi/extra/x.rs (vpair, target x): cargo check --all-targets did not compile this target"
# The same test, compiled (test = true) and safe: its path, as cargo
# gives it to rustc and in metadata, matches.
d=$(copy)
mkdir -p "$d/crates/capi/extra"
printf '#[test]\nfn x() {\n    assert_eq!(vpair::two(), 2);\n}\n' > "$d/crates/capi/extra/x.rs"
printf '\n[[test]]\nname = "x"\npath = "../capi/extra/x.rs"\n' >> "$d/crates/vpair/Cargo.toml"
run "$d"
verdict "a compiled test of vpair's in the fence crate's directory passes" 0 "lint-unsafe: PASS"

# One package's compile of a file does not vouch for another package's
# skipped target of the same name and file: vpair's test t (test =
# false) has unsafe code under its default feature; safe compiles the
# same file, by absolute path, as its own test t, without the feature.
d=$(copy)
printf '[features]\ndefault = ["loud"]\nloud = []\n\n[[test]]\nname = "t"\ntest = false\n' >> "$d/crates/vpair/Cargo.toml"
printf '#![cfg_attr(feature = "loud", allow(unsafe_code))]\n#[cfg(feature = "loud")]\n#[test]\nfn loud() {\n    let x = 0u8;\n    let _ = unsafe { *(&raw const x) };\n}\n' > "$d/crates/vpair/tests/t.rs"
printf '\n[[test]]\nname = "t"\npath = "%s"\n' "$d/crates/vpair/tests/t.rs" >> "$d/crates/safe/Cargo.toml"
run "$d"
verdict "another package's compile of the same target file does not vouch for vpair's" 1 \
  "FAIL crates/vpair/tests/t.rs (vpair, target t): cargo check --all-targets did not compile this target"

# Metadata whose targets the script cannot read (here a cargo shim
# renames src_path, as a format change might) refuses the run rather
# than checking no target.
mkdir -p "$tmp/shim"
real_cargo=$(command -v cargo)
cat > "$tmp/shim/cargo" <<EOF
#!/bin/sh
if [ "\$1" = metadata ]; then
  "$real_cargo" "\$@" | sed 's/"src_path":/"source_path":/g'
else
  exec "$real_cargo" "\$@"
fi
EOF
chmod +x "$tmp/shim/cargo"
run "$fx" PATH="$tmp/shim:$PATH"
verdict "metadata with no target it can read refuses the run" 2 "cargo metadata gave no target lint-unsafe.sh could read for"

# A check that leaves a member out checks nothing for it: one member
# (vpair, whose test could lower the workspace lint unseen), and every
# member but the fence crates.
run "$fx" UNSAFE_CHECK_FLAGS="--exclude vpair"
verdict "a cargo check that leaves one member out fails" 1 "cargo check compiled vpair through no wrapper"
run "$fx" UNSAFE_CHECK_FLAGS="--exclude capi --exclude safe --exclude vpair"
verdict "a cargo check that compiles only fence crates fails" 1 "cargo check compiled no member but capi cmem through the wrapper"

# UNSAFE_CRATES naming every member leaves nothing to check.
run "$fx" UNSAFE_CRATES="capi cmem safe vpair"
verdict "UNSAFE_CRATES naming every member refuses the run" 2 "UNSAFE_CRATES names every workspace member"

# A name in a literal string is read, so the member stays in the checks.
d=$(copy)
awk '{ sub(/^name = "vpair"$/, "name = \047vpair\047"); print }' "$d/crates/vpair/Cargo.toml" > "$tmp/m" && cp "$tmp/m" "$d/crates/vpair/Cargo.toml"
grep -qx "name = 'vpair'" "$d/crates/vpair/Cargo.toml" || fail "the literal-string case: the manifest was not rewritten"
run "$d" UNSAFE_CHECK_FLAGS="--exclude vpair"
verdict "a member named in a literal string and left out fails" 1 "cargo check compiled vpair through no wrapper"
run "$d"
verdict "a member named in a literal string passes when checked" 0 "lint-unsafe: PASS"

# The names are cargo's, so no manifest text misleads them: dotted keys,
# a multiline string holding a `name = ` line before the real one, and a
# name that is not the directory's (an ID with NAME@).
d=$(copy)
printf 'package.name = "vpair"\npackage.version = "0.1.0"\npackage.edition = "2021"\nlints.workspace = true\n' > "$d/crates/vpair/Cargo.toml"
run "$d" UNSAFE_CHECK_FLAGS="--exclude vpair"
verdict "a member named by dotted keys and left out fails" 1 "cargo check compiled vpair through no wrapper"
d=$(copy)
awk '{ print } /^\[package\]$/ { print "description = \"\"\""; print "name = \"safe\""; print "\"\"\"" }' "$d/crates/vpair/Cargo.toml" > "$tmp/m" && cp "$tmp/m" "$d/crates/vpair/Cargo.toml"
run "$d" UNSAFE_CHECK_FLAGS="--exclude vpair"
verdict "a name line in a multiline string does not hide the member left out" 1 "cargo check compiled vpair through no wrapper"
d=$(copy)
awk '{ print } /^\[package\]$/ { print "description = \"\"\""; print "name = \"cmem\""; print "\"\"\"" }' "$d/crates/vpair/Cargo.toml" > "$tmp/m" && cp "$tmp/m" "$d/crates/vpair/Cargo.toml"
printf 'pub fn two() -> u8 {\n    2\n}\n' > "$d/crates/vpair/src/lib.rs"
run "$d"
verdict "a fence crate's name in a multiline string does not make the member a fence crate" 1 "FAIL crates/vpair/src/lib.rs lacks"
d=$(copy)
awk '{ sub(/^name = "vpair"$/, "name = \"vp\""); print }' "$d/crates/vpair/Cargo.toml" > "$tmp/m" && cp "$tmp/m" "$d/crates/vpair/Cargo.toml"
for f in "$d"/crates/vpair/src/bin/tool.rs "$d"/crates/vpair/tests/t.rs; do
  awk '{ gsub(/vpair::/, "vp::"); print }' "$f" > "$tmp/m" && cp "$tmp/m" "$f"
done
(cd "$d" && cargo generate-lockfile --offline -q) || fail "cargo generate-lockfile for the renamed member"
run "$d" UNSAFE_CHECK_FLAGS="--exclude vp"
verdict "a member whose name is not its directory's, left out, fails" 1 "cargo check compiled vp through no wrapper"
run "$d"
verdict "a member whose name is not its directory's passes when checked" 0 "lint-unsafe: PASS"

echo "PASS"
