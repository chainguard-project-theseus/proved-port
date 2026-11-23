#!/bin/sh
# kit/abi-elf-test.sh: tests of kit/make/abi-elf.sh (the abi-elf-test
# check). Each case builds a real shared object with cc and a version
# script, runs the script on it against a map, and checks the exit
# status and the lines it must print: a library that matches its map
# (named nodes with a parent, comments of both kinds), a wrong SONAME,
# a library it may not need (libm), thread-locals past and within
# TLS_MAX and with no TLS_MAX, a thread-locals size that is not hex
# (a stub readelf), a symbol that leaks, one the map lists that is not
# exported, one at the wrong version, a name in two nodes with its
# default version where DEFAULTS gives it (the first node or the last)
# or not, a missing or doubled default, DEFAULTS entries that do not
# parse or name no listed node, a weak and a unique export,
# version names that differ by a `.`, absolute symbols
# (leaked, listed, and one named for a node at another version, a stub
# readelf), a library with no version definitions, a node with
# another parent, two parents or missing, the map forms it does not read
# (a wildcard, an anonymous node, an extern block, a second parent, an
# unterminated node or comment, a node with no `{`, a name with no `;`),
# a missing library or map, a file readelf cannot read, and usage. Run
# it from the repository root. It needs a POSIX
# shell, BusyBox's tools, a C compiler and binutils.
set -eu

script=$(pwd)/kit/make/abi-elf.sh
[ -f "$script" ] || { echo "FAIL: run it from the repository root (no kit/make/abi-elf.sh)" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

loaders="libc.so.6 ld-linux-x86-64.so.2 ld-linux-aarch64.so.1"

# lib NAME MAP [CC ARGS...]: libNAME.so.1 from a.c with MAP as its
# version script and SONAME libNAME.so.1.
lib() {
  n=$1
  m=$2
  shift 2
  cc -shared -fPIC -o "lib$n.so.1" -Wl,-soname,"lib$n.so.1" -Wl,--version-script="$m" a.c "$@" \
    || fail "cc could not build lib$n.so.1"
}

# run ARGS...: the script with ARGS; its output (stdout and stderr) in
# out, its status in rc.
run() {
  if sh "$script" "$@" > out 2>&1; then rc=0; else rc=$?; fi
}

# want WHAT RC LINE...: the run exited RC and printed each LINE whole.
want() {
  what=$1
  code=$2
  shift 2
  ok=1
  [ "$rc" -eq "$code" ] || ok=0
  for l in "$@"; do
    grep -qxF -- "$l" out || ok=0
  done
  if [ "$ok" -eq 0 ]; then
    printf 'got (exit %s):\n' "$rc" >&2
    cat out >&2
    printf 'want (exit %s), with:\n' "$code" >&2
    printf '%s\n' "$@" >&2
    fail "$what"
  fi
  echo "ok $what"
}

# wantnot WHAT TEXT: the last run's output does not hold TEXT.
wantnot() {
  if grep -qF -- "$2" out; then
    cat out >&2
    fail "$1: got a line with $2, want none"
  fi
}

cat > a.c <<'EOF'
int foo(void) { return 1; }
int bar(void) { return 2; }
int baz(void) { return 3; }
int hid(void) { return 4; }
EOF
cat > good.map <<'EOF'
# The library's symbol versions.
V1 {
  global:
    foo; /* the first */
    bar;
  local: *;
};

/* A later node, empty but for one. */
V2 { global: baz; } V1;
EOF
lib good good.map
run libgood.so.1 good.map libgood.so.1 "$loaders"
want "a library that matches its map" 0 \
  "SONAME: libgood.so.1" "version definitions:" "  V1" "  V2 <- V1" \
  "exports: 3, map: 3" "ok abi-elf: libgood.so.1"
wantnot "a library that matches its map" "FAIL"
run libgood.so.1 good.map libother.so.1 "$loaders"
want "a wrong SONAME" 1 "FAIL: libgood.so.1: SONAME is 'libgood.so.1', want 'libother.so.1'"
wantnot "a wrong SONAME" "ok abi-elf"

# A library it may not need: libm, through a call to it.
cat > m.c <<'EOF'
#include <math.h>
double usem(double x) { return cos(x); }
EOF
cat > m.map <<'EOF'
V1 { global: foo; bar; usem; local: *; };
V2 { global: baz; } V1;
EOF
lib m m.map m.c -Wl,--no-as-needed -lm
run libm.so.1 m.map libm.so.1 "$loaders"
want "a library it may not need" 1 "FAIL: libm.so.1 needs libm.so.6, which is not one of: $loaders"

# Thread-locals: 800 bytes, past 512 and within 1024; no TLS_MAX, no
# bound.
cat > t.c <<'EOF'
__thread int tl[200];
int tlget(int i) { return tl[i]; }
EOF
cat > t.map <<'EOF'
V1 { global: foo; bar; tlget; local: *; };
V2 { global: baz; } V1;
EOF
lib t t.map t.c
run libt.so.1 t.map libt.so.1 "$loaders" 512
want "thread-locals past TLS_MAX" 1 "FAIL: libt.so.1: thread-locals are 800 bytes, past TLS_MAX 512"
run libt.so.1 t.map libt.so.1 "$loaders" 1024
want "thread-locals within TLS_MAX" 0 "ok abi-elf: libt.so.1"
run libt.so.1 t.map libt.so.1 "$loaders"
want "thread-locals with no TLS_MAX" 0 "ok abi-elf: libt.so.1"
run libgood.so.1 good.map libgood.so.1 "$loaders" 0
want "no thread-locals, TLS_MAX 0" 0 "TLS: none" "ok abi-elf: libgood.so.1"

# A symbol that leaks: the library's own script has no `local: *;`.
cat > leak.map <<'EOF'
V1 { global: foo; bar; hid; local: *; };
V2 { global: baz; } V1;
EOF
lib leak leak.map
run libleak.so.1 good.map libleak.so.1 "$loaders"
want "a symbol the map does not list" 1 "FAIL: libleak.so.1 exports hid, which good.map does not list"

# Each comment kind counts only outside the other: a `#` inside a
# `/* */` comment is its text (so `bar;` after it is read), and a `/*`
# inside a `#` comment is its text.
cat > hashcom.map <<'EOF'
V1 {
  global: foo;
  /* # note */
  bar;
  /* end */
  local: *;
};
V2 { global: baz; } V1;
EOF
run libgood.so.1 hashcom.map libgood.so.1 "$loaders"
want "a # inside a /* */ comment, the map matching" 0 "exports: 3, map: 3" "ok abi-elf: libgood.so.1"
cat > nobar.map <<'EOF'
V1 { global: foo; local: *; };
V2 { global: baz; } V1;
EOF
lib nobar nobar.map
run libnobar.so.1 hashcom.map libnobar.so.1 "$loaders"
want "a # inside a /* */ comment, a name after it not exported" 1 \
  "FAIL: hashcom.map lists bar in V1, which libnobar.so.1 does not export"
cat > slashcom.map <<'EOF'
V1 {
  global: foo; # see /* the note
  bar;
  local: *;
};
V2 { global: baz; } V1;
EOF
run libgood.so.1 slashcom.map libgood.so.1 "$loaders"
want "a /* inside a # comment" 0 "exports: 3, map: 3" "ok abi-elf: libgood.so.1"
printf 'V1 { global: foo; /* # */ bar; local: *; };\nV2 { global: baz; } V1;\n' > hashcom2.map
run libgood.so.1 hashcom2.map libgood.so.1 "$loaders"
want "a name after a /* # */ comment on its line" 0 "exports: 3, map: 3" "ok abi-elf: libgood.so.1"

# An absolute symbol (an assembly `.set`) is an export like any other;
# only a version node's own ABS symbol is not.
cat > abs.c <<'EOF'
__asm__(".globl leak\n.set leak, 1");
EOF
cat > abs.map <<'EOF'
V1 { global: foo; bar; leak; local: *; };
V2 { global: baz; } V1;
EOF
lib abs abs.map abs.c
run libabs.so.1 good.map libabs.so.1 "$loaders"
want "an absolute symbol the map does not list" 1 "FAIL: libabs.so.1 exports leak, which good.map does not list" \
  "exports: 4, map: 3"
run libabs.so.1 abs.map libabs.so.1 "$loaders"
want "an absolute symbol the map lists" 0 "exports: 4, map: 4" "ok abi-elf: libabs.so.1"
# A library with no version definitions: its exports carry none, so an
# empty map lists none of them.
# Its absolute symbol has no version either, and is still an export.
cc -shared -fPIC -o libplain.so.1 -Wl,-soname,libplain.so.1 a.c abs.c || fail "cc could not build libplain.so.1"
: > empty.map
run libplain.so.1 empty.map libplain.so.1 "$loaders"
want "a library with no version definitions against an empty map" 1 \
  "FAIL: libplain.so.1 exports foo, which empty.map does not list" \
  "FAIL: libplain.so.1 exports leak, which empty.map does not list" "exports: 5, map: 0"
# An ABS symbol named for a version node but at another version is no
# node's own (GNU ld refuses to link one, so a stub readelf adds it).
mkdir dstub
cat > dstub/readelf <<EOF
#!/bin/sh
$(command -v readelf) "\$@" || exit
if [ "\$1" = --dyn-syms ]; then
  echo '    99: 0000000000000001     0 NOTYPE  GLOBAL DEFAULT  ABS V1@@V2'
fi
EOF
chmod +x dstub/readelf
if PATH=$tmp/dstub:$PATH sh "$script" libgood.so.1 good.map libgood.so.1 "$loaders" > out 2>&1; then rc=0; else rc=$?; fi
want "an ABS symbol named for a node, at another version" 1 \
  "FAIL: libgood.so.1 exports V1, which good.map does not list" "exports: 4, map: 3"

# A symbol the map lists that the library does not export.
cat > more.map <<'EOF'
V1 { global: foo; bar; gone; local: *; };
V2 { global: baz; } V1;
EOF
run libgood.so.1 more.map libgood.so.1 "$loaders"
want "a listed symbol not exported" 1 "FAIL: more.map lists gone in V1, which libgood.so.1 does not export"
wantnot "a listed symbol not exported, once" "no default version"

# A symbol at the wrong version.
cat > early.map <<'EOF'
V1 { global: foo; bar; baz; local: *; };
V2 { } V1;
EOF
lib early early.map
run libearly.so.1 good.map libearly.so.1 "$loaders"
want "a symbol at the wrong version" 1 "FAIL: libearly.so.1 exports baz at version 'V1', where good.map lists it in V2"

# A node with another parent, and one missing.
cat > orphan.map <<'EOF'
V1 { global: foo; bar; local: *; };
V2 { global: baz; };
EOF
lib orphan orphan.map
run liborphan.so.1 good.map liborphan.so.1 "$loaders"
want "a node with another parent" 1 \
  "FAIL: good.map's node V2 (parent V1) is not a version definition of liborphan.so.1 as given" \
  "FAIL: liborphan.so.1 defines version V2 (parent -), which good.map does not give as such"
cat > three.map <<'EOF'
V1 { global: foo; bar; local: *; };
V2 { global: baz; } V1;
V3 { } V2;
EOF
run libgood.so.1 three.map libgood.so.1 "$loaders"
want "a node the library lacks" 1 "FAIL: three.map's node V3 (parent V2) is not a version definition of libgood.so.1 as given"
# A library node with two parents (GNU ld takes them, and writes them
# last first: Parent 1 is V0), against a map that gives it the one
# readelf prints first alone.
cat > twop.map <<'EOF'
V0 { global: foo; local: *; };
V1 { global: bar; } V0;
V2 { global: baz; } V1 V0;
EOF
lib twop twop.map
cat > onep.map <<'EOF'
V0 { global: foo; local: *; };
V1 { global: bar; } V0;
V2 { global: baz; } V0;
EOF
run libtwop.so.1 onep.map libtwop.so.1 "$loaders"
want "a library node with a second parent" 1 "  V2 <- V0+V1" \
  "FAIL: libtwop.so.1 defines version V2 (parent V0+V1), which onep.map does not give as such"

# A name MAP lists in two nodes (an old and a new `foo`, through
# .symver) must be exported at both versions.
cat > sv.c <<'EOF'
int foo_old(void) { return 1; }
int foo_new(void) { return 2; }
__asm__(".symver foo_old, foo@V1");
__asm__(".symver foo_new, foo@@V2");
EOF
cat > sv.map <<'EOF'
V1 { global: foo; local: *; };
V2 { global: foo; } V1;
EOF
cc -shared -fPIC -o libsv.so.1 -Wl,-soname,libsv.so.1 -Wl,--version-script=sv.map sv.c || fail "cc could not build libsv.so.1"
run libsv.so.1 sv.map libsv.so.1 "$loaders" "" foo@V2
want "a name exported at both nodes MAP lists it in" 0 "exports: 2, map: 2" "ok abi-elf: libsv.so.1"
cat > one.map <<'EOF'
V1 { global: foo; local: *; };
V2 { } V1;
EOF
cat > one.c <<'EOF'
int foo(void) { return 1; }
EOF
cc -shared -fPIC -o libone.so.1 -Wl,-soname,libone.so.1 -Wl,--version-script=one.map one.c || fail "cc could not build libone.so.1"
run libone.so.1 sv.map libone.so.1 "$loaders"
want "a name MAP lists in two nodes, exported at one" 1 \
  "FAIL: sv.map lists foo in V2, which libone.so.1 does not export" "exports: 1, map: 2"
run libsv.so.1 one.map libsv.so.1 "$loaders"
want "a name exported at a second version MAP does not list it at" 1 \
  "FAIL: libsv.so.1 exports foo at version 'V2', where one.map lists it in V1"
printf 'V1 { global: foo; local: *; };\nV2 { global: foo; } V1;\nV3 { } V2;\n' > sv3.map
run libone.so.1 sv3.map libone.so.1 "$loaders"
want "a name MAP lists in two nodes of three, exported at one" 1 \
  "FAIL: sv3.map lists foo in V2, which libone.so.1 does not export"
printf 'V1 { local: *; };\nV2 { global: foo; } V1;\nV3 { global: foo; } V2;\n' > sv4.map
run libone.so.1 sv4.map libone.so.1 "$loaders"
want "an export at a version MAP lists none of its nodes at" 1 \
  "FAIL: libone.so.1 exports foo at version 'V1', where sv4.map lists it in V2 V3"

# The default version (`@@`), which a new link binds: each listed name
# needs exactly one, at its one node or at DEFAULTS' entry. The order of
# MAP's nodes says nothing: libsv defaults to the last node listing foo,
# libswap to the first, and each passes with its own entry.
cat > nd.c <<'EOF'
int foo_impl(void) { return 1; }
__asm__(".symver foo_impl, foo@V1");
EOF
printf 'V1 { global: foo; local: *; };\n' > nd.map
cc -shared -fPIC -o libnd.so.1 -Wl,-soname,libnd.so.1 -Wl,--version-script=nd.map nd.c || fail "cc could not build libnd.so.1"
run libnd.so.1 nd.map libnd.so.1 "$loaders"
want "a name exported at a non-default version alone" 1 \
  "FAIL: libnd.so.1 exports foo at no default version (@@), so a new link does not bind it"
cat > swap.c <<'EOF'
int foo_old(void) { return 1; }
int foo_new(void) { return 2; }
__asm__(".symver foo_old, foo@@V1");
__asm__(".symver foo_new, foo@V2");
EOF
cc -shared -fPIC -o libswap.so.1 -Wl,-soname,libswap.so.1 -Wl,--version-script=sv.map swap.c || fail "cc could not build libswap.so.1"
run libswap.so.1 sv.map libswap.so.1 "$loaders" "" foo@V1
want "the default at the first of two nodes, as DEFAULTS gives it" 0 "exports: 2, map: 2" "ok abi-elf: libswap.so.1"
run libswap.so.1 sv.map libswap.so.1 "$loaders" "" foo@V2
want "the default at another node than DEFAULTS gives" 1 \
  "FAIL: libswap.so.1's default version of foo is V1, where DEFAULTS gives foo@V2"
run libsv.so.1 sv.map libsv.so.1 "$loaders" "" foo@V1
want "the default at the last of two nodes, DEFAULTS giving the first" 1 \
  "FAIL: libsv.so.1's default version of foo is V2, where DEFAULTS gives foo@V1"
run libswap.so.1 sv.map libswap.so.1 "$loaders"
want "a name in two nodes, no DEFAULTS entry" 1 \
  "FAIL: sv.map lists foo in V1 V2 and DEFAULTS gives none: give the C library's default as foo@NODE (its readelf --dyn-syms shows foo@@NODE)"
wantnot "a name in two nodes, no DEFAULTS entry" "ok abi-elf"
run libsv.so.1 sv.map libsv.so.1 "$loaders" 512 foo@V2
want "DEFAULTS beside a TLS_MAX" 0 "ok abi-elf: libsv.so.1"
run libsv.so.1 sv.map libsv.so.1 "$loaders" "" "foo@V2 foo@V1"
want "DEFAULTS giving a name twice" 2 "usage: abi-elf.sh SO MAP SONAME NEEDED [TLS_MAX [DEFAULTS]]: DEFAULTS gives foo twice"
run libsv.so.1 sv.map libsv.so.1 "$loaders" "" "foo@V2 bar@V3"
want "a DEFAULTS entry for a node MAP does not list the name in" 1 \
  "FAIL: DEFAULTS gives bar@V3, but sv.map does not list bar in V3"
run libnd.so.1 nd.map libnd.so.1 "$loaders" "" foo@V1
want "a DEFAULTS entry for a name exported at no default" 1 \
  "FAIL: libnd.so.1 exports foo at no default version (@@), so a new link does not bind it"
# A file foo@V9 here, which a glob foo@V* would expand to.
: > 'foo@V9'
for e in foo V2 @V2 foo@ foo@V1@V2 'foo@V*'; do
  run libsv.so.1 sv.map libsv.so.1 "$loaders" "" "$e"
  case $e in
    *'*') want "a DEFAULTS entry that is text, not a glob: $e" 1 \
      "FAIL: DEFAULTS gives foo@V*, but sv.map does not list foo in V*" ;;
    *) want "a DEFAULTS entry that is not name@NODE: $e" 2 \
      "usage: abi-elf.sh SO MAP SONAME NEEDED [TLS_MAX [DEFAULTS]]: DEFAULTS entry $e is not name@NODE" ;;
  esac
done
# Two default versions of one name (GNU ld refuses to link that, so a
# stub readelf adds a second).
mkdir twostub
cat > twostub/readelf <<EOF
#!/bin/sh
$(command -v readelf) "\$@" || exit
if [ "\$1" = --dyn-syms ]; then
  echo '    99: 0000000000001000     8 FUNC    GLOBAL DEFAULT   12 foo@@V1'
fi
EOF
chmod +x twostub/readelf
if PATH=$tmp/twostub:$PATH sh "$script" libsv.so.1 sv.map libsv.so.1 "$loaders" "" foo@V2 > out 2>&1; then rc=0; else rc=$?; fi
want "a name with two default versions" 1 "FAIL: libsv.so.1 exports foo at more than one default version: V1 V2"
# An export with no version, of a name MAP lists in a node.
cc -shared -fPIC -o libnov.so.1 -Wl,-soname,libnov.so.1 one.c || fail "cc could not build libnov.so.1"
run libnov.so.1 one.map libnov.so.1 "$loaders"
want "an unversioned export of a listed name" 1 \
  "FAIL: libnov.so.1 exports foo with no version, where one.map lists it in V1"

# Version names are compared as text: a `.` in one (PCRE2_10.47) is no
# wildcard. An extra empty node V1.0 beside V1_0 fails, and so does a
# map's V1.0 the library lacks beside a V1x0 it has.
cat > dot.map <<'EOF'
V1_0 { global: foo; bar; baz; local: *; };
V1.0 { };
EOF
lib dot dot.map
printf 'V1_0 { global: foo; bar; baz; local: *; };\n' > dot1.map
run libdot.so.1 dot1.map libdot.so.1 "$loaders"
want "an extra empty node whose name differs by a ." 1 \
  "FAIL: libdot.so.1 defines version V1.0 (parent -), which dot1.map does not give as such"
printf 'V1x0 { global: foo; bar; baz; local: *; };\n' > x0.map
lib x0 x0.map
printf 'V1x0 { global: foo; bar; baz; local: *; };\nV1.0 { };\n' > dot2.map
run libx0.so.1 dot2.map libx0.so.1 "$loaders"
want "a map's node the library lacks, its name matching another by a ." 1 \
  "FAIL: dot2.map's node V1.0 (parent -) is not a version definition of libx0.so.1 as given"

# A unique definition (STB_GNU_UNIQUE, as g++ gives static data in an
# inline function) is an export too.
cat > u.c <<'EOF'
__asm__(".globl uq\n.type uq, @gnu_unique_object\n.data\nuq: .long 1\n.text\n");
EOF
cat > u.map <<'EOF'
V1 { global: foo; bar; uq; local: *; };
V2 { global: baz; } V1;
EOF
lib u u.map u.c
run libu.so.1 good.map libu.so.1 "$loaders"
want "a unique symbol the map does not list" 1 "FAIL: libu.so.1 exports uq, which good.map does not list"
run libu.so.1 u.map libu.so.1 "$loaders"
want "a unique symbol the map lists" 0 "exports: 4, map: 4" "ok abi-elf: libu.so.1"

# A weak definition is an export like any other.
cat > w.c <<'EOF'
__attribute__((weak)) int wk(void) { return 5; }
EOF
cat > w.map <<'EOF'
V1 { global: foo; bar; wk; local: *; };
V2 { global: baz; } V1;
EOF
lib w w.map w.c
run libw.so.1 w.map libw.so.1 "$loaders"
want "a weak export the map lists" 0 "exports: 4, map: 4" "ok abi-elf: libw.so.1"
run libw.so.1 good.map libw.so.1 "$loaders"
want "a weak export the map does not list" 1 "FAIL: libw.so.1 exports wk, which good.map does not list"

# A thread-locals size readelf would not print (a stub readelf for -lW
# only): not a hex number.
mkdir stub
cat > stub/readelf <<EOF
#!/bin/sh
if [ "\$1" = -lW ]; then
  echo '  TLS            0x000e00 0x0000000000001e00 0x0000000000001e00 0x000000 0x0zz RW  0x8'
  exit 0
fi
exec $(command -v readelf) "\$@"
EOF
chmod +x stub/readelf
if PATH=$tmp/stub:$PATH sh "$script" libgood.so.1 good.map libgood.so.1 "$loaders" 512 > out 2>&1; then rc=0; else rc=$?; fi
want "a thread-locals size that is not hex" 1 "TLS: 0x0zz" "FAIL: libgood.so.1: thread-locals size '0x0zz' is not a hex number"

# The map forms it does not read.
for c in \
  "wild|V1 { global: f*; local: *; };|error: wild.map: node V1 has a wildcard in global: f*" \
  "anon|{ global: foo; };|error: anon.map: an anonymous node" \
  "ext|V1 { global: extern \"C++\" { x; }; };|error: ext.map: node V1 has an extern block" \
  "two|V1 { }; V2 { } V1 V0;|error: two.map: node V2 has a second parent V0" \
  "open|V1 { global: foo;|error: open.map: the map ends inside node V1" \
  "cmt|V1 { /* x };|error: cmt.map: an unterminated /* comment" \
  "nobrace|V1 global: foo; };|error: nobrace.map: node V1 has no {" \
  "nosemi|V1 { global: foo bar; };|error: nosemi.map: node V1: foo is not followed by ;" \
  "glue|V1 { global: foo/* a comment is a blank */bar; };|error: glue.map: node V1: foo is not followed by ;"; do
  name=${c%%|*}
  rest=${c#*|}
  printf '%s\n' "${rest%%|*}" > "$name.map"
  run libgood.so.1 "$name.map" libgood.so.1 "$loaders"
  want "a map with $name" 1 "${rest#*|}"
done

# Missing files, a file readelf cannot read, and usage.
run libnone.so.1 good.map libnone.so.1 "$loaders"
want "a missing library" 1 "error: libnone.so.1 missing"
run libgood.so.1 none.map libgood.so.1 "$loaders"
want "a missing map" 1 "error: none.map missing"
printf 'not ELF\n' > libtext.so.1
run libtext.so.1 good.map libtext.so.1 "$loaders"
if [ "$rc" -ne 1 ] || ! grep -q '^error: readelf -d cannot read libtext.so.1' out; then
  cat out >&2
  fail "a file readelf cannot read: got = exit $rc, want = exit 1 naming it"
fi
echo "ok a file readelf cannot read"
run libgood.so.1 good.map libgood.so.1
want "too few arguments" 2 "usage: abi-elf.sh SO MAP SONAME NEEDED [TLS_MAX [DEFAULTS]]"
run libgood.so.1 good.map "" "$loaders"
want "an empty SONAME" 2 "usage: abi-elf.sh SO MAP SONAME NEEDED [TLS_MAX [DEFAULTS]]: SONAME is empty"
run libgood.so.1 good.map libgood.so.1 " "
want "an empty NEEDED" 2 "usage: abi-elf.sh SO MAP SONAME NEEDED [TLS_MAX [DEFAULTS]]: NEEDED is empty"
run libgood.so.1 good.map libgood.so.1 "$loaders" 0x200
want "a TLS_MAX that is not a number" 2 "usage: abi-elf.sh SO MAP SONAME NEEDED [TLS_MAX [DEFAULTS]]: TLS_MAX is not a number of bytes: 0x200"
run libgood.so.1 good.map libgood.so.1 "$loaders" "" "" extra
want "too many arguments" 2 "usage: abi-elf.sh SO MAP SONAME NEEDED [TLS_MAX [DEFAULTS]]"
echo PASS
