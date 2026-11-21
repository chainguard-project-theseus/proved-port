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
