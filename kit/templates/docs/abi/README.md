# Known differences from {LIBRARY} {UPSTREAM_VERSION}

{PORT} matches {LIBRARY} {UPSTREAM_VERSION}'s ABI: its header constants,
exported names, sonames, symbol version nodes, error codes and error
texts. Each deliberate difference is an entry in this directory, added in
the pull request that makes it. The C library decides what is correct: a
difference found by the conformance ratchet, the differential runner or
fuzz-diff is either a bug to fix or an entry here, and every deviations
line and every known-divergence predicate cites the entry it rests on.

## Entry format

```markdown
### <ID>: <one-line summary>

- **Status:** active | planned | removed (<date>, PR #n)
- **Since:** {PORT} <version> (PR #n)
- **Area:** {AREAS: the parts of the API, such as compile | match | config | header | symbols | limits}
- **Surface:** the exact functions, macros, options or error codes involved
- **{LIBRARY} {UPSTREAM_VERSION}:** what upstream does
- **{PORT}:** what {PORT} does instead
- **Why:** one or two sentences, with a design or research reference
- **Affected consumers:** Wolfi packages (from the consumer survey)
  that hit this, and how; "none known" if the survey found none
- **Detection:** how a consumer or test can tell (a probe, a grep pattern
  for consumer sources, a test name)
- **Patch recipe:** what a package patch must change: file, function, and
  the replacement logic. Write "none needed" if consumers cope unchanged,
  and say why.
```

Each entry is a file of its own, `docs/abi/ABI-<nnn>.md`, whose first
line is the entry's heading. A pull request that adds a difference adds
one file, so two pull requests that each add an entry touch different
files, and this file stays as it is.

## IDs

IDs are `ABI-<nnn>`, three digits, assigned in order and never reused.

- **Picking an ID.** The next ID is one more than the highest
  `docs/abi/ABI-<nnn>.md` on `main`. A removed entry keeps its file and
  says when and why it went, so its ID stays taken.
- **No gaps.** The files run from ABI-001 to the highest with no number
  missing. An abandoned pull request leaves no gap: its ID never reaches
  `main`, so the next pull request takes the same number.
- **Two pull requests with the same ID.** Both add the same file name,
  so the second to rebase onto `main` stops on an add/add conflict in
  `docs/abi/ABI-<nnn>.md`. Keep `main`'s file as it is, move your entry
  to the new next ID (rename the file and change its heading), and change
  every citation of the old ID that your pull request adds: deviations
  reasons, known-differences rows, code comments, tests and docs.

## Checking the ledger

Every citation resolves: the deviations list, the fuzz-diff known table
and the tests name only IDs that have a file here. Before calling a phase
done, list the entries and their headings, and check that each file's
heading names its own ID and that no number is missing:

```sh
grep -m1 '^### ABI-' docs/abi/ABI-*.md
```
