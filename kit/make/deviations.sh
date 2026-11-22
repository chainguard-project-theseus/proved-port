#!/bin/sh
# deviations.sh [--update] RESULTS LIST LEDGER: the deviations ratchet
# (`make deviations`, proved-port.mk), which the theseus-proved-port
# package installs in /usr/share/proved-port/make/. The per-block
# comparison of the PCRE2 port's conformance check, in POSIX sh: the
# port's run (upstream's test data through upstream's driver, split
# into blocks) stays in the port and writes RESULTS; this holds RESULTS
# to LIST.
#
# RESULTS has one line per key of the run, `PASS KEY` or `FAIL KEY`
# (the PCRE2 port: `FAIL testinput2:1234`, a pattern block at a line of
# a test input). A key is one word and appears once. A line of any
# other form, a blank line, a key twice, or no line at all fails: the
# run did not write what it should. Nothing is checked, or updated, then.
# That RESULTS holds the whole run is the port's to prove: this cannot
# tell a run cut short (its first block written, the rest lost) from a
# small one, so the port's run must fail on its own when it does not
# finish (check each step's status, no pipeline that drops it), and the
# port calls this only after it succeeded.
#
# LIST is the deviations list: `KEY ABI-nnn: reason` lines, one per
# failing key the port accepts, in any order. A line whose first
# non-blank is `#` is a comment; blank lines count for nothing. LEDGER
# is the port's ABI ledger, where each deliberate difference is an
# entry: a directory whose `ABI-*.md` files are the entries (the kit
# template's docs/abi/; its README.md is not read), or one file of
# entries (the PCRE2 port's docs/abi-changes.md). An entry starts at a
# `### ABI-nnn: summary` heading and ends at the next Markdown heading
# (an ATX `#` heading at most 3 blanks in, or a setext `===` or `---`
# underline directly under a paragraph line; a `---` after a blank
# line, or under a list item, a block quote or their lazy continuation
# lines, is a thematic break and ends nothing) or its file's end, and
# holds exactly one `- **Status:**` line; it is active when that line reads `- **Status:** active` (then nothing, or
# a blank, comma or parenthesis and more). Fenced code blocks (``` or
# ~~~) are quoted text: their headings and Status lines do not count.
# As in CommonMark, a fence closes only on a run of its opener's
# character at least as long, with nothing after it but blanks, and a
# backtick run with a backtick after it (inline code) opens none. One
# difference: a fence line counts at any indentation (CommonMark reads
# one indented 4 or more as indented code), so a fence inside a nested
# list item is still one. A Status line counts only at a line's start.
# A malformed ledger fails before anything is checked or updated: an
# entry with no Status line or more than one (an old line left behind),
# a Status line outside any entry (under another heading, or past a
# line read as ending the entry: a misread heading fails, whichever
# way), a line outside fences holding `**Status:**` in any other form
# (`* **Status:**`, indented, more blanks), a code fence that is never
# closed (it would hide every later line), an indented one (in a list
# item) still open at a non-blank line indented less than its opener
# (rendered, it ends with its item), an opening fence indented with a
# tab (rendered at a line's start, indented code), a closing fence indented otherwise
# than its opener (4 spaces more is code when rendered; a tab is
# columns this reader does not count), a line outside fences starting,
# at any indentation or after list or quote markers, with raw HTML
# (`<!--`, `<details>`, `</div>`...: rendered, an HTML block, whose
# lines are not Markdown; an autolink `<https://...>` is no HTML), a
# run of three backticks or tildes on a line that is not a fence line
# (after a list or quote marker it opens a fence when rendered; a
# recipe's fence starts on its own line), an ID with two entry headings (a copied heading, a reused ID), or, in the
# directory form, an `ABI-*.md` file whose name is not
# `ABI-<digits>.md` or that does not hold exactly one entry heading,
# its own ID's.
#
# Strict both ways (exit 1):
#
#   - a failing key that is not listed is a regression;
#   - a listed key that passes, or is not a key of this run, is stale;
#   - a listed key without a reason, listed twice, whose reason does not
#     start with `ABI-nnn:` or says nothing after it, or whose ABI-nnn
#     is not an active entry of LEDGER, is unexplained.
#
# --update first drops from LIST the listed keys that pass or are not
# keys of this run, keeping comments, blank lines and every other line
# as they are, then checks. It never adds a line, so a regression still
# fails.
#
# Each kind prints at most 50 lines. With DEVIATIONS_OUT set to a
# directory, the full lists are written there, one key per line, in
# RESULTS order (new-failures, fixed) or LIST order (stale,
# unexplained, the latter as `KEY: why`), so the port can show what a
# new failure's block printed (the PCRE2 port shows the first five).
#
# Exit 0: the failing keys are exactly the listed ones, each explained;
# 1: they are not, or RESULTS, LIST or LEDGER is missing, unreadable or
# malformed; 2: usage.
set -eu
usage() { echo "usage: deviations.sh [--update] RESULTS LIST LEDGER" >&2; exit 2; }
update=no
if [ "${1:-}" = --update ]; then update=yes; shift; fi
[ $# -eq 3 ] || usage
res=$1
list=$2
ledger=$3
LC_ALL=C
export LC_ALL
[ -f "$res" ] || { echo "error: no results file $res"; exit 1; }
[ -f "$list" ] || { echo "error: no deviations list $list"; exit 1; }
[ -e "$ledger" ] || { echo "error: no ABI ledger $ledger"; exit 1; }
# The names awk and the tools read: awk takes a relative operand with
# `=` in it (deviations=amd64.txt) as a variable assignment, not a file,
# and would read nothing; a ./ prefix keeps each a file. Messages keep
# the names as given.
op() { case $1 in /*) printf '%s' "$1" ;; *) printf './%s' "$1" ;; esac; }
res_f=$(op "$res")
list_f=$(op "$list")
ledger_f=$(op "$ledger")

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
out=${DEVIATIONS_OUT:-$tmp/out}
mkdir -p "$out" || { echo "error: cannot make $out"; exit 1; }
# Its files are awk operands too (show below): o=1/new-failures would
# be an assignment.
out=$(op "$out")

# RESULTS: every line is PASS or FAIL and one key, each key once.
RES=$res awk '
  BEGIN { r = ENVIRON["RES"] }
  NF == 2 && ($1 == "PASS" || $1 == "FAIL") {
    if ($2 in seen) { print "error: " r ":" FNR ": " $2 " appears twice (first at line " seen[$2] ")"; bad = 1; next }
    seen[$2] = FNR; next
  }
  NF == 0 { print "error: " r ":" FNR ": a blank line, not `PASS KEY` or `FAIL KEY`"; bad = 1; next }
  { print "error: " r ":" FNR ": not a `PASS KEY` or `FAIL KEY` line: " $0; bad = 1 }
  END {
    if (NR == 0) { print "error: " r " is empty: the run wrote no results"; bad = 1 }
    exit bad
  }' "$res_f" > "$tmp/malformed" || {
  rc=$?
  if [ -s "$tmp/malformed" ]; then head -50 "$tmp/malformed"; else echo "error: cannot read $res (awk exit $rc)"; fi
  exit 1
}

# The ledger's active entries, one ID per line.
if [ -d "$ledger" ]; then
  find "$ledger_f" -maxdepth 1 -name 'ABI-*.md' -type f > "$tmp/files" || { echo "error: cannot read $ledger"; exit 1; }
  sort "$tmp/files" > "$tmp/ledger-files"
else
  printf '%s\n' "$ledger_f" > "$tmp/ledger-files"
fi
: > "$tmp/active"
: > "$tmp/heads"
: > "$tmp/ledger-bad"
# One awk per file: an entry ends at its file's end. Any Markdown
# heading ends an entry (a `#include` line in a code sample does not).
# Each entry heading goes to heads, each active ID to active. In the
# directory form a file must be ABI-<digits>.md holding one entry
# heading, its own ID's.
while IFS= read -r f; do
  want=
  shown=$ledger
  if [ -d "$ledger" ]; then
    want=${f##*/}
    shown=$ledger/$want
    want=${want%.md}
  fi
  F=$shown WANT=$want HEADS=$tmp/heads BAD=$tmp/ledger-bad awk '
    # An entry ends: active only with exactly one Status line, and that
    # one active; any other count is malformed.
    function end_entry() {
      if (id != "") {
        if (ns != 1) print ENVIRON["F"] ": " id " has " ns " Status lines; want one" >> ENVIRON["BAD"]
        else if (act) print id
      }
      id = ""; ns = 0; act = 0
    }
    # fence_line(): is this line, past its leading blanks, a run of 3 or
    # more backticks or tildes? Sets fch (the character), flen (the
    # run) and frest (what follows it).
    function fence_line(   t) {
      t = $0; sub(/^[ \t]*/, "", t)
      fch = substr(t, 1, 1)
      if (fch != "`" && fch != "~") return 0
      flen = 0
      while (substr(t, flen + 1, 1) == fch) flen++
      frest = substr(t, flen + 1)
      return flen >= 3
    }
    # Fenced code (a patch recipe, the entry format) is quoted text:
    # neither its headings nor its Status lines count. As in CommonMark,
    # a fence closes only on a run of its opener'"'"'s character, at least
    # as long, with nothing after it but blanks, so a shorter or other
    # fence (or a ```markdown line) quoted inside it stays quoted.
    # An indented fence (in a list item) ends with its item when
    # rendered: a non-blank line indented less than its opener, while
    # it is open, is malformed (it would hide what follows), and
    # closes it here.
    fence && oi > 0 && $0 !~ /^[ \t]*$/ {
      match($0, /^[ \t]*/)
      if (RLENGTH < oi) {
        print ENVIRON["F"] ":" fline ": an indented code fence not closed before a line indented less" >> ENVIRON["BAD"]
        fence = 0
      }
    }
    fence_line() {
      # A backtick run with a backtick after it opens nothing: it is
      # inline code (```x``` at a line'"'"'s start), as in CommonMark.
      if (!fence) {
        if (fch == "`" && index(frest, "`")) { prev = !block; next }
        fence = 1; oc = fch; ol = flen; fline = FNR
        match($0, /^[ \t]*/); oi = RLENGTH; oindent = substr($0, 1, RLENGTH)
        # A tab before an opener reaches a column this reader does not
        # count (at a line'"'"'s start, column 4: indented code, no fence).
        if (index(oindent, "\t")) print ENVIRON["F"] ":" FNR ": a code fence indented with a tab" >> ENVIRON["BAD"]
        next
      }
      if (fch == oc && flen >= ol && frest ~ /^[ \t]*$/) {
        # A closer indented otherwise than its opener (4 spaces more is
        # code in CommonMark; a tab is columns this reader cannot
        # count) may not close what it renders as closing: malformed.
        match($0, /^[ \t]*/)
        if (substr($0, 1, RLENGTH) != oindent) print ENVIRON["F"] ":" FNR ": a closing code fence indented otherwise than its opener (line " fline ")" >> ENVIRON["BAD"]
        fence = 0; prev = 0; block = 0; next
      }
    }
    fence { next }
    # Every fence line has left above (an opener, a closer, inline
    # backticks at a line'"'"'s start). A run of three backticks or tildes
    # anywhere else (after a list or quote marker it opens a fence when
    # rendered) is one this reader would read past: malformed.
    /```|~~~/ { print ENVIRON["F"] ":" FNR ": a code fence run on a line that is not a fence line" >> ENVIRON["BAD"] }
    # Raw HTML at a line'"'"'s start opens an HTML block (a comment, a
    # <details>, a <div>...): rendered, its lines are not Markdown (a
    # comment hides them, a fence line in one opens nothing), which this
    # reader cannot follow, so the line is malformed. An autolink
    # (<https://...>) is no HTML.
    # At any indentation, as fence lines are read (in a list item an
    # HTML block is indented by its item), and after any list or quote
    # markers (`- <details>`, `1. <div>`, `> <p>`: an item'"'"'s content
    # can open with one).
    /^[ \t]*((([-*+]|[0-9]+[.)])[ \t]+|>[ \t]*))*<[A-Za-z\/!?]/ && !/^[ \t]*((([-*+]|[0-9]+[.)])[ \t]+|>[ \t]*))*<[A-Za-z][A-Za-z0-9+.-]*:/ {
      print ENVIRON["F"] ":" FNR ": raw HTML (rendered, its lines are not read as Markdown)" >> ENVIRON["BAD"]
    }
    # Headings, as CommonMark reads them: an ATX heading (`#` to
    # `######`... at most 3 blanks in), or a setext underline (a line of
    # `=` or `-` alone under a line of text: prev is 1 after a non-blank
    # line that is no heading or fence).
    /^( |  |   )?#+([ \t]|$)/ {
      end_entry()
      if ($1 == "###" && $2 ~ /^ABI-[0-9]+:$/) {
        id = substr($2, 1, length($2) - 1)
        print id >> ENVIRON["HEADS"]
        n++; if (n == 1) first = id
      }
      prev = 0; block = 0
      next
    }
    prev && /^( |  |   )?(=+|-+)[ \t]*$/ { end_entry(); prev = 0; next }
    # A Status line belongs to an entry. One outside any (under another
    # heading, or past a line read as ending the entry) is malformed, so
    # a heading misread either way fails rather than hide a Status.
    /^- \*\*Status:\*\*/ {
      if (id == "") print ENVIRON["F"] ":" FNR ": a Status line outside any ABI-nnn entry" >> ENVIRON["BAD"]
      ns++
      if ($0 ~ /^- \*\*Status:\*\* active([ ,(]|$)/) act = 1
    }
    # A Status field in any other form (`* **Status:**`, indented, more
    # blanks) would go uncounted: malformed rather than read past.
    /\*\*Status:\*\*/ && !/^- \*\*Status:\*\*/ {
      print ENVIRON["F"] ":" FNR ": a Status field not written `- **Status:**` at a line'"'"'s start" >> ENVIRON["BAD"]
    }
    # A setext underline counts only under a paragraph line: under a
    # list item or a block quote (and their lazy continuation lines, up
    # to a blank line) it is a thematic break. A line of text no blanks
    # in, outside such a block, starts a paragraph.
    /^[ \t]*$/ { block = 0; prev = 0; next }
    /^( |  |   )?([-*+]|[0-9]+[.)])([ \t]|$)/ || /^( |  |   )?>/ { block = 1; prev = 0; next }
    { prev = !block }
    END {
      end_entry()
      # A fence never closed hides every later line (an indented one in
      # a list item, which ends with its item when rendered, too).
      if (fence) print ENVIRON["F"] ":" fline ": a code fence that is never closed" >> ENVIRON["BAD"]
      w = ENVIRON["WANT"]
      if (w == "") exit
      if (w !~ /^ABI-[0-9]+$/) print ENVIRON["F"] ": not an ABI-<digits>.md entry file name" >> ENVIRON["BAD"]
      else if (n != 1 || first != w) print ENVIRON["F"] ": holds " n + 0 " entry headings" (n ? ", the first " first : "") "; want one, ### " w ":" >> ENVIRON["BAD"]
    }
  ' "$f" >> "$tmp/active" || { echo "error: cannot read $shown"; exit 1; }
done < "$tmp/ledger-files"
# An ID is one entry: two headings with one ID (a copied heading, a
# reused ID) leave which one the list cites unclear.
sort "$tmp/heads" > "$tmp/heads-sorted"
uniq -d "$tmp/heads-sorted" > "$tmp/dups"
while IFS= read -r d; do
  echo "$ledger: $d has more than one entry heading" >> "$tmp/ledger-bad"
done < "$tmp/dups"
if [ -s "$tmp/ledger-bad" ]; then
  awk '{ print "error: ledger: " $0 }' "$tmp/ledger-bad"
  echo "error: the ABI ledger $ledger is malformed; nothing was checked or updated"
  exit 1
fi

# Inputs read; only now does --update touch LIST.
if [ "$update" = yes ]; then
  # Keep comments, blank lines and the listed keys that still fail.
  awk 'FILENAME == ARGV[1] { if ($1 == "FAIL") failing[$2] = 1; next }
       NF == 0 || $1 ~ /^#/ || ($1 in failing) { print }' \
    "$res_f" "$list_f" > "$tmp/updated" || { echo "error: cannot read $list"; exit 1; }
  # cat, not mv: LIST keeps its mode and stays the file it was.
  cat "$tmp/updated" > "$list" || { echo "error: cannot write $list"; exit 1; }
  echo "updated $list"
fi

# The listed lines, comments and blank lines dropped.
awk 'NF > 0 && $1 !~ /^#/' "$list_f" > "$tmp/listed" || { echo "error: cannot read $list"; exit 1; }

# Compare: new failures and fixed keys in RESULTS order, stale ones in
# LIST order. Each list starts empty: DEVIATIONS_OUT may hold an
# earlier run's.
: > "$out/new-failures"
: > "$out/fixed"
: > "$out/stale"
# Paths through ENVIRON, not awk -v, which would read backslashes as
# escapes. FILENAME, not NR == FNR: the list may be empty.
OUT=$out awk '
  BEGIN { new = ENVIRON["OUT"] "/new-failures"; fixed = ENVIRON["OUT"] "/fixed"; stale = ENVIRON["OUT"] "/stale" }
  FILENAME == ARGV[1] { listed[$1] = 1; order[++n] = $1; next }
  $1 == "FAIL" && !($2 in listed) { print $2 > new }
  $1 == "PASS" && ($2 in listed) { print $2 > fixed }
  { seen[$2] = 1 }
  END {
    for (i = 1; i <= n; i++) if (!(order[i] in seen) && !(order[i] in done)) { print order[i] > stale; done[order[i]] = 1 }
  }' "$tmp/listed" "$res_f" || { echo "error: awk failed comparing $res with $list"; exit 1; }

# Every listed key needs a reason, listed once, that starts with an
# active entry of LEDGER (`ABI-nnn:`) and says something after it.
LEDGER=$ledger awk '
  FILENAME == ARGV[1] { active[$1] = 1; next }
  NF < 2 { print $1 ": no reason"; next }
  seen[$1]++ { print $1 ": listed twice"; next }
  $2 !~ /^ABI-[0-9]+:$/ { print $1 ": the reason does not start with ABI-nnn:"; next }
  NF < 3 { print $1 ": the reason says nothing after " $2; next }
  {
    id = $2; sub(/:$/, "", id)
    if (!(id in active)) print $1 ": " id " is not an active entry of " ENVIRON["LEDGER"]
  }' "$tmp/active" "$tmp/listed" > "$out/unexplained" \
  || { echo "error: awk failed on $list"; exit 1; }

total=$(wc -l < "$res" | tr -d ' ')
# Counted as the results check reads a line (blank- or tab-separated).
nfail=$(awk '$1 == "FAIL" { n++ } END { print n + 0 }' "$res_f") || { echo "error: cannot read $res"; exit 1; }
echo "total: $((total - nfail))/$total pass"

# show FILE BEFORE AFTER: FILE's first 50 lines as `FAIL: BEFORE<line>AFTER`,
# then a count of the rest. awk, not sed: LIST's name may hold any byte.
show() {
  BEFORE=$2 AFTER=$3 awk '
    NR <= 50 { print "FAIL: " ENVIRON["BEFORE"] $0 ENVIRON["AFTER"] }
    END { if (NR > 50) print "FAIL: ... and " NR - 50 " more" }' "$1" \
    || { echo "error: awk failed on $1"; exit 1; }
}
bad=no
show "$out/fixed" "listed in $list but passes: " ""
show "$out/stale" "listed in $list but not a key of this run: " ""
show "$out/unexplained" "unexplained deviation: " ""
nfixed=$(wc -l < "$out/fixed" | tr -d ' ')
nstale=$(wc -l < "$out/stale" | tr -d ' ')
if [ "$nfixed" != 0 ] || [ "$nstale" != 0 ]; then
  echo "FAIL: $nfixed listed keys pass and $nstale are not keys of this run;" \
    "run the ratchet with --update to drop them from $list"
  bad=yes
fi
[ ! -s "$out/unexplained" ] || bad=yes
nnew=$(wc -l < "$out/new-failures" | tr -d ' ')
if [ "$nnew" != 0 ]; then
  show "$out/new-failures" "" " fails and is not in $list"
  echo "FAIL: $nnew failing keys are not in $list (a regression: fix it, do not list it)"
  bad=yes
fi
if [ "$bad" = yes ]; then exit 1; fi
echo "PASS: the failing keys are exactly the $(wc -l < "$tmp/listed" | tr -d ' ') listed in $list, each with its reason"
