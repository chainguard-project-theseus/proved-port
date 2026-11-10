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
   `{VERUS_RELEASE}`, `{KANI_RELEASE}`, `{RUST_STREAM}`, `{KIT_VERSION}`);
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
