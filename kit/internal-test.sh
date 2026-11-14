#!/bin/sh
# kit/internal-test.sh: fail if a file the kit ships names something
# internal to the kit's maintainers (the internal check). The kit is
# public, and it ships into ports, some of them public: the
# theseus-proved-port package installs the counter built from kit/count,
# kit/make and kit/templates, and a new port copies kit/templates. So
# none of those files may name an internal repository or organization,
# an internal tool or URL, or a person. kit/templates/README.md, the
# kit's own notes on the templates, is not copied and is not held; nor
# is kit/count/target, the build output. The markers are matched without
# regard to case; a word marker (mono, matt) matches only as a word, so
# monotonic and monomorphized pass. The kit's own public repository,
# chainguard-project-theseus/proved-port, is the one exemption, by
# exactly that name: any other chainguard-project- name is flagged,
# chainguard-project-theseus/challoc on the same line included.
#
# Then the mutants: each sample below is planted in a scratch copy of a
# shipped file under kit/count, under kit/make and under kit/templates,
# and the scan must name it; planted in kit/templates/README.md or under
# kit/count/target, it must not. With any one marker left out, some
# sample must name no marker. The check runs it with BusyBox's tools. Run
# it from anywhere.
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

# exempt: the awk program that deletes the kit's own repository name,
# chainguard-project-theseus/proved-port, from each line before the
# markers are matched. It reads a line as tokens, the runs of characters
# a GitHub owner or repository name may hold ([A-Za-z0-9_.-]), and
# deletes only an owner token that is exactly chainguard-project-theseus,
# a slash, and a repository token that is exactly proved-port, or
# proved-port. with no slash after it (the dot ending a sentence). Any
# other token stays whole, so a longer name on either side
# (corp-chainguard-project-theseus, proved-port-x, proved-port.x,
# proved-port..x) keeps its marker.
# shellcheck disable=SC2016 # $0 is awk's, not the shell's
exempt='{
  out = ""
  s = $0
  while (s != "") {
    if (!match(s, /^[A-Za-z0-9_.-]+/)) {
      out = out substr(s, 1, 1)
      s = substr(s, 2)
      continue
    }
    t = substr(s, 1, RLENGTH)
    s = substr(s, RLENGTH + 1)
    if (t == "chainguard-project-theseus" && substr(s, 1, 1) == "/" && match(substr(s, 2), /^[A-Za-z0-9_.-]+/)) {
      r = substr(s, 2, RLENGTH)
      if (r == "proved-port" || (r == "proved-port." && substr(s, RLENGTH + 2, 1) != "/")) {
        s = substr(s, RLENGTH + 2)
        continue
      }
    }
    out = out t
  }
  print out
}'

# A sample for each marker, one a line, in mixed case. Each names one
# marker only, so leaving any marker out leaves its sample unnamed.
samples='See Chainguard-Dev/infra for the grant.
moved to chainguard-project-theseus/challoc
chainguard-project-theseus/proved-port and chainguard-project-theseus/challoc
the chainguard-project-theseus/proved-port-internal fork
the chainguard-project-theseus/proved-port.internal mirror
the chainguard-project-theseus/proved-port..internal mirror
the corp-chainguard-project-theseus/proved-port mirror
see chainguard-project-theseus/proved-port./internal
chainguard-project-theseus:proved-port
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
Monomorphic code, not matte paint
the kit (chainguard-project-theseus/proved-port)
https://github.com/chainguard-project-theseus/proved-port/blob/main/kit/count/README.md
the marketplace is chainguard-project-theseus/proved-port.
chainguard-project-theseus/proved-port'

# shipped ROOT: the files under ROOT the kit ships, one a line. Every
# scan runs in a condition, where set -e is off, so a missing root or a
# failing find fails here rather than leaving that tree unscanned.
shipped() {
  for _shipped_d in "$1/kit/count" "$1/kit/make" "$1/kit/templates"; do
    [ -d "$_shipped_d" ] || fail "no directory $_shipped_d"
  done
  find "$1/kit/count" "$1/kit/make" "$1/kit/templates" \
    -path "$1/kit/count/target" -prune -o \
    -type f ! -path "$1/kit/templates/README.md" -print ||
    fail "find failed under $1"
}

# scan ROOT: succeed when no shipped file under ROOT names a marker;
# otherwise fail with each line that does in $tmp/hits, as FILE:LINE:TEXT
# with the exempt name deleted from TEXT.
scan() {
  : > "$tmp/hits"
  shipped "$1" > "$tmp/files"
  [ -s "$tmp/files" ] || fail "no shipped files under $1"
  while IFS= read -r _scan_f; do
    awk "$exempt" "$_scan_f" > "$tmp/scrubbed" || fail "awk failed on $_scan_f"
    grep -niE -e "$markers" "$tmp/scrubbed" > "$tmp/hit" || [ $? -eq 1 ] || fail "grep failed on $_scan_f"
    awk -v f="$_scan_f" '{ print f ":" $0 }' "$tmp/hit" >> "$tmp/hits"
  done < "$tmp/files"
  [ ! -s "$tmp/hits" ]
}

if ! scan "$repo"; then
  cat "$tmp/hits" >&2
  fail "the files above ship to ports and name something internal; reword them (kit/internal-test.sh)"
fi
echo "ok no internal references in $(wc -l < "$tmp/files" | tr -d ' ') shipped files"

# The mutants run on a scratch root holding one shipped file from each
# of kit/count, kit/make and kit/templates, the templates' README and a
# build output.
copy=$tmp/copy
in_count=kit/count/README.md
in_make=kit/make/headers.sh
in_templates=kit/templates/.chainguard/reviewers.yaml
for f in $in_count $in_make $in_templates kit/templates/README.md; do
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
  for f in $in_count $in_make $in_templates; do
    plant "$f" "$s"
    scan "$copy" && fail "the scan misses \"$s\" in $f"
    grep -q "^$copy/$f:" "$tmp/hits" || fail "the scan names another file than $f for \"$s\""
    unplant "$f"
  done
  for f in kit/templates/README.md kit/count/target/out; do
    plant "$f" "$s"
    scan "$copy" || fail "the scan holds $f, which no port gets, to \"$s\""
    unplant "$f"
  done
  n=$((n + 1))
done <<EOF
$samples
EOF
[ "$n" -ge 28 ] || fail "only $n samples ran"
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
  printf '%s\n' "$samples" | awk "$exempt" | grep -viqE -e "$2" ||
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
  for f in $in_count $in_make $in_templates; do
    plant "$f" "$s"
    scan "$copy" || { cat "$tmp/hits" >&2; fail "the scan names \"$s\" in $f, which names nothing internal"; }
    unplant "$f"
  done
done <<EOF
$clean
EOF
echo "ok the word markers pass words that only start with them, and the exemption passes the kit's own repository by its exact name"
