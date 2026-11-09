# Proofs: Verus and Kani practice

Both pcre3 and challoc verify in place: functions inside `verus! { ... }`
are checked for overflow, out-of-bounds indexing, reachable panics,
termination and their `requires`/`ensures`. Ghost code erases from
ordinary builds, so the shipped library is unchanged.

## Choosing the spec

- Always specify against an independent definition when one exists,
  never against a restatement of the code. pcre3's `utf::decode` is
  specified with vstd's `valid_first_scalar` and `decode_first_scalar`,
  so the proof checks the decoder against a separate definition of
  UTF-8.
- Treat generated tables as the specification of their lookups, and say
  so in the trusted base (pcre3: the UCD tables; their match with
  upstream is checked by `ucdgen --check`, not proved).
- Prefer theorems that remove a whole vulnerability class over theorems
  that rule out one defect. Pick them from upstream's CVE survey.
- Write the spec of the slow path first (an interpreter's state model,
  `vm_run_c`), so every fast path has something to be proved equal to.
- For whole-compile equality, prove an observational statement over a
  key rather than full functional correctness. pcre3's CD1 proves the
  cache compares the whole key and checks mechanically that compile
  reads nothing but its key.

## Writing verifiable Rust

Choose constructs Verus can check. pcre3's `AGENTS.md` keeps a table,
re-checked on each Verus upgrade:

| instead of | write |
|---|---|
| iterator adapters | index `while` loops with `invariant` and `decreases` |
| `dyn Fn` | a generic `F: FnMut` with `call_requires`, or one in-`verus!` trait |
| `thread_local!`, `Cell`, `OnceLock`, `Mutex` | an explicit field, eager construction, or one shared trusted shim |
| `HashMap` with a struct key | integer keys or a verified search over a sorted `Vec` |
| `Display` with `write!` | render into a `String` with verified helpers |

A rewrite made only to get through Verus must behave the same, and the
crate's tests must pass unchanged.

## Refinement layering

Each layer extends the spec first, then the code over it, in its own PR:

1. A layer's theorem file states what is proved, where (file and
   function), and the premises it needs from callers.
2. Its `### Seams` table lists only the seams that layer introduces;
   later layers cite seam ids, never copy rows.
3. A fast-path layer is proved only for the operations the spec covers;
   anything else declines to the slow path, and the decline is tested.

pcre3's JIT1 is the model: JS1 (memory safety) and JS2 (bounded steps)
from a certificate; JS3 (the code answers as its program); JS4 (the
program answers as the table); JS5 (W^X and the call); JS6 (a call
answered natively answers as with no native code, over the
never-JIT-compiled baseline); JS7 (what a pattern can put in the code);
JS8 (no silent fallback, tested). A profile maps code only once every
one of its theorems holds.

DE1 (direct emit equals the full compile) grew a construct at a time,
23 items, each with its proof, and the emit declines at the first unit
it does not cover. Grow a fast path's subset the same way.

## Transport over a neutral layer (multi-arch)

Never port proofs per architecture. pcre3's design:

- A neutral operation enum with a meaning of its own (`op::nstep`) that
  names no architecture's model.
- Per architecture, a lowering into machine instructions and a proved
  refinement per operation (`lemma_a64_step`, `lemma_x64_step`).
- The JIT theorems are stated over the operations and transported to
  each architecture through the run equalities.
- Each refactor PR keeps the existing architecture's bytes identical (a
  golden byte test) or proves the change.
- The instruction model stays one trusted row per machine instruction;
  for x86-64 that includes the claim that the processor decodes the
  bytes as the model says.

The result: x86-64 reached functional parity in about ten layered PRs
with no new trusted seam beyond the instruction model and the mapping.

## Trusted-seam discipline

A seam is anything a verified function relies on without proof:
`assume_specification` contracts for `core`, `external_body` functions,
`external_fn_specification`, `external_type_specification` and
`external_trait_specification` items, `#[verifier::external]` code,
`axiom fn` declarations, `assume` and `admit` in proof code, and
`Tracked::assume_new` ghost-token mints for the C caller's premises.
SKILL.md's audit grep matches every one of these spellings.

- One `external_body` or `assume_specification` around the single
  unverifiable call, with a contract stating only what callers rely on.
- One row per seam, with an id that is never reused, in the owning
  theorem's table (pcre3) or a generated per-file ledger (challoc's
  `verus/seams.sh --write`, checked by `--check` in CI).
- Statuses: trusted, discharged (another tool covers the whole
  contract), discharged on premises (named C or platform premises stay),
  partly checked.
- A Kani discharge needs four things in the same change: the harness,
  a `verus/seams/kani-<id>.md` record (contract quoted, harnesses,
  inputs and bounds, toolchain, mutation check), the row's status, and
  the inert `#[cfg_attr(any(), discharged_by = "kani")]` marker. The
  counter fails a marker with no record.
- Cross-project discharge by reading is allowed but stays trusted:
  pcre3's `c_malloc`/`c_free` seam is discharged by reading challoc's
  theorems at a pinned commit, and the record says nothing re-checks it.
- challoc adds tools per seam kind: Isla traces inline assembly on
  Sail's Arm and x86 models; `make emptyasm` checks empty `asm!`
  templates emit nothing; loom and GenMC check copies of atomics
  protocols (partly checked only, since they run copies); pin tests fail
  when a copied source line changes.
- Guard tests: a trusted item that is sound only at certain call sites
  gets a unit test that scans `src/` and fails on a new call site
  (challoc's `assert_only_at`).

Never add any of the seam spellings above to make a proof pass. Lint
for them (pcre3's `ci/lint-unsafe.sh`,
challoc's `no_assume_or_admit_in_verified_code`).

## The FLOOR ratchet

A source-reading counter (`verus/count`, built on `verus_syn`) classes
every executable function and line as verified, trusted, discharged or
outside `verus!`, leaving out test-only and verifier-only code. It
fails when:

- a shipped share drops below `verus/FLOOR`;
- FLOOR is set below the base revision's copy without a new
  `lowered FIGURE P REASON` line for that figure and value;
- a discharged marker has no record;
- a recursion cycle or a nondeterministic compile read is not in its
  theorem's table (pcre3's RN1 and CD1 checks).

Rules:

- A change that raises a share raises FLOOR in the same PR, to the
  figures the check names. When two PRs both raise it, the second to
  merge re-measures on the new base; never pick a side of the conflict.
- A drop is closed by verifying more code, in the PR or in a small PR
  that lands first. A `lowered` line is the last resort, and its reason
  names exactly which functions or lines became trusted or outside, with
  counts (pcre3's FLOOR has examples).
- Code built only for the verifier still counts if it is trusted: a
  shipped proof can rest on it.

## rlimit headroom

Z3's resource count is deterministic, so a proof near its limit passes
every run of its own PR and fails on main once a neighbour in the same
module adds a little. Each PR passes on its own base; the merge fails.

- Gate in CI: every function's share of its rlimit stays under
  `max-share` (pcre3: 65%), read from Verus's `--output-json` statistics.
  The allowlist only shrinks; a raised `#[verifier::rlimit(N)]` needs an
  `rlimit-raised NAME N REASON` line.
- Work to 55%: when a function crosses 55% in development, restore
  headroom before pushing.
- Restore in this order: `#[verifier::spinoff_prover]`; split the proof
  into lemmas, each on its own prover; a smaller context (fewer facts in
  scope, a lemma in place of an inline proof). Raise the rlimit last, if
  at all. Never weaken a contract or add an `assume`.
- Prefer `requires`/`ensures` pairs over implications in builder
  contracts; pcre3 measured implications at 2 to 4 times the cost.
- Keep a large derived view out of commonly named types: pcre3's CD1
  found a `View` on `CompileOptions` tripled a neighbour's rlimit use.

## Mutants: proving that tests and proofs bite

Every specification change and every safety check gets a mutation check
in the same PR:

- Proof mutants: break the code the way the spec is meant to catch and
  confirm Verus rejects it.
- Checker mutants: mutate emitted code (drop a bound check, invert a
  guard, replace an advancing add with a move) and confirm the test-only
  checker refuses every mutant of each kind, with counts in the theorem
  file.
- Kani mutants: a wrong body, or a wrong expected value in the harness,
  must report `VERIFICATION:- FAILED`.
- Test mutants: a new test must fail on the base or on a deliberate
  mutation (`is_default` inverted, an anchoring check removed).

## Test doubles and duplicated decisions

A test that exercises a path production does not take certifies
something untrue, just as a double that diverges does (mono's
`sentinel/docs/testing-doubles.md`). For ports:

- When a decision exists on a fast and a slow path, test the one that
  runs, or assert the two agree on a shared table.
- When a restated body stands in for executable code in a proof
  (challoc's verification-only theorems), add a test that compares the
  restatement's calls with the body's (`src/api/restated.rs`).
- When a fast path is compiled only on some architectures, a test must
  fail when no code was made, on each architecture's CI leg.

## What to prove versus what to test

| prove | test |
|---|---|
| memory safety of anything that maps or runs generated code | that refused code falls back (JS8) |
| equality of fast path and slow path | equality with the C library (the oracle is outside Verus) |
| bounds: offsets, ovector values, work, recursion depth, heap limit | consumer behaviour, error text |
| allocator invariants: disjoint blocks, metadata integrity, valid frees never abort | weak-memory behaviour on hardware (stress tests) |
| what attacker input can place in executable bytes | that generated tables match upstream's |

Run Verus locally only while writing new proofs, and then focused
(`cargo verus focus`, `--verify-only-module`). Let CI's leg verify
rebases, conflict-only restacks, and doc or test-only changes.
