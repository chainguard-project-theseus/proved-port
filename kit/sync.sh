#!/bin/sh
# kit/sync.sh: vendor the kit counter (kit/count) and the make targets
# (kit/make) into a port.
#
#   kit/sync.sh PORT_ROOT          copy kit/count at this clone's HEAD
#                                  into PORT_ROOT/verus/count, and
#                                  kit/make into PORT_ROOT/verus/make,
#                                  and record the commit in
#                                  verus/count/KIT
#   kit/sync.sh --check PORT_ROOT  fail unless PORT_ROOT/verus/count and
#                                  PORT_ROOT/verus/make hold the same
#                                  entries as the copies of the commit
#                                  the KIT file records: the same bytes,
#                                  executable bits, and symlinks with the
#                                  same targets
#
# verus/make also gets, under templates/, the kit templates its
# headers.sh holds the port's files to: the kit's Verus files, which
# kit/make/VERUS-FILES names, each as NAME.tmpl, so that no vendored path
# ends in the name of a file a port's code owners protect
# (reviewers.yaml) and a re-sync needs no owner of those files. A commit
# from before that file (v0.2.0) gets
# what that commit's sync.sh copied, its verus/FLOOR, RLIMIT, REACH and
# MINTS and every .chainguard/ file with a `# kit: proved-port` line, so
# --check holds a copy to the rule of the commit KIT records. A commit
# from before kit/make has none, and its copy has no verus/make.
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
make=$port/verus/make
kit=$dest/KIT

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# extract SHA DIR: kit/count at SHA, into DIR.
extract() {
  mkdir -p "$2"
  git -C "$repo" archive --format=tar "$1:kit/count" | tar -xf - -C "$2"
}

# extract_make SHA DIR: kit/make at SHA, and under templates/ the
# templates headers.sh reads, into DIR; nothing when SHA has no kit/make.
extract_make() {
  git -C "$repo" cat-file -e "$1:kit/make" 2>/dev/null || return 0
  mkdir -p "$2" "$tmp/templates-$1"
  git -C "$repo" archive --format=tar "$1:kit/make" | tar -xf - -C "$2"
  git -C "$repo" archive --format=tar "$1:kit/templates" | tar -xf - -C "$tmp/templates-$1"
  if [ -f "$2/VERUS-FILES" ]; then
    grep -v -e '^#' -e '^$' "$2/VERUS-FILES" | LC_ALL=C sort > "$tmp/templates-$1.list"
    while IFS= read -r f; do
      [ -f "$tmp/templates-$1/$f" ] || { echo "$0: kit/make/VERUS-FILES at $1 names $f, which kit/templates lacks" >&2; exit 1; }
      mkdir -p "$2/templates/$(dirname "$f")"
      cp "$tmp/templates-$1/$f" "$2/templates/$f.tmpl"
    done < "$tmp/templates-$1.list"
    return 0
  fi
  # A commit from before kit/make/VERUS-FILES (v0.2.0): the four ratchet
  # files and every .chainguard/ file with a `# kit:` line.
  (cd "$tmp/templates-$1" && find verus .chainguard -type f) | LC_ALL=C sort > "$tmp/templates-$1.list"
  while IFS= read -r f; do
    case $f in
      verus/FLOOR | verus/RLIMIT | verus/REACH | verus/MINTS | .chainguard/*) ;;
      *) continue ;;
    esac
    grep -q '^# kit: proved-port' "$tmp/templates-$1/$f" || continue
    mkdir -p "$2/templates/$(dirname "$f")"
    cp "$tmp/templates-$1/$f" "$2/templates/$f"
  done < "$tmp/templates-$1.list"
}

# files DIR: every file under DIR but KIT and target/, one per line.
files() {
  [ -d "$1" ] || return 0
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
  drift=
  # compare WANT GOT WHAT: whether directory GOT holds what WANT does,
  # WANT being WHAT at $sha.
  compare() {
    files "$1" > "$tmp/want"
    files "$2" > "$tmp/got"
    if ! cmp -s "$tmp/want" "$tmp/got"; then
      echo "$0: the file lists differ (< $3 at $sha, > $2):" >&2
      diff "$tmp/want" "$tmp/got" >&2 || true
      drift=1
    fi
    while IFS= read -r f; do
      want=$(kind "$1/$f")
      got=$(kind "$2/$f")
      # A missing entry is already in the file-list diff above.
      [ "$got" = missing ] && continue
      if [ "$got" != "$want" ]; then
        echo "$0: $2/${f#./} is $got; in $3 at $sha it is $want" >&2
        drift=1
      elif [ ! -L "$2/$f" ] && ! cmp -s "$1/$f" "$2/$f"; then
        echo "$0: $2/${f#./} differs from $3 at $sha" >&2
        drift=1
      fi
    done < "$tmp/want"
  }
  extract "$sha" "$tmp/kit"
  compare "$tmp/kit" "$dest" kit/count
  extract_make "$sha" "$tmp/make"
  compare "$tmp/make" "$make" "kit/make (and its templates)"
  if [ -n "$drift" ]; then
    echo "$0: re-run kit/sync.sh $port to restore it" >&2
    exit 1
  fi
  echo "ok $dest matches kit/count at $sha"
  if [ -d "$tmp/make" ]; then
    echo "ok $make matches kit/make at $sha"
  fi
  exit 0
fi

if [ -n "$(git -C "$repo" status --porcelain -- kit/count kit/make kit/templates)" ]; then
  echo "$0: kit/count, kit/make or kit/templates has uncommitted changes; the copy records a commit, so commit them first" >&2
  exit 1
fi
# A verus/count with no KIT file is the port's own counter, not a copy
# this script made; it is the port's to delete. So is a verus/make
# beside no KIT file.
if [ -e "$dest" ] && [ ! -f "$kit" ]; then
  echo "$0: $dest exists and has no KIT file, so it is not a vendored copy; delete it first" >&2
  exit 1
fi
if [ -e "$make" ] && [ ! -f "$kit" ]; then
  echo "$0: $make exists and $kit does not, so it is not a vendored copy; delete it first" >&2
  exit 1
fi
sha=$(git -C "$repo" rev-parse HEAD)
tag=$(git -C "$repo" describe --tags --exact-match HEAD 2>/dev/null) || tag=none

rm -rf "$dest" "$make"
extract "$sha" "$dest"
extract_make "$sha" "$make"
cat > "$kit" <<EOF
# kit: verus-count and verus/make vendored from the proved-port kit's
# kit/count and kit/make by kit/sync.sh. Do not edit either directory:
# change kit/count or kit/make there, then re-run kit/sync.sh PORT_ROOT,
# and check both with kit/sync.sh --check.
commit $sha
tag $tag
EOF
echo "synced $dest to kit/count at $sha (tag $tag)"
[ ! -d "$make" ] || echo "synced $make to kit/make at $sha"
