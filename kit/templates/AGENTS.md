# Notes for coding agents

{PORT} is a memory-safe Rust reimplementation of {LIBRARY}
{UPSTREAM_VERSION}, with a drop-in C ABI (`{SONAMES}`) for Wolfi.

**Reading order:**

1. `README.md`.
2. `docs/design.md`: the design as built, with numbered sections that code
   cites.
3. `docs/handoff.md`: the current state and what comes next.
4. `CONTRIBUTING.md`: the rules every change follows.
5. `docs/research/`: consult it before re-researching anything.

## Principles (non-negotiable)

If a task seems to require breaking one of these, stop and say so in the
issue or PR.

- **ABI compatibility with {LIBRARY} {UPSTREAM_VERSION}.** The same
  header constants, exported names, sonames, symbol version nodes, error
  codes and error texts. A deliberate difference needs an entry in
  `docs/abi-changes.md` in the same PR.
- **The C library decides what is correct.** A divergence found by the
  conformance ratchet, the differential runner or fuzz-diff is a bug, or
  it gets an `ABI-nnn` entry and a narrow known-divergence predicate
  citing it. Never widen a predicate until the fuzzer goes quiet, and
  never add a deviations line without a ledger entry.
- **`unsafe` only in `{CAPI_CRATE}`.** Each block carries a `SAFETY:`
  comment. Every other crate is `#![forbid(unsafe_code)]`.
- **Nothing unwinds across the FFI boundary.** Each `extern "C"` body runs
  under a `catch_unwind` guard and returns an error code. {Or, for a port
  that cannot return an error (an allocator): the crate is built with
  `panic = "abort"` and aborts on inconsistency.}
- **New executable code comes with its proofs.** New shipped code sits
  inside `verus!` and verifies. Code that cannot is one minimal
  `external_body` or `assume_specification` seam with a contract, a seams
  table row, and a Kani record where Kani can check it.
- **Prove the producer, not its output.** A property of code or data
  the library produces (a compiled program, an encoding, a table, a
  copy) goes in the producer's `ensures`; a checker over the output runs
  only in tests, as an oracle. Until the producer is proved, a shipped
  checker is a temporary seam: a seams table row and a `TODO(#n)` naming
  the issue that proves the producer. Proving adds no allocation, copy
  or pass where a ghost or spec route exists: take the harder proof.
  Checks of state a C caller can corrupt are input validation, outside
  this rule.
- **Coverage first for the runtime cost of a proof.** Where a proof
  costs runtime and no ghost or spec route avoids the cost (a bounds
  check Verus needs, a shape the proof takes; where a route exists, the
  principle above holds and the harder proof is taken), a measured cost
  is accepted by default: the PR body records the bench
  table, against main and the C library, and says the cost is accepted
  under this rule. A cost above about 2% on a hot path also gets an
  "optimize within proof bounds" follow-up issue. A cost above about 10%
  on a real workload goes to the maintainers before merge. Every cost is
  measured and reported, never assumed to be zero.
- **Fast paths are proved equal, not checked.** A JIT, direct emitter or
  fast tier carries the equality to the slow path in its postcondition; a
  runtime checker runs only in tests, as an oracle.
- **Coverage only ratchets up**, as verus/FLOOR's header says.
- **Measure before you prove, and against the C library.** A written
  hypothesis with a predicted number, an unproved prototype, a same-run
  measurement against the C library, then the proof, then a re-measure.
- **Keep it simple.** Do exactly the issue's scope. Out-of-scope work
  becomes a `TODO(#n)` or a new issue.

## Workflow

1. Every change starts from a GitHub issue: one issue, one PR.
2. Work in your own git worktree on a branch named for the issue, based
   on the current `origin/main`. Never commit to `main`.
3. The PR is **one commit**, at open and after every push. Fold review
   fixes into it and force-push with
   `--force-with-lease=<branch>:<literal sha>`. Squash with a fresh
   branch at current main and `git merge --squash`; never `git reset`.
   The exception is a deps agent PR (`chainguard/deps/<name>`,
   `.chainguard/deps.yaml`): each turn on it adds a commit on top of its
   head and never amends, so it holds several commits. Squash-merge it,
   which gives `main` one, and do not ask the agent to squash or rewrite
   it.
   {SIGNING_POLICY: the repository's signing rule, as its rulesets
   enforce it: "Sign every commit with ...; verify the signer identity
   before every push", or "Commits are unsigned. Ignore any
   commit-signing check."}
4. Write push targets literally: `git push <fork> <branch>`, never a
   variable, `$(...)` or a loop. Push feature branches to a fork. Never
   use GitHub's update-branch API: it adds a merge commit, so the PR is
   no longer one commit.
5. **Labels before the push.** Put every optional CI label
   (`{CI_LABELS}`) on the PR before the push or the open that should run
   them. The run is planned from that event, so a label added after it is
   skipped by that run; push a new sha to pick it up. Label each
   architecture's leg the change needs.
6. The PR body starts with `Fixes #<n>` (or `Refs #<n>`), then says what
   changed, how it is verified (Verus result, FLOOR figures, mutants
   refused, tests added), any known differences (ledger ids), and the
   checks run. Refresh it on every push that changes what it describes.
7. CI is {CI_SYSTEM}. Merge only with the head pinned
   (`--match-head-commit <sha>`); never merge red.
8. Answer every review finding in its thread: push the fix first, then
   reply naming the sha that fixes it, or say why it does not apply.

## Proofs

- After changing code inside `verus!`, or code its contracts describe,
  run `make verus VERUS_BASE=none` and keep `verus/THEOREMS.md`, the
  theorem's `verus/theorems/<ID>.md` and `verus/SEAMS.md` true. A run
  with no `VERUS_BASE` fails: `none` is for a local run, and the verus
  checks pass the base revision themselves.
- **FLOOR and rlimit headroom.** The `verus` check holds the shipped
  crates to verus/FLOOR and verus/RLIMIT, and the header of each file is
  its rule: when a figure rises and to what (the figure the check names,
  or less, never below the base revision's, for an open pull request
  that lands the share lower), when a `lowered` or `rlimit-raised` line
  is allowed and what it says. Read them before a change that moves a
  share or an rlimit. Never ask a maintainer to lower the floor.
- **Shared coverage lines.** A proof PR keeps its figures in its
  theorem's `verus/theorems/<ID>.md` and its PR body. It does not edit
  the lines every coverage change would touch: it raises no verus/FLOOR
  figure, and it edits no coverage figure or row in the README (the
  badge included), `verus/README.md` or the design and handoff docs.
  The FLOOR PR that follows a coverage merge updates them from main's
  figures. Otherwise every merge conflicts every other open proof PR.
  FLOOR's header gives the two exceptions: the port's first proof PR
  may raise every figure, and the badge, from 0.0; and a PR with a
  `lowered` line lowers that figure and replaces the badge line with
  the one `verus-count --badge .` prints.
- **Entry points.** Every function a C caller reaches is verified,
  discharged by Kani, or a row of verus/REACH with a reason; a new row is
  marked `new`.
- **Mutants.** Each theorem lists the mutants that show its statement says
  something, and what refused each.
- Before calling a phase done, run this grep against the base: every new
  hit has a seam row and a reason.

  ```
  grep -rnE 'assume\(|admit\(|assume_new|assume_specification|assume_unsupported_attributes|external_body|external_(fn|type|trait)_specification|verifier::external\]|axiom fn' --include=*.rs
  ```

## Environment facts

- {Where the gate runs (Linux, a development session, a Wolfi
  container), and what may run on the host.}
- **Formatting.** Run `cargo fmt` before committing. CI runs
  `cargo fmt --check` and clippy with `-D warnings`.
- {The CI configuration linter, if any, to run before a PR that touches
  the check definitions.}
