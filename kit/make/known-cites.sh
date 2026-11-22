#!/bin/sh
# known-cites.sh FILE LEDGER REPO: every row of a known-divergence table
# cites an active ABI ledger entry or an open issue (`make known-cites`,
# proved-port.mk), which the theseus-proved-port package installs in
# /usr/share/proved-port/make/. The citation check of the PCRE2 port's
# differential fuzzer (its fuzz-diff script's --check-known), in POSIX
# sh: a known divergence class excuses a difference between the port and
# the C library only while the difference is deliberate (an active
# entry) or tracked (an open issue).
#
# FILE is the table's source (the PCRE2 port's fuzz/diff/known.c). A row
# is a line whose first non-blank is `KNOWN("<cite>"` (KNOWN_CITES_MACRO
# in the environment names another macro, `known!` in Rust say), the
# cite one string literal followed, past blanks, by `,` or `)` (C joins
# a literal after it, `"ABI-001" "0"` is ABI-0010, so that fails). Rows
# are read line by line, so this reader fails what it cannot read with
# certainty rather than go past it: a line starting with the macro in
# any other form (`KNOWN(cite`, `KNOWN (`, `KNOWN /* c */ (`, `KNOWN`
# alone or before a `\` with the rest on later lines, `known![...]`),
# and any other use of the macro on a line (a second row, a row after
# other text or behind a comment opener). A use is the macro's name as
# a whole word followed, past blanks, by `(`, or by what may come
# before the C preprocessor takes the `(`: the line's end, a `\` or a
# comment opener. For a Rust macro (`known!`) it is the name without
# its `!` as a whole word, then past blanks the `!`, then past blanks
# `(`, `[`, `{`, a comment opener or the line's end (`known !(` and
# `known! [` are uses; rustfmt writes `known!(`). A name split from its
# `!` across lines is not read as one. A carriage return (CRLF line
# ends) is a blank, as compilers read it. The one use allowed
# besides a row is the name a `#define` defines or an `#undef`
# undefines; any other whole-word use on such a line (an alias,
# `#define K KNOWN`, whose rows this would not read) fails, as does a
# Rust rename of the macro (its bare name, then `as`: `use known as
# k`). So does a
# FILE with no row. The name before other text (`KNOWN with its cite`
# in a comment) is no use. A row inside a block comment or `#if 0` that
# starts its line is read like any other: it must cite as one does.
#
# A cite is one of:
#
#   - `ABI-nnn`: an active entry of LEDGER, the port's ABI ledger, read
#     by ledger-active.sh beside this script (a directory of
#     `ABI-nnn.md` entry files, or one file of entries; its header says
#     what an entry is and what makes a ledger malformed: a malformed
#     ledger fails);
#   - `TODO(#n)`: an open issue of REPO (n with no leading zero).
#
# Anything else fails. REPO is `owner/name`, the repository the port's
# issues are in, then any names it had before it moved (blank-separated
# in the one argument), which older descriptions and commits use. A
# TODO(#n) fails when the issue is closed:
#
#   - by its live state, `gh issue view n -R owner/name`, where gh
#     reaches the repository (`gh repo view owner/name` answers, asked
#     once): there an issue gh cannot view (no such issue) fails too.
#     Where gh is missing or cannot reach it, the run fails, naming the
#     issues it did not check: only the live state catches a row added
#     after its issue closed, or an issue closed without a keyword. The
#     port's check needs gh with a credential for the repository.
#     KNOWN_CITES_OFFLINE=1 in the environment (a local run without
#     one) makes that a NOTE, and the PASS line then says each such
#     issue was not found closed rather than open;
#   - when the pull request's description, KNOWN_CITES_PR_BODY in the
#     environment where the port's check binds it (set, even empty,
#     means bound), closes #n with one of GitHub's closing keywords
#     (`Fixes #n`, `closes owner/name#n`, `Resolves:
#     https://github.com/owner/name/issues/n`, any case): a fix deletes
#     its row in the same change;
#   - when a commit since the one from which FILE has held a row of
#     the cite without a gap (rows, not mentions in comments) closes
#     #n so: a fix that landed
#     while the row's change was in review. This needs FILE in a full
#     git clone; in a shallow one, or without git, it is skipped and a
#     NOTE says so. A row not yet committed (a local run on an edited
#     table) has no such commit, and nothing is checked for it here.
#
# Exit 0: every row cites an active entry or an issue not found closed;
# 1: a row does not, or FILE or LEDGER is missing, unreadable or
# malformed; 2: usage.
set -eu
usage() { echo "usage: known-cites.sh FILE LEDGER 'OWNER/NAME [OLD/NAME...]'" >&2; exit 2; }
[ $# -eq 3 ] || usage
file=$1
ledger=$2
repos=$3
macro=${KNOWN_CITES_MACRO:-KNOWN}
offline=${KNOWN_CITES_OFFLINE:-}
case $offline in
  ''|1) ;;
  *) echo "error: KNOWN_CITES_OFFLINE is '$offline': set it to 1 for a run without gh, or leave it empty" >&2; exit 2 ;;
esac
LC_ALL=C
export LC_ALL
case $macro in
  ''|[0-9]*|*[!A-Za-z0-9_!]*|*!?*) echo "error: KNOWN_CITES_MACRO '$macro' is not a macro name (KNOWN, or known! in Rust)" >&2; exit 2 ;;
esac
# Split on blanks with globbing off, so `o/*` stays itself (and fails
# below) rather than become the names of files.
set -f
# shellcheck disable=SC2086 # the repositories are blank-separated
set -- $repos
[ $# -gt 0 ] || { echo "error: no repository: name the port's issues' OWNER/NAME" >&2; exit 2; }
repo=$1
for r in "$@"; do
  case $r in
    */*/*|/*|*/|*[!A-Za-z0-9_./-]*) echo "error: '$r' is not an OWNER/NAME repository" >&2; exit 2 ;;
    */*) ;;
    *) echo "error: '$r' is not an OWNER/NAME repository" >&2; exit 2 ;;
  esac
done
[ -f "$file" ] || { echo "error: no table source $file"; exit 1; }
[ -e "$ledger" ] || { echo "error: no ABI ledger $ledger"; exit 1; }
# awk takes a relative operand with `=` in it as an assignment, not a
# file; a ./ prefix keeps it a file. Messages keep the name as given.
case $file in /*) file_f=$file ;; *) file_f=./$file ;; esac

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# The rows' cites, one per line as `LINE CITE`; a line this reader
# cannot read with certainty goes to bad.
FILE=$file MACRO=$macro BAD=$tmp/bad awk '
  BEGIN {
    m = ENVIRON["MACRO"]; n = length(m); bad = ENVIRON["BAD"]; printf "" > bad
    # A Rust macro (known!) is invoked as the name, then `!` (blanks may
    # come between), then a delimiter: the scan looks for the name.
    rust = substr(m, n, 1) == "!"
    b = rust ? substr(m, 1, n - 1) : m; nb = length(b)
  }
  # word_at(s, p): the name (b) stands at p of s as a whole word.
  function word_at(s, p) {
    return (p == 1 || substr(s, p - 1, 1) !~ /[A-Za-z0-9_]/) && substr(s, p + nb, 1) !~ /[A-Za-z0-9_]/
  }
  # use_at(s, p): the macro may be invoked at p of s: its name as a
  # whole word (not the end of UNKNOWN nor the start of KNOWN_CLASSES),
  # for a Rust macro then, past blanks, its `!`; then, past blanks, an
  # opening `(` ([ or { too for a Rust macro), or what may come before
  # the compiler takes the delimiter: the line'"'"'s end, a comment, or
  # for C a `\`.
  function use_at(s, p,   r) {
    if (p > 1 && substr(s, p - 1, 1) ~ /[A-Za-z0-9_]/) return 0
    r = substr(s, p + nb)
    sub(/^[[:space:]]*/, "", r)
    if (rust) {
      if (substr(r, 1, 1) != "!") return 0
      r = substr(r, 2); sub(/^[[:space:]]*/, "", r)
      return r == "" || r ~ /^([([{]|\/\*|\/\/)/
    }
    return r == "" || r ~ /^(\(|\\|\/\*|\/\/)/
  }
  {
    # A carriage return (a CRLF file) is a blank, as compilers read it.
    gsub(/\r/, " ")
    t = $0; sub(/^[[:space:]]*/, "", t)
    # The use the line may have: a row at its start, or the macro a
    # #define defines (or #undef undefines).
    own = 0
    if (substr(t, 1, nb) == b && use_at(t, 1)) {
      rest = substr(t, n + 1)
      # The cite is one string literal, then `,` or `)`: C joins a
      # literal after it ("ABI-001" "0" is ABI-0010), and a macro after
      # it can add more, so anything else is no row this can read.
      if (substr(rest, 1, 2) != "(\"" || (q = index(substr(rest, 3), "\"")) == 0 || substr(rest, q + 3) !~ /^[[:space:]]*[,)]/) {
        print ENVIRON["FILE"] ":" FNR ": a " m " line not of the form " m "(\"<cite>\", ...): " t > bad
        next
      }
      print FNR " " substr(rest, 3, q - 1)
      own = 1
    } else if (t ~ /^#[ \t]*(define|undef)[ \t]/) {
      d = t; sub(/^#[ \t]*(define|undef)[ \t]+/, "", d)
      if (substr(d, 1, n) == m) own = length(t) - length(d) + 1
    }
    # Any other use is a row this reader would not read (a second row
    # on the line, one after other text or in a comment, the cite on a
    # later line): it fails rather than go unchecked. On a #define or
    # #undef line, any other whole-word use fails: an alias
    # (`#define K KNOWN`) would hide rows written with it.
    pp = t ~ /^#[ \t]*(define|undef)[ \t]/
    off = 0
    while ((p = index(substr(t, off + 1), b)) > 0) {
      off += p
      if (off == own) continue
      # A Rust rename (`use known as k`, `{known as k}`) is an alias
      # too: rows written `k!(...)` would go unread.
      if (use_at(t, off) || (pp && word_at(t, off)) || (rust && word_at(t, off) && substr(t, off + nb) ~ /^[[:space:]]+as([^A-Za-z0-9_]|$)/)) {
        print ENVIRON["FILE"] ":" FNR ": " m " used other than as a row at its line'"'"'s start (one row per line): " t > bad
        next
      }
    }
  }' "$file_f" > "$tmp/rows" || { echo "error: cannot read $file"; exit 1; }
if [ -s "$tmp/bad" ]; then
  awk '{ print "error: " $0 }' "$tmp/bad"
  exit 1
fi
rows=$(wc -l < "$tmp/rows" | tr -d ' ')
[ "$rows" -gt 0 ] || { echo "error: $file has no $macro(\"<cite>\", ...) row"; exit 1; }

# The ledger's active entries.
here=$(dirname "$0")
if sh "$here/ledger-active.sh" "$ledger" > "$tmp/active"; then :; else
  rc=$?
  if [ -s "$tmp/active" ]; then cat "$tmp/active"; else echo "error: $here/ledger-active.sh failed on $ledger (exit $rc)"; fi
  [ "$rc" -ne 1 ] || echo "error: the ABI ledger $ledger is malformed; no cite was checked"
  exit 1
fi

# close_re N: an ERE for a line that closes issue N of one of the
# repositories with one of GitHub's closing keywords (grep -Ei): `#N`,
# `OWNER/NAME#N` or the issue's URL. Dots in a name match only dots.
close_re() {
  alts='#'
  for r in $repos; do
    e=$(printf '%s' "$r" | sed 's/[.]/[.]/g')
    alts="$alts|$e#|https://github[.]com/$e/issues/"
  done
  printf '%s' "(^|[^[:alnum:]_])(close[sd]?|fix(e[sd])?|resolve[sd]?):?[[:space:]]+($alts)$1([^0-9]|\$)"
}

body=
bound=no
if [ -n "${KNOWN_CITES_PR_BODY+set}" ]; then
  bound=yes
  body=$(printf '%s\n' "$KNOWN_CITES_PR_BODY" | tr -d '\r')
fi
# The history, where FILE is in a full clone with a commit. git runs in
# FILE's directory, on FILE's own name.
dir=$(dirname "$file")
base=$(basename "$file")
history=no
if command -v git > /dev/null 2>&1 \
  && git -C "$dir" -c safe.directory='*' rev-parse -q --verify HEAD > /dev/null 2>&1 \
  && [ "$(git -C "$dir" -c safe.directory='*' rev-parse --is-shallow-repository 2> /dev/null)" = false ]; then
  history=yes
fi
# gh answers for the repository: asked once, at the first TODO(#n)
# (unknown until then). Where it does, an issue it cannot view (no such
# issue: a typo, another repository's number) fails; where it does not
# (gh missing, no network, no credential), the issue is unverified.
gh_ok=unknown
command -v gh > /dev/null 2>&1 || gh_ok=no

bad=no
unverified=
while IFS= read -r rec; do
  # `LINE CITE`, the cite as written (blanks in it make it no cite).
  line=${rec%% *}
  cite=${rec#* }
  case $cite in
    ABI-*)
      case ${cite#ABI-} in
        ''|*[!0-9]*) echo "FAIL: $file:$line: $cite is not an ABI-nnn or TODO(#n) cite"; bad=yes; continue ;;
      esac
      if ! grep -qx -e "$cite" "$tmp/active"; then
        echo "FAIL: $file:$line: $cite is not an active entry of $ledger"
        bad=yes
      fi ;;
    'TODO(#'*')')
      n=${cite#'TODO(#'}
      n=${n%')'}
      # A leading zero (#012) is an issue gh reads as #12 that no
      # closing keyword names so: no cite.
      case $n in
        ''|0*|*[!0-9]*) echo "FAIL: $file:$line: $cite is not an ABI-nnn or TODO(#n) cite"; bad=yes; continue ;;
      esac
      re=$(close_re "$n")
      if [ "$gh_ok" = unknown ]; then
        if gh repo view "$repo" --json name > /dev/null 2>&1; then gh_ok=yes; else gh_ok=no; fi
      fi
      if [ "$gh_ok" = no ]; then
        case " $unverified " in *" #$n "*) ;; *) unverified="$unverified #$n" ;; esac
      elif state=$(gh issue view "$n" -R "$repo" --json state -q .state 2> "$tmp/gh-err"); then
        if [ "$state" != OPEN ]; then
          echo "FAIL: $file:$line: $cite: issue #$n of $repo is $state"
          bad=yes
        fi
      else
        echo "FAIL: $file:$line: $cite: gh reaches $repo but cannot view issue #$n: $(head -n 1 "$tmp/gh-err")"
        bad=yes
      fi
      if [ "$bound" = yes ] && printf '%s\n' "$body" | grep -Eiq -e "$re"; then
        echo "FAIL: $file:$line: $cite: this pull request's description closes #$n: delete the row (and its tests)"
        bad=yes
      fi
      if [ "$history" = yes ]; then
        # since: the commit from which FILE has held a row of the cite
        # without a gap (the newest commits that change FILE, back to
        # the last one before which it held none). A count of the
        # quoted cite would move with a second row or a comment's
        # quote; the rows' presence does not.
        since=
        commits=$(git -C "$dir" -c safe.directory='*' log --format=%H -- "$base") \
          || { echo "error: git log failed on $file"; exit 1; }
        for c in $commits; do
          # FILE at C: absent (it did not exist yet) ends the run; a
          # snapshot git cannot read is an error, not an absence.
          oid=$(git -C "$dir" -c safe.directory='*' rev-parse -q --verify "$c:./$base" 2> /dev/null) || break
          git -C "$dir" -c safe.directory='*' cat-file blob "$oid" > "$tmp/snap" \
            || { echo "error: git cannot read $file at $(printf '%.7s' "$c")"; exit 1; }
          # A row of the cite (the macro, then the quoted cite, at a
          # line's start), not any mention: a comment quoting it does
          # not hold the run.
          if M=$macro C=$cite awk '
            BEGIN { r = ENVIRON["M"] "(\"" ENVIRON["C"] "\"" }
            { t = $0; sub(/^[[:space:]]*/, "", t); if (substr(t, 1, length(r)) == r) { f = 1; exit } }
            END { exit !f }' "$tmp/snap"; then
            since=$c
          else
            break
          fi
        done
        if [ -n "$since" ]; then
          by=$(git -C "$dir" -c safe.directory='*' log -E -i --grep="$re" --format='%h %s' "$since..HEAD") \
            || { echo "error: git log failed in $dir"; exit 1; }
          by=$(printf '%s\n' "$by" | head -n 1)
          if [ -n "$by" ]; then
            echo "FAIL: $file:$line: $cite: commit $by closes #$n after $(printf '%.7s' "$since"), since which $file has cited it"
            bad=yes
          fi
        fi
      fi ;;
    *) echo "FAIL: $file:$line: $cite is not an ABI-nnn or TODO(#n) cite"; bad=yes ;;
  esac
done < "$tmp/rows"
# An issue whose live state was not checked fails, unless the caller
# says it runs without gh on purpose (KNOWN_CITES_OFFLINE=1, a local
# run): only the live state catches a row added after its issue closed,
# or an issue closed without a keyword.
if [ -n "$unverified" ] && [ "$offline" != 1 ]; then
  echo "FAIL: the live state of$unverified was not checked: gh is missing, or it cannot reach $repo; give the check gh with a credential for it, or set KNOWN_CITES_OFFLINE=1 for a run without"
  bad=yes
fi
if [ "$bad" = yes ]; then
  echo "FAIL: a row of $file cites nothing, an entry not active in $ledger, or a closed issue"
  exit 1
fi
[ "$bound" = yes ] || echo "NOTE: no pull request description bound (KNOWN_CITES_PR_BODY unset); its closing keywords were not checked"
[ "$history" = yes ] || echo "NOTE: $file is not in a full git clone; commits closing an issue were not checked"
if [ -n "$unverified" ]; then
  echo "NOTE: KNOWN_CITES_OFFLINE=1: the live state of$unverified was not checked"
  echo "PASS: $rows rows of $file, each citing an active entry of $ledger or an issue not found closed"
else
  echo "PASS: $rows rows of $file, each citing an active entry of $ledger or an open issue"
fi
