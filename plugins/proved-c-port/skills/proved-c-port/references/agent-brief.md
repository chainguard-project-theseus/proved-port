# Sub-agent brief template

Paste the block below into every sub-agent brief for a port, then fill
the `{PLACEHOLDERS}`. A sub-agent knows only what the brief says, so the
hard rules travel with every brief, every time. Delete a section only
when it cannot apply (a doc-only task has no box section).

Pick the brief's kind first:

- **prototype**: no Verus, proofs may be stubbed; differential tests
  and a box arm decide. Never becomes a PR until proved.
- **prove**: a measured shape, made to verify; FLOOR and rlimit gates.
- **land**: restack, gate, sign, prepare the push; the lead pushes.
- **box**: build arms, run, pull, write up, stop the box.
- **review**: read-only; findings as Blocking or Question only.

## The block

```text
TASK ({KIND}): {ONE-PARAGRAPH GOAL}. Scope is exactly this; anything
else becomes a TODO(#n) or a note in your report.

REPO: {REPO} at {LITERAL MAIN SHA}. Work only in your worktree
{WORKTREE PATH} on branch {BRANCH}. Read AGENTS.md and CONTRIBUTING.md
first; their principles are non-negotiable. If the task seems to need
breaking one, stop and report.

HARD RULES
{PERSONAL OR ORG RULES}
- Never `git reset` (any mode). Never `git add -A` over a tree with
  untracked docs. No scratch git experiments in shared clones.
- No new assume, admit, Tracked::assume_new, assume_specification,
  external_body, external_fn_specification, external_type_specification,
  external_trait_specification, #[verifier::external], axiom fn, or
  weaker contract to make a proof pass. A seam needs a ledger row and, where
  Kani can check it, a harness and a record in the same change.
- FLOOR: never lower a share if verifying more code can close the gap.
  If impossible, report why; do not add a `lowered` line on your own.
- rlimit: keep every function under {RLIMIT TARGET} of its rlimit in
  development, below the CI gate {RLIMIT GATE} (pcre3: 55% and 65%).
  Restore headroom by spinoff_prover, then a lemma split, then a smaller
  context. Never raise an rlimit without saying so in the report.
- Every new test or spec change comes with a mutant that fails it.
- Prototypes: no Verus. Prove only shapes the box has confirmed.
- Run Verus locally only for new proof work, focused (`cargo verus
  focus`, `--verify-only-module`). Let CI verify rebases and
  conflict-only restacks.
- Use one CARGO_TARGET_DIR under {SCRATCH}; delete it when done.

GATE (run before declaring done; paste the summary lines)
Run these {WHERE: locally | through the repo's remote runner, never on
the workstation (chainz: `make <target>` via scripts/develop-run.sh)}.
- {BUILD CMD}            e.g. cargo build --locked --release
- {TEST CMD, scoped}     e.g. cargo test -p {CRATE} -- {FILTER}
- {LINT CMD}             e.g. cargo fmt --check; cargo clippy -D warnings
- {VERUS CMD}            e.g. make verus (full) or cargo verus focus -p {CRATE}
- {COUNTER CMD}          e.g. verus/count --floor ../FLOOR: report figures
- {MUTANT CMD}           the mutants you ran and that each failed

COMMIT AND SIGN
- One commit on {BASE SHA}. Message: title, what and why, then the
  attribution lines the session gives.
- Sign with the repo's configured signing (plain `git commit`). Do not
  override signing environment variables unless told to.
- Verify: `git verify-commit HEAD` must name {EXPECTED SIGNER}. If it
  names anyone else, stop and report; do not re-sign.

PUSH ({ALLOWED | NOT ALLOWED: the lead pushes})
- If allowed: `git -C {LITERAL REPO PATH} push {FORK} {BRANCH}` or, for
  an update, `--force-with-lease={BRANCH}:{LITERAL OLD SHA}`. Literal
  names only: no variables, $(...), loops or xargs. Never --force,
  never --no-verify, never the update-branch API.
- Add CI labels {LABELS} before the push, never after.

BOX ({IF ANY})
- Arms named by sha; record tarball and library md5s; md5 every input
  before and after. Output identity across arms first.
- Start the box only for the run; stop it after the pull; log both
  times and the cost.
- Prefer SPE or precise events for attribution; perf record skids.

REPORT (your final message, in this order)
1. Outcome in one line: done, blocked (on what), or abandoned (why).
2. Commits: branch, sha, base sha, signer identity as verified.
3. Gate: each command and its summary line (Verus errors, FLOOR
   figures, max rlimit share, tests passed, mutants refused).
4. Trust delta: every new or changed seam, with size and reason. "None"
   if none.
5. Numbers: for box runs, the three comparisons as tallies, every row,
   arms by sha.
6. Decisions you made, each with the alternative you rejected.
7. Open questions for the lead.
Absolute paths only.
```

Personal and organisation rules (tooling bans, time zones, prose style,
where PR bodies live, which fork to push to, which machines to use, how
many agents to run at once) do not belong here. Keep them in a small
overlay skill of your own and paste them in place of the exact token
`{PERSONAL OR ORG RULES}`; with no overlay, delete that line.

## Notes on using it

- Name the main sha literally. "Current main" drifts while the agent
  works, and `FETCH_HEAD` is overwritten by other fetches.
- Say whether the agent may push. The usual split: agents prepare
  verified local branches and PR bodies; the lead pushes, comments and
  arms auto-merge.
- Give a box agent the queue order: landing confirmations first,
  investigations after.
- Ask for decisions in the report, not mid-task, unless the agent is
  blocked.
- Check disk before adding agents: build directories fill a laptop
  quickly.
