# shellcheck shell=sh
# ci/lib.sh: helpers shared by the ci/ scripts. Sourced, not run: a script
# loads it from its own directory with
#
#   # shellcheck source=ci/lib.sh
#   . "$(dirname "$0")/lib.sh"
#
# and keeps its own pins and invocations. Defines functions only. A port
# that has a ci/lib.sh already adds these three functions to it; one that
# adds its own helpers adds them below.

# fetch URL SHA256 FILE: download URL to FILE and check its SHA-256.
fetch() {
  curl -sSfL --proto '=https' --retry 3 -o "$3" "$1"
  echo "$2  $3" | sha256sum -c -
}

# rust_host NAME: print the Rust host triple of this machine, or fail with
# a message prefixed NAME on a machine the rustup-init pins do not cover.
rust_host() {
  case $(uname -m) in
    x86_64) echo x86_64-unknown-linux-gnu ;;
    aarch64) echo aarch64-unknown-linux-gnu ;;
    *) echo "$1: unsupported machine $(uname -m)" >&2; return 1 ;;
  esac
}

# rustup_pinned NAME DIR MANIFEST_URL MANIFEST_SHA256 TOOLCHAIN [ARGS...]:
# install TOOLCHAIN with rustup, pinned. RUSTUP_HOME and CARGO_HOME must be
# exported, with $CARGO_HOME/bin on PATH; DIR holds the downloads.
#
# rustup-init is pinned by version and SHA-256 and installed once. The
# toolchain is pinned by the SHA-256 of its channel manifest at
# MANIFEST_URL, and installed with `rustup toolchain install` plus ARGS
# (targets, components). rustup checks every component it installs against
# that manifest's hashes, and this checks that the manifest rustup
# installed from lists only hashes the pinned manifest lists. Then it
# exports RUSTUP_TOOLCHAIN=TOOLCHAIN. NAME prefixes the messages.
rustup_pinned() {
  _rp_name=$1 _rp_dir=$2 _rp_url=$3 _rp_sum=$4 _rp_toolchain=$5
  shift 5
  _rp_host=$(rust_host "$_rp_name")
  # The SHA-256s of https://static.rust-lang.org/rustup/archive/<version>/<host>/rustup-init.
  _rp_version="{RUSTUP_RELEASE}"
  case $_rp_host in
    x86_64-*) _rp_init_sum="{RUSTUP_INIT_SHA256_X86_64}" ;;
    aarch64-*) _rp_init_sum="{RUSTUP_INIT_SHA256_AARCH64}" ;;
  esac
  if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
    fetch "https://static.rust-lang.org/rustup/archive/$_rp_version/$_rp_host/rustup-init" \
      "$_rp_init_sum" "$_rp_dir/rustup-init"
    chmod +x "$_rp_dir/rustup-init"
    "$_rp_dir/rustup-init" -y --quiet --no-modify-path --no-update-default-toolchain \
      --profile minimal --default-toolchain none
  fi
  fetch "$_rp_url" "$_rp_sum" "$_rp_dir/channel.toml"
  rustup toolchain install --no-self-update --profile minimal "$@" "$_rp_toolchain"
  # rustup keeps the manifest it installed from, rewritten (so not byte for
  # byte): every hash in it must be one the pinned manifest lists.
  _rp_installed=$RUSTUP_HOME/toolchains/$_rp_toolchain-$_rp_host/lib/rustlib/multirust-channel-manifest.toml
  grep -oE '[0-9a-f]{64}' "$_rp_dir/channel.toml" | sort -u >"$_rp_dir/pinned.sums"
  _rp_extra=$(grep -oE '[0-9a-f]{64}' "$_rp_installed" | sort -u | grep -vxFf "$_rp_dir/pinned.sums" || true)
  if [ -n "$_rp_extra" ] || ! grep -q '[0-9a-f]\{64\}' "$_rp_installed"; then
    echo "$_rp_name: $_rp_installed does not match the pinned channel manifest" >&2
    exit 1
  fi
  export RUSTUP_TOOLCHAIN="$_rp_toolchain"
  rustc --version
}
