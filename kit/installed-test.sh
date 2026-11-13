#!/bin/sh
# kit/installed-test.sh: tests of the installed mode of kit/make (the
# count-installed check). It installs the kit into a scratch root the way
# the theseus-proved-port package's build does: the counter built from
# kit/count as usr/bin/verus-count, kit/make/* (mode 0644) in
# usr/share/proved-port/make/, all of kit/templates in
# usr/share/proved-port/templates/, and usr/share/proved-port/KIT with
# the commit and tag. Then a fixture port runs:
#   - vendored (kit/sync.sh's copy, `include verus/make/proved-port.mk`)
#     and installed (`include $(PROVED_PORT_HOME)/make/proved-port.mk`),
#     and the counter's output is the same in both;
#   - `make -n` shows that installed mode runs verus-count from PATH and
#     builds nothing, even with a verus/count/Cargo.toml in the port;
#   - PROVED_PORT_VERSION: a version KIT does not record, and a missing
#     KIT, fail every kit target before it runs anything; another
#     verus-count first on PATH fails verus and verus-count; the
#     recorded one passes; vendored mode does not read it;
#   - vendored, VERUS_COUNT is read from the command line only.
# Run it from the repository root. It builds the counter (cargo).
set -eu

export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0='*'

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# expect WHAT GOT WANT: GOT and WANT are the same lines.
expect() {
  if [ "$2" != "$3" ]; then
    printf 'got:\n%s\nwant:\n%s\n' "$2" "$3" >&2
    fail "$1"
  fi
  echo "ok $1"
}

other=0.0.1-other
target=${CARGO_TARGET_DIR:-$tmp/target}

# --- the scratch kit: this tree's kit/, committed -------------------------
kitrepo=$tmp/kit
mkdir -p "$kitrepo/kit"
cp kit/sync.sh "$kitrepo/kit/sync.sh"
cp -R kit/make kit/templates "$kitrepo/kit/"
mkdir "$kitrepo/kit/count"
for f in Cargo.toml Cargo.lock src; do
  cp -R "kit/count/$f" "$kitrepo/kit/count/"
done
git -C "$kitrepo" init -q
git -C "$kitrepo" add .
git -C "$kitrepo" -c user.name=installed-test -c user.email=installed-test@example.com \
  -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -qm kit
sha=$(git -C "$kitrepo" rev-parse HEAD)

# --- the install, as the package builds it --------------------------------
cargo build --locked --release --manifest-path "$kitrepo/kit/count/Cargo.toml" --target-dir "$target"
# The package's version is its tag's, which is the counter's --version.
version=$("$target/release/verus-count" --version | sed 's/^verus-count //')
[ -n "$version" ] || fail "verus-count --version printed no version"
root=$tmp/root
home=$root/usr/share/proved-port
mkdir -p "$root/usr/bin" "$home/make" "$home/templates"
install -m755 "$target/release/verus-count" "$root/usr/bin/verus-count"
(
  cd "$kitrepo"
  install -m644 kit/make/* "$home/make/"
  cp -a kit/templates/. "$home/templates/"
  {
    echo "# kit: the proved-port kit installed by the theseus-proved-port package."
    printf 'commit '
    git rev-parse HEAD
    echo "tag v$version"
  } > "$home/KIT"
)
PATH=$root/usr/bin:$PATH
export PATH

# --- the fixture port, twice ----------------------------------------------
# fixture DIR: a port with one crate, verified and unverified functions,
# FLOOR at 0.0 and a count.toml with [floor].
fixture() {
  mkdir -p "$1/crates/a/src" "$1/verus"
  printf '[package]\nname = "a"\nversion = "0.0.0"\nedition = "2021"\n' > "$1/crates/a/Cargo.toml"
  printf 'pub fn f() {}\n\nverus! {\npub fn g() {}\n\npub fn h(x: u8) -> u8 {\n    x\n}\n}\n\npub fn i() {}\n' > "$1/crates/a/src/lib.rs"
  printf '[crates]\ndir = "crates"\n\n[floor]\npath = "verus/FLOOR"\n' > "$1/verus/count.toml"
  printf 'functions 0.0\nlines 0.0\nchecked-functions 0.0\nchecked-lines 0.0\n' > "$1/verus/FLOOR"
}

vendored=$tmp/vendored
fixture "$vendored"
sh "$kitrepo/kit/sync.sh" "$vendored" > /dev/null
cat > "$vendored/Makefile" <<'EOF'
VERUS_CRATES = a
include verus/make/proved-port.mk
EOF

installed=$tmp/installed
fixture "$installed"
cat > "$installed/Makefile" <<EOF
VERUS_CRATES = a
PROVED_PORT_VERSION = $version
PROVED_PORT_HOME ?= $home
include \$(PROVED_PORT_HOME)/make/proved-port.mk
EOF

# A counter of the installed release at another path, for VERUS_COUNT.
mkdir "$tmp/opt"
printf '#!/bin/sh\nexec %s "$@"\n' "$root/usr/bin/verus-count" > "$tmp/opt/verus-count"
chmod +x "$tmp/opt/verus-count"

# shows PORT TARGET [VAR=VALUE...]: `make -n TARGET` in PORT, each run of
# spaces one space.
shows() {
  _port=$1
  shift
  make -s -n -C "$_port" "$@" | tr -s ' ' | sed 's/ $//'
}

# --- the counter's output is the same ------------------------------------
count_args='VERUS_BASE=none VERUS_RLIMIT= VERUS_COUNT_TARGET='"$target"
# shellcheck disable=SC2086  # count_args is a list of words
make -s -C "$vendored" verus-count $count_args VERUS_COUNT_FLAGS=--list > "$tmp/vendored.out" 2> "$tmp/vendored.err" \
  || { cat "$tmp/vendored.err" >&2; fail "make verus-count, vendored"; }
grep -q 'Finished' "$tmp/vendored.err" \
  || { cat "$tmp/vendored.err" >&2; fail "make verus-count, vendored: got = no cargo build, want = the copy built first"; }
# shellcheck disable=SC2086
make -s -C "$installed" verus-count $count_args VERUS_COUNT_FLAGS=--list > "$tmp/installed.out" 2> "$tmp/installed.err" \
  || { cat "$tmp/installed.err" >&2; fail "make verus-count, installed"; }
grep -q '^shipped' "$tmp/vendored.out" || { cat "$tmp/vendored.out" >&2; fail "the vendored counter printed no shipped row"; }
expect "the installed counter's output is the vendored counter's" \
  "$(cat "$tmp/installed.out")" "$(cat "$tmp/vendored.out")"
[ ! -s "$tmp/installed.err" ] || { cat "$tmp/installed.err" >&2; fail "make verus-count, installed: got = output on stderr (a build?), want = none"; }
echo "ok make verus-count, installed, builds nothing"

# --- make -n, installed --------------------------------------------------
expect "make -n verus, installed: verus-count from PATH, no build" \
  "$(shows "$installed" verus VERUS_BASE=abc123)" \
  "mkdir -p target/
cargo verus focus --locked -p a -- --time-expanded --output-json > target/verus-stats.json
verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --base-rev abc123 ."
mkdir -p "$installed/verus/count"
echo '[package]' > "$installed/verus/count/Cargo.toml"
expect "make -n verus-count, installed, with a verus/count/Cargo.toml in the port" \
  "$(shows "$installed" verus-count VERUS_BASE=none)" \
  "verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --no-base ."
rm -r "$installed/verus/count"
expect "make -n verus-count, installed, VERUS_COUNT given" \
  "$(shows "$installed" verus-count VERUS_BASE=none VERUS_COUNT="$tmp/opt/verus-count")" \
  "$tmp/opt/verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --no-base ."
expect "make verus-count-test, installed, runs no cargo test" \
  "$(make -s -C "$installed" verus-count-test)" \
  "verus-count-test: verus-count is not built here; its package tests it"
expect "make -n kani-list, installed: the package's kani.sh" \
  "$(shows "$installed" kani-list)" "sh $home/make/kani.sh --list"
expect "make -n verus-count, vendored, VERUS_COUNT on the command line" \
  "$(shows "$vendored" verus-count VERUS_BASE=none VERUS_COUNT=/opt/bin/verus-count)" \
  "/opt/bin/verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --no-base ."
expect "make -n verus-count, vendored, VERUS_COUNT in the environment: the copy, built" \
  "$(VERUS_COUNT=/opt/bin/verus-count shows "$vendored" verus-count VERUS_BASE=none)" \
  "cargo build --locked --release --manifest-path verus/count/Cargo.toml --target-dir target/verus-count
target/verus-count/release/verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --no-base ."
expect "make -n verus-count, vendored, VERUS_COUNT in the environment with -e: the copy, built" \
  "$(VERUS_COUNT=/opt/bin/verus-count shows "$vendored" -e verus-count VERUS_BASE=none)" \
  "cargo build --locked --release --manifest-path verus/count/Cargo.toml --target-dir target/verus-count
target/verus-count/release/verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --no-base ."
expect "make -n verus-count-test, vendored, VERUS_COUNT on the command line: the copy's tests" \
  "$(shows "$vendored" verus-count-test VERUS_COUNT=/opt/bin/verus-count)" \
  "cargo test --locked --manifest-path verus/count/Cargo.toml --target-dir target/verus-count"
# A Makefile that sets VERUS_COUNT before the include, and one that sets
# it after: neither replaces the copy.
late=$tmp/late
mkdir "$late"
cp -R "$vendored/verus" "$vendored/crates" "$late/"
for where in before after; do
  if [ "$where" = before ]; then
    printf 'VERUS_CRATES = a\nVERUS_COUNT = /opt/bin/verus-count\ninclude verus/make/proved-port.mk\n' > "$late/Makefile"
  else
    printf 'VERUS_CRATES = a\ninclude verus/make/proved-port.mk\nVERUS_COUNT = /opt/bin/verus-count\n' > "$late/Makefile"
  fi
  expect "make -n verus-count, vendored, VERUS_COUNT set in the Makefile $where the include: the copy, built" \
    "$(shows "$late" verus-count VERUS_BASE=none)" \
    "cargo build --locked --release --manifest-path verus/count/Cargo.toml --target-dir target/verus-count
target/verus-count/release/verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --no-base ."
  # With -e and VERUS_COUNT in the environment too, its origin is
  # `environment override`.
  expect "make -e -n verus-count, vendored, VERUS_COUNT in the environment and the Makefile ($where): the copy, built" \
    "$(VERUS_COUNT=/opt/bin/other-count shows "$late" -e verus-count VERUS_BASE=none)" \
    "cargo build --locked --release --manifest-path verus/count/Cargo.toml --target-dir target/verus-count
target/verus-count/release/verus-count --floor verus/FLOOR --rlimit target/verus-stats.json --no-base ."
done

# --- PROVED_PORT_VERSION -------------------------------------------------
for t in verus verus-proofs verus-count verus-count-test verus-headers kani kani-list; do
  make -s -n -C "$installed" "$t" VERUS_BASE=none PROVED_PORT_VERSION="$version" > /dev/null 2> "$tmp/err" \
    || { cat "$tmp/err" >&2; fail "make -n $t, installed, PROVED_PORT_VERSION=$version (KIT's): got = failure, want = success"; }
  if make -s -n -C "$installed" "$t" VERUS_BASE=none PROVED_PORT_VERSION="$other" > "$tmp/out" 2> "$tmp/err"; then
    fail "make -n $t, installed, PROVED_PORT_VERSION=$other: got = success, want = failure"
  fi
  grep -q "PROVED_PORT_VERSION is $other, but $home/KIT records tag v$version" "$tmp/err" \
    || { cat "$tmp/err" >&2; fail "make -n $t: the mismatch message does not name both versions"; }
  [ ! -s "$tmp/out" ] || { cat "$tmp/out" >&2; fail "make -n $t, a mismatched version: got = commands, want = none"; }
  make -s -n -C "$vendored" "$t" VERUS_BASE=none PROVED_PORT_VERSION="$other" > /dev/null 2> "$tmp/err" \
    || { cat "$tmp/err" >&2; fail "make -n $t, vendored, PROVED_PORT_VERSION=$other: got = failure, want = success (vendored does not read it)"; }
  # No pin, as when a Makefile drops or misspells the line: installed
  # fails; vendored does not read it.
  if make -s -n -C "$installed" "$t" VERUS_BASE=none PROVED_PORT_VERSION= > "$tmp/out" 2> "$tmp/err"; then
    fail "make -n $t, installed, PROVED_PORT_VERSION empty: got = success, want = failure"
  fi
  grep -q "PROVED_PORT_VERSION is not set" "$tmp/err" \
    || { cat "$tmp/err" >&2; fail "make -n $t, no pin: the message does not say so"; }
  [ ! -s "$tmp/out" ] || { cat "$tmp/out" >&2; fail "make -n $t, no pin: got = commands, want = none"; }
  make -s -n -C "$vendored" "$t" VERUS_BASE=none PROVED_PORT_VERSION= > /dev/null 2> "$tmp/err" \
    || { cat "$tmp/err" >&2; fail "make -n $t, vendored, PROVED_PORT_VERSION empty: got = failure, want = success"; }
  echo "ok make -n $t: PROVED_PORT_VERSION, installed and vendored"
done
sed 's/^PROVED_PORT_VERSION = /PROVED_PORT_VERISON = /' "$installed/Makefile" > "$tmp/misspelt"
cp "$installed/Makefile" "$tmp/makefile.saved"
cp "$tmp/misspelt" "$installed/Makefile"
if make -s -n -C "$installed" verus-count VERUS_BASE=none > /dev/null 2> "$tmp/err"; then
  fail "make -n verus-count, installed, a misspelt PROVED_PORT_VERSION line: got = success, want = failure"
fi
grep -q "PROVED_PORT_VERSION is not set" "$tmp/err" || { cat "$tmp/err" >&2; fail "a misspelt pin: the message does not say it is not set"; }
cp "$tmp/makefile.saved" "$installed/Makefile"
echo "ok make -n verus-count, installed, fails a misspelt pin line"
mv "$home/KIT" "$tmp/kit-file.saved"
for pin in "PROVED_PORT_VERSION=$version" "PROVED_PORT_VERSION="; do
  if make -s -n -C "$installed" verus-count VERUS_BASE=none "$pin" > /dev/null 2> "$tmp/err"; then
    fail "make -n verus-count, installed, no KIT, $pin: got = success, want = failure"
  fi
  grep -q "KIT records no tag" "$tmp/err" || { cat "$tmp/err" >&2; fail "no KIT, $pin: the message does not say so"; }
done
mv "$tmp/kit-file.saved" "$home/KIT"
echo "ok a missing KIT fails installed mode, pin or no pin"

# Another counter first on PATH, of another version: the targets that run
# the counter fail on its --version; the others pass on KIT alone.
mkdir "$tmp/stale"
# Another release, and one whose version only starts with the pinned one.
for stale in "$other" "${version}1"; do
  printf '#!/bin/sh\necho "verus-count %s"\n' "$stale" > "$tmp/stale/verus-count"
  chmod +x "$tmp/stale/verus-count"
  for t in verus verus-count; do
    if PATH=$tmp/stale:$PATH make -s -n -C "$installed" "$t" VERUS_BASE=none PROVED_PORT_VERSION="$version" > "$tmp/out" 2> "$tmp/err"; then
      fail "make -n $t, installed, verus-count $stale first on PATH: got = success, want = failure"
    fi
    grep -q "$home/KIT records tag v$version, but \`verus-count --version\` prints verus-count $stale" "$tmp/err" \
      || { cat "$tmp/err" >&2; fail "make -n $t, verus-count $stale: the message does not name its version"; }
    [ ! -s "$tmp/out" ] || { cat "$tmp/out" >&2; fail "make -n $t, verus-count $stale: got = commands, want = none"; }
    echo "ok make -n $t fails verus-count $stale first on PATH"
  done
done
PATH=$tmp/stale:$PATH make -s -n -C "$installed" kani-list PROVED_PORT_VERSION="$version" > /dev/null \
  || fail "make -n kani-list, a stale counter: got = failure, want = success (it runs no counter)"

echo "ok kit $sha installed as the package installs it"
echo PASS
