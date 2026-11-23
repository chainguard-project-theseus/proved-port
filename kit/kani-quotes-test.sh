#!/bin/sh
# shellcheck disable=SC2016 # the records' Markdown backquotes are literal text
# kit/kani-quotes-test.sh: tests of kit/make/kani-quotes.sh (the
# kani-quotes-test check). Each case writes Kani records and the sources
# they quote by hand, runs the script, and checks its exit status and its
# whole output, spelled out by hand (none is taken from a run of the
# script under test): a record that matches (two chunks, other
# indentation, reworded and dropped comments), a changed clause, a second
# chunk that differs, a chunk that runs past the source's end, a changed
# end-of-line comment, the source named by the first backquoted .rs
# path before the first heading (not a later one, not another
# backquoted path), a second root, the record parts it needs (no
# source, a source under no root, no section 1, no fenced block there,
# a fence in another section or before section 1, an unclosed fence,
# every block of section 1 read (a drifted second block, a second
# block left open),
# comments alone, an empty block), a source under both roots, where a
# chunk must end (a clause added after the quote's last one, the
# signature alone, a last line ending with `{` or `;`, the body or `;`
# next, the source's end, a later place that ends), several records in
# name order, SKIP (a failing record skipped is read and says why, a
# matching one fails, a name that is no record or no kani-*.md name),
# a directory with no record
# or only skipped ones, a missing directory and usage. Run it from the
# repository root. It needs a POSIX shell and BusyBox's tools.
set -eu

script=$(pwd)/kit/make/kani-quotes.sh
[ -f "$script" ] || { echo "FAIL: run it from the repository root (no kit/make/kani-quotes.sh)" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# run ARGS...: the script with ARGS; its output (stdout and stderr) in
# out, its status in rc.
run() {
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

# rec DIR NAME OPENING: a record DIR/NAME whose opening line is OPENING,
# with the section 1 quote read from stdin.
rec() {
  {
    printf '# `x`\n\n%s\n\n## 1. Contract\n\nAt this record'"'"'s commit:\n\n```rust\n' "$3"
    cat
    printf '```\n\n## 2. Statement\n\n- `src/kani/a.rs`, `a_get`.\n'
  } > "$1/$2"
}

mkdir -p seams src vs/std_specs
cat > src/a.rs <<'EOF'
use vstd::prelude::*;

verus! {

/// Gets element `i` of `v`.
///
/// Trusted (S-a1): an index within bounds.
#[cfg_attr(any(), discharged_by = "kani")]
#[verifier::external_body]
pub fn get(v: &Vec<u8>, i: usize) -> (r: u8)
    requires
        i < v.len(),
    ensures
        r == v[i as int],
{
    v[i]  // in bounds
}

/// The length of `v`.
#[verifier::external_body]
pub fn len(v: &Vec<u8>) -> (r: usize)
    ensures
        r == v.len(),
{
    v.len()
}

} // verus!
EOF

# A record that matches: two chunks, its own indentation, a reworded
# doc comment, a comment the source lacks.
rec seams kani-s-a1.md 'Seam S-a1 of `verus/theorems/T1.md`: `get` (external_body, `src/a.rs`).' <<'EOF'
/// Gets element `i` of `v`, as the record words it.
#[cfg_attr(any(), discharged_by = "kani")]
#[verifier::external_body]
pub fn get(v: &Vec<u8>, i: usize) -> (r: u8)
  requires
    i < v.len(),
  // a note of the record's own
  ensures
    r == v[i as int],

#[verifier::external_body]
pub fn len(v: &Vec<u8>) -> (r: usize)
    ensures  r == v.len(),
EOF
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: chunk 2 of section 1 is not in ./src/a.rs: its first 2 lines are, then: ensures r == v.len(),
EOF
want "a clause the source splits over two lines fails" 1
rec seams kani-s-a1.md 'Seam S-a1 of `verus/theorems/T1.md`: `get` (external_body, `src/a.rs`).' <<'EOF'
/// Gets element `i` of `v`, as the record words it.
#[cfg_attr(any(), discharged_by = "kani")]
#[verifier::external_body]
pub fn get(v: &Vec<u8>, i: usize) -> (r: u8)
  requires
    i < v.len(),
  // a note of the record's own
  ensures
    r == v[i as int],


#[verifier::external_body]
	pub fn len(v: &Vec<u8>)   -> (r: usize)
    ensures
        r == v.len(),
{
    v.len()
}
EOF
run seams .
cat > want <<'EOF'
ok   kani-s-a1.md: 2 chunks match ./src/a.rs
EOF
want "a record that matches its source" 0
cp seams/kani-s-a1.md a1.good

# Trailing blanks and a tab on a quoted line are not compared.
printf 'pub fn len(v: &Vec<u8>) -> (r: usize) \t \n    ensures\t\n        r == v.len(), \n' | rec seams kani-s-a1.md 'Seam S-a1: `len` (external_body, `src/a.rs`).'
run seams .
cat > want <<'EOF'
ok   kani-s-a1.md: 1 chunks match ./src/a.rs
EOF
want "trailing blanks on quoted lines match" 0

# A changed clause: the chunk's longest match and the line after it.
rec seams kani-s-a1.md 'Seam S-a1: `get` (external_body, `src/a.rs`).' <<'EOF'
#[verifier::external_body]
pub fn get(v: &Vec<u8>, i: usize) -> (r: u8)
    requires
        i <= v.len(),
EOF
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: chunk 1 of section 1 is not in ./src/a.rs: its first 3 lines are, then: i <= v.len(),
EOF
want "a changed requires clause fails" 1

# A chunk that runs past the source's end: its last line is not there.
rec seams kani-s-a1.md 'Seam S-a1: `get` (external_body, `src/a.rs`).' <<'EOF'
} // verus!
}
EOF
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: chunk 1 of section 1 is not in ./src/a.rs: its first 1 lines are, then: }
EOF
want "a chunk past the source's end fails" 1

# A comment at the end of a code line is part of it.
rec seams kani-s-a1.md 'Seam S-a1: `get` (external_body, `src/a.rs`).' <<'EOF'
{
    v[i]  // in range
}
EOF
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: chunk 1 of section 1 is not in ./src/a.rs: its first 1 lines are, then: v[i] // in range
EOF
want "a changed comment at a line's end fails" 1
rec seams kani-s-a1.md 'Seam S-a1: `get` (external_body, `src/a.rs`).' <<'EOF'
{
    v[i]  // in bounds
}
EOF
run seams .
cat > want <<'EOF'
ok   kani-s-a1.md: 1 chunks match ./src/a.rs
EOF
want "a body quoted with its end-of-line comment matches" 0

# The source is the first backquoted .rs path before the first heading:
# not another backquoted path, not one after the heading.
cp src/a.rs src/b.rs
rec seams kani-s-a1.md 'Seam S-a1 of `verus/SEAMS.md` (`get`, in `src/b.rs`, beside `src/a.rs`).' <<'EOF'
pub fn len(v: &Vec<u8>) -> (r: usize)
    ensures
        r == v.len(),
EOF
run seams .
cat > want <<'EOF'
ok   kani-s-a1.md: 1 chunks match ./src/b.rs
EOF
want "the first backquoted .rs path names the source" 0
rec seams kani-s-a1.md 'Seam S-a1 of `verus/SEAMS.md`: `get`, with no file.' <<'EOF'
pub fn len(v: &Vec<u8>) -> (r: usize)
EOF
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: names no source (a backquoted .rs path before its first heading)
EOF
want "a record that names no source before its first heading fails" 1

# A second root: a record that quotes vstd's std_specs.
cat > vs/std_specs/option.rs <<'EOF'
pub assume_specification<T: Copy>[ Option::<&T>::copied ](opt: Option<&T>) -> (res: Option<T>)
    ensures
        opt.is_none() ==> res.is_none(),
;
EOF
rec seams kani-s-a1.md 'Seam S3: `Option::copied` (assume_spec, `std_specs/option.rs`).' <<'EOF'
pub assume_specification<T: Copy>[ Option::<&T>::copied ](opt: Option<&T>) -> (res: Option<T>)
    ensures
        opt.is_none() ==> res.is_none(),
EOF
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: its source std_specs/option.rs is under no root of: .
EOF
want "a source under no root fails" 1
run seams ". vs"
cat > want <<'EOF'
ok   kani-s-a1.md: 1 chunks match vs/std_specs/option.rs
EOF
want "a source under the second root matches" 0
# Under both roots, the first root's file is the one read.
mkdir -p vs/src
printf 'pub fn len(v: &Vec<u8>) -> (r: u64)\n' > vs/src/a.rs
rec seams kani-s-a1.md 'Seam S-a1: `len` (external_body, `src/a.rs`).' <<'EOF'
pub fn len(v: &Vec<u8>) -> (r: usize)
    ensures
        r == v.len(),
EOF
run seams ". vs"
cat > want <<'EOF'
ok   kani-s-a1.md: 1 chunks match ./src/a.rs
EOF
want "a source under both roots is the first root's" 0
run seams "vs ."
cat > want <<'EOF'
FAIL kani-s-a1.md: chunk 1 of section 1 is not in vs/src/a.rs: its first 0 lines are, then: pub fn len(v: &Vec<u8>) -> (r: usize)
EOF
want "roots are tried in their order" 1
rm -r vs/src

# A fenced block before section 1 is not the quote, and a blank line
# at the quote's end opens no chunk.
printf '# `x`\n\nSeam S-a1: `get` (`src/a.rs`).\n\n```text\nnot code\n```\n\n## 1. Contract\n\n```rust\npub fn len(v: &Vec<u8>) -> (r: usize)\n    ensures\n        r == v.len(),\n\n```\n' > seams/kani-s-a1.md
run seams .
cat > want <<'EOF'
ok   kani-s-a1.md: 1 chunks match ./src/a.rs
EOF
want "a fence before section 1 is not the quote" 0

# The record parts it needs.
printf '# `x`\n\nSeam S-a1: `get` (`src/a.rs`).\n\n## 2. Statement\n\n```rust\npub fn get\n```\n' > seams/kani-s-a1.md
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: has no section 1 (a heading opening `## 1.`)
EOF
want "a record with no section 1 fails" 1
printf '# `x`\n\nSeam S-a1: `get` (`src/a.rs`).\n\n## 1. Contract\n\nNo block.\n\n## 2. Statement\n\n```rust\npub fn get(v: &Vec<u8>, i: usize) -> (r: u8)\n```\n' > seams/kani-s-a1.md
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: section 1 has no fenced block
EOF
want "a fence in section 2 alone is no quote" 1
printf '# `x`\n\nSeam S-a1: `get` (`src/a.rs`).\n\n## 1. Contract\n\n```rust\npub fn get(v: &Vec<u8>, i: usize) -> (r: u8)\n' > seams/kani-s-a1.md
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: section 1's fenced block is not closed
EOF
want "an unclosed fence fails" 1
printf '# `x`\n\nSeam S-a1: `get` (`src/a.rs`).\n\n## 1. Contract\n\n```rust\npub fn get(v: &Vec<u8>, i: usize) -> (r: u8)\n\n## 2. Statement\n\n```\n' > seams/kani-s-a1.md
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: section 1's fenced block is not closed
EOF
want "a fence section 1 leaves open fails" 1

# Every fenced block of section 1 is read (a per-file record, one block
# per function), each block's end a chunk break; a block in section 2
# is not read.
printf 'pub fn a(x: u8) -> u8 {\n    x\n}\n\npub fn b(x: u8) -> u8 {\n    x\n}\n' > src/f.rs
printf '# `x`\n\nPer-file record of `src/f.rs`.\n\n## 1. Contract\n\n```rust\npub fn a(x: u8) -> u8 {\n```\n\nProperties of a.\n\n```rust\npub fn b(x: u8) -> u8 {\n```\n\n## 2. Statement\n\n```rust\nnot in the source\n```\n' > seams/kani-s-a1.md
run seams .
cat > want <<'EOF'
ok   kani-s-a1.md: 2 chunks match ./src/f.rs
EOF
want "two blocks in section 1, both matching, and one in section 2" 0
printf '# `x`\n\nPer-file record of `src/f.rs`.\n\n## 1. Contract\n\n```rust\npub fn a(x: u8) -> u8 {\n```\n\nProperties of a.\n\n```rust\npub fn b(x: u16) -> u16 {\n```\n' > seams/kani-s-a1.md
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: chunk 2 of section 1 is not in ./src/f.rs: its first 0 lines are, then: pub fn b(x: u16) -> u16 {
EOF
want "a second block that drifted fails" 1
printf '# `x`\n\nPer-file record of `src/f.rs`.\n\n## 1. Contract\n\n```rust\npub fn a(x: u8) -> u8 {\n```\n\n```rust\npub fn b(x: u8) -> u8 {\n' > seams/kani-s-a1.md
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: section 1's fenced block is not closed
EOF
want "a second block left open fails" 1
rec seams kani-s-a1.md 'Seam S-a1: `get` (external_body, `src/a.rs`).' <<'EOF'
/// Only a doc comment.

// And a plain one.
EOF
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: section 1 quotes no code (comments alone, or nothing)
EOF
want "a quote of comments alone fails" 1
printf '# `x`\n\nSeam S-a1: `get` (`src/a.rs`).\n\n## 1. Contract\n\n```rust\n```\n' > seams/kani-s-a1.md
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: section 1 quotes no code (comments alone, or nothing)
EOF
want "an empty fenced block fails" 1

# A chunk ends where its item's text does: a clause the source adds
# after the quote's last one fails, and so does a chunk of the
# signature alone; a last line ending with `{`, a source that ends, and
# a later place where the chunk ends pass.
cat > src/c.rs <<'EOF'
#[verifier::external_body]
pub fn get(v: &Vec<u8>, i: usize) -> (r: u8)
    requires
        i < v.len(),
    ensures
        r == v[i as int],
        r == 0,
{
    v[i]
}

pub trait Size {
    fn size(&self) -> (r: usize);
    fn other(&self);
}

pub fn one() -> (r: u8) {
    1
}

#[verifier::external_body]
pub fn get(v: &Vec<u8>, i: usize) -> (r: u8)
    requires
        i < v.len(),
    ensures
        r == v[i as int],
;
pub fn last() -> (r: u8)
    ensures
        r == 2,
EOF
rec seams kani-s-a1.md 'Seam S-a1: `get` (external_body, `src/c.rs`).' <<'EOF'
#[verifier::external_body]
pub fn get(v: &Vec<u8>, i: usize) -> (r: u8)
    requires
        i < v.len(),
    ensures
        r == v[i as int],
        r == 0,
EOF
run seams .
cat > want <<'EOF'
ok   kani-s-a1.md: 1 chunks match ./src/c.rs
EOF
want "a chunk through the item's last clause matches" 0
printf 'pub fn get(v: &Vec<u8>, i: usize) -> (r: u8)\n' | rec seams kani-s-a1.md 'Seam S-a1: `get` (external_body, `src/c.rs`).'
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: chunk 1 of section 1 stops before its item does in ./src/c.rs, which goes on: requires
EOF
want "a chunk of the signature alone fails" 1
sed '/r == 0,/d; /^;$/d' src/c.rs > src/d.rs
rec seams kani-s-a1.md 'Seam S-a1: `get` (external_body, `src/d.rs`).' <<'EOF'
pub fn get(v: &Vec<u8>, i: usize) -> (r: u8)
    requires
        i < v.len(),
    ensures
        r == v[i as int],
EOF
run seams .
cat > want <<'EOF'
ok   kani-s-a1.md: 1 chunks match ./src/d.rs
EOF
want "a chunk the body follows matches" 0
cp src/c.rs src/d.rs
run seams .
cat > want <<'EOF'
ok   kani-s-a1.md: 1 chunks match ./src/d.rs
EOF
want "a chunk that goes on at one place and ends at a later one matches" 0
sed '/^;$/d' src/c.rs > src/d.rs
run seams .
cat > want <<'EOF'
FAIL kani-s-a1.md: chunk 1 of section 1 stops before its item does in ./src/d.rs, which goes on: r == 0,
EOF
want "a clause added after the quote's last one fails" 1
printf 'pub fn one() -> (r: u8) {\n\npub fn last() -> (r: u8)\n    ensures\n        r == 2,\n' | rec seams kani-s-a1.md 'Seam S-a1: `one` (`src/c.rs`).'
run seams .
cat > want <<'EOF'
ok   kani-s-a1.md: 2 chunks match ./src/c.rs
EOF
want "a chunk ending with { and one at the source's end match" 0
printf 'fn size(&self) -> (r: usize);\n' | rec seams kani-s-a1.md 'Seam S-a1: `size` (`src/c.rs`).'
run seams .
cat > want <<'EOF'
ok   kani-s-a1.md: 1 chunks match ./src/c.rs
EOF
want "a chunk ending with ; matches" 0

# Several records, in name order; one fails, so the run does.
cp a1.good seams/kani-s-a1.md
rec seams kani-s-a0.md 'Seam S-a0: `len` (external_body, `src/a.rs`).' <<'EOF'
pub fn len(v: &Vec<u8>) -> (r: u64)
EOF
run seams .
cat > want <<'EOF'
FAIL kani-s-a0.md: chunk 1 of section 1 is not in ./src/a.rs: its first 0 lines are, then: pub fn len(v: &Vec<u8>) -> (r: u64)
ok   kani-s-a1.md: 2 chunks match ./src/a.rs
EOF
want "two records, the first failing" 1
# Files that are not kani-*.md records are not read.
printf 'not a record\n' > seams/SEAMS.md
printf 'not a record\n' > seams/kani-notes.txt

# SKIP: a failing record skipped is still read, and says why it fails;
# a skipped record that matches fails the run; names that are no
# record, or no kani-*.md name, are usage errors.
run seams . kani-s-a0.md
cat > want <<'EOF'
skip kani-s-a0.md: chunk 1 of section 1 is not in ./src/a.rs: its first 0 lines are, then: pub fn len(v: &Vec<u8>) -> (r: u64)
ok   kani-s-a1.md: 2 chunks match ./src/a.rs
EOF
want "a failing record SKIP names passes, with a skip line" 0
run seams . "kani-s-a1.md kani-s-a0.md"
cat > want <<'EOF'
skip kani-s-a0.md: chunk 1 of section 1 is not in ./src/a.rs: its first 0 lines are, then: pub fn len(v: &Vec<u8>) -> (r: u64)
FAIL kani-s-a1.md: SKIP names it, but 2 chunks match ./src/a.rs: take it out of SKIP
FAIL: seams holds no kani-*.md record but those SKIP names
EOF
want "a matching record SKIP names fails" 1
mkdir seams2
cp seams/kani-s-a0.md seams/kani-s-a1.md seams2/
printf '# `x`\n\nSeam S-a2: `len` (`src/none.rs`).\n' > seams2/kani-s-a2.md
run seams2 . "kani-s-a1.md kani-s-a2.md"
cat > want <<'EOF'
FAIL kani-s-a0.md: chunk 1 of section 1 is not in ./src/a.rs: its first 0 lines are, then: pub fn len(v: &Vec<u8>) -> (r: u64)
FAIL kani-s-a1.md: SKIP names it, but 2 chunks match ./src/a.rs: take it out of SKIP
skip kani-s-a2.md: its source src/none.rs is under no root of: .
EOF
want "a skipped record's other failures are skip lines" 1
run seams . "kani-s-a0.md kani-s-a9.md"
cat > want <<'EOF'
usage: kani-quotes.sh DIR ROOTS [SKIP]: SKIP names kani-s-a9.md, which is no record in seams
EOF
want "SKIP naming no record is usage" 2
run seams . SEAMS.md
cat > want <<'EOF'
usage: kani-quotes.sh DIR ROOTS [SKIP]: SKIP names SEAMS.md, which is not a kani-*.md name
EOF
want "SKIP naming another file is usage" 2

# A directory with no record, or only skipped ones, checks nothing.
mkdir empty
printf 'x\n' > empty/README.md
run empty .
cat > want <<'EOF'
FAIL: empty holds no kani-*.md record
EOF
want "a directory with no record fails" 1
mkdir seams3
cp seams2/kani-s-a0.md seams2/kani-s-a2.md seams3/
run seams3 . "kani-s-a0.md kani-s-a2.md"
cat > want <<'EOF'
skip kani-s-a0.md: chunk 1 of section 1 is not in ./src/a.rs: its first 0 lines are, then: pub fn len(v: &Vec<u8>) -> (r: u64)
skip kani-s-a2.md: its source src/none.rs is under no root of: .
FAIL: seams3 holds no kani-*.md record but those SKIP names
EOF
want "a directory whose records are all skipped fails" 1
run none .
cat > want <<'EOF'
FAIL: none is not a directory
EOF
want "a missing directory fails" 1

# Usage.
run seams
cat > want <<'EOF'
usage: kani-quotes.sh DIR ROOTS [SKIP]
EOF
want "one argument is usage" 2
run seams . "" extra
want "four arguments are usage" 2
run seams " "
cat > want <<'EOF'
usage: kani-quotes.sh DIR ROOTS [SKIP]: ROOTS is empty
EOF
want "blank ROOTS is usage" 2
echo PASS
