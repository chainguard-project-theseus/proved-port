# Landing: commits, stacks, pushes, reviews

A port that proves and measures well still stalls if landing is
expensive. These rules keep each PR cheap to review, cheap to rebase,
and safe to merge while main moves every 15 minutes.

## One commit per PR

- Every PR branch is one commit, at open and after every push. Fold
  review fixes into it and force-push with a lease.
- Squash with a fresh branch at current main, `git merge --squash
  <head>`, then `git commit`. Never `git reset`, soft or otherwise.
- Follow the repository's signing policy. Where commits are signed
  (for example with gitsign), verify the signer identity on every
  commit before pushing: `git verify-commit HEAD` names the expected
  address, or run `gitsign verify --certificate-identity=...` from the
  main checkout (it cannot open a linked worktree). A wrong-identity
  signature stops the agent; it does not re-sign on its own.
- Follow its sign-off rule too, which is separate from signing:
  challoc's CONTRIBUTING.md asks for a Developer Certificate of Origin
  `Signed-off-by:` line on every commit (`git commit -s`), on top of
  the signature.
- An approved, green PR about to merge is left alone. If it must change
  after approval, say so in a reply.

## Layered stacks

- Land big work as a stack of single-commit PRs in dependency order,
  each proved and gated on its own (pcre3's JIT: encoder and model;
  certificate; equality; mapping and hook; native DFA; fast entry; JS7).
- Keep each child rebased onto its parent's exact head once the parent
  has settled (review answered, CI mostly green, auto-merge armed), so
  the child's PR goes up minutes after the parent merges.
- Check for conflicts cheaply with `git merge-tree`; rebase only when it
  reports one or the parent changed.
- Resolve FLOOR conflicts by re-measuring on the new base, never by
  picking a side.
- Stage children as drafts naming their base PR, to front-load review.

## Pushing

- Write push targets literally: `git push <fork> <branch>` or several
  literal branches in one command. Never a variable, `$(...)`, a loop,
  or `xargs`.
- Use `--force-with-lease=<branch>:<literal sha>`, never `--force`.
- Push from inside the repository: `cd <dir>` as its own command, then
  push. Where `cd` does not carry over (sub-agents, a clone outside the
  working directories), use `git -C <literal path> push ...`. Check the
  remote: a bare `cd` that did not persist once sent a push to the
  wrong repository.
- Push feature branches to a fork; never to the upstream default branch.
- Never use GitHub's update-branch API or button on a repo that
  requires single signed commits: it adds an unsigned merge commit.
  Rebase locally, verify the signature, push with a lease.
- When rebasing, name main's sha explicitly (`git ls-remote ...
  refs/heads/main`); `FETCH_HEAD` is overwritten by other fetches.
- If a combined push of several refs fails server-side, push them
  separately and confirm each PR head moved.

## Labels and CI runs

- Put optional CI labels on the PR before the push or the open that
  should run them. The run is planned from that event, so a label added
  after it is skipped by that run, even seconds later; re-push a new
  sha to pick it up.
- One label requests one leg, and where a repository splits a leg by
  architecture, each architecture's leg has its own label. pcre3's:
  `ci/run:differential` (the amd64 differential only),
  `ci/run:conformance/differential-arm64` (the aarch64 differential,
  the one that runs the native aarch64 JIT), `ci/run:fuzz-diff`,
  `ci/run:fuzz-abi`, `ci/run:fuzz` and `ci/run:consumers`. Label every
  architecture's leg the change needs; a green amd64 differential says
  nothing about aarch64.
- Never rename a CI check key in the same change that relies on it; the
  run is refused. Add the new key, land it, then remove the old one.
- Re-running a failed check reuses the run's original merge ref. If a
  failure came from main, update the branch by rebasing, not rerunning.
- Run `chainctl checks validate` (or the CI system's linter) before any
  PR that touches the check definitions.

## Merging

- Merge only with `--match-head-commit <sha>`, and with auto-merge
  pinned to the head once the PR is one signed commit with no open
  Blocking finding.
- A green PR behind main may merge if `git merge-tree` is clean and the
  commits main gained touch none of the PR's files. A FLOOR change on
  main counts as overlap for every PR.
- After any force-push, re-arm auto-merge pinned to the new head.
- A reviewer bot re-reviews only a new head. After answering a question
  that needs no code change, amend (or rebase) and push a new signed sha.
- Land optimistically: once proved, gated and box-confirmed, push and
  arm auto-merge at once. Proved work sitting as local refs is waste.
- When main breaks, fix it on main first (a fix-forward PR, merged as
  soon as green); bisect afterwards.

## Review threads

- Answer every finding in its thread: fixed (say where, with the sha
  that fixes it) or not applicable (say why).
- Push the fix first, then reply. A reply that names a sha not yet on
  the PR is wrong.
- After replying, confirm no pending review of your own is left
  unsubmitted (PENDING == 0).
- A thread whose last comment is a reviewer's "Verified at <sha>" or
  "Confirmed at <sha>" is not answered, even if that reviewer posts as
  the PR author's account. Only a fix reply naming a sha after the
  reviewer's comments answers it.
- Resolve only threads you started, once they are fixed or answered.
- Read bot notices too (check requesters, scanner output); act on the
  ones that apply and reply with a reason when declining.

## PR body

- Start with the issue line (`Fixes #n`, or `Refs #n` for upstream
  issues and multi-PR work).
- Say what changed, how it is verified (Verus result, FLOOR figures,
  mutants refused, tests added), any known differences (ledger ids), and
  the checks run.
- For perf changes, carry the box numbers with their audit trail (see
  `references/perf.md`, "Reporting in a PR").
- Refresh the body on every push that changes what it describes.
- Keep PR bodies and reply drafts in an untracked directory in the
  clone (pcre3 used `.prbodies/`), never in the commit.
