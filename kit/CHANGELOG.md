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

### Porting notes

1. Nothing is required. A port may replace the shared part of its
   `ci/lint-unsafe.sh` with `make lint-unsafe`: set `UNSAFE_CRATES` (and
   `UNSAFE_WORKSPACE_LINTS = no` if its manifests have no workspace
   lints) before the include, run `make lint-unsafe` from the script or
   the check, and keep only the rules of its own (a mint seam list, an
   unsafe budget, a SAFETY: comment per block) in the script. Paste
   both scripts' verdicts on the switching commit in the pull request.
   A port whose Makefile has the no-kit guard (kit/count/README.md,
   "Adopting: the package") adds `lint-unsafe` to its
   `PROVED_PORT_TARGETS`, so the target fails with the kit's message
   where the package is missing.

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
