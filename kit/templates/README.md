# Templates

Skeletons a new port copies: the proof ledgers' file headers, the seams
and theorem documents, the Kani record, the known-differences ledger, and
outlines of `AGENTS.md` and `CONTRIBUTING.md`. They come from
wolfi-dev/pcre3 at `5216e03d` and wolfi-dev/challoc at `7ef336c4` with
the project names taken out.
The rule text in each header (how FLOOR ratchets, how RLIMIT headroom
and REACH rows work) is the contract the counter enforces and agents
read, so keep it as written in the port's own file; fill in the
placeholders and the rows.

A port takes the kit as the theseus-proved-port package, pinned
`theseus-proved-port~{KIT_VERSION}` (v0.3.0 or later). Its copies of
these files carry no `# kit:` line: the release is the package pin (the
Checks environments, `.chainguard/ci/env/dev.yaml` and the Makefile's
`PROVED_PORT_VERSION`). `make verus-headers`
(`/usr/share/proved-port/make/headers.sh`) holds each file's header,
its whole leading `#` comment, to the installed template's, so the
port's own comments go after a blank line below it.

`AGENTS.md` and `CONTRIBUTING.md` point at those headers rather than
restating them.

`make verus-headers` holds only the kit's Verus files, which
`kit/make/VERUS-FILES` names (the package installs that list beside
`headers.sh`, with every template under its own name in
`/usr/share/proved-port/templates/`, and holds only the listed ones):
the headers of `verus/FLOOR`, `RLIMIT`,
`REACH`, `MINTS` and `verus/count.toml`, of `.chainguard/ci/verus.yaml`,
`.chainguard/ci/miri.yaml` and `.chainguard/reviewers.yaml`, and the
kit rule (the item that starts `- The vendored kit`) of
`.chainguard/review-prompt.md`, which carries no kit line. A port has
no copy of the counter or the make targets. Every other template here is a starting point,
not held: the port reshapes it to its own checks, and its copy needs no
kit line ("Repository configuration" below).

The placeholders are `{PORT}` and its kin throughout. A port started
from rust-port-template, whose files say `__PROJECT__`, writes
`{PORT}`'s value there too: the kit's spelling is the one these
templates and their checks use.

The FLOOR raise rule is the same in every port: FLOOR and the README
badge rise in a pull request of their own, to the figures the check
names, never in the pull request that earns them, so that pull requests
landing in parallel do not conflict there. The one exception is the
port's first proof pull request, which may raise FLOOR from 0.0 in the
same pull request, whether it brings the counter or the counter landed
earlier with FLOOR at 0.0 (FLOOR's header; `kit/count/README.md`,
"Bootstrapping a port").

| template | copy to | what it holds |
|---|---|---|
| `verus/FLOOR` | `verus/FLOOR` | the coverage ratchet: four figures, `lowered` lines |
| `verus/count.toml` | `verus/count.toml` | the counter's configuration: `[crates]`, with the `cfg` atoms the proof run settles (`verus_sets`, `verus_never_sets`), `[floor]` and `[badge]`; add the sections of the checks the port runs (kit/count/README.md, "Configuration") |
| `PORT-README.md` | `README.md` | the port's README: its title, the Verus badge at FLOOR's figures (0.0 until the first proof), and outline sections |
| `verus/RLIMIT` | `verus/RLIMIT` | rlimit headroom: `max-share`, `allow`, `rlimit-raised` |
| `verus/REACH` | `verus/REACH` | functions a C caller reaches that Verus does not check |
| `verus/MINTS` | `verus/MINTS` | the C caller's token kinds and mints, for a port that mints tokens through one trusted function |
| `verus/README.md` | `verus/README.md` | the sections the headers cite: Coverage (the run commands, the `shipped` row, FLOOR and the badge, which links to `verus/README.md#coverage`) and Counter in prose, shaped on pcre3, expat-rs and chainz#59; the rest as headings |
| `verus/SEAMS.md` | `verus/SEAMS.md` | the core-library seams table and its columns, for hand-written seams tables. A port on the counter's generated-ledger back end (`[ledger]`) keeps a SEAMS.md that defines the ledger's kinds, categories, tools and statuses, with a "Status" section, which verus/FLOOR's header cites (challoc's; kit step 3 templates it) |
| `verus/THEOREMS.md` | `verus/THEOREMS.md` | the trusted base and the theorem index |
| `verus/theorems/TEMPLATE.md` | `verus/theorems/<ID>.md` | one theorem: statement, mutants, checks, what it does not say, seams |
| `verus/seams/kani-TEMPLATE.md` | `verus/seams/kani-<seam id>.md` | one Kani record: contract, statement, translation, review, tests |
| `docs/abi-changes.md` | `docs/abi-changes.md` | the `ABI-nnn` known-differences ledger |
| `AGENTS.md` | `AGENTS.md` | outline: non-negotiables, workflow, proofs |
| `CONTRIBUTING.md` | `CONTRIBUTING.md` | outline: hard rules, tests, FLOOR and rlimit, sending a change |

## Applying them

1. Copy each file you need to its path in the port. Copy
   `theorems/TEMPLATE.md` and `seams/kani-TEMPLATE.md` to their real
   names; never leave the `TEMPLATE` files in the port, since the
   counter reads every `verus/theorems/*.md` as a seams table and every
   `verus/seams/kani-*.md` as a record.
2. Delete each `# kit: proved-port v{KIT_VERSION} ({KIT_SHA})` line a
   template still has, held or a starting point: a port's files carry none.
   Then replace every `{PLACEHOLDER}`. A placeholder in capitals is a value
   (`{PORT}`, `{LIBRARY}`, `{UPSTREAM_VERSION}`, `{CAPI_CRATE}`,
   `{VERUS_RELEASE}`, `{KANI_RELEASE}`, `{RUST_STREAM}`, `{MSRV}`,
   `{KIT_VERSION}`);
   one with a colon (`{SIGNING_POLICY: ...}`) is a value its text
   describes, and where it lists choices split by `|`, keep one; one in
   a sentence (`{How to run the gate ...}`) is prose to write, or to
   delete with its item when it does not apply. When you are done, this
   finds none:

   ```sh
   grep -rnE '\{[A-Z][A-Z_]*[}:]|\{[A-Z][a-z]' README.md AGENTS.md CONTRIBUTING.md docs/abi-changes.md verus/
   ```
3. Start FLOOR at the counter's first run: record the figures its
   `shipped` row names, rounded down, as the check prints them. A port
   with no proofs yet starts at `0.0`, and its first proof pull request
   may raise it in the same pull request. From then on it rises only in
   pull requests of its own (above).
   README.md's badge line gives FLOOR's `functions` and `lines`, and with
   `[badge]` the check fails when it does not, so each change that moves
   those figures replaces the line with what `verus-count --badge .`
   prints (kit/count/README.md, "The badge").
4. Delete the sections that do not apply: `verus/MINTS` for a port whose
   C API makes no ghost tokens on trust, `verus/REACH`'s macro-root item
   for a port without an exports macro, the unwinding rule's allocator
   variant for a port that returns error codes.
5. Keep the files true from then on: a change that adds a theorem adds
   its file and index row, a seam its row, a Kani-discharged seam its
   record.

The coverage counter that reads FLOOR, RLIMIT, REACH, MINTS, the seams
tables and the Kani records is kit step 2 (`kit/PLAN.md`).

## Repository configuration

The Chainguard Checks files a port's repository needs: its checks, its
development and deps-agent environments, its reviewer and its code
owners. Each copies to the same path in the port (Checks reads them only
at the repository root, so the copies here run nothing). They come from
wolfi-dev/pcre3 main at `99a29cd9` (#1078 and #1079 merged),
wolfi-dev/challoc main at `6268155e` (#688 merged) with #689
(`2e452c18`), wolfi-dev/chainz at `1e79acd2` with #63 (`6cb6fb26`) and
#64 (`8a32ff7d`), and wolfi-dev/expat-rs main at `0f99b297` (#201
merged) with #202 (`98588c1f`).

| template | what it holds | shaped on |
|---|---|---|
| `.chainguard/ci/lint.yaml` | rustfmt, clippy, the source rules (`ci/lint-unsafe.sh`, `ci/header-check.sh`), shellcheck from the packaged `shellcheck~` | pcre3, expat-rs; shellcheck as chainz#63 |
| `.chainguard/ci/test.yaml` | cargo test and `ci/abi.sh` on amd64 and arm64; msrv on the `rust-version` toolchain | pcre3, expat-rs, chainz#63 |
| `.chainguard/ci/verus.yaml` | `verus` and `kani` against the pull request's base; `verus-main` and `kani-main` on main, `verus-main` against the start of the push; one architecture (large-arm64, 60 and 30 minutes) by default, with a commented `verus-amd64` for proved code that differs by target | pcre3, challoc, expat-rs; `verus-amd64` as challoc's `verus-arm64` |
| `.chainguard/ci/fuzz.yaml` | `fuzz`, `fuzz-diff` and `fuzz-abi` on request, each fuzzer after its self-test; `known-citations` on every pull request | pcre3, expat-rs |
| `.chainguard/ci/conformance.yaml` | the deviations ratchet on both architectures; `differential` on request | pcre3, chainz, expat-rs |
| `.chainguard/ci/consumers.yaml` | one instance per consumer, on request | pcre3, chainz, expat-rs |
| `.chainguard/ci/env/default.yaml` | the empty default environment | all four |
| `.chainguard/ci/env/dev.yaml` | the union of the checks' packages (consumers.yaml's apk-tools and wolfi-keys included), for sessions and the deps agent | pcre3, chainz, challoc#688 |
| `.chainguard/develop.yaml` | sessions `from: .chainguard/ci/env/dev.yaml` | pcre3, chainz, challoc#688 |
| `.chainguard/deps.yaml` | the Checks deps agent: crates, toolchain (Rust, Verus with vstd, and Kani as one set), msrv (rust-version with the msrv check's toolchain), kit (the theseus-proved-port pin with `PROVED_PORT_VERSION` and the held headers), tools, the reference library, consumers | pcre3 main (#1079), challoc#689, chainz#64, expat-rs main (#201) |
| `.chainguard/reviewers.yaml` | the Checks reviewer `{PORT}`: `approve: true`, the read limits, no `diff_lines` (deprecated), and `rerun_on_checks` on the presubmit aggregate, since the reviewer reads its results | chainz and challoc |
| `.chainguard/review-prompt.md` | the sections the four prompts share, plus the runtime-costs-of-proof check that cites AGENTS.md; the port writes its own checks. Evidence reads the presubmit results, as chainz's does; the kit rule holds the kit's pins to one release, moved as one | all four; Evidence from chainz, the kit rule from pcre3's counter rule, the check from proved-c-port's "Prove the producer, not its output" (#13) |
| `.github/codeowners.yaml` | the codeowners-gate sidecar: one gated `*` entry, the maintainers and `bot:{PORT}` | pcre3, challoc, chainz |
| `.github/CODEOWNERS` | the sidecar's rendering, without the `bot:` owner | pcre3, challoc, chainz |

Of these, the Verus files (`.chainguard/ci/verus.yaml`,
`.chainguard/ci/miri.yaml`, `.chainguard/reviewers.yaml` and the
kit rule of `.chainguard/review-prompt.md`) are held to the kit
by `make verus-headers`. The rest (conformance, consumers, fuzz, test,
lint, the environments, `develop.yaml`, `deps.yaml`, the codeowners
files) are starting points: the port copies them, then reshapes them to
its own checks, and nothing holds them to the kit afterwards (expat-rs
and rust-port-template each keep a shape of their own for several,
wolfi-dev/expat-rs#221 and wolfi-dev/rust-port-template#13). Their kit
lines say which kit release a copy started from; a port may keep or
drop them.

### The required Verus check set

A port has every Verus check name from its first commit, so its check
selection only grows. Chainguard Checks refuses a run whose change
deselects one of its own checks (a check key deleted or renamed, the
way chainz#59 hit it), so a port that starts without a check and drops
it later cannot land that change; one that adds a check later is fine,
but has no gate there until it does. From the first commit:

| file | check | trigger |
|---|---|---|
| `.chainguard/ci/verus.yaml` | `verus` (`make verus` against the pull request's base) | presubmit |
| `.chainguard/ci/verus.yaml` | `verus-main` (`make verus` against the start of the push) | postsubmit |
| `.chainguard/ci/verus.yaml` | `kani` (`make kani`), for a port that will discharge seams with Kani | presubmit |
| `.chainguard/ci/verus.yaml` | `kani-main` (`make kani`), likewise | postsubmit |
| `.chainguard/ci/test.yaml` | `msrv` (`cargo build --release` on the `rust-version` toolchain) | presubmit, postsubmit |

Each passes on a port with no proofs yet:

- `verus` and `verus-main` pass at FLOOR 0.0 with the 0.0 badge, on a
  port with no `verus!` code, with the base Checks sets (a pull
  request's, or the start of a push to main) and also when that base
  predates `verus/`. On such a
  port `cargo verus focus` (Verus 0.2026.10.18) warns that no crate
  opted into verification, exits 0 and writes empty statistics, which
  the counter reads as no verified function; the counter's
  `a_port_with_no_proofs_passes_at_floor_zero` test holds both that and
  statistics with no module times.
- `kani` and `kani-main` fail when the selection has no harnesses,
  since cargo kani passes a crate with none and so would check nothing.
  A port with no harnesses yet declares it in its Makefile, before the
  include:

  ```make
  KANI_NONE = yes
  PROVED_PORT_VERSION = {KIT_VERSION}
  PROVED_PORT_HOME ?= /usr/share/proved-port
  include $(PROVED_PORT_HOME)/make/proved-port.mk
  ```

  `make kani` then lists the harnesses, passes when there are none, and
  fails when there are any, so the pull request that adds the first
  harness deletes the line. `kani.sh`'s header says the same.

A port that will never use Kani (expat-rs) leaves `kani` and
`kani-main` out from its first commit, as verus.yaml's header says.

Where the ports differ, a template takes the shape most of them share,
and its comments say what a port may change: a second architecture for
the proofs (verus.yaml's `verus-amd64`), a larger class with its reason,
no Kani (verus.yaml's header lists what to delete), or consumers from
packages that run on every pull request. Check keys are
the ones the ports use, so the `ci/run:<check>` labels carry over. The
deps entries each repeat the rules the agent must keep, since it sees
only its entry's prompt: FLOOR never goes down and gains no `lowered`
line, the kit's pin (`theseus-proved-port~`, `PROVED_PORT_VERSION`, and
the proved-port marketplace ref in a `.claude/settings.json` that names
one) moves only through the kit entry, and a reference bump
updates `docs/abi-changes.md`, the deviations lists and the
known-differences rows in the same pull request. The kit entry moves
every kit pin together, takes the kit's header text where `make
verus-headers` reports drift, and leaves a bump whose counter fails a
gate the old one passed for a person. The toolchain entry
commits the pin moves before it runs any suite, so a run that outlasts
the deps agent's two-hour budget still pushes a draft, then runs every
suite in the guest, Kani included, as pcre3's does (#1080). A port whose
suites do not fit the budget together leaves `make kani` to the pull
request's kani check and fixes what it reports on a turn, as chainz's
and challoc's do; its body then says Kani was left to the check. A port
with this file deletes `.github/dependabot.yml`.

The `verus` and `verus-main` checks run no counter tests: the package's
own build tests the counter. `make verus` fails when the installed KIT
or `verus-count --version` names another release than
`PROVED_PORT_VERSION`, so a stale install or another counter on PATH
fails.

The verus checks compare verus/FLOOR and verus/RLIMIT with one base, by
one rule: the base Checks sets (`CHAINGUARD_BASE_REVISION`), which both
`verus` and `verus-main` read. On a pull request (`verus`, presubmit) it
is the pull request's base. On main (`verus-main`, postsubmit) it is the
start of the push the run covers: Checks settles a burst of pushes into
one run at the last, and names as its base the tip its previous run
planned (when that is a first-parent ancestor, else the first parent).
`HEAD^` is not the base there: when a run covers two merges, `HEAD^` is
the first of them, and that merge would never be compared with main
before it. A run with no base fails rather than SKIP.

Every verus run compares FLOOR with that base. Where verus/FLOOR exists, a
run with no base fails, never SKIPs: the port's `make verus` passes
`--base-rev` and fails when `VERUS_BASE` is empty, as verus.yaml's
header says. A wrapper that prints SKIP and passes when no base is set
(chainz#59's `verus/count.sh`) lets main lower FLOOR unseen. A local
run passes `VERUS_BASE=none`, which gives the counter `--no-base`; the
counter refuses a `--floor` run with neither `--base-rev` nor
`--no-base` (proved-port#20).

A port whose `.chainguard/ci/identities.yaml` has a `trust` section
places chainguard[bot] (107423345), which opens the deps agent's pull
requests, in the group its member subring trusts, as challoc's `bots`
group does; without it those pull requests run at ring 0 and cannot
reach crates.io. deps.yaml's header says the same, since the kit ships
no identities.yaml template.

Apply them as above: copy, fill the placeholders (also `{MSRV}`,
`{SHELLCHECK_RELEASE}`, `{CLANG_STREAM}`, `{REFERENCE_PACKAGES}`, the
reference library's Wolfi packages as a comma-separated list such as
`zlib~1.3.2.1, zlib-dev~1.3.2.1`, `{REFERENCE_NAME}`, `{VERUS_CRATES}`,
`{PORT_ENV}`, `{DEPS_HOUR}`, `{CONSUMER_NAME}`, `{MAINTAINERS}` and
`{MAINTAINER_HANDLES}`), and delete the checks and entries that do not
apply. Then `chainctl checks validate` passes, and these find none (the
YAML files hold flow maps such as `{CONSUMER: selftest}`, so they take
the stricter pattern):

```sh
grep -rnE '\{[A-Z][A-Z_]*\}|\{[A-Z][a-z]' .chainguard/ci .chainguard/*.yaml .github/codeowners.yaml .github/CODEOWNERS
grep -nE '\{[A-Z][A-Z_]*[}:]|\{[A-Z][a-z]' .chainguard/review-prompt.md
```

Render `.github/CODEOWNERS` from the sidecar with the codeowners-gate
tool's apply step rather than by hand; the template shows what it
writes.

### The required check

A repository file cannot make a check required. The default branch's
protection must require `Chainguard Checks (presubmit)`, pinned to the
`chainguard` check app so that no other app can satisfy it, and nothing
else: the aggregate fails whenever a presubmit check fails, and the
on-request checks stay out of it. That setting is a repository ruleset
managed in chainguard-dev/infra's github-iac
(`github-iac/env/wolfi-dev/main.tf`), not made by hand: a new port adds
its name to the `required_checks` ruleset's `for_each`, which requires
that context with the `chainguard` app's `integration_id`.
chainguard-dev/infra#5389 is the example: it adds chainz, expat-rs, pcre3
and proved-port, importing pcre3's hand-made ruleset to add the app pin.

### The optional Miri checks

A port whose C API crate has unsafe code may run its tests under Miri.
These files are optional: a port without them deletes the deps.yaml
`miri` entry and env/dev.yaml's `libgcc`. They come from wolfi-dev/pcre3
main at `0c42c50d` and wolfi-dev/expat-rs#202 (`98588c1f`).

| template | copy to | what it holds |
|---|---|---|
| `.chainguard/ci/miri.yaml` | `.chainguard/ci/miri.yaml` | `miri` (Stacked Borrows, the thread seeds and the plain known tests) on every pull request and merge; `miri-tree` (Tree Borrows and the `tree-borrows` known tests) on request |
| `ci/miri.sh` | `ci/miri.sh` | the passes, sharded one per vCPU, with a dated nightly from rustup |
| `ci/miri-known.txt` | `ci/miri-known.txt` | the tests that hit an open, Miri-confirmed bug: `TEST #ISSUE REASON`, or `TEST #ISSUE tree-borrows REASON` for a report only Tree Borrows makes |
| `ci/lib.sh` | `ci/lib.sh` | `fetch`, `rust_host` and `rustup_pinned`, which pin rustup-init and the nightly's channel manifest by SHA-256 |

The deps.yaml `miri` entry moves the nightly and the rustup-init pin. A
port where another script installs a toolchain with `rustup_pinned`
(pcre3's ci/test-32.sh) moves the rustup-init pin in the entry that owns
that script instead (pcre3's toolchain entry), and the `miri` entry
leaves it alone.

A known test must still fail with undefined behaviour (expat-rs's rule):
ci/miri.sh leaves a plain row's test out of every pass, runs it alone
under Stacked Borrows (the `miri` check) and fails when it passes, so
the fix deletes its row, and when it fails another way, so the row is
updated. A `tree-borrows` row is for a report only Tree Borrows makes:
its test runs in the Stacked Borrows passes (the thread seeds included)
like any other, is left out of the Tree Borrows pass, and is run alone
there (the `miri-tree` check) under the same rule. A row that names no
listed test, or no `#issue`, fails it too. A test Miri cannot run is not a row: it says so
with `cfg(miri)`, or its file goes in `nomiri`.

Where the two ports differ, ci/miri.sh's settings choose:

- `nomiri`: the test files Miri cannot run. expat-rs runs only the
  library's tests, since tests/c_link_test.rs links C; pcre3 runs every
  test binary. The template finds the binaries as pcre3 does and leaves
  out those `nomiri` names.
- `thread_tests`: pcre3's tests start threads, so it runs them again
  over the scheduler seeds 0..4; expat-rs's start none. With `no`, a test
  that starts a thread fails the script until it says `yes`. The thread
  tests are found by scanning the source, as pcre3 does. pcre3 fails
  when a name it finds is not in the Miri listing. The template skips
  that name instead, as a test that `cfg(not(miri))` or a feature leaves
  out of the Miri build. A name that matches more than one listed test
  still fails.
- `flags`: pcre3 passes `-Zmiri-permissive-provenance` for the one
  integer-to-pointer cast it keeps (its recursion guard); expat-rs passes
  nothing.
- pcre3 has no known list, expat-rs lists three tests (#168, #170,
  #171). The template keeps the list, empty.
- expat-rs runs one Miri process per test, so the first undefined
  behaviour in a shard does not hide the shard's later tests; pcre3 runs
  each shard's tests of a binary in one process, which is faster. The
  template does as pcre3 does; the failing shard's log names the test.
- pcre3's ci/miri.sh installs curl in a session that lacks it; the
  template's env/dev.yaml has curl, so the template does not.

Fill the placeholders as above, also `{MIRI_NIGHTLY}`,
`{MIRI_MANIFEST_SHA256}`, `{CAPI_CRATE_DIR}` (the crate's directory, such
as `crates/{CAPI_CRATE}`), `{RUSTUP_RELEASE}` and the two
`{RUSTUP_INIT_SHA256_...}` values (both sources pin rustup 1.29.2), and
keep one choice of each of `nomiri`, `thread_tests` and `flags`. Add a
`miri` target to the Makefile that runs `sh ci/miri.sh` in a session.
The shell files use `${VAR:-}`, so they take a pattern that skips `${`;
this finds none when you are done:

```sh
grep -nE '(^|[^$])\{[A-Z][A-Z0-9_]*[}:]|\{[A-Z][a-z]' ci/miri.sh ci/lib.sh ci/miri-known.txt .chainguard/ci/miri.yaml
```
