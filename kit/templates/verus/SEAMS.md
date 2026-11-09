# Seams: what the Verus proofs take on trust

Every theorem in [THEOREMS.md](THEOREMS.md) and `verus/theorems/` is
conditional: Verus checks the verified code against the contracts of the
items it cannot see into and assumes those contracts hold. A seam is one
such item. A seam leaves the trusted set only when Verus verifies it, or
when another tool (Kani) checks its contract and a record under
`verus/seams/` says how.

This table holds the core-library seams: the `core` and `std` contracts in
`{VSPEC_FILE}`, which every crate shares. A seam that belongs to one
theorem (an `external_body` function, a crate-internal contract, a premise
of the C caller) is in that theorem's `### Seams` table instead. To list
them all:

```sh
grep -h '^| S' verus/SEAMS.md verus/theorems/*.md
```

Verus rejects a second contract for the same item, so each `core` item has
one row here. Where vstd states a contract itself, the row names vstd: the
contract is still trusted, now as vstd's.

## Columns

- **#**: the seam id, `S<n>` here and `S-<family><n>` in a theorem's
  table: `<family>` is the letters of the theorem's id in lower case,
  and `<n>` the next number across that family's theorems, with no
  hyphen between them (`S-cr9`, a seam of one of the `CR` theorems).
  Ids are assigned once and never reused; a theorem that rests on
  another theorem's seam names its id instead of copying the row.
  Ids assigned in an older form keep it, since an id is never renamed:
  pcre3's `S-dn1-1` (the theorem's whole id, a hyphen, then the number)
  and `S-jit-2` (a hyphen after the family) are two. New ids take the
  form above, numbered after the family's ids in that form. The counter
  reads any id that starts `S-`.
- **item**: the items the seam covers, as the source names them.
- **kind**: `assume_specification`, `external_body`,
  `external_fn_specification`, `external_type_specification`,
  `external_trait_specification`, `#[verifier::external]`, `axiom`, or a
  token minted on trust (`Tracked::assume_new`).
- **category**: `core library`, `std library`, `permanent (the C
  caller's contract)`, `operating system`, `machine model`, a
  dependency (`the memchr crate`), `Verus limit` (a construct Verus
  rejects), or the port code outside `verus!` it stands for.
- **where**: the file that states the contract.
- **contract**: what the seam assumes, then `grounds:` (the documentation
  quoted at the contract, or the observed result) and `used by` (the
  theorems that rest on it).
- **proposed tool**: `Kani`, `Verus` (once something it lacks lands),
  `none needed`, or `none: the premise`.
- **status**: `trusted`, `discharged by Kani {KANI_RELEASE}
  (`verus/seams/kani-<id in lower case>.md`)` with the bounds the
  record names, `verified over <seam>` once the item is inside
  `verus!`, or `retired (<theorem>): <why>` once the seam is gone (the
  item verified, or no longer built outside tests). A retired row
  stays, so its id is not reused.

| # | item | kind | category | where | contract | proposed tool | status |
|---|---|---|---|---|---|---|---|
| S1 | `{ITEM}` | assume_specification | core library | `{VSPEC_FILE}` | {CONTRACT}; grounds: the `core` documentation, quoted in {VSPEC_FILE_NAME}; used by {THEOREM_IDS} | Kani | trusted |
