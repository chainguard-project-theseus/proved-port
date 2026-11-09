# Verifying {PORT} with Verus

{What the proofs cover, how to run them (`make verus`, `make kani`), and
where the theorems, seams and ledgers live.}

## What counts as verified

{Which functions count as verified, discharged, trusted or outside, and
what each class means for a C caller. verus/MINTS cites this section.}

## Coverage

{What the counter counts (the `shipped` row, the crates it leaves out),
how the shares are computed, and how verus/FLOOR holds main to them.
verus/FLOOR cites this section.}

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
