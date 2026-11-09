# {ID}. {TITLE}

`{FILE}`, `{FUNCTION}`. {ONE_PARAGRAPH: what is proved, for which
inputs, and the premises it needs from callers (the `requires`).}

1. **{CLAUSE}**: {The `ensures` in words, with the spec function it is
   stated against}.
2. **{CLAUSE}**: {STATEMENT}.

{The design or upstream reference the statement comes from, if any: an
RFC, a CVE class it rules out, a section of the C library's
documentation.}

## Mutants

Each mutant is a small change to the code or to the proof that a correct
statement must refuse. List every one tried, and what refused it:

- {MUTANT}: {Verus reports the postcondition `...` fails | the
  invariant `...` fails | test `...` fails}.
- {MUTANT}: {WHAT_REFUSED_IT}.

A statement no mutant can break says nothing; add the mutant that shows
it does.

## Checks

- `make verus`: {CRATE}'s results line, and the counter's `shipped`
  row before and after ({COUNTS: N of M functions, P%}).
- {Release asm or codegen of the changed functions, when the change
  rewrote shipped code to verify it: instruction counts and panic sites
  before and after.}
- {Tests added or changed, by name.}

## What it does not say

{The limits of the statement, so no one reads more into it: the inputs
it does not cover, the properties it does not claim (termination,
timing, agreement with the C library), the seams it rests on that are
still trusted, and the code paths outside `verus!` that it does not
reach.}

### Seams

| # | item | kind | category | where | contract | proposed tool | status |
|---|---|---|---|---|---|---|---|
| S-{FAMILY}{N} | `{ITEM}` | external_body | {CATEGORY} | `{FILE}` | {CONTRACT}; grounds: {GROUNDS} | {TOOL} | trusted |

{Or: "No new seams; it rests on S1 and S-cr2." A theorem that rests on
another theorem's seam names its id here instead of copying the row.
`{FAMILY}` is this theorem id's letters in lower case and `{N}` the next
number across the family (SEAMS.md, "Columns").}
