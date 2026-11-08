# Pitfalls: dated lessons, one rule each

Each entry is a rule, then the incident that taught it. Dates are
Pacific, and are when it happened in pcre3 or challoc; `#n` is a pull
request in that repository, dated by its GitHub record.

## Proofs and coverage

- **Gate rlimit headroom per function, not only proof success.**
  2026-10-28: pcre3 #551 passed its own checks, and once merged, main's
  Verus run failed with `lemma_read_live` over its rlimit (#592 gave it
  its own prover). The same day #583 (VS1), which had passed its own
  run, tipped `Machine::run`'s loop over its rlimit once main held #589
  (LE1). #620 then added the per-function headroom gate.
- **Resolve FLOOR conflicts by re-measuring, never by picking a side.**
  2026-10-29: pcre3 #689 was approved minutes after it opened, then
  updated from main four times, with a fresh presubmit each time, before
  it merged, because FLOOR raises kept landing on main.
- **Close a coverage shortfall by verifying more, not by lowering.**
  2026-11-04: the fast compile tier (pcre3 #972) fell just under the
  lines floor; a small PR verifying five functions (15 shipped lines,
  #987) landed first instead.
- **Count verifier-only trusted code as trusted.** 2026-11-07: the
  counter left out a trusted function built only under
  `verus_keep_ghost`, though shipped proofs rested on it (pcre3 #1042).
- **Do not narrow a spec's ensures without saying who consumed it.**
  2026-11-07: pcre3 #1035's anchors layer weakened `lead_of`'s ensures;
  it shipped only after confirming nothing used the stronger form.

## Fast paths and architectures

- **Gate on the CI leg that runs the target architecture, not on host
  tests.** 2026-11-07: on pcre3 #1045, a `cfg` guard in `bt::make`
  returned `None` off aarch64, so the x86-64 backtracking JIT never ran
  native code. Every aarch64 host test passed; the box run below caught
  it, and the fix added an x86-only test that code is made.
- **Run an instruction-count sanity gate before a full box run.**
  2026-11-07: the first x86 BT box run (keep-x86bt, 17:02 to 17:47) was
  stopped after 45 minutes when the fast-path arm measured equal to the
  slow-path arm; a one-call gate would have caught it in seconds.
- **Make a test fail when no native code was made.** Same incident: the
  tests that compared native and interpreted answers passed because
  both sides were the interpreter.
- **Keep the runtime checker as a test oracle, not the safety argument.**
  2026-10-31: the checker cost 47% of a one-pass pattern's native build
  and had never refused a shipped program; pcre3 #818 mapped the
  anchored tier's code on its proof instead.

## Performance

- **Match the emulator's LLVM to the shipping toolchain.** An emulated
  instruction count from a different LLVM version measures different
  code, and its deltas do not transfer. 2026-11-07: rustup's 1.99.1
  (LLVM 23) showed pcre3 #1046 at lit -1 / lit2 +12 a compile; the box's
  Wolfi 1.99.1 (system LLVM 22) measured +97 / +177 (run keep-grpa4).
  Rebuilding with rustup's 1.98.1, also LLVM 22, reproduced the
  grouped-shape cost.
- **Use Arm SPE or precise events for attribution.** 2026-11-08:
  `perf record` skid put a register spill in `engine::compile` into
  libc; uprobes and SPE showed libc identical in both arms.
- **Confirm `inline(never)` and `inline(always)` fixes on the box.**
  2026-11-07: a local revision of pcre3 #1047, never pushed and measured
  only on the box (run keep-grpa6), added
  `#[inline(never)]` on `vm::starts::wt_check` and `unconsumed_init`.
  It restored `finish_tail`'s size, but that run measured it
  worse than the previous head on every row up to K=64 (lit +62 against
  +39 over the base, same run). The revision was dropped, and #1047
  merged from its previous head, whose `#[inline(never)]` is on
  `linear::set_program` and `linear::prove_required` and which the box
  had already measured (run keep-grpa5).
- **Treat type-layout changes as perf changes.** 2026-11-07: pcre3
  #1028 removed a field it had made dead (`Item::digit`), which changed
  `Item`'s tail padding, moved an `Option` niche
  and cost +35 to +79 instructions per compile in a same-run box
  comparison; the Mac analysis had called it free. #1034 restored the
  field.
- **Name arms by sha and md5-check them.** 2026-10-28: an arm label
  pointed at the wrong commit and a "bisect" blamed pcre3 #574, which
  only touches tests; its endpoints built byte-identical libraries.
- **Compare like with like.** 2026-10-28: tables compared the port's
  interpreter with the C library's JIT; the baseline became the C
  library rebuilt with its JIT compiled out.
- **Do not prove before measuring.** 2026-10-31: a fully verified search
  change regressed a PHP row by 56%.
- **Read the box's clock in its timezone.** 2026-11-06: ledger entries
  stamped from a UTC uptime clock were read as Pacific.
- **Make the emulator run the JIT before trusting it on a JIT path.**
  2026-11-08: pcre3's instruction emulator could not execute mapped
  native code, so every earlier emulated count on the fast find route
  measured the interpreter fallback; teaching it to run code above the
  text segment (cache maintenance, lane loads) came first.
- **Box-confirm any change to a per-match hot path before pushing.**
  2026-11-08: pcre3's F1 routed the fast find through a verified call;
  the emulator predicted +1 instruction a match, and the change waited
  on a same-run box arm before it was published.

## CI and landing

- **Put CI labels on the PR before the push or the open that should run
  them.** The run is planned from the push or open event, so a label
  added after that event is not in its plan, even when it lands before
  the run's checks appear. 2026-11-07: pcre3 #1046 was opened with its
  `ci/run:*` labels added eight seconds later, five seconds before its
  check run was created, and the requested legs still read "skipped:
  nothing to run"; #1047's labels came one second after its run was
  created, with the same result. Each needed a new push.
- **Never rename a CI check key in place.** The run is refused as a
  change that deselects its own check; keep the key and change its
  steps, or add the new key first and remove the old one later.
  2026-11-07: pcre3 #1048 renamed `x86jit` to `nox86jit` and its
  presubmit reported "Run creation failed" with no checks run.
- **Never use the update-branch API on a single-signed-commit repo.**
  2026-11-07: it added an unsigned merge commit to pcre3 #1030; the fix
  was a local signed rebase and a lease push.
- **Push with `git -C <literal path>` where `cd` does not persist.**
  2026-11-07: while opening pcre3 #1046, a `cd` into the clone did not
  carry over, and the push ran against a different repository's remote,
  which rejected it.
- **Treat a reviewer's "Verified at" as an open thread.** 2026-11-07: a
  landing sweep hid two open threads on pcre3 #1042 for about six hours
  because their last comment came from the author's account, posted by
  a review agent.
- **Drop slow optional legs before the landing push.** 2026-10-30:
  `fuzz-diff` and `differential` labels made every run slow, so PRs lost
  merge races; they came off pcre3 #698 once they had passed. Request
  them for evidence, drop them once passed.
- **Make reachability gates expand macros from their definitions.**
  2026-11-08: review of pcre3 #1052, the gate that every function a C
  entry point reaches is verified or allowlisted, found it did not
  expand `width::fn_ptr!`, so the generated callback conversions, and
  any helper they called, escaped it. Before it merged, the counter
  learned to expand the real `macro_rules!` and to fail closed on
  anything it cannot expand.
- **Keep agent state outside the session's temporary directory.**
  2026-11-08: a session restart retired its scratchpad and killed three
  agents mid-change; their uncommitted edits survived only because the
  worktrees were git worktrees. Keep worktrees, progress notes and box
  scripts in durable paths, and have every agent write a progress file a
  successor can resume from.
- **Two PRs can each pass and their merge fail.** 2026-11-05: the fast
  compile tier (pcre3 #972) and a dense-gate change crossed; a test
  unwrapped a prefilter the fast tier never builds. #998 fixed it
  forward on main within the hour. Fix forward on main first.
- **Retry split pushes after a server-side ref error.** 2026-11-05: a
  combined push of pcre3 #990 and #995 hit `commit_refs_rules`;
  separate pushes went through, and #990's head had silently stayed on
  the old sha until it was pushed again.
- **Verify the signer after every commit.** 2026-11-02: in pcre3 a
  signing cache issued certificates for the wrong account, and the
  commits signed from it carried that identity: a restack of #894, then
  a re-sign of #911. A signer check caught each before the push, and
  both were signed again.

## Allocators (challoc)

- **Verify every build variant in CI, not only the default.**
  2026-10-27: no Verus leg ran the 8-byte quantum build, and
  `Huge::take` exceeded its rlimit only there, on x86-64; challoc #230
  deferred a parameter over it and #240 cut the solver cost. #264
  (2026-10-28) added the quantum-8 build to the Verus checks.
- **Gate checks on the repository they run in after a rename.**
  2026-10-28: challoc moved from chainguard-dev to wolfi-dev, and the
  kernel-floor checks, then gated on the pull request's head repository
  being `chainguard-dev/challoc`, skipped on every pull request until
  #278 changed the name the same day. (Since #366, 2026-10-31, they run
  only on pushes to main.)
- **Report the failing test name into the CI summary.** 2026-10-29: a
  floor cell failed on main after challoc #306 merged, and `ci/floor.sh`
  copies only each cell's `floor:` summary lines into the log, so the
  failing cgroup test's name never reached the Checks log; it had to be
  read from the cell's results bundle.
