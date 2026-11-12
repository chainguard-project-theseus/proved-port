#!/bin/sh
# verus/make/headers.sh [PORT_ROOT]: hold a port's template-derived
# headers to the kit's (`make verus-headers`, verus/make/proved-port.mk).
# Vendored from wolfi-dev/proved-port's kit/make by kit/sync.sh, which
# copies beside it, under verus/make/templates/, each kit template of
# verus/FLOOR, RLIMIT, REACH and MINTS and of .chainguard/ that carries a
# `# kit: proved-port` line, as they are at the kit commit
# verus/count/KIT records.
#
# For each of those templates the port has (one it lacks is skipped and
# named), the header is the file's leading `#` comment lines up to its
# `# kit:` line. The port's header must hold the template's text, where
# each placeholder (`{NAME}`, `{NAME: guidance}`, `{Guidance}`) stands for
# any text the port fills in, and a placeholder that fills whole lines
# (a list item, a sentence of its own) may be deleted with its lines. The
# comparison ignores white space and the `#` that starts each line, so a
# filled placeholder may rewrap its paragraph. Comment lines the port
# adds after its `# kit:` line are its own.
#
# VERUS_FLOOR (from the environment, as proved-port.mk passes it; a path
# under PORT_ROOT, default verus/FLOOR) is the port's ratchet file, which
# the verus/FLOOR template's header holds (chainz: verus/SHARES).
#
# The port's `# kit: proved-port VERSION` line must name the commit
# verus/count/KIT records (in full or a prefix of 7 or more characters),
# or its tag: the headers are the kit's text at that version.
#
# Exit 0: every header matches; 1: one drifted or a file is missing; 2:
# usage.
set -eu

[ $# -le 1 ] || { echo "usage: $0 [PORT_ROOT]" >&2; exit 2; }
root=${1:-.}
floor=${VERUS_FLOOR:-verus/FLOOR}
templates=$root/verus/make/templates
kit=$root/verus/count/KIT
[ -d "$templates" ] || { echo "headers: no $templates; re-run kit/sync.sh" >&2; exit 1; }
[ -f "$kit" ] || { echo "headers: no $kit; re-run kit/sync.sh" >&2; exit 1; }
sha=$(sed -n 's/^commit //p' "$kit")
tag=$(sed -n 's/^tag //p' "$kit")
[ -n "$sha" ] || { echo "headers: $kit records no commit" >&2; exit 1; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# header FILE: FILE's leading `#` lines up to and including its first
# `# kit:` line; fails when the leading lines have none.
header() {
  awk '!/^#/ { exit } { print } /^# kit:/ { found = 1; exit } END { exit !found }' "$1"
}

# version LINE: the VERSION of a `# kit: proved-port VERSION` line.
version() {
  echo "$1" | sed -n 's/^# kit: proved-port \([^ ]*\) *$/\1/p'
}

# known VERSION: whether VERSION names the commit or tag KIT records.
known() {
  [ -n "$1" ] || return 1
  case $1 in
    "$sha" | "$tag") [ "$1" != none ] ;;
    *)
      [ ${#1} -ge 7 ] || return 1
      case $sha in "$1"*) return 0 ;; esac
      return 1
      ;;
  esac
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

find "$templates" -type f | LC_ALL=C sort > "$tmp/list"
[ -s "$tmp/list" ] || { echo "headers: $templates holds no templates; re-run kit/sync.sh" >&2; exit 1; }
drift=
while IFS= read -r t; do
  rel=${t#"$templates"/}
  # path: the port's file the template rel holds.
  path=$rel
  [ "$rel" != verus/FLOOR ] || path=$floor
  f=$root/$path
  if [ ! -f "$f" ]; then
    echo "skip $path: the port has none"
    continue
  fi
  header "$t" > "$tmp/t" || { echo "skip $path: the kit's template has no # kit: line"; continue; }
  if ! header "$f" > "$tmp/p"; then
    echo "headers: $path: its leading comment has no \`# kit: proved-port VERSION\` line (the kit's: verus/make/templates/$rel)" >&2
    drift=1
    continue
  fi
  line=$(tail -n 1 "$tmp/p")
  v=$(version "$line")
  if ! known "$v"; then
    echo "headers: $path: \`$line\` does not name the kit commit verus/count/KIT records ($sha, tag $tag)" >&2
    drift=1
  fi
  sed '$d' "$tmp/t" > "$tmp/t.body"
  sed '$d' "$tmp/p" > "$tmp/p.body"
  if ! missing=$(match "$tmp/t.body" "$tmp/p.body"); then
    echo "headers: $path: the header drifted from the kit's (verus/make/templates/$rel); it lacks: $missing" >&2
    drift=1
    continue
  fi
  echo "ok $path"
done < "$tmp/list"
if [ -n "$drift" ]; then
  echo "headers: restore the kit's text, or change kit/templates in wolfi-dev/proved-port and re-sync" >&2
  exit 1
fi
