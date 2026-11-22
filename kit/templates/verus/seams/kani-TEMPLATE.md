# `{ITEM}`

Seam {SEAM_ID} of `{SEAMS_TABLE}`: `{ITEM}` ({KIND}, `{FILE}`). Discharged
by Kani {KANI_RELEASE}. The harnesses below check the contract as Verus
states it.

{Name this file `kani-<seam id in lower case>.md`, as `kani-s1.md` or
`kani-s-cr9.md`. The kit counter (verus-count) fails a run in which a
function carries the
`#[cfg_attr(any(), discharged_by = "kani")]` marker and no record names
its file in backquotes and quotes `fn NAME`; section 1 is where it looks.}

{For shipped functions outside `verus!`, which have no seam and no
Verus contract, write one record per source file, covering every marked
function in it: name it `kani-<crate>-<path under src/ without .rs,
each / as ->.md`, as `kani-ffi-stream.md` for `crates/ffi/src/stream.rs`
or `kani-ffi-io-read.md` for `crates/ffi/src/io/read.rs`, so no two
records collide. Title it with the file, and open with "`{FILE}`,
outside `verus!`. Discharged by Kani {KANI_RELEASE}. The harnesses
below check the properties section 1 states.", then list each
function by its path in backquotes, a method with its owner, as
`A::new`, `B<u8>::new` or `<A as From<u8>>::from`, a free function with
its inline modules, as `a::init`, or alone at the file's top, as `init`
(kit/count/README.md, "Two ways to discharge a seam"). In
section 1 quote each function's doc comment, signature and marker line,
then state below the block each property the harnesses assert. Section
2 names each harness's file and name in backquotes, as it does for a
seam: the counter fails a marked function outside `verus!` unless a
record that covers it names it by its path and names the file and name
of a `#[kani::proof]` harness of its crate whose body calls it (and,
for a method, mentions its owner). Sections 2 and 3 then read those
properties where they say `requires` and `ensures`.}

## 1. Contract

At this record's commit:

```rust
{CONTRACT: quoted from the source: the doc comment, the signature
with `requires` and `ensures`, and, for a trusted function, its
`#[cfg_attr(any(), discharged_by = "kani")]` marker line.}
```

## 2. Statement

Kani {KANI_RELEASE}, from the `kani` package that the `verus` check and
the development sessions install, run by `make kani`, reports
`VERIFICATION:- SUCCESSFUL` for each harness:

- `{HARNESS_FILE}`, `{HARNESS_NAME}`.

Each harness assumes the contract's `requires`, calls the function on the
inputs section 3 gives, and asserts each `ensures` clause, with spec
functions written out as executable code.

## 3. Translation

- **Inputs.** {Every value of the type, or the bound: lengths up to N,
  every alignment, offsets 0 to K. Say what lies past the bound and that
  it stays trusted.}
- **Spec functions.** {Each spec function the contract names, and the
  executable code the harness writes it as.}
- **Toolchain.** The package builds `kani-compiler` and the `core`,
  `alloc` and `std` Kani checks against from rust {RUSTC_VERSION}, the
  `{RUST_STREAM}` release {PORT} builds with and Verus runs on, so the
  harnesses check that release's code.

## 4. Review

Written by {AUTHOR} for {CHANGE}, {DATE}. {Reviewed by {PERSON} on
{DATE} | Not yet reviewed by a person.}

## 5. Tests

Mutation check (Kani reports `VERIFICATION:- FAILED` for each):
{MUTANT: a change to the harness or the function that breaks one
`ensures`}. `make kani` re-runs the harnesses on every pull request (the
`verus` check). {Until a check compares section 1 with the source: a
change to the contract, or to the types the proofs use it at, needs the
harness and this record revisited by hand.}
