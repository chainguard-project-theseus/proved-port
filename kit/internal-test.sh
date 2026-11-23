#!/bin/sh
# kit/internal-test.sh: fail if a file the kit ships names something
# internal to the kit's maintainers (the internal check). The kit is
# public, and it ships into ports, some of them public: the
# theseus-proved-port package installs the counter built from kit/count,
# kit/make, kit/templates and kit/CHANGELOG.md, and a new port copies
# kit/templates. So
# none of those files may name an internal repository or organization
# (the theseus ports' private repositories among them),
# an internal tool or URL, or a person. This script may not spell a
# private repository's name either (CONTRIBUTING.md), so the markers
# catch the cg-NAME prefix most of the private ports share, and the
# other private names are held as sha256 digests (digests, below): each
# name-like run of a shipped file is hashed and compared, and a match is
# reported by file and line, not by name. kit/templates/README.md, the
# kit's own notes on the templates, is not copied and is not held; nor
# is kit/count/target, the build output. The markers are matched without
# regard to case; a word marker (matt) matches only as a word, so
# matte passes. The kit's own public repository,
# chainguard-project-theseus/proved-port, is the one exemption, by
# exactly that name: any other chainguard-project- name is flagged,
# chainguard-project-theseus/challoc on the same line included.
#
# Then the mutants: each sample below is planted in a scratch copy of a
# shipped file under kit/count, under kit/make and under kit/templates,
# and in kit/CHANGELOG.md, and the scan must name it; planted in kit/templates/README.md or under
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
plain='chainguard-dev/|chainguard-project-|cg-[a-z0-9]|wolfi-dev/|chainguard-internal|codeowners-gate|ai-cicd/|chainbox|guardener|slack|mattmoor|mattomata|dlorenc|tcnghia|vreeland|imjasonh|decided \('
words='matt|justin|nghia'

# The digests: the sha256, in hex, of each private name the cg- marker
# misses (the private repositories without that prefix, two names
# private ports had before, and the monorepo's), written in lower case
# with any `.` or `_` as `-` and hashed with no newline. Sorted, one a
# line. The scan only ever hashes one to three parts joined by single
# `-` (runs, below), so a name of four or more parts, or one with `--`
# or a leading or trailing separator, would never match. A private name that is also an everyday word or another public
# project's name is left out (two of the organization's, and one
# repository outside it): a digest of it would fail ordinary text, so
# review holds those. The digests keep the names from being spelled;
# they are not a secret (CONTRIBUTING.md). To add a name, see
# CONTRIBUTING.md.
digests='2369469ad526a931a50bebcb9154f1355b9d8d1320d5cf86cae100a0310fac54
6fae9cb219534e00845d394bf212910623fae2ff0f5320b48be2c55dd61157f1
70648cb782994b5514b61227e71700ec897006c396951c9b3380d9effdec50f8
72c71261ad6ef574a63b5c33458ea1435e7b3ab1f5e9b20667b53f3a88648556
8033b5716ee24490ccd59dce333d49e569bfd8426b5e0007c95ccc7a0c2642c7
d7de34b17b4691aa77ab8c17afdd440b004bc87d5d9ab991a8296d032fadb867
dd7a477ace2a5e999b3c565c284487822c8184f37a7a52b318307507faec5ae5'

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
samples='See Chainguard-Dev/example for the grant.
moved to chainguard-project-theseus/challoc
chainguard-project-theseus/proved-port and chainguard-project-theseus/challoc
the chainguard-project-theseus/proved-port-internal fork
the chainguard-project-theseus/proved-port.internal mirror
the chainguard-project-theseus/proved-port..internal mirror
the corp-chainguard-project-theseus/proved-port mirror
see chainguard-project-theseus/proved-port./internal
chainguard-project-theseus:proved-port
wolfi-dev/proved-port#21
the CG-Example port
the apply step (a Chainguard-internal tool)
render it with codeowners-gate
ai-cicd/sandbox/docs/identities.md
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
chainguard-project-theseus/proved-port
the pcre2-capi crate of the PCRE2 port
the Expat port'

# shipped ROOT: the files under ROOT the kit ships, one a line. Every
# scan runs in a condition, where set -e is off, so a missing root or a
# failing find fails here rather than leaving that tree unscanned.
shipped() {
  for _shipped_d in "$1/kit/count" "$1/kit/make" "$1/kit/templates"; do
    [ -d "$_shipped_d" ] || fail "no directory $_shipped_d"
  done
  [ -f "$1/kit/CHANGELOG.md" ] || fail "no file $1/kit/CHANGELOG.md"
  find "$1/kit/count" "$1/kit/make" "$1/kit/templates" "$1/kit/CHANGELOG.md" \
    -path "$1/kit/count/target" -prune -o \
    -type f ! -path "$1/kit/templates/README.md" -print ||
    fail "find failed under $1"
}

# runs: the awk program that prints, for each line of each file named
# in the file list, every name-like run the line holds, as
# FILE<TAB>LINE<TAB>COLUMN<TAB>RUN, COLUMN being where the run's token
# starts (in bytes, from 1). A line is read in lower case as tokens, the
# runs of [a-z0-9_.-]; each token is split at its `-`, `.` and `_`
# into parts, and each run of one to three adjacent parts (no name the
# digests hold has more) is printed
# joined with `-`. So a name is found as a word, inside a path or a
# crate name (NAME-capi, NAME_capi, NAME.so), and in either separator
# form, but not inside a longer part (NAMEs, xNAME).
# shellcheck disable=SC2016 # $0 is awk's, not the shell's
runs='BEGIN {
  while ((getline f < list) > 0) {
    n = 0
    while ((r = (getline line < f)) > 0) {
      n++
      rest = tolower(line)
      col = 0
      while (match(rest, /[a-z0-9_.-]+/)) {
        col += RSTART
        tok = substr(rest, RSTART, RLENGTH)
        rest = substr(rest, RSTART + RLENGTH)
        np = 0
        nraw = split(tok, raw, /[-._]+/)
        for (k = 1; k <= nraw; k++) if (raw[k] != "") part[++np] = raw[k]
        for (i = 1; i <= np; i++) {
          s = part[i]
          print f "\t" n "\t" col "\t" s
          for (j = i + 1; j <= np && j < i + 3; j++) {
            s = s "-" part[j]
            print f "\t" n "\t" col "\t" s
          }
        }
        col += RLENGTH - 1
      }
    }
    if (r < 0) { print "cannot read " f > "/dev/stderr"; exit 1 }
    close(f)
  }
}'

# named DIGESTS: append to $tmp/hits each line of the files in
# $tmp/files that holds a run (runs, above) whose sha256 is in the
# newline-separated DIGESTS, as FILE:LINE:(names a private repository
# in the word at column N; digest match). The name itself is not
# printed; the column shows the contributor which word to reword. Each distinct run is
# hashed once per script run, and kept in $tmp/cache as HASH<TAB>RUN:
# the runs not yet there are written to a file each and hashed by one
# find -exec.
named() {
  awk -v list="$tmp/files" "$runs" > "$tmp/runs" || fail "awk failed reading the shipped files"
  [ -f "$tmp/cache" ] || : > "$tmp/cache"
  rm -rf "$tmp/h"
  mkdir "$tmp/h"
  cut -f4 "$tmp/runs" | sort -u |
    awk -v cache="$tmp/cache" -v d="$tmp/h" '
      BEGIN { while ((getline l < cache) > 0) { sub(/^[^\t]*\t/, "", l); have[l] = 1 } }
      !($0 in have) { f = d "/" NR; printf "%s", $0 > f; close(f); print NR "\t" $0 }
    ' > "$tmp/runmap" || fail "could not write the runs to hash"
  if [ -s "$tmp/runmap" ]; then
    find "$tmp/h" -type f -exec sha256sum {} + > "$tmp/sums" || fail "sha256sum failed"
    awk -v map="$tmp/runmap" '
      BEGIN { while ((getline l < map) > 0) { i = index(l, "\t"); run[substr(l, 1, i - 1)] = substr(l, i + 1) } }
      { k = $2; sub(/.*\//, "", k); if (!(k in run)) exit 1; print $1 "\t" run[k] }
    ' "$tmp/sums" >> "$tmp/cache" || fail "a hashed file is not a run"
  fi
  printf '%s\n' "$1" > "$tmp/want"
  awk -v want="$tmp/want" -v cache="$tmp/cache" '
    BEGIN {
      while ((getline l < want) > 0) w[l] = 1
      while ((getline l < cache) > 0) {
        i = index(l, "\t")
        if (substr(l, 1, i - 1) in w) bad[substr(l, i + 1)] = 1
      }
      FS = "\t"
    }
    $4 in bad { print $1 ":" $2 ":(names a private repository in the word at column " $3 "; digest match)" }
  ' "$tmp/runs" | sort -u >> "$tmp/hits"
}

# used DIGESTS: succeed when the last named() matched against exactly
# the newline-separated DIGESTS. Each real pass is followed by
# used "$digests", so it cannot run with the digests dropped or emptied
# and still print its ok line.
used() {
  printf '%s\n' "$1" | cmp -s - "$tmp/want" 2> /dev/null
}

# scan ROOT DIGESTS: succeed when no shipped file under ROOT names a
# marker or a name whose digest is in the newline-separated DIGESTS;
# otherwise fail with each line that does in $tmp/hits, as
# FILE:LINE:TEXT with the exempt name deleted from TEXT (for a digest,
# FILE:LINE and a note in place of TEXT). An empty DIGESTS skips the
# digest scan: the marker self-tests, which plant no digest's name,
# pass "" to stay fast. The real passes name $digests where they call.
scan() {
  rm -f "$tmp/want"
  : > "$tmp/hits"
  shipped "$1" > "$tmp/files"
  [ -s "$tmp/files" ] || fail "no shipped files under $1"
  while IFS= read -r _scan_f; do
    awk "$exempt" "$_scan_f" > "$tmp/scrubbed" || fail "awk failed on $_scan_f"
    grep -niE -e "$markers" "$tmp/scrubbed" > "$tmp/hit" || [ $? -eq 1 ] || fail "grep failed on $_scan_f"
    awk -v f="$_scan_f" '{ print f ":" $0 }' "$tmp/hit" >> "$tmp/hits"
  done < "$tmp/files"
  [ -z "$2" ] || named "$2"
  [ ! -s "$tmp/hits" ]
}

if ! scan "$repo" "$digests"; then
  cat "$tmp/hits" >&2
  fail "the files above ship to ports and name something internal; reword them (kit/internal-test.sh)"
fi
used "$digests" || fail "the pass above did not match against the digests"
echo "ok no internal references in $(wc -l < "$tmp/files" | tr -d ' ') shipped files"

# CONTRIBUTING.md's rule covers the whole tree, so every file git
# tracks, shipped or not, is held to the digests (not to the markers,
# which the kit's own docs may name: the review bot, a person).
# tree ROOT DIGESTS: succeed when no file git tracks under ROOT holds a
# run whose digest is in the newline-separated DIGESTS; otherwise fail
# with the hits in $tmp/hits.
tree() {
  git -C "$1" ls-files > "$tmp/tracked" || fail "git ls-files failed in $1"
  awk -v r="$1" '{ print r "/" $0 }' "$tmp/tracked" > "$tmp/files"
  [ "$(wc -l < "$tmp/files")" -gt "$(shipped "$1" | wc -l)" ] || fail "git tracks no more files under $1 than the kit ships"
  : > "$tmp/hits"
  named "$2"
  [ ! -s "$tmp/hits" ]
}
if ! tree "$repo" "$digests"; then
  cat "$tmp/hits" >&2
  fail "the files above name a private repository; reword them (CONTRIBUTING.md)"
fi
used "$digests" || fail "the pass above did not match against the digests"
echo "ok no private repository's name in any of $(wc -l < "$tmp/files" | tr -d ' ') tracked files"

# The mutants run on a scratch root holding one shipped file from each
# of kit/count, kit/make and kit/templates, kit/CHANGELOG.md, the
# templates' README and a build output.
copy=$tmp/copy
in_count=kit/count/README.md
in_make=kit/make/headers.sh
in_templates=kit/templates/.chainguard/reviewers.yaml
in_changelog=kit/CHANGELOG.md
for f in $in_count $in_make $in_templates $in_changelog kit/templates/README.md; do
  mkdir -p "$copy/$(dirname "$f")"
  cp "$repo/$f" "$copy/$f"
done
mkdir -p "$copy/kit/count/target"
echo built > "$copy/kit/count/target/out"
scan "$copy" "$digests" || { cat "$tmp/hits" >&2; fail "the scratch root has a marker before any is planted"; }

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
  for f in $in_count $in_make $in_templates $in_changelog; do
    plant "$f" "$s"
    scan "$copy" "" && fail "the scan misses \"$s\" in $f"
    grep -q "^$copy/$f:" "$tmp/hits" || fail "the scan names another file than $f for \"$s\""
    unplant "$f"
  done
  for f in kit/templates/README.md kit/count/target/out; do
    plant "$f" "$s"
    scan "$copy" "" || fail "the scan holds $f, which no port gets, to \"$s\""
    unplant "$f"
  done
  n=$((n + 1))
done <<EOF
$samples
EOF
[ "$n" -ge 27 ] || fail "only $n samples ran"
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
  for f in $in_count $in_make $in_templates $in_changelog; do
    plant "$f" "$s"
    scan "$copy" "" || { cat "$tmp/hits" >&2; fail "the scan names \"$s\" in $f, which names nothing internal"; }
    unplant "$f"
  done
done <<EOF
$clean
EOF
echo "ok the word markers pass words that only start with them, and the exemption passes the kit's own repository by its exact name"

# The digests: each is 64 lower-case hex digits, and the list is sorted
# with no repeats, so a pasted name or a stray line fails here.
# shape LIST: succeed when the newline-separated LIST is such a list;
# otherwise print why and fail.
shape() {
  printf '%s\n' "$1" > "$tmp/digests"
  if grep -qvE '^[0-9a-f]{64}$' "$tmp/digests"; then
    echo "a digest is not 64 lower-case hex digits (line $(grep -nvE '^[0-9a-f]{64}$' "$tmp/digests" | head -1 | cut -d: -f1))"
    return 1
  fi
  sort -u "$tmp/digests" | cmp -s - "$tmp/digests" || { echo "the digests are not sorted with no repeats"; return 1; }
}
why=$(shape "$digests") || fail "$why"
d=$(printf '%s\n' "$digests" | wc -l | tr -d ' ')
[ "$d" -ge 7 ] || fail "only $d digests"
a=$(printf '%064d' 1)
b=$(printf '%064d' 2)
for bad in "$a
${b}0" "$a
$(printf '%063d' 2)" "$a
$(printf '%063dA' 0)" "$a
name" "$a

$b" "$b
$a" "$a
$a"; do
  shape "$bad" > /dev/null && fail "the digest shape check passes a bad list: $(printf '%s' "$bad" | tr '\n' ' ')"
done
shape "$a
$b" > /dev/null || fail "the digest shape check fails a good list"
echo "ok the $d digests are sorted sha256 hex, and the shape check fails each of 7 bad lists"

# The digest scan, tested with the digest of a neutral name in place of
# a private one: zz-private-example. Planted in each form a name takes
# (a word, a crate or library name, either separator, any case) in a
# shipped file, it is named by file and line and the name is not
# printed; planted in what no port gets, it is not named; and runs that
# only hold it inside a longer part, or split across tokens, pass.
test_name=zz-private-example
test_digest=$(printf '%s' "$test_name" | sha256sum | cut -c1-64)
with_test="$digests
$test_digest"
scan "$copy" "$with_test" || { cat "$tmp/hits" >&2; fail "the scratch root holds a digest's name before any is planted"; }
# Each sample is COLUMN TEXT: where the word holding the name starts.
named_samples='5 the Zz-Private-Example port
8 crates/zz_private_example-capi/src/lib.rs
1 zz.private.example.so.1
6 see  ZZ-PRIVATE-EXAMPLE#12
4 an Old-ZZ-private-example-Fork'
named_clean='the zz-private-examples port
the xzz-private-example port
zz private example
zz-private and example'
k=0
while read -r c s; do
  for f in $in_count $in_make $in_templates $in_changelog; do
    plant "$f" "$s"
    scan "$copy" "$with_test" && fail "the digest scan misses \"$s\" in $f"
    want="$copy/$f:$(wc -l < "$copy/$f" | tr -d ' '):(names a private repository in the word at column $c; digest match)"
    [ "$(cat "$tmp/hits")" = "$want" ] ||
      { cat "$tmp/hits" >&2; fail "the digest scan does not name $f by line and column $c (alone) for \"$s\""; }
    unplant "$f"
  done
  for f in kit/templates/README.md kit/count/target/out; do
    plant "$f" "$s"
    scan "$copy" "$with_test" || fail "the digest scan holds $f, which no port gets, to \"$s\""
    unplant "$f"
  done
  k=$((k + 1))
done <<EOF
$named_samples
EOF
[ "$k" -ge 5 ] || fail "only $k digest samples ran"
while IFS= read -r s; do
  for f in $in_count $in_make $in_templates $in_changelog; do
    plant "$f" "$s"
    scan "$copy" "$with_test" || { cat "$tmp/hits" >&2; fail "the digest scan names \"$s\" in $f, which holds no whole name"; }
    unplant "$f"
  done
done <<EOF
$named_clean
EOF
plant "$in_make" "the Zz-Private-Example port"
scan "$copy" "$digests" || fail "the digest scan names a name whose digest is not listed"
used "$digests" || fail "used fails the digests the scan matched against"
used "$with_test" && fail "used passes digests the scan did not match against"
scan "$copy" "" || fail "the scan with no digests names something"
used "$digests" && fail "used passes after a scan that matched against no digests"
unplant "$in_make"
echo "ok the digest scan names each of $k planted forms by file, line and column, not by name, and passes longer parts and split tokens"

# The whole-tree pass, on the scratch root as a git repository: a
# tracked file no port gets is held to the digests too; a file git does
# not track is not.
git -C "$copy" init -q || fail "git init failed in the scratch root"
git -C "$copy" add -A || fail "git add failed in the scratch root"
tree "$copy" "$with_test" || { cat "$tmp/hits" >&2; fail "the whole-tree pass names something before any is planted"; }
plant kit/templates/README.md "the Zz-Private-Example port"
tree "$copy" "$with_test" && fail "the whole-tree pass misses a name in kit/templates/README.md, which git tracks"
grep -q "^$copy/kit/templates/README.md:[0-9]*:(names a private repository in the word at column 5; digest match)\$" "$tmp/hits" ||
  { cat "$tmp/hits" >&2; fail "the whole-tree pass does not name kit/templates/README.md"; }
unplant kit/templates/README.md
echo "the Zz-Private-Example port" > "$copy/untracked.txt"
tree "$copy" "$with_test" || { cat "$tmp/hits" >&2; fail "the whole-tree pass names a file git does not track"; }
echo "ok the whole-tree pass names a tracked file no port gets, and passes an untracked one"
