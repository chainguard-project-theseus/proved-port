# Theorems

What the Verus proofs establish today, each with its statement, where it
is proved, and what it rests on. Every theorem also rests on the common
trusted base (below) and on the core-library seams in [SEAMS.md](SEAMS.md).

This file holds the trusted base. Every theorem has a file of its own,
`verus/theorems/<ID>.md`, named after the lead id of the change that
proves it (the kit's
[theorem skeleton](https://github.com/wolfi-dev/proved-port/blob/{KIT_SHA}/kit/templates/verus/theorems/TEMPLATE.md)
is its form).
It holds the theorem's statement and a `### Seams` table, in SEAMS.md's
columns, of the seams that change introduces. Seams are numbered once, so
a theorem that rests on another's seam names its id.

## Trusted base

Verus {VERUS_RELEASE} and Z3, rustc {RUSTC_VERSION} (the toolchain Verus
verifies with; the shipped build uses Wolfi's `{RUST_STREAM}`), vstd's
specifications of `core` (slice indexing, integer operations).
{GENERATED_TABLES: name each generated file a theorem treats as its
specification, and how it is checked against its source; delete when
there are none.}

## Index

| id | statement | file |
|---|---|---|
| {ID} | {ONE_LINE_STATEMENT} | [theorems/{ID}.md](theorems/{ID}.md) |
