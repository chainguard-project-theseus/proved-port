#!/bin/sh
# kit/fuzz-diff-test.sh: tests of kit/make/fuzz-diff.c, the differential
# fuzzer's generic driver shell (the fuzz-diff-test check). It builds
# stub shared libraries with cc (two that differ on one input, one that
# takes a symbol from a library it depends on, one with no such symbol)
# and a test driver on the shell, written here, that loads two of them as
# a port's driver does and compares one function on each argument. Each
# case checks the exit status, stdout and stderr, spelled out by hand
# from the PCRE2 port's fuzz/diff/driver.c messages (none is taken from
# a run of the shell): the preflight's refusals (no path, a relative
# path, a file that does not load, a missing symbol, a symbol that
# resolves into a dependency, one file loaded twice or through a link),
# the run modes (PREFLIGHT, CONTINUE, the abort without it), the known
# log (once per name per process, by string), the units at 8, 16 and 32
# bits, base64 against BusyBox's base64, and the init refusals. Run it
# from the repository root. It needs a POSIX shell, BusyBox's tools and
# a C compiler.
set -eu

kit=$(pwd)/kit/make
[ -f "$kit/fuzz-diff.c" ] || { echo "FAIL: run it from the repository root (no kit/make/fuzz-diff.c)" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"
cc=${CC:-cc}

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# --- the stub libraries and the test driver ---------------------------------------
cat > a.c <<'EOF'
int add(int x) { return x + 1; }
int ver(void) { return 1; }
EOF
# b differs from a on 5 and 6.
cat > b.c <<'EOF'
int add(int x) { return x == 5 || x == 6 ? 0 : x + 1; }
int ver(void) { return 2; }
EOF
# shim has ver itself and add only through libdep.so, which it needs.
cat > dep.c <<'EOF'
int add(int x) { return x + 1; }
EOF
cat > shim.c <<'EOF'
int ver(void) { return 3; }
EOF
cat > none.c <<'EOF'
int ver(void) { return 4; }
EOF
$cc -shared -fPIC -o liba.so a.c
$cc -shared -fPIC -o libb.so b.c
$cc -shared -fPIC -o libdep.so dep.c
$cc -shared -fPIC -o libshim.so shim.c -L. -Wl,--no-as-needed -ldep -Wl,-rpath,"$tmp"
$cc -shared -fPIC -o libnone.so none.c
ln -s liba.so liblink.so
echo 'not an object' > libtext.so

# t MODE ARGS: run: loads T_A and T_B, resolves add and ver in both, checks
# they are distinct, prints a banner, ppd_ready, then compares add on
# each argument (a "k" prefix makes a difference a known class);
# units W HEX...: ppd_put_units at width W; b64 S: ppd_put_base64 of S;
# b64n S N: of S's first N bytes; known NAME...: ppd_known on each;
# known-buf NAME...: the same through one reused buffer; die: ppd_die;
# noinit: ppd_open before ppd_init. T_PREFIX is the environment prefix
# (default T_DIFF).
cat > t.c <<'EOF'
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fuzz-diff.h"

int main(int argc, char **argv) {
  const char *mode = argc > 1 ? argv[1] : "";
  if (strcmp(mode, "noinit") == 0) {
    struct ppd_lib a = {"ref", "/x.so", 0, 0, 0};
    ppd_open(&a);
    return 0;
  }
  const char *prefix = getenv("T_PREFIX");
  ppd_init("t-diff", prefix != NULL ? prefix : "T_DIFF");
  if (strcmp(mode, "units") == 0) {
    unsigned w = (unsigned)atoi(argv[2]);
    size_t n = (size_t)argc - 3;
    uint8_t u8[16];
    uint16_t u16[16];
    uint32_t u32[16];
    for (size_t i = 0; i < n && i < 16; i++) {
      unsigned long v = strtoul(argv[3 + i], NULL, 16);
      u8[i] = (uint8_t)v;
      u16[i] = (uint16_t)v;
      u32[i] = (uint32_t)v;
    }
    ppd_put_units(stdout, w == 8 ? (void *)u8 : w == 16 ? (void *)u16 : (void *)u32, n, w);
    putchar('\n');
    return 0;
  }
  if (strcmp(mode, "b64") == 0) {
    ppd_put_base64(stdout, (const uint8_t *)argv[2], strlen(argv[2]));
    return 0;
  }
  if (strcmp(mode, "b64n") == 0) {
    ppd_put_base64(stdout, (const uint8_t *)argv[2], (size_t)atoi(argv[3]));
    return 0;
  }
  if (strcmp(mode, "known") == 0) {
    for (int i = 2; i < argc; i++) ppd_known("known divergence", argv[i]);
    return 0;
  }
  if (strcmp(mode, "known-buf") == 0) {
    char buf[64];
    for (int i = 2; i < argc; i++) {
      snprintf(buf, sizeof buf, "%s", argv[i]);
      ppd_known("known divergence", buf);
    }
    return 0;
  }
  if (strcmp(mode, "die") == 0) ppd_die("%s and %d", "this", 7);
  struct ppd_lib a = {"ref", getenv("T_A"), 0, 0, 0};
  struct ppd_lib b = {"port", getenv("T_B"), 0, 0, 0};
  ppd_open(&a);
  ppd_open(&b);
  int (*fa)(int), (*fb)(int), (*va)(void), (*vb)(void);
  void *p = ppd_sym(&a, "add");
  memcpy(&fa, &p, sizeof p);
  p = ppd_sym(&b, "add");
  memcpy(&fb, &p, sizeof p);
  p = ppd_sym(&a, "ver");
  memcpy(&va, &p, sizeof p);
  p = ppd_sym(&b, "ver");
  memcpy(&vb, &p, sizeof p);
  ppd_distinct(&a, &b);
  printf("t-diff: ref %s (%d), port %s (%d)\n", a.file, va(), b.file, vb());
  ppd_ready();
  for (int i = 2; i < argc; i++) {
    const char *s = argv[i];
    int known = s[0] == 'k';
    int x = atoi(s + known);
    int ra = fa(x), rb = fb(x);
    if (ra == rb) continue;
    if (known) {
      ppd_known("known divergence", "#1 k-class");
      continue;
    }
    ppd_divergence("add", x == 6 ? "(six)" : NULL);
    fprintf(stderr, "  ref  rc=%d\n  port rc=%d\n", ra, rb);
    ppd_finding_end((const uint8_t *)s, strlen(s));
  }
  printf("t-diff: done (keep going %d)\n", ppd_keep_going());
  return 0;
}
EOF
$cc -std=c11 -Wall -Wextra -Werror -I"$kit" -o t t.c "$kit/fuzz-diff.c" -ldl

# run ARGS...: ./t ARGS with the environment set before it; stdout in
# out, stderr in err, the status in rc. A subshell that execs it, so a
# shell's note on a killed child ("Aborted") goes to this script's
# stderr, not to err.
run() {
  if (exec ./t "$@" > out 2> err); then rc=0; else rc=$?; fi
}

# want WHAT RC: the run exited RC, printed exactly wout on stdout and
# exactly werr on stderr.
want() {
  if [ "$rc" -ne "$2" ] || ! cmp -s wout out || ! cmp -s werr err; then
    printf 'got (exit %s), stdout:\n' "$rc" >&2
    cat out >&2
    echo 'stderr:' >&2
    cat err >&2
    printf 'want (exit %s), stdout:\n' "$2" >&2
    cat wout >&2
    echo 'stderr:' >&2
    cat werr >&2
    fail "$1"
  fi
  echo "ok $1"
}

a=$tmp/liba.so
b=$tmp/libb.so
export T_A="$a" T_B="$b"

# --- loading and the preflight ----------------------------------------------------
run run 1 2 3
printf 't-diff: ref %s (1), port %s (2)\nt-diff: done (keep going 0)\n' "$a" "$b" > wout
: > werr
want "two libraries that agree on every input" 0

T_DIFF_PREFLIGHT='' run run 5
printf 't-diff: ref %s (1), port %s (2)\n' "$a" "$b" > wout
: > werr
want "PREFLIGHT (set, empty) exits 0 after the banner, comparing nothing" 0

T_B='' run run 1
: > wout
echo 't-diff: port: no library path' > werr
want "an empty library path" 1

unset T_B
run run 1
want "no library path" 1
export T_B="$b"

T_B=libb.so run run 1
echo 't-diff: port: libb.so is not an absolute path' > werr
want "a relative path" 1

T_B=./libb.so run run 1
echo 't-diff: port: ./libb.so is not an absolute path' > werr
want "a ./ path" 1

T_B=$tmp/libmissing.so run run 1
if [ "$rc" -ne 1 ] || [ -s out ] || ! grep -q "^t-diff: port: $tmp/libmissing.so: cannot open shared object file" err; then
  fail "a missing file: got = exit $rc, $(cat err), want = exit 1 with dlerror's message"
fi
echo "ok a missing file names dlerror's message"

T_B=$tmp/libtext.so run run 1
if [ "$rc" -ne 1 ] || [ -s out ] || ! grep -q "^t-diff: port: $tmp/libtext.so: " err; then
  fail "a file that is not an object: got = exit $rc, $(cat err), want = exit 1 with dlerror's message"
fi
echo "ok a file that is not an object"

T_B=$tmp/libnone.so run run 1
echo 't-diff: port: no add' > werr
want "a library without a symbol" 1

T_B=$tmp/libshim.so run run 1
echo "t-diff: port: add resolves outside the library's object (to $tmp/libdep.so)" > werr
want "a symbol that resolves into a library the library needs" 1

T_B=$a run run 1
echo "t-diff: both tables resolve to one object ($a, $a)" > werr
want "one library loaded twice" 1

T_B=$tmp/liblink.so run run 1
echo "t-diff: both tables resolve to one object ($a, $a)" > werr
want "one library twice, through a link" 1

# --- findings and the run modes ---------------------------------------------------
run run 4 5 7
printf 't-diff: ref %s (1), port %s (2)\n' "$a" "$b" > wout
cat > werr <<'EOF'
t-diff: DIVERGENCE in add
  ref  rc=6
  port rc=0
  reproducer (base64): NQ==
EOF
# 134: killed by SIGABRT (128 + 6).
want "a finding prints its reproducer and aborts" 134

T_DIFF_CONTINUE=1 run run 5 3 6 k5
printf 't-diff: ref %s (1), port %s (2)\nt-diff: done (keep going 1)\n' "$a" "$b" > wout
cat > werr <<'EOF'
t-diff: DIVERGENCE in add
  ref  rc=6
  port rc=0
  reproducer (base64): NQ==
t-diff: DIVERGENCE in add (six)
  ref  rc=7
  port rc=0
  reproducer (base64): Ng==
t-diff: known divergence #1 k-class
t-diff: 2 divergences
EOF
want "CONTINUE reports every finding and exits 1 at the end" 1

T_DIFF_CONTINUE=1 run run 1 k5 k6 2
printf 't-diff: ref %s (1), port %s (2)\nt-diff: done (keep going 1)\n' "$a" "$b" > wout
echo 't-diff: known divergence #1 k-class' > werr
want "CONTINUE with only known classes exits 0, naming the class once" 0

T_PREFIX=OTHER T_DIFF_CONTINUE=1 T_DIFF_PREFLIGHT=1 run run 5
if [ "$rc" -le 128 ]; then fail "another prefix: got = exit $rc, want = killed by SIGABRT (T_DIFF_* are not its variables)"; fi
echo "ok another prefix reads its own variables, not T_DIFF_*"

T_PREFIX=OTHER OTHER_PREFLIGHT=1 run run 5
printf 't-diff: ref %s (1), port %s (2)\n' "$a" "$b" > wout
: > werr
want "another prefix's PREFLIGHT" 0

# --- the known log ----------------------------------------------------------------
T_DIFF_KNOWN_LOG=$tmp/known.log run known '#7 x' '#8 y' '#7 x' '#8 y' '#9 z'
: > wout
cat > werr <<'EOF'
t-diff: known divergence #7 x
t-diff: known divergence #8 y
t-diff: known divergence #9 z
EOF
want "each name once per process, compared as a string" 0
run known-buf '#7 x' '#8 y' '#7 x' '#9 z'
cat > werr <<'EOF'
t-diff: known divergence #7 x
t-diff: known divergence #8 y
t-diff: known divergence #9 z
EOF
want "names passed through one reused buffer, each kept as a copy" 0
T_DIFF_KNOWN_LOG=$tmp/known.log run known '#8 y'
printf '#7 x\n#8 y\n#9 z\n#8 y\n' > want
cmp -s want known.log || fail "the known log: got = $(cat known.log), want = $(cat want)"
echo "ok the known log gets one line per name per process, appended"

T_DIFF_KNOWN_LOG=$tmp/no/such/dir/known.log run known '#7 x'
printf 't-diff: known divergence #7 x\nt-diff: cannot open %s/no/such/dir/known.log\n' "$tmp" > werr
want "a known log that cannot be opened is reported, not fatal" 0

# A name of 300 bytes is logged whole (no fixed line buffer).
long=$(printf '%0300d' 7)
T_DIFF_KNOWN_LOG=$tmp/long.log run known "$long"
echo "t-diff: known divergence $long" > werr
want "a 300-byte name" 0
echo "$long" > want
cmp -s want long.log || fail "a 300-byte name in the known log: got = $(cat long.log), want = it whole"
echo "ok a 300-byte name is logged whole"

set --
i=0
while [ "$i" -lt 66 ]; do set -- "$@" "n$i"; i=$((i + 1)); done
run known "$@" n0 n63 n64 n65
: > werr
i=0
while [ "$i" -lt 66 ]; do echo "t-diff: known divergence n$i" >> werr; i=$((i + 1)); done
printf 't-diff: known divergence n64\nt-diff: known divergence n65\n' >> werr
want "64 names held (the 64th again is quiet), past them each call prints" 0

# --- the report's parts -----------------------------------------------------------
run units 8 61 22 5c 0 7f ff 20
printf '%s\n' '"a\"\\\x00\x7f\xff " (7 bytes)' > wout
: > werr
want "units at 8 bits" 0

run units 16 61 0 7f 100 ffff 22
printf '%s\n' '"a\x{0}\x{7f}\x{100}\x{ffff}\"" (6 units)' > wout
want "units at 16 bits" 0

run units 32 7e 10ffff ffffffff 5c
printf '%s\n' '"~\x{10ffff}\x{ffffffff}\\" (4 units)' > wout
want "units at 32 bits" 0

run units 8
printf '%s\n' '"" (0 bytes)' > wout
want "no units" 0

run units 12 61
: > wout
echo 't-diff: ppd_put_units: width 12 is not 8, 16 or 32' > werr
want "another width" 1

# No input is an empty line (the reproducer line still ends), where
# base64 prints nothing.
run b64 ''
echo > wout
: > werr
want "base64 of nothing, an empty line" 0
for s in a ab abc abcd abcde 'hello, world' '~~~???>>>'; do
  run b64 "$s"
  printf '%s' "$s" | base64 > wout
  : > werr
  want "base64 of '$s' as BusyBox's base64" 0
done
# The bytes after the input are not read: the first 1, 2 and 4 bytes of
# a longer string.
for n in 1 2 4; do
  run b64n abcdefgh "$n"
  printf '%s' abcdefgh | head -c "$n" | base64 > wout
  want "base64 of the first $n bytes of abcdefgh" 0
done

# --- init -------------------------------------------------------------------------
run die
: > wout
echo 't-diff: this and 7' > werr
want "ppd_die" 1

run noinit
echo 'fuzz-diff: ppd_open before ppd_init' > werr
want "ppd_open before ppd_init" 1

T_PREFIX='' run b64 a
echo 't-diff: ppd_init: no environment prefix' > werr
want "an empty prefix" 1

T_PREFIX=$(printf '%0201d' 0) run b64 a
echo 't-diff: ppd_init: the environment prefix is longer than 200 bytes' > werr
want "a prefix longer than 200 bytes" 1

echo "ok fuzz-diff-test"
