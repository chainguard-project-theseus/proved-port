#!/bin/sh
# kit/sync-test.sh: tests of kit/sync.sh (the count-sync check). Syncs
# into a scratch port, then shows --check passes on the copy and fails
# on each kind of drift, and that a sync removes what the kit does not
# hold. Run it from the repository root.
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

echo "PASS"
