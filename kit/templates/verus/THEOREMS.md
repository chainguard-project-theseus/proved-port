# Theorems

The trusted base every Verus theorem rests on, beyond the core-library
seams in [SEAMS.md](SEAMS.md). Each theorem's statement, where it is
proved and its premises are in its own file under `verus/theorems/`.

This file holds the trusted base. Every theorem has a file of its own,
`verus/theorems/<ID>.md`, named after the lead id of the change that
proves it (the proved-port kit's theorem skeleton,
`kit/templates/verus/theorems/TEMPLATE.md` at `v{KIT_VERSION}`, is its form).
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

Each theorem file's first line, `# <ID>. <title>` (the skeleton's
heading), is its index entry. This file keeps no list of its own, so a
change that adds a theorem adds its file and leaves this file alone. To
list every theorem with its title:

```sh
grep -m1 '^# ' verus/theorems/*.md
```
