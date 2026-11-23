# Vulnerability classes

Every vulnerability class that has hit {LIBRARY} upstream, and what
rules it out of {PORT}: a theorem in [THEOREMS.md](THEOREMS.md)
(`verus/theorems/<ID>.md`), the construction of the port, or the open
`vuln class:` issue that will prove one. {RESEARCH_NOTE: name the
survey these rows start from, such as `docs/research/cves.md`, and say
that this file keeps the mapping to proofs current; delete when there
is none.}

A class is ruled out for every input, not only for a reproducer.
"Every input" means every argument of the exported functions a row
names that meets the C caller's premise: a theorem here states its
property (every index in bounds, no size computation that overflows,
recursion under a stated limit, work bounded by a stated function of
the input's length) over all of them. A theorem on an inner function
rules a class out only when each clause of its `requires` is the C
caller's premise or is proved on every route in from those exported
functions (the theorem file's Premises table says which); anything
less is `partial:`. The CVE's reproducer, where one is public, is a
regression test beside the theorem. "Gap" means no theorem states the
property for every input; it does not mean {PORT} has the bug.

## What the construction gives, and what it does not

{Say which crates forbid `unsafe` (and the check that holds the fence),
and what follows there: an out-of-bounds index or an overflowing
operation (`overflow-checks = true` in every profile) panics instead of
corrupting memory, and the borrow checker rules out a use after free.
Say where a panic goes (the entry points' `catch_unwind` and the error
value C sees), so that what is left of a memory-safety class in those
crates is denial of service, which a theorem removes by proving the
panic unreachable. Name the crate that holds the `unsafe` code (the C
ABI) and the premise its theorems take about the C caller's pointers
and lengths: there a memory-safety class stays one, and its rows come
first among the class theorems. The blowup classes (entity expansion,
recursion, quadratic work) are not memory-safety bugs, and safe Rust
does nothing for them; there the theorem is a bound on work or depth.}

Every theorem rests on the trusted base in [THEOREMS.md](THEOREMS.md)
and on its seams ([SEAMS.md](SEAMS.md) and each theorem's `### Seams`
table). A row whose theorem rests on more than that (a seam, a premise
a test pins, a `requires` its verified callers do not prove on every
route in, code outside `verus!`) says so, and is `partial:`.

## How to read a row

- **id** is the CVE, the upstream advisory, the upstream ChangeLog item
  or fix commit for a security fix that got no CVE, or the OSS-Fuzz id.
  Each id appears once. A row for a class with no single id is
  `(class)`.
- **year** is the year in the id, or of the release that fixed it.
- **class** is the row's primary class, one of those in "Classes".
- **upstream** names the faulty upstream function and its file.
- **{PORT} counterpart** names the crate and module that correspond to
  it and the exported functions that reach it, or says there is none
  (the port does not ship that tool or feature).
- **ruled out by** lists the theorem ids that rule the class out for
  every input of those exported functions, each written `ID (#n)` with
  the issue that proved it, then a clause on what it proves (a word
  starting with a capital letter and holding a digit, right before
  ` (#n)`, reads as a theorem id; each id carries its own ` (#n)`, in
  plain text, not code font or a link). `partial:` says what is proved and what is not;
  `gap: #<n>` names the class's open issue;
  `not shipped` marks a row with no counterpart. `by construction:`
  names the fact and the check that holds it (the `unsafe` fence's
  check, a type the counterpart cannot be given). A memory-safety class
  ruled out by construction in a crate that forbids `unsafe` still
  leaves the panic on the same input, so its row also gives the
  theorem id that proves the panic unreachable or a `gap: #<n>`.
- **sources** links the CVE record (`https://nvd.nist.gov/vuln/detail/<id>`,
  or `https://www.cve.org/CVERecord?id=<id>`), the upstream advisory
  and the fix commit or issue the record cites.

## Catalog

| id | year | class | upstream | {PORT} counterpart | ruled out by | sources |
|---|---|---|---|---|---|---|
| {CVE_ID} | {YEAR} | {CLASS: such as heap over-read, heap buffer overflow, integer overflow in a size computation, unbounded recursion, excessive work, use after free, double free, uninitialized read, NULL dereference, state confusion} | {UPSTREAM: `function` (file.c)} | {COUNTERPART: `crate` `module`, reached from `exported_fn`} | {RULED_OUT_BY: ID (#n), what it proves for every input of the exported functions} | {SOURCES: the NVD record, the fix commit} |

## Classes

Each class below is a property over every input, in the sense above.
For each, the theorems that prove it today, what holds by
construction, what is partial or trusted, and the gap.

### {Class}

**Property.** {The property, stated for every input of the exported
functions that reach the counterparts: every read at an index inside
its buffer; every size computation without overflow; recursion depth
at most a stated function of the input; work at most a stated function
of its length.}

**Proved.** {The theorem ids, each `ID (#n)`, with the exported
functions each covers, or "none".}

**By construction.** {The fact and the check that holds it, and for a
memory-safety class the theorem or gap for the panic that is left;
delete when nothing.}

**Partial or trusted.** {What the theorems rest on beyond the trusted
base: seams, premises, code outside `verus!`; delete when nothing.}

**Gap.** {`gap: #<n>`, the `vuln class:` issue that will prove the
rest, or "none".}

## Gaps

Each gap is one issue, titled `vuln class: <class> (<ids>)`, which
proves the class's property for every input and adds the public
reproducers as regression tests. A class with no theorem gets its
issue when its rows land here.

1. **{GAP_ISSUE: #n `vuln class: <class> (<ids>)`}.** {The property to
   prove, and the functions it covers.}

## Keeping this current

With the line `vuln-classes<TAB>verus/VULN-CLASSES.md` in
`verus/seams.rules`, `proved-port-seams -check` reads this file in the
layout above: each theorem cited `ID (#n)` in a row or a **Proved.**
paragraph has its `verus/theorems/<ID>.md`, each row's class has a
section, each `gap: #<n>` is an item of Gaps, and each item of Gaps is
cited; a ruled out by cell that cites neither form and is neither `not
shipped` nor `by construction:`, a class section without its
**Proved.** or **Gap.** paragraph, such a paragraph that cites none and
is not "none", and an id in code font or a link before ` (#n)` fail. An
id written without its own ` (#n)` is not read, and neither is a
`by construction:` row's check nor whether a memory-safety row also
names its panic's theorem or gap. Whether a cited issue is still open,
and everything else here, stays true only by review. A new
upstream CVE or advisory adds its row in the change that learns of it,
with the record fetched. A change that proves a class replaces the
class's issue number with its theorem ids, in the class's section and
in each row it covers. A change that renames or removes a theorem file
updates every row and section citing its id, and a `vuln class:` issue
closed without a theorem moves its rows to `partial:` or a new gap.
Each row's class has a section, and each `gap: #<n>` is in the Gaps
list. A live bug the research turns up in {PORT}
itself, or an undisclosed one upstream, is a security finding: report
it privately before writing anything public, and add it here only once
it is fixed and disclosed.
