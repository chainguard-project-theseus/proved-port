#!/bin/sh
# kit/sync-test.sh: tests of kit/sync.sh (the count-sync check). Syncs
# into a scratch port, then shows --check passes on the copy and fails
# on each kind of drift, in verus/count and in verus/make, and that a
# sync removes what the kit does not hold. Run it from the repository
# root.
set -eu

# The checkout may belong to another user than the one running the
# check; git refuses such a repository unless it is marked safe.
export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0='*'

sync=kit/sync.sh
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
port=$tmp/port
mkdir "$port"
count=$port/verus/count

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# drifted WHAT: --check must fail, then a sync must restore the copy.
drifted() {
  if sh "$sync" --check "$port" > "$tmp/out" 2>&1; then
    cat "$tmp/out" >&2
    fail "--check passed after $1"
  fi
  echo "ok --check fails after $1"
  sh "$sync" "$port" > /dev/null
  sh "$sync" --check "$port" > /dev/null || fail "--check failed after re-sync ($1)"
}

sh "$sync" "$port"
[ -f "$count/KIT" ] || fail "no KIT file"
[ -f "$count/Cargo.toml" ] || fail "no Cargo.toml"
[ ! -e "$count/target" ] || fail "the copy has a target/"
grep -qx "commit $(git rev-parse HEAD)" "$count/KIT" || fail "KIT does not record HEAD"
sh "$sync" --check "$port" || fail "--check failed on a fresh copy"
echo "ok --check passes on a fresh copy"

echo '// drift' >> "$count/src/main.rs"
drifted "an edited file"

rm "$count/Cargo.lock"
drifted "a deleted file"

echo stale > "$count/stale.txt"
drifted "an added file"
[ ! -e "$count/stale.txt" ] || fail "sync left a file the kit does not hold"

chmod +x "$count/Cargo.toml"
drifted "a changed executable bit"

rm "$count/Cargo.toml"
ln -s nowhere "$count/Cargo.toml"
drifted "a dangling symlink in place of a file"

cp "$count/Cargo.toml" "$tmp/outside.toml"
rm "$count/Cargo.toml"
ln -s "$tmp/outside.toml" "$count/Cargo.toml"
drifted "a symlink to a file with the same bytes in place of the file"

sed 's/^commit .*/commit 0000000000000000000000000000000000000000/' "$count/KIT" > "$tmp/KIT"
cp "$tmp/KIT" "$count/KIT"
drifted "a KIT naming an unknown commit"

# Build output is not part of the copy: --check ignores it, a sync
# removes it.
mkdir -p "$count/target/debug"
echo built > "$count/target/debug/verus-count"
sh "$sync" --check "$port" > /dev/null || fail "--check failed on a target/"
echo "ok --check ignores target/"
sh "$sync" "$port" > /dev/null
[ ! -e "$count/target" ] || fail "sync left target/"
echo "ok sync removes target/"

# kit/make lands in verus/make, with the templates headers.sh reads:
# the four verus/ ratchet files and the .chainguard/ files with a
# `# kit:` line, and no other.
make=$port/verus/make
for f in proved-port.mk kani.sh headers.sh templates/verus/FLOOR \
  templates/verus/RLIMIT templates/.chainguard/ci/verus.yaml; do
  [ -f "$make/$f" ] || fail "the copy has no verus/make/$f"
done
for f in templates/.chainguard/review-prompt.md templates/verus/README.md \
  templates/verus/SEAMS.md templates/ci/lib.sh templates/README.md; do
  [ ! -e "$make/$f" ] || fail "the copy has verus/make/$f, which headers.sh does not read"
done
echo "ok sync copies kit/make and the header templates to verus/make"

echo '# drift' >> "$make/proved-port.mk"
drifted "an edited verus/make file"

rm "$make/templates/verus/FLOOR"
drifted "a deleted verus/make template"

echo stale > "$make/stale.txt"
drifted "a file added to verus/make"
[ ! -e "$make/stale.txt" ] || fail "sync left a file kit/make does not hold"

rm -r "$make"
drifted "a deleted verus/make"

# A verus/count with no KIT file is the port's own; sync leaves it.
own=$tmp/own
mkdir -p "$own/verus/count"
echo mine > "$own/verus/count/main.rs"
if sh "$sync" "$own" > "$tmp/out" 2>&1; then
  fail "sync replaced a verus/count with no KIT file"
fi
[ -f "$own/verus/count/main.rs" ] || fail "sync deleted a verus/count with no KIT file"
echo "ok sync refuses a verus/count with no KIT file"

if sh "$sync" --check "$own" > /dev/null 2>&1; then
  fail "--check passed with no KIT file"
fi
echo "ok --check fails with no KIT file"

# A verus/make beside no KIT file is the port's own too.
own=$tmp/ownmake
mkdir -p "$own/verus/make"
echo mine > "$own/verus/make/mine.mk"
if sh "$sync" "$own" > "$tmp/out" 2>&1; then
  fail "sync replaced a verus/make with no KIT file"
fi
[ -f "$own/verus/make/mine.mk" ] || fail "sync deleted a verus/make with no KIT file"
echo "ok sync refuses a verus/make with no KIT file"

# kit/count holds no symlink, so a scratch kit repository with one shows
# --check compares a symlink's target and refuses a file in its place.
kitrepo=$tmp/kitrepo
mkdir -p "$kitrepo/kit/count"
cp "$sync" "$kitrepo/kit/sync.sh"
echo a > "$kitrepo/kit/count/a.txt"
echo b > "$kitrepo/kit/count/b.txt"
ln -s a.txt "$kitrepo/kit/count/link"
git -C "$kitrepo" init -q
git -C "$kitrepo" add .
git -C "$kitrepo" -c user.name=sync-test -c user.email=sync-test@example.com \
  -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -qm kit
sync=$kitrepo/kit/sync.sh
port=$tmp/linkport
mkdir "$port"
count=$port/verus/count
sh "$sync" "$port" > /dev/null
[ -L "$count/link" ] || fail "sync did not copy the symlink as a symlink"
sh "$sync" --check "$port" > /dev/null || fail "--check failed on a fresh copy with a symlink"
echo "ok --check passes on a fresh copy with a symlink"

rm "$count/link"
cp "$count/a.txt" "$count/link"
drifted "a file with the target's bytes in place of a symlink"

rm "$count/link"
ln -s b.txt "$count/link"
drifted "a symlink retargeted"

# The scratch kit has no kit/make, as a kit commit from before it had
# none: its copy has no verus/make, and one there is drift.
[ ! -e "$port/verus/make" ] || fail "sync from a kit without kit/make made a verus/make"
echo "ok sync from a kit without kit/make makes no verus/make"
mkdir -p "$port/verus/make"
echo stray > "$port/verus/make/proved-port.mk"
drifted "a verus/make the kit commit does not hold"
[ ! -e "$port/verus/make" ] || fail "sync left a verus/make the kit commit does not hold"

echo "PASS"
