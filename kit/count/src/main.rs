//! verus-count: counts the executable functions of a proved port's crates
//! by how Verus sees them, and holds them to the port's FLOOR
//! (kit/count/README.md). The port's `verus/count.toml` ([`config`]) names
//! its crates, its test-only `cfg`s and features, and where each table the
//! checks below read lives; a check whose section it leaves out does not
//! run. The paths below are the PCRE2 port's, which the defaults follow.
//!
//! A function is *verified* when it sits inside `verus! { ... }`, or
//! carries Verus's attribute form (`#[verus_verify]` or `#[verus_spec]`,
//! directly or under a `cfg_attr` whose predicate Verus's build is sure
//! to meet),
//! is not `#[verifier::external_body]`,
//! `#[verifier::external]` or `verus_verify(external_body)`, and its body
//! calls none of `assume`, `admit` and `Tracked::assume_new` (which mints
//! a ghost token on trust), and no `#[verifier::exec_allows_no_decreases_clause]`
//! covers it (on it, or on the `impl`, trait or module around it: Verus
//! then proves none of its loops or recursion ends; covering an
//! out-of-line `mod x;`, on it or on a module or file around it, it
//! fails the run);
//! *trusted* when it sits inside
//! `verus!` but is one of those; and *outside* when it is not inside
//! `verus!` at all. A trusted function marked
//! `#[cfg_attr(any(), discharged_by = "kani")]` is *discharged* instead:
//! Kani checks its contract (verus/README.md, "Kani"), on the bounded inputs
//! its record names, so it counts neither as verified nor as trusted. The
//! marker is inert in every build (`any()` is false), and a marked function
//! no `verus/seams/kani-*.md` record covers fails the run. With a
//! `[ledger]` section, a generated seam ledger ([`ledger`]) says which
//! trusted functions are discharged instead, and the marker is refused.
//! `#[verifier::assume_unsupported_attributes]` changes no class: Verus
//! still checks the body, and takes only the item's `no_mangle`,
//! `export_name` or `link_section` on trust. Each function inside
//! `verus!` that carries it is a seam of its own, the linkage alone: the
//! report lists and counts them after the tables ([`print_assumed`]), and
//! with `[ledger]` each needs an `unsupported-attribute` entry
//! ([`ledger::match_assumed`]), and the count is the ledger's.
//! `spec` and `proof` functions are ghost code, erased from ordinary
//! builds, and are counted apart. Items under the test-only `cfg`s
//! (`#[cfg(test)]`, `#[cfg(kani)]`, the Kani harnesses) or test-only
//! features (`#[cfg(feature = "test-util")]`, which only tests enable; a
//! build of the workspace that cargo resolves with one enabled fails the
//! run) are left out, with the files of such modules and, for a `mod.rs`
//! one, its directory. Items built only for Verus (`verus_keep_ghost`, or
//! a configured Verus-only feature) are read for their rlimits and their
//! trusted functions alone. A shipped function under a `cfg` Verus's build
//! might not meet (on the function, or on an `impl`, trait, inline `mod`,
//! `verus!` block or out-of-line `mod x;` around it), or under a
//! `cfg_attr` it might not apply, is *trusted* rather than verified: it
//! ships, and nothing proves Verus compiled it. Only the atoms the
//! configuration settles there are sure: `verus_keep_ghost`, the
//! Verus-only features, the test-only atoms (never set), and the
//! `verus_sets` and `verus_never_sets` atoms; a platform such as
//! `target_os = "windows"`, or any other atom, may be either. A port
//! that declares its proof runs (`[[runs]]`, #21) has each run's `sets`
//! and `never_sets` settle atoms in that run alone, where a set value of
//! a single-valued key also settles that key's other values as never;
//! an item is classed in each run, and is verified where it is sure in
//! some run and unsure in none, so one only one run compiles
//! (`not(target_arch = "x86_64")` beside an aarch64 run) may be.
//! Lines run from a function's `fn` keyword to its closing brace.
//!
//! cargo verus has Verus check only a package whose Cargo.toml sets
//! `[package.metadata.verus] verify = true`, whatever `-p` selects. A
//! crate with code Verus checks (a function inside `verus!` or of the
//! attribute form, or a proof built only for Verus) whose Cargo.toml does
//! not fails the run (exit 1), each such crate named ([`opted_in`]):
//! nothing would check the functions the tally credits as verified.
//!
//! A module declaration whose file the source walk does not read (a
//! `#[path]` to a file not named `*.rs` or outside the crate's `src/`,
//! under either reading of where the declaring file's modules live)
//! fails the run (exit 1), each named ([`unread_module_files`]), unless
//! only tests build it: rustc compiles its functions, so leaving them
//! out would raise the verified share.
//!
//! A function an item macro writes counts too ([`Counter::item_macro`]):
//! an invocation of a `macro_rules!` the crate defines is expanded
//! ([`expand`]), and each function in the expansion counts as outside
//! `verus!`, listed at the invocation's line, its lines those of the
//! invocation and the definition it came from. An item macro the crate
//! does not define counts as one function outside `verus!`, named
//! `NAME!`, so no function leaves the tally for being written by a macro;
//! one the crate defines that does not expand fails the run. The
//! exceptions are std's item macros that write no function
//! (`compile_error!`, `thread_local!`), by `::std::` or bare where
//! nothing may rebind the name. A `thread_local!` imported by name or
//! invoked by a path from `std` or `core` is std's only where no `use`
//! in the module (for the import in 2015, at the root too) or `extern
//! crate .. as` gives another crate that name ([`Counter::use_rebinds`]),
//! and, for the import, where no module of the crate has that name as
//! rustc reads a `use` path (in 2015 one at the root, from 2018 on one
//! in the module: [`Counter::use_module`]).
//! A bare `thread_local!` that a `*` or a
//! `#[macro_use]` may rebind is std's only where each such source is
//! shown not to bring another ([`Counter::other_thread_local`]): a glob
//! of Verus's prelude, of std or core, or of a module of the crate that
//! brings in no `thread_local`, and a `#[macro_use]` of std's or Verus's
//! crates, each by a name no Cargo rename or `use` alias rebinds; any
//! other source counts it as another crate's macro. Fragments
//! match by the crate's edition ([`walked_editions`]): below 2024 an
//! `expr` is an `expr_2021` (no `const` block, no `_`), and below 2021 a
//! `pat` is a `pat_param` (no top-level `|`). A crate whose read targets
//! (library, binaries, and tests, benches or examples under `src/`) sit
//! on both sides of 2021 or of 2024 fails the run.
//! `compile_error!`, which writes nothing that ships ([`STD_NO_FN`]),
//! counts nothing where it is provably the standard library's
//! ([`Counter::std_writes_nothing`]). `verus!` is Verus's bare or as
//! `::vstd::prelude::verus!` / `::verus_builtin_macros::verus!`, written
//! by the crate or by a macro it expands; a block of another path, or of
//! one of these whose rule does not hold, that writes a function fails
//! the run. Each form has a rule true by rustc's own semantics ([`Took`]),
//! read from the crate's bindings rather than by resolving names.
//!
//! Usage: `verus-count [--config FILE] [--list] [--floor FILE [--base-rev
//! REV | --no-base]] [--rlimit STATS] ROOT`, with ROOT the repository root (default
//! `.`) and FILE the configuration (default `ROOT/verus/count.toml`);
//! `--version` prints the counter's version, the kit release it comes
//! from; `--help` (`-h`) prints the usage ([`USAGE`]) and exits 0; and
//! `verus-count [--config FILE] --badge ROOT` prints the coverage badge
//! line ROOT's FLOOR gives ([`badge_line`]).
//! `--list` prints one line per executable function: crate, class, lines,
//! file:line and name, before the table; every check runs as without it.
//! `--floor FILE` fails the run when the `shipped`
//! row's verified share of functions or of lines, or its verified and
//! discharged share, is below the figure FILE records (verus/FLOOR), and
//! names the figure to record when the share has risen past it; FILE must
//! be the configuration's `[floor] path`. With `[floor] room_functions`
//! or `room_lines` (#100), the figure named is the share with that many
//! more unverified functions and lines in its denominators, and no figure
//! is named while the share is within that room; the pass or failure is
//! the share's alone. `--base-rev REV` also fails a
//! figure FILE sets below the figure in REV's FLOOR (at REV's `[floor]
//! path`) without a new `lowered` line for it, and fails when REV does
//! not resolve. With `[floor] renamed_from`, REV's copy may give the
//! checked figures under that older prefix, so the change that renames
//! them compares with its base. It also fails when the configuration
//! removes or narrows a gate `REV`'s copy of it runs with no new
//! `[[narrowed]]` line naming the change ([`check_config`]), since a gate
//! whose section is gone does not run at all. `--floor` with neither
//! `--base-rev` nor `--no-base` fails the run (exit 1), so a run that
//! should compare with a base cannot pass by leaving the base off;
//! `--no-base` says there is none to compare with (a local run, or the
//! change that adds the first FLOOR), and with `--base-rev` it is an error
//! (exit 2).
//!
//! With `[badge]`, `--floor FILE` also fails unless the badge file
//! (`README.md`) holds exactly one rendered Verus badge, in any case or
//! form ([`is_badge`]; fences and HTML comments do not render), and it is
//! the line `--badge` prints for FILE's `functions` and `lines` figures
//! ([`check_badge`]), so the badge a reader sees cannot drift from FLOOR.
//!
//! With `[recursion]`, it checks recursion (verus/theorems/RN1.md): it fails on a
//! cycle of direct calls, within one file, through a function of a shipped
//! crate that Verus does not check (outside `verus!`, or trusted or
//! discharged), and on one through a verified function that RN1.md's table
//! does not name.
//!
//! With `[loops]`, it checks nested loops ([`nested_loops`], #76): it fails
//! on a function of a shipped crate that nests a loop in a loop, or calls
//! from inside a loop a function that loops (by name within its file, as
//! the recursion check reads calls), that the work-bound theorem's table
//! does not name.
//!
//! `--rlimit STATS` also checks rlimit headroom ([`rlimit`]; verus/README.md,
//! "Headroom"): STATS is what `cargo verus verify -- --time-expanded
//! --output-json` printed, and the run fails when a function of a crate it
//! covers used more of its rlimit than verus/RLIMIT allows. With
//! `--base-rev REV`, verus/RLIMIT may not loosen `REV:verus/RLIMIT`, and a
//! function's `#[verifier::rlimit]` may rise past REV's only with a new
//! `rlimit-raised` line in verus/RLIMIT. A function that loses its
//! attribute runs under the default (10), so dropping one below the
//! default is a rise too. A function's limit is matched to the statistics
//! by the name Verus gives it: a method's is its type's path, from the
//! module that defines the type, not the module of its `impl`
//! ([`Counter::name_methods`]). A `#[verifier::rlimit]` on a function the
//! statistics do not name fails the run, and every such limit is named.
//! A run against a base whose tree has the RLIMIT file (the path the
//! base's configuration gives it) fails without `--rlimit` (exit 1), so
//! a change cannot turn the headroom check off by dropping the flag
//! (`VERUS_RLIMIT =`); a `--no-base` run is not held.
//! Statistics in which a document starts before the one before it ends
//! fail the run too ("documents interleave", exit 2): cargo printed two
//! crates' documents at once, which `make verus-proofs` prevents with
//! cargo -j 1 (`VERUS_JOBS`).
//!
//! With `[inputs]`, it checks compile's inputs (verus/theorems/CD1.md,
//! [`inputs`]): it fails when a function reachable from the configured
//! roots (`Regex::new`), or from a builder a pattern runs on first use,
//! reads a static that can change, a thread-local, the environment or a
//! seeded `HashMap`, unless CD1.md's table names the read, and on a row of
//! that table no such read matches.
//!
//! With `[reach]`, it checks what a C caller reaches ([`reach`], verus/REACH): it fails
//! when a function reachable from an entry point (an exported symbol, a
//! function a `const` or `static` hands out, a `width::exports!` body or
//! a `FromC` conversion) is neither verified nor discharged nor a row of
//! verus/REACH, on a row that is verified or unreached, on a seam citation
//! no seams table has, and, with `--base-rev REV`, on a row
//! `REV:verus/REACH` lacks that is not marked `new`. It also fails on a
//! shipped file whose export blocks it cannot read ([`reach::exported`]):
//! one that does not parse, one that imports the exports macro under
//! another name, and one whose invocations of it (found by tokens, plain
//! or raw) outnumber the blocks the scan reads.
//!
//! With `[mints]`, it checks the C caller's token mints ([`mints`],
//! verus/MINTS): it fails on a kind of token (`impl CToken for KIND`) or a
//! function that calls `c_mint` that verus/MINTS does not list, on a mint
//! that returns another kind than its row names, on a stale or duplicate
//! row, and on an alias of `c_mint` or `CToken` through which either would
//! go unseen.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

mod config;
mod expand;
mod features;
mod inputs;
mod ledger;
mod mints;
mod reach;
mod rlimit;

use config::Config;

use proc_macro2::{TokenStream, TokenTree};
use verus_syn::{
    Attribute, FnMode, ImplItem, Item, Meta, Signature, TraitItem, parse_file, parse2,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Class {
    Verified,
    Trusted,
    Outside,
    Discharged,
}

#[derive(Clone, Default)]
struct Tally {
    fns: [usize; 4],
    lines: [usize; 4],
    ghost: usize,
}

impl Tally {
    fn add(&mut self, t: &Tally) {
        for i in 0..4 {
            self.fns[i] += t.fns[i];
            self.lines[i] += t.lines[i];
        }
        self.ghost += t.ghost;
    }
}

struct Counter {
    krate: String,
    file: PathBuf,
    list: bool,
    tallies: BTreeMap<String, Tally>,
    /// Files and `verus!` blocks that could not be read or parsed. Their
    /// functions are missing from every count, so any of them fails the run.
    errors: usize,
    /// Each discharged function's file, relative to the root, and name, for
    /// [`unrecorded`].
    discharged: Vec<(PathBuf, String)>,
    /// Every executable function, for [`recursive`].
    fns: Vec<FnRef>,
    /// Each source file a shipped build compiles, with its crate, whether
    /// or not it holds a function: the files [`entry_reach`] scans for
    /// export blocks.
    files: Vec<(String, PathBuf)>,
    /// Every `static`, for the compile-inputs check ([`inputs`]).
    statics: Vec<inputs::Static>,
    /// The structs, enums and aliases, for which statics are plain data
    /// ([`inputs::plain_types`]).
    types: inputs::Types,
    /// The `use .. as` renames and `type` aliases ([`inputs::aliases_in`]).
    aliases: inputs::Aliases,
    /// Every trait's name, for the compile-inputs check.
    traits: std::collections::BTreeSet<String>,
    /// The type (an `impl`) or trait whose functions [`Counter::items`] is
    /// walking; empty for free functions.
    owner: String,
    /// Whether [`Counter::owner`] is an `impl`'s type, not a trait.
    owner_impl: bool,
    /// The type parameters of the `impl` [`Counter::items`] is walking
    /// (`impl<S: Spec> Tr for S` declares `S`).
    owner_params: Vec<String>,
    /// The path (module, then name) of each struct, enum, union and `type`
    /// alias the walk read, with the module it is visible within
    /// ([`Counter::vis_scope`]; `None` for a `pub(in ..)` path the counter
    /// cannot place) and whether Verus's build might not compile it
    /// ([`Built::Unsure`]), by which [`Counter::name_methods`] finds the
    /// type an `impl` names.
    type_paths: BTreeMap<Vec<String>, (Option<Vec<String>>, bool)>,
    /// The modules that may bind a name the walk does not see: those with
    /// an item macro invocation the counter does not expand (another
    /// crate's, `include!`), which may write any type, and those with a
    /// `use` an expansion writes, which [`Counter::uses`] does not hold.
    /// Either binding shadows a glob's, so [`Counter::type_path`] takes no
    /// glob's type in such a module.
    opaque: std::collections::BTreeSet<Vec<String>>,
    /// Each method of an `impl` that [`Counter::record`] named by the
    /// `impl`'s module, renamed by its type's once the crate is walked
    /// ([`Counter::name_methods`]).
    methods: Vec<Method>,
    /// The module [`Counter::items`] is walking, as Verus names it: the
    /// crate (`pcre2_syntax`), then each module of the file's path and of
    /// any inline `mod` around the item.
    module: Vec<String>,
    /// The instance of `module` [`Counter::items`] is walking
    /// ([`Binding::scope`]): the file, or the innermost inline `mod` around
    /// the item.
    scope: String,
    /// Every function's `#[verifier::rlimit]`, ghost ones too, for the
    /// headroom check ([`rlimit::shares`]): Verus-only code's too, which
    /// Verus checks.
    limits: Vec<rlimit::Limit>,
    /// Every function Verus checks, rlimit or not, by the name a
    /// [`rlimit::Limit`] would give it, with where it starts: a function
    /// here that has no limit has the default ([`rlimit::check_raised`]).
    verus_fns: BTreeMap<String, String>,
    /// Whether the code [`Counter::items`] is walking is built by a
    /// shipped build: false inside a module, `impl` or trait built only
    /// for Verus, whose functions are walked for their rlimits and not
    /// counted.
    shipped: bool,
    /// Whether Verus's build is sure to compile the code
    /// [`Counter::items`] is walking: false inside a module, `impl`, trait
    /// or `verus!` block whose `cfg` it might not meet
    /// ([`Built::Unsure`]), whose verified functions count as trusted.
    sure: bool,
    /// Whether the code [`Counter::items`] is walking is under
    /// `#[verifier::exec_allows_no_decreases_clause]` ([`allows_no_decreases`]):
    /// a module, `impl` or trait that carries it, or a file whose inner
    /// attributes do. Verus proves no termination for its exec functions,
    /// whose verified ones count as trusted.
    no_decreases: bool,
    /// The `cfg` predicates of the code [`Counter::items`] is walking: its
    /// file's ([`module_guards`]) and those of each module, `impl`, trait
    /// and `verus!` block around it. An item is built where they and its
    /// own all hold ([`guarded_built`]).
    guards: Vec<String>,
    /// `width::fn_ptr!`'s definition and invocations, expanded once every
    /// file is read ([`Counter::expand_fn_ptrs`]).
    fn_ptrs: FnPtrs,
    /// The functions a `const` or `static` initializer names as a value
    /// (`MallocPtr(default_malloc)`), by crate and name: whoever holds the
    /// pointer may call it, so each is an entry point ([`Counter::finish`]).
    addressed: Vec<(String, String)>,
    /// The `cfg` atoms that hold only in test builds or only in Verus's
    /// ([`Atoms`]).
    atoms: Atoms,
    /// The `fn_ptr!` macro's name (`[reach] fn_ptr_macro`), if any.
    fn_ptr_macro: Option<String>,
    /// Each trusted function's crate, `FILE:LINE` and lines, for the
    /// generated ledger ([`ledger::match_ledger`]).
    trusted: Vec<(String, String, usize)>,
    /// The `FILE:LINE` of each function inside `verus!` with
    /// `#[verifier::assume_unsupported_attributes]` ([`assumes_attributes`]),
    /// a seam of the linkage alone ([`ledger::match_assumed`]).
    assumed: Vec<String>,
    /// Whether ghost functions of code built only for Verus count in the
    /// ghost column (`[crates] verus_only_ghost`).
    verus_only_ghost: bool,
    /// The crate's `macro_rules!` definitions, by name, for
    /// [`Counter::item_macro`].
    macros: BTreeMap<String, Vec<MacroDef>>,
    /// Each module's `use` paths, for [`Counter::item_macro`] to resolve a
    /// macro an invocation names by an import ([`use_paths`]).
    uses: BTreeMap<Vec<String>, Vec<UsePath>>,
    /// While [`Counter::item_macro`] walks an expansion: the outermost
    /// invocation's line, which each function it writes is listed at.
    expansion: Option<usize>,
    /// How deep the expansion being walked is nested.
    depth: usize,
    /// While [`Counter::item_macro`] compares the definitions an
    /// invocation may expand: the rows a candidate's expansion writes,
    /// gathered in place of the tally ([`Counter::tally`]).
    dry: Option<Vec<(Class, String, usize)>>,
    /// The invocations that resolved to no definition and were counted as
    /// another crate's macro (`NAME!`), with where they are: once the crate
    /// is read, one whose name the crate defines fails the run
    /// ([`Counter::walk_crate`]), so that no function the crate's macro
    /// writes leaves the tally for a lookup the counter does not follow.
    unresolved: Vec<(String, String)>,
    /// The invocations that resolved to some definition, each with where
    /// it is, its module, its outermost invocation's line and its name:
    /// once the crate is read, one a `macro_rules!` that an expansion in a
    /// file read later writes, carried in by a `#[macro_use] mod`, may
    /// reach fails the run ([`Counter::walk_crate`]).
    invoked: Vec<Invoked>,
    /// The file being walked: its module, and the scope and
    /// [`MacroDef::carried_at`] a definition at its top has, which a
    /// `macro_rules!` an expansion writes there takes.
    carry: Carry,
    /// While the walk is in inline modules of the file: whether each, from
    /// the outermost, has `#[macro_use]`, for a `macro_rules!` an
    /// expansion writes there.
    inline_macro_use: Vec<bool>,
    /// Every module of the crate, by path from its root, so that a `use`
    /// path whose first segment is one of them reads as this crate's.
    modules: std::collections::BTreeSet<Vec<String>>,
    /// The modules a `mod` item under a `cfg` of its own declares
    /// ([`cfg_gated_mods`]), and those whose file has a `#![cfg]`: a build
    /// may leave each out.
    cfg_modules: std::collections::BTreeSet<Vec<String>>,
    /// For each crate root (the library, each binary and other target),
    /// the files it may compile and those it surely names by their place
    /// under src/ ([`Counter::add_proof_trees`]), by which
    /// [`Counter::certain`] decides whether a file's `super` is the module
    /// its place gives.
    proof_trees: Vec<ProofTree>,
    /// The crates a `#[macro_use] extern crate` at the crate root loads
    /// macros from ([`macro_use_crates`]): their names are in scope in
    /// every module, ahead of the standard library's prelude.
    macro_use: Vec<String>,
    /// Every binding the crate's sources write, but those only tests
    /// build ([`Binding`]): what the rules read ([`Took`]).
    bindings: Vec<Binding>,
    /// Every binding a macro expansion writes, read once the crate is
    /// walked, against what the walk took.
    emitted: Vec<Binding>,
    /// The crate's edition (its Cargo.toml's, or the workspace's it
    /// inherits; 2015 when it gives none, as cargo reads it).
    edition: u32,
    /// Each of std's and Verus's crate names ([`STD_OR_VERUS_CRATES`])
    /// that a dependency of the crate takes for another package (`vstd =
    /// { package = "other" }`), with why.
    renamed_deps: Vec<(String, String)>,
    /// Each `verus!` block holding a function, and each `compile_error!`
    /// counted as nothing, that the walk took on its rule: the form, the
    /// place, the module and the name the rule reads. An expansion that
    /// writes a binding breaking it fails the run once the crate is walked.
    took: Vec<(Took, String, Vec<String>, String)>,
}

/// One path a `use` names: its segments as written (the imported item
/// last), whether it ends in `*`, and the name it brings into scope (the
/// last segment, or the new name of a `use .. as`; empty for `*`).
type UseTarget = (Vec<String>, bool, String);

/// A [`UseTarget`] of this crate, with where its `use` (with its `cfg`s
/// and its enclosing modules') is built: a test-only import imports
/// nothing a shipped build sees.
type UsePath = (Vec<String>, bool, String, Built);

/// One `macro_rules!` definition: the file and line it is at, where it is
/// built, where its textual scope ends, the paths that name it, and its
/// rules, or why they could not be read.
#[derive(Clone)]
struct MacroDef {
    file: PathBuf,
    line: usize,
    /// Whether an expansion wrote it: it is listed at the outermost
    /// invocation's line, and is in scope from that invocation on.
    emitted: bool,
    /// The module it is in.
    module: Vec<String>,
    /// The paths that name it: the crate root's for `#[macro_export]`, and
    /// its module's for a `use NAME;` there (`pub(crate) use exports;`).
    paths: Vec<Vec<String>>,
    /// Where its `cfg`s, its enclosing modules' and its file's let it be
    /// built: a definition a build does not compile defines nothing there.
    built: Built,
    /// Whether a `cfg` on it or an enclosing item may be true or false in a
    /// build ([`Built::Unsure`]), so that it may not shadow an earlier
    /// definition of its name.
    unsure: bool,
    /// The `cfg` predicates on it and around it. [`Built::Shipped`] says
    /// only that a shipped build can compile it and Verus's surely does (a
    /// `verus_sets` atom), so in shipped code it shadows an earlier
    /// definition only where a shipped build compiling the invocation
    /// surely compiles it ([`shipped_may_skip`]).
    guards: Vec<String>,
    /// The module whose end ends its textual scope: the module it is in,
    /// or for one in a `#[macro_use]` module (inline, or a file declared
    /// `#[macro_use] mod name;`), the module that one is in.
    scope: Vec<String>,
    /// For one at the top of a file a `#[macro_use] mod name;` declares
    /// (and of a file that one's file so declares, outward), each declaring
    /// file and the line of its `mod`: in that file it is in scope from
    /// that line on, as a definition written there would be.
    carried_at: Vec<(PathBuf, usize)>,
    /// Whether a module along [`MacroDef::carried_at`] is declared by more
    /// than one `mod` (under `cfg`s): in another file its place is then
    /// not one line, so it is a candidate there that ends no chain.
    split: bool,
    /// Whether it has `#[macro_export]`, which puts it at the crate root.
    exported: bool,
    rules: Result<expand::Rules, String>,
}

/// The narrower of two places an item can be built.
fn narrower(a: Built, b: Built) -> Built {
    match (a, b) {
        (Built::Test, _) | (_, Built::Test) => Built::Test,
        (Built::Verus, _) | (_, Built::Verus) => Built::Verus,
        (Built::Unsure, _) | (_, Built::Unsure) => Built::Unsure,
        _ => Built::Shipped,
    }
}

/// An expansion nested deeper than this is an error: rustc's default
/// `recursion_limit`.
const MAX_DEPTH: usize = 128;

/// The `cfg` atoms, without whitespace, that the configuration settles:
/// those only test builds meet (`test`, `kani`, `feature="test-util"`);
/// those only Verus's build meets (`verus_keep_ghost`, and `feature="F"`
/// for each configured Verus-only feature); and those Verus's build is
/// sure to meet (`sets`, `[crates] verus_sets`) or never meets (`never`,
/// `verus_never_sets`), which a shipped build may or may not. Every other
/// atom may be true or false in any build.
#[derive(Debug, Clone, Default)]
struct Atoms {
    test: Vec<String>,
    verus: Vec<String>,
    sets: Vec<String>,
    never: Vec<String>,
    /// Each declared proof run's `(sets, never)` (`[[runs]]`, #21), whose
    /// classes [`verus_can`] combines; empty without `[[runs]]`, when
    /// `sets` and `never` are the one run's.
    runs: Vec<(Vec<String>, Vec<String>)>,
    /// Whether `sets` and `never` are one declared run's ([`Atoms::each_run`]),
    /// so a value of a single-valued key `sets` holds rules out the key's
    /// other values. The shorthand may hold several runs' values, so it
    /// does not.
    one_run: bool,
}

impl Atoms {
    fn of(c: &Config) -> Atoms {
        let runs = if c.runs.is_empty() {
            Vec::new()
        } else {
            c.proof_runs().into_iter().map(|(_, s, n)| (s, n)).collect()
        };
        Atoms {
            test: c.test_atoms(),
            verus: c.verus_atoms(),
            sets: c.verus_set_atoms(),
            never: c.verus_never_atoms(),
            runs,
            one_run: false,
        }
    }

    /// Each proof run's atoms, as one-run [`Atoms`]: the declared runs, or
    /// these atoms themselves without any.
    fn each_run(&self) -> Vec<Atoms> {
        if self.runs.is_empty() {
            return vec![self.clone()];
        }
        self.runs
            .iter()
            .map(|(sets, never)| Atoms {
                test: self.test.clone(),
                verus: self.verus.clone(),
                sets: sets.clone(),
                never: never.clone(),
                runs: Vec::new(),
                one_run: true,
            })
            .collect()
    }
}

/// [`Can`] for the predicate `p` across the proof runs (#21): without
/// `[[runs]]`, [`cfg_eval`] in Verus's build. With them, `p` is classed in
/// each run, and it can be true where some run may compile it, and can be
/// false unless it is sure in some run and unsure in none, so `t && !f`
/// reads "verified": every run that might ship it also verifies it.
fn verus_can(p: &str, atoms: &Atoms) -> Result<Can, String> {
    if atoms.runs.is_empty() {
        return cfg_eval(p, true, atoms);
    }
    let (mut t, mut sure, mut unsure) = (false, false, false);
    for run in atoms.each_run() {
        let c = cfg_eval(p, true, &run)?;
        t |= c.t;
        sure |= c.t && !c.f;
        unsure |= c.t && c.f;
    }
    Ok(Can {
        t,
        f: unsure || !sure,
    })
}

impl Counter {
    /// A counter that has read nothing yet.
    fn new(list: bool, atoms: Atoms, fn_ptr_macro: Option<String>) -> Counter {
        Counter {
            krate: String::new(),
            file: PathBuf::new(),
            list,
            tallies: BTreeMap::new(),
            errors: 0,
            discharged: Vec::new(),
            fns: Vec::new(),
            files: Vec::new(),
            statics: Vec::new(),
            types: BTreeMap::new(),
            aliases: BTreeMap::new(),
            traits: std::collections::BTreeSet::new(),
            owner: String::new(),
            owner_impl: false,
            owner_params: Vec::new(),
            type_paths: BTreeMap::new(),
            opaque: std::collections::BTreeSet::new(),
            methods: Vec::new(),
            module: Vec::new(),
            scope: String::new(),
            limits: Vec::new(),
            verus_fns: BTreeMap::new(),
            shipped: true,
            sure: true,
            no_decreases: false,
            guards: Vec::new(),
            fn_ptrs: FnPtrs::default(),
            addressed: Vec::new(),
            atoms,
            fn_ptr_macro,
            trusted: Vec::new(),
            assumed: Vec::new(),
            verus_only_ghost: false,
            macros: BTreeMap::new(),
            uses: BTreeMap::new(),
            dry: None,
            unresolved: Vec::new(),
            invoked: Vec::new(),
            carry: Default::default(),
            inline_macro_use: Vec::new(),
            modules: std::collections::BTreeSet::new(),
            cfg_modules: std::collections::BTreeSet::new(),
            proof_trees: Vec::new(),
            macro_use: Vec::new(),
            bindings: Vec::new(),
            emitted: Vec::new(),
            edition: 2015,
            renamed_deps: Vec::new(),
            took: Vec::new(),
            expansion: None,
            depth: 0,
        }
    }
}

/// A function declared in another's body, as [`Counter::nested`] reads
/// it: its attributes, signature and body.
type NestedFn<'a> = (&'a [Attribute], &'a Signature, &'a verus_syn::Block);

/// The `fn_ptr!` macro's one rule and its invocations: the definition
/// lives in one file (pcre2-capi's width.rs) and the invocations in
/// others, read in any order.
#[derive(Default)]
struct FnPtrs {
    /// The transcriber of `macro_rules! fn_ptr`'s rule, and how many rules
    /// it has (the expansion understands exactly one).
    rule: Option<TokenStream>,
    rules: usize,
    /// Each invocation: crate, file, line and the tokens it passes.
    uses: Vec<(String, PathBuf, usize, TokenStream)>,
}

/// A method of an `impl` Verus checks, as [`Counter::record`] first names
/// it: that name (the `impl`'s module, type and method), where it starts,
/// the `impl`'s module, its type as written, its type parameters, and the
/// method's name.
struct Method {
    name: String,
    at: String,
    module: Vec<String>,
    owner: String,
    params: Vec<String>,
    ident: String,
}

/// One executable function, for the recursion check ([`recursive`]): its
/// crate, file and line, owner ([`Counter::owner`]), name, class, and the
/// functions its body calls directly ([`direct_calls`]).
struct FnRef {
    krate: String,
    file: PathBuf,
    line: usize,
    owner: String,
    name: String,
    class: Class,
    calls: Vec<(bool, String)>,
    /// Its body's loops, for the nested-loop check ([`nested_loops`]).
    loops: LoopShape,
    /// What the body names, for the compile-inputs check ([`inputs`]).
    body: inputs::Body,
    /// Its parameters, the receiver aside, and whether it has one.
    params: usize,
    receiver: bool,
    /// Whether a caller outside the walk can enter it ([`reach`]): it
    /// exports its symbol ([`exports_symbol`]), or a `const` or `static`
    /// hands out its address ([`Counter::addressed`]).
    entry: bool,
}

/// `rule`, a `macro_rules!` transcriber, with each `$var` replaced by its
/// binding in `binds`, `$crate` by `crate`, and each `$( .. )` repetition
/// written once when every metavariable in it is bound, else not at all
/// (an optional or empty fragment). A metavariable `binds` does not name,
/// or one with no binding outside a repetition, is an error.
fn transcribe(
    rule: TokenStream,
    binds: &BTreeMap<&str, Option<TokenStream>>,
) -> Result<TokenStream, String> {
    fn vars(ts: TokenStream, out: &mut Vec<String>) {
        let mut it = ts.into_iter().peekable();
        while let Some(t) = it.next() {
            match t {
                TokenTree::Punct(p) if p.as_char() == '$' => {
                    if let Some(TokenTree::Ident(v)) = it.peek() {
                        out.push(v.to_string());
                    }
                }
                TokenTree::Group(g) => vars(g.stream(), out),
                _ => {}
            }
        }
    }
    let mut out = TokenStream::new();
    let mut it = rule.into_iter().peekable();
    while let Some(t) = it.next() {
        match t {
            TokenTree::Punct(p) if p.as_char() == '$' => match it.next() {
                Some(TokenTree::Ident(v)) if v == "crate" => {
                    out.extend([TokenTree::Ident(proc_macro2::Ident::new("crate", v.span()))]);
                }
                Some(TokenTree::Ident(v)) => match binds.get(v.to_string().as_str()) {
                    Some(Some(b)) => out.extend(b.clone()),
                    Some(None) => return Err(format!("leaves `${v}` unbound")),
                    None => return Err(format!("uses `${v}`, which it does not bind")),
                },
                Some(TokenTree::Group(g)) => {
                    // The separator, if any, then the repetition operator.
                    while let Some(TokenTree::Punct(op)) = it.peek() {
                        let c = op.as_char();
                        it.next();
                        if matches!(c, '*' | '?' | '+') {
                            break;
                        }
                    }
                    let mut inner = Vec::new();
                    vars(g.stream(), &mut inner);
                    for v in &inner {
                        if !binds.contains_key(v.as_str()) && v != "crate" {
                            return Err(format!("uses `${v}`, which it does not bind"));
                        }
                    }
                    if inner
                        .iter()
                        .all(|v| v == "crate" || binds.get(v.as_str()).is_some_and(Option::is_some))
                    {
                        out.extend(transcribe(g.stream(), binds)?);
                    }
                }
                _ => return Err("has a `$` that starts no metavariable".into()),
            },
            TokenTree::Group(g) => {
                let mut ng = proc_macro2::Group::new(g.delimiter(), transcribe(g.stream(), binds)?);
                ng.set_span(g.span());
                out.extend([TokenTree::Group(ng)]);
            }
            t => out.extend([t]),
        }
    }
    Ok(out)
}

/// Whether `sig`'s ABI is one C can call: `extern "C"`, `extern
/// "C-unwind"`, or a bare `extern`, which is C.
fn is_c_abi(sig: &Signature) -> bool {
    sig.abi.as_ref().is_some_and(|a| {
        a.name
            .as_ref()
            .is_none_or(|n| matches!(n.value().as_str(), "C" | "C-unwind"))
    })
}

/// Whether C can call the function by its name in some shipped build:
/// `#[no_mangle]` or `#[export_name = ..]`, either one inside
/// `#[unsafe(..)]` too, read through [`applied_attrs`], so under
/// `cfg_attr`s nested to any depth and beside other attributes. One a
/// `cfg_attr` applies counts when a shipped build can meet its predicates
/// (by [`cfg_eval`]): a feature, in `verus_sets` or not, a platform, or
/// any atom the configuration does not settle, so no function a shipped
/// C caller may enter drops out of the walk. One whose predicates no
/// shipped build meets (`any()`, a test atom, a Verus-only one) does not.
/// A predicate it cannot read is an error. That, not the ABI, makes a
/// function an entry point ([`entry_roots`]); the ABI is then
/// [`is_c_abi`]'s to check ([`Counter::record`]).
fn exports_symbol(attrs: &[Attribute], atoms: &Atoms) -> Result<bool, String> {
    let names = |t: Option<&TokenTree>| matches!(t, Some(TokenTree::Ident(i)) if i == "no_mangle" || i == "export_name");
    for (preds, toks) in applied_attrs(attrs) {
        let exports = names(toks.first())
            || match toks.as_slice() {
                [TokenTree::Ident(u), TokenTree::Group(g), ..]
                    if u == "unsafe" && g.delimiter() == proc_macro2::Delimiter::Parenthesis =>
                {
                    names(g.stream().into_iter().next().as_ref())
                }
                _ => false,
            };
        if exports && (preds.is_empty() || cfg_eval(&all_of(&preds), false, atoms)?.t) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// A signature's parameters, the receiver aside, and whether it has one.
fn arity(sig: &Signature) -> (usize, bool) {
    let receiver = sig
        .inputs
        .iter()
        .any(|a| matches!(a.kind, verus_syn::FnArgKind::Receiver(_)));
    (sig.inputs.len() - usize::from(receiver), receiver)
}

/// What the first pass learns of the module files ([`module_files`]):
/// each file's own inner `#![cfg(..)]` predicates (`inner`); each
/// out-of-line declaration of a file, with the file that declares it and
/// the `cfg` predicates it is under in that file (`declared`: its own,
/// `#[cfg(..)] mod name;`, and those of the inline modules, `verus!`
/// blocks and macro bodies around it), one record per declaration, since a
/// file declared twice is built where either holds; and the files a
/// `#[path]` names (`path_named`), which rustc reads as a `mod.rs`, so the
/// modules they declare are their siblings; and the crate's root files
/// (`roots`: `src/lib.rs`, `src/main.rs`, and each `src/bin` target),
/// which nothing declares. [`module_guards`] reads a file's guards from
/// them, and from those of the files that declare it. `alternate` holds
/// the declarations a file makes under the other reading of where its
/// modules live, for a file another target may read the other way (a
/// root read as a module, a main.rs or lib.rs that is a module, a
/// `#[path]`-named file declared by its place elsewhere);
/// [`Counter::add_proof_trees`] and [`module_guards`] read them, as files
/// a target may compile.
#[derive(Default)]
struct ModFiles {
    inner: Vec<(PathBuf, Vec<String>)>,
    declared: Vec<Declared>,
    path_named: Vec<PathBuf>,
    roots: Vec<PathBuf>,
    alternate: Vec<Declared>,
}

/// One out-of-line declaration ([`ModFiles`]): the file declared, the file
/// declaring it, the `cfg` predicates it is under there, and whether it is
/// inside an inline module whose directory a path attribute chose
/// (`under_path`), so that the file's location does not give its module's
/// name ([`Counter::add_proof_trees`]).
struct Declared {
    file: PathBuf,
    from: PathBuf,
    guards: Vec<String>,
    under_path: bool,
}

impl ModFiles {
    /// The directory of the modules `file` declares ([`module_dir`]), a
    /// crate root and a file a `#[path]` names counting as a `mod.rs`.
    fn dir_of(&self, file: &Path) -> Option<PathBuf> {
        if self.path_named.iter().chain(&self.roots).any(|p| p == file) {
            return file.parent().map(Path::to_path_buf);
        }
        module_dir(file)
    }
}

/// The atom a module file whose declaration the counter cannot read is
/// guarded by: one declared only inside a macro body other than `verus!`
/// (`cfg_if! { if #[cfg(windows)] { mod win; } }`), whose `cfg`s the
/// counter does not evaluate, and a crate file no declaration it reads
/// names (one a `#[cfg_attr(P, path = "..")]` under a macro names, or
/// one some other way reaches). No configuration settles this atom, so
/// the file is unsure ([`Built::Unsure`]): it counts as trusted, never as
/// verified.
const UNREAD_DECLARATION: &str = "verus_count_unread_declaration";

/// The edition of the crate in `dir` (its Cargo.toml's `[package]
/// edition`, or, with `edition.workspace = true`, `[workspace.package]
/// edition` of `root`'s Cargo.toml; 2015 when none is given, as cargo
/// reads it; the lowest of that and any target's own, `[lib] edition`
/// and the like), and each of std's and Verus's crate names
/// ([`STD_OR_VERUS_CRATES`]) that a dependency table (`[dependencies]`, `[dev-dependencies]`,
/// `[build-dependencies]`, or a `[target.*]` one) takes for another
/// package (`vstd = { package = "other" }`, directly or through a
/// `{ workspace = true }` entry of `[workspace.dependencies]`), or
/// inherits from an entry the root does not give, with why. A key with
/// no `package` is the package of its name, as cargo requires. A
/// manifest that does not parse gives 2015, where no `::` path is taken.
fn crate_manifest(root: &Path, dir: &Path) -> (u32, Vec<(String, String)>) {
    let read = |p: &Path| -> toml::Table {
        fs::read_to_string(p.join("Cargo.toml"))
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    };
    let manifest = read(dir);
    let workspace = read(root);
    let workspace = workspace.get("workspace");
    let inherits =
        |v: &toml::Value| v.get("workspace").and_then(toml::Value::as_bool) == Some(true);
    let package = package_edition(&manifest, workspace);
    // A target may set its own edition (`[lib] edition = "2015"`): the
    // lowest of the package's and every target's is the crate's.
    let targets = manifest.get("lib").into_iter().chain(
        ["bin", "example", "test", "bench"]
            .iter()
            .filter_map(|k| manifest.get(*k)?.as_array())
            .flatten(),
    );
    let edition = targets
        .filter_map(|t| t.get("edition"))
        .map(|e| e.as_str().and_then(|e| e.parse().ok()).unwrap_or(2015))
        .fold(package, u32::min);
    let mut tables: Vec<&toml::Value> = ["dependencies", "dev-dependencies", "build-dependencies"]
        .iter()
        .filter_map(|k| manifest.get(*k))
        .collect();
    if let Some(toml::Value::Table(targets)) = manifest.get("target") {
        for t in targets.values() {
            for k in ["dependencies", "dev-dependencies", "build-dependencies"] {
                tables.extend(t.get(k));
            }
        }
    }
    let mut renamed = Vec::new();
    for table in tables.into_iter().filter_map(toml::Value::as_table) {
        for (key, dep) in table {
            let name = key.replace('-', "_");
            if !STD_OR_VERUS_CRATES.contains(&name.as_str()) {
                continue;
            }
            // `{ workspace = true }` takes the workspace's entry of the key,
            // its `package` too; one the root does not give is unknown.
            let dep = match inherits(dep) {
                true => match workspace.and_then(|w| w.get("dependencies")?.get(key)) {
                    Some(d) => d,
                    None => {
                        renamed.push((
                            name.clone(),
                            format!("Cargo.toml's dependency `{key}` inherits a `[workspace.dependencies]` entry the workspace root does not give"),
                        ));
                        continue;
                    }
                },
                false => dep,
            };
            if let Some(p) = dep.get("package").and_then(toml::Value::as_str)
                && p.replace('-', "_") != name
            {
                renamed.push((
                    name.clone(),
                    format!("Cargo.toml's dependency `{key}` is the package `{p}`"),
                ));
            }
        }
    }
    (edition, renamed)
}

/// The `[package] edition` of `manifest`, or with `edition.workspace =
/// true` the `[workspace.package] edition` of `workspace` (the root's
/// `[workspace]` table); 2015 when none is given, as cargo reads it.
fn package_edition(manifest: &toml::Table, workspace: Option<&toml::Value>) -> u32 {
    match manifest.get("package").and_then(|p| p.get("edition")) {
        Some(toml::Value::String(e)) => e.parse().unwrap_or(2015),
        Some(t @ toml::Value::Table(_))
            if t.get("workspace").and_then(toml::Value::as_bool) == Some(true) =>
        {
            workspace
                .and_then(|w| w.get("package")?.get("edition")?.as_str()?.parse().ok())
                .unwrap_or(2015)
        }
        _ => 2015,
    }
}

/// The editions of the crate's targets whose files the walk reads, by
/// which its macros are matched ([`expand::set_edition`]): the library's
/// (`[lib] edition`, else the package's), where the package has a library
/// (a `[lib] path`, or `src/lib.rs` unless `autolib = false` and no
/// `[lib]`), and each binary's (`[[bin]]
/// edition`, else the package's), with workspace inheritance as
/// [`crate_manifest`] reads it. The binaries are the `[[bin]]` tables and,
/// unless `autobins = false`, those cargo finds as [`crate_roots`] does
/// (`src/main.rs`, each `src/bin/NAME.rs` and `src/bin/NAME/main.rs`), at
/// the package's edition but where a `[[bin]]` of that name or path gives
/// its own. A `[[test]]`, `[[bench]]` or `[[example]]` ships nothing, but
/// one whose `path` is a file under `src/` is a root the walk reads as
/// shipped, so its edition is listed too; one outside `src/` is never
/// read.
fn walked_editions(root: &Path, dir: &Path) -> Vec<u32> {
    let read = |p: &Path| -> toml::Table {
        fs::read_to_string(p.join("Cargo.toml"))
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    };
    let manifest = read(dir);
    let workspace = read(root);
    let package = package_edition(&manifest, workspace.get("workspace"));
    let own = |t: &toml::Value| {
        t.get("edition").map_or(package, |e| {
            e.as_str().and_then(|e| e.parse().ok()).unwrap_or(2015)
        })
    };
    let lib = manifest.get("lib");
    let autolib = manifest
        .get("package")
        .and_then(|p| p.get("autolib"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    let has_lib = lib.is_some_and(|l| l.get("path").is_some())
        || ((lib.is_some() || autolib) && dir.join("src/lib.rs").is_file());
    let mut out: Vec<u32> = has_lib
        .then(|| lib.map_or(package, own))
        .into_iter()
        .collect();
    let bins: Vec<&toml::Value> = manifest
        .get("bin")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .collect();
    out.extend(bins.iter().map(|b| own(b)));
    let src = fs::canonicalize(dir.join("src")).ok();
    for kind in ["test", "bench", "example"] {
        out.extend(
            manifest
                .get(kind)
                .and_then(toml::Value::as_array)
                .into_iter()
                .flatten()
                .filter(|t| {
                    t.get("path")
                        .and_then(toml::Value::as_str)
                        .and_then(|p| fs::canonicalize(dir.join(p)).ok())
                        .is_some_and(|p| src.as_ref().is_some_and(|s| p.starts_with(s)))
                })
                .map(own),
        );
    }
    let autobins = manifest
        .get("package")
        .and_then(|p| p.get("autobins"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    if autobins {
        let src = dir.join("src");
        let package_name = manifest
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        let mut found: Vec<(String, PathBuf)> = Vec::new();
        if src.join("main.rs").is_file() {
            found.push((package_name.to_owned(), src.join("main.rs")));
        }
        if let Ok(rd) = fs::read_dir(src.join("bin")) {
            for e in rd.flatten() {
                let p = e.path();
                let (name, file) = if p.is_dir() {
                    (
                        e.file_name().to_string_lossy().into_owned(),
                        p.join("main.rs"),
                    )
                } else if p.extension().is_some_and(|x| x == "rs") {
                    let stem = p.file_stem().unwrap_or_default().to_string_lossy();
                    (stem.into_owned(), p)
                } else {
                    continue;
                };
                if file.is_file() {
                    found.push((name, file));
                }
            }
        }
        let declared = |name: &str, file: &Path| {
            bins.iter().any(|b| {
                b.get("name").and_then(toml::Value::as_str) == Some(name)
                    || b.get("path")
                        .and_then(toml::Value::as_str)
                        .is_some_and(|p| canonical(&dir.join(p)) == canonical(file))
            })
        };
        out.extend(
            found
                .iter()
                .filter(|(name, file)| !declared(name, file))
                .map(|_| package),
        );
    }
    // A package with no target the walk reads matches by its own edition.
    if out.is_empty() {
        out.push(package);
    }
    out
}

/// The root files of the crate in `dir`, canonical, as its Cargo.toml
/// declares its targets: the `[lib]` `path`, else `src/lib.rs` unless
/// `autolib = false`; each `path` of a `[[bin]]`, `[[example]]`, `[[test]]`
/// and `[[bench]]`; and, unless `autobins = false`, the binaries cargo
/// finds (`src/main.rs`, each `src/bin/NAME.rs` and `src/bin/NAME/main.rs`).
/// The counter reads only `src`, so the examples, tests and benches cargo
/// finds outside it do not matter. With `autobins = false`, a binary file
/// no `[[bin]]` names is no root, so a module it would hold counts as
/// undeclared, never verified. A manifest it cannot read gives the default
/// targets.
fn crate_roots(dir: &Path) -> Vec<PathBuf> {
    let manifest: toml::Table = fs::read_to_string(dir.join("Cargo.toml"))
        .ok()
        .and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_default();
    let auto = |key: &str| {
        manifest
            .get("package")
            .and_then(|p| p.get(key))
            .and_then(toml::Value::as_bool)
            .unwrap_or(true)
    };
    let path_of = |t: &toml::Value| {
        t.get("path")
            .and_then(toml::Value::as_str)
            .map(|p| dir.join(p))
    };
    let src = dir.join("src");
    let mut roots = Vec::new();
    match manifest.get("lib").and_then(path_of) {
        Some(p) => roots.push(p),
        None if auto("autolib") => roots.push(src.join("lib.rs")),
        None => {}
    }
    for kind in ["bin", "example", "test", "bench"] {
        if let Some(targets) = manifest.get(kind).and_then(toml::Value::as_array) {
            roots.extend(targets.iter().filter_map(path_of));
        }
    }
    if auto("autobins") {
        roots.push(src.join("main.rs"));
        if let Ok(rd) = fs::read_dir(src.join("bin")) {
            for e in rd.flatten() {
                let p = e.path();
                roots.push(if p.is_dir() { p.join("main.rs") } else { p });
            }
        }
    }
    roots
        .into_iter()
        .filter_map(|r| fs::canonicalize(r).ok())
        .collect()
}

/// [`ModFiles`] for the parsed module files `files`, each with its path,
/// of the crate whose root files are `roots` ([`crate_roots`]). The pass
/// runs twice: the first learns which files a `#[path]` names, so that the
/// second resolves the modules those files declare beside them. Errors
/// are those of the second pass.
fn module_files(
    files: &[(PathBuf, verus_syn::File)],
    roots: Vec<PathBuf>,
    atoms: &Atoms,
    errors: &mut Vec<String>,
) -> ModFiles {
    let mut first = ModFiles {
        roots: roots.clone(),
        ..ModFiles::default()
    };
    for (f, parsed) in files {
        test_module_files(f, parsed, atoms, &mut first, &mut Vec::new());
    }
    let mut out = ModFiles {
        path_named: first.path_named,
        roots,
        ..ModFiles::default()
    };
    for (f, parsed) in files {
        test_module_files(f, parsed, atoms, &mut out, errors);
    }
    out
}

/// Adds to `out` the file `file` when its inner attributes, `attrs`, hold
/// a `cfg` (`#![cfg(target_os = "windows")]` atop a module file or a crate
/// root, which then guards every file under it), and then the files of
/// modules declared in `file` under a `cfg`
/// ([`ModFiles`]), whether it builds them only for tests (`#[cfg(test)]
/// mod name;`, `#[cfg(kani)]`), only for Verus (`#[cfg(verus_keep_ghost)]`,
/// `#[cfg(any(test, verus_keep_ghost))]`), where Verus's build might not
/// compile them (`#[cfg(not(miri))]` with `miri` unsettled), or in every
/// build (`#[cfg(target_arch = "x86_64")]` with it declared, which a
/// function under another architecture's `cfg` in the file conflicts with).
/// It descends into inline modules, `verus!` blocks and other item macros'
/// bodies, whose `cfg`s the declarations inside them inherit; one inside
/// another macro's body is also guarded by [`UNREAD_DECLARATION`]. A
/// `#[cfg_attr(P, path = "..")]` names the module's file where `P` holds
/// and no earlier path attribute's predicate does ([`ordered_paths`], in
/// source order, a bare `#[path]` ending the list), and the default file
/// is the one where none of them applies. A
/// module's file is the one its `#[path]` names, relative to the directory of the
/// module that declares it (`file`'s, for a top-level declaration; an
/// inline module's, inside one), or else `name.rs` or `name/mod.rs` in the
/// directory of the modules the enclosing module declares
/// ([`ModFiles::dir_of`], then each inline module's name, or its `#[path]`
/// from the declaring module's directory). Each is canonical, as `main`
/// compares them. A `cfg` it cannot read is an error, in `errors`, and the
/// module is skipped.
fn test_module_files(
    file: &Path,
    parsed: &verus_syn::File,
    atoms: &Atoms,
    out: &mut ModFiles,
    errors: &mut Vec<String>,
) {
    match attrs_built(&parsed.attrs, atoms) {
        Err(e) => errors.push(format!("{}: {e}", file.display())),
        Ok(_) => {
            let own = cfg_predicates(&parsed.attrs);
            if !own.is_empty()
                && let Ok(f) = fs::canonicalize(file)
            {
                out.inner.push((f, own));
            }
        }
    }
    let canonical = canonical(file);
    let Some(dir) = out.dir_of(&canonical) else {
        return;
    };
    let Some(base) = canonical.parent().map(Path::to_path_buf) else {
        return;
    };
    declared_module_files(file, (&dir, &base), &parsed.items, &[], atoms, out, errors);
    // Where a file's modules live depends on how a target reads it: beside
    // it for a crate root, a `#[path]`-named file or a `mod.rs`-like
    // main.rs or lib.rs, under `stem/` for a module file declared by its
    // place. A target may read any file but a mod.rs the other way (a root
    // read as `mod tool;`, a main.rs that is `a::main`, a file one target's
    // `#[path]` names and another declares by its place, and any file only
    // that other reading reaches), so every such file has both readings.
    let named = canonical.file_name().and_then(|n| n.to_str());
    let other = match dir == base {
        true => base.join(canonical.file_stem().unwrap_or_default()),
        false => base.clone(),
    };
    if named != Some("mod.rs") && other != dir {
        let mut alt = ModFiles {
            path_named: out.path_named.clone(),
            roots: out.roots.clone(),
            ..ModFiles::default()
        };
        declared_module_files(
            file,
            (&other, &base),
            &parsed.items,
            &[],
            atoms,
            &mut alt,
            &mut Vec::new(),
        );
        out.alternate
            .extend(alt.declared.into_iter().map(|d| Declared {
                under_path: true,
                ..d
            }));
    }
}

/// [`test_module_files`] for the items of one module: `dir` is where its
/// `mod name;` files live, `base` the declaring module's directory, which
/// a `#[path]` is relative to, and `guards` the `cfg` predicates of the
/// inline modules and macros around it.
fn declared_module_files(
    file: &Path,
    (dir, base): (&Path, &Path),
    items: &[Item],
    guards: &[String],
    atoms: &Atoms,
    out: &mut ModFiles,
    errors: &mut Vec<String>,
) {
    for item in items {
        let attrs = match item {
            Item::Mod(m) => &m.attrs,
            Item::Macro(m) => &m.attrs,
            _ => continue,
        };
        if let Err(e) = attrs_built(attrs, atoms) {
            errors.push(format!("{}: {e}", file.display()));
            continue;
        }
        let mut inner = guards.to_vec();
        inner.extend(cfg_predicates(attrs));
        if let Item::Macro(m) = item {
            if verus_path(&m.mac.path) {
                // The walk reports a body it cannot parse.
                if let Ok(f) = parse2::<verus_syn::File>(m.mac.tokens.clone()) {
                    declared_module_files(file, (dir, base), &f.items, &inner, atoms, out, errors);
                }
            } else {
                inner.push(UNREAD_DECLARATION.to_owned());
                let mut found = Vec::new();
                macro_module_files(m.mac.tokens.clone(), (dir, base), false, &mut found);
                let from = canonical(file);
                for (f, named, under_path) in found {
                    let Ok(f) = fs::canonicalize(f) else { continue };
                    if named {
                        out.path_named.push(f.clone());
                    }
                    out.declared.push(Declared {
                        file: f,
                        from: from.clone(),
                        guards: inner.clone(),
                        under_path,
                    });
                }
            }
            continue;
        }
        let Item::Mod(m) = item else { continue };
        let name = m.ident.to_string();
        if let Some((_, items)) = &m.content {
            // The inline module's directory is chosen as an out-of-line
            // module's file is: by its path attributes in source order.
            for (path, extra) in path_choices(&m.attrs) {
                let named = path.is_some();
                let sub = match path {
                    Some(p) => base.join(p),
                    None => dir.join(&name),
                };
                let mut guards = inner.clone();
                guards.extend(extra);
                let before = out.declared.len();
                declared_module_files(file, (&sub, &sub), items, &guards, atoms, out, errors);
                if named {
                    for d in &mut out.declared[before..] {
                        d.under_path = true;
                    }
                }
            }
            continue;
        }
        // Each file the declaration may name, whether a `#[path]` names
        // it, and the predicates beyond `inner` under which it does.
        let mut candidates: Vec<(PathBuf, bool, Vec<String>)> = Vec::new();
        for (path, extra) in path_choices(&m.attrs) {
            match path {
                Some(p) => candidates.push((base.join(p), true, extra)),
                None => candidates.extend(
                    [
                        dir.join(format!("{name}.rs")),
                        dir.join(&name).join("mod.rs"),
                    ]
                    .into_iter()
                    .map(|f| (f, false, extra.clone())),
                ),
            }
        }
        for (f, named, extra) in candidates {
            let Ok(f) = fs::canonicalize(f) else { continue };
            if named {
                out.path_named.push(f.clone());
            }
            let mut guards = inner.clone();
            guards.extend(extra);
            out.declared.push(Declared {
                file: f,
                from: canonical(file),
                guards,
                under_path: false,
            });
        }
    }
}

/// An error for each module file a declaration in `mods` names that the
/// source walk did not read (`walked`, canonical): a `#[path]` to a file
/// not named `*.rs`, or to one outside the crate's `src/`, or a link the
/// walk does not follow to one. rustc compiles its functions, so leaving
/// them out of the counts would raise the verified share. A file only
/// tests build is left out of the counts anyway, so it may sit anywhere.
/// The declarations under the other reading of where a file's modules
/// live (`alternate`: a main.rs or lib.rs that is a module declares them
/// under `stem/`) count too, as a target may compile them.
///
/// The exemption reads reachability, not composed guards. A shipped build
/// may compile an entry point (a crate root, or a file no declaration of
/// either reading names, which may be a target the counter does not
/// know), and any file a declaration names from such a file when that
/// declaration is not test-only on its own (by its guards and the
/// declaring file's inner `cfg`s). Such a declaration fails when the walk
/// did not read its file. Each step is judged alone, so guards that keep
/// a path out of shipped builds only together give a false failure,
/// never a pass. The file's own inner `cfg`s are not read, as the walk
/// did not read the file: one whose `#![cfg(test)]` alone keeps it out of
/// shipped builds fails too.
fn unread_module_files(walked: &[PathBuf], mods: &ModFiles, atoms: &Atoms) -> Vec<String> {
    let both: Vec<&Declared> = mods.declared.iter().chain(&mods.alternate).collect();
    let inner = |f: &Path| -> Vec<String> {
        mods.inner
            .iter()
            .filter(|(i, _)| i == f)
            .flat_map(|(_, g)| g.iter().cloned())
            .collect()
    };
    // Whether a declaration is test-only on its own.
    let test_only = |d: &Declared| {
        let g = [d.guards.clone(), inner(&d.from)].concat();
        guarded_built(&g, atoms).is_ok_and(|b| b == Built::Test)
    };
    let mut shipped: Vec<PathBuf> = mods.roots.clone();
    for d in &both {
        if !both.iter().any(|e| e.file == d.from) && !shipped.contains(&d.from) {
            shipped.push(d.from.clone());
        }
    }
    loop {
        let more: Vec<PathBuf> = both
            .iter()
            .filter(|d| shipped.contains(&d.from) && !shipped.contains(&d.file) && !test_only(d))
            .map(|d| d.file.clone())
            .collect();
        if more.is_empty() {
            break;
        }
        for f in more {
            if !shipped.contains(&f) {
                shipped.push(f);
            }
        }
    }
    let mut out: Vec<String> = Vec::new();
    for d in &both {
        if walked.contains(&d.file) || !shipped.contains(&d.from) || test_only(d) {
            continue;
        }
        let e = format!(
            "{}: a module declaration names {}, which the counter does not read (it reads the `*.rs` files under the crate's src/); name the file `*.rs` under src/, or put the module under a test-only `cfg`",
            d.from.display(),
            d.file.display()
        );
        if !out.contains(&e) {
            out.push(e);
        }
    }
    out
}

/// The places a module with `attrs` may be: each path its path attributes
/// give ([`ordered_paths`]), or `None` for its default place, each with the
/// predicates under which it is the one. rustc takes the first path
/// attribute in effect, in source order: each path is the module's where
/// its predicate holds and no earlier one's does; a bare `#[path]` holds
/// wherever none earlier does and ends the list; the default is the
/// module's where no path applies.
fn path_choices(attrs: &[Attribute]) -> Vec<(Option<String>, Vec<String>)> {
    let excluded = |earlier: &[String]| {
        (!earlier.is_empty()).then(|| format!("not(any({}))", earlier.join(",")))
    };
    let mut out = Vec::new();
    let mut earlier: Vec<String> = Vec::new();
    for (pred, p) in ordered_paths(attrs) {
        let mut extra: Vec<String> = pred.iter().cloned().collect();
        extra.extend(excluded(&earlier));
        out.push((Some(p), extra));
        match pred {
            Some(pred) => earlier.push(pred),
            None => return out,
        }
    }
    out.push((None, excluded(&earlier).into_iter().collect()));
    out
}

/// The path attributes of `attrs` in source order: each `#[path = ".."]`
/// with no predicate, and each `path = ".."` a `cfg_attr` applies, at any
/// depth ([`applied_attrs`]), with the conjunction of the predicates
/// around it.
fn ordered_paths(attrs: &[Attribute]) -> Vec<(Option<String>, String)> {
    applied_attrs(attrs)
        .into_iter()
        .filter_map(|(preds, toks)| {
            path_in_attribute(toks.into_iter().collect())
                .map(|p| ((!preds.is_empty()).then(|| all_of(&preds)), p))
        })
        .collect()
}

/// The files of the `mod name;` declarations in a macro body, `tokens`,
/// at any depth, each with whether a `#[path]` named it: the file a
/// `#[path = ".."]` before the declaration names, relative to `base` (the
/// declaring module's directory), or else `name.rs` or `name/mod.rs` under
/// `dir`. Inside an inline `mod outer { .. }` in the body, both are its
/// directory: `outer` under `dir`, or its own `#[path]` from `base`. Each
/// file also carries whether it is declared inside an inline module whose
/// directory a `#[path]` chose (`under_path` here, or one in the body), as
/// [`Declared::under_path`].
fn macro_module_files(
    tokens: TokenStream,
    (dir, base): (&Path, &Path),
    under_path: bool,
    out: &mut Vec<(PathBuf, bool, bool)>,
) {
    let toks: Vec<TokenTree> = tokens.into_iter().collect();
    // The `#[path]` of the item the attributes so far precede.
    let mut path: Option<String> = None;
    let mut skip = 0;
    for (i, t) in toks.iter().enumerate() {
        if skip > 0 {
            skip -= 1;
            continue;
        }
        match (t, toks.get(i + 1), toks.get(i + 2)) {
            (TokenTree::Punct(p), Some(TokenTree::Group(g)), _)
                if p.as_char() == '#' && g.delimiter() == proc_macro2::Delimiter::Bracket =>
            {
                if let Some(p) = path_in_attribute(g.stream()) {
                    path = Some(p);
                }
                skip = 1;
            }
            (TokenTree::Ident(k), Some(TokenTree::Ident(name)), Some(TokenTree::Punct(p)))
                if k == "mod" && p.as_char() == ';' =>
            {
                match path.take() {
                    Some(p) => out.push((base.join(p), true, under_path)),
                    None => {
                        out.push((dir.join(format!("{name}.rs")), false, under_path));
                        out.push((dir.join(name.to_string()).join("mod.rs"), false, under_path));
                    }
                }
                skip = 2;
            }
            (TokenTree::Ident(k), Some(TokenTree::Ident(name)), Some(TokenTree::Group(g)))
                if k == "mod" && g.delimiter() == proc_macro2::Delimiter::Brace =>
            {
                let (sub, chosen) = match path.take() {
                    Some(p) => (base.join(p), true),
                    None => (dir.join(name.to_string()), under_path),
                };
                macro_module_files(g.stream(), (&sub, &sub), chosen, out);
                skip = 2;
            }
            // A restricted visibility (`pub(crate)`, `pub(super)`,
            // `pub(in path)`) is part of the item the attributes precede.
            (TokenTree::Group(g), _, _)
                if g.delimiter() == proc_macro2::Delimiter::Parenthesis
                    && i > 0
                    && matches!(&toks[i - 1], TokenTree::Ident(v) if v == "pub") => {}
            (TokenTree::Group(g), _, _) => {
                path = None;
                macro_module_files(g.stream(), (dir, base), under_path, out);
            }
            (TokenTree::Punct(p), _, _) if p.as_char() == ';' => path = None,
            _ => {}
        }
    }
}

/// The string an attribute's tokens, `path = ".."`, give, if they are a
/// `#[path]`'s.
fn path_in_attribute(tokens: TokenStream) -> Option<String> {
    let toks: Vec<TokenTree> = tokens.into_iter().collect();
    let [
        TokenTree::Ident(k),
        TokenTree::Punct(eq),
        TokenTree::Literal(l),
    ] = toks.as_slice()
    else {
        return None;
    };
    if k != "path" || eq.as_char() != '=' {
        return None;
    }
    match verus_syn::Lit::new(l.clone()) {
        verus_syn::Lit::Str(s) => Some(s.value()),
        _ => None,
    }
}

/// `file`, canonical, or as given when it cannot be.
fn canonical(file: &Path) -> PathBuf {
    fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf())
}

/// The directory of the modules a module file declares: a `mod.rs`'s (or
/// crate root's) own, else `foo/` beside `foo.rs`.
fn module_dir(file: &Path) -> Option<PathBuf> {
    match file.file_name().and_then(|n| n.to_str()) {
        Some("mod.rs" | "lib.rs" | "main.rs") => file.parent().map(Path::to_path_buf),
        _ => file
            .parent()
            .map(|p| p.join(file.file_stem().unwrap_or_default())),
    }
}

/// The `cfg` predicates that must hold for the file `f` to be built, given
/// the module files `mods` records: its inner `#![cfg(..)]`s, and for each
/// declaration of it, that declaration's guards and the declaring file's
/// own ([`module_guards`] of it). The declarations are those of every
/// reading that may compile it: the primary one's and the other's
/// ([`ModFiles::alternate`]). A file declared once takes that
/// declaration's; one declared several times is built where any
/// declaration holds, so it takes their `any(all(..), ..)`, and nothing
/// more when one declaration needs nothing. A crate root is built by its
/// own target too, which needs nothing, so a root has its inner `cfg`s
/// alone, whatever declares it as a module. A crate file no declaration
/// of the primary reading names, other than a root, is unsure
/// ([`UNREAD_DECLARATION`]) and takes the guards of each recorded file
/// whose module directory ([`module_dir`]) it is under, so one under a
/// test-only module stays test-only; the other reading's declarations of
/// it are alternatives beside that one, as they only widen.
fn module_guards(f: &Path, mods: &ModFiles) -> Vec<String> {
    guards_of(f, mods, &mut Vec::new())
}

/// [`module_guards`], with `seen` the files whose guards are being read,
/// so that a declaration cycle ends.
fn guards_of(f: &Path, mods: &ModFiles, seen: &mut Vec<PathBuf>) -> Vec<String> {
    if seen.iter().any(|s| s == f) {
        return Vec::new();
    }
    seen.push(f.to_path_buf());
    let mut out: Vec<String> = mods
        .inner
        .iter()
        .filter(|(i, _)| i == f)
        .flat_map(|(_, g)| g.iter().cloned())
        .collect();
    // Every reading that may compile the file: the declarations of the
    // primary one and of the other ([`ModFiles::alternate`]), and for a
    // crate root its own target, which needs no declaration.
    // An alternate declaration is a reading some target may not make
    // (a root's `mod foo;` read under `src/lib/`), so it shows only that
    // the file may be built: it is unsure ([`UNREAD_DECLARATION`]), and
    // so never verified through it.
    let declarations: Vec<(&Declared, bool)> = mods
        .declared
        .iter()
        .map(|d| (d, false))
        .chain(mods.alternate.iter().map(|d| (d, true)))
        .filter(|(d, _)| d.file == f)
        .collect();
    let root = mods.roots.iter().any(|r| r == f);
    // No declaration of the primary reading names this file, so nothing
    // the counter reads shows where it is built (an `include!` may
    // compile it): it is unsure, and keeps the guards of the files whose
    // module directory holds it.
    let undeclared = !root && declarations.iter().all(|(_, alternate)| *alternate);
    let fallback = |seen: &mut Vec<PathBuf>| {
        let mut g = vec![UNREAD_DECLARATION.to_owned()];
        let mut above: Vec<&PathBuf> = mods
            .inner
            .iter()
            .map(|(m, _)| m)
            .chain(mods.declared.iter().map(|d| &d.file))
            .filter(|m| *m != f && module_dir(m).is_some_and(|d| f.starts_with(d)))
            .collect();
        above.sort();
        above.dedup();
        for m in above {
            g.extend(guards_of(m, mods, seen));
        }
        g
    };
    if declarations.is_empty() && !root {
        out.extend(fallback(seen));
    } else if !declarations.is_empty() {
        let mut alternatives: Vec<Vec<String>> = declarations
            .iter()
            .map(|(d, alternate)| {
                let mut g = d.guards.clone();
                if *alternate {
                    g.push(UNREAD_DECLARATION.to_owned());
                }
                g.extend(guards_of(&d.from, mods, seen));
                g
            })
            .collect();
        if root {
            alternatives.push(Vec::new());
        }
        // The other reading's declarations only widen where the file is
        // built: one the primary reading does not declare keeps the
        // fallback beside them.
        if undeclared {
            alternatives.push(fallback(seen));
        }
        match alternatives.as_slice() {
            [one] => out.extend(one.iter().cloned()),
            several if several.iter().any(Vec::is_empty) => {}
            several => out.push(format!(
                "any({})",
                several
                    .iter()
                    .map(|g| format!("all({})", g.join(",")))
                    .collect::<Vec<_>>()
                    .join(",")
            )),
        }
    }
    seen.pop();
    out
}

/// Where the file `f` is built ([`guarded_built`] of its
/// [`module_guards`]): a test-only module inside one Verus might skip is
/// test-only, and one under `target_arch = "x86_64"` inside one under
/// `target_arch = "aarch64"` is built nowhere, so test-only too.
fn module_class(f: &Path, mods: &ModFiles, atoms: &Atoms) -> Result<Built, String> {
    guarded_built(&module_guards(f, mods), atoms)
}

/// Where a `cfg` lets an item be built: by a shipped build, and surely by
/// Verus's; by a shipped build, where Verus's build might not compile it
/// (`unix`, or any atom the configuration does not settle), so nothing
/// proves Verus checked it; only where Verus checks the code
/// (`verus_keep_ghost`), so Verus checks it and no shipped build compiles
/// it; or only for tests (`test`, `kani`, the `test-util` feature, which
/// only tests enable: [`test_util_enablers`] fails the run otherwise),
/// which Verus does not see either.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Built {
    Shipped,
    Unsure,
    Verus,
    Test,
}

/// Whether a `cfg` predicate can be true, and can be false, in one kind of
/// build.
#[derive(Clone, Copy)]
struct Can {
    t: bool,
    f: bool,
}

/// [`Can`] for the predicate `p` in a shipped build (`verus` false: none
/// of `atoms`' test or Verus atoms) or a Verus build (the Verus atoms and
/// the ones it is sure to set, and none of the test ones or the ones it
/// never sets). Any other name or feature may be either.
/// `all`, `any` and `not` nest to any depth; a predicate it cannot read
/// is an error.
fn cfg_eval(p: &str, verus: bool, atoms: &Atoms) -> Result<Can, String> {
    let p = p.trim();
    let call = |name: &str| {
        p.strip_prefix(name)
            .map(str::trim_start)
            .and_then(|r| r.strip_prefix('('))
            .and_then(|r| r.strip_suffix(')'))
    };
    if let Some(inner) = call("all") {
        // A conjunction that needs two values of one single-valued key
        // holds in no build, whatever the atoms say on their own.
        if cfg_forced(p, verus, atoms)?.is_none() {
            return Ok(Can { t: false, f: true });
        }
        let mut c = Can { t: true, f: false };
        for a in cfg_args(inner)? {
            let x = cfg_eval(a, verus, atoms)?;
            c.t &= x.t;
            c.f |= x.f;
        }
        return Ok(c);
    }
    if let Some(inner) = call("any") {
        let mut c = Can { t: false, f: true };
        for a in cfg_args(inner)? {
            let x = cfg_eval(a, verus, atoms)?;
            c.t |= x.t;
            c.f &= x.f;
        }
        return Ok(c);
    }
    if let Some(inner) = call("not") {
        let args = cfg_args(inner)?;
        let [a] = args.as_slice() else {
            return Err(format!("unreadable cfg predicate `{p}`: not takes one"));
        };
        let x = cfg_eval(a, verus, atoms)?;
        return Ok(Can { t: x.f, f: x.t });
    }
    let w: String = p.split_whitespace().collect();
    // Outside a string, a name has no parenthesis or comma.
    let bare: String = w.split('"').step_by(2).collect();
    if w.is_empty() || bare.contains(['(', ')', ',']) {
        return Err(format!("unreadable cfg predicate `{p}`"));
    }
    Ok(if atoms.test.contains(&w) {
        Can { t: false, f: true }
    } else if atoms.verus.contains(&w) {
        Can {
            t: verus,
            f: !verus,
        }
    } else if verus && atoms.sets.contains(&w) {
        Can { t: true, f: false }
    } else if verus && atoms.never.contains(&w) {
        Can { t: false, f: true }
    } else if verus && atoms.one_run && other_value_set(&w, &atoms.sets) {
        // One declared run (#21) sets one value of a single-valued key,
        // so every other value of it is false there.
        Can { t: false, f: true }
    } else {
        Can { t: true, f: true }
    })
}

/// Whether `sets` holds another value of the single-valued key
/// ([`SINGLE_VALUED`]) of the atom `w` (`target_arch="aarch64"` for
/// `target_arch="x86_64"`), both without whitespace.
fn other_value_set(w: &str, sets: &[String]) -> bool {
    let Some((k, _)) = w.split_once('=') else {
        return false;
    };
    SINGLE_VALUED.contains(&k)
        && sets
            .iter()
            .any(|s| s.as_str() != w && s.split_once('=').is_some_and(|(sk, _)| sk == k))
}

/// The `cfg` keys rustc gives exactly one value per target (`rustc --print
/// cfg`): `target_family` is not one (emscripten is both `unix` and
/// `wasm`), nor are `feature`, `target_feature` and `target_has_atomic`.
const SINGLE_VALUED: [&str; 8] = [
    "target_arch",
    "target_os",
    "target_env",
    "target_abi",
    "target_vendor",
    "target_endian",
    "target_pointer_width",
    "panic",
];

/// Single-valued keys and the one value a predicate needs each to have.
type Forced = std::collections::BTreeMap<String, String>;

/// The single-valued `key="value"` atoms ([`SINGLE_VALUED`]) the predicate
/// `p` needs to hold, without whitespace: each conjunct's, and those every
/// alternative of an `any` needs. `None` when no build can meet it: it
/// needs two values of one key (`all(target_arch = "x86_64", target_arch
/// = "aarch64")`), or it is an `any` none of whose alternatives can hold
/// (`any()`). An alternative that cannot hold in the build `verus` and
/// `atoms` describe ([`cfg_eval`]: `test`, here) needs nothing of the
/// others, so `all(target_arch = "x86_64", any(target_arch = "aarch64",
/// test))` needs both values. A `not` needs nothing. A predicate it cannot
/// read is an error, as [`cfg_eval`] reads it.
fn cfg_forced(p: &str, verus: bool, atoms: &Atoms) -> Result<Option<Forced>, String> {
    let p = p.trim();
    let call = |name: &str| {
        p.strip_prefix(name)
            .map(str::trim_start)
            .and_then(|r| r.strip_prefix('('))
            .and_then(|r| r.strip_suffix(')'))
    };
    if let Some(inner) = call("all") {
        let mut out = Forced::new();
        for a in cfg_args(inner)? {
            let Some(f) = cfg_forced(a, verus, atoms)? else {
                return Ok(None);
            };
            for (k, v) in f {
                if out.get(&k).is_some_and(|w| *w != v) {
                    return Ok(None);
                }
                out.insert(k, v);
            }
        }
        return Ok(Some(out));
    }
    if let Some(inner) = call("any") {
        let mut out: Option<Forced> = None;
        for a in cfg_args(inner)? {
            if !cfg_eval(a, verus, atoms)?.t {
                continue;
            }
            let Some(f) = cfg_forced(a, verus, atoms)? else {
                continue;
            };
            out = Some(match out {
                None => f,
                Some(o) => o.into_iter().filter(|(k, v)| f.get(k) == Some(v)).collect(),
            });
        }
        return Ok(out);
    }
    if call("not").is_some() {
        return Ok(Some(Forced::new()));
    }
    let w: String = p.split_whitespace().collect();
    Ok(Some(match w.split_once('=') {
        Some((k, v)) if SINGLE_VALUED.contains(&k) => [(k.to_owned(), v.to_owned())].into(),
        _ => Forced::new(),
    }))
}

/// The ways the predicate `p` can hold, each as the atoms of the keys in
/// `multi` it needs true (without whitespace): `all` joins its arguments'
/// ways, `any` lists each argument's, and `not` swaps the two, so an atom
/// under an odd number of `not`s is needed false and adds nothing, and one
/// under an even number is needed true. With `negated`, the ways `p` can
/// fail. A predicate it cannot read is an error.
fn cfg_ways(
    p: &str,
    negated: bool,
    multi: &[String],
) -> Result<Vec<std::collections::BTreeSet<String>>, String> {
    let p = p.trim();
    let call = |name: &str| {
        p.strip_prefix(name)
            .map(str::trim_start)
            .and_then(|r| r.strip_prefix('('))
            .and_then(|r| r.strip_suffix(')'))
    };
    if let Some(inner) = call("not") {
        let mut out = Vec::new();
        for a in cfg_args(inner)? {
            out.extend(cfg_ways(a, !negated, multi)?);
        }
        return Ok(out);
    }
    for (name, joins) in [("all", !negated), ("any", negated)] {
        let Some(inner) = call(name) else { continue };
        let mut out = vec![std::collections::BTreeSet::new()];
        if !joins {
            out.clear();
        }
        for a in cfg_args(inner)? {
            let ways = cfg_ways(a, negated, multi)?;
            out = if joins {
                out.iter()
                    .flat_map(|o| ways.iter().map(move |w| o.union(w).cloned().collect()))
                    .collect()
            } else {
                out.into_iter().chain(ways).collect()
            };
        }
        return Ok(out);
    }
    let w: String = p.split_whitespace().collect();
    let counts = !negated
        && w.split_once('=')
            .is_some_and(|(k, _)| multi.iter().any(|m| m == k));
    Ok(vec![if counts {
        [w].into()
    } else {
        std::collections::BTreeSet::new()
    }])
}

/// Whether the predicate `p` names atoms of more than one of the proof
/// runs the configuration declares: `verus_sets` holds two values of a
/// single-valued key ([`SINGLE_VALUED`]) when a port runs Verus on several
/// hosts (challoc's `target_arch = "x86_64"` and `target_arch =
/// "aarch64"`), and the counter reads each atom on its own, not each run's
/// set. Under one such atom a function is checked by the run that sets it.
/// When some way the predicate can hold ([`cfg_ways`]) needs two or more
/// distinct ones true (`target_os = "macos"` on a module and `target_arch
/// = "x86_64"` on a function in it, with both oses and both arches
/// declared, or the same with `not(not(target_os = "macos"))`), it may
/// ship on a host no run is, so it is not sure. Alternatives of an `any`
/// are ways of their own: `any(target_arch = "x86_64", target_arch =
/// "aarch64")` needs one atom either way.
fn names_several_runs(p: &str, atoms: &Atoms) -> Result<bool, String> {
    let key = |a: &str| a.split_once('=').map(|(k, _)| k.to_owned());
    let multi: Vec<String> = SINGLE_VALUED
        .iter()
        .filter(|k| {
            atoms
                .sets
                .iter()
                .filter(|a| key(a).as_deref() == Some(**k))
                .count()
                > 1
        })
        .map(|k| (*k).to_owned())
        .collect();
    if multi.is_empty() {
        return Ok(false);
    }
    Ok(cfg_ways(p, false, &multi)?.iter().any(|w| w.len() > 1))
}

/// The arguments of an `all`, `any` or `not`: split at the commas outside
/// any parentheses and string.
fn cfg_args(inner: &str) -> Result<Vec<&str>, String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut quoted = false;
    let mut start = 0;
    for (i, ch) in inner.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => {
                depth -= 1;
                if depth < 0 {
                    return Err(format!(
                        "unreadable cfg predicate `{inner}`: a `)` too many"
                    ));
                }
            }
            ',' if !quoted && depth == 0 => {
                out.push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if depth != 0 || quoted {
        return Err(format!("unreadable cfg predicate `{inner}`: unbalanced"));
    }
    // A trailing comma ends the list; an empty argument before it is an
    // error, read as an empty name.
    if !inner[start..].trim().is_empty() {
        out.push(&inner[start..]);
    }
    Ok(out)
}

/// Where the `cfg` predicate `tokens` lets an item be built ([`Built`]).
/// A shipped item is [`Built::Unsure`] when the predicate can be false in
/// Verus's build: only the atoms the configuration settles there are sure,
/// and only one run's of them at a time ([`names_several_runs`]).
fn cfg_built(tokens: &str, atoms: &Atoms) -> Result<Built, String> {
    let verus = verus_can(tokens, atoms)?;
    Ok(if cfg_eval(tokens, false, atoms)?.t {
        if verus.f || names_several_runs(tokens, atoms)? {
            Built::Unsure
        } else {
            Built::Shipped
        }
    } else if verus.t {
        Built::Verus
    } else {
        Built::Test
    })
}

/// Each attribute `attrs` apply, through `cfg_attr`s nested to any depth,
/// as rustc expands them: its tokens (`path = ".."`, `cfg(Q)`,
/// `verus_verify`) with the predicates of the `cfg_attr`s around it, all of
/// which must hold for it to apply. `cfg_attr(P, a, b)` applies each of `a`
/// and `b` under `P`; an attribute outside any `cfg_attr` has none.
fn applied_attrs(attrs: &[Attribute]) -> Vec<(Vec<String>, Vec<TokenTree>)> {
    let mut out = Vec::new();
    for a in attrs {
        let toks: Vec<TokenTree> = verus_syn::__private::ToTokens::to_token_stream(&a.meta)
            .into_iter()
            .collect();
        applied(&toks, &[], &mut out);
    }
    out
}

/// [`applied_attrs`] for one attribute's tokens, `toks`, under `preds`.
fn applied(toks: &[TokenTree], preds: &[String], out: &mut Vec<(Vec<String>, Vec<TokenTree>)>) {
    if let [TokenTree::Ident(i), TokenTree::Group(g)] = toks
        && i == "cfg_attr"
        && g.delimiter() == proc_macro2::Delimiter::Parenthesis
    {
        let inner: Vec<TokenTree> = g.stream().into_iter().collect();
        let mut parts = inner.split(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == ','));
        let Some(predicate) = parts.next() else {
            return;
        };
        let mut preds = preds.to_vec();
        preds.push(
            predicate
                .iter()
                .cloned()
                .collect::<TokenStream>()
                .to_string(),
        );
        for part in parts.filter(|p| !p.is_empty()) {
            applied(part, &preds, out);
        }
        return;
    }
    out.push((preds.to_vec(), toks.to_vec()));
}

/// The conjunction of the non-empty `preds`, as one predicate.
fn all_of(preds: &[String]) -> String {
    match preds {
        [one] => one.clone(),
        _ => format!("all({})", preds.join(",")),
    }
}

/// The predicates of `attrs`' `cfg`s, all of which must hold: each
/// `#[cfg(Q)]`'s `Q`, and each `cfg(Q)` a `cfg_attr` applies under `P`
/// (at any depth, [`applied_attrs`]) as `any(not(P), Q)`, since the item is
/// left out only where `P` holds and `Q` does not.
fn cfg_predicates(attrs: &[Attribute]) -> Vec<String> {
    let mut out = Vec::new();
    for (preds, toks) in applied_attrs(attrs) {
        if let [TokenTree::Ident(id), TokenTree::Group(g)] = toks.as_slice()
            && id == "cfg"
            && g.delimiter() == proc_macro2::Delimiter::Parenthesis
        {
            out.push(if preds.is_empty() {
                g.stream().to_string()
            } else {
                format!("any(not({}),{})", all_of(&preds), g.stream())
            });
        }
    }
    out
}

/// Where an item under every one of the `cfg` predicates `guards` is
/// built: their conjunction, read as one predicate, so that two guards
/// needing two values of one single-valued key (`#[cfg(target_arch =
/// "x86_64")]` on a module, `#[cfg(target_arch = "aarch64")]` on a
/// function in it) hold in no build ([`cfg_forced`]).
fn guarded_built(guards: &[String], atoms: &Atoms) -> Result<Built, String> {
    match guards {
        [] => Ok(Built::Shipped),
        [one] => cfg_built(one, atoms),
        _ => cfg_built(&format!("all({})", guards.join(",")), atoms),
    }
}

/// Whether Verus's build is sure to compile an item under every one of
/// the `cfg` predicates `guards`: their conjunction, read as one predicate
/// as [`guarded_built`] reads it, cannot be false there and needs at most
/// one run's atoms ([`names_several_runs`]). Two guards needing two values
/// of one single-valued key, each sure alone, hold in no build.
fn guarded_verus_sure(guards: &[String], atoms: &Atoms) -> Result<bool, String> {
    if guards.is_empty() {
        return Ok(true);
    }
    let all = format!("all({})", guards.join(","));
    let can = verus_can(&all, atoms)?;
    Ok(can.t && !can.f && !names_several_runs(&all, atoms)?)
}

/// Whether every proof run that compiles a module is sure to compile an
/// item in it: the item under the module's `cfg`s `shared` (those of the
/// file and the modules around it, which every invocation in the module
/// is under too) and its own `own` (its attributes and the `verus!`
/// blocks around it). Their conjunction must be sure
/// ([`guarded_verus_sure`]), and `own` must hold in each run, not only in
/// one: where `verus_sets` holds two values of a single-valued key
/// ([`SINGLE_VALUED`]), each run sets one of them, so an atom of that key
/// is unsettled for `own` unless `shared` forces the key's value
/// ([`cfg_forced`], read with those atoms unsettled as well), which every
/// run compiling the module then has. So
/// `#[cfg(target_arch = "x86_64")] use vstd::prelude::*;` beside an
/// unguarded `verus!` is not sure, as the aarch64 run compiles the
/// invocation without the import, while the same import in a module under
/// `target_arch = "x86_64"` is.
fn every_run_sure(shared: &[String], own: &[String], atoms: &Atoms) -> Result<bool, String> {
    if !guarded_verus_sure(&[shared, own].concat(), atoms)? {
        return Ok(false);
    }
    if own.is_empty() {
        return Ok(true);
    }
    // With [[runs]] declared (#21), each run that may compile the module
    // must be sure to compile the item there: `own` holds wherever
    // `shared` does, in that run's atoms.
    if !atoms.runs.is_empty() {
        let own = all_of(own);
        let implied = match shared {
            [] => own,
            _ => format!("any(not({}),{own})", all_of(shared)),
        };
        for run in atoms.each_run() {
            let can = cfg_eval(&implied, true, &run)?;
            if can.f {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    let key = |a: &str| a.split_once('=').map(|(k, _)| k.to_owned());
    let multi: Vec<&str> = SINGLE_VALUED
        .iter()
        .copied()
        .filter(|k| {
            atoms
                .sets
                .iter()
                .filter(|a| key(a).as_deref() == Some(*k))
                .count()
                > 1
        })
        .collect();
    if multi.is_empty() {
        return Ok(true);
    }
    let mut each = atoms.clone();
    each.sets
        .retain(|a| key(a).is_none_or(|k| !multi.contains(&k.as_str())));
    // What `shared` forces is read with those atoms unsettled too: read
    // with them sure, `any(target_arch = "x86_64", not(target_arch =
    // "x86_64"))` would skip its second alternative as unable to hold,
    // and force a value the aarch64 run does not have.
    let forced = match shared {
        [] => Forced::new(),
        _ => cfg_forced(&format!("all({})", shared.join(",")), true, &each)?.unwrap_or_default(),
    };
    for a in &atoms.sets {
        let Some(k) = key(a) else { continue };
        let Some(v) = forced.get(&k).filter(|_| multi.contains(&k.as_str())) else {
            continue;
        };
        if *a == format!("{k}={v}") {
            each.sets.push(a.clone());
        } else {
            each.never.push(a.clone());
        }
    }
    let can = cfg_eval(&format!("all({})", own.join(",")), true, &each)?;
    Ok(can.t && !can.f)
}

/// Where `attrs`' `cfg`s, all of which must hold, let an item be built.
fn attrs_built(attrs: &[Attribute], atoms: &Atoms) -> Result<Built, String> {
    guarded_built(&cfg_predicates(attrs), atoms)
}

/// Whether a `cfg` predicate keeps an item out of shipped builds.
#[cfg(test)]
fn cfg_needs_test(tokens: &str, atoms: &Atoms) -> bool {
    matches!(cfg_built(tokens, atoms), Ok(Built::Test | Built::Verus))
}

/// The packages that a build of the workspace at `root` compiles with one
/// of the test-only `features`, each with the feature, as cargo itself
/// resolves them: `cargo tree` over every member, for every target, along
/// normal and build edges only, once with the default features and once
/// with every member's every other feature too ([`member_features`]).
/// The counter reads the manifests instead
/// ([`features::test_util_enablers`], #136), so that it needs no cargo;
/// the tests hold that reading to this one, cargo's.
#[cfg(test)]
fn cargo_enablers(root: &Path, test: &[String]) -> Result<Vec<(String, String)>, String> {
    let features = member_features(root, test)?;
    let mut enablers = Vec::new();
    for extra in [None, Some(features.join(","))] {
        let mut args = vec![
            "tree",
            "--locked",
            "--offline",
            "--workspace",
            "--target",
            "all",
            "--edges",
            "normal,build",
            "--prefix",
            "none",
            "--format",
            "{p}\t{f}",
        ];
        if let Some(f) = extra.as_deref().filter(|f| !f.is_empty()) {
            args.extend(["--features", f]);
        }
        let out = cargo(root, &args)?;
        enablers.extend(out.lines().flat_map(|l| {
            let (package, features) = l.split_once('\t').unwrap_or_default();
            features
                .trim_end_matches(" (*)")
                .split(',')
                .filter(|f| test.iter().any(|t| t == f.trim()))
                .map(|f| (package.to_string(), f.trim().to_string()))
                .collect::<Vec<_>>()
        }));
    }
    enablers.sort();
    enablers.dedup();
    Ok(enablers)
}

/// Every feature of every workspace member under `root` but the test-only
/// ones (`test`), as `member/feature`, from `cargo metadata`.
#[cfg(test)]
fn member_features(root: &Path, test: &[String]) -> Result<Vec<String>, String> {
    let out = cargo(
        root,
        &[
            "metadata",
            "--format-version",
            "1",
            "--locked",
            "--offline",
            "--no-deps",
        ],
    )?;
    let meta: serde_json::Value = serde_json::from_str(&out)
        .map_err(|e| format!("cargo metadata in {}: {e}", root.display()))?;
    let mut features = Vec::new();
    for p in meta["packages"].as_array().into_iter().flatten() {
        let Some(name) = p["name"].as_str() else {
            continue;
        };
        for f in p["features"].as_object().into_iter().flat_map(|o| o.keys()) {
            if !test.contains(f) {
                features.push(format!("{name}/{f}"));
            }
        }
    }
    Ok(features)
}

/// `cargo args` in `root` (cargo from `$CARGO`, as `cargo run` and `cargo
/// test` set it, or else from `PATH`): its standard output, or an error.
#[cfg(test)]
fn cargo(root: &Path, args: &[&str]) -> Result<String, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = std::process::Command::new(&cargo)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run {}: {e}", cargo.to_string_lossy()))?;
    if !out.status.success() {
        return Err(format!(
            "cargo {} failed in {}: {}",
            args.first().unwrap_or(&""),
            root.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The `#[cfg_attr(any(), discharged_by = "kani")]` marker: Kani checks
/// this trusted function's contract (verus/README.md, "Kani"). The marker
/// is inert: `discharged_by` is no attribute rustc knows, so it may only
/// sit under predicates that hold in no build. It is read through
/// [`applied_attrs`], so it counts nested or beside other attributes
/// (`cfg_attr(any(), inline, discharged_by = "kani")`), and only where the
/// predicates around it hold in no build whatever the configuration
/// declares (`any()`, or `all(any(), unix)`). Under one that some build
/// may meet (`windows`, `test`) it fails closed: the function is not
/// discharged, as it would not be in that build.
fn is_discharged(attrs: &[Attribute]) -> bool {
    applied_attrs(attrs).into_iter().any(|(preds, toks)| {
        toks.iter()
            .cloned()
            .collect::<TokenStream>()
            .to_string()
            .split_whitespace()
            .collect::<String>()
            == "discharged_by=\"kani\""
            && !preds.is_empty()
            && never_holds(&all_of(&preds))
    })
}

/// Whether the predicate `p` holds in no build, with no atom settled:
/// neither a shipped build nor Verus's can meet it. One it cannot read is
/// taken as one that may hold.
fn never_holds(p: &str) -> bool {
    let none = Atoms::default();
    [false, true]
        .into_iter()
        .all(|verus| cfg_eval(p, verus, &none).is_ok_and(|c| !c.t))
}

/// Whether `attrs` apply `#[name]` in some build: directly, or through a
/// `cfg_attr` whatever its predicate ([`applied_attrs`]).
fn has_applied(attrs: &[Attribute], name: &str) -> bool {
    applied_attrs(attrs)
        .into_iter()
        .any(|(_, toks)| matches!(toks.first(), Some(TokenTree::Ident(i)) if i == name))
}

/// Whether `attrs` make a function external to Verus (`external_body`,
/// `verifier::external`, `verus_verify(external..)`), read through
/// [`applied_attrs`]. It fails closed: an attribute a `cfg_attr` applies
/// counts whatever its predicate, so a function external in some build is
/// trusted, never verified.
fn is_external(attrs: &[Attribute]) -> bool {
    applied_attrs(attrs).into_iter().any(|(_, toks)| {
        // A `name = value` attribute (a doc comment) is read by its name
        // alone, so prose that mentions `external_body` is not one.
        let text = toks
            .into_iter()
            .take_while(|t| !matches!(t, TokenTree::Punct(p) if p.as_char() == '='))
            .collect::<TokenStream>()
            .to_string();
        text.contains("external_body")
            || text.contains("verifier :: external") && !text.contains("external_fn_specification")
            || text.contains("verus_verify (external")
    })
}

/// Whether `attrs` hold `#[verifier::assume_unsupported_attributes]`, read
/// through [`applied_attrs`]. Verus then takes the item's attributes it
/// cannot check (`no_mangle`, `export_name`, `link_section`) on trust,
/// and still checks its body: the function keeps its class, and the
/// attribute is a seam of its own ([`ledger::UNSUPPORTED_ATTRIBUTE`]). It
/// fails closed, so no such seam goes unlisted: the name counts anywhere
/// in an attribute's tokens outside a literal, so in any path and in
/// `#[verifier(assume_unsupported_attributes)]`, which `verus!` rewrites
/// to the path form, and one a `cfg_attr` applies counts whatever its
/// predicate.
fn assumes_attributes(attrs: &[Attribute]) -> bool {
    fn names(toks: TokenStream) -> bool {
        toks.into_iter().any(|t| match t {
            TokenTree::Ident(i) => i == "assume_unsupported_attributes",
            TokenTree::Group(g) => names(g.stream()),
            _ => false,
        })
    }
    applied_attrs(attrs)
        .into_iter()
        .any(|(_, toks)| names(toks.into_iter().collect()))
}

/// Whether `attrs` hold `#[verifier::exec_allows_no_decreases_clause]`,
/// read through [`applied_attrs`]: Verus then checks the exec functions
/// it covers with no `decreases` on their loops or recursion, so proves
/// none of them terminates. It fails closed as [`assumes_attributes`]
/// does: the name counts anywhere in an attribute's tokens outside a
/// literal, so in any path and in
/// `#[verifier(exec_allows_no_decreases_clause)]`, and one a `cfg_attr`
/// applies counts whatever its predicate.
fn allows_no_decreases(attrs: &[Attribute]) -> bool {
    fn names(toks: TokenStream) -> bool {
        toks.into_iter().any(|t| match t {
            TokenTree::Ident(i) => i == "exec_allows_no_decreases_clause",
            TokenTree::Group(g) => names(g.stream()),
            _ => false,
        })
    }
    applied_attrs(attrs)
        .into_iter()
        .any(|(_, toks)| names(toks.into_iter().collect()))
}

/// Whether `attrs` hold Verus's attribute form, which marks a plain Rust
/// function outside `verus!` for verification: `#[verus_verify]` or
/// `#[verus_spec(..)]` (by any path), alone or as an attribute a
/// `cfg_attr` applies (`#[cfg_attr(feature = "verus", verus_spec(..))]`,
/// the form a port whose MSRV is below vstd's uses). Such a function is
/// classed as one inside `verus!` is ([`classify`]): `Some(true)` when
/// Verus's build is sure to apply it, `Some(false)` when it might not, and
/// `None` when it never does. A `cfg_attr` is sure when Verus's build is
/// sure to meet its predicate (by [`cfg_eval`]: it can be true there and
/// cannot be false), as a configured Verus-only feature,
/// `verus_keep_ghost`, a `verus_sets` atom or `not` of a
/// `verus_never_sets` one is. One no build meets (`any()`) or only a test
/// build meets (`test`) never applies the attribute to the code Verus
/// checks. One that names a platform or any other atom the configuration
/// does not settle (`target_os = "windows"`) may not apply it on the host
/// Verus runs on, so the function counts as trusted, as one under a `cfg`
/// Verus might not meet does ([`Built::Unsure`]). A predicate it cannot
/// read is an error. A `cfg_attr`'s predicate is read together with the
/// `cfg`s the function is under, `guards`: one under `target_arch =
/// "x86_64"` whose attribute needs `target_arch = "aarch64"` never applies.
/// With `[[runs]]` declared (#21), the attribute is sure only when every
/// run that may compile the function is sure to apply it
/// ([`every_run_sure`]); the combined class of all runs is not enough, as
/// a run where the `cfg_attr` is false compiles the function without it.
fn verus_attribute(
    attrs: &[Attribute],
    guards: &[String],
    atoms: &Atoms,
) -> Result<Option<bool>, String> {
    const NAMES: [&str; 2] = ["verus_verify", "verus_spec"];
    let mut maybe = false;
    for (preds, toks) in applied_attrs(attrs) {
        // The attribute's path is the tokens before its arguments; it is
        // the attribute form by its last segment.
        let last = toks
            .iter()
            .take_while(|t| {
                !matches!(t, TokenTree::Group(_))
                    && !matches!(t, TokenTree::Punct(p) if p.as_char() == '=')
            })
            .filter_map(|t| match t {
                TokenTree::Ident(i) => Some(i.to_string()),
                _ => None,
            })
            .last();
        if !last.is_some_and(|l| NAMES.contains(&l.as_str())) {
            continue;
        }
        if preds.is_empty() {
            return Ok(Some(true));
        }
        // With [[runs]] declared (#21), each run that may compile the
        // function must be sure to apply the attribute: a run where the
        // `cfg_attr` is false compiles the function without it.
        if !atoms.runs.is_empty() && every_run_sure(guards, &preds, atoms)? {
            return Ok(Some(true));
        }
        let mut all = guards.to_vec();
        all.extend(preds);
        let all = format!("all({})", all.join(","));
        let can = verus_can(&all, atoms)?;
        if atoms.runs.is_empty() && can.t && !can.f && !names_several_runs(&all, atoms)? {
            return Ok(Some(true));
        }
        maybe |= can.t;
    }
    Ok(maybe.then_some(false))
}

/// The `#[verifier::rlimit(N)]` among `attrs`: `Some(Some(N))`,
/// `Some(None)` for `infinity`, or `None` without one. An argument that
/// is neither a number nor `infinity` is an error.
fn rlimit_attr(attrs: &[Attribute]) -> Result<Option<Option<f32>>, String> {
    for a in attrs {
        let Meta::List(l) = &a.meta else { continue };
        if path_string(&l.path) != "verifier :: rlimit" {
            continue;
        }
        let arg = l.tokens.to_string();
        return match arg.trim() {
            "infinity" => Ok(Some(None)),
            n => n
                .parse::<f32>()
                .map(|v| Some(Some(v)))
                .map_err(|_| format!("#[verifier::rlimit({n})]: not a number or infinity")),
        };
    }
    Ok(None)
}

/// The name Verus gives an `impl`'s type or a trait, from
/// [`Counter::owner`]: a path's last segment, without generic arguments
/// (`Machine < 'a , U >` is `Machine`); `None` for free functions. A type
/// that is no path (`& m :: S`, `* const S`, `dyn m :: Tr`, a tuple) has
/// no name Verus gives it, so its whole text is kept, which no function
/// of the statistics has; so is a qualified path's (`< u8 as Tr > ::
/// Out`), which is no free function's either.
fn owner_name(owner: &str) -> Option<String> {
    if owner.trim().is_empty() {
        return None;
    }
    let base = owner.split('<').next().unwrap_or("").trim();
    if base.is_empty() {
        // A qualified path, `< u8 as Tr > :: Out`.
        return Some(owner.trim().to_owned());
    }
    let segs: Vec<&str> = base.split("::").map(str::trim).collect();
    let ident = |s: &str| {
        let s = s.strip_prefix("r#").unwrap_or(s);
        !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
    };
    let plain = segs
        .iter()
        .enumerate()
        .all(|(i, s)| ident(s) || (i == 0 && s.is_empty()));
    Some(if plain { segs[segs.len() - 1] } else { base }.to_owned())
}

/// The module path Verus names a file's items under: the crate's name
/// with `-` as `_`, then the file's path under `src/`, less `lib.rs`,
/// `main.rs` and `mod.rs` and the `.rs` extension.
fn module_path(krate: &str, rel: &Path) -> Vec<String> {
    let parts: Vec<String> = rel
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let mut out = vec![krate.replace('-', "_")];
    for (i, p) in parts.iter().enumerate() {
        if i + 1 < parts.len() {
            out.push(p.clone());
        } else if !matches!(p.as_str(), "lib.rs" | "main.rs" | "mod.rs") {
            out.push(p.trim_end_matches(".rs").to_owned());
        }
    }
    out
}

fn path_string(p: &verus_syn::Path) -> String {
    p.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join(" :: ")
}

fn calls_assume(tokens: TokenStream) -> bool {
    let mut prev_ident: Option<String> = None;
    for tt in tokens {
        match tt {
            TokenTree::Ident(i) => prev_ident = Some(i.to_string()),
            TokenTree::Group(g) => {
                if g.delimiter() == proc_macro2::Delimiter::Parenthesis
                    && matches!(
                        prev_ident.as_deref(),
                        Some("assume") | Some("admit") | Some("assume_new")
                    )
                {
                    return true;
                }
                if calls_assume(g.stream()) {
                    return true;
                }
                prev_ident = None;
            }
            _ => prev_ident = None,
        }
    }
    false
}

fn is_ghost(sig: &Signature) -> bool {
    !matches!(sig.mode, FnMode::Exec(_) | FnMode::Default)
}

/// A proof function that is taken on trust: an `axiom fn`, or a `proof fn`
/// that is `external_body` or calls `assume`, `admit` or
/// `Tracked::assume_new`. Its `ensures` is
/// assumed wherever it is used, so it is a seam, the strongest kind.
fn is_trusted_proof(sig: &Signature, attrs: &[Attribute], body: Option<&TokenStream>) -> bool {
    match sig.mode {
        FnMode::ProofAxiom(_) => true,
        FnMode::Proof(_) => is_external(attrs) || body.is_some_and(|b| calls_assume(b.clone())),
        _ => false,
    }
}

/// How the tally classes a function, `in_verus` when it is under `verus!`:
/// `None` for a ghost one (spec or proof) that is not a trusted proof, which
/// is counted apart. Shipped code and code built only for Verus are classed
/// by this one rule.
fn classify(
    in_verus: bool,
    sig: &Signature,
    attrs: &[Attribute],
    body: Option<&TokenStream>,
) -> Option<Class> {
    if !in_verus {
        return Some(Class::Outside);
    }
    let trusted_proof = is_trusted_proof(sig, attrs, body);
    if is_ghost(sig) && !trusted_proof {
        return None;
    }
    Some(
        if trusted_proof || is_external(attrs) || body.is_none_or(|b| calls_assume(b.clone())) {
            if is_discharged(attrs) {
                Class::Discharged
            } else {
                Class::Trusted
            }
        } else {
            Class::Verified
        },
    )
}

impl Counter {
    /// The `cfg` predicates an item with `attrs` here is under: those
    /// around it ([`Counter::guards`]) and its own.
    fn guards_with(&self, attrs: &[Attribute]) -> Vec<String> {
        let mut g = self.guards.clone();
        g.extend(cfg_predicates(attrs));
        g
    }

    /// Where an item with `attrs` here is built ([`guarded_built`] of
    /// [`Counter::guards_with`]); a `cfg` it cannot read is an error, and
    /// the item is taken as shipped.
    fn built(&mut self, attrs: &[Attribute]) -> Built {
        match guarded_built(&self.guards_with(attrs), &self.atoms) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{}: {e}", self.file.display());
                self.errors += 1;
                Built::Shipped
            }
        }
    }

    /// Enters code (a module, `impl`, trait or `verus!` block) with `attrs`
    /// whose `cfg` classes it `built`, other than test-only: returns what
    /// [`Counter::leave`] restores.
    fn enter(&mut self, built: Built, attrs: &[Attribute]) -> (bool, bool, bool, usize) {
        let was = (
            self.shipped,
            self.sure,
            self.no_decreases,
            self.guards.len(),
        );
        self.shipped &= matches!(built, Built::Shipped | Built::Unsure);
        self.sure &= built != Built::Unsure;
        self.no_decreases |= allows_no_decreases(attrs);
        self.guards.extend(cfg_predicates(attrs));
        was
    }

    /// Leaves the code [`Counter::enter`] entered.
    fn leave(&mut self, (shipped, sure, no_decreases, guards): (bool, bool, bool, usize)) {
        (self.shipped, self.sure, self.no_decreases) = (shipped, sure, no_decreases);
        self.guards.truncate(guards);
    }

    /// Whether a shipped build compiles an item with `attrs` here.
    fn item_ships(&mut self, attrs: &[Attribute]) -> bool {
        self.shipped && matches!(self.built(attrs), Built::Shipped | Built::Unsure)
    }

    fn record(
        &mut self,
        in_verus: bool,
        attrs: &[Attribute],
        sig: &Signature,
        body: Option<TokenStream>,
        start: usize,
        close: proc_macro2::Span,
    ) -> Option<Class> {
        let end = close.end().line;
        let built = self.built(attrs);
        if built == Built::Test {
            return None;
        }
        // Verus's attribute form verifies a function outside `verus!`, if
        // Verus's build applies it.
        let (in_verus, applied) = if in_verus {
            (true, true)
        } else {
            match verus_attribute(attrs, &self.guards_with(attrs), &self.atoms) {
                Ok(Some(sure)) => (true, sure),
                Ok(None) => (false, true),
                Err(e) => {
                    eprintln!("{}:{start}: {e}", self.file.display());
                    self.errors += 1;
                    (false, true)
                }
            }
        };
        // Whether Verus's build is sure to compile this function, and so
        // to check it: no `cfg` on it or around it that Verus's build might
        // not meet, and the attribute form sure to apply.
        let sure = self.sure && built != Built::Unsure && applied;
        // Verus checks code built only for it, so its rlimits count for
        // the headroom check, though the code is not shipped.
        if in_verus {
            let mut name = self.module.clone();
            name.extend(owner_name(&self.owner));
            name.push(sig.ident.to_string());
            let name = name.join("::");
            let at = format!("{}:{start}", self.file.display());
            self.verus_fns
                .entry(name.clone())
                .or_insert_with(|| at.clone());
            if self.owner_impl {
                self.methods.push(Method {
                    name: name.clone(),
                    at: at.clone(),
                    module: self.module.clone(),
                    owner: self.owner.clone(),
                    params: self.owner_params.clone(),
                    ident: sig.ident.to_string(),
                });
            }
            match rlimit_attr(attrs) {
                Ok(None) => {}
                Ok(Some(units)) => self.limits.push(rlimit::Limit { name, at, units }),
                Err(e) => {
                    eprintln!("{}:{start}: {e}", self.file.display());
                    self.errors += 1;
                }
            }
        }
        let class = classify(in_verus, sig, attrs, body.as_ref());
        // An exec function Verus lets go without a `decreases` (on it, or
        // on the module, `impl` or trait around it) is trusted: nothing
        // proves its loops and recursion end, Verus's check or a Kani
        // record ([`Class::Discharged`]) alike.
        let no_decreases = self.no_decreases || allows_no_decreases(attrs);
        let class = match class {
            Some(Class::Verified | Class::Discharged) if no_decreases && !is_ghost(sig) => {
                Some(Class::Trusted)
            }
            c => c,
        };
        let (at, lines) = match &self.expansion {
            Some(line) => (*line, expansion_lines(sig, body.as_ref(), close)),
            None => (start, end.saturating_sub(start) + 1),
        };
        if in_verus && assumes_attributes(attrs) {
            match &mut self.dry {
                // A candidate definition's walk: its seam is a row of what
                // it writes, so two candidates that differ only in the
                // attribute count differently and fail the run, rather
                // than leave one build's seams out of the report.
                Some(rows) => rows.push((
                    Class::Trusted,
                    format!("#[verifier::assume_unsupported_attributes] {}", sig.ident),
                    0,
                )),
                None => self.assumed.push(format!("{}:{at}", self.file.display())),
            }
        }
        if built == Built::Verus || !self.shipped {
            // A shipped build compiles none of this code, so its verified,
            // ghost and outside functions leave the tally. A trusted one
            // still counts: a shipped function's proof can rest on it, so
            // it is a seam.
            // With `verus_only_ghost`, its ghost functions join the ghost
            // column, which Verus checks all the same.
            match class {
                Some(class @ (Class::Trusted | Class::Discharged)) => {
                    self.tally(class, sig.ident.to_string(), at, lines);
                }
                None if self.verus_only_ghost => {
                    self.tallies.entry(self.krate.clone()).or_default().ghost += 1;
                }
                _ => {}
            }
            return None;
        }
        let Some(class) = class else {
            self.tallies.entry(self.krate.clone()).or_default().ghost += 1;
            return None;
        };
        // A shipped function Verus's build might not compile ships with
        // nothing to prove it checked: it is trusted.
        let class = if class == Class::Verified && !sure {
            Class::Trusted
        } else {
            class
        };
        // A macro's expansion is counted, not walked: the walks read the
        // configured macros their own way (`[reach]`'s `exports_macro`
        // and `fn_ptr_macro`).
        if self.expansion.is_some() {
            self.tally(class, sig.ident.to_string(), at, lines);
            return None;
        }
        let mut calls = Vec::new();
        let mut names = inputs::Body::default();
        let mut loops = LoopShape::default();
        if let Some(b) = &body {
            direct_calls(b.clone(), &mut calls);
            loops = loop_shape(b.clone());
            names = inputs::body(b.clone());
            inputs::statics_in(b.clone(), false, &mut self.statics);
            inputs::aliases_in(b.clone(), &mut self.aliases);
        }
        let entry = self.entry(attrs, sig, start);
        self.fns.push(FnRef {
            krate: self.krate.clone(),
            file: self.file.clone(),
            line: start,
            owner: self.owner.clone(),
            name: sig.ident.to_string(),
            class,
            calls,
            loops,
            body: names,
            params: arity(sig).0,
            receiver: arity(sig).1,
            entry,
        });
        self.tally(class, sig.ident.to_string(), at, lines);
        Some(class)
    }

    /// Counts function `name`, of class `class`, `lines` long and listed at
    /// line `start`, in the tally, lists it under `--list`, and keeps a
    /// discharged one for the record check ([`unrecorded`]).
    fn tally(&mut self, class: Class, name: String, start: usize, lines: usize) {
        if let Some(rows) = &mut self.dry {
            rows.push((class, name, lines));
            return;
        }
        let t = self.tallies.entry(self.krate.clone()).or_default();
        t.fns[class as usize] += 1;
        t.lines[class as usize] += lines;
        if self.list {
            println!(
                "{}\t{:?}\t{}\t{}:{}\t{}",
                self.krate,
                class,
                lines,
                self.file.display(),
                start,
                name
            );
        }
        if class == Class::Discharged {
            self.discharged.push((self.file.clone(), name));
        } else if class == Class::Trusted {
            self.trusted.push((
                self.krate.clone(),
                format!("{}:{start}", self.file.display()),
                lines,
            ));
        }
    }

    /// Keeps each function a `const` or `static` initializer `expr` names
    /// as a value: a whole argument that is a function's name, as the
    /// walk reads one ([`inputs::Arity::ByName`]), such as `default_malloc`
    /// in `MallocPtr(default_malloc)`. A call, even one of unknown arity
    /// (`helper(make::<u8, u16>())`), does not hand out its callee.
    fn addressed_in(&mut self, expr: Option<&verus_syn::Expr>) {
        let Some(e) = expr else { return };
        let b = inputs::body(verus_syn::__private::ToTokens::to_token_stream(e));
        for s in b.sites {
            if let inputs::Site::Free(n, inputs::Arity::ByName)
            | inputs::Site::Path(_, n, inputs::Arity::ByName) = s
            {
                self.addressed.push((self.krate.clone(), n));
            }
        }
    }

    /// What can be done only once every file is read: the `fn_ptr!`
    /// expansions ([`Counter::expand_fn_ptrs`]), and marking each free
    /// function a `const` or `static` hands out ([`Counter::addressed`])
    /// as an entry point.
    fn finish(&mut self) {
        self.expand_fn_ptrs();
        let addressed: std::collections::BTreeSet<(String, String)> =
            std::mem::take(&mut self.addressed).into_iter().collect();
        for f in &mut self.fns {
            if f.owner.is_empty() && addressed.contains(&(f.krate.clone(), f.name.clone())) {
                f.entry = true;
            }
        }
    }

    /// Keeps a `width::fn_ptr!` invocation for [`Counter::expand_fn_ptrs`].
    fn fn_ptr(&mut self, m: &verus_syn::ItemMacro) {
        let line = m.mac.bang_token.span.start().line;
        self.fn_ptrs.uses.push((
            self.krate.clone(),
            self.file.clone(),
            line,
            m.mac.tokens.clone(),
        ));
    }

    /// Keeps `macro_rules! fn_ptr`'s transcriber, and counts its rules.
    fn fn_ptr_rule(&mut self, tokens: &TokenStream) {
        let toks: Vec<TokenTree> = tokens.clone().into_iter().collect();
        for w in toks.windows(2) {
            if let [TokenTree::Punct(p), TokenTree::Group(g)] = w
                && p.as_char() == '>'
                && g.delimiter() == proc_macro2::Delimiter::Brace
            {
                self.fn_ptrs.rules += 1;
                self.fn_ptrs.rule = Some(g.stream());
            }
        }
    }

    /// The functions each `width::fn_ptr!` invocation writes: its
    /// definition's transcriber, with the invocation's struct bound to its
    /// metavariables, parsed. For pcre2-capi that is the struct's `Clone`
    /// and the `FromC` conversion an export applies to a C function
    /// pointer argument, and whatever those bodies call is walked from
    /// them. They are outside `verus!`, and are recorded for the walks
    /// alone, as nested functions are: no tally counts them. Fails closed:
    /// an invocation with no definition, a definition of more than one
    /// rule, a metavariable the expansion does not bind, or an expansion
    /// that does not parse is an error.
    fn expand_fn_ptrs(&mut self) {
        use verus_syn::__private::ToTokens;
        let uses = std::mem::take(&mut self.fn_ptrs.uses);
        for (krate, file, line, tokens) in uses {
            let at = format!("{}:{line}", file.display());
            let rule = match (&self.fn_ptrs.rule, self.fn_ptrs.rules) {
                (Some(r), 1) => r.clone(),
                (_, n) => {
                    eprintln!("{at}: fn_ptr! has {n} macro_rules rules, not the one it expands");
                    self.errors += 1;
                    continue;
                }
            };
            let expanded = parse2::<verus_syn::ItemStruct>(tokens)
                .map_err(|e| format!("does not declare a struct: {e}"))
                .and_then(|s| {
                    let field = s.fields.iter().next().ok_or("its struct has no field")?;
                    let p = s
                        .generics
                        .type_params()
                        .next()
                        .map(|t| t.ident.to_token_stream());
                    let binds: BTreeMap<&str, Option<TokenStream>> = [
                        ("attr", None),
                        ("vis", Some(s.vis.to_token_stream())),
                        ("name", Some(s.ident.to_token_stream())),
                        ("p", p),
                        ("fn", Some(field.ty.to_token_stream())),
                    ]
                    .into();
                    let src = transcribe(rule, &binds)?;
                    parse2::<verus_syn::File>(src).map_err(|e| format!("does not parse: {e}"))
                });
            let expanded = match expanded {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("{at}: fn_ptr!'s expansion {e}");
                    self.errors += 1;
                    continue;
                }
            };
            for item in &expanded.items {
                let Item::Impl(i) = item else { continue };
                let owner = i.self_ty.to_token_stream().to_string();
                for ii in &i.items {
                    let ImplItem::Fn(g) = ii else { continue };
                    let body: TokenStream = g
                        .block
                        .stmts
                        .iter()
                        .map(ToTokens::to_token_stream)
                        .collect();
                    let mut calls = Vec::new();
                    let names = inputs::body(body.clone());
                    let loops = loop_shape(body.clone());
                    direct_calls(body, &mut calls);
                    self.fns.push(FnRef {
                        krate: krate.clone(),
                        file: file.clone(),
                        line,
                        owner: owner.clone(),
                        name: g.sig.ident.to_string(),
                        class: Class::Outside,
                        calls,
                        loops,
                        body: names,
                        params: arity(&g.sig).0,
                        receiver: arity(&g.sig).1,
                        entry: false,
                    });
                }
            }
        }
    }

    /// Whether a function with `attrs` and `sig`, at `line`, is a C entry
    /// point ([`exports_symbol`]). An exported symbol whose ABI C cannot
    /// call ([`is_c_abi`]) is an error, and so is a `cfg_attr` predicate it
    /// cannot read, which leaves the function an entry point.
    fn entry(&mut self, attrs: &[Attribute], sig: &Signature, line: usize) -> bool {
        let entry = exports_symbol(attrs, &self.atoms).unwrap_or_else(|e| {
            eprintln!("{}:{line}: `{}`: {e}", self.file.display(), sig.ident);
            self.errors += 1;
            true
        });
        if entry && !is_c_abi(sig) {
            eprintln!(
                "{}:{line}: `{}` exports its symbol but its ABI is not C's (extern \"C\", \"C-unwind\" or bare extern)",
                self.file.display(),
                sig.ident
            );
            self.errors += 1;
        }
        entry
    }

    /// The functions declared in `stmts`, the body of a function of class
    /// `class`, at any depth of its blocks and expressions: free
    /// functions, and the methods of an `impl` written there (such as a
    /// guard's `Drop`), each of that class. They are recorded
    /// for the recursion check ([`recursive`]) and the walks ([`inputs`],
    /// [`reach`]); no tally counts them, as none counts a nested function.
    fn nested(&mut self, stmts: &[verus_syn::Stmt], class: Class) {
        use verus_syn::__private::ToTokens;
        use verus_syn::visit::Visit;
        // The items written anywhere in the body's expressions (an `if`'s
        // block, a closure's, a `match` arm's), not only among its own
        // statements; each one's own body is read in turn below.
        // An item is kept, and an inline module's items, and those in a
        // `const` or `static`'s initializer (`const _: () = { .. };`), are
        // read too; a function's own body is read where it is recorded.
        // Each is kept with the `cfg`s of the items around it in the body
        // and its own.
        struct BodyItems<'a> {
            found: Vec<(&'a Item, Vec<String>)>,
            guards: Vec<String>,
        }
        impl<'a> Visit<'a> for BodyItems<'a> {
            fn visit_item(&mut self, i: &'a Item) {
                let attrs: &[Attribute] = match i {
                    Item::Fn(f) => &f.attrs,
                    Item::Mod(m) => &m.attrs,
                    Item::Impl(m) => &m.attrs,
                    Item::Trait(t) => &t.attrs,
                    Item::Const(c) => &c.attrs,
                    Item::Static(s) => &s.attrs,
                    _ => &[],
                };
                let was = self.guards.len();
                self.guards.extend(cfg_predicates(attrs));
                self.found.push((i, self.guards.clone()));
                match i {
                    Item::Mod(m) => {
                        for inner in m.content.iter().flat_map(|(_, items)| items) {
                            self.visit_item(inner);
                        }
                    }
                    Item::Const(c) => {
                        if let Some(e) = &c.expr {
                            self.visit_expr(e);
                        }
                    }
                    Item::Static(s) => {
                        if let Some(e) = &s.expr {
                            self.visit_expr(e);
                        }
                    }
                    _ => {}
                }
                self.guards.truncate(was);
            }
        }
        let mut found = BodyItems {
            found: Vec::new(),
            guards: Vec::new(),
        };
        for s in stmts {
            found.visit_stmt(s);
        }
        for (item, guards) in found.found {
            let (owner, fns): (String, Vec<NestedFn>) = match item {
                Item::Fn(g) => (String::new(), vec![(&g.attrs[..], &g.sig, &*g.block)]),
                Item::Impl(i) => (
                    i.self_ty.to_token_stream().to_string(),
                    i.items
                        .iter()
                        .filter_map(|ii| match ii {
                            ImplItem::Fn(f) => Some((&f.attrs[..], &f.sig, &f.block)),
                            _ => None,
                        })
                        .collect(),
                ),
                // A trait's default methods.
                Item::Trait(t) => (
                    t.ident.to_string(),
                    t.items
                        .iter()
                        .filter_map(|ti| match ti {
                            TraitItem::Fn(f) => {
                                f.default.as_ref().map(|b| (&f.attrs[..], &f.sig, b))
                            }
                            _ => None,
                        })
                        .collect(),
                ),
                _ => continue,
            };
            for (attrs, sig, block) in fns {
                // One built only for tests (by its own `cfg`, or one of an
                // item around it) ships nothing, and is not recorded; the
                // `cfg`s go on to the functions in its body.
                let was = self.guards.len();
                self.guards.extend(guards.iter().cloned());
                if self.built(attrs) == Built::Test {
                    self.guards.truncate(was);
                    continue;
                }
                self.guards.extend(cfg_predicates(attrs));
                let body: TokenStream = block.stmts.iter().map(ToTokens::to_token_stream).collect();
                let mut calls = Vec::new();
                let names = inputs::body(body.clone());
                let loops = loop_shape(body.clone());
                direct_calls(body, &mut calls);
                let line = sig.fn_token.span.start().line;
                let entry = self.entry(attrs, sig, line);
                self.fns.push(FnRef {
                    krate: self.krate.clone(),
                    file: self.file.clone(),
                    line,
                    owner: owner.clone(),
                    name: sig.ident.to_string(),
                    class,
                    calls,
                    loops,
                    body: names,
                    params: arity(sig).0,
                    receiver: arity(sig).1,
                    entry,
                });
                self.nested(&block.stmts, class);
                self.guards.truncate(was);
            }
        }
    }

    fn items(&mut self, items: &[Item], in_verus: bool) {
        for item in items {
            // The bindings an expansion writes, which the rules read once
            // the crate is walked ([`Counter::walk_crate`]).
            if let Some(at) = self.expansion {
                let base = Binding {
                    module: self.module.clone(),
                    shipped: true,
                    verus: true,
                    ..Binding::default()
                };
                let mut found = Vec::new();
                match item {
                    Item::Macro(m) if m.mac.path.is_ident("macro_rules") => {
                        if let Some(i) = &m.ident {
                            found.push(Binding {
                                name: i.to_string(),
                                ..base.clone()
                            });
                        }
                    }
                    Item::ExternCrate(e) if has_applied(&e.attrs, "macro_use") => {
                        found.push(Binding {
                            name: e.ident.to_string(),
                            macro_use: true,
                            ..base.clone()
                        });
                    }
                    _ => {}
                }
                let file = self.file.clone();
                let bind_at = BindAt {
                    file: &file,
                    module: self.module.clone(),
                    guards: Vec::new(),
                    shared: 0,
                    scope: self.scope.clone(),
                    expansion: Some(at),
                };
                bindings_in(
                    std::slice::from_ref(item),
                    &bind_at,
                    &self.atoms,
                    &mut found,
                );
                // Every build may compile what an expansion writes: its
                // `cfg`s are not read.
                for b in &mut found {
                    b.expanded = true;
                    b.shipped = true;
                    b.verus = true;
                    b.verus_sure = false;
                    b.place = format!("{}:{at}", file.display());
                }
                self.emitted.extend(found);
            }
            // A type an expansion writes is kept too: it shadows a glob
            // import of the same name, as one the file writes does.
            let defined = match item {
                Item::Struct(s) => Some((&s.attrs, &s.vis, &s.ident)),
                Item::Enum(e) => Some((&e.attrs, &e.vis, &e.ident)),
                Item::Union(u) => Some((&u.attrs, &u.vis, &u.ident)),
                Item::Type(t) => Some((&t.attrs, &t.vis, &t.ident)),
                _ => None,
            };
            if let Some((attrs, vis, ident)) = defined {
                let built = guarded_built(&self.guards_with(attrs), &self.atoms).ok();
                if built != Some(Built::Test) {
                    let scope = self.vis_scope(vis);
                    self.type_paths.insert(
                        [self.module.clone(), vec![ident.to_string()]].concat(),
                        (
                            scope,
                            !self.sure || !matches!(built, Some(Built::Shipped | Built::Verus)),
                        ),
                    );
                }
            }
            // A `use` an expansion writes binds names [`Counter::uses`]
            // does not hold (it is read from the files' own items), which
            // shadow the module's globs ([`Counter::opaque`]).
            if self.expansion.is_some()
                && let Item::Use(u) = item
                && guarded_built(&self.guards_with(&u.attrs), &self.atoms).ok() != Some(Built::Test)
            {
                self.opaque.insert(self.module.clone());
            }
            // An expansion is walked for its functions alone
            // ([`Counter::record`]).
            if self.expansion.is_some()
                && !matches!(
                    item,
                    Item::Fn(_) | Item::Impl(_) | Item::Trait(_) | Item::Mod(_) | Item::Macro(_)
                )
            {
                continue;
            }
            match item {
                Item::Fn(f) => {
                    let start = f.sig.fn_token.span.start().line;
                    let end = f.block.brace_token.span.close();
                    let body = Some(
                        f.block
                            .stmts
                            .iter()
                            .map(verus_syn::__private::ToTokens::to_token_stream)
                            .collect(),
                    );
                    if let Some(c) = self.record(in_verus, &f.attrs, &f.sig, body, start, end) {
                        self.nested(&f.block.stmts, c);
                    }
                }
                Item::Impl(i) => {
                    let built = self.built(&i.attrs);
                    if built == Built::Test {
                        continue;
                    }
                    let was = self.enter(built, &i.attrs);
                    let outer = std::mem::replace(
                        &mut self.owner,
                        verus_syn::__private::ToTokens::to_token_stream(&*i.self_ty).to_string(),
                    );
                    let outer_impl = std::mem::replace(&mut self.owner_impl, true);
                    let outer_params = std::mem::replace(
                        &mut self.owner_params,
                        i.generics
                            .params
                            .iter()
                            .filter_map(|p| match p {
                                verus_syn::GenericParam::Type(t) => Some(t.ident.to_string()),
                                _ => None,
                            })
                            .collect(),
                    );
                    for ii in &i.items {
                        if let ImplItem::Fn(f) = ii {
                            let start = f.sig.fn_token.span.start().line;
                            let end = f.block.brace_token.span.close();
                            let body = Some(
                                f.block
                                    .stmts
                                    .iter()
                                    .map(verus_syn::__private::ToTokens::to_token_stream)
                                    .collect(),
                            );
                            let mut attrs = f.attrs.clone();
                            attrs.extend(
                                i.attrs
                                    .iter()
                                    .filter(|a| is_external(std::slice::from_ref(a)))
                                    .cloned(),
                            );
                            if let Some(c) = self.record(in_verus, &attrs, &f.sig, body, start, end)
                            {
                                self.nested(&f.block.stmts, c);
                            }
                        } else if let ImplItem::Macro(im) = ii {
                            let mut wrap: TokenStream = i
                                .attrs
                                .iter()
                                .filter(|a| is_external(std::slice::from_ref(a)))
                                .map(verus_syn::__private::ToTokens::to_token_stream)
                                .collect();
                            wrap.extend(verus_syn::__private::ToTokens::to_token_stream(
                                &i.impl_token,
                            ));
                            wrap.extend(verus_syn::__private::ToTokens::to_token_stream(
                                &*i.self_ty,
                            ));
                            self.item_macro(&im.attrs, &im.mac, Some(wrap));
                        }
                    }
                    self.owner = outer;
                    self.owner_impl = outer_impl;
                    self.owner_params = outer_params;
                    self.leave(was);
                }
                Item::Trait(t) => {
                    let built = self.built(&t.attrs);
                    if built == Built::Test {
                        continue;
                    }
                    let was = self.enter(built, &t.attrs);
                    if self.shipped {
                        self.traits.insert(t.ident.to_string());
                    }
                    let outer = std::mem::replace(&mut self.owner, t.ident.to_string());
                    let outer_impl = std::mem::replace(&mut self.owner_impl, false);
                    for ti in &t.items {
                        if let TraitItem::Fn(f) = ti
                            && let Some(block) = &f.default
                        {
                            let start = f.sig.fn_token.span.start().line;
                            let end = block.brace_token.span.close();
                            let body = Some(
                                block
                                    .stmts
                                    .iter()
                                    .map(verus_syn::__private::ToTokens::to_token_stream)
                                    .collect(),
                            );
                            if let Some(c) =
                                self.record(in_verus, &f.attrs, &f.sig, body, start, end)
                            {
                                self.nested(&block.stmts, c);
                            }
                        } else if let TraitItem::Macro(tm) = ti {
                            let mut wrap =
                                verus_syn::__private::ToTokens::to_token_stream(&t.trait_token);
                            wrap.extend(verus_syn::__private::ToTokens::to_token_stream(&t.ident));
                            self.item_macro(&tm.attrs, &tm.mac, Some(wrap));
                        }
                    }
                    self.owner = outer;
                    self.owner_impl = outer_impl;
                    self.leave(was);
                }
                Item::Mod(m) => {
                    let built = self.built(&m.attrs);
                    if built != Built::Test
                        && let Some((_, inner)) = &m.content
                    {
                        let was = self.enter(built, &m.attrs);
                        self.module.push(m.ident.to_string());
                        self.inline_macro_use.push(has_attr(&m.attrs, "macro_use"));
                        let instance =
                            mod_instance(&self.file, self.expansion, m.ident.span().start());
                        let outer = std::mem::replace(&mut self.scope, instance);
                        self.items(inner, in_verus);
                        self.scope = outer;
                        self.inline_macro_use.pop();
                        self.module.pop();
                        self.leave(was);
                    } else if built != Built::Test
                        && m.content.is_none()
                        && (self.no_decreases || allows_no_decreases(&m.attrs))
                    {
                        // The attribute covers the module, on it or on a
                        // module or file around it, but the module's file,
                        // and every file under it, is walked on its own,
                        // so it would not reach the functions it covers.
                        eprintln!(
                            "{}:{}: #[verifier::exec_allows_no_decreases_clause] covers `mod {};`, a module the counter reads file by file: put it on the functions, impls, traits or inline modules it covers",
                            self.file.display(),
                            m.mod_token.span.start().line,
                            m.ident
                        );
                        self.errors += 1;
                    }
                }
                Item::Static(s) if self.item_ships(&s.attrs) => {
                    self.statics.push(inputs::Static {
                        name: s.ident.to_string(),
                        forced: !matches!(s.mutability, verus_syn::StaticMutability::None),
                        ty: verus_syn::__private::ToTokens::to_token_stream(&*s.ty),
                    });
                    self.addressed_in(s.expr.as_deref());
                }
                Item::Const(c) if self.item_ships(&c.attrs) => {
                    self.addressed_in(c.expr.as_deref());
                }
                Item::Struct(s) if self.item_ships(&s.attrs) => {
                    let fields = s
                        .fields
                        .iter()
                        .map(|f| verus_syn::__private::ToTokens::to_token_stream(&f.ty));
                    inputs::add_type(&mut self.types, s.ident.to_string(), fields);
                }
                Item::Enum(e) if self.item_ships(&e.attrs) => {
                    let fields = e
                        .variants
                        .iter()
                        .flat_map(|v| v.fields.iter())
                        .map(|f| verus_syn::__private::ToTokens::to_token_stream(&f.ty));
                    inputs::add_type(&mut self.types, e.ident.to_string(), fields);
                }
                Item::Type(t) if self.item_ships(&t.attrs) => {
                    inputs::aliases_in(
                        verus_syn::__private::ToTokens::to_token_stream(t),
                        &mut self.aliases,
                    );
                    inputs::add_type(
                        &mut self.types,
                        t.ident.to_string(),
                        [verus_syn::__private::ToTokens::to_token_stream(&*t.ty)],
                    );
                }
                Item::Use(u) if self.item_ships(&u.attrs) => {
                    inputs::aliases_in(
                        verus_syn::__private::ToTokens::to_token_stream(u),
                        &mut self.aliases,
                    );
                }
                Item::Macro(m)
                    if m.mac.path.is_ident("thread_local")
                        && self.expansion.is_none()
                        && self.item_ships(&m.attrs) =>
                {
                    inputs::statics_in(m.mac.tokens.clone(), true, &mut self.statics);
                    // One not shown to be std's is read as any other macro
                    // of its name ([`Counter::item_macro`]).
                    if !self.std_thread_local(&m.mac.path) {
                        self.item_macro(&m.attrs, &m.mac, None);
                    }
                }
                Item::Macro(m)
                    if self.fn_ptr_macro.as_ref().is_some_and(|f| {
                        m.mac.path.segments.last().is_some_and(|s| s.ident == f)
                    }) && self.expansion.is_none()
                        && self.item_ships(&m.attrs) =>
                {
                    self.fn_ptr(m);
                    self.item_macro(&m.attrs, &m.mac, None);
                }
                Item::Macro(m)
                    if m.mac.path.is_ident("macro_rules")
                        && self.expansion.is_none()
                        && self
                            .fn_ptr_macro
                            .as_ref()
                            .is_some_and(|f| m.ident.as_ref().is_some_and(|i| i == f))
                        && self.item_ships(&m.attrs) =>
                {
                    self.fn_ptr_rule(&m.mac.tokens);
                }
                Item::Macro(m) if verus_path(&m.mac.path) => {
                    // A `verus!` block under a `cfg` is built where it says
                    // (`#[cfg(feature = "verus")] verus! { .. }`).
                    let built = self.built(&m.attrs);
                    if built == Built::Test {
                        continue;
                    }
                    // One that may write a function is Verus's only by the
                    // rule of its form ([`Took`]): bare, or a path with a
                    // leading `::`; any other path fails the run.
                    if holds_fn(m.mac.tokens.clone()) {
                        let place = format!(
                            "{}:{}",
                            self.file.display(),
                            m.mac.bang_token.span.start().line
                        );
                        let segs = segments(&m.mac.path);
                        let colon = m.mac.path.leading_colon.is_some();
                        let written =
                            format!("{}{}!", if colon { "::" } else { "" }, segs.join("::"));
                        let (kind, name) = match (colon, segs.len()) {
                            (false, 1) => (Took::VerusBare, "verus".to_owned()),
                            (true, _) => (Took::VerusPath, segs[0].clone()),
                            (false, _) => {
                                eprintln!(
                                    "{place}: {written} may not be Verus's `verus!`: without a leading `::` its first segment may name an item of this crate; write `::{}!` or a bare `verus!`",
                                    segs.join("::")
                                );
                                self.errors += 1;
                                continue;
                            }
                        };
                        if let Some(s) = self.doubt(&self.bindings, kind, &name, &self.module) {
                            eprintln!(
                                "{place}: {written} may not be Verus's `verus!`: {s}; remove or rename it"
                            );
                            self.errors += 1;
                            continue;
                        }
                        self.took.push((kind, place, self.module.clone(), name));
                    }
                    let was = self.enter(built, &m.attrs);
                    match parse2::<verus_syn::File>(m.mac.tokens.clone()) {
                        Ok(f) => self.items(&f.items, true),
                        Err(e) => {
                            eprintln!("{}: verus! block: {e}", self.file.display());
                            self.errors += 1;
                        }
                    }
                    self.leave(was);
                }
                // A `macro_rules!` an expansion writes defines its macro from
                // here on, as rustc's would: kept as written at the outermost
                // invocation's line, in the module the expansion is in.
                Item::Macro(m)
                    if m.mac.path.is_ident("macro_rules")
                        && self.expansion.is_some()
                        && m.ident.is_some() =>
                {
                    let at = self.expansion.unwrap_or_default();
                    let name = m
                        .ident
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    let file = self.file.clone();
                    // Its scope ends with the innermost module around it
                    // without `#[macro_use]`; where every one up to the
                    // file's top has it, it is carried out as one written
                    // at the top is.
                    let through = self
                        .inline_macro_use
                        .iter()
                        .rev()
                        .take_while(|u| **u)
                        .count();
                    let (scope, carried_at, split) = if through == self.inline_macro_use.len()
                        && self.module.starts_with(&self.carry.0)
                    {
                        (self.carry.1.clone(), self.carry.2.clone(), self.carry.3)
                    } else {
                        let keep = self.module.len().saturating_sub(through);
                        (self.module[..keep].to_vec(), Vec::new(), false)
                    };
                    let mut def = macro_def(
                        m,
                        &DefsAt {
                            file: &file,
                            module: self.module.clone(),
                            built: match (self.shipped, self.sure) {
                                (false, _) => Built::Verus,
                                (true, false) => Built::Unsure,
                                (true, true) => Built::Shipped,
                            },
                            guards: self.guards.clone(),
                            scope,
                            carried_at,
                            split,
                        },
                        &self.atoms,
                        true,
                    );
                    def.line = at;
                    self.macros.entry(name).or_default().push(def);
                }
                Item::Macro(m) if !m.mac.path.is_ident("macro_rules") => {
                    self.item_macro(&m.attrs, &m.mac, None);
                }
                _ => {}
            }
        }
    }

    /// Reads crate [`Counter::krate`], in directory `k` under `root`, as
    /// every run reads a crate, the working tree's or a base revision's
    /// ([`base_limits`]): its module files placed and classed by their
    /// declarations ([`module_files`], [`module_class`]), a test-only one
    /// left out, a Verus-only one read for its rlimits alone, one Verus's
    /// build might not compile counting no function verified. The crate's
    /// `macro_rules!` definitions are kept first, from every file
    /// ([`macro_defs`]), so that an invocation in one file expands a
    /// definition in another; then each file is walked under its modules'
    /// `cfg`s. `tests`, if given, gathers the `#[test]` functions a ledger
    /// may name.
    fn walk_crate(
        &mut self,
        cfg: &Config,
        (root, k): (&Path, &Path),
        mut tests: Option<&mut std::collections::BTreeSet<String>>,
    ) {
        let src = k.join("src");
        let src = src.as_path();
        let mut paths = Vec::new();
        rust_files(src, &mut paths);
        // A same-length stand-in for an identifier Verus keeps as a keyword
        // (the PCRE2 port's `assert::`) keeps the line numbers.
        let read: Vec<(PathBuf, Option<verus_syn::Result<verus_syn::File>>)> = paths
            .into_iter()
            .map(|f| {
                let p = fs::read_to_string(&f)
                    .ok()
                    .map(|t| parse_file(&cfg.stand_in(&t)));
                (f, p)
            })
            .collect();
        let mut ok: Vec<(PathBuf, verus_syn::File)> = Vec::with_capacity(read.len());
        for (f, p) in &read {
            if let Some(Ok(file)) = p {
                if let Some(t) = tests.as_deref_mut() {
                    ledger::test_fns(&ledger::module_path(src, f), &file.items, t);
                }
                ok.push((f.clone(), file.clone()));
            }
        }
        let mut bad = Vec::new();
        let mods = module_files(&ok, crate_roots(k), &self.atoms, &mut bad);
        bad.extend(unread_module_files(
            &read.iter().map(|(f, _)| canonical(f)).collect::<Vec<_>>(),
            &mods,
            &self.atoms,
        ));
        for e in &bad {
            eprintln!("{e}");
        }
        self.errors += bad.len();
        // Each file's class (a `cfg` [`module_files`] reported counts as
        // shipped) and the `cfg`s of the modules around it.
        let classes: Vec<(Built, Vec<String>)> = read
            .iter()
            .map(|(f, _)| {
                let cf = canonical(f);
                (
                    module_class(&cf, &mods, &self.atoms).unwrap_or(Built::Shipped),
                    module_guards(&cf, &mods),
                )
            })
            .collect();
        let class = |i: usize| classes[i].0;
        let files: Vec<PathBuf> = read.iter().map(|(f, _)| f.clone()).collect();
        let parsed: Vec<Option<verus_syn::Result<verus_syn::File>>> =
            read.into_iter().map(|(_, p)| p).collect();
        self.macros.clear();
        self.uses.clear();
        // The modules a `#[macro_use] mod name;` declares, whose textual
        // scope carries on into the module that declares them.
        let mut carried = Carried::new();
        // The inline modules' declarations: the scope of a file `mod` one
        // with `#[macro_use]` declares carries on outward through it.
        let mut inline_carried = InlineMods::new();
        self.modules.clear();
        self.cfg_modules.clear();
        self.macro_use.clear();
        (self.edition, self.renamed_deps) = crate_manifest(root, k);
        // Macros match by the edition of the target compiling them. The
        // counter does not split the files it reads by target, so targets
        // that read `expr` or `pat` differently fail the run.
        let walked = walked_editions(root, k);
        let era = |e: &u32| expand::matcher_era(*e);
        if walked.iter().map(era).min() != walked.iter().map(era).max() {
            eprintln!(
                "{}: the targets whose files are read (the library, binaries, and any test, bench or example under src/) set editions {walked:?}, which read a macro's `expr` or `pat` fragment differently (2024's `expr` takes `const` blocks and `_`, 2021's `pat` a top-level `|`); give them one side of 2021 and of 2024",
                k.join("Cargo.toml").display()
            );
            self.errors += 1;
        }
        expand::set_edition(walked[0]);
        let mut bindings = Vec::new();
        for (i, (f, p)) in files.iter().zip(&parsed).enumerate() {
            let Some(Ok(file)) = p else { continue };
            let module = module_path(&self.krate, f.strip_prefix(src).unwrap_or(f));
            self.modules.insert(module.clone());
            macro_use_mods(
                &file.items,
                &module,
                f.strip_prefix(root).unwrap_or(f),
                (&mut carried, &mut inline_carried, &mut self.modules),
            );
            cfg_gated_mods(&file.items, &module, &mut self.cfg_modules);
            // A module file's own `#![cfg]` may leave it out too.
            if !cfg_predicates(&file.attrs).is_empty() {
                self.cfg_modules.insert(module.clone());
            }
            if class(i) == Built::Test {
                continue;
            }
            // rustc takes a `#[macro_use] extern crate` only at a crate
            // root; every file's is read.
            macro_use_crates(&file.items, &self.atoms, &mut self.macro_use);
            let rel = f.strip_prefix(root).unwrap_or(f);
            let at = BindAt {
                file: rel,
                module,
                guards: classes[i].1.clone(),
                shared: classes[i].1.len(),
                scope: rel.display().to_string(),
                expansion: None,
            };
            bindings_in(&file.items, &at, &self.atoms, &mut bindings);
        }
        self.add_proof_trees(&mods, (root, src), &files);
        // Each file's module, and the scope and chain a definition at its
        // top has: each `#[macro_use] mod name;` carrying it out, and each
        // inline `#[macro_use]` module around such a `mod` (whose own
        // file and line the `mod`'s step already gives). A module declared
        // by more than one `mod` (under `cfg`s) has no one place in its
        // parent: its definitions are not ordered against the others
        // there.
        let carry: Vec<Carry> = files
            .iter()
            .map(|f| {
                let module = module_path(&self.krate, f.strip_prefix(src).unwrap_or(f));
                let mut scope = module.clone();
                let mut carried_at = Vec::new();
                let mut split = false;
                loop {
                    if let Some(decls) = carried.get(&scope)
                        && let Some(c) = decls.iter().flatten().min_by_key(|(_, l)| *l)
                    {
                        carried_at.push(c.clone());
                        split |= decls.len() > 1;
                    } else if !carried_at.is_empty()
                        && inline_carried
                            .get(&scope)
                            .is_some_and(|d| d.contains(&true))
                    {
                        // A file `mod` that two declarations of an inline
                        // module both hold is itself declared twice.
                    } else {
                        break;
                    }
                    scope.pop();
                }
                (module, scope, carried_at, split)
            })
            .collect();
        for (i, (f, p)) in files.iter().zip(&parsed).enumerate() {
            let Some(Ok(file)) = p else { continue };
            let (module, scope, carried_at, split) = carry[i].clone();
            let at = DefsAt {
                file: f.strip_prefix(root).unwrap_or(f),
                scope,
                module,
                built: class(i),
                guards: classes[i].1.clone(),
                carried_at,
                split,
            };
            macro_defs(
                &file.items,
                &at,
                &self.atoms,
                (&mut self.macros, &mut self.uses),
            );
        }
        // A definition's module re-exporting it (`pub(crate) use exports;`,
        // or `pub(crate) use ex as build;` under the new name) gives it that
        // module's path, under each name it is exported as.
        for (name, defs) in &mut self.macros {
            for d in defs.iter_mut() {
                let exports: Vec<String> = self
                    .uses
                    .get(&d.module)
                    .into_iter()
                    .flatten()
                    .filter(|(p, glob, _, built)| {
                        !glob
                            && *built != Built::Test
                            && (p.as_slice() == std::slice::from_ref(name)
                                || p == &["self".to_owned(), name.clone()])
                    })
                    .map(|(_, _, named, _)| named.clone())
                    .collect();
                for named in exports {
                    let mut path = d.module.clone();
                    path.push(named);
                    if !d.paths.contains(&path) {
                        d.paths.push(path);
                    }
                }
            }
        }
        self.follow_reexports();
        // Beside the bindings the files write ([`bindings_in`]): the
        // `macro_rules!` and the `#[macro_use]` crates.
        for (name, defs) in &self.macros {
            for d in defs.iter().filter(|d| d.built != Built::Test) {
                bindings.push(Binding {
                    module: d.module.clone(),
                    name: name.clone(),
                    place: format!("`macro_rules! {name}` at {}:{}", d.file.display(), d.line),
                    shipped: matches!(d.built, Built::Shipped | Built::Unsure),
                    verus: true,
                    ..Binding::default()
                });
            }
        }
        for k in &self.macro_use {
            bindings.push(Binding {
                module: vec![self.krate.replace('-', "_")],
                name: k.clone(),
                macro_use: true,
                place: "the crate root".to_owned(),
                shipped: true,
                verus: true,
                ..Binding::default()
            });
        }
        self.bindings = bindings;
        self.emitted.clear();
        self.took.clear();
        self.invoked.clear();
        for (i, (f, p)) in files.iter().zip(&parsed).enumerate() {
            // A module built only for Verus is read for its rlimits alone,
            // and one Verus's build might not compile counts no function
            // verified. Its items are read under its modules' `cfg`s.
            (self.shipped, self.sure) = match class(i) {
                Built::Test => continue,
                Built::Verus => (false, true),
                Built::Unsure => (true, false),
                Built::Shipped => (true, true),
            };
            self.guards = classes[i].1.clone();
            self.file = f.strip_prefix(root).unwrap_or(f).to_path_buf();
            if self.shipped {
                self.files.push((self.krate.clone(), self.file.clone()));
            }
            self.module = module_path(&self.krate, f.strip_prefix(src).unwrap_or(f));
            self.carry = carry[i].clone();
            self.scope = self.file.display().to_string();
            self.no_decreases = false;
            match p {
                Some(Ok(file)) => {
                    // A file's inner `#![verifier::exec_allows_no_decreases_clause]`
                    // covers its own functions; a `mod x;` under it fails
                    // the run ([`Counter::items`]), since `x`'s file is
                    // walked on its own.
                    self.no_decreases = allows_no_decreases(&file.attrs);
                    self.items(&file.items, false);
                    self.no_decreases = false;
                }
                Some(Err(e)) => {
                    eprintln!("{}:{}: {e}", self.file.display(), e.span().start().line);
                    self.errors += 1;
                }
                None => {
                    eprintln!("{}: unreadable", f.display());
                    self.errors += 1;
                }
            }
        }
        // An invocation counted as another crate's macro whose name this
        // crate defines (a definition reached by a path the counter does
        // not follow, or one an expansion in a file read later writes)
        // fails the run: its functions would otherwise leave the tally.
        for (here, name) in std::mem::take(&mut self.unresolved) {
            // Defined under that name, or exported under it (`pub(crate) use
            // ex as build;`).
            let defined = self.macros.iter().any(|(n, defs)| {
                defs.iter().any(|d| {
                    d.built != Built::Test
                        && (*n == name || d.paths.iter().any(|p| p.last() == Some(&name)))
                })
            });
            if defined {
                eprintln!(
                    "{here}: {name}! resolves to none of this crate's definitions of `{name}`, though rustc may reach one; invoke it by its definition's path, or in that definition's textual scope"
                );
                self.errors += 1;
            }
        }
        // A `macro_rules!` an expansion writes is known only once its own
        // file is walked: an invocation of its name in a file walked
        // before it, in a module in its scope, may expand it, and fails
        // the run. One carried out by a `#[macro_use] mod` is in the
        // declaring file's scope from that `mod` on; in any other file
        // (a module file of its scope, such as one a later `mod`
        // declares) the resolution reaches it as one of the others,
        // wherever the invocation is.
        let rels: Vec<PathBuf> = files
            .iter()
            .map(|f| f.strip_prefix(root).unwrap_or(f).to_path_buf())
            .collect();
        let order = |p: &Path| rels.iter().position(|r| r == p);
        for (here, file, module, at, name) in std::mem::take(&mut self.invoked) {
            let late = self.macros.get(&name).into_iter().flatten().find(|d| {
                d.emitted
                    && d.built != Built::Test
                    && order(&file) < order(&d.file)
                    && module.starts_with(&d.scope)
                    && (d.carried_at.iter().all(|(f, _)| *f != file)
                        || d.carried_at.iter().any(|(f, l)| *f == file && *l <= at))
            });
            if let Some(d) = late {
                eprintln!(
                    "{here}: {name}! may expand the `macro_rules! {name}` that an expansion writes at {}:{}, which may be in scope here but is in a file the counter reads after this one; write that `macro_rules!` outside the macro",
                    d.file.display(),
                    d.line
                );
                self.errors += 1;
            }
        }
        // The walk took each `verus!` and `compile_error!` on the sources'
        // bindings; a binding an expansion writes that breaks its rule
        // fails the run, wherever the expansion is.
        let took = std::mem::take(&mut self.took);
        for (kind, at, module, name) in &took {
            if let Some(s) = self.doubt(&self.emitted, *kind, name, module) {
                eprintln!(
                    "{at}: {s} may make this macro another than the {} one the counter took it for; write that binding outside the macro, or remove it",
                    match kind {
                        Took::StdPath | Took::StdBare => "std",
                        Took::VerusPath | Took::VerusBare => "Verus",
                    }
                );
                self.errors += 1;
            }
        }
        self.name_methods();
    }

    /// Renames each method [`Counter::record`] named by its `impl`'s
    /// module to the name Verus gives it: the path of the item the `impl`'s
    /// type resolves to, then the method's name (rust_verify's
    /// `register_friendly_path_as_rust_name`), which may be in another
    /// module than the `impl`. A method whose type [`Counter::type_path`]
    /// cannot find keeps its first name; a limit on it then names no
    /// function of the statistics, which fails the check
    /// ([`rlimit::shares`]).
    fn name_methods(&mut self) {
        for m in std::mem::take(&mut self.methods) {
            let Some(mut path) = self.type_path(&m.module, &m.owner, &m.params) else {
                continue;
            };
            path.push(m.ident);
            let name = path.join("::");
            if name == m.name {
                continue;
            }
            for l in self
                .limits
                .iter_mut()
                .filter(|l| l.name == m.name && l.at == m.at)
            {
                l.name = name.clone();
            }
            if self.verus_fns.get(&m.name) == Some(&m.at) {
                self.verus_fns.remove(&m.name);
            }
            self.verus_fns.entry(name).or_insert(m.at);
        }
    }

    /// The path (module, then name) of the struct, enum, union or `type`
    /// alias that an `impl` in module `here`, with type parameters
    /// `params`, names as `owner` (its type's tokens), among those the walk
    /// read ([`Counter::type_paths`]), found as rustc's scoping finds it:
    /// a path's last segment in the module the rest names from `here`; a
    /// bare name `here` defines; else the one `use` in `here` that imports
    /// it by name, which settles it even when it names no type the walk
    /// read (another crate's, a re-export); else the one type that
    /// `here`'s glob imports of this crate's modules bring in, defined in
    /// the glob's module and visible from `here` (`pub` of any kind, or
    /// the glob's module an ancestor of `here`). `None` for anything else,
    /// whose methods keep the `impl`'s module, so a limit on one names no
    /// function of the statistics rather than another function's: one of
    /// `params`, a reference, a slice, another crate's type; a name that a
    /// `use` Verus's build might not compile imports (`Built::Unsure`), or
    /// that two do; and a name a glob's module may pass on without
    /// defining it (it imports the name, or has a glob of its own, which
    /// the counter does not follow), or that a glob of this crate the
    /// counter cannot place may bring in.
    fn type_path(&self, here: &[String], owner: &str, params: &[String]) -> Option<Vec<String>> {
        let segs: Vec<String> = owner
            .split('<')
            .next()
            .unwrap_or("")
            .split("::")
            .map(|s| s.trim().to_owned())
            .collect();
        if segs.iter().any(String::is_empty) {
            return None;
        }
        let found = |module: Vec<String>, name: &String| {
            let p = [module, vec![name.clone()]].concat();
            self.type_paths.contains_key(&p).then_some(p)
        };
        let (name, prefix) = segs.split_last()?;
        if !prefix.is_empty() {
            return found(self.path_module(here, prefix, true)?, name);
        }
        if params.contains(name) {
            return None;
        }
        if let Some(p) = found(here.to_vec(), name) {
            return Some(p);
        }
        let uses = self.sure_uses(here);
        let named: Vec<&&UsePath> = uses
            .iter()
            .filter(|(_, glob, n, _)| !glob && n == name)
            .collect();
        match named.as_slice() {
            [] => {}
            [(p, _, _, built)] if *built != Built::Unsure => {
                let (last, from) = p.split_last()?;
                return found(self.path_module(here, from, false)?, last);
            }
            _ => return None,
        }
        // A type an unread macro writes in `here`, or a `use` an expansion
        // writes there, shadows every glob's, as one written out does.
        if self.opaque.contains(here) {
            return None;
        }
        // The candidates are the definitions the placed globs bring in. A
        // glob the counter cannot place (another crate's), or a module's
        // re-export it does not follow, may bring the name in only where
        // no candidate does, or rustc would find the name ambiguous; so
        // one candidate is the name's type, and none leaves it unplaced.
        let mut candidates = Vec::new();
        for (p, _, _, built) in uses.iter().filter(|(_, glob, ..)| *glob) {
            let Some(m) = self.path_module(here, p, false) else {
                continue;
            };
            let path = [m, vec![name.clone()]].concat();
            match self.type_paths.get(&path) {
                None => {}
                // A glob, or a definition, Verus's build might not compile
                // may leave the name to one the counter cannot place.
                Some((_, true)) => return None,
                Some(_) if *built == Built::Unsure => return None,
                // A glob brings in only what `here` can see; a scope the
                // counter cannot place is taken for one it cannot.
                Some((scope, false)) if scope.as_ref().is_some_and(|s| here.starts_with(s)) => {
                    candidates.push(path);
                }
                Some(_) => {}
            }
        }
        match candidates.as_slice() {
            [p] => Some(p.clone()),
            _ => None,
        }
    }

    /// The `use`s of `module` but those built only for tests.
    fn sure_uses(&self, module: &[String]) -> Vec<&UsePath> {
        self.uses
            .get(module)
            .into_iter()
            .flatten()
            .filter(|(_, _, _, built)| *built != Built::Test)
            .collect()
    }

    /// The module of this crate a path's module segments `segs`, written in
    /// module `here`, name, as edition 2018 and later resolve them: from
    /// `crate`, `self` or `super`; else from a module `here` declares, or,
    /// with `follow`, from the one `use` in `here` Verus's build is sure
    /// to compile that imports the first segment by name (`use
    /// crate::x::state;` then `state::S`). A first segment that names
    /// neither may be another crate (an extern crate beside a root module
    /// of its name), so it gives `None`: the crate root's modules are not
    /// in scope below the root.
    fn path_module(&self, here: &[String], segs: &[String], follow: bool) -> Option<Vec<String>> {
        let (first, rest) = segs.split_first()?;
        if matches!(first.as_str(), "crate" | "self" | "super") {
            return self.module_from(here, segs);
        }
        let child = [here, segs].concat();
        if self.modules.contains(&child) {
            return Some(child);
        }
        if !follow {
            return None;
        }
        let uses = self.sure_uses(here);
        let named: Vec<&&UsePath> = uses
            .iter()
            .filter(|(_, glob, n, _)| !glob && n == first)
            .collect();
        let [(p, _, _, built)] = named.as_slice() else {
            return None;
        };
        if *built == Built::Unsure {
            return None;
        }
        let m = [self.path_module(here, p, false)?, rest.to_vec()].concat();
        self.modules.contains(&m).then_some(m)
    }

    /// The module an item with `vis`, in [`Counter::module`], is visible
    /// within, as rustc's privacy reads it: the crate for `pub` and
    /// `pub(crate)`, the item's module for none and `pub(self)`, its
    /// parent for `pub(super)`, and the module a `pub(in path)` names;
    /// `None` when the counter cannot place that module.
    fn vis_scope(&self, vis: &verus_syn::Visibility) -> Option<Vec<String>> {
        match vis {
            verus_syn::Visibility::Public(_) => self.module.get(..1).map(<[String]>::to_vec),
            verus_syn::Visibility::Inherited => Some(self.module.clone()),
            verus_syn::Visibility::Restricted(r) => {
                let segs: Vec<String> = r
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                match segs.as_slice() {
                    [c] if c == "crate" => self.module.get(..1).map(<[String]>::to_vec),
                    [s] if s == "self" => Some(self.module.clone()),
                    [s] if s == "super" => {
                        self.module.split_last().map(|(_, parent)| parent.to_vec())
                    }
                    _ if r.in_token.is_some() => self.module_of(&segs),
                    _ => None,
                }
            }
        }
    }

    /// Follows the crate's `use` chains to its macros: a module's `use` of
    /// a path that names a definition (`pub(crate) use crate::v::build;`)
    /// gives the definition that module's path under the name the `use`
    /// brings in, and a module's `use p::*;` gives each definition `p`
    /// names its path there too, until no `use` adds a path. A `use` built
    /// only for tests adds none.
    fn follow_reexports(&mut self) {
        // Each pass can only add paths; a chain is no longer than the
        // crate has modules.
        for _ in 0..=self.modules.len() {
            let mut add: Vec<(String, usize, Vec<String>)> = Vec::new();
            for (module, uses) in &self.uses {
                for (p, glob, named, built) in uses {
                    if *built == Built::Test {
                        continue;
                    }
                    for (name, defs) in &self.macros {
                        for (i, d) in defs.iter().enumerate() {
                            let new: Vec<Vec<String>> = d
                                .paths
                                .iter()
                                .filter_map(|dp| match glob {
                                    false => names_from(module, p, dp)
                                        .then(|| [module.clone(), vec![named.clone()]].concat()),
                                    true => {
                                        let (last, parent) = dp.split_last()?;
                                        names_from(module, p, parent)
                                            .then(|| [module.clone(), vec![last.clone()]].concat())
                                    }
                                })
                                .filter(|n| !d.paths.contains(n))
                                .collect();
                            add.extend(new.into_iter().map(|n| (name.clone(), i, n)));
                        }
                    }
                }
            }
            if add.is_empty() {
                return;
            }
            for (name, i, path) in add {
                if let Some(d) = self.macros.get_mut(&name).and_then(|v| v.get_mut(i))
                    && !d.paths.contains(&path)
                {
                    d.paths.push(path);
                }
            }
        }
    }

    /// The module of the crate that `segs`, written in this module, names:
    /// through `crate`, `self` or `super`, or a module beside this one or
    /// at the root.
    fn module_of(&self, segs: &[String]) -> Option<Vec<String>> {
        self.module_from(&self.module, segs)
    }

    /// The module of the crate that `segs`, written in module `here`,
    /// names ([`Counter::module_of`]).
    fn module_from(&self, here: &[String], segs: &[String]) -> Option<Vec<String>> {
        let up = segs.iter().take_while(|s| *s == "super").count();
        let candidates = match segs.first().map(String::as_str) {
            Some("crate") => vec![[here.get(..1)?, &segs[1..]].concat()],
            Some("self") => vec![[here, &segs[1..]].concat()],
            Some("super") => {
                let base = here.len().checked_sub(up)?;
                vec![[&here[..base], &segs[up..]].concat()]
            }
            _ => vec![[here, segs].concat(), [here.get(..1)?, segs].concat()],
        };
        candidates.into_iter().find(|m| self.modules.contains(m))
    }

    /// Counts the functions macro invocation `mac`, an item or an `impl`'s
    /// or trait's item with `attrs`, writes, so that none leaves the tally
    /// for being written by a macro. In an `impl` or trait, `wrap` is the
    /// header (`impl Type`, `trait Name`, with an `impl`'s `external`
    /// attribute) its expansion is read inside. A `macro_rules!`
    /// definition of the crate ([`Counter::macros`]) resolves it, as rustc
    /// resolves it ([`Counter::resolve`]); several must count the same
    /// rows, class and lines included ([`Counter::dry_rows`]), else it is
    /// an error. Its expansion
    /// ([`expand::Rules::expand`]) is walked as code outside `verus!`
    /// (Verus's `verus!` does not expand the macros it holds; a `verus!`
    /// block the expansion writes is read as one), and each function in it
    /// is counted, listed at the invocation's line, with its lines from the
    /// invocation and the definition ([`expansion_lines`]). A macro the
    /// crate does not define cannot be read, so the invocation counts as
    /// one function outside `verus!`, named `NAME!`, over the invocation's
    /// lines. A definition that resolves but does not expand, or an
    /// expansion that does not parse, is an error. std's `thread_local!`
    /// ([`Counter::std_thread_local`]) writes statics, not functions; any
    /// other macro of the name is read as above.
    fn item_macro(
        &mut self,
        attrs: &[Attribute],
        mac: &verus_syn::Macro,
        wrap: Option<TokenStream>,
    ) {
        let Some(name) = mac.path.segments.last().map(|s| s.ident.to_string()) else {
            return;
        };
        // std's `thread_local!` writes statics alone. Any other may write
        // any item, so its module's globs place no type
        // ([`Counter::opaque`]); one shown to be another macro
        // ([`Counter::other_thread_local`]) is read as any other macro:
        // the crate's own expanded, another crate's counted as one
        // function.
        if name == "thread_local" {
            if self.std_thread_local(&mac.path) {
                return;
            }
            self.opaque.insert(self.module.clone());
            if !self.other_thread_local(&mac.path) {
                return;
            }
        }
        let built = self.built(attrs);
        if built == Built::Test {
            return;
        }
        let bang = mac.bang_token.span;
        let line = bang.start().line;
        let outer = self.expansion.is_none();
        let at = *self.expansion.get_or_insert(line);
        // The expansion is read under the invocation's `cfg`s: one Verus's
        // build might not meet ([`Built::Unsure`]) leaves its functions
        // shipped but not sure, so trusted, never verified.
        let was = self.enter(built, attrs);
        // Each definition the invocation could expand, by where it is.
        let here = format!("{}:{line}", self.file.display());
        let found: Vec<(String, Result<expand::Rules, String>)> =
            match self.resolve(&name, &mac.path, at) {
                Ok(defs) => defs
                    .into_iter()
                    .map(|d| (format!("{}:{}", d.file.display(), d.line), d.rules.clone()))
                    .collect(),
                Err(e) => {
                    eprintln!("{here}: {e}");
                    self.errors += 1;
                    self.leave(was);
                    if outer {
                        self.expansion = None;
                    }
                    return;
                }
            };
        if !found.is_empty() {
            self.invoked.push((
                here.clone(),
                self.file.clone(),
                self.module.clone(),
                at,
                name.clone(),
            ));
        }
        // The expansion by `rules`, read as items (inside `wrap`, in an
        // `impl` or trait).
        let expand = |rules: &expand::Rules| {
            rules.expand(mac.tokens.clone()).and_then(|t| {
                let src = match &wrap {
                    Some(w) => {
                        let mut s = w.clone();
                        s.extend([TokenTree::Group(proc_macro2::Group::new(
                            proc_macro2::Delimiter::Brace,
                            t,
                        ))]);
                        s
                    }
                    None => t,
                };
                parse2::<verus_syn::File>(src).map_err(|e| e.to_string())
            })
        };
        match found {
            defs if defs.is_empty() => {
                // What it writes is unread, and may be a type of any name,
                // whatever its tokens hold (`include!`, a fixed output, a
                // pasted name): the module's globs place no type
                // ([`Counter::type_path`]). `compile_error!` writes none.
                if self.std_writes_nothing(&name, &mac.path).is_none() {
                    self.opaque.insert(self.module.clone());
                }
                // A name the module imports from this crate (`use
                // crate::w::ex as build;`, `use w::ex;` with `w` a module of
                // the crate) is the crate's macro, by a path the counter did
                // not follow. Any other is checked, under the name it was
                // imported as, once the crate is read.
                let imported = self.imported(&name, false);
                if imported.iter().any(|p| self.local(p)) {
                    eprintln!(
                        "{here}: {name}! is imported from this crate by a path the counter does not follow; invoke it by its definition's path"
                    );
                    self.errors += 1;
                } else {
                    self.unresolved.push((here.clone(), name.clone()));
                    for p in &imported {
                        if let Some(orig) = p.last()
                            && *orig != name
                        {
                            self.unresolved.push((here.clone(), orig.clone()));
                        }
                    }
                }
                if self.shipped {
                    if let Some((kind, read)) = self.std_writes_nothing(&name, &mac.path) {
                        self.took
                            .push((kind, here.clone(), self.module.clone(), read));
                    } else {
                        let mut seen = std::collections::BTreeSet::new();
                        place(bang, &mut seen);
                        token_lines(mac.tokens.clone(), &mut seen);
                        place(mac.delimiter.span().close(), &mut seen);
                        self.tally(Class::Outside, format!("{name}!"), at, seen.len());
                    }
                }
            }
            _ if self.depth >= MAX_DEPTH => {
                eprintln!("{here}: {name}! expands more than {MAX_DEPTH} macros deep");
                self.errors += 1;
            }
            defs => {
                // Each definition that could be the one rustc expands here:
                // more than one (a pair under `cfg`s the counter cannot
                // settle) must write the same functions, or the count would
                // depend on which.
                let mut files = Vec::new();
                for (at, rules) in &defs {
                    match rules
                        .as_ref()
                        .map_err(|e| (true, e.clone()))
                        .and_then(|r| expand(r).map_err(|e| (false, e)))
                    {
                        Ok(f) => files.push(f),
                        Err((true, e)) => {
                            eprintln!("{here}: {name}!'s definition at {at} cannot be read: {e}");
                            self.errors += 1;
                        }
                        Err((false, e)) => {
                            eprintln!("{here}: {name}! does not expand: {e}");
                            self.errors += 1;
                        }
                    }
                }
                // The rows each writes, class and lines too, from a walk
                // that leaves the count as it was.
                let rows: Vec<Option<Vec<(Class, String, usize)>>> = if files.len() > 1 {
                    files.iter().map(|f| self.dry_rows(f)).collect()
                } else {
                    Vec::new()
                };
                let at: Vec<&str> = defs.iter().map(|(a, _)| a.as_str()).collect();
                if rows.iter().any(Option::is_none) {
                    eprintln!(
                        "{here}: {name}! could be any of {} definitions ({}), and one of them does not count (above)",
                        defs.len(),
                        at.join(", ")
                    );
                    self.errors += 1;
                } else if rows.windows(2).any(|w| w[0] != w[1]) {
                    eprintln!(
                        "{here}: {name}! could be any of {} definitions ({}), which count differently",
                        defs.len(),
                        at.join(", ")
                    );
                    self.errors += 1;
                } else if files.len() == defs.len()
                    && let Some(f) = files.first()
                {
                    self.depth += 1;
                    self.items(&f.items, false);
                    self.depth -= 1;
                }
            }
        }
        self.leave(was);
        if outer {
            self.expansion = None;
        }
    }

    /// The definitions of macro `name` that an invocation by `path`, at
    /// line `at` of the walk's file, may expand, as rustc resolves it,
    /// among those this build compiles:
    ///
    /// - a path of more than one segment (`width::exports!`,
    ///   `$crate::m!`) names the definitions it is a path of: the crate
    ///   root's for `#[macro_export]`, a module's for its `use NAME;`;
    /// - a bare name takes the last definition before it in its file whose
    ///   textual scope it is in, with each before that while the later is
    ///   under a `cfg` the counter cannot settle ([`attrs_unsure`]);
    /// - failing that, each definition the module imports by name or `*`,
    ///   each in another file whose textual scope reaches here (an
    ///   enclosing module's, or one `#[macro_use] mod` carries out), and,
    ///   at the crate root, each `#[macro_export]` puts there.
    ///
    /// It is an error when some definition is found and an imported path
    /// the invocation may stand for (a module alias's, or a `use` of the
    /// name) names none: the build that takes that path would expand what
    /// the counter cannot see.
    fn resolve(
        &self,
        name: &str,
        path: &verus_syn::Path,
        at: usize,
    ) -> Result<Vec<&MacroDef>, String> {
        // A definition under a `cfg` the counter cannot settle may be the
        // one: it is a candidate.
        let active = |d: &&MacroDef| match d.built {
            Built::Shipped | Built::Unsure => true,
            Built::Verus => !self.shipped,
            Built::Test => false,
        };
        // The definitions that `path`, read from this module, names: of any
        // name, as a re-export under a new name (`pub(crate) use ex as
        // build;`) makes `w::build` a path of `ex`.
        let by_path = |path: &[String]| -> Vec<&MacroDef> {
            self.macros
                .values()
                .flatten()
                .filter(active)
                .filter(|d| d.paths.iter().any(|p| names(&self.module, path, p)))
                .collect()
        };
        // The definitions each of `paths` names, and the paths that name
        // none.
        let gather = |paths: Vec<Vec<String>>| -> (Vec<&MacroDef>, Vec<String>) {
            let mut found = Vec::new();
            let mut missing = Vec::new();
            for p in paths {
                let named = by_path(&p);
                if named.is_empty() {
                    missing.push(p.join("::"));
                }
                found.extend(named);
            }
            (found, missing)
        };
        // An imported path that names none while another names some.
        let missed = |missing: &[String], any: bool| match missing.first() {
            Some(m) if any => Err(format!(
                "{name}! may stand for `{m}`, which names none of this crate's definitions of `{name}` by a path the counter follows, where another path it may stand for does; invoke it by its definition's path"
            )),
            _ => Ok(()),
        };
        let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        if segs.len() > 1 {
            // A first segment the module imports (`use crate::w as x;`)
            // stands for each path it imports: under `cfg`s the counter
            // cannot settle there may be several, and each one's
            // definitions are candidates, compared as any others are.
            // A module of the crate of that name, beside this one or at the
            // root, is one more: the path as written.
            let imported = self.imported(&segs[0], false);
            if imported.is_empty() {
                let found = by_path(&segs);
                // A module of the crate that brings the name in by a `use`
                // the counter could not follow to a definition holds the
                // crate's macro out of sight.
                if found.is_empty()
                    && let Some(m) = self.module_of(&segs[..segs.len() - 1])
                    && self.binds(&m, name)
                {
                    return Err(format!(
                        "{}! names `{name}` in `{}`, which brings it in by a `use` the counter does not follow; invoke it by its definition's path",
                        segs.join("::"),
                        m.join("::")
                    ));
                }
                return Ok(found);
            }
            let mut paths: Vec<Vec<String>> = imported
                .into_iter()
                .map(|p| [p, segs[1..].to_vec()].concat())
                .collect();
            if self.local(&segs[..1]) {
                paths.push(segs.clone());
            }
            let (found, missing) = gather(paths);
            missed(&missing, !found.is_empty())?;
            return Ok(joined(Vec::new(), found));
        }
        let defs: &[MacroDef] = self.macros.get(name).map_or(&[], Vec::as_slice);
        let visible = |d: &&MacroDef| self.module.starts_with(&d.scope);
        // Latest first: by line, an emitted definition after any written
        // on its line, and among equals (the definitions one expansion
        // writes, all at its invocation's line) the one registered later.
        // One a `#[macro_use] mod name;` here carries out of its file is
        // placed by its path: that `mod`'s line here, then the line of each
        // `mod` inward along the chain, then its own line, compared in that
        // order, so a definition in `j` after its `mod k;` shadows one in
        // `k`, and one in `k` shadows one in `j` before it.
        // Each step of a place is a line and, for a `mod`, the file it
        // leads into (`None` for the definition's own line).
        let here = |d: &MacroDef| -> Option<DefPlace> {
            if d.file == self.file {
                return Some(vec![(d.line, None)]);
            }
            let i = d.carried_at.iter().position(|(f, _)| *f == self.file)?;
            let mut place: DefPlace = (0..=i)
                .rev()
                .map(|k| {
                    let into = if k == 0 {
                        d.file.clone()
                    } else {
                        d.carried_at[k - 1].0.clone()
                    };
                    (d.carried_at[k].1, Some(into))
                })
                .collect();
            place.push((d.line, None));
            Some(place)
        };
        let lines = |p: &[(usize, Option<PathBuf>)]| p.iter().map(|(l, _)| *l).collect::<Vec<_>>();
        let mut before: Vec<(usize, DefPlace, &MacroDef)> = defs
            .iter()
            .enumerate()
            .filter(|(_, d)| active(d) && visible(d))
            .filter_map(|(i, d)| here(d).map(|l| (i, l, d)))
            .filter(|(.., d)| !(d.split && d.file != self.file))
            .filter(|(_, l, d)| l[0].0 < at || (d.emitted && d.file == self.file && d.line == at))
            .collect();
        // One a `mod` on the invocation's own line carries in is not
        // ordered against the invocation, so it is a candidate, but it
        // ends no chain. So is one carried in through a module several
        // `mod`s declare, from the first of them on.
        let on_line: Vec<&MacroDef> = defs
            .iter()
            .filter(|d| active(d) && visible(d) && d.file != self.file)
            .filter(|d| here(d).is_some_and(|l| l[0].0 == at || (d.split && l[0].0 < at)))
            .collect();
        before.sort_by(|(i, l, d), (j, m, e)| {
            (lines(m), e.emitted, j).cmp(&(lines(l), d.emitted, i))
        });
        // Two places that reach one line and go on from it differently (a
        // definition on the line of a `mod` that carries the other in, or
        // two `mod`s on one line into different files) are not ordered:
        // the order on a line is not read, so both stay candidates.
        let unordered = |a: &[(usize, Option<PathBuf>)], b: &[(usize, Option<PathBuf>)]| {
            for (x, y) in a.iter().zip(b) {
                if x.0 != y.0 {
                    return false;
                }
                if x.1 != y.1 {
                    return true;
                }
            }
            false
        };
        // When every one is under such a `cfg`, a build may compile none of
        // them, and then the definitions below are rustc's: they join.
        // In shipped code, one a shipped build may skip does not end them
        // either, though Verus's build is sure to compile it.
        let shadows = |d: &MacroDef| {
            !d.unsure && !(self.shipped && shipped_may_skip(&d.guards, &self.guards, &self.atoms))
        };
        if let Some(i) = before.iter().position(|(_, _, d)| shadows(d)) {
            let key = &before[i].1;
            let chain: Vec<&MacroDef> = before
                .iter()
                .enumerate()
                .filter(|(k, (_, l, _))| *k <= i || unordered(l, key))
                .map(|(_, (_, _, d))| *d)
                .collect();
            return Ok(joined(chain, on_line));
        }
        let textual: Vec<&MacroDef> = joined(before.iter().map(|(.., d)| *d).collect(), on_line);
        let others: Vec<&MacroDef> = defs
            .iter()
            .filter(active)
            .filter(|d| {
                (d.file != self.file && visible(d)) || (d.exported && self.module.len() == 1)
            })
            .collect();
        // A `*` may hold the name or not, so only a `use` of the name is
        // held to naming a definition. But a `*` of a module of the crate
        // that re-exports the name by a `use` the counter could not follow
        // to a definition ([`Counter::follow_reexports`]) does hold it, and
        // what it brings in is out of sight: an error.
        let (globbed, _) = gather(self.imported(name, true));
        for p in self.globs() {
            if by_path(&[p.clone(), vec![name.to_owned()]].concat()).is_empty()
                && let Some(m) = self.module_of(&p)
                && self.binds(&m, name)
            {
                return Err(format!(
                    "{name}! may come through `{}::*`, which re-exports `{name}` by a path the counter does not follow; invoke it by its definition's path",
                    p.join("::")
                ));
            }
        }
        let (named, missing) = gather(self.imported(name, false));
        let out = joined(joined(joined(textual, others), globbed), named);
        missed(&missing, !out.is_empty())?;
        Ok(out)
    }

    /// Whether module `m` brings `name` in by a `use` of it (not a `*`, and
    /// not one built only for tests).
    fn binds(&self, m: &[String], name: &str) -> bool {
        self.uses.get(m).is_some_and(|us| {
            us.iter()
                .any(|(_, glob, named, built)| !glob && *built != Built::Test && named == name)
        })
    }

    /// The paths this module's `use p::*;`s glob, where the code walked can
    /// be built with them ([`Counter::imported`]).
    fn globs(&self) -> Vec<Vec<String>> {
        self.uses
            .get(&self.module)
            .into_iter()
            .flatten()
            .filter(|(_, glob, _, built)| {
                *glob
                    && match built {
                        Built::Shipped | Built::Unsure => true,
                        Built::Verus => !self.shipped,
                        Built::Test => false,
                    }
            })
            .map(|(p, ..)| p.clone())
            .collect()
    }

    /// The paths, as written, of what this module's `use`s bring into
    /// scope as `name`: the path of a `use p::name;` or `use p::orig as
    /// name;`, and with `globs`, `p::name` for each `use p::*;`. A `use`
    /// is one only where the code walked can be built with it: a test-only
    /// `use` never, a Verus-only one in Verus-only code, any other (one
    /// under a `cfg` the counter cannot settle too) always.
    fn imported(&self, name: &str, globs: bool) -> Vec<Vec<String>> {
        self.uses
            .get(&self.module)
            .into_iter()
            .flatten()
            .filter(|(_, _, _, built)| match built {
                Built::Shipped | Built::Unsure => true,
                Built::Verus => !self.shipped,
                Built::Test => false,
            })
            .filter_map(|(p, glob, named, _)| match glob {
                true => globs.then(|| [p.clone(), vec![name.to_owned()]].concat()),
                false => (named == name).then(|| p.clone()),
            })
            .collect()
    }

    /// Why the rule of `kind` ([`Took`]) does not hold for `name` (the
    /// leading `::` segment, or the bare macro's name) invoked in module
    /// `at`, given `bindings`, if it does not: for a leading `::`, an
    /// edition before 2018 (where `::name` is this crate's root) or a
    /// dependency renamed to `name` ([`Counter::renamed_deps`]); then the
    /// first binding that breaks it ([`breaks`]), read in Verus's build for
    /// `verus!` and a shipped build for `compile_error!`.
    fn doubt(&self, bindings: &[Binding], kind: Took, name: &str, at: &[String]) -> Option<String> {
        if matches!(kind, Took::VerusPath | Took::StdPath) {
            if self.edition < 2018 {
                return Some(format!(
                    "the crate's edition is {}, where `::{name}` names this crate's root module",
                    self.edition
                ));
            }
            if let Some((_, why)) = self.renamed_deps.iter().find(|(n, _)| n == name) {
                return Some(why.clone());
            }
        }
        // A bare `verus!` comes from Verus's crates (`vstd::prelude::*`,
        // `#[macro_use] extern crate vstd;`): one renamed to another
        // package may bring another `verus`.
        if kind == Took::VerusBare
            && let Some((_, why)) = self
                .renamed_deps
                .iter()
                .find(|(n, _)| matches!(n.as_str(), "vstd" | "verus_builtin_macros"))
        {
            return Some(why.clone());
        }
        // A bare `compile_error!` comes from the prelude of the crate the
        // name `std` (or `core`) loads: one renamed to another package
        // (cargo passes it as `--extern std=..`) brings its own.
        if kind == Took::StdBare
            && let Some((_, why)) = self
                .renamed_deps
                .iter()
                .find(|(n, _)| matches!(n.as_str(), "std" | "core" | "alloc"))
        {
            return Some(why.clone());
        }
        let broken = bindings
            .iter()
            .filter(|b| match kind {
                Took::StdPath | Took::StdBare => b.shipped,
                Took::VerusPath | Took::VerusBare => b.verus,
            })
            .find(|b| breaks(b, kind, name, at))
            .map(Binding::describe);
        if broken.is_some() || kind != Took::VerusBare {
            return broken;
        }
        self.unsettled(bindings, at, &mut std::collections::BTreeSet::new())
            .map(|b| {
                format!(
                    "{} may bring another crate's `verus`, and no import of Verus's (`use vstd::prelude::*;` or `use vstd::prelude::verus;`) beside it settles the name",
                    b.describe()
                )
            })
    }

    /// Adds to [`Counter::proof_trees`] each root's files, from the
    /// declarations [`module_files`] read (`mods`), with no `cfg` read: a
    /// module's parent is its parent whether or not a build compiles it.
    /// A file is possibly the root's where any declaration from a possible
    /// file reaches it, under either reading of where that file's modules
    /// live (`ModFiles::alternate`). It is surely the root's, as the module
    /// its place under src/ names ([`module_path`]), where it is reached by
    /// place from src/lib.rs or src/main.rs.
    ///
    /// A file a `#[path]` names (a `#[cfg_attr(.., path = ..)]` choice too),
    /// one declared inside an inline module whose directory a path
    /// attribute chose, or under the other reading (`Declared::under_path`),
    /// a module file named main.rs or lib.rs, and another target's root
    /// hold a module their location does not name (`module_path` reads the
    /// location, rustc the declarations). So they, and every file declared
    /// from one at any depth, are never sure, even where another
    /// declaration reaches them by place, since rustc compiles a file once
    /// per declaration; and
    /// a root other than src/lib.rs and src/main.rs has no sure file at
    /// all. Each stays possible, so [`Counter::certain`] does not take its
    /// place for its module.
    fn add_proof_trees(&mut self, mods: &ModFiles, (root, src): (&Path, &Path), files: &[PathBuf]) {
        let canon: Vec<PathBuf> = files.iter().map(|f| canonical(f)).collect();
        let rel = |f: &PathBuf| f.strip_prefix(root).unwrap_or(f).display().to_string();
        let src_dir = canonical(src);
        let root_named = |f: &Path| {
            f.file_name()
                .is_some_and(|n| n == "lib.rs" || n == "main.rs")
        };
        let reach = |r: &PathBuf, keep: &dyn Fn(&Declared) -> bool| {
            let mut reached = std::collections::BTreeSet::from([r.clone()]);
            loop {
                let next: Vec<PathBuf> = mods
                    .declared
                    .iter()
                    .chain(&mods.alternate)
                    .filter(|d| reached.contains(&d.from) && !reached.contains(&d.file))
                    .filter(|d| keep(d))
                    .map(|d| d.file.clone())
                    .collect();
                if next.is_empty() {
                    break;
                }
                reached.extend(next);
            }
            reached
        };
        let maybes: Vec<_> = mods.roots.iter().map(|r| reach(r, &|_| true)).collect();
        // A declaration from a possible file names a module its file's
        // place does not: one inside an inline module whose directory a
        // path attribute chose, or under the other reading
        // (`Declared::under_path`), and one of a file a `#[path]` names, a
        // module file named main.rs or lib.rs (`name::main` to rustc,
        // while `module_path` drops that last segment), or another target's
        // root, whose modules' directory is its own (`ModFiles::dir_of`).
        // rustc compiles a file once for each declaration that reaches it,
        // so such a file, and every file declared from one, is also a
        // module its place does not name, whatever other declaration
        // reaches it by place.
        let edges = || mods.declared.iter().chain(&mods.alternate);
        let mut unplaced: std::collections::BTreeSet<&PathBuf> = edges()
            .filter(|d| maybes.iter().any(|m| m.contains(&d.from)))
            .filter(|d| {
                d.under_path
                    || mods.path_named.contains(&d.file)
                    || root_named(&d.file)
                    || mods.roots.contains(&d.file)
            })
            .map(|d| &d.file)
            .collect();
        loop {
            let next: Vec<&PathBuf> = edges()
                .filter(|d| unplaced.contains(&d.from) && !unplaced.contains(&d.file))
                .map(|d| &d.file)
                .collect();
            if next.is_empty() {
                break;
            }
            unplaced.extend(next);
        }
        let by_place = |d: &Declared| !unplaced.contains(&d.file);
        for (r, maybe) in mods.roots.iter().zip(&maybes) {
            // `module_path` names a file by its place under src/ from the
            // crate root there. A root elsewhere (src/bin/NAME.rs, or a
            // `[lib]` or `[[bin]]` `path`) names its modules from its own
            // `crate::`, so its trees hold no sure file, only the files it
            // may compile, and a `*` in any of them reads no module as this
            // crate's.
            let named = r.parent() == Some(src_dir.as_path()) && root_named(r);
            let sure = match named {
                true => reach(r, &by_place),
                false => std::collections::BTreeSet::new(),
            };
            let mut tree = ProofTree::default();
            for (f, cf) in files.iter().zip(&canon) {
                if maybe.contains(cf) {
                    tree.maybe.insert(rel(f));
                }
                if sure.contains(cf) {
                    tree.sure.insert(rel(f));
                }
            }
            self.proof_trees.push(tree);
        }
    }

    /// Whether the file of a binding of module instance `scope` (its start,
    /// [`Binding::scope`]) is surely the module its place under src/ names
    /// ([`module_path`]), so that its `super` is the parent that place gives:
    /// every root that may compile the file surely names it by its place
    /// ([`Counter::proof_trees`]), and some root does.
    fn certain(&self, scope: &str) -> bool {
        let file = scope.split(':').next().unwrap_or(scope);
        let mut named = false;
        for t in &self.proof_trees {
            if t.maybe.contains(file) && !t.sure.contains(file) {
                return false;
            }
            named |= t.sure.contains(file);
        }
        named
    }

    /// The `*` a bare `verus!` in module `at` may take another crate's
    /// `verus` through (a dependency's own macro of that name), if any,
    /// among the crate's bindings and `extra`'s, read in Verus's build: any
    /// `*` but `use super::*;`, or that one where the parent holds such a
    /// `*`, unless the module that holds it also imports Verus's where
    /// every proof run that compiles the module is sure to compile the
    /// import ([`every_run_sure`]). A `use`
    /// of Verus's by name shadows a `*`, and a `*` of Verus's beside one
    /// that brought another would make the name ambiguous, which rustc
    /// refuses. `seen` holds the modules already read.
    fn unsettled<'a>(
        &'a self,
        extra: &'a [Binding],
        at: &[String],
        seen: &mut std::collections::BTreeSet<Vec<String>>,
    ) -> Option<&'a Binding> {
        if !seen.insert(at.to_vec()) {
            return None;
        }
        let here: Vec<&Binding> = self
            .bindings
            .iter()
            .chain(extra)
            .filter(|b| b.verus && b.module == at)
            .collect();
        // A `*` that may bring another `verus` is read where Verus's build
        // may compile it; an import that settles the name, only where that
        // build is sure to, and only for the `*`s of its own module instance
        // ([`Binding::scope`]): two inline modules of one name under
        // exclusive `cfg`s share a path, and one's import is not in the
        // runs that compile the other.
        let confirmed: std::collections::BTreeSet<&str> = here
            .iter()
            .filter(|b| {
                b.verus_sure
                    && match b.glob {
                        true => one_of(&b.path, &VERUS_GLOBS),
                        false => b.name == "verus" && one_of(&b.path, &VERUS_PATHS),
                    }
            })
            .map(|b| b.scope.as_str())
            .collect();
        for b in here.into_iter().filter(|b| {
            b.glob && !one_of(&b.path, &VERUS_GLOBS) && !confirmed.contains(b.scope.as_str())
        }) {
            // Only `use super::*;` (`super` segments alone) is followed, into
            // the module that holds this one, and only where the `*`'s file
            // surely is the module its place names ([`Counter::certain`]),
            // so that place gives that parent. Any other `*` may name
            // another crate's module, or an alias of one (`pub use other as
            // q;` behind `crate::q`), in some target or run, and settles
            // nothing.
            let parent_glob =
                !b.absolute && !b.path.is_empty() && b.path.iter().all(|s| s == "super");
            let local = match parent_glob && self.certain(&b.scope) {
                true => self.module_from(at, &b.path),
                false => None,
            }
            .filter(|m| self.modules.contains(m));
            match local {
                Some(m) => {
                    if let Some(far) = self.unsettled(extra, &m, seen) {
                        return Some(far);
                    }
                }
                None => return Some(b),
            }
        }
        None
    }

    /// Whether the `thread_local` invoked by `path` is shown to be std's,
    /// which writes statics alone and so shadows no type: `::std::` where
    /// that rule holds ([`Took::StdPath`]), or the bare name where the
    /// crate defines no macro of that name and the bare rule holds
    /// ([`Took::StdBare`]: no binding of the name, no `#[macro_use]` of
    /// another crate, and no `*` in the module). Any other `thread_local`
    /// (another crate's, this crate's own, one a binding the walk cannot
    /// see may bring, or `std::` written without the leading `::`) may
    /// write any item, so the caller makes its module's globs place no
    /// type ([`Counter::opaque`]). A module with a `*` and a bare
    /// `thread_local!` therefore places no glob type, which fails closed:
    /// `::std::thread_local!` places them again. The rules are read over
    /// the bindings either build may compile: Verus's names are the ones
    /// the limits take, and a binding only Verus's build has (under
    /// `verus_keep_ghost`) can bring another `thread_local` there.
    fn std_thread_local(&self, path: &verus_syn::Path) -> bool {
        let segs = segments(path);
        let (kind, read) = if segs.len() == 1 && path.leading_colon.is_none() {
            if self.macros.contains_key("thread_local") {
                return false;
            }
            (Took::StdBare, "thread_local")
        } else if path.leading_colon.is_some() && segs == ["std", "thread_local"] {
            (Took::StdPath, "std")
        } else {
            return false;
        };
        // `doubt` reads the std rules over shipped bindings; give it every
        // binding either build may compile, each as shipped.
        let either: Vec<Binding> = self
            .bindings
            .iter()
            .filter(|b| b.shipped || b.verus)
            .map(|b| Binding {
                shipped: true,
                ..b.clone()
            })
            .collect();
        self.doubt(&either, kind, read, &self.module).is_none()
    }

    /// Whether the `thread_local` invoked by `path` is shown to be a macro
    /// other than std's, so that it counts as one (#68): the bare name
    /// where the crate defines a macro of that name, or where a `use` of
    /// this module imports the name from a path that does not start at
    /// `std` or `core`; or a path whose first segment is neither
    /// (`elsewhere::thread_local!`, `crate::thread_local!`). A `std` or
    /// `core` first segment that a `use` or an `extern crate .. as` gives
    /// another crate is neither (#137; [`Counter::use_rebinds`]), nor, in
    /// an import, one that names a module of the crate as rustc reads a
    /// `use` path (#140; [`Counter::use_module`]). A bare one
    /// only a `*` or a `#[macro_use]` may bind to another counts too
    /// (#122), unless each such source is shown not to bring one
    /// ([`Counter::foreign_glob`]): a `#[macro_use]` of a crate other than
    /// std's and Verus's ([`STD_OR_VERUS_CRATES`]), or of one of those a
    /// Cargo dependency is renamed to, or a `*` that may hold another
    /// `thread_local`, counts it.
    fn other_thread_local(&self, path: &verus_syn::Path) -> bool {
        let segs = segments(path);
        if segs.len() == 1 && path.leading_colon.is_none() {
            self.macros.contains_key("thread_local")
                || self.imported("thread_local", false).iter().any(|p| {
                    !self.std_crate(p.first())
                        || p.first().is_some_and(|f| {
                            self.use_rebinds(&self.module, f) || self.use_module(&self.module, f)
                        })
                })
                || self
                    .macro_use
                    .iter()
                    .any(|c| !STD_OR_VERUS_CRATES.contains(&c.as_str()) || self.renamed(c))
                || self.foreign_glob(&self.module, &mut Vec::new())
        } else {
            // A first segment that is a module of this crate (`mod core`)
            // shadows the extern prelude's `std` or `core`, and so does a
            // `use` here or an `extern crate .. as` that gives another
            // crate that name (#137). The path is read in this module's
            // scope in every edition, so a root `use` rebinds it only at
            // the root.
            !self.std_crate(segs.first())
                || self.local(&segs)
                || segs
                    .first()
                    .is_some_and(|f| self.binds(&self.module, f) || self.extern_alias(f))
        }
    }

    /// Whether `first`, a path's first segment, is `std` or `core`, unless
    /// a Cargo dependency is renamed to it.
    fn std_crate(&self, first: Option<&String>) -> bool {
        first.is_some_and(|f| matches!(f.as_str(), "std" | "core") && !self.renamed(f))
    }

    /// Whether a Cargo dependency of the crate is renamed to `name`
    /// ([`Counter::renamed_deps`]).
    fn renamed(&self, name: &str) -> bool {
        self.renamed_deps.iter().any(|(n, _)| n == name)
    }

    /// Whether module `path` of the crate is declared where every build
    /// compiling its parent compiles it: no `mod` item of it is under a
    /// `cfg` of its own ([`Counter::cfg_modules`]).
    fn surely_declared(&self, path: &[String]) -> bool {
        !self.cfg_modules.contains(path)
    }

    /// Whether module `m` has a glob, one a shipped build or Verus's may
    /// compile, that may bring a name of any kind: any but a glob of
    /// std's, core's or Verus's by a name no Cargo rename takes (a glob
    /// of a module of the crate, by any path or alias, or of another
    /// crate, may).
    fn glob_may_bring_any(&self, m: &[String]) -> bool {
        self.bindings.iter().any(|b| {
            // The glob's first segment is the crate of that name only
            // where no `use` here or `extern crate .. as` rebinds it and no
            // module of the crate has that name.
            let other = b.path.first().is_some_and(|f| {
                self.binds(m, f)
                    || self.extern_alias(f)
                    || self.module_from(m, std::slice::from_ref(f)).is_some()
            });
            b.glob
                && b.module == m
                && (b.shipped || b.verus)
                && (other
                    || !((one_of(&b.path, &VERUS_GLOBS) && !self.renamed(&b.path[0]))
                        || self.std_crate(b.path.first())))
        })
    }

    /// Whether a `use` path's first segment `f`, in module `m`, may name a
    /// crate other than the one of that name: a `use` here (`use
    /// elsewhere as std;`) or an `extern crate .. as` gives another crate
    /// that name, or, in 2015 (where the path is read from the crate
    /// root), a root `use` does or a root glob not shown to be std's,
    /// core's or Verus's may (it may bring a module of that name).
    fn use_rebinds(&self, m: &[String], f: &str) -> bool {
        self.binds(m, f)
            || (self.edition < 2018 && (self.binds(&m[..1], f) || self.glob_may_bring_any(&m[..1])))
            || self.extern_alias(f)
    }

    /// Whether a `use` path's plain first segment `f`, in module `m`, may
    /// name a module of the crate, as rustc reads a `use` path: in 2015 one
    /// at the crate root, from 2018 on one in `m` (#140). A module some
    /// build may leave out still may. A leading `::` is not read (the
    /// module's `use` paths are kept without it), so from 2018 on `use
    /// ::core::thread_local;` beside a `mod core` in `m` may too, though
    /// rustc reads it as core's: that fails the run (closed), and writing
    /// `::std::thread_local!` at the invocation, or no `mod core` beside
    /// the import, passes.
    fn use_module(&self, m: &[String], f: &str) -> bool {
        let base = if self.edition < 2018 { &m[..1] } else { m };
        self.modules.contains(&[base, &[f.to_owned()]].concat())
    }

    /// Whether an `extern crate path as name;` a shipped build or Verus's
    /// may compile gives `name` to a crate of another name.
    fn extern_alias(&self, name: &str) -> bool {
        self.bindings
            .iter()
            .any(|b| b.extern_crate && b.name == name && b.path != [name] && (b.shipped || b.verus))
    }

    /// Whether a `use p::*;` of module `m` (one not built only for tests)
    /// may bring a `thread_local` other than std's (#122). A glob of a
    /// module of this crate (whatever its name, a `mod core` included)
    /// may only where that module brings the name in by a `use`
    /// ([`Counter::binds`]) or has such a glob itself, read through every
    /// glob it has (`seen` ends a cycle). A plain first segment names such
    /// a module as rustc reads a `use` path: from 2018 on one in scope in
    /// `m` (none after a leading `::`), in 2015 one at the crate root,
    /// whatever `m` holds. Of a crate's globs, Verus's ([`VERUS_GLOBS`])
    /// and std's or core's cannot, where no Cargo rename
    /// ([`Counter::renamed`]), `use` in `m` (or, in 2015, at the root),
    /// or `extern crate .. as`
    /// ([`Counter::extern_alias`]) gives another crate that name, and no
    /// module of the crate has the first segment's name there (the rest
    /// would then be a re-export the counter does not follow); any other
    /// may.
    fn foreign_glob(&self, m: &[String], seen: &mut Vec<Vec<String>>) -> bool {
        if seen.iter().any(|s| s == m) {
            return false;
        }
        seen.push(m.to_vec());
        // Each glob of `m` a shipped build or Verus's may compile, with
        // whether it has a leading `::`.
        let globs: Vec<(Vec<String>, bool)> = self
            .bindings
            .iter()
            .filter(|b| b.glob && b.module == m && (b.shipped || b.verus))
            .map(|b| (b.path.clone(), b.absolute))
            .collect();
        globs.iter().any(|(p, absolute)| {
            // A plain first segment names, from 2018 on, a module in
            // scope here or an extern crate, and after a leading `::` an
            // extern crate alone; in 2015, a module at the crate root or
            // an extern crate there. A module of the crate it names is
            // read, whatever its name (a `mod core` included).
            let lookup = |q: &[String]| {
                match q.first().map(String::as_str) {
                    Some("crate" | "self" | "super") => self.module_from(m, q),
                    _ if self.edition < 2018 => Some([&m[..1], q].concat()),
                    _ if *absolute => None,
                    _ => Some([m, q].concat()),
                }
                .filter(|l| self.modules.contains(l))
            };
            let local = lookup(p);
            // A rebound first segment may be what the path names, beside a
            // module of that name under `cfg`s: then each may bring the
            // name.
            let rebound = p.first().is_some_and(|f| self.use_rebinds(m, f));
            if let Some(local) = local {
                let brings = rebound
                    || self.binds(&local, "thread_local")
                    || self.foreign_glob(&local, seen);
                // A module some build may leave out (under a `cfg` the
                // counter cannot settle) leaves the crate of that name as
                // the other reading.
                let named = p
                    .iter()
                    .filter(|s| !matches!(s.as_str(), "crate" | "self" | "super"))
                    .count();
                let base = local.len() - named;
                if brings || (base + 1..=local.len()).all(|k| self.surely_declared(&local[..k])) {
                    return brings;
                }
            }
            // A first segment that names a module of the crate, where the
            // rest is no module of it (a re-export the counter does not
            // follow: `mod core { pub(crate) use elsewhere::cell; }`), may
            // be any crate's.
            if p.len() > 1 && lookup(&p[..1]).is_some() {
                return true;
            }
            // A crate's: std's, core's and Verus's bring none, unless so
            // rebound.
            rebound
                || !((one_of(p, &VERUS_GLOBS) && !self.renamed(&p[0])) || self.std_crate(p.first()))
        })
    }

    /// How item macro `name`, invoked by `path` and defined nowhere in the
    /// crate, is one of the standard library's that write nothing that
    /// ships ([`STD_NO_FN`]), if it is: by `::core::` or `::std::`, or by
    /// its bare name, where the rule of that form holds ([`Took`]).
    /// Anything else counts as a function.
    fn std_writes_nothing(&self, name: &str, path: &verus_syn::Path) -> Option<(Took, String)> {
        let (_, paths) = STD_NO_FN.iter().find(|(n, _)| *n == name)?;
        let segs = segments(path);
        let (kind, read) = if segs.len() == 1 && path.leading_colon.is_none() {
            (Took::StdBare, name.to_owned())
        } else if path.leading_colon.is_some() && one_of(&segs, paths) {
            (Took::StdPath, segs[0].clone())
        } else {
            return None;
        };
        self.doubt(&self.bindings, kind, &read, &self.module)
            .is_none()
            .then_some((kind, read))
    }

    /// Whether `path`, a non-`*` `use` path in this module, names something
    /// of this crate: it starts at `crate`, `self` or `super`, or at a
    /// module of the crate, beside this one or at the root.
    fn local(&self, path: &[String]) -> bool {
        match path.first().map(String::as_str) {
            Some("crate" | "self" | "super") => true,
            Some(first) => {
                let here = [self.module.clone(), vec![first.to_owned()]].concat();
                let root = [self.module[..1].to_vec(), vec![first.to_owned()]].concat();
                self.modules.contains(&here) || self.modules.contains(&root)
            }
            None => false,
        }
    }

    /// The rows `f`, an expansion, writes (class, name and lines), from a
    /// walk of it that leaves the count as it was.
    fn dry_rows(&mut self, f: &verus_syn::File) -> Option<Vec<(Class, String, usize)>> {
        let tallies = self.tallies.clone();
        let (limits, errors, unresolved) = (self.limits.len(), self.errors, self.unresolved.len());
        let methods = self.methods.len();
        let verus_fns = self.verus_fns.clone();
        let macros = self.macros.clone();
        let outer = self.dry.replace(Vec::new());
        self.depth += 1;
        self.items(&f.items, false);
        self.depth -= 1;
        let rows = std::mem::replace(&mut self.dry, outer).unwrap_or_default();
        // A candidate whose walk failed (a nested invocation that does not
        // expand, or is itself ambiguous) has no rows to compare: `None`.
        let failed = self.errors > errors;
        self.tallies = tallies;
        self.limits.truncate(limits);
        self.methods.truncate(methods);
        self.unresolved.truncate(unresolved);
        // The bindings each candidate writes, and the blocks it took on
        // their rules, are kept: any of them may be the one rustc
        // expands, so each is checked once the crate is walked.
        self.errors = errors;
        self.verus_fns = verus_fns;
        self.macros = macros;
        (!failed).then_some(rows)
    }
}

/// `out` with each of `more` it does not hold yet, in order.
fn joined<'a>(mut out: Vec<&'a MacroDef>, more: Vec<&'a MacroDef>) -> Vec<&'a MacroDef> {
    for d in more {
        if !out.iter().any(|t| std::ptr::eq(*t, d)) {
            out.push(d);
        }
    }
    out
}

/// Whether `segs`, a path written in module `module`, names `full`, a
/// path from the crate root: through `crate::`, `self::` or `super::`
/// exactly, else as its last segments, which a `use` of a parent brings
/// in scope (`width::exports` in a module that has `width`).
fn names(module: &[String], segs: &[String], full: &[String]) -> bool {
    match segs.first().map(String::as_str) {
        Some("crate") => full.get(1..) == segs.get(1..),
        Some("self" | "super") => {
            let up = segs.iter().take_while(|s| *s == "super").count();
            let rest = &segs[up.max(usize::from(segs[0] == "self"))..];
            let Some(base) = module.len().checked_sub(up) else {
                return false;
            };
            [&module[..base], rest].concat() == full
        }
        _ => full.ends_with(segs),
    }
}

/// [`names`], with a path from neither `crate`, `self` nor `super` read
/// as from `module` or from the crate root alone, as a `use` reads it
/// ([`Counter::follow_reexports`]): `use ex;` in `w` names `w::ex`, not
/// every `ex`.
fn names_from(module: &[String], segs: &[String], full: &[String]) -> bool {
    match segs.first().map(String::as_str) {
        Some("crate" | "self" | "super") => names(module, segs, full),
        _ => {
            [module, segs].concat() == full
                || module
                    .first()
                    .is_some_and(|root| [std::slice::from_ref(root), segs].concat() == full)
        }
    }
}

/// A source line: the file a token was parsed from, and its line there.
type Place = (String, usize);

/// Adds to `seen` the line `s` starts on, in the file it was parsed from.
/// Each file the counter parses is its own source to proc-macro2, named
/// apart (`Span::file`), so lines of the same number in the invocation's
/// file and a definition's are two lines, and one line is one line.
fn place(s: proc_macro2::Span, seen: &mut std::collections::BTreeSet<Place>) {
    seen.insert((s.file(), s.start().line));
}

/// Adds to `seen` the line of each token of `ts`, and of each group's
/// closing delimiter ([`place`]).
fn token_lines(ts: TokenStream, seen: &mut std::collections::BTreeSet<Place>) {
    for t in ts {
        // A fragment forwarded opaque is an invisible group, with no
        // delimiters of its own to place ([`expand`]).
        if let TokenTree::Group(g) = &t
            && g.delimiter() == proc_macro2::Delimiter::None
        {
            token_lines(g.stream(), seen);
            continue;
        }
        place(t.span(), seen);
        if let TokenTree::Group(g) = t {
            place(g.span_close(), seen);
            token_lines(g.stream(), seen);
        }
    }
}

/// The lines of a function a macro wrote: each line one of its tokens
/// (signature, body and closing brace `close`) came from, in the
/// invocation or in a definition, so that a line added to either adds to
/// it.
fn expansion_lines(sig: &Signature, body: Option<&TokenStream>, close: proc_macro2::Span) -> usize {
    let mut seen = std::collections::BTreeSet::new();
    token_lines(
        verus_syn::__private::ToTokens::to_token_stream(sig),
        &mut seen,
    );
    if let Some(b) = body {
        token_lines(b.clone(), &mut seen);
    }
    place(close, &mut seen);
    seen.len()
}

/// Where [`macro_defs`] is reading: the file, the module it is in, where
/// that module is built ([`Built::Unsure`] under a `cfg` the counter
/// cannot settle), and the module that ends a definition's textual scope
/// there.
struct DefsAt<'a> {
    file: &'a Path,
    module: Vec<String>,
    built: Built,
    /// The `cfg` predicates of the file and the items around (as
    /// [`Counter::guards`] holds them).
    guards: Vec<String>,
    scope: Vec<String>,
    /// [`MacroDef::carried_at`] for a definition here.
    carried_at: Vec<(PathBuf, usize)>,
    /// [`MacroDef::split`] for a definition here.
    split: bool,
}

/// Whether `attrs` has `#[name]` itself, not through a `cfg_attr`: the
/// expander's markers (`#[macro_use] mod`, `#[macro_export]`), where one a
/// `cfg_attr` applies only in some build would carry definitions into
/// scope in the others ([`has_applied`] reads those, for the rules,
/// where reading more fails closed).
fn has_attr(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|a| a.path().is_ident(name))
}

/// The paths of more than one segment that name Verus's `verus!`: its
/// crate's, and `vstd::prelude`'s re-export of it.
const VERUS_PATHS: [&[&str]; 2] = [
    &["vstd", "prelude", "verus"],
    &["verus_builtin_macros", "verus"],
];

/// The modules of [`VERUS_PATHS`], whose `*` imports Verus's `verus!`.
const VERUS_GLOBS: [&[&str]; 2] = [&["vstd", "prelude"], &["verus_builtin_macros"]];

/// The crates whose `#[macro_use] extern crate` cannot bring a
/// `thread_local` other than std's ([`Counter::other_thread_local`]):
/// the standard library's and Verus's.
const STD_OR_VERUS_CRATES: [&str; 6] = [
    "std",
    "core",
    "alloc",
    "vstd",
    "verus_builtin",
    "verus_builtin_macros",
];

/// The standard library's item macros that write nothing that ships,
/// each with the paths of more than one segment that name it: only
/// `compile_error!`, which stops the build, and is in the prelude. The
/// rest of std's and core's macros are left out: `global_asm!` can write
/// executable code (a symbol Rust calls through an `extern` block) that
/// the counter cannot tell from a directive, `thread_local!` writes
/// statics (read as such), `include!` writes whatever its file holds, and
/// the others (`asm!`, `concat!`, `env!`, ...) are expressions, which
/// rustc refuses in item position. An item macro not named here counts
/// as a function.
const STD_NO_FN: [(&str, &[&[&str]]); 1] = [(
    "compile_error",
    &[&["core", "compile_error"], &["std", "compile_error"]],
)];

/// The names whose items the rules read ([`Took`]): the items of these
/// names are gathered as bindings.
const SHADOW_NAMES: [&str; 6] = [
    "vstd",
    "verus_builtin_macros",
    "verus",
    "compile_error",
    "core",
    "std",
];

/// The forms of a macro the walk takes as Verus's or std's, each by a
/// rule that holds by rustc's own semantics rather than by resolving
/// names ([`Counter::doubt`]); the bindings a macro expansion writes are
/// held to the same rule once the crate is walked.
#[derive(Clone, Copy, PartialEq)]
enum Took {
    /// `::vstd::prelude::verus!` or `::verus_builtin_macros::verus!`: in
    /// edition 2018 and later a leading `::` names an extern crate, so
    /// only an `extern crate .. as` of the name or a dependency renamed to
    /// it can make it another.
    VerusPath,
    /// A bare `verus!`: no binding of `vstd`, `verus_builtin_macros` or
    /// `verus` but Verus's own, no `#[macro_use] extern crate` but
    /// Verus's or std's, and no `*` in its module but Verus's unless the
    /// module imports Verus's too (`use vstd::prelude::*;`).
    VerusBare,
    /// `::core::compile_error!` or `::std::compile_error!`, by the rule
    /// for a leading `::`.
    StdPath,
    /// A bare `compile_error!`: nothing in the crate binds the name (an
    /// item, `macro_rules!`, `use` or `extern crate`), no `#[macro_use]
    /// extern crate` but std's, and no `*` in its module.
    StdBare,
}

/// A name the crate binds, in `module`, at `place`: an item or module of
/// that name (`path` empty), a `use` or `extern crate` of `path` as
/// `name`, a `*` of `path` (`glob`), or a `#[macro_use] extern crate
/// name` (`macro_use`); `expanded` when a macro expansion writes it.
/// `shipped` and `verus` say whether a shipped build and Verus's may
/// compile it.
#[derive(Clone, Debug, Default)]
struct Binding {
    module: Vec<String>,
    path: Vec<String>,
    glob: bool,
    macro_use: bool,
    name: String,
    place: String,
    expanded: bool,
    shipped: bool,
    verus: bool,
    /// Whether Verus's build is sure to compile it (`verus` says only that
    /// it may); a binding written by an expansion is never taken as sure.
    verus_sure: bool,
    /// The instance of `module` it is in: its file (root-relative), or the
    /// inline `mod` item that holds it, by that item's file, line and
    /// column. Two inline modules of one name under exclusive `cfg`s share
    /// a path but not an instance.
    scope: String,
    /// An `extern crate path as name;`.
    extern_crate: bool,
    /// A `use ::path..;`, whose path names an extern crate.
    absolute: bool,
}

impl Binding {
    /// The binding, as a message names it.
    fn describe(&self) -> String {
        let what = if self.macro_use {
            format!("`#[macro_use] extern crate {};`", self.name)
        } else if self.glob {
            let root = if self.absolute { "::" } else { "" };
            format!("`use {root}{}::*;`", self.path.join("::"))
        } else if self.path.is_empty() {
            format!("an item or module named `{}`", self.name)
        } else if self.extern_crate {
            format!("`extern crate {} as {};`", self.path.join("::"), self.name)
        } else {
            format!("`use {} as {};`", self.path.join("::"), self.name)
        };
        let by = if self.expanded {
            ", which a macro expansion writes"
        } else {
            ""
        };
        format!("{what} ({}{by})", self.place)
    }
}

/// Whether binding `b` breaks the rule of `kind` ([`Took`]) for `name`
/// (the leading `::` segment, or the bare macro's name) invoked in module
/// `at`. For a leading `::`: an `extern crate` that puts another crate in
/// the extern prelude under `name`. For a bare `verus!`: a binding of
/// `vstd`, `verus_builtin_macros` or `verus` but the crate itself or the
/// macro's own path (`use vstd::prelude::verus;`), or a `#[macro_use]` of
/// another crate. For a bare `compile_error!`: any binding of the name,
/// a `#[macro_use]` of another crate, or a `*` in module `at`.
fn breaks(b: &Binding, kind: Took, name: &str, at: &[String]) -> bool {
    match kind {
        Took::VerusPath | Took::StdPath => b.extern_crate && b.name == name && b.path != [name],
        Took::VerusBare => {
            if b.macro_use {
                return !matches!(
                    b.name.as_str(),
                    "vstd" | "verus_builtin_macros" | "std" | "core" | "alloc"
                );
            }
            !b.glob
                && matches!(b.name.as_str(), "vstd" | "verus_builtin_macros" | "verus")
                && b.path != [b.name.as_str()]
                && !one_of(&b.path, &VERUS_PATHS)
        }
        Took::StdBare => {
            if b.macro_use {
                return !matches!(b.name.as_str(), "std" | "core" | "alloc");
            }
            match b.glob {
                true => b.module == at,
                false => b.name == name,
            }
        }
    }
}

/// Where [`bindings_in`] reads: the file, the module, and the `cfg`s of
/// the file and the items around, of which the first `shared` are the
/// module's own (the file's and the modules' around), which every item
/// in the module is under ([`every_run_sure`]), the module instance,
/// `scope` ([`Binding::scope`]), and the line of the invocation whose
/// expansion the items are, if they are one ([`mod_instance`]).
struct BindAt<'a> {
    file: &'a Path,
    module: Vec<String>,
    guards: Vec<String>,
    shared: usize,
    scope: String,
    expansion: Option<usize>,
}

/// One root's files ([`Counter::add_proof_trees`]), root-relative: those
/// the root may compile (`maybe`, through any declaration and either
/// reading of where a file's modules live), and those it surely compiles
/// as the module their place under src/ names (`sure`).
#[derive(Default)]
struct ProofTree {
    sure: std::collections::BTreeSet<String>,
    maybe: std::collections::BTreeSet<String>,
}

/// The module instance ([`Binding::scope`]) of an inline `mod` whose name
/// starts at `start` in `file`. An expansion's `mod` carries the spans of
/// the macro's definition, which may be in another file at the line and
/// column of a `mod` there, so its instance names the invocation
/// (`expansion`, its line) instead, in a form no `mod` in a source file
/// takes.
fn mod_instance(file: &Path, expansion: Option<usize>, start: proc_macro2::LineColumn) -> String {
    match expansion {
        None => format!("{}:{}:{}", file.display(), start.line, start.column),
        Some(at) => format!(
            "{}:{at}: a mod the expansion writes, at {}:{}",
            file.display(),
            start.line,
            start.column
        ),
    }
}

/// Adds to `out` the bindings among `items` (in inline modules and
/// `verus!` blocks too) that the rules read ([`Took`]): every `use`,
/// `extern crate` and `*`, and every item or module named one of
/// [`SHADOW_NAMES`] (an enum's variants among them), each with whether a
/// shipped build and Verus's may compile it, under its `cfg`s and those
/// around it.
fn bindings_in(items: &[Item], at: &BindAt, atoms: &Atoms, out: &mut Vec<Binding>) {
    for item in items {
        let attrs: &[Attribute] = match item {
            Item::Use(u) => &u.attrs,
            Item::ExternCrate(e) => &e.attrs,
            Item::Mod(m) => &m.attrs,
            Item::Macro(m) => &m.attrs,
            Item::Fn(f) => &f.attrs,
            Item::Struct(s) => &s.attrs,
            Item::Enum(e) => &e.attrs,
            Item::Union(u) => &u.attrs,
            Item::Type(t) => &t.attrs,
            Item::Const(c) => &c.attrs,
            Item::Static(s) => &s.attrs,
            Item::Trait(t) => &t.attrs,
            Item::TraitAlias(t) => &t.attrs,
            _ => &[],
        };
        let mut guards = at.guards.clone();
        guards.extend(cfg_predicates(attrs));
        let sees = |verus: bool, atoms: &Atoms| {
            guards
                .iter()
                .all(|p| cfg_eval(p, verus, atoms).map_or(true, |c| c.t))
        };
        // Verus sees it where some proof run may (#21).
        let (shipped, verus) = (
            sees(false, atoms),
            atoms.each_run().iter().any(|r| sees(true, r)),
        );
        let base = Binding {
            module: at.module.clone(),
            shipped,
            verus,
            verus_sure: every_run_sure(&guards[..at.shared], &guards[at.shared..], atoms)
                .unwrap_or(false),
            scope: at.scope.clone(),
            ..Binding::default()
        };
        let line = |s: proc_macro2::Span| format!("{}:{}", at.file.display(), s.start().line);
        match item {
            Item::Use(u) => {
                let mut targets = Vec::new();
                use_paths(&u.tree, &mut Vec::new(), &mut targets);
                out.extend(targets.into_iter().map(|(path, glob, name)| Binding {
                    path,
                    glob,
                    name,
                    absolute: u.leading_colon.is_some(),
                    place: line(u.use_token.span),
                    ..base.clone()
                }));
            }
            Item::ExternCrate(e) => out.push(Binding {
                path: vec![e.ident.to_string()],
                name: e.rename.as_ref().map_or(&e.ident, |(_, r)| r).to_string(),
                place: line(e.extern_token.span),
                extern_crate: true,
                ..base.clone()
            }),
            _ => {}
        }
        let mut idents: Vec<&proc_macro2::Ident> = Vec::new();
        match item {
            Item::Mod(m) => idents.push(&m.ident),
            Item::Fn(f) => idents.push(&f.sig.ident),
            Item::Struct(s) => idents.push(&s.ident),
            Item::Enum(e) => {
                idents.push(&e.ident);
                idents.extend(e.variants.iter().map(|v| &v.ident));
            }
            Item::Union(u) => idents.push(&u.ident),
            Item::Type(t) => idents.push(&t.ident),
            Item::Const(c) => idents.push(&c.ident),
            Item::Static(s) => idents.push(&s.ident),
            Item::Trait(t) => idents.push(&t.ident),
            Item::TraitAlias(t) => idents.push(&t.ident),
            _ => {}
        }
        for i in idents
            .into_iter()
            .filter(|i| SHADOW_NAMES.iter().any(|n| *i == n))
        {
            out.push(Binding {
                name: i.to_string(),
                place: line(i.span()),
                ..base.clone()
            });
        }
        let inner = |module: Vec<String>, shared: usize, scope: String| BindAt {
            file: at.file,
            module,
            guards: guards.clone(),
            shared,
            scope,
            expansion: at.expansion,
        };
        match item {
            // An inline module's `cfg`s are its own items' shared ones, and
            // it is a module instance of its own; a `verus!` block's `cfg`s
            // cover only what it holds, in the instance around it.
            Item::Mod(m) => {
                if let Some((_, items)) = &m.content {
                    let module = [&at.module[..], &[m.ident.to_string()]].concat();
                    let scope = mod_instance(at.file, at.expansion, m.ident.span().start());
                    bindings_in(items, &inner(module, guards.len(), scope), atoms, out);
                }
            }
            Item::Macro(m) if verus_path(&m.mac.path) => {
                if let Ok(f) = parse2::<verus_syn::File>(m.mac.tokens.clone()) {
                    let block = inner(at.module.clone(), at.shared, at.scope.clone());
                    bindings_in(&f.items, &block, atoms, out);
                }
            }
            _ => {}
        }
    }
}

/// Whether `tokens` hold a `fn` at any depth: a `verus!` block that may
/// write a function.
fn holds_fn(tokens: TokenStream) -> bool {
    tokens.into_iter().any(|t| match t {
        TokenTree::Ident(i) => i == "fn",
        TokenTree::Group(g) => holds_fn(g.stream()),
        _ => false,
    })
}

/// The segments of `path`, as written.
fn segments(path: &verus_syn::Path) -> Vec<String> {
    path.segments.iter().map(|s| s.ident.to_string()).collect()
}

/// Whether `segs` is one of `paths`.
fn one_of(segs: &[String], paths: &[&[&str]]) -> bool {
    paths
        .iter()
        .any(|p| segs.iter().map(String::as_str).eq(p.iter().copied()))
}

/// Adds to `out` the crate of each `#[macro_use] extern crate` among
/// `items` (`macro_use` applied directly or through a `cfg_attr`), in
/// `verus!` blocks too, but those built only for tests.
fn macro_use_crates(items: &[Item], atoms: &Atoms, out: &mut Vec<String>) {
    for item in items {
        match item {
            Item::ExternCrate(e)
                if has_applied(&e.attrs, "macro_use")
                    && attrs_built(&e.attrs, atoms) != Ok(Built::Test) =>
            {
                out.push(e.ident.to_string());
            }
            Item::Macro(m) if verus_path(&m.mac.path) => {
                if let Ok(f) = parse2::<verus_syn::File>(m.mac.tokens.clone()) {
                    macro_use_crates(&f.items, atoms, out);
                }
            }
            _ => {}
        }
    }
}

/// Whether an item macro invoked by `path` is Verus's `verus!` by the
/// path's shape: a bare `verus`, or one of [`VERUS_PATHS`] with or
/// without a leading `::`. The walk takes a block that writes a function
/// only bare or with a leading `::`, by the rule of that form ([`Took`]),
/// and fails the run on any other.
pub(crate) fn verus_path(path: &verus_syn::Path) -> bool {
    let segs = segments(path);
    (path.leading_colon.is_none() && segs == ["verus"]) || one_of(&segs, &VERUS_PATHS)
}

/// The modules among `items` (in `module`, and in its inline modules)
/// that `mod name;` declares in a file of their own, into `carried`, each
/// declaration with `#[macro_use]` with the declaring `file` and the line
/// of its `mod`: the textual scope of a definition at such a file's top
/// carries on into the module that declares it, from that line on. One
/// without is kept too, as a declaration that does not carry. Each
/// declaration of an inline module goes into `inline`, with whether it
/// has `#[macro_use]`: that scope carries on through one that has. Every
/// module `items` declares goes into `all`, one declared inside a
/// `verus!` block too.
fn macro_use_mods(
    items: &[Item],
    module: &[String],
    file: &Path,
    (carried, inline, all): (
        &mut Carried,
        &mut InlineMods,
        &mut std::collections::BTreeSet<Vec<String>>,
    ),
) {
    for item in items {
        if let Item::Macro(v) = item
            && verus_path(&v.mac.path)
            && let Ok(f) = parse2::<verus_syn::File>(v.mac.tokens.clone())
        {
            macro_use_mods(&f.items, module, file, (carried, inline, all));
            continue;
        }
        let Item::Mod(m) = item else { continue };
        let mut inner = module.to_vec();
        inner.push(m.ident.to_string());
        all.insert(inner.clone());
        match &m.content {
            None => carried.entry(inner).or_default().push(
                has_attr(&m.attrs, "macro_use")
                    .then(|| (file.to_path_buf(), m.mod_token.span.start().line)),
            ),
            Some((_, items)) => {
                inline
                    .entry(inner.clone())
                    .or_default()
                    .push(has_attr(&m.attrs, "macro_use"));
                macro_use_mods(items, &inner, file, (carried, inline, all));
            }
        }
    }
}

/// The modules among `items` (in `module`, and in its inline modules)
/// whose `mod` item is under a `cfg` of its own, or inside a `verus!`
/// block that is, and every module inside such a one, into `out`: a
/// build may leave each out ([`Counter::foreign_glob`]).
fn cfg_gated_mods(
    items: &[Item],
    module: &[String],
    out: &mut std::collections::BTreeSet<Vec<String>>,
) {
    for item in items {
        if let Item::Macro(v) = item
            && verus_path(&v.mac.path)
            && let Ok(f) = parse2::<verus_syn::File>(v.mac.tokens.clone())
        {
            if cfg_predicates(&v.attrs).is_empty() {
                cfg_gated_mods(&f.items, module, out);
            } else {
                // Every module the block declares is under its `cfg`.
                let mut all = std::collections::BTreeSet::new();
                let (mut c, mut i) = (Carried::new(), InlineMods::new());
                macro_use_mods(&f.items, module, Path::new(""), (&mut c, &mut i, &mut all));
                out.extend(all);
            }
            continue;
        }
        let Item::Mod(m) = item else { continue };
        let inner = [module, &[m.ident.to_string()]].concat();
        if !cfg_predicates(&m.attrs).is_empty() {
            out.insert(inner.clone());
        }
        if let Some((_, items)) = &m.content {
            cfg_gated_mods(items, &inner, out);
        }
    }
}

/// The paths `tree`, a `use`, names, under `prefix`.
fn use_paths(tree: &verus_syn::UseTree, prefix: &mut Vec<String>, out: &mut Vec<UseTarget>) {
    match tree {
        verus_syn::UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            use_paths(&p.tree, prefix, out);
            prefix.pop();
        }
        verus_syn::UseTree::Name(n) => {
            let mut p = prefix.clone();
            p.push(n.ident.to_string());
            out.push((p, false, n.ident.to_string()));
        }
        verus_syn::UseTree::Glob(_) => out.push((prefix.clone(), true, String::new())),
        verus_syn::UseTree::Group(g) => {
            for t in &g.items {
                use_paths(t, prefix, out);
            }
        }
        // A rename keeps the path it imports, under its new name.
        verus_syn::UseTree::Rename(r) => {
            let mut p = prefix.clone();
            p.push(r.ident.to_string());
            out.push((p, false, r.rename.to_string()));
        }
    }
}

/// Whether a shipped build that compiles code under the `cfg` predicates
/// `at` may leave out an item under `guards`: the conjunction of those of
/// `guards` that `at` does not hold can be false there. One the counter
/// cannot read may be.
fn shipped_may_skip(guards: &[String], at: &[String], atoms: &Atoms) -> bool {
    let own: Vec<&str> = guards
        .iter()
        .filter(|g| !at.contains(g))
        .map(String::as_str)
        .collect();
    !own.is_empty()
        && !cfg_eval(&format!("all({})", own.join(",")), false, atoms).is_ok_and(|c| !c.f)
}

/// The definition `m`, a `macro_rules!` item, at `at`.
fn macro_def(m: &verus_syn::ItemMacro, at: &DefsAt, atoms: &Atoms, emitted: bool) -> MacroDef {
    let built = narrower(
        at.built,
        attrs_built(&m.attrs, atoms).unwrap_or(Built::Shipped),
    );
    MacroDef {
        file: at.file.to_path_buf(),
        line: m.mac.bang_token.span.start().line,
        emitted,
        module: at.module.clone(),
        paths: if has_attr(&m.attrs, "macro_export") {
            vec![vec![
                at.module.first().cloned().unwrap_or_default(),
                m.ident
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            ]]
        } else {
            Vec::new()
        },
        built,
        unsure: built == Built::Unsure,
        guards: [&at.guards[..], &cfg_predicates(&m.attrs)].concat(),
        scope: at.scope.clone(),
        carried_at: at.carried_at.clone(),
        split: at.split,
        exported: has_attr(&m.attrs, "macro_export"),
        rules: expand::Rules::parse(m.mac.tokens.clone()),
    }
}

/// The crate's `macro_rules!` definitions, by name.
type MacroDefs = BTreeMap<String, Vec<MacroDef>>;

/// The modules a `#[macro_use] mod name;` declares in a file of their own
/// ([`macro_use_mods`]), each with every `mod` declaring it in a file of
/// its own (under `cfg`s, one module may have several): the file and the
/// line of each with `#[macro_use]`, `None` for each without.
type Carried = BTreeMap<Vec<String>, Vec<Option<(PathBuf, usize)>>>;

/// An invocation [`Counter::resolve`] found definitions for: where it is
/// (`file:line`), its file, its module, its outermost invocation's line
/// and its name.
type Invoked = (String, PathBuf, Vec<String>, usize, String);

/// A file's module, and the scope, [`MacroDef::carried_at`] and
/// [`MacroDef::split`] of a definition at its top.
type Carry = (Vec<String>, Vec<String>, Vec<(PathBuf, usize)>, bool);

/// Each declaration of each inline module ([`macro_use_mods`]): whether it
/// has `#[macro_use]`.
type InlineMods = BTreeMap<Vec<String>, Vec<bool>>;

/// Where a definition stands in an invocation's file
/// ([`Counter::resolve`]): each `mod` step's line with the file it leads
/// into, then the definition's own line (`None`).
type DefPlace = Vec<(usize, Option<PathBuf>)>;

/// Each module's `use` paths.
type Uses = BTreeMap<Vec<String>, Vec<UsePath>>;

/// Keeps each `macro_rules!` definition in `items` (and in the inline
/// modules and `verus!` blocks among them) under its name, for
/// [`Counter::item_macro`], with where it is built (under `#[cfg(test)]`,
/// in a test-only module or file, it is built for tests alone) and how far
/// its scope reaches, and each module's `use` paths. `at` is where the
/// items are.
fn macro_defs(
    items: &[Item],
    at: &DefsAt,
    atoms: &Atoms,
    (out, uses): (&mut MacroDefs, &mut Uses),
) {
    let built = |attrs: &[Attribute]| {
        narrower(
            at.built,
            attrs_built(attrs, atoms).unwrap_or(Built::Shipped),
        )
    };
    let guards = |attrs: &[Attribute]| [&at.guards[..], &cfg_predicates(attrs)].concat();
    for item in items {
        match item {
            Item::Macro(m) if m.mac.path.is_ident("macro_rules") => {
                if let Some(name) = &m.ident {
                    out.entry(name.to_string())
                        .or_default()
                        .push(macro_def(m, at, atoms, false));
                }
            }
            Item::Use(u) => {
                let mut targets = Vec::new();
                use_paths(&u.tree, &mut Vec::new(), &mut targets);
                let b = built(&u.attrs);
                uses.entry(at.module.clone()).or_default().extend(
                    targets
                        .into_iter()
                        .map(|(p, glob, named)| (p, glob, named, b)),
                );
            }
            Item::Macro(m) if verus_path(&m.mac.path) => {
                if let Ok(f) = parse2::<verus_syn::File>(m.mac.tokens.clone()) {
                    let inner_at = DefsAt {
                        file: at.file,
                        module: at.module.clone(),
                        built: built(&m.attrs),
                        guards: guards(&m.attrs),
                        scope: at.scope.clone(),
                        carried_at: at.carried_at.clone(),
                        split: at.split,
                    };
                    macro_defs(&f.items, &inner_at, atoms, (out, uses));
                }
            }
            Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    let mut module = at.module.clone();
                    module.push(m.ident.to_string());
                    let (scope, carried_at, split) = if has_attr(&m.attrs, "macro_use") {
                        (at.scope.clone(), at.carried_at.clone(), at.split)
                    } else {
                        (module.clone(), Vec::new(), false)
                    };
                    let inner_at = DefsAt {
                        file: at.file,
                        module,
                        built: built(&m.attrs),
                        guards: guards(&m.attrs),
                        scope,
                        carried_at,
                        split,
                    };
                    macro_defs(inner, &inner_at, atoms, (out, uses));
                }
            }
            _ => {}
        }
    }
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// A share to a tenth of a percent, rounded down as the floor rounds it.
fn pct(a: usize, b: usize) -> String {
    if b == 0 {
        "-".into()
    } else {
        format!("{}%", shown(tenths(a, b)))
    }
}

fn shown(t: u32) -> String {
    format!("{}.{}", t / 10, t % 10)
}

/// Whether a crate ships: every crate but one whose Cargo.toml says
/// `publish = false` (pcre2-testdata, the conformance harness's reader).
fn ships(krate: &Path) -> bool {
    fs::read_to_string(krate.join("Cargo.toml")).map_or(true, |t| !unpublished(&t))
}

/// Whether the Cargo.toml `text` sets its package's `publish` to `false`,
/// read as TOML: a `publish = false` in another table
/// (`[package.metadata]`) is not the package's. A manifest that does not
/// parse, or a `publish` of any other value, publishes: the crate ships.
fn unpublished(text: &str) -> bool {
    toml::from_str::<toml::Table>(text)
        .ok()
        .and_then(|t| t.get("package")?.get("publish")?.as_bool())
        == Some(false)
}

/// A crate the counter reads: its name, its directory (holding `src/`),
/// whether it ships ([`ships`], or the configuration's `ships`), and
/// whether its Cargo.toml decides that (`by_rule`: a `dir` crate, or a
/// member without `ships`).
struct Krate {
    name: String,
    dir: PathBuf,
    ships: bool,
    by_rule: bool,
}

/// The crates `c` names under `root`, in reading order: each
/// subdirectory of `[crates] dir` with a `src/`, in path order, named for
/// the subdirectory, then each of `[crates] members`.
fn crate_dirs(root: &Path, c: &Config) -> Result<Vec<Krate>, String> {
    // `--base-rev` compares crates by their configured paths
    // (`Config::narrowed_from`), so two spellings of one directory, through
    // `..`, a symlink or a path from `/`, would let a crate the base read
    // pass as a new one.
    for p in c
        .crates
        .dir
        .iter()
        .chain(c.crates.members.iter().map(|m| &m.path))
    {
        let path = Path::new(p);
        if path.has_root() {
            return Err(format!(
                "[crates] path {p} is not relative: give each crate's directory as one path under {}",
                root.display()
            ));
        }
        if path
            .components()
            .any(|k| k == std::path::Component::ParentDir)
        {
            return Err(format!(
                "[crates] path {p} has a `..`: give each crate's directory as one path under {}",
                root.display()
            ));
        }
        if let Some(link) = symlink_in(root, path) {
            return Err(format!(
                "[crates] path {p} passes through {}, a symlink: give each crate's directory as one path under {}",
                link.display(),
                root.display()
            ));
        }
    }
    let mut out = Vec::new();
    if let Some(d) = &c.crates.dir {
        let Ok(rd) = fs::read_dir(root.join(d)) else {
            return Err(format!("no {d}/ under {}", root.display()));
        };
        let mut dirs: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.join("src").is_dir())
            .collect();
        dirs.sort();
        out.extend(dirs.into_iter().map(|p| {
            Krate {
                name: p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                ships: ships(&p),
                by_rule: true,
                dir: p,
            }
        }));
    }
    for m in &c.crates.members {
        let p = root.join(&m.path);
        if !p.join("src").is_dir() {
            return Err(format!("crate {}: no src/ under {}", m.name, p.display()));
        }
        out.push(Krate {
            name: m.name.clone(),
            ships: m.ships.unwrap_or_else(|| ships(&p)),
            by_rule: m.ships.is_none(),
            dir: p,
        });
    }
    // A crate of `dir` that is a link, or sources reached through one,
    // would read another crate's code under a new name, as a configured
    // path through a symlink would.
    for k in &out {
        let rel = k.dir.strip_prefix(root).unwrap_or(&k.dir);
        if let Some(link) = symlink_in(root, rel).or_else(|| symlink_below(root, &rel.join("src")))
        {
            return Err(format!(
                "[crates] crate {} reads {}, a symlink: give each crate's directory and sources as files under {}",
                k.name,
                link.display(),
                root.display()
            ));
        }
    }
    // The tally and the shipped row go by crate name, so a name read
    // twice would let one `ships = false` take both out, and a directory
    // read twice would count its crate twice.
    for (i, k) in out.iter().enumerate() {
        let dir = fs::canonicalize(&k.dir).unwrap_or_else(|_| k.dir.clone());
        if let Some(j) = out[..i].iter().find(|j| {
            j.name == k.name || fs::canonicalize(&j.dir).unwrap_or_else(|_| j.dir.clone()) == dir
        }) {
            return Err(format!(
                "[crates] reads crate {} ({}) and crate {} ({}): give each crate one name and one directory",
                j.name,
                j.dir.display(),
                k.name,
                k.dir.display()
            ));
        }
    }
    Ok(out)
}

/// The seams tables `c` names under `root`: its `tables`, then each `*.md`
/// of its `theorems` directory, in path order; and how the report names
/// them (`verus/SEAMS.md, verus/theorems/*.md`).
fn seam_tables(root: &Path, c: &config::Seams) -> (Vec<String>, String) {
    let mut texts: Vec<String> = c
        .tables
        .iter()
        .map(|t| fs::read_to_string(root.join(t)).unwrap_or_default())
        .collect();
    let mut names = c.tables.clone();
    if let Some(dir) = &c.theorems {
        let mut theorems: Vec<PathBuf> = fs::read_dir(root.join(dir))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "md"))
            .collect();
        theorems.sort();
        texts.extend(theorems.iter().filter_map(|p| fs::read_to_string(p).ok()));
        names.push(format!("{dir}/*.md"));
    }
    (texts, names.join(", "))
}

/// What `--help` prints: the forms of the command line and each option.
/// kit/count/README.md has the whole story.
const USAGE: &str = "\
usage: verus-count [--config FILE] [--list] [--floor FILE [--base-rev REV | --no-base]] [--rlimit STATS] [ROOT]
       verus-count [--config FILE] --badge [ROOT]
       verus-count --version | --help

ROOT is the repository root (default .), and FILE the configuration
(default ROOT/verus/count.toml).

  --config FILE   read the configuration from FILE
  --list          print one line per executable function before the table
  --floor FILE    fail when a share of the shipped row is below FILE's figure
  --base-rev REV  also hold FLOOR, RLIMIT, REACH and the configuration to REV's
  --no-base       run --floor with no base (a local run, or the first FLOOR)
  --rlimit STATS  hold each verified function to verus/RLIMIT, from Verus's
                  --time-expanded --output-json statistics
  --badge         print the README badge line FLOOR's figures give
  --version       print the counter's version
  --help, -h      print this

An unknown option or a second ROOT exits 2. See kit/count/README.md.";

/// Exits 2 with `e` printed.
fn usage(e: impl std::fmt::Display) -> ! {
    eprintln!("{e}");
    std::process::exit(2);
}

/// The command line, as `parse_args` reads it.
#[derive(Debug, Default, PartialEq)]
struct Args {
    list: bool,
    version: bool,
    help: bool,
    badge: bool,
    floor: Option<PathBuf>,
    base_rev: Option<String>,
    no_base: bool,
    stats: Option<PathBuf>,
    config_file: Option<PathBuf>,
    root: Option<PathBuf>,
}

/// Reads the arguments after the program name. An argument that starts
/// with `-` and is not an option fails, as does a second ROOT: a
/// misspelt `--flor FILE` would otherwise be read as ROOT, and the run
/// would pass with no floor checked. `--badge` with a checking option
/// fails too, since it prints the badge line and checks nothing, and so
/// does `--help` or `--version` with any other argument: either prints
/// and exits 0, so a check's command line that gained one would pass
/// with nothing checked.
fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut out = Args::default();
    let mut given = 0;
    while let Some(a) = args.next() {
        given += 1;
        match a.as_str() {
            "--list" => out.list = true,
            "--version" => out.version = true,
            "--help" | "-h" => out.help = true,
            "--no-base" => out.no_base = true,
            "--badge" => out.badge = true,
            "--floor" | "--base-rev" | "--rlimit" | "--config" => {
                let v = args.next().ok_or_else(|| format!("{a} needs a value"))?;
                match a.as_str() {
                    "--floor" => out.floor = Some(PathBuf::from(v)),
                    "--rlimit" => out.stats = Some(PathBuf::from(v)),
                    "--config" => out.config_file = Some(PathBuf::from(v)),
                    _ => out.base_rev = Some(v),
                }
            }
            _ if a.starts_with('-') => return Err(format!("unknown option {a}")),
            _ if out.root.is_some() => {
                return Err(format!("a second ROOT {a}: give one repository root"));
            }
            _ => out.root = Some(PathBuf::from(a)),
        }
    }
    if (out.help || out.version) && given > 1 {
        return Err(
            "--help and --version print and check nothing: give either one alone".to_owned(),
        );
    }
    if out.no_base && out.base_rev.is_some() {
        return Err("--no-base with --base-rev: give one".to_owned());
    }
    if out.badge
        && (out.list
            || out.floor.is_some()
            || out.base_rev.is_some()
            || out.no_base
            || out.stats.is_some())
    {
        return Err(
            "--badge prints the badge line and checks nothing: give it without --list, --floor, --base-rev, --no-base or --rlimit".into(),
        );
    }
    Ok(out)
}

fn main() {
    let Args {
        list,
        version,
        help,
        badge,
        floor,
        base_rev,
        no_base,
        stats,
        config_file,
        root,
    } = parse_args(std::env::args().skip(1)).unwrap_or_else(|e| usage(e));
    if help {
        println!("{USAGE}");
        return;
    }
    if version {
        println!("verus-count {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    // A FLOOR held with no base would pass a lowered figure, a dropped
    // gate or a loosened RLIMIT, so leaving the base off takes a flag.
    if floor.is_some() && base_rev.is_none() && !no_base {
        eprintln!(
            "FAIL coverage floor: --floor with no --base-rev, so a lowered figure or a dropped gate would pass: give --base-rev REV, or --no-base for a local run or the change that adds the first FLOOR"
        );
        std::process::exit(1);
    }
    let root = root.unwrap_or_else(|| PathBuf::from("."));
    let config_file = config_file.unwrap_or_else(|| root.join("verus/count.toml"));
    let cfg = Config::read(&config_file).unwrap_or_else(|e| usage(e));
    if badge {
        let path = root.join(&cfg.floor.path);
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| usage(format!("{}: {e}", path.display())));
        let head = Figures::new(&cfg.floor.checked, &cfg.floor.path)
            .read(&text, &path.display().to_string());
        println!("{}", badge_line(&head.floor));
        return;
    }
    let base = base_rev.map(|rev| Base {
        cfg: check_config(&root, &config_file, &cfg, &rev),
        rev,
    });
    if stats.is_none()
        && let Some(b) = &base
    {
        rlimit_required(&root, b, &cfg);
    }
    let crates = crate_dirs(&root, &cfg).unwrap_or_else(|e| usage(e));
    let mut unshipped = Vec::new();
    let mut c = Counter {
        verus_only_ghost: cfg.crates.verus_only_ghost,
        ..Counter::new(
            list,
            Atoms::of(&cfg),
            cfg.reach.as_ref().and_then(|r| r.fn_ptr_macro.clone()),
        )
    };
    // Test-only features' items are left out as test-only (cfg_needs_test):
    // no build of the workspace may enable one, as cargo resolves the
    // features.
    if !cfg.crates.test_features.is_empty() {
        let enablers = match features::test_util_enablers(&root, &cfg.crates.test_features) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("FAIL test-util: {e}");
                std::process::exit(2);
            }
        };
        for (p, f) in &enablers {
            eprintln!(
                "FAIL test-util: a build of the workspace compiles {p} with its `{f}` feature, which only tests' dev-dependencies may enable, so a shipped build could compile the items verus-count leaves out as test-only"
            );
        }
        if !enablers.is_empty() {
            std::process::exit(1);
        }
    }
    // The `#[test]` fns a ledger status's `callers:` clause may name.
    let mut tests = std::collections::BTreeSet::new();
    for Krate {
        name,
        dir: k,
        ships,
        ..
    } in &crates
    {
        c.krate = name.clone();
        c.tallies.entry(c.krate.clone()).or_default();
        if !ships {
            unshipped.push(c.krate.clone());
        }
        c.walk_crate(&cfg, (&root, k), cfg.ledger.is_some().then_some(&mut tests));
    }
    c.finish();
    if !opted_in(&crates, &verus_crates(&c)) {
        std::process::exit(1);
    }
    if cfg.ledger.is_some() && !c.discharged.is_empty() {
        for (file, name) in &c.discharged {
            eprintln!(
                "FAIL discharged: {}: `{name}` has the discharged_by = \"kani\" marker, but with [ledger] configured the ledger, not the marker, says what is discharged",
                file.display()
            );
        }
        std::process::exit(1);
    }
    let records = cfg.kani.records.as_deref();
    let missing = unrecorded(
        &c.discharged,
        &records.map_or_else(Vec::new, |d| kani_records(&root.join(d))),
    );
    for (file, name) in &missing {
        eprintln!(
            "FAIL discharged: {}: `{name}` has the discharged_by = \"kani\" marker, but no {}/kani-*.md record names its file and quotes `fn {name}`",
            file.display(),
            records.unwrap_or("[kani] records")
        );
    }
    if !missing.is_empty() {
        std::process::exit(1);
    }
    if let Some(r) = &cfg.recursion {
        check_recursion(&root, &r.table, &c.fns, &unshipped);
    }
    if let Some(l) = &cfg.loops {
        check_loops(&root, &l.table, &c.fns, &unshipped);
    }
    let (tables, tables_named) = seam_tables(&root, &cfg.seams);
    let seams = reach::seam_ids(tables.iter().map(String::as_str));
    if let Some(i) = &cfg.inputs
        && !compile_inputs(&root, i, &c)
    {
        std::process::exit(1);
    }
    if let Some(r) = &cfg.reach
        && !entry_reach(
            &root,
            r,
            &cfg.crates.stand_ins,
            &c,
            &unshipped,
            (&seams, &tables_named),
            base.as_ref(),
        )
    {
        std::process::exit(1);
    }
    if let Some(m) = &cfg.mints {
        let names = mints::Names {
            mint: m.mint.clone(),
            token: m.token_trait.clone(),
            home: m.home.clone(),
            list: m.path.clone(),
            tables: tables_named.clone(),
            stand_ins: cfg.crates.stand_ins.clone(),
        };
        if !token_mints(&root, names, &crates, &seams) {
            std::process::exit(1);
        }
    }
    let (shipped, room) = report(&root, &cfg, &c, &unshipped, &tests);
    let mut failed = false;
    if let Some(f) = floor {
        let fig = Figures::new(&cfg.floor.checked, &cfg.floor.path);
        let head = fig.read(
            &fs::read_to_string(&f).unwrap_or_else(|e| usage(format!("{}: {e}", f.display()))),
            &f.display().to_string(),
        );
        // The base's FLOOR is read at the base configuration's `[floor]
        // path`, so that path must be the file a head run reads: else one
        // change could point it at a file no run holds, and every later
        // run would compare with that file, or with none. Nor may the path
        // pass through a symlink, as the configuration may not
        // (`check_config`): once a link merged, the base's copy would be
        // the link's text.
        if let Some(link) = symlink_in(&root, Path::new(&cfg.floor.path)) {
            usage(format!(
                "FAIL coverage floor: [floor] path {} passes through {}, a symlink, so a later base's copy would be the link's text: give `path` the file itself",
                cfg.floor.path,
                link.display()
            ));
        }
        match (
            fs::canonicalize(&f),
            fs::canonicalize(root.join(&cfg.floor.path)),
        ) {
            (Ok(a), Ok(b)) if a == b => {}
            _ => usage(format!(
                "FAIL coverage floor: --floor {} is not {}'s [floor] path {}: give `path` the file the run holds",
                f.display(),
                root.display(),
                cfg.floor.path
            )),
        }
        if let Some(name) = fig.missing(&head.floor) {
            usage(format!("{}: want a {name} figure", f.display()));
        }
        let (mut ok, report) = fig.check_with_room(&shipped, &head.floor, &room);
        print!("{report}");
        if let Some(b) = &cfg.badge {
            ok &= check_badge(&root, &b.path, &cfg.floor.path, &head.floor);
        }
        if let Some(b) = &base {
            let rev = &b.rev;
            match b.copy(&root, &cfg.floor.path, GateFile::Floor) {
                Ok((path, Some(text))) => {
                    let base = fig
                        .for_base(cfg.floor.renamed_from.as_deref())
                        .read(&text, &format!("{rev}:{path}"));
                    let (lowering_ok, report) = fig.lowering(&head, &base);
                    print!("{report}");
                    ok &= lowering_ok;
                }
                Ok((path, None)) => {
                    println!("ok coverage floor: {rev} has no {path} to compare with")
                }
                Err(e) => {
                    eprintln!("FAIL coverage floor: {e}");
                    std::process::exit(2);
                }
            }
        }
        failed |= !ok;
    }
    if let Some(s) = stats {
        failed |= !check_rlimit(
            &root,
            &cfg,
            &crates,
            &s,
            &verus_crates(&c),
            &c,
            base.as_ref(),
        );
    }
    if failed {
        std::process::exit(1);
    }
}

/// Fails a run with no `--rlimit` (exit 1) whose base revision has the
/// RLIMIT file, at the path the base's configuration gives it: the base
/// held each verified function to verus/RLIMIT, and a run without the
/// statistics holds none, so dropping `--rlimit` (`VERUS_RLIMIT =` in a
/// port's Makefile) would turn the gate off with every check green. A
/// base that does not resolve fails the run (exit 2), as the floor's
/// comparison does. A run with `--no-base` has no base and is not held.
fn rlimit_required(root: &Path, base: &Base, cfg: &Config) {
    let path = base.cfg.as_ref().unwrap_or(cfg).rlimit.path.as_str();
    match base_file(root, &base.rev, path) {
        Ok(None) => {}
        Ok(Some(_)) => {
            eprintln!(
                "FAIL rlimit headroom: {}:{path} holds each verified function to its rlimit share, but this run has no --rlimit, so nothing is held to it: give --rlimit STATS (make verus with VERUS_RLIMIT non-empty, its default)",
                base.rev
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("FAIL rlimit headroom: {e}");
            std::process::exit(2);
        }
    }
}

/// The crates with code Verus checks, as Verus spells their names: each
/// with a function the walk recorded for Verus ([`Counter::verus_fns`]),
/// its proofs built only for Verus (`verus_keep_ghost`) too, which no
/// coverage tally counts, and each whose tally counts a function inside
/// `verus!`.
fn verus_crates(c: &Counter) -> Vec<String> {
    c.verus_fns
        .keys()
        .filter_map(|n| n.split("::").next().map(str::to_owned))
        .chain(
            c.tallies
                .iter()
                .filter(|(_, t)| {
                    t.ghost + t.fns.iter().sum::<usize>() > t.fns[Class::Outside as usize]
                })
                .map(|(k, _)| k.replace('-', "_")),
        )
        .collect::<std::collections::BTreeSet<String>>()
        .into_iter()
        .collect()
}

/// Whether the package in `dir` opts into verification: its Cargo.toml
/// sets `[package.metadata.verus] verify = true`. cargo verus has Verus
/// check only such a package, whatever `-p` selects (cargo-verus's
/// `make_cargo_plan` sets `__VERUS_DRIVER_VERIFY_` for no other), so
/// another one's `verus!` code is compiled and never checked. A manifest
/// that cannot be read or parsed opts in to nothing.
fn opts_in(dir: &Path) -> bool {
    fs::read_to_string(dir.join("Cargo.toml"))
        .ok()
        .and_then(|t| toml::from_str::<toml::Table>(&t).ok())
        .and_then(|m| {
            m.get("package")?
                .get("metadata")?
                .get("verus")?
                .get("verify")?
                .as_bool()
        })
        == Some(true)
}

/// Whether each of `crates` with code Verus checks (`verus`, as
/// [`verus_crates`] spells them) opts into verification ([`opts_in`]),
/// with a line for each that does not: Verus never checks its code, so
/// the tally would credit its functions as verified with no proof behind
/// them.
fn opted_in(crates: &[Krate], verus: &[String]) -> bool {
    let mut ok = true;
    for k in crates {
        if verus.contains(&k.name.replace('-', "_")) && !opts_in(&k.dir) {
            eprintln!(
                "FAIL verification opt-in: {} has code Verus checks, but its Cargo.toml does not set `[package.metadata.verus] verify = true`, so cargo verus never verifies it; add that table",
                k.dir.display()
            );
            ok = false;
        }
    }
    ok
}

/// Holds the configuration to `rev`'s copy ([`Config::explain`]): exits 1
/// when it removes or narrows a gate the base runs with no new
/// `[[narrowed]]` line naming that change, or has a new line that names
/// no narrowing, and 2 when the base's copy does not parse. A crate whose
/// Cargo.toml gained `publish = false` ([`newly_unpublished`]) is such a
/// narrowing too. Each named
/// narrowing is printed with its reason, so review sees it. The base's copy is read at the
/// configuration's path under `root`, as the repository names it, or at
/// `verus/count.toml` when the configuration lies outside `root`. A
/// configuration under `root` reached through a symlink exits 2: the
/// base would be read at the link's path while the head is read through
/// it, and once a link merged, the base's copy would be the link's text.
/// A base with no copy at that path is held to the one configuration
/// file of its tree, and one with none is the change that adopts the
/// counter, with nothing to compare, unless it has a FLOOR
/// ([`base_config`]). The base's copy, when there is one, for
/// [`Base::copy`].
fn check_config(root: &Path, file: &Path, cfg: &Config, rev: &str) -> Option<Config> {
    let under = std::path::absolute(file)
        .ok()
        .zip(std::path::absolute(root).ok())
        .and_then(|(f, r)| f.strip_prefix(r).ok().map(Path::to_path_buf));
    if let Some(link) = under.as_deref().and_then(|u| symlink_in(root, u)) {
        eprintln!(
            "FAIL configuration: {} is a symlink, so {rev}'s copy cannot be compared with the head's: give the configuration as a file",
            link.display()
        );
        std::process::exit(2);
    }
    let in_root = under.is_some();
    let rel = under.unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG));
    let rel = rel.to_string_lossy();
    let (base_rel, text) = base_config(root, rev, (&rel, in_root), cfg).unwrap_or_else(|e| {
        eprintln!("FAIL configuration: {e}");
        std::process::exit(2);
    })?;
    let base = Config::parse(&text).unwrap_or_else(|e| {
        eprintln!("FAIL configuration: {rev}:{base_rel}: {e}");
        std::process::exit(2);
    });
    let rev_rel = format!("{rev}:{base_rel}");
    let crate_changes = base_dir_narrowed(root, cfg, &base, rev)
        .and_then(|mut v| {
            v.extend(newly_unpublished(root, cfg, &base, rev)?);
            Ok(v)
        })
        .unwrap_or_else(|e| {
            eprintln!("FAIL configuration: {e}");
            std::process::exit(2);
        });
    let explained = cfg.explain(&base, crate_changes);
    for (n, reason) in &explained.named {
        println!("ok configuration: {rel}: {n}, with a reason: {reason}");
    }
    for n in &explained.unnamed {
        eprintln!(
            "FAIL configuration: {rel}: {n}, where {rev_rel} runs it: restore it, or give it a new [[narrowed]] line with change = {:?} and a reason",
            n.change
        );
    }
    for change in &explained.stale {
        eprintln!(
            "FAIL configuration: {rel}: the [[narrowed]] line for {change:?} names no change from {rev_rel}: remove it, or name the change as this run prints it"
        );
    }
    if !explained.unnamed.is_empty() || !explained.stale.is_empty() {
        std::process::exit(1);
    }
    if explained.named.is_empty() {
        println!("ok configuration: {rel} keeps every gate {rev_rel} runs");
    } else {
        println!("ok configuration: {rel} keeps every other gate {rev_rel} runs");
    }
    Some(base)
}

/// The configuration path the counter reads when `--config` is not given.
const DEFAULT_CONFIG: &str = "verus/count.toml";

/// `rev`'s configuration to hold the head's to ([`check_config`]), with
/// the path it was read at. The head's configuration is held to `rev`'s
/// copy at `rel` (the head's path under ROOT, or [`DEFAULT_CONFIG`] for
/// one outside ROOT, `in_root` false), or, when `rev` has none there, to
/// the one file of its tree that is a configuration ([`base_configs`],
/// [`DEFAULT_CONFIG`] among them); with several, which one `rev` ran is
/// unknown, and the run exits 1 naming them. One outside ROOT with no
/// base configuration is compared with nothing.
///
/// Under ROOT, `None` when `rev` has no configuration and no FLOOR (at the head's
/// `[floor] path` or `verus/FLOOR`): the change that adopts the counter,
/// with nothing to compare. A `rev` with a FLOOR but no configuration
/// ran its gates from a file this run cannot find, so the run exits 1
/// rather than compare with nothing: otherwise a head that moved its
/// configuration with `--config` would narrow every gate, and move
/// FLOOR, REACH and RLIMIT, unchecked.
fn base_config(
    root: &Path,
    rev: &str,
    (rel, in_root): (&str, bool),
    cfg: &Config,
) -> Result<Option<(String, String)>, String> {
    if let Some(text) = base_file(root, rev, rel)? {
        return Ok(Some((rel.to_string(), text)));
    }
    // The head's configuration as the messages name it: one outside ROOT
    // has no path the base could share.
    let head = if in_root {
        rel.to_string()
    } else {
        "the configuration outside ROOT".to_string()
    };
    let found = base_configs(root, rev)?;
    match found.as_slice() {
        [] => {}
        [(path, text)] => {
            println!(
                "ok configuration: {rev} has no {rel}, so {head} is held to {rev}:{path}, the one configuration {rev} has"
            );
            return Ok(Some((path.clone(), text.clone())));
        }
        several => {
            let paths: Vec<&str> = several.iter().map(|(p, _)| p.as_str()).collect();
            eprintln!(
                "FAIL configuration: {rev} has no {rel}, and {} configuration files ({}), so which one it ran is unknown: keep the configuration at the path {rev} reads it from in this change, and move it in a change of its own",
                paths.len(),
                paths.join(", ")
            );
            std::process::exit(1);
        }
    }
    // A configuration outside ROOT with no base configuration to hold it
    // to is compared with nothing, FLOOR or not: a port may generate its
    // configuration outside the tree, and the base cannot hold it.
    if !in_root {
        println!("ok configuration: {rev} has no {rel} to compare with");
        return Ok(None);
    }
    let mut floors = vec![cfg.floor.path.as_str()];
    if cfg.floor.path != "verus/FLOOR" {
        floors.push("verus/FLOOR");
    }
    for floor in floors {
        if base_file(root, rev, floor)?.is_some() {
            eprintln!(
                "FAIL configuration: {rev} has {floor} but no configuration, so the gates it ran cannot be compared with {rel}'s: a port whose FLOOR predates its configuration adds {rel} in a change that runs the verus check with VERUS_BASE=none, as the change that adds the first FLOOR does"
            );
            std::process::exit(1);
        }
    }
    let shown = if rel == DEFAULT_CONFIG {
        rel.to_string()
    } else {
        format!("{rel} or other configuration")
    };
    println!("ok configuration: {rev} has no {shown} to compare with");
    Ok(None)
}

/// The configuration files of `rev`'s tree, as `(path, text)` in path
/// order: each `.toml` file but a `Cargo.toml` with a top-level `crates`
/// key, which every configuration has and other `.toml` files
/// (`rustfmt.toml`, `.cargo/config.toml`) do not. `git grep` lists the
/// candidates in one call. One that [`Config::parse`] refuses (a key
/// this release dropped or renamed, say), any candidate that is not TOML,
/// and one `git grep` lists that cannot be read back, are errors naming
/// it (exit 2), as the base's copy at the head's path is, rather than be
/// passed over.
fn base_configs(root: &Path, rev: &str) -> Result<Vec<(String, String)>, String> {
    let commit = base_commit(root, rev, "its configuration")?;
    let listed = git_in(
        root,
        &[
            "grep",
            "-l",
            "-z",
            "-F",
            "crates",
            &commit,
            "--",
            "*.toml",
            ":(exclude,glob)**/Cargo.toml",
        ],
    )?;
    // git grep exits 1 when nothing matches. Its exit status says only
    // whether some file matched: a blob it cannot read is reported on
    // stderr and treated as no match, so any stderr fails the run too,
    // rather than leave the list empty or short.
    let failed = !listed.status.success() && listed.status.code() != Some(1);
    if failed || !listed.stderr.is_empty() {
        return Err(format!(
            "git grep crates {commit}: {}",
            String::from_utf8_lossy(&listed.stderr).trim()
        ));
    }
    // -z gives each path verbatim, NUL-terminated, where Git would quote
    // one with unusual characters. One the counter cannot read back
    // exactly (not UTF-8, or not under the commit) fails, rather than be
    // passed over as no configuration.
    let mut found = Vec::new();
    let prefix = format!("{commit}:");
    for entry in listed.stdout.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let path = std::str::from_utf8(entry)
            .ok()
            .and_then(|e| e.strip_prefix(&prefix))
            .ok_or_else(|| {
                format!(
                    "git grep crates {commit}: {:?} is not a UTF-8 path under the commit, so whether it is {rev}'s configuration cannot be read",
                    String::from_utf8_lossy(entry)
                )
            })?;
        // git grep lists paths relative to ROOT, as `base_file` reads
        // them; one that still reads back as nothing fails, rather than be
        // passed over.
        let text = base_file(root, rev, path)?.ok_or_else(|| {
            format!(
                "git grep crates {commit}: {path} is listed but cannot be read back from {rev}, so whether it is {rev}'s configuration cannot be read"
            )
        })?;
        // A candidate that is not TOML may be a configuration this run
        // cannot read, so it fails as one that does not parse.
        let table = text
            .parse::<toml::Table>()
            .map_err(|e| format!("{rev}:{path}: {e}"))?;
        if !table.contains_key("crates") {
            continue;
        }
        Config::parse(&text).map_err(|e| format!("{rev}:{path}: {e}"))?;
        found.push((path.to_string(), text));
    }
    found.sort();
    Ok(found)
}

/// The crates that leave the shipped tally because their own Cargo.toml
/// gained `publish = false` since `rev`: each crate the head reads whose
/// Cargo.toml decides that it ships and says it does not, where `base`
/// read the same crate by that rule too, at the same directory or under
/// the same name (a member moved, or a crate of `dir` made a member), and
/// `rev`'s Cargo.toml there does not say so. The configuration compares
/// the rest: a crate the head no longer reads, or whose `ships` it sets,
/// is a narrowing of [`Config::narrowed_from`]'s or [`base_dir_narrowed`]'s.
/// A crate `rev` lacks under its name and at its directory is new, with
/// nothing to compare. Each is a narrowing the `[[narrowed]]` lines name.
fn newly_unpublished(
    root: &Path,
    cfg: &Config,
    base: &Config,
    rev: &str,
) -> Result<Vec<config::Narrowing>, String> {
    // A head whose crates do not read fails the run later, with the reason.
    let Ok(crates) = crate_dirs(root, cfg) else {
        return Ok(Vec::new());
    };
    let in_base_dir = match &base.crates.dir {
        Some(d) => base_dir_crates(root, rev, d)?,
        None => Vec::new(),
    };
    let mut out = Vec::new();
    for k in crates.iter().filter(|k| k.by_rule && !k.ships) {
        let rel = crate_path(root, k);
        // Where the base read this crate by the rule: at its directory, or
        // under its name.
        let mut at = Vec::new();
        if base.reads_by_rule(&rel) {
            at.push(rel.clone());
        }
        match base.crates.members.iter().find(|m| m.name == k.name) {
            Some(m) if m.ships.is_none() => at.push(m.path.clone()),
            Some(_) => {}
            None => {
                if let Some(d) = &base.crates.dir
                    && in_base_dir.contains(&k.name)
                {
                    at.push(format!("{d}/{}", k.name));
                }
            }
        }
        for p in at {
            let manifest = config::normal(&p).join("Cargo.toml");
            if let Some(text) = base_file(root, rev, &manifest.to_string_lossy())?
                && !unpublished(&text)
            {
                out.push(config::Narrowing {
                    change: format!(
                        "[crates] crate {:?} at {rel:?} gains publish = false",
                        k.name
                    ),
                    effect: "so it no longer counts as shipped".into(),
                });
                break;
            }
        }
    }
    Ok(out)
}

/// The crates `rev` read through its `[crates] dir` that the head no
/// longer reads as it did: with the same `dir`, a crate the head reads
/// neither under its name nor at its directory (`is no longer read`, as a
/// member dropped is), or reads under its name somewhere else (`moves`, as
/// a member moved does); and, whatever the head's `dir`, one the head reads
/// under its name as a member with `ships` set to anything but `true`
/// where the configuration compares nothing (a member at a path `base`
/// neither names nor reads through `dir`). The configuration alone cannot
/// see these: it names `dir`, not the crates in it. So a `dir` crate
/// renamed is the old crate no longer read and a new one, as a member
/// renamed to a new path is.
fn base_dir_narrowed(
    root: &Path,
    cfg: &Config,
    base: &Config,
    rev: &str,
) -> Result<Vec<config::Narrowing>, String> {
    let Some(d) = &base.crates.dir else {
        return Ok(Vec::new());
    };
    let Ok(crates) = crate_dirs(root, cfg) else {
        return Ok(Vec::new());
    };
    let same_dir = cfg.crates.dir.as_ref().map(|h| config::normal(h)) == Some(config::normal(d));
    let mut out = Vec::new();
    for name in base_dir_crates(root, rev, d)? {
        let was = config::normal(d).join(&name);
        let was = was.to_string_lossy();
        if crates.iter().any(|k| crate_path(root, k) == was) {
            continue;
        }
        let Some(k) = crates.iter().find(|k| k.name == name) else {
            if same_dir {
                out.push(config::Narrowing {
                    change: format!("[crates] crate {name:?} at {was:?} is no longer read"),
                    effect: "so it is no longer read".into(),
                });
            }
            continue;
        };
        let now = crate_path(root, k);
        if same_dir {
            out.push(config::Narrowing {
                change: format!("[crates] crate {name:?} moves from {was:?} to {now:?}"),
                effect: "so the crate it read may no longer be read".into(),
            });
        }
        let compared = base
            .crates
            .members
            .iter()
            .any(|m| config::normal(&m.path) == config::normal(&now))
            || base.reads_by_rule(&now);
        if !k.by_rule && !k.ships && !compared {
            out.push(config::Narrowing {
                change: format!(
                    "[crates] member {name:?} at {now:?} changes ships from unset to false"
                ),
                effect: "so the crate the base read there may no longer count as shipped".into(),
            });
        }
    }
    Ok(out)
}

/// `k`'s directory relative to `root`, as the narrowings name it: `.` for
/// the root.
fn crate_path(root: &Path, k: &Krate) -> String {
    let rel = k.dir.strip_prefix(root).unwrap_or(&k.dir);
    let rel = config::normal(&rel.to_string_lossy());
    if rel.as_os_str().is_empty() {
        ".".into()
    } else {
        rel.to_string_lossy().into_owned()
    }
}

/// The first of `rel`'s paths under `root` (`verus`, then
/// `verus/count.toml` or `verus/FLOOR`) that is a symlink.
fn symlink_in(root: &Path, rel: &Path) -> Option<PathBuf> {
    let mut at = PathBuf::new();
    rel.components().find_map(|c| {
        at.push(c);
        fs::symlink_metadata(root.join(&at))
            .is_ok_and(|m| m.file_type().is_symlink())
            .then(|| at.clone())
    })
}

/// The first path at or below `rel` under `root`, in path order, that is
/// a symlink the source walk ([`rust_files`]) reads through: `rel` itself
/// (a crate's `src/`), or below it one named `*.rs`, or one to a directory
/// that holds a `.rs` file or a cycle at any depth (the walk follows
/// directory links). Any other link below `rel` (a `LICENSE`, a `locales`
/// directory of `.ftl` files) is a file the walk never reads (#71).
fn symlink_below(root: &Path, rel: &Path) -> Option<PathBuf> {
    let meta = fs::symlink_metadata(root.join(rel)).ok()?;
    if meta.file_type().is_symlink() {
        return Some(rel.to_path_buf());
    }
    read_link_under(root, rel, &meta)
}

/// The first link under `rel` (a directory, `meta` its metadata) that the
/// source walk reads through, as [`symlink_below`] defines it.
fn read_link_under(root: &Path, rel: &Path, meta: &fs::Metadata) -> Option<PathBuf> {
    if !meta.is_dir() {
        return None;
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(root.join(rel))
        .ok()?
        .flatten()
        .map(|e| rel.join(e.file_name()))
        .collect();
    entries.sort();
    entries.iter().find_map(|e| {
        let meta = fs::symlink_metadata(root.join(e)).ok()?;
        if !meta.file_type().is_symlink() {
            return read_link_under(root, e, &meta);
        }
        let read = e.extension().is_some_and(|x| x == "rs")
            || (dir_or_unreadable(&root.join(e))
                && reaches_rs(&root.join(e), &mut std::collections::HashSet::new()));
        read.then(|| e.clone())
    })
}

/// Whether [`rust_files`] would read a `.rs` file under `dir`, following
/// links as it does, or come back to a directory it is already in (a
/// cycle, which the walk would follow without end). `seen` holds the
/// canonical directories on the way down. A directory it cannot resolve
/// or list counts as one that does, so a read error refuses the link
/// rather than pass it unread.
fn reaches_rs(dir: &Path, seen: &mut std::collections::HashSet<PathBuf>) -> bool {
    let Ok(canon) = fs::canonicalize(dir) else {
        return true;
    };
    if !seen.insert(canon.clone()) {
        return true;
    }
    let found = match fs::read_dir(dir) {
        Err(_) => true,
        Ok(entries) => entries.into_iter().any(|e| {
            let Ok(e) = e else {
                return true;
            };
            let p = e.path();
            if dir_or_unreadable(&p) {
                reaches_rs(&p, seen)
            } else {
                p.extension().is_some_and(|x| x == "rs")
            }
        }),
    };
    seen.remove(&canon);
    found
}

/// Whether `p`, followed through links, is a directory, or cannot be
/// stat'd for a reason other than its absence (a link loop, a permission
/// or I/O error), which [`reaches_rs`] then refuses. A dangling link is
/// neither: the walk reads nothing through it.
fn dir_or_unreadable(p: &Path) -> bool {
    match fs::metadata(p) {
        Ok(m) => m.is_dir(),
        Err(e) => e.kind() != std::io::ErrorKind::NotFound,
    }
}

/// The revision `--base-rev` names, and its copy of the configuration
/// when it has one ([`check_config`]).
struct Base {
    rev: String,
    cfg: Option<Config>,
}

/// A file a gate holds to its base revision's copy.
#[derive(Clone, Copy, Debug)]
enum GateFile {
    Floor,
    Rlimit,
    Reach,
}

impl Base {
    /// `rev`'s copy of `file`, and the path it was read at: the path
    /// `rev`'s configuration gives the file (`[floor] path`, which
    /// defaults, `[rlimit] path`, likewise, or `[reach] path`). So a
    /// change that moves FLOOR, RLIMIT or REACH, or points the key at a
    /// path the base lacks, is still compared with the base's copy. With
    /// no base configuration, or no `[reach]` in it, the path is `head`,
    /// the head's. A file whose section the base's configuration has but
    /// `rev` lacks is an error; any other absence is `None`, nothing to
    /// compare.
    fn copy(
        &self,
        root: &Path,
        head: &str,
        file: GateFile,
    ) -> Result<(String, Option<String>), String> {
        let rev = &self.rev;
        let (path, declared) = self
            .cfg
            .as_ref()
            .and_then(|c| match file {
                GateFile::Floor => Some((c.floor.path.as_str(), c.floor.declared)),
                GateFile::Rlimit => Some((c.rlimit.path.as_str(), c.rlimit.declared)),
                GateFile::Reach => c.reach.as_ref().map(|r| (r.path.as_str(), true)),
            })
            .unwrap_or((head, false));
        let text = base_file(root, rev, path)?;
        if text.is_none() && declared {
            return Err(format!(
                "{rev}'s configuration names {path}, but {rev} has no {path}, so there is no base copy to compare with"
            ));
        }
        Ok((path.to_owned(), text))
    }
}

/// The recursion check (`[recursion]`, the PCRE2 port's verus/theorems/RN1.md):
/// exits 1 on a cycle through a function Verus does not check, or through
/// a verified one that `table`'s table does not name ([`documented`]).
fn check_recursion(root: &Path, table: &str, fns: &[FnRef], unshipped: &[String]) {
    let rn1 = fs::read_to_string(root.join(table)).unwrap_or_default();
    let (verified, cycles): (Vec<_>, Vec<_>) = recursive(fns, unshipped)
        .into_iter()
        .partition(|(i, _)| fns[*i].class == Class::Verified);
    let undocumented: Vec<_> = verified
        .iter()
        .filter(|(i, _)| !documented(&rn1, &fns[*i]))
        .collect();
    for (i, path) in &undocumented {
        let f = &fns[*i];
        eprintln!(
            "FAIL recursion: {}:{}: `{}` recurses ({}), but {table}'s table has no row naming its file and quoting `fn {}`",
            f.file.display(),
            f.line,
            f.name,
            path.join(" -> "),
            f.name
        );
    }
    for (i, path) in &cycles {
        let f = &fns[*i];
        eprintln!(
            "FAIL recursion: {}:{}: `{}` ({:?}) recurses: {} ({table})",
            f.file.display(),
            f.line,
            f.name,
            f.class,
            path.join(" -> ")
        );
    }
    if !cycles.is_empty() || !undocumented.is_empty() {
        std::process::exit(1);
    }
}

/// Prints the tables, fails the run on a file that did not parse, and
/// returns the `shipped` row's four figures. With the Kani marker (no
/// `[ledger]`), one table: each crate, `total` and `shipped`, the
/// discharged functions in a column of their own. With `[ledger]`, the
/// table without that column, then the ledger's combined table, each with
/// a row per crate and, for more than one crate, `total` and `shipped`;
/// a premise file, the ledger or a ledger entry it cannot read exits 2,
/// and a ledger error ([`ledger::match_ledger`]) exits 1.
fn report(
    root: &Path,
    cfg: &Config,
    c: &Counter,
    unshipped: &[String],
    tests: &std::collections::BTreeSet<String>,
) -> (Floor, Floor) {
    let room = (cfg.floor.room_functions, cfg.floor.room_lines);
    let mut total = Tally::default();
    let mut shipped = Tally::default();
    for (k, t) in &c.tallies {
        total.add(t);
        if !unshipped.contains(k) {
            shipped.add(t);
        }
    }
    let Some(l) = &cfg.ledger else {
        println!(
            "crate\tverified\ttrusted\tdischarged\toutside\tverified%\tchecked%\tlines verified\tlines trusted\tlines discharged\tlines outside\tlines verified%\tlines checked%\tghost"
        );
        for (k, t) in &c.tallies {
            print_row(k, t);
        }
        print_row("total", &total);
        print_row("shipped", &shipped);
        print_assumed(&c.assumed, c.assumed.len());
        fail_on_errors(c.errors);
        return (
            shares(&shipped, (0, 0), (0, 0)),
            shares(&shipped, (0, 0), room),
        );
    };
    let several = c.tallies.len() > 1;
    println!(
        "crate\tverified\ttrusted\toutside\tverified%\tlines verified\tlines trusted\tlines outside\tlines verified%\tghost"
    );
    for (k, t) in &c.tallies {
        print_ledger_row(k, t);
    }
    if several {
        print_ledger_row("total", &total);
        print_ledger_row("shipped", &shipped);
    }
    fail_on_errors(c.errors);
    let premises = ledger::Premises {
        isla: l
            .isla
            .as_ref()
            .map(|p| ledger::Labels::read(root, p).unwrap_or_else(|e| usage(e))),
        kani: l
            .kani
            .as_ref()
            .map(|p| ledger::Labels::read(root, p).unwrap_or_else(|e| usage(e))),
        tests: tests.clone(),
    };
    let seams = ledger::read_ledger(&root.join(&l.dir), &premises)
        .unwrap_or_else(|e| usage(format!("{}: {e}", l.dir)));
    let (checked, mut warnings, errors) = ledger::match_ledger(&c.trusted, &seams);
    let (assumed_seams, w) = ledger::match_assumed(&c.assumed, &seams);
    warnings.extend(w);
    for w in &warnings {
        eprintln!("warning: {w}");
    }
    println!(
        "crate\tverified+discharged\tdischarged\tpartly checked\tverified+discharged%\tlines verified+discharged\tlines discharged\tlines partly checked\tlines verified+discharged%"
    );
    let mut total_checked = ledger::Checked::default();
    let mut shipped_checked = ledger::Checked::default();
    for (k, t) in &c.tallies {
        let ch = checked.get(k).copied().unwrap_or_default();
        print_combined_row(k, t, &ch);
        total_checked.add(&ch);
        if !unshipped.contains(k) {
            shipped_checked.add(&ch);
        }
    }
    if several {
        print_combined_row("total", &total, &total_checked);
        print_combined_row("shipped", &shipped, &shipped_checked);
    }
    print_assumed(&c.assumed, assumed_seams);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("FAIL ledger: {e}");
        }
        std::process::exit(1);
    }
    (
        shares(&shipped, shipped_checked.discharged, (0, 0)),
        shares(&shipped, shipped_checked.discharged, room),
    )
}

/// The trust summary's linkage seams, after the tables, when there are
/// any: `seams` (the ledger's `unsupported-attribute` entries, or without
/// a ledger the functions themselves), then each function inside
/// `verus!` with `#[verifier::assume_unsupported_attributes]`, `fns`.
/// Each function keeps its class; only its `no_mangle`, `export_name` or
/// `link_section` is trusted, with the port's ABI tests as the evidence.
fn print_assumed(fns: &[String], seams: usize) {
    if seams == 0 && fns.is_empty() {
        return;
    }
    println!("unsupported-attribute seams (linkage trusted, bodies keep their class)\t{seams}");
    for loc in fns {
        println!("unsupported-attribute\t{loc}");
    }
}

/// One row of the ledger back end's first table: the classes Verus sees,
/// without the Kani marker's discharged column.
fn print_ledger_row(name: &str, t: &Tally) {
    let fns: usize = t.fns.iter().sum();
    let lines: usize = t.lines.iter().sum();
    let [v, tr, o] = [Class::Verified, Class::Trusted, Class::Outside].map(|c| c as usize);
    println!(
        "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        t.fns[v],
        t.fns[tr],
        t.fns[o],
        pct(t.fns[v], fns),
        t.lines[v],
        t.lines[tr],
        t.lines[o],
        pct(t.lines[v], lines),
        t.ghost
    );
}

/// One row of the ledger back end's combined table: verified and
/// discharged, discharged, partly checked, and the combined share.
fn print_combined_row(name: &str, t: &Tally, c: &ledger::Checked) {
    let fns: usize = t.fns.iter().sum();
    let lines: usize = t.lines.iter().sum();
    let v = Class::Verified as usize;
    let (vf, vl) = (t.fns[v] + c.discharged.0, t.lines[v] + c.discharged.1);
    println!(
        "{name}\t{vf}\t{}\t{}\t{}\t{vl}\t{}\t{}\t{}",
        c.discharged.0,
        c.partly.0,
        pct(vf, fns),
        c.discharged.1,
        c.partly.1,
        pct(vl, lines),
    );
}

/// The headroom check ([`rlimit`]): reads Verus's statistics from
/// `stats` and the RLIMIT file under `root`, prints each crate's results and
/// the report, and, with `base`, holds the RLIMIT file to that revision's
/// copy ([`Base::copy`]). A crate of `crates` (those with `verus!` code)
/// that the statistics leave out fails the run with `base`
/// ([`rlimit::check_covered`]).
/// `dirs` are the crates' directories, whose base sources [`base_limits`]
/// reads; `c` holds the head's limits and every function Verus checks
/// ([`Counter::verus_fns`]). Whether it passes.
fn check_rlimit(
    root: &Path,
    cfg: &Config,
    dirs: &[Krate],
    stats: &Path,
    crates: &[String],
    c: &Counter,
    base: Option<&Base>,
) -> bool {
    let limits = &c.limits;
    let list = cfg.rlimit.path.as_str();
    let fail = |e: String| -> ! {
        eprintln!("FAIL rlimit headroom: {e}");
        std::process::exit(2);
    };
    let text =
        fs::read_to_string(stats).unwrap_or_else(|e| fail(format!("{}: {e}", stats.display())));
    let stats =
        rlimit::read_stats(&text).unwrap_or_else(|e| fail(format!("{}: {e}", stats.display())));
    let file = root.join(list);
    let head = fs::read_to_string(&file)
        .map_err(|e| e.to_string())
        .and_then(|t| rlimit::parse_rlimit_file(&t))
        .unwrap_or_else(|e| fail(format!("{}: {e}", file.display())));
    for (k, s) in stats.iter().filter(|(k, _)| crates.contains(k)) {
        println!("{k}: {} verified, {} errors", s.verified, s.errors);
    }
    let (covered, report) = rlimit::check_covered(crates, &stats, base.is_some());
    print!("{report}");
    let shares = rlimit::shares(&stats, crates, limits)
        .unwrap_or_else(|e| fail(e.join("\nFAIL rlimit headroom: ")));
    let (mut ok, report) = rlimit::check(&shares, &head, 15, list);
    ok &= covered;
    print!("{report}");
    if let Some(b) = base {
        let rev = b.rev.as_str();
        let base = match b.copy(root, list, GateFile::Rlimit) {
            Ok((path, Some(text))) => {
                let base = rlimit::parse_rlimit_file(&text)
                    .unwrap_or_else(|e| fail(format!("{rev}:{path}: {e}")));
                let (kept, report) = rlimit::check_ratchet(&head, &base, list);
                print!("{report}");
                ok &= kept;
                base
            }
            Ok((path, None)) => {
                println!("ok rlimit headroom: {rev} has no {path} to compare with");
                rlimit::RlimitFile::default()
            }
            Err(e) => fail(e),
        };
        let was = base_limits(root, cfg, dirs, rev).unwrap_or_else(|e| fail(e));
        let (kept, report) = rlimit::check_raised(limits, &c.verus_fns, &was, &head, &base, list);
        print!("{report}");
        ok &= kept;
    }
    ok
}

/// The `#[verifier::rlimit]` limits of `rev`'s sources of the crates in
/// `dirs` (name and directory, under `root`), read from `rev`'s copy of
/// each crate's files by the walk the working tree's are read with
/// ([`Counter::walk_crate`]): test-only modules left out, and the crate's
/// `macro_rules!` collected and expanded, so that a limit a macro writes,
/// with its definition in any file, is the base's limit too.
fn base_limits(
    root: &Path,
    cfg: &Config,
    dirs: &[Krate],
    rev: &str,
) -> Result<Vec<rlimit::Limit>, String> {
    // Each crate's `src/`, relative to the root, as git names paths.
    let srcs: Vec<(String, PathBuf)> = dirs
        .iter()
        .map(|k| {
            let rel: PathBuf = k
                .dir
                .strip_prefix(root)
                .unwrap_or(&k.dir)
                .join("src")
                .components()
                .filter(|c| !matches!(c, std::path::Component::CurDir))
                .collect();
            (k.name.clone(), rel)
        })
        .collect();
    let git = |args: &[&str]| git_in(root, args);
    let mut c = Counter::new(
        false,
        Atoms::of(cfg),
        cfg.reach.as_ref().and_then(|r| r.fn_ptr_macro.clone()),
    );
    // The revision's copy of each crate (its sources and its Cargo.toml,
    // which names its roots) is written to a scratch directory and read by
    // the walk the working tree is read with.
    let scratch = std::env::temp_dir().join(format!(
        "verus-count-base-{}-{}",
        std::process::id(),
        rev.replace(|ch: char| !ch.is_ascii_alphanumeric(), "_")
    ));
    let _ = fs::remove_dir_all(&scratch);
    let read = (|| -> Result<(), String> {
        // The revision's root Cargo.toml, whose `[workspace]` tables a
        // crate inherits its edition and dependencies from
        // ([`crate_manifest`]), when the revision has one.
        let shown = git(&["show", &in_rev(rev, "Cargo.toml")])?;
        if shown.status.success() {
            fs::create_dir_all(&scratch).map_err(|e| format!("{}: {e}", scratch.display()))?;
            fs::write(scratch.join("Cargo.toml"), &shown.stdout)
                .map_err(|e| format!("{}: {e}", scratch.display()))?;
        }
        for (krate, src) in &srcs {
            let kdir = src.parent().unwrap_or(Path::new("")).to_path_buf();
            // -z gives each path verbatim; ls-tree lists them relative to
            // ROOT, as [`in_rev`] reads them.
            let listed = git(&[
                "ls-tree",
                "-r",
                "-z",
                "--name-only",
                rev,
                "--",
                &src.display().to_string(),
                &kdir.join("Cargo.toml").display().to_string(),
            ])?;
            if !listed.status.success() {
                return Err(format!(
                    "git ls-tree {rev}: {}",
                    String::from_utf8_lossy(&listed.stderr).trim()
                ));
            }
            for entry in listed.stdout.split(|b| *b == 0).filter(|e| !e.is_empty()) {
                let path = std::str::from_utf8(entry).map_err(|_| {
                    format!(
                        "git ls-tree {rev}: {:?} is not a UTF-8 path, so the base's crate cannot be read",
                        String::from_utf8_lossy(entry)
                    )
                })?;
                if !path.ends_with(".rs") && !path.ends_with("Cargo.toml") {
                    continue;
                }
                let shown = git(&["show", &in_rev(rev, path)])?;
                if !shown.status.success() {
                    return Err(format!(
                        "git show {rev}:{path}: {}",
                        String::from_utf8_lossy(&shown.stderr).trim()
                    ));
                }
                let to = scratch.join(path);
                if let Some(d) = to.parent() {
                    fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
                }
                fs::write(&to, &shown.stdout).map_err(|e| format!("{}: {e}", to.display()))?;
            }
            fs::create_dir_all(scratch.join(src)).map_err(|e| format!("{e}"))?;
            c.krate = krate.clone();
            c.walk_crate(cfg, (&scratch, &scratch.join(&kdir)), None);
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&scratch);
    read?;
    if c.errors > 0 {
        return Err(format!(
            "{rev}: {} errors reading its sources for their rlimits",
            c.errors
        ));
    }
    Ok(c.limits)
}

/// The figures of `t`, as verus/FLOOR records them: the verified share of
/// functions and of lines, then the verified and discharged share of each,
/// with `ledger` the functions and lines the generated ledger discharges
/// besides the Kani marker's, and `room` that many more unverified
/// functions and lines (`[floor] room_functions` and `room_lines`, #100)
/// in each denominator.
fn shares(t: &Tally, ledger: (usize, usize), room: (usize, usize)) -> Floor {
    let (fns, lines): (usize, usize) = (t.fns.iter().sum(), t.lines.iter().sum());
    let (fns, lines) = (fns + room.0, lines + room.1);
    let (v, d) = (Class::Verified as usize, Class::Discharged as usize);
    Floor([
        Some(tenths(t.fns[v], fns)),
        Some(tenths(t.lines[v], lines)),
        Some(tenths(t.fns[v] + t.fns[d] + ledger.0, fns)),
        Some(tenths(t.lines[v] + t.lines[d] + ledger.1, lines)),
    ])
}

/// One row of the table. `verified%` is the share Verus checks;
/// `checked%` adds the functions Kani discharges, which hold on the
/// bounded inputs their records name.
fn print_row(name: &str, t: &Tally) {
    let fns: usize = t.fns.iter().sum();
    let lines: usize = t.lines.iter().sum();
    let [v, tr, o, d] = [
        Class::Verified,
        Class::Trusted,
        Class::Outside,
        Class::Discharged,
    ]
    .map(|c| c as usize);
    println!(
        "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        t.fns[v],
        t.fns[tr],
        t.fns[d],
        t.fns[o],
        pct(t.fns[v], fns),
        pct(t.fns[v] + t.fns[d], fns),
        t.lines[v],
        t.lines[tr],
        t.lines[d],
        t.lines[o],
        pct(t.lines[v], lines),
        pct(t.lines[v] + t.lines[d], lines),
        t.ghost
    );
}

/// The object name of `path` in `rev`, with `path` relative to ROOT (the
/// directory `git -C` runs in): `REV:./PATH`, which Git resolves from
/// there, where `REV:PATH` resolves from the repository's top level and
/// so misses every file when ROOT is below it.
fn in_rev(rev: &str, path: &str) -> String {
    format!("{rev}:./{}", path.trim_start_matches("./"))
}

/// `REV:FILE` under `root`'s repository (FILE verus/FLOOR or
/// verus/RLIMIT), failing closed: a
/// revision that does not resolve to a commit (a shallow clone, another
/// uid's checkout git refuses, an empty name) is an error, and only a
/// resolved commit without the file is `None`. Whether the file exists
/// is read from the trees alone (`rev-parse --verify`), so a file whose
/// blob cannot be read is an error from `git show`, not `None`.
fn base_file(root: &Path, rev: &str, file: &str) -> Result<Option<String>, String> {
    let commit = base_commit(root, rev, file)?;
    let path = in_rev(&commit, file);
    if !git_in(root, &["rev-parse", "--verify", "--quiet", &path])?
        .status
        .success()
    {
        return Ok(None);
    }
    let shown = git_in(root, &["show", &path])?;
    if !shown.status.success() {
        return Err(format!(
            "git show {path}: {}",
            String::from_utf8_lossy(&shown.stderr).trim()
        ));
    }
    Ok(Some(String::from_utf8_lossy(&shown.stdout).into_owned()))
}

/// The crates `rev`'s tree holds under `dir`, as [`crate_dirs`] reads a
/// `dir` at the head: each subdirectory with a `src/` directory, by name,
/// in path order. A `dir` the tree lacks holds none.
fn base_dir_crates(root: &Path, rev: &str, dir: &str) -> Result<Vec<String>, String> {
    let commit = base_commit(root, rev, dir)?;
    let dir = config::normal(dir).to_string_lossy().into_owned();
    let tree = in_rev(&commit, &dir);
    if !git_in(root, &["cat-file", "-e", &tree])?.status.success() {
        return Ok(Vec::new());
    }
    // Run below the top level, ls-tree keeps only the entries under the
    // working directory's path, even of a tree named by `REV:./DIR`;
    // `--full-tree` lists the whole tree it is given.
    let listed = git_in(
        root,
        &["ls-tree", "--full-tree", "-z", "-d", "--name-only", &tree],
    )?;
    if !listed.status.success() {
        return Err(format!(
            "git ls-tree {tree}: {}",
            String::from_utf8_lossy(&listed.stderr).trim()
        ));
    }
    let mut out = Vec::new();
    for name in String::from_utf8_lossy(&listed.stdout)
        .split('\0')
        .filter(|n| !n.is_empty())
    {
        let src = Path::new(&dir).join(name).join("src");
        let kind = git_in(
            root,
            &["cat-file", "-t", &in_rev(&commit, &src.to_string_lossy())],
        )?;
        if kind.status.success() && String::from_utf8_lossy(&kind.stdout).trim() == "tree" {
            out.push(name.to_owned());
        }
    }
    Ok(out)
}

/// The commit `rev` names under `root`'s repository, failing closed as
/// [`base_file`] does, for reading `what` there.
fn base_commit(root: &Path, rev: &str, what: &str) -> Result<String, String> {
    let commit = git_in(
        root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{rev}^{{commit}}"),
        ],
    )?;
    if !commit.status.success() {
        return Err(format!(
            "base revision {rev:?} does not resolve to a commit, so {what} cannot be compared with it"
        ));
    }
    Ok(String::from_utf8_lossy(&commit.stdout).trim().to_owned())
}

/// `git ARGS` in `root`'s repository, whoever owns the checkout.
fn git_in(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["-c", "safe.directory=*"])
        // These change how git reads every pathspec the counter gives it
        // (`*.toml` taken literally, `*` no longer matching `/`), so a
        // search would find nothing and compare nothing (#117).
        .env_remove("GIT_LITERAL_PATHSPECS")
        .env_remove("GIT_GLOB_PATHSPECS")
        .env_remove("GIT_NOGLOB_PATHSPECS")
        .env_remove("GIT_ICASE_PATHSPECS")
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))
}

/// The figures the FLOOR file holds, and the file: the verified share of
/// functions and of their lines (`functions`, `lines`), then the share
/// verified or discharged of each, named with the configured prefix
/// (`checked-functions` and `checked-lines`, or challoc's `combined-`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Figures {
    names: [String; 4],
    file: String,
    /// The names a base revision's copy may still give the last two
    /// figures under (`[floor] renamed_from`), read as those figures.
    renamed: Option<[String; 2]>,
}

impl Default for Figures {
    fn default() -> Self {
        let c = config::Floor::default();
        Figures::new(&c.checked, &c.path)
    }
}

/// What each figure counts, for the report.
const MEANING: [&str; 4] = [
    "verified",
    "verified",
    "verified or discharged",
    "verified or discharged",
];

/// The figures every FLOOR file gives, the first of [`Figures::names`].
/// The others are required of the file under test (`main`) but may be
/// missing from a base revision's copy, which is then not compared on
/// them.
const REQUIRED: usize = 2;

/// Shares in tenths of a percent, rounded down (84 is 8.4%), by
/// [`Figures::names`]' index; `None` where a file gives none.
#[derive(Debug, PartialEq, Eq)]
struct Floor([Option<u32>; 4]);

fn tenths(a: usize, b: usize) -> u32 {
    (a * 1000)
        .checked_div(b)
        .map_or(0, |t| u32::try_from(t).unwrap_or(u32::MAX))
}

/// verus/FLOOR: the figures, and the `lowered` lines that give a reason
/// for each figure set below the base revision's.
#[derive(Debug, PartialEq, Eq)]
struct FloorFile {
    floor: Floor,
    lowered: Vec<(String, u32)>,
}

impl Figures {
    /// The figures with the verified-or-discharged ones named
    /// `{checked}-functions` and `{checked}-lines`, of the FLOOR file
    /// `file`.
    fn new(checked: &str, file: &str) -> Figures {
        Figures {
            names: [
                "functions".into(),
                "lines".into(),
                format!("{checked}-functions"),
                format!("{checked}-lines"),
            ],
            file: file.into(),
            renamed: None,
        }
    }

    /// These figures as a base revision's copy may give them: with
    /// `renamed_from`, its `{renamed_from}-functions` and
    /// `{renamed_from}-lines` are read as the checked figures, and a
    /// `lowered` line naming one as naming that figure.
    fn for_base(&self, renamed_from: Option<&str>) -> Figures {
        Figures {
            renamed: renamed_from.map(|p| [format!("{p}-functions"), format!("{p}-lines")]),
            ..self.clone()
        }
    }

    /// [`Figures::parse`], or exits naming `name` and the error.
    fn read(&self, text: &str, name: &str) -> FloorFile {
        self.parse(text).unwrap_or_else(|e| {
            eprintln!("{name}: {e}");
            std::process::exit(2);
        })
    }

    /// Reads the FLOOR file: `#` comments, one `FIGURE P` line per figure (P a
    /// percentage with at most one decimal; `functions` and `lines` required,
    /// `checked-functions` and `checked-lines` optional here), and any number
    /// of `lowered FIGURE P REASON` lines.
    fn parse(&self, text: &str) -> Result<FloorFile, String> {
        let mut floor = Floor([None; 4]);
        let mut lowered = Vec::new();
        for (i, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let words: Vec<&str> = line.split_whitespace().collect();
            let n = i + 1;
            let figure = |key: &str, value: &str| -> Result<(usize, u32), String> {
                let renamed = || {
                    let old = self.renamed.as_ref()?;
                    old.iter().position(|f| f == key).map(|j| REQUIRED + j)
                };
                let Some(k) = self.names.iter().position(|f| f == key).or_else(renamed) else {
                    return Err(format!("line {n}: unknown figure {key:?}"));
                };
                parse_tenths(value)
                    .map(|v| (k, v))
                    .ok_or_else(|| format!("line {n}: {value:?} is not P or P.D"))
            };
            match words.as_slice() {
                ["lowered", key, value, reason @ ..] => {
                    if reason.is_empty() {
                        return Err(format!("line {n}: a lowered line needs a reason"));
                    }
                    let (k, v) = figure(key, value)?;
                    lowered.push((self.names[k].clone(), v));
                }
                [key, value] => {
                    let (k, v) = figure(key, value)?;
                    if floor.0[k].replace(v).is_some() {
                        return Err(format!("line {n}: {key} given twice"));
                    }
                }
                _ => {
                    return Err(format!(
                        "line {n}: want `FIGURE P` (FIGURE one of {}) or `lowered FIGURE P REASON`",
                        self.names.join(", ")
                    ));
                }
            }
        }
        if floor.0[..REQUIRED].iter().any(Option::is_none) {
            return Err("want both a functions and a lines figure".into());
        }
        Ok(FloorFile { floor, lowered })
    }

    /// The first figure `f` does not give, if any: the file under test gives
    /// them all.
    fn missing(&self, f: &Floor) -> Option<&str> {
        f.0.iter()
            .position(Option::is_none)
            .map(|k| self.names[k].as_str())
    }

    /// Whether each figure in `head` is at least the base revision's, or is
    /// set lower with a `lowered FIGURE P REASON` line naming that figure and
    /// value that the base's file does not already have, so lowering the floor
    /// is never silent and an old reason never covers a new drop. A figure
    /// the base does not give is not compared.
    fn lowering(&self, head: &FloorFile, base: &FloorFile) -> (bool, String) {
        let file = &self.file;
        let mut ok = true;
        let mut report = String::new();
        for (k, name) in self.names.iter().enumerate() {
            let (Some(h), Some(b)) = (head.floor.0[k], base.floor.0[k]) else {
                continue;
            };
            if h >= b {
                continue;
            }
            let explained = head.lowered.iter().any(|(key, v)| {
                key == name && *v == h && !base.lowered.iter().any(|(bk, bv)| bk == key && bv == v)
            });
            if explained {
                report += &format!(
                    "ok coverage floor: {name} lowered from {}% to {}%, with a reason\n",
                    shown(b),
                    shown(h)
                );
            } else {
                ok = false;
                report += &format!(
                    "FAIL coverage floor: {file} lowers {name} from {}% to {}% with no `lowered {name} {} <reason>` line\n",
                    shown(b),
                    shown(h),
                    shown(h)
                );
            }
        }
        (ok, report)
    }

    /// Whether `got` meets `want`, with one line per figure `want` gives
    /// saying so. A figure above its floor names the value to record, so the
    /// floor only rises.
    #[cfg(test)]
    fn check(&self, got: &Floor, want: &Floor) -> (bool, String) {
        self.check_with_room(got, want, got)
    }

    /// [`Figures::check`], where the figure a raise records is `room`'s
    /// (the share with `[floor]`'s room for unverified code, #100) rather
    /// than `got`'s: a share above its floor names `room`'s figure to
    /// record where that is above the floor, and no raise where it is not.
    /// Whether the run passes is `got`'s alone.
    fn check_with_room(&self, got: &Floor, want: &Floor, room: &Floor) -> (bool, String) {
        let file = &self.file;
        let mut ok = true;
        let mut report = String::new();
        for (k, name) in self.names.iter().enumerate() {
            let (Some(g), Some(w)) = (got.0[k], want.0[k]) else {
                continue;
            };
            let r = room.0[k].unwrap_or(g);
            let what = MEANING[k];
            if g < w {
                ok = false;
                report += &format!(
                    "FAIL coverage floor: {name} {what} {}%, below the floor's {}%\n",
                    shown(g),
                    shown(w)
                );
            } else if g > w && r > w {
                report += &format!(
                    "ok coverage floor: {name} {what} {}%, above the floor's {}%; raise {file} to `{name} {}`\n",
                    shown(g),
                    shown(w),
                    shown(r)
                );
            } else if g > w {
                report += &format!(
                    "ok coverage floor: {name} {what} {}%, above the floor's {}% but within [floor]'s room for unverified code ({}%): no raise\n",
                    shown(g),
                    shown(w),
                    shown(r)
                );
            } else {
                report += &format!(
                    "ok coverage floor: {name} {what} {}%, at the floor\n",
                    shown(g)
                );
            }
        }
        (ok, report)
    }
}

/// The one coverage badge line FLOOR's `functions` and `lines` figures
/// give (`[badge]`, `--badge`): the figures with one decimal, in the alt
/// text and in the shields.io URL, linking to verus/README.md's
/// "Coverage". The repositories are private, so shields.io cannot read
/// the figures; they live in the line, and [`check_badge`] holds them to
/// FLOOR.
fn badge_line(floor: &Floor) -> String {
    let [functions, lines, ..] = floor.0.map(|v| shown(v.unwrap_or(0)));
    format!(
        "[![Verus: {functions}% of functions, {lines}% of lines](https://img.shields.io/badge/Verus-{functions}%25%20of%20functions%2C%20{lines}%25%20of%20lines-blue)](verus/README.md#coverage)"
    )
}

/// Whether `rendered`, the part of a line Markdown renders, holds a Verus
/// badge in any form: `verus` in any case, with `shields.io` or an image
/// (`![`, or an HTML `<img`, from any host). The match is wide on purpose:
/// a line it takes for a badge that is not one fails the gate, while a
/// second badge it missed would pass.
fn is_badge(rendered: &str) -> bool {
    let lower = rendered.to_lowercase();
    lower.contains("verus")
        && (lower.contains("shields.io") || lower.contains("![") || lower.contains("<img"))
}

/// The fence a line opens or closes: its character (`` ` `` or `~`) and
/// how many, after up to three spaces of indent. CommonMark refuses a
/// backtick fence whose info string holds a backtick, since that line is
/// inline code.
fn fence(line: &str) -> Option<(char, usize)> {
    let rest = line.trim_start_matches(' ');
    if line.len() - rest.len() > 3 {
        return None;
    }
    let c = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let n = rest.chars().take_while(|x| *x == c).count();
    (n >= 3 && !(c == '`' && rest[n..].contains('`'))).then_some((c, n))
}

/// Each line of `text`, numbered from 1, with the part Markdown renders,
/// in two readings ([`Rendered`]). In both, a line inside a fenced code
/// block renders as none, and the spans inside HTML comments (`<!--` to
/// `-->`, across lines; `<!-->` and `<!--->` are whole comments, as
/// CommonMark reads them) are dropped. The strict reading hides a fence
/// or a comment never closed to the end of the file. The lenient reading
/// reads its opener as text instead; reads as text a `<!--` in a backtick
/// code span, escaped (`\<!--`), on a line indented 4 or more spaces, or
/// whose container ends before its `-->` ([`Ends`]: an inline comment's
/// paragraph, a list item, a block quote); and ends a fence
/// indented by k > 0 spaces at the first non-blank line indented less (a
/// list item's fence ends with the item), since a renderer may read the
/// file either way.
fn rendered_lines(text: &str) -> Vec<Rendered<'_>> {
    let (strict, _) = rendered_scan(text, &[], false);
    // The openers (line index, byte offset) the lenient reading takes as
    // text, not as the start of a fence or comment, because it found them
    // unclosed.
    let mut ignored: Vec<(usize, usize)> = Vec::new();
    let lenient = loop {
        match rendered_scan(text, &ignored, true) {
            (out, None) => break out,
            (_, Some(opener)) => ignored.push(opener),
        }
    };
    strict
        .into_iter()
        .zip(lenient)
        .map(|((n, line, s), (_, _, l))| (n, line, s, l))
        .collect()
}

/// A line of [`rendered_lines`]: its number from 1, its text, and the
/// part of it Markdown renders in the strict and the lenient reading.
type Rendered<'t> = (usize, &'t str, String, String);

/// What [`rendered_scan`] gives: each line (its number, text and rendered
/// part), and the unclosed opener (line index, byte offset), if any.
type Scan<'t> = (Vec<(usize, &'t str, String)>, Option<(usize, usize)>);

/// The byte ranges of `line`'s backtick code spans: a run of n backticks
/// up to the next run of exactly n on the line.
fn code_spans(line: &str) -> Vec<(usize, usize)> {
    let b = line.as_bytes();
    let run = |at: usize| b[at..].iter().take_while(|c| **c == b'`').count();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'`' {
            i += 1;
            continue;
        }
        let n = run(i);
        let mut j = i + n;
        let mut end = None;
        while j < b.len() {
            if b[j] == b'`' {
                let m = run(j);
                if m == n {
                    end = Some(j + m);
                    break;
                }
                j += m;
            } else {
                j += 1;
            }
        }
        match end {
            Some(e) => {
                spans.push((i, e));
                i = e;
            }
            None => i += n,
        }
    }
    spans
}

/// What ends a comment's container before its `-->`, in the lenient
/// reading of [`rendered_scan`], by what precedes its `<!--` on its line.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Ends {
    /// `<!--` at the start of a line (after up to 3 spaces) opens an HTML
    /// block, which only its `-->` ends.
    Closer,
    /// After container markers (list markers and `>`, nested in any
    /// order), an HTML block in them, whose content starts at column
    /// `col`: a line with fewer `>` markers than the `quote` quotes among
    /// them ends the innermost ([`quote_depth`]), and a non-blank line
    /// indented less than `col` ends a list item among them (`item`), as
    /// an HTML block has no lazy continuation. An ordered marker that
    /// does not start at 1 cannot interrupt a paragraph, so the line may
    /// continue the one before it and the `<!--` be inline (`para`): the
    /// comment then ends where either reading ends it.
    Container {
        col: usize,
        quote: usize,
        item: bool,
        para: bool,
    },
    /// Anywhere else, an inline comment: a blank line or a line that
    /// starts a block ends its paragraph. The paragraph's text starts at
    /// column `col`, and its container's content starts there or before,
    /// so a block start indented up to `col + 3` columns ends it (more
    /// than CommonMark's 3 past the content column when the text is
    /// indented past it: a false failure, never a pass).
    Paragraph { col: usize },
}

impl Ends {
    /// The container of a `<!--` that `before` precedes on its line.
    fn of(before: &str) -> Ends {
        let (cols, at) = lead(before);
        if cols > 3 {
            return Ends::Paragraph { col: cols };
        }
        let mut rest = &before[at..];
        if rest.is_empty() {
            return Ends::Closer;
        }
        let (mut quote, mut item, mut para) = (0, false, false);
        loop {
            rest = rest.trim_start_matches([' ', '\t']);
            if let Some(r) = rest.strip_prefix('>') {
                quote += 1;
                rest = r;
            } else if let Some(n) = list_marker(rest) {
                item = true;
                let digits = &rest[..n - 2];
                para |= !digits.is_empty() && digits.parse::<u32>() != Ok(1);
                rest = &rest[n..];
            } else {
                break;
            }
        }
        if rest.is_empty() {
            Ends::Container {
                col: columns(before),
                quote,
                item,
                para,
            }
        } else {
            Ends::Paragraph {
                col: columns(&before[..before.len() - rest.len()]),
            }
        }
    }

    /// Whether `line`, after the opener's, ends the container.
    fn ends_at(self, line: &str) -> bool {
        let blank = line.trim().is_empty();
        let (indent, at) = lead(line);
        let rest = &line[at..];
        match self {
            Ends::Closer => false,
            Ends::Container {
                col,
                quote,
                item,
                para,
            } => {
                quote_depth(line) < quote
                    || (item && !blank && indent < col)
                    || (para && Ends::Paragraph { col }.ends_at(line))
            }
            Ends::Paragraph { col } => {
                blank
                    || (indent <= col + 3
                        && (rest.starts_with('#')
                            || rest.starts_with('>')
                            || fence(rest).is_some()
                            || list_marker(rest).is_some_and(|n| n < rest.len())
                            // An HTML block start: `<` and a tag name, `</`,
                            // `<?`, or `<!` (a declaration, CDATA or a
                            // comment), each of which can end a paragraph.
                            || rest
                                .strip_prefix('<')
                                .and_then(|r| r.chars().next())
                                .is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '/' | '?' | '!'))
                            || ['-', '*', '_', '='].iter().any(|c| {
                                rest.chars().all(|x| x == *c || x == ' ' || x == '\t')
                                    && rest.chars().filter(|x| x == c).count() >= 3
                            })))
            }
        }
    }
}

/// Whether what precedes a `<!--` on its line, `before`, puts it in an
/// indented code block: 4 or more columns of indent at the line's start,
/// or after a `>` and its one optional space, or after a list marker and
/// its space, so the `<!--` is code text, not a comment.
///
/// Columns count from the line's start, so a tab reaches the next
/// multiple of 4 from where it sits, and the optional space after a `>`
/// or a marker takes one column of a tab, leaving the rest as indent
/// (CommonMark's tab rule, `>\t\tfoo`).
fn in_code(before: &str) -> bool {
    let mut rest = before;
    // The column `rest` starts at, and the columns of a tab already
    // reached that count as indent (a tab the optional space took one
    // column of).
    let (mut col, mut spare) = (0, 0);
    loop {
        let at = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let end = columns_from(col, &rest[..at]);
        if spare + end - col >= 4 {
            return true;
        }
        (col, spare) = (end, 0);
        rest = &rest[at..];
        let n = if rest.starts_with('>') {
            1
        } else if let Some(n) = list_marker(rest) {
            n - 1
        } else {
            return false;
        };
        col += n;
        rest = &rest[n..];
        if let Some(r) = rest.strip_prefix(' ') {
            (col, rest) = (col + 1, r);
        } else if let Some(r) = rest.strip_prefix('\t') {
            let w = 4 - col % 4;
            (col, spare, rest) = (col + w, w - 1, r);
        }
    }
}

/// The column `text` ends at when it starts at column `col`, a tab
/// reaching the next multiple of 4.
fn columns_from(col: usize, text: &str) -> usize {
    text.chars()
        .fold(col, |c, x| if x == '\t' { c + 4 - c % 4 } else { c + 1 })
}

/// How many block quotes `line` continues: its `>` markers, each after
/// up to 3 columns of indent.
fn quote_depth(line: &str) -> usize {
    let mut rest = line;
    let mut depth = 0;
    loop {
        let (cols, at) = lead(rest);
        match rest[at..].strip_prefix('>') {
            Some(r) if cols <= 3 => {
                depth += 1;
                rest = r;
            }
            _ => return depth,
        }
    }
}

/// The columns `text` spans, a tab reaching the next multiple of 4, as
/// CommonMark counts indentation.
fn columns(text: &str) -> usize {
    columns_from(0, text)
}

/// The leading spaces and tabs of `line`: the columns they span and their
/// length in bytes.
fn lead(line: &str) -> (usize, usize) {
    let at = line.len() - line.trim_start_matches([' ', '\t']).len();
    (columns(&line[..at]), at)
}

/// The length of the list marker `rest` starts with (`-`, `*`, `+`, or
/// up to 9 digits and `.` or `)`), followed by a space or a tab, if it
/// has one.
fn list_marker(rest: &str) -> Option<usize> {
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let n = match rest.as_bytes().first()? {
        b'-' | b'*' | b'+' => 1,
        _ if (1..=9).contains(&digits)
            && matches!(rest.as_bytes().get(digits), Some(b'.' | b')')) =>
        {
            digits + 1
        }
        _ => return None,
    };
    rest[n..].starts_with([' ', '\t']).then_some(n + 1)
}

/// One pass of [`rendered_lines`], strict or `lenient`, with the openers
/// in `ignored` read as text: each line with the part of it that renders,
/// and the opener (line index, byte offset) of the fence or comment still
/// open at the end of `text`, if one is.
fn rendered_scan<'t>(text: &'t str, ignored: &[(usize, usize)], lenient: bool) -> Scan<'t> {
    // Indentation in columns, a tab to the next multiple of 4.
    let indent = |l: &str| lead(l).0;
    // An open fence: its character, length, indent and line index.
    let mut open: Option<(char, usize, usize, usize)> = None;
    // An open comment: its opener (line index, byte offset), and what ends
    // its container before its `-->` ([`Ends`]).
    let mut comment: Option<(usize, usize, Ends)> = None;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        // The lenient reading takes a comment's opener as text when its
        // container ends before its `-->`: a paragraph at a blank line or
        // a line that starts a block, a list item at a line indented less,
        // a block quote at a line without `>`.
        if lenient
            && let Some((ci, co, ends)) = comment
            && ends.ends_at(line)
        {
            return (out, Some((ci, co)));
        }
        if let Some((c, n, k, _)) = open {
            if lenient && k > 0 && !line.trim().is_empty() && indent(line) < k {
                open = None;
            } else {
                if fence(line)
                    .is_some_and(|(d, m)| d == c && m >= n && line.trim().chars().all(|x| x == c))
                {
                    open = None;
                }
                out.push((i + 1, line, String::new()));
                continue;
            }
        }
        if comment.is_none()
            && !ignored.contains(&(i, 0))
            && let Some((c, n)) = fence(line)
        {
            open = Some((c, n, indent(line), i));
            out.push((i + 1, line, String::new()));
            continue;
        }
        let spans = if lenient {
            code_spans(line)
        } else {
            Vec::new()
        };
        // The lenient reading also takes as text a `<!--` that renders as
        // text: in a backtick code span, escaped (`\<!--`), or in an
        // indented code block, at the line's start or inside a quote or a
        // list item ([`in_code`]).
        let text_at = |pos: usize| {
            ignored.contains(&(i, pos))
                || (lenient
                    && (spans.iter().any(|(s, e)| *s < pos && pos < *e)
                        || line[..pos].ends_with('\\')
                        || in_code(&line[..pos])))
        };
        let mut shown = String::new();
        let mut at = 0;
        loop {
            let rest = &line[at..];
            if comment.is_some() {
                match rest.find("-->") {
                    Some(end) => {
                        comment = None;
                        at += end + 3;
                    }
                    None => break,
                }
            } else {
                match rest.find("<!--") {
                    Some(start) if text_at(at + start) => {
                        shown += &rest[..start + 4];
                        at += start + 4;
                    }
                    Some(start) => {
                        shown += &rest[..start];
                        let after = &rest[start + 4..];
                        // `<!-->` and `<!--->` end where they start.
                        if let Some(close) = [">", "->"].iter().find(|c| after.starts_with(**c)) {
                            at += start + 4 + close.len();
                        } else {
                            comment = Some((i, at + start, Ends::of(&line[..at + start])));
                            at += start + 4;
                        }
                    }
                    None => {
                        shown += rest;
                        break;
                    }
                }
            }
        }
        out.push((i + 1, line, shown));
    }
    let unclosed = match (open, comment) {
        (Some((_, _, _, i)), _) => Some((i, 0)),
        (None, opener) => opener.map(|(i, at, _)| (i, at)),
    };
    (out, unclosed)
}

/// Holds `text`, the badge file `name`, to `want`, the line
/// [`badge_line`] gives for the FLOOR file `floor`: exactly one line holds
/// a Verus badge ([`is_badge`]) in either reading of [`rendered_lines`],
/// and it is `want`, rendered in both. The report line, or the failure.
fn badge_report(text: &str, name: &str, floor: &str, want: &str) -> Result<String, String> {
    let found: Vec<(usize, &str, bool)> = rendered_lines(text)
        .into_iter()
        .filter(|(_, _, s, l)| is_badge(s) || is_badge(l))
        .map(|(n, line, s, l)| (n, line, is_badge(&s) && is_badge(&l)))
        .collect();
    match found.as_slice() {
        [] => Err(format!(
            "FAIL badge: {name} has no Verus badge; add this line, which gives {floor}'s functions and lines figures:\n{want}"
        )),
        [(n, line, true)] if *line == want => Ok(format!(
            "ok badge: {name}:{n} gives {floor}'s functions and lines figures"
        )),
        [(n, line, false)] if *line == want => Err(format!(
            "FAIL badge: {name}:{n} is the badge but may not render: one reading of the file has it inside a fence or an HTML comment (one never closed, `<!--` in inline code, or a list item's fence); move it above that opener, keeping this line:\n{want}"
        )),
        [(n, _, _)] => Err(format!(
            "FAIL badge: {name}:{n} is not the badge {floor}'s functions and lines figures give; replace it with this line:\n{want}"
        )),
        _ => Err(format!(
            "FAIL badge: {name} has {} Verus badges (lines {}); keep one, this line:\n{want}",
            found.len(),
            found
                .iter()
                .map(|(n, _, _)| n.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The badge gate (`[badge]`, run with `--floor`): `path` under `root`
/// must hold the one badge line `floor_file`'s figures `floor` give
/// ([`badge_report`]), so the badge cannot drift from FLOOR. A `path` that
/// is not relative, has a `..`, or passes through a symlink exits 2, as a
/// `[floor] path` through one does: the base comparison of `[badge] path`
/// ([`Config::narrowed_from`]) goes by the path, which another spelling or
/// a link would slip past. Returns whether the badge passes.
fn check_badge(root: &Path, path: &str, floor_file: &str, floor: &Floor) -> bool {
    let rel = Path::new(path);
    if rel.components().any(|c| {
        !matches!(
            c,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    }) {
        usage(format!(
            "FAIL badge: [badge] path {path} is not a relative path without `..`: give the file under {}",
            root.display()
        ));
    }
    if let Some(link) = symlink_in(root, rel) {
        usage(format!(
            "FAIL badge: [badge] path {path} passes through {}, a symlink: give `path` the file itself",
            link.display()
        ));
    }
    let text = match fs::read_to_string(root.join(rel)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("FAIL badge: {path}: {e}");
            return false;
        }
    };
    match badge_report(&text, path, floor_file, &badge_line(floor)) {
        Ok(r) => {
            println!("{r}");
            true
        }
        Err(e) => {
            eprintln!("{e}");
            false
        }
    }
}

/// `P` or `P.D` as tenths of a percent, at most 100.0.
fn parse_tenths(s: &str) -> Option<u32> {
    parse_tenths_to(s, 1000)
}

/// `P` or `P.D` as tenths of a percent, at most `max` tenths.
fn parse_tenths_to(s: &str, max: u32) -> Option<u32> {
    let (whole, frac) = s.split_once('.').unwrap_or((s, "0"));
    if whole.is_empty()
        || frac.len() != 1
        || !whole
            .bytes()
            .chain(frac.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let v = whole
        .parse::<u32>()
        .ok()?
        .checked_mul(10)?
        .checked_add(frac.parse().ok()?)?;
    (v <= max).then_some(v)
}

/// The text of every `kani-*.md` record in `dir` (verus/seams).
fn kani_records(dir: &Path) -> Vec<String> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("kani-") && n.ends_with(".md"))
        })
        .filter_map(|p| fs::read_to_string(p).ok())
        .collect()
}

/// The discharged functions no Kani record covers. A record covers one
/// when it names the function's file in backquotes and quotes `fn NAME`
/// (followed by no further identifier character), as a record's contract
/// section does. The marker alone moves a function into the checked share,
/// so a marker without its record fails the run.
fn unrecorded(discharged: &[(PathBuf, String)], records: &[String]) -> Vec<(PathBuf, String)> {
    discharged
        .iter()
        .filter(|(file, name)| {
            let file = format!("`{}`", file.display());
            let quote = format!("fn {name}");
            !records.iter().any(|r| {
                r.contains(&file)
                    && r.match_indices(&quote).any(|(i, _)| {
                        !r[i + quote.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
                    })
            })
        })
        .cloned()
        .collect()
}

/// Appends to `out` each function `tokens` calls directly, at any depth of
/// nesting: `(false, name)` for `name(..)`, a free function of the file, and
/// `(true, name)` for `Self::name(..)` or `self.name(..)`, one of the same
/// `impl` or trait, each with or without a turbofish (`name::<T>(..)`). A call
/// through another path or receiver (`Vec::new()`, `self.v.get(..)`) names
/// some other function, so it is left out; so are macro calls and
/// declarations (`fn name(..)`).
/// The index after `toks[i]`, past a turbofish (`::<..>`, balanced, with
/// `->` inside it not closing it) when one follows.
fn past_turbofish(toks: &[TokenTree], i: usize) -> usize {
    let punct =
        |k: usize, c: char| matches!(toks.get(k), Some(TokenTree::Punct(p)) if p.as_char() == c);
    if !(punct(i + 1, ':') && punct(i + 2, ':') && punct(i + 3, '<')) {
        return i + 1;
    }
    let mut depth = 0usize;
    let mut k = i + 3;
    while k < toks.len() {
        if punct(k, '<') {
            depth += 1;
        } else if punct(k, '>') && !punct(k - 1, '-') {
            depth -= 1;
            if depth == 0 {
                return k + 1;
            }
        }
        k += 1;
    }
    i + 1
}

fn direct_calls(tokens: TokenStream, out: &mut Vec<(bool, String)>) {
    let toks: Vec<TokenTree> = tokens.into_iter().collect();
    for (i, t) in toks.iter().enumerate() {
        if let TokenTree::Group(g) = t {
            direct_calls(g.stream(), out);
            continue;
        }
        let TokenTree::Ident(id) = t else { continue };
        let paren = matches!(toks.get(past_turbofish(&toks, i)),
            Some(TokenTree::Group(g)) if g.delimiter() == proc_macro2::Delimiter::Parenthesis);
        if !paren {
            continue;
        }
        let before = |k: usize| i.checked_sub(k).and_then(|j| toks.get(j));
        let call = match before(1) {
            None => Some(false),
            Some(TokenTree::Punct(p)) if p.as_char() == '.' => {
                matches!(before(2), Some(TokenTree::Ident(r)) if r == "self").then_some(true)
            }
            Some(TokenTree::Punct(p)) if p.as_char() == ':' => {
                matches!(before(3), Some(TokenTree::Ident(r)) if r == "Self").then_some(true)
            }
            Some(TokenTree::Ident(k)) if k == "fn" => None,
            Some(_) => Some(false),
        };
        if let Some(method) = call {
            out.push((method, id.to_string()));
        }
    }
}

/// A closure `let` binds ([`LoopShape::closures`]): its name, whether
/// its body loops, and its direct calls ([`direct_calls`]).
type Closure = (String, bool, Vec<(bool, String)>);

/// The loops of a function's body, for the nested-loop check
/// ([`nested_loops`], #76).
#[derive(Debug, Default, PartialEq)]
struct LoopShape {
    /// Whether the body has a loop (`loop`, `while` or `for`) at all.
    loops: bool,
    /// The line of each loop inside another loop's body.
    nested: Vec<usize>,
    /// Each direct call ([`direct_calls`]) inside a loop's body (or a
    /// `while`'s condition), with the line of the outermost loop around it.
    in_loop: Vec<(bool, String, usize)>,
    /// Each name a `let` binds (through a type), with whether its
    /// initializer loops and the initializer's direct calls, so a closure
    /// is kept whatever wraps it (`Box::new(|..| ..)`, `&`, parentheses).
    /// One that loops, itself or through a call, makes a call of its name
    /// a call that loops ([`nested_loops`]).
    closures: Vec<Closure>,
    /// Whether the body did not parse as a block, so its loops are
    /// unknown: the check then names the function (fails closed).
    unread: bool,
    /// The line of each macro invocation that holds `loop`, `while` or
    /// `for`: its expansion's loop structure is unknown, so the check
    /// names the function (fails closed).
    macros: Vec<usize>,
}

/// The loops of `tokens`, a function's body ([`LoopShape`]), parsed as a
/// block (Verus's syntax included) and read by its structure: `loop`,
/// `while` and `for` expressions, whatever their headers hold (an
/// `unsafe` block, a `match`). A `while`'s condition runs each iteration
/// and is read as inside the loop; a `for`'s iterator runs once and is
/// not, but a closure in it (`.map(|b| ..)`) runs per item and is. A
/// nested item (a function, an `impl`) is its own and is left out. A
/// closure's loops are the body's, and each name a `let` binds is kept
/// with its initializer's loops and calls
/// ([`LoopShape::closures`]). A macro
/// invocation is tokens: one that holds `loop`, `while` or `for` counts
/// as a loop at its line. A body that does not parse is `unread`.
fn loop_shape(tokens: TokenStream) -> LoopShape {
    use verus_syn::__private::ToTokens;
    use verus_syn::visit::Visit;
    struct V {
        /// The line of the outermost loop the walk is inside.
        at: Option<usize>,
        /// While a `for`'s iterator is walked, that `for`'s line: a
        /// closure there runs per item.
        iter_of: Option<usize>,
        shape: LoopShape,
    }
    impl V {
        /// A loop at `line`, whose each iteration runs `each`.
        fn enter(&mut self, line: usize, each: &[TokenStream], walk: impl FnOnce(&mut V)) {
            self.shape.loops = true;
            if self.at.is_some() {
                self.shape.nested.push(line);
            }
            let at = self.at.unwrap_or(line);
            let mut calls = Vec::new();
            for t in each {
                direct_calls(t.clone(), &mut calls);
            }
            self.shape
                .in_loop
                .extend(calls.into_iter().map(|(m, n)| (m, n, at)));
            let was = self.at.replace(at);
            walk(self);
            self.at = was;
        }
    }
    /// Whether `tokens` hold a loop keyword, at any depth.
    fn has_loop_word(tokens: TokenStream) -> bool {
        tokens.into_iter().any(|t| match t {
            TokenTree::Ident(i) => matches!(i.to_string().as_str(), "loop" | "while" | "for"),
            TokenTree::Group(g) => has_loop_word(g.stream()),
            _ => false,
        })
    }
    impl<'a> Visit<'a> for V {
        fn visit_item(&mut self, _: &'a Item) {}
        fn visit_expr_while(&mut self, w: &'a verus_syn::ExprWhile) {
            let each = [w.cond.to_token_stream(), w.body.to_token_stream()];
            self.enter(w.while_token.span.start().line, &each, |v| {
                v.visit_expr(&w.cond);
                v.visit_block(&w.body);
            });
        }
        fn visit_expr_for_loop(&mut self, f: &'a verus_syn::ExprForLoop) {
            let was = self.iter_of.replace(f.for_token.span.start().line);
            self.visit_expr(&f.expr);
            self.iter_of = was;
            let each = [f.body.to_token_stream()];
            self.enter(f.for_token.span.start().line, &each, |v| {
                v.visit_block(&f.body)
            });
        }
        fn visit_expr_loop(&mut self, l: &'a verus_syn::ExprLoop) {
            let each = [l.body.to_token_stream()];
            self.enter(l.loop_token.span.start().line, &each, |v| {
                v.visit_block(&l.body)
            });
        }
        fn visit_expr_closure(&mut self, c: &'a verus_syn::ExprClosure) {
            // In a `for`'s iterator, a closure runs per item: its body is
            // inside that loop.
            match self.iter_of.take() {
                Some(line) => {
                    let each = [c.body.to_token_stream()];
                    self.enter(line, &each, |v| v.visit_expr(&c.body));
                    self.iter_of = Some(line);
                }
                None => verus_syn::visit::visit_expr_closure(self, c),
            }
        }
        fn visit_local(&mut self, l: &'a verus_syn::Local) {
            // The name bound, through a type (`let s: T = ..`).
            let mut pat = &l.pat;
            while let verus_syn::Pat::Type(t) = pat {
                pat = &t.pat;
            }
            // The whole initializer, whatever wraps a closure in it
            // (`Box::new(..)`, `Some(..)`, `&`, parentheses): a call of the
            // name loops where it loops or calls what loops. A name that is
            // never called costs nothing.
            if let (verus_syn::Pat::Ident(p), Some(init)) = (pat, &l.init) {
                let mut inner = V {
                    at: None,
                    iter_of: None,
                    shape: LoopShape::default(),
                };
                inner.visit_expr(&init.expr);
                let mut calls = Vec::new();
                direct_calls(init.expr.to_token_stream(), &mut calls);
                self.shape
                    .closures
                    .push((p.ident.to_string(), inner.shape.loops, calls));
            }
            verus_syn::visit::visit_local(self, l);
        }
        fn visit_macro(&mut self, m: &'a verus_syn::Macro) {
            if has_loop_word(m.tokens.clone()) {
                let line = m
                    .path
                    .segments
                    .first()
                    .map_or(0, |s| s.ident.span().start().line);
                self.enter(line, std::slice::from_ref(&m.tokens), |_| {});
                // What the expansion nests is not read: the function is
                // named (fails closed).
                self.shape.macros.push(line);
            }
        }
    }
    let block = TokenTree::Group(proc_macro2::Group::new(
        proc_macro2::Delimiter::Brace,
        tokens,
    ));
    let Ok(block) = verus_syn::parse2::<verus_syn::Block>(block.into()) else {
        return LoopShape {
            unread: true,
            ..LoopShape::default()
        };
    };
    let mut v = V {
        at: None,
        iter_of: None,
        shape: LoopShape::default(),
    };
    v.visit_block(&block);
    let mut shape = v.shape;
    shape.in_loop.sort();
    shape.in_loop.dedup();
    shape.nested.sort();
    shape.nested.dedup();
    shape
}

/// The nested-loop check (#76): each function of a shipped crate whose
/// work a loop's `decreases` does not bound, with why: a loop inside
/// another loop, or a call inside a loop to a function that loops, itself
/// or through what it calls (each loop's iterations are bounded, but not
/// the product). Calls are read as the recursion check reads them
/// ([`recursive`]): by name within a file, a free function for a plain
/// call and the function's own owner's for `self.f()` or `Self::f()`; a
/// call across files, or through another receiver or path, is not seen.
fn nested_loops(fns: &[FnRef], unshipped: &[String]) -> Vec<(usize, Vec<String>)> {
    use std::collections::BTreeSet;
    type Key<'a> = (&'a Path, &'a str, &'a str);
    let key = |f: &FnRef, method: bool, name: &str| -> (PathBuf, String, String) {
        (
            f.file.clone(),
            if method {
                f.owner.clone()
            } else {
                String::new()
            },
            name.to_owned(),
        )
    };
    // The functions that loop, themselves or through a call: a fixpoint
    // over the direct calls, by name.
    let mut looping: BTreeSet<Key> = fns
        .iter()
        .filter(|f| f.loops.loops || f.loops.unread)
        .map(|f| (f.file.as_path(), f.owner.as_str(), f.name.as_str()))
        .collect();
    loop {
        let before = looping.len();
        for f in fns {
            let me: Key = (&f.file, &f.owner, &f.name);
            if looping.contains(&me) {
                continue;
            }
            let calls_looping = f.calls.iter().any(|(m, c)| {
                looping.contains(&(
                    f.file.as_path(),
                    if *m { f.owner.as_str() } else { "" },
                    c.as_str(),
                ))
            });
            if calls_looping {
                looping.insert(me);
            }
        }
        if looping.len() == before {
            break;
        }
    }
    let mut found = Vec::new();
    for (i, f) in fns.iter().enumerate() {
        if unshipped.contains(&f.krate) {
            continue;
        }
        let mut why: Vec<String> = f
            .loops
            .nested
            .iter()
            .map(|l| format!("a loop at line {l} inside another"))
            .collect();
        if f.loops.unread {
            why.push("a body the check cannot parse".to_owned());
        }
        why.extend(f.loops.macros.iter().map(|l| {
            format!("a macro invocation at line {l} that holds a loop, whose nesting the check cannot read")
        }));
        // The function's `let` closures that loop, themselves, through a
        // call of a function that loops, or through another such closure.
        let mut closures: BTreeSet<&str> = BTreeSet::new();
        loop {
            let before = closures.len();
            for (name, loops, calls) in &f.loops.closures {
                let through = calls.iter().any(|(m, c)| {
                    (!*m && closures.contains(c.as_str()))
                        || looping.contains(&(
                            f.file.as_path(),
                            if *m { f.owner.as_str() } else { "" },
                            c.as_str(),
                        ))
                });
                if *loops || through {
                    closures.insert(name);
                }
            }
            if closures.len() == before {
                break;
            }
        }
        for (m, c, at) in &f.loops.in_loop {
            if !*m && closures.contains(c.as_str()) {
                why.push(format!(
                    "a call to the closure `{c}`, which loops, inside the loop at line {at}"
                ));
                continue;
            }
            let (file, owner, name) = key(f, *m, c);
            if looping.contains(&(file.as_path(), owner.as_str(), name.as_str())) {
                why.push(format!(
                    "a call to `{c}`, which loops, inside the loop at line {at}"
                ));
            }
        }
        if !why.is_empty() {
            found.push((i, why));
        }
    }
    found
}

/// The nested-loop check (`[loops]`, #76): exits 1 on each function
/// [`nested_loops`] finds that `table`'s table does not name
/// ([`documented`]), the work-bound theorem that bounds its product.
fn check_loops(root: &Path, table: &str, fns: &[FnRef], unshipped: &[String]) {
    let text = fs::read_to_string(root.join(table)).unwrap_or_default();
    let mut failed = false;
    for (i, why) in nested_loops(fns, unshipped) {
        let f = &fns[i];
        if documented(&text, f) {
            continue;
        }
        failed = true;
        eprintln!(
            "FAIL loops: {}:{}: `{}` has {}, so its loops' `decreases` measures do not bound its work, but {table}'s table has no row naming its file and quoting `fn {}`",
            f.file.display(),
            f.line,
            f.name,
            why.join("; "),
            f.name
        );
    }
    if failed {
        std::process::exit(1);
    }
}

/// The recursion check: each function whose name lies on a cycle of direct calls
/// ([`direct_calls`]) among the functions of its file, with the cycle, in the shipped crates. Verus
/// checks recursion through verified functions (each needs a `decreases`
/// whose bound verus/theorems/RN1.md states); the bodies it does not check
/// may not recurse at all (RN1). Functions are matched by name within a
/// file, which joins two methods of one name in one file: such a cycle is
/// reported too. Recursion through a call across files, or through another
/// receiver or path, is not seen (RN1).
fn recursive(fns: &[FnRef], unshipped: &[String]) -> Vec<(usize, Vec<String>)> {
    use std::collections::{BTreeSet, VecDeque};
    type Key<'a> = (&'a Path, &'a str, &'a str);
    let mut edges: BTreeMap<Key, BTreeSet<Key>> = BTreeMap::new();
    let mut names: BTreeSet<Key> = BTreeSet::new();
    for f in fns {
        names.insert((&f.file, &f.owner, &f.name));
    }
    for f in fns {
        let e = edges.entry((&f.file, &f.owner, &f.name)).or_default();
        for (method, c) in &f.calls {
            let to = (
                f.file.as_path(),
                if *method { f.owner.as_str() } else { "" },
                c.as_str(),
            );
            if names.contains(&to) {
                e.insert(to);
            }
        }
    }
    // The shortest cycle through `start`, by breadth-first search back to it.
    let cycle = |start: Key| -> Option<Vec<String>> {
        let mut parent: BTreeMap<Key, Key> = BTreeMap::new();
        let mut queue = VecDeque::new();
        for &n in edges.get(&start)? {
            if n == start {
                return Some(vec![start.2.to_string(), start.2.to_string()]);
            }
            if let std::collections::btree_map::Entry::Vacant(e) = parent.entry(n) {
                e.insert(start);
                queue.push_back(n);
            }
        }
        while let Some(n) = queue.pop_front() {
            for &m in edges.get(&n).into_iter().flatten() {
                if m == start {
                    let mut back = Vec::new();
                    let mut at = n;
                    while at != start {
                        back.push(at.2.to_string());
                        at = parent[&at];
                    }
                    back.push(start.2.to_string());
                    back.reverse();
                    back.push(start.2.to_string());
                    return Some(back);
                }
                if m != start
                    && let std::collections::btree_map::Entry::Vacant(e) = parent.entry(m)
                {
                    e.insert(n);
                    queue.push_back(m);
                }
            }
        }
        None
    };
    let mut found = Vec::new();
    for (i, f) in fns.iter().enumerate() {
        if unshipped.contains(&f.krate) {
            continue;
        }
        if let Some(c) = cycle((&f.file, &f.owner, &f.name)) {
            found.push((i, c));
        }
    }
    found
}

/// The functions of `fns` as the walks ([`inputs`]) read them.
fn funcs(fns: &[FnRef]) -> Vec<inputs::Func<'_>> {
    fns.iter()
        .map(|f| inputs::Func {
            krate: &f.krate,
            file: &f.file,
            line: f.line,
            owner: inputs::base(&f.owner),
            name: &f.name,
            params: f.params,
            receiver: f.receiver,
            body: &f.body,
        })
        .collect()
}

/// The compile-inputs check (`[inputs]`, the PCRE2 port's verus/theorems/CD1.md,
/// [`inputs`]): every hidden input a function reachable from the
/// configured roots reads is in the table `i` names, and every row of the
/// table is still read. Prints what it finds; `false` when it fails.
fn compile_inputs(root: &Path, i: &config::Inputs, c: &Counter) -> bool {
    let cd1 = fs::read_to_string(root.join(&i.table)).unwrap_or_default();
    let table = &i.table;
    let short = Path::new(table)
        .file_name()
        .map_or(table.clone(), |n| n.to_string_lossy().into_owned());
    let describe = &i.describe;
    let roots = inputs::Roots {
        crates: i.crates.iter().map(String::as_str).collect(),
        roots: &i.roots,
        lazy: i.lazy,
    };
    let funcs = funcs(&c.fns);
    let found = inputs::findings(&funcs, &c.statics, &c.traits, &c.types, &c.aliases, &roots);
    let reached = inputs::reachable(&funcs, &c.traits, &c.aliases, &roots).len();
    let mut ok = true;
    let mut resolved = Vec::new();
    for (r, matched) in i.roots.iter().zip(inputs::resolved_roots(&funcs, &roots)) {
        let name = r
            .owner
            .as_ref()
            .map_or(r.name.clone(), |o| format!("{o}::{}", r.name));
        if matched.is_empty() {
            ok = false;
            eprintln!(
                "FAIL compile inputs: the root `{name}` in a file ending in {} matches no function of the crates {}, so the walk never starts from it; correct its file, owner or name",
                r.file,
                i.crates.join(", ")
            );
        }
        for m in matched {
            let f = &funcs[m];
            resolved.push(format!("{name} at {}:{}", f.file.display(), f.line));
        }
    }
    let mut allowed = 0;
    for x in &found {
        let f = &funcs[x.func];
        if inputs::allowed(&cd1, f, &x.input) {
            allowed += 1;
            continue;
        }
        ok = false;
        eprintln!(
            "FAIL compile inputs: {}:{}: `{}` reads `{}` and is reachable from {describe} ({}), but {table}'s table has no row naming the input, its file and `fn {}`",
            f.file.display(),
            f.line,
            f.name,
            x.input,
            x.chain.join(" -> "),
            f.name
        );
    }
    for row in inputs::stale_rows(&cd1, &funcs, &found) {
        ok = false;
        eprintln!("FAIL compile inputs: {table} allows a read no reachable function makes: {row}");
    }
    if ok {
        println!(
            "ok compile inputs: {reached} functions reachable from {describe} (roots {}); {allowed} reads of hidden inputs, each in {short}'s table",
            resolved.join(", ")
        );
    }
    ok
}

/// The functions of `crates` a C caller enters by ([`reach`]): each one
/// that exports its symbol (`#[no_mangle]` or `#[export_name]`, whose ABI
/// [`Counter::entry`] holds to C's) or whose address a `const` or
/// `static` hands out (`FnRef::entry`); each free function an exports
/// macro block (the PCRE2 port's `width::exports!`) in its file exports
/// (`exported`); and each `conversion` method, one-argument and without a
/// receiver (the PCRE2 port's `from_c`, the `width::FromC` conversions the exports'
/// wrappers apply to their arguments: the identity one, and those
/// `width::fn_ptr!` writes).
fn entry_roots(
    fns: &[FnRef],
    crates: &[&str],
    exported: &std::collections::BTreeSet<(PathBuf, String)>,
    conversion: Option<&str>,
) -> std::collections::BTreeSet<usize> {
    (0..fns.len())
        .filter(|&i| {
            let f = &fns[i];
            crates.contains(&f.krate.as_str())
                && (f.entry
                    || (f.owner.is_empty() && exported.contains(&(f.file.clone(), f.name.clone())))
                    || (conversion == Some(f.name.as_str())
                        && !f.owner.is_empty()
                        && !f.receiver
                        && f.params == 1))
        })
        .collect()
}

/// Each body an `exports!` block names (`exported`) that no free function
/// of `crates` in the block's file matches: the block's wrappers call a
/// body [`entry_roots`] cannot root, so it is an error rather than a root
/// that drops out (a body moved to another module and imported, say).
fn unmatched_exports(
    fns: &[FnRef],
    crates: &[&str],
    exported: &std::collections::BTreeSet<(PathBuf, String)>,
    mac: &str,
) -> Vec<String> {
    exported
        .iter()
        .filter(|(file, name)| {
            !fns.iter().any(|f| {
                crates.contains(&f.krate.as_str())
                    && f.owner.is_empty()
                    && &f.file == file
                    && &f.name == name
            })
        })
        .map(|(file, name)| {
            format!(
                "{}: an {mac}! block exports `{name}`, but the file defines no free function of that name, so it roots nothing; define the body in the block's file",
                file.display()
            )
        })
        .collect()
}

/// The entry-point reachability gate (`[reach]` `r`, [`reach`], verus/REACH):
/// every function of a shipped crate reachable from a C entry point is
/// verified, discharged, or listed in REACH with a reason, and every row
/// of REACH is such a function, citing seams of `seams` (their IDs, and
/// the tables as the report names them); with `base`, a row that
/// revision's REACH ([`Base::copy`]) does not have is marked `new`
/// ([`reach::check_added`]). Prints what it finds; `false` when it fails.
fn entry_reach(
    root: &Path,
    r: &config::Reach,
    stand_ins: &[[String; 2]],
    c: &Counter,
    unshipped: &[String],
    seams: (&std::collections::BTreeSet<String>, &str),
    base: Option<&Base>,
) -> bool {
    let (fns, traits, aliases) = (&c.fns, &c.traits, &c.aliases);
    let list = r.path.as_str();
    // A base revision's REACH is read at its `[reach] path`, so the file
    // must exist wherever the section names it, rows or none.
    let text = match fs::read_to_string(root.join(list)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "FAIL entry reach: {list}: {e}: [reach] names it, so it must exist, if only with no rows"
            );
            return false;
        }
    };
    let funcs = funcs(fns);
    let shipped: std::collections::BTreeSet<&str> = fns
        .iter()
        .map(|f| f.krate.as_str())
        .chain(c.files.iter().map(|(k, _)| k.as_str()))
        .filter(|k| !unshipped.iter().any(|u| u == k))
        .collect();
    let crates: Vec<&str> = shipped.into_iter().collect();
    // Every file a shipped build compiles, not only those holding a
    // function (an export block in a file of `use`s alone still exports),
    // and each that holds one.
    let files: std::collections::BTreeSet<PathBuf> = c
        .files
        .iter()
        .map(|(k, f)| (k, f))
        .chain(fns.iter().map(|f| (&f.krate, &f.file)))
        .filter(|(k, _)| crates.contains(&k.as_str()))
        .map(|(_, f)| f.clone())
        .collect();
    let (exported, unread) = match &r.exports_macro {
        Some(mac) => reach::exported(root, &files, mac, stand_ins),
        None => Default::default(),
    };
    let roots = entry_roots(fns, &crates, &exported, r.conversion.as_deref());
    let parent = inputs::walk(
        &funcs,
        traits,
        aliases,
        &crates,
        &|i| roots.contains(&i),
        false,
    );
    let at = |k: usize| {
        format!(
            "{} ({}:{})",
            funcs[k].name,
            funcs[k].file.display(),
            funcs[k].line
        )
    };
    let reached: Vec<reach::Reached> = parent
        .keys()
        .map(|&i| {
            let mut chain = vec![at(i)];
            let mut up = parent[&i];
            while let Some(p) = up {
                chain.push(at(p));
                up = parent[&p];
            }
            chain.reverse();
            reach::Reached {
                file: fns[i].file.display().to_string(),
                line: fns[i].line,
                key: reach::key(&funcs[i].owner, &fns[i].name),
                checked: matches!(fns[i].class, Class::Verified | Class::Discharged),
                chain,
            }
        })
        .collect();
    let head = reach::rows(&text);
    let mut report = reach::check(roots.len(), &reached, &head, list);
    let mac = r.exports_macro.as_deref().unwrap_or("exports");
    report.failures.extend(
        unread
            .into_iter()
            .map(|e| format!("cannot read the {mac}! blocks of {e}")),
    );
    report
        .failures
        .extend(unmatched_exports(fns, &crates, &exported, mac));
    report
        .failures
        .extend(reach::check_seams(&head, seams.0, list, seams.1));
    if let Some(b) = base {
        let rev = b.rev.as_str();
        match b.copy(root, list, GateFile::Reach) {
            Ok((_, Some(base))) => {
                report
                    .failures
                    .extend(reach::check_added(&head, &reach::rows(&base), rev, list))
            }
            Ok((path, None)) => println!("ok entry reach: {rev} has no {path} to compare with"),
            Err(e) => report.failures.push(e),
        }
    }
    for f in &report.failures {
        eprintln!("FAIL entry reach: {f}");
    }
    // The trusted functions of the walked crates no root reaches: the
    // rest of FLOOR's trusted count beside REACH's trusted rows.
    let unreached: Vec<&FnRef> = fns
        .iter()
        .enumerate()
        .filter(|(i, f)| {
            f.class == Class::Trusted
                && crates.contains(&f.krate.as_str())
                && !parent.contains_key(i)
        })
        .map(|(_, f)| f)
        .collect();
    if report.failures.is_empty() {
        println!(
            "ok entry reach: {} functions reachable from {} C entry points; {} verified or discharged, {} in {list}; {} trusted functions no entry point reaches",
            report.reached,
            report.roots,
            report.checked,
            report.listed,
            unreached.len()
        );
        for f in &unreached {
            println!(
                "  unreached trusted: {}:{} {}",
                f.file.display(),
                f.line,
                reach::key(&inputs::base(&f.owner), &f.name)
            );
        }
    }
    report.failures.is_empty()
}

/// The token-mint gate (`[mints]`, [`mints`]): reads every crate's
/// sources (`crates`, name and directory) and the MINTS file under
/// `root`, holds them to `names` and the seam IDs `seams`, and prints the
/// failures or the ok line. Fails closed: a missing MINTS file, a file it
/// cannot read or parse, or a row it cannot read fails it. Whether it
/// passes.
fn token_mints(
    root: &Path,
    names: mints::Names,
    crates: &[Krate],
    seams: &std::collections::BTreeSet<String>,
) -> bool {
    let list = names.list.clone();
    let mint = names.mint.clone();
    let text = match fs::read_to_string(root.join(&list)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("FAIL token mints: {list}: {e}");
            return false;
        }
    };
    let (rows, mut fails) = mints::rows(&text, &list);
    let mut scan = mints::Scan::new(names);
    for k in crates {
        let mut files = Vec::new();
        rust_files(&k.dir.join("src"), &mut files);
        for f in files {
            let rel = f.strip_prefix(root).unwrap_or(&f).display().to_string();
            match fs::read_to_string(&f) {
                Ok(text) => mints::scan_file(&rel, &text, &mut scan),
                Err(e) => fails.push(format!("{rel}: {e}")),
            }
        }
    }
    fails.extend(mints::check(&scan, &rows, seams));
    for f in &fails {
        eprintln!("FAIL token mints: {f}");
    }
    if fails.is_empty() {
        println!(
            "ok token mints: {} token kinds and {} mints, each a {list} row; {mint}, in {}, is the one place a C caller's token is made",
            scan.kinds.len(),
            scan.mints.len(),
            scan.defs.first().map(|(f, _)| f.as_str()).unwrap_or("?")
        );
    }
    fails.is_empty()
}

/// Whether RN1.md documents the verified recursive function `f`: a table
/// row that names its file in backquotes and quotes `fn NAME`.
fn documented(rn1: &str, f: &FnRef) -> bool {
    let file = format!("`{}`", f.file.display());
    let quote = format!("`fn {}`", f.name);
    rn1.lines()
        .any(|l| l.starts_with('|') && l.contains(&file) && l.contains(&quote))
}

fn fail_on_errors(errors: usize) {
    if errors > 0 {
        eprintln!("{errors} files or verus! blocks did not parse; the counts leave them out");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `cfg` atoms of the PCRE2 port's configuration, which the fixtures use:
    /// `test`, `kani` and the `test-util` feature test-only,
    /// `verus_keep_ghost` and the `verus` feature Verus-only, the `jit`
    /// feature sure in Verus's build and `miri` never set there.
    fn atoms() -> Atoms {
        Atoms {
            test: vec!["test".into(), "kani".into(), "feature=\"test-util\"".into()],
            verus: vec!["verus_keep_ghost".into(), "feature=\"verus\"".into()],
            sets: vec!["feature=\"jit\"".into()],
            never: vec!["miri".into()],
            runs: Vec::new(),
            one_run: false,
        }
    }

    /// challoc's two runs declared as `[[runs]]` (#21): x86_64 and
    /// aarch64, each never setting `loom`.
    fn two_runs() -> Atoms {
        Atoms::of(
            &Config::parse(
                "[crates]\ndir = \"crates\"\n\
                 [[runs]]\nname = \"verus\"\nsets = ['target_arch = \"x86_64\"']\nnever_sets = [\"loom\"]\n\
                 [[runs]]\nname = \"verus-arm64\"\nsets = ['target_arch = \"aarch64\"']\nnever_sets = [\"loom\"]\n",
            )
            .expect("parses"),
        )
    }

    /// Verus's attribute form under two declared runs (#21): it applies
    /// surely only where every run that may compile the function applies
    /// it. Unguarded, `cfg_attr(not(target_arch = "x86_64"), ..)` leaves
    /// the x86_64 run compiling the function without it: trusted. Under
    /// `target_arch = "aarch64"`, only the aarch64 run compiles it, and
    /// applies it: verified.
    #[test]
    fn per_run_attribute_form_applies_in_every_run_that_compiles() {
        let src = r#"
#[cfg_attr(not(target_arch = "x86_64"), verus_verify)]
fn unguarded() {}
#[cfg(target_arch = "aarch64")]
#[cfg_attr(not(target_arch = "x86_64"), verus_verify)]
fn on_arm() {}
#[cfg_attr(any(target_arch = "x86_64", target_arch = "aarch64"), verus_verify)]
fn both() {}
"#;
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from("lib.rs"),
            module: vec!["k".into()],
            ..Counter::new(false, two_runs(), None)
        };
        c.items(&parse_file(src).expect("parses").items, false);
        assert_eq!(c.errors, 0, "errors: got = {}, wanted = 0", c.errors);
        let t = c.tallies.remove("k").unwrap_or_default();
        let want = [2, 1, 0, 0];
        assert_eq!(t.fns, want, "fns: got = {:?}, wanted = {want:?}", t.fns);
    }

    /// A binding Verus's build may see under two declared runs (#21): one
    /// only the aarch64 run compiles is seen (the x86_64 run never sets
    /// its arch), and one no run compiles (`loom`) is not.
    #[test]
    fn per_run_bindings_are_seen_where_some_run_compiles() {
        let src = r#"
#[cfg(target_arch = "aarch64")]
use other::verus;
#[cfg(loom)]
use loom::verus as v;
use plain::verus as w;
"#;
        let at = BindAt {
            file: Path::new("lib.rs"),
            module: vec!["k".into()],
            guards: Vec::new(),
            shared: 0,
            scope: "lib.rs".into(),
            expansion: None,
        };
        let mut out = Vec::new();
        bindings_in(
            &parse_file(src).expect("parses").items,
            &at,
            &two_runs(),
            &mut out,
        );
        let got: Vec<(&str, bool)> = out.iter().map(|b| (b.name.as_str(), b.verus)).collect();
        let want = [("verus", true), ("v", false), ("w", true)];
        assert_eq!(got, want, "got = {got:?}, wanted = {want:?}");
    }

    /// #21's table: an item is classed in each run, verified where it is
    /// sure in some run and unsure in none, trusted where some run may
    /// ship it unchecked, and left out where no build compiles it.
    #[test]
    fn per_run_atoms_class_run_by_run() {
        let runs = two_runs();
        for (cfg, want) in [
            ("target_arch = \"x86_64\"", Built::Shipped),
            ("target_arch = \"aarch64\"", Built::Shipped),
            // Never in the x86_64 run, sure in the aarch64 one: verified
            // (trusted under the shorthand).
            ("not (target_arch = \"x86_64\")", Built::Shipped),
            (
                "all (target_arch = \"x86_64\" , any (target_arch = \"aarch64\" , unix))",
                Built::Unsure,
            ),
            (
                "all (target_arch = \"x86_64\" , target_arch = \"aarch64\")",
                Built::Test,
            ),
            // Sure in one run, never in the other: verified.
            (
                "any (target_arch = \"x86_64\" , target_arch = \"aarch64\")",
                Built::Shipped,
            ),
            // Sure in one run, unsure in the other: trusted.
            ("any (target_arch = \"x86_64\" , unix)", Built::Unsure),
            // Never in every run, yet a shipped build may: trusted.
            ("loom", Built::Unsure),
        ] {
            let got = cfg_built(cfg, &runs);
            assert_eq!(got, Ok(want), "{cfg}: got = {got:?}, wanted = {want:?}");
        }
        // In a module, each run that may compile the module must be sure
        // to compile the item there.
        let s = |v: &[&str]| v.iter().map(|p| (*p).to_owned()).collect::<Vec<_>>();
        for (shared, own, want) in [
            (&[][..], &["target_arch = \"x86_64\""][..], false),
            (
                &["target_arch = \"x86_64\""][..],
                &["target_arch = \"x86_64\""][..],
                true,
            ),
            (&["target_arch = \"x86_64\""][..], &["unix"][..], false),
            (
                &["target_arch = \"x86_64\""][..],
                &["not (target_arch = \"aarch64\")"][..],
                true,
            ),
            (
                &["not (loom)"][..],
                &["any (target_arch = \"x86_64\" , target_arch = \"aarch64\")"][..],
                true,
            ),
        ] {
            let got = every_run_sure(&s(shared), &s(own), &runs);
            assert_eq!(
                got,
                Ok(want),
                "{shared:?} {own:?}: got = {got:?}, wanted = {want}"
            );
        }
    }

    fn test_util() -> Vec<String> {
        vec!["test-util".into()]
    }

    /// A counter as the PCRE2 port's configuration sets one up, with its `fn_ptr`
    /// macro.
    fn counter() -> Counter {
        Counter::new(false, atoms(), Some("fn_ptr".into()))
    }

    /// The tally of one crate whose only file is `src`.
    fn count(src: &str) -> (Tally, usize) {
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from("lib.rs"),
            module: vec!["k".into()],
            ..counter()
        };
        c.items(&parse_file(src).expect("parses").items, false);
        (c.tallies.remove("k").unwrap_or_default(), c.errors)
    }

    /// The guard behind leaving `test-util` items out, through cargo's own
    /// resolution: a workspace member, shipped or not, that enables the
    /// feature outside its dev-dependencies fails the run, whatever the
    /// manifest's TOML form; one whose tests do does not.
    #[test]
    fn test_util_only_in_dev_dependencies() {
        const ROOT: &str =
            "[workspace]\nresolver = \"3\"\nmembers = [\"crates/lib\", \"crates/data\"]\n";
        const LIB: &str = "[package]\nname = \"lib\"\nedition = \"2024\"\n\n[features]\ntest-util = []\njit = []\n";
        const DATA: &str = "crates/data/Cargo.toml";
        let data = |body: &str| {
            format!("[package]\nname = \"data\"\nedition = \"2024\"\npublish = false\n\n{body}")
        };
        let rows: Vec<(Vec<(&str, String)>, bool)> = vec![
            // The tests' own sections, a member's own `test-util` feature that
            // forwards it, and a comment.
            (
                vec![(
                    DATA,
                    data(
                        "[dependencies]\nlib = { path = \"../lib\" }\n\n[dev-dependencies]\nlib = { path = \"../lib\", features = [\"test-util\"] }\n",
                    ),
                )],
                false,
            ),
            (
                vec![(DATA, data("[dev-dependencies.lib]\npath = '../lib'\nfeatures = ['test-util']\n"))],
                false,
            ),
            (
                vec![(
                    DATA,
                    data(
                        "[target.'cfg(unix)'.dev-dependencies]\nlib = { path = \"../lib\", features = [\"test-util\"] }\n",
                    ),
                )],
                false,
            ),
            (
                vec![(
                    DATA,
                    data(
                        "[dependencies]\nlib = { path = \"../lib\" }\n\n[features]\ntest-util = [\"lib/test-util\"]\n",
                    ),
                )],
                false,
            ),
            (
                vec![(DATA, data("[dependencies]\nlib = { path = \"../lib\" } # not \"test-util\"\n"))],
                false,
            ),
            // Normal and build dependencies, in every form.
            (
                vec![(
                    DATA,
                    data("[dependencies]\nlib = { path = \"../lib\", features = [\"test-util\"] }\n"),
                )],
                true,
            ),
            (
                vec![(DATA, data("[dependencies.lib]\npath = \"../lib\"\nfeatures = [\"test-util\"]\n"))],
                true,
            ),
            (
                vec![(
                    DATA,
                    data(
                        "[target.'cfg(windows)'.dependencies]\nlib = { path = \"../lib\", features = [\"test-util\"] }\n",
                    ),
                )],
                true,
            ),
            (
                vec![(
                    DATA,
                    data("[build-dependencies]\nlib = { path = \"../lib\", features = [\"test-util\"] }\n"),
                )],
                true,
            ),
            (
                vec![
                    (
                        "Cargo.toml",
                        format!(
                            "{ROOT}\n[workspace.dependencies]\nlib = {{ path = \"crates/lib\", features = [\"test-util\"] }}\n"
                        ),
                    ),
                    (DATA, data("[dependencies]\nlib.workspace = true\n")),
                ],
                true,
            ),
            // Features that forward it, on by default or not (found in
            // review: pcre2-capi's `hardened`, which docs/design.md, ci/abi.sh
            // and ci/consumers.sh build with, is not on by default), or name it
            // by default.
            (
                vec![(
                    DATA,
                    data(
                        "[dependencies]\nlib = { path = \"../lib\" }\n\n[features]\nx = ['lib/test-util']\n",
                    ),
                )],
                true,
            ),
            (
                vec![(
                    DATA,
                    data(
                        "[dependencies]\nlib = { path = \"../lib\" }\n\n[features]\nhardened = [\"lib/jit\", \"lib/test-util\"]\n",
                    ),
                )],
                true,
            ),
            (
                vec![(
                    DATA,
                    data(
                        "[dependencies]\nlib = { path = \"../lib\", optional = true }\n\n[features]\nx = [\"dep:lib\", \"lib?/test-util\"]\ndefault = [\"x\"]\n",
                    ),
                )],
                true,
            ),
            (
                vec![
                    ("crates/lib/Cargo.toml", format!("{LIB}default = [\n    \"jit\",\n    \"test-util\",\n]\n")),
                    (DATA, data("[dependencies]\nlib = { path = \"../lib\" }\n")),
                ],
                true,
            ),
            // The forms a manifest scanner missed (found in review): single
            // quotes, a `#` inside them, and a globbed member.
            (
                vec![(
                    DATA,
                    data("[dependencies]\nlib = { path = '../lib', features = ['test-util'] }\n"),
                )],
                true,
            ),
            (
                vec![(
                    DATA,
                    data(
                        "[dependencies]\nlib = { path = \"../lib\" }\n\n[features]\ndefault = ['lib/test-util']\n",
                    ),
                )],
                true,
            ),
            (
                vec![
                    (
                        "Cargo.toml",
                        "[workspace]\nresolver = \"3\"\nmembers = [\"crates/l#b\", \"crates/data\"]\n".to_string(),
                    ),
                    ("crates/l#b/Cargo.toml", LIB.to_string()),
                    (
                        DATA,
                        data("[dependencies]\nlib = { path = '../l#b', features = [\"test-util\"] }\n"),
                    ),
                ],
                true,
            ),
            (
                vec![
                    (
                        "Cargo.toml",
                        "[workspace]\nresolver = \"3\"\nmembers = [\"crates/lib\", \"extra/*\"]\n".to_string(),
                    ),
                    (
                        "extra/data/Cargo.toml",
                        data("[dependencies]\nlib = { path = \"../../crates/lib\", features = [\"test-util\"] }\n"),
                    ),
                ],
                true,
            ),
            // The forms the manifests' reading (#136) must follow as cargo
            // does: a renamed dependency, by its key in a feature, and
            // through the workspace's entry.
            (
                vec![(
                    DATA,
                    data(
                        "[dependencies]\nother = { path = \"../lib\", package = \"lib\", features = [\"test-util\"] }\n",
                    ),
                )],
                true,
            ),
            (
                vec![(
                    DATA,
                    data(
                        "[dependencies]\nother = { path = \"../lib\", package = \"lib\" }\n\n[features]\nx = [\"other/test-util\"]\n",
                    ),
                )],
                true,
            ),
            (
                vec![
                    (
                        "Cargo.toml",
                        format!(
                            "{ROOT}\n[workspace.dependencies]\nother = {{ path = \"crates/lib\", package = \"lib\" }}\n"
                        ),
                    ),
                    (DATA, data("[dependencies]\nother = { workspace = true, features = [\"test-util\"] }\n")),
                ],
                true,
            ),
            // A feature of a dev-dependency only, which only tests build.
            (
                vec![(
                    DATA,
                    data(
                        "[dev-dependencies]\nlib = { path = \"../lib\" }\n\n[features]\nx = [\"lib/test-util\"]\n",
                    ),
                )],
                false,
            ),
            // A path dependency that is no member: what it asks counts where
            // a build compiles it, a member's normal dependency, and not
            // where only tests do.
            (
                vec![
                    ("Cargo.toml", format!("{ROOT}exclude = [\"helper\"]\n")),
                    (DATA, data("[dependencies]\nhelper = { path = \"../../helper\" }\n")),
                    (
                        "helper/Cargo.toml",
                        "[package]\nname = \"helper\"\nedition = \"2024\"\n\n[dependencies]\nlib = { path = \"../crates/lib\", features = [\"test-util\"] }\n".to_string(),
                    ),
                ],
                true,
            ),
            (
                vec![
                    ("Cargo.toml", format!("{ROOT}exclude = [\"helper\"]\n")),
                    (DATA, data("[dev-dependencies]\nhelper = { path = \"../../helper\" }\n")),
                    (
                        "helper/Cargo.toml",
                        "[package]\nname = \"helper\"\nedition = \"2024\"\n\n[dependencies]\nlib = { path = \"../crates/lib\", features = [\"test-util\"] }\n".to_string(),
                    ),
                ],
                false,
            ),
            // A non-member's own feature that forwards it, which nothing
            // turns on, is off; one its dependent asks for is on.
            (
                vec![
                    ("Cargo.toml", format!("{ROOT}exclude = [\"helper\"]\n")),
                    (DATA, data("[dependencies]\nhelper = { path = \"../../helper\" }\n")),
                    (
                        "helper/Cargo.toml",
                        "[package]\nname = \"helper\"\nedition = \"2024\"\n\n[dependencies]\nlib = { path = \"../crates/lib\" }\n\n[features]\nx = [\"lib/test-util\"]\n".to_string(),
                    ),
                ],
                false,
            ),
            (
                vec![
                    ("Cargo.toml", format!("{ROOT}exclude = [\"helper\"]\n")),
                    (DATA, data("[dependencies]\nhelper = { path = \"../../helper\", features = [\"x\"] }\n")),
                    (
                        "helper/Cargo.toml",
                        "[package]\nname = \"helper\"\nedition = \"2024\"\n\n[dependencies]\nlib = { path = \"../crates/lib\" }\n\n[features]\nx = [\"lib/test-util\"]\n".to_string(),
                    ),
                ],
                true,
            ),
            // A path dependency inside the workspace's directory is a
            // member, so its own features are on, even only a dev-dependency.
            (
                vec![
                    (DATA, data("[dev-dependencies]\nhelper = { path = \"../helper\" }\n")),
                    (
                        "crates/helper/Cargo.toml",
                        "[package]\nname = \"helper\"\nedition = \"2024\"\n\n[dependencies]\nlib = { path = \"../lib\" }\n\n[features]\nx = [\"lib/test-util\"]\n".to_string(),
                    ),
                ],
                true,
            ),
            // An excluded directory is no member.
            (
                vec![
                    (
                        "Cargo.toml",
                        format!("{ROOT}exclude = [\"crates/helper\"]\n"),
                    ),
                    (DATA, data("[dependencies]\nhelper = { path = \"../helper\" }\n")),
                    (
                        "crates/helper/Cargo.toml",
                        "[package]\nname = \"helper\"\nedition = \"2024\"\n\n[dependencies]\nlib = { path = \"../lib\" }\n\n[features]\nx = [\"lib/test-util\"]\n".to_string(),
                    ),
                ],
                false,
            ),
            // Resolver 1 (named, or a virtual manifest's with none named)
            // unifies a member's dev-dependencies' features into its
            // normal build (found in review).
            (
                vec![
                    (
                        "Cargo.toml",
                        "[workspace]\nresolver = \"1\"\nmembers = [\"crates/lib\", \"crates/data\"]\n".to_string(),
                    ),
                    (
                        DATA,
                        data(
                            "[dependencies]\nlib = { path = \"../lib\" }\n\n[dev-dependencies]\nlib = { path = \"../lib\", features = [\"test-util\"] }\n",
                        ),
                    ),
                ],
                true,
            ),
            (
                vec![
                    (
                        "Cargo.toml",
                        "[workspace]\nmembers = [\"crates/lib\", \"crates/data\"]\n".to_string(),
                    ),
                    (
                        DATA,
                        data(
                            "[dependencies]\nlib = { path = \"../lib\" }\n\n[dev-dependencies]\nlib = { path = \"../lib\", features = [\"test-util\"] }\n",
                        ),
                    ),
                ],
                true,
            ),
            (
                vec![
                    (
                        "Cargo.toml",
                        "[workspace]\nresolver = \"2\"\nmembers = [\"crates/lib\", \"crates/data\"]\n".to_string(),
                    ),
                    (
                        DATA,
                        data(
                            "[dependencies]\nlib = { path = \"../lib\" }\n\n[dev-dependencies]\nlib = { path = \"../lib\", features = [\"test-util\"] }\n",
                        ),
                    ),
                ],
                false,
            ),
            // An optional dependency's implicit feature, which `dep:`
            // suppresses (found in review).
            (
                vec![
                    ("Cargo.toml", format!("{ROOT}exclude = [\"helper\"]\n")),
                    (
                        "crates/lib/Cargo.toml",
                        "[package]\nname = \"lib\"\nedition = \"2024\"\n\n[dependencies]\ntest-util = { package = \"helper\", path = \"../../helper\", optional = true }\n\n[features]\nx = [\"test-util\"]\n".to_string(),
                    ),
                    (
                        "helper/Cargo.toml",
                        "[package]\nname = \"helper\"\nedition = \"2024\"\n".to_string(),
                    ),
                ],
                true,
            ),
            (
                vec![
                    ("Cargo.toml", format!("{ROOT}exclude = [\"helper\"]\n")),
                    (
                        "crates/lib/Cargo.toml",
                        "[package]\nname = \"lib\"\nedition = \"2024\"\n\n[dependencies]\ntest-util = { package = \"helper\", path = \"../../helper\", optional = true }\n\n[features]\nx = [\"dep:test-util\"]\n".to_string(),
                    ),
                    (
                        "helper/Cargo.toml",
                        "[package]\nname = \"helper\"\nedition = \"2024\"\n".to_string(),
                    ),
                ],
                false,
            ),
        ];
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        for (i, (files, want)) in rows.into_iter().enumerate() {
            let root = std::env::temp_dir()
                .join(format!("verus-count-test-util-{}-{i}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            let base = [
                ("Cargo.toml", ROOT.to_string()),
                ("crates/lib/Cargo.toml", LIB.to_string()),
                (DATA, data("")),
            ];
            for (p, t) in base.into_iter().chain(files.iter().cloned()) {
                let f = root.join(p);
                fs::create_dir_all(f.parent().unwrap().join("src")).unwrap();
                fs::write(f.parent().unwrap().join("src/lib.rs"), "").unwrap();
                fs::write(&f, t).unwrap();
            }
            let lock = std::process::Command::new(&cargo)
                .args(["generate-lockfile", "--offline"])
                .current_dir(&root)
                .output()
                .unwrap();
            assert!(
                lock.status.success(),
                "{files:?}: {}",
                String::from_utf8_lossy(&lock.stderr)
            );
            // The manifests' reading, and cargo's own resolution, which it
            // must match.
            let names = |e: Vec<(String, String)>| -> Vec<String> {
                e.iter()
                    .map(|(p, f)| {
                        assert_eq!(f, "test-util");
                        p.split(' ').next().unwrap_or_default().to_string()
                    })
                    .collect()
            };
            let got = names(features::test_util_enablers(&root, &test_util()).unwrap());
            let cargo_got = names(cargo_enablers(&root, &test_util()).unwrap());
            let wanted: Vec<String> = if want { vec!["lib".into()] } else { Vec::new() };
            assert_eq!(
                (&got, &cargo_got),
                (&wanted, &wanted),
                "{files:?}: (got, cargo's) = ({got:?}, {cargo_got:?}), wanted = {wanted:?}"
            );
            let _ = fs::remove_dir_all(&root);
        }
    }

    /// A module declared for tests alone is left out with its file, one
    /// declared for Verus alone is read for its rlimits only, and one whose
    /// `cfg` Verus's build might not meet ships unsure, whichever of the
    /// forms the cfg takes; a module a shipped build compiles, and Verus's
    /// is sure to, is none of them.
    #[test]
    fn test_and_verification_modules_are_left_out() {
        let root = std::env::temp_dir().join(format!("verus-count-modules-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        let lib = "#[cfg(any(test, verus_keep_ghost))]\nmod grouped;\n#[cfg(verus_keep_ghost)]\nmod layout;\n#[cfg(test)]\nmod tests;\nmod shipped;\n#[cfg(any(test, unix))]\nmod either;\n#[cfg(all(feature = \"jit\", not(miri)))]\nmod settled;\n";
        fs::write(root.join("src/lib.rs"), lib).unwrap();
        for m in ["grouped", "layout", "tests", "shipped", "either", "settled"] {
            fs::write(root.join(format!("src/{m}.rs")), "").unwrap();
        }
        // A test-only module inside an unsure one is test-only.
        fs::create_dir_all(root.join("src/either")).unwrap();
        fs::write(root.join("src/either.rs"), "#[cfg(test)]\nmod t;\n").unwrap();
        fs::write(root.join("src/either/t.rs"), "").unwrap();
        let file = parse_file(lib).unwrap();
        let mut mods = ModFiles {
            roots: crate_roots(&root),
            ..ModFiles::default()
        };
        let mut errors = Vec::new();
        test_module_files(
            &root.join("src/lib.rs"),
            &file,
            &atoms(),
            &mut mods,
            &mut errors,
        );
        test_module_files(
            &root.join("src/either.rs"),
            &parse_file("#[cfg(test)]\nmod t;\n").unwrap(),
            &atoms(),
            &mut mods,
            &mut errors,
        );
        assert!(errors.is_empty(), "errors: got = {errors:?}, wanted = []");
        // A file a test-, Verus-only or unsure module declares, at any
        // depth, inherits its class, beside a `foo.rs` (`foo/bar.rs`) as
        // beside a `mod.rs`.
        for sub in [
            "layout/deep.rs",
            "layout/deep/deeper.rs",
            "tests/inner.rs",
            "shipped/x.rs",
            "layout_other.rs",
            "either/x.rs",
        ] {
            let p = root.join("src").join(sub);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, "").unwrap();
        }
        for (sub, want) in [
            ("layout.rs", Built::Verus),
            ("layout/deep.rs", Built::Verus),
            ("layout/deep/deeper.rs", Built::Verus),
            ("grouped.rs", Built::Verus),
            ("tests.rs", Built::Test),
            ("tests/inner.rs", Built::Test),
            ("shipped.rs", Built::Shipped),
            // Files no declaration names: unsure, under the guards of
            // the modules whose directories hold them.
            ("shipped/x.rs", Built::Unsure),
            ("layout_other.rs", Built::Unsure),
            ("lib.rs", Built::Shipped),
            ("settled.rs", Built::Shipped),
            ("either.rs", Built::Unsure),
            ("either/x.rs", Built::Unsure),
            ("either/t.rs", Built::Test),
        ] {
            let f = fs::canonicalize(root.join("src").join(sub)).unwrap();
            let got = module_class(&f, &mods, &atoms());
            assert_eq!(got, Ok(want), "{sub}: got = {got:?}, wanted = {want:?}");
        }
        let _ = fs::remove_dir_all(&root);
    }

    /// The manifests' reading (#136) fails closed on each form it does not
    /// read, and where the lockfile shows what the manifests do not.
    #[test]
    fn test_features_reading_fails_closed() {
        const LIB: &str =
            "[package]\nname = \"lib\"\nedition = \"2024\"\n\n[features]\ntest-util = []\n";
        const DATA: &str = "[package]\nname = \"data\"\nedition = \"2024\"\n\n";
        const LOCK: &str = "version = 4\n\n[[package]]\nname = \"data\"\nversion = \"0.0.0\"\ndependencies = [\"lib\"]\n\n[[package]]\nname = \"lib\"\nversion = \"0.0.0\"\n";
        const ROOT: &str = "[workspace]\nmembers = [\"crates/lib\", \"crates/data\"]\n";
        const EXT: &str = "\n[[package]]\nname = \"ext\"\nversion = \"1.0.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n";
        let none = || Vec::<(&str, String)>::new();
        for (case, root_toml, data, lock, extra, want) in [
            (
                "a glob other than a trailing /*",
                "[workspace]\nmembers = [\"crates/l*\", \"crates/data\"]\n",
                "",
                LOCK.to_owned(),
                none(),
                "glob other than a trailing",
            ),
            (
                "a lockfile package no manifest names",
                "[workspace]\nmembers = [\"crates/lib\"]\n",
                "",
                LOCK.to_owned(),
                none(),
                "has the package data with no source",
            ),
            (
                "a feature naming no dependency",
                ROOT,
                "[features]\nx = [\"nope/test-util\"]\n",
                LOCK.to_owned(),
                none(),
                "has no dependency `nope`",
            ),
            (
                "a dependency's features that are not a list",
                ROOT,
                "[dependencies]\nlib = { path = \"../lib\", features = \"test-util\" }\n",
                LOCK.to_owned(),
                none(),
                "dependency `lib`'s `features` is not a list of strings",
            ),
            (
                "a feature that is not a list",
                ROOT,
                "[features]\nx = \"lib/test-util\"\n",
                LOCK.to_owned(),
                none(),
                "features.x is not a list of strings",
            ),
            (
                "[features] that is not a table",
                ROOT,
                "features = [\"x\"]\n[package]\nname = \"data\"\n",
                LOCK.to_owned(),
                none(),
                "[features] is not a table",
            ),
            (
                "workspace = true with no entry",
                ROOT,
                "[dependencies]\nlib = { workspace = true }\n",
                LOCK.to_owned(),
                none(),
                "the workspace has no such dependency",
            ),
            (
                "a package with no name",
                ROOT,
                "[package]\nedition = \"2024\"\n",
                LOCK.to_owned(),
                none(),
                "[package] has no name",
            ),
            (
                "[target] that is not a table",
                ROOT,
                "target = 1\n[package]\nname = \"data\"\n",
                LOCK.to_owned(),
                none(),
                "[target] is not a table",
            ),
            (
                "a dependency section that is not a table",
                ROOT,
                "dependencies = 1\n[package]\nname = \"data\"\n",
                LOCK.to_owned(),
                none(),
                "a dependency section is not a table",
            ),
            (
                "a dependency that is neither a version nor a table",
                ROOT,
                "[dependencies]\nlib = 1\n",
                LOCK.to_owned(),
                none(),
                "dependency `lib` is neither a version nor a table",
            ),
            (
                "a workspace dependency that is neither a version nor a table",
                "[workspace]\nmembers = [\"crates/lib\", \"crates/data\"]\n[workspace.dependencies]\nlib = 1\n",
                "[dependencies]\nlib = { workspace = true }\n",
                LOCK.to_owned(),
                none(),
                "dependency `lib` is neither a version nor a table",
            ),
            (
                "a package that is not a string",
                ROOT,
                "[dependencies]\nlib = { path = \"../lib\", package = 1 }\n",
                LOCK.to_owned(),
                none(),
                "`package` is not a string",
            ),
            (
                "a path that is not a string",
                ROOT,
                "[dependencies]\nlib = { path = 1 }\n",
                LOCK.to_owned(),
                none(),
                "`path` is not a string",
            ),
            (
                "an optional that is not a boolean",
                ROOT,
                "[dependencies]\nlib = { path = \"../lib\", optional = 1 }\n",
                LOCK.to_owned(),
                none(),
                "`optional` is not a boolean",
            ),
            (
                "a resolver it does not read",
                "[workspace]\nresolver = \"4\"\nmembers = [\"crates/lib\", \"crates/data\"]\n",
                "",
                LOCK.to_owned(),
                none(),
                "is not one verus-count reads",
            ),
            (
                "a lockfile `package` that is not an array",
                ROOT,
                "",
                "version = 4\npackage = 1\n".to_owned(),
                none(),
                "`package` is not an array",
            ),
            (
                "a lockfile package with no name",
                ROOT,
                "",
                format!("{LOCK}\n[[package]]\nversion = \"1.0.0\"\n"),
                none(),
                "a Cargo.lock package has no name",
            ),
            (
                "a lockfile dependency that is not a string",
                ROOT,
                "",
                format!("{LOCK}{EXT}dependencies = [1]\n"),
                none(),
                "a Cargo.lock dependency is not a string",
            ),
            (
                "a package with a source that depends on one with a test-only feature",
                ROOT,
                "",
                format!("{LOCK}{EXT}dependencies = [\"lib\"]\n"),
                none(),
                "a package with a source, depend on lib, which has the test-only feature `test-util`",
            ),
            (
                "two manifests of one name",
                ROOT,
                "[dependencies]\nlib = { path = \"../lib\" }\nother = { path = \"../../dup\", package = \"lib\" }\n",
                LOCK.to_owned(),
                vec![("dup/Cargo.toml", LIB.to_owned())],
                "two manifests name the package lib",
            ),
            (
                "a member the lockfile does not have (a stale lockfile)",
                "[workspace]\nmembers = [\"crates/lib\", \"crates/data\", \"crates/x\"]\n",
                "",
                LOCK.to_owned(),
                vec![(
                    "crates/x/Cargo.toml",
                    "[package]\nname = \"x\"\nedition = \"2024\"\n\n[dependencies]\nlib = { path = \"../lib\", features = [\"test-util\"] }\n".to_owned(),
                )],
                "a build compiles the package x, but Cargo.lock",
            ),
            ("no lockfile", ROOT, "", String::new(), none(), "cannot read"),
        ] {
            let root = std::env::temp_dir()
                .join(format!("verus-count-fails-closed-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            // A case that writes its own `[package]` replaces the whole
            // manifest.
            let data = match data.contains("[package]") {
                true => data.to_owned(),
                false => format!("{DATA}{data}"),
            };
            for (p, t) in [
                ("Cargo.toml", root_toml.to_owned()),
                ("crates/lib/Cargo.toml", LIB.to_owned()),
                ("crates/data/Cargo.toml", data),
            ]
            .into_iter()
            .chain(extra)
            {
                let f = root.join(p);
                fs::create_dir_all(f.parent().unwrap()).unwrap();
                fs::write(&f, t).unwrap();
            }
            if !lock.is_empty() {
                fs::write(root.join("Cargo.lock"), lock).unwrap();
            }
            let got = features::test_util_enablers(&root, &test_util());
            assert!(
                got.as_ref().is_err_and(|e| e.contains(want)),
                "{case}: got = {got:?}, wanted an error with {want:?}"
            );
            let _ = fs::remove_dir_all(&root);
        }
        // A package with a source that depends on a local one with no
        // test-only feature of its own may ask for any of its features: one
        // that forwards a test-only feature reaches it (found in review).
        let root = std::env::temp_dir().join(format!("verus-count-patched-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (p, t) in [
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/lib\"]\n\n[patch.crates-io]\nmid = { path = \"mid\" }\n",
            ),
            ("crates/lib/Cargo.toml", LIB),
            (
                "mid/Cargo.toml",
                "[package]\nname = \"mid\"\nedition = \"2024\"\n\n[dependencies]\nlib = { path = \"../crates/lib\" }\n\n[features]\nx = [\"lib/test-util\"]\n",
            ),
        ] {
            let f = root.join(p);
            fs::create_dir_all(f.parent().unwrap()).unwrap();
            fs::write(&f, t).unwrap();
        }
        fs::write(
            root.join("Cargo.lock"),
            format!("version = 4\n\n[[package]]\nname = \"lib\"\nversion = \"0.0.0\"\n\n[[package]]\nname = \"mid\"\nversion = \"0.0.0\"\ndependencies = [\"lib\"]\n{EXT}dependencies = [\"mid\"]\n"),
        )
        .unwrap();
        assert_eq!(
            features::test_util_enablers(&root, &test_util()),
            Ok(vec![("lib".to_owned(), "test-util".to_owned())])
        );
        // An optional dependency's implicit feature is a feature such a
        // package may ask for, unless a `dep:` entry suppresses it.
        for (x, want) in [("test-util", false), ("dep:test-util", true)] {
            fs::write(
                root.join("crates/lib/Cargo.toml"),
                format!("[package]\nname = \"lib\"\nedition = \"2024\"\n\n[dependencies]\ntest-util = {{ package = \"helper\", path = \"../../helper\", optional = true }}\n\n[features]\nx = [\"{x}\"]\n"),
            )
            .unwrap();
            fs::create_dir_all(root.join("helper")).unwrap();
            fs::write(
                root.join("helper/Cargo.toml"),
                "[package]\nname = \"helper\"\nedition = \"2024\"\n",
            )
            .unwrap();
            fs::write(
                root.join("Cargo.lock"),
                format!("version = 4\n\n[[package]]\nname = \"helper\"\nversion = \"0.0.0\"\n\n[[package]]\nname = \"lib\"\nversion = \"0.0.0\"\n\n[[package]]\nname = \"mid\"\nversion = \"0.0.0\"\ndependencies = [\"lib\"]\n{EXT}dependencies = [\"lib\"]\n"),
            )
            .unwrap();
            let got = features::test_util_enablers(&root, &test_util());
            assert_eq!(got.is_ok(), want, "{x}: got = {got:?}");
        }
        let _ = fs::remove_dir_all(&root);
    }

    /// A workspace cargo cannot resolve (here, no lockfile under
    /// `--locked`) is an error, not a pass.
    #[test]
    fn test_util_check_fails_without_resolution() {
        let root =
            std::env::temp_dir().join(format!("verus-count-unresolved-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "").unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"lib\"\nedition = \"2024\"\n\n[workspace]\n",
        )
        .unwrap();
        assert!(features::test_util_enablers(&root, &test_util()).is_err());
        assert!(cargo_enablers(&root, &test_util()).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    /// A misspelt option fails instead of becoming ROOT, so a run cannot
    /// pass with a gate quietly left off.
    #[test]
    fn unknown_options_and_a_second_root_fail() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| s.to_string()));
        assert_eq!(
            parse(&[
                "--floor",
                "verus/FLOOR",
                "--base-rev",
                "HEAD^",
                "--rlimit",
                "s.json",
                "--list",
                "r"
            ]),
            Ok(Args {
                list: true,
                floor: Some("verus/FLOOR".into()),
                base_rev: Some("HEAD^".into()),
                stats: Some("s.json".into()),
                root: Some("r".into()),
                ..Args::default()
            })
        );
        assert_eq!(parse(&[]), Ok(Args::default()));
        for help in ["--help", "-h"] {
            assert_eq!(
                parse(&[help]),
                Ok(Args {
                    help: true,
                    ..Args::default()
                })
            );
        }
        assert_eq!(
            parse(&["--floor", "verus/FLOOR", "--no-base"]),
            Ok(Args {
                floor: Some("verus/FLOOR".into()),
                no_base: true,
                ..Args::default()
            })
        );
        assert_eq!(
            parse(&["--badge", "--config", "c.toml", "r"]),
            Ok(Args {
                badge: true,
                config_file: Some("c.toml".into()),
                root: Some("r".into()),
                ..Args::default()
            })
        );
        for (args, want) in [
            (
                &["--no-base", "--base-rev", "HEAD^"][..],
                "--no-base with --base-rev",
            ),
            (&["--flor", "verus/FLOOR", "."][..], "unknown option --flor"),
            (&["-l", "."][..], "unknown option -l"),
            (&["--helps"][..], "unknown option --helps"),
            (
                &["--floor", "verus/FLOOR", "--base-rev", "HEAD", "-h", "."][..],
                "--help and --version print and check nothing",
            ),
            (&["--help", "."][..], "--help and --version print"),
            (&["--list", "--help"][..], "--help and --version print"),
            (
                &["--version", "--no-base"][..],
                "--help and --version print",
            ),
            (&["--badge", "--version"][..], "--help and --version print"),
            (&["--help", "--version"][..], "--help and --version print"),
            (&["verus/FLOOR", "."][..], "a second ROOT ."),
            (&["--floor"][..], "--floor needs a value"),
            (&["--badge", "--floor", "verus/FLOOR"][..], "--badge prints"),
            (&["--badge", "--base-rev", "HEAD"][..], "--badge prints"),
            (&["--badge", "--no-base"][..], "--badge prints"),
            (&["--list", "--badge"][..], "--badge prints"),
            (&["--badge", "--rlimit", "s.json"][..], "--badge prints"),
        ] {
            let got = parse(args).unwrap_err();
            assert!(
                got.contains(want),
                "{args:?}: got = {got:?}, wanted = {want:?}"
            );
        }
    }

    /// The badge line is challoc's form with FLOOR's figures, one decimal
    /// each, in the alt text and the URL; a file passes only with that
    /// line, once. Negative: no badge, the alt text or the URL alone off
    /// by a tenth, the line with other text on it, two badges (even two
    /// canonical lines, or a second in any case or form), and the
    /// canonical line only where it does not render (a fence or an HTML
    /// comment) each fail, naming the line to write. A badge that does not
    /// render is not a second one.
    #[test]
    fn the_badge_gives_floors_figures_once() {
        let floor = Floor([Some(863), Some(944), Some(900), Some(950)]);
        let want = badge_line(&floor);
        assert_eq!(
            want,
            "[![Verus: 86.3% of functions, 94.4% of lines](https://img.shields.io/badge/Verus-86.3%25%20of%20functions%2C%2094.4%25%20of%20lines-blue)](verus/README.md#coverage)"
        );
        assert_eq!(
            badge_line(&Floor([Some(1000), Some(0), None, None])),
            "[![Verus: 100.0% of functions, 0.0% of lines](https://img.shields.io/badge/Verus-100.0%25%20of%20functions%2C%200.0%25%20of%20lines-blue)](verus/README.md#coverage)"
        );
        let readme = |badge: &str| format!("# port\n\n{badge}\n\nText.\n");
        assert_eq!(
            badge_report(&readme(&want), "README.md", "verus/FLOOR", &want),
            Ok("ok badge: README.md:3 gives verus/FLOOR's functions and lines figures".to_owned())
        );
        let alt = want.replacen("86.3% of functions", "86.2% of functions", 1);
        // Another Verus badge where it does not render is not a second one.
        for hidden in [
            format!("```markdown\n{alt}\n```"),
            format!("~~~\n{alt}\n~~~"),
            format!("<!--\n{alt}\n-->"),
            format!("<!-- {alt} -->"),
            // An HTML block at the start of a line crosses blank lines.
            format!("<!--\n\n{alt}\n-->"),
            // A list item's fence holds a tab-indented line: a tab is 4
            // columns, past the fence's 2, so the line stays in it.
            format!("- x\n\n  ```\n\t{alt}\n  ```"),
            // A comment in a nested quote whose lines keep both `>`.
            format!("> > <!--\n> > {alt}\n> > -->"),
            // A `1.` item, which may interrupt the paragraph, holds an
            // HTML block across a blank line.
            format!("1. <!--\n\n   {alt}\n   -->"),
            // A nested item's paragraph: a quote 4 columns past its
            // content column continues it, so the comment stays open.
            format!("- a\n  - Text <!-- x\n        > {alt}\n    -->"),
        ] {
            assert_eq!(
                badge_report(
                    &readme(&format!("{want}\n{hidden}")),
                    "README.md",
                    "verus/FLOOR",
                    &want
                ),
                Ok(
                    "ok badge: README.md:3 gives verus/FLOOR's functions and lines figures"
                        .to_owned()
                ),
                "{hidden:?}"
            );
        }
        let url = want.replacen("86.3%25", "86.2%25", 1);
        let alt_lines = want.replacen("94.4% of lines", "94.1% of lines", 1);
        let url_lines = want.replacen("94.4%25", "94.1%25", 1);
        for (text, wanted) in [
            (
                "# port\n\nText.\n".to_owned(),
                "FAIL badge: README.md has no Verus badge; add this line, which gives verus/FLOOR's functions and lines figures:\n",
            ),
            (
                readme(&alt),
                "FAIL badge: README.md:3 is not the badge verus/FLOOR's functions and lines figures give; replace it with this line:\n",
            ),
            (readme(&url), "FAIL badge: README.md:3 is not the badge"),
            (
                readme(&alt_lines),
                "FAIL badge: README.md:3 is not the badge",
            ),
            (
                readme(&url_lines),
                "FAIL badge: README.md:3 is not the badge",
            ),
            (
                readme(&format!("{want} [![CI](x)](y)")),
                "FAIL badge: README.md:3 is not the badge",
            ),
            (
                readme(&format!("{want}\n{want}")),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 4); keep one, this line:\n",
            ),
            (
                readme(&format!(
                    "{want}\n![Verus](https://img.shields.io/badge/Verus-1%25-blue)"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 4)",
            ),
            (
                readme(&format!(
                    "{want}\n[![Verus: 90%](https://example.com/v.svg)](x)"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 4)",
            ),
            // A Verus badge in any case: a lowercase label, and the
            // `static/v1` endpoint with no `[` before the `![`.
            (
                readme(&format!(
                    "{want}\n![verus](https://img.shields.io/badge/verus-99%25%20verified-green)"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 4)",
            ),
            (
                readme(&format!(
                    "{want}\n![Verus](https://img.shields.io/static/v1?label=Verus&message=99%25&color=blue)"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 4)",
            ),
            (
                readme(&format!("{want}\n[![VERUS](x.svg)](y)")),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 4)",
            ),
            (
                readme(&format!(
                    "{want}\n<img src=\"https://img.shields.io/badge/verus-1%25-blue\">"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 4)",
            ),
            // The canonical line where it does not render: inside a
            // multi-line HTML comment, a ``` fence, a ~~~ fence, and an
            // inline comment; and with a stale badge rendered outside.
            (
                readme(&format!("<!--\n{want}\n-->")),
                "FAIL badge: README.md has no Verus badge",
            ),
            (
                readme(&format!("```markdown\n{want}\n```")),
                "FAIL badge: README.md has no Verus badge",
            ),
            (
                readme(&format!("~~~\n{want}\n~~~")),
                "FAIL badge: README.md has no Verus badge",
            ),
            (
                readme(&format!("<!-- {want} -->")),
                "FAIL badge: README.md has no Verus badge",
            ),
            (
                readme(&format!("<!--\n{want}\n-->\n{alt}")),
                "FAIL badge: README.md:6 is not the badge",
            ),
            (
                readme(&format!("```\n{want}\n```\n{alt}")),
                "FAIL badge: README.md:6 is not the badge",
            ),
            // A fence only closes on a line of its own character, at least
            // as long, so a badge after "````" inside a ~~~ fence, or after
            // "```" inside a ```` fence, stays inside it.
            (
                readme(&format!("~~~\n````\n{want}\n~~~")),
                "FAIL badge: README.md has no Verus badge",
            ),
            (
                readme(&format!("````\n```\n{want}\n````")),
                "FAIL badge: README.md has no Verus badge",
            ),
            // A comment ends at `-->`, and text after it renders; inline
            // code with three backticks is not a fence.
            (
                readme(&format!("<!-- note --> {want}")),
                "FAIL badge: README.md:3 is not the badge",
            ),
            (
                readme(&format!("```x``` {want}")),
                "FAIL badge: README.md:3 is not the badge",
            ),
            // An `<img>` badge from a host other than shields.io.
            (
                readme(&format!(
                    "{want}\n<img alt=\"Verus\" src=\"https://badgen.net/badge/Verus/99%25/green\">"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 4)",
            ),
            // A fence or a comment never closed hides nothing: a list
            // item's fence that the renderer ends with the item, `<!--` in
            // inline code, and `<!-->`, a whole comment.
            (
                readme(&format!(
                    "{want}\n- step one\n  ```\n  example\nStale: ![Verus](https://img.shields.io/badge/Verus-99%25-green)"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 7)",
            ),
            (
                readme(&format!(
                    "{want}\nWrite `<!--` to start a comment.\nStale: ![Verus](https://img.shields.io/badge/Verus-99%25-green)"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            // The later `-->` would close `<!-->` or `<!--->` read as an
            // opener, hiding the stale badge in both readings, so these
            // rows hold only while they are whole comments.
            (
                readme(&format!(
                    "{want}\n<!--> Stale: ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n<!-- note -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 4)",
            ),
            (
                readme(&format!(
                    "{want}\n<!---> Stale: ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n<!-- note -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 4)",
            ),
            // The strict reading keeps a fence indented 1 to 3 spaces open
            // across a line indented less, as CommonMark does: only the
            // lenient reading ends it, so the canonical line is not
            // credited.
            (
                readme(&format!("  ```\n{want}\n  ```")),
                "FAIL badge: README.md:4 is the badge but may not render",
            ),
            // A `<!--` that renders as text, with a `-->` later: escaped,
            // in an indented code block, and mid-line with a blank line
            // before its `-->` (an inline comment cannot cross one).
            (
                readme(&format!(
                    "{want}\n\nUse \\<!-- here.\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n<!-- x -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 6)",
            ),
            (
                readme(&format!(
                    "{want}\n\n    <!-- example\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n<!-- x -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 6)",
            ),
            (
                readme(&format!(
                    "{want}\n\nText <!-- here.\n\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n\n<!-- x -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 7)",
            ),
            // A comment whose container ends before its `-->`: an HTML
            // block in a list item or a block quote (no lazy continuation),
            // and an inline comment whose paragraph a list item or a
            // heading ends.
            (
                readme(&format!(
                    "{want}\n- <!--\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            (
                readme(&format!(
                    "{want}\n> <!--\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            (
                readme(&format!(
                    "{want}\nText <!-- x\n- ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            (
                readme(&format!(
                    "{want}\nText <!-- x\n# ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            // Each other line that ends a paragraph (#120): a block quote,
            // a thematic break, a fence, an ordered list item, and an HTML
            // block start (the centered `<p><img>` badge).
            (
                readme(&format!(
                    "{want}\nText <!-- x\n> ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            (
                readme(&format!(
                    "{want}\nText <!-- x\n***\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 6)",
            ),
            (
                readme(&format!(
                    "{want}\nText <!-- x\n~~~\n~~~\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 7)",
            ),
            (
                readme(&format!(
                    "{want}\nText <!-- x\n1. ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            (
                readme(&format!(
                    "{want}\nText <!-- x\n<p align=\"center\"><img alt=\"Verus\" src=\"https://img.shields.io/badge/Verus-99%25-green\"></p>\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            // An HTML block in an ordered item, in an item's quote, and in
            // an item a tab separates from its marker; `<!--` after a tab,
            // an indented code block.
            (
                readme(&format!(
                    "{want}\n1. <!--\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            // `2.` cannot interrupt a paragraph, so `2. <!-- x` may
            // continue the badge's paragraph, its `<!--` inline: the
            // comment ends where a paragraph would, at a quote line or a
            // blank line, though the item's rule would keep it open.
            (
                readme(&format!(
                    "{want}\n2. <!-- x\n   > ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n   -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            (
                readme(&format!(
                    "{want}\n2. <!-- x\n\n   ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n   -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 6)",
            ),
            // A paragraph in a nested item: a quote at the item's content
            // column (4), past 3 from the line's start, interrupts it.
            (
                readme(&format!(
                    "{want}\n- a\n  - Text <!-- x\n    > ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n    -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 6)",
            ),
            (
                readme(&format!(
                    "{want}\n- a\n  2. <!-- x\n     > ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n     -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 6)",
            ),
            // The same with a fence at the nested item's content column:
            // `fence` is read on the line after its indent.
            (
                readme(&format!(
                    "{want}\n- a\n  - Text <!-- x\n    ~~~\n    ~~~\n    ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n    -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 8)",
            ),
            (
                readme(&format!(
                    "{want}\n- > <!--\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            (
                readme(&format!(
                    "{want}\n-\t<!--\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            // The same with the badge and `-->` indented 2: a tab after
            // the marker puts the item's content at column 4, so a line
            // indented 2 ends the item (counting bytes, it would not).
            (
                readme(&format!(
                    "{want}\n-\t<!--\n  ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n  -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            (
                readme(&format!(
                    "{want}\n\n\t<!-- example\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n<!-- x -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 6)",
            ),
            // The same closed by a bare `-->`, which ends no paragraph, so
            // only reading the tab-indented `<!--` as code shows the badge.
            (
                readme(&format!(
                    "{want}\n\n\t<!-- example\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 6)",
            ),
            // More HTML block starts that end a paragraph (`<?`, `<!`, a
            // closing tag), a thematic break of tab-separated `_`, and an
            // indented code block inside a quote.
            (
                readme(&format!(
                    "{want}\nText <!-- x\n<?x?>\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 6)",
            ),
            (
                readme(&format!(
                    "{want}\nText <!-- x\n<!DOCTYPE html>\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 6)",
            ),
            (
                readme(&format!(
                    "{want}\nText <!-- x\n</div><img alt=\"Verus\" src=\"https://img.shields.io/badge/Verus-99%25-green\">\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            (
                readme(&format!(
                    "{want}\nText <!-- x\n_\t_\t_\n![Verus](https://img.shields.io/badge/Verus-99%25-green)\n-->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 6)",
            ),
            (
                readme(&format!(
                    "{want}\n>     <!--\n> ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n> -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            // The same with a tab after `>`: the optional space takes one
            // of its 3 columns, so 2 spaces more make 4; and a comment in
            // a nested quote, which a line with one `>` ends.
            (
                readme(&format!(
                    "{want}\n>\t  <!--\n> ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n> -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            (
                readme(&format!(
                    "{want}\n> > <!--\n> ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n> > -->"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            // A closed comment still hides, and an unclosed opener later
            // in the file does not reopen it.
            (
                readme(&format!("<!--\n{want}\n-->\n{alt}\nWrite `<!--` here.")),
                "FAIL badge: README.md:6 is not the badge",
            ),
            // The canonical line only after an opener never closed: the
            // strict reading hides it, so it is not credited.
            (
                readme(&format!("<!--\n{want}\n")),
                "FAIL badge: README.md:4 is the badge but may not render",
            ),
            (
                readme(&format!("```\n{want}\n")),
                "FAIL badge: README.md:4 is the badge but may not render",
            ),
            // Closers later in the file: `<!--` in inline code closed by
            // `-->` in inline code, and a list item's fence that a later
            // top-level fence would close. The stale badge renders.
            (
                readme(&format!(
                    "{want}\nWrite `<!--` to open a comment.\nStale: ![Verus](https://img.shields.io/badge/Verus-99%25-green)\nEnd it with `-->`."
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 5)",
            ),
            (
                readme(&format!(
                    "{want}\n- step\n  ```\n  ex\nStale: ![Verus](https://img.shields.io/badge/Verus-99%25-green)\n```\ncode\n```"
                )),
                "FAIL badge: README.md has 2 Verus badges (lines 3, 7)",
            ),
        ] {
            let got = badge_report(&text, "README.md", "verus/FLOOR", &want);
            assert!(
                got.as_ref()
                    .is_err_and(|e| e.contains(wanted) && e.ends_with(&want)),
                "{text:?}: got = {got:?}, wanted = Err({wanted:?} ... {want:?})"
            );
        }
    }

    #[test]
    fn cfg_predicates_that_hold_only_under_test() {
        for (cfg, want) in [
            ("test", true),
            ("all (test , feature = \"x\")", true),
            ("all (feature = \"x\" , test)", true),
            ("not (test)", false),
            ("any (test , feature = \"x\")", false),
            ("feature = \"test-util\"", true),
            ("feature = \"other-util\"", false),
            ("any (test , feature = \"test-util\")", true),
            ("any (feature = \"test-util\" , unix)", false),
            ("all (feature = \"test-util\" , unix)", true),
            ("not (feature = \"test-util\")", false),
            ("all (not (test) , unix)", false),
            ("kani", true),
            ("all (kani , feature = \"x\")", true),
            ("not (kani)", false),
            ("any (kani , test)", true),
            // `any()` holds nowhere: no build compiles it.
            ("any ()", true),
            ("verus_keep_ghost", true),
            ("any (test , verus_keep_ghost)", true),
            ("all (verus_keep_ghost , unix)", true),
            ("not (verus_keep_ghost)", false),
            ("any (verus_keep_ghost , unix)", false),
            // Nested predicates, read to any depth.
            ("any (test , all (test , unix))", true),
            ("any (test , all (verus_keep_ghost , unix))", true),
            ("any (test , all (not (test) , unix))", false),
            ("not (any (test , kani))", false),
            ("not (all (unix , not (test)))", false),
            (
                "all (feature = \"jit\" , not (miri) , any (test , feature = \"x86-jit\"))",
                false,
            ),
            ("all (feature = \"a,b\" , test)", true),
            // A configured Verus-only feature, as `verus_keep_ghost`.
            ("feature = \"verus\"", true),
            ("any (feature = \"verus\" , unix)", false),
        ] {
            assert_eq!(
                cfg_needs_test(cfg, &atoms()),
                want,
                "{cfg}: got = {}, wanted = {want}",
                !want
            );
        }
        // The atoms come from the configuration: with none configured but
        // the defaults, `test-util` and `verus` are ordinary features, and
        // `feature = "verus"` builds where Verus's does.
        let plain = Atoms::of(&Config::parse("[crates]\ndir = \"crates\"\n").expect("parses"));
        assert!(!cfg_needs_test("feature = \"test-util\"", &plain));
        assert!(!cfg_needs_test("feature = \"verus\"", &plain));
        assert_eq!(cfg_built("feature = \"verus\"", &atoms()), Ok(Built::Verus));
        assert_eq!(cfg_built("kani", &plain), Ok(Built::Test));
    }

    /// A `cfg` predicate the counter cannot read is an error, not a guess:
    /// unbalanced, an empty argument, `not` of other than one.
    #[test]
    fn unreadable_cfg_predicates_are_errors() {
        for cfg in [
            "all (test , any (kani)",
            "any (test) )",
            "all (test , , unix)",
            "not (test , kani)",
            "",
        ] {
            assert!(
                cfg_built(cfg, &atoms()).is_err(),
                "{cfg:?}: got = Ok, wanted = an error"
            );
        }
        // Where a nested predicate lets an item be built.
        for (cfg, want) in [
            (
                "any (target_arch = \"aarch64\" , all (target_os = \"linux\" , not (miri)))",
                Built::Unsure,
            ),
            ("any (test , all (verus_keep_ghost , unix))", Built::Verus),
            ("all (test , any (verus_keep_ghost , unix))", Built::Test),
        ] {
            let got = cfg_built(cfg, &atoms());
            assert_eq!(got, Ok(want), "{cfg}: got = {got:?}, wanted = {want:?}");
        }
    }

    /// A `cfg` a shipped build can meet is sure in Verus's build only
    /// when the configuration settles it there: a `verus_sets` atom, `not`
    /// of a `verus_never_sets` one, or a predicate those, the Verus atoms
    /// and the test atoms decide. Anything else Verus's build might not
    /// meet, the issue's `target_os = "windows"` included.
    #[test]
    fn only_settled_atoms_are_sure_in_verus_s_build() {
        for (cfg, want) in [
            ("feature = \"jit\"", Built::Shipped),
            ("not (miri)", Built::Shipped),
            ("all (feature = \"jit\" , not (miri))", Built::Shipped),
            ("any (unix , feature = \"jit\")", Built::Shipped),
            ("not (test)", Built::Shipped),
            ("not (verus_keep_ghost)", Built::Unsure),
            ("any (verus_keep_ghost , unix)", Built::Shipped),
            ("target_os = \"windows\"", Built::Unsure),
            ("unix", Built::Unsure),
            ("miri", Built::Unsure),
            ("not (feature = \"jit\")", Built::Unsure),
            ("all (feature = \"jit\" , unix)", Built::Unsure),
            ("feature = \"x86-jit\"", Built::Unsure),
            ("any (miri , test)", Built::Unsure),
        ] {
            let got = cfg_built(cfg, &atoms());
            assert_eq!(got, Ok(want), "{cfg}: got = {got:?}, wanted = {want:?}");
        }
        // Several `cfg`s that must hold together: the narrowest class.
        let (shipped, unsure, verus, test) =
            ("feature = \"jit\"", "unix", "verus_keep_ghost", "test");
        for (a, b, want) in [
            (shipped, unsure, Built::Unsure),
            (unsure, verus, Built::Verus),
            (unsure, test, Built::Test),
            (shipped, shipped, Built::Shipped),
        ] {
            for (x, y) in [(a, b), (b, a)] {
                let got = guarded_built(&[x.to_owned(), y.to_owned()], &atoms());
                assert_eq!(
                    got,
                    Ok(want),
                    "{x} and {y}: got = {got:?}, wanted = {want:?}"
                );
            }
        }
        // Two `cfg`s, each needing its own value of one key, hold in no
        // build together, as their `all` does.
        let arches = Atoms {
            sets: vec![
                "target_arch=\"x86_64\"".to_owned(),
                "target_arch=\"aarch64\"".to_owned(),
            ],
            ..atoms()
        };
        let got = guarded_built(
            &[
                "target_arch = \"x86_64\"".to_owned(),
                "target_arch = \"aarch64\"".to_owned(),
            ],
            &arches,
        );
        assert_eq!(
            got,
            Ok(Built::Test),
            "two architectures' cfgs: got = {got:?}, wanted = Test"
        );
        // One key, two values: no build meets it, though each atom on its
        // own is sure where both are declared (challoc's two runs). Keys
        // rustc gives several values (`target_family`, `feature`) and a
        // `not` of the impossible conjunction are not that.
        let both = Atoms {
            sets: vec![
                "target_arch=\"x86_64\"".into(),
                "target_arch=\"aarch64\"".into(),
            ],
            ..atoms()
        };
        for (cfg, want) in [
            (
                "all (target_arch = \"x86_64\" , target_arch = \"aarch64\")",
                Built::Test,
            ),
            (
                "all (target_arch = \"x86_64\" , any (target_arch = \"aarch64\" , test))",
                Built::Test,
            ),
            (
                "any (all (target_arch = \"x86_64\" , target_arch = \"aarch64\") , kani)",
                Built::Test,
            ),
            (
                "all (target_os = \"linux\" , all (unix , target_os = \"macos\"))",
                Built::Test,
            ),
            ("all (panic = \"abort\" , panic = \"unwind\")", Built::Test),
            (
                "all (target_pointer_width = \"32\" , target_pointer_width = \"64\")",
                Built::Test,
            ),
            (
                "not (all (target_arch = \"x86_64\" , target_arch = \"aarch64\"))",
                Built::Shipped,
            ),
            (
                "all (target_family = \"unix\" , target_family = \"wasm\")",
                Built::Unsure,
            ),
            ("all (feature = \"a\" , feature = \"b\")", Built::Unsure),
            ("target_arch = \"x86_64\"", Built::Shipped),
            // Each atom is read on its own: the aarch64 run checks this,
            // but the counter cannot tell the runs apart.
            ("not (target_arch = \"x86_64\")", Built::Unsure),
        ] {
            let got = cfg_built(cfg, &both);
            assert_eq!(got, Ok(want), "{cfg}: got = {got:?}, wanted = {want:?}");
        }
        // Without the declarations, `jit` and `miri` are unsettled.
        let plain = Atoms::of(&Config::parse("[crates]\ndir = \"crates\"\n").expect("parses"));
        for cfg in ["feature = \"jit\"", "not (miri)"] {
            let got = cfg_built(cfg, &plain);
            assert_eq!(
                got,
                Ok(Built::Unsure),
                "{cfg}: got = {got:?}, wanted = Unsure"
            );
        }
    }

    /// The functions [`recursive`] finds recursing in one file, `src`, that
    /// Verus does not check (`verified` false), or that it does (true).
    fn recursing(src: &str, verified: bool) -> Vec<String> {
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from("lib.rs"),
            module: vec!["k".into()],
            ..counter()
        };
        c.items(&parse_file(src).expect("parses").items, false);
        recursive(&c.fns, &[])
            .into_iter()
            .filter(|(i, _)| (c.fns[*i].class == Class::Verified) == verified)
            .map(|(i, path)| format!("{}: {}", c.fns[i].name, path.join(" -> ")))
            .collect()
    }

    /// What [`nested_loops`] finds in one file, `src`, as `name: why; ..`.
    fn nesting(src: &str) -> Vec<String> {
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from("lib.rs"),
            module: vec!["k".into()],
            ..counter()
        };
        c.items(&parse_file(src).expect("parses").items, false);
        assert_eq!(c.errors, 0, "{src}");
        nested_loops(&c.fns, &[])
            .into_iter()
            .map(|(i, why)| format!("{}: {}", c.fns[i].name, why.join("; ")))
            .collect()
    }

    /// The nested-loop check (#76): a loop in a loop, and a call in a loop
    /// to a function that loops, itself or through a call, each with its
    /// line; a single loop, loops one after another, and what only looks
    /// like a loop (an `impl .. for`, `for<'a>`, a nested function's own
    /// loop) are not.
    #[test]
    fn nested_loops_are_reported() {
        // A work-bound mutant of a CRC table update: an inner loop that
        // rescans the bytes before `i`, with its own `decreases`.
        assert_eq!(
            nesting(
                "verus! {\nfn update_table(t: &mut [u32]) {\n    let mut i = 0;\n    while i < t.len()\n        invariant i <= t.len(),\n        decreases t.len() - i,\n    {\n        let mut j = 0;\n        while j < i\n            invariant j <= i,\n            decreases i - j,\n        {\n            j += 1;\n        }\n        i += 1;\n    }\n}\n}\n"
            ),
            ["update_table: a loop at line 9 inside another"]
        );
        assert_eq!(
            nesting(
                "fn f(a: &[u8]) {\n    for i in 0..a.len() {\n        for _ in 0..i {}\n    }\n}\n"
            ),
            ["f: a loop at line 3 inside another"]
        );
        assert_eq!(
            nesting(
                "fn f() {\n    'o: loop {\n        while g() { break 'o; }\n    }\n}\nfn g() -> bool { true }\n"
            ),
            ["f: a loop at line 3 inside another"]
        );
        // A call in a loop, or in its header, to a function that loops,
        // directly, through another call, or as `self.` or `Self::`.
        assert_eq!(
            nesting(
                "fn g(n: u32) { for _ in 0..n {} }\nfn f() {\n    for _ in 0..4 {\n        g(3);\n    }\n}\n"
            ),
            ["f: a call to `g`, which loops, inside the loop at line 3"]
        );
        assert_eq!(
            nesting(
                "fn h() { for _ in 0..2 {} }\nfn g() { h() }\nfn f() {\n    loop { g(); break; }\n}\n"
            ),
            ["f: a call to `g`, which loops, inside the loop at line 4"]
        );
        // Through two calls, written before what they call (a fixpoint, not
        // one pass).
        assert_eq!(
            nesting(
                "fn f() {\n    loop { g(); break; }\n}\nfn g() { k() }\nfn k() { h() }\nfn h() { for _ in 0..2 {} }\n"
            ),
            ["f: a call to `g`, which loops, inside the loop at line 2"]
        );
        assert_eq!(
            nesting("fn g() -> bool { for _ in 0..2 {} true }\nfn f() {\n    while g() {}\n}\n"),
            ["f: a call to `g`, which loops, inside the loop at line 3"]
        );
        assert_eq!(
            nesting(
                "struct S;\nimpl S {\n    fn g(&self) { for _ in 0..2 {} }\n    fn f(&self) {\n        for _ in 0..2 { self.g(); Self::g(self) }\n    }\n}\n"
            ),
            ["f: a call to `g`, which loops, inside the loop at line 5"]
        );
        // Not: one loop, loops in sequence, a call in a loop to a function
        // that does not loop, a call outside any loop to one that does.
        assert!(nesting("fn f(n: u32) { for _ in 0..n {} let mut i = 0; while i < n { i += 1; } loop { break; } }").is_empty());
        assert!(nesting("fn g() -> u8 { 1 }\nfn f() { for _ in 0..2 { g(); } }").is_empty());
        assert!(
            nesting("fn g() { for _ in 0..2 {} }\nfn f() { g(); for _ in 0..2 {} }").is_empty()
        );
        // Not loops: an `impl .. for` and a `for<'a>` inside a loop, and a
        // nested function's own loop beside one.
        assert!(nesting("fn f() { for _ in 0..2 { struct A; impl Clone for A { fn clone(&self) -> A { A } } } }").is_empty());
        assert!(
            nesting("fn f() { for _ in 0..2 { let _g: &dyn for<'a> Fn(&'a u8) = &|_| {}; } }")
                .is_empty()
        );
        assert!(nesting("fn f() { fn inner() { for _ in 0..2 {} } for _ in 0..2 {} }").is_empty());
        assert!(nesting("fn f() { for _ in 0..2 { fn inner() { for _ in 0..2 {} } } }").is_empty());
        // Found in review: a fn-pointer type is no nested item, and a
        // header may hold a brace group (`unsafe`, a block, `match`).
        assert_eq!(
            nesting(
                "fn id(x: u32) -> u32 { x }\nfn f(n: u32) {\n    let _g: fn(u32) -> u32 = id;\n    let _h = id as fn(u32) -> u32;\n    for i in 0..n {\n        for _ in 0..i {}\n    }\n}\n"
            ),
            ["f: a loop at line 6 inside another"]
        );
        assert_eq!(
            nesting(
                "fn f(p: *const u8, n: usize) {\n    for b in unsafe { core::slice::from_raw_parts(p, n) } {\n        for _ in 0..*b {}\n    }\n}\n"
            ),
            ["f: a loop at line 3 inside another"]
        );
        assert_eq!(
            nesting(
                "fn f(n: u32) {\n    let mut i = 0;\n    while { i += 1; i < n } {\n        for _ in 0..i {}\n    }\n}\n"
            ),
            ["f: a loop at line 4 inside another"]
        );
        assert_eq!(
            nesting(
                "fn f(s: u8) {\n    while match s { 0 => true, _ => false } {\n        loop { break; }\n    }\n}\n"
            ),
            ["f: a loop at line 3 inside another"]
        );
        // A closure bound by `let` whose body loops, called in a loop.
        assert_eq!(
            nesting(
                "fn f(n: u32) {\n    let scan = |m: u32| { for _ in 0..m {} };\n    for i in 0..n {\n        scan(i);\n    }\n}\n"
            ),
            ["f: a call to the closure `scan`, which loops, inside the loop at line 3"]
        );
        // A closure that loops through a call, or through another closure,
        // and one bound with a type or behind a `&` (found in review).
        assert_eq!(
            nesting(
                "fn g(n: u32) { for _ in 0..n {} }\nfn f(n: u32) {\n    let s = |m: u32| g(m);\n    for i in 0..n {\n        s(i);\n    }\n}\n"
            ),
            ["f: a call to the closure `s`, which loops, inside the loop at line 4"]
        );
        assert_eq!(
            nesting(
                "fn f(n: u32) {\n    let a = |m: u32| { for _ in 0..m {} };\n    let b = |m: u32| a(m);\n    for i in 0..n {\n        b(i);\n    }\n}\n"
            ),
            ["f: a call to the closure `b`, which loops, inside the loop at line 4"]
        );
        assert_eq!(
            nesting(
                "fn f(n: u32) {\n    let s: &dyn Fn(u32) = &|m: u32| { for _ in 0..m {} };\n    let t: fn(u32) = |m: u32| { for _ in 0..m {} };\n    for i in 0..n {\n        s(i);\n        t(i);\n    }\n}\n"
            ),
            [
                "f: a call to the closure `s`, which loops, inside the loop at line 4; a call to the closure `t`, which loops, inside the loop at line 4"
            ]
        );
        // Whatever wraps the closure (found in review).
        assert_eq!(
            nesting(
                "fn f(n: u32) {\n    let s = Box::new(|m: u32| { for _ in 0..m {} });\n    let t = (&|m: u32| { for _ in 0..m {} });\n    for i in 0..n {\n        s(i);\n        t(i);\n    }\n}\n"
            ),
            [
                "f: a call to the closure `s`, which loops, inside the loop at line 4; a call to the closure `t`, which loops, inside the loop at line 4"
            ]
        );
        // A closure that does not loop, called in a loop, is no finding.
        assert!(
            nesting("fn f(n: u32) { let s = |m: u32| m + 1; for i in 0..n { s(i); } }").is_empty()
        );
        // A closure in a `for`'s iterator runs per item (found in review).
        assert_eq!(
            nesting(
                "fn g(n: u8) { for _ in 0..n {} }\nfn f(v: &[u8]) {\n    for _ in v.iter().map(|b| g(*b)) {}\n}\n"
            ),
            ["f: a call to `g`, which loops, inside the loop at line 3"]
        );
        assert_eq!(
            nesting(
                "fn f(v: &[u8]) {\n    for _ in v.iter().map(|b| {\n        let mut k = 0;\n        while k < *b { k += 1; }\n        k\n    }) {}\n}\n"
            ),
            ["f: a loop at line 4 inside another"]
        );
        // A macro invocation in a loop that holds a loop keyword.
        assert_eq!(
            nesting("fn f() {\n    for _ in 0..2 {\n        run! { while true {} }\n    }\n}\n"),
            [
                "f: a loop at line 3 inside another; a macro invocation at line 3 that holds a loop, whose nesting the check cannot read"
            ]
        );
        // Nesting wholly inside a macro invocation is not read, so the
        // function is named (found in review).
        assert_eq!(
            nesting("fn f(n: usize) {\n    run! { for i in 0..n { for _ in 0..i {} } }\n}\n"),
            [
                "f: a macro invocation at line 2 that holds a loop, whose nesting the check cannot read"
            ]
        );
        // A `for`'s iterator runs once: a looping call there is outside
        // the loop.
        assert!(
            nesting("fn g() -> u32 { for _ in 0..2 {} 2 }\nfn f() { for _ in g()..9 {} }")
                .is_empty()
        );
        // A body that does not parse is unread, and fails closed.
        let shape = loop_shape("let = ;".parse().unwrap());
        assert!(shape.unread, "{shape:?}");
        let fn_ref = |name: &str, calls: Vec<(bool, String)>, loops: LoopShape| FnRef {
            krate: "k".into(),
            file: PathBuf::from("lib.rs"),
            line: 1,
            owner: String::new(),
            name: name.into(),
            class: Class::Outside,
            calls,
            loops,
            body: inputs::Body::default(),
            params: 0,
            receiver: false,
            entry: false,
        };
        let g = fn_ref("g", Vec::new(), shape);
        let found = nested_loops(std::slice::from_ref(&g), &[]);
        assert_eq!(
            found,
            [(0, vec!["a body the check cannot parse".to_owned()])]
        );
        // An unread callee counts as one that loops, so a caller that calls
        // it in a loop is named too (found in review).
        let f = fn_ref(
            "f",
            vec![(false, "g".into())],
            LoopShape {
                loops: true,
                in_loop: vec![(false, "g".into(), 2)],
                ..LoopShape::default()
            },
        );
        let found = nested_loops(&[g, f], &[]);
        assert_eq!(
            found,
            [
                (0, vec!["a body the check cannot parse".to_owned()]),
                (
                    1,
                    vec!["a call to `g`, which loops, inside the loop at line 2".to_owned()]
                )
            ]
        );
    }

    #[test]
    fn recursion_outside_verus_is_reported() {
        // Positive: direct, through `Self::` and `self.`, and mutual.
        assert_eq!(
            recursing("fn f(n: u32) { if n > 0 { f(n - 1) } }", false),
            ["f: f -> f"]
        );
        assert_eq!(
            recursing(
                "struct S; impl S { fn g(&self) { Self::g(self) } fn h(&self) { self.h() } }",
                false
            ),
            ["g: g -> g", "h: h -> h"]
        );
        // With a turbofish, and through a nested function.
        assert_eq!(
            recursing(
                "struct S; impl S { fn t<const N: bool>(&self) { self.t::<N>() } \
                 fn u<T: Fn() -> u8>(&self) { Self::u::<T>(self) } } \
                 fn v<const N: bool>() { v::<N>() }",
                false
            ),
            ["t: t -> t", "u: u -> u", "v: v -> v"]
        );
        assert_eq!(
            recursing(
                "fn outer() { fn inner(n: u32) { inner(n) } inner(1) }",
                false
            ),
            ["inner: inner -> inner"]
        );
        // A nested function in an expression's block: an `if`'s, a
        // closure's, a `match` arm's, a block's, a loop's; and an `impl`'s
        // method in one.
        assert_eq!(
            recursing(
                "fn outer(c: bool) { if c { fn a(n: u32) { a(n) } a(1) } \
                 let _ = || { fn b(n: u32) { b(n) } }; \
                 match c { true => { fn d(n: u32) { d(n) } } _ => {} } \
                 { fn e(n: u32) { e(n) } } \
                 loop { fn g(n: u32) { g(n) } break; } \
                 if c { struct S; impl S { fn h(&self) { self.h() } } } }",
                false
            ),
            [
                "a: a -> a",
                "b: b -> b",
                "d: d -> d",
                "e: e -> e",
                "g: g -> g",
                "h: h -> h"
            ]
        );
        // A function in a module, a trait's default method, and one in a
        // `const` initializer, each written in a body.
        assert_eq!(
            recursing(
                "fn outer() { mod m { fn i(n: u32) { i(n) } } \
                 trait T { fn j(&self) { self.j() } } \
                 const _: () = { fn k(n: u32) { k(n) } }; }",
                false
            ),
            ["i: i -> i", "j: j -> j", "k: k -> k"]
        );
        // A nested function built only for tests ships nothing: by its own
        // `cfg`, a local module's, an `impl`'s, or an enclosing nested
        // function's. One built otherwise (`not(test)`, a feature) is still
        // reported, beside the gated ones.
        assert_eq!(
            recursing(
                "fn outer() { if true { #[cfg(test)] fn helper() { helper(); } } \
                 #[cfg(test)] mod t { fn in_mod() { in_mod(); } } \
                 struct S; #[cfg(test)] impl S { fn m(&self) { self.m() } } \
                 #[cfg(test)] fn wrap() { fn deep() { deep(); } } \
                 #[cfg(not(test))] fn shipped() { shipped(); } \
                 #[cfg(feature = \"x\")] mod f { fn featured() { featured(); } } \
                 { fn plain() { plain(); } } }",
                false
            ),
            [
                "shipped: shipped -> shipped",
                "featured: featured -> featured",
                "plain: plain -> plain"
            ]
        );
        assert_eq!(
            recursing(
                "fn a() { b() } fn b() { if x() { a() } } fn x() -> bool { true }",
                false
            ),
            ["a: a -> b -> a", "b: b -> a -> b"]
        );
        // Trusted bodies are Verus's blind spot too.
        assert_eq!(
            recursing(
                "verus! { #[verifier::external_body] fn t(n: u32) { t(n) } }",
                false
            ),
            ["t: t -> t"]
        );
        // Negative: a verified function (Verus checks its `decreases`), a
        // call through another path or receiver, a macro, a declaration.
        assert!(
            recursing(
                "verus! { fn v(n: u32) decreases n { if n > 0 { v(n - 1) } } }",
                false
            )
            .is_empty()
        );
        assert_eq!(
            recursing(
                "verus! { fn v(n: u32) decreases n { if n > 0 { v(n - 1) } } }",
                true
            ),
            ["v: v -> v"]
        );
        assert!(
            recursing(
                "struct S; impl S { fn new() -> Vec<u8> { Vec::new() } }",
                false
            )
            .is_empty()
        );
        assert!(
            recursing(
                "struct S { v: Vec<u8> } impl S { fn get(&self) -> Option<&u8> { self.v.get(0) } }",
                false
            )
            .is_empty()
        );
        assert!(recursing("fn p() { p!(1); fn q() {} }", false).is_empty());
    }

    /// The module an item in `k::a::b` is visible within, for each
    /// visibility: the crate for `pub` and `pub(crate)`, `k::a::b` for none
    /// and `pub(self)`, `k::a` for `pub(super)`, the module a `pub(in ..)`
    /// names, and none for a `pub(in ..)` naming no module of the crate.
    #[test]
    fn an_item_is_visible_within_its_visibility_s_module() {
        let m = |s: &str| -> Vec<String> { s.split("::").map(str::to_owned).collect() };
        let c = Counter {
            module: m("k::a::b"),
            modules: ["k", "k::a", "k::a::b", "k::c"]
                .into_iter()
                .map(m)
                .collect(),
            ..counter()
        };
        for (vis, want) in [
            ("pub", Some("k")),
            ("pub(crate)", Some("k")),
            ("", Some("k::a::b")),
            ("pub(self)", Some("k::a::b")),
            ("pub(super)", Some("k::a")),
            ("pub(in crate::c)", Some("k::c")),
            ("pub(in super::super)", Some("k")),
            ("pub(in crate::gone)", None),
        ] {
            let parsed: verus_syn::Visibility = verus_syn::parse_str(vis).expect("parses");
            assert_eq!(
                c.vis_scope(&parsed),
                want.map(m),
                "{vis:?}: got = left, wanted = right"
            );
        }
    }

    /// A path's first segment that is not `crate`, `self` or `super`, in
    /// `k::split`: a module `k::split` declares; with `follow`, through the
    /// one `use` there that imports it by name, only when Verus's build is
    /// sure to compile that `use` (not under a `cfg` it might not meet,
    /// where another crate of that name may stand in); never a module only
    /// the crate root declares.
    #[test]
    fn a_path_s_first_segment_reaches_a_module_only_through_scope() {
        let m = |s: &str| -> Vec<String> { s.split("::").map(str::to_owned).collect() };
        let modules: std::collections::BTreeSet<Vec<String>> = [
            "k",
            "k::split",
            "k::split::inner",
            "k::x",
            "k::x::state",
            "k::root",
        ]
        .into_iter()
        .map(m)
        .collect();
        for (built, segs, follow, want) in [
            (Built::Shipped, "state", true, Some("k::x::state")),
            (Built::Verus, "state", true, Some("k::x::state")),
            (Built::Unsure, "state", true, None),
            (Built::Test, "state", true, None),
            (Built::Shipped, "state", false, None),
            (Built::Shipped, "inner", false, Some("k::split::inner")),
            (Built::Shipped, "root", true, None),
            (Built::Shipped, "crate::root", false, Some("k::root")),
        ] {
            let c = Counter {
                modules: modules.clone(),
                uses: [(
                    m("k::split"),
                    vec![(m("crate::x::state"), false, "state".to_owned(), built)],
                )]
                .into_iter()
                .collect(),
                ..counter()
            };
            assert_eq!(
                c.path_module(&m("k::split"), &m(segs), follow),
                want.map(m),
                "{segs:?}, follow {follow}, the use {built:?}: got = left, wanted = right"
            );
        }
    }

    /// The functions `src`, one file of crate `k`, records, by its walk.
    fn walked(src: &str) -> Counter {
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from("crates/k/src/lib.rs"),
            module: vec!["k".into()],
            ..counter()
        };
        c.items(&parse_file(src).expect("parses").items, false);
        c.finish();
        c
    }

    /// The rules ([`breaks`]) over every kind of binding, for each form: a
    /// leading `::` is broken only by an `extern crate .. as` of its name;
    /// a bare `verus!` by a binding of a Verus name but the crate itself or
    /// the macro's own path, or a `#[macro_use]` of another crate; a bare
    /// `compile_error!` by any binding of the name, a `#[macro_use]` of
    /// another crate, or a `*` in its module (`k`), not in another (`k::s`).
    #[test]
    fn a_binding_breaks_only_its_forms_rule() {
        use Took::{StdBare as S, StdPath as SP, VerusBare as B, VerusPath as P};
        // Each row: the binding (`item NAME`, `use NAME <- PATH`, `extern
        // NAME <- CRATE`, `glob [s] PATH` (in `k::s` with `s`), `macro_use
        // CRATE`), the form, and whether it breaks the form's rule for
        // `vstd` (P), `verus` (B), `core` (SP) or `compile_error` (S).
        #[rustfmt::skip]
        let rows = [
            ("item vstd", P, false),
            ("use vstd <- other", P, false),
            ("glob other", P, false),
            ("macro_use other", P, false),
            ("extern vstd <- other", P, true),
            ("extern vstd <- vstd", P, false),
            ("extern core <- asmkit", SP, true),
            ("extern std <- asmkit", SP, false),
            ("item verus", B, true),
            ("item vstd", B, true),
            ("use vstd <- vstd", B, false),
            ("use verus <- vstd::prelude::verus", B, false),
            ("use verus <- other::verus", B, true),
            ("extern vstd <- other", B, true),
            ("glob other", B, false),
            ("macro_use other", B, true),
            ("macro_use vstd", B, false),
            ("macro_use std", B, false),
            ("item compile_error", S, true),
            ("use compile_error <- core::compile_error", S, true),
            ("use compile_error <- asmkit::compile_error", S, true),
            ("use core <- asmkit", S, false),
            ("glob core::ptr", S, true),
            ("glob vstd::prelude", S, true),
            ("glob s other", S, false),
            ("macro_use std", S, false),
            ("macro_use vstd", S, true),
        ];
        for (row, kind, want) in rows {
            let name = match kind {
                P => "vstd",
                B => "verus",
                SP => "core",
                S => "compile_error",
            };
            let (what, rest) = row.split_once(' ').expect("a row");
            let split = |p: &str| -> Vec<String> {
                p.trim_start_matches("::")
                    .split("::")
                    .map(str::to_owned)
                    .collect()
            };
            let k = vec!["k".to_owned()];
            let b = match what {
                "item" => Binding {
                    name: rest.into(),
                    module: k.clone(),
                    ..Binding::default()
                },
                "use" | "extern" => {
                    let (name, path) = rest.split_once(" <- ").expect("a use");
                    Binding {
                        name: name.into(),
                        path: split(path),
                        extern_crate: what == "extern",
                        module: k.clone(),
                        ..Binding::default()
                    }
                }
                "glob" => {
                    let (module, path) = match rest.strip_prefix("s ") {
                        Some(p) => (vec!["k".to_owned(), "s".to_owned()], p),
                        None => (k.clone(), rest),
                    };
                    Binding {
                        path: split(path),
                        glob: true,
                        module,
                        ..Binding::default()
                    }
                }
                _ => Binding {
                    name: rest.into(),
                    macro_use: true,
                    module: k.clone(),
                    ..Binding::default()
                },
            };
            let got = breaks(&b, kind, name, &k);
            assert!(
                got == want,
                "{row} (for {name}): got = {got}, wanted = {want}"
            );
        }
    }

    /// pcre2-capi's `width::fn_ptr!`, as width.rs defines it.
    const FN_PTR_RULE: &str = r#"
macro_rules! fn_ptr {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident $(<$p:ident>)? ($fn:ty);
    ) => {
        $(#[$attr])*
        #[repr(transparent)]
        #[derive(Debug)]
        $vis struct $name $(<$p>)? (pub(crate) $fn);

        impl $(<$p>)? Clone for $name $(<$p>)? {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl $(<$p>)? Copy for $name $(<$p>)? {}

        impl $(<$p>)? $crate::width::FromC<Option<$fn>> for Option<$name $(<$p>)?> {
            #[inline(always)]
            fn from_c(f: Option<$fn>) -> Self {
                f.map($name)
            }
        }
    };
}
"#;

    /// What the C entry points of `src`, one file of crate `k`, reach.
    fn entry_walked(src: &str) -> (Counter, Vec<String>) {
        let c = walked(src);
        let funcs: Vec<inputs::Func> = c
            .fns
            .iter()
            .map(|f| inputs::Func {
                krate: &f.krate,
                file: &f.file,
                line: f.line,
                owner: inputs::base(&f.owner),
                name: &f.name,
                params: f.params,
                receiver: f.receiver,
                body: &f.body,
            })
            .collect();
        let roots = entry_roots(
            &c.fns,
            &["k"],
            &std::collections::BTreeSet::new(),
            Some("from_c"),
        );
        let parent = inputs::walk(
            &funcs,
            &c.traits,
            &c.aliases,
            &["k"],
            &|i| roots.contains(&i),
            false,
        );
        let mut names: Vec<String> = parent.keys().map(|&i| c.fns[i].name.clone()).collect();
        names.sort();
        (c, names)
    }

    // A C callback argument's `fn_ptr!` conversion is expanded from the
    // macro's own definition, so a helper its body calls is reached from
    // the entry points, whichever file holds the definition. Negative: a
    // helper nothing calls is not.
    #[test]
    fn fn_ptr_bodies_come_from_its_definition() {
        let rule = FN_PTR_RULE.replace("f.map($name)", "c_helper(); f.map($name)");
        assert_ne!(rule, FN_PTR_RULE);
        let src = r#"
width::fn_ptr! {
    /// A callback.
    pub(crate) struct Callback(unsafe extern "C" fn(i32) -> i32);
}
fn c_helper() {}
fn unused_helper() {}
"#
        .to_string()
            + &rule;
        let (c, reached) = entry_walked(&src);
        assert_eq!(c.errors, 0);
        assert!(reached.contains(&"from_c".to_string()), "{reached:?}");
        assert!(reached.contains(&"c_helper".to_string()), "{reached:?}");
        assert!(
            !reached.contains(&"unused_helper".to_string()),
            "{reached:?}"
        );
    }

    // Negative: the expansion fails closed. An invocation with no
    // definition, a definition of two rules, and a transcriber that uses a
    // metavariable the expansion does not bind are each an error.
    #[test]
    fn fn_ptr_expansion_fails_closed() {
        let call = "width::fn_ptr! { pub(crate) struct P(unsafe extern \"C\" fn()); }\n";
        assert_eq!(walked(call).errors, 1);
        let two = FN_PTR_RULE.replace("    };\n}", "    };\n    () => {};\n}");
        assert_ne!(two, FN_PTR_RULE);
        assert_eq!(walked(&(call.to_string() + &two)).errors, 1);
        let unbound = FN_PTR_RULE.replace("f.map($name)", "f.map($other)");
        assert_eq!(walked(&(call.to_string() + &unbound)).errors, 1);
        assert_eq!(walked(&(call.to_string() + FN_PTR_RULE)).errors, 0);
    }

    // The entry points: functions that export their symbol (`no_mangle`,
    // `export_name`, inside `unsafe(..)` or not, with an `extern "C"`,
    // `"C-unwind"` or bare `extern` ABI), a function a `const` hands out,
    // an exported body, and the `FromC` conversions (the identity one and
    // `fn_ptr!`'s). Negative: an `extern "C"` function that exports no
    // symbol, a method named like the exported body, a helper,
    // `fn_ptr!`'s `Clone` and a three-argument `from_c` are not; nor is
    // anything of another crate.
    #[test]
    fn entry_roots_are_the_c_entry_points() {
        let src = FN_PTR_RULE.to_string()
            + r#"
#[unsafe(no_mangle)]
extern "C" fn c_entry() {}
#[no_mangle]
pub unsafe extern fn bare_entry() {}
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn unwind_entry() {}
#[unsafe(export_name = "named_entry")]
extern "C" fn renamed_entry() {}
pub extern "C" fn not_exported() {}
unsafe extern "C-unwind" fn handed_out(n: usize) {}
const HANDED: P = P(handed_out);
unsafe fn body_a<U>() {}
struct S;
impl S { fn body_a(&self) {} }
impl Buf { fn from_c(p: usize, n: usize, t: usize) -> Self { Buf } }
fn helper() {}
impl<T> FromC<T> for T { fn from_c(t: T) -> T { t } }
width::fn_ptr! {
    /// A pointer.
    pub(crate) struct P(unsafe extern "C" fn());
}
"#;
        let c = walked(&src);
        assert_eq!(c.errors, 0);
        let exported: std::collections::BTreeSet<(PathBuf, String)> =
            [(PathBuf::from("crates/k/src/lib.rs"), "body_a".to_string())].into();
        let roots = |crates: &[&str], conversion: Option<&str>| {
            let mut got: Vec<String> = entry_roots(&c.fns, crates, &exported, conversion)
                .into_iter()
                .map(|i| format!("{}::{}", inputs::base(&c.fns[i].owner), c.fns[i].name))
                .collect();
            got.sort();
            got
        };
        let named = |crates: &[&str]| roots(crates, Some("from_c"));
        // With no conversion configured, the `from_c` methods root nothing.
        assert!(
            !roots(&["k"], None).iter().any(|r| r.ends_with("from_c")),
            "{:?}",
            roots(&["k"], None)
        );
        assert_eq!(
            named(&["k"]),
            [
                "::bare_entry",
                "::body_a",
                "::c_entry",
                "::handed_out",
                "::renamed_entry",
                "::unwind_entry",
                "Option::from_c",
                "T::from_c"
            ]
        );
        assert!(
            c.fns
                .iter()
                .any(|f| f.name == "clone" && f.owner.starts_with('P'))
        );
        assert_eq!(named(&["other"]), Vec::<String>::new());
    }

    // An export a `cfg_attr` applies is an entry point when a shipped
    // build can meet the predicate (#153): a feature
    // in `verus_sets` (`jit`) or outside it, a platform, nested and beside
    // other attributes, with a literal or a macro as the name. Negative:
    // one under a predicate no shipped build meets (`any()`, `test`,
    // `kani`, the Verus-only `feature = "verus"`, a conjunction needing
    // two values of one key) is not, nor is a function whose other
    // attributes only name an export (a doc comment, `inline`). An
    // unreadable predicate is an error, and leaves the function an entry
    // point; so is a non-C ABI under an export a shipped build applies.
    #[test]
    fn an_export_under_cfg_attr_is_an_entry_where_a_shipped_build_applies_it() {
        let c = walked(
            r#"
#[cfg_attr(feature = "export-symbols", unsafe(export_name = prefix!(BZ2_a)))]
pub extern "C" fn outside_sets_macro() {}
#[cfg_attr(feature = "jit", unsafe(export_name = prefix!(BZ2_b)))]
pub extern "C" fn in_sets_macro() {}
#[cfg_attr(feature = "export-symbols", unsafe(export_name = "BZ2_c"))]
pub extern "C" fn outside_sets_literal() {}
#[cfg_attr(feature = "jit", unsafe(export_name = "BZ2_d"))]
pub extern "C" fn in_sets_literal() {}
#[cfg_attr(feature = "export-symbols", export_name = "BZ2_e")]
pub extern "C" fn bare_export_name() {}
#[cfg_attr(unix, unsafe(no_mangle))]
pub extern "C" fn platform_no_mangle() {}
#[cfg_attr(feature = "jit", cfg_attr(unix, inline, unsafe(no_mangle)))]
pub extern "C" fn nested_beside() {}
#[cfg_attr(any(), unsafe(no_mangle))]
pub extern "C" fn never() {}
#[cfg_attr(test, unsafe(export_name = prefix!(BZ2_f)))]
pub extern "C" fn test_only() {}
#[cfg_attr(kani, unsafe(no_mangle))]
pub extern "C" fn kani_only() {}
#[cfg_attr(feature = "verus", unsafe(no_mangle))]
pub extern "C" fn verus_only() {}
#[cfg_attr(feature = "jit", cfg_attr(test, unsafe(no_mangle)))]
pub extern "C" fn nested_test() {}
#[cfg_attr(all(target_arch = "x86_64", target_arch = "aarch64"), unsafe(no_mangle))]
pub extern "C" fn two_arches() {}
/// Not `#[no_mangle]`.
#[cfg_attr(feature = "jit", inline)]
pub extern "C" fn names_only() {}
"#,
        );
        assert_eq!(c.errors, 0);
        let mut entries: Vec<&str> = c
            .fns
            .iter()
            .filter(|f| f.entry)
            .map(|f| f.name.as_str())
            .collect();
        entries.sort_unstable();
        assert_eq!(
            entries,
            [
                "bare_export_name",
                "in_sets_literal",
                "in_sets_macro",
                "nested_beside",
                "outside_sets_literal",
                "outside_sets_macro",
                "platform_no_mangle"
            ]
        );
        let bad = walked(
            "#[cfg_attr(feature = \"jit\", unsafe(no_mangle))]\nfn rust_abi() {}\n\
             #[cfg_attr(test, unsafe(no_mangle))]\nfn test_rust_abi() {}\n\
             #[cfg_attr(nonsense(x), unsafe(no_mangle))]\nextern \"C\" fn unreadable() {}\n",
        );
        assert_eq!(bad.errors, 2);
        assert!(bad.fns.iter().any(|f| f.name == "unreadable" && f.entry));
        assert!(
            bad.fns
                .iter()
                .any(|f| f.name == "test_rust_abi" && !f.entry)
        );
    }

    // A function a `static` or `const` passes by name, bare or by path, is
    // an entry point. Negative: one it only calls is not, though a `<` or
    // a closure in the call's arguments leaves the arity unknown.
    #[test]
    fn a_static_hands_out_only_what_it_passes_by_name() {
        let c = walked(
            "fn passed() {}
fn pathed() {}
fn helper(x: u8) -> u8 { x }
fn run(f: u8) -> u8 { f }
fn make<A, B>() -> u8 { 0 }
static S: u8 = helper(make::<u8, u16>());
static R: u8 = run(|x| x);
static Q: P = P(passed);
const C: P = P(self::pathed);
",
        );
        assert_eq!(c.errors, 0, "errors: got = {}, wanted = 0", c.errors);
        let got: Vec<&str> = c
            .fns
            .iter()
            .filter(|f| f.entry)
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(
            got,
            ["passed", "pathed"],
            "entry points: got = {got:?}, wanted = [passed, pathed]"
        );
    }

    // A configured root that matches a function of the configured crates
    // passes, with an empty table (a port's first CD1). Negative: a root
    // misspelled, moved to another file, of another owner, or in a crate
    // the walk does not follow fails, though the table has no rows for a
    // stale row to fail on.
    #[test]
    fn a_root_that_matches_nothing_fails() {
        let c = walked("struct R;\nimpl R { fn new() {} }\nfn other() {}\n");
        let root = |file: &str, owner: Option<&str>, name: &str| config::Root {
            file: file.into(),
            owner: owner.map(Into::into),
            name: name.into(),
        };
        let run = |roots: Vec<config::Root>, crates: &[&str]| {
            compile_inputs(
                Path::new("/nonexistent"),
                &config::Inputs {
                    table: "CD1.md".into(),
                    crates: crates.iter().map(|k| k.to_string()).collect(),
                    roots,
                    lazy: false,
                    describe: "the roots".into(),
                },
                &c,
            )
        };
        let good = || {
            vec![
                root("k/src/lib.rs", Some("R"), "new"),
                root("lib.rs", None, "other"),
            ]
        };
        assert!(
            run(good(), &["k"]),
            "matching roots: got = fail, wanted = ok"
        );
        for (bad, crates) in [
            (root("k/src/lib.rs", Some("R"), "nwe"), &["k"]),
            (root("k/src/moved.rs", Some("R"), "new"), &["k"]),
            (root("k/src/lib.rs", Some("S"), "new"), &["k"]),
            (root("k/src/lib.rs", None, "other"), &["other"]),
        ] {
            let mut roots = good();
            roots.push(bad);
            assert!(
                !run(roots, crates),
                "unmatched root: got = ok, wanted = fail"
            );
        }
    }

    // An exported body defined as a free function in the block's file
    // matches. Negative: one the file does not define (moved elsewhere and
    // imported), or defines only as a method, is an error.
    #[test]
    fn unmatched_exports_are_errors() {
        let c = walked("fn body_a<U>() {}\nstruct S;\nimpl S { fn body_c(&self) {} }\n");
        let file = PathBuf::from("crates/k/src/lib.rs");
        let exported: std::collections::BTreeSet<(PathBuf, String)> =
            ["body_a", "body_b", "body_c"]
                .into_iter()
                .map(|n| (file.clone(), n.to_string()))
                .collect();
        let got = unmatched_exports(&c.fns, &["k"], &exported, "exports");
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(
            got[0].contains("an exports! block exports `body_b`"),
            "{got:?}"
        );
        assert!(got[1].contains("exports `body_c`"), "{got:?}");
        assert_eq!(
            unmatched_exports(&c.fns, &["other"], &exported, "exports").len(),
            3
        );
    }

    // Negative: a function that exports its symbol with an ABI C cannot
    // call is an error.
    #[test]
    fn exported_symbol_needs_a_c_abi() {
        assert_eq!(walked("#[no_mangle]\nextern \"C\" fn ok() {}\n").errors, 0);
        assert_eq!(
            walked("#[no_mangle]\nextern \"Rust\" fn bad() {}\n").errors,
            1
        );
        assert_eq!(walked("#[unsafe(no_mangle)]\nfn bad() {}\n").errors, 1);
    }

    // A method of an `impl` written inside a function body (a guard's
    // `Drop`, as `without_cancellation`'s) is recorded, with its owner and
    // the function's class, and the entry walk reaches it. Negative: such
    // a method in a function nothing calls is not reached.
    #[test]
    fn nested_impl_methods_are_walked() {
        let src = r#"
#[unsafe(no_mangle)]
extern "C" fn entry() { guarded() }
fn guarded() {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) { restore_state() }
    }
    let _r = Restore(true);
}
fn restore_state() {}
fn uncalled() {
    struct Other;
    impl Other {
        fn method_never_called(&self) {}
    }
}
"#;
        let (c, reached) = entry_walked(src);
        assert_eq!(c.errors, 0);
        let drop = c.fns.iter().find(|f| f.name == "drop");
        assert!(
            drop.is_some_and(|f| inputs::base(&f.owner) == "Restore" && f.class == Class::Outside),
            "{:?}",
            c.fns
                .iter()
                .map(|f| (&f.owner, &f.name))
                .collect::<Vec<_>>()
        );
        for name in ["guarded", "drop", "restore_state"] {
            assert!(reached.contains(&name.to_string()), "{name}: {reached:?}");
        }
        assert!(c.fns.iter().any(|f| f.name == "method_never_called"));
        assert!(
            !reached.contains(&"method_never_called".to_string()),
            "{reached:?}"
        );
    }

    #[test]
    fn verified_recursion_needs_a_row() {
        let f = FnRef {
            krate: "k".into(),
            file: PathBuf::from("crates/k/src/a.rs"),
            line: 1,
            owner: String::new(),
            name: "walk".into(),
            class: Class::Verified,
            calls: Vec::new(),
            loops: LoopShape::default(),
            body: inputs::Body::default(),
            params: 0,
            receiver: false,
            entry: false,
        };
        assert!(documented("| `fn walk` | `crates/k/src/a.rs` | 3 |", &f));
        // Negative: another file, another name, or not a table row.
        assert!(!documented("| `fn walk` | `crates/k/src/b.rs` | 3 |", &f));
        assert!(!documented("| `fn walker` | `crates/k/src/a.rs` | 3 |", &f));
        assert!(!documented("`fn walk` in `crates/k/src/a.rs`", &f));
    }

    /// The markers read through `cfg_attr` go through the one expander,
    /// [`applied_attrs`]. `discharged_by = "kani"` counts nested or beside
    /// other attributes, and only under predicates that hold in no build
    /// whatever is declared; under one some build may meet it fails
    /// closed. An `external_body` a `cfg_attr` applies counts whatever its
    /// predicate (the function is trusted), as does an
    /// `assume_unsupported_attributes` (its seam is listed), and prose in a
    /// doc comment is no attribute.
    #[test]
    fn markers_are_read_through_every_cfg_attr() {
        let attrs = |src: &str| {
            let f = parse_file(&format!("{src}\nfn f() {{}}\n")).unwrap();
            let Item::Fn(func) = &f.items[0] else {
                panic!("not a function: {src}")
            };
            func.attrs.clone()
        };
        for (src, want) in [
            ("#[cfg_attr(any(), discharged_by = \"kani\")]", true),
            (
                "#[cfg_attr(unix, cfg_attr(any(), discharged_by = \"kani\"))]",
                true,
            ),
            (
                "#[cfg_attr(any(), cfg_attr(windows, discharged_by = \"kani\"))]",
                true,
            ),
            ("#[cfg_attr(any(), inline, discharged_by = \"kani\")]", true),
            (
                "#[cfg_attr(all(target_arch = \"x86_64\", target_arch = \"aarch64\"), discharged_by = \"kani\")]",
                true,
            ),
            ("#[cfg_attr(windows, discharged_by = \"kani\")]", false),
            (
                "#[cfg_attr(unix, cfg_attr(windows, discharged_by = \"kani\"))]",
                false,
            ),
            ("#[cfg_attr(unix, inline, discharged_by = \"kani\")]", false),
            ("#[cfg_attr(test, discharged_by = \"kani\")]", false),
            ("#[cfg_attr(any(), discharged_by = \"other\")]", false),
        ] {
            let got = is_discharged(&attrs(src));
            assert_eq!(got, want, "discharged {src}: got = {got}, wanted = {want}");
        }
        for (src, want) in [
            ("#[verifier::external_body]", true),
            (
                "#[cfg_attr(unix, cfg_attr(feature = \"verus\", verifier::external_body))]",
                true,
            ),
            (
                "#[cfg_attr(windows, inline, verus_verify(external_body))]",
                true,
            ),
            ("/// Not `external_body`: it is verified.", false),
            ("#[verifier::external_fn_specification]", false),
        ] {
            let got = is_external(&attrs(src));
            assert_eq!(got, want, "external {src}: got = {got}, wanted = {want}");
        }
        for (src, want) in [
            ("#[verifier::assume_unsupported_attributes]", true),
            ("#[verus::verifier::assume_unsupported_attributes]", true),
            (
                "#[cfg_attr(windows, inline, verifier::assume_unsupported_attributes)]",
                true,
            ),
            ("#[verifier(assume_unsupported_attributes)]", true),
            (
                "#[cfg_attr(unix, verifier(assume_unsupported_attributes))]",
                true,
            ),
            // Not a form Verus reads, but listed: the check fails closed.
            ("#[verifier::assume_unsupported_attributes(x)]", true),
            ("#[unsafe(no_mangle)]", false),
            ("#[verifier(external_body)]", false),
            ("/// Not `assume_unsupported_attributes`.", false),
            (
                "#[doc = \"verifier::assume_unsupported_attributes\"]",
                false,
            ),
        ] {
            let got = assumes_attributes(&attrs(src));
            assert_eq!(got, want, "assumes {src}: got = {got}, wanted = {want}");
        }
        for (src, want) in [
            ("#[verifier::exec_allows_no_decreases_clause]", true),
            ("#[verifier(exec_allows_no_decreases_clause)]", true),
            (
                "#[cfg_attr(windows, verifier::exec_allows_no_decreases_clause)]",
                true,
            ),
            ("#[verifier::loop_isolation(false)]", false),
            ("/// Not `exec_allows_no_decreases_clause`.", false),
            (
                "#[doc = \"verifier::exec_allows_no_decreases_clause\"]",
                false,
            ),
        ] {
            let got = allows_no_decreases(&attrs(src));
            assert_eq!(
                got, want,
                "no decreases {src}: got = {got}, wanted = {want}"
            );
        }
    }

    /// `exec_allows_no_decreases_clause` lets Verus check exec functions
    /// with no `decreases`, so nothing proves they terminate: each exec
    /// function it covers (on the function, or on the `impl`, trait or
    /// inline module around it, outer or inner) is trusted, not verified.
    /// A function beside them keeps its class, as do ghost functions,
    /// and code outside `verus!`. On an out-of-line `mod x;`, whose file
    /// the counter walks on its own, it fails the run.
    #[test]
    fn no_decreases_functions_are_trusted() {
        let c = walked(
            "verus! {\n\
            #[verifier::exec_allows_no_decreases_clause]\n\
            fn on_fn(n: u32) { loop {} }\n\
            fn beside() {}\n\
            struct S;\n\
            #[verifier::exec_allows_no_decreases_clause]\n\
            impl S { fn on_impl(&self) {} fn on_impl2(&self) {} }\n\
            impl S { fn plain_impl(&self) {} }\n\
            #[verifier::exec_allows_no_decreases_clause]\n\
            trait T { fn on_trait(&self) {} }\n\
            #[verifier::exec_allows_no_decreases_clause]\n\
            mod outer { fn on_mod() {} }\n\
            mod inner { #![verifier::exec_allows_no_decreases_clause] fn on_inner() {} }\n\
            #[verifier::exec_allows_no_decreases_clause]\n\
            proof fn ghostly() {}\n\
            fn after() {}\n\
            #[cfg_attr(any(), discharged_by = \"kani\")]\n\
            #[verifier::external_body]\n\
            fn marked() {}\n\
            #[verifier::exec_allows_no_decreases_clause]\n\
            #[cfg_attr(any(), discharged_by = \"kani\")]\n\
            #[verifier::external_body]\n\
            fn marked_no_decreases() {}\n\
            #[verifier::exec_allows_no_decreases_clause]\n\
            mod marked_mod {\n\
            #[cfg_attr(any(), discharged_by = \"kani\")]\n\
            #[verifier::external_body]\n\
            fn marked_in_mod() {}\n\
            #[cfg_attr(any(), discharged_by = \"kani\")]\n\
            #[verifier::external_body]\n\
            proof fn marked_proof() {}\n\
            #[cfg(verus_keep_ghost)]\n\
            fn verus_only() {}\n\
            #[cfg(verus_keep_ghost)]\n\
            #[cfg_attr(any(), discharged_by = \"kani\")]\n\
            #[verifier::external_body]\n\
            fn verus_only_marked() {}\n\
            }\n\
            #[cfg(verus_keep_ghost)]\n\
            fn verus_only_plain() {}\n\
            }\n\
            #[verifier::exec_allows_no_decreases_clause]\n\
            fn not_verus() {}\n",
        );
        assert_eq!(c.errors, 0, "errors: got = {}, wanted = 0", c.errors);
        let t = &c.tallies["k"];
        // Trusted: the six above, the two marked exec functions it covers,
        // which a Kani record does not discharge, and the two Verus-only
        // exec functions it covers, which a shipped proof could rest on.
        // Discharged: `marked`, and `marked_proof`, a ghost function the
        // attribute leaves alone. `verus_only_plain` stays out of the
        // tally, as a verified Verus-only function does.
        assert_eq!(
            t.fns,
            [3, 10, 1, 2],
            "verified, trusted, outside, discharged: got = {:?}, wanted = [3, 10, 1, 2]",
            t.fns
        );
        assert_eq!(t.ghost, 1, "ghost: got = {}, wanted = 1", t.ghost);
        // `mod elsewhere;` under the attribute, on it or on an inline
        // module around it (outer or inner), is one error each; with no
        // attribute, none.
        for (src, want) in [
            (
                "#[verifier::exec_allows_no_decreases_clause]\nmod elsewhere;\n",
                1,
            ),
            (
                "#[verifier::exec_allows_no_decreases_clause]\nmod outer { mod elsewhere; }\n",
                1,
            ),
            (
                "mod outer { #![verifier::exec_allows_no_decreases_clause] mod elsewhere; }\n",
                1,
            ),
            ("mod outer { mod elsewhere; }\nmod elsewhere2;\n", 0),
        ] {
            let got = walked(src).errors;
            assert_eq!(got, want, "{src:?}: got = {got} errors, wanted = {want}");
        }
    }

    #[test]
    fn functions_are_classed_as_verus_sees_them() {
        let (t, errors) = count(
            r#"
            verus! {
                fn verified() {}
                #[verifier::external_body]
                fn external_body() {}
                #[verifier::external]
                fn external() {}
                fn assumes() { assume(false); }
                fn admits() { if true { admit(); } }
                spec fn ghost() -> bool { true }
                #[verifier::external_body]
                spec fn external_ghost() -> bool { true }
                proof fn lemma() {}
                axiom fn axiom() ensures true;
                #[verifier::external_body]
                proof fn external_lemma() {}
                proof fn admitted() { admit(); }
                fn mints() -> Tracked<u8> { Tracked::assume_new() }
                proof fn mints_in_proof() { let tracked t: u8 = Tracked::<u8>::assume_new().get(); }
                #[cfg(test)]
                fn test_only() {}
                #[cfg(kani)]
                fn kani_only() {}
                #[cfg_attr(any(), discharged_by = "kani")]
                #[verifier::external_body]
                fn discharged() {}
                #[cfg_attr(any(), discharged_by = "kani")]
                axiom fn discharged_axiom() ensures true;
                #[cfg_attr(any(), discharged_by = "kani")]
                proof fn discharged_lemma() {}
                #[cfg_attr(any(), discharged_by = "kani")]
                fn discharged_but_verified() {}
                #[cfg_attr(any(), discharged_by = "other")]
                #[verifier::external_body]
                fn other_tool() {}
                // Built only for Verus: checked, never shipped, so none
                // of these is counted, whatever holds it.
                #[cfg(verus_keep_ghost)]
                fn verus_only() {}
                #[cfg(verus_keep_ghost)]
                impl Machine { fn verus_only_method(&self) {} }
                #[cfg(verus_keep_ghost)]
                trait VerusOnly { fn verus_only_default(&self) {} }
                #[cfg(any(test, verus_keep_ghost))]
                mod verus_only_mod { fn verus_only_inner() {} }
                // Built only for Verus but trusted: a shipped proof can
                // rest on it, so each counts as trusted.
                #[cfg(verus_keep_ghost)]
                #[verifier::external_body]
                fn verus_only_external() {}
                #[cfg(verus_keep_ghost)]
                proof fn verus_only_admits() { admit(); }
                #[cfg(verus_keep_ghost)]
                mod verus_only_seams { #[verifier::external_body] fn inner_external() {} }
                // Built only for Verus and ghost but no trusted proof, as
                // shipped: none is counted.
                #[cfg(verus_keep_ghost)]
                #[verifier::external_body]
                spec fn verus_only_external_ghost() -> bool { true }
                #[cfg(verus_keep_ghost)]
                mod verus_only_ghosts { #[verifier::external_body] spec fn inner_ghost() -> bool { true } }
            }
            fn outside() {}
            #[cfg(not(test))]
            fn shipped_outside() {}
            #[cfg(test)]
            fn test_only_outside() {}
            #[cfg(test)]
            mod tests { fn helper() {} }
            #[cfg(kani)]
            mod proofs { #[kani::proof] fn harness() {} }
            "#,
        );
        assert_eq!(errors, 0, "errors: got = {errors}, wanted = 0");
        assert_eq!(
            t.fns,
            [2, 13, 2, 2],
            "verified, trusted, outside, discharged: got = {:?}, wanted = [2, 13, 2, 2]",
            t.fns
        );
        assert_eq!(t.ghost, 4, "ghost: got = {}, wanted = 4", t.ghost);
    }

    /// `assume_unsupported_attributes` trusts the linkage, not the body:
    /// each function keeps the class it has without it, and every one
    /// inside `verus!` is kept, at its `fn` line, for the trust summary.
    #[test]
    fn assumed_attributes_keep_the_class_and_are_listed() {
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from("lib.rs"),
            module: vec!["k".into()],
            ..counter()
        };
        let src = "verus! {\n\
            #[unsafe(no_mangle)]\n\
            #[verifier::assume_unsupported_attributes]\n\
            pub extern \"C\" fn exported() {}\n\
            #[cfg_attr(any(), discharged_by = \"kani\")]\n\
            #[unsafe(export_name = \"e\")]\n\
            #[verifier::assume_unsupported_attributes]\n\
            #[verifier::external_body]\n\
            pub extern \"C\" fn exported_and_marked() {}\n\
            #[unsafe(export_name = \"t\")]\n\
            #[verifier::assume_unsupported_attributes]\n\
            #[verifier::external_body]\n\
            pub extern \"C\" fn exported_trusted() {}\n\
            fn plain() {}\n\
            }\n\
            #[unsafe(no_mangle)]\n\
            #[verifier::assume_unsupported_attributes]\n\
            pub extern \"C\" fn outside() {}\n";
        c.items(&parse_file(src).expect("parses").items, false);
        assert_eq!(c.errors, 0, "errors: got = {}, wanted = 0", c.errors);
        let t = c.tallies.remove("k").unwrap_or_default();
        assert_eq!(
            t.fns,
            [2, 1, 1, 1],
            "verified, trusted, outside, discharged: got = {:?}, wanted = [2, 1, 1, 1]",
            t.fns
        );
        let want = ["lib.rs:4", "lib.rs:9", "lib.rs:13"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            c.assumed, want,
            "assumed: got = {:?}, wanted = {want:?}",
            c.assumed
        );
    }

    /// A marked function built only for Verus is discharged as a shipped
    /// one is, so it needs a Kani record too. The case is S6's usize fill
    /// axiom (`crates/pcre2-unicode/src/vspec.rs`) moved under
    /// `cfg(verus_keep_ghost)`, against `verus/seams/kani-s6.md` with and
    /// without its quote.
    #[test]
    fn a_verus_only_discharged_function_needs_a_record() {
        let file = "crates/pcre2-unicode/src/vspec.rs";
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from(file),
            module: vec!["k".into()],
            ..counter()
        };
        c.items(
            &parse_file(
                r#"
                verus! {
                pub uninterp spec fn fill_copies<T>() -> bool;
                #[cfg(verus_keep_ghost)]
                #[cfg_attr(any(), discharged_by = "kani")]
                #[verifier::external_body]
                pub proof fn axiom_fill_copies_usize()
                    ensures
                        fill_copies::<usize>(),
                {
                }
                #[cfg_attr(any(), discharged_by = "kani")]
                #[verifier::external_body]
                pub proof fn axiom_fill_copies_option_usize()
                    ensures
                        fill_copies::<Option<usize>>(),
                {
                }
                }
                "#,
            )
            .expect("parses")
            .items,
            false,
        );
        assert_eq!(c.errors, 0, "errors: got = {}, wanted = 0", c.errors);
        let t = c.tallies.remove("k").unwrap_or_default();
        assert_eq!(
            t.fns,
            [0, 0, 0, 2],
            "fns: got = {:?}, wanted = [0, 0, 0, 2]",
            t.fns
        );
        let d = |n: &str| (PathBuf::from(file), n.to_owned());
        let want = vec![
            d("axiom_fill_copies_usize"),
            d("axiom_fill_copies_option_usize"),
        ];
        assert_eq!(
            c.discharged, want,
            "discharged: got = {:?}, wanted = {want:?}",
            c.discharged
        );
        let usize_quote = "#[cfg_attr(any(), discharged_by = \"kani\")]\n\
            #[verifier::external_body]\n\
            pub proof fn axiom_fill_copies_usize()\n    ensures\n        fill_copies::<usize>(),\n{\n}\n";
        let option_quote = "#[cfg_attr(any(), discharged_by = \"kani\")]\n\
            #[verifier::external_body]\n\
            pub proof fn axiom_fill_copies_option_usize()\n    ensures\n        fill_copies::<Option<usize>>(),\n{\n}\n";
        let record = |quotes: &str| {
            format!(
                "Seam S6 of `verus/SEAMS.md` (`{file}`).\n\n## 1. Contract\n\n```rust\n{quotes}```\n"
            )
        };
        let got = unrecorded(
            &c.discharged,
            &[record(&format!("{usize_quote}{option_quote}"))],
        );
        assert!(
            got.is_empty(),
            "with both quotes: got = {got:?}, wanted = []"
        );
        let got = unrecorded(&c.discharged, &[record(option_quote)]);
        let want = vec![d("axiom_fill_copies_usize")];
        assert_eq!(
            got, want,
            "without the usize quote: got = {got:?}, wanted = {want:?}"
        );
    }

    /// Code under an item built only for Verus (`shipped` false) counts
    /// nowhere in the tally, even a plain verified fn, but its rlimit is
    /// still registered for the headroom check.
    #[test]
    fn unshipped_code_is_not_tallied_but_keeps_its_rlimit() {
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from("lib.rs"),
            module: vec!["k".into()],
            shipped: false,
            ..counter()
        };
        c.items(
            &parse_file("verus! { #[verifier::rlimit(20)] fn plain() {} }")
                .expect("parses")
                .items,
            false,
        );
        assert_eq!(c.errors, 0, "errors: got = {}, wanted = 0", c.errors);
        let t = c.tallies.remove("k").unwrap_or_default();
        assert_eq!(
            t.fns,
            [0, 0, 0, 0],
            "fns: got = {:?}, wanted = [0, 0, 0, 0]",
            t.fns
        );
        let got: Vec<_> = c.limits.into_iter().map(|l| (l.name, l.units)).collect();
        let want = vec![("k::plain".to_owned(), Some(20.0))];
        assert_eq!(got, want, "limits: got = {got:?}, wanted = {want:?}");
    }

    /// A trusted fn under an item built only for Verus (`shipped` false)
    /// still counts as trusted, a seam a shipped proof can rest on, but
    /// joins no list of shipped functions.
    #[test]
    fn unshipped_trusted_code_is_tallied_as_trusted() {
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from("lib.rs"),
            module: vec!["k".into()],
            shipped: false,
            ..counter()
        };
        c.items(
            &parse_file("verus! {\n#[verifier::external_body]\nfn seam() {\n}\n}")
                .expect("parses")
                .items,
            false,
        );
        assert_eq!(c.errors, 0, "errors: got = {}, wanted = 0", c.errors);
        let t = c.tallies.remove("k").unwrap_or_default();
        assert_eq!(
            t.fns,
            [0, 1, 0, 0],
            "fns: got = {:?}, wanted = [0, 1, 0, 0]",
            t.fns
        );
        assert_eq!(
            t.lines,
            [0, 2, 0, 0],
            "lines: got = {:?}, wanted = [0, 2, 0, 0]",
            t.lines
        );
        assert!(
            c.fns.is_empty(),
            "fn list: got = {} entries, wanted = 0",
            c.fns.len()
        );
    }

    #[test]
    fn the_floor_file_parses_and_rejects_what_it_should() {
        let got = Figures::default().parse(
            "# c\nfunctions 8.4 # now\n\nlines 10\nlowered lines 10 the bulk test helpers moved in\n",
        );
        assert_eq!(
            got,
            Ok(FloorFile {
                floor: Floor([Some(84), Some(100), None, None]),
                lowered: vec![("lines".into(), 100)],
            }),
            "floor: got = {got:?}, wanted = 8.4 and 10.0, lines lowered to 10.0"
        );
        let got = Figures::default().parse(
            "functions 8.4\nlines 10\nchecked-functions 20.5\nchecked-lines 30\nlowered checked-lines 30 a reason\n",
        );
        assert_eq!(
            got,
            Ok(FloorFile {
                floor: Floor([Some(84), Some(100), Some(205), Some(300)]),
                lowered: vec![("checked-lines".into(), 300)],
            }),
            "floor: got = {got:?}, wanted = all four figures, checked-lines lowered to 30.0"
        );
        let full = got.expect("parses").floor;
        assert_eq!(
            Figures::default().missing(&full),
            None,
            "missing of a full file: got = {:?}, wanted = None",
            Figures::default().missing(&full)
        );
        let partial = Figures::default()
            .parse("functions 8.4\nlines 10\nchecked-lines 30\n")
            .expect("parses")
            .floor;
        assert_eq!(
            Figures::default().missing(&partial),
            Some("checked-functions"),
            "missing: got = {:?}, wanted = checked-functions",
            Figures::default().missing(&partial)
        );
        for bad in [
            "functions 8.4\nlines 1\nchecked-functions 1\nchecked-functions 2\n",
            "checked-functions 1\nchecked-lines 1\n",
            "functions 8.4\nlines 1\nchecked 3\n",
            "functions 8.4\nlines 1\nlowered lines 1\n",
            "functions 8.4\nlines 1\nlowered branches 1 why\n",
            "functions 8.4\n",
            "functions 8.4\nlines 1.25\n",
            "functions 8.4\nlines -1\n",
            "functions 8.4\nlines 100.1\n",
            "functions 8.4\nfunctions 8.5\nlines 1\n",
            "functions 8.4\nlines 1\nbranches 3\n",
            "functions 8.4 9\nlines 1\n",
            "functions .5\nlines 1\n",
        ] {
            assert!(
                Figures::default().parse(bad).is_err(),
                "{bad:?}: got = Ok, wanted = Err"
            );
        }
    }

    #[test]
    fn lowering_the_floor_needs_a_reason_for_that_figure() {
        let head = |text: &str| Figures::default().parse(text).expect("parses");
        let base = head("functions 13.7\nlines 13.1\nlowered functions 13.4 an old reason\n");
        let cases = [
            (
                "functions 13.4\nlines 13.1\nlowered functions 13.4 an old reason\n",
                false,
            ),
            ("functions 13.7\nlines 13.1\n", true),
            ("functions 14.0\nlines 13.1\n", true),
            ("functions 13.6\nlines 13.1\n", false),
            (
                "functions 13.6\nlines 13.1\nlowered functions 13.6 x moved out\n",
                true,
            ),
            // A reworded reason does not make a line new: the base already
            // names that figure and value.
            (
                "functions 13.4\nlines 13.1\nlowered functions 13.4 an old reason\nlowered functions 13.4 a new reason\n",
                false,
            ),
            (
                "functions 13.6\nlines 13.1\nlowered functions 13.5 stale\n",
                false,
            ),
            (
                "functions 13.6\nlines 13.1\nlowered lines 13.6 wrong figure\n",
                false,
            ),
        ];
        for (text, want) in cases {
            let (ok, report) = Figures::default().lowering(&head(text), &base);
            assert_eq!(ok, want, "{text:?}: got = {ok}, wanted = {want}\n{report}");
        }
        // A base without the checked figures (from before Kani counted) is
        // compared on the two it has; one with them, on all four.
        let checked = "checked-functions 20.0\nchecked-lines 30.0\n";
        let (ok, report) = Figures::default().lowering(
            &head(&format!("functions 13.7\nlines 13.1\n{checked}")),
            &base,
        );
        assert!(
            ok,
            "against a base without them: got = fail, wanted = ok\n{report}"
        );
        let base = head(&format!("functions 13.7\nlines 13.1\n{checked}"));
        for (text, want) in [
            ("checked-functions 20.0\nchecked-lines 30.0\n", true),
            ("checked-functions 19.9\nchecked-lines 30.0\n", false),
            (
                "checked-functions 19.9\nchecked-lines 30.0\nlowered checked-functions 19.9 x moved out\n",
                true,
            ),
            (
                "checked-functions 20.0\nchecked-lines 29.0\nlowered functions 29.0 wrong figure\n",
                false,
            ),
        ] {
            let text = format!("functions 13.7\nlines 13.1\n{text}");
            let (ok, report) = Figures::default().lowering(&head(&text), &base);
            assert_eq!(ok, want, "{text:?}: got = {ok}, wanted = {want}\n{report}");
        }
    }

    #[test]
    fn the_base_floor_fails_closed() {
        let dir = std::env::temp_dir().join(format!("verus-count-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("verus")).expect("mkdir");
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args([
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@t",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .status()
                .is_ok_and(|s| s.success());
            assert!(ok, "git {args:?}: got = failure, wanted = success");
        };
        git(&["init", "-q"]);
        fs::write(dir.join("a"), "a").expect("write");
        git(&["add", "a"]);
        git(&["commit", "-q", "-m", "no floor"]);
        fs::write(dir.join("verus/FLOOR"), "functions 1.0\nlines 2.0\n").expect("write");
        git(&["add", "verus/FLOOR"]);
        git(&["commit", "-q", "-m", "floor"]);
        let got = base_file(&dir, "HEAD", "verus/FLOOR");
        assert_eq!(
            got,
            Ok(Some("functions 1.0\nlines 2.0\n".into())),
            "HEAD: got = {got:?}, wanted = its FLOOR"
        );
        let got = base_file(&dir, "HEAD^", "verus/FLOOR");
        assert_eq!(got, Ok(None), "HEAD^: got = {got:?}, wanted = no FLOOR");
        for rev in ["HEAD~5", "", "no-such-branch"] {
            let got = base_file(&dir, rev, "verus/FLOOR");
            assert!(got.is_err(), "{rev:?}: got = {got:?}, wanted = an error");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    /// `--base-rev`'s copy of FLOOR, RLIMIT and REACH is read at the path
    /// the base's configuration gives it, so a head that points
    /// `[floor] path`, `[rlimit] path` or `[reach] path` at a file the
    /// base lacks is still compared with the base's file. Negative: a
    /// file whose section the base's configuration has, but the base
    /// lacks, is an error. A `[reach]` the base leaves out, or no base
    /// configuration, keeps the head's path, and an absent file there or
    /// at an undeclared default is nothing to compare.
    #[test]
    fn the_base_copy_is_read_at_the_base_configurations_path() {
        use GateFile::{Floor, Reach, Rlimit};
        let dir = std::env::temp_dir().join(format!("verus-count-copy-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("verus")).expect("mkdir");
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args([
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@t",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .status()
                .is_ok_and(|s| s.success());
            assert!(ok, "git {args:?}: got = failure, wanted = success");
        };
        git(&["init", "-q"]);
        fs::write(dir.join("a"), "a").expect("write");
        git(&["add", "a"]);
        git(&["commit", "-q", "-m", "no gate files"]);
        for f in ["FLOOR", "RLIMIT", "REACH"] {
            fs::write(dir.join("verus").join(f), f).expect("write");
        }
        git(&["add", "verus"]);
        git(&["commit", "-q", "-m", "gate files"]);
        let declared = || {
            Config::parse(
                "[crates]\ndir = \"crates\"\n[floor]\npath = \"verus/FLOOR\"\n[rlimit]\npath = \"verus/RLIMIT\"\n[reach]\npath = \"verus/REACH\"\n",
            )
            .expect("parses")
        };
        let bare = || Config::parse("[crates]\ndir = \"crates\"\n").expect("parses");
        let base = |rev: &str, cfg: Option<Config>| Base {
            rev: rev.into(),
            cfg,
        };
        let copy = |b: &Base, head: &str, at: GateFile| b.copy(&dir, head, at);
        // The head points each path at a file the base lacks: the base's
        // own file is read, with or without the section for FLOOR and
        // RLIMIT, whose default path the base then reads.
        for cfg in [declared, bare] {
            for (head, at, want) in [
                ("verus/FLOOR.old", Floor, "FLOOR"),
                ("verus/RLIMIT.old", Rlimit, "RLIMIT"),
            ] {
                let got = copy(&base("HEAD", Some(cfg())), head, at);
                let path = format!("verus/{want}");
                assert_eq!(
                    got,
                    Ok((path.clone(), Some(want.into()))),
                    "{head}: got = {got:?}, wanted = {path}"
                );
            }
        }
        let got = copy(&base("HEAD", Some(declared())), "verus/REACH.old", Reach);
        assert_eq!(
            got,
            Ok(("verus/REACH".into(), Some("REACH".into()))),
            "[reach] path: got = {got:?}, wanted = verus/REACH"
        );
        // The base's configuration has the section, but the base not the
        // file.
        for (at, path) in [
            (Floor, "verus/FLOOR"),
            (Rlimit, "verus/RLIMIT"),
            (Reach, "verus/REACH"),
        ] {
            let got = copy(&base("HEAD^", Some(declared())), path, at);
            assert!(
                got.as_ref()
                    .is_err_and(|e| e.contains(&format!("HEAD^ has no {path}"))),
                "{path}: got = {got:?}, wanted = an error"
            );
        }
        // Nothing the base declares: an absent file is nothing to compare.
        for (head, at, want) in [
            ("verus/FLOOR.new", Floor, "verus/FLOOR"),
            ("verus/RLIMIT.new", Rlimit, "verus/RLIMIT"),
            ("verus/REACH.new", Reach, "verus/REACH.new"),
        ] {
            let got = copy(&base("HEAD^", Some(bare())), head, at);
            assert_eq!(
                got,
                Ok((want.into(), None)),
                "{head}: got = {got:?}, wanted = none at {want}"
            );
        }
        // No base configuration: the head's path.
        for (head, want) in [
            ("verus/FLOOR", Some("FLOOR".to_owned())),
            ("verus/FLOOR.old", None),
        ] {
            let got = copy(&base("HEAD", None), head, Floor);
            assert_eq!(
                got,
                Ok((head.into(), want.clone())),
                "{head}: got = {got:?}, wanted = {want:?}"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    /// With `[floor]`'s room (#100), a share above the floor names the
    /// room's figure to record, or no raise where the room's figure is not
    /// above the floor; the pass or failure is the share's alone.
    #[test]
    fn a_rise_leaves_the_floor_s_room() {
        let fig = Figures::default();
        let want = Floor([Some(500), Some(500), Some(500), Some(500)]);
        let got = Floor([Some(600), Some(600), Some(600), Some(490)]);
        let room = Floor([Some(550), Some(500), Some(450), Some(400)]);
        let (ok, report) = fig.check_with_room(&got, &want, &room);
        assert!(!ok, "{report}");
        assert!(
            report.contains("functions verified 60.0%, above the floor's 50.0%; raise verus/FLOOR to `functions 55.0`"),
            "{report}"
        );
        for name in [
            "lines verified 60.0%",
            "checked-functions verified or discharged 60.0%",
        ] {
            assert!(
                report.contains(&format!(
                    "ok coverage floor: {name}, above the floor's 50.0% but within [floor]'s room"
                )),
                "{name}: {report}"
            );
        }
        assert!(
            report.contains("FAIL coverage floor: checked-lines verified or discharged 49.0%, below the floor's 50.0%"),
            "{report}"
        );
        // With no room, the figure to record is the share itself.
        let (_, report) = fig.check(&got, &want);
        assert!(
            report.contains("raise verus/FLOOR to `functions 60.0`"),
            "{report}"
        );
    }

    #[test]
    fn the_floor_fails_below_and_names_a_rise() {
        let want = Floor([Some(84), Some(105), Some(200), Some(300)]);
        let (ok, report) =
            Figures::default().check(&Floor([Some(84), Some(105), Some(200), Some(300)]), &want);
        assert!(ok, "at the floor: got = fail, wanted = ok\n{report}");
        let (ok, report) =
            Figures::default().check(&Floor([Some(83), Some(120), Some(200), Some(300)]), &want);
        assert!(!ok, "below on functions: got = ok, wanted = fail\n{report}");
        let (ok, report) =
            Figures::default().check(&Floor([Some(84), Some(105), Some(199), Some(310)]), &want);
        assert!(
            !ok,
            "below on checked-functions: got = ok, wanted = fail\n{report}"
        );
        assert!(
            report.contains(
                "FAIL coverage floor: checked-functions verified or discharged 19.9%, below the floor's 20.0%"
            ),
            "report: got = {report:?}"
        );
        assert!(
            report.contains("raise verus/FLOOR to `checked-lines 31.0`"),
            "report: got = {report:?}"
        );
        let (_, report) =
            Figures::default().check(&Floor([Some(83), Some(120), Some(200), Some(300)]), &want);
        assert!(
            report.contains("FAIL coverage floor: functions verified 8.3%, below the floor's 8.4%"),
            "report: got = {report:?}"
        );
        assert!(
            report.contains("raise verus/FLOOR to `lines 12.0`"),
            "report: got = {report:?}"
        );
        assert_eq!(
            tenths(1, 3),
            333,
            "tenths(1, 3): got = {}, wanted = 333",
            tenths(1, 3)
        );
        assert_eq!(
            tenths(0, 0),
            0,
            "tenths(0, 0): got = {}, wanted = 0",
            tenths(0, 0)
        );
    }

    #[test]
    fn a_discharged_function_needs_a_record() {
        let d = |f: &str, n: &str| (PathBuf::from(f), n.to_owned());
        let records = vec![
            "Seam S-x (`crates/a/src/x.rs`).\n```rust\nfn sort(v: &mut Ranges)\n```".to_owned(),
        ];
        let got = unrecorded(
            &[
                d("crates/a/src/x.rs", "sort"),
                d("crates/a/src/x.rs", "sor"),
                d("crates/a/src/y.rs", "sort"),
                d("crates/a/src/x.rs", "dedup"),
            ],
            &records,
        );
        let want = vec![
            d("crates/a/src/x.rs", "sor"),
            d("crates/a/src/y.rs", "sort"),
            d("crates/a/src/x.rs", "dedup"),
        ];
        assert_eq!(got, want, "unrecorded: got = {got:?}, wanted = {want:?}");
    }

    #[test]
    fn an_unparsable_verus_block_is_an_error() {
        let (t, errors) = count("verus! { fn 1() {} }");
        assert_eq!(errors, 1, "errors: got = {errors}, wanted = 1");
        assert_eq!(
            t.fns,
            [0, 0, 0, 0],
            "fns: got = {:?}, wanted = [0, 0, 0, 0]",
            t.fns
        );
    }

    /// The `#[verifier::rlimit]` limits one file, `lib.rs` of crate `k`,
    /// declares, as `(name, units)`, with the parse errors.
    fn limits(src: &str) -> (Vec<(String, Option<f32>)>, usize) {
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from("lib.rs"),
            module: vec!["k".into()],
            ..counter()
        };
        c.items(&parse_file(src).expect("parses").items, false);
        let got = c.limits.into_iter().map(|l| (l.name, l.units)).collect();
        (got, c.errors)
    }

    // The walk keeps every function Verus checks, with or without an
    // rlimit, named as `rlimit::Limit` names it, so a sub-default limit
    // dropped from a free function or a method raises it to the default.
    // Negative: a function outside `verus!` is not kept.
    #[test]
    fn functions_verus_checks_are_kept_for_the_rlimit_ratchet() {
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from("lib.rs"),
            module: vec!["k".into()],
            ..counter()
        };
        let src = "verus! {
fn f() {}
impl T {
    fn g(&self) {}
}
}
fn outside() {}
";
        c.items(&parse_file(src).expect("parses").items, false);
        assert_eq!(c.errors, 0, "errors: got = {}, wanted = 0", c.errors);
        let want: BTreeMap<String, String> = [
            ("k::T::g".to_string(), "lib.rs:4".to_string()),
            ("k::f".to_string(), "lib.rs:2".to_string()),
        ]
        .into();
        assert_eq!(
            c.verus_fns, want,
            "verus_fns: got = {:?}, wanted = {want:?}",
            c.verus_fns
        );
        let base = [
            rlimit::Limit {
                name: "k::f".into(),
                at: "lib.rs:2".into(),
                units: Some(5.0),
            },
            rlimit::Limit {
                name: "k::T::g".into(),
                at: "lib.rs:4".into(),
                units: Some(5.0),
            },
        ];
        let none = rlimit::parse_rlimit_file("max-share 70\n").expect("parses");
        let (ok, report) =
            rlimit::check_raised(&c.limits, &c.verus_fns, &base, &none, &none, "verus/RLIMIT");
        assert!(!ok, "check_raised: got = {ok}, wanted = false ({report})");
        for name in ["k::f", "k::T::g"] {
            assert!(
                report.contains(&format!("raises {name}'s rlimit from 5 to 10")),
                "report for {name}: got = {report:?}"
            );
        }
    }

    #[test]
    fn rlimit_attributes_are_named_as_verus_names_them() {
        let (got, errors) = limits(
            "verus! {
                #[verifier::rlimit(30)] fn free() {}
                impl<'a, U: Unit> Machine<'a, U> {
                    #[verifier::spinoff_prover]
                    #[verifier::rlimit(60)]
                    fn run(&mut self) {}
                    fn plain(&self) {}
                }
                trait Walk { #[verifier::rlimit(infinity)] fn walk(&self) {} }
                proof fn lemma_a() {}
                #[verifier::rlimit(15)] proof fn lemma_b() {}
                mod inner { #[verifier::rlimit(20)] fn deep() {} }
                #[cfg(test)] mod tests { #[verifier::rlimit(99)] fn t() {} }
                #[cfg(verus_keep_ghost)] #[verifier::rlimit(20)] fn ghostly() {}
                #[cfg(any(test, verus_keep_ghost))] mod grouped { #[verifier::rlimit(25)] fn g() {} }
                #[cfg(kani)] #[verifier::rlimit(98)] fn harness() {}
            }
            #[verifier::rlimit(40)] fn outside() {}",
        );
        assert_eq!(errors, 0, "errors: got = {errors}, wanted = 0");
        let want = vec![
            ("k::free".to_owned(), Some(30.0)),
            ("k::Machine::run".to_owned(), Some(60.0)),
            ("k::Walk::walk".to_owned(), None),
            ("k::lemma_b".to_owned(), Some(15.0)),
            ("k::inner::deep".to_owned(), Some(20.0)),
            ("k::ghostly".to_owned(), Some(20.0)),
            ("k::grouped::g".to_owned(), Some(25.0)),
        ];
        assert_eq!(got, want, "limits: got = {got:?}, wanted = {want:?}");
        // Negative: an argument that is neither a number nor infinity.
        let (got, errors) = limits("verus! { #[verifier::rlimit(lots)] fn f() {} }");
        assert_eq!(errors, 1, "errors: got = {errors}, wanted = 1");
        assert!(got.is_empty(), "limits: got = {got:?}, wanted = []");
        // Negative: a `cfg` the counter cannot read.
        let (_, errors) = limits("verus! { #[cfg(all(test,,unix))] fn odd() {} }");
        assert_eq!(errors, 1, "errors: got = {errors}, wanted = 1");
    }

    #[test]
    fn module_paths_follow_the_file() {
        for (krate, rel, want) in [
            ("pcre2-syntax", "lib.rs", "pcre2_syntax"),
            ("pcre2", "vm/exec.rs", "pcre2::vm::exec"),
            ("pcre2", "vm/mod.rs", "pcre2::vm"),
            ("pcre2", "vm/assert.rs", "pcre2::vm::assert"),
        ] {
            let got = module_path(krate, Path::new(rel)).join("::");
            assert_eq!(
                got, want,
                "module_path({krate}, {rel}): got = {got}, wanted = {want}"
            );
        }
        for (owner, want) in [
            ("Machine < 'a , U >", Some("Machine")),
            ("crate :: vm :: Machine", Some("Machine")),
            (":: core :: X", Some("X")),
            ("r#exec", Some("r#exec")),
            // No path: the whole text, which no function of Verus's has.
            ("& m :: S", Some("& m :: S")),
            ("& mut m :: S", Some("& mut m :: S")),
            ("* const m :: S", Some("* const m :: S")),
            ("dyn m :: Tr", Some("dyn m :: Tr")),
            ("& S", Some("& S")),
            ("< u8 as Tr2 > :: Out", Some("< u8 as Tr2 > :: Out")),
            ("", None),
        ] {
            let got = owner_name(owner);
            assert_eq!(
                got.as_deref(),
                want,
                "owner_name({owner:?}): got = {got:?}, wanted = {want:?}"
            );
        }
    }

    /// A trusted function is recorded at its `fn` line, after its
    /// attributes, as `FILE:LINE` with its line count: the key the
    /// generated ledger's entries are matched on ([`ledger::match_ledger`]).
    /// challoc's counter's test of the same name.
    #[test]
    fn the_counter_records_where_trusted_functions_start() {
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from("src/a.rs"),
            module: vec!["k".into()],
            ..counter()
        };
        let src = "verus! {\n    fn verified() {}\n    #[verifier::external_body]\n    fn trusted() {\n        1;\n    }\n}\n";
        c.items(&parse_file(src).expect("parses").items, false);
        let want = vec![("k".to_owned(), "src/a.rs:4".to_owned(), 3)];
        assert_eq!(
            c.trusted, want,
            "trusted: got = {:?}, wanted = {want:?}",
            c.trusted
        );
    }

    /// Verus's attribute form (the Expat port's: a contract in `verus_spec` under
    /// a `cfg_attr`, for a port whose MSRV is below vstd's) verifies a plain
    /// function outside `verus!`, under a predicate the configuration
    /// settles (the Verus feature, a `verus_sets` feature, `not` of a
    /// `verus_never_sets` atom); `verus_verify(external_body)` is trusted,
    /// and so is a `cfg_attr` whose predicate Verus's build may not meet (a
    /// platform, alone or beside the Verus feature), which may leave the
    /// function unchecked. Negative: a `cfg_attr` of another attribute, a
    /// path that only passes through a `verus_spec` module, or a `cfg_attr`
    /// whose predicate Verus's build never meets (`any()`, `test`, a
    /// predicate with a test atom, the negated Verus feature, a
    /// `verus_never_sets` atom), is not the attribute form.
    #[test]
    fn the_attribute_form_counts_as_verus_code() {
        let src = r#"
#[cfg_attr(feature = "verus", verus_verify)]
#[cfg_attr(feature = "verus", verus_spec(r =>
    requires n <= 4,
    ensures r == (n > 4),
))]
pub fn spec_attr(n: usize) -> bool { n > 4 }
#[verus_verify]
fn direct() {}
#[vstd::prelude::verus_spec(ensures true)]
fn by_path() {}
#[cfg_attr(feature = "verus", verus_verify(external_body))]
fn trusted_attr() {}
#[cfg_attr(feature = "verus", verus_verify)]
fn assumes() { proof!(assume(false)); }
#[cfg_attr(feature = "verus", inline)]
fn other_attr() {}
#[cfg_attr(feature = "verus", verus_spec::helper)]
fn through_a_module() {}
#[cfg_attr(any(), verus_verify)]
fn never() {}
#[cfg_attr(test, verus_verify)]
fn in_tests() {}
#[cfg_attr(all(feature = "verus", kani), verus_verify)]
fn with_a_test_atom() {}
#[cfg_attr(not(feature = "verus"), verus_spec(ensures true))]
fn not_for_verus() {}
#[cfg_attr(target_os = "windows", vstd::prelude::verus_verify)]
fn on_one_platform() {}
#[cfg_attr(all(feature = "verus", unix), verus_verify)]
fn with_a_platform() {}
#[cfg_attr(any(feature = "verus", unix), verus_verify)]
fn either() {}
#[cfg_attr(all(feature = "jit", not(miri)), verus_verify)]
fn settled() {}
#[cfg_attr(miri, verus_verify)]
fn under_miri() {}
fn plain() {}
#[cfg(feature = "verus")]
verus! {
    spec fn lead(b: u8) -> nat { 0 }
    fn verus_only() {}
}
"#;
        let (t, errors) = count(src);
        assert_eq!(errors, 0, "errors: got = {errors}, wanted = 0");
        // spec_attr, direct, by_path, either (the Verus feature settles
        // the `any`) and settled verified; trusted_attr, assumes,
        // on_one_platform and with_a_platform trusted; other_attr,
        // through_a_module, never, in_tests, with_a_test_atom,
        // not_for_verus, under_miri and plain outside. The `verus!` block
        // is built only for Verus (`feature = "verus"`), so its exec
        // function and its spec function leave the tally.
        let want = [5, 4, 8, 0];
        assert_eq!(t.fns, want, "fns: got = {:?}, wanted = {want:?}", t.fns);
        assert_eq!(t.ghost, 0, "ghost: got = {}, wanted = 0", t.ghost);
        // With `verus_only_ghost`, the Verus-only spec function joins the
        // ghost column, and nothing else moves.
        let mut c = Counter {
            krate: "k".into(),
            file: PathBuf::from("lib.rs"),
            module: vec!["k".into()],
            verus_only_ghost: true,
            ..counter()
        };
        c.items(&parse_file(src).expect("parses").items, false);
        let t = c.tallies.remove("k").unwrap_or_default();
        assert_eq!(t.fns, want, "fns: got = {:?}, wanted = {want:?}", t.fns);
        assert_eq!(t.ghost, 1, "ghost: got = {}, wanted = 1", t.ghost);
    }

    /// The FLOOR file's verified-or-discharged figures carry the configured
    /// prefix (challoc's `combined-`), in what it reads and what it says.
    #[test]
    fn the_checked_figures_take_the_configured_prefix() {
        let fig = Figures::new("combined", "x/FLOOR");
        let head = fig
            .parse("functions 1.0\nlines 2.0\ncombined-functions 3.0\ncombined-lines 4.0\n")
            .expect("parses");
        assert_eq!(fig.missing(&head.floor), None);
        assert!(
            fig.parse("functions 1.0\nlines 2.0\nchecked-lines 4.0\n")
                .is_err()
        );
        let (ok, report) = fig.check(
            &Floor([Some(10), Some(20), Some(31), Some(40)]),
            &head.floor,
        );
        assert!(ok, "{report}");
        assert!(
            report.contains("raise x/FLOOR to `combined-functions 3.1`"),
            "report: got = {report:?}"
        );
        let base = fig
            .parse("functions 1.0\nlines 2.0\ncombined-functions 3.5\n")
            .unwrap();
        let (ok, report) = fig.lowering(&head, &base);
        assert!(!ok);
        assert!(
            report.contains("x/FLOOR lowers combined-functions from 3.5% to 3.0%"),
            "report: got = {report:?}"
        );
        // The ledger's discharged functions count in the checked figures.
        let mut t = Tally::default();
        t.fns[Class::Verified as usize] = 6;
        t.fns[Class::Trusted as usize] = 4;
        t.lines[Class::Verified as usize] = 60;
        t.lines[Class::Trusted as usize] = 40;
        assert_eq!(
            shares(&t, (2, 30), (0, 0)),
            Floor([Some(600), Some(600), Some(800), Some(900)])
        );
        // With room for 25 more unverified functions and 250 more lines
        // (#100): 6/35, 60/350, 8/35 and 90/350, rounded down.
        assert_eq!(
            shares(&t, (2, 30), (25, 250)),
            Floor([Some(171), Some(171), Some(228), Some(257)])
        );
    }

    /// With `[floor] renamed_from`, the base revision's copy is read under
    /// the old prefix as well, so the change that renames the checked
    /// figures (challoc's `combined-` to `checked-`) still compares with
    /// its base; the file under test takes the new names alone.
    #[test]
    fn a_base_may_give_the_checked_figures_their_old_names() {
        let fig = Figures::new("checked", "x/FLOOR");
        let old = "functions 1.0\nlines 2.0\ncombined-functions 3.5\ncombined-lines 4.0\nlowered combined-lines 3.9 an old reason\n";
        assert!(
            fig.parse(old).is_err(),
            "old names without renamed_from: got = Ok, wanted = Err"
        );
        let base = fig.for_base(Some("combined")).parse(old).expect("parses");
        assert_eq!(
            base,
            FloorFile {
                floor: Floor([Some(10), Some(20), Some(35), Some(40)]),
                lowered: vec![("checked-lines".into(), 39)],
            },
            "base: got = {base:?}, wanted = the old figures as the checked ones"
        );
        assert!(
            fig.for_base(Some("combined"))
                .parse("functions 1.0\nlines 2.0\ncombined-lines 4.0\nchecked-lines 4.0\n")
                .is_err(),
            "a figure under both names: got = Ok, wanted = Err"
        );
        for (text, want) in [
            ("checked-functions 3.5\nchecked-lines 4.0\n", true),
            ("checked-functions 3.4\nchecked-lines 4.0\n", false),
            (
                "checked-functions 3.4\nchecked-lines 4.0\nlowered checked-functions 3.4 moved out\n",
                true,
            ),
            // The base's reason, under its old name, is not new here.
            (
                "checked-functions 3.5\nchecked-lines 3.9\nlowered checked-lines 3.9 a new reason\n",
                false,
            ),
            (
                "checked-functions 3.5\nchecked-lines 3.8\nlowered checked-lines 3.8 moved out\n",
                true,
            ),
            ("checked-functions 3.5\nchecked-lines 3.8\n", false),
        ] {
            let text = format!("functions 1.0\nlines 2.0\n{text}");
            let head = fig.parse(&text).expect("parses");
            let (ok, report) = fig.lowering(&head, &base);
            assert_eq!(ok, want, "{text:?}: got = {ok}, wanted = {want}\n{report}");
        }
    }

    /// A crate's edition is its own, the workspace's it inherits, or 2015
    /// (none given, a workspace that gives none, a manifest that does not
    /// parse); a dependency key among `vstd`, `verus_builtin_macros`,
    /// `core` and `std` (hyphens read as `_`) whose `package` is another
    /// is renamed, in any dependency table, a `[target.*]` one too.
    #[test]
    fn the_manifest_gives_the_edition_and_renamed_dependencies() {
        let root =
            std::env::temp_dir().join(format!("verus-count-manifest-{}", std::process::id()));
        let k = root.join("crates/k");
        fs::create_dir_all(&k).unwrap();
        for (ws, krate, edition, renamed) in [
            ("", "[package]\nedition = \"2024\"\n", 2024, vec![]),
            ("", "[package]\nname = \"k\"\n", 2015, vec![]),
            ("", "[package\n", 2015, vec![]),
            (
                "[workspace.package]\nedition = \"2021\"\n",
                "[package]\nedition.workspace = true\n",
                2021,
                vec![],
            ),
            (
                "[workspace]\n",
                "[package]\nedition.workspace = true\n",
                2015,
                vec![],
            ),
            (
                "",
                "[package]\nedition = \"2024\"\n[dependencies]\nvstd = { package = \"other\" }\nfoo = { package = \"vstd\" }\n",
                2024,
                vec!["vstd"],
            ),
            (
                "",
                "[package]\nedition = \"2024\"\n[target.'cfg(unix)'.dev-dependencies]\nverus-builtin-macros = { package = \"x\" }\n",
                2024,
                vec!["verus_builtin_macros"],
            ),
            (
                "",
                "[package]\nedition = \"2024\"\n[dependencies]\nvstd = { package = \"vstd\" }\nverus-builtin-macros = { package = \"verus_builtin_macros\" }\n",
                2024,
                vec![],
            ),
            // Through `[workspace.dependencies]`: renamed there, the same
            // package, or an entry the root does not give.
            (
                "[workspace.dependencies]\nvstd = { package = \"other\", path = \"x\" }\n",
                "[package]\nedition = \"2024\"\n[dependencies]\nvstd = { workspace = true }\n",
                2024,
                vec!["vstd"],
            ),
            (
                "[workspace.dependencies]\nvstd = { version = \"1\" }\n",
                "[package]\nedition = \"2024\"\n[target.'cfg(unix)'.dependencies]\nvstd.workspace = true\n",
                2024,
                vec![],
            ),
            (
                "[workspace]\n",
                "[package]\nedition = \"2024\"\n[dependencies]\nvstd = { workspace = true }\n",
                2024,
                vec!["vstd"],
            ),
            // A target's own edition, the lowest one.
            (
                "",
                "[package]\nedition = \"2024\"\n[lib]\nedition = \"2015\"\n",
                2015,
                vec![],
            ),
            (
                "",
                "[package]\nedition = \"2024\"\n[[bin]]\nname = \"b\"\nedition = \"2021\"\n",
                2021,
                vec![],
            ),
        ] {
            fs::write(root.join("Cargo.toml"), ws).unwrap();
            fs::write(k.join("Cargo.toml"), krate).unwrap();
            let (got, deps) = crate_manifest(&root, &k);
            let got_renamed: Vec<&str> = deps.iter().map(|(n, _)| n.as_str()).collect();
            assert!(
                got == edition && got_renamed == renamed,
                "{krate:?}: got = {got}, {got_renamed:?}; wanted = {edition}, {renamed:?}"
            );
        }
        let _ = fs::remove_dir_all(&root);
    }

    /// The editions macros match by are the library's (where the package
    /// has one) and each binary's, their own or the package's (inherited
    /// from the workspace too), and
    /// those of the tests, benches and examples whose `path` is a file
    /// under `src/`; one with no `path`, or a path outside `src/`, is not
    /// read.
    #[test]
    fn the_walked_targets_give_the_matching_editions() {
        let root = std::env::temp_dir().join(format!("verus-count-shipped-{}", std::process::id()));
        let k = root.join("crates/k");
        fs::create_dir_all(k.join("src")).unwrap();
        fs::create_dir_all(k.join("tests")).unwrap();
        fs::write(k.join("src/legacy.rs"), "").unwrap();
        fs::write(k.join("src/lib.rs"), "").unwrap();
        fs::write(k.join("tests/t.rs"), "").unwrap();
        for (ws, krate, want) in [
            // `autolib = false` leaves `src/lib.rs` out, but not a `[lib]`.
            (
                "",
                "[package]\nedition = \"2024\"\nautolib = false\n[[bin]]\nname = \"a\"\npath = \"src/legacy.rs\"\nedition = \"2021\"\n",
                vec![2021],
            ),
            (
                "",
                "[package]\nedition = \"2024\"\nautolib = false\n[lib]\nedition = \"2021\"\n",
                vec![2021],
            ),
            (
                "",
                "[package]\nedition = \"2024\"\n[[test]]\nname = \"t\"\npath = \"src/legacy.rs\"\nedition = \"2021\"\n",
                vec![2024, 2021],
            ),
            (
                "",
                "[package]\nedition = \"2024\"\n[[bench]]\nname = \"b\"\npath = \"src/legacy.rs\"\nedition = \"2018\"\n",
                vec![2024, 2018],
            ),
            (
                "",
                "[package]\nedition = \"2024\"\n[[example]]\nname = \"e\"\npath = \"src/../src/legacy.rs\"\n",
                vec![2024, 2024],
            ),
            (
                "",
                "[package]\nedition = \"2024\"\n[[test]]\nname = \"t\"\npath = \"tests/t.rs\"\nedition = \"2021\"\n[[example]]\nname = \"e\"\npath = \"src/missing.rs\"\nedition = \"2021\"\n",
                vec![2024],
            ),
            ("", "[package]\nedition = \"2024\"\n", vec![2024]),
            ("", "[package]\nname = \"k\"\n", vec![2015]),
            (
                "",
                "[package]\nedition = \"2024\"\n[[test]]\nname = \"t\"\nedition = \"2021\"\n[[bench]]\nname = \"b\"\nedition = \"2021\"\n[[example]]\nname = \"e\"\nedition = \"2015\"\n",
                vec![2024],
            ),
            (
                "[workspace.package]\nedition = \"2024\"\n",
                "[package]\nedition.workspace = true\n[lib]\nedition = \"2021\"\n[[bin]]\nname = \"a\"\n[[bin]]\nname = \"b\"\nedition = \"2018\"\n",
                vec![2021, 2024, 2018],
            ),
        ] {
            fs::write(root.join("Cargo.toml"), ws).unwrap();
            fs::write(k.join("Cargo.toml"), krate).unwrap();
            let got = walked_editions(&root, &k);
            assert_eq!(got, want, "{krate:?}: got = {got:?}, wanted = {want:?}");
        }
        // With no `src/lib.rs`, a package has a library only by a `[lib]
        // path`: a binary-only one gives its binaries' editions alone, and
        // one with no target the package's.
        fs::remove_file(k.join("src/lib.rs")).unwrap();
        fs::write(k.join("src/main.rs"), "").unwrap();
        for (krate, want) in [
            (
                "[package]\nname = \"k\"\nedition = \"2024\"\nautobins = false\n[[bin]]\nname = \"k\"\npath = \"src/main.rs\"\nedition = \"2021\"\n",
                vec![2021],
            ),
            ("[package]\nname = \"k\"\nedition = \"2021\"\n", vec![2021]),
            (
                "[package]\nname = \"k\"\nedition = \"2024\"\nautobins = false\n[lib]\npath = \"src/legacy.rs\"\nedition = \"2021\"\n",
                vec![2021],
            ),
            (
                "[package]\nname = \"k\"\nedition = \"2018\"\nautobins = false\n",
                vec![2018],
            ),
        ] {
            fs::write(root.join("Cargo.toml"), "").unwrap();
            fs::write(k.join("Cargo.toml"), krate).unwrap();
            let got = walked_editions(&root, &k);
            assert_eq!(got, want, "{krate:?}: got = {got:?}, wanted = {want:?}");
        }
        let _ = fs::remove_dir_all(&root);
    }

    /// `publish = false` is the package's only in its `[package]` table:
    /// one under `[package.metadata]` or another table, in a comment or a
    /// string, or in a manifest that does not parse leaves the crate
    /// shipped.
    #[test]
    fn publish_is_read_from_the_package_table() {
        for (text, want) in [
            ("[package]\nname = \"a\"\npublish = false\n", true),
            ("[package]\npublish=false # never\n", true),
            ("package = { name = \"a\", publish = false }\n", true),
            (
                "[package]\nname = \"a\"\n\n[package.metadata]\npublish = false\n",
                false,
            ),
            (
                "[package]\nname = \"a\"\n\n[package.metadata.x]\npublish = false\n",
                false,
            ),
            ("[package]\nname = \"a\"\n# publish = false\n", false),
            (
                "[package]\ndescription = \"\"\"\npublish = false\n\"\"\"\n",
                false,
            ),
            ("[package]\npublish = true\n", false),
            ("[package]\npublish = [\"crates-io\"]\n", false),
            ("[package\npublish = false\n", false),
        ] {
            let got = unpublished(text);
            assert!(got == want, "{text:?}: got = {got}, wanted = {want}");
        }
    }

    /// The crates come from the configuration: `dir`'s subdirectories with a
    /// `src/`, then the members, each shipping unless its Cargo.toml says
    /// `publish = false` or its `ships` says otherwise.
    #[test]
    fn crates_come_from_the_configuration() {
        let root = std::env::temp_dir().join(format!("verus-count-crates-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for d in ["crates/a/src", "crates/b/src", "crates/notes", "src"] {
            fs::create_dir_all(root.join(d)).unwrap();
        }
        fs::write(
            root.join("crates/b/Cargo.toml"),
            "[package]\npublish = false\n",
        )
        .unwrap();
        fs::write(root.join("Cargo.toml"), "[package]\npublish = false\n").unwrap();
        std::os::unix::fs::symlink("crates/a", root.join("alias")).unwrap();
        std::os::unix::fs::symlink("crates", root.join("linked")).unwrap();
        // Links a configured path does not name: a crate of a `dir` that is
        // one, a `src` that is one, and a source file that is one.
        for d in ["more/real/src", "other", "deep/src"] {
            fs::create_dir_all(root.join(d)).unwrap();
        }
        std::os::unix::fs::symlink("../crates/a", root.join("more/alias")).unwrap();
        std::os::unix::fs::symlink("../crates/a/src", root.join("other/src")).unwrap();
        std::os::unix::fs::symlink("../../crates/a/src/lib.rs", root.join("deep/src/lib.rs"))
            .unwrap();
        // Links below a crate's `src/` (#71): uutils' `LICENSE` and a
        // `locales` directory of `.ftl` files, which the source walk never
        // reads, beside a directory link holding a `.rs` file and a cycle,
        // which it reads through.
        for d in [
            "lic/src/uu",
            "lic/data/locales",
            "rsdir/src",
            "rsdir/data",
            "cycle/src/m",
            "diamond/src/uu",
            "diamond/data/ftl",
            "eloop/src",
            "dangle/src",
            "nloop/src",
            "nloop/data",
        ] {
            fs::create_dir_all(root.join(d)).unwrap();
        }
        // A diamond: two links to one `.ftl` directory under a linked
        // directory, which holds no `.rs` file and no cycle.
        fs::write(root.join("diamond/data/ftl/en-US.ftl"), "x = y\n").unwrap();
        std::os::unix::fs::symlink("ftl", root.join("diamond/data/l1")).unwrap();
        std::os::unix::fs::symlink("ftl", root.join("diamond/data/l2")).unwrap();
        std::os::unix::fs::symlink("../../data", root.join("diamond/src/uu/x")).unwrap();
        // A link loop, which no stat resolves (ELOOP), and a dangling
        // link, which the walk reads nothing through.
        std::os::unix::fs::symlink("l2", root.join("eloop/src/l1")).unwrap();
        std::os::unix::fs::symlink("l1", root.join("eloop/src/l2")).unwrap();
        // The same loop inside a linked directory's target.
        std::os::unix::fs::symlink("../data", root.join("nloop/src/x")).unwrap();
        std::os::unix::fs::symlink("l2", root.join("nloop/data/l1")).unwrap();
        std::os::unix::fs::symlink("l1", root.join("nloop/data/l2")).unwrap();
        std::os::unix::fs::symlink("../nowhere", root.join("dangle/src/gone")).unwrap();
        fs::write(root.join("crates/a/src/lib.rs"), "").unwrap();
        fs::write(root.join("lic/LICENSE"), "MIT\n").unwrap();
        fs::write(root.join("lic/data/locales/en-US.ftl"), "x = y\n").unwrap();
        std::os::unix::fs::symlink("../../LICENSE", root.join("lic/src/uu/LICENSE")).unwrap();
        std::os::unix::fs::symlink("../../data/locales", root.join("lic/src/uu/locales")).unwrap();
        fs::write(root.join("rsdir/data/gen.rs"), "").unwrap();
        std::os::unix::fs::symlink("../data", root.join("rsdir/src/gen")).unwrap();
        std::os::unix::fs::symlink("..", root.join("cycle/src/m/up")).unwrap();
        let read = |text: &str| -> Vec<(String, bool)> {
            crate_dirs(&root, &Config::parse(text).expect("parses"))
                .expect("reads")
                .into_iter()
                .map(|k| (k.name, k.ships))
                .collect()
        };
        assert_eq!(
            read("[crates]\ndir = \"crates\"\n"),
            [("a".to_string(), true), ("b".to_string(), false)]
        );
        assert_eq!(
            read("[crates]\nmembers = [{ name = \"top\", path = \".\" }]\n"),
            [("top".to_string(), false)]
        );
        assert_eq!(
            read("[crates]\nmembers = [{ name = \"top\", path = \".\", ships = true }]\n"),
            [("top".to_string(), true)]
        );
        let got = crate_dirs(
            &root,
            &Config::parse("[crates]\nmembers = [{ name = \"n\", path = \"crates/notes\" }]\n")
                .unwrap(),
        );
        assert!(
            got.is_err(),
            "a member without src/: got = Ok, wanted = an error"
        );
        let got = crate_dirs(&root, &Config::parse("[crates]\ndir = \"gone\"\n").unwrap());
        assert!(got.is_err(), "a missing dir: got = Ok, wanted = an error");
        // A crate read twice, under one name or from one directory, is
        // refused: the tally goes by name, so `ships = false` on the second
        // would take the first out of the shipped row too.
        for (text, want) in [
            (
                "[crates]\ndir = \"crates\"\nmembers = [{ name = \"a\", path = \"crates/a\", ships = false }]\n",
                "reads crate a (",
            ),
            (
                "[crates]\nmembers = [{ name = \"a\", path = \"crates/a\" }, { name = \"a\", path = \"crates/b\" }]\n",
                "reads crate a (",
            ),
            (
                "[crates]\nmembers = [{ name = \"x\", path = \"crates/a\" }, { name = \"y\", path = \"crates/a\" }]\n",
                "reads crate x (",
            ),
            // Nor is a path that spells a directory another way, through
            // `..` or a symlink, which the base comparison would take for
            // a new crate.
            (
                "[crates]\nmembers = [{ name = \"r\", path = \"crates/../crates/a\" }]\n",
                "[crates] path crates/../crates/a has a `..`",
            ),
            (
                "[crates]\ndir = \"crates/..\"\n",
                "[crates] path crates/.. has a `..`",
            ),
            (
                "[crates]\nmembers = [{ name = \"r\", path = \"alias\" }]\n",
                "[crates] path alias passes through alias, a symlink",
            ),
            (
                "[crates]\nmembers = [{ name = \"r\", path = \"linked/a\" }]\n",
                "[crates] path linked/a passes through linked, a symlink",
            ),
            (
                "[crates]\ndir = \"linked\"\n",
                "[crates] path linked passes through linked, a symlink",
            ),
            (
                "[crates]\ndir = \"more\"\n",
                "[crates] crate alias reads more/alias, a symlink",
            ),
            (
                "[crates]\nmembers = [{ name = \"o\", path = \"other\" }]\n",
                "[crates] crate o reads other/src, a symlink",
            ),
            (
                "[crates]\nmembers = [{ name = \"d\", path = \"deep\" }]\n",
                "[crates] crate d reads deep/src/lib.rs, a symlink",
            ),
            (
                "[crates]\nmembers = [{ name = \"g\", path = \"rsdir\" }]\n",
                "[crates] crate g reads rsdir/src/gen, a symlink",
            ),
            (
                "[crates]\nmembers = [{ name = \"c\", path = \"cycle\" }]\n",
                "[crates] crate c reads cycle/src/m/up, a symlink",
            ),
            (
                "[crates]\nmembers = [{ name = \"e\", path = \"eloop\" }]\n",
                "[crates] crate e reads eloop/src/l1, a symlink",
            ),
            (
                "[crates]\nmembers = [{ name = \"n\", path = \"nloop\" }]\n",
                "[crates] crate n reads nloop/src/x, a symlink",
            ),
        ] {
            let got = crate_dirs(&root, &Config::parse(text).expect("parses"));
            assert!(
                got.as_ref().is_err_and(|e| e.contains(want)),
                "{text:?}: got = {:?}, wanted = an error with {want:?}",
                got.map(|k| k.into_iter().map(|k| k.name).collect::<Vec<_>>())
            );
        }
        assert_eq!(
            read("[crates]\nmembers = [{ name = \"l\", path = \"lic\" }]\n"),
            [("l".to_string(), true)]
        );
        assert_eq!(
            read("[crates]\nmembers = [{ name = \"m\", path = \"diamond\" }]\n"),
            [("m".to_string(), true)]
        );
        assert_eq!(
            read("[crates]\nmembers = [{ name = \"g\", path = \"dangle\" }]\n"),
            [("g".to_string(), true)]
        );
        // A directory the guard cannot resolve or list refuses the link.
        // A mode-000 directory is listable as root, so a missing path and
        // a regular file stand in for the two read errors.
        let mut seen = std::collections::HashSet::new();
        assert!(
            reaches_rs(&root.join("gone"), &mut seen),
            "a directory that does not resolve: got = passed, wanted = refused"
        );
        assert!(
            reaches_rs(&root.join("lic/LICENSE"), &mut seen),
            "a directory that cannot be listed: got = passed, wanted = refused"
        );
        assert!(seen.is_empty(), "seen after the walk: {seen:?}");
        // A path from `/`, even to a crate under the root, is refused:
        // `root.join` would read it while it compares unequal to the base's
        // relative path.
        let abs = root.join("crates/a").to_string_lossy().into_owned();
        let got = crate_dirs(
            &root,
            &Config::parse(&format!(
                "[crates]\nmembers = [{{ name = \"r\", path = {abs:?} }}]\n"
            ))
            .expect("parses"),
        );
        let want = format!("[crates] path {abs} is not relative");
        assert!(
            got.as_ref().is_err_and(|e| e.contains(&want)),
            "an absolute member path: got = {:?}, wanted = an error with {want:?}",
            got.map(|k| k.into_iter().map(|k| k.name).collect::<Vec<_>>())
        );
        let _ = fs::remove_dir_all(&root);
    }
}
