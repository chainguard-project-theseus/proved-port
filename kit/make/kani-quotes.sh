#!/bin/sh
# kani-quotes.sh DIR ROOTS [SKIP]: every Kani record in DIR still quotes
# the source it names (`make kani-quotes`, proved-port.mk), which the
# theseus-proved-port package installs in /usr/share/proved-port/make/.
#
# A Kani record (kit/templates/verus/seams/kani-TEMPLATE.md) quotes, in
# its section 1, the contract its harnesses check: the doc comment, the
# signature with its clauses and the marker line, as the source had them
# when the record was written. A later change to the contract leaves the
# harness checking what the record says, not what Verus now assumes, and
# nothing else notices. For each `kani-*.md` record in DIR, in name order
# (but the ones SKIP names), this reads:
#
#   - its source: the first backquoted path ending in `.rs` in the text
#     before its first `## ` heading (`{FILE}` in the template's opening
#     line), looked up under each directory of ROOTS in turn (one
#     argument, blank-separated: `.` for the port's tree, then, say, a
#     vstd source tree for a record that quotes vstd's `std_specs/`);
#   - its quote: every fenced block (from a line opening with three
#     backquotes to the next) of the section whose heading opens `## 1.`,
#     cut into chunks at blank lines and at each block's end, since a
#     record may quote several items, in one block or one block each.
#
# Each line of the quote and of the source is read with its leading and
# trailing blanks dropped and each run of blanks made one; a line that is
# then empty, or that opens with `//` (a doc or plain comment, which
# states no contract), is dropped. Each chunk with a line left must
# appear in the source as consecutive lines, read the same way, and end
# where its item's text does: its last line ends with `{`, `;` or `}`,
# or the source's next line opens with `{` or `;` (the body, or the end
# of a specification), or the source ends. So reindenting, rewording a
# whole-line comment or moving an item passes, and a changed clause,
# signature, attribute or body line fails; so does a clause added after
# the quote's last one, and a comment at the end of a code line, which
# is read as part of it.
#
# It prints, per record, `ok   RECORD: N chunks match SOURCE` or a `FAIL
# RECORD: ...` line naming what is wrong: no source named, a source under
# no root, no section 1, no fenced block there or one left open, a
# quote of comments alone or of nothing, the first chunk that does not
# match (by its first line read as above), or the first that stops
# before its item does (by the source's next line). DIR with no
# `kani-*.md` record but those SKIP names fails too (a port that names
# the wrong directory checks nothing). SKIP is blank-separated record
# file names (`kani-s3.md`); a name in SKIP that is no record in DIR is
# a usage error. A record SKIP names is still read: if it fails, a
# `skip RECORD: ...` line says why and the run goes on; if it matches,
# the run fails (`take it out of SKIP`), so a stale entry does not hide
# the record's later drift.
#
# Exit 0: every record's quote matches; 1: a record fails, or DIR holds
# none; 2: usage.
set -eu
usage() {
  echo "usage: kani-quotes.sh DIR ROOTS [SKIP]${1:+: $1}" >&2
  exit 2
}
[ $# -eq 2 ] || [ $# -eq 3 ] || usage
dir=$1
roots=$2
skip=${3:-}
LC_ALL=C
export LC_ALL
[ -n "$(printf '%s' "$roots" | tr -d ' \t')" ] || usage "ROOTS is empty"
[ -d "$dir" ] || { echo "FAIL: $dir is not a directory"; exit 1; }
for s in $skip; do
  case $s in
    kani-*.md) ;;
    *) usage "SKIP names $s, which is not a kani-*.md name" ;;
  esac
  [ -f "$dir/$s" ] || usage "SKIP names $s, which is no record in $dir"
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# norm FILE: FILE's lines read as the header says, one per line; a blank
# line stands for each line that was empty before reading (a chunk
# break), and a line read away to nothing but for a comment is left out.
norm() {
  awk '{
    line = $0
    gsub(/[ \t]+/, " ", line)
    sub(/^ /, "", line)
    sub(/ $/, "", line)
    if (line == "") { print ""; next }
    if (substr(line, 1, 2) == "//") next
    print line
  }' "$1"
}

# bad WHAT, good WHAT: the record's verdict. A record SKIP names is read
# too: one that fails prints a `skip` line and leaves the status alone;
# one that matches fails the run, so a stale entry in SKIP does not hide
# the record's later drift, and SKIP only shrinks.
bad() {
  if [ "$skipped" -eq 1 ]; then
    echo "skip $name: $*"
  else
    echo "FAIL $name: $*"
    fail=1
  fi
}
good() {
  if [ "$skipped" -eq 1 ]; then
    echo "FAIL $name: SKIP names it, but $*: take it out of SKIP"
    fail=1
  else
    echo "ok   $name: $*"
  fi
}

fail=0
n=0
for r in "$dir"/kani-*.md; do
  [ -f "$r" ] || continue
  name=${r##*/}
  skipped=0
  for s in $skip; do
    [ "$s" = "$name" ] && skipped=1
  done
  [ "$skipped" -eq 1 ] || n=$((n + 1))
  # The source: the first backquoted .rs path before the first heading.
  src=$(awk '
    /^## / { exit }
    {
      line = $0
      while (match(line, /`[^` ]+`/)) {
        t = substr(line, RSTART + 1, RLENGTH - 2)
        line = substr(line, RSTART + RLENGTH)
        if (t ~ /\.rs$/) { print t; exit }
      }
    }' "$r")
  if [ -z "$src" ]; then
    bad "names no source (a backquoted .rs path before its first heading)"
    continue
  fi
  found=
  for root in $roots; do
    if [ -f "$root/$src" ]; then
      found=$root/$src
      break
    fi
  done
  if [ -z "$found" ]; then
    bad "its source $src is under no root of: $roots"
    continue
  fi
  # The quote: every fenced block of section 1 (a per-file record may
  # quote one block per function), a blank line after each so that no
  # chunk runs from one block into the next. The status says which part
  # is missing: 3 no section 1, 4 no fence, 5 a fence left open.
  rc=0
  awk '
    /^## / { if (s) exit; if (/^## 1\./) { s = 1; seen = 1 }; next }
    !s { next }
    /^```/ { if (inb) { inb = 0; print "" } else { inb = 1; nb++ }; next }
    inb { print }
    END { if (!seen) exit 3; if (!nb) exit 4; if (inb) exit 5 }' "$r" > "$tmp/quote" || rc=$?
  case $rc in
    0) ;;
    3) bad "has no section 1 (a heading opening \`## 1.\`)"; continue ;;
    4) bad "section 1 has no fenced block"; continue ;;
    5) bad "section 1's fenced block is not closed"; continue ;;
    *) bad "cannot read it (awk exit $rc)"; continue ;;
  esac
  norm "$tmp/quote" > "$tmp/q"
  norm "$found" | awk 'NF' > "$tmp/s"
  # Each chunk of q (runs of lines between blank ones) must be a run of
  # consecutive lines of s that ends where its item's text does: its
  # last line ends with `{`, `;` or `}`, or the source's next line opens
  # with `{` or `;`, or there is none. So a clause added after the
  # quote's last one (a new `ensures`) fails. Prints the chunk count;
  # or, for the first chunk that is no run of s, `miss K P LINE`: chunk
  # K, whose first P lines are the most of it any place in s holds, and
  # its next line; or, for the first whose every run goes on,
  # `short K NEXT`: the source's line after the first such run.
  out=$(awk '
    FILENAME == ARGV[1] {
      if ($0 == "") { if (len[c] > 0) c++; next }
      if (c == 0) c = 1
      q[c, ++len[c]] = $0
      next
    }
    { l[++m] = $0 }
    END {
      if (c > 0 && len[c] == 0) c--
      for (k = 1; k <= c; k++) {
        best = 0; ok = 0; next_line = ""
        for (i = 1; i <= m && !ok; i++) {
          j = 0
          while (j < len[k] && i + j <= m && l[i + j] == q[k, j + 1]) j++
          if (j > best) best = j
          if (j < len[k]) continue
          e = i + j
          if (q[k, j] ~ /[{;}]$/ || e > m || l[e] ~ /^[{;]/) ok = 1
          else if (next_line == "") next_line = l[e]
        }
        if (ok) continue
        if (best < len[k]) print "miss " k " " best " " q[k, best + 1]
        else print "short " k " " next_line
        exit
      }
      print c + 0
    }' "$tmp/q" "$tmp/s")
  case $out in
    0) bad "section 1 quotes no code (comments alone, or nothing)" ;;
    "miss "*)
      rest=${out#miss }
      k=${rest%% *}
      rest=${rest#* }
      p=${rest%% *}
      bad "chunk $k of section 1 is not in $found: its first $p lines are, then: ${rest#* }" ;;
    "short "*)
      rest=${out#short }
      bad "chunk ${rest%% *} of section 1 stops before its item does in $found, which goes on: ${rest#* }" ;;
    *) good "$out chunks match $found" ;;
  esac
done
if [ "$n" -eq 0 ]; then
  echo "FAIL: $dir holds no kani-*.md record${skip:+ but those SKIP names}"
  exit 1
fi
exit "$fail"
