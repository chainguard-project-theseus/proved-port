# Performance: measured against the C library

The goal is to match the C library on every row, then beat it. Parity on
a geometric mean is not the goal: every row slower than the C library
is a work item until it is root-caused and closed.

## Hypothesis first, prototype second, proof last

1. Write the hypothesis with a predicted number: what moves, by how
   much, and why.
2. Run the cheapest experiment that could falsify it: a hack arm, a
   counter, a profile, an emulated count.
3. Build an unproved prototype (proofs may be stubbed or broken), check
   correctness with the differential tests, and measure it on the box.
4. Only a measured win gets the proving pass. Start the proof while the
   box run is queued only if it is cheap and the win is very likely.
5. After proving, re-measure on the box. A proof can change code
   layout, inlining and spills.

Why: pcre3 fully verified a search-tries change that then regressed a
PHP row by 56%. Verus time on a dead end is the most expensive waste in
the loop.

Correctness fixes are the exception: prove them directly.

## Arms

- Name every port arm after the exact commit it was built from
  (`keep-3ce1923e`), never a reused label. Record the mapping once, in
  the write-up. A run may carry a label (pcre3's run `keep-grpa6`); the
  port arms in it may not. The C library's baseline arms (pcre3's `A`
  and `A0`) are not built from the port, so the write-up identifies
  them by package version, build flags and library md5 instead.
- Build arms from `git archive <sha> | gzip -n` tarballs and record the
  tarball md5 (it must match on the Mac and the box) and the library
  md5.
- md5 every tarball, library, driver and script before and after the
  run; the write-up says "md5s before = after (N files)".
- Use identical-code controls: two shas that build byte-identical
  libraries must measure the same. If they do not, the bench is broken,
  not the code.
- A bisect needs arms from verified shas, an md5 control, and at least
  five interleaved rounds.

## The three comparisons

Every table shown to decision-makers has exactly these, all from one run
with the arms interleaved:

1. the port without its fast path against the C library without its
   fast path (pcre3: arm A0, Wolfi's pcre2 rebuilt with `--disable-jit`,
   so every consumer falls back to the interpreter and no per-consumer
   switch can be missed);
2. the port with its fast path against the C library with its fast path;
3. the port's fast path against the port without it.

Branch before/after numbers (head against main) go in the PR's audit
trail, not in the headline table. Mark any cell from a different run or
not yet measured; never fill it from another run.

Correct: "noJIT vs pcre2 A0: M 45 / 4 / 55 (WIN / PAR / BEHIND,
instructions)" for arm `keep-7ffd6880` (pcre3 main at #1032), with all
three tallies from the one run that measured it.

Wrong: comparing the port's interpreter with the C library's JIT without
saying so (pcre3's early Varnish -30% was mostly that).

## Full tables

- Show every row with every column. Mark rows that did not run; never
  drop rows that did not move.
- Report instructions and cycles. Instructions are stable; cycles are
  what users feel. A row that moves only in cycles at flat instructions
  is usually code placement.
- Write a noise baseline (an A/A run) and judge deltas against it.
- Check output identity first: every arm prints the same output as every
  other and as the C library on every row, or the run is invalid.

## Boxes

- Use bare-metal cloud machines for reportable numbers (pcre3:
  `c4a-highmem-96-metal` for aarch64, `c4-standard-288-metal` for
  x86-64). Shared runners and laptops are for plumbing only.
- Run them on demand: start for a queued arm, run, pull, write up, stop.
  Stop, not delete: disks and setup persist. Log start and stop times
  and the cost of each session.
- Pin the box: SMT off, governor and turbo recorded, NMI watchdog off,
  one pinned CPU (`taskset`), a lock around every phase (`withlock.sh`),
  each run a transient `systemd-run` unit with its own log.
- Make the setup re-runnable after each start; per-boot settings
  (SMT, governor, bind mounts) do not survive a stop.
- Bootstrap the box's run tree from the distro (Wolfi) at the versions
  the C library ships, and log any version no longer served.
- Landing confirmations preempt investigations on the box.

challoc's harness (`bench/cb`) is the more formal version: fresh spot
VMs per run, a pinned bench image, paired ABBA blocks with one fresh
process per run, a sentinel per block, steal-time exclusion, and a
per-row verdict from the mean paired log contrast with a Student-t
interval against the row class's margin, Holm-corrected for the
headline. Use that shape when numbers gate merges.

## Emulated instruction counts

For small deltas (tens of instructions per compile), an emulator that
interprets the static library from a function entry and counts every
retired instruction gives exact, noise-free attribution per function and
per block (the kit's `emu`, an aarch64 interpreter in Go).

- Link the emulated driver with the same LLVM and linker the shipping
  build uses. An emulated count from a different LLVM measures a
  different program.
- Confirm emulated findings on the box before acting; use the emulator
  to explain, the box to decide.

## Attribution

- Prefer precise sampling: Arm SPE (`arm_spe_0`, retired ops, an
  interval with jitter) on aarch64, PEBS precise events on x86-64.
- Interrupt-based `perf record` skids. In pcre3's grpa7 run it put a
  per-pattern-byte stack reload in `engine::compile` into libc; uprobes
  on libc showed identical call counts and sizes in both arms, and SPE
  put the cost back where it was.
- Cross-check attribution with uprobes on the suspect library's entry
  points (counts and size histograms per arm).
- Raise `perf_event_max_sample_rate` and lower
  `perf_cpu_time_max_percent` only for the measuring unit, and restore
  and log them after.

## Layout, inlining and spills

- Pin code alignment in the build config so unrelated size changes do
  not move hot loops (pcre3: `-C llvm-args=-align-all-functions=6
  -C llvm-args=-align-loops=64`, about 10% more `.text`). A comparison
  build that sets `RUSTFLAGS` must carry the same flags; the bench
  script checks the rustc command line and refuses otherwise.
- Small type-layout changes move LLVM decisions. Removing one enum
  variant changed `Item`'s tail padding, moved an `Option` niche, and
  re-split 64-byte copies into a per-byte stack reload; restoring the
  variant made `compile::<u8>` instruction-identical again.
- `#[inline(never)]` and `#[inline(always)]` changes are hypotheses
  until the box confirms them; a Mac or emulated win can vanish on the
  target.
- For "no codegen change" refactors, prove it instead of measuring it:
  challoc's `scripts/codegen-diff.sh` must read `changed 0` on the hot
  set, allowing only relocation, padding and erratum-stub differences.
- In challoc, running the diff is not the whole obligation. The PR body
  of any change to shipped source carries the four codegen cells (x86-64
  and aarch64, each at quantum 16 and quantum 8) from the head's own
  run, and the review gate holds the PR until they are pasted. Cells
  from an earlier head do not count; the run id is optional.
- Pin the hot exports' fast-path cost: challoc's `perf-budget` check
  holds `malloc`, `free` and friends to exact instruction, load, store,
  branch and cache-line counts, with no atomic RMW, fence, call, syscall
  or back edge on the fast path.

## Reporting in a PR

The PR body carries:

- the box and arch, the run id or unit name, and the script md5;
- the arms by sha with tarball and library md5s;
- output identity and md5 stability results;
- the three comparisons as full tallies, plus the affected rows;
- the head against base table for the rows the change targets, with
  bounds;
- an explanation for every row that moved more than noise.

Write it up in this order:

1. A title naming the run, the arms and the start and end times with
   their time zone.
2. An arms table: label, commit, what the arm is, tarball md5, library
   md5 (and whether each library was built for this run or reused).
3. A shape line: unit name, script md5, build, measure and round times,
   and where the result files are.
4. The headline: output identity, md5s before = after, and whether the
   run agrees with the previous one on the arms they share.
5. The three comparisons as tallies.
6. The head-against-base table for the rows the change targets, each
   cell with its half-split noise bound and a verdict (faster, slower,
   flat).
7. A "Reading" section: what moved, the attribution for it, and what
   was not settled.
