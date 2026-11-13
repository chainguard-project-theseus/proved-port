# Starter kit: plan

A plan for moving the reusable tooling of proved C-ABI ports into
`kit/` in this repository, from wolfi-dev/pcre3 (main `5216e03d`),
wolfi-dev/challoc (main `7ef336c4`) and the box and emulator scripts
that are not yet in any repository. Section 5 gives each step's
status.

Each step is a **move**: the tool lands here, the port that had it
consumes it from the kit through its pin (section 4), and the in-tree
copy is deleted in the same step. A step is done only when pcre3 (and
challoc, where it had the tool) builds and runs the tool from the kit
and no copy is left behind. Two copies would mean two rule sets that
drift, which is what the move exists to prevent.

Both pcre3 and challoc align to every kit piece: each switches to it in
the PR that deletes its in-tree copy, never in a separate earlier or
later one, and each kit piece is shown to give the same figures as both
ports' copies before it lands (kit/count/README.md, "Parity", for step
2).

Paths below are relative to each source repository. Tools marked
**local only** live in Matt's working notes (`pcre3-work/`) and in no
repository; step 0 commits them here before anything depends on them.

Coupling scale (the work to make a tool serve more than one port):
**low** (crate names, paths or lists become arguments), **medium** (a
config file replaces hard-coded crate names, paths or ids), **high**
(rewrite around the idea; the code is a reference only).

## 1. Inventory

### Proof tooling

| tool | path | what it does | coupling | to make generic |
|---|---|---|---|---|
| coverage counter | pcre3 `verus/count/` (`main.rs` 3899, `inputs.rs` 1633, `rlimit.rs` 765, `reach.rs` 489 lines) | parses sources with `verus_syn`; classes functions and lines as verified, trusted, discharged, outside; FLOOR with `lowered` lines against `--base-rev`; RLIMIT headroom from Verus's JSON stats with `allow` and `rlimit-raised`; Kani marker/record check; test-only cfg evaluation; shipped-crate selection via `cargo tree` | medium | crate list, `publish = false` rule, feature names (`test-util`) and record directory into a `verus/count.toml`; split the pcre3-only checks (below) behind flags |
| counter, single-crate cut | challoc `verus/count/src/main.rs` (2070 lines) | same counter cut to one crate; reads the generated seam ledger to class discharged functions by `src/FILE:LINE` | medium | merge into one counter with two ledger back ends (per-theorem tables, generated per-file ledger) |
| recursion check | inside pcre3 `verus/count` | fails on call cycles through unverified shipped functions, and verified cycles not named in RN1's table | medium | table path and format into config |
| entry-point reachability | pcre3 `verus/count/src/reach.rs`, `verus/REACH` | every function a C entry point reaches is verified or listed, with `macro_rules!` expanded from their definitions and anything unexpandable failing closed | medium | root selection (exported `extern "C"` functions) is generic; the allowlist path into config |
| compile-input check | pcre3 `verus/count/src/inputs.rs` | fails when code reachable from named roots reads mutable statics, `thread_local!`, env or seeded `HashMap`, unless a table row names it | high | roots and table become config; the reachability walk is generic, the root names are not |
| seam ledger generator | challoc `verus/seams.sh` (1604 lines, shell) | generates `verus/ledger/src/**.rs.md` from source: kind, verbatim contract, CRC hashes, category, proposed tool, status; `--check` fails when stale; per-file overrides | medium | category and tool heuristics into a rules file; consider a Go rewrite (shell at this size is hard to maintain) |
| ledger self-test mutants | challoc `verus/seams-mutants.sh` | sed mutants of `seams.sh` (weakened citation and hash rules) that its self-tests must catch | medium | follows whatever form the ledger generator takes |
| Kani runner | pcre3 `verus/kani/run.sh`; challoc `ci/kani-regular.sh`, `scripts/kani-boolcmp.sh` | per-crate `cargo kani` with job limits, shards in regular format failing on `warning: ignoring`, a lint for bool ordering comparisons in harnesses | low | crate list and feature matrix as arguments |
| Kani record template | pcre3 `verus/seams/kani-*.md` (53 records) | five sections: contract, statement, translation, review, tests (mutation check) | low | ship as a template plus a checker that the quoted contract matches the source |
| unsafe and assume lint | pcre3 `ci/lint-unsafe.sh`; challoc's `no_assume_or_admit_in_verified_code` test | crate roots carry `forbid(unsafe_code)`; `assume_new` only in allowed token seams; no `assume`/`admit` in shipped verified code | low | allowed-seam list into config |
| guard tests | challoc `testutil::assert_only_at` | a trusted item's call sites are pinned; a new site fails | low | lift into a small crate |
| restated-body test | challoc `src/api/restated.rs` | compares exec call names of a body and its verification-only restatement | medium | parameterize the body list |
| inline-asm checks | challoc `verus/isla/`, `verus/emptyasm/` | Isla traces of compiled `asm!` against seam contracts; empty templates emit no instruction, with a `nop` control | high | only for ports with inline assembly; keep as reference |
| loom pins | challoc `tests/loom/tests/pins.rs` | fails when a source line a model copies changes | medium | the pin file format is generic |
| skeletons | pcre3 `verus/README.md`, `THEOREMS.md`, `SEAMS.md`, `TODO.md`, `FLOOR`, `RLIMIT`, `theorems/README.md`; challoc `verus/RESULTS.md` | the document structure: trusted base, one file per theorem with `### Seams`, seam columns, FLOOR and RLIMIT headers explaining the rules | low | strip to templates with placeholders |

### Comparison harnesses

| tool | path | what it does | coupling | to make generic |
|---|---|---|---|---|
| ABI check | pcre3 `ci/abi.sh`, `ci/header-check.sh`, `scripts/link-so.sh`, `crates/pcre3-capi/maps/` | SONAME, version nodes, export set vs map, C smoke tests, header constants vs the distro `-dev` package | medium | library names, map paths, header prefix regex into config |
| glibc ceiling check | challoc `ci/glibc-max.sh` | the `.so` needs no `GLIBC_` version above the floor | low | none |
| conformance ratchet | pcre3 `ci/conformance.sh`, `tests/upstream/` | upstream test data through upstream's driver, per-block failures against a reasoned deviations list, strict both ways, `--update` only removes | high | the ratchet (block keys, reason must cite a ledger id, both-ways strictness) is generic; the driver patching is pcre2-specific |
| differential runner | pcre3 `ci/conformance-diff.sh` | same inputs on port and C library through the unpatched upstream driver, advisory ratchet | high | reference only |
| fuzz-diff driver | pcre3 `fuzz/diff/` (`driver.c`, `known.c`, `known_test.c`, `diff.h`), `ci/fuzz-diff.sh` | one C driver `dlopen`s both libraries `RTLD_LOCAL`; preflight (`dladdr`, distinct objects, version strings); known-divergence predicates citing ledger ids; selftest; base64 reproducers | medium | split into a generic shell (loading, preflight, voices, known-table checking, citation lint, reproducer output) and a per-library comparison file |
| ABI fuzz | pcre3 `ci/fuzz-abi.sh`, `fuzz/abi/` | upstream's OSS-Fuzz harness under ASan/UBSan, guard bytes, tracking allocator, internal-error abort | medium | harness path and patch set as inputs |
| engine fuzz | pcre3 `ci/fuzz.sh`, `fuzz/` | libFuzzer on the safe engine on stable Rust with SanitizerCoverage flags | low | target names |
| consumers | pcre3 `ci/consumers.sh`, `tests/consumers/`; challoc `tests/consumers/` | build dependents from pinned sources against the C `-dev`, run their tests with the port first on the path, expected-failures per consumer, PASS/FAIL/SKIP summary | medium | the runner and `fetch`/`loaded` helpers are generic; each consumer's `run.sh` is not |
| skip accounting | challoc `ci/skip-summary.sh`, `ci/expected-skips/`, `ci/known-failures.sh` | `SKIP` lines against a per-cell manifest; a new skip fails, a vanished one is a notice; known failures scoped by cell | low | none |
| ledger format | pcre3 `docs/abi-changes.md` "Entry format" | the `ABI-nnn` template | low | template plus a lint (ids in order, every citation resolves) |

### Performance tooling

| tool | path | what it does | coupling | to make generic |
|---|---|---|---|---|
| in-repo bench | pcre3 `ci/bench.sh`, `bench/pcre3-bench.c`, `bench/README.md` | one C program against the C `-dev` run four ways (C, C+JIT, port, port+JIT); counts must agree or `MISMATCH`; checks alignment flags on the rustc line | medium | workloads are per library; the four-way runner, result-agreement check and flag check are generic |
| alignment pin | pcre3 `.cargo/config.toml` | `-align-all-functions=6`, `-align-loops=64` | low | a documented snippet; each port keeps its own `.cargo/config.toml` |
| box setup | local only: `pcre3-work/x86box/setup.sh` | metal box: SMT off, governor, NMI watchdog, clock record; Wolfi run and build trees from `wolfi-base` pinned to the C library's versions; re-runnable per boot | medium | package list and paths into a config; split per-boot from install steps |
| box lock and build | local only: `pcre3-work/x86box/box/home/{withlock.sh,build.sh,keep-build.sh,build-a0.sh}` | phase lock; build an arm from a `git archive` tarball into `/opt/lib/<arm>`; build the C library with its JIT disabled | medium | build commands per project |
| run scripts | local only: `pcre3-work/build1103/*/keep-*.sh` (aarch64 box), `pcre3-work/x86box/box/home/{keep-*.sh,rbx2.sh,phpbase.sh}` (x86-64 box) | one `systemd-run` unit per run: md5 before, phases under the lock, interleaved rounds, `perf stat` events, md5 after | high | extract the skeleton (arms by sha, md5 bracket, phase loop, round rotation, event list); workloads stay per project |
| pull scripts | local only: `pcre3-work/build1103/*/pull.sh` | fetch results over `gcloud compute ssh`, md5 consistency, output identity across arms, empty-counter check, arm relabelling | medium | box name, zone and arm map as arguments |
| stats | local only: `pcre3-work/build1103/{spillp1,x86bt2}/cmp.awk` (per-row medians, half-split noise bounds, verdicts), `pcre3-work/build1103/{grpa5,grpa6}/tab.awk` (markdown tables). The noise-baseline, three-way tally and table-join tools (`noise.awk`, `mkreps.sh`, `jointab.sh`, `board.awk`) were lost with a retired session directory; no copy survives | per-row medians, half-split noise bounds, verdicts against a noise baseline, three-way tallies, markdown tables | low | one Go tool replacing the awk family (decided 2026-11-08); the lost tools are rewritten from this description and the write-ups that used them |
| attribution | local only: `pcre3-work/build1103/grpa7/{keep-grpa7spe.sh,spe.awk,u.awk,mcins.awk,sym.awk}` | Arm SPE capture and symbolization, libc uprobes with argument histograms, a memcpy instruction model | medium | probe lists per project; SPE flow generic |
| emulator | local only, three variants: `pcre3-work/tools/regr2/emu/` (Go, 1862 lines) with `pcre3-work/tools/regr2/shapes.sh`; `pcre3-work/f1emu/emu/` (1932 lines, also runs mapped JIT code: cache maintenance, `CTR_EL0`, lane loads); `pcre3-work/sc/emu/` (a working copy with further SIMD changes) | interprets a static aarch64 ELF from an entry, counts retired instructions per function and block between markers; driver linked with the toolchain's `rust-lld` | medium | reconcile the three variants into one first; already library-agnostic; needs an x86-64 decoder for the second arch, a documented driver ABI (`emu_mark`), and a check that the linked LLVM matches the shipping toolchain |
| codegen diff | challoc `scripts/codegen-diff.sh`, `ci/codegen.sh`, `scripts/codegen-fncmp.py`, `ci/codegen-fncmp-test.sh` | machine-code diff of the hot set from the merge base; reloc-only, padding-only, erratum-stub classes | medium | hot-set list into config; rewrite the Python comparer in Go (decided 2026-11-08) |
| fast-path budget | challoc `scripts/perf-budget.sh`, `scripts/perf-budget.rs`, `verus/perf-budget*.txt` | instruction, load, store, branch and cache-line counts per hot export; no atomic RMW, fence, call, syscall, back edge | medium | export list and pinned compiler into config |
| noalias check | challoc `ci/heap-noalias.sh` | rustc IR shows no `noalias` on a `&mut` receiver written through raw pointers | medium | type name as argument |
| paired harness | challoc `bench/cb/` (Python package), `rows.toml`, `suites/`, `cells/` | fresh VMs, pinned image, ABBA blocks, Student-t verdicts, Holm correction, A/A calibration | high | reference design only: it stays in challoc, and the kit's Go tools borrow its statistics (decided 2026-11-08) |

### CI definitions

| tool | path | what it does | coupling | to make generic |
|---|---|---|---|---|
| verus check | pcre3 `.chainguard/ci/verus.yaml`; challoc `verus.yaml` | pinned `verus~` and `kani~` packages, proofs and counter against `CHAINGUARD_BASE_REVISION`, main postsubmit against the previous commit (the kit's template takes `CHAINGUARD_BASE_REVISION` there too, the start of the settled push), arm64 for Z3 speed | low | package pins and `make` target names |
| comparison legs | pcre3 `.chainguard/ci/{conformance,fuzz,consumers}.yaml` | per-arch checks, on-request legs by label | medium | leg names and scripts |
| test matrix | pcre3 `test.yaml`; challoc `test.yaml`, `floor.yaml`, `tlscheck.yaml` | arch by build-variant cells; old-kernel floor cells on GCE | medium | matrix as data |
| lint | pcre3 `lint.yaml`, `ci/shellcheck.sh`; challoc `lint.yaml`, `ci/shellcheck.sh` | fmt, clippy `-D warnings`, source rules, shellcheck pinned by SHA-256, msrv | low | the shellcheck fetcher and its pin move; the check files stay per port |
| dev env | pcre3 `.chainguard/ci/env/dev.yaml`, `.chainguard/develop.yaml`, `scripts/develop-run.sh` | one Wolfi package set shared by CI and dev sessions; upload worktree, run, pull outputs, delete | medium | package list; outputs list |
| Makefile targets | pcre3 `Makefile` (`verus`, `kani`, `remote`, `bench`), challoc `Makefile` | `make verus` = `cargo verus focus -p ... -- --time-expanded --output-json > stats`, then the counter with `--floor`, `--rlimit`, `--base-rev` | low | crate list variable |

## 2. Proposed layout

```
kit/
  README.md                  what the kit is, and the skill that uses it
  reference/                 step 0: the local-only box, stats and emulator
                             scripts as they were, until each moves below
  templates/                 FLOOR, RLIMIT, REACH, MINTS, SEAMS, THEOREMS, the verus
                             README's headings, theorem and Kani record skeletons,
                             ABI-nnn ledger, AGENTS and CONTRIBUTING outlines (step 1)
  count/                     one counter: config-driven crates, both ledger back ends
  verus/
    seams/                   ledger generator and checker (Go)
    lint/                    unsafe/assume lint, guard-test helper crate
  abi/
    check/                   abi.sh, header-check, glibc-max, map lint
    ledger/                  ABI-nnn citation lint (the entry format is
                             templates/docs/abi-changes.md)
  diff/
    fuzz-diff/               generic dlopen driver shell, known-table harness, citation lint
    ratchet/                 per-block deviations ratchet
    consumers/               runner and helpers; example consumer
    skips/                   skip and known-failure accounting
  perf/
    box/                     setup, lock, arm build, md5 bracket, run skeleton, pull
    stats/                   medians, noise bounds, three-way tables (Go)
    emu/                     the aarch64 emulator, with an x86-64 decoder to come
    attribution/             SPE and uprobe flows
    codegen/                 codegen diff, fast-path budget, noalias check
  templates/.chainguard/     check templates (lint, test, verus, fuzz, conformance,
                             consumers), environments, develop, deps, reviewers and
                             the review prompt; templates/.github/ the codeowners
                             sidecar (step 8)
  make/
    proved-port.mk           include-able verus, kani, remote, bench targets
  docs/
    adopting.md              order of adoption, mapped to the skill's phases
```

## 3. What each part needs to become generic

1. **One configuration file** (`verus/count.toml`): crate names, shipped
   crate rule, test-only cfg names and features, ledger style, record
   directory, hot-export list, box names and zones, package pins.
2. **Split the counter's project-specific checks** (RN1 recursion table,
   CD1 compile reads) behind opt-in flags with table paths from config.
3. **Decide one ledger style** or support both: pcre3's per-theorem
   tables are written by hand; challoc's per-file ledger is generated
   and checked. The generated form scales better.
4. **Separate the fuzz-diff shell from the comparison**: loading,
   preflight, voices, reproducer output and known-table discipline are
   generic; what to call and compare is a per-library file.
5. **Extract the box run skeleton**: arms by sha, tarball and library
   md5s, the md5 bracket, round rotation, phase lock, `perf stat` event
   lists per arch, one `systemd-run` unit per run, a sanity gate before
   the full run, start and stop with cost logging.
6. **Replace absolute paths in the box scripts** (each sets a working
   directory on one machine, and some name a GCP project and zone) with
   arguments or config.
7. **Rewrite Python and large shell in Go** (decided 2026-11-08):
   `codegen-fncmp.py`, `seams.sh` and the awk family, so the kit has one
   toolchain besides the Rust counter. challoc's `cb` stays in challoc
   as a reference design.
8. **Add the second architecture to the emulator** (x86-64), and a
   toolchain check that refuses a driver linked with a different LLVM
   than the shipping build.
9. **Template the docs** with placeholders and keep the explanations of
   the rules (FLOOR header, RLIMIT header, ledger format) verbatim, since
   they are the contract agents read.

## 4. How a port pins the kit

TODO: once proved-port (and the ports) are public, publish verus-count as a package (crates.io or a Wolfi apk installed as a pinned environment package) and replace vendoring.

Now: **vendoring** (decided 2026-11-09). While this repository is
internal, a port's check guests cannot reach it, so step 2's install
mechanism is a copy: `kit/sync.sh PORT_ROOT` copies `kit/count` at a
commit of this repository into the port's `verus/count/` and records
the commit in `verus/count/KIT`, and `kit/sync.sh --check PORT_ROOT`,
run by whoever changes the copy, fails on any drift from that commit
(`kit/count/README.md`, "Adopting: vendoring"). The port's checks build
the copy; they need no egress beyond crates.io. A port moves to a newer
kit in a PR that re-runs the sync.

Later, once the TODO above is done: **a released, versioned package**. Each kit tag
(`vX.Y.Z`) builds one package holding the counter binary, the Go
tools, the scripts and the templates. A port installs it in each
check's `environment: packages:` pinned with `~`, the way pcre3 already
pins `verus~0.2026.10.18` and `kani~0.68.0`, and puts the same pin in
`.chainguard/ci/env/dev.yaml` for development sessions. A port moves to
a new kit in its own PR that changes those pins and nothing else, so
every check runs against the new kit before it lands. For `make verus`
on a Mac, the Makefile names the pin once (`KIT_VERSION`) and refuses a
locally installed counter whose `--version` differs.

Rejected:

- **git submodule.** Chainguard Checks' host-side checkout does not
  fetch submodules (mono's Checks migrator flags `actions/checkout`
  `with.submodules` for that reason), so each check would have to fetch
  the kit itself, with github.com egress and credentials for a
  non-public repository. pcre3's verus checks allow only crates.io
  egress today.
- **`go run` of a tagged module.** Needs module-proxy egress and, while
  this repository is internal, GitHub credentials in every check; and it
  covers only the Go tools, not the Rust counter or the shell scripts.

Prerequisite, to settle before the package replaces vendoring: a package built here must be
installable in pcre3's and challoc's check guests. This repository is
internal, so that needs a package repository those guests can read. If
there is none, the fallback is a release tarball fetched and checked by
SHA-256 in a script like pcre3's `ci/shellcheck.sh`, which needs the
releases readable without credentials. Steps 0 and 1 need neither.

## 5. Move order

Every step names what leaves each port and what the port changes. Check
keys never change in a move (a renamed key refuses the run); only the
commands under them do. The exit of every step from 2 on is the same:
the port's checks pass with the kit's tool at the pinned version, the
figures or verdicts match the in-tree tool's on the switching commit
(both run once, numbers pasted in the port's PR), and the in-tree copy
is deleted in that PR.

0. **Commit the local-only tools** to `kit/reference/` as they are,
   with GCP project ids, zones and home paths replaced by variables:
   box setup, lock and build, run and pull scripts, `cmp.awk`,
   `tab.awk`, the grpa7 attribution scripts, and the three emulator
   variants. No port changes. Exit: no row in section 1 says local only.
1. **Templates** (verus README, THEOREMS, SEAMS, TODO, FLOOR and RLIMIT
   headers, theorem and Kani record skeletons, the `ABI-nnn` entry
   format). A port keeps each header's full text as written in its own
   file: the checks, the reviewer and agents read the port's copy, and
   this repository is internal. Each header gains one line naming the
   kit release and commit it came from
   (`# kit: proved-port v{KIT_VERSION} ({KIT_SHA})`, such as
   `proved-port v0.2.0 (<sha>)`; `kit/templates/README.md`), and a
   later lint diffs the port's header against the template at that
   commit, so a copy cannot drift unseen; a kit bump that changes a
   header moves the port's copy and its version line together. What the
   port drops are restatements elsewhere: `AGENTS.md`, `CONTRIBUTING.md`,
   `verus/README.md` and the check files point at the header instead.
   pcre3: `verus/FLOOR`, `verus/RLIMIT`, `verus/REACH` and `verus/MINTS`
   take the version line; `docs/abi-changes.md` "Entry format" and the
   Kani records keep their text and follow the templates. challoc: the
   same for `verus/README.md` and `verus/RESULTS.md`; `verus/FLOOR`
   waits for step 2, because challoc's counter reads
   `combined-functions` and `combined-lines` where the template names
   the kit counter's `checked-*`. The FLOOR template's raise rule is one
   rule for every port: FLOOR and the badge rise in their own PR, to the
   figures the check names, never in the PR that earns them (challoc's,
   expat-rs's and chainz#59's rule; pcre3 moves to it from same-PR).
   Exit: neither port restates a header's rule
   outside the header, and each header names its kit version.
   **Status:** the kit side is done: `kit/templates/` (FLOOR, RLIMIT,
   REACH, MINTS, SEAMS.md, THEOREMS.md, a verus README.md with the
   sections the headers cite and Coverage and Counter in prose, the
   theorem and Kani record skeletons, the `ABI-nnn` ledger, AGENTS.md and
   CONTRIBUTING.md outlines; `kit/templates/README.md` says how to apply
   them). The rest of the verus README's prose and the TODO skeleton are
   not templated yet. pcre3's restatements point at its headers
   (wolfi-dev/pcre3#1073). The version lines take the
   `v{KIT_VERSION} ({KIT_SHA})` form, naming the release tag and the
   commit. The lint that diffs a header against its template is
   `kit/make/headers.sh` (`make verus-headers`), vendored with the
   counter: it reads the templates of the kit commit `verus/count/KIT`
   records and takes a version line naming that commit, or its tag once
   the first release (section 4) gives one.
2. **Counter, unsafe/assume lint and Kani runner.**
   - pcre3: delete `verus/count/` and `verus/kani/run.sh`; add
     `verus/count.toml` (the four crates, the `test-util` feature, the
     record directory, RN1's table, CD1's roots, `verus/REACH`). The
     Makefile's `verus` recipe runs the kit counter in place of
     `cd verus/count && cargo run ...`, with the same `--floor`,
     `--rlimit` and `--base-rev` arguments; `kani` runs the kit runner,
     keeping `KANI_ONLY`, `KANI_EXCLUDE` and `KANI_JOBS`. In
     `.chainguard/ci/verus.yaml` the `counter tests` step leaves the
     `verus` and `verus-main` checks (the kit's own CI tests the
     counter), and the check builds the vendored copy (section 4);
     `make verus` and `make kani` stay the commands. `lint.yaml`'s
     `forbid(unsafe_code) roots` step runs the kit lint in place of
     `ci/lint-unsafe.sh`, which is deleted.
   - challoc: delete `verus/count/`; the Makefile's `verus` recipe (the
     `cd verus/count && cargo run` line) runs the kit counter with the
     generated-ledger back end, and `verus/FLOOR` renames its
     `combined-*` lines to `checked-*` and keeps the template's header
     text, with the kit version line, and the own-PR raise rule (its
     `verus/count.toml` sets `renamed_from = "combined"`, so the PR's
     base copy still
     compares: kit/count/README.md, "Parity"); `verus.yaml` drops its
     `counter tests` step; `kani.yaml`'s `sh ci/kani-regular.sh` and the
     `kani-boolcmp` lint's `scripts/kani-boolcmp.sh` call the kit's,
     and both scripts are deleted.
   - **Status:** the counter is in `kit/count/`, configured by a port's
     `verus/count.toml`, with both back ends (the Kani marker and the
     generated ledger) and pcre3's REACH, CD1, MINTS, recursion and
     rlimit checks behind sections of the configuration. Its output is
     byte-identical to challoc's counter (main `fcde6b2`) and to
     pcre3's (main `158cf87a`) but for the `ok compile inputs` line,
     which also names the roots it resolved, under
     `kit/count/configs/`. It also counts Verus's attribute form, for
     expat-rs. `.chainguard/ci/count.yaml` runs its tests, clippy and
     rustfmt. pcre3's switch PR carries the call-arity and dropped-rlimit
     fixes the kit made to pcre3's code, so the two match on the
     switching commit. The install mechanism is vendoring (section 4):
     a port's `verus/count/` is the copy `kit/sync.sh` makes, so
     "delete `verus/count/`" above means replacing the in-tree counter
     with that copy, and `kit/sync.sh --check` is what shows no other
     copy is left. `count-sync` tests the sync. The Kani runner is
     `kit/make/kani.sh` (`make kani`), vendored with the counter into
     `verus/make/`: pcre3's crate halves, chainz's one harness per
     instance and challoc's weighted shards with a fail pattern, each
     as variables (`make-template` tests it with a stub cargo). Still
     to come in this step: the unsafe/assume lint, and the pcre3 and
     challoc PRs that switch to the vendored kit counter.
3. **Seam ledger generator** (Go rewrite of challoc's `verus/seams.sh`,
   with `verus/seams-mutants.sh` as its tests). challoc: the Makefile's
   `sh verus/seams.sh --check` runs the kit generator; the generated
   `verus/ledger/` must come out byte-identical; both scripts are
   deleted. pcre3 keeps its hand-written per-theorem tables until it
   chooses the generated form.
4. **ABI checks and the deviations ratchet.** pcre3: `ci/abi.sh` and
   `ci/header-check.sh` move (config: library names,
   `crates/pcre3-capi/maps/`, the `PCRE2_`/`REG_` prefixes);
   `test.yaml`'s `C ABI smoke` step, `lint.yaml`'s `header constants vs
   pcre2-dev` step and the Makefile's `test`, `smoke` and `lint`
   recipes call the kit's; `ci/conformance.sh` keeps its pcre2test
   patching and calls the kit ratchet for the per-block comparison;
   `ci/README.md` rows follow. challoc: `ci/glibc-max.sh` moves and its
   callers change the same way.
5. **Comparison harnesses.** pcre3: `fuzz/diff/` splits into the kit's
   generic driver shell and pcre3's comparison file and `known.c`;
   `ci/fuzz-diff.sh` builds the kit driver against them, under the same
   `fuzz-diff` check key. `ci/consumers.sh` and its `fetch`/`loaded`
   helpers move; each `tests/consumers/<name>/run.sh` stays.
   challoc: `ci/skip-summary.sh` and `ci/known-failures.sh` move, and
   `ci/cell.sh` calls the kit's; `ci/expected-skips/` and
   `ci/known-failures.txt` stay.
6. **Box skeleton, pull and stats.** Out of `kit/reference/` into
   `kit/perf/` (stats rewritten in Go). pcre3: the generic parts of
   `ci/bench.sh` (the four-way runner, the result-agreement check, the
   rustc flag check) move, and the Makefile's `bench` recipe calls the
   kit runner with `bench/pcre3-bench.c`; pcre3's box workloads, which
   lived only in the local run scripts, are committed to pcre3 as
   workload files the kit skeleton reads. Exit: an A/A run with the kit
   skeleton on pcre3's workloads is within noise, and the
   `kit/reference/` copies are deleted.
7. **Emulator, codegen diff, fast-path budget, noalias check.**
   Emulator: the three variants reconciled into `kit/perf/emu/` (no port
   had it in tree). challoc: `scripts/codegen-diff.sh`, `ci/codegen.sh`,
   `scripts/codegen-fncmp.py` (rewritten in Go) and
   `ci/codegen-fncmp-test.sh` move; `codegen.yaml`'s `codegen` and
   `codegen-arm64` checks and `lint.yaml`'s `codegen-fncmp` check call
   the kit's, and `python-3.13` leaves `codegen.yaml`'s packages.
   `scripts/perf-budget.sh` and `.rs` move (the tables
   `verus/perf-budget*.txt` stay in challoc), and `perf-budget.yaml`
   calls the kit's; `ci/heap-noalias.sh` moves, with `codegen.yaml`'s
   `heap-noalias` check calling it. Exit includes the same codegen
   verdict as before on a recent shipped-source PR's base and head.
8. **CI templates and the Makefile include**, once the above have
   stable interfaces. pcre3 and challoc `include` the kit's
   `proved-port.mk` for the `verus`, `kani` and `bench` recipes and
   delete their own. The `.chainguard/ci/*.yaml` files stay in each port
   (Checks reads them from the repository); the kit ships templates for
   new ports and a lint that a port's checks call the kit's targets.
   **Status:** `kit/make/proved-port.mk` is the include, vendored by
   `kit/sync.sh` to `verus/make/`, with the `verus` (and its
   `verus-proofs` and `verus-count` halves), `verus-count-test`,
   `verus-headers`, `kani` and `kani-list` targets; `bench` and
   `remote` are still to come. The templates are in
   `kit/templates/.chainguard/` and
   `kit/templates/.github/` (`kit/templates/README.md`, "Repository
   configuration"): the check files in the shape pcre3, challoc, chainz
   and expat-rs share, the default and dev environments, `develop.yaml`,
   the deps agent's `deps.yaml` (which replaces Dependabot), the
   reviewer's `reviewers.yaml` and prompt skeleton, and the codeowners
   sidecar with its rendering. Each, filled with a sample port's values,
   passes `chainctl checks validate`. One setting is not a file: the
   default branch requires `Chainguard Checks (presubmit)` pinned to the
   `chainguard` check app, through chainguard-dev/infra's github-iac
   (chainguard-dev/infra#5389; `kit/templates/README.md`, "The required
   check"). Still to come: the Makefile include and the lint.
