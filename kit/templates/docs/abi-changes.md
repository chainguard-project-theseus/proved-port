# Known differences from {LIBRARY} {UPSTREAM_VERSION}

{PORT} matches {LIBRARY} {UPSTREAM_VERSION}'s ABI: its header constants,
exported names, sonames, symbol version nodes, error codes and error
texts. Each deliberate difference is an entry here, added in the pull
request that makes it. The C library decides what is correct: a
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

IDs are `ABI-<nnn>`, assigned in order and never reused. A removed entry
keeps its id and says when and why it went.

Every citation resolves: the deviations list, the fuzz-diff known table
and the tests name only ids that have an entry here. Compare the count
with `grep -c '^### ABI-' docs/abi-changes.md` before calling a phase
done.

## Entries

### ABI-001: {SUMMARY}

- **Status:** active
- **Since:** {PORT} 0.1.0 (bootstrap)
- **Area:** {AREA}
- **Surface:** {SURFACE}
- **{LIBRARY} {UPSTREAM_VERSION}:** {WHAT_UPSTREAM_DOES}
- **{PORT}:** {WHAT_THE_PORT_DOES}
- **Why:** {WHY}
- **Affected consumers:** none known
- **Detection:** {DETECTION}
- **Patch recipe:** none needed: {WHY_CONSUMERS_COPE}
