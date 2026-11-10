#!/bin/sh
# kit/shellcheck.sh: lint every kit/*.sh file git tracks (the shellcheck
# check), so a new kit script is covered as soon as it is added. Run it
# from the repository root.
# Wolfi does not package the linter, so this fetches the pinned upstream
# release and checks its SHA-256 (the digests GitHub publishes for the
# release assets) before running it: challoc's ci/shellcheck.sh, with
# the same pin.
set -eu
version=v0.11.0
case $(uname -m) in
  x86_64) sum=8c3be12b05d5c177a04c29e3c78ce89ac86f1595681cab149b65b97c4e227198 ;;
  aarch64) sum=12b331c1d2db6b9eb13cfca64306b1b157a86eb69db83023e261eaa7e7c14588 ;;
  *) echo "unsupported machine $(uname -m)" >&2; exit 1 ;;
esac
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
tarball=$dir/shellcheck.tar.xz
curl -sSfL -o "$tarball" \
  "https://github.com/koalaman/shellcheck/releases/download/$version/shellcheck-$version.linux.$(uname -m).tar.xz"
echo "$sum  $tarball" | sha256sum -c -
tar -xJf "$tarball" -C "$dir"
"$dir/shellcheck-$version/shellcheck" --version
# A command substitution, not a pipe, so a git failure stops the script.
files=$(git -c safe.directory='*' ls-files -- 'kit/*.sh')
test -n "$files"
# The paths have no spaces; split them into arguments.
# shellcheck disable=SC2086
"$dir/shellcheck-$version/shellcheck" $files
