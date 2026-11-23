# proved-port kit changes

What each release of the kit changes, and what a port does when it
moves its pin to that release. The theseus-proved-port package installs
this file as `/usr/share/proved-port/CHANGELOG.md`, and `kit/install.sh`
as `PREFIX/share/proved-port/CHANGELOG.md`.

A bump applies the porting notes of every release newer than the one
the pin moves from, up to and including the one it moves to, oldest
first, and no others. Each `## X.Y.Z` heading is a release; KIT records
it as `tag vX.Y.Z`. The deps agent's kit entry
(`kit/templates/.chainguard/deps.yaml`) makes the bump; whatever the
notes say, it ends with `make verus-headers`, taking the kit's text
where it reports drift.

## 0.4.14

### Changes

- `make noalias` finds the library's LLVM IR with cargo 1.100 (Wolfi's
  rust-1.100) and later. That cargo's build directory layout writes
  each unit's outputs to `release/build/PACKAGE/HASH/out` (or
  `TRIPLE/release/build/PACKAGE/HASH/out`) instead of `release/deps`,
  so with 0.4.13 and earlier every build failed with `no LLVM IR for
  CRATE`. It now reads, and clears before each build, the out
  directories of the library's units (those whose fingerprint has a
  `lib-CRATE` file) as well as `deps/`, so an older cargo works as
  before. A build script's `OUT_DIR`, a sibling there, is neither
  scanned nor cleared.
- `make lint-unsafe` lets through the one `--force-warn` cargo 1.100
  and later pass every rustc themselves,
  `--force-warn=unused_crate_dependencies` (which leaves unsafe_code's
  level alone), written either way, also in an @FILE argument file.
  With 0.4.13 and earlier every member's compile on that cargo was
  refused as getting a flag that outranks `-Funsafe_code`. Any other
  `--force-warn` (of unsafe_code, a group, or with no lint after it) is
  still refused, as is `--cap-lints`.

### Porting notes

1. A port on rust-1.99 or older changes nothing for these two targets.
   A port moving to rust-1.100 or later moves its pin to 0.4.14 first,
   or in the same change as its rust stream, as 0.3.1's note 1 lists
   (`PROVED_PORT_VERSION` and every `theseus-proved-port~` pin), once
   the 0.4.14 package is out: on an older kit, `make noalias` fails with
   `no LLVM IR` and `make lint-unsafe` refuses
   `--force-warn=unused_crate_dependencies`. Then run both targets on
   the new toolchain.

## 0.4.13

### Changes

- The reach check's manifest reader (`proved-port-seams -check` with
  `[reach]`) reads a TOML 1.1 multi-line inline table (#246). Its braces
  are tracked over its lines with an array's brackets, innermost last,
  so the lines inside it are the value's: its `}` line no longer fails
  as "neither a table header nor a key", and a key inside it (`path` in
  `x = {` .. `}` under `[lib]`, which is `lib.x.path`) is not read as
  the table's own. A line starting with `[` inside it fails, as inside
  an array. Three forms, none a valid manifest, now fail too: a `]` or
  `}` that closes a delimiter of the other kind (`x = [1}`), a `{` after
  a value closes (`x = { a = 1 } { b = 2 }`), and an inline table left
  open at the end of its line and the manifest (before, `x = { a = 1`
  alone passed). The two messages name both kinds ("an unmatched `]` or
  `}`", "a `[` or `{` after a value closes").
- The seam ledger generator's pins checks (#248), seams.sh's `pins()`
  and `copies()`: a rules line `pins<TAB>FILE` (each FILE once; a port
  may give several) makes `-check` hold FILE's
  `SRC<TAB>LINE[<TAB>NAME:ROLE]` rows in place, with seams.sh's
  messages. SRC's rows appear in it in order, each on the line after
  the one before (blank and `//` lines skipped, lines trimmed), a `...`
  row allowing any lines between, and the first line of each run occurs
  in SRC exactly once; for each NAME the `NAME:orig` rows equal the
  `NAME:copy` rows and each side's rows are consecutive. A FILE or SRC
  that is gone, and an overrides file that names FILE with no row for
  its source file, fail. Where seams.sh was lax, the check fails
  closed: a row of one field (seams.sh skipped it), an empty field, a
  SRC that is no clean path inside the tree, a `...` row that does not
  stand between two rows of its SRC, and a `...` row with a third field
  (seams.sh counted it as a row of its side, so a tagged `...` kept a
  side consecutive across a gap no row pins) fail, and so does a FILE
  with no rows (empty, or only comments), which holds nothing in place.
  In a local run at this commit (#249's description has the commands;
  no check repeats it), challoc 2e71814 passed `-check` with `pins
  verus/kani/pins.txt`, and on ten broken variants of its pins file and
  source the check printed what seams.sh's functions print. A port
  without a `pins` line sees no change.
- The seam ledger generator's mutant runner (`kit/verus/seams/mutants.sh`)
  names one occurrence of a repeated FROM (#250): FILE written
  `FILE#N/M` is the Nth of M occurrences (1 <= N <= M, M >= 2, each at
  most 9 digits with no leading zero), and the line fails unless FROM
  occurs exactly M times; only the Nth is replaced, and the result lines
  name `FILE#N/M`. The count catches an occurrence added or removed,
  not one moved, so a `FILE#N/M` row may not be marked equivalent (a
  mark names one place: widen FROM until it occurs once). A plain FILE
  still needs FROM exactly once. Any
  mutant that leaves its file unchanged (TO equal to FROM) now fails as
  a bad line, though marked equivalent. `testdata/mutants.txt` gains
  #218's hand mutants whose FROM occurs two to four times, at every
  occurrence (58 rows: 55 killed, 3 marked equivalent with their
  reasons, those three with FROMs widened to occur once), and two tests close the gaps they found: the paren form
  `#[verifier(assume_unsupported_attributes)]`, and the blanks a block
  comment leaves in an export's contract. Nothing is installed; ports
  see no change.
- The seam ledger generator resolves test-crate citations (#252),
  seams.sh's `loomtest()`: with a rules line
  `cite-test<TAB>TAG<TAB>DIR[<TAB>CFG...]`, `-check` resolves each TAG
  citation `MOD::TEST` to a top-level `fn TEST() {` in DIR/MOD.rs with
  `#[test]` and no `#[should_panic]` above it, whose `///` lines or body
  name the seam's function in backquotes, in a file that names the
  seam's source file in backquotes, with `loomtest()`'s matching. Where
  `loomtest()` was lax, the check fails closed: the test may carry no
  cfg, cfg_attr or ignore attribute but `#[cfg(CFG)]` for a CFG field,
  DIR/lib.rs must declare `mod MOD;` at its top level under no other
  gate and no `#[path]`, neither file may hold an inner cfg attribute
  but `#![cfg(CFG)]`, the test crate's manifest (DIR/../Cargo.toml,
  read as the reach check reads the main crate's) must keep DIR/lib.rs
  its root and tested (no `[lib] path` elsewhere, `autolib = false`,
  `test = false` or `harness = false`), the last attribute run after
  other code or an item's closing delimiter on its line (`}
  #[should_panic]`, `const C: u8 = 0; #[inline] const D: u8 = 0;
  #[ignore]`) attaches to the next item, the body ends at the delimiter
  that closes it (code or a comment after it on its line is not the
  test's), and lines in block comments or literals are not
  read. It also reads Rust's attachment where `loomtest()` dropped a
  test: attributes and doc lines in any order and across blank and
  comment lines. A CFG holding a literal (`feature = "x"`) is a rules
  error. The template's `seams.rules` carries challoc's line,
  commented. In a local run at this commit (#252's pull request has the
  commands; no check repeats it), challoc 2e71814 passed `-check` with
  `cite-test	loom	tests/loom/src	test` (25 loom citations, 13
  tests), and with one cited test made `#[should_panic]` both seams.sh
  and this check refused it. A port without a `cite-test` line sees no
  change.
- The seam ledger generator resolves cases-table citations (#254),
  seams.sh's `genmctest()`: with a rules line
  `cite-case<TAB>TAG<TAB>DIR`, `-check` resolves each TAG citation
  `CASE [MODEL ...]` against DIR/cases, read as challoc's
  `tests/genmc/run.sh` reads it (`name | template | models | constants
  | expected` lines, blank and `#` lines skipped, fields trimmed): the
  case expects `pass` alone (a `fail` case is a control), runs under
  exactly the cited models in that order, and its template
  DIR/TEMPLATE.rs names the seam's source file and function in
  backquotes, with `genmctest()`'s matching. Where `genmctest()` was
  lax, the check fails closed: a case named on two lines, a case line
  with an empty field or fewer than five fields, a template that is not
  a word, and a template that ends inside a block comment or literal
  are refused, and a citation must write one blank between its words.
  A tag takes one of cite-fn, cite-assert, cite-test and cite-case (the
  rules error for a second one now names cite-case too). The template's
  `seams.rules` carries challoc's line, commented. In a local run at
  this commit (#254's pull request has the commands; no check repeats
  it), challoc 2e71814 passed `-check` with
  `cite-case	GenMC	tests/genmc` (25 GenMC citations), and with one cited case turned
  into a `fail` case both seams.sh and this check refused the same 5
  citations. A port without a `cite-case` line sees no change.
- The package installs a fourth Go tool, `proved-port-calls`
  (`kit/checks/calls`, #256), the first half of PLAN step 8's lint that
  a port's checks call the kit's targets. It reads no check file: its
  input is the resolved configuration `chainctl checks explain
  --output=json .` prints, and a rules file of `call CHECK TARGET...
  [with NAME...]` lines (the check exists, gates a merge, installs
  theseus-proved-port, sets no variable but VERUS_THREADS (digits or the
  guest's CPUs), VERUS_BASE (one expansion of the base Checks sets) and
  the names `with` lists (to plain values or one expansion of a variable
  the check sets; never a name make, the shell, the loader, cargo or git
  reads, nor one of the kit's own, the Kani split knobs included),
  starts no services, and runs `make TARGET` in a step read with
  certainty, after only such steps: `make ARG...` or
  `sh -c` with exactly one simple command, options that run every goal,
  literal goals) and `forbid PATH` lines (no step, service or variable
  names an in-tree copy the kit replaced). `kit/install.sh` builds it with the other
  three, and `count-installed` checks it as it checks them; go.yaml's
  `calls-test` runs its tests, with challoc 7138852's resolved
  configuration (two values made neutral) as a fixture. The next item
  runs it in a port.
- `make kit-calls` (#258), the second half of PLAN step 8: the target
  writes `chainctl checks explain --output=json .` to a temporary file
  outside the tree (a failed explain fails the target, printing its
  diagnostics) and runs `proved-port-calls -rules $(KIT_CALLS_RULES)`
  over it (`KIT_CALLS_RULES ?= ci/kit-calls.rules`, `KIT_CALLS ?=
  proved-port-calls`; the target fails while either is empty). The
  templates gain `ci/kit-calls.rules` (call lines for verus,
  verus-main, kani, kani-main and source-rules, and forbid lines for
  the in-tree copies the kit's targets replaced), and lint.yaml's
  source-rules check runs `make kit-calls` after `make lint-unsafe`,
  with `chainctl~{CHAINCTL_RELEASE}` in its packages and in
  env/dev.yaml's (the deps template's tools entry moves the pin and
  runs `make kit-calls`). Two rules
  change in the checker: a call check with a `when.changed` list now
  fails its rule (a list that names none of the paths the target reads
  lets it pass unrun), and `with` refuses every name starting `KIT_`
  (KIT_CALLS_RULES chooses the rules). A check that splits Kani between
  instances stays no call check (the checker refuses every KANI_ name);
  the rules template says so and lists only the unsplit kani and
  kani-main. make-test covers the target with a stub chainctl and
  checker; go.yaml's `kit-calls-template` runs it with the real
  chainctl over the template's check files filled with sample values.
  calls-test's TestTemplateCalls requires a call line for each kit
  target each template check's steps name, over a fixture of explain's
  output (`kit/checks/calls/testdata/templates.json`), and
  `kit-calls-template` runs it over the real chainctl's output too.
- The templates gain `verus/VULN-CLASSES.md` (#264), a skeleton of the
  vulnerability-class catalog every port keeps: scope and the
  every-input rule (every argument of the exported functions a row
  names that meets the C caller's premise; a theorem on an inner
  function whose `requires` is not proved on every route in is
  `partial:`), what the construction gives and what it does not, how
  to read a row, the catalog table (`id | year | class | upstream |
  {PORT} counterpart | ruled out by | sources`; `by construction:`
  names its check, and a memory-safety row also names the theorem or
  gap for the panic left), a section per class (property, proved, by
  construction, partial or trusted, gap), the `vuln class:` gaps list,
  and keeping it current. It is a starting point the port reshapes;
  only the opt-in check below reads it, and `make verus-headers` does
  not hold it. The skill's survey step (references/lifecycle.md)
  starts the catalog from it.
- The seam ledger generator checks a vulnerability-class catalog in the
  template's layout (#266): with a rules line `vuln-classes<TAB>FILE`
  (at most once), `-check` reads the first table under FILE's `##
  Catalog` (its `class` and `ruled out by` columns found by header
  name, so other columns and their order are the port's), each `###`
  section under `## Classes` with its **Proved.** and **Gap.**
  paragraphs, and the items of `## Gaps`. It fails a theorem cited `ID
  (#n)` (ID starting with a capital letter and holding a digit) in a
  row's ruled out by cell or a **Proved.** paragraph with no
  `theorems/ID.md` beside FILE; a row's class with no section
  (compared with case folded); a `gap: #n` in a row or a **Gap.**
  paragraph with no item of Gaps naming `#n` on its first line, and an
  item no row or section cites so. A paragraph's lines are read joined,
  so a citation wrapped over two lines reads whole, and a line directly
  under the rows that starts no other block is a row, leading `|` or
  not, as GitHub renders it. So that a citation in another form fails
  rather than go unread, it also fails a ruled out by cell with no `ID
  (#n)`, no `gap: #n`, no `not shipped` and no `by construction:` (a
  `by construction:` row's named check, and whether a memory-safety
  row also names its panic's theorem or gap, stay true by review); a
  class section without its **Proved.** or **Gap.** paragraph (a
  misnamed `**Proved**.` included); a **Proved.** paragraph with no `ID
  (#n)` and a **Gap.** paragraph with no `gap: #n` that does not read
  "none"; and an id in code font or a link before ` (#n)`. An id
  written with no ` (#n)` of its own is not read. It fails as well a
  row whose cell count is not the header's, a row with no class, a
  second section of a class, a Gaps item naming no issue, a header
  without either column, a header with no separator row under it
  (whether a row, a heading, a fence or the end of the file follows; a
  header and separator with no rows pass), a second table under
  Catalog, and a FILE with no Catalog table or none at all. Whether a cited issue is still open
  needs the network and is not checked. The template catalog's row,
  **Proved.** and **Gap.** forms now say `ID (#n)` and `gap: #<n>` (or
  "none"), its "Keeping this current"
  names the check, and the template's `verus/seams.rules` holds the
  line commented. A port without the line sees no change.
- The counter's version is 0.4.13, ready for the v0.4.13 tag.

### Porting notes

1. Nothing to change. A manifest that passed 0.4.12's reach check passes
   this one unless it is one Cargo cannot read; one with a multi-line
   inline table, which failed, now passes when the rest of it does.
2. Optional: a port whose own script checks a pins file (challoc's
   `pins()` and `copies()` in `verus/seams.sh` over
   `verus/kani/pins.txt`) can add `pins<TAB>verus/kani/pins.txt` to
   `verus/seams.rules` and drop those functions, then run
   `proved-port-seams -check`: a row of one field, an empty field, a
   path that is not clean, or a `...` row first, last, doubled for its
   file or with a third field now fails and names the row, and a pins
   file with no rows fails. A port that does neither changes nothing.
3. Optional: a port whose own script resolves loom citations (challoc's
   `loomtest()` in `verus/seams.sh`) can add
   `cite-test<TAB>loom<TAB>tests/loom/src<TAB>test` to
   `verus/seams.rules` and drop that function. A cited test under
   `#[ignore]` or a cfg other than `#[cfg(test)]`, or in a file
   `tests/loom/src/lib.rs` does not declare ungated, now fails, as does
   a `tests/loom/Cargo.toml` that is missing, moves the library root or
   turns its tests off.
4. Optional: a port whose own script resolves GenMC citations (challoc's
   `genmctest()` in `verus/seams.sh`) can add
   `cite-case<TAB>GenMC<TAB>tests/genmc` to `verus/seams.rules` and drop
   that function. A case named on two lines of `tests/genmc/cases`, a
   cited case line with an empty field, or a template that ends inside
   a block comment or literal now fails, as does a citation with two
   blanks between its words.
5. The package's recipe builds `proved-port-calls` beside the other
   three Go tools (one more `go/build` of `kit/checks/calls` as
   `proved-port-calls`, or nothing more where the recipe runs
   `kit/install.sh`).
6. Optional: to run the lint, copy `templates/ci/kit-calls.rules` to
   `ci/kit-calls.rules` (a call line for each check of the port's that
   runs a kit target; drop the kani lines for a port without Kani, and
   the lines of a check that splits Kani), add the step `make
   kit-calls` after `make lint-unsafe` in lint.yaml's source-rules, and
   add `chainctl~{CHAINCTL_RELEASE}` to that check's packages and
   env/dev.yaml's, and the chainctl pin to the tools entry of
   .chainguard/deps.yaml. A check
   listed with a `when.changed` list fails its rule: drop the list or
   the line. A forbid line for a path where the port keeps a script of
   its own fails too: drop the line or rename the script. The template
   does not forbid `ci/lint-unsafe.sh`, where ports keep their own
   SAFETY: comment check. A port that does none of this changes
   nothing.
7. Nothing to change. A port without a `verus/VULN-CLASSES.md` starts
   one from `templates/verus/VULN-CLASSES.md`; a port that has one may
   keep its own layout.
8. Optional: a port whose catalog is in the template's layout (the
   `## Catalog` table with `class` and `ruled out by` columns, `###`
   class sections under `## Classes`, the `## Gaps` list, theorems
   cited `ID (#n)`) can add `vuln-classes<TAB>verus/VULN-CLASSES.md` to
   `verus/seams.rules`; `proved-port-seams -check` then fails each `ID
   (#n)` and `gap: #n` citation that does not resolve, a ruled out by
   cell citing neither form that is neither `not shipped` nor `by
   construction:`, and a class section whose **Proved.** or **Gap.**
   paragraph is missing or cites neither form (write "none" in a
   paragraph with nothing to cite). A port with another layout, or
   without the line, changes nothing.
9. After note 5, move the pin together, to 0.4.13, as 0.3.1's note 1
   lists, once the 0.4.13 package, the first with `proved-port-calls`,
   is out. Then, on the new kit, run `make verus-headers`: no file it
   holds changed in this release, so it reports no drift. Notes 2, 3,
   4, 6 and 8 are optional and may follow in later commits. A port
   moving from 0.4.11 or earlier applies the notes of each release in
   between first.

## 0.4.12

### Changes

- `make lint-unsafe` reads each member's crate roots from cargo
  metadata (#186). They are the files of its lib and bin targets,
  wherever they are, or, for a member with no lib and no bin target (one
  of benches or tests alone), the files of its test, bench and example
  targets. Each must forbid unsafe code as before
  (`#![forbid(unsafe_code)]`, or both lines of the Verus pair). Before,
  the roots were the files at cargo's default paths (`src/lib.rs`,
  `src/main.rs`, `src/bin/`), so a bench-only member failed with "has no
  crate root", a lib or bin at a path of its own (`[lib] path`,
  `[[bin]] path`) was not read (a member with only such roots failed the
  same way), and a file at a default path that cargo does not build
  (`autolib = false`, `autobins = false`) was read. The forbid check on
  the command line is unchanged; it already reached every target.
- The kit entry of the templated `.chainguard/deps.yaml` prepares a
  `lowered` line where a release's notes say a share its counting rule
  drops below FLOOR takes one (#188). Its prompt said FLOOR never gains
  a `lowered` line from the entry, while 0.4.5's notes 3 and 7 (#74,
  #68) say such a share takes one. Now the entry saves the old
  counter's `--list` and figures before it installs the new package,
  and, for such a release, checks against them that every newly
  trusted, outside or counted function is one the notes' rule
  describes; with no saved list it leaves FLOOR as it is. For a
  drop the rule alone explains, with nothing in the port's code or
  ledgers changed, it sets the figure to the share the check reports,
  adds a `lowered FIGURE P REASON` line naming the kit release and
  change by the kit's full name (`kit 0.4.5,
  chainguard-project-theseus/proved-port#74`), what now counts as
  trusted or outside and the counts before and after, and replaces
  README.md's Verus badge where `functions` or `lines` moves (the
  entry's paths gain `README.md` for that line alone, and the prompt
  says so where it limits changes to the paths). It leaves that
  bump a draft: the FLOOR header puts verifying more first, so a person
  chooses between the line and more proofs, and ledger rows the notes
  ask for are listed in the body for that person. Every other gate the
  old counter passed still leaves the bump a draft with nothing fixed.
  The other entries keep "never gains a `lowered` line".
  `kit/count/README.md`'s "Lowering FLOOR is not the way to adopt a
  counter change" names the exception.
- `make skip-summary` (#190, `skip-summary.sh`, installed beside
  perf-budget.sh): challoc's expected-skips check (`ci/skip-summary.sh`)
  as a kit target, in POSIX sh. A port whose gated tests print `SKIP
  <key> <reason>` and pass when they cannot run names the test log in
  `SKIP_LOG` and the manifest it is held to in `SKIP_MANIFEST` (both
  required). A key the manifest lacks fails (exit 1, "new skip (lost
  coverage)"); a manifest key that did not appear is a notice; a missing
  manifest fails and prints the one to commit; a missing or empty log,
  a log or manifest it cannot read, or a tool that fails on one, fails;
  and `CI_SKIP_UPDATE=1` writes the observed keys and passes. A markdown
  table of the skips goes to stdout. A `SKIP` after a libtest `test
  NAME ... ` prefix on the same line (the skip landed before that
  test's result) counts too (#195). It is no proof that the tests ran
  or passed: the caller still fails on the test step's own status. On
  the same inputs its output, exit status and written manifest are
  challoc's script's, but for four cases: such a prefixed `SKIP`, which
  challoc's did not count (so a new one passed there and fails here); a
  key that starts with `-`,
  which challoc's `grep` read as an option (so the key showed as new in
  the table, after a grep usage message) and the kit's matches; a log
  that cannot be read, which challoc's script read as empty (so it
  passed) and the kit's fails (an unreadable manifest fails in both;
  challoc's exits with sed's status, under pipefail); and an empty log,
  which challoc's passed and the kit's fails. No existing target or
  check changes.
- `make glibc-max` (#193, `glibc-max.sh`, installed beside
  perf-budget.sh): challoc's glibc ceiling check (`ci/glibc-max.sh`) as
  a kit target, in POSIX sh. A port that ships a glibc-linked library
  names it in `GLIBC_MAX_LIB` and the highest `GLIBC_` symbol version it
  may need in `GLIBC_MAX` (both required; 2.44 is Wolfi's glibc stream).
  It reads the library's version needs (`.gnu.version_r`) with
  binutils' `readelf -V`, prints them, and fails (exit 1) when the
  highest is above `GLIBC_MAX`, when there is none (not a glibc build),
  or when the library is missing or readelf cannot read it. A `GLIBC_`
  node the library defines (`.gnu.version_d`) is no need. An ABI marker
  need (`GLIBC_ABI_DT_RELR`, from `-z pack-relative-relocs`) needs the
  glibc release that added it (2.36), printed on a line of its own and
  held to `GLIBC_MAX` too; a marker the script's table lacks fails,
  naming it. On the same inputs its output and exit status are
  challoc's script's, but for five cases: the versions line has no
  trailing blank; a readelf failure prints `error: readelf -V cannot
  read SO` (challoc's exits with readelf's status and no line of its
  own); a `GLIBC_MAX` that is not a version (`2.44a`, empty) is a usage
  error (exit 2), where challoc's compared it with `sort -V` as given; a
  library that defines `GLIBC_` nodes, whose definitions challoc's
  counted as needs; and a library with an ABI marker need, which
  challoc's did not read. No existing target or check changes.
- The Kani marker (`#[cfg_attr(any(), discharged_by = "kani")]`) now
  discharges a shipped function outside `verus!`, as it does a trusted
  one inside. Before, the counter classed every function outside
  `verus!` as outside before it read the marker, so a port whose FFI
  layer is plain Rust that Kani checks could not count those checks.
  Such a function needs a `kani-*.md` record that names its file and
  quotes `fn NAME`, as any marked function does, or the run fails
  (exit 1), and with `[ledger]` its marker is refused, as any shipped
  one is. Since it has no contract to quote, the counter checks three
  more things, and each failure fails the run (exit 1):
  - its record names it in backquotes by its path: a method with its
    owner (`A::new`, `B<u8>::new`, `<A as From<u8>>::from`), a free
    function with its inline modules (`a::init`) or alone at its file's
    top (`init`);
  - no other marked function in its file has the same path (`cfg`
    twins do);
  - a record that covers it names, each in backquotes, the file and
    name of a `#[kani::proof]` harness of its crate, in that file and
    under `cfg`s `make kani` builds (not `cfg(test)`), whose body calls
    it by name and, for a method, mentions its owner.

  kit/count/README.md, "Two ways to discharge a seam", gives each rule
  and its limits: the harness check is by name, so a call of that name
  counts whoever defines it. REACH takes such a function as checked;
  the functions nested in its body stay outside, in REACH too.
  `exec_allows_no_decreases_clause` around it changes nothing, since
  the attribute covers no code outside `verus!`. One built only for
  Verus leaves the tally, as outside code built only for Verus does,
  and its marker discharges nothing. A crate whose functions are all
  outside `verus!`, marked or not, is not taken as one Verus checks,
  so it needs no `[package.metadata.verus] verify = true`.
- The `verus/FLOOR` template's header says `checked-functions` and
  `checked-lines` add the functions Kani discharges, not only the
  trusted ones.
- The `kani-TEMPLATE.md` record template, the `verus/README.md`
  template's "Kani" section, the templates README and the skill's
  `references/proofs.md` say how to discharge a function outside
  `verus!`: the harness, one record per source file
  (`kani-<crate>-<path under src/, / as ->.md`) covering every marked
  function in it, and the marker, with no seam row.
- `make known-failures` (#196, `known-failures.sh`, installed beside
  perf-budget.sh): challoc's known-failures check
  (`ci/known-failures.sh`) as a kit target, in POSIX sh. After a failed
  test run, a port names the log in `KNOWN_FAILURES_LOG` and the
  failures it may have in `KNOWN_FAILURES_LIST` (both required; `#`
  comments, and ` @GLOB` scopes an entry to the cells `CI_CELL` names).
  A failure is a libtest `test NAME ... FAILED` line, listed by its
  whole NAME, blanks included (a should_panic test's `tests::boom -
  should panic`, a doctest's `src/lib.rs - parse (line 12)`), or a name
  in libtest's last `failures:` block before its `test result: FAILED.`
  line, which also catches a FAILED that landed on a line of its own
  (one thread, with output libtest does not capture before it). The
  block gives the bare name, without a FAILED line's ` - should panic`:
  where a FAILED line names the test it adds nothing, so the whole name
  stays the key; a test named only in the block is keyed by its bare
  name. A block that names fewer tests than its result line's `M
  failed` fails the run. Or a key printed
  by the port's optional `KNOWN_FAILURES_EXTRA` script, run as `sh
  EXTRA LOG` (challoc's cgroup verdicts move there). Every failure listed
  passes, each as a warning line; an unlisted one fails (exit 1), as do
  no failure at all (a build or other step failed), a missing log or
  list, one it cannot read, a tool that fails on one, and an EXTRA that
  fails. A test binary that crashed (a signal or an abort: its `running
  N tests` line has no `test result:` line after it, or cargo reports a
  signal or an exit status that no `test result: FAILED.` line of that
  binary explains, as from exit-time teardown, a sanitizer, or a
  `harness = false` test that panicked) is a failure,
  `crashed: TARGET`, TARGET from cargo's `Running TARGET (...)` line
  (`crashed: tests/b.rs`; #200); the key is the target text alone, so
  in a workspace run the same target in two packages shares one key,
  as test names do. It does not check that every test
  group ran (one that never started prints nothing); a port that needs
  that checks it itself, as challoc's `ci/test-cell.sh` does. A key is the test's name
  alone, not its test binary (two files' `fn roundtrip()` share one
  key), and a listed test that fails for another reason passes as
  known; an entry in scope that did not fail is printed as a notice
  (`notice: listed but did not fail, drop it from LIST once fixed`),
  which challoc's did not print, and does not change the exit status.
  The log must be uncoloured: a `test result:` line that is neither
  `ok.` nor `FAILED.` (libtest's `--color=always`) fails the run.
  With challoc's OUTCOME parsing as EXTRA, its output and exit status
  are challoc's script's on the same inputs, but for the notices, the
  no-failure line, which names no cgroup programs ("no test did"); a
  missing log, which names it (challoc's gave the no-failure line); a
  missing list, which fails with a line of its own (challoc's exited
  with sed's status); a failing test whose name has a blank (a
  should_panic test or a doctest), and one whose FAILED is not on its
  test's line, neither of which challoc's counted, so an unlisted one
  beside a listed failure passed there and fails here; and a failures
  block naming fewer tests than its result line counts, which fails
  here. No existing target or check changes.
- `kit/count`: a `[crates]` member takes a new optional key,
  `skip_packages` (#198). With `skip_packages = true`, the source walk
  of that crate leaves out each directory below its `src/` that holds a
  Cargo.toml: another package's sources, which cargo builds as its own
  crate. Before, such a crate counted those packages' files as its own,
  so uutils' root `coreutils` crate, whose `src/uu/` holds every
  utility's package, counted almost every utility a second time.
  Every reader of the crate's sources leaves out the
  same directories: the walk of the working tree and of a `--base-rev`
  revision, the symlink check (a link in a left-out directory is no
  longer refused, since nothing reads through it) and the token-mint
  scan. A module declaration of the crate (a `#[path]`), a target's
  root file (by the path cargo names, so through a link too) or an
  `include!` of a string literal in a left-out directory fails the run,
  since rustc compiles that file as part of the crate. Any other
  `include!` in such a crate fails the run too, since the counter
  cannot resolve the file it names (a `concat!` of literals, another
  macro, a `macro_rules!` body's `include!($p)`), except
  `concat!(env!("OUT_DIR"), ...)` with string literals alone after
  `env!("OUT_DIR")` and no `..` component in them, a file of the
  build's output directory, while the crate defines no `macro_rules!`
  named `concat` or `env` and imports neither from outside `std`,
  `core` or `alloc` (`use dep::concat;`, `use crate::p as env;`), which
  would shadow the builtins, and no `#[macro_use] extern crate` (under
  a `cfg_attr` too) loads
  a crate but std's or Verus's (or one of those renamed) and no glob in
  the including file is of anything but a path at std, core or alloc
  or Verus's prelude (`use dep::*;`, `use super::*;`), either of which
  may bring in another. One inside a `macro_rules!` definition fails whatever its
  argument but that form, since rustc resolves its path from the file
  that invokes the macro. A `use` that imports `include` under any
  name (`use core::include as inc;`) fails too, since an invocation
  through it does not name `include`, and so does a bare `include`
  anywhere among a macro invocation's argument tokens, at any depth
  (`call!(include, "p")`, `call!([include] "p")`, which the macro may
  forward to an invocation), but for std's formatting, assertion and
  collection macros and `verus!` where the crate defines no
  `macro_rules!` of that name, no `use` brings one in from outside
  std, core or alloc or renames anything to it, and it is not invoked
  through another crate's path (`dep::vec!`), nor through std, core
  or alloc where the crate rebinds that name (`use dep as core;`,
  `use dep::core;`,
  `extern crate dep as std;`, `mod core`, a Cargo dependency renamed
  to it), by path or by import, nor where such a `#[macro_use] extern
  crate` or such a glob in the file may bring one in. In a `macro_rules!` body every
  `include` fails but the `OUT_DIR` form and a path through a module
  of that name (two joined `:`), since the body can splice an
  invocation from its input (`include!$g`); the raw `r#include`
  counts as `include` everywhere. `include_str!` and `include_bytes!` read no
  code and are not checked. One limit: a file reached only through a
  macro of another crate, or a proc macro, that wraps `include!` is not
  seen (no token of the invoking file names `include`). Each left-out directory must be read by a configured crate
  of its own (a member at that directory, or a crate of `dir` there),
  else the run fails, so its code counts somewhere, and so does a
  library or binary root that crate declares outside its own `src/`,
  which its walk does not read, or an `include!` of its files held as
  a crate with the key holds them, against all of its `src/`. As a
  backstop for every include form at once, each `*.rs` file under a
  left-out directory must be one that crate reads (under its `src/`)
  or one cargo builds apart (under `tests/`, `benches/`, `examples/`,
  or `build.rs`), else the run fails, since no row counts it; in a
  package nested under it that is itself a configured crate (`x/derive`,
  `x/fuzz`), a file that crate's walk reads or builds apart passes the
  same way, and any other of its files fails; its roots and includes
  are held as the counting crate's are, wherever it sits (under
  `tests/` too). A package under a left-out directory's (or a nested
  configured crate's) `tests/`, `benches/` or `examples/`
  (`x/tests/fixture`) may be a shipped crate's path dependency, so it
  must be a configured crate too, else the run fails. The run
  prints a line for each
  left-out directory, `crate NAME leaves out DIR (counted as crate
  OTHER)`.
  With `--base-rev`, turning the key on
  for a crate the base read is a narrowing and needs a `[[narrowed]]`
  line; a new member that sets it, or turning it off, is not. The key
  is off by default, so no port's count moves.
- `make header-check` (#203, `header-check.sh`, installed beside
  perf-budget.sh): the PCRE2 port's header constant check
  (`ci/header-check.sh`) as a kit target, in POSIX sh. A port names its
  public headers in `HEADER_CHECK_HEADERS` and the prefixes of the C
  library's macros in `HEADER_CHECK_PREFIXES` (both required: `FOO_
  REG_`); each header's `#define`s and `#undef`s under those prefixes
  must equal those of the file of its name in `HEADER_CHECK_UPSTREAM`
  (default `/usr/include`, which the C library's -dev package fills),
  continued lines joined as C splices them (blanks or a CR after the
  backslash too, nothing left at the join), comments blanked as C
  reads them (across lines, `//`, not inside a string or character
  literal) and blanks outside the literals squeezed. The lines are compared as a set, in any order, so the
  `#if`/`#ifdef` branch a line sits in is not seen: a header that
  swaps the values of two branches passes (a limit; closing it needs
  the preprocessor). A difference prints the unified diff and fails
  (exit 1). On the same inputs its output and exit status are the PCRE2
  port's script's, but for five cases: a continued definition's later
  lines, and the text between two comments on a line, are compared
  (that script read a definition's first line alone, and dropped
  everything from a line's first `/*` to its last `*/`); comments are
  read as C reads them, and `#undef` lines are compared; a
  header with no definition under the prefixes on either side fails
  (a prefix that matches nothing compares nothing), and so does a
  prefix with no definition in any header of the run (`PCRE2_ REGX_`, a
  misspelled `REG_`, beside one that matches); a missing header of
  the port's fails with a line of its own (that script failed in grep);
  and `#  define` and `#define` are the same line (that script kept the
  blank after `#`, so the two spellings differed and its diff's hunk
  offsets can differ from this one's). A prefix that is no C identifier
  is a usage error (exit 2). No existing target or check changes.
- `make abi-elf` (#205, `abi-elf.sh`, installed beside
  perf-budget.sh): the per-library ELF checks of the PCRE2 port's
  `ci/abi.sh` (its checks 1 to 3) as a kit target, in POSIX sh with
  binutils' readelf, reading the port's linker version script in place
  of the PCRE2 names written into that script. `make abi-elf
  ABI_ELF_LIB=SO ABI_ELF_MAP=MAP ABI_ELF_SONAME=NAME` checks that SO's
  SONAME is NAME; that each library it needs is one of `ABI_ELF_NEEDED`
  (by default `libc.so.6` and the x86_64 and aarch64 loaders); with
  `ABI_ELF_TLS_MAX` (bytes), that its thread-locals fit; that its
  version definitions (but the file's own BASE entry) are exactly MAP's
  nodes (names compared as text), each with MAP's parent; and that its
  exports (defined global, weak or unique dynamic symbols, an absolute
  one too, but each version node's own symbol) are exactly MAP's
  `global:` names, each at its node's version (a name in two nodes at
  both), each with exactly one default version (`@@`, the one a new
  link binds): at its node, or, for a name MAP lists in more than one
  node, at the node `ABI_ELF_DEFAULTS` gives it (`foo@V2`, written
  from the C library's own `foo@@V2`; the order of MAP's nodes does
  not set it, and a name in more than one node with no entry fails).
  MAP is read for the forms
  a C library's map uses (named nodes,
  `global:`/`local:`, one parent, `#` and `/* */` comments, each only
  outside the other); a wildcard in `global:`, an `extern` block, an
  anonymous node or a second parent fails naming it (exit 1). A failed
  check prints a `FAIL:` line and exits 1; usage exits 2. No existing
  target or check changes.
- `make kani-quotes` (#207, `kani-quotes.sh`, installed beside
  perf-budget.sh): each Kani record (`kani-*.md` in `KANI_QUOTES_DIR`,
  by default `verus/seams`) must still quote, in its section 1, the
  source it names. The source is the first backquoted `.rs` path before
  the record's first heading, looked up under each of
  `KANI_QUOTES_ROOTS` in turn (by default `.`; a vstd source tree for a
  record that quotes vstd's `std_specs/`). The quote is every fenced
  block of section 1, cut into chunks at blank lines and at each
  block's end; each chunk, read
  with blanks squeezed and whole-line `//` comments dropped, must be
  consecutive lines of the source read the same way, and end where its
  item's text does (its last line ends with `{`, `;` or `}`, or the
  source's next line opens with `{` or `;`, or the source ends), so a
  clause added after the quote's last one fails. A failed record prints
  the chunk and its first line the source does not hold (or the source
  line it stops before), and the run exits 1; so does a record with no
  source, a source under no root, no section 1 or fenced block, a quote
  of comments alone, and a directory with no record. A record
  `KANI_QUOTES_SKIP` names is still read: one that fails prints a
  `skip` line, and one that matches fails the run, so the list only
  shrinks; a name there that is no record is a usage error (exit 2).
  The Kani record template's section 1 asks for the clauses through
  the last, and its section 5 names the check. No existing target or
  check changes.
- `make deviations` (#209, `deviations.sh`, installed beside
  perf-budget.sh): the deviations ratchet, the per-block comparison of
  the PCRE2 port's conformance check. The port's run writes
  `DEVIATIONS_RESULTS`, a `PASS KEY` or `FAIL KEY` line per key (a
  pattern block); `DEVIATIONS_LIST` holds `KEY ABI-nnn: reason` lines
  (`#` comments, blank lines); `DEVIATIONS_LEDGER` is the ABI ledger, a
  directory of `ABI-nnn.md` entries (by default `docs/abi`, the kit
  template's) or one file of `### ABI-nnn:` entries. Strict both ways
  (exit 1): a failing key that is not listed; a listed key that passes
  or is not a key of the run; a listed key with no reason, listed
  twice, whose reason does not start with `ABI-nnn:` or says nothing
  after it, or whose entry's Status is not `active`. Malformed results
  (another word, a key twice, a third field, a blank line, none) fail,
  and so does a malformed ledger (an entry with no `- **Status:**` line
  or more than one, fenced code not counting; a Status line outside
  any entry, or one in another form (`* **Status:**`, indented); a
  code fence never closed, or an indented one still open at a line
  indented less, an opener indented with a tab, or a closer indented
  otherwise than its opener; a line starting with raw HTML (`<!--`,
  `<details>`, at any indentation or after list or quote markers)
  outside a fence; a run of three backticks or tildes on a line that
  is not a fence line (`- ```` included); an ID with
  two entry headings; in the directory form, an
  `ABI-*.md` file not named `ABI-<digits>.md` or not
  holding exactly its own ID's heading), before anything is checked or
  updated. `DEVIATIONS_UPDATE=1` drops the listed keys that pass or are
  gone, keeping comments, blank lines and every other line, then
  checks; it never adds a line, and any other non-blank value fails the
  target. Each kind prints at most 50 lines; `DEVIATIONS_OUT`
  names a directory for the full lists. No existing target or check
  changes.
- `make known-cites` (#211, `known-cites.sh`, installed beside
  deviations.sh): the citation check of the PCRE2 port's differential
  fuzzer. Each row of the known-divergence table `KNOWN_CITES_FILE`, a
  line whose first non-blank is `KNOWN("<cite>"` (`KNOWN_CITES_MACRO`
  names another macro, `known!` say), the cite one string literal then
  `,` or `)`, cites `ABI-nnn`, an active entry
  of `KNOWN_CITES_LEDGER` (by default `docs/abi`), or `TODO(#n)`, an
  issue of `KNOWN_CITES_REPO` (`owner/name`, then any names it had
  before it moved) that is not closed: by its live state where `gh`
  reaches the repository (there an issue gh cannot view, no such
  issue, fails too; where gh is missing or cannot reach it, the run
  fails naming the issues it could not check, unless
  `KNOWN_CITES_OFFLINE=1` makes that a NOTE for a local run), by the
  pull request's description in `KNOWN_CITES_PR_BODY` where the port's
  check binds it, or by a commit since the one from which the table
  has held the cite without a gap, in a full clone (a NOTE says when it is
  skipped). Any other cite fails, as does any use of the macro but a
  row at a line's start and the name a `#define` defines or an `#undef`
  undefines (another form, the `(` on a later line or after a comment,
  `[` or `{` for a Rust macro, blanks around its `!`, a second row on
  a line, an alias `#define` or Rust `use known as k`; a carriage
  return counts as a blank), a
  table with no row and a malformed ledger. The ledger reader moves
  out of deviations.sh into `ledger-active.sh`, installed beside both,
  so the two read a ledger the same way; `make deviations`' behaviour
  and output do not change. No existing target or check changes.
- `make consumers-run` (#213, `consumers.sh`, installed beside
  known-cites.sh): the consumer harness of the PCRE2 port, its runner
  and its `fetch` and `loaded` helpers, with the build, the swap
  directory and the version check left to the port. `consumers.sh run
  LIB [NAME...]` runs each `CONSUMERS_DIR/<name>/run.sh` (by default
  `tests/consumers`; every one, or `$CONSUMER`, when no name is given)
  against the swap directory `CONSUMERS_LIB` in an empty
  `CONSUMERS_WORK/<name>/` (by default `target/consumers`), after
  installing the Wolfi packages its `packages` file lists, and prints
  `PASS|FAIL|SKIP <name>` and a summary: a consumer passes when run.sh
  exits 0 (77 is SKIP), proved with `loaded` that the port's library
  was loaded, and reported only failure ids its
  `expected-failures.txt` lists (a listed id that passed is a NOTICE).
  run.sh gets `P_LIB`, `P_ROOT`, `P_WORK`, `P_FAILURES`, `P_CACHE`
  and `P_HARNESS`, with P `CONSUMERS_PREFIX` (default `CONSUMER`), so
  a port keeps its run.sh files' names. `fetch URL SHA256 [FILE]` is
  the cached, SHA-256-checked download (without curl a download fails
  saying so; the runner installs curl when missing where apk runs as
  root, and goes on without it otherwise); `loaded [--lib SONAME]...
  PROGRAM [ARG...]` passes only if the loader initialized each soname
  (default `CONSUMERS_SONAME`) from a file directly in the swap
  directory and from nowhere else. Unlike the port's, a consumer with no
  run.sh always fails (the port's list of planned consumers stays in
  the port), a library in a subdirectory of the swap directory does not
  count, a swap directory with an entry that resolves outside it (a
  link to the system's copy, which the loader would name by the link)
  or a dangling link is refused, a consumer whose failures file is gone
  or whose `expected-failures.txt` is no readable file fails, and a
  swap directory or work directory with a blank, `:` or
  quote in its path is refused. The target is `consumers-run`, so a
  port's own `consumers` target can call it. No existing target or
  check changes.
- The differential fuzzer's generic driver shell (#215,
  `fuzz-diff.h` and `fuzz-diff.c`, installed beside consumers.sh): the
  PCRE2 port's `fuzz/diff/driver.c` without its decoding, comparisons,
  version checks and known-divergence table, which stay in the port. A
  port compiles `fuzz-diff.c` into its libFuzzer driver (`-I` the
  installed `make/` directory). `ppd_open` loads a library by absolute
  path with `RTLD_LOCAL`; `ppd_sym` resolves a symbol and refuses one
  that is missing or lies outside that library's own object;
  `ppd_distinct` refuses two libraries that are one object; `ppd_ready`
  ends the preflight under the run modes `P_PREFLIGHT` (exit 0) and
  `P_CONTINUE` (count findings, exit 1 at the end), with P the prefix
  the port gives `ppd_init`; `ppd_known` names a known class or skipped
  call once per process and appends it to `P_KNOWN_LOG`;
  `ppd_divergence`, `ppd_put_units` (8, 16 or 32 bits) and
  `ppd_finding_end` (the base64 reproducer, then `abort()`) write the
  report in the port's format. Unlike the port's, a symbol must resolve
  into its library's own object, not only into the same object as the
  library's other symbols (a wrapper whose symbols all come from a
  library it needs is refused, naming that library), known names are
  compared as strings rather than as pointers and kept as copies (a
  name may come from a reused buffer), a `P_KNOWN_LOG` that cannot be
  opened prints `<name>: cannot open <path>` on stderr (the port's
  skipped it silently), a known name of any length is logged (the
  port's 128-byte line buffer dropped one of 127 bytes or more), past
  64 distinct names each call prints and logs its name (the port's
  printed and logged nothing more once its 64 slots were full), an
  empty library path is refused as `<label>: no library path` (the
  port's said `<label>:  is not an absolute path`), and under
  `P_CONTINUE` the exit flushes stdout. No package recipe change (the package
  installs all of `kit/make/`); no existing target or check changes.
- The seam ledger generator's generic core (#217, `kit/verus/seams`,
  Go): challoc's `verus/seams.sh` scan, overrides, ledger files,
  totals, hash rules and claims nothing rules, with what is challoc's
  in a rules file the port keeps (`verus/seams.rules`; a commented
  starting point is `kit/templates/verus/seams.rules`). `-write`
  writes the ledger, `-check` fails where seams.sh's `--check` fails
  for those rules, and `-entries` prints one line per seam for the
  checks a port's statuses cite, which stay in the port (nine
  tab-separated fields, a backslash, newline, tab or carriage return
  in any field written `\\`, `\n`, `\t` or `\r`, so an
  unsupported-attribute contract's attribute lines stay on its line,
  #221). Unlike
  seams.sh, a `|` in an item's name is escaped in the ledger's table
  (seams.sh meant to, but BusyBox awk dropped the backslash), a source
  file's last line without a newline stays its own line, a
  `#[cfg(test)]` item whose body opens and closes on its first line
  ends there (seams.sh skipped on to the next line that opens a brace,
  dropping the seams between from the ledger), the source is read with
  block comments (nesting), strings and raw strings (`r"..."`,
  `r#"..."#`, `br`, `cr`) carried across lines, their contents no code
  (seams.sh read each line alone, did not know block comments or raw
  strings, and so could miss or invent a brace and swallow the seams
  after it; and it read `'\''`, a non-ASCII char literal and
  `'\u{10FFFF}'` wrongly, leaving a quote to open a false literal), a
  file the scan ends inside a string, raw string, block comment,
  attribute or item, or with its braces unbalanced, is an input error
  naming the line where that began (seams.sh went on and could leave
  seams out), an item on its
  attribute's line is read (`#[a] fn f()`, `#[cfg(test)] mod t;`; seams.sh
  took the whole line as the attribute), attribute paths split over
  lines or spaced (`verifier :: external_body`, `cfg( test )`) are
  matched, a trusting attribute on an item no kind reads (a macro-written
  fn name, a lowercase const or static, a union) is an input error
  naming the file and line (seams.sh left the seam out), a `discharged`
  or partly checked status that holds no cite tag fails `-check` (as
  seams.sh's citation checks did; resolving the citations stays in the
  port), a hash leaves out block comments as well as `//` ones (seams.sh
  kept a block comment's text) and keeps the
  whitespace inside string and char literals, raw strings included,
  while a quote in a comment opens none (seams.sh made every
  whitespace run one space, so `"a  b"` edited to `"a b"` kept a
  checked status's hashes; challoc's recorded hashes at deda0db are
  unchanged), a fn under `#[verifier::external_fn_specification]` (the
  older form of `assume_specification`) is an `assume_spec` seam named
  by the fn (seams.sh did not read it), the paren form
  `#[verifier(external_body)]` is read as external_body, a second
  overrides line for one name is refused (seams.sh kept the last), and
  a rules file is refused when a single-valued rule repeats or a path
  leaves the tree. The package does not install it yet, so no port changes
  anything; the go.yaml check `seams-test` runs its tests.
- The seam ledger generator's export macro rule, generalised (#219):
  `export-macro MACRO FILE FORM VAR...` reads the `slash` form
  (`fn a / b(..)`, challoc's; `pair MACRO FILE V1 V2` stays and means
  `export-macro MACRO FILE slash V1 V2`) or the `list` form
  (`fn body [a, b, c] (..)`, its names on one line or over several up
  to the `]`, the declaration's own attribute lines in every name's
  contract either way), each name taking the attributes above `fn $VAR(` for
  one VAR, or for the VAR in its place when there is one per name. An
  invocation may name the macro by path (`width::exports! {`), with
  blanks or a comment before its `!`. An export's attributes are the
  attribute lines directly above its template, the macro's file read
  with the lexer (lines of comments only, block comments over several
  lines included, skipped; a comment inside or after an attribute
  dropped from its text; a bracket or `fn $VAR(` in a comment or
  literal no code; any other line starts them again, where the pair
  rule read raw lines and only restarted at a `pub extern "C" fn`
  line), and its item line keeps the template's qualifiers
  (`pub unsafe extern "C" fn NAME (MACRO!)`; the pair rule always
  wrote `pub extern "C"`). Fail closed: a `fn` line directly inside an
  invocation that the form does not read, names that fit neither one
  VAR nor one per name, a list the invocation ends inside, a nested
  invocation, any other use of the macro (another delimiter, an
  expression, names on the invocation's own line, a name and its `!`
  split over lines, found in each file's tokens), a renamed import
  (`use w::MACRO as m;`, whose invocations go by `m!`, also split over
  lines), a `macro_rules! MACRO` outside FILE or a second one in it
  (its exports would take FILE's template), code after the
  definition's closing brace on its line (`} MACRO! {`, which would
  read as part of the definition), a second `fn $VAR(` template for
  one VAR in the definition (another arm, `fn $VAR (` with blanks or
  its head split over lines included; a lone template so split cannot
  be read and is refused too; the template search also stops at the definition's
  close, a definition on one line included), and an invocation
  whose tokens do not read as the form's declarations (`#[..]`*, `fn`
  and the names, then a rest to a `;` or through a `{..}` body) or
  name other exports than its lines gave (a `fn` split from its names,
  two declarations on a line, a grammar with no `fn`, forwarded
  tokens) are input errors (the pair rule passed over each, leaving
  its exports out). challoc's ledgers at deda0db are unchanged, and
  the parity run on its 30 older trees gives #218's results line for
  line.
- The seam ledger generator skips a `#[cfg(test)]` item whose body
  opens on a later line (#227): after a first line that starts an
  item with a body (fn, impl, mod, trait, struct, enum, union,
  macro_rules!) and does not end in `;`, it waits for the body, as it
  did only after a `fn` signature (`mod t` then `{`, or an impl with a
  `where` line, were counted as the crate's; seams.sh had the same
  limit). Any other first line (a field, variant, parameter or match
  arm) is still the whole item, its continuation read as code, and a
  line that
  closes more braces than it opens ends the pending item and is read
  as code (`#[cfg(test)]` / `b: u8 }` left the depth one too high and
  failed the file). challoc's ledgers are unchanged.
- The seam ledger generator's mutant runner (#225):
  `kit/verus/seams/mutants.sh` applies each hand mutant in
  `kit/verus/seams/testdata/mutants.txt` (a file, text that occurs once
  in it, its replacement, and an optional `equivalent: REASON`) to a
  copy of `kit/` and runs the generator's tests; a survivor not marked
  equivalent, a marked one that is killed, or text that no longer
  occurs once fails. The go.yaml check `seams-mutants` runs it after
  `kit/seams-mutants-test.sh`. Nothing is installed; ports see no
  change.
- The seam ledger generator's citations (#229): `seams -cites` prints
  one line per citation a status makes (overrides file, source file,
  line, the overrides line's name, tag, and the check as written,
  escaped as `-entries` escapes), parsed as seams.sh's `cites()` parsed
  them, so a port's citation checks read these lines in place of its own
  parse of the statuses. `-check` also fails a `TAG: ` that names
  nothing (`Kani: ;`, seams.sh's `none`), on any status, and a
  discharged status that cites a tag the rules mark `partial`
  (`cite-tag TAG partial`, a check that covers only part of a contract:
  challoc's loom and GenMC, its `loomdischarged` and `genmcdischarged`);
  the template rules mark loom and GenMC so. A `cite-tag` line with a
  third field other than `partial`, a tag given twice, or a tag holding
  `;`, `)` or `:` is a rules error. On challoc's overrides at 7d10fca,
  `-cites` gives seams.sh's `cites()` lines (its forms put together)
  and `-check` passes. Nothing is installed; ports see no change.
- The seam ledger generator resolves citations (#231). `-check` fails
  each citation of a tag that does not resolve, when the rules give the
  tag one of these lines:
  - `cite-fn TAG ATTR [CFG...]`: the citation is a path to a fn with
    the attribute line `#[ATTR]`. The fn is found in the path's file and
    its inline mods, by rustfmt's indentation, with no cfg, cfg_attr or
    ignore attribute on it but `#[cfg(CFG)]`. `cite-module NAME DIR` reads paths that
    start `NAME::` under DIR.
  - `cite-assert TAG`: one top-level labeled `const _: () = assert!(`
    with no attribute above it, in a `.rs` file of the crate.
  - `cite-row TAG FILE HEADING PART...`: a row in a table, checked after
    the tag's first check passes.

  These are seams.sh's `harness()`, `assertion()` and `guardrow()`, as
  its `checkcite()` used them for Kani, callers and const assert. The
  template rules give Kani `cite-fn Kani kani::proof kani` and
  `cite-module kani_proofs src/kani`, give const assert `cite-assert`,
  and carry challoc's callers lines as a comment.

  On challoc 7d10fca the results match `checkcite()` on its 151
  distinct Kani, callers and const assert citations and on 634 altered
  ones (a renamed fn, a dropped or added
  mod, `kani::` for `kani_proofs::`, a renamed label, the wrong file, a
  Kani harness cited as a guard test). Where the two differ, it is by
  design:
  - Files are read with the lexer. An attribute, mod, fn, label or
    assertion inside a block comment or literal counts for nothing,
    where seams.sh read it, and a file that ends inside one resolves
    nothing.
  - A trailing `//` comment after `#[ATTR]` is accepted.
  - Comment lines at any indentation, and block comment lines, keep the
    attribute. seams.sh kept it only across `//` lines at the fn's own
    indentation.
  - A cite-fn fn under `#[cfg(..)]`, `#[cfg_attr(..)]` or `#[ignore]`
    is refused as gated, since no build may compile or run it, as a
    gated assertion already was. The failure says the fn is gated, not
    that it is missing. Which attributes count:
    - every attribute Rust applies to the fn: any above it, up to the
      previous line of other code, across blank lines, comment lines
      and indentation;
    - the attribute anywhere on its line, or wrapped by rustfmt over
      lines up to its closing `]`, with or without blanks between `#`
      and `[` (two tokens in Rust's grammar), or a lone `#` line with
      the `[..]` on the next line of code. The seam scanner reads
      `# [..]` too since #233.

    The tag's cite-fn line may allow `#[cfg(CFG)]`, whole on its line and
    as written (`feature = "std"` matches only that literal), for each
    CFG field after ATTR: the cfg the tool builds under. The
    template allows `kani` for Kani and `test` for challoc's commented
    callers line. seams.sh accepted all of these. A cfg on an enclosing
    mod, or on the file's `mod` line, is not read (challoc's Kani mods
    are `cfg(kani)`, and its guard tests sit in `cfg(test)` mods).
  - A const assert LABEL that is blank is refused; it would match any
    `// ` comment.
  - An assertion is refused when the last line of code above its
    label's comment block is an attribute, even across block comments
    and comment lines. seams.sh looked only at the last nonblank line,
    so a comment between the attribute and the label hid it.
  - An assertion's FILE must be a `.rs` file under SOURCE or a
    cite-module DIR, and not a test file, so that a build compiles it.
    seams.sh read any file. Since #237 FILE must also be reached
    ungated from the crate root (see that bullet).
  - `cite-module` applies to every cite-fn tag (seams.sh read
    `kani_proofs::` only for Kani).
  - Each failure names the seam.

  The kit's challoc golden tree holds only nine of challoc's sources, so
  `testdata/challoc.rules` gains no cite-fn lines. Nothing is installed;
  ports see no change.
- The seam ledger generator reads an attribute written with blanks
  between `#`, `!` and `[` (#233). These are separate tokens in Rust's
  grammar, so `# [verifier::external_body]` compiles and Verus takes the
  item on trust. The scan used to read that line as plain code, which
  left its seam out of the ledger silently; seams.sh had the same limit.
  Now it is an attribute line everywhere the scan reads attributes:
  - a trusting attribute, which makes a seam;
  - an unsupported-attribute item's contract lines;
  - an export macro template's attribute lines.

  `#[cfg(test)]` already read blanks there. A line whose code ends in
  `#` or `#!` with its `[` on a later line is an input error naming the
  line, because the scan does not join the two. That covers the `#`
  alone on its line, after an attribute (`#[a] #`, also where a wrapped
  one closes) or after other code. A raw string's closing `"#` is not
  such a line. rustfmt writes neither form:
  challoc's ledgers (7d10fca and 11 older trees) and a second port's 684
  `-entries` lines are unchanged. Nothing is installed; ports see no
  change.
- The seam ledger generator's not-Copy checks of tracked types (#235),
  seams.sh's `trackedcheck()` and `notcopycheck()`. The rules line
  `notcopy FILE TAG` names the file of the checks and the cite-assert
  tag that tracked-type seams cite them with. Then `-check` fails:
  - each `tracked struct` or `tracked enum` in a source or test file,
    seam or not, with no check in FILE under its label that it is
    neither Copy nor Clone (`<T as NotCopy<_>>::HOLDS` and `<T as
    NotClone<_>>::HOLDS`), or with one gated by an attribute;
  - each second tracked type of a name already seen;
  - each tracked-type seam whose TAG citations hold no such check of
    its type.

  A check counts only in a FILE that also holds the second impls
  (`impl<T: Copy> NotCopy<..> for T {}` and its Clone twin), neither
  under an attribute, since a cfg the build does not meet would let
  every check compile for Copy types too. Where this differs from
  seams.sh, it is stricter:
  - Files are read with the lexer, so a declaration, check or impl
    inside a comment or literal counts for nothing. seams.sh read them,
    and dropped only `//` comments from a check's text.
  - A `tracked struct`/`enum` counts anywhere on its code line, after
    attributes on that line too. seams.sh required indentation and a
    visibility alone before it.
  - A gated impl is refused; seams.sh accepted it.
  - Every declaration on a line is read, not only the first.
  - A declaration whose name is not an identifier is an error, since no
    check can be held to it by name: a macro's `tracked struct $n`, or
    the name on a later line. seams.sh skipped it.

  A const assertion checks a generic type only at the arguments it
  names. The doc and the template comment tell a port to write it at
  Copy arguments (`K<u8>`). Since #237 FILE must also be reached
  ungated from the crate root.

  The template carries challoc's line as a comment.

  On challoc 7d10fca, `-check` with the line passes. I removed each of
  its 60 tracked types' checks in turn, then the NotCopy impl: the
  failing types equal `trackedcheck()`'s every time, and the two
  tracked-type seams it fails equal `seams.sh --check`'s
  `notcopycheck` lines. Nothing is installed; ports see no change.
- A cited assertion's FILE and the notcopy FILE must be compiled in
  every build (#237). `-check` fails a const assert citation, or the
  notcopy line once, when any file from FILE up to the crate root
  (SOURCE/lib.rs or SOURCE/main.rs) holds an inner cfg attribute:
  `#![cfg(..)]`, or a `#![cfg_attr(..)]` that adds a cfg. One that adds
  a lint level is harmless. It also fails when one of those files is
  not declared by a `mod NAME;` line in its parent (lib.rs or main.rs
  for a file directly under SOURCE, else `DIR.rs` or `DIR/mod.rs`, a
  `mod.rs` standing for its directory), or is declared under a cfg,
  cfg_attr or ignore attribute (wrapped, on the line, or across comment
  and blank lines) or under `#[path]`, which the check does not follow.
  Only a `mod NAME;` at the parent's top level counts. One inside any
  delimiters is reported as not reached: a `cfg_if!`, a `macro_rules!`
  body, a macro's arguments in `(..)` or `[..]`, or an inline
  `mod outer { .. }`, which declares `DIR/outer/NAME.rs`. An attribute
  after other code on the line before (`mod a; #[cfg(..)]`) attaches to
  the `mod` line, as in Rust. An inner attribute counts only at the
  file's top level; one inside a block (`mod tests { #![cfg(test)] }`)
  belongs to that block. A `#` that ends a line, with its `[` on the
  next line of code, is read as one attribute with it, as Rust reads
  it.

  The crate root is SOURCE/lib.rs, or SOURCE/main.rs when there is no
  lib.rs. With both present, a library build compiles lib.rs's tree
  alone, so a file reached only from main.rs fails, and so does main.rs
  itself. The check reads only those default roots. When the package's
  Cargo.toml (beside SOURCE) moves the library (`[lib] path` other than
  lib.rs) or turns discovery off (`autolib = false`, `autobins =
  false`), the walk fails rather than credit a leftover default root
  that Cargo does not build. With main.rs as the root, any `[lib]` table
  fails too, since a library elsewhere never compiles main.rs's tree.
  So does any `[[bin]]` table, which may move the binary, gate it on
  `required-features`, or (edition 2015) turn discovery off. Comments are stripped. The target
  settings in a form the check does not read, a dotted key (`lib.path`,
  `package.autolib`) or a top-level inline table (`lib = {..}`,
  `bin = [..]`, `package = {..}`), fail as well. Keys and headers are
  compared with blanks, tabs and quotes removed, as TOML allows them
  (`path\t=`, `"path" =`, `["lib"]`). The manifest is read line by line,
  so it fails closed on what such a reader cannot follow:
  - a missing Cargo.toml beside SOURCE, or one with no `[package]`
    table (a workspace's virtual manifest);
  - a multi-line string (`"""`, `'''`);
  - a line starting with `[` that is not a whole header (a line of a
    multi-line value);
  - an escape in a key or header;
  - an absolute `[lib] path`.
  `DIR.rs` is no parent when it is a crate root or a `mod.rs`:
  lib.rs's `mod x;` declares `src/x.rs`, never `src/lib/x.rs`. An
  item's head split over lines (`pub` / `mod x;`, `mod` / `x;`) is read
  as one item, so the attributes above it still apply.

  A file under a cite-module DIR is held to a weaker rule: it is
  compiled when the tool's build runs (challoc's `src/kani` under
  `cfg(kani)`), and neither its inner attributes nor the `#[path]` line
  that reaches the DIR are read. `DIR.rs` gets the same exemption only
  outside SOURCE; under SOURCE it is an ordinary module file. A
  cite-module DIR that is SOURCE or holds it is a rules error, since it
  would exempt every file.

  The cited assertion itself (cite-assert, and the not-Copy checks and
  their second impls) must also stand at the file's top level, at depth
  0 of braces, brackets and parentheses. An unindented assertion inside
  a gated inline `mod inner { .. }` no longer counts. seams.sh read
  FILE alone. On challoc
  7d10fca every cited file is reached and `-check` passes. Nothing is
  installed; ports see no change.
- The reach check's manifest reader, two more fail-closed rules (#239).
  - A value's brackets are tracked over its lines. A line starting with
    `[` while an array is still open belongs to the value, even one that
    reads as a whole header (`["x"]`, a last element with no comma), and
    it fails rather than change the table. Brackets, and the `#` that
    starts a comment, are read outside quoted strings, so a `"]"` or
    `"#"` in a string cannot close the count early. A string that does
    not close on its line fails. So does a value still open at the end
    of the manifest, one with a `]` anywhere along a line that closes
    more than are open, or one with a `[` after it closes on its line,
    outside an inline table's braces (`1], [2,`; `x = [1] [2]`; but `x =
    { a = [1], b = [2] }` passes) (#241): none is a valid manifest, so
    Cargo builds nothing from it. Outside a value, a line that is
    neither a table header nor a key (an `=` outside quoted strings)
    fails too, such as a stray token. A key is read up to the first `=`
    outside quoted strings. A UTF-8 byte-order mark at the start of the
    manifest is dropped, as Cargo drops it, so the header or key behind
    it is read; one anywhere else fails.
  - Cargo's default roots are `src/lib.rs` and `src/main.rs` beside the
    manifest. A SOURCE root elsewhere (`source lib`) counts only when a
    `[lib] path` names it exactly.

  challoc passes unchanged. Nothing is installed; ports see no change.
- The package installs the kit's Go tools (#243): `proved-port-fncmp`
  (the codegen comparer, `kit/perf/codegen/fncmp`), `proved-port-stats`
  (the bench summaries, `kit/perf/stats`) and `proved-port-seams` (the
  seam ledger generator, `kit/verus/seams`), in `/usr/bin` beside
  `verus-count`. The names carry the kit's prefix because `fncmp`,
  `stats` and `seams` are too generic for `/usr/bin`. `kit/install.sh`
  builds and installs them the same way under `PREFIX/bin` (it now needs
  go, with `GOTOOLCHAIN=local`; it builds every tool before it touches
  PREFIX, and reads a relative `CARGO_TARGET_DIR` from the caller's
  directory for the Go builds too). `CODEGEN_FNCMP` is not held to KIT's
  release as `verus-count` is: a host with several `kit/install.sh`
  prefixes puts the pinned one's `bin/` first on PATH. `CODEGEN_FNCMP` (`make codegen`, `codegen-capture`,
  `codegen-compare`) defaults to `proved-port-fncmp` (it was `fncmp`),
  and the missing-comparer message says to install the kit. The seams
  rules template's `command` is `proved-port-seams -write` (it was
  `seams -write`). Each tool prints what it printed before; only the
  names change. `count-installed` installs `go-1.27` and checks that each
  binary is installed with mode 0755 and is its tool, and that a tool
  that does not build fails the install with an earlier one left as it
  was. The package's recipe is outside this repository: the 0.4.12
  package is the first that builds the three tools, so a port relying
  on them moves its pin to 0.4.12 once that package is out, as for any
  release.
- The counter's version is 0.4.12, ready for the v0.4.12 tag.

### Porting notes

1. Run `make lint-unsafe` after the bump. A port that kept a bench-only
   or test-only member out of it (or carried a local patch for it) can
   check it now: add `#![forbid(unsafe_code)]` to each of its test,
   bench and example roots. Two cases can newly fail, each naming the
   file: a lib or bin at a path of its own without the forbid (add it),
   and a member with no lib or bin target that still has a file at a
   default path cargo does not build, which was read in place of its
   test, bench and example roots (add the forbid to those). A port with
   neither changes nothing.
2. The deps template carries no `# kit:` header and `make verus-headers`
   does not hold it, so a port takes the change by hand: in its
   `.chainguard/deps.yaml` kit entry, add `"README.md"` to `paths`,
   take the template's text from "If the pin is already that release,
   change nothing." to "commit the pin moves before you run anything
   else;" (the old counter's saved view) and from "Make only the
   changes" to "goes in the body.", and replace its text from "verus/FLOOR never goes down" to "fix nothing
   in the port's code or ledgers," with the template's text from
   "verus/RLIMIT gains no" to the same words (the RLIMIT sentence moved
   ahead of the FLOOR one, so the template's range starts there). A port
   that leaves its prompt as it is changes nothing else: its kit bumps
   that cross such a counting-rule change stay drafts for a person, who
   writes the line.
3. Optional: a port with its own copy of challoc's `ci/skip-summary.sh`
   can call `make skip-summary SKIP_LOG=LOG SKIP_MANIFEST=MANIFEST` in
   its place and delete the copy, keeping its test step's own status as
   a separate failure (the target does not check that the tests ran or
   passed). A port that copied the no-kit guard in
   `kit/count/README.md` adds `skip-summary` to its
   `PROVED_PORT_TARGETS` when it starts to use it. A port that does
   neither changes nothing.
4. Optional: a port with its own copy of challoc's `ci/glibc-max.sh`
   can call `make glibc-max GLIBC_MAX_LIB=SO GLIBC_MAX=VERSION` in its
   place (its check environment keeps binutils for readelf) and delete
   the copy. A port that copied the no-kit guard in
   `kit/count/README.md` adds `glibc-max` to its `PROVED_PORT_TARGETS`
   when it starts to use it. A port that does neither changes nothing.
5. Every port, before moving its pin to 0.4.12: find the marker on
   shipped functions outside `verus!` (`grep -rn 'discharged_by *=
   *"kani"' --include=*.rs .`, then whether each hit sits inside a
   `verus!` block). Before, each such marker counted nothing; now each
   makes its function discharged. Without `[ledger]`, add a record for
   it (`kani-TEMPLATE.md`) that names it by its path and names a harness
   that calls it, or remove the marker, and with `[reach]`
   drop any REACH row that names it. With `[ledger]`, remove the
   marker: the ledger says what is discharged. A port with none sees
   nothing from this note.
6. Every port, with the pin bump: `make verus-headers` reports
   verus/FLOOR's header as drifted, since the template's "add the
   trusted functions Kani discharges" is now "add the functions Kani
   discharges". Take the kit's text there.
7. Optional: a port with its own copy of challoc's
   `ci/known-failures.sh` can call `make known-failures
   KNOWN_FAILURES_LOG=LOG KNOWN_FAILURES_LIST=LIST` in its place, moving
   any failure source of its own besides libtest's FAILED lines into a
   script it names in `KNOWN_FAILURES_EXTRA`, and delete the copy. It
   keeps its own check that every test group ran (challoc's
   `ci/test-cell.sh` counts `test result:` lines): the target sees a
   test binary that crashed (`crashed: TARGET`), not one that never
   started, provided its log holds cargo's stderr as well as stdout
   (`2>&1`, as challoc's ci/make.sh tees it): cargo writes its
   `Running` and exit-status lines there. Its list's keys name tests, not
   test binaries, and a notice now names each entry in scope that did
   not fail, to drop once its test is fixed. A port that copied the
   no-kit guard in `kit/count/README.md` adds `known-failures` to its
   `PROVED_PORT_TARGETS` when it starts to use it. A port that does
   neither changes nothing.
8. `skip_packages` is optional. A port whose crate holds other packages
   under its `src/` (uutils' root `coreutils` crate, with every
   utility's package under `src/uu/`) can count that crate by adding it
   as a member with `skip_packages = true`, and each directory it leaves
   out as a member of its own if it is not one already (uutils' root
   crate leaves out `src/uu/stdbuf`, so `stdbuf` becomes a member too);
   the run fails on a left-out directory no member reads, or whose
   member declares a library or binary root outside its `src/` or
   includes a file from outside it, and
   prints a
   `crate NAME leaves out DIR (counted as crate OTHER)` line for each
   one it does. A port that turns the key on
   for a member it already counts needs a `[[narrowed]]` line naming the
   change the run prints (`[crates] member "NAME" changes skip_packages
   from false to true`). A port that does neither changes nothing.
9. Optional: a port with its own copy of the PCRE2 port's
   `ci/header-check.sh` can call `make header-check
   HEADER_CHECK_HEADERS="include/foo.h ..." HEADER_CHECK_PREFIXES="FOO_
   ..."` in its place (its check environment keeps the C library's -dev
   package for the upstream headers) and delete the copy. A port that
   copied the no-kit guard in `kit/count/README.md` adds `header-check`
   to its `PROVED_PORT_TARGETS` when it starts to use it. A port that
   does neither changes nothing.
10. Optional: a port whose ABI check reads its library's SONAME, needs,
    thread-locals, version nodes and exports (the PCRE2 port's
    `ci/abi.sh` checks 1 to 3) can call `make abi-elf` once per library
    in their place, with binutils in that check's environment, and keep
    its own C tests and consumer checks. Where its map lists a name in
    more than one node, it sets `ABI_ELF_DEFAULTS` from the C library's
    `readelf --dyn-syms` (`foo@V2` for `foo@@V2`). A port that copied the no-kit
    guard in `kit/count/README.md` adds `abi-elf` to its
    `PROVED_PORT_TARGETS` when it starts to use it. A port that does
    neither changes nothing.
11. Optional: a port with Kani records can run `make kani-quotes` in a
    check (busybox is enough), with `KANI_QUOTES_ROOTS=". VSTD"` where a
    record quotes vstd (VSTD: the vstd source of the version the port
    pins, as cargo unpacks it). Its first run may fail on records whose
    source has moved on since they were written; bring each record's
    section 1 back to the source (and its harness, where the contract
    changed), or name it in `KANI_QUOTES_SKIP` until it is, and take the
    name out once it matches (the run fails while it stays). A port that
    copied the no-kit guard in `kit/count/README.md` adds `kani-quotes`
    to its `PROVED_PORT_TARGETS` when it starts to use it. A port that
    does neither changes nothing.
12. Optional: a port with a conformance ratchet (the PCRE2 port's
    `ci/conformance.sh`) can keep its run, have it write a `PASS KEY` /
    `FAIL KEY` line per block, and call `make deviations
    DEVIATIONS_RESULTS=... DEVIATIONS_LIST=...` (with
    `DEVIATIONS_LEDGER=docs/abi-changes.md` where its ledger is one
    file) in place of its own comparison, its selftest and its update
    step (`DEVIATIONS_UPDATE=1`). It keeps showing a new failure's
    block itself, from the full lists `DEVIATIONS_OUT` names. Its run
    must still fail on its own when it does not finish (each step's
    status checked, none lost in a pipeline): the target cannot tell a
    run cut short from a small one, so a short `DEVIATIONS_RESULTS`
    must never reach it. Some
    things a port's own ratchet may have let through now fail, so it
    checks them before it switches: a listed reason of `ABI-nnn:`
    alone, with nothing after it; a ledger entry with no
    `- **Status:**` line or more than one outside fenced code (an old
    line left behind); a Status line outside any entry, or in another
    form than `- **Status:**` at a line's start; a code fence never
    closed, an indented one still open at a line indented less, or a
    closer indented otherwise than its opener, or an opener indented
    with a tab; a line starting with raw HTML (`<!--`, `<details>`,
    also indented or after a list marker) outside a fence (an entry retired by commenting it out is marked
    `removed` instead, and a folded recipe goes unfolded); a run of
    three backticks or tildes on a line that is not a fence line (a
    recipe's fence starts on its own line, not after `- `); an ID with
    two
    entry headings in its ledger (a
    copied heading, a reused ID); and, in a `docs/abi/`
    directory, an `ABI-*.md` file not named `ABI-<digits>.md`, or not
    holding exactly one entry heading, its own ID's (as the template's
    README checks: `grep -m1 '^### ABI-' docs/abi/ABI-*.md`). A
    port that copied the no-kit guard in `kit/count/README.md` adds
    `deviations` to its `PROVED_PORT_TARGETS` when it starts to use
    it. A port that does neither changes nothing.
13. Optional: a port with a known-divergence table (the PCRE2 port's
    `fuzz/diff/known.c`) can call `make known-cites
    KNOWN_CITES_FILE=... KNOWN_CITES_REPO=...` (with
    `KNOWN_CITES_LEDGER=docs/abi-changes.md` where its ledger is one
    file, and the repository's earlier names after its own) in place of
    its own citation check, binding the pull request's description to
    `KNOWN_CITES_PR_BODY` as it did its own variable. The check's
    environment needs busybox, git for the history part (skipped with
    a NOTE where missing or shallow), and, where the table has a
    `TODO(#n)` row, gh with a credential that reaches the repository:
    without it the check fails (set `KNOWN_CITES_OFFLINE=1` only for a
    local run). Before it
    switches it checks what may now fail: a ledger the kit's reader
    finds malformed (note 12 lists the forms), a row whose cite is
    written another way than `ABI-nnn` or `TODO(#n)`, a `TODO(#n)`
    whose issue does not exist, and a use of the macro that is no row
    (a row split over lines, two on a line, the macro's name as the
    last word of a comment line, an `#ifdef` of it). A
    port that copied the no-kit guard in `kit/count/README.md` adds
    `known-cites` to its `PROVED_PORT_TARGETS` when it starts to use it.
    A port that does neither changes nothing.
14. Optional: a port with a consumer harness (the PCRE2 port's
    `ci/consumers.sh`) can keep its build, swap directory and version
    check and then call `make consumers-run CONSUMERS_LIB=<swap dir>
    CONSUMERS_SONAME=<default sonames> CONSUMERS_PREFIX=<its prefix>
    CONSUMERS_NAMES=$(NAME)` (or `sh consumers.sh run` with the same
    variables in the environment) in place of its runner and helpers;
    its run.sh files keep their variable names through the prefix. The
    check's environment needs busybox, glibc's loader for `loaded`, and
    curl for a consumer that calls `fetch` (the runner installs it when
    missing where apk runs as root, like the port's; where it cannot,
    the run goes on and only `fetch` fails, where the port's runner
    stopped). Before it switches it
    checks what may now fail: a planned consumer with no run.sh (it
    fails rather than skips; drop it from the check's instances until
    its run.sh lands), a run.sh that relies on a library loaded
    from a subdirectory of the swap directory, a swap directory filled
    with links to files outside it (copy with `cp -P`, which keeps the
    links between the port's own files), and a run.sh that removes its
    work directory's files. The proof file a passed
    `loaded` writes in the work directory is `.consumers-loaded`, and
    its line starts `loaded:`. A port that copied the no-kit guard in
    `kit/count/README.md` adds `consumers-run` to its
    `PROVED_PORT_TARGETS` when it starts to use it. A port that does
    neither changes nothing.
15. Optional: a port with a differential fuzzer (the PCRE2 port's
    `fuzz/diff/`) can build its driver on the kit's shell: compile
    `$(PROVED_PORT_MAKE)fuzz-diff.c` with `-I$(PROVED_PORT_MAKE)` beside
    its own sources, call `ppd_init` with its message name and
    environment prefix first, load each library with `ppd_open` and
    `ppd_sym` in place of its own `dlopen`/`dladdr` code, check the two
    with `ppd_distinct`, keep its version checks (with `ppd_die`) and
    banner, and end its initializer with `ppd_ready`; its report calls
    `ppd_divergence`, `ppd_put_units`, `ppd_known` and
    `ppd_finding_end`. With the same message name and prefix, its
    preflight messages, banner, reports and known log stay as they
    were, but for the cases the 0.4.12 change lists. Before it switches
    it checks that each library it loads exports its symbols itself.
    A port without a differential fuzzer changes nothing.
16. A port that runs `make codegen` (or `codegen-capture`,
    `codegen-compare`) with a comparer it built from the kit's tree
    drops that build and its `CODEGEN_FNCMP` setting: the default,
    `proved-port-fncmp`, is the package's (on a host without the
    package, `kit/install.sh` puts it in `PREFIX/bin`; put that on
    PATH). A port that set `CODEGEN_FNCMP` to another comparer keeps
    it. A port that relied on the old default `fncmp` on PATH either
    drops that binary or sets `CODEGEN_FNCMP = fncmp`. A port that
    builds stats or the seams generator from the kit's tree may run
    `proved-port-stats` and `proved-port-seams` instead; a port with a
    `verus/seams.rules` whose `command` is `seams -write` sets it to
    how it now runs the generator (`proved-port-seams -write`), which
    changes the command each ledger file's header names, so it
    rewrites its ledger (`proved-port-seams -write`) in the same
    commit. A port that uses none of these changes nothing.
17. After notes 2 and 5, move the pin together, to 0.4.12, as 0.3.1's
    note 1 lists, once the 0.4.12 package is out. Then, on the new
    kit, run `make verus-headers` (note 6 is the drift it reports),
    `make lint-unsafe` (note 1), and note 16 where it applies. A port
    moving from 0.4.10 or earlier applies the notes of each release in
    between first.

## 0.4.11

### Changes

- With a generated ledger (`[ledger]`), a function seam (`external_body`,
  `external_body proof`, `axiom`) at the `fn` line of a trusted function
  that `exec_allows_no_decreases_clause` covers no longer makes it
  discharged or partly checked (#169). The function counts unchecked
  whatever the seam's status, as the README's "Functions with no
  termination proof" already said (nothing checks that it ends), and
  the run warns once per line when the status says discharged or partly
  checked. This holds whether the attribute alone makes the function
  trusted or it is `external_body` too, and at a macro invocation's
  line for every function there when the attribute covers one. Before,
  such a seam, stale or hand-kept, moved the function into the
  discharged or partly checked share, and the stale-seam check did not
  fire since the function was still trusted.
- With `[ledger]`, the Kani marker (`discharged_by = "kani"`) on a
  function the attribute covers fails the run (exit 1, "FAIL discharged:
  ... the ledger, not the marker, says what is discharged"), as it does
  on any other function in that mode (#169). Before, the attribute made
  such a function trusted first, so its marker went unrefused.
- In every port, with or without `[ledger]`: candidate definitions of a
  macro (under `cfg`s the counter cannot settle) that differ only in
  whether the attribute covers an `external_body` function they write,
  or only in the Kani marker on a function the attribute covers, now
  count differently and fail the run (exit 1, "m! could be any of N
  definitions"), as candidates that differ in the marker on any other
  function already do (#169).
- `make perf-budget` and `make perf-budget-print` (#173,
  `perf-budget.sh` and its checker `perf-budget.rs`, installed beside
  codegen.sh): challoc's fast-path budget check (`scripts/perf-budget.sh`
  and `scripts/perf-budget.rs`) as kit targets. For each build
  `PERF_BUDGET_BUILDS` names (by default one, `default`, each with its
  table in `PERF_BUDGET_TABLE_<name>` and its own cargo flags in
  `PERF_BUDGET_FLAGS_<name>`), it builds the release library
  `PERF_BUDGET_LIB` in its own target directory and checks every fast
  path of each hot export the table names against its instruction,
  load, store, branch and cache-line counts, with no atomic RMW, fence,
  call, syscall or indirect jump in the export's body and no back edge.
  The checker is challoc's, changed only in its text, and reads the same
  table format, so challoc's tables stay as they are; `perf-budget-print`
  prints the measured lines (the checker's `--print`). The script builds
  the checker with `rustc -O` from the installed source, so the package
  recipe needs no change. Unlike challoc's script:
  - the library, the builds, their tables and flags are settings, not
    challoc's two quanta, and each build has its own target directory;
  - the self-test mutates the first `export` the table names (or
    `PERF_BUDGET_SELFTEST_EXPORT`) and the table's first `budget`,
    `paths` and named `cell` lines, not `malloc` and challoc's lines; a
    mutation that changes nothing fails the run, and with no cell a
    budget line names, the cell mutation is skipped, saying so;
  - the stale-cell mutation moves the cell 1 MiB into its static rather
    than to offset 0, and the path mutation rewrites the first count
    rather than deleting it, so neither can coincide with a real one;
  - a failed build fails the run (exit 1) with cargo's output, and a
    setting missing or malformed exits 2 before any build.

  It is a new target: no existing target or check changes. The no-kit
  guard in `kit/count/README.md` lists the two; a port that copied the
  guard adds them to its `PROVED_PORT_TARGETS` when it starts to use
  them.
- The counter's version is 0.4.11, ready for the v0.4.11 tag.

### Porting notes

1. Every port, before moving its pin to 0.4.11: find the macros with
   more than one definition that write functions with
   `exec_allows_no_decreases_clause` (`grep -rn
   exec_allows_no_decreases_clause --include=*.rs .`, then the
   `macro_rules!` around each hit), and make the definitions agree on
   `external_body` and on the Kani marker for each function the
   attribute covers. A port with no such macro sees nothing from this
   note.
2. A port with `[ledger]`, also before the bump: for each function the
   attribute covers (each hit above, and the functions under each
   `impl`, trait or module it is on), check that none has a discharged
   or partly checked function seam and none has the Kani marker. Such a
   seam now lowers the combined share by that function and warns: give
   the row an unchecked status (`trusted`), or prove the function's
   termination and drop the attribute. A marker fails the run: remove
   it.
3. Checked here: `verus-count .` over challoc 053c930 and 60db24a7
   (both with `[ledger]`) prints the same output, byte for byte, with
   the same exit status, at 0.4.11 and 0.4.10.
4. Nothing is required for `make perf-budget`. A port with a fast-path
   budget check like challoc's may replace its scripts with it: set
   `PERF_BUDGET_LIB`, `PERF_BUDGET_BUILDS` and a
   `PERF_BUDGET_TABLE_<name>` (and `PERF_BUDGET_FLAGS_<name>` where a
   build needs its own flags) before the include; give the check the
   kit pinned, `make` and `binutils` beside its pinned rust stream; and
   add the two targets to the no-kit guard's `PROVED_PORT_TARGETS` if
   the Makefile has one. challoc's settings are
   `PERF_BUDGET_BUILDS = q16 q8`,
   `PERF_BUDGET_TABLE_q16 = verus/perf-budget.txt`,
   `PERF_BUDGET_TABLE_q8 = verus/perf-budget-q8.txt`,
   `PERF_BUDGET_FLAGS_q8 = --no-default-features --features quantum8`
   and `PERF_BUDGET_CARGO_FLAGS = --locked`; with them, at challoc
   053c930 and rustc 1.99.1, both builds pass and the checker's output
   is the same as challoc's script's, line for line.
5. After notes 1 and 2, move the pin together, to 0.4.11, as 0.3.1's
   note 1 lists, then run `make verus-headers`. A port moving from 0.4.9
   or earlier applies the notes of each release in between first. No
   header or existing target changes.

## 0.4.10

### Changes

- With a generated ledger (`[ledger]`), a `no_decreases` seam at the
  `fn` line of a function that `exec_allows_no_decreases_clause` alone
  makes trusted is that function's ledger entry when no `external_body`
  or `axiom` seam is there (#166). A function trusted for another reason
  as well (`external_body`, an `assume`, or a shipped function under a
  `cfg` Verus's build might not meet) still warns without a seam of
  that reason, as does every function at a macro invocation's line when
  one of the functions it writes is. Candidate definitions of a macro
  that differ only in that (the attribute alone, or `external_body` too)
  now count differently and fail the run (exit 1), as candidates that
  differ in `assume_unsupported_attributes` already do. Since 0.4.5
  (#74) the
  counter printed "trusted function at FILE:LINE has no ledger entry"
  for each such function, though 0.4.5's porting note 3 asks for exactly
  that row; challoc's `verus/seams.sh` writes one per function and got
  16 warnings. The function still counts unchecked, whatever the
  status: nothing outside Verus checks that it ends. The figures do not
  change; only the warnings go.
- `make codegen`, `make codegen-capture` and `make codegen-compare`
  (#168, `codegen.sh`, installed beside noalias.sh): challoc's codegen
  checks (`scripts/codegen-diff.sh`, `ci/codegen.sh` and
  `ci/codegen-notices.sh`) as kit targets, comparing with the kit's
  fncmp (`kit/perf/codegen/fncmp`, #164) in place of challoc's
  `scripts/codegen-fncmp.py`. For each build `CODEGEN_BUILDS` names (by
  default one, `default`, each with its own cargo flags in
  `CODEGEN_FLAGS_<name>`, and `CODEGEN_CARGO_FLAGS` for all), it builds
  the release library `CODEGEN_LIB` in its own target directory and
  compares its machine code with the base's: the `.text` hash, the
  section sizes and, when the hash differs, fncmp's per-function report.
  It exits 2 when a function changed. `make codegen` captures the base
  revision (`CHAINGUARD_BASE_REVISION`, else `CODEGEN_BASE`) in a git
  worktree, compares the tree with it, and prints each build's verdicts
  as `::notice` lines; `codegen-capture` and `codegen-compare` do the
  same in two steps, around a local change, keeping the capture in
  `CODEGEN_DIR`. Unlike challoc's scripts:
  - the builds and the library are settings, not challoc's two quanta;
  - the base's capture builds in a target directory of its own, so an
    absolute `CARGO_TARGET_DIR` cannot leave the tree's build fresh
    from the base's and compare the base with itself;
  - a failed build fails the run (exit 1) with cargo's output, where
    challoc's piped it through `tail` and went on;
  - its report says `fncmp failed (exit N)` where challoc's names the
    Python script, and the uncommitted-changes note covers the whole
    tree, not only `src`, `Cargo.toml` and `build.rs`;
  - it fails closed (exit 1, no verdict): the `.text` hash fails when
    objcopy or sha256sum does, rather than compare two empty hashes as
    identical; each build removes the library an earlier run left
    (in `release/` or under a target triple) and takes the one it
    wrote; compare checks every build's capture, and compare and base
    look up the comparer, before any build; base checks that its
    revision resolves before the capture.

  The package does not install fncmp: a port builds it from the kit's
  `kit/perf/codegen/fncmp` with go at the tag it pins and names it in
  `CODEGEN_FNCMP` (by default `fncmp`, from PATH). It is a new target:
  no existing target or check changes. The no-kit guard in
  `kit/count/README.md` lists the three; a port that copied the guard
  adds them to its `PROVED_PORT_TARGETS` when it starts to use them.
- The counter's version is 0.4.10, ready for the v0.4.10 tag.

### Porting notes

1. Move the pin together, to 0.4.10, as 0.3.1's note 1 lists, then run
   `make verus-headers`. A port moving from 0.4.8 or earlier applies the
   notes of each release in between first. No figure, header or existing
   target changes, and with a generated ledger the counter prints fewer
   warnings (#166). One run can newly fail: a macro with candidate
   definitions under `cfg`s the counter cannot settle, where one writes
   a function with `exec_allows_no_decreases_clause` alone and another
   writes it `external_body` too. Before the bump, `grep -rn
   exec_allows_no_decreases_clause --include=*.rs .` finds the
   candidates to check; make them agree, or give the function the same
   attributes in each.
2. Nothing is required for `make codegen`. A port with codegen checks
   like challoc's may replace its scripts with it: set `CODEGEN_LIB`,
   and `CODEGEN_BUILDS` with a `CODEGEN_FLAGS_<name>` for each build
   that needs its own flags, before the include; give the checks the kit
   pinned, `make`, `go` (to build fncmp) and `binutils`; and add the
   three targets to the no-kit guard's `PROVED_PORT_TARGETS` if the
   Makefile has one.

## 0.4.9

### Changes

- `make noalias` (#159, `noalias.sh`, installed beside kani.sh):
  challoc's `heap-noalias` check (`ci/heap-noalias.sh`) as a kit
  target. For each build `NOALIAS_BUILDS` names (by default one,
  `default`, each with its own cargo flags in `NOALIAS_FLAGS_<name>`,
  and `NOALIAS_CARGO_FLAGS` for all), it builds the release library
  with `cargo rustc --release --lib -- --emit=llvm-ir -C
  no-prepopulate-passes` and LTO off, in its own target directory, and
  fails when an inherent method of `NOALIAS_TYPE` (the type, with its
  module path, `heap::Heap`) in the crate `NOALIAS_CRATE` takes a
  `noalias` `&mut self`, or when no method of the type with a
  `&mut self` is found. A `noalias readonly` `&self` is listed, not
  failed. Every run first scans challoc's saved `define` lines
  (`noalias.sh --self-test`). Unlike challoc's script:
  - it reads each symbol's type path through its backrefs, so the type
    must be `NOALIAS_TYPE` exactly. challoc's pattern also took a type of
    the same name in a module below, and missed an impl in a module that
    is not below the type's.
  - it handles a library that builds no cdylib or dylib, whose IR is
    `CRATE-HASH.ll`, a release profile with several codegen units (one
    `.ll` per unit), and a build with one or more target triples (IR
    under `TRIPLE/release/deps`), each target scanned on its own and
    required to pass on its own.
  - it builds at `-C opt-level=3` whatever the profile says: rustc
    marks parameters `noalias` only when it optimizes, so at opt-level 0
    a run would pass on IR that shows nothing. An opt-level in
    `RUSTFLAGS` or a config's rustflags comes after it and wins, so a
    build in which no method's `self` is `noundef` (which rustc adds only
    when it optimizes) fails as unoptimized.
  - a `self` passed by value (`dead_on_return noalias`, the caller's
    copy) is listed, not failed, as a `noalias readonly` `&self` is.
  - a rerun in the same target directory builds the library again
    rather than scan nothing.

  It is a new target: no existing target or check
  changes. The no-kit guard in `kit/count/README.md` lists it; a port
  that copied the guard adds `noalias` to its `PROVED_PORT_TARGETS`
  when it starts to use the target.
- The counter's version is 0.4.9, ready for the v0.4.9 tag. The counter
  is otherwise unchanged from 0.4.8.

### Porting notes

1. Move the pin together, to 0.4.9, as 0.3.1's note 1 lists, then run
   `make verus-headers`. A port moving from 0.4.7 or earlier applies the
   notes of each release in between first. Nothing else changes for a
   port that does not use `make noalias`: no counter output, header or
   existing target changes.
2. Nothing is required for `make noalias`. A port with a check like
   challoc's `heap-noalias` may replace its script with the target: set
   `NOALIAS_CRATE`, `NOALIAS_TYPE`, and `NOALIAS_BUILDS` with a
   `NOALIAS_FLAGS_<name>` for each build that needs its own flags, before
   the include. Have the check run `make noalias`, with the kit pinned in
   its environment and `make` in its packages, and add `noalias` to the
   no-kit guard's `PROVED_PORT_TARGETS` if the Makefile has one. The
   kit's target fails some runs the port's script passed, each a
   difference the Changes entry above describes:
   - a method in an impl in a module that is not below the type's is
     now read, so its `noalias` `&mut self` fails the run;
   - a type of the same name in another module no longer counts, so it
     no longer supplies the `&mut self` witness;
   - a build whose rustflags set opt-level 0 fails as unoptimized, and
     one whose profile sets it is checked at opt-level 3 all the same,
     where the script saw no `noalias` at all;
   - with several targets, each must pass on its own.

   A port that adopts it compares the two verdicts on its main in that
   pull request.

## 0.4.8

### Changes

- `make lint-unsafe` (#150, `lint-unsafe.sh`, installed beside kani.sh):
  the unsafe-code rule each port's `ci/lint-unsafe.sh` carries a copy
  of. Every workspace member but the packages `UNSAFE_CRATES` names (the
  C API crate, and any other fence crate) forbids unsafe code at each
  crate root (`#![forbid(unsafe_code)]`, or both lines of the Verus
  pair, read as inner attributes, so the text in a comment or a string
  does not count), and every target of each checks with `-Funsafe_code`
  on rustc's command line, through a `RUSTC_WORKSPACE_WRAPPER` that
  leaves the fence crates as they are (so another member may depend on
  one). With `UNSAFE_WORKSPACE_LINTS = yes`, the default, the root
  manifest also denies or forbids `unsafe_code` in
  `[workspace.lints.rust]` and every other member inherits it.
  `UNSAFE_MANIFEST` (by default `KANI_MANIFEST`) and
  `UNSAFE_CHECK_FLAGS` cover a workspace below the root and features.
  It is a new target: no existing target or check changes.
- REACH takes an export a `cfg_attr` applies as an entry point (#153):
  `no_mangle` or `export_name`, inside `unsafe(..)` or not, under
  `cfg_attr`s nested to any depth, where a shipped build can meet the
  predicate (a feature, in `verus_sets` or not, a platform). Under a
  predicate no shipped build meets (`any()`, a test atom, a Verus-only
  feature) it is not one. Before, only an export outside any `cfg_attr`
  was, so a `#[cfg_attr(feature = "..", unsafe(export_name = ..))]`
  function and what only it calls were never walked. An exported symbol
  whose ABI is not C's is an error under a `cfg_attr` too, and so is a
  `cfg_attr` predicate on an export that the counter cannot read.
- The templates' `source-rules` check (lint.yaml) runs `make lint-unsafe`
  in place of a `ci/lint-unsafe.sh` the kit never templated (#152). Its
  environment gains the rust stream, build-base, make and
  `theseus-proved-port~` (pinned as verus.yaml pins it), and the
  crates.io egress the clippy check has; review-prompt.md and the
  templates README cite the target, and the README shows the Makefile's
  `UNSAFE_CRATES = {CAPI_CRATE}`. The step checks no SAFETY: comments:
  the README tells a port to set clippy's `undocumented_unsafe_blocks`
  to deny where the C API crate's lints come from (the workspace's, or
  its own `[lints.clippy]` when it does not inherit them), so the clippy
  check holds them, or to keep its own SAFETY: check as a later step. A
  template change moves no port.
- The counter's version is 0.4.8, ready for the v0.4.8 tag.

### Porting notes

1. Move the pin together, to 0.4.8, as 0.3.1's note 1 lists, then run
   `make verus-headers`. A port moving from 0.4.6 or earlier applies the
   notes of each release in between first. Note 3 is the one change a
   bump can see: a port with an export under a `cfg_attr` may gain REACH
   rows. The counter built at 0.4.7 and at this release gave identical
   output (standard output, standard error, exit code, `--list`
   included) on the eleven ports the v0.4.8 release pull request names.
2. Nothing is required for `make lint-unsafe`. A port may replace the shared part of its
   `ci/lint-unsafe.sh` with `make lint-unsafe`: set `UNSAFE_CRATES` (and
   `UNSAFE_WORKSPACE_LINTS = no` if its manifests have no workspace
   lints) before the include, run `make lint-unsafe` from the script or
   the check, and keep only the rules of its own (a mint seam list, an
   unsafe budget, a SAFETY: comment per block) in the script. Paste
   both scripts' verdicts on the switching commit in the pull request.
   A port whose Makefile has the no-kit guard (kit/count/README.md,
   "Adopting: the package") adds `lint-unsafe` to its
   `PROVED_PORT_TARGETS`, so the target fails with the kit's message
   where the package is missing. A port that takes the template's
   `source-rules` check as it is also adds the template's environment
   and `network` to the check, and keeps its own rules as a step after
   `make lint-unsafe`. Its `SAFETY:` comment check stays such a step
   unless its C API crate already denies clippy's
   `undocumented_unsafe_blocks` (kit/templates/README.md says where).
3. A port with `[reach]` whose exports sit under a `cfg_attr` now has
   them as entry points: `verus-count` fails on each unverified function
   they newly reach (no row goes stale, since entry points are only
   added). List each one in REACH as a `new FILE NAME REASON` row (the
   verus check runs with `--base-rev`, which fails on a row the base's
   REACH lacks unless it is marked `new`), or verify it. The deps kit
   entry leaves such a bump to a person, as its prompt says of a new
   REACH row. Every port's run, `[reach]` or not, also fails
   on a function a shipped build exports through a `cfg_attr` whose ABI
   is not C's. A port with no export under a `cfg_attr` sees no change.

## 0.4.7

### Changes

- `[floor]` takes `room_functions` and `room_lines` (#100, default 0
  each): the `raise FLOOR to` figure a run prints is then the share
  with that many more unverified functions and lines, and a share above
  the floor but within that room prints no raise. Whether the run
  passes is unchanged. A port with neither key prints what it did. The
  room is a port's own choice: one that sets the keys states its room
  in a paragraph of its FLOOR below the header.
- The templates keep the `ABI-nnn` ledger one file per entry (#101):
  `kit/templates/docs/abi-changes.md` is now `docs/abi/README.md` (the
  entry format, the ID rules, and what to do when two pull requests
  take the same ID) and `docs/abi/ABI-001.md` (the first entry). The
  AGENTS.md, CONTRIBUTING.md, review-prompt.md, deps.yaml, fuzz.yaml and
  conformance.yaml templates and the skill's abi-compat.md cite the new
  paths. No counter or make target reads the ledger.
- `verus/count.toml` takes `[[runs]]` (#21): each proof run's `name`,
  and the atoms it is sure to set (`sets`) and never sets
  (`never_sets`). An item is classed in each run and verified where it
  is sure in some run and unsure in none, so a port that runs Verus on
  several hosts is counted run by run. `[crates] verus_sets` and
  `verus_never_sets` stay as one run's shorthand and cannot sit beside
  `[[runs]]`. A port with no `[[runs]]` counts as it did. Under
  `--base-rev`, a run added, removed or renamed, an atom added to a
  run, or `[[runs]]` replacing the shorthand, is a narrowing. A run may set only one value of each
  single-valued key (`target_arch`, `target_os`, ...).
- The counter's version is 0.4.7, ready for the v0.4.7 tag.

### Porting notes

1. Move the pin together, to 0.4.7, as 0.3.1's note 1 lists, then run
   `make verus-headers`. A port moving from 0.4.5 or earlier applies the
   notes of each release in between first. Nothing else changes: the
   counter built at 0.4.6 and at this release gave identical output
   (standard output, standard error, exit code, `--list` included) on
   the eleven ports the v0.4.7 release pull request names, and
   `room_functions`, `room_lines` and `[[runs]]` are new keys a port
   turns on by adding them. A port that keeps its own room paragraph in
   verus/FLOOR may set the two room keys to the figures it states.
2. The ledger layout is a port's own move, not part of the bump: a port
   still on `docs/abi-changes.md` keeps it, and the kit entry leaves it
   alone. A port that moves splits each `### ABI-nnn` entry into
   `docs/abi/ABI-nnn.md`, keeps the file's preamble as
   `docs/abi/README.md`, and changes every citation of
   `docs/abi-changes.md` (its AGENTS.md, CONTRIBUTING.md, review prompt,
   deps.yaml paths and prompts, and check descriptions), in a pull
   request of its own.
3. A port that runs Verus on several hosts (challoc: x86_64 and
   aarch64) may declare each run as `[[runs]]` in place of the
   shorthand, in its own pull request with a `[[narrowed]]` line for
   `[[runs]] replaces the [crates] verus_sets shorthand` and one per
   run added or removed (the shorthand's run is `verus`, so a head
   with no run of that name also removes it). Items only one run compiles, such as one
   under `not(target_arch = "x86_64")`, may then count as verified.

## 0.4.6

### Changes

- Two more shapes of a carried `macro_rules!` (#134). One at the top of
  a file whose `#[macro_use] mod name;` sits inside an inline
  `#[macro_use]` module is carried out through that module too, so it
  shadows an earlier definition of its name after the module; it was
  in scope only inside the module, and what it writes was in no count.
  One an expansion writes at the top of a carried file, or in an inline
  `#[macro_use]` module, is carried as one written there; it was in
  scope in its own file, or its own module, alone. The counter reads
  files in path order, and an invocation a definition an expansion
  writes may reach (a carried one from its `mod` on, or any in a file
  of a module in its scope), in a file read before the definition's,
  fails the run with "may expand the `macro_rules! NAME` that an
  expansion writes at FILE:LINE": write that `macro_rules!` outside the
  macro.
- A `macro_rules!` under a `cfg` that a shipped build may not meet
  where the invocation is built no longer shadows the definition of its
  name before it (#26). A `verus_sets` atom (`feature = "jit"`, or one
  of two `target_arch` values) made it sure for Verus's build, so the
  invocation took it alone, and what the earlier definition writes,
  which a shipped build without the atom compiles, was in no count. Both
  are candidates now: a pair that counts differently fails the run as
  "could be any of 2 definitions", as the same pair under `cfg(unix)`
  does. No port's run changes (each port's counter output is identical
  before and after).
- The counter fails the run (exit 1) when a module declaration names a
  file it does not read (#124): a `#[path]` to a file not named `*.rs`
  (`#[path = "gen"] mod gen;`) or to a `.rs` file outside the crate's
  `src/`, under either reading of where the declaring file's modules
  live (a main.rs or lib.rs that is a module declares them under
  `stem/`). rustc builds that file's functions, and the counts left them
  out, raising the verified share. A module only tests build is exempt:
  one whose declaration, or every path of declarations to it from a
  crate root (or from a file nothing declares), has a test-only step.
  No port's run changes (each port's counter output is identical before
  and after).
- A `macro_rules!` at the top of a file that `#[macro_use] mod name;`
  declares is in the declaring file's textual scope from that `mod`'s
  line on, as rustc reads it (#128). It did not shadow an earlier
  definition of its name there, so an invocation after the `mod` took
  the earlier one, and what the later one writes was in no count. It
  now orders against the declaring file's definitions at that line,
  also through a chain of such files (by each `mod`'s line along the
  chain, then its own). The order within one line is not read: one on
  the line of the `mod` that carries the other in, or two carried in by
  two `mod`s on one line, are candidates beside each other, and so is
  one a `mod` on the invocation's own line carries in. A module that
  several `mod`s declare (under `cfg`s, one with `#[macro_use]` and one
  in another place, or one without) has no one line: a definition it
  carries in is a candidate from the first such `mod` on and ends no
  chain, so where the builds' readings count differently the run fails.
  A message
  naming several candidates from one carried file now lists them
  latest first, as it does for one file's own. No port's run changes
  (each port's counter output, `--list` included, is identical before
  and after).
- The counter classes a file from every reading that may compile it
  (#47): the declarations of both readings of where a file's modules
  live (a lib.rs or main.rs that is a module declares them under
  `stem/`), and for a crate root its own target. A file only the other
  reading reaches without a guard, or a binary root the library
  declares as a module under `cfg(test)`, was classed test-only, and
  its functions left the denominator; they now count. The other reading
  may be one no target makes, so a file it names is unsure through it:
  counted, never verified. No port's run changes (each port's counter
  output, `--list` included, is identical before and after).
- kani.sh removes a `kani-list.json` in the port's root before each
  `cargo kani list` (#125), and fails (exit 1, naming the crate) when
  a listing that passes writes none. It read whatever file sat there,
  so a stale one from a manual `cargo kani list` would have stood for
  a crate whose listing wrote nothing. Kani 0.68.0 writes the file on
  every listing, so a port sees no change.
- The counter's git calls ignore `GIT_LITERAL_PATHSPECS`,
  `GIT_GLOB_PATHSPECS`, `GIT_NOGLOB_PATHSPECS` and
  `GIT_ICASE_PATHSPECS` in its environment (#117). With
  `GIT_GLOB_PATHSPECS=1` or `GIT_LITERAL_PATHSPECS=1`, the search for
  the base's configuration (#89) found nothing, so a moved `--config`
  was compared with nothing.
- A bare `thread_local!` beside a `*` or a `#[macro_use]` that may
  bring another crate's `thread_local` is read as that other macro
  (#122): it counts as one function outside `verus!`, `thread_local!`,
  or fails the run where it may come through a re-export the counter
  does not follow, as any other crate's macro does. It counted nothing.
  A glob of Verus's prelude, of std or core, or of a module of the
  crate that imports nothing of the name, and a `#[macro_use]` of std's
  or Verus's crates, leave it std's, as before, each by a name no Cargo
  rename, `use` alias or `extern crate .. as` rebinds. A glob's plain
  first segment names a module of the crate as rustc reads a `use` path
  in the crate's edition (none after a leading `::` from 2018 on), and
  such a module is read whatever its name. No port's run changes (each
  port's counter output, `--list` included, is identical before and
  after).
- A `thread_local!` imported by name (`use core::thread_local;`) or
  invoked by a path (`core::thread_local!`) whose first segment is
  `std` or `core` counts as another crate's macro where a `use` or an
  `extern crate .. as` gives another crate that name (#137): in the
  module, or for an import in a 2015 crate at the root too. It counted
  nothing. No port's run changes (each port's counter output, `--list`
  included, is identical before and after).
- `[crates] test_features` no longer runs cargo (#136): the counter
  reads which builds may turn a test-only feature on from the
  workspace's manifests and `Cargo.lock`, so the check runs in a job
  with no cargo and in one where offline resolution fails (a `vstd`
  pin cargo cannot see from crates.io's index). It reads what `cargo
  tree` over the workspace's normal and build edges, with every
  member's other features on, read before: renamed and workspace
  dependencies, `name/feature` and `name?/feature`, target-specific
  sections, path dependencies that are not members, members by a
  trailing `/*`, and `exclude`; an optional dependency's implicit
  feature; under resolver 1, a member's dev-dependencies' features,
  which it unifies into normal builds; and any feature but a test-only
  one of a local package that a package with a source depends on (as a
  `[patch]` may make it). It fails the run on a form it does not read
  (another member glob, a malformed entry, a feature naming no
  dependency, an unknown resolver), on a package with no source in
  `Cargo.lock` whose manifest it does not find, on a compiled package
  `Cargo.lock` does not have (a stale lockfile), and on a package with
  a source that depends on one with a test-only feature.
  The kit's tests hold it to cargo's own answer on every case. No
  port's run changes (each port's counter output, `--list` included,
  is identical before and after).
- A bare `thread_local!` imported by name through a `std` or `core`
  that names a module of the crate, as rustc reads a `use` path (in
  2015 a root `mod core`), is read as any other imported macro (#140):
  a re-export the counter does not follow fails the run. It counted
  nothing. No port's run changes (each port's counter output, `--list`
  included, is identical before and after).
- A new check, on with `[loops] table = FILE` (#76; README "Nested
  loops"): the run fails on each function of a shipped crate that
  nests a loop inside another, or calls from inside a loop a function
  that loops (by name within its file, as `[recursion]` reads calls),
  unless FILE's table names its file and quotes `fn NAME`: the
  work-bound theorem's row that bounds the product. A port with no
  `[loops]` section runs as before; removing the section after a base
  revision has it fails a `--base-rev` run, as for the other gates.
- The counter's version is 0.4.6, ready for the v0.4.6 tag.

### Porting notes

1. Move the pin together, to 0.4.6, as 0.3.1's note 1 lists, then run
   `make verus-headers`. A port moving from 0.4.4 or earlier applies the
   notes of each release in between first. Nothing else changes: each
   counter change above gave identical counter output (`--list`
   included) on the eleven ports its pull request compared, built at
   its base and its head (the v0.4.6 release pull request, #144, names
   the ports' commits and the command), and `[loops]` is a new check a
   port turns on by adding the section.
2. A port that kept a test-only feature in `[crates] verus_never_sets`
   because `test_features` needed cargo in its floor or verus job
   (#136) may now move it to `test_features`. That is a narrowing under
   `--base-rev` (`[crates] test_features adds "NAME"`): add a
   `[[narrowed]]` line with that `change` and a reason, and remove the
   atom from `verus_never_sets` in the same change, since an atom in
   both fails the run. The move takes the functions under that feature
   out of the counts, where under `verus_never_sets` they counted but
   were never verified.

## 0.4.5

### Changes

- The counter classes an exec function that
  `#[verifier::exec_allows_no_decreases_clause]` covers as trusted, not
  verified (#74): one that carries it, or one in an `impl`, trait,
  inline module or file that does. Verus checks such a function's loops
  and recursion with no `decreases`, so nothing proves it terminates,
  and the counter credited it as verified. Where it covers an
  out-of-line `mod x;` (on it, on an inline module around it, or atop
  the file that declares it), the attribute fails the run (exit 1),
  since the counter reads each module file on its own. No stand-in may rewrite the name, and the template AGENTS.md's
  escape-hatch grep lists it.
- The counter holds a configuration that `--config` moved to a path the
  base lacks (under ROOT, or outside it where the base has no
  `verus/count.toml`) to the one configuration file of the base's tree
  (a `.toml` other than a `Cargo.toml` with a top-level `crates` key,
  `verus/count.toml` among them) (#89). It read the base only at the
  head's path and, finding nothing there, compared no gate and read the
  base's FLOOR, REACH and RLIMIT at the head's paths, so a head could
  move its configuration, narrow its gates and move those files in one
  change and pass. A base with several such files fails the run (exit
  1), naming them, and one such file that does not parse, a name the
  search cannot read back, or a blob Git cannot read, fails it with
  exit 2. A base copy at the head's path whose blob Git cannot read
  now fails with exit 2 too, where it read as absent. A base with none
  still passes as the change that adopts the counter, unless it has a
  FLOOR (at the head's `[floor] path` or `verus/FLOOR`) and the
  configuration is under ROOT: that run now fails (exit 1).
- The counter refuses only the symlinks under a crate's `src/` that its
  source walk reads through (#71): one named `*.rs`, or one to a
  directory holding a `.rs` file or a cycle at any depth, or one whose
  target, or anything under it, it cannot stat, resolve or list (a link
  loop); a dangling link is passed over. It refused
  any link there, so uutils, which links a `LICENSE` into each
  utility's directory and some `locales` directories of `.ftl` files,
  could not count its `coreutils` and `uu_stdbuf` crates. A crate
  whose directory or `src/` is a link fails as before.
- The counter reads a `thread_local!` shown to be another macro than
  std's as any macro of its name (#68). It skipped every macro of the
  name, so a crate's own `macro_rules! thread_local` that writes
  functions, or another crate's `thread_local!`, added no row and its
  functions left the denominator. Now the crate's own is expanded and
  its functions counted, and another crate's (by a path such as
  `elsewhere::thread_local!`, or bare after a `use` of the name from
  outside `std` and `core`) counts as one function outside `verus!`,
  `thread_local!`, as another crate's macro does; so does `std::` or
  `core::` when a Cargo dependency is renamed to that name. std's, and
  a bare one only a `*` or a `#[macro_use]` may bind to another (#122
  tracks it), count nothing, as before.
- kani.sh takes `KANI_MANIFEST`, a workspace manifest it passes as
  `--manifest-path` to `cargo fetch` and every `cargo kani` call
  (#71), and proved-port.mk sets it from the `--manifest-path` in
  `VERUS_CARGO_FLAGS`, so a port whose workspace is not at its root
  (a uutils port's `crates/uutils`) sets the path once for Verus and
  Kani.
  kani.sh ran cargo in make's directory, where such a port has no
  Cargo.toml, so `cargo fetch` failed even with `KANI_NONE = yes`. A
  `KANI_MANIFEST` that names no file, or has a space, exits 2.
- `verus/THEOREMS.md`'s template drops its Index table (#102). Each
  theorem file's first line, `# <ID>. <title>` (the theorem skeleton's
  heading), is its index entry, and the Index section gives a command
  that lists them. The table repeated those headings, nothing checked
  that a row matched its file, and every proof change edited the one
  shared file, so two proof changes conflicted there even when nothing
  else overlapped. Neither the counter nor the make targets read the
  table.
- The template `.chainguard/develop.yaml` lists `claude` (Claude Code,
  for the owner agents that run in development sessions) in its own
  `environment.packages`, beside the `from:` dev.yaml, so no check
  installs it (#111). A port takes it into its develop.yaml by hand;
  nothing holds the file to the template.
- `verus/FLOOR`'s header names the second kind of `lowered` line (#73):
  a change that deletes verified code, whole functions or lines of one,
  takes it from both the verified count and the total, and can lower a
  share with no code becoming trusted or outside. Its REASON names the
  functions removed or shortened, says that nothing became trusted or
  outside, and gives the counts. The header described only the
  trusted-or-outside kind, so a reviewer told to hold `lowered` lines to
  the header could refuse a deletion's line. "Close the gap by verifying
  more code first" stays the first step, and the proved-c-port skill
  (SKILL.md, references/proofs.md) says the same. The counter reads
  `lowered` lines as before.
- The counter reads the base revision's files at their paths relative
  to ROOT (`REV:./PATH`), also when ROOT is below the repository's top
  level (#114). It read `REV:PATH`, which Git resolves from the top
  level, so for such a ROOT every base read (the configuration, FLOOR,
  RLIMIT, REACH, the crates' sources and manifests) found nothing and
  each ratchet compared with nothing. A port whose ROOT is the top
  level reads the same files as before. The base crates' file list is
  now read verbatim (`git ls-tree -z`), so a name Git would quote is
  read too, and one that is not UTF-8 fails the run (exit 2). The
  crates of a base `[crates] dir` are listed from the whole tree
  (`ls-tree --full-tree`), so a crate the head deletes there is a
  narrowing below the top level too.
- The badge gate's lenient reading also ends a comment whose paragraph
  an HTML block start ends (`<` and a letter, `</`, `<?`, `<!`: the
  `<p align="center"><img ...>` badge), one in nested containers
  (`- > <!--`), one in a nested quote at a line with fewer `>`
  (`> > <!--`, then `> text`), one after an ordered marker other than
  `1.` also where a paragraph would end it (`2. <!--` cannot interrupt
  a paragraph, so it may continue one), one whose paragraph a block
  start ends up to 3 columns past where the paragraph's text starts
  (`  - Text <!--`, then `    > badge` at the nested item's content
  column), and reads indentation as CommonMark does: a tab counts to the next multiple of 4 columns from where it
  sits, separates a list marker, gives one column to the space after a
  `>` and keeps the rest as indent, and a `<!--` 4 columns into a line,
  a quote or an item is indented code (#120). Each of these READMEs hid a stale second badge and now fails.
  The tag rule is wider than CommonMark's (a plain tag such as `<img`
  cannot end a paragraph), so an old badge on such a line inside a
  mid-line comment now counts too; see the porting note.
- The badge gate counts more second badges (#22): an HTML `<img`
  badge from any host (not only shields.io), and a badge that renders
  in either of two readings of the file. The strict reading hides a
  fence or an HTML comment never closed to the end of the file, as
  before; the lenient one reads that opener as text, reads as text a
  `<!--` in a backtick code span, escaped, on a line indented 4 or
  more spaces, or one whose paragraph, list item or block quote ends
  before its `-->`, and
  ends an indented fence (a list
  item's) at the first line indented less. `<!-->` and `<!--->` are
  whole comments. A README with a stale second badge in one of these
  shapes now fails (exit 1), naming its line, and so does one whose
  canonical badge renders in only one reading (after a fence or a
  comment the file never closes).
- The template AGENTS.md and review prompt record two rules ports had
  each been adding by hand (#75). "Coverage first for the runtime cost
  of a proof": where no ghost or spec route avoids it, a measured cost
  is accepted by default and recorded with its bench table, a cost
  above about 2% on a hot path gets an "optimize within proof bounds"
  follow-up issue, and one above about 10% on a real workload goes to
  the maintainers; the review prompt holds such a PR only when that
  record is missing. "Shared coverage lines": a proof PR keeps its
  figures in its theorem file and body, and the FLOOR PR updates the
  README, `verus/README.md` and design or handoff coverage lines from
  main, with FLOOR's header's two exceptions (the first proof PR's raise
  from 0.0, and the badge a `lowered` line moves). The skill (SKILL.md,
  references/proofs.md) says the same. These are starting-point
  templates nothing holds, so a port takes the text into its own
  AGENTS.md and `.chainguard/review-prompt.md` by hand, or keeps the
  rules it already wrote.
- The counter's version is 0.4.5, ready for the v0.4.5 tag.

### Porting notes

1. Move the pin together, to 0.4.5, as 0.3.1's note 1 lists, then run
   `make verus-headers`. A port moving from 0.4.3 or earlier applies the
   notes of each release in between first.
2. Optional: a port may delete its `verus/THEOREMS.md` Index table and
   take the template's Index section. First check that each
   `verus/theorems/<ID>.md` starts with `# <ID>. <title>`, and fix
   those that do not. This loop prints each file whose first line is
   not a heading:
   `for f in verus/theorems/*.md; do head -n1 "$f" | grep -q '^# ' || echo "$f"; done`. Then change the port's own
   instructions (AGENTS.md, CONTRIBUTING.md, review prompts) that tell a
   change to add an index row. A port that keeps its table changes
   nothing.
3. Before the bump, run `grep -rn exec_allows_no_decreases_clause
   --include=*.rs .`. A port with no hit changes nothing for #74. For
   each hit, prove termination instead (give each loop and recursive
   call its `decreases` and delete the attribute), or take the functions
   it covers as trusted: each needs a seams row of kind `no_decreases`
   (the SEAMS.md template's kind list has it), and a share that falls
   below FLOOR needs a `lowered` line naming them. Where the attribute
   covers an out-of-line `mod x;` (on it, on an inline module around
   it, or atop its file), move it onto the functions, impls, traits or
   inline modules it covers.
4. Optional: a port whose `.chainguard/develop.yaml` does not list
   claude adds the template's `environment.packages: [claude]` block
   beside its `from:` (#111), and the clause of the template's comment
   that says why it is there and not in dev.yaml. A port that already
   lists it, or runs no agents in its sessions, changes nothing.
5. For #89, a port whose configuration has a base copy (it stays at one
   path under ROOT, or is outside ROOT beside a base
   `verus/count.toml`), or is outside ROOT with no configuration in the
   tree, changes nothing. One that moves it is held to the base's copy
   at its old path, which must
   be the base's only configuration file; with several (a kept older
   `verus/count.toml` counts), it keeps the path its base reads in that
   change, and deletes the others in a change of their own. A port
   whose FLOOR predates its configuration adds the configuration in a
   change that runs the verus check with `VERUS_BASE=none`, as the
   change that adds the first FLOOR does.
6. For #22 and #120, run `make verus` after the bump. A README that
   now fails as "N Verus badges" names the line. If it is a stale badge
   that renders, delete it. If it is an old badge kept inside an HTML
   comment that opens mid-line (`Text <!--`), on a line that starts
   with an HTML tag (`<img ...>`) or a tab, put the comment on lines of
   its own (`<!--` at the start of a line, `-->` after the badge) or
   delete the old badge. The same holds for a comment that opens after
   an ordered marker other than `1.` (`2. <!--`, the second item of a
   list), which may continue a paragraph, so it now ends at a blank
   line or a line that starts a block: put `<!--` on a line of its own
   at the item's content column (`   <!--`), or delete the old badge. A
   README with one rendered badge and none of these comments changes
   nothing.
7. For #68, a port changes nothing unless it defines its own
   `macro_rules! thread_local`, imports the name with a `use` from
   outside `std` and `core` (in a `{ .. }` list over several lines
   too), invokes `thread_local!` by another path, or renames a Cargo
   dependency to `std` or `core`. `grep -rnw thread_local --include=*.rs .`
   lists every mention; each that is not `thread_local!`,
   `std::thread_local!`, `::std::thread_local!` or
   `use std::thread_local;` is a candidate. Each such invocation now
   adds rows, and a share that falls below FLOOR needs a `lowered` line
   naming them.
8. For #71, a port whose `VERUS_CARGO_FLAGS` names no
   `--manifest-path` changes nothing. One that names one (a uutils port)
   now has `make kani`, `kani-list` and `kani-regular` run cargo in that
   workspace too, so it can drop `KANI_NONE = yes` once its harnesses
   are there. A port whose Kani harnesses are in another workspace sets
   `KANI_MANIFEST` to its manifest, or to empty for make's directory.

## 0.4.4

### Changes

- `make verus-proofs` (and `make verus`) writes cargo's `CACHEDIR.TAG`
  into `VERUS_TARGET_DIR` when the directory has none, before it makes
  `VERUS_STATS`'s directory (#96). The recipe made that directory (the
  default `VERUS_STATS` is in it) before cargo ran, and cargo writes the
  tag only into a target directory it creates itself. cargo also refuses
  to clean a directory without the tag ("missing or invalid
  `CACHEDIR.TAG`"). So in verify mode, whose clean is `VERUS_TARGET_DIR`
  itself, `make verus` failed on a fresh checkout before any proof ran
  (from 0.4.1). Focus mode's `verus-partial/` is a directory cargo
  creates, so its clean was unaffected.
- The counter's version is 0.4.4, ready for the v0.4.4 tag.

### Porting notes

1. Move the pin together, to 0.4.4, as 0.3.1's note 1 lists, then run
   `make verus-headers`. A port moving from 0.4.2 or earlier applies the
   notes of each release in between first. Nothing else changes.
2. A verify-mode checkout whose target directory an earlier kit made
   (it has no `CACHEDIR.TAG`) gets the tag on its next run, and the
   clean then runs as it does on a fresh checkout.

## 0.4.3

### Changes

- `make verus-proofs` (and `make verus`) passes `-j VERUS_JOBS` after
  the `-p` list and `VERUS_CARGO_FLAGS`, just before `--` (#91). 0.4.2
  put it first, and cargo verus refuses a `-j` before a `-p` ("Args
  forwarded to Cargo must precede args forwarded to Verus"), so
  `make verus` exited 2 before any proof ran in every port that sets
  `VERUS_CRATES` and kept `VERUS_JOBS` at its default of 1.
- The clean before cargo verus, and its `cargo metadata` when
  `VERUS_CRATES` is empty, get the `--manifest-path P` (or
  `--manifest-path=P`) that `VERUS_CARGO_FLAGS` gives cargo verus (#93).
  A port whose workspace is in a subdirectory failed at once on 0.4.1
  and 0.4.2 with "could not find `Cargo.toml`".
- The counter's version is 0.4.3, ready for the v0.4.3 tag.

### Porting notes

1. Move the pin together, to 0.4.3, as 0.3.1's note 1 lists, then run
   `make verus-headers`. A port moving from 0.4.0 or 0.4.1 applies
   0.4.1's and 0.4.2's notes first.
2. A port that set `VERUS_JOBS =` (empty) only to get past 0.4.2's
   `-j` order, and whose cargo verus checks more than one crate, drops
   the override: `-j 1` keeps the crates' statistics from interleaving
   (0.4.2's first change).
3. A port whose `VERUS_CARGO_FLAGS` names another workspace's manifest
   (`--manifest-path`) sets `VERUS_TARGET_DIR` to that workspace's
   target directory, the one cargo verus builds in (for a workspace in
   crates/w: `VERUS_TARGET_DIR = crates/w/target`). The kit does not guess
   it, and with the default `target` the clean removes nothing there:
   a warm rerun runs no Verus and writes empty statistics, as before
   0.4.1.

## 0.4.2

### Changes

- `make verus-proofs` (and `make verus`) runs `cargo verus` with
  `-j VERUS_JOBS`, 1 by default (#79). cargo re-prints each crate's
  output a line at a time, so two crates Verus checked at once mixed
  their pretty-printed `--output-json` documents in `VERUS_STATS`, and
  the rlimit headroom check failed on a JSON parse error after every
  proof had passed. Empty `VERUS_JOBS` leaves cargo's default. The
  counter now refuses statistics in which one document starts before
  the one before it ends ("documents interleave", exit 2), naming both
  lines and `-j 1`.
- With `VERUS_CRATES` and `VERUS_CLEAN` both empty, the clean takes the
  workspace's default members from `cargo metadata` (#81), the
  packages cargo verus checks with no `-p`: the root package for a
  package manifest, every member (or `default-members`) for a virtual
  manifest. 0.4.1 ran `cargo pkgid`, which fails on a virtual manifest.
  When cargo metadata names none, the target fails before any clean.
- verus/FLOOR's header: a FLOOR pull request may record a figure below
  the one the check names, never below the base revision's, when an
  open pull request lands that share lower without a `lowered` line;
  its description names that pull request (#82). verus/README.md,
  AGENTS.md, the review prompt's Verus item and the skill say so too.
- A counter run with `--base-rev` and no `--rlimit` fails when the base
  has verus/RLIMIT (#86), so `VERUS_RLIMIT =` cannot turn the headroom
  check off.
- The shipped files no longer name the theseus ports' private
  repositories, and `kit/internal-test.sh` flags those names. The
  counter's example configuration for the PCRE2 port is now
  `kit/count/configs/pcre2.toml`, under the upstream library's name.
- The counter's version is 0.4.2, ready for the v0.4.2 tag.

### Porting notes

1. Move the pin together, to 0.4.2, as 0.3.1's note 1 lists, then run
   `make verus-headers` and take the kit's FLOOR header.
2. Copy the review prompt template's new Verus-item sentence (the FLOOR
   pull request that records a figure below the one the check names)
   into the port's `.chainguard/review-prompt.md`: `make verus-headers`
   holds only its `- The kit` item.
3. A port that set `VERUS_CLEAN` only because its root manifest is
   virtual (0.4.1's `cargo pkgid` failure, in focus mode with an empty
   `VERUS_CRATES`) may drop it. Keep it where cargo verus checks more
   than the default members: verify mode's verified workspace
   dependencies, or a `--workspace` in `VERUS_CARGO_FLAGS` (0.4.1's
   note 2 still applies).
4. A port that runs `make verus` with `VERUS_RLIMIT =` while its main has
   verus/RLIMIT now fails its checks: drop the override.
5. A port whose cargo verus checks one crate may set `VERUS_JOBS =`
   (empty) to build its dependencies in parallel again.
6. A port whose comments cite the counter's example configuration by its
   old file name cites `kit/count/configs/pcre2.toml`. No port passes
   that path to the counter.

## 0.4.1

### Changes

- `make verus-proofs` (and `make verus`) cleans the checked packages'
  earlier results before `cargo verus` runs (#72). cargo kept them when
  nothing it fingerprints had changed, so a rerun ran no Verus, wrote
  empty statistics, and failed the rlimit headroom check with "no
  statistics". `VERUS_CLEAN` names the packages (default `VERUS_CRATES`,
  or the root package when that is empty), and `VERUS_TARGET_DIR` names
  cargo's target directory (default `CARGO_TARGET_DIR`, else `target`).
  The clean takes the profile and targets `VERUS_CARGO_FLAGS` builds
  with (`-r`, `--release`, `--profile`, `--target`). The counter's
  no-statistics note names `make verus-proofs` rather than a bare
  `cargo clean -p`, which misses focus mode's results.
- The counter's version is 0.4.1, ready for the v0.4.1 tag.

### Porting notes

1. Move the pin together, to 0.4.1, as 0.3.1's note 1 lists.
2. A port whose `cargo verus` checks packages that `VERUS_CRATES` does
   not name (`VERUS_MODE = verify` over a workspace with an empty
   `VERUS_CRATES`, whose root package is not the only one checked) sets
   `VERUS_CLEAN` to every package Verus checks. A port whose cargo target
   directory is set elsewhere than `CARGO_TARGET_DIR` (a
   `.cargo/config.toml` `target-dir`) sets `VERUS_TARGET_DIR` to it.

## 0.4.0

### Changes

- The counter reads Verus 0.2026.11.02's syntax: its `verus_syn` pin
  moves to `=0.0.0-2026-11-01-0251`, the vstd that release names.
- The `unsupported-attribute` seam kind. Verus 0.2026.11.02 refuses
  `no_mangle`, `export_name` and `link_section` inside `verus!` unless
  the item carries `#[verifier::assume_unsupported_attributes]`, and
  then takes the attribute on trust while still checking the body. The
  counter keeps each such function's class (a verified export stays
  verified), prints the count of these seams and each function after
  the tables, and with a generated ledger requires an
  `unsupported-attribute` entry, status `trusted`, for each
  (kit/count/README.md, "Linkage attributes Verus takes on trust").
- The counter's version is 0.4.0, ready for the v0.4.0 tag.

### Porting notes

1. Move the pin together, to 0.4.0, as 0.3.1's note 1 lists, and move
   to Verus 0.2026.11.02 in the same change: every `verus~` pin
   (`.chainguard/ci/*.yaml` and the sessions' environment), and vstd
   (with `verus_builtin` and `verus_builtin_macros` where the port pins
   them) to `=0.0.0-2026-11-01-0251` in every Cargo.toml and Cargo.lock
   that names it, sub-crates included. Rust stays 1.99.
2. Add `#[verifier::assume_unsupported_attributes]` to each item inside
   `verus!` that Verus now refuses for `no_mangle`, `export_name` or
   `link_section` (once in an export macro's body where one writes
   them). No function changes class. List each item as an
   `unsupported-attribute` seam, status `trusted`, its grounds the ABI
   test that checks the symbol or section: a row of the seams table, or,
   with a generated ledger, an entry the generator writes at the item's
   line for every such item, statics included, and for a function a
   macro writes at the invocation's line, where the counter lists it
   (then regenerate the ledger). The audit grep (the skill's SKILL.md
   and the AGENTS.md template) now matches the attribute: copy the
   template's grep into the port's AGENTS.md, which `make
   verus-headers` does not hold.
3. Delete the port's own `assume_specification`s that vstd now states
   (`get_unchecked(_mut)`, `split_at_unchecked`, slice `swap`,
   `cmp::min`/`max` and their `_by`/`_by_key` forms, `assert_failed`,
   `Vec`'s `AsRef`), and re-point a Kani record that discharged one at
   vstd's contract or record it as vstd's trust. Replace
   `IteratorSpec::will_return_none` with `IteratorSpec::produces(&iter).all(|p:
   bool| p)`, move a const outside `verus!` that a verified type uses
   into `verus!`, and mark a `use vstd::prelude::{Ghost, Tracked}` that
   only Verus's build uses
   `#[cfg_attr(not(verus_keep_ghost), allow(unused_imports))]`.
4. Run `make verus` and hold every function under verus/RLIMIT's share
   cap without raising an rlimit: split a proof that crosses it.

## 0.3.1

### Changes

- `make verify` runs every target the Makefile's `VERIFY_TARGETS` lists,
  in order, by default `verus kani` (#61). A port's package build runs
  it, with `VERUS_BASE=none`, and nothing else.
- `make kani-regular` runs every Kani harness one at a time in regular
  format with CBMC's `--verbosity 9`, and fails on
  `KANI_IGNORING_PATTERN` (`warning: ignoring`), which the terse output
  of `KANI_JOBS` hides (#61). It refuses a `KANI_CBMC_ARGS` that sets
  `--verbosity`: CBMC takes the first, so a lower level would hide the
  line it scans for.
- The counter fails a crate with code Verus checks whose Cargo.toml does
  not opt into verification (#53): cargo verus checks only a package
  that sets `[package.metadata.verus] verify = true`, and the counter
  had credited the others' `verus!` functions as verified.
- `--rlimit` names a method by its type's module, and names every miss
  (#52); the counter reads a Verus keyword naming a binding outside
  `verus!` as an identifier (#54).
- `kani.sh` prints why a `cargo kani list` failed (#55).
- The template verus checks run `make verus-headers` before `make verus`
  (#56), and the templates' wording no longer names a port (#57).
- `kit/install.sh PREFIX` installs a kit checkout as the package lays it
  out, for a host without the package (#58).
- This file (#64), whose notes the template deps entry for the kit
  applies, with no waiting period for a kit release (#67).

### Porting notes

1. Move the pin together, to 0.3.1: `PROVED_PORT_VERSION` in the
   Makefile, every `theseus-proved-port~` pin (`.chainguard/ci/*.yaml`,
   and the sessions' environment, `.chainguard/develop.yaml` or the file
   its `from:` names), and the `ref` of the proved-port marketplace in
   `.claude/settings.json` (`v0.3.1`).
2. The verification opt-in. 0.3.1 is the first release whose counter
   requires each crate with code Verus checks to set

   ```toml
   [package.metadata.verus]
   verify = true
   ```

   in its Cargo.toml, and fails one that lacks it with a `FAIL
   verification opt-in` line naming the crate. A new port sets it when
   it adopts the kit (kit/count/README.md, "Bootstrapping a port", step
   1). A bump adds none: where the counter names a crate, the bump
   changes no manifest and leaves the crate for a person, who adds the
   table to it or shows that it holds no code Verus checks. What holds
   such code is not always plain from the tree (a test crate can take
   Verus files in by `#[path]`, a script can match `verus!` in its own
   source).
3. Set `VERIFY_TARGETS` in the Makefile, before the include, to the
   targets the port's checks verify with Verus and Kani, and the
   controls those checks run, when that is more than `verus kani`: a
   further proof configuration (another feature set), a Kani run at
   another feature set, the regular-format run, a control a Verus or
   Kani check runs. Tests, Miri, fuzzing, conformance and other suites
   stay out. Give each such step a target of the port's own, not named
   like a file or directory in the port's root, and have the checks call
   it, so the package build and the checks run the same commands. A port
   without Kani sets `VERIFY_TARGETS = verus`. Add `kani-regular` and
   `verify` to `PROVED_PORT_TARGETS`, the targets the Makefile's no-kit
   guard lists.
4. A port whose checks ran the regular-format Kani run by hand
   (`KANI_JOBS= KANI_FORMAT=regular KANI_CBMC_ARGS='--verbosity 9'
   KANI_FAIL_PATTERN=...`) calls `make kani-regular` instead, with its
   shard variables, and with no `--verbosity` in `KANI_CBMC_ARGS`, which
   `make kani-regular` refuses.
5. Run `make verus-headers` and take the kit's text where it reports
   drift: `.chainguard/ci/verus.yaml` (whose verus checks now run `make
   verus-headers` first) and the kit rule of
   `.chainguard/review-prompt.md` are the files 0.3.1 changes.
6. On a host without the package, install the kit from a clone at
   v0.3.1 with `sh kit/install.sh "$HOME/.local"` (PORT-README,
   "Building and testing").

## 0.3.0

### Changes

- The kit is the theseus-proved-port package (#42, #45, #46, #48, #49):
  the counter as `verus-count`, the make targets, the templates and a
  KIT record under `/usr/share/proved-port/`. Vendored mode is gone.

### Porting notes

For a port that leaves vendored mode (0.2.4 or before):

1. Pin `theseus-proved-port~0.3.0` in the checks and the sessions'
   environment, set `PROVED_PORT_VERSION = 0.3.0`, and include
   `$(PROVED_PORT_HOME)/make/proved-port.mk` behind the guard
   (kit/count/README.md, "Adopting: the package").
2. Delete the vendored `verus/make/` and `verus/count/`, and the `# kit:`
   lines of the kit's Verus files.
