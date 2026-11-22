#!/bin/sh
# kit/consumers-test.sh: tests of kit/make/consumers.sh (the
# consumers-test check). It builds a shared library libfoo.so.1 with cc
# into a swap directory, a copy of it elsewhere, and programs that load
# one or the other, then checks each subcommand's exit status and
# output, spelled out by hand from the PCRE2 port's ci/consumers.sh's
# format (none is taken from a run of the script under test):
#   - loaded: the swap directory's copy (directly, through a child
#     process, under a prefix), a copy from elsewhere (DT_RPATH), from a
#     subdirectory, no load at all, the default soname list, no soname,
#     bad sonames, an unset environment and usage;
#   - fetch: a file:// download, the cache, a wrong SHA-256 (no file
#     kept), a tampered cached file, bad SHA-256s and file names, a
#     failed download and an unset cache;
#   - run: pass with an expected failure and a NOTICE, an unexpected
#     failure, no proof, SKIP with and without a reason, a non-zero
#     exit, a missing run.sh, the environment run.sh gets (default and
#     CG_FOO prefixes), packages through a stub apk, every consumer,
#     CONSUMER, bad names, a bad prefix and bad swap directories.
# Run it from the repository root. It needs a POSIX shell, BusyBox's
# tools, a C compiler with glibc (LD_DEBUG), and curl.
set -eu

script=$(pwd)/kit/make/consumers.sh
[ -f "$script" ] || { echo "FAIL: run it from the repository root (no kit/make/consumers.sh)" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"
tmp=$(pwd)

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# go [ARGS...]: the script with ARGS; its output (stdout and stderr) in
# out, its status in rc.
go() {
  if sh "$script" "$@" > out 2>&1; then rc=0; else rc=$?; fi
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

# ------------------------------------------------------------ the libraries
mkdir lib other lib/sub nolib
printf 'int foo(void) { return 42; }\n' > foo.c
cc -shared -fPIC -Wl,-soname,libfoo.so.1 -o lib/libfoo.so.1 foo.c
cp lib/libfoo.so.1 other/libfoo.so.1
cp lib/libfoo.so.1 lib/sub/libfoo.so.1
printf 'int foo(void);\nint main(void) { return foo() == 42 ? 0 : 1; }\n' > prog.c
cc -o prog prog.c "$tmp/lib/libfoo.so.1"
# DT_RPATH (not RUNPATH) is searched before LD_LIBRARY_PATH.
cc -o prog-other prog.c "$tmp/lib/libfoo.so.1" -Wl,--disable-new-dtags,-rpath,"$tmp/other"
cc -o prog-sub prog.c "$tmp/lib/libfoo.so.1" -Wl,--disable-new-dtags,-rpath,"$tmp/lib/sub"
printf 'int main(void) { return 3; }\n' > none.c
cc -o prog-none none.c
# Another library whose name starts with libfoo.so: not libfoo.so.1.
mkdir solo
cc -shared -fPIC -Wl,-soname,libfoo.solo -o solo/libfoo.solo foo.c
cc -o prog-solo prog.c "$tmp/solo/libfoo.solo" -Wl,--disable-new-dtags,-rpath,"$tmp/solo"
printf '#!/bin/sh\n"%s/prog"\n' "$tmp" > parent
chmod +x parent
LD_LIBRARY_PATH=$tmp/lib ./prog || fail "the test program does not run on libfoo.so.1"

mkdir work
# A loaded run's environment, as consumers.sh run gives it.
lenv() { env CONSUMER_LIB="$tmp/lib" CONSUMER_WORK="$tmp/work" "$@"; }

# ------------------------------------------------------------------ loaded
lgo() {
  if lenv sh "$script" loaded "$@" > out 2>&1; then rc=0; else rc=$?; fi
}

lgo --lib libfoo.so.1 "$tmp/prog"
echo "loaded: $tmp/prog: $tmp/lib/libfoo.so.1" > want
want "loaded: the swap directory's libfoo.so.1" 0
echo "loaded: $tmp/prog: $tmp/lib/libfoo.so.1" > want
cmp -s want work/.consumers-loaded || { cat work/.consumers-loaded >&2; fail "loaded: the proof file"; }
echo "ok loaded: the proof file"

lgo --lib=libfoo.so.1 "$tmp/parent"
echo "loaded: $tmp/parent: $tmp/lib/libfoo.so.1" > want
want "loaded: a child process counts, --lib=" 0

if lenv CONSUMERS_SONAME=libfoo.so.1 sh "$script" loaded "$tmp/prog" > out 2>&1; then rc=0; else rc=$?; fi
echo "loaded: $tmp/prog: $tmp/lib/libfoo.so.1" > want
want "loaded: CONSUMERS_SONAME is the default" 0

lgo --lib libfoo.so.1 -- "$tmp/prog"
echo "loaded: $tmp/prog: $tmp/lib/libfoo.so.1" > want
want "loaded: -- ends the options" 0

if CONSUMERS_PREFIX=CG_FOO CG_FOO_LIB="$tmp/lib/" CG_FOO_WORK="$tmp/work" \
  sh "$script" loaded --lib libfoo.so.1 "$tmp/prog" > out 2>&1; then rc=0; else rc=$?; fi
echo "loaded: $tmp/prog: $tmp/lib/libfoo.so.1" > want
want "loaded: the CG_FOO prefix, LIB with a trailing /" 0

lgo --lib libfoo.so.1 "$tmp/prog-other"
echo "consumers: loaded: $tmp/prog-other loaded $tmp/other/libfoo.so.1, not libfoo.so.1 from $tmp/lib" > want
want "loaded: a copy from elsewhere fails" 1

lgo --lib libfoo.so.1 "$tmp/prog-sub"
echo "consumers: loaded: $tmp/prog-sub loaded $tmp/lib/sub/libfoo.so.1, not libfoo.so.1 directly from $tmp/lib" > want
want "loaded: a copy in a subdirectory of LIB fails" 1

lgo --lib libfoo.so.1 "$tmp/prog-none"
echo "consumers: loaded: $tmp/prog-none did not load libfoo.so.1" > want
want "loaded: a program that does not load it fails (its exit 3 ignored)" 1

lgo --lib libfoo.so.1 --lib libbar.so.2 "$tmp/prog"
echo "consumers: loaded: $tmp/prog did not load libbar.so.2" > want
want "loaded: every --lib must load" 1

lgo --lib libfoo.so.1 "$tmp/prog-solo"
echo "consumers: loaded: $tmp/prog-solo did not load libfoo.so.1" > want
want "loaded: libfoo.solo is not libfoo.so.1" 1

lgo "$tmp/prog"
echo "consumers: loaded: no SONAME: pass --lib SONAME or set CONSUMERS_SONAME" > want
want "loaded: no soname is usage" 2

if lenv CONSUMERS_SONAME=' ' sh "$script" loaded "$tmp/prog" > out 2>&1; then rc=0; else rc=$?; fi
want "loaded: a blank CONSUMERS_SONAME is no soname" 2
tab=$(printf '\t')
if lenv CONSUMERS_SONAME="$tab" sh "$script" loaded "$tmp/prog" > out 2>&1; then rc=0; else rc=$?; fi
want "loaded: a tab CONSUMERS_SONAME is no soname" 2

lgo --lib libfoo.so "$tmp/prog"
echo "consumers: loaded: 'libfoo.so' is not a lib*.so.N soname" > want
want "loaded: a soname without a version is usage" 2

lgo --lib ../libfoo.so.1 "$tmp/prog"
echo "consumers: loaded: bad library name '../libfoo.so.1'" > want
want "loaded: a path is no soname" 2

if lenv CONSUMERS_SONAME='libfoo.so.1 x/y' sh "$script" loaded "$tmp/prog" > out 2>&1; then rc=0; else rc=$?; fi
echo "consumers: loaded: bad library name 'x/y'" > want
want "loaded: CONSUMERS_SONAME's words are checked" 2

lgo --lib libfoo.so.1
echo "consumers: usage: consumers.sh loaded [--lib SONAME]... PROGRAM [ARG...]" > want
want "loaded: no PROGRAM is usage" 2

lgo --lib
want "loaded: --lib with no SONAME is usage" 2

if sh "$script" loaded --lib libfoo.so.1 "$tmp/prog" > out 2>&1; then rc=0; else rc=$?; fi
echo "consumers: loaded: CONSUMER_LIB is not set (loaded runs under consumers.sh run)" > want
want "loaded: no CONSUMER_LIB is usage" 2

if CONSUMER_LIB="$tmp/lib" sh "$script" loaded --lib libfoo.so.1 "$tmp/prog" > out 2>&1; then rc=0; else rc=$?; fi
echo "consumers: loaded: CONSUMER_WORK is not set (loaded runs under consumers.sh run)" > want
want "loaded: no CONSUMER_WORK is usage" 2

if CONSUMER_LIB="$tmp/a:b" CONSUMER_WORK="$tmp/work" sh "$script" loaded --lib libfoo.so.1 "$tmp/prog" > out 2>&1; then rc=0; else rc=$?; fi
echo "consumers: CONSUMER_LIB '$tmp/a:b' has a character the harness cannot carry (blank, :, quote, ...)" > want
want "loaded: a LIB with : is usage" 2

# ------------------------------------------------------------------- fetch
mkdir cache
printf 'kit consumers test\n' > blob
sum=$(sha256sum blob | cut -d' ' -f1)
bad=0000000000000000000000000000000000000000000000000000000000000000
fgo() {
  if CONSUMER_CACHE="$tmp/cache" sh "$script" fetch "$@" > out 2> err; then rc=0; else rc=$?; fi
}

fgo "file://$tmp/blob" "$sum"
echo "$tmp/cache/blob" > want
want "fetch: a download, named after the URL" 0
cmp -s blob cache/blob || fail "fetch: the downloaded file differs"
grep -qxF "fetch: file://$tmp/blob" err || { cat err >&2; fail "fetch: the URL on stderr"; }

fgo "file://$tmp/blob" "$sum"
echo "$tmp/cache/blob" > want
want "fetch: from the cache" 0
grep -qxF "fetch: blob (cached, SHA-256 ok)" err || { cat err >&2; fail "fetch: the cache line on stderr"; }

fgo "file://$tmp/blob" "$sum" ok
echo "$tmp/cache/ok" > want
want "fetch: FILE names the cached file" 0

printf 'tampered\n' > cache/ok
fgo "file://$tmp/blob" "$sum" ok
echo "$tmp/cache/ok" > want
want "fetch: a tampered cached file is fetched again" 0
cmp -s blob cache/ok || fail "fetch: the tampered file was kept"

fgo "file://$tmp/blob" "$bad" wrong
cat err > out
echo "consumers: fetch: SHA-256 mismatch for file://$tmp/blob: got $sum, want $bad" > want
grep -v '^fetch: ' out > out2 || true
mv out2 out
want "fetch: a wrong SHA-256 fails" 1
[ ! -e cache/wrong ] && [ ! -e cache/wrong.part ] || fail "fetch: a file with a wrong SHA-256 was kept"
echo "ok fetch: a wrong SHA-256 keeps no file"

fgo "file://$tmp/nosuch" "$sum" gone
grep -v '^fetch: \|^curl: ' err > out || true
echo "consumers: fetch: download of file://$tmp/nosuch failed" > want
want "fetch: a failed download fails" 1
[ ! -e cache/gone ] && [ ! -e cache/gone.part ] || fail "fetch: a failed download kept a file"

# No curl on PATH: a cached file still passes; a download fails, saying so.
mkdir nocurl
for t in sh sha256sum cut mkdir rm mv cat; do ln -s "$(command -v "$t")" "nocurl/$t"; done
if CONSUMER_CACHE="$tmp/cache" PATH="$tmp/nocurl" sh "$script" fetch "file://$tmp/blob" "$sum" > out 2> err; then rc=0; else rc=$?; fi
echo "$tmp/cache/blob" > want
want "fetch: no curl, a cached file" 0
if CONSUMER_CACHE="$tmp/cache" PATH="$tmp/nocurl" sh "$script" fetch "file://$tmp/blob" "$sum" new > out 2>&1; then rc=0; else rc=$?; fi
echo "consumers: fetch: no curl to download file://$tmp/blob (list curl in the consumer's packages file or the check's environment)" > want
want "fetch: no curl, a download fails" 1
[ ! -e cache/new ] || fail "fetch: no curl, a file was written"

fgo "file://$tmp/blob" "$(echo "$sum" | tr a-f A-F)"
cat err > out
echo "consumers: fetch: '$(echo "$sum" | tr a-f A-F)' is not a lowercase hex SHA-256" > want
want "fetch: an uppercase SHA-256 fails" 1

fgo "file://$tmp/blob" abc
cat err > out
echo "consumers: fetch: 'abc' is not a SHA-256" > want
want "fetch: a short SHA-256 fails" 1

for f in . .. a/b -x; do
  fgo "file://$tmp/blob" "$sum" "$f"
  cat err > out
  echo "consumers: fetch: bad file name '$f'" > want
  want "fetch: file name '$f' fails" 1
done

fgo "file://$tmp/" "$sum"
cat err > out
echo "consumers: fetch: bad file name ''" > want
want "fetch: a URL ending in / needs FILE" 1

fgo "file://$tmp/blob"
cat err > out
echo "consumers: usage: consumers.sh fetch URL SHA256 [FILE]" > want
want "fetch: no SHA256 is usage" 2

if sh "$script" fetch "file://$tmp/blob" "$sum" > out 2>&1; then rc=0; else rc=$?; fi
echo "consumers: fetch: CONSUMER_CACHE is not set (fetch runs under consumers.sh run)" > want
want "fetch: no CONSUMER_CACHE is usage" 2

# --------------------------------------------------------------------- run
mkdir -p repo/tests/consumers repo/swap
cp lib/libfoo.so.1 repo/swap/
cp prog repo/prog
c=repo/tests/consumers
mkc() { mkdir -p "$c/$1"; cat > "$c/$1/run.sh"; }

mkc good <<'EOF'
echo "good runs"
sh "$CONSUMER_HARNESS" loaded --lib libfoo.so.1 "$CONSUMER_ROOT/prog"
echo t1 >> "$CONSUMER_FAILURES"
echo t1 >> "$CONSUMER_FAILURES"
EOF
cat > "$c/good/expected-failures.txt" <<'EOF'
# known
t1  # fails with the C library too
  t2 extra words
EOF
mkc bad <<'EOF'
sh "$CONSUMER_HARNESS" loaded --lib libfoo.so.1 "$CONSUMER_ROOT/prog"
printf 't3\nt1\n\n' >> "$CONSUMER_FAILURES"
EOF
echo t1 > "$c/bad/expected-failures.txt"
mkc noproof <<'EOF'
echo "no proof"
EOF
mkc skip <<'EOF'
echo "SKIP first"
echo "SKIP no thing here"
exit 77
EOF
mkc skipnoreason <<'EOF'
exit 77
EOF
mkc exit3 <<'EOF'
echo "broken" >&2
exit 3
EOF
mkc failedproof <<'EOF'
sh "$CONSUMER_HARNESS" loaded --lib libbar.so.1 "$CONSUMER_ROOT/prog" || true
EOF
mkdir "$c/norun"

rgo() {
  if (cd repo && sh "$script" run "$@") > out 2>&1; then rc=0; else rc=$?; fi
}

w=$tmp/repo/target/consumers
rgo swap good
cat > want <<EOF
swap directory $tmp/repo/swap: libfoo.so.1
== consumer good
good runs
loaded: $tmp/repo/prog: $tmp/repo/swap/libfoo.so.1
NOTICE good: t2 is in expected-failures.txt but passed; remove its line
PASS good: 1 expected failures
== summary
PASS good: 1 expected failures
consumers: 1 passed, 0 failed, 0 skipped
EOF
want "run: an expected failure passes, a listed id that passed is a NOTICE" 0
printf 'good runs\nloaded: %s/repo/prog: %s/repo/swap/libfoo.so.1\n' "$tmp" "$tmp" > want
cmp -s want "$w/good/log" || { cat "$w/good/log" >&2; fail "run: the consumer's log"; }
echo "ok run: the consumer's log"

rgo swap bad
cat > want <<EOF
swap directory $tmp/repo/swap: libfoo.so.1
== consumer bad
loaded: $tmp/repo/prog: $tmp/repo/swap/libfoo.so.1
FAIL bad: unexpected failures: t3
== summary
FAIL bad: unexpected failures: t3
consumers: 0 passed, 1 failed, 0 skipped
EOF
want "run: an unlisted failure fails" 1

rgo swap noproof skip skipnoreason exit3 failedproof norun nosuch
cat > want <<EOF
swap directory $tmp/repo/swap: libfoo.so.1
== consumer noproof
no proof
FAIL noproof: run.sh did not prove the port's library was loaded (sh "\$CONSUMER_HARNESS" loaded ...)
== consumer skip
SKIP first
SKIP no thing here
SKIP skip: no thing here
== consumer skipnoreason
SKIP skipnoreason: run.sh exited 77
== consumer exit3
broken
FAIL exit3: run.sh exited 3 (log: $w/exit3/log)
== consumer failedproof
consumers: loaded: $tmp/repo/prog did not load libbar.so.1
FAIL failedproof: run.sh did not prove the port's library was loaded (sh "\$CONSUMER_HARNESS" loaded ...)
== consumer norun
FAIL norun: no $tmp/repo/tests/consumers/norun/run.sh
== consumer nosuch
FAIL nosuch: no $tmp/repo/tests/consumers/nosuch/run.sh
== summary
FAIL noproof: run.sh did not prove the port's library was loaded (sh "\$CONSUMER_HARNESS" loaded ...)
SKIP skip: no thing here
SKIP skipnoreason: run.sh exited 77
FAIL exit3: run.sh exited 3 (log: $w/exit3/log)
FAIL failedproof: run.sh did not prove the port's library was loaded (sh "\$CONSUMER_HARNESS" loaded ...)
FAIL norun: no $tmp/repo/tests/consumers/norun/run.sh
FAIL nosuch: no $tmp/repo/tests/consumers/nosuch/run.sh
consumers: 0 passed, 5 failed, 2 skipped
EOF
want "run: no proof, SKIP, a non-zero exit, a failed proof, no run.sh" 1

# The environment run.sh gets, with a fresh work directory.
mkc envdump <<'EOF'
for v in LIB ROOT WORK FAILURES CACHE HARNESS; do
  eval "echo $v=\${${CONSUMERS_PREFIX}_$v}"
done
echo "PREFIX=$CONSUMERS_PREFIX SONAME=$CONSUMERS_SONAME"
echo "PWD=$(pwd)"
ls -A | grep -v '^log$'
read -r x || echo "stdin empty"
sh "$(eval "echo \${${CONSUMERS_PREFIX}_HARNESS}")" loaded "$(eval "echo \${${CONSUMERS_PREFIX}_ROOT}")/prog"
EOF
mkdir -p "$w/envdump"
: > "$w/envdump/stale"
# The runner's own stdin has a line, which run.sh must not see.
echo "a line on the runner's stdin" > stdin
if (cd repo && CONSUMERS_SONAME=libfoo.so.1 sh "$script" run swap/ envdump) < stdin > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<EOF
swap directory $tmp/repo/swap: libfoo.so.1
== consumer envdump
LIB=$tmp/repo/swap
ROOT=$tmp/repo
WORK=$w/envdump
FAILURES=$w/envdump/failures
CACHE=$w/cache
HARNESS=$script
PREFIX=CONSUMER SONAME=libfoo.so.1
PWD=$w/envdump
failures
stdin empty
loaded: $tmp/repo/prog: $tmp/repo/swap/libfoo.so.1
PASS envdump: 0 expected failures
== summary
PASS envdump: 0 expected failures
consumers: 1 passed, 0 failed, 0 skipped
EOF
want "run: the environment, a fresh work directory, stdin /dev/null" 0

if (cd repo && CONSUMERS_PREFIX=CG_FOO CONSUMERS_SONAME=libfoo.so.1 CONSUMERS_WORK="$tmp/w2" \
  sh "$script" run "$tmp/repo/swap" envdump) > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<EOF
swap directory $tmp/repo/swap: libfoo.so.1
== consumer envdump
LIB=$tmp/repo/swap
ROOT=$tmp/repo
WORK=$tmp/w2/envdump
FAILURES=$tmp/w2/envdump/failures
CACHE=$tmp/w2/cache
HARNESS=$script
PREFIX=CG_FOO SONAME=libfoo.so.1
PWD=$tmp/w2/envdump
failures
stdin empty
loaded: $tmp/repo/prog: $tmp/repo/swap/libfoo.so.1
PASS envdump: 0 expected failures
== summary
PASS envdump: 0 expected failures
consumers: 1 passed, 0 failed, 0 skipped
EOF
want "run: the CG_FOO prefix and CONSUMERS_WORK" 0

# CONSUMERS_DIR, CONSUMER, and every consumer.
mkdir -p repo/other/a repo/other/b repo/other/c
for n in a b; do
  # shellcheck disable=SC2016 # run.sh expands them
  printf 'sh "$CONSUMER_HARNESS" loaded --lib libfoo.so.1 "$CONSUMER_ROOT/prog" > /dev/null\n' > "repo/other/$n/run.sh"
done
if (cd repo && CONSUMERS_DIR=other sh "$script" run swap) > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<EOF
swap directory $tmp/repo/swap: libfoo.so.1
== consumer a
PASS a: 0 expected failures
== consumer b
PASS b: 0 expected failures
== summary
PASS a: 0 expected failures
PASS b: 0 expected failures
consumers: 2 passed, 0 failed, 0 skipped
EOF
want "run: no NAME runs every DIR/*/run.sh (not c, which has none)" 0

if (cd repo && CONSUMERS_DIR=other CONSUMER=b sh "$script" run swap) > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<EOF
swap directory $tmp/repo/swap: libfoo.so.1
== consumer b
PASS b: 0 expected failures
== summary
PASS b: 0 expected failures
consumers: 1 passed, 0 failed, 0 skipped
EOF
want "run: CONSUMER names the one to run" 0

if (cd repo && CONSUMERS_DIR=other/c sh "$script" run swap) > out 2>&1; then rc=0; else rc=$?; fi
echo "consumers: run: no consumers to run (no $tmp/repo/other/c/*/run.sh)" > want
want "run: no consumers at all fails" 1

if (cd repo && CONSUMERS_DIR=nosuch sh "$script" run swap) > out 2>&1; then rc=0; else rc=$?; fi
echo "consumers: run: no consumer directory $tmp/repo/nosuch (CONSUMERS_DIR)" > want
want "run: a missing CONSUMERS_DIR fails" 1

# Packages, through a stub apk that has pkg-a installed and logs adds.
mkdir stub
cat > stub/apk <<EOF
#!/bin/sh
echo "apk \$*" >> "$tmp/apk.log"
case "\$*" in
  "info -e pkg-a") exit 0 ;;
  "info -e "*) exit 1 ;;
  "add --no-cache pkg-b pkg-c") exit 0 ;;
  "add --no-cache pkg-d") exit 1 ;;
  "add --no-cache --repositories-file /dev/null --repository https://packages.wolfi.dev/os pkg-d") exit 1 ;;
esac
exit 9
EOF
printf '#!/bin/sh\necho 0\n' > stub/id
chmod +x stub/apk stub/id
mkdir -p "$c/pk" "$c/pkfail"
cp "$c/good/run.sh" "$c/pk/run.sh"
cp "$c/good/expected-failures.txt" "$c/pk/"
printf '# needs\npkg-a  # installed\npkg-b pkg-c\n\n' > "$c/pk/packages"
cp "$c/pk/run.sh" "$c/pkfail/run.sh"
echo pkg-d > "$c/pkfail/packages"
if (cd repo && PATH=$tmp/stub:$PATH sh "$script" run swap pk pkfail) > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<EOF
swap directory $tmp/repo/swap: libfoo.so.1
== consumer pk
apk add pkg-b pkg-c
good runs
loaded: $tmp/repo/prog: $tmp/repo/swap/libfoo.so.1
NOTICE pk: t2 is in expected-failures.txt but passed; remove its line
PASS pk: 1 expected failures
== consumer pkfail
apk add pkg-d
apk add failed with repositories [$(paste -sd ' ' /etc/apk/repositories 2> /dev/null || true)]; retrying from https://packages.wolfi.dev/os only
FAIL pkfail: could not install its packages
== summary
PASS pk: 1 expected failures
FAIL pkfail: could not install its packages
consumers: 1 passed, 1 failed, 0 skipped
EOF
want "run: packages, only the missing ones; a failed install fails" 1
cat > want <<EOF
apk info -e pkg-a
apk info -e pkg-b
apk info -e pkg-c
apk add --no-cache pkg-b pkg-c
apk info -e pkg-d
apk add --no-cache pkg-d
apk add --no-cache --repositories-file /dev/null --repository https://packages.wolfi.dev/os pkg-d
EOF
cmp -s want apk.log || { cat apk.log >&2; fail "run: the apk calls"; }
echo "ok run: the apk calls"

printf '#!/bin/sh\necho 1000\n' > stub/id
if (cd repo && PATH=$tmp/stub:$PATH sh "$script" run swap pkfail) > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<EOF
swap directory $tmp/repo/swap: libfoo.so.1
== consumer pkfail
consumers: missing packages: pkg-d (and no apk as root to install them)
FAIL pkfail: could not install its packages
== summary
FAIL pkfail: could not install its packages
consumers: 0 passed, 1 failed, 0 skipped
EOF
want "run: missing packages and not root fails" 1

# No curl and no apk: the run goes on, saying so; a consumer that does not
# fetch passes, one that fetches fails.
mkdir nocurl2
for t in sh ls paste sort comm awk sed tee wc tr env mkdir mktemp rm cat mv cut sha256sum tail readlink; do
  ln -s "$(command -v "$t")" "nocurl2/$t"
done
mkc fetcher <<EOF
sh "\$CONSUMER_HARNESS" loaded --lib libfoo.so.1 "\$CONSUMER_ROOT/prog" > /dev/null
sh "\$CONSUMER_HARNESS" fetch "file://$tmp/blob" "$sum" > /dev/null
EOF
if (cd repo && PATH=$tmp/nocurl2 sh "$script" run swap good fetcher) > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<EOF
consumers: missing packages: curl (and no apk as root to install them)
consumers: no curl: a consumer that calls fetch fails
swap directory $tmp/repo/swap: libfoo.so.1
== consumer good
good runs
loaded: $tmp/repo/prog: $tmp/repo/swap/libfoo.so.1
NOTICE good: t2 is in expected-failures.txt but passed; remove its line
PASS good: 1 expected failures
== consumer fetcher
consumers: fetch: no curl to download file://$tmp/blob (list curl in the consumer's packages file or the check's environment)
FAIL fetcher: run.sh exited 1 (log: $w/fetcher/log)
== summary
PASS good: 1 expected failures
FAIL fetcher: run.sh exited 1 (log: $w/fetcher/log)
consumers: 1 passed, 1 failed, 0 skipped
EOF
want "run: no curl, no apk: the run goes on; fetch fails" 1

# A run.sh that records a failure, then removes everything in its cwd
# (the proof file, a dotfile, stays): its failures file is gone, which
# fails rather than reading as no failures.
mkc wipe <<'EOF'
sh "$CONSUMER_HARNESS" loaded --lib libfoo.so.1 "$CONSUMER_ROOT/prog" > /dev/null
echo realfailure >> "$CONSUMER_FAILURES"
rm -rf ./*
EOF
# An expected-failures.txt that cannot be read (a directory).
mkc expdir <<'EOF'
sh "$CONSUMER_HARNESS" loaded --lib libfoo.so.1 "$CONSUMER_ROOT/prog" > /dev/null
echo t1 >> "$CONSUMER_FAILURES"
EOF
mkdir "$c/expdir/expected-failures.txt"
if (cd repo && sh "$script" run swap wipe expdir) > out 2>&1; then rc=0; else rc=$?; fi
grep -v '^sed: ' out > out2 || true
mv out2 out
cat > want <<EOF
swap directory $tmp/repo/swap: libfoo.so.1
== consumer wipe
FAIL wipe: cannot read its failures file (\$CONSUMER_FAILURES, $w/wipe/failures)
== consumer expdir
FAIL expdir: cannot read its expected-failures.txt
== summary
FAIL wipe: cannot read its failures file (\$CONSUMER_FAILURES, $w/wipe/failures)
FAIL expdir: cannot read its expected-failures.txt
consumers: 0 passed, 2 failed, 0 skipped
EOF
want "run: a removed failures file and an unreadable expected-failures.txt fail" 1

# Links in LIB: within LIB they pass; to a file outside LIB, or
# dangling, the run fails before any consumer runs.
mkdir repo/swapl repo/swapx repo/swapd
cp lib/libfoo.so.1 repo/swapl/libfoo.so.1.2.3
ln -s libfoo.so.1.2.3 repo/swapl/libfoo.so.1
if (cd repo && sh "$script" run swapl good) > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<EOF
swap directory $tmp/repo/swapl: libfoo.so.1 libfoo.so.1.2.3
== consumer good
good runs
loaded: $tmp/repo/prog: $tmp/repo/swapl/libfoo.so.1
NOTICE good: t2 is in expected-failures.txt but passed; remove its line
PASS good: 1 expected failures
== summary
PASS good: 1 expected failures
consumers: 1 passed, 0 failed, 0 skipped
EOF
want "run: a link within LIB passes" 0
ln -s "$tmp/other/libfoo.so.1" repo/swapx/libfoo.so.1
rgo swapx good
echo "consumers: run: $tmp/repo/swapx/libfoo.so.1 resolves to $tmp/other/libfoo.so.1, outside LIB $tmp/repo/swapx: copy the port's libraries into LIB, with links only to files there" > want
want "run: a link to a copy outside LIB fails" 1
ln -s "$tmp/other/libfoo.so.1" repo/swapx/.hidden
rm repo/swapx/libfoo.so.1
cp lib/libfoo.so.1 repo/swapx/
rgo swapx good
echo "consumers: run: $tmp/repo/swapx/.hidden resolves to $tmp/other/libfoo.so.1, outside LIB $tmp/repo/swapx: copy the port's libraries into LIB, with links only to files there" > want
want "run: a hidden link outside LIB fails" 1
cp lib/libfoo.so.1 repo/swapd/
ln -s libfoo.so.1.9 repo/swapd/libfoo.so.1.x
rgo swapd good
echo "consumers: run: $tmp/repo/swapd/libfoo.so.1.x is a dangling link" > want
want "run: a dangling link in LIB fails" 1

# Usage and bad swap directories.
for n in Bad a_b -x ''; do
  rgo swap "$n"
  echo "consumers: run: bad consumer name '$n'" > want
  want "run: name '$n' is usage" 2
done
rgo
echo "consumers: usage: consumers.sh run LIB [NAME...]" > want
want "run: no LIB is usage" 2
rgo nosuch good
echo "consumers: run: LIB 'nosuch' is not a directory" > want
want "run: a missing LIB fails" 1
mkdir repo/empty
rgo empty good
echo "consumers: run: LIB $tmp/repo/empty is empty: copy the port's libraries into it first" > want
want "run: an empty LIB fails" 1
mkdir 'repo/a b'
cp lib/libfoo.so.1 'repo/a b/'
rgo 'a b' good
echo "consumers: LIB '$tmp/repo/a b' has a character the harness cannot carry (blank, :, quote, ...)" > want
want "run: a LIB with a blank is usage" 2
if (cd repo && CONSUMERS_PREFIX=cg sh "$script" run swap good) > out 2>&1; then rc=0; else rc=$?; fi
echo "consumers: CONSUMERS_PREFIX 'cg' must start with a capital letter" > want
want "run: a lowercase prefix is usage" 2
if (cd repo && CONSUMERS_PREFIX='A;B' sh "$script" run swap good) > out 2>&1; then rc=0; else rc=$?; fi
echo "consumers: CONSUMERS_PREFIX 'A;B' must be capitals, digits and _" > want
want "run: a prefix with ; is usage" 2

go
echo "consumers: usage: consumers.sh run LIB [NAME...] | fetch URL SHA256 [FILE] | loaded [--lib SONAME]... PROGRAM [ARG...]" > want
want "no subcommand is usage" 2
go --help
head -n 1 out | grep -q '^consumers.sh run LIB' || { cat out >&2; fail "--help prints the header"; }
echo "ok --help prints the header"

echo "PASS"
