#!/bin/sh
# kit/internal-test.sh: fail if a file a port vendors or copies from the
# kit names something internal to the kit's maintainers (the internal
# check). A port may be public, and kit/sync.sh vendors kit/count and
# kit/make into it while a new port copies kit/templates, so none of
# those files may name an internal repository or organization, an
# internal tool or URL, or a person. Nor may the header kit/sync.sh
# writes into each port's verus/count/KIT (its `cat > "$kit" <<EOF`
# text). kit/templates/README.md, the kit's own notes on the templates,
# is not copied and is not held; nor is kit/count/target, the build
# output, or the rest of kit/sync.sh. The markers are matched without
# regard to case; a word marker (mono, matt) matches only as a word, so
# monotonic and monomorphized pass.
#
# Then the mutants: each sample below is planted in a scratch copy of a
# vendored file under kit/count and under kit/make, of a copied template
# and of sync.sh's KIT header, and the scan must name it; planted in
# kit/templates/README.md, under kit/count/target or in sync.sh outside
# that header, it must not. With any one marker left out, some sample
# must name no marker. The check runs it with BusyBox's tools. Run it
# from anywhere.
set -eu

repo=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# The markers, compared in lower case: each plain marker matches
# anywhere, each word marker only as a word.
plain='chainguard-dev/|chainguard-project-|wolfi-dev/|chainguard-internal|codeowners-gate|ai-cicd/|chainbox|guardener|slack|mattmoor|mattomata|dlorenc|tcnghia|vreeland|imjasonh|decided \('
words='mono|matt|justin|nghia'

# markers_of PLAIN WORDS: the extended regular expression that matches
# the plain markers PLAIN and the word markers WORDS.
markers_of() {
  printf '%s\n' "$1|(^|[^a-z0-9_])($2)([^a-z0-9_]|$)"
}
markers=$(markers_of "$plain" "$words")

# A sample for each marker, one a line, in mixed case. Each names one
# marker only, so leaving any marker out leaves its sample unnamed.
samples='See Chainguard-Dev/infra for the grant.
moved to chainguard-project-theseus/challoc
wolfi-dev/proved-port#21
the apply step (a Chainguard-internal tool)
render it with codeowners-gate
ai-cicd/sandbox/docs/identities.md
Mono'"'"'s code-reviewer README
mono
https://chainbox-console.example/run/1
the Guardener link
ask in #proof-Slack
@MattMoor
mattomata@example.com
dlorenc
tcnghia'"'"'s renamed member
Vreeland'"'"'s review
imjasonh
decided (by the reviewers)
as Matt said
Justin'"'"'s scrub
Nghia found it'

# Text the markers must leave alone.
clean='a monotonic count
the monomorphized body
Monomorphic code, not matte paint'

# shipped ROOT: the files under ROOT a port vendors or copies, one a line.
# Every scan runs in a condition, where set -e is off, so a missing root
# or a failing find fails here rather than leaving that tree unscanned.
shipped() {
  for _shipped_d in "$1/kit/count" "$1/kit/make" "$1/kit/templates"; do
    [ -d "$_shipped_d" ] || fail "no directory $_shipped_d"
  done
  find "$1/kit/count" "$1/kit/make" "$1/kit/templates" \
    -path "$1/kit/count/target" -prune -o \
    -type f ! -path "$1/kit/templates/README.md" -print ||
    fail "find failed under $1"
}

# kit_header ROOT: the text ROOT's kit/sync.sh writes to a port's
# verus/count/KIT, its `cat > "$kit" <<EOF` here-document.
kit_header() {
  awk '/^cat > "\$kit" <<EOF$/ { on = 1; next } on && /^EOF$/ { exit } on { print }' "$1/kit/sync.sh"
}

# scan ROOT: succeed when no shipped file under ROOT, nor the KIT header
# its kit/sync.sh writes, names a marker; otherwise fail with each line
# that does in $tmp/hits.
scan() {
  : > "$tmp/hits"
  shipped "$1" > "$tmp/files"
  [ -s "$tmp/files" ] || fail "no shipped files under $1"
  kit_header "$1" > "$tmp/sync.sh-KIT"
  [ -s "$tmp/sync.sh-KIT" ] || fail "no KIT here-document in $1/kit/sync.sh"
  echo "$tmp/sync.sh-KIT" >> "$tmp/files"
  while IFS= read -r _scan_f; do
    grep -niHE -e "$markers" "$_scan_f" >> "$tmp/hits" || [ $? -eq 1 ] || fail "grep failed on $_scan_f"
  done < "$tmp/files"
  [ ! -s "$tmp/hits" ]
}

if ! scan "$repo"; then
  cat "$tmp/hits" >&2
  fail "the files above ship to ports and name something internal; reword them (kit/internal-test.sh)"
fi
echo "ok no internal references in $(($(wc -l < "$tmp/files") - 1)) shipped files and sync.sh's KIT header"

# The mutants run on a scratch root holding one vendored file from each
# of kit/count and kit/make, one copied template, kit/sync.sh, the
# templates' README and a build output.
copy=$tmp/copy
vendored=kit/count/README.md
made=kit/make/headers.sh
copied=kit/templates/.chainguard/reviewers.yaml
for f in $vendored $made $copied kit/sync.sh kit/templates/README.md; do
  mkdir -p "$copy/$(dirname "$f")"
  cp "$repo/$f" "$copy/$f"
done
mkdir -p "$copy/kit/count/target"
echo built > "$copy/kit/count/target/out"
scan "$copy" || { cat "$tmp/hits" >&2; fail "the scratch root has a marker before any is planted"; }

# plant FILE TEXT: FILE in the scratch root with TEXT added as its last line.
plant() {
  cp "$copy/$1" "$tmp/keep"
  echo "$2" >> "$copy/$1"
}
# unplant FILE: FILE as it was before plant.
unplant() {
  cp "$tmp/keep" "$copy/$1"
}

n=0
while IFS= read -r s; do
  for f in $vendored $made $copied; do
    plant "$f" "$s"
    scan "$copy" && fail "the scan misses \"$s\" in $f"
    grep -q "^$copy/$f:" "$tmp/hits" || fail "the scan names another file than $f for \"$s\""
    unplant "$f"
  done
  # Into the KIT header: the line after sync.sh's `cat > "$kit" <<EOF`.
  cp "$copy/kit/sync.sh" "$tmp/keep"
  awk -v s="# $s" '{ print } /^cat > "\$kit" <<EOF$/ { print s }' "$tmp/keep" > "$copy/kit/sync.sh"
  scan "$copy" && fail "the scan misses \"$s\" in sync.sh's KIT header"
  grep -q "^$tmp/sync.sh-KIT:" "$tmp/hits" || fail "the scan names another file than sync.sh's KIT header for \"$s\""
  unplant kit/sync.sh
  for f in kit/templates/README.md kit/count/target/out kit/sync.sh; do
    plant "$f" "$s"
    scan "$copy" || fail "the scan holds $f, which no port gets, to \"$s\""
    unplant "$f"
  done
  n=$((n + 1))
done <<EOF
$samples
EOF
[ "$n" -ge 21 ] || fail "only $n samples ran"
echo "ok the scan names each of $n planted markers, and none in what no port gets"

# without LIST I: the |-separated LIST with its Ith entry left out.
without() {
  printf '%s\n' "$1" | awk -F'|' -v i="$2" '{
    o = ""
    for (j = 1; j <= NF; j++) if (j != i) o = o (o == "" ? "" : "|") $j
    print o
  }'
}
# unnamed MARKER EXPR: fail unless, with MARKER left out to give EXPR,
# some sample names no marker, so the planted-marker checks above fail.
unnamed() {
  printf '%s\n' "$samples" | grep -viqE -e "$2" ||
    fail "with $1 left out every sample still names a marker; give it a sample of its own"
}
m=0
for list in plain words; do
  case $list in
    plain) all=$plain ;;
    words) all=$words ;;
  esac
  count=$(printf '%s\n' "$all" | awk -F'|' '{ print NF }')
  i=1
  while [ "$i" -le "$count" ]; do
    alt=$(printf '%s\n' "$all" | cut -d'|' -f"$i")
    if [ "$list" = plain ]; then
      unnamed "$alt" "$(markers_of "$(without "$plain" "$i")" "$words")"
    else
      unnamed "$alt" "$(markers_of "$plain" "$(without "$words" "$i")")"
    fi
    i=$((i + 1))
    m=$((m + 1))
  done
done
[ "$m" -ge 20 ] || fail "only $m markers were left out"
echo "ok leaving out any one of the $m markers leaves a sample unnamed"

while IFS= read -r s; do
  plant $vendored "$s"
  scan "$copy" || { cat "$tmp/hits" >&2; fail "the scan names \"$s\", which names nothing internal"; }
  unplant $vendored
done <<EOF
$clean
EOF
echo "ok the word markers pass words that only start with them"
