#!/bin/sh
# kit/install.sh PREFIX: installs this checkout's kit under PREFIX as the
# theseus-proved-port package installs it under /usr: the counter built
# from kit/count as PREFIX/bin/verus-count, the kit's Go tools built
# with go as PREFIX/bin/proved-port-fncmp (kit/perf/codegen/fncmp, the
# codegen comparer), PREFIX/bin/proved-port-stats (kit/perf/stats, the
# bench summaries) and PREFIX/bin/proved-port-seams (kit/verus/seams,
# the seam ledger generator), kit/make/* (mode 0644) in
# PREFIX/share/proved-port/make/, all of kit/templates in
# PREFIX/share/proved-port/templates/, kit/CHANGELOG.md (the releases'
# porting notes) as PREFIX/share/proved-port/CHANGELOG.md, and
# PREFIX/share/proved-port/KIT
# with the checkout's commit and the release the counter's --version
# names. It is how a host without the package (a Mac) runs a port's kit
# targets from a proved-port clone:
#
#   git -C <clone> checkout vX.Y.Z
#   sh <clone>/kit/install.sh "$HOME/.local"
#   make verus PROVED_PORT_HOME="$HOME/.local/share/proved-port" \
#     VERUS_COUNT="$HOME/.local/bin/verus-count" VERUS_BASE=none
#
# The port's PROVED_PORT_VERSION must be the release KIT records, as with
# the package, so check out the tag the port pins. KIT records the
# release the counter's --version names, which is the last one until the
# next is tagged, so the install refuses (exit 1) unless the checkout is
# that release's tag with no change under kit/: else KIT would pass the
# kit targets' version check for files that are not that release's.
# --dev installs a development tree all the same, saying so; a port's
# runs against it check nothing about a release.
# kit/installed-test.sh installs the kit through this script. It builds
# the counter (cargo; CARGO_TARGET_DIR, if set, is where) and the Go
# tools (go, with GOTOOLCHAIN=local and GOFLAGS=-mod=mod as the kit's
# Go checks build them, into the same target directory's
# go/), all before it touches PREFIX, so a failed build leaves an
# earlier install as it was.
set -eu

dev=
if [ "${1-}" = --dev ]; then
  dev=1
  shift
fi
[ $# -eq 1 ] || { echo "usage: kit/install.sh [--dev] PREFIX" >&2; exit 2; }
prefix=$1
kit=$(cd "$(dirname "$0")" && pwd)
target=${CARGO_TARGET_DIR:-$kit/count/target}
# Absolute, as cargo reads a relative one from here: the Go builds run
# from each module's directory.
case $target in
  /*) ;;
  *) target=$(pwd)/$target ;;
esac

cargo build --locked --release --manifest-path "$kit/count/Cargo.toml" --target-dir "$target"
# The Go tools: the package's names, a module directory each. The names
# carry the kit's prefix because fncmp, stats and seams are too generic
# for /usr/bin.
gotools='fncmp:perf/codegen/fncmp stats:perf/stats seams:verus/seams'
mkdir -p "$target/go"
for t in $gotools; do
  (cd "$kit/${t#*:}" && GOTOOLCHAIN=local GOFLAGS=-mod=mod CGO_ENABLED=0 go build -trimpath -o "$target/go/proved-port-${t%%:*}" .)
done
# The package's version is its tag's, which is the counter's --version.
version=$("$target/release/verus-count" --version | sed 's/^verus-count //')
[ -n "$version" ] || { echo "kit/install.sh: verus-count --version printed no version" >&2; exit 1; }
commit=$(git -C "$kit" rev-parse HEAD)
tagged=$(git -C "$kit" rev-parse -q --verify "refs/tags/v$version^{commit}" || true)
changed=$(git -C "$kit" status --porcelain -- .)
why=
if [ "$tagged" != "$commit" ]; then
  why="$commit is not tag v$version"
elif [ -n "$changed" ]; then
  why="kit/ has changes not in tag v$version"
fi
if [ -n "$why" ]; then
  if [ -z "$dev" ]; then
    echo "kit/install.sh: $why, so KIT's tag v$version would misdescribe the install: check out the tag the port pins, or pass --dev for a development install" >&2
    exit 1
  fi
  echo "kit/install.sh: --dev: $why; KIT records tag v$version all the same" >&2
fi

home=$prefix/share/proved-port
# Over an earlier install, its KIT goes first, so an install that fails
# part way leaves no KIT, which every kit target refuses, rather than the
# earlier KIT over a mix of files. make/ and templates/ are replaced
# whole, as a package upgrade replaces them: a file this tree does not
# have does not stay.
rm -f "$home/KIT"
rm -rf "$home/make" "$home/templates"
mkdir -p "$prefix/bin" "$home/make" "$home/templates"
install -m755 "$target/release/verus-count" "$prefix/bin/verus-count"
for t in $gotools; do
  install -m755 "$target/go/proved-port-${t%%:*}" "$prefix/bin/proved-port-${t%%:*}"
done
install -m644 "$kit"/make/* "$home/make/"
cp -a "$kit/templates/." "$home/templates/"
install -m644 "$kit/CHANGELOG.md" "$home/CHANGELOG.md"
{
  echo "# kit: the proved-port kit installed by the theseus-proved-port package."
  echo "commit $commit"
  echo "tag v$version"
} > "$home/KIT"
echo "kit/install.sh: kit v$version ($commit) in $home, verus-count and proved-port-{fncmp,stats,seams} in $prefix/bin"
