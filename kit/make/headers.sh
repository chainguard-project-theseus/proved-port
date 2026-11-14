#!/bin/sh
# headers.sh [--installed TEMPLATES KIT FILES] [PORT_ROOT]: hold a port's
# copies of the kit's Verus files to the kit's text (`make
# verus-headers`, proved-port.mk). The theseus-proved-port package
# installs it in /usr/share/proved-port/make/, beside VERUS-FILES, with
# the kit's templates in /usr/share/proved-port/templates/ and KIT in
# /usr/share/proved-port/.
#
# TEMPLATES is the installed templates' directory, KIT the installed KIT
# and FILES the list of the kit's Verus files (VERUS-FILES). Without
# --installed they are PROVED_PORT_HOME/templates and PROVED_PORT_HOME/KIT
# (PROVED_PORT_HOME as proved-port.mk exports it; unset, the directory
# above this script's, where the package installs both) and the
# VERUS-FILES beside this script. Only these arguments, or the
# PROVED_PORT_HOME proved-port.mk gives, choose the paths. KIT must be
# there, and a drift message names the release it records. PORT_ROOT
# defaults to `.`; nothing under it but the Verus files themselves is
# read, so a verus/make or verus/count a port still has from an earlier,
# vendored release is not.
#
# The files held are the ones FILES names, each against the template of
# the same name: verus/FLOOR, RLIMIT, REACH, MINTS and count.toml,
# .chainguard/ci/verus.yaml, miri.yaml and .chainguard/reviewers.yaml,
# and .chainguard/review-prompt.md. The installed directory holds the
# kit's other templates too (conformance, consumers, fuzz, test, lint,
# the environments, develop.yaml, deps.yaml): starting points the port
# reshapes, which this does not hold.
#
# For each of those templates the port has (one it lacks is skipped and
# named), the header is the file's leading `#` comment lines, ending
# before a `# kit:` line where the file has one; that line is not read or
# checked, in the template or the port. The port's header must hold the
# template's text, where each placeholder (`{NAME}`, `{NAME: guidance}`,
# `{Guidance}`) stands for any text the port fills in, and a placeholder
# that fills whole lines (a list item, a sentence of its own) may be
# deleted with its lines. The comparison ignores white space and the `#`
# that starts each line, so a filled placeholder may rewrap its
# paragraph. The port's own comments go after its header, past a blank
# line.
#
# .chainguard/review-prompt.md is Markdown, with no `#` header: the port's
# prompt must hold, anywhere and ignoring white space, the template's
# vendored-kit rule, its `- The vendored kit` list item.
#
# VERUS_FLOOR (from the environment, as proved-port.mk passes it; a path
# under PORT_ROOT, default verus/FLOOR) is the port's ratchet file, which
# the verus/FLOOR template's header holds (a port with a verus/floor/
# directory names it otherwise, verus/SHARES say, as verus/FLOOR and
# verus/floor/ are one path on a case-insensitive file system).
#
# Exit 0: every header matches; 1: one drifted or a file is missing; 2:
# usage.
set -eu

usage="usage: $0 [--installed TEMPLATES KIT FILES] [PORT_ROOT]"
if [ "${1:-}" = --installed ]; then
  [ $# -ge 4 ] || { echo "$usage" >&2; exit 2; }
  templates=$2
  kit=$3
  files=$4
  shift 4
else
  here=$(cd "$(dirname "$0")" && pwd)
  home=${PROVED_PORT_HOME:-$(dirname "$here")}
  templates=$home/templates
  kit=$home/KIT
  files=$here/VERUS-FILES
fi
[ $# -le 1 ] || { echo "$usage" >&2; exit 2; }
root=${1:-.}
floor=${VERUS_FLOOR:-verus/FLOOR}
fix='reinstall the theseus-proved-port package'
[ -d "$templates" ] || { echo "headers: no $templates; $fix" >&2; exit 1; }
[ -f "$kit" ] || { echo "headers: no $kit; $fix" >&2; exit 1; }
[ -f "$files" ] || { echo "headers: no $files, the list of the kit's Verus files; $fix" >&2; exit 1; }
tag=$(sed -n 's/^tag //p' "$kit")

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# body FILE: FILE's leading `#` lines, up to but not including a
# `# kit:` line.
body() {
  awk '!/^#/ || /^# kit:/ { exit } { print }' "$1"
}

# kit_rule FILE: FILE's vendored-kit rule, the list item that starts
# `- The vendored kit`, up to the next item, heading or blank line.
kit_rule() {
  awk '
    on && (/^- / || /^#/ || /^[ \t]*$/) { exit }
    /^- The vendored kit/ { on = 1 }
    on { print }' "$1"
}

# match TEMPLATE PORT: whether the port header (PORT, its `# kit:` line
# left out) holds the template's text (TEMPLATE, likewise), each
# placeholder matching any text. On a mismatch it prints the first
# stretch of the template's text the port does not have.
match() {
  awk '
    FNR == NR { t[++nt] = $0; next }
    { p[++np] = $0 }
    # squeeze S: S without its white space.
    function squeeze(s) { gsub(/[ \t\n]+/, "", s); return s }
    # tidy S: S with each run of white space one space, for messages.
    function tidy(s) { gsub(/[ \t\n]+/, " ", s); sub(/^ /, "", s); sub(/ $/, "", s); return s }
    # lit(S): end the current literal stretch with S, start a placeholder.
    function lit(s) { n++; L[n] = s }
    END {
      # The template with each line'"'"'s leading # dropped, joined by
      # newlines.
      text = ""
      for (i = 1; i <= nt; i++) { s = t[i]; sub(/^#/, "", s); text = text s "\n" }
      ptext = ""
      for (i = 1; i <= np; i++) { s = p[i]; sub(/^#/, "", s); ptext = ptext s "\n" }
      # Split the template at its placeholders: a { followed by a
      # capital letter, up to the next }.
      n = 0; cur = ""; lead = 0; trail = 0
      while (match(text, /[{][A-Z]/)) {
        before = substr(text, 1, RSTART - 1)
        rest = substr(text, RSTART)
        shut = index(rest, "}")
        if (shut == 0) { cur = cur before; text = rest; break }
        after = substr(rest, shut + 1)
        # A placeholder that fills whole lines takes them with it: only
        # spaces or a list dash before it on its line, only spaces or a
        # stop after it on its line.
        k = 0
        for (j = length(before); j > 0; j--) if (substr(before, j, 1) == "\n") { k = j; break }
        head = substr(before, k + 1)
        nl = index(after, "\n")
        tail = nl ? substr(after, 1, nl - 1) : after
        if (head ~ /^[ -]*$/ && tail ~ /^[.,;]?[ ]*$/) {
          before = substr(before, 1, k)
          after = nl ? substr(after, nl + 1) : ""
        }
        cur = cur before
        if (n == 0 && squeeze(cur) == "") lead = 1
        else lit(cur)
        cur = ""
        text = after
      }
      cur = cur text
      if (squeeze(cur) == "") trail = 1
      else lit(cur)
      if (n > 0 && squeeze(L[n]) == "") n--
      P = squeeze(ptext)
      pos = 1
      for (i = 1; i <= n; i++) {
        s = squeeze(L[i])
        if (s == "") continue
        if (i == 1 && !lead) {
          if (substr(P, 1, length(s)) != s) { bad = i; break }
          pos = length(s) + 1
          if (i == n && !trail && length(P) != length(s)) { bad = i; break }
          continue
        }
        if (i == n && !trail) {
          at = length(P) - length(s) + 1
          if (at < pos || substr(P, at) != s) { bad = i; break }
          pos = length(P) + 1
          continue
        }
        at = index(substr(P, pos), s)
        if (at == 0) { bad = i; break }
        pos += at - 1 + length(s)
      }
      if (n == 0 && !lead && !trail && P != "") bad = 1
      if (bad) {
        m = tidy(L[bad])
        if (length(m) > 160) m = substr(m, 1, 160) "..."
        print m
        exit 1
      }
    }' "$1" "$2"
}


# Each listed template, which must be installed: one missing would leave
# its file unheld.
grep -v -e '^#' -e '^[[:space:]]*$' "$files" > "$tmp/files" || true
[ -s "$tmp/files" ] || { echo "headers: $files names no files; $fix" >&2; exit 1; }
while IFS= read -r rel; do
  if [ ! -f "$templates/$rel" ]; then
    echo "headers: $files names $rel, but $templates/$rel is not installed; $fix" >&2
    exit 1
  fi
done < "$tmp/files"
LC_ALL=C sort "$tmp/files" > "$tmp/list"
drift=
while IFS= read -r rel; do
  # t: the template; path: the port's file it holds.
  t=$templates/$rel
  path=$rel
  [ "$rel" != verus/FLOOR ] || path=$floor
  f=$root/$path
  if [ ! -f "$f" ]; then
    echo "skip $path: the port has none"
    continue
  fi
  if [ "$rel" = .chainguard/review-prompt.md ]; then
    # The rule, between placeholders that stand for the rest of the
    # port's prompt, so match finds it anywhere in the file.
    {
      echo '{The port text before it}'
      kit_rule "$t"
      echo '{The port text after it}'
    } > "$tmp/t.body"
    # With no rule between them, the two placeholders match any prompt:
    # fail rather than pass every port's.
    if [ "$(wc -l < "$tmp/t.body")" -le 2 ]; then
      echo "headers: $t has no \`- The vendored kit\` item; $fix" >&2
      drift=1
      continue
    fi
    if ! missing=$(match "$tmp/t.body" "$f"); then
      echo "headers: $path: the vendored-kit rule drifted from the kit's (the \`- The vendored kit\` item of $t); it lacks: $missing" >&2
      drift=1
      continue
    fi
    echo "ok $path"
    continue
  fi
  # A template with no leading comment would hold nothing, so it fails
  # rather than pass every port's copy.
  body "$t" > "$tmp/t.body"
  if [ ! -s "$tmp/t.body" ]; then
    echo "headers: $t has no leading comment, so it holds nothing; $fix" >&2
    drift=1
    continue
  fi
  body "$f" > "$tmp/p.body"
  if ! missing=$(match "$tmp/t.body" "$tmp/p.body"); then
    echo "headers: $path: the header drifted from the kit's ($t); it lacks: $missing" >&2
    drift=1
    continue
  fi
  echo "ok $path"
done < "$tmp/list"
if [ -n "$drift" ]; then
  echo "headers: the installed kit is ${tag:-untagged} ($kit)" >&2
  echo "headers: restore the kit's text, or change kit/templates in the proved-port kit and move the package pin to the release that carries it" >&2
  exit 1
fi
