# Verifying {PORT} with Verus

{What the proofs cover, how to run them (`make verus`, `make kani`), and
where the theorems, seams and ledgers live.}

## What counts as verified

{Which functions count as verified, discharged, trusted or outside, and
what each class means for a C caller. verus/MINTS cites this section.}

## Coverage

```sh
make verus VERUS_BASE=none     # locally: the proofs, then the counter with FLOOR and RLIMIT
cargo run --locked --release --manifest-path verus/count/Cargo.toml \
  --target-dir target/verus-count -- .           # the shares, per crate
cargo run --locked --release --manifest-path verus/count/Cargo.toml \
  --target-dir target/verus-count -- --list .    # one line per function
```

The counter (verus/count, "Counter" below) reads every shipped crate's
source, `verus!` blocks included, and classes each executable function,
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
`VERUS_BASE=REV` (the verus check passes the pull request's base,
verus-main the commit before its own) it also fails a figure set below
REV's copy unless a new `lowered` line names that figure and value, and
it fails when REV does not resolve. It never skips that comparison:
with no `VERUS_BASE` it fails before it runs anything. `VERUS_BASE=none`
(the counter's `--no-base`) runs with no base, for a local run and the
proof pull request that adds the first FLOOR; no check passes it.
FLOOR and the badge rise in a pull request of their own, to the figures
the check names, never in the pull request that earns them (FLOOR's
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

`verus/count/` is the proved-port kit's coverage counter
(`verus-count`), vendored: [count/KIT](count/KIT) names the proved-port
commit it was copied from, and [count.toml](count.toml) configures it
for {PORT} (the crates, the test-only `cfg`s and features, and where
each table this file names lives; count/README.md, "Configuration").
count/README.md states every rule it enforces, and the headers of
FLOOR, RLIMIT, REACH and MINTS state each file's. Those headers are the
kit's templates filled in for {PORT}, and each ends with a
`# kit: proved-port vX.Y.Z (SHA)` line naming the kit release and commit
it matches.

The same sync vendors the kit's make targets to `verus/make/`
(`proved-port.mk`, which the Makefile includes for `make verus` and
`make kani`, its scripts, and the templates `make verus-headers` holds
the headers to), under the same KIT.

Never edit either copy. A change to the counter or the make targets
lands in the kit's `kit/count` or `kit/make` first, in a
proved-port pull request; then a {PORT} pull request runs
`kit/sync.sh <{PORT} checkout>` from a clone of proved-port at that
commit, which replaces `verus/count/` and `verus/make/` and rewrites
KIT, and `kit/sync.sh --check <{PORT} checkout>`, which fails unless
both copies are exactly the commit KIT names. Name that commit in the pull
request: the checks here cannot reach proved-port.
The verus check runs the copy's tests (`counter tests`) before the
proofs. The copy is a workspace of its own with its own Cargo.lock,
built under `target/verus-count` so `git status` stays clean, and none
of its dependencies reach the shipped crates.
