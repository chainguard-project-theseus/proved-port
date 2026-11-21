# verus-count

The coverage counter of a proved port: it reads the port's sources with
`verus_syn`, classes every executable function and its lines as
verified, trusted, discharged or outside `verus!`, and holds the shipped
crates' shares to `verus/FLOOR`. With its optional checks it also gates
recursion, entry-point reachability (REACH), compile inputs, the C
caller's token mints (MINTS) and rlimit headroom (RLIMIT). It is
The PCRE2 port's `verus/count` made generic, with challoc's generated-ledger back
end added; `src/main.rs`'s module documentation states every rule.

## Running

```sh
cargo build --locked --release
verus-count [--config FILE] [--list] [--floor FILE [--base-rev REV | --no-base]] [--rlimit STATS] ROOT
verus-count [--config FILE] --badge ROOT
```

ROOT is the repository root (default `.`) and FILE the configuration
(default `ROOT/verus/count.toml`). `--list` prints one line per
executable function before the table, and changes nothing else: every
check the other options name still runs; `--floor` holds the `shipped` row to the FLOOR
file; `--base-rev` also holds FLOOR (and REACH, RLIMIT and the
configuration) to that revision's copies, and fails when the revision
does not resolve. Each copy is read at its path relative to ROOT, also
when ROOT is below the repository's top level (`REV:./PATH`, run in
ROOT). `--floor` needs one of the two base flags: with
neither the run fails (exit 1), so a CI run that loses its base fails
rather than skip every base comparison. `--no-base` says there is no
base to compare with: a local or development run, or the proof PR that
adds the port's first FLOOR. It is never a CI run's flag once FLOOR is
on the base, and giving it with `--base-rev` fails the run (exit 2);
`--rlimit` reads the statistics `cargo verus ... -- --time-expanded
--output-json` printed, and names each function as Verus does: a method
by its type's path, from the module that defines the type, wherever its
`impl` is. A `#[verifier::rlimit]` on a function the statistics do not
name fails the run (exit 2), and the failure names every such limit.
A run with `--base-rev REV` and no `--rlimit` fails (exit 1) when REV has
the RLIMIT file (`[rlimit] path` in REV's configuration): with RLIMIT on
the base, dropping `--rlimit` (`VERUS_RLIMIT =`) would turn the
headroom check off. A `--no-base` run needs no `--rlimit`.
Statistics in which one crate's document starts before another's ends
fail the run too (exit 2, "documents interleave"): cargo re-prints each
crate's output a line at a time, so crates Verus checks in parallel mix
their pretty-printed documents, and `make verus-proofs` runs cargo with
`-j 1` (`VERUS_JOBS`) so that they do not.
`--version` prints the counter's version, which
is the kit release (`verus-count 0.3.0`): `kit/count/Cargo.toml`'s
version, which the release sets before it tags (Release, below).
`--help` (or `-h`) prints the usage and exits 0. Either takes no other
argument, since it checks nothing. `--badge` prints the coverage badge line (The badge, below) for the
figures in ROOT's `[floor] path`, and takes no checking option. An
unknown option or a second ROOT fails the run (exit 2), so a misspelt
`--flor` cannot leave the floor unchecked.

## Configuration

`verus/count.toml`, every path relative to the root. Unknown keys are
an error. A check whose section is left out does not run, and prints
nothing. So with `--base-rev REV` the run fails (exit 1) when the
configuration turns off or narrows a gate `REV`'s copy runs: a
`[recursion]`, `[loops]`, `[reach]`, `[inputs]`, `[mints]` or `[badge]`
section removed; a `[badge]` `path` moved to another file; a
`[reach]` `exports_macro`, `fn_ptr_macro` or `conversion` removed or
renamed; an `[inputs]` crate or root removed or `lazy` turned off; a
`[crates]` `dir` removed or changed, a member removed, moved to another
`path`, or its `ships` changed to anything but `true` (each compared on
its own, so a member that moves and unships makes two narrowings, and
also by path: whatever the head reads at a crate the base read, a
member under any name or a `dir` it sits in, which reads it with
`ships` unset, so a rename, a member moved onto a crate of the base's
`dir`, and a crate handed between `dir` and members are each compared);
a crate of the base's `dir` (each subdirectory of it with a `src/` in
the base's tree) that the head, with the same `dir`, reads neither at
its directory nor under its name, as a member removed is, or reads under
its name somewhere else, as a member moved is, or reads as a member
with `ships` set to anything but `true` at a path the base's
configuration does not compare;
a `test_cfgs`, `test_features`
or `verus_features` name added; a `verus_sets` or `verus_never_sets`
atom added (compared as atoms, so respacing one is not a change); a
`[[runs]]` run added, removed or renamed, an atom added to a run, or
`[[runs]]` replacing the shorthand (#21); a
`stand_ins` pair added; or a crate
whose own Cargo.toml gains `publish = false`, where the base read it
with its Cargo.toml deciding whether it ships (a `dir` crate, or a
member without `ships`), at the same directory or under the same name,
and the base's Cargo.toml there does not say so. A crate the base read
under no name and at no directory is new, with nothing to compare: so a
`dir` crate whose directory is renamed is the old crate no longer read
and a new crate, as a member renamed to a new path is. Renaming a table
or a mint is not a narrowing. The base's
copy is read at the configuration's path under ROOT, as the repository
names it, or at `verus/count.toml` when `--config` names a file outside
ROOT. A base with no copy at that path is read at the one configuration
file of its tree (a `.toml` other than a `Cargo.toml` with a top-level
`crates` key, `verus/count.toml` among them), so a head that moves its
configuration with `--config`, under ROOT or out of it, is held to the
gates the base ran (`ok configuration: REV has no ..., so ... is held to
REV:..., the one configuration REV has`). A base with several such
files fails the run (exit 1), naming them, since which one it ran is
unknown: a change that moves the configuration keeps the path its base
reads. A base with none and no FLOOR (at the head's `[floor] path` or
`verus/FLOOR`), the change that adopts the counter, prints `ok
configuration: REV has no ... to compare with`; under ROOT, one with a
FLOOR ran its gates from a configuration the run cannot find, and fails
the run (exit 1), while a configuration outside ROOT is still compared
with nothing there, since a port may write its configuration outside
the tree. A base copy that does not parse, found at the head's path or
by the search (any candidate that is not TOML included), fails the run
(exit 2), as does a file the search lists but cannot read back (a name
that is not UTF-8, or a blob Git cannot read); a base copy at the head's path whose blob Git
cannot read fails it too, rather than read as absent. One route stays
open: a head that switches back to a path the base still has, such as
an older `verus/count.toml` the base kept beside the file it ran, is
compared with that copy, which may run fewer gates. A port that moves
its configuration deletes the old copy in the same or the next change,
and review reads a `--config` change in the Makefile. A configuration under ROOT
reached through a symlink, the file or a directory above it, fails the
run (exit 2), since the base would be read at the link's path while the
head is read through it.

A narrowing made on purpose (a root that moves with its function, a
crate directory renamed, a gate a port retires) is named the way FLOOR
names a lowered figure: a `[[narrowed]]` line whose `change` is the
narrowing as the failing run prints it, before `, so ...`, and whose
`reason` says why.

```toml
[[narrowed]]
change = "[inputs] roots drops Regex::new in crates/pcre2/src/regex.rs"
reason = "Regex::new moved to builder.rs, which the new root names"
```

The run then prints `ok configuration: ...: <the narrowing>, with a
reason: <reason>`, so review sees it. As with FLOOR's `lowered` lines, a
line the base already has covers nothing new, so a later narrowing of
the same kind needs its own; such a line stays as history and is not
checked. A narrowing made again with the same text (a test cfg dropped
and added back, a section restored and retired again) takes one more
line with that `change` than the base has; a history line reworded is
still history. FLOOR does not count its lines: a figure lowered again to
a value a base `lowered` line already names has no new line to give, so
lowering it there takes an earlier change that removes the old line. A new line that names no narrowing of the base fails the
run (exit 1), and one with no reason does not parse (exit 2).

There is no line for a base copy the counter cannot read: a kit release
that drops or renames a key makes the port's move to it exit 2 against
every base that still has it. Such a change runs with `--no-base` in
place of `--base-rev`, and then with no base comparison at all, for the configuration, FLOOR,
RLIMIT or REACH, so it needs a maintainer's review in their place.

The base's FLOOR, RLIMIT and REACH are read at the paths the base's
configuration gives them (`[floor] path`, `[rlimit] path`, `[reach]
path`), so moving one of those files, or pointing its key at a path the
base lacks, still compares the head's file with the base's. So that a
base path is always a file some run read: `--floor FILE` must be ROOT's
`[floor] path` (exit 2 otherwise, compared after resolving both), a
`[floor] path` through a symlink fails the run (exit 2), as a
configuration through one does, and a
`[reach]` whose file is missing fails the run (exit 1; the file may
hold no rows). A file whose section the base's configuration has, but
the base lacks, fails the run (exit 2 for FLOOR and RLIMIT, 1 for
REACH). With no base configuration, or no `[reach]` in it, the head's
path is read, and a base without the file prints `ok ...: REV has no
... to compare with`. A crate read twice, under one name or from one
directory, fails the run (exit 2): the tally goes by name. So does a
`[crates]` `dir` or member `path` that is not relative, has a `..` or
passes through a symlink, and a crate whose directory or `src/` is a
symlink, or with a symlink under `src/` that the source walk reads
through (one named `*.rs`, or one to a directory holding a `.rs` file or
a cycle at any depth, or one whose target, or anything under it, the
counter cannot stat, resolve or list, such as a link loop; a dangling
link is passed over) (exit 2), since the base comparison matches crates
by path and another spelling of a directory, or another crate's sources
reached through a link, would pass for a new crate. Any other link
under `src/` (a `LICENSE`, a `locales` directory of `.ftl` files) is
not a source and is passed over.
[configs/pcre2.toml](configs/pcre2.toml) and
[configs/challoc.toml](configs/challoc.toml) are complete examples.

| section | key | meaning |
|---|---|---|
| `[crates]` | `dir` | each subdirectory with a `src/` is a crate, named for it |
| | `members` | `{ name, path, ships }` crates named explicitly; `ships` overrides the `publish = false` rule (a crate ships unless its Cargo.toml, read as TOML, sets `[package]` `publish = false`; one under `[package.metadata]` does not count) |
| | `test_cfgs` | `cfg` names only test builds set (default `test`, `kani`) |
| | `test_features` | features only tests enable; a workspace build that enables one fails the run (read from the manifests and `Cargo.lock`, with no cargo; a form it does not read fails the run) |
| | `verus_features` | features only Verus's build enables, as `verus_keep_ghost` |
| | `verus_sets` | `cfg` atoms Verus's build is sure to set, which shipped builds may set too (`'feature = "jit"'` for a default feature the proof run keeps, `"unix"`): "Verus's build", below |
| | `verus_never_sets` | `cfg` atoms Verus's build never sets (`"miri"`), so `not(miri)` is sure there |
| | `stand_ins` | `[from, to]` same-length replacements before parsing, for identifiers Verus keeps as keywords that the counter does not handle itself (a Verus keyword naming a field, a parameter or a `let` binding outside `verus!`, such as `exec: u8` or `let tracked = ..`, needs none: it is read as a raw identifier); one that would rewrite a name the counter classes code by (`external_body`, `assume_unsupported_attributes`, `exec_allows_no_decreases_clause`, `assume`, `admit`, `verus_verify`, `discharged_by`, `cfg`, `fn`, `unsafe`, a test cfg, a configured macro or mint, ...) is refused |
| | `verus_only_ghost` | count the ghost functions of Verus-only code in the ghost column |
| `[floor]` | `path`, `checked`, `renamed_from`, `room_functions`, `room_lines` | the FLOOR file (default `verus/FLOOR`); the prefix of the two verified-or-discharged figures (default `checked`); an older prefix the base revision's copy may still give them under, for the change that renames them (challoc's `combined`); room for unverified code a raise leaves (#100, default 0 each): the `raise FLOOR to` figure the run prints is the share with that many more unverified functions and lines in its denominators, and a share within that room prints no raise. Neither changes whether the run passes. The room is a port's own choice (#100 keeps it per port): the template FLOOR header keeps its rule, so a port that sets the keys states its room in a paragraph of its FLOOR below the header |
| `[[runs]]` | `name`, `sets`, `never_sets` | each proof run and the atoms it is sure to set and never sets (#21), in place of `verus_sets` and `verus_never_sets`: "Verus's build", below |
| `[rlimit]` | `path` | the RLIMIT file (default `verus/RLIMIT`) |
| `[kani]` | `records` | the directory of `kani-*.md` records that cover the `discharged_by = "kani"` marker |
| `[seams]` | `tables`, `theorems` | the seams tables, and a directory whose every `*.md` holds one |
| `[recursion]` | `table` | turns the recursion check on; the table that names verified recursion |
| `[loops]` | `table` | turns the nested-loop check on ("Nested loops"); the work-bound theorem's table that names each function that nests loops |
| `[reach]` | `path`, `exports_macro`, `fn_ptr_macro`, `conversion` | turns REACH on; the port's exports macro, function-pointer macro and argument conversion, each optional |
| `[inputs]` | `table`, `crates`, `roots`, `lazy`, `describe` | turns the compile-inputs check on; a root that matches no function of `crates` fails it |
| `[mints]` | `path`, `mint`, `token_trait`, `home` | turns the token-mint gate on |
| `[badge]` | `path` | turns the badge gate on: with `--floor`, the file (default `README.md`) holds FLOOR's figures in one badge line (The badge, below) |
| `[ledger]` | `dir`, `isla`, `kani` | discharges from a generated seam ledger instead of the Kani marker; the premise files an Isla or Kani status may cite |
| `[[narrowed]]` | `change`, `reason` | a narrowing of the base's configuration made on purpose, named as the run prints it, and why |

A minimal configuration is two lines:

```toml
[crates]
dir = "crates"
```

### Verus's build

A crate's functions count as verified only when cargo verus checks the
crate at all, which needs its Cargo.toml to set `[package.metadata.verus]
verify = true`. A crate with a function inside `verus!` or of the
attribute form, or a proof built only for Verus, that does not fails the
run (exit 1), and each such crate is named. The table is necessary, not
sufficient. A crate that opts in must also be named in `VERUS_CRATES`
(`make verus`'s default `cargo verus focus` checks only those), or,
under `VERUS_MODE = verify`, be one of them or a dependency of one; and
no crate it names may list it in `[package.metadata.verus]
trusted_crates`, which cargo verus skips unless given `--verify-trusted`.
The counter does not read `VERUS_CRATES` or `trusted_crates`. The rlimit
check catches these where it runs: with `--rlimit` against a base, as
`make verus` runs it unless `VERUS_RLIMIT` is emptied, it fails a crate
with Verus code that Verus's statistics leave out
(`rlimit::check_covered`). A run with `--no-base` (`VERUS_BASE=none`,
as the proof pull request that adds the first FLOOR may use) or without
`--rlimit` has no such check: `--no-base` only notes the crate.

A function counts as verified only when Verus's build is sure to
compile it. A `cfg` reaches a function from its own attributes, from an
enclosing `impl`, trait, inline `mod` or `verus!` block, from an
out-of-line `#[cfg(..)] mod x;`, which classes `x`'s file and every file
it declares, from an inline module or `verus!` block around such a
declaration (`#[cfg(..)] mod inline { mod x; }`), and from a file's inner
`#![cfg(..)]`, which classes it and every file it declares (a crate
root's, the whole crate). `cfg_attr`s apply as rustc expands them,
nested to any depth (`cfg_attr(P, cfg_attr(R, ..))` applies under
`all(P, R)`) and each attribute of `cfg_attr(P, a, b)` under `P`, for
`cfg`s, `#[path]`s and Verus's attribute form alike: `#[cfg_attr(P,
cfg(Q))]` counts as the `cfg` `any(not(P), Q)`. A `no_mangle` or
`export_name` under `cfg_attr(P, ..)` makes the function a C entry
point for REACH where a shipped build can meet `P` (a feature, in
`verus_sets` or not), and not where none can (`any()`, `test`, a
Verus-only feature). Every `cfg` a function is under, its own and those
around it, is read as one conjunction. A file declared more than once
(`#[cfg(target_os = "linux")] #[path = "unix.rs"] mod sys;` beside the
same for macos) is built where any declaration holds, under either
reading of where the declaring file's modules live (a lib.rs that is a
module, `a::lib`, declares `mod child;` as `a/lib/child.rs`), and a
root is built by its own target whatever declares it as a module
(`#[cfg(test)] mod main;` in lib.rs leaves the binary shipped). The
other reading may be one no target makes (a root lib.rs's `mod foo;`
read as `src/lib/foo.rs`), so a file it names is unsure through it: it
counts, but never as verified. Those readings only widen where a file
is built: one no declaration of the primary reading names is unsure
as a file nothing declares is (an `include!` may compile it), whatever
the other reading's declarations need. A file a `#[path]`
names declares its modules beside it, as a `mod.rs` does. A
`#[cfg_attr(P, path = "..")]` names the module's file where `P` holds
and no earlier path attribute applies, as rustc takes the first in
effect: a bare `#[path]` applies wherever none earlier does and ends the
list, and the default `name.rs` is the file where none applies. An inline
module's directory is chosen the same way. A declaration whose file the
counter does not read, a `#[path]` to a file not named `*.rs` or to one
outside the crate's `src/`, fails the run (exit 1), since rustc builds
its functions and the counts would leave them out; one only tests build
may name any file. "Only tests" is read by reachability: a declaration
fails when its declaring file is reachable from a crate root (or from a
file nothing declares, which may be a target the counter does not
know) along declarations that are each not test-only on their own, and
it is not test-only on its own either. That holds under either reading of where a file's
modules live, so a lib.rs that is a module (`a::lib`, whose modules a
target reads under `a/lib/`) is checked both ways. A file declared only
inside another macro's body (`cfg_if! { if #[cfg(windows)] { mod win; }
}`, or the file a `#[path]` there names) is unsure whatever the
configuration settles, since the counter does not evaluate the macro.
So is any crate file but a root that no declaration the counter reads
names: nothing shows where it is built, so it is never verified. The
roots are the targets the crate's Cargo.toml declares (the `[lib]`
`path`, each `[[bin]]`, `[[example]]`, `[[test]]` and `[[bench]]`
`path`), with `src/lib.rs` unless `autolib = false` and, unless
`autobins = false`, the binaries cargo finds (`src/main.rs`,
`src/bin/NAME.rs`, `src/bin/NAME/main.rs`). A root declares its modules
beside it, as `lib.rs` does. A function some shipped build compiles, under a `cfg` Verus's
build might not meet, counts as trusted: it ships, and nothing proves
Verus compiled it. Its proof cannot count, and a shipped function's
proof may rest on it, as on any trusted function (so REACH and the
recursion check treat it as one).

Only the atoms the configuration settles are sure there:
`verus_keep_ghost` and the `verus_features` (set), the test atoms (never
set), and the two lists below. Every other atom, a platform
(`target_os = "windows"`, `unix`), `miri`, or a default feature, may be
either, since the counter cannot tell what the proof run passes. A port
declares what its proof run (the host, the features `make verus`
enables) settles:

```toml
[crates]
dir = "crates"
# The proof run builds the default features, which include jit.
verus_sets = ['feature = "jit"']
# Verus never sets miri.
verus_never_sets = ["miri"]
```

Each entry is one atom, written as a `cfg` writes it (a name, or `name =
"value"`), and the run fails (exit 2) on a predicate (`not(miri)`), and
on an atom the configuration already settles (a test atom, a Verus atom,
or one named in both lists). Adding one is a narrowing (Configuration,
above), since it lets the functions it guards count as verified: name it
with a `[[narrowed]]` line. A feature in `verus_sets` still ships: one
that only Verus's build enables belongs in `verus_features`, which takes
its items out of the shipped tally.

A port that runs Verus on several hosts declares each run (#21) as a
`[[runs]]` table, with the atoms that run is sure to set (`sets`) and
never sets (`never_sets`), written as `verus_sets` writes them; the two
`[crates]` lists are then left out, as they are one run's shorthand:

```toml
[[runs]]
name = "verus"
sets = ['target_arch = "x86_64"']
never_sets = ["loom"]

[[runs]]
name = "verus-arm64"
sets = ['target_arch = "aarch64"']
never_sets = ["loom"]
```

An item is classed in each run as sure, never or unsure, under that
run's atoms alone; a run that sets one value of a single-valued key
(below) never sets its other values. It is verified where it is sure in
at least one run and unsure in none; trusted where some run might ship
it without compiling it for Verus, or no run compiles it but a shipped
build may; and left out where no build compiles it. So on challoc's two
runs, `not(target_arch = "x86_64")` is verified (never in the x86_64
run, sure in the aarch64 one), and `all(target_arch = "x86_64",
any(target_arch = "aarch64", unix))` is trusted (the x86_64 run compiles
it only under `unix`, which it does not settle). An item in a module is
sure only if each run that may compile the module is sure to compile
the item there. A run with no name, a name given twice, an entry that is
not one atom or is settled already, a run that sets two values of one
single-valued key (a run is one target), or `[[runs]]` beside
`verus_sets` or `verus_never_sets`, fails the run (exit 2). Under
`--base-rev`, a run added, removed or renamed (a run removed may be the
one where an item was unsure), an atom added to a run, or `[[runs]]`
replacing the shorthand (even with the same atoms, since a run's set
value of a single-valued key rules out its other values), is a
narrowing; the base's shorthand is its one run, `verus`.

Without `[[runs]]`, a port that runs Verus on several hosts names every
run's atoms in the shorthand (challoc, which runs it on x86_64 and
natively on aarch64: `verus_sets = ['target_arch = "x86_64"',
'target_arch = "aarch64"']`), since a function under either is checked
by the run that sets it, and the rules below apply.

A predicate that needs two values of one key rustc gives each target
one value of (`target_arch`, `target_os`, `target_env`, `target_abi`,
`target_vendor`, `target_endian`, `target_pointer_width`, `panic`; not
`target_family`, `feature` or `target_feature`) holds in no build,
whatever the atoms say on their own. So with both architectures
declared, an item under `all(target_arch = "x86_64", target_arch =
"aarch64")`, or `all(target_arch = "x86_64", any(target_arch =
"aarch64", test))`, is left out as no build compiles it, and Verus's
attribute form under such a `cfg_attr` never applies, so the function is
outside. The same holds for two `cfg`s that must hold together, on the
function (`#[cfg(target_arch = "x86_64")] #[cfg(target_arch =
"aarch64")]`) or one on it and one around it.

Otherwise the counter reads each atom on its own, not each run's set,
so it cannot tell the runs apart. Where `verus_sets` holds two values of
a single-valued key, a function is sure only if every way the `cfg`s it
is under can hold together needs at most one of the atoms of such keys
true (an atom under an even number of `not`s counts; each alternative of
an `any` is a way of its own, so `any(target_arch = "x86_64",
target_arch = "aarch64")` is sure): `target_arch = "x86_64"` alone is
checked by the run that sets it. Under two, it may ship on a host no run
is, so it counts as trusted. With an
x86_64 Linux run and an aarch64 macOS run (both arches and both oses
declared), `#[cfg(target_os = "macos")] mod mac { #[cfg(target_arch =
"x86_64")] fn f() }` ships on x86_64 macOS, which neither run is, and
`f` is trusted, as it is with the module under `not(not(target_os =
"macos"))`; so is a function under `all(target_arch = "x86_64",
any(target_arch = "aarch64", unix))`. `not(target_arch = "x86_64")`
stays unsure too: the function counts as trusted though the aarch64 run
checks it. Declaring the runs (`[[runs]]`, above) credits these where a
run checks them.
Since each atom is read on its own, a file declared under `feature =
"std"` and again under `not(feature = "std")`, which every build
compiles, stays unsure too, unless the configuration settles `std`.

#### Bringing this rule in on a pin move

The port PR that moves its pin to this counter carries the port's
declarations in the same PR, each with a `[[narrowed]]` line: with
neither list, every verified function under an unsettled `cfg` turns
trusted at once, and FLOOR (and REACH, for one an entry point reaches)
fails on that PR.

- The PCRE2 port: `verus_sets = ['feature = "jit"']` and `verus_never_sets =
  ["miri"]`, as [configs/pcre2.toml](configs/pcre2.toml) has. Without
  them, 27 verified functions of pcre2-capi count as trusted
  (functions 91.5% to 90.8%), and REACH fails on exec.rs.
- The Expat port and a zlib port: none. Neither has a verified function under an
  unsettled `cfg`, and `--list` is unchanged.
- challoc: [configs/challoc.toml](configs/challoc.toml)'s declarations,
  read from challoc main `09edeb9`: `verus_sets` names both
  architectures, and `loom` is a test cfg, not a never-set atom.

A function that stays trusted under honest declarations is verified
more: bring its proof under a `cfg` the proof run settles. Where a
figure still drops, FLOOR takes a `lowered` line with the measured
change and its reason, as for any other drop. Declaring an atom the
proof run does not set, to get the figure back, is the one wrong
answer: it credits a proof no run checked.

An atom applies to every crate under `[crates]`, while a cargo feature
is per crate. `feature = "jit"` in `verus_sets` asserts that the proof
run enables `jit` in every configured crate that has a feature of that
name, so a second crate with a non-default `jit` would be credited
wrongly. In the PCRE2 port only pcre2-capi has `jit`.

## The badge

A port's README shows its Verus coverage as a shields.io badge. The
repositories are private, so shields.io cannot read the figures: they
are written into the line, alt text and URL both, and a badge kept by
hand drifts from FLOOR. With

```toml
[badge]
path = "README.md"
```

a run with `--floor FILE` also fails (exit 1) unless the file holds
exactly one Verus badge, and that line is exactly the one FILE's
`functions` and `lines` figures give, each with one decimal. A Verus
badge is any rendered line holding `verus` in any case together with
`shields.io`, a Markdown image (`![`) or an HTML `<img`, so a second
badge with a lowercase label, another shields.io endpoint, or an
`<img>` tag from any host counts. Lines inside fenced code blocks
(```` ``` ```` or `~~~`) and the text inside HTML comments (`<!--` to
`-->`, across lines; `<!-->` and `<!--->` are whole comments) do not
render, so they are neither the badge nor a second one: an example
badge in a fence is not counted, and the canonical line only inside a
comment or a fence fails as no badge. Since a renderer may end a fence
or a comment sooner than the scan does, the gate reads the file twice.
The strict reading hides a fence or a comment the file never closes to
the end of the file. The lenient reading takes that opener as text,
takes as text a `<!--` that renders as text (inside a backtick code
span, escaped as `\<!--`, on a line indented 4 or more spaces, or one
whose container ends before its `-->`: an inline comment's paragraph
at a blank line or a line that starts a block, indented up to 3
columns past where the paragraph's text starts (a heading, a quote, a
fence, a list item, a thematic break, or a line starting `<` and a
letter, `</`, `<?` or `<!`, such as `<p align="center">`; the last is
wider than CommonMark, whose plain tags cannot end a paragraph, so an
old badge on such a line inside a mid-line comment counts), a list item
at a line indented less, a block
quote at a line with fewer `>` than its depth (`> > <!--` ends at
`> text`), nested ones (`- > <!--`) at either, and an item whose
ordered marker does not start at 1 (`2. <!--`, which cannot interrupt a
paragraph, so may continue one) also where a paragraph would end; a
`<!--` at the start of a line opens an HTML block, which crosses blank
lines; a tab counts to the next multiple of 4 columns from where it
sits, and separates a list marker as a space does; the optional space
after a `>` or a marker takes one column of a tab, the rest counting as
indent; a `<!--` 4 or more columns past a `>` and its optional space,
or past a list marker's content, is indented code), and
ends a fence
indented by some spaces at the first non-blank line indented less (a
list item's fence ends with the item). A line that holds a badge in
either reading counts, so a stale badge after one of these openers is a
second badge even when a closer comes later in the file. The canonical
line passes only when it renders in both readings: after an opener the
file never closes, it fails (exit 1) as a badge that may not render.
The line FILE's figures give is:

```markdown
[![Verus: 86.3% of functions, 94.4% of lines](https://img.shields.io/badge/Verus-86.3%25%20of%20functions%2C%2094.4%25%20of%20lines-blue)](verus/README.md#coverage)
```

The failures name the line to write:

```text
FAIL badge: README.md has no Verus badge; add this line, which gives verus/FLOOR's functions and lines figures:
FAIL badge: README.md:3 is not the badge verus/FLOOR's functions and lines figures give; replace it with this line:
FAIL badge: README.md has 2 Verus badges (lines 3, 9); keep one, this line:
```

each followed by the line, and a pass prints `ok badge: README.md:3
gives verus/FLOOR's functions and lines figures`. A missing file fails
too (exit 1), and a `path` that is not relative, has a `..` or passes
through a symlink exits 2, as a `[floor] path` through one does. The
badge follows FLOOR, not the counted shares, so it moves only when FLOOR
does: the pull request that raises FLOOR, one of its own but for the
port's first proof pull request (FLOOR's header), updates the badge too, pasting what `verus-count --badge .`
prints. Without `--floor` the badge
is not checked. Removing `[badge]` or moving its `path` is a narrowing
(Configuration, above). The link is `verus/README.md#coverage` wherever
the file sits, so keep the file at the root.

## Two ways to discharge a seam

- **The Kani marker** (the PCRE2 port): a trusted function carries
  `#[cfg_attr(any(), discharged_by = "kani")]` and a `kani-*.md` record
  quotes it. The report is one table with a `discharged` column, and the
  checked figures add the discharged functions. The marker is read through
  `cfg_attr`s as rustc expands them, nested or beside other attributes,
  and counts only where the predicates around it hold in no build
  whatever the configuration declares (`any()`); under one some build may
  meet (`windows`) the function is not discharged.
- **The generated ledger** (challoc, `[ledger]`): `verus/seams.sh`
  writes `verus/ledger/**.rs.md` from the source, and each entry's
  status says whether its seam is discharged, partly checked or
  trusted. The report is challoc's two tables, the second the combined
  share. The marker is refused in this mode, so a function cannot be
  discharged twice.

## Functions with no termination proof

`#[verifier::exec_allows_no_decreases_clause]` lets Verus check exec
functions whose loops and recursion have no `decreases`, so nothing
proves they end. Each exec function it covers counts as trusted, not
verified, nor discharged even with the Kani marker (a Kani record does
not prove termination either): one that carries it, and each in an
`impl`, trait or inline module that carries it (outer, or inner `#![..]`
inside the braces), or in a file whose inner attributes carry it. The counter reads it as it
reads `assume_unsupported_attributes`: in the path form or as
`#[verifier(exec_allows_no_decreases_clause)]`, and under a `cfg_attr`
whatever the predicate. Where it covers an out-of-line `mod x;` (on
it, on an inline module around it, or atop the file that declares it),
it fails the run (exit 1), naming the declaration: the counter
reads each module file on its own, so it would not reach the functions
it covers; put it on those functions, impls, traits or inline modules.
Ghost functions and code outside `verus!` keep their classes.

## Nested loops

A `decreases` measure bounds each exec loop's iterations, but not the
product of nested ones: an inner loop with its own `decreases`
verifies, and the work becomes quadratic. With `[loops] table = FILE`
(#76), the run fails (exit 1) on each function of a shipped crate (any
class: verified, trusted, discharged or outside `verus!`) that nests a
loop (`loop`, `while` or `for`) inside another, or calls, from inside a
loop's body or header, a function that loops, itself or through what it
calls, unless FILE's table has a row that names its file in backquotes
and quotes `fn NAME`, as `[recursion]`'s RN1 table does. That row is
the work-bound theorem's: it states the bound on the product (a ghost
work counter spanning both loops, or why the inner one is constant).

Calls are read as the recursion check reads them: by name within the
function's file, a plain call `g(..)` as the file's free function `g`,
and `self.g(..)` or `Self::g(..)` as its owner's method. A call across
files, or through another receiver or path, a call of a closure not
bound by `let` in the function (one passed in, or stored in a field),
a function a `macro_rules!` expansion writes (the check reads only the
code outside expansions, as the other walks do), and a loop inside std
(an iterator adapter's) are not seen, and the theorem covers them. The body is parsed (Verus's syntax included) and
read by its structure: a loop's header may hold any expression (an
`unsafe` block, a `match`); a `while`'s condition runs each iteration
and counts as inside the loop, a `for`'s iterator runs once and does
not, but a closure in the iterator (`.map(|b| ..)`) runs per item and
counts as inside it. A nested item (a function, an `impl`) is its own
and is left out. A closure's loops count as the body's, and a name a
`let` binds (with a type too) whose initializer loops, itself, through
a call of a function that loops, or through another such name, makes a
call of the name a call that loops, whatever wraps a closure there
(`Box::new(|..| ..)`, `&`, parentheses). A macro invocation
in a body is tokens whose expansion the check does not read: one that
holds `loop`, `while` or `for` counts as a loop at its line, and names
its function, whatever it nests. So does a body that does not parse,
and a function that calls such a body's function from inside a loop
(each fails closed until the table names it).

## Linkage attributes Verus takes on trust

Verus refuses `no_mangle`, `export_name` and `link_section` inside
`verus!` unless the item carries
`#[verifier::assume_unsupported_attributes]`, and then takes the
attribute on trust. What is trusted is the symbol name or section the
attribute gives the item, not the item's code: Verus still checks the
body, so the attribute changes no function's class (a verified export
stays verified). The evidence for the linkage is the port's ABI tests,
its exported-symbol checks.

Each such item is a seam of its own, of kind `unsupported-attribute`.
The counter reads the attribute in the path form or as
`#[verifier(assume_unsupported_attributes)]`, directly or under any
`cfg_attr`, and
after the tables prints the count of these seams and each function's
`FILE:LINE`. With a generated ledger, the generator writes an
`unsupported-attribute` entry at each such item's line, statics and
other items included: for a function, the line the counter lists it at,
its `fn` line, or for one a macro writes the invocation's line (as an
`external_body` entry takes, "Functions a macro writes"), with status
`trusted`, since nothing in Verus or Kani checks an attribute: any other
status text, an unknown or empty one included, fails the run (exit 2),
and a function with the attribute and no entry is a warning. The entry
classes no function, so it may share a line with the function's
`external_body` seam, and the count is the ledger's. Two candidate
definitions of a macro (under `cfg`s the counter cannot settle) that
differ in the attribute count differently and fail the run, so no
build's linkage seams drop out of the report.

Verus's attribute form (`#[verus_verify]`, `#[verus_spec(..)]`, alone
or under a `cfg_attr`) counts as code inside `verus!`, for a port whose
MSRV keeps vstd optional (the skill's proofs.md, "When vstd cannot be a
dependency of every build"). Under a `cfg_attr` it is verified only when
Verus's build is sure to meet the predicate ("Verus's build", above): a
configured `verus_features` feature, `verus_keep_ghost`, a `verus_sets`
atom, `not` of a `verus_never_sets` one, or a predicate those settle. A
platform or any other atom the configuration does not settle
(`target_os = "windows"`, or `all(feature = "verus", unix)`) may leave
the attribute off where Verus runs, and the function unchecked, so it
counts as trusted, as a function under a `cfg` Verus might not meet
does. With `[[runs]]`, the attribute is sure only when every run that
may compile the function is sure to apply it: on challoc's two runs, an
unguarded function under `cfg_attr(not(target_arch = "x86_64"),
verus_verify)` is trusted, since the x86_64 run compiles it without the
attribute, while the same function under `#[cfg(target_arch =
"aarch64")]` is verified. A predicate Verus's build never meets (`any()`, `test`, a
`verus_never_sets` atom) never applies the attribute, so the function is
outside.

## Functions a macro writes

A function an item macro writes is counted like any other, so moving
code into a macro never takes it out of the denominator. An invocation
at item position (in a module, an `impl` or a trait) of a `macro_rules!`
the crate defines is expanded. The definition is one the build compiles
(not one under `#[cfg(test)]` or in a test-only module, `verus!` block
or file), found as rustc finds it. An invocation by a path
(`width::exports!`, `$crate::m!`) takes the definitions that path names:
one `#[macro_export]` puts at the crate root, or one its module
re-exports (`pub(crate) use exports;`, or `pub(crate) use ex as build;`
for `w::build!` and a `*` of `w`), with a first segment the module
imports (`use crate::w as x;` then `x::ex!`) read as each path it
imports, and as written too where a module of the crate has that name
(two under `cfg(unix)` and `cfg(not(unix))` give two candidates). The
crate's `use`s are followed to a fixed point: a module's `use` of a path
that names a definition (`pub(crate) use crate::v::build;`, or `use
crate::w::build as make;`, a rename) gives the definition that module's
path under the name the `use` brings in, and a module's `use p::*;`
gives it each path `p` names there, until no `use` adds one. A `use`
counts only where its own `cfg`s, and its enclosing modules', let it be
built: a test-only `use` imports nothing for a shipped build, and one
under a `cfg` the counter cannot settle is a candidate. An invocation
by a bare
name takes the last definition of that name before it in its file whose
module the invocation is in (a definition in a module that has closed
is out of scope, unless `#[macro_use]` carries it out; one at the top
of a file a `#[macro_use] mod name;` here declares (directly, or inside
inline `#[macro_use]` modules), or that such a file carries in the
same way, one an expansion writes at such a file's top (or in inline
`#[macro_use]` modules there) included,
stands at that `mod`'s line, then at each
inner `mod`'s line along the chain, then at its own, compared in that
order; the order within one line is not read, so one on the line of
a `mod` that carries another in, or two carried in by two `mod`s on
one line, are not ordered against each other, and both are taken; one
a `mod` on the invocation's own line carries in is taken beside the
others, and ends none of them, as is one carried in through a module
several `mod`s declare under `cfg`s, from the first of them on; the
counter reads files in path order, so an invocation that one an
expansion writes in a file read after the invocation's may reach (a
carried one from its `mod` on, or one whose scope holds the
invocation's module, from any other file) fails the run), and, while
that
one is under a `cfg` the counter cannot settle (a platform or a
feature) and so may not be built, each before it back to the first
under no such `cfg`. In code a shipped build compiles, a definition
under a `cfg` that a shipped build may not meet where the invocation
is built (a `verus_sets` atom such as `feature = "jit"`, or one
`target_arch` of two) is such a one too, though Verus's build is sure
to compile it: a shipped build without the feature compiles the
definition before it. Failing an unconditional one, it also takes each
definition its module imports by `use` under that name (`use w::ex;`,
or `use w::ex as build;` for `build!`, the imported path read from the
module), each in another file whose textual scope reaches the
invocation (an enclosing module's, or one a `#[macro_use] mod name;`
carries out of its file; not a sibling file's), and, at the crate root,
each `#[macro_export]` puts there. A `macro_rules!` an expansion writes
defines its macro from there on, a later one shadowing an earlier one
as in source. The expansion follows rustc's
rules (each rule tried in order; `ident`, `ty`, `expr`, `block`, `tt`
and the other fragment kinds, a `tt` and a repetition's separator being
one token as rustc's lexer makes them (`<<`, `::`, `'a`), a `literal`
taking `true` and `false` and a `stmt` stopping before a `let`'s `;`; a
fragment other than an `ident`, `lifetime` or `tt` forwarded to another
macro staying one fragment of its kind, which no literal token matches
and only a kind whose parser reads it does, as rustc decides (an `expr`
reads a `literal`, and a `literal` an `expr` that is a literal; a
`path` reads a `ty` that is a path; a `ty` is never an `expr`; a
forwarded `vis`, empty or not, is one `vis`; any other pair rustc's
matcher tries, the counter does not read, and the run fails, saying so:
a `literal` from a forwarded `1 + 2` or `-x`, a `pat` from a forwarded
`expr`, an `item` or `stmt` from a forwarded `vis`; rewrite the macro to
match the fragment as its own kind);
nested repetitions; `$crate`; a `$name` the rule does not bind written
through, as a macro that writes a `macro_rules!` needs; an invocation
the expansion writes is expanded in turn), and each function in it
counts as outside `verus!`, since `verus!` does not expand the macros it
holds.

The fragments match by the crate's edition, as rustc's do. An
`expr_2021` begins at neither `const` nor `_` (and no `expr` at `let`),
and below edition 2024 an `expr` is an `expr_2021`; below 2021 a `pat`
is a `pat_param`, which takes no top-level `|`. The edition is that of
the targets whose files the counter reads: the library (`[lib] edition`,
else the package's), where there is one (a `[lib] path`, or
`src/lib.rs` unless `autolib = false` with no `[lib]`), each binary (each `[[bin]]`'s, else the package's,
and those cargo finds under `autobins`), and any `[[test]]`, `[[bench]]`
or `[[example]]` whose `path` is under `src/`, with `edition.workspace =
true` followed and 2015 when no key is given. The counter does not split
a crate's files by target, so a crate whose read targets sit on both
sides of 2021 or of 2024 fails the run (exit 1); give them one edition
there.

An expansion is read under its invocation's `cfg`s, as code a
module holds is: one under a `cfg` Verus's build might not meet
(`#[cfg(unix)] vone!(v1);`) ships its functions trusted, never
verified, and a definition under such a `cfg` is a candidate, never
dropped. The walks (REACH, recursion, nested loops, compile inputs,
mints) read only the
code outside expansions, with `[reach]`'s `exports_macro` and
`fn_ptr_macro` as before. REACH scans every shipped file for the
exports macro's blocks, found by their tokens (the name, plain or raw,
then `!` and a delimited group, so `exports != n` is none), and fails
closed: a file that does not lex or parse, one that imports the macro
under another name (`use width::exports as wrap;`; a cast such as
`exports as u32` is no import), and one whose
invocations outnumber the blocks the scan reads (an invocation inside
another macro, a function body or an initializer) each fail the run.
Write each block at the top of a module or `verus!` block, under the
macro's own name. The rlimit check reads the base revision's
sources the same way, so a `#[verifier::rlimit]` a macro writes is held
to the base's.

A row lists a written function at the invocation's line. Its lines are
the source lines its tokens come from, in the invocation's file and in
each definition's, so a line added to an export's body in the invocation
adds a line to that export. Two consequences:

- A definition line counts once for each function each invocation
  writes from it: a three-line body in a definition that writes 22
  functions is 66 lines, as 22 hand-written copies would be. Outside
  `verus!` that keeps the denominator honest; once the definition writes
  `verus! { .. }`, its lines count as many times on the verified side,
  which is as many functions as Verus checks.
- A line with no token (a blank line or a comment inside an invocation)
  is no line of a written function, where a hand-written function counts
  every line from `fn` to its closing brace.

An item macro the crate does not define (another crate's) cannot be
expanded, so the invocation counts as one function outside `verus!`,
named `NAME!`, over its lines: visible in `--list`, never missing. Each
of these fails the run (exit 1), as the counter cannot give a count
rustc would agree with:

- an invocation that resolves to none of the crate's definitions of its
  name, or of the name it was imported as, though the crate has one
  (reached by a path the counter does not follow, or a definition an
  expansion in a file read later writes); one whose name the module
  imports from this crate (a `use` path from `crate`, `self`, `super` or
  a module of the crate) by a path that names no definition; one by a
  path into a module of the crate, or a `*` of one, that brings the name
  in by a `use` the counter cannot follow to a definition: invoke it by
  its definition's path, or in that definition's textual scope;
- an invocation one of whose imported paths (a module alias's, or a
  `use` of its name, under `cfg`s the counter cannot settle) names no
  definition by a path the counter follows while another names one:
  the build that takes the first would expand what the counter cannot
  see;
- a macro the crate defines whose invocation no rule matches, or whose
  expansion does not parse;
- more than one definition that can reach the invocation (a pair under
  `cfg`s the counter cannot settle, such as `unix` and `not(unix)`, in
  the invoking file or another) whose expansions count differently (a
  different function, class or line count), or one of whose expansions
  fails to count (an invocation it writes that does not expand or is
  itself ambiguous); ones that count the same are expanded as the first;
- an expansion nested more than 128 deep (rustc's default
  `recursion_limit`);
- a repetition matched in more than 4096 ways, a bound on the search
  that a long `$($t:tt)*` reaches, since each iteration is a way to
  stop.

`count.toml` has no key that lets a run past these: the way forward is
to rewrite the macro so it expands as rustc's would within the bounds
(split a long invocation, keep one definition with the `cfg` inside its
body). A zlib port, the PCRE2 port and the Expat port come nowhere near the bounds (at most
41 iterations of one repetition, and an expansion two deep).
std's `thread_local!` writes statics, and counts nothing. One shown to
be another macro is read as any macro of its name: the bare name where
the crate defines a `macro_rules! thread_local` (expanded, and its
functions counted) or where a `use` imports the name from a path that
does not start at `std` or `core`, and a path whose first segment is
neither (`elsewhere::thread_local!`). A `std` or `core` that starts
such an import or path is that crate's only where no Cargo dependency
is renamed to it and neither a `use` in the module nor an `extern
crate elsewhere as core;` gives another crate that name
(`use elsewhere as core; core::thread_local!` counts); for the
import, in 2015, a `use` at the crate root or a root glob but std's,
core's or Verus's rebinds it too (a 2015 `use` path is read from the
root), while the path is read in its own module in every edition, so
a root `use` rebinds it only at the root. Nor is an import's `std` or
`core` that crate where it names a module of the crate as rustc reads
a `use` path (in 2015 one at the root, from 2018 on one in the
module): the import is read as any other macro's, and a re-export the
counter does not follow fails the run. Another crate's counts as one
function outside `verus!`, `thread_local!`. A bare one beside a `*` or
a `#[macro_use]` is read the same way when one of them may bring
another crate's `thread_local`: a `#[macro_use]` of a crate other than
std's (`std`, `core`, `alloc`) or Verus's (`vstd`, `verus_builtin`,
`verus_builtin_macros`), or a `*` other than Verus's
(`vstd::prelude::*`, `verus_builtin_macros::*`), std's or core's, and
other than one of a module of the crate that imports nothing of the
name (by a `use` of it or by such a `*` of its own, read through each
`*` it has). In a `*`, as in the import above, a crate's name is
exempt only where no Cargo dependency is
renamed to it and neither a `use` in the module, or in 2015 at the
crate root, where the path is read from (`use elsewhere as core;`),
nor an `extern crate elsewhere as core;` gives another crate
that name; in 2015 any root glob but std's, core's or Verus's (one of
a module of the crate by any path or alias, or of another crate, or
one whose first segment a `use`, an `extern crate .. as` or a module
of the crate takes) may bring a module of that name, so with one the
name is never exempt. A
`*` path's plain first segment names a module of the
crate as rustc reads it, and that module is read whatever its name (a
`mod core` too): from 2018 on, a module in scope where the `use` is
(none after a leading `::`), and in 2015, a module at the crate root
(else, in both, a crate). Where such an alias gives the name too (a
`cfg` may select either), the alias counts it whatever the module
holds. A path whose first segment names a module of the crate but
whose rest names none (`mod core { pub(crate) use elsewhere::cell; }`
beside `use core::cell::*;`) goes through a re-export the counter does
not follow, so it counts it too. A module whose `mod` item (or one
around it on the path) is under a `cfg` of its own, or in a `verus!`
block that is, or whose file has a `#![cfg]`, may be left out of a
build, so the path is read as a crate's too, and counts where either
reading may bring another `thread_local`.
One beside only those counts nothing, as std's; its
module's globs place no `impl`'s type, as before.

`compile_error!`, which writes nothing that ships, counts nothing
either, but only by a rule true by rustc's own semantics. The counter
does not resolve names; it reads the crate's bindings (its sources' and
every macro expansion's, in a shipped build, `#[macro_use]` read through
`cfg_attr`s too):

- `::core::compile_error!` and `::std::compile_error!`: in edition 2018
  and later a leading `::` names an extern crate, so these count nothing
  unless the crate's edition is 2015 (cargo's default when none is
  given; the lowest of the package's, the workspace's it inherits, and
  any target's own, `[lib] edition`), a root `extern crate .. as core`
  (or `std`) puts another crate under the name, or a Cargo.toml
  dependency of the name is another package (`std = { package = "other"
  }`, in any dependency table, or inherited by `{ workspace = true }`
  from `[workspace.dependencies]`; one the root does not give is taken
  as another). A `--base-rev` walk reads the root Cargo.toml of that
  revision;
- a bare `compile_error!`: counts nothing where nothing in the crate
  binds the name (an item, `macro_rules!`, `use` or `extern crate`, std's
  own `use core::compile_error;` too), no `#[macro_use] extern crate` but
  `std`, `core` or `alloc` is in the crate, no dependency named `std`,
  `core` or `alloc` is another package (whose prelude it would be), and
  no `*` is in its module;
- any other path (`core::compile_error!` without `::`) counts as a
  function.

A binding a macro expansion writes that breaks the rule fails the run
(exit 1), from every candidate definition the counter compares (a pair
under `cfg`s it cannot settle), not only the one it walks. Every other
std macro in item position counts as a function. `global_asm!` always
counts
as one: it can write executable code (a symbol Rust calls through an
`extern` block), which the counter cannot tell from a directive.
`include!` writes what its file holds, and the expression macros
(`asm!`, `concat!`) are not items.

One case stays quiet: a `*` that would bring a crate macro's name in
only through a further `*` the counter cannot follow (`pub use
me::w::*;` through an `extern crate self as me;`). The invocation then
counts as another crate's macro, one `NAME!` row, unless the crate
defines that name, which fails the run as above.

Adopting a kit release that expands macros lowers a port's shares with no
change to the port, as functions that were in no denominator join it
(a zlib port from 31.5% to 27.3% of functions, the PCRE2 port from 91.5% to 85.6%), so
the first pin move fails its FLOOR. A port holds its FLOOR by verifying
more: it brings the macros' bodies inside `verus!` (below), verified or
trusted at a seam, and the pin move waits until the port's figures hold
its FLOOR. Lowering FLOOR is not the way to adopt a counter change.

To raise the share, put the functions a macro writes inside `verus!`
by writing them there, or have the macro write a `verus! { .. }` block,
which is read as one. The block may name Verus's macro by its path
with a leading `::` (`::vstd::prelude::verus!`,
`::verus_builtin_macros::verus!`), as challoc's `verified_export_pair!`
does. A `verus!` block that may write a function (it holds `fn`) is
Verus's, read in Verus's build:

- with a leading `::`, by the rule above for `vstd` or
  `verus_builtin_macros` (edition 2018 or later, no root `extern crate ..
  as` of the name, no dependency of the name that is another package);
- bare, where nothing in the crate binds `vstd`, `verus_builtin_macros` or
  `verus` but the crate itself (`extern crate vstd;`) or the macro's own
  path (`use vstd::prelude::verus;`), and no `#[macro_use] extern crate`
  but `vstd`, `verus_builtin_macros`, `std`, `core` or `alloc` is in it,
  and no dependency named `vstd` or `verus_builtin_macros` is another
  package. Beside a `*` that may bring another crate's `verus` (a
  dependency can export its own), it counts as Verus's only beside a
  sure import of Verus's in the same module instance (`use
  vstd::prelude::*;` or `use vstd::prelude::verus;`), or where the `*` is
  `use super::*;` in a file whose module identity is certain and the
  parent settles. Anything else fails the run.

  An import is sure where every proof run that compiles the module
  compiles the import. Every `cfg` it is under, its own and those around it, is read
  as one conjunction that must be sure, as for a function (Verus's
  build, above), so `#[cfg(target_arch = "x86_64")] #[cfg(target_arch =
  "aarch64")]` on it settles nothing. Unlike a function, the import's
  own `cfg`s (on it, or on a `verus!` block around it) must also hold in
  each run: where `verus_sets` holds two values of a single-valued key,
  an atom of that key counts there only when the modules around the
  import force the key's value. With challoc's two architectures,
  `#[cfg(target_arch = "x86_64")] use vstd::prelude::*;` settles nothing,
  since the aarch64 run compiles the invocation without it, while the
  same import in a `#[cfg(target_arch = "x86_64")] mod` does. The import
  settles only the `*`s of its own module instance: two inline modules of
  one name under exclusive `cfg`s (`#[cfg(target_arch = "x86_64")] mod q`
  and `#[cfg(target_arch = "aarch64")] mod q`) share a path, but one's
  import of Verus's does not settle the other's `*`, and a `*` a macro
  writes is in the instance the macro is invoked in (a `mod` it writes
  is an instance of its own).

  Any other `*` (`use other::*;`, `crate::`, `self::` or deeper paths,
  and every path in edition 2015) may name another crate's module, or an
  alias of one, in some target or run. A file's module identity is
  certain where every target root that may compile it (`src/lib.rs` or
  `src/main.rs`) reaches it by its place under src/, and no declaration
  reaches it, at any depth, through a file a `#[path]` names, one inside
  an inline module whose directory a `#[path]` chose, a module file named
  `main.rs` or `lib.rs`, or another target's root, even beside a
  declaration by its place (rustc compiles a file once per
  declaration); a root elsewhere (`src/bin/NAME.rs`, or a `[lib]` or
  `[[bin]]` `path`) that may compile it leaves it uncertain;
- by any other path (`vstd::prelude::verus!` without `::`) never: the run
  fails (exit 1), as it does where a rule does not hold, naming why.

Any other path ending in `verus` (`other::verus!`, `::verus!`) is
another crate's macro.

## Parity

The parity runs below predate macro expansion: a port whose crates
invoke their own item macros (pcre2-capi's `width::exports!` and
`fn_ptr!`) now counts their functions as well, so its table and
`--list` differ by those rows.

Run on the same commits, the kit counter's full output (standard
output, standard error and `--list`) was byte-identical to each port's
in-tree counter, but for one line. The PCRE2 port's runs were on its
tree of the time, under a configuration that
[configs/pcre2.toml](configs/pcre2.toml) keeps as an example, with
the crates and paths named `pcre2-*` here. The port carries its own
`verus/count.toml`, with its own crate names.

| port | commit | configuration | `--floor verus/FLOOR --base-rev HEAD^` | `--list` |
|---|---|---|---|---|
| the PCRE2 port | its main at the time | `configs/pcre2.toml` | identical but the `ok compile inputs` line | identical but that line (3,919 lines) |
| challoc | challoc main `fcde6b2` | `configs/challoc.toml` | identical | identical (1,674 lines) |
| challoc | challoc main `39751a02` | `configs/challoc.toml` | identical (also with no flag, and `--list --floor verus/FLOOR --base-rev fcde6b2`) | identical (1,672 lines) |

challoc's rows ran under the copy of `configs/challoc.toml` of their
time. The copy here is read from challoc main `09edeb9`, where the
kit's `--list` under it is byte-identical to one under challoc's own
`verus/count.toml` (1,894 lines).

The kit's `ok compile inputs` line also names each root it resolved
and where (`(roots Regex::new at FILE:LINE, encode_as at FILE:LINE)`);
the PCRE2 port's does not name them. The PCRE2 port's `--rlimit` path, with statistics written for the
purpose (every crate, every function with a `#[verifier::rlimit]`) and
`--base-rev HEAD^`, differs in the same line alone.

Where the counters still differ, on inputs neither port has today:

- The PCRE2 port's counter counts the commas of generic arguments in a call's
  arguments (`helper(make::<u8, u16>())`) as the call's, and drops the
  edge to a one-parameter `helper`; the kit leaves such a call's arity
  unknown. The PCRE2 port's counter with the same fix gave the same output on
  that commit.
- The PCRE2 port's counter lets a function that keeps its body but loses a
  `#[verifier::rlimit(N)]` below the default rise to the default with
  no `rlimit-raised` line; the kit requires one. The PCRE2 port has no limit
  below the default. (challoc's counter has neither the call walk nor
  the rlimit check.)
- The kit refuses a stand-in that would rewrite a name it classes code
  by, and a compile-inputs root that matches no function of the
  configured crates; it credits Verus's attribute form under a
  `cfg_attr` only when Verus's build is sure to meet the predicate.
  Neither port has a stand-in, root or attribute these refuse.
- The kit counts a function under a `cfg` Verus's build might not meet
  as trusted ("Verus's build"); the PCRE2 port's and challoc's counters take
  every atom they do not configure to hold where Verus runs. On the PCRE2 port's
  main at the time, pcre2-capi's `jit` and `miri` guards are the only such
  `cfg`s on a verified or trusted function. With `verus_sets = ['feature
  = "jit"']` and `verus_never_sets = ["miri"]`, as
  [configs/pcre2.toml](configs/pcre2.toml) has, `--list` is the same as
  the PCRE2 port's counter's rule gives; without them, 27 verified functions
  of pcre2-capi (exec.rs and tables.rs's `Libc` ctype items) count as
  trusted. On challoc main `0497dc34`, its `target_arch` and
  `loom` guards are the only ones: with the declarations of that time
  (`verus_sets` both architectures, `verus_never_sets = ["loom"]`) `--list`
  is the same as challoc's counter's rule gives, and without them 7
  verified functions count as trusted.
- With `--base-rev`, the kit also compares `verus/count.toml` with the
  base revision's (Configuration, above), which neither port's counter
  had, and prints one `ok configuration` line first. The rest of the
  output is unchanged.

- challoc's counter takes a missing `combined-*` figure in the FLOOR
  under test as 0.0; the kit's requires all four (a base revision's
  copy may still lack them). challoc's FLOOR has given all four since
  `4f7b840b`, so no base revision a pull request compares with lacks
  them.
- Renaming challoc's `combined-*` figures to `checked-*` (kit/PLAN.md,
  step 2) needs `renamed_from = "combined"` in the change that renames
  them: without it the base revision's copy names figures the
  configuration does not, and the run stops (`unknown figure
  "combined-functions"`, exit 2). With it, on `39751a02` with FLOOR's
  lines renamed, `--floor verus/FLOOR --base-rev HEAD` passes with the
  same four figures, and a `checked-lines` set 0.1 below the base's
  fails for want of a `lowered` line. The key may stay after the rename;
  it only widens what a base's copy may name.
- challoc's counter reads an inline `#[cfg(verus_keep_ghost)] mod { }`
  as shipped code; the kit reads it as Verus-only, as the PCRE2 port's does, so
  only its trusted functions count.
- challoc's ledger reads only `src/FILE:LINE` entries; the kit's reads
  any `FILE.rs:LINE`, so a ledger may cover several crates.
- The failure text for a ledger status that names a loader or kernel
  premise without its `callers:` clause, and for an unlisted compile
  input, words its reason from the configuration rather than from
  challoc's and the PCRE2 port's file names.

## Bootstrapping a port

A port with no counter and FLOOR 0.0 adopts the counter in its first
proof PR (the skill's lifecycle.md, phase 4):

1. Pin the kit's package ("Adopting: the package", below), add
   `verus/count.toml` (the two-line form above is enough to start)
   and run the counter in the verus check:
   `verus-count --floor verus/FLOOR --base-rev "$BASE" .`. A base with
   no FLOOR prints `ok coverage floor: ... has no verus/FLOOR to compare
   with`; this PR may instead pass `--no-base`, which no later PR may.
   Each crate with Verus code sets `[package.metadata.verus] verify =
   true` in its Cargo.toml, and is in the Makefile's `VERUS_CRATES`
   (Verus's build, above, has the cases `verify` mode adds):
   cargo verus has Verus check no package without the table, whatever
   `-p` selects, so the counter fails the run (exit 1) for each crate
   with code Verus checks that lacks it, rather than credit as verified
   what nothing checked (Verus's build, above).
2. Copy `kit/templates/verus/FLOOR`, whose figures are all 0.0.
3. On the PR's base the counter names 0.0 for every figure; on the PR it
   names the figures the proof reaches. Record those in FLOOR in the
   same PR, although FLOOR otherwise rises only in a PR of its own
   (its header): this first raise cannot come before the counter and the
   proof.
4. Add `[badge]` to `verus/count.toml` and put the line
   `verus-count --badge .` prints on the README's third line, under
   its title (The badge, above).

A port that landed the counter before its first proof, with FLOOR at
0.0 (one set up from the kit's templates, or one that adopted the
counter in a PR of its own), skips steps 1, 2 and 4. Its first proof
PR may still raise FLOOR, and the badge with it, from 0.0 to the
figures the counter names on that PR, in the same PR (step 3). Every
later raise is a PR of its own.

A port with a `verus/floor/` directory names the FLOOR file something
else (`verus/SHARES`, say), since `verus/FLOOR` and `verus/floor/`
are one path on a case-insensitive file system. Name it in both places:
the `--floor` flag, which the counter reads at the head, and
`[floor] path` in `verus/count.toml`, which it reads at `--base-rev`
(`--floor verus/SHARES` and `path = "verus/SHARES"`). With only
the key set, step 1's `--floor verus/FLOOR` reads the directory and the
counter exits 2.

On a pull request head of a zlib port, the two-line configuration counts
16 verified functions in zlib-core, the number that port's own
`verus/FLOOR` records (`verified 16`, an older single-figure format
the kit does not read), and names `functions 4.1`, `lines 12.8`. On
the Expat port's first-proof worktree, with `verus_features = ["verus"]`, it counts
`utf8::invalid_char` (57 lines) as verified, the only one, as the
pilot's hand count did.

## Adopting: the package

Each kit release from v0.3.0 is the Wolfi package `theseus-proved-port`,
built from the release's tag, and the package is the one way a port
takes the kit. It installs:

- `/usr/bin/verus-count`, this crate, whose `--version` prints
  `verus-count X.Y.Z` for release vX.Y.Z;
- `/usr/share/proved-port/make/`: `proved-port.mk`, `kani.sh`,
  `headers.sh`, `lint-unsafe.sh` and `VERUS-FILES` (`kit/make`);
- `/usr/share/proved-port/templates/`: all of `kit/templates`, under
  their own names;
- `/usr/share/proved-port/CHANGELOG.md`: `kit/CHANGELOG.md`, each
  release's changes and porting notes;
- `/usr/share/proved-port/KIT`: `commit <sha>` and `tag vX.Y.Z`.

A port takes it this way:

- It pins the package, `theseus-proved-port~X.Y.Z`, in the packages of
  every check that runs a kit target (`.chainguard/ci/verus.yaml`) and
  in the environment development sessions and the deps agent install
  (`.chainguard/develop.yaml`, or the file its `from:` names).
- Its Makefile sets its variables, then names the same release and
  includes the installed make targets. The recommended form is this guard, which
  includes them where the package is installed and, where it is not,
  gives each kit target a recipe that fails saying so, so every other
  target still runs:

  ```make
  # The proved-port kit, from the theseus-proved-port package. Set the
  # port's variables (VERUS_CRATES, KANI_CRATES, KANI_NONE,
  # VERIFY_TARGETS, ...) above this. VERIFY_TARGETS lists every target the
  # checks verify with; `make verify` runs them, as the package build does.
  # PROVED_PORT_VERSION is the kit release, which the theseus-proved-port
  # pins in the checks and in the development sessions' environment name
  # too; the kit targets fail on any other install. Where the package is not
  # installed, the kit targets fail with the message below, and every other
  # target still runs; point PROVED_PORT_HOME at a root laid out as
  # /usr/share/proved-port (make/, templates/ and KIT) and VERUS_COUNT at a
  # counter built from the kit at that tag to run them.
  PROVED_PORT_VERSION = X.Y.Z
  PROVED_PORT_HOME ?= /usr/share/proved-port
  ifneq ($(wildcard $(PROVED_PORT_HOME)/make/proved-port.mk),)
  include $(PROVED_PORT_HOME)/make/proved-port.mk
  else
  PROVED_PORT_TARGETS = verus verus-proofs verus-count verus-count-test verus-headers kani kani-list kani-regular lint-unsafe verify
  .PHONY: $(PROVED_PORT_TARGETS)
  $(PROVED_PORT_TARGETS):
  	@echo "$@: no proved-port kit at $(PROVED_PORT_HOME): install theseus-proved-port~$(PROVED_PORT_VERSION), or set PROVED_PORT_HOME to a kit install root (make/, templates/ and KIT, as the package lays out /usr/share/proved-port)" >&2; exit 1
  endif
  ```

  A bare `include $(PROVED_PORT_HOME)/make/proved-port.mk` works too,
  but where the package is missing make then refuses every target, the
  port's own `make test` included.

  With `PROVED_PORT_VERSION` set, every kit target fails unless
  `PROVED_PORT_HOME/KIT` records `tag vX.Y.Z`, and `make verus` and
  `make verus-count` fail unless `verus-count --version` prints
  `verus-count X.Y.Z`, so neither a stale install nor another counter
  earlier on PATH passes for the pinned one. `make verus-count-test`
  runs no tests (the package's build runs them) but makes the same
  release check.
- Its held files carry no `# kit:` line: the release is the package
  pin, and `make verus-headers` reads none.
- The package pins, `PROVED_PORT_VERSION` and the proved-port
  marketplace `ref` in `.claude/settings.json` move
  together, with any new header text and what the release's CHANGELOG
  asks of ports within that entry's paths, in a pull request of their
  own (the kit entry of `kit/templates/.chainguard/deps.yaml`).

Where the package is not installed (a Mac, a development host), install
the kit from a clone of this repository checked out at the pinned tag,
with one command, `kit/install.sh PREFIX`. It lays the kit out under
PREFIX as the package does under `/usr`: the counter as
`PREFIX/bin/verus-count`, and `make/`, `templates/`, `CHANGELOG.md` and
a `KIT` that records the commit and release in
`PREFIX/share/proved-port`. Then run
the port's kit targets with that root and counter:

```sh
git -C <clone> checkout vX.Y.Z
sh <clone>/kit/install.sh "$HOME/.local"
make verus PROVED_PORT_HOME="$HOME/.local/share/proved-port" \
  VERUS_COUNT="$HOME/.local/bin/verus-count" VERUS_BASE=none
```

A checkout's own `kit/` is not such a root: it has no `KIT`, and the
kit targets fail without one, as they do on a stale install.
`install.sh` refuses (exit 1) a checkout that is not the tag of the
release it would record, or that has a change under `kit/`, since the
counter's `--version` names the last release until the next one is
tagged and `KIT` would then pass the version check for another release's
files. `kit/install.sh --dev PREFIX` installs such a development tree
all the same, saying so. Over an earlier install it removes that
install's `KIT` before writing anything and replaces `make/` and
`templates/` whole, so a reinstall that fails part way leaves no `KIT`,
which the kit targets refuse. `kit/installed-test.sh` installs
the kit through it. A fix the port needs in the
counter or the make targets lands here and reaches the port with the
next release.

## Moving from a vendored copy

A port that carries vendored `verus/count/` and `verus/make/` copies is
on kit 0.2.4, and moves to the package in one pull request. That pull
request deletes `verus/count/` (with its `KIT` and `Cargo.lock`) and
`verus/make/`, with every reviewer or code-owner row that names them;
drops the `# kit:` line from each file that has one, held or a
starting point, replacing it with a blank line where the port's own
comments follow it, so that they stay out of the header `make
verus-headers` holds to the template (the header is the file's leading
comment, up to its first non-comment line); replaces the review prompt's
`- The vendored kit` item with the template's `- The kit` item; pins
`theseus-proved-port~0.3.0` as "Adopting: the package" says, with the
proved-port marketplace in `.claude/settings.json` at `v0.3.0` (the
`kit/templates/.claude/settings.json` template); and includes the
installed `proved-port.mk` in place of `verus/make/proved-port.mk`.

## Release

A release is a `vX.Y.Z` tag on main (v0.1.0 through v0.2.4 so far).
The counter's version is the release's: the pull request that opens a
release sets `version` in `Cargo.toml` and `Cargo.lock` to X.Y.Z before
the tag, and `--version` prints `verus-count X.Y.Z` from that commit
on. A counter built from main between that pull request and the tag
already names the coming release.

From v0.3.0 on, each tag is packaged as theseus-proved-port X.Y.Z,
which installs `verus-count` and kit/make, the templates and a KIT
file, and that package is the one way a port takes the kit. A port
pins it with `~` and sets PROVED_PORT_VERSION in its Makefile, and
kit/make/proved-port.mk refuses a counter whose `--version` is another
release. So a tag that kept the previous release's version fails `make
verus` in every port that moves its pin to it. Nothing in this
directory checks the bump by itself: the cli tests only refuse the
0.0.0 placeholder.

Each pull request for a release adds its change to that release's
section of kit/CHANGELOG.md, and what a port must do on the bump to its
"Porting notes". The package installs the file as
`/usr/share/proved-port/CHANGELOG.md` (kit/install.sh as
`PREFIX/share/proved-port/CHANGELOG.md`), and kit/installed-test.sh
fails unless the installed copy has porting notes for the release the
counter names.
