# Lifecycle: phases and exit criteria

The order matters more than the speed. pcre3 went from an empty
workspace to a full PCRE2 10.49 drop-in in about two days because the
oracle and the comparison harnesses existed before most of the engine
did; it then spent two weeks proving and speeding up what it had, with
the harnesses catching every regression on the way.

## Phase 0: scope and principles

Write these down in `AGENTS.md` and `CONTRIBUTING.md` before the first
PR, as non-negotiable principles an agent must stop and report on rather
than bend:

- The exact upstream release tracked (pcre3: PCRE2 10.49; one at a
  time).
- The support floor (challoc: glibc 2.44+, kernel 5.14+, x86_64 and
  aarch64 with 4, 16 and 64 KiB pages). A change must not raise it.
- Where `unsafe` may live, and the panic policy at the boundary.
- That every deliberate difference is a numbered ledger entry.
- That new executable code comes with its proofs (once phase 4 starts).

Set up the repository from proved-port's `kit/templates/` ("Repository
configuration" in its README): the `.chainguard/ci/` checks and the
`env/default.yaml` and `env/dev.yaml` environments, `.chainguard/develop.yaml`,
`.chainguard/deps.yaml` in place of Dependabot, the Checks reviewer
(`reviewers.yaml` and `review-prompt.md`), and the `.github/codeowners.yaml`
sidecar with its rendered `CODEOWNERS`. The required check is not a
file: add the repository to chainguard-dev/infra's github-iac ruleset
(as chainguard-dev/infra#5389 does).

Only the kit's Verus files stay held to the kit afterwards
(`make verus-headers`; `kit/make/VERUS-FILES` names them): the headers
of `verus/FLOOR`, `RLIMIT`, `REACH`, `MINTS`, `verus/count.toml`,
`.chainguard/ci/verus.yaml`, `miri.yaml` and `reviewers.yaml`, and the
vendored-kit rule of `review-prompt.md`. The other check files
(conformance, consumers, fuzz, test, lint, the environments,
`develop.yaml`, `deps.yaml`) are starting points; reshape them to the
port's checks.

Start with every Verus check name, so the check selection only grows:
Chainguard Checks refuses a change that deselects one of its own checks
(chainz #59 hit it), so a check dropped later cannot land. From the
first commit the port has `verus` and `verus-main`, `kani` and
`kani-main` when it will discharge seams with Kani, and `msrv` in
`.chainguard/ci/test.yaml` (`kit/templates/README.md`, "The required
Verus check set"). Each passes before the first proof: `make verus`
passes at FLOOR 0.0 with the 0.0 badge, and `make kani` passes with no
harnesses only when the Makefile declares `KANI_NONE = yes`; without
the declaration a run with nothing to check fails, and with it a run
that finds a harness fails, so the first harness deletes the line.

Exit: the principles are in the repo, and the CI aggregate check
(`Chainguard Checks (presubmit)` or equivalent) is the only required
check.

## Phase 1: ABI inventory and the C oracle

- Vendor upstream's public headers unchanged. Only a marked "additions"
  block is yours. `header-check.sh` diffs every `PREFIX_*` definition
  against the distro's `-dev` package.
- Write the linker version map by hand from upstream's. `abi.sh` fails
  when the export set and the map differ, checks SONAME and version
  nodes, and loads the library under a real consumer (pcre3: Wolfi's
  `grep`).
- Write down struct layouts the ABI exposes, and which are opaque.
- Survey consumers (pcre3: `docs/research/consumers.md`,
  `api-usage.md`): who calls what, with which options. This decides what
  the ledger's "Affected consumers" field can say.
- Survey upstream's CVEs and ChangeLog by bug class before designing.
  pcre3 picked its theorems from that survey (error offsets bounded,
  ovector sanity, bounded recursion, heap limit honoured).
- Turn the bug classes into numbered security rules, each naming the
  check that enforces it ("Review" where none can), and map every CVE to
  its rule and a pinning test. expat-rs's S1 to S14 (`CONTRIBUTING.md`)
  and `docs/research/cves.md` are the model; theorems then cite the rule
  they discharge.
- Pick the oracle's source. A port that tracks the distro compares with
  its `-dev` package; a port pinned to one upstream release builds the
  oracle from upstream's release tarball, checked by SHA-256, so the
  oracle does not move when the distro does (expat-rs's
  `ci/fuzz-diff.sh`).

Exit: the header, map and smoke checks pass against the C library
itself, and the consumer survey exists.

## Phase 2: comparison harnesses before the port

Build the harnesses first, run them against the C library, and keep
them as CI legs. `references/abi-compat.md` has the detail.

| harness | pcre3 | challoc |
|---|---|---|
| upstream's suite through upstream's driver | `ci/conformance.sh testdata`, a per-block ratchet | vendored glibc and mimalloc suites under `LD_PRELOAD` |
| differential runner | `conformance.sh differential` against Wolfi's libpcre2, plain and `-jit` | `tests/c` programs, `cb` against glibc and jemalloc |
| differential fuzzer | `fuzz/diff/driver.c`, both libraries `dlopen`ed into one driver | `fuzz/fuzz.c`, random operations on threads |
| ABI fuzzer | upstream's OSS-Fuzz harness, ASan and UBSan | tlscheck build's fuzz |
| consumers | grep, git, glib, nginx, php, qt, fish, rizin | Valkey, Ruby, Redis |

Exit: each harness's `--selftest` passes, and each runs on both target
architectures.

## Phase 3: port

- One issue, one worktree, one PR. Issues name the files they own, so
  parallel agents own disjoint files.
- Shared contracts (the AST, the IR, the VM instruction set) change only
  additively, and the PR says so.
- Split crates so the C ABI crate holds all `unsafe` and the engine
  crates forbid it.
- No length pre-pass, no fixed buffers, no unbounded native recursion,
  checked size arithmetic, one budget for every limit (pcre3 design 6).
- Every behaviour change comes with a test, and security-relevant
  behaviour with negative tests: NULL arguments, truncated buffers, bad
  magic, invalid UTF, limits that must trip, overflowing sizes, unknown
  option bits.
- A test that cannot check its claim on a host prints `SKIP <name>
  <reason>`. It never passes silently.
- A port may replace the C a piece at a time over the unedited vendored
  source instead of starting with no C left. chainz builds zlib with
  `Z_PREFIX`, so the C's internal calls go to `z_*` names, and Rust
  supplies those names piece by piece. Check that every export is Rust
  or forwards to the vendored C, and that the forwarding list only
  shrinks.

Exit: the conformance ratchet's deviations are all ledger entries; the
differential and fuzz-diff legs are green or every red is a ledger entry
or an open bug.

## Phase 4: prove in layers

`references/proofs.md` has the practice. The shape of the work:

1. Bring the Verus toolchain into CI as a pinned package first (pcre3
   landed its first theorem before the CI gate existed and paid for it
   in post-merge rlimit breaks).
2. Bootstrap the counter with the first proof, unless the port already
   has it from its setup (a port set up with `kit/sync.sh` and the
   templates has the counter and FLOOR 0.0 in its first commit, and
   `make verus` passes there). A
   port at FLOOR 0.0 with no counter cannot land a proof through a gate that refuses
   unmeasured `verus!` code, and changing the gate to let one through
   reads as a CI bypass. So the first proof PR does it all at once: it
   adds `verus/count.toml` and runs the kit counter
   (`kit/count/README.md`, "Bootstrapping") in the verus check, copies
   the kit's `verus/FLOOR` template, and raises FLOOR from 0.0 to the
   figures the counter names on that PR, with `[badge]` set and the
   README's badge line at those figures (`verus-count --badge .`). Run the counter on the PR's
   base too: every figure there is 0.0, which shows the gate measured
   before the proof moved it. This raise is the one exception to the
   kit's raise rule, which every port follows: FLOOR and the badge rise
   in a PR of their own, to the figures the check names, never in the
   PR that earns them (`references/proofs.md`, "The FLOOR ratchet"). No
   figure can rise before a counter and a proof exist, so the first
   raise lands with them (expat-rs #193).
   A port that already has its own counter adopts the kit's in a PR of
   its own and keeps any check of its own that is stricter (chainz #59
   kept its seam-spelling rejection and per-theorem floor beside it).
   Where the port has a `verus/floor/` directory, name the kit's FLOOR
   file something else, in both the counter's `--floor` flag (the head
   read) and `[floor] path` in `verus/count.toml` (the `--base-rev`
   read): on a case-insensitive file system `verus/FLOOR` and
   `verus/floor/` are the same path (`verus/SHARES` in both, say).
   **Vendor the counter:** the ports' check guests cannot reach this
   repository, so the port carries a copy. From a clone of this
   repository, `kit/sync.sh PORT_ROOT` copies `kit/count` at the clone's
   HEAD to `verus/count/`, and `kit/make` (the make targets, Kani
   runner and header lint) to `verus/make/`, and records the commit in
   `verus/count/KIT`; the verus check builds the counter from there
   (`cargo build --locked --release --manifest-path verus/count/Cargo.toml`)
   and runs it. Never edit either copy: a fix lands in `kit/count` or
   `kit/make` first, and the port re-syncs in its own PR. Before pushing
   a change to `verus/count/` or `verus/make/`, run
   `kit/sync.sh --check PORT_ROOT`, which fails on any drift in either
   from the recorded commit (the port's CI cannot run it), and name the
   commit in the PR body (`kit/count/README.md`, "Adopting:
   vendoring").
3. Prove leaf modules first (UTF decoding, tables, parsers), where vstd
   already has a specification to prove against.
4. Move outward one theorem per PR: each lands `verus/theorems/<ID>.md`
   with a statement, a seams table and mutation checks.
5. Then the interpreter's state model, so later fast paths have
   something to be proved equal to.

pcre3's history shows the layering: T1 (UTF decoding) on day 3, then
parse (PA*), lower (IR*, IC*), analysis (AW*, AP1), compile (EN3, T3),
VM (T2, T5, VS*), and only then the JIT (JIT1) and the direct emit (DE1).

Exit per layer: Verus 0 errors on the PR's base and on main after the
merge, FLOOR at or above main's, every new seam recorded.

## Phase 5: fast paths

A JIT, a direct compile path, or a fast tier is the riskiest code in a
port: it turns attacker input into executable code or skips a pipeline
stage. pcre3's rules:

- Prototype it unproved, measure it on the box, and drop it if it misses
  its predicted number.
- Land it in layers in dependency order: encoder and model, then the
  safety certificate, then the equality theorem, then the mapping, then
  the hook that turns it on.
- The end state is correct by construction: the emitter's postcondition
  proves the certificate (memory safety, bounded steps), the equality to
  the interpreter, and what attacker input can place in the code. The
  runtime checker becomes test-only. pcre3 measured its checker at 47% of
  a native build before dropping it.
- A second architecture shares the proofs through a neutral layer (see
  `references/proofs.md`, "Transport").
- A fast path the C library has always had (zlib's `inffast`, which
  chainz ports as `inflate/fast.rs`) is on from the start, so phase 6
  does not apply; it still needs the equality theorem and a fuzz voice
  without it. The differential against the C library cannot see a
  misreading both share, since the C library runs the same algorithm.

The fuzz voice can land before the proof. chainz #61's shape:

- A cargo feature, off by default, compiles the fast path's call out;
  a second feature plants a mutant for the self-test. No shipped build
  enables either.
- Show the default library is unchanged: build it at the base and at
  the change in the same environment and compare `.text`, `.rodata`,
  `.eh_frame` and `.dynsym` byte for byte. Explain any other section
  that differs (there, panic `Location` line numbers).
- The self-test replays the seeds with the plant against the slow build
  and requires a difference; the slow side carries the plant too, so a
  clean run also shows the slow build never reaches the fast path.
- The C library's own fast and slow paths may disagree on some outputs.
  Before excluding one, show it on two builds of the C library, with and
  without its fast-path call, and that the port's fast build matches the
  C library there; record each exclusion and its evidence in the PR and
  the design doc.

Exit: the equality theorem holds; fuzz-diff runs a voice that compares
the fast path with the slow path; the differential leg runs with the
fast path on each architecture natively.

## Phase 6: default on

For a fast path the C library always had, there is nothing to flip;
the leg that runs without it is the phase 5 fuzz voice.

- Flip the feature in the default build in its own PR, last.
- Keep the build without it, tested and linted, as its own CI leg
  (pcre3's `x86jit` leg tests `--no-default-features --features jit`).
- Confirm on the target architecture's box that the default build
  actually runs native code (an instruction-count sanity gate before the
  full run). A cfg guard that compiles the fast path out on one arch
  passes every host test on the other.

Exit: the default build runs native code on every claimed architecture,
and the full three-way perf table for that architecture is in the PR.
