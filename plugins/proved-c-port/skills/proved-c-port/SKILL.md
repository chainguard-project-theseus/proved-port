---
name: proved-c-port
description: Port a C library to memory-safe Rust behind a drop-in C ABI, then prove it with Verus and Kani and benchmark it against the C original. Use when asked to "rewrite a C library in Rust", "replace libfoo with a Rust drop-in", keep ABI or soname compatibility, build a differential or fuzz-diff harness against the C oracle, write Verus proofs or Kani harnesses for a port, manage a verified-coverage FLOOR or rlimit headroom, add a JIT or fast path that must be proved equal to the slow path, or measure a port against the C library on bare-metal boxes. Distilled from pcre3 (PCRE2) and challoc (malloc).
---

# Proved C ports

Use this skill when porting a C library to Rust where the Rust library
must load in place of the C one (same header, sonames, symbols, error
codes) and must be proved safe, shown correct against the C original,
and measured at least as fast. The worked examples are wolfi-dev/pcre3
(a PCRE2 10.49 replacement) and wolfi-dev/challoc (a jemalloc-compatible
`malloc`).

The C library is the specification you did not have to write. Every rule
below exists to keep the port tied to it: compare against it, prove what
the comparison cannot reach, and measure against it on the same machine
in the same run.

## When it applies

Apply it when all of these hold:

- An existing C library has real consumers you cannot rebuild or patch
  (distro packages, `LD_PRELOAD` users).
- The port ships as a shared object with the C library's ABI.
- Safety claims will be made about the port (memory safety, bounded
  work, no metadata corruption), so they need proofs, not prose.

Do not apply the proof parts to throwaway prototypes. Prototype first,
measure, then prove the shape that wins (see `references/perf.md`).

## The loop

Every phase has an exit criterion. Do not start a phase before the one
above it meets its criterion. `references/lifecycle.md` has the detail.

1. **ABI inventory and C oracle.** List every exported symbol, version
   node, header constant, struct layout, error code and error text. Load
   the C library as an oracle in the same process as the port. Exit:
   `abi.sh` and `header-check.sh` equivalents pass against the C
   library's own `-dev` package.
2. **Comparison harnesses before code.** Upstream's test suite through
   upstream's driver, a differential runner, a differential fuzzer
   (`fuzz-diff`) and an ABI fuzzer under ASan/UBSan. Exit: each harness
   runs against the C library itself and passes its self-test.
3. **Port.** One issue, one PR, crate boundaries that keep `unsafe` in
   one crate. Exit: the conformance ratchet only shrinks; every
   remaining difference has a numbered ledger entry.
4. **Prove in layers.** Spec first, then refinement layers, each its own
   PR with a theorem file, a seams table and mutants. Exit: the FLOOR
   rises and nothing new is trusted without a record.
5. **Fast paths.** JIT, direct emit, fast tiers: proved equal to the
   slow path, never merely checked at runtime. Exit: the equality
   theorem, the differential legs green on every target arch.
6. **Default on.** Flip the feature in the default build last, with a CI
   leg that keeps the build without it. Exit: every leg runs the native
   code on every arch it claims.

## Non-negotiable rules

Each rule has a correct and a wrong form. The references explain why.

### The C library decides what is correct

Correct: a divergence found by fuzz-diff is a bug, or it gets an
`ABI-nnn` ledger entry and a narrow predicate in `known.c` citing it
(or, where the surface is small, the driver steers around the input
class and cites the entry: `references/abi-compat.md`).

Wrong: widening a known-divergence predicate until the fuzzer goes
quiet, or adding a line to the deviations list without a ledger entry.

### `unsafe` lives in one crate, and nothing unwinds into C

Correct: every crate but the C API crate is `#![forbid(unsafe_code)]`;
every `extern "C"` body runs under a `catch_unwind` guard that returns
an error code. (challoc is `no_std` with `panic = "abort"` instead: an
allocator aborts on inconsistency, it never continues.) Use one guard
shape for every export and lint for it (expat-rs's `ffi_guard` and
`ci/lint-ffi-guard.sh`), with panic-injection tests that the guard
leaks nothing. Where a library hands out long-lived handles, a caught
panic also poisons the handle: later calls on it fail, and the end call
still frees it (chainz's `poison_on_panic`, ABI-013).

Wrong: a safe `fn` pointer type that lets a forbid-unsafe crate jump to
an arbitrary address (pcre3's `RawFind`, flagged in its trust audit).

### New executable code comes with its proofs

Correct: new shipped code sits inside `verus!` and verifies; code that
cannot is one minimal `external_body` or `assume_specification` seam
with a contract, a ledger row, and a Kani record where Kani can check
it.

Wrong: adding a trusted seam to make a proof pass. The spellings are
`assume`, `admit`, `Tracked::assume_new`, `assume_specification`,
`external_body`, `external_fn_specification`,
`external_type_specification`, `external_trait_specification`,
`#[verifier::external]` and `axiom fn`.

### Prove the producer, not its output

Correct: when code or data the library produces must have a property (a
compiled program's shape, an emitted encoding, a table's contents, a
copy's bytes), the producer's `ensures` establishes it. Restructuring the
producer so the contract can be stated is fine when behaviour stays
identical. A checker over the output exists only as a test oracle.
Proving something adds no runtime cost wherever a ghost or spec route
exists: take the harder proof. Until the producer is proved, a shipped
checker is a temporary seam: a seams table row and a `TODO(#n)` naming
the issue that proves the producer (pcre3 #818 kept its checker on the
find tier, tracked by #824). The rule covers passes in the shipped
library over output it produced. An offline generator checked in CI
costs no run, and checks of state a C caller can corrupt (challoc's
metadata checks) are input validation; both are outside it.

Wrong: a verified runtime checker over the output as the shipped
argument, a shipped checker with no tracking issue, or offering one as
the recommended option. It costs every run (pcre3's JIT checker was 47%
of a one-pass native build) and only catches a bad output; a proved
producer never makes one. Equally wrong: an allocation, copy or pass
added only to make a property provable where a ghost or spec route
exists. Where none exists, the cost is measured (`references/proofs.md`,
"Writing verifiable Rust").

### Fast paths are proved equal, not checked

Correct: the emitter's postcondition carries the safety certificate and
the equality to the interpreter; the runtime checker runs only in tests,
as an oracle over corpora and mutants.

Wrong: shipping a runtime checker as the safety argument, or a fast path
whose equality is "tested on the corpus".

### Coverage only ratchets up

Correct: a PR that drops a share closes the gap by verifying more code;
a `lowered FIGURE P REASON` line is the last resort and names exactly
what became trusted.

Wrong: asking a maintainer to lower the floor when more verification is
possible.

### Measure before you prove, and measure against the C library

Correct: a written hypothesis with a predicted number, an unproved
prototype, a same-run box measurement with md5-checked arms named by
sha, then the proof, then a re-measure.

Wrong: proving an optimization before measuring it, or quoting numbers
from different runs side by side.

### Land small and literal

Correct: one commit per PR, signed where the repository's policy
requires it (`references/landing.md`), except a deps agent PR
(`chainguard/deps/<name>`), which holds a commit per turn, is
squash-merged, and is never rewritten on request; labels set before the push,
literal push targets, no update-branch API, every review thread
answered after the push that fixes it.

Wrong: a branch-update merge commit, a computed push target, or a reply
posted before the fix is pushed.

The port's own `AGENTS.md` and `CONTRIBUTING.md` win on any convention
they state (the issue line, who merges, sign-off, the raise rule); this
skill fills in what they leave out.

## References

- [lifecycle.md](references/lifecycle.md): phases, exit criteria, how
  the work was layered.
- [proofs.md](references/proofs.md): Verus spec choice, refinement
  layers, seams, FLOOR, rlimit headroom, mutants, what to prove versus
  test.
- [abi-compat.md](references/abi-compat.md): the oracle, the comparison
  legs, the known-differences ledger, upstream bugs, consumers.
- [perf.md](references/perf.md): hypotheses, arms, the three
  comparisons, boxes, emulated counts, attribution, full tables.
- [landing.md](references/landing.md): commits, stacks, pushes, labels,
  review threads, PR bodies.
- [agent-brief.md](references/agent-brief.md): the block to paste into
  every sub-agent brief.
- [pitfalls.md](references/pitfalls.md): dated lessons, one rule each.
- [allocator.md](references/allocator.md): what is different when the
  C library is `malloc`.

## Audit

Before calling a phase done, check:

- This grep against the base, which matches every trusted-seam spelling
  in the rule above: every new hit has a seam row and a reason.

  ```
  grep -rnE 'assume\(|admit\(|assume_new|assume_specification|external_body|external_(fn|type|trait)_specification|verifier::external\]|axiom fn' --include=*.rs
  ```
- A FLOOR raise replaces the README's Verus badge in the same PR
  (`references/proofs.md`, "The FLOOR ratchet").
- The FLOOR and RLIMIT files against the base revision: a lowered figure
  has a new `lowered` line; a raised rlimit has an `rlimit-raised` line.
- A new checker over code or data the library produced runs only in
  tests, or ships as a temporary seam with a seams row and a `TODO(#n)`
  toward proving the producer. No allocation, copy
  or pass exists only to make a proof go through where a ghost or spec
  route would.
- The PR body: box numbers name the arms by sha, the run, and all three
  comparisons, with every row.
- `grep -cE '^#+ ABI-'` in the ledger (ports use `##` or `###`
  headings) against the deviations and `known.c` citations: no citation
  names a missing entry, and no `TODO(#n)` names a closed issue.
- Every `unsafe {` block has a `SAFETY:` comment naming the invariant it
  relies on (chainz's and expat-rs's `ci/lint-unsafe.sh`).
