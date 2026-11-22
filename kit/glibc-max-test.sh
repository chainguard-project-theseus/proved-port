#!/bin/sh
# kit/glibc-max-test.sh: tests of kit/make/glibc-max.sh (the
# glibc-max-test check). Most cases put a stub readelf first on PATH
# that prints a hand-written `readelf -V --wide` listing, and check the
# script's exit status and its output, spelled out by hand from
# challoc's ci/glibc-max.sh's format (none is taken from a run of the
# script under test): versions below, at and above MAX, version order
# (2.14 above 2.4), the needs it does not count (GLIBC_PRIVATE,
# GLIBC_ABI_DT_RELR, the .gnu.version section's names), duplicates
# across libraries, no GLIBC_ need, a readelf or grep failure, a missing
# SO and usage. The last cases build real shared objects with cc and
# read them with binutils' readelf. Run it from the repository root. It
# needs a POSIX shell, BusyBox's tools, a C compiler and binutils.
set -eu

script=$(pwd)/kit/make/glibc-max.sh
[ -f "$script" ] || { echo "FAIL: run it from the repository root (no kit/make/glibc-max.sh)" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# The stub readelf prints the file listing, or exits with the status in
# the file status if there is one.
mkdir stub
cat > stub/readelf <<'EOF'
#!/bin/sh
d=$(dirname "$0")
[ "$*" = "-V --wide lib.so" ] || { echo "stub readelf: got = $*, want = -V --wide lib.so" >&2; exit 3; }
if [ -f "$d/status" ]; then echo "readelf: Error: stub failure" >&2; exit "$(cat "$d/status")"; fi
cat "$d/listing"
EOF
chmod +x stub/readelf
: > lib.so

# run [ARGS...]: the script with ARGS, the stub readelf first on PATH;
# its output (stdout and stderr) in out, its status in rc.
run() {
  if PATH=$tmp/stub:$PATH sh "$script" "$@" > out 2>&1; then rc=0; else rc=$?; fi
}

# want WHAT RC: the run exited RC and printed exactly the file want.
want() {
  if [ "$rc" -ne "$2" ] || ! cmp -s want out; then
    printf 'got (exit %s):\n' "$rc" >&2
    cat out >&2
    printf 'want (exit %s):\n' "$2" >&2
    cat want >&2
    fail "$1"
  fi
  echo "ok $1"
}

# listing NAME...: a version-needs listing with these Name: entries
# under libc.so.6, after a .gnu.version section naming GLIBC_2.99 (which
# is not a need: no `Name:`).
listing() {
  {
    echo "Version symbols section '.gnu.version' contains 4 entries:"
    echo "  000:   0 (*local*)       2 (GLIBC_2.99)    0 (*local*)       1 (*global*)"
    echo
    echo "Version needs section '.gnu.version_r' contains 1 entry:"
    echo "  000000: Version: 1  File: libc.so.6  Cnt: $#"
    for n in "$@"; do
      echo "  0x0010:   Name: $n  Flags: none  Version: 2"
    done
  } > stub/listing
}

# --- below, at and above MAX ------------------------------------------------------
listing GLIBC_2.2.5 GLIBC_2.3.4 GLIBC_2.14
run lib.so 2.44
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.2.5 2.3.4 2.14
ok: highest GLIBC_2.14 <= GLIBC_2.44
EOF
want "needs below MAX pass, listed in version order" 0

listing GLIBC_2.34 GLIBC_2.44 GLIBC_2.2.5
run lib.so 2.44
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.2.5 2.34 2.44
ok: highest GLIBC_2.44 <= GLIBC_2.44
EOF
want "a need equal to MAX passes" 0
# Its temporary files are removed (mktemp follows TMPDIR).
mkdir td
if TMPDIR=$tmp/td PATH=$tmp/stub:$PATH sh "$script" lib.so 2.44 > out 2>&1; then rc=0; else rc=$?; fi
[ "$rc" -eq 0 ] && [ -z "$(ls -A td)" ] || { ls -A td >&2; fail "temporary files: got = exit $rc and these left, want = exit 0 and none"; }
echo "ok its temporary files are removed"

listing GLIBC_2.34 GLIBC_2.45 GLIBC_2.2.5
run lib.so 2.44
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.2.5 2.34 2.45
error: lib.so needs GLIBC_2.45, above GLIBC_2.44
EOF
want "a need above MAX fails, naming it" 1

# --- version order, not string order ------------------------------------------------
listing GLIBC_2.14 GLIBC_2.3.4
run lib.so 2.4
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.3.4 2.14
error: lib.so needs GLIBC_2.14, above GLIBC_2.4
EOF
want "2.14 is above 2.4" 1
listing GLIBC_2.3.4
run lib.so 2.3
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.3.4
error: lib.so needs GLIBC_2.3.4, above GLIBC_2.3
EOF
want "2.3.4 is above 2.3" 1
run lib.so 2.14
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.3.4
ok: highest GLIBC_2.3.4 <= GLIBC_2.14
EOF
want "2.3.4 is below 2.14" 0

# --- the needs it does not count, and duplicates -------------------------------------
listing GLIBC_PRIVATE GLIBC_ABI_DT_RELR GLIBC_2.17 GLIBC_2.17
printf '%s\n' "  000000: Version: 1  File: libm.so.6  Cnt: 1" \
  "  0x0040:   Name: GLIBC_2.17  Flags: none  Version: 5" >> stub/listing
run lib.so 2.44
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.17
GLIBC_ABI_DT_RELR needs glibc 2.36
ok: highest GLIBC_2.17 <= GLIBC_2.44
EOF
want "GLIBC_PRIVATE and .gnu.version names are not versions, duplicates count once, and GLIBC_ABI_DT_RELR needs 2.36" 0

# --- ABI markers ----------------------------------------------------------------------
# GLIBC_ABI_DT_RELR (glibc 2.36) against a MAX below it fails, though
# every symbol version is below MAX; at 2.36 it passes.
listing GLIBC_2.17 GLIBC_ABI_DT_RELR
run lib.so 2.35
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.17
GLIBC_ABI_DT_RELR needs glibc 2.36
error: lib.so needs GLIBC_ABI_DT_RELR (glibc 2.36), above GLIBC_2.35
EOF
want "GLIBC_ABI_DT_RELR above MAX fails, though the symbol versions are below it" 1
run lib.so 2.36
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.17
GLIBC_ABI_DT_RELR needs glibc 2.36
ok: highest GLIBC_2.17 <= GLIBC_2.36
EOF
want "GLIBC_ABI_DT_RELR at MAX passes" 0
# Needed from two libraries, a marker is named once.
printf '%s\n' "  000000: Version: 1  File: libm.so.6  Cnt: 1" \
  "  0x0040:   Name: GLIBC_ABI_DT_RELR  Flags: none  Version: 5" >> stub/listing
run lib.so 2.36
want "a marker needed twice is named once" 0
# A marker the table lacks fails, naming it.
listing GLIBC_2.17 GLIBC_ABI_NEW_THING
run lib.so 2.44
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.17
error: lib.so needs GLIBC_ABI_NEW_THING, an ABI marker glibc-max.sh does not know the glibc release of; add it to the script's table
EOF
want "an ABI marker the table lacks fails, naming it" 1

listing GLIBC_PRIVATE
run lib.so 2.44
printf '%s\n' 'error: lib.so needs no GLIBC_ version at all (not a glibc build?)' > want
want "no GLIBC_ version need fails: not a glibc build" 1
: > stub/listing
run lib.so 2.44
want "an empty listing fails the same way" 1

# --- the version definition section is not read --------------------------------------
# defs NAME...: a .gnu.version_d block defining these nodes, as binutils
# prints it, put before the needs listing in stub/listing.
defs() {
  {
    echo "Version definition section '.gnu.version_d' contains $(($# + 1)) entries:"
    echo "  000000: Rev: 1  Flags: BASE  Index: 1  Cnt: 1  Name: lib.so"
    for n in "$@"; do
      echo "  0x001c: Rev: 1  Flags: none  Index: 2  Cnt: 1  Name: $n"
    done
    echo
    cat stub/listing
  } > stub/defs
  mv stub/defs stub/listing
}
listing GLIBC_2.2.5 GLIBC_2.17
defs GLIBC_2.99 GLIBC_2.2.5
run lib.so 2.44
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.2.5 2.17
ok: highest GLIBC_2.17 <= GLIBC_2.44
EOF
want "a GLIBC_ node SO defines above MAX is no need: needs below MAX pass" 0
printf '%s\n' "Version definition section '.gnu.version_d' contains 2 entries:" \
  "  000000: Rev: 1  Flags: BASE  Index: 1  Cnt: 1  Name: lib.so" \
  "  0x001c: Rev: 1  Flags: none  Index: 2  Cnt: 1  Name: GLIBC_2.17" > stub/listing
run lib.so 2.44
printf '%s\n' 'error: lib.so needs no GLIBC_ version at all (not a glibc build?)' > want
want "GLIBC_ nodes SO defines, with no needs, fail: not a glibc build" 1
# The needs section first, then the definitions: only the needs count.
listing GLIBC_2.17
printf '%s\n' "Version definition section '.gnu.version_d' contains 1 entry:" \
  "  0x001c: Rev: 1  Flags: none  Index: 2  Cnt: 1  Name: GLIBC_2.99" >> stub/listing
run lib.so 2.44
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.17
ok: highest GLIBC_2.17 <= GLIBC_2.44
EOF
want "a definition section after the needs ends them" 0
# An ABI marker SO defines is no need either.
listing GLIBC_2.17
defs GLIBC_ABI_DT_RELR
run lib.so 2.35
cat > want <<'EOF'
GLIBC_ versions lib.so needs: 2.17
ok: highest GLIBC_2.17 <= GLIBC_2.35
EOF
want "an ABI marker SO defines is no need" 0

# --- failures reading SO -------------------------------------------------------------
listing GLIBC_2.17
echo 1 > stub/status
run lib.so 2.44
printf '%s\n' 'readelf: Error: stub failure' 'error: readelf -V cannot read lib.so (exit 1)' > want
want "a readelf failure fails, not a pass" 1
rm stub/status
# grep's read error (2) is told apart from no match (1). The stub grep
# fails only the needs grep (-oE); the MAX check runs the real one.
mkdir stub-grep
cat > stub-grep/grep <<'EOF'
#!/bin/sh
if [ "$1" = -oE ]; then echo "grep: read error" >&2; exit 2; fi
PATH=$REAL_PATH exec grep "$@"
EOF
chmod +x stub-grep/grep
REAL_PATH=$PATH
export REAL_PATH
if PATH=$tmp/stub-grep:$tmp/stub:$PATH sh "$script" lib.so 2.44 > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' 'grep: read error' "error: cannot read readelf's output for lib.so (grep exit 2)" > want
want "a grep read error fails, not no needs" 1
mkdir stub-awk
# shellcheck disable=SC2016 # $0 is the stub's, expanded when it runs
printf '%s\n' '#!/bin/sh' 'echo "$0: failed" >&2' 'exit 2' > stub-awk/awk
chmod +x stub-awk/awk
if PATH=$tmp/stub-awk:$tmp/stub:$PATH sh "$script" lib.so 2.44 > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' "$tmp/stub-awk/awk: failed" "error: awk failed on readelf's output for lib.so" > want
want "an awk that fails on the listing fails, not no needs" 1
# A sed that fails on the marker list fails, not "no markers" (the stub
# fails only that sed; the others run the real one).
mkdir stub-sed
cat > stub-sed/sed <<'EOF'
#!/bin/sh
if [ "$1" = 's/Name: //' ]; then echo "sed: failed" >&2; exit 2; fi
PATH=$REAL_PATH exec sed "$@"
EOF
chmod +x stub-sed/sed
listing GLIBC_2.17 GLIBC_ABI_DT_RELR
if PATH=$tmp/stub-sed:$tmp/stub:$PATH sh "$script" lib.so 2.35 > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' 'GLIBC_ versions lib.so needs: 2.17' 'sed: failed' "error: sed failed on readelf's output for lib.so" > want
want "a sed that fails on the marker list fails, not no markers" 1
# A sed that prints part of the versions and then fails: the higher
# need it did not print must not pass unseen.
mkdir stub-sed-part
cat > stub-sed-part/sed <<'EOF'
#!/bin/sh
if [ "$1" = 's/Name: GLIBC_//' ]; then PATH=$REAL_PATH sed "$@" | head -n 1; echo "sed: read error" >&2; exit 2; fi
PATH=$REAL_PATH exec sed "$@"
EOF
chmod +x stub-sed-part/sed
listing GLIBC_2.2.5 GLIBC_2.45
if PATH=$tmp/stub-sed-part:$tmp/stub:$PATH sh "$script" lib.so 2.44 > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' 'sed: read error' "error: sed failed on readelf's output for lib.so" > want
want "a sed that prints part of the versions and fails fails, not a pass on the part" 1
# A sort -V that prints part of the comparison and fails: MAX alone
# printed first must not pass a need above it.
mkdir stub-sort-part
cat > stub-sort-part/sort <<'EOF'
#!/bin/sh
if [ "$1" = -V ]; then PATH=$REAL_PATH sort "$@" | head -n 1; echo "sort: read error" >&2; exit 2; fi
PATH=$REAL_PATH exec sort "$@"
EOF
chmod +x stub-sort-part/sort
if PATH=$tmp/stub-sort-part:$tmp/stub:$PATH sh "$script" lib.so 2.44 > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' 'GLIBC_ versions lib.so needs: 2.2.5 2.45' 'sort: read error' 'error: sort -V failed comparing 2.45 with 2.44' > want
want "a sort -V that prints part of the comparison and fails fails, not a pass" 1
run missing.so 2.44
printf '%s\n' 'error: missing.so missing' > want
want "a missing SO fails" 1
mkdir dir.so
run dir.so 2.44
printf '%s\n' 'error: dir.so missing' > want
want "an SO that is a directory fails as missing, before readelf" 1

# --- usage --------------------------------------------------------------------------
run lib.so
printf '%s\n' 'usage: glibc-max.sh SO MAX' > want
want "one argument is a usage error" 2
run lib.so 2.44 extra
want "three arguments are a usage error" 2
for m in abc 2.44a 2. .44 '' '2.44 '; do
  run lib.so "$m"
  printf '%s\n' "usage: glibc-max.sh SO MAX: MAX is not a version (2.44): $m" > want
  want "MAX '$m' is a usage error" 2
done

# --- real shared objects ---------------------------------------------------------------
# A library that calls into libc needs some GLIBC_ version (2.2.5 on
# x86_64, 2.17 on aarch64, and memcpy's 2.14 on x86_64), which depends
# on the arch and the toolchain, so these pin the verdict, not the list.
command -v cc > /dev/null || fail "no cc: the check's environment needs a C compiler"
command -v readelf > /dev/null || fail "no readelf: the check's environment needs binutils"
printf '%s\n' '#include <stdio.h>' '#include <string.h>' \
  'int f(char *d, const char *s, unsigned long n) { memcpy(d, s, n); return printf("%s", d); }' > a.c
cc -shared -fPIC -o liba.so a.c
if sh "$script" liba.so 99.0 > out 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 0 ] || ! grep -q '^GLIBC_ versions liba.so needs: 2\.' out || ! grep -qx 'ok: highest GLIBC_2\.[0-9.]* <= GLIBC_99.0' out; then
  cat out >&2
  fail "a real libc-linked library, MAX 99.0: got = exit $rc, want = exit 0 with its 2.x needs and ok"
fi
echo "ok a real libc-linked library passes MAX 99.0"
if sh "$script" liba.so 2.0 > out 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 1 ] || ! grep -qx 'error: liba.so needs GLIBC_2\.[0-9.]*, above GLIBC_2.0' out; then
  cat out >&2
  fail "a real libc-linked library, MAX 2.0: got = exit $rc, want = exit 1 naming its highest need"
fi
echo "ok a real libc-linked library fails MAX 2.0"
printf '%s\n' 'int g(int x) { return x + 1; }' > b.c
cc -shared -fPIC -nostdlib -o libb.so b.c
if sh "$script" libb.so 2.44 > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' 'error: libb.so needs no GLIBC_ version at all (not a glibc build?)' > want
if [ "$rc" -ne 1 ] || ! cmp -s want out; then
  cat out >&2
  fail "a real library linked with no libc: got = exit $rc, want = exit 1, not a glibc build"
fi
echo "ok a real library linked with no libc fails: not a glibc build"
# One linked with no libc that defines a GLIBC_ node (a version script)
# needs none: still not a glibc build.
printf '%s\n' 'GLIBC_2.99 { global: g; local: *; };' > b.map
cc -shared -fPIC -nostdlib -Wl,--version-script=b.map -o libd.so b.c
readelf -V --wide libd.so | grep -q 'Name: GLIBC_2.99' || fail "libd.so: readelf shows no GLIBC_2.99 definition (the fixture did not build as meant)"
if sh "$script" libd.so 2.44 > out 2>&1; then rc=0; else rc=$?; fi
printf '%s\n' 'error: libd.so needs no GLIBC_ version at all (not a glibc build?)' > want
if [ "$rc" -ne 1 ] || ! cmp -s want out; then
  cat out >&2
  fail "a real library that defines GLIBC_2.99 and needs none: got = exit $rc, want = exit 1, not a glibc build"
fi
echo "ok a real library that defines GLIBC_2.99 and needs none fails: not a glibc build"
# One linked with -z pack-relative-relocs needs GLIBC_ABI_DT_RELR (glibc
# 2.36): MAX 2.35 fails on it, MAX 2.36 passes.
printf '%s\n' '#include <stdio.h>' 'static int x; int *ps[] = { &x, &x, &x };' \
  'int q(void) { return printf("%p", (void *)ps[0]); }' > r.c
cc -shared -fPIC -Wl,-z,pack-relative-relocs -o libr.so r.c
readelf -V --wide libr.so | grep -q 'Name: GLIBC_ABI_DT_RELR' || fail "libr.so: readelf shows no GLIBC_ABI_DT_RELR need (the fixture did not build as meant)"
if sh "$script" libr.so 2.35 > out 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 1 ] || ! grep -qxF 'error: libr.so needs GLIBC_ABI_DT_RELR (glibc 2.36), above GLIBC_2.35' out; then
  cat out >&2
  fail "a real library with DT_RELR, MAX 2.35: got = exit $rc, want = exit 1 naming GLIBC_ABI_DT_RELR"
fi
if sh "$script" libr.so 2.36 > out 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 0 ] || ! grep -qxF 'GLIBC_ABI_DT_RELR needs glibc 2.36' out; then
  cat out >&2
  fail "a real library with DT_RELR, MAX 2.36: got = exit $rc, want = exit 0 with the marker's line"
fi
echo "ok a real library with DT_RELR fails MAX 2.35 and passes MAX 2.36"
cp a.c not-elf.so
if sh "$script" not-elf.so 2.44 > out 2>&1; then rc=0; else rc=$?; fi
if [ "$rc" -ne 1 ] || ! grep -qx 'error: readelf -V cannot read not-elf.so (exit [1-9][0-9]*)' out; then
  cat out >&2
  fail "a file that is not ELF: got = exit $rc, want = exit 1, readelf cannot read it"
fi
echo "ok a file that is not ELF fails: readelf cannot read it"

echo "PASS"
