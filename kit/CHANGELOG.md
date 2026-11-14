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

## 0.3.1

### Changes

- `make verify` runs every target the Makefile's `VERIFY_TARGETS` lists,
  in order, by default `verus kani` (#61). A port's package build runs
  it, with `VERUS_BASE=none`, and nothing else.
- `make kani-regular` runs every Kani harness one at a time in regular
  format with CBMC's `--verbosity 9`, and fails on
  `KANI_IGNORING_PATTERN` (`warning: ignoring`), which the terse output
  of `KANI_JOBS` hides (#61).
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
2. Opt every crate with Verus code into verification, in the same pull
   request: add

   ```toml
   [package.metadata.verus]
   verify = true
   ```

   to each crate the counter's `FAIL verification opt-in` line names. A
   crate with no Verus code needs nothing.
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
   shard variables.
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
