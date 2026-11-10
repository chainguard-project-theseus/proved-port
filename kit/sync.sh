#!/bin/sh
# kit/sync.sh: vendor the kit counter (kit/count) into a port.
#
#   kit/sync.sh PORT_ROOT          copy kit/count at this clone's HEAD
#                                  into PORT_ROOT/verus/count and record
#                                  the commit in verus/count/KIT
#   kit/sync.sh --check PORT_ROOT  fail unless PORT_ROOT/verus/count
#                                  holds the same entries as kit/count at
#                                  the commit its KIT file records: the
#                                  same bytes, executable bits, and
#                                  symlinks with the same targets
#
# The copy comes from `git archive` of the commit, not the working tree,
# so it is exactly what the recorded commit holds: no target/, no
# uncommitted edit, and no file the kit has since deleted. A port's CI
# cannot reach this repository, so --check is run here, by whoever
# changes the vendored copy (kit/count/README.md, "Adopting:
# vendoring").
set -eu

usage() {
  echo "usage: $0 [--check] PORT_ROOT" >&2
  exit 2
}

check=
case ${1-} in
  --check) check=1; shift ;;
  -*) usage ;;
esac
[ $# -eq 1 ] || usage
port=$1
[ -d "$port" ] || { echo "$0: $port is not a directory" >&2; exit 2; }

repo=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
dest=$port/verus/count
kit=$dest/KIT

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# extract SHA DIR: kit/count at SHA, into DIR.
extract() {
  mkdir -p "$2"
  git -C "$repo" archive --format=tar "$1:kit/count" | tar -xf - -C "$2"
}

# files DIR: every file under DIR but KIT and target/, one per line.
files() {
  (cd "$1" && find . \( -path ./target -o -path ./KIT \) -prune -o \
    \( -type f -o -type l \) -print) | LC_ALL=C sort
}

# kind PATH: what PATH is, without following a symlink: a symlink names
# its target, and a regular file says whether it is executable. A
# symlink in place of a file is drift even when its target has the same
# bytes, because the copy then depends on a file outside the commit.
kind() {
  if [ -L "$1" ]; then
    echo "a symlink to $(readlink "$1")"
  elif [ -f "$1" ] && [ -x "$1" ]; then
    echo "an executable file"
  elif [ -f "$1" ]; then
    echo "a non-executable file"
  elif [ -e "$1" ]; then
    echo "not a regular file"
  else
    echo missing
  fi
}

if [ -n "$check" ]; then
  [ -f "$kit" ] || { echo "$0: $kit is missing" >&2; exit 1; }
  sha=$(sed -n 's/^commit //p' "$kit")
  tag=$(sed -n 's/^tag //p' "$kit")
  [ -n "$sha" ] || { echo "$0: $kit records no commit" >&2; exit 1; }
  git -C "$repo" cat-file -e "$sha^{commit}" 2>/dev/null || {
    echo "$0: commit $sha ($kit) is not in $repo; fetch it" >&2
    exit 1
  }
  if [ "$tag" != none ]; then
    [ "$(git -C "$repo" rev-parse "refs/tags/$tag^{commit}" 2>/dev/null)" = "$sha" ] || {
      echo "$0: tag $tag ($kit) does not name $sha" >&2
      exit 1
    }
  fi
  extract "$sha" "$tmp/kit"
  files "$tmp/kit" > "$tmp/want"
  files "$dest" > "$tmp/got"
  drift=
  if ! cmp -s "$tmp/want" "$tmp/got"; then
    echo "$0: the file lists differ (< kit/count at $sha, > $dest):" >&2
    diff "$tmp/want" "$tmp/got" >&2 || true
    drift=1
  fi
  while IFS= read -r f; do
    want=$(kind "$tmp/kit/$f")
    got=$(kind "$dest/$f")
    # A missing entry is already in the file-list diff above.
    [ "$got" = missing ] && continue
    if [ "$got" != "$want" ]; then
      echo "$0: $dest/${f#./} is $got; in kit/count at $sha it is $want" >&2
      drift=1
    elif [ ! -L "$dest/$f" ] && ! cmp -s "$tmp/kit/$f" "$dest/$f"; then
      echo "$0: $dest/${f#./} differs from kit/count at $sha" >&2
      drift=1
    fi
  done < "$tmp/want"
  if [ -n "$drift" ]; then
    echo "$0: re-run kit/sync.sh $port to restore it" >&2
    exit 1
  fi
  echo "ok $dest matches kit/count at $sha"
  exit 0
fi

if [ -n "$(git -C "$repo" status --porcelain -- kit/count)" ]; then
  echo "$0: kit/count has uncommitted changes; the copy records a commit, so commit them first" >&2
  exit 1
fi
# A verus/count with no KIT file is the port's own counter, not a copy
# this script made; it is the port's to delete.
if [ -e "$dest" ] && [ ! -f "$kit" ]; then
  echo "$0: $dest exists and has no KIT file, so it is not a vendored copy; delete it first" >&2
  exit 1
fi
sha=$(git -C "$repo" rev-parse HEAD)
tag=$(git -C "$repo" describe --tags --exact-match HEAD 2>/dev/null) || tag=none

rm -rf "$dest"
extract "$sha" "$dest"
cat > "$kit" <<EOF
# kit: verus-count vendored from wolfi-dev/proved-port kit/count by
# kit/sync.sh. Do not edit this directory: change kit/count there, then
# re-run kit/sync.sh PORT_ROOT, and check it with kit/sync.sh --check.
commit $sha
tag $tag
EOF
echo "synced $dest to kit/count at $sha (tag $tag)"
