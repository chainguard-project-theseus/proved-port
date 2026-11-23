#!/bin/sh
# abi-elf.sh SO MAP SONAME NEEDED [TLS_MAX [DEFAULTS]]: the shared library SO's
# dynamic ABI is the one its linker version script MAP declares (`make
# abi-elf`, proved-port.mk), which the theseus-proved-port package
# installs in /usr/share/proved-port/make/. The per-library ELF checks
# of the PCRE2 port's ci/abi.sh, reading the port's version script in
# place of the PCRE2 names written into that script.
#
# A drop-in replacement library must load where the C library did and
# bind every symbol a consumer was linked against, at the version it
# was linked against. This reads SO with binutils' readelf and checks:
#
#   1. its SONAME (`readelf -d`) is SONAME;
#   2. each library it needs (DT_NEEDED) is one of NEEDED (one argument,
#      the names separated by blanks: libc and the loader, so that the
#      port pulls in nothing a consumer of the C library would not have,
#      such as libgcc_s.so.1);
#   3. with TLS_MAX (bytes, digits), its thread-locals (PT_TLS's MemSiz,
#      `readelf -lW`) are at most TLS_MAX: glibc's optional static TLS
#      pool, 512 bytes by default (glibc.rtld.optional_static_tls), is
#      shared by every dlopened library, and one that does not fit takes
#      the slow dynamic TLS path on each access;
#   4. its version definitions (`.gnu.version_d`, but the BASE entry,
#      which names the file) are exactly MAP's nodes, each with the
#      parent MAP gives it (`NODE { ... } PARENT;`), in any order;
#   5. its exports (defined dynamic symbols, global, weak or unique
#      (STB_GNU_UNIQUE, g++'s static data in inline functions), but the
#      version nodes' own symbols: ABS, named for a version definition
#      and carrying no other version; any other ABS symbol, such as an
#      assembly `.set`, counts) are exactly MAP's `global:` names,
#      each at the version of the node that lists it (`name@@NODE`, or
#      `name@NODE`; a name two nodes list, at both), so that nothing
#      else leaks (Rust std symbols, mangled names) and nothing MAP
#      promises is missing;
#   6. each name MAP lists and SO exports has exactly one default
#      version (`name@@NODE`, the one a new link binds), at the C
#      library's: for a name MAP lists in one node, that node; for a
#      name it lists in more than one (an old and a new `foo`), the node
#      DEFAULTS gives it. DEFAULTS (one argument, `name@NODE` entries
#      separated by blanks, empty for none) is written from the C
#      library's own exports (`readelf --dyn-syms` shows `foo@@V2`):
#      `.symver` in the C sources, not the order of MAP's nodes, sets
#      it, so a name in more than one node with no entry fails, and so
#      does an entry for a node MAP does not list the name in. A
#      program built against SO then binds where one built against the
#      C library does.
#
# MAP is read as GNU ld reads a version script, for the forms a C
# library's map uses: `#` comments to the end of the line and `/* */`
# comments, each kind only outside the other (`/* # */` is one comment);
# named nodes, each `NAME { [global:] sym; ... [local: ...;]
# } [PARENT];`. A wildcard, `extern "C++"` block, anonymous node or
# second parent in MAP is not read and fails (exit 1), naming it.
#
# It prints what it read (SONAME, NEEDED, TLS, the version definitions,
# the export and map counts), a `FAIL: ...` line for each check that
# fails, and `ok abi-elf: SO` when all pass.
#
# Exit 0: every check passes; 1: a check fails, or SO or MAP is missing
# or unreadable, or MAP holds a form it does not read; 2: usage.
set -eu
usage() {
  echo "usage: abi-elf.sh SO MAP SONAME NEEDED [TLS_MAX [DEFAULTS]]${1:+: $1}" >&2
  exit 2
}
[ $# -ge 4 ] && [ $# -le 6 ] || usage
so=$1
map=$2
soname=$3
needed_ok=$4
tls_max=${5:-}
defaults=${6:-}
LC_ALL=C
export LC_ALL
[ -n "$soname" ] || usage "SONAME is empty"
[ -n "$(printf '%s' "$needed_ok" | tr -d ' \t')" ] || usage "NEEDED is empty"
if [ -n "$tls_max" ]; then
  printf '%s\n' "$tls_max" | grep -Eqx '[0-9]+' || usage "TLS_MAX is not a number of bytes: $tls_max"
fi
# DEFAULTS: one `NAME NODE` line per entry, each name once (read with
# globbing off: an entry is text).
set -f
seen=" "
dlines=""
for e in $defaults; do
  case $e in
    @* | *@ | *@*@*) usage "DEFAULTS entry $e is not name@NODE" ;;
    *@*) ;;
    *) usage "DEFAULTS entry $e is not name@NODE" ;;
  esac
  case $seen in
    *" ${e%%@*} "*) usage "DEFAULTS gives ${e%%@*} twice" ;;
  esac
  seen="$seen${e%%@*} "
  dlines="$dlines${e%%@*} ${e#*@}
"
done
set +f
[ -f "$so" ] || { echo "error: $so missing"; exit 1; }
[ -f "$map" ] || { echo "error: $map missing"; exit 1; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0
bad() {
  echo "FAIL: $*"
  fail=1
}
# elf ARGS...: readelf ARGS SO into $tmp/elf, its failure fatal.
elf() {
  rc=0
  readelf "$@" "$so" > "$tmp/elf" 2> "$tmp/err" || rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "error: readelf $* cannot read $so (exit $rc): $(head -n 1 "$tmp/err")"
    exit 1
  fi
}

# MAP: one `def NODE PARENT` line per node (PARENT `-` for none) and one
# `sym NAME NODE` line per global name, or `error ...` lines.
rc=0
awk '
  { raw = raw $0 "\n" }
  END {
    # Comments, read in one pass so that each kind counts only outside
    # the other: a `/* */` one (a `#` inside it is its text) and a `#`
    # one to the end of its line (a `/*` inside it is its text). Each
    # becomes a blank; then each brace, semicolon and colon is a token.
    text = ""; incom = 0; inhash = 0; n = length(raw)
    for (i = 1; i <= n; i++) {
      c = substr(raw, i, 1)
      if (incom) {
        if (c == "*" && substr(raw, i + 1, 1) == "/") { incom = 0; i++; text = text " " }
        continue
      }
      if (inhash) {
        if (c == "\n") { inhash = 0; text = text " " }
        continue
      }
      if (c == "/" && substr(raw, i + 1, 1) == "*") { incom = 1; i++; continue }
      if (c == "#") { inhash = 1; continue }
      text = text (c == "\n" ? " " : c)
    }
    if (incom) { print "error: an unterminated /* comment"; exit }
    gsub(/[{};:]/, " & ", text)
    n = split(text, tok, /[ \t]+/)
    state = "top"
    for (k = 1; k <= n; k++) {
      t = tok[k]
      if (t == "") continue
      if (state == "top") {
        if (t == "{") { print "error: an anonymous node"; exit }
        node = t; state = "open"; continue
      }
      if (state == "open") {
        if (t != "{") { print "error: node " node " has no {"; exit }
        state = "body"; mode = "global"; continue
      }
      if (state == "body") {
        if (t == "}") { state = "close"; parent = "-"; continue }
        if ((t == "global" || t == "local") && tok[k + 1] == ":") { mode = t; k++; continue }
        if (t == "extern") { print "error: node " node " has an extern block"; exit }
        if (t == ";") continue
        if (tok[k + 1] != ";") { print "error: node " node ": " t " is not followed by ;"; exit }
        if (mode == "global") {
          if (t ~ /[*?\[]/) { print "error: node " node " has a wildcard in global: " t; exit }
          print "sym " t " " node
        }
        continue
      }
      if (state == "close") {
        if (t == ";") { print "def " node " " parent; state = "top"; continue }
        if (parent != "-") { print "error: node " node " has a second parent " t; exit }
        parent = t; continue
      }
    }
    if (state != "top") print "error: the map ends inside node " node
  }' "$map" > "$tmp/map" || rc=$?
[ "$rc" -eq 0 ] || { echo "error: cannot read $map (awk exit $rc)"; exit 1; }
if grep -q '^error' "$tmp/map"; then
  echo "error: $map: $(sed -n 's/^error: //p' "$tmp/map" | head -n 1)"
  exit 1
fi

# 1-2: SONAME and DT_NEEDED.
elf -d
got_soname=$(sed -n 's/.*(SONAME).*Library soname: \[\(.*\)\]/\1/p' "$tmp/elf")
echo "SONAME: $got_soname"
[ "$got_soname" = "$soname" ] || bad "$so: SONAME is '$got_soname', want '$soname'"
needed=$(sed -n 's/.*(NEEDED).*Shared library: \[\(.*\)\]/\1/p' "$tmp/elf" | tr '\n' ' ')
echo "NEEDED: $needed"
for n in $needed; do
  ok=0
  for a in $needed_ok; do
    [ "$n" = "$a" ] && ok=1
  done
  [ "$ok" -eq 1 ] || bad "$so needs $n, which is not one of: $needed_ok"
done

# 3: thread-locals, MemSiz of the TLS segment.
elf -lW
tls=$(awk '$1 == "TLS" { print $6 }' "$tmp/elf")
echo "TLS: ${tls:-none}"
if [ -n "$tls_max" ] && [ -n "$tls" ]; then
  case $tls in
    0x | 0x*[!0-9a-fA-F]* | [!0]* | 0[!x]*) bad "$so: thread-locals size '$tls' is not a hex number" ;;
    *)
      if [ $((tls)) -gt "$tls_max" ]; then
        bad "$so: thread-locals are $((tls)) bytes, past TLS_MAX $tls_max"
      fi ;;
  esac
fi

# 4: version definitions, NODE PARENT, but the BASE entry.
elf -V -W
awk '
  /^Version definition section/ { in_d = 1; next }
  /^Version / { in_d = 0 }
  !in_d { next }
  /Name:/ { if (name != "") print name, parent; base = /Flags: BASE/; name = base ? "" : $NF; parent = "-"; next }
  /Parent 1:/ { if (name != "") parent = $NF }
  /Parent [2-9]/ { if (name != "") parent = parent "+" $NF }
  END { if (name != "") print name, parent }' "$tmp/elf" | sort > "$tmp/defs"
sed -n 's/^def //p' "$tmp/map" | sort > "$tmp/want-defs"
echo "version definitions:"
sed -e 's/ -$//' -e 's/ / <- /' -e 's/^/  /' "$tmp/defs"
while read -r d p; do
  grep -qxF "$d $p" "$tmp/defs" || bad "$map's node $d (parent $p) is not a version definition of $so as given"
done < "$tmp/want-defs"
while read -r d p; do
  grep -qxF "$d $p" "$tmp/want-defs" || bad "$so defines version $d (parent $p), which $map does not give as such"
done < "$tmp/defs"

# 5: exports, NAME VERSION, against the map's NAME NODE.
elf --dyn-syms -W
# A version node's own symbol is an ABS one named for a version
# definition, with no version of its own (or its own name); every other
# ABS definition (`.set leak, 1` in assembly) is an export like any.
awk 'FILENAME == ARGV[1] { node[$1] = 1; next }
     ($5 == "GLOBAL" || $5 == "WEAK" || $5 == "UNIQUE") && $7 != "UND" && NF >= 8 {
       n = $8; v = ""; d = "-"
       if ((i = index(n, "@")) > 0) {
         v = substr(n, i + 1); n = substr(n, 1, i - 1); d = "@"
         if (sub(/^@/, "", v)) d = "@@"
       }
       if ($7 == "ABS" && (n in node) && (v == "" || v == n)) next
       print n, (v == "" ? "-" : v), d
     }' "$tmp/defs" "$tmp/elf" | sort > "$tmp/exports"
sed -n 's/^sym //p' "$tmp/map" | sort > "$tmp/want-syms"
# Both ways by NAME VERSION pairs, so a name MAP lists in two nodes (an
# old and a new `foo`) must be exported at both. The name and version
# go through the environment: awk -v would read escapes. An export's
# third field is `@@` (its default version), `@` (a version a new link
# does not bind) or `-` (none, its version `-` too).
# pair FILE NAME VERSION: FILE has a line opening NAME VERSION.
pair() {
  n=$2 v=$3 awk '$1 == ENVIRON["n"] && $2 == ENVIRON["v"] { f = 1 } END { exit !f }' "$1"
}
while read -r name ver _; do
  nodes=$(n=$name awk '$1 == ENVIRON["n"] { printf "%s%s", s, $2; s = " " }' "$tmp/want-syms")
  if [ -z "$nodes" ]; then
    bad "$so exports $name, which $map does not list"
  elif [ "$ver" = - ]; then
    bad "$so exports $name with no version, where $map lists it in $nodes"
  elif ! pair "$tmp/want-syms" "$name" "$ver"; then
    bad "$so exports $name at version '$ver', where $map lists it in $nodes"
  fi
done < "$tmp/exports"
while read -r name node; do
  pair "$tmp/exports" "$name" "$node" \
    || bad "$map lists $name in $node, which $so does not export"
done < "$tmp/want-syms"

# 6: the default version. A new link against SO (`cc main.c -lfoo`)
# binds each name to its `@@` export alone, so each name MAP lists and
# SO exports must have exactly one, at the C library's: the one node MAP
# lists it in, or DEFAULTS' entry for it. A `foo@V1` alone serves
# programs linked before but no new one.
printf '%s' "$dlines" > "$tmp/defaults"
while read -r name node; do
  pair "$tmp/want-syms" "$name" "$node" \
    || bad "DEFAULTS gives $name@$node, but $map does not list $name in $node"
done < "$tmp/defaults"
awk '{ print $1 }' "$tmp/want-syms" | sort -u > "$tmp/names"
while read -r name; do
  # A name SO does not export at all has failed above already.
  n=$name awk '$1 == ENVIRON["n"] { f = 1 } END { exit !f }' "$tmp/exports" || continue
  nodes=$(n=$name awk '$1 == ENVIRON["n"] { printf "%s%s", s, $2; s = " " }' "$tmp/want-syms")
  defs=$(n=$name awk '$1 == ENVIRON["n"] && $3 == "@@" { printf "%s%s", s, $2; s = " " }' "$tmp/exports")
  want=$(n=$name awk '$1 == ENVIRON["n"] { print $2 }' "$tmp/defaults")
  # For a name MAP lists in one node, check 5 has put every export of
  # it, its default too, at that node.
  case $defs in
    "") bad "$so exports $name at no default version (@@), so a new link does not bind it" ;;
    *" "*) bad "$so exports $name at more than one default version: $defs" ;;
    *)
      if [ -n "$want" ]; then
        [ "$defs" = "$want" ] || bad "$so's default version of $name is $defs, where DEFAULTS gives $name@$want"
      else
        case $nodes in
          *" "*) bad "$map lists $name in $nodes and DEFAULTS gives none: give the C library's default as $name@NODE (its readelf --dyn-syms shows $name@@NODE)" ;;
        esac
      fi ;;
  esac
done < "$tmp/names"
echo "exports: $(wc -l < "$tmp/exports" | tr -d ' '), map: $(wc -l < "$tmp/want-syms" | tr -d ' ')"

[ "$fail" -eq 0 ] && echo "ok abi-elf: $so"
exit "$fail"
