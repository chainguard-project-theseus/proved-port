#!/bin/sh
# consumers.sh run LIB [NAME...] | fetch URL SHA256 [FILE] | loaded
# [--lib SONAME]... PROGRAM [ARG...]: the consumer harness (`make
# consumers-run`, proved-port.mk), which the theseus-proved-port package
# installs in /usr/share/proved-port/make/. The PCRE2 port's
# ci/consumers.sh runner and helpers, with its build, its swap directory
# and its version check left to the port: the port builds its libraries,
# copies them (and nothing else) into the swap directory LIB, checks
# them as it likes, then runs `consumers.sh run LIB`. run refuses a LIB
# with an entry that resolves (readlink -f) outside LIB, or a dangling
# link: the loader names the entry, not its target, so a link to the
# system's copy would pass `loaded`. Links within LIB are fine.
#
# run LIB [NAME...]
#   Runs DIR/<name>/run.sh for each NAME (DIR is $CONSUMERS_DIR, default
#   tests/consumers, relative to the current directory, the repository
#   root) and prints `PASS|FAIL|SKIP <name>` lines and a summary,
#   `consumers: P passed, F failed, S skipped`. With no NAME it runs
#   $CONSUMER when set and not empty, or else every DIR/*/run.sh. A name
#   is lowercase letters, digits and `-`. Per consumer it installs the
#   Wolfi packages the consumer's `packages` file lists (one or more per
#   line, `#` comments; apk as root, only the missing ones), runs run.sh
#   with sh in an empty WORK/<name>/ (WORK is $CONSUMERS_WORK, default
#   target/consumers; run.sh's output is also in its `log`), and then
#   requires:
#
#     - exit 0 (77 is SKIP, with run.sh's last `SKIP ...` line as the
#       reason; anything else fails);
#     - proof that the port's library was loaded: run.sh called
#       `loaded` (below) and it passed;
#     - every test id run.sh appended to $P_FAILURES is listed in the
#       consumer's expected-failures.txt (one id per line, its first
#       word, `#` comments). A listed id that passed prints a NOTICE
#       (remove its line). Ids compare as whole lines, bytewise
#       (LC_ALL=C), so an id with blanks can never be listed: it fails.
#
#   A consumer with no run.sh fails. Exit 1 if any consumer failed.
#
# run.sh gets, in its environment, with P the prefix $CONSUMERS_PREFIX
# (default CONSUMER; capitals, digits and `_`, so a port's existing
# run.sh files keep their names, e.g. CG_FOO):
#
#   P_LIB       LIB, as an absolute path
#   P_ROOT      the repository root (the current directory)
#   P_WORK      the consumer's empty scratch directory (also the cwd)
#   P_FAILURES  the file to append failing test ids to, one per line
#   P_CACHE     the download cache WORK/cache, kept across runs
#   P_HARNESS   this script, for the helpers: sh "$P_HARNESS" fetch ...
#
# and CONSUMERS_PREFIX and CONSUMERS_SONAME, which the helpers read.
#
# fetch URL SHA256 [FILE]
#   Downloads URL (curl) into $P_CACHE/FILE (default: the URL's last
#   path segment) unless a file with that SHA-256 is already there,
#   verifies the SHA-256 (64 lowercase hex digits) and prints the path.
#   A cached file is checked again; a mismatch fails and keeps no file.
#
# loaded [--lib SONAME]... PROGRAM [ARG...]
#   Runs PROGRAM with LD_DEBUG=libs and LD_LIBRARY_PATH=$P_LIB (its exit
#   status is ignored; stdin, stdout and stderr are /dev/null) and passes
#   only if the dynamic loader initialized each SONAME (default: the
#   words of $CONSUMERS_SONAME; none at all is a usage error) from a file
#   directly in $P_LIB and from nowhere else, in PROGRAM's process or any
#   it starts. Any file of the library counts: for libfoo.so.1, libfoo.so
#   and libfoo.so.* (a dlopen of the real file name). On a pass it
#   prints `loaded: PROGRAM: PATH...` and records it in $P_WORK. glibc's
#   loader only (LD_DEBUG).
#
# Exit status: 0 pass, 1 fail, 2 usage. Needs sh, sha256sum, readlink -f
# (BusyBox's or GNU's), curl (for
# fetch; run installs it when missing and apk runs as root, and goes on
# without it otherwise), and to install packages, apk as root.
set -eu

die() { echo "consumers: $*" >&2; exit 1; }
usage() { echo "consumers: $*" >&2; exit 2; }

self=$0
case $self in
  /*) ;;
  *) self=$(cd "$(dirname "$self")" && pwd)/${self##*/} ;;
esac

prefix=${CONSUMERS_PREFIX:-CONSUMER}
case $prefix in
  [A-Z]*) ;;
  *) usage "CONSUMERS_PREFIX '$prefix' must start with a capital letter" ;;
esac
case $prefix in
  *[!A-Z0-9_]*) usage "CONSUMERS_PREFIX '$prefix' must be capitals, digits and _" ;;
esac

# var NAME: the value of $P_NAME, empty when unset.
var() { eval "printf '%s' \"\${${prefix}_$1:-}\""; }

# A path the loader and the word lists below can carry.
plain_path() {
  case $2 in
    "") usage "$1 is empty" ;;
    *[!A-Za-z0-9._/+@=,-]*) usage "$1 '$2' has a character the harness cannot carry (blank, :, quote, ...)" ;;
  esac
}

sha256_of() { sha256sum "$1" | cut -d' ' -f1; }

# ---------------------------------------------------------------- helpers
fetch() {
  [ $# -ge 2 ] && [ $# -le 3 ] || usage "usage: consumers.sh fetch URL SHA256 [FILE]"
  url=$1 want=$2
  file=${3:-${url##*/}}
  cache=$(var CACHE)
  [ -n "$cache" ] || usage "fetch: ${prefix}_CACHE is not set (fetch runs under consumers.sh run)"
  case $want in
    *[!0-9a-f]*) die "fetch: '$want' is not a lowercase hex SHA-256" ;;
  esac
  [ ${#want} = 64 ] || die "fetch: '$want' is not a SHA-256"
  case $file in
    "" | . | .. | */* | -*) die "fetch: bad file name '$file'" ;;
  esac
  mkdir -p "$cache"
  dest=$cache/$file
  if [ -f "$dest" ] && [ "$(sha256_of "$dest")" = "$want" ]; then
    echo "fetch: $file (cached, SHA-256 ok)" >&2
    echo "$dest"
    return 0
  fi
  command -v curl > /dev/null 2>&1 ||
    die "fetch: no curl to download $url (list curl in the consumer's packages file or the check's environment)"
  rm -f "$dest" "$dest.part"
  echo "fetch: $url" >&2
  curl -sSfL --retry 3 -o "$dest.part" -- "$url" || {
    rm -f "$dest.part"
    die "fetch: download of $url failed"
  }
  got=$(sha256_of "$dest.part") || { rm -f "$dest.part"; die "fetch: cannot hash $dest.part"; }
  if [ "$got" != "$want" ]; then
    rm -f "$dest.part"
    die "fetch: SHA-256 mismatch for $url: got $got, want $want"
  fi
  mv "$dest.part" "$dest"
  echo "$dest"
}

check_soname() {
  case $1 in
    *[!A-Za-z0-9._+-]* | *..*) usage "loaded: bad library name '$1'" ;;
    lib?*.so.[0-9]*) ;;
    *) usage "loaded: '$1' is not a lib*.so.N soname" ;;
  esac
}

loaded() {
  libs=
  while [ $# -gt 0 ]; do
    case $1 in
      --lib) [ $# -ge 2 ] || usage "usage: consumers.sh loaded [--lib SONAME]... PROGRAM [ARG...]"
        check_soname "$2"; libs="$libs $2"; shift 2 ;;
      --lib=*) check_soname "${1#--lib=}"; libs="$libs ${1#--lib=}"; shift ;;
      --) shift; break ;;
      *) break ;;
    esac
  done
  [ $# -ge 1 ] || usage "usage: consumers.sh loaded [--lib SONAME]... PROGRAM [ARG...]"
  lib=$(var LIB)
  work=$(var WORK)
  [ -n "$lib" ] || usage "loaded: ${prefix}_LIB is not set (loaded runs under consumers.sh run)"
  [ -n "$work" ] || usage "loaded: ${prefix}_WORK is not set (loaded runs under consumers.sh run)"
  plain_path "${prefix}_LIB" "$lib"
  lib=${lib%/}
  [ -n "$libs" ] || libs=${CONSUMERS_SONAME:-}
  n=0
  for l in $libs; do check_soname "$l"; n=$((n + 1)); done
  [ "$n" -gt 0 ] || usage "loaded: no SONAME: pass --lib SONAME or set CONSUMERS_SONAME"
  ld=$(mktemp -d) || die "loaded: mktemp failed"
  # LD_DEBUG_OUTPUT writes one file per process, so child processes count
  # too and the program's own stderr stays clean.
  LD_DEBUG=libs LD_DEBUG_OUTPUT=$ld/ld LD_LIBRARY_PATH=$lib \
    "$@" > /dev/null 2>&1 < /dev/null || true
  all=$(cat "$ld"/ld.* 2> /dev/null | sed -n 's/.*calling init: //p' | LC_ALL=C sort -u)
  rm -rf "$ld"
  proof=
  for l in $libs; do
    stem=${l%%.so.*}.so
    inits=
    for i in $all; do
      case ${i##*/} in
        "$stem" | "$stem".*) inits="$inits $i" ;;
      esac
    done
    [ -n "$inits" ] || die "loaded: $1 did not load $l"
    for i in $inits; do
      case $i in
        "$lib"/*/*) die "loaded: $1 loaded $i, not $l directly from $lib" ;;
        "$lib"/*) ;;
        *) die "loaded: $1 loaded $i, not $l from $lib" ;;
      esac
    done
    proof="$proof$inits"
  done
  echo "loaded: $1:$proof" | tee -a "$work/.consumers-loaded"
}

# ------------------------------------------------------------------- run
# Installs Wolfi packages that are not installed yet. In CI the check's
# environment carries the common ones; a consumer's `packages` file names
# the rest.
need_packages() {
  missing=
  for p in "$@"; do
    apk info -e "$p" > /dev/null 2>&1 || missing="$missing $p"
  done
  [ -n "$missing" ] || return 0
  if ! command -v apk > /dev/null 2>&1 || [ "$(id -u)" != 0 ]; then
    echo "consumers: missing packages:$missing (and no apk as root to install them)" >&2
    return 1
  fi
  echo "apk add$missing"
  # shellcheck disable=SC2086 # one word per package
  apk add --no-cache $missing > /dev/null && return 0
  # A check's guest is built from a package list, and its repositories
  # file may name a host it cannot reach. Wolfi's own repository is the
  # one wolfi-keys trusts.
  wolfi=https://packages.wolfi.dev/os
  echo "apk add failed with repositories [$(paste -sd ' ' /etc/apk/repositories 2> /dev/null || true)]; retrying from $wolfi only"
  # shellcheck disable=SC2086 # one word per package
  apk add --no-cache --repositories-file /dev/null --repository "$wolfi" $missing > /dev/null
}

# The words of a list file, without `#` comments.
list_words() { awk '{ sub(/#.*/, ""); for (i = 1; i <= NF; i++) print $i }' "$1"; }

run() {
  [ $# -ge 1 ] || usage "usage: consumers.sh run LIB [NAME...]"
  lib=$1
  shift
  [ -d "$lib" ] || die "run: LIB '$lib' is not a directory"
  lib=$(cd "$lib" && pwd) || die "run: cannot enter LIB"
  plain_path LIB "$lib"
  [ -n "$(ls -A "$lib")" ] || die "run: LIB $lib is empty: copy the port's libraries into it first"
  # The loader prints the path it opened, LIB's entry, not where a
  # symlink points: an entry that resolves outside LIB (a link to the
  # system's copy) would pass `loaded` while the consumer runs on
  # another library. Links within LIB (libfoo.so.1 -> libfoo.so.1.2.3)
  # are fine.
  plib=$(cd "$lib" && pwd -P) || die "run: cannot resolve LIB"
  for f in "$lib"/* "$lib"/.[!.]* "$lib"/..?*; do
    [ -e "$f" ] || [ -L "$f" ] || continue
    # Before readlink, which in BusyBox resolves a dangling relative
    # link against the cwd.
    [ -e "$f" ] || die "run: $f is a dangling link"
    r=$(readlink -f "$f") && [ -n "$r" ] || die "run: cannot resolve $f (readlink -f)"
    case $r in
      "$plib"/*) ;;
      *) die "run: $f resolves to $r, outside LIB $plib: copy the port's libraries into LIB, with links only to files there" ;;
    esac
  done
  root=$(pwd)
  dir=${CONSUMERS_DIR:-tests/consumers}
  work=${CONSUMERS_WORK:-target/consumers}
  case $dir in /*) ;; *) dir=$root/$dir ;; esac
  case $work in /*) ;; *) work=$root/$work ;; esac
  [ -d "$dir" ] || die "run: no consumer directory $dir (CONSUMERS_DIR)"
  plain_path CONSUMERS_WORK "$work"
  if [ $# = 0 ]; then
    if [ -n "${CONSUMER:-}" ]; then
      set -- "$CONSUMER"
    else
      for r in "$dir"/*/run.sh; do
        [ -f "$r" ] || continue
        d=${r%/run.sh}
        set -- "$@" "${d##*/}"
      done
    fi
  fi
  [ $# -gt 0 ] || die "run: no consumers to run (no $dir/*/run.sh)"
  for n in "$@"; do
    case $n in
      "" | -* | *[!a-z0-9-]*) usage "run: bad consumer name '$n'" ;;
    esac
  done
  mkdir -p "$work/cache"
  # fetch needs curl. Install it when it is missing and apk runs as
  # root; where it cannot, only a consumer that fetches fails (fetch
  # says why).
  if ! command -v curl > /dev/null 2>&1; then
    need_packages curl || echo "consumers: no curl: a consumer that calls fetch fails" >&2
  fi
  # shellcheck disable=SC2012 # a listing for the log, not parsed
  echo "swap directory $lib: $(ls -A "$lib" | paste -sd ' ' -)"

  pass=0 fail=0 skip=0
  results=
  result() {
    echo "$1 $2${3:+: $3}"
    results="$results$1 $2${3:+: $3}
"
    case $1 in
      PASS) pass=$((pass + 1)) ;;
      FAIL) fail=$((fail + 1)) ;;
      SKIP) skip=$((skip + 1)) ;;
    esac
  }

  for name in "$@"; do
    cdir=$dir/$name
    echo "== consumer $name"
    if [ ! -f "$cdir/run.sh" ]; then
      result FAIL "$name" "no $cdir/run.sh"
      continue
    fi
    out=$work/$name
    rm -rf "$out"
    mkdir -p "$out"
    if [ -f "$cdir/packages" ]; then
      pkgs=$(list_words "$cdir/packages") ||
        { result FAIL "$name" "cannot read its packages file"; continue; }
      # shellcheck disable=SC2086 # one word per package
      need_packages $pkgs ||
        { result FAIL "$name" "could not install its packages"; continue; }
    fi
    : > "$out/failures"
    # The subshell records run.sh's exit status, which the pipe into tee
    # would lose.
    (
      cd "$out" || exit 1
      rc=0
      env "${prefix}_LIB=$lib" "${prefix}_ROOT=$root" "${prefix}_WORK=$out" \
        "${prefix}_FAILURES=$out/failures" "${prefix}_CACHE=$work/cache" \
        "${prefix}_HARNESS=$self" "CONSUMERS_PREFIX=$prefix" \
        "CONSUMERS_SONAME=${CONSUMERS_SONAME:-}" \
        sh "$cdir/run.sh" < /dev/null 2>&1 || rc=$?
      echo "$rc" > "$out/.status"
    ) | tee "$out/log"
    rc=$(cat "$out/.status" 2> /dev/null || echo 1)
    if [ "$rc" = 77 ]; then
      reason=$(sed -n 's/^SKIP //p' "$out/log" | tail -n 1)
      result SKIP "$name" "${reason:-run.sh exited 77}"
      continue
    fi
    if [ "$rc" != 0 ]; then
      result FAIL "$name" "run.sh exited $rc (log: $out/log)"
      continue
    fi
    if [ ! -s "$out/.consumers-loaded" ]; then
      result FAIL "$name" "run.sh did not prove the port's library was loaded (sh \"\$${prefix}_HARNESS\" loaded ...)"
      continue
    fi
    # Each step on its own, not in a pipe, whose status would be only
    # its last command's: a failures file run.sh removed (it is in
    # run.sh's cwd) must fail, not read as no failures.
    if [ ! -f "$out/failures" ] ||
      ! LC_ALL=C sort -u "$out/failures" > "$out/failures.all" ||
      ! sed '/^$/d' "$out/failures.all" > "$out/failures.sorted"; then
      result FAIL "$name" "cannot read its failures file (\$${prefix}_FAILURES, $out/failures)"
      continue
    fi
    : > "$out/expected.sorted"
    exp=$cdir/expected-failures.txt
    # Not a regular file (a directory, a dangling link) is unreadable
    # too: BusyBox's sed passes over a directory.
    if { [ -e "$exp" ] || [ -L "$exp" ]; } &&
      { [ ! -f "$exp" ] || ! sed -e 's/#.*//' "$exp" > "$out/expected.raw" ||
        ! awk 'NF { print $1 }' "$out/expected.raw" > "$out/expected.ids" ||
        ! LC_ALL=C sort -u "$out/expected.ids" > "$out/expected.sorted"; }; then
      result FAIL "$name" "cannot read its expected-failures.txt"
      continue
    fi
    unexpected=$(LC_ALL=C comm -23 "$out/failures.sorted" "$out/expected.sorted")
    fixed=$(LC_ALL=C comm -13 "$out/failures.sorted" "$out/expected.sorted")
    for t in $fixed; do
      echo "NOTICE $name: $t is in expected-failures.txt but passed; remove its line"
    done
    nfail=$(wc -l < "$out/failures.sorted" | tr -d ' ')
    if [ -n "$unexpected" ]; then
      result FAIL "$name" "unexpected failures: $(echo "$unexpected" | paste -sd ' ' -)"
    else
      result PASS "$name" "$nfail expected failures"
    fi
  done

  echo "== summary"
  printf '%s' "$results"
  echo "consumers: $pass passed, $fail failed, $skip skipped"
  [ "$fail" = 0 ]
}

case ${1:-} in
  run) shift; run "$@" ;;
  fetch) shift; fetch "$@" ;;
  loaded) shift; loaded "$@" ;;
  -h | --help) sed -n '2,/^set -eu/p' "$0" | sed '$d; s/^# \{0,1\}//' ;;
  *) usage "usage: consumers.sh run LIB [NAME...] | fetch URL SHA256 [FILE] | loaded [--lib SONAME]... PROGRAM [ARG...]" ;;
esac
