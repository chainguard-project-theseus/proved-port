# Contributing to {PORT}

{Who may contribute, and whom to contact.}

Read `README.md`, then `docs/design.md` and `AGENTS.md`. The research
behind the design is in `docs/research/`. Check it before re-researching
anything.

## Support scope

{PORT} supports {PLATFORMS} on {ARCHITECTURES}. It targets one upstream
release at a time, currently {LIBRARY} {UPSTREAM_VERSION}. {Out of scope:
...}

## Development environment

{How to run the gate: natively on Wolfi with the packages the CI
environment names, or in a development session. The Makefile targets
(`build`, `test`, `lint`, `smoke`, `verus`, `kani`, `bench`) and what
each runs.}

Toolchain: the Wolfi stream `{RUST_STREAM}` is used in the check files,
the development environment and the package's `rust-version`; bump them
together. The `verus` package (pinned `verus~{VERUS_RELEASE}`) depends
on that stream, and its release is the one `Cargo.toml`'s `vstd` pin
names, so a Verus bump moves the stream, the `verus` pins and the vstd
pin together. The `kani` package (pinned `kani~{KANI_RELEASE}`) is built
against that stream too, and its release is the one the Kani records
name, so a Kani bump moves the pins and the records together.

## Hard rules

- **ABI.** {PORT} matches {LIBRARY} {UPSTREAM_VERSION}: the header
  constants, exported names, sonames, version nodes, error codes and
  texts, and the public struct layouts. Any deliberate difference gets a
  `docs/abi-changes.md` entry in the same PR, in that file's format.
- **Exports.** Every exported function is a `pub extern "C"` function
  with `#[unsafe(no_mangle)]`, listed in its library's map
  (`{MAP_FILES}`). The ABI check fails if an export set and its map
  differ.
- **Headers.** {The headers are upstream's; only a marked block is
  ours.} The header check must pass.
- **Stable Rust only.**
- **Generated files** ({GENERATED_FILES}) are regenerated with their
  generator, never edited by hand.

## Security rules

1. `unsafe` exists only in `{CAPI_CRATE}`. Every other crate root says
   `#![forbid(unsafe_code)]`, or, in a crate Verus checks,
   `#![cfg_attr(not(verus_keep_ghost), forbid(unsafe_code))]`. Each
   `unsafe` block has a `SAFETY:` comment naming the invariant or caller
   contract it relies on. Keep one unsafe operation per block.
2. Nothing unwinds into C ({Or: the crate aborts on inconsistency.}).
3. No unbounded recursion, no fixed buffers, no unchecked size
   arithmetic. {The port's limits and how they are enforced.}

## Tests

- **Proofs come with the code.** New executable code in a shipped crate
  is written inside `verus!` and verifies (verus/README.md). The `verus`
  check fails when a proof or a Kani harness fails, or when a change
  breaks verus/FLOOR or verus/RLIMIT; the header of each file is its
  rule (how the figures rise, when a `lowered` or `rlimit-raised` line
  is allowed).
- **Security-relevant behaviour gets negative tests**: NULL arguments,
  truncated buffers, foreign pointers, limits that must trip, overflowing
  sizes, unknown option bits. Show that the bad input is refused with the
  right code and that nothing was written.
- A test that cannot check its claim on a host prints a `SKIP <name>
  <reason>` line. It never passes silently.
- C ABI behaviour is tested from C as well.

## Performance

- A performance change carries a written hypothesis with a predicted
  number, then same-run numbers against the C library on a quiet
  machine, with the arms named by sha and every row of the table.
- Code alignment is pinned in `.cargo/config.toml`; keep those flags in
  any build you compare.

## Sending a change

- One logical change per PR, as **one commit**, following the
  workflow in `AGENTS.md`: issue, branch, `Fixes #n`, labels set before
  the push, literal push targets, merge on green with the head pinned.
- {Licence statement.}

## Code review

{The review process: the reviewer bot, its focus file, who must approve
which paths. Answer a finding in its thread: fix it and say where, or
say why it does not apply.}
