# Templates

Skeletons a new port copies: the proof ledgers' file headers, the seams
and theorem documents, the Kani record, the known-differences ledger, and
outlines of `AGENTS.md` and `CONTRIBUTING.md`. They come from
wolfi-dev/pcre3 at `5216e03d` and wolfi-dev/challoc at `7ef336c4` with
the project names taken out.
The rule text in each header (how FLOOR ratchets, how RLIMIT headroom
and REACH rows work) is the contract the counter enforces and agents
read, so keep it as written in the port's own file; fill in the
placeholders and the rows. Each header ends with a
`# kit: proved-port kit/v{KIT_VERSION}` line naming the kit version it
came from, and a lint is to diff the port's header against the template
at that version (kit/PLAN.md, step 1). `AGENTS.md` and `CONTRIBUTING.md`
point at those headers rather than restating them.

| template | copy to | what it holds |
|---|---|---|
| `verus/FLOOR` | `verus/FLOOR` | the coverage ratchet: four figures, `lowered` lines |
| `verus/RLIMIT` | `verus/RLIMIT` | rlimit headroom: `max-share`, `allow`, `rlimit-raised` |
| `verus/REACH` | `verus/REACH` | functions a C caller reaches that Verus does not check |
| `verus/MINTS` | `verus/MINTS` | the C caller's token kinds and mints, for a port that mints tokens through one trusted function |
| `verus/README.md` | `verus/README.md` | headings only: the five sections the headers cite |
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
2. Replace every `{PLACEHOLDER}`. A placeholder in capitals is a value
   (`{PORT}`, `{LIBRARY}`, `{UPSTREAM_VERSION}`, `{CAPI_CRATE}`,
   `{VERUS_RELEASE}`, `{KANI_RELEASE}`, `{RUST_STREAM}`, `{MSRV}`,
   `{KIT_VERSION}`);
   one with a colon (`{SIGNING_POLICY: ...}`) is a value its text
   describes, and where it lists choices split by `|`, keep one; one in
   a sentence (`{How to run the gate ...}`) is prose to write, or to
   delete with its item when it does not apply. When you are done, this
   finds none:

   ```sh
   grep -rnE '\{[A-Z][A-Z_]*[}:]|\{[A-Z][a-z]' AGENTS.md CONTRIBUTING.md docs/abi-changes.md verus/
   ```
3. Start FLOOR at the counter's first run: record the figures its
   `shipped` row names, rounded down, as the check prints them. A port
   with no proofs yet starts at `0.0` and rises with its first theorem.
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
| `.chainguard/ci/verus.yaml` | `verus` and `kani` against the pull request's base; `verus-main` and `kani-main` on main against `HEAD^` | pcre3, challoc, expat-rs |
| `.chainguard/ci/fuzz.yaml` | `fuzz`, `fuzz-diff` and `fuzz-abi` on request, each fuzzer after its self-test; `known-citations` on every pull request | pcre3, expat-rs |
| `.chainguard/ci/conformance.yaml` | the deviations ratchet on both architectures; `differential` on request | pcre3, chainz, expat-rs |
| `.chainguard/ci/consumers.yaml` | one instance per consumer, on request | pcre3, chainz, expat-rs |
| `.chainguard/ci/env/default.yaml` | the empty default environment | all four |
| `.chainguard/ci/env/dev.yaml` | the union of the checks' packages (consumers.yaml's apk-tools and wolfi-keys included), for sessions and the deps agent | pcre3, chainz, challoc#688 |
| `.chainguard/develop.yaml` | sessions `from: .chainguard/ci/env/dev.yaml` | pcre3, chainz, challoc#688 |
| `.chainguard/deps.yaml` | the Checks deps agent: crates, toolchain (Rust, Verus with vstd, and Kani as one set), msrv (rust-version with the msrv check's toolchain), tools, the reference library, consumers | pcre3 main (#1079), challoc#689, chainz#64, expat-rs main (#201) |
| `.chainguard/reviewers.yaml` | the Checks reviewer `{PORT}`: `approve: true`, the read limits, and `rerun_on_checks` only with the Results half of review-prompt.md's `{CI_RULE}` | challoc and chainz (Results, rerun); pcre3 main (#1078) and expat-rs#202 (Ignore CI, no rerun) |
| `.chainguard/review-prompt.md` | the sections the four prompts share, plus the runtime-costs-of-proof check that cites AGENTS.md; the port writes its own checks | all four; the check from proved-c-port's "Prove the producer, not its output" (#13) |
| `.github/codeowners.yaml` | the codeowners-gate sidecar: one gated `*` entry, the maintainers and `bot:{PORT}` | pcre3, challoc, chainz |
| `.github/CODEOWNERS` | the sidecar's rendering, without the `bot:` owner | pcre3, challoc, chainz |

Where the ports differ, a template takes the shape most of them share,
and its comments say what a port may change: a check on one architecture
only, no Kani (delete `kani`, `kani-main` and the `kani~` pins), or
consumers from packages that run on every pull request. Check keys are
the ones the ports use, so the `ci/run:<check>` labels carry over. The
deps entries each repeat the rules the agent must keep, since it sees
only its entry's prompt: FLOOR never goes down and gains no `lowered`
line, `verus/count` changes only by `kit/sync.sh`, and a reference bump
updates `docs/abi-changes.md`, the deviations lists and the
known-differences rows in the same pull request. The toolchain entry
commits the pin moves before it runs any suite, so a run that outlasts
the deps agent's two-hour budget still pushes a draft, then runs every
suite in the guest, Kani included, as pcre3's does (#1080). A port whose
suites do not fit the budget together leaves `make kani` to the pull
request's kani check and fixes what it reports on a turn, as chainz's
and challoc's do; its body then says Kani was left to the check. A port
with this file deletes `.github/dependabot.yml`.

The `verus` and `verus-main` checks run verus/count's own tests before
the proofs, as all three sources do. A port's CI cannot run
`kit/sync.sh --check` (the kit is internal), so those tests are the only
check in the port of a hand-edited copy.

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
apply. The `{CI_RULE}` choice in review-prompt.md also decides
reviewers.yaml's `rerun_on_checks`: keep it with Results, delete it with
Ignore CI, as that file's comment says. Then `chainctl checks validate` passes, and these find none (the
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
