#!/bin/sh
# lint-unsafe.sh [--self-test] [PORT_ROOT]: the unsafe-code rule every
# proved port shares (`make lint-unsafe`, proved-port.mk), which the
# theseus-proved-port package installs in /usr/share/proved-port/make/.
# PORT_ROOT is the port's root (by default the current directory).
#
# The packages UNSAFE_CRATES names (the C API crate, and any other fence
# crate) are the only ones allowed `unsafe`. For every other workspace
# member, it checks that:
#
#   - each crate root cargo finds by itself (src/lib.rs, src/main.rs,
#     src/bin/*.rs, src/bin/*/main.rs), of which there is at least one,
#     forbids unsafe code: the inner attribute #![forbid(unsafe_code)]
#     (alone or in a list), or both lines of the Verus pair
#       #![cfg_attr(not(verus_keep_ghost), forbid(unsafe_code))]
#       #![cfg_attr(verus_keep_ghost, deny(unsafe_code))]
#     which forbids it in every ordinary build and only denies it while
#     Verus runs (Verus expands each assume_specification into an unsafe
#     item, which ordinary builds erase);
#   - every target (lib, bins, tests, benches, examples, build.rs)
#     checks with unsafe_code forbidden on the command line, and every
#     target cargo metadata lists for the member must have reached rustc
#     (one with `test = false`, `bench = false` or required-features
#     off, which --all-targets skips, fails the run):
#     `cargo check --locked --workspace --all-targets` runs rustc for
#     each member through a RUSTC_WORKSPACE_WRAPPER that adds
#     -Funsafe_code last, unless the package cargo is compiling
#     (CARGO_PKG_NAME) is one UNSAFE_CRATES names. No attribute can
#     lower a command-line forbid (rustc's E0453), and it reaches
#     macro-expanded code. The fence crates build as they always do, so
#     another member may depend on one (a C memory crate the others
#     call through), and RUSTFLAGS, and with them the build cache's
#     other entries, stay as they are. A RUSTC_WORKSPACE_WRAPPER the
#     caller set still runs, inside this one. cargo compiles every
#     member again on each run (the wrapper is new each time), so a
#     result cached from a build without it never stands in for the
#     check; and a run in which any member but the fence crates was not
#     compiled through it (left out, or taken as fresh) fails. So does
#     a member's rustc getting --cap-lints or --force-warn, which
#     outrank a forbid, from the caller's flags or cargo configuration,
#     directly or in an @FILE argument file.
#
# With UNSAFE_WORKSPACE_LINTS = yes (the default), it also checks that
# the root manifest sets unsafe_code to "deny" or "forbid" (or a table
# with that level) in [workspace.lints.rust], and that every member but
# the fence crates inherits the workspace lints with
# `[lints] workspace = true`: so a plain cargo build denies unsafe code
# as well. A fence crate may inherit them (with "deny", its root's
# #![allow(unsafe_code)] lowers the lint) or not (with "forbid", which
# no allow can lower, it must not). A port whose
# manifests have no workspace lints sets it to no.
#
# A root is read through its inner attributes (inner_attributes below),
# so an attribute's text in a comment, in a string or after the first
# item is not one. Each name in UNSAFE_CRATES must be a workspace member,
# so a misspelt name fails rather than excluding nothing; empty, or
# naming every member (which would leave nothing to check), refuses the
# run (exit 2), as does a member ID in cargo metadata it cannot read.
# The members and their names are cargo's (metadata's
# workspace_members), not read from the manifests. Rules of a port's
# own (a list of the functions that may mint a ghost token, an
# unsafe-block budget, a SAFETY: comment for each block) stay in the
# port's script, which runs this one first.
#
# The environment; empty is the same as unset:
#
#   UNSAFE_CRATES           the packages allowed unsafe, by name (required)
#   UNSAFE_WORKSPACE_LINTS  yes (the default) or no
#   UNSAFE_MANIFEST         the workspace manifest cargo reads, from
#                           PORT_ROOT (by default PORT_ROOT/Cargo.toml):
#                           a uutils port's crates/uutils/Cargo.toml
#   UNSAFE_CHECK_FLAGS      further flags for the cargo check (features).
#                           --all-targets skips a target whose
#                           required-features are off, so a run in which
#                           a member but the fence crates has such a
#                           target (cargo metadata lists them) fails
#                           unless these flags hold --all-features.
#                           Flags that leave a member out (--exclude,
#                           -p) fail the run.
#
# It first runs its self-test: the root rule on planted roots, those it
# must accept and those it must refuse, so a broken reader fails the run
# instead of passing every root. --self-test runs that alone and needs
# no cargo. Exit 0: every check passed; 1: one failed; 2: the run could
# not start (usage, UNSAFE_CRATES, UNSAFE_WORKSPACE_LINTS, cargo
# metadata, a member it cannot read).
set -eu

self_test_only=
if [ "${1-}" = --self-test ]; then
  self_test_only=1
  shift
fi
[ $# -le 1 ] || { echo "usage: lint-unsafe.sh [--self-test] [PORT_ROOT]" >&2; exit 2; }

fail_count=0
pass() { echo "ok   $1"; }
fail() { fail_count=$((fail_count + 1)); echo "FAIL $1"; }

# inner_attributes FILE: the inner attributes of the Rust source FILE, one
# on a line, from its opening bracket to the bracket that closes it.
#
# A file's inner attributes stand before its first item, with nothing but
# blanks and comments around them (a block comment may hold others), so
# that is all this reads: the attribute's text in a comment, in a string or
# after an item is not an attribute. Inside an attribute, blanks and
# comments are left out, but for one blank where they part two characters
# of identifiers; a string's contents are left out ("" stands for it), and
# so are a character literal's (''). A string is raw only where nothing
# stands between its prefix (r, br or cr, after no other character of an
# identifier), its hashes and its quote; it ends at the quote and as many
# hashes, and only a string that is not raw has escapes. A quote is a
# character literal where an escape, or one character and a quote, follow
# it, and otherwise a lifetime's. An attribute that the file ends in
# before it closes, or a quote that is neither, ends the reading, and
# what it has printed is all there is. A first line that starts with #!
# and is no attribute is a command line for a shell.
inner_attributes() {
  LC_ALL=C awk '
    # past(i): the first position from i on that is not blank and not in a
    # comment.
    function past(i,    depth) {
      while (i <= n) {
        if (substr(s, i, 1) ~ /[ \t\r\n]/) i++
        else if (substr(s, i, 2) == "//") {
          while (i <= n && substr(s, i, 1) != "\n") i++
        } else if (substr(s, i, 2) == "/*") {
          depth = 1
          i += 2
          while (i <= n && depth > 0) {
            if (substr(s, i, 2) == "/*") { depth++; i += 2 }
            else if (substr(s, i, 2) == "*/") { depth--; i += 2 }
            else i++
          }
        } else break
      }
      return i
    }
    # ident(j): the character at j is one of an identifier: an ASCII
    # letter, digit or underscore, or a byte of a character beyond ASCII.
    function ident(j,    c) {
      if (j < 1 || j > n) return 0
      c = substr(s, j, 1)
      return c ~ /[A-Za-z0-9_]/ || c !~ /[\t\n\r -~]/
    }
    { s = s $0 "\n" }
    END {
      n = length(s)
      i = 1
      if (substr(s, 1, 2) == "#!" && substr(s, past(3), 1) != "[")
        while (i <= n && substr(s, i, 1) != "\n") i++
      while (1) {
        i = past(i)
        if (substr(s, i, 2) != "#!") break
        i = past(i + 2)
        if (substr(s, i, 1) != "[") break
        attr = ""
        open = 0
        closed = 0
        while (i <= n) {
          j = past(i)
          if (j > i && ident(i - 1) && ident(j)) attr = attr " "
          i = j
          if (i > n) break
          c = substr(s, i, 1)
          if (c == "\"") {
            # The hashes and the prefix, back from the quote with nothing
            # between them.
            h = i - 1
            while (h >= 1 && substr(s, h, 1) == "#") h--
            p = h
            if (p >= 1 && substr(s, p, 1) == "r") {
              p--
              if (p >= 1 && substr(s, p, 1) ~ /[bc]/) p--
              raw = !ident(p)
            } else raw = 0
            quote = raw ? "\"" substr(s, h + 1, i - 1 - h) : "\""
            i++
            while (i <= n && substr(s, i, length(quote)) != quote) i += (!raw && substr(s, i, 1) == "\\") ? 2 : 1
            if (i > n) break
            i += length(quote)
            attr = attr "\"\""
            continue
          }
          if (c == "\x27") {
            # A character literal, or the quote of a lifetime.
            if (substr(s, i + 1, 1) == "\\") {
              j = i + 1
              while (j <= n && substr(s, j, 1) != "\x27") j += (substr(s, j, 1) == "\\") ? 2 : 1
            } else {
              # One character: an ASCII one, or the bytes of another.
              j = i + 2
              if (substr(s, i + 1, 1) !~ /[\t\n\r -~]/)
                while (j <= n && substr(s, j, 1) !~ /[\t\n\r -~]/) j++
            }
            if (j <= n && substr(s, j, 1) == "\x27") {
              i = j + 1
              attr = attr "\x27\x27"
              continue
            }
            if (!ident(i + 1)) exit 0
            attr = attr c
            i++
            continue
          }
          attr = attr c
          i++
          if (c == "[" || c == "(") open++
          else if ((c == "]" || c == ")") && --open == 0) { closed = 1; break }
        }
        if (!closed) break
        print attr
      }
    }' "$1"
}

# root_forbids FILE: the crate root FILE forbids unsafe code, with the
# inner attribute #![forbid(unsafe_code)] (alone or in a list) or with
# both attributes of the Verus pair.
root_forbids() {
  _attrs=$(inner_attributes "$1")
  printf '%s\n' "$_attrs" | grep -Eq '^\[forbid\((.*,)?unsafe_code(,.*)?\)\]$' && return 0
  printf '%s\n' "$_attrs" | grep -Fqx '[cfg_attr(not(verus_keep_ghost),forbid(unsafe_code))]' &&
    printf '%s\n' "$_attrs" | grep -Fqx '[cfg_attr(verus_keep_ghost,deny(unsafe_code))]'
}

# ----------------------------------------------------------------- self-test
echo "=== lint-unsafe: self-test ==="
plant=$(mktemp -d)
trap 'rm -rf "$plant"' EXIT
# expect NAME VERDICT LINE...: plant LINE... as a root and check that
# root_forbids gives VERDICT (ok or refuse).
expect() {
  _name=$1 _want=$2
  shift 2
  printf '%s\n' "$@" > "$plant/lib.rs"
  if root_forbids "$plant/lib.rs"; then _got=ok; else _got=refuse; fi
  if [ "$_got" = "$_want" ]; then
    pass "self-test: $_name: $_want"
  else
    fail "self-test: $_name: got = $_got, want = $_want"
  fi
}
pair_forbid='#![cfg_attr(not(verus_keep_ghost), forbid(unsafe_code))]'
pair_deny='#![cfg_attr(verus_keep_ghost, deny(unsafe_code))]'
expect "forbid" ok '#![forbid(unsafe_code)]'
expect "forbid in a list" ok '#![forbid(missing_docs, unsafe_code)]'
expect "forbid wrapped by rustfmt" ok '#![forbid(' '    missing_docs,' '    unsafe_code' ')]'
expect "the Verus pair" ok "$pair_forbid" "$pair_deny" 'pub fn f() {}'
expect "the Verus pair, deny first" ok "$pair_deny" "$pair_forbid"
expect "no attribute" refuse 'pub fn f() {}'
expect "deny alone" refuse '#![deny(unsafe_code)]'
expect "another lint forbidden" refuse '#![forbid(unsafe_op_in_unsafe_fn)]'
expect "a lint named with unsafe_code in it" refuse '#![forbid(not_unsafe_code)]'
expect "the pair's forbid alone" refuse "$pair_forbid"
expect "the pair's deny alone" refuse "$pair_deny"
expect "the pair commented out" refuse "// $pair_forbid" "// $pair_deny"
expect "the pair with its cfgs swapped" refuse \
  '#![cfg_attr(verus_keep_ghost, forbid(unsafe_code))]' \
  '#![cfg_attr(not(verus_keep_ghost), deny(unsafe_code))]'
expect "forbid under another cfg" refuse '#![cfg_attr(test, forbid(unsafe_code))]'
# The attribute's text where it is no attribute.
expect "forbid in a block comment" refuse '/*' '#![forbid(unsafe_code)]' '*/' 'pub fn f() {}'
expect "unsafe_code in a comment after another forbid" refuse \
  '#![forbid(missing_docs)] // unsafe_code' 'pub fn f() {}'
expect "forbid after an item" refuse 'pub fn f() {}' '#![forbid(unsafe_code)]'
expect "the Verus pair in a block comment" refuse '/*' "$pair_forbid" "$pair_deny" '*/'
# Strings and character literals in an attribute: the forbid after each of
# these lines is in a string.
expect "forbid in a string of an attribute" refuse \
  '#![doc = "] #![forbid(unsafe_code)] //"]' 'pub fn f() {}'
expect "a blank between r and the quote" refuse \
  '#![cfg_attr(any(), x(r "\"))] #![forbid(unsafe_code)] //"))]' 'pub fn f() {}'
expect "a quote in a character literal" refuse \
  "#![cfg_attr(any(), x('\"', \"))] #![forbid(unsafe_code)] //\"))]" 'pub fn f() {}'
# An attribute that never closes is not read: its text alone would match.
expect "an attribute that never closes" refuse '#![forbid(x,(,unsafe_code)]'
# And where the attribute is one, after strings, raw strings, character
# literals and lifetimes in another, a comment, or a shell's #! line.
expect "forbid after a raw string" ok '#![doc = r#"a "b" c"#]' '#![forbid(unsafe_code)]'
expect "forbid after literals and a lifetime" ok \
  "#![cfg_attr(any(), x('\"', '\\'', 'a, b\"\\\"\"))]" '#![forbid(unsafe_code)]'
expect "the Verus pair after a doc comment" ok '//! The crate.' "$pair_forbid" "$pair_deny"
expect "forbid after a #! command line" ok '#!/usr/bin/env run-cargo-script' '#![forbid(unsafe_code)]'
if [ "$fail_count" -ne 0 ]; then
  echo "lint-unsafe: the self-test failed: the root reader is broken, so no root's verdict can be trusted" >&2
  exit 1
fi
[ -z "$self_test_only" ] || exit 0

# --------------------------------------------------------------- settings
root=${1:-.}
root=$(cd "$root" && pwd)
crates=$(printf '%s\n' "${UNSAFE_CRATES-}" | tr -s '[:blank:]' '\n' | sed '/^$/d')
if [ -z "$crates" ]; then
  echo "lint-unsafe: UNSAFE_CRATES is empty: name the packages allowed unsafe code (the C API crate) in the port's Makefile" >&2
  exit 2
fi
lints=${UNSAFE_WORKSPACE_LINTS:-yes}
case $lints in
  yes | no) ;;
  *) echo "lint-unsafe: UNSAFE_WORKSPACE_LINTS is $lints: yes or no" >&2; exit 2 ;;
esac
manifest=${UNSAFE_MANIFEST:-Cargo.toml}
case $manifest in
  /*) ;;
  *) manifest=$root/$manifest ;;
esac
[ -f "$manifest" ] || { echo "lint-unsafe: no workspace manifest $manifest" >&2; exit 2; }
rel() { printf '%s\n' "${1#"$root"/}"; }

# The members, one `NAME<tab>MANIFEST` a line, from cargo metadata's
# workspace_members: each is a package ID, which names the package and
# its directory, so the names are cargo's own, not read from the
# manifests. An ID is `path+file://DIR#VERSION` when the name is DIR's
# last component, `path+file://DIR#NAME@VERSION` when it is not
# (cargo 1.77 and later), or `NAME VERSION (path+file://DIR)` (earlier).
# An ID it cannot read (a percent-encoded path) refuses the run.
# cargo metadata prints JSON with no line breaks.
if ! meta=$(cd "$root" && cargo metadata --no-deps --format-version 1 --offline --manifest-path "$manifest"); then
  echo "lint-unsafe: cargo metadata failed (above)" >&2
  exit 2
fi
tab=$(printf '\t')
ids=$(printf '%s\n' "$meta" | grep -o '"workspace_members":\[[^]]*\]' | sed 's/^"workspace_members":\[//; s/\]$//' | tr ',' '\n' | sed 's/^"//; s/"$//')
members=
while IFS= read -r id; do
  [ -n "$id" ] || continue
  case $id in
    path+file://*'#'*)
      dir=${id#path+file://}
      frag=${dir##*#}
      dir=${dir%#*}
      case $frag in
        *@*) p=${frag%@*} ;;
        *) p=${dir##*/} ;;
      esac
      ;;
    *' (path+file://'*')')
      p=${id%% *}
      dir=${id#*(path+file://}
      dir=${dir%)}
      ;;
    *)
      echo "lint-unsafe: cargo metadata names a workspace member lint-unsafe.sh cannot read: $id" >&2
      exit 2
      ;;
  esac
  case $dir$p in
    *%* | *"$tab"* | '')
      echo "lint-unsafe: cargo metadata names a workspace member lint-unsafe.sh cannot read: $id" >&2
      exit 2
      ;;
  esac
  [ -f "$dir/Cargo.toml" ] || { echo "lint-unsafe: no $dir/Cargo.toml for the workspace member $id" >&2; exit 2; }
  members="$members$p$tab$dir/Cargo.toml
"
done <<IDS
$ids
IDS
if [ -z "$members" ]; then
  echo "lint-unsafe: cargo metadata lists no workspace members" >&2
  exit 2
fi
names=$(printf '%s' "$members" | cut -f1)
manifests=$(printf '%s' "$members" | cut -f2-)
# The members the forbid applies to: none would leave nothing to check.
others=$(printf '%s\n' "$names" | while IFS= read -r p; do
  printf '%s\n' "$crates" | grep -Fqx -- "$p" || echo "$p"
done)
if [ -z "$others" ]; then
  echo "lint-unsafe: UNSAFE_CRATES names every workspace member, so there is nothing to forbid unsafe code in" >&2
  exit 2
fi

# is_fence MANIFEST: MANIFEST's package is one UNSAFE_CRATES names.
is_fence() {
  _p=$(printf '%s' "$members" | awk -F "$tab" -v m="$1" '$2 == m { print $1; exit }')
  printf '%s\n' "$crates" | grep -Fqx -- "$_p"
}

# ------------------------------------------------------------ fence crates
echo "=== lint-unsafe: the crates allowed unsafe (UNSAFE_CRATES) ==="
while IFS= read -r c; do
  if printf '%s' "$names" | grep -Fqx -- "$c"; then
    pass "$c is a workspace member allowed unsafe"
  else
    fail "$c (UNSAFE_CRATES) is not a workspace member"
  fi
done <<CRATES
$crates
CRATES

# ---------------------------------------------------------- workspace lints
if [ "$lints" = yes ]; then
  echo "=== lint-unsafe: workspace lints ==="
  # unsafe_code = "deny" or "forbid", or a table with that level, in
  # [workspace.lints.rust].
  if awk '/^[[:space:]]*\[/ { inrust = ($0 ~ /^[[:space:]]*\[workspace\.lints\.rust\][[:space:]]*(#.*)?$/) }
      inrust && /^[[:space:]]*unsafe_code[[:space:]]*=[[:space:]]*("(deny|forbid)"|\{[^}]*level[[:space:]]*=[[:space:]]*"(deny|forbid)")/ { found = 1 }
      END { exit !found }' "$manifest"; then
    pass "$(rel "$manifest") denies unsafe_code for the workspace"
  else
    fail "$(rel "$manifest") lacks unsafe_code = \"deny\" (or \"forbid\") in [workspace.lints.rust]"
  fi
  while IFS= read -r m; do
    ! is_fence "$m" || continue
    # workspace = true in a [lints] table, or, before the first table,
    # lints.workspace = true or lints = { workspace = true }.
    if awk '/^[[:space:]]*\[/ { intable = 1; inlints = ($0 ~ /^[[:space:]]*\[lints\][[:space:]]*(#.*)?$/); next }
        inlints && /^[[:space:]]*workspace[[:space:]]*=[[:space:]]*true/ { found = 1 }
        !intable && /^[[:space:]]*lints[[:space:]]*(\.[[:space:]]*workspace[[:space:]]*=[[:space:]]*true|=[[:space:]]*\{[^}]*workspace[[:space:]]*=[[:space:]]*true)/ { found = 1 }
        END { exit !found }' "$m"; then
      pass "$(rel "$m") inherits the workspace lints"
    else
      fail "$(rel "$m") lacks [lints] workspace = true"
    fi
  done <<MANIFESTS
$manifests
MANIFESTS
fi

# --------------------------------------------------------------- crate roots
echo "=== lint-unsafe: crate roots ==="
while IFS= read -r m; do
  ! is_fence "$m" || continue
  dir=$(dirname "$m")
  n=0
  for r in "$dir/src/lib.rs" "$dir/src/main.rs" "$dir"/src/bin/*.rs "$dir"/src/bin/*/main.rs; do
    [ -f "$r" ] || continue
    n=$((n + 1))
    if root_forbids "$r"; then
      pass "$(rel "$r")"
    else
      fail "$(rel "$r") lacks #![forbid(unsafe_code)] (or the Verus pair, both lines)"
    fi
  done
  [ "$n" -gt 0 ] || fail "$(rel "$dir") has no crate root (src/lib.rs, src/main.rs, src/bin/)"
done <<MANIFESTS
$manifests
MANIFESTS

# ---------------------------------------------------------- forbid check
echo "=== lint-unsafe: every other target with -Funsafe_code ==="
# The targets of the members but the fence crates, one
# `PACKAGE<tab>GATED<tab>CRATE<tab>SRC_PATH` a line (GATED is 1 for a
# target with required-features). A target belongs to the package whose
# object in cargo metadata's "packages" array holds it in its "targets"
# array (pkg_targets below reads the JSON's structure, strings and
# escapes included), whatever directory its src_path is in: a target
# path may lie outside its package's directory, or inside another
# member's. cargo check --all-targets skips,
# without a word, a target whose required-features the run does not
# turn on, and a test or bench target that says `test = false` or
# `bench = false`; the forbid never reaches one it skips. So the wrapper
# logs every source file rustc compiles for these members, and each
# target's src_path must be among them (below). A target with
# required-features fails here, with what to do, unless
# UNSAFE_CHECK_FLAGS hold --all-features.
printf '%s' "$members" > "$plant/members"
#
# A target is its package, its crate name (its name with - as _, as
# cargo passes --crate-name) and its src_path, all of which the wrapper
# logs: two
# targets of one member that share a src_path (a skipped test and an
# example compiling the same file, say) fail the run, since a compile
# of one would not show the other was checked. A member but the fence
# crates for which no target is found refuses the run: every package
# has one, so none means the metadata was misread.
# pkg_targets: from cargo metadata's JSON on its input, one
# `MANIFEST_PATH<tab>TARGET_OBJECT` line for each object in the
# "targets" array of each object in the root's "packages" array. It
# walks the text a character at a time, keeping the nesting level and
# the key each level last named, and skips the inside of strings, so
# braces, brackets or keys in a string (a description) count for
# nothing.
# shellcheck disable=SC2016  # awk's program, not the shell's
pkg_targets='{ s = s $0 "\n" }
END {
  n = length(s); lv = 0; instr = 0; esc = 0; str = ""
  for (i = 1; i <= n; i++) {
    c = substr(s, i, 1)
    if (intgt) obj = obj c
    if (instr) {
      if (esc) { esc = 0; str = str c }
      else if (c == "\\") { esc = 1; str = str c }
      else if (c == "\"") {
        instr = 0
        if (substr(s, i + 1, 1) == ":") key[lv] = str
        else if (lv == 3 && inpkgs && key[3] == "manifest_path") mp = str
      } else str = str c
      continue
    }
    if (c == "\"") { instr = 1; str = ""; continue }
    if (c == "{" || c == "[") {
      lv++; key[lv] = ""
      if (lv == 2 && c == "[" && key[1] == "packages") inpkgs = 1
      if (lv == 3 && inpkgs && c == "{") { mp = ""; nt = 0 }
      if (lv == 4 && inpkgs && c == "[" && key[3] == "targets") intg = 1
      if (lv == 5 && intg && c == "{") { intgt = 1; obj = c }
    } else if (c == "}" || c == "]") {
      if (lv == 5 && intgt) { t[++nt] = obj; intgt = 0 }
      if (lv == 4 && intg) intg = 0
      if (lv == 3 && inpkgs && c == "}") for (k = 1; k <= nt; k++) print mp "\t" t[k]
      if (lv == 2 && inpkgs) inpkgs = 0
      lv--
    }
  }
}'
targets=$(printf '%s\n' "$meta" | awk "$pkg_targets" |
  awk -F "$tab" '
    NR == FNR { pkg[$2] = $1; next }
    ($1 in pkg) {
      g = index($2, "\"required-features\":[\"") > 0
      if (!match($2, /"name":"[^"]*"/)) next
      n = substr($2, RSTART + 8, RLENGTH - 9); gsub(/-/, "_", n)
      if (match($2, /"src_path":"[^"]*"/)) print pkg[$1] "\t" g "\t" n "\t" substr($2, RSTART + 12, RLENGTH - 13) }' "$plant/members" - |
  while IFS="$tab" read -r p g n src; do
    printf '%s\n' "$crates" | grep -Fqx -- "$p" || printf '%s\t%s\t%s\t%s\n' "$p" "$g" "$n" "$src"
  done)
untargeted=$(printf '%s\n' "$others" | while IFS= read -r p; do
  printf '%s\n' "$targets" | cut -f1 | grep -Fqx -- "$p" || echo "$p"
done)
if [ -n "$untargeted" ]; then
  echo "lint-unsafe: cargo metadata gave no target lint-unsafe.sh could read for $(printf '%s\n' "$untargeted" | tr '\n' ' ' | sed 's/ $//')" >&2
  exit 2
fi
shared=$(printf '%s\n' "$targets" | cut -f1,4 | sort | uniq -d)
while IFS="$tab" read -r p src; do
  [ -n "$src" ] || continue
  fail "$(rel "$src") ($p) is the source of more than one of its targets, so a compile of one would not show the others were checked: give each target its own file"
done <<SHARED
$shared
SHARED
case " ${UNSAFE_CHECK_FLAGS-} " in
  *" --all-features "*) ;;
  *)
    while IFS="$tab" read -r p g n src; do
      [ "$g" = 1 ] || continue
      fail "$(rel "$src") ($p) has required-features, so cargo check --all-targets skips it: add --all-features to UNSAFE_CHECK_FLAGS"
    done <<TARGETS
$targets
TARGETS
    ;;
esac
# The wrapper cargo runs each member's rustc through. It logs each
# member it forbids unsafe code in, so a run in which cargo compiled
# none of them, or left any of them out, fails below.
#
# Two rustc flags outrank a command-line forbid: --cap-lints (rustc caps
# every lint level, forbid too, at its level) and --force-warn (which
# turns a lint into a warning whatever its level). A member's rustc that
# gets either, from RUSTFLAGS, CARGO_ENCODED_RUSTFLAGS or a cargo
# configuration's rustflags (cargo passes --cap-lints itself only for
# dependencies outside the workspace, which the wrapper never sees), is
# refused: the wrapper names the flag and fails that compile.
wrapper=$plant/rustc-forbid-unsafe
cat > "$wrapper" <<'WRAPPER'
#!/bin/sh
# lint-unsafe.sh's RUSTC_WORKSPACE_WRAPPER: cargo runs it as
# `WRAPPER RUSTC ARGS...` for each workspace member.
# refuse FLAG: fail this compile, naming the flag.
refuse() {
  echo "$CARGO_PKG_NAME $1" >> "$LINT_UNSAFE_WEAK"
  echo "lint-unsafe: $CARGO_PKG_NAME's rustc gets $1, which outranks -Funsafe_code: take it out of RUSTFLAGS, CARGO_ENCODED_RUSTFLAGS or the cargo configuration's rustflags" >&2
  exit 1
}
# check ARG: refuse ARG if it outranks the forbid.
check() {
  case $1 in
    --cap-lints | --cap-lints=* | --force-warn | --force-warn=*) refuse "$1" ;;
  esac
}
if [ -n "${CARGO_PKG_NAME-}" ] && ! printf '%s\n' "$LINT_UNSAFE_FENCE" | grep -Fqx -- "$CARGO_PKG_NAME"; then
  for a in "$@"; do
    case $a in
      @*)
        # rustc reads an @FILE argument as the arguments FILE holds, one
        # a line; they are checked as the others are. One it cannot read
        # is refused.
        [ -f "${a#@}" ] && [ -r "${a#@}" ] || refuse "$a (an argument file lint-unsafe.sh cannot read)"
        while IFS= read -r l || [ -n "$l" ]; do
          # rustc splits the file into lines as Rust's str::lines does,
          # which drops a CR before each line break.
          check "${l%"$(printf '\r')"}"
        done < "${a#@}"
        ;;
      *) check "$a" ;;
    esac
  done
  echo "$CARGO_PKG_NAME" >> "$LINT_UNSAFE_LOG"
  # The package, crate name and source file this rustc compiles, as
  # `PACKAGE<tab>CRATE<tab>PATH`, the path absolute: cargo names it
  # relative to the directory it runs rustc in. All three identify a
  # target, so one package's compile of a file never vouches for
  # another package's target of the same name and path.
  crate= src= prev=
  for a in "$@"; do
    if [ "$prev" = --crate-name ]; then
      crate=$a
    else
      case $a in
        -*) ;;
        /*.rs) src=$a ;;
        *.rs) src=$(pwd)/$a ;;
      esac
    fi
    prev=$a
  done
  printf '%s\t%s\t%s\n' "$CARGO_PKG_NAME" "$crate" "$src" >> "$LINT_UNSAFE_SRCS"
  set -- "$@" -Funsafe_code
fi
if [ -n "${LINT_UNSAFE_INNER-}" ]; then
  exec "$LINT_UNSAFE_INNER" "$@"
fi
exec "$@"
WRAPPER
chmod +x "$wrapper"
log=$plant/forbidden
weak=$plant/weakened
srcs=$plant/sources
: > "$log"
: > "$weak"
: > "$srcs"
set -- cargo check --locked --workspace --all-targets --manifest-path "$manifest"
# The port's flags split on blanks, as make passes them.
# shellcheck disable=SC2086
set -- "$@" ${UNSAFE_CHECK_FLAGS-}
fenced=$(printf '%s\n' "$crates" | tr '\n' ' ' | sed 's/ $//')
echo "+ $* (rustc -Funsafe_code for every member but $fenced)"
# The caller's wrapper, which this one runs.
inner=${RUSTC_WORKSPACE_WRAPPER-}
if (cd "$root" &&
  LINT_UNSAFE_FENCE=$crates LINT_UNSAFE_LOG=$log LINT_UNSAFE_WEAK=$weak LINT_UNSAFE_SRCS=$srcs LINT_UNSAFE_INNER=$inner \
    RUSTC_WORKSPACE_WRAPPER=$wrapper "$@"); then
  checked=1
else
  checked=
fi
if [ -s "$weak" ]; then
  fail "a member's rustc gets a flag that outranks -Funsafe_code ($(sort -u "$weak" | tr '\n' ';' | sed 's/;$//; s/;/; /g')): take it out of RUSTFLAGS, CARGO_ENCODED_RUSTFLAGS or the cargo configuration's rustflags"
elif [ -n "$checked" ]; then
  # Every member but the fence crates must have been compiled through
  # the wrapper: one that UNSAFE_CHECK_FLAGS leave out (--exclude, -p),
  # or that cargo took as fresh from a build without it, was not checked.
  missing=$(printf '%s\n' "$others" | while IFS= read -r p; do
    grep -Fqx -- "$p" "$log" || echo "$p"
  done)
  if [ ! -s "$log" ]; then
    fail "cargo check compiled no member but $fenced through the wrapper, so it checked nothing"
  elif [ -n "$missing" ]; then
    fail "cargo check compiled $(printf '%s\n' "$missing" | tr '\n' ' ' | sed 's/ $//') through no wrapper, so it checked nothing for them (UNSAFE_CHECK_FLAGS may leave out no member)"
  else
    # Every target of those members, by its src_path: one cargo
    # skipped (test = false, bench = false, required-features off) was
    # not checked.
    skipped=$(printf '%s\n' "$targets" | while IFS="$tab" read -r p g n src; do
      [ -n "$src" ] || continue
      grep -Fqx -- "$p$tab$n$tab$src" "$srcs" || printf '%s (%s, target %s)\n' "$(rel "$src")" "$p" "$n"
    done)
    if [ -n "$skipped" ]; then
      printf '%s\n' "$skipped" | while IFS= read -r t; do
        fail "$t: cargo check --all-targets did not compile this target (test = false, bench = false, or required-features off), so the forbid never reached it"
      done
      fail_count=$((fail_count + $(printf '%s\n' "$skipped" | wc -l)))
    else
      pass "every target but $fenced's checks with unsafe_code forbidden (members checked: $(sort -u "$log" | wc -l | tr -d ' '), targets: $(printf '%s\n' "$targets" | grep -c .))"
    fi
  fi
else
  fail "a crate other than $fenced uses unsafe code, lowers unsafe_code or does not build (cargo check above)"
fi

if [ "$fail_count" -ne 0 ]; then
  echo "lint-unsafe: $fail_count failed" >&2
  exit 1
fi
echo "lint-unsafe: PASS"
