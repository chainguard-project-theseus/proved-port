# Verifying {PORT} with Verus

{What the proofs cover, how to run them (`make verus`, `make kani`), and
where the theorems, seams and ledgers live.}

## What counts as verified

{Which functions count as verified, discharged, trusted or outside, and
what each class means for a C caller. verus/MINTS cites this section.}

## Coverage

```sh
make verus VERUS_BASE=none     # locally: the proofs, then the counter with FLOOR and RLIMIT
verus-count .                  # the shares, per crate
verus-count --list .           # one line per function
```

`verus-count` is the counter the theseus-proved-port package installs
("Counter" below). It reads every shipped crate's source, `verus!` blocks included, and classes each executable function,
and the lines from its `fn` to its closing brace, as verified, trusted,
discharged or outside ("What counts as verified"). Test-only and
verifier-only code is left out. It reads source and does not run Verus,
so "verified" means "under `verus!` and not trusted"; `make verus` is
what checks the proofs.

It prints a `total` row and a `shipped` row. The `shipped` row is
{SHIPPED_CRATES: the crates it counts, as verus/FLOOR's header names
them, and the crates it leaves out, with why}. Shares are to a tenth of
a percent, rounded down: `verified%` is Verus's alone, and `checked%`
adds the functions Kani discharges ("Kani").

[FLOOR](FLOOR) holds the `shipped` row's four shares. `make verus` fails
below any figure, and above one names the figure to record. With
`VERUS_BASE=REV` (both verus checks pass the base Checks sets: the pull
request's base, or on main the start of the push the run covers) it
also fails a figure set below
REV's copy unless a new `lowered` line names that figure and value, and
it fails when REV does not resolve. It never skips that comparison:
with no `VERUS_BASE` it fails before it runs anything. `VERUS_BASE=none`
(the counter's `--no-base`) runs with no base, for a local run and the
proof pull request that adds the first FLOOR; no check passes it.
FLOOR and the badge rise in a pull request of their own, to the figures
the check names, never in the pull request that earns them, but for the
port's first proof pull request, which may raise them from 0.0 (FLOOR's
header).

The README's badge shows FLOOR's `functions` and `lines` figures and
links here (`verus/README.md#coverage`). With the counter's `[badge]`
gate on in verus/count.toml, the verus check fails when the badge and
FLOOR disagree, and `verus-count --badge .` prints the line to use.

## Entry points

{How the counter walks from each entry point a C caller reaches, and how
verus/REACH lists what it reaches that Verus does not check. verus/REACH
cites this section.}

## Headroom

{How rlimit use is measured and why a function near its limit breaks
main later. verus/RLIMIT cites this section.}

## Kani

{Which trusted functions Kani discharges, the `discharged_by = "kani"`
marker, the records under verus/seams/, and how `make kani` runs them.
verus/FLOOR cites this section.}

## Counter

`verus-count` is the proved-port kit's coverage counter. {PORT} takes
the kit as the theseus-proved-port package, pinned
(`theseus-proved-port~X.Y.Z`) in `.chainguard/ci/verus.yaml` and
`.chainguard/ci/env/dev.yaml`, with the same release as the Makefile's
`PROVED_PORT_VERSION`. The package installs the counter on PATH and the
kit's make targets under `/usr/share/proved-port/make/` (`proved-port.mk`,
which the Makefile includes for `make verus` and `make kani`, and its
scripts), with the templates `make verus-headers` holds the headers to
and a KIT file naming the release's tag and commit. `make` fails a kit
target when the installed KIT or `verus-count --version` names another
release than `PROVED_PORT_VERSION`. The pins move together (with the
proved-port marketplace ref in `.claude/settings.json`), in a pull
request of their own (`.chainguard/deps.yaml`, the kit entry). {PORT}
carries no copy of the counter or the make targets: a change to either
lands in the kit's `kit/count` or `kit/make`, in a proved-port pull
request, and reaches {PORT} with the next kit release.

[count.toml](count.toml) configures the counter for {PORT} (the crates,
the test-only `cfg`s and features, and where each table this file names
lives; the kit's `kit/count/README.md`, "Configuration", which also
states every rule the counter enforces), and the headers of FLOOR,
RLIMIT, REACH and MINTS state each file's. Those headers are the kit's
templates filled in for {PORT}, held to the release the package pin
names.
