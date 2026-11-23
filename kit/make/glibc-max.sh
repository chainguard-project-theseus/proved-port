#!/bin/sh
# glibc-max.sh SO MAX: SO must need no GLIBC_ symbol version above MAX
# (`make glibc-max`, proved-port.mk), which the theseus-proved-port
# package installs in /usr/share/proved-port/make/. challoc's
# ci/glibc-max.sh, in POSIX sh.
#
# A port that ships a glibc-linked library supports the glibc stream it
# is built for (challoc: 2.44, Wolfi's); a library that needs a newer
# symbol version will not load there. This reads SO's version-needs
# section (.gnu.version_r) with binutils' `readelf -V --wide`, takes
# every `Name: GLIBC_<n>[.<n>...]` there (GLIBC_PRIVATE is not a
# version), and compares the highest with MAX in version order (`sort
# -V`). An ABI marker need (`GLIBC_ABI_<NAME>`) needs the glibc release
# that added it, from the table below (GLIBC_ABI_DT_RELR, from linking
# with `-z pack-relative-relocs`: 2.36); one the table lacks fails. The
# version definition section (.gnu.version_d) is not read: a GLIBC_
# node SO defines for binary compatibility (a libcrypt replacement's
# GLIBC_2.2.5) is no need. It prints the versions SO needs and each
# marker's release, then the verdict:
#
#   - the highest at or below MAX passes;
#   - one above MAX fails (exit 1), naming it, as does a marker whose
#     release is above MAX, or one the table lacks;
#   - no GLIBC_ version at all fails: SO is not a glibc build;
#   - a missing SO, or one readelf cannot read, fails.
#
# MAX is a version, digits in dot-separated parts (2.44); anything else
# is a usage error.
#
# Exit 0: the highest need is at or below MAX; 1: above it, none, or SO
# missing or unreadable; 2: usage.
set -eu
[ $# -eq 2 ] || { echo "usage: glibc-max.sh SO MAX" >&2; exit 2; }
so=$1
max=$2
LC_ALL=C
export LC_ALL
printf '%s\n' "$max" | grep -Eqx '[0-9]+(\.[0-9]+)*' \
  || { echo "usage: glibc-max.sh SO MAX: MAX is not a version (2.44): $max" >&2; exit 2; }
[ -f "$so" ] || { echo "error: $so missing"; exit 1; }

out=$(mktemp)
hits=$(mktemp)
vers_file=$(mktemp)
trap 'rm -f "$out" "$hits" "$vers_file"' EXIT
# Each status is kept: in a pipeline a readelf failure, or grep's read
# error (2 or more, where 1 is no match), would be lost.
rc=0
readelf -V --wide "$so" > "$out" || rc=$?
[ "$rc" -eq 0 ] || { echo "error: readelf -V cannot read $so (exit $rc)"; exit 1; }
# Only the version-needs section's lines: readelf prints a `Name:` on
# each version definition (.gnu.version_d) too, and a library that
# defines its own GLIBC_ nodes needs none of them. Each section starts
# with a `Version ... section` heading.
awk '/^Version / { needs = /^Version needs section/; next } needs' "$out" > "$hits" \
  || { echo "error: awk failed on readelf's output for $so"; exit 1; }
cp "$hits" "$out"
rc=0
grep -oE 'Name: GLIBC_[0-9]+(\.[0-9]+)*' "$out" > "$hits" || rc=$?
[ "$rc" -le 1 ] || { echo "error: cannot read readelf's output for $so (grep exit $rc)"; exit 1; }
sed 's/Name: GLIBC_//' "$hits" > "$vers_file" || { echo "error: sed failed on readelf's output for $so"; exit 1; }
sort -uV "$vers_file" > "$hits"
# Plain command substitutions, not pipelines: set -e sees each status.
vers=$(cat "$hits")
highest=$(tail -n 1 "$hits")
[ -n "$vers" ] || { echo "error: $so needs no GLIBC_ version at all (not a glibc build?)"; exit 1; }
echo "GLIBC_ versions $so needs: $(printf '%s\n' "$vers" | tr '\n' ' ' | sed 's/ $//')"
# The GLIBC_ABI_ markers: each needs the glibc release that added it.
rc=0
grep -oE 'Name: GLIBC_ABI_[A-Za-z0-9_]+' "$out" > "$hits" || rc=$?
[ "$rc" -le 1 ] || { echo "error: cannot read readelf's output for $so (grep exit $rc)"; exit 1; }
sed 's/Name: //' "$hits" > "$out" || { echo "error: sed failed on readelf's output for $so"; exit 1; }
sort -u "$out" > "$hits"
# above V: whether V sorts after MAX in version order. Through files, not
# a pipeline: a sort that printed part of its output and failed would
# otherwise leave MAX last. It may run in an `if`, where set -e is off,
# so sort's status is checked by hand.
above() {
  printf '%s\n%s\n' "$1" "$max" > "$out"
  sort -V "$out" > "$vers_file" || { echo "error: sort -V failed comparing $1 with $max"; exit 1; }
  [ "$(tail -n 1 "$vers_file")" != "$max" ]
}
while read -r m; do
  case $m in
    GLIBC_ABI_DT_RELR) since=2.36 ;;
    *)
      echo "error: $so needs $m, an ABI marker glibc-max.sh does not know the glibc release of; add it to the script's table"
      exit 1
      ;;
  esac
  echo "$m needs glibc $since"
  if above "$since"; then
    echo "error: $so needs $m (glibc $since), above GLIBC_$max"
    exit 1
  fi
done < "$hits"
if above "$highest"; then
  echo "error: $so needs GLIBC_$highest, above GLIBC_$max"
  exit 1
fi
echo "ok: highest GLIBC_$highest <= GLIBC_$max"
