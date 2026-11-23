#!/bin/sh
# header-check.sh UPSTREAM PREFIXES HEADER...: every macro definition
# whose name starts with one of PREFIXES in each of the port's public
# headers HEADER is the C library's own (`make header-check`,
# proved-port.mk), which the theseus-proved-port package installs in
# /usr/share/proved-port/make/. The PCRE2 port's ci/header-check.sh,
# with its header names, upstream directory and prefixes as arguments.
#
# A port's public headers are copies of the C library's with the port's
# own additions; an edit that changes an ABI constant (a flag's value,
# an error code) compiles cleanly against the port and breaks every
# consumer built against the C library. For each HEADER (include/foo.h)
# this compares its `#define NAME ...` and `#undef NAME` lines, for NAME
# starting with a prefix of PREFIXES (one argument, the prefixes
# separated by blanks: "FOO_ REG_"), with those of UPSTREAM/<HEADER's
# file name> (UPSTREAM is the C library's installed include directory,
# /usr/include from its -dev package). A line ending in a backslash is
# joined with the next as C splices it (blanks or a CR after the
# backslash, as in a CRLF header, still continue it, and the backslash
# and line end leave nothing in their place), so a definition's whole
# body is compared; each comment becomes a blank, as in C (`/* ... */`,
# across lines too, and `// ...`), but not a comment marker inside a
# string or character literal ("/*old"); the directive is read as
# `#define` or `#undef` however it is spaced (`#  define`), and blanks
# outside the literals are squeezed (inside them each byte is kept);
# values, suffixes and parentheses must match exactly. The lines are compared
# as a sorted set, so order does not matter, and so neither does the
# `#if`/`#ifdef` branch a line sits in: a header that swaps the values
# of two branches passes (a limit; closing it needs the preprocessor).
# The port's own macros under another prefix are not compared. For each
# header it prints `ok   NAME: N definitions match UPSTREAM/NAME`, or
# the unified diff (upstream's lines first) and `FAIL NAME differs from
# UPSTREAM/NAME`.
#
# Unlike the PCRE2 port's script, a header with no definition under PREFIXES on
# either side fails (a prefix that matches nothing compares nothing),
# and so does a prefix with no definition in any header of the run
# (`FAIL: no definition under P in any header`), a
# missing HEADER of the port's fails with a message, `# define` and
# `#define` are the same line (that script kept the blank after `#`),
# a continued definition's later lines and the text between two
# comments on a line are compared (that script read a definition's
# first line alone and dropped everything from the first `/*` to the
# last `*/`), comments are read as C reads them (across lines, `//`,
# not inside a literal), and `#undef` lines are compared too.
#
# Exit 0: every header matches; 1: a header differs, has no definition
# under PREFIXES, or is missing (ours or upstream's, which stops the
# run); 2: usage.
set -eu
usage() {
  echo "usage: header-check.sh UPSTREAM PREFIXES HEADER...${1:+: $1}" >&2
  exit 2
}
[ $# -ge 3 ] || usage
up=$1
prefixes=$2
shift 2
LC_ALL=C
export LC_ALL
alt=
for p in $prefixes; do
  printf '%s\n' "$p" | grep -Eqx '[A-Za-z_][A-Za-z0-9_]*' \
    || usage "PREFIXES holds $p, not a C identifier prefix"
  alt=${alt:+$alt|}$p
done
[ -n "$alt" ] || usage "PREFIXES is empty"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
# defs FILE: FILE's definitions under PREFIXES, normalized and sorted.
# grep's own status is kept: in the pipeline its read error would be
# lost (1, no match, is no error).
defs() {
  # A line ending in a backslash continues on the next: joined first, so
  # a definition's whole body is compared. As C reads it: blanks and a
  # CR after the backslash still end the line there (CRLF headers, a
  # stray blank; GCC and Clang splice those too), so each line's
  # trailing blanks and CR go first, and the backslash and line end go
  # with nothing in their place (`"abc\` then `def"` is `"abcdef"`; a
  # blank before the backslash stays).
  rc=0
  sed -e 's/[[:space:]]*$//' -e ':a' -e '/\\$/{N;s/\\\n//;s/[[:space:]]*$//;ba' -e '}' "$1" > "$tmp/joined" || rc=$?
  [ "$rc" -eq 0 ] || { echo "FAIL: cannot read $1 (sed exit $rc)" >&2; exit 1; }
  # Each comment becomes a blank, as in C: a `/* */` one (across lines
  # too, its lines kept as lines) and a `//` one, but not a comment
  # marker inside a string or character literal ("/*old"). Outside the
  # literals each run of blanks (and comments) becomes one blank; inside
  # them every byte is kept (`"a  b"` is not `"a b"`).
  rc=0
  awk '
    function blank() { if (!lb) { o = o " "; lb = 1 } }
    {
      s = $0; o = ""; n = length(s); i = 1; lb = 0
      while (i <= n) {
        c = substr(s, i, 1); d = substr(s, i, 2)
        if (incom) {
          if (d == "*/") { incom = 0; blank(); i += 2 } else i++
          continue
        }
        if (q != "") {
          o = o c
          if (c == "\\" && i < n) { o = o substr(s, i + 1, 1); i += 2; continue }
          if (c == q) q = ""
          i++
          continue
        }
        if (d == "/*") { incom = 1; i += 2; continue }
        if (d == "//") break
        if (c == " " || c == "\t" || c == "\r" || c == "\f" || c == "\v") { blank(); i++; continue }
        if (c == "\"" || c == "'"'"'") q = c
        o = o c
        lb = 0
        i++
      }
      q = ""
      print o
    }' "$tmp/joined" > "$tmp/bare" || rc=$?
  [ "$rc" -eq 0 ] || { echo "FAIL: cannot read $1 (awk exit $rc)" >&2; exit 1; }
  # The definitions under PREFIXES, and each `#undef` of such a name.
  rc=0
  grep -E "^[[:space:]]*#[[:space:]]*(define|undef)[[:space:]]+($alt)" "$tmp/bare" > "$tmp/raw" || rc=$?
  [ "$rc" -le 1 ] || { echo "FAIL: cannot read $1 (grep exit $rc)" >&2; exit 1; }
  sed -E -e 's/^[[:space:]]*#[[:space:]]*(define|undef)/#\1/' \
    -e 's/ $//' "$tmp/raw" | sort
}
fail=0
: > "$tmp/all"
for ours in "$@"; do
  h=${ours##*/}
  if [ ! -f "$up/$h" ]; then
    echo "FAIL: $up/$h not found (install the C library's -dev package)"
    exit 1
  fi
  [ -f "$ours" ] || { echo "FAIL: $ours not found"; exit 1; }
  defs "$ours" > "$tmp/ours"
  defs "$up/$h" > "$tmp/upstream"
  cat "$tmp/ours" "$tmp/upstream" >> "$tmp/all"
  n=$(wc -l < "$tmp/ours" | tr -d ' ')
  if [ ! -s "$tmp/ours" ] && [ ! -s "$tmp/upstream" ]; then
    echo "FAIL $h: no definition under $prefixes in $ours or $up/$h"
    fail=1
  elif diff -u "$tmp/upstream" "$tmp/ours"; then
    echo "ok   $h: $n definitions match $up/$h"
  else
    echo "FAIL $h differs from $up/$h"
    fail=1
  fi
done
# Each prefix must name a definition somewhere in the run (ours or
# upstream's, in any header): one that matches nothing (REGX_ for
# REG_, beside a FOO_ that matches) compares nothing. A prefix is a C
# identifier (checked above), so it reads as itself in the pattern.
for p in $prefixes; do
  if ! grep -Eq "^#(define|undef) $p" "$tmp/all"; then
    echo "FAIL: no definition under $p in any header"
    fail=1
  fi
done
exit $fail
