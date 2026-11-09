//! verus-count: counts the executable functions of a proved port's crates
//! by how Verus sees them, and holds them to the port's FLOOR
//! (kit/count/README.md). The port's `verus/count.toml` ([`config`]) names
//! its crates, its test-only `cfg`s and features, and where each table the
//! checks below read lives; a check whose section it leaves out does not
//! run. The paths below are pcre3's, which the defaults follow.
//!
//! A function is *verified* when it sits inside `verus! { ... }`, or
//! carries Verus's attribute form (`#[verus_verify]` or `#[verus_spec]`,
//! directly or under a `cfg_attr` whose predicate Verus's build meets),
//! is not `#[verifier::external_body]`,
//! `#[verifier::external]` or `verus_verify(external_body)`, and its body
//! calls none of `assume`, `admit` and `Tracked::assume_new` (which mints
//! a ghost token on trust); *trusted* when it sits inside
//! `verus!` but is one of those; and *outside* when it is not inside
//! `verus!` at all. A trusted function marked
//! `#[cfg_attr(any(), discharged_by = "kani")]` is *discharged* instead:
//! Kani checks its contract (verus/README.md, "Kani"), on the bounded inputs
//! its record names, so it counts neither as verified nor as trusted. The
//! marker is inert in every build (`any()` is false), and a marked function
//! no `verus/seams/kani-*.md` record covers fails the run. With a
//! `[ledger]` section, a generated seam ledger ([`ledger`]) says which
//! trusted functions are discharged instead, and the marker is refused.
//! `spec` and `proof` functions are ghost code, erased from ordinary
//! builds, and are counted apart. Items under the test-only `cfg`s
//! (`#[cfg(test)]`, `#[cfg(kani)]`, the Kani harnesses) or test-only
//! features (`#[cfg(feature = "test-util")]`, which only tests enable; a
//! build of the workspace that cargo resolves with one enabled fails the
//! run) are left out, with the files of such modules and, for a `mod.rs`
//! one, its directory. Items built only for Verus (`verus_keep_ghost`, or
//! a configured Verus-only feature) are read for their rlimits and their
//! trusted functions alone. Lines run from a function's `fn` keyword to
//! its closing brace.
//!
//! Usage: `verus-count [--config FILE] [--list] [--floor FILE [--base-rev
//! REV]] [--rlimit STATS] ROOT`, with ROOT the repository root (default
//! `.`) and FILE the configuration (default `ROOT/verus/count.toml`);
//! `--version` prints the counter's version.
//! `--list` prints one line per executable function: crate, class, lines,
//! file:line and name. `--floor FILE` fails the run when the `shipped`
//! row's verified share of functions or of lines, or its verified and
//! discharged share, is below the figure FILE records (verus/FLOOR), and
//! names the figure to record when the share has risen past it.
//! `--base-rev REV` also fails a figure FILE sets below `REV:verus/FLOOR`'s
//! without a new `lowered` line for it, and fails when REV does not
//! resolve. With `[floor] renamed_from`, REV's copy may give the checked
//! figures under that older prefix, so the change that renames them
//! compares with its base.
//!
//! With `[recursion]`, it checks recursion (verus/theorems/RN1.md): it fails on a
//! cycle of direct calls, within one file, through a function of a shipped
//! crate that Verus does not check (outside `verus!`, or trusted or
//! discharged), and on one through a verified function that RN1.md's table
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
//! default is a rise too.
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
//! `REV:verus/REACH` lacks that is not marked `new`.
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

#[derive(Default)]
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
    /// The module [`Counter::items`] is walking, as Verus names it: the
    /// crate (`pcre3_syntax`), then each module of the file's path and of
    /// any inline `mod` around the item.
    module: Vec<String>,
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
    /// Whether ghost functions of code built only for Verus count in the
    /// ghost column (`[crates] verus_only_ghost`).
    verus_only_ghost: bool,
}

/// The `cfg` atoms, without whitespace, that a shipped build never meets:
/// those only test builds meet (`test`, `kani`, `feature="test-util"`),
/// and those only Verus's build meets (`verus_keep_ghost`, and
/// `feature="F"` for each configured Verus-only feature).
#[derive(Debug, Clone, Default)]
struct Atoms {
    test: Vec<String>,
    verus: Vec<String>,
}

impl Atoms {
    fn of(c: &Config) -> Atoms {
        Atoms {
            test: c.test_atoms(),
            verus: std::iter::once("verus_keep_ghost".to_owned())
                .chain(
                    c.crates
                        .verus_features
                        .iter()
                        .map(|f| format!("feature=\"{f}\"")),
                )
                .collect(),
        }
    }
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
            statics: Vec::new(),
            types: BTreeMap::new(),
            aliases: BTreeMap::new(),
            traits: std::collections::BTreeSet::new(),
            owner: String::new(),
            module: Vec::new(),
            limits: Vec::new(),
            verus_fns: BTreeMap::new(),
            shipped: true,
            fn_ptrs: FnPtrs::default(),
            addressed: Vec::new(),
            atoms,
            fn_ptr_macro,
            trusted: Vec::new(),
            verus_only_ghost: false,
        }
    }
}

/// A function declared in another's body, as [`Counter::nested`] reads
/// it: its attributes, signature and body.
type NestedFn<'a> = (&'a [Attribute], &'a Signature, &'a verus_syn::Block);

/// The `fn_ptr!` macro's one rule and its invocations: the definition
/// lives in one file (pcre3-capi's width.rs) and the invocations in
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

/// Whether C can call the function by its name: `#[no_mangle]` or
/// `#[export_name = ..]`, either one inside `#[unsafe(..)]` too. That, not
/// the ABI, makes a function an entry point ([`entry_roots`]); the ABI is
/// then [`is_c_abi`]'s to check ([`Counter::record`]).
fn exports_symbol(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        let tokens = verus_syn::__private::ToTokens::to_token_stream(&a.meta);
        let mut idents = Vec::new();
        let mut stack = vec![tokens];
        while let Some(ts) = stack.pop() {
            for t in ts {
                match t {
                    TokenTree::Ident(i) => idents.push(i.to_string()),
                    TokenTree::Group(g) if g.delimiter() == proc_macro2::Delimiter::Parenthesis => {
                        stack.push(g.stream());
                    }
                    _ => {}
                }
            }
        }
        match idents.first().map(String::as_str) {
            Some("no_mangle" | "export_name") => true,
            Some("unsafe") => idents
                .get(1)
                .is_some_and(|i| i == "no_mangle" || i == "export_name"),
            _ => false,
        }
    })
}

/// A signature's parameters, the receiver aside, and whether it has one.
fn arity(sig: &Signature) -> (usize, bool) {
    let receiver = sig
        .inputs
        .iter()
        .any(|a| matches!(a.kind, verus_syn::FnArgKind::Receiver(_)));
    (sig.inputs.len() - usize::from(receiver), receiver)
}

/// The files of modules declared in `file` that no shipped build compiles:
/// those built only for tests (`#[cfg(test)] mod name;`, `#[cfg(kani)]`)
/// into `tests`, which nothing reads, and those built only for Verus
/// (`#[cfg(verus_keep_ghost)]`, `#[cfg(any(test, verus_keep_ghost))]`) into
/// `verus`, which are read for their rlimits and not counted. A module's
/// file is the one its `#[path]` names, relative to `file`'s directory, or
/// else `name.rs` or `name/mod.rs` beside it (or under its directory, for a
/// `mod.rs` or crate root). Each is canonical, as `main` compares them. A
/// `cfg` it cannot read is an error, in `errors`.
fn test_module_files(
    file: &Path,
    items: &[Item],
    atoms: &Atoms,
    tests: &mut Vec<PathBuf>,
    verus: &mut Vec<PathBuf>,
    errors: &mut Vec<String>,
) {
    let Some(dir) = module_dir(file) else { return };
    for item in items {
        let Item::Mod(m) = item else { continue };
        let built = match attrs_built(&m.attrs, atoms) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("{}: {e}", file.display()));
                continue;
            }
        };
        let out = match built {
            Built::Shipped => continue,
            Built::Test => &mut *tests,
            Built::Verus => &mut *verus,
        };
        if m.content.is_none() {
            let candidates = match (path_attr(&m.attrs), file.parent()) {
                (Some(p), Some(d)) => vec![d.join(p)],
                (Some(_), None) => Vec::new(),
                (None, _) => {
                    let name = m.ident.to_string();
                    vec![
                        dir.join(format!("{name}.rs")),
                        dir.join(name).join("mod.rs"),
                    ]
                }
            };
            out.extend(
                candidates
                    .into_iter()
                    .filter_map(|f| fs::canonicalize(f).ok()),
            );
        }
    }
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

/// Where the file `f` is built, given the module files declared for tests
/// alone (`tests`) and for Verus alone (`verus`): as one of those, or as a
/// file one of them declares, at any depth, which inherits its class; else
/// shipped.
fn module_class(f: &Path, tests: &[PathBuf], verus: &[PathBuf]) -> Built {
    let under = |fs: &[PathBuf]| {
        fs.iter()
            .any(|m| m == f || module_dir(m).is_some_and(|d| f.starts_with(d)))
    };
    if under(tests) {
        Built::Test
    } else if under(verus) {
        Built::Verus
    } else {
        Built::Shipped
    }
}

/// The `#[path = "..."]` of a module declaration, if it has one.
fn path_attr(attrs: &[Attribute]) -> Option<String> {
    attrs.iter().find_map(|a| match &a.meta {
        Meta::NameValue(nv) if nv.path.is_ident("path") => match &nv.value {
            verus_syn::Expr::Lit(l) => match &l.lit {
                verus_syn::Lit::Str(s) => Some(s.value()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    })
}

/// Where a `cfg` lets an item be built: by a shipped build; only where
/// Verus checks the code (`verus_keep_ghost`), so Verus checks it and no
/// shipped build compiles it; or only for tests (`test`, `kani`, the
/// `test-util` feature, which only tests enable: [`test_util_enablers`]
/// fails the run otherwise), which Verus does not see either.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Built {
    Shipped,
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
/// none of the test ones). Any other name or feature may be either.
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
    } else {
        Can { t: true, f: true }
    })
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
fn cfg_built(tokens: &str, atoms: &Atoms) -> Result<Built, String> {
    Ok(if cfg_eval(tokens, false, atoms)?.t {
        Built::Shipped
    } else if cfg_eval(tokens, true, atoms)?.t {
        Built::Verus
    } else {
        Built::Test
    })
}

/// Where `attrs`' `cfg`s, all of which must hold, let an item be built.
fn attrs_built(attrs: &[Attribute], atoms: &Atoms) -> Result<Built, String> {
    let mut b = Built::Shipped;
    for a in attrs {
        if a.path().is_ident("cfg")
            && let Meta::List(l) = &a.meta
        {
            b = match (b, cfg_built(&l.tokens.to_string(), atoms)?) {
                (Built::Test, _) | (_, Built::Test) => Built::Test,
                (Built::Verus, _) | (_, Built::Verus) => Built::Verus,
                _ => Built::Shipped,
            };
        }
    }
    Ok(b)
}

/// Whether a `cfg` predicate keeps an item out of shipped builds.
#[cfg(test)]
fn cfg_needs_test(tokens: &str, atoms: &Atoms) -> bool {
    matches!(cfg_built(tokens, atoms), Ok(Built::Test | Built::Verus))
}

/// The packages that a build of the workspace at `root` compiles with one
/// of the test-only `features` (pcre3's `test-util`), each with the
/// feature, so that [`cfg_needs_test`] could not leave its
/// items out, as cargo itself resolves them: `cargo tree` over every
/// member (a plain `cargo build` builds them all and unifies their
/// features), for every target, along normal and build edges only, once
/// with the default features and once with every member's every other
/// feature too ([`member_features`]), as a build such as
/// `--features pcre3-capi/hardened` turns them on. Tests' dev-dependencies
/// are the one place that may enable it, with a member's own `test-util`
/// feature, which forwards it. Reading cargo's resolution rather than the
/// manifests leaves no TOML form (quoting, tables, forwarding features,
/// globbed members) to miss. cargo runs `--locked --offline`: the lockfile
/// is checked in, and `make verus` has fetched every dependency before
/// verus-count runs. Its failure, or no cargo at all, is an error rather
/// than a pass.
fn test_util_enablers(root: &Path, test: &[String]) -> Result<Vec<(String, String)>, String> {
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
/// this trusted function's contract (verus/README.md, "Kani").
fn is_discharged(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg_attr")
            && matches!(&a.meta, Meta::List(l)
                if l.tokens.to_string().split_whitespace().collect::<String>()
                    == "any(),discharged_by=\"kani\"")
    })
}

fn is_external(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        let text = quote_attr(a);
        text.contains("external_body")
            || text.contains("verifier :: external") && !text.contains("external_fn_specification")
            || text.contains("verus_verify (external")
    })
}

/// Whether `attrs` hold Verus's attribute form, which marks a plain Rust
/// function outside `verus!` for verification: `#[verus_verify]` or
/// `#[verus_spec(..)]` (by any path), alone or as an attribute a
/// `cfg_attr` applies (`#[cfg_attr(feature = "verus", verus_spec(..))]`,
/// the form a port whose MSRV is below vstd's uses). Such a function is
/// classed as one inside `verus!` is ([`classify`]). A `cfg_attr` counts
/// only when Verus's build can meet its predicate (by [`cfg_eval`]): one
/// no build meets (`any()`) or only a test build meets (`test`) never
/// applies the attribute to the code Verus checks. A predicate it cannot
/// read is an error.
fn verus_attribute(attrs: &[Attribute], atoms: &Atoms) -> Result<bool, String> {
    const NAMES: [&str; 2] = ["verus_verify", "verus_spec"];
    for a in attrs {
        if a.path()
            .segments
            .last()
            .is_some_and(|s| NAMES.iter().any(|n| s.ident == n))
        {
            return Ok(true);
        }
        // `cfg_attr(PREDICATE, ATTR, ..)`: an ATTR at the top level after
        // the predicate's comma, by its last path segment.
        let Meta::List(l) = &a.meta else { continue };
        if !l.path.is_ident("cfg_attr") {
            continue;
        }
        let toks: Vec<TokenTree> = l.tokens.clone().into_iter().collect();
        let Some(comma) = toks
            .iter()
            .position(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == ','))
        else {
            continue;
        };
        let names_one = toks[comma + 1..].iter().enumerate().any(|(i, t)| {
            matches!(t, TokenTree::Ident(id) if NAMES.iter().any(|n| id == n))
                && !matches!(
                    toks.get(comma + 1 + i + 1),
                    Some(TokenTree::Punct(p)) if p.as_char() == ':'
                )
        });
        if !names_one {
            continue;
        }
        let predicate: TokenStream = toks[..comma].iter().cloned().collect();
        if cfg_eval(&predicate.to_string(), true, atoms)?.t {
            return Ok(true);
        }
    }
    Ok(false)
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
/// [`Counter::owner`]: its last path segment, without generic arguments
/// (`Machine < 'a , U >` is `Machine`); `None` for free functions.
fn owner_name(owner: &str) -> Option<String> {
    let base = owner.split('<').next().unwrap_or("");
    let last = base.rsplit("::").next().unwrap_or("").trim();
    (!last.is_empty()).then(|| last.to_owned())
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

fn quote_attr(a: &Attribute) -> String {
    match &a.meta {
        Meta::Path(p) => path_string(p),
        Meta::List(l) => format!("{} ({})", path_string(&l.path), l.tokens),
        Meta::NameValue(nv) => path_string(&nv.path),
    }
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
    /// Where `attrs` let an item be built ([`attrs_built`]); a `cfg` it
    /// cannot read is an error, and the item is taken as shipped.
    fn built(&mut self, attrs: &[Attribute]) -> Built {
        match attrs_built(attrs, &self.atoms) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{}: {e}", self.file.display());
                self.errors += 1;
                Built::Shipped
            }
        }
    }

    /// Whether a shipped build compiles an item with `attrs` here.
    fn item_ships(&mut self, attrs: &[Attribute]) -> bool {
        self.shipped && self.built(attrs) == Built::Shipped
    }

    fn record(
        &mut self,
        in_verus: bool,
        attrs: &[Attribute],
        sig: &Signature,
        body: Option<TokenStream>,
        start: usize,
        end: usize,
    ) -> Option<Class> {
        let built = self.built(attrs);
        if built == Built::Test {
            return None;
        }
        // Verus's attribute form verifies a function outside `verus!`.
        let in_verus = in_verus
            || verus_attribute(attrs, &self.atoms).unwrap_or_else(|e| {
                eprintln!("{}:{start}: {e}", self.file.display());
                self.errors += 1;
                false
            });
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
        if built == Built::Verus || !self.shipped {
            // A shipped build compiles none of this code, so its verified,
            // ghost and outside functions leave the tally. A trusted one
            // still counts: a shipped function's proof can rest on it, so
            // it is a seam.
            // With `verus_only_ghost`, its ghost functions join the ghost
            // column, which Verus checks all the same.
            match class {
                Some(class @ (Class::Trusted | Class::Discharged)) => {
                    self.tally(class, sig.ident.to_string(), start, end);
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
        let mut calls = Vec::new();
        let mut names = inputs::Body::default();
        if let Some(b) = &body {
            direct_calls(b.clone(), &mut calls);
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
            body: names,
            params: arity(sig).0,
            receiver: arity(sig).1,
            entry,
        });
        self.tally(class, sig.ident.to_string(), start, end);
        Some(class)
    }

    /// Counts function `name`, of class `class` on lines `start` to `end`,
    /// in the tally, lists it under `--list`, and keeps a discharged one for
    /// the record check ([`unrecorded`]).
    fn tally(&mut self, class: Class, name: String, start: usize, end: usize) {
        let lines = end.saturating_sub(start) + 1;
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
    /// metavariables, parsed. For pcre3-capi that is the struct's `Clone`
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
                    direct_calls(body, &mut calls);
                    self.fns.push(FnRef {
                        krate: krate.clone(),
                        file: file.clone(),
                        line,
                        owner: owner.clone(),
                        name: g.sig.ident.to_string(),
                        class: Class::Outside,
                        calls,
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
    /// call ([`is_c_abi`]) is an error.
    fn entry(&mut self, attrs: &[Attribute], sig: &Signature, line: usize) -> bool {
        let entry = exports_symbol(attrs);
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
    /// `class`: free functions, and the methods of an `impl` written there
    /// (such as a guard's `Drop`), each of that class. They are recorded
    /// for the recursion check ([`recursive`]) and the walks ([`inputs`],
    /// [`reach`]); no tally counts them, as none counts a nested function.
    fn nested(&mut self, stmts: &[verus_syn::Stmt], class: Class) {
        use verus_syn::__private::ToTokens;
        for s in stmts {
            let (owner, fns): (String, Vec<NestedFn>) = match s {
                verus_syn::Stmt::Item(Item::Fn(g)) => {
                    (String::new(), vec![(&g.attrs[..], &g.sig, &*g.block)])
                }
                verus_syn::Stmt::Item(Item::Impl(i)) => (
                    i.self_ty.to_token_stream().to_string(),
                    i.items
                        .iter()
                        .filter_map(|ii| match ii {
                            ImplItem::Fn(f) => Some((&f.attrs[..], &f.sig, &f.block)),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => continue,
            };
            for (attrs, sig, block) in fns {
                let body: TokenStream = block.stmts.iter().map(ToTokens::to_token_stream).collect();
                let mut calls = Vec::new();
                let names = inputs::body(body.clone());
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
                    body: names,
                    params: arity(sig).0,
                    receiver: arity(sig).1,
                    entry,
                });
                self.nested(&block.stmts, class);
            }
        }
    }

    fn items(&mut self, items: &[Item], in_verus: bool) {
        for item in items {
            match item {
                Item::Fn(f) => {
                    let start = f.sig.fn_token.span.start().line;
                    let end = f.block.brace_token.span.close().end().line;
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
                    let was = self.shipped;
                    self.shipped &= built == Built::Shipped;
                    let outer = std::mem::replace(
                        &mut self.owner,
                        verus_syn::__private::ToTokens::to_token_stream(&*i.self_ty).to_string(),
                    );
                    for ii in &i.items {
                        if let ImplItem::Fn(f) = ii {
                            let start = f.sig.fn_token.span.start().line;
                            let end = f.block.brace_token.span.close().end().line;
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
                        }
                    }
                    self.owner = outer;
                    self.shipped = was;
                }
                Item::Trait(t) => {
                    let built = self.built(&t.attrs);
                    if built == Built::Test {
                        continue;
                    }
                    let was = self.shipped;
                    self.shipped &= built == Built::Shipped;
                    if self.shipped {
                        self.traits.insert(t.ident.to_string());
                    }
                    let outer = std::mem::replace(&mut self.owner, t.ident.to_string());
                    for ti in &t.items {
                        if let TraitItem::Fn(f) = ti
                            && let Some(block) = &f.default
                        {
                            let start = f.sig.fn_token.span.start().line;
                            let end = block.brace_token.span.close().end().line;
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
                        }
                    }
                    self.owner = outer;
                    self.shipped = was;
                }
                Item::Mod(m) => {
                    let built = self.built(&m.attrs);
                    if built != Built::Test
                        && let Some((_, inner)) = &m.content
                    {
                        let was = self.shipped;
                        self.shipped &= built == Built::Shipped;
                        self.module.push(m.ident.to_string());
                        self.items(inner, in_verus);
                        self.module.pop();
                        self.shipped = was;
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
                    if m.mac.path.is_ident("thread_local") && self.item_ships(&m.attrs) =>
                {
                    inputs::statics_in(m.mac.tokens.clone(), true, &mut self.statics);
                }
                Item::Macro(m)
                    if self.fn_ptr_macro.as_ref().is_some_and(|f| {
                        m.mac.path.segments.last().is_some_and(|s| s.ident == f)
                    }) && self.item_ships(&m.attrs) =>
                {
                    self.fn_ptr(m);
                }
                Item::Macro(m)
                    if m.mac.path.is_ident("macro_rules")
                        && self
                            .fn_ptr_macro
                            .as_ref()
                            .is_some_and(|f| m.ident.as_ref().is_some_and(|i| i == f))
                        && self.item_ships(&m.attrs) =>
                {
                    self.fn_ptr_rule(&m.mac.tokens);
                }
                Item::Macro(m) if m.mac.path.is_ident("verus") => {
                    // A `verus!` block under a `cfg` is built where it says
                    // (`#[cfg(feature = "verus")] verus! { .. }`).
                    let built = self.built(&m.attrs);
                    if built == Built::Test {
                        continue;
                    }
                    let was = self.shipped;
                    self.shipped &= built == Built::Shipped;
                    match parse2::<verus_syn::File>(m.mac.tokens.clone()) {
                        Ok(f) => self.items(&f.items, true),
                        Err(e) => {
                            eprintln!("{}: verus! block: {e}", self.file.display());
                            self.errors += 1;
                        }
                    }
                    self.shipped = was;
                }
                _ => {}
            }
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
/// `publish = false` (pcre3-testdata, the conformance harness's reader).
fn ships(krate: &Path) -> bool {
    fs::read_to_string(krate.join("Cargo.toml")).map_or(true, |t| {
        !t.lines()
            .any(|l| l.split_whitespace().collect::<String>() == "publish=false")
    })
}

/// A crate the counter reads: its name, its directory (holding `src/`),
/// and whether it ships ([`ships`], or the configuration's `ships`).
struct Krate {
    name: String,
    dir: PathBuf,
    ships: bool,
}

/// The crates `c` names under `root`, in reading order: each
/// subdirectory of `[crates] dir` with a `src/`, in path order, named for
/// the subdirectory, then each of `[crates] members`.
fn crate_dirs(root: &Path, c: &Config) -> Result<Vec<Krate>, String> {
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
            dir: p,
        });
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

/// Exits 2 with `e` printed.
fn usage(e: impl std::fmt::Display) -> ! {
    eprintln!("{e}");
    std::process::exit(2);
}

fn main() {
    let mut list = false;
    let mut floor = None;
    let mut base_rev = None;
    let mut stats = None;
    let mut config_file = None;
    let mut root = PathBuf::from(".");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--list" {
            list = true;
        } else if a == "--version" {
            println!("verus-count {}", env!("CARGO_PKG_VERSION"));
            return;
        } else if a == "--floor" || a == "--base-rev" || a == "--rlimit" || a == "--config" {
            let Some(v) = args.next() else {
                usage(format!("{a} needs a value"));
            };
            if a == "--floor" {
                floor = Some(PathBuf::from(v));
            } else if a == "--rlimit" {
                stats = Some(PathBuf::from(v));
            } else if a == "--config" {
                config_file = Some(PathBuf::from(v));
            } else {
                base_rev = Some(v);
            }
        } else {
            root = PathBuf::from(a);
        }
    }
    let config_file = config_file.unwrap_or_else(|| root.join("verus/count.toml"));
    let cfg = Config::read(&config_file).unwrap_or_else(|e| usage(e));
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
        let enablers = match test_util_enablers(&root, &cfg.crates.test_features) {
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
    } in &crates
    {
        c.krate = name.clone();
        c.tallies.entry(c.krate.clone()).or_default();
        if !ships {
            unshipped.push(c.krate.clone());
        }
        let mut files = Vec::new();
        rust_files(&k.join("src"), &mut files);
        let mut skip = Vec::new();
        let mut verus_only = Vec::new();
        let mut bad = Vec::new();
        for f in &files {
            if let Ok(text) = fs::read_to_string(f)
                && let Ok(file) = parse_file(&cfg.stand_in(&text))
            {
                test_module_files(
                    f,
                    &file.items,
                    &c.atoms,
                    &mut skip,
                    &mut verus_only,
                    &mut bad,
                );
                if cfg.ledger.is_some() {
                    let module = ledger::module_path(&k.join("src"), f);
                    ledger::test_fns(&module, &file.items, &mut tests);
                }
            }
        }
        for e in &bad {
            eprintln!("{e}");
        }
        c.errors += bad.len();
        for f in files {
            let cf = fs::canonicalize(&f).unwrap_or_else(|_| f.clone());
            match module_class(&cf, &skip, &verus_only) {
                Built::Test => continue,
                // A module built only for Verus is read for its rlimits alone.
                Built::Verus => c.shipped = false,
                Built::Shipped => c.shipped = true,
            }
            let Ok(text) = fs::read_to_string(&f) else {
                eprintln!("{}: unreadable", f.display());
                c.errors += 1;
                continue;
            };
            c.file = f.strip_prefix(&root).unwrap_or(&f).to_path_buf();
            c.module = module_path(&c.krate, f.strip_prefix(k.join("src")).unwrap_or(&f));
            // A same-length stand-in for an identifier Verus keeps as a
            // keyword (pcre3's `assert::`) keeps the line numbers.
            match parse_file(&cfg.stand_in(&text)) {
                Ok(file) => c.items(&file.items, false),
                Err(e) => {
                    eprintln!("{}:{}: {e}", c.file.display(), e.span().start().line);
                    c.errors += 1;
                }
            }
        }
    }
    c.finish();
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
            base_rev.as_deref(),
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
    if list {
        fail_on_errors(c.errors);
        return;
    }
    let shipped = report(&root, &cfg, &c, &unshipped, &tests);
    let mut failed = false;
    if let Some(f) = floor {
        let fig = Figures::new(&cfg.floor.checked, &cfg.floor.path);
        let head = fig.read(
            &fs::read_to_string(&f).unwrap_or_else(|e| usage(format!("{}: {e}", f.display()))),
            &f.display().to_string(),
        );
        if let Some(name) = fig.missing(&head.floor) {
            usage(format!("{}: want a {name} figure", f.display()));
        }
        let (mut ok, report) = fig.check(&shipped, &head.floor);
        print!("{report}");
        if let Some(rev) = &base_rev {
            let path = &cfg.floor.path;
            match base_file(&root, rev, path) {
                Ok(Some(text)) => {
                    let base = fig
                        .for_base(cfg.floor.renamed_from.as_deref())
                        .read(&text, &format!("{rev}:{path}"));
                    let (lowering_ok, report) = fig.lowering(&head, &base);
                    print!("{report}");
                    ok &= lowering_ok;
                }
                Ok(None) => println!("ok coverage floor: {rev} has no {path} to compare with"),
                Err(e) => {
                    eprintln!("FAIL coverage floor: {e}");
                    std::process::exit(2);
                }
            }
        }
        failed |= !ok;
    }
    if let Some(s) = stats {
        // The crates with `verus!` code, as Verus spells their names.
        let verus_crates: Vec<String> = c
            .tallies
            .iter()
            .filter(|(_, t)| t.ghost + t.fns.iter().sum::<usize>() > t.fns[Class::Outside as usize])
            .map(|(k, _)| k.replace('-', "_"))
            .collect();
        failed |= !check_rlimit(
            &root,
            &cfg,
            &crates,
            &s,
            &verus_crates,
            &c,
            base_rev.as_deref(),
        );
    }
    if failed {
        std::process::exit(1);
    }
}

/// The recursion check (`[recursion]`, pcre3's verus/theorems/RN1.md):
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
) -> Floor {
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
        fail_on_errors(c.errors);
        return shares(&shipped, (0, 0));
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
    let (checked, warnings, errors) = ledger::match_ledger(&c.trusted, &seams);
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
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("FAIL ledger: {e}");
        }
        std::process::exit(1);
    }
    shares(&shipped, shipped_checked.discharged)
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
/// the report, and, with `base_rev`, holds the RLIMIT file to that revision's
/// copy. A crate of `crates` (those with `verus!` code) that the statistics
/// leave out fails the run with `base_rev` ([`rlimit::check_covered`]).
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
    base_rev: Option<&str>,
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
    let (covered, report) = rlimit::check_covered(crates, &stats, base_rev.is_some());
    print!("{report}");
    let shares = rlimit::shares(&stats, crates, limits).unwrap_or_else(|e| fail(e));
    let (mut ok, report) = rlimit::check(&shares, &head, 15, list);
    ok &= covered;
    print!("{report}");
    if let Some(rev) = base_rev {
        let base = match base_file(root, rev, list) {
            Ok(Some(text)) => {
                let base = rlimit::parse_rlimit_file(&text)
                    .unwrap_or_else(|e| fail(format!("{rev}:{list}: {e}")));
                let (kept, report) = rlimit::check_ratchet(&head, &base, list);
                print!("{report}");
                ok &= kept;
                base
            }
            Ok(None) => {
                println!("ok rlimit headroom: {rev} has no {list} to compare with");
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
/// `dirs` (name and directory, under `root`), as [`Counter`] reads them
/// from the working tree: each file `git grep` finds the attribute in,
/// parsed from `rev`'s copy.
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
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["-c", "safe.directory=*"])
            .args(args)
            .output()
            .map_err(|e| format!("git: {e}"))
    };
    let mut args = vec!["grep", "-l", "-F", "verifier::rlimit", rev, "--"];
    let src_args: Vec<String> = srcs.iter().map(|(_, s)| s.display().to_string()).collect();
    args.extend(src_args.iter().map(String::as_str));
    let found = git(&args)?;
    // git grep exits 1 when nothing matches.
    if !found.status.success() && found.status.code() != Some(1) {
        return Err(format!(
            "git grep in {rev}: {}",
            String::from_utf8_lossy(&found.stderr).trim()
        ));
    }
    let mut c = Counter::new(
        false,
        Atoms::of(cfg),
        cfg.reach.as_ref().and_then(|r| r.fn_ptr_macro.clone()),
    );
    for line in String::from_utf8_lossy(&found.stdout).lines() {
        // `REV:<crate dir>/src/<path>`.
        let path = line.strip_prefix(&format!("{rev}:")).unwrap_or(line);
        let rel = Path::new(path);
        let Some((krate, under)) = srcs
            .iter()
            .find_map(|(k, s)| rel.strip_prefix(s).ok().map(|u| (k, u)))
        else {
            continue;
        };
        let shown = git(&["show", &format!("{rev}:{path}")])?;
        if !shown.status.success() {
            return Err(format!(
                "git show {rev}:{path}: {}",
                String::from_utf8_lossy(&shown.stderr).trim()
            ));
        }
        let text = cfg.stand_in(&String::from_utf8_lossy(&shown.stdout));
        c.krate = krate.clone();
        c.file = rel.to_path_buf();
        c.module = module_path(&c.krate, under);
        match parse_file(&text) {
            Ok(file) => c.items(&file.items, false),
            Err(e) => return Err(format!("{rev}:{path}: {e}")),
        }
    }
    if c.errors > 0 {
        return Err(format!(
            "{rev}: {} rlimit attributes did not parse",
            c.errors
        ));
    }
    Ok(c.limits)
}

/// The figures of `t`, as verus/FLOOR records them: the verified share of
/// functions and of lines, then the verified and discharged share of each,
/// with `ledger` the functions and lines the generated ledger discharges
/// besides the Kani marker's.
fn shares(t: &Tally, ledger: (usize, usize)) -> Floor {
    let (fns, lines) = (t.fns.iter().sum(), t.lines.iter().sum());
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

/// `REV:FILE` under `root`'s repository (FILE verus/FLOOR or
/// verus/RLIMIT), failing closed: a
/// revision that does not resolve to a commit (a shallow clone, another
/// uid's checkout git refuses, an empty name) is an error, and only a
/// resolved commit without the file is `None`.
fn base_file(root: &Path, rev: &str, file: &str) -> Result<Option<String>, String> {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["-c", "safe.directory=*"])
            .args(args)
            .output()
            .map_err(|e| format!("git: {e}"))
    };
    let commit = git(&[
        "rev-parse",
        "--verify",
        "--quiet",
        &format!("{rev}^{{commit}}"),
    ])?;
    if !commit.status.success() {
        return Err(format!(
            "base revision {rev:?} does not resolve to a commit, so {file} cannot be compared with it"
        ));
    }
    let commit = String::from_utf8_lossy(&commit.stdout).trim().to_owned();
    let path = format!("{commit}:{file}");
    if !git(&["cat-file", "-e", &path])?.status.success() {
        return Ok(None);
    }
    let shown = git(&["show", &path])?;
    if !shown.status.success() {
        return Err(format!(
            "git show {path}: {}",
            String::from_utf8_lossy(&shown.stderr).trim()
        ));
    }
    Ok(Some(String::from_utf8_lossy(&shown.stdout).into_owned()))
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
    fn check(&self, got: &Floor, want: &Floor) -> (bool, String) {
        let file = &self.file;
        let mut ok = true;
        let mut report = String::new();
        for (k, name) in self.names.iter().enumerate() {
            let (Some(g), Some(w)) = (got.0[k], want.0[k]) else {
                continue;
            };
            let what = MEANING[k];
            if g < w {
                ok = false;
                report += &format!(
                    "FAIL coverage floor: {name} {what} {}%, below the floor's {}%\n",
                    shown(g),
                    shown(w)
                );
            } else if g > w {
                report += &format!(
                    "ok coverage floor: {name} {what} {}%, above the floor's {}%; raise {file} to `{name} {}`\n",
                    shown(g),
                    shown(w),
                    shown(g)
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

/// The compile-inputs check (`[inputs]`, pcre3's verus/theorems/CD1.md,
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
/// macro block (pcre3's `width::exports!`) in its file exports
/// (`exported`); and each `conversion` method, one-argument and without a
/// receiver (pcre3's `from_c`, the `width::FromC` conversions the exports'
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
/// the tables as the report names them); with `base_rev`, a row that
/// revision's REACH does not have is marked `new`
/// ([`reach::check_added`]). Prints what it finds; `false` when it fails.
fn entry_reach(
    root: &Path,
    r: &config::Reach,
    stand_ins: &[[String; 2]],
    c: &Counter,
    unshipped: &[String],
    seams: (&std::collections::BTreeSet<String>, &str),
    base_rev: Option<&str>,
) -> bool {
    let (fns, traits, aliases) = (&c.fns, &c.traits, &c.aliases);
    let list = r.path.as_str();
    let text = fs::read_to_string(root.join(list)).unwrap_or_default();
    let funcs = funcs(fns);
    let shipped: std::collections::BTreeSet<&str> = fns
        .iter()
        .map(|f| f.krate.as_str())
        .filter(|k| !unshipped.iter().any(|u| u == k))
        .collect();
    let crates: Vec<&str> = shipped.into_iter().collect();
    let files: std::collections::BTreeSet<PathBuf> = fns
        .iter()
        .filter(|f| crates.contains(&f.krate.as_str()))
        .map(|f| f.file.clone())
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
    if let Some(rev) = base_rev {
        match base_file(root, rev, list) {
            Ok(Some(base)) => {
                report
                    .failures
                    .extend(reach::check_added(&head, &reach::rows(&base), rev, list))
            }
            Ok(None) => println!("ok entry reach: {rev} has no {list} to compare with"),
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

    /// The `cfg` atoms of pcre3's configuration, which the fixtures use:
    /// `test`, `kani` and the `test-util` feature test-only, and
    /// `verus_keep_ghost` and the `verus` feature Verus-only.
    fn atoms() -> Atoms {
        Atoms {
            test: vec!["test".into(), "kani".into(), "feature=\"test-util\"".into()],
            verus: vec!["verus_keep_ghost".into(), "feature=\"verus\"".into()],
        }
    }

    fn test_util() -> Vec<String> {
        vec!["test-util".into()]
    }

    /// A counter as pcre3's configuration sets one up, with its `fn_ptr`
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
            // Features that forward it, on by default or not (#668's fourth
            // review: pcre3-capi's `hardened`, which docs/design.md, ci/abi.sh
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
            // The forms a manifest scanner missed (#668's reviews): single
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
            let got: Vec<String> = test_util_enablers(&root, &test_util())
                .unwrap()
                .iter()
                .map(|(p, f)| {
                    assert_eq!(f, "test-util");
                    p.split(' ').next().unwrap_or_default().to_string()
                })
                .collect();
            let wanted: Vec<String> = if want { vec!["lib".into()] } else { Vec::new() };
            assert_eq!(got, wanted, "{files:?}: got = {got:?}, wanted = {wanted:?}");
            let _ = fs::remove_dir_all(&root);
        }
    }

    /// A module declared for tests alone is left out with its file, and one
    /// declared for Verus alone is read for its rlimits only, whichever of
    /// the forms the cfg takes; a module a shipped build compiles is
    /// neither.
    #[test]
    fn test_and_verification_modules_are_left_out() {
        let root = std::env::temp_dir().join(format!("verus-count-modules-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        let lib = "#[cfg(any(test, verus_keep_ghost))]\nmod grouped;\n#[cfg(verus_keep_ghost)]\nmod layout;\n#[cfg(test)]\nmod tests;\nmod shipped;\n#[cfg(any(test, unix))]\nmod either;\n";
        fs::write(root.join("src/lib.rs"), lib).unwrap();
        for m in ["grouped", "layout", "tests", "shipped", "either"] {
            fs::write(root.join(format!("src/{m}.rs")), "").unwrap();
        }
        let file = parse_file(lib).unwrap();
        let (mut tests, mut verus, mut errors) = (Vec::new(), Vec::new(), Vec::new());
        test_module_files(
            &root.join("src/lib.rs"),
            &file.items,
            &atoms(),
            &mut tests,
            &mut verus,
            &mut errors,
        );
        let stems = |fs: &[PathBuf]| -> Vec<String> {
            let mut s: Vec<String> = fs
                .iter()
                .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .collect();
            s.sort();
            s
        };
        let (tests, verus) = (stems(&tests), stems(&verus));
        assert_eq!(
            tests,
            ["tests"],
            "test-only: got = {tests:?}, wanted = [tests]"
        );
        assert_eq!(
            verus,
            ["grouped", "layout"],
            "Verus-only: got = {verus:?}, wanted = [grouped, layout]"
        );
        assert!(errors.is_empty(), "errors: got = {errors:?}, wanted = []");
        // A file a test- or Verus-only module declares, at any depth,
        // inherits its class, beside a `foo.rs` (`foo/bar.rs`) as beside a
        // `mod.rs`.
        let (mut tests_f, mut verus_f) = (Vec::new(), Vec::new());
        test_module_files(
            &root.join("src/lib.rs"),
            &file.items,
            &atoms(),
            &mut tests_f,
            &mut verus_f,
            &mut errors,
        );
        for sub in [
            "layout/deep.rs",
            "layout/deep/deeper.rs",
            "tests/inner.rs",
            "shipped/x.rs",
            "layout_other.rs",
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
            ("shipped/x.rs", Built::Shipped),
            ("layout_other.rs", Built::Shipped),
        ] {
            let f = fs::canonicalize(root.join("src").join(sub)).unwrap();
            let got = module_class(&f, &tests_f, &verus_f);
            assert_eq!(got, want, "{sub}: got = {got:?}, wanted = {want:?}");
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
        assert!(test_util_enablers(&root, &test_util()).is_err());
        let _ = fs::remove_dir_all(&root);
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
                Built::Shipped,
            ),
            ("any (test , all (verus_keep_ghost , unix))", Built::Verus),
            ("all (test , any (verus_keep_ghost , unix))", Built::Test),
        ] {
            let got = cfg_built(cfg, &atoms());
            assert_eq!(got, Ok(want), "{cfg}: got = {got:?}, wanted = {want:?}");
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

    /// pcre3-capi's `width::fn_ptr!`, as width.rs defines it.
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

    /// A marked function built only for Verus is discharged as a shipped
    /// one is, so it needs a Kani record too. The case is S6's usize fill
    /// axiom (`crates/pcre3-unicode/src/vspec.rs`) moved under
    /// `cfg(verus_keep_ghost)`, against `verus/seams/kani-s6.md` with and
    /// without its quote.
    #[test]
    fn a_verus_only_discharged_function_needs_a_record() {
        let file = "crates/pcre3-unicode/src/vspec.rs";
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
            ("pcre3-syntax", "lib.rs", "pcre3_syntax"),
            ("pcre3", "vm/exec.rs", "pcre3::vm::exec"),
            ("pcre3", "vm/mod.rs", "pcre3::vm"),
            ("pcre3", "vm/assert.rs", "pcre3::vm::assert"),
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

    /// Verus's attribute form (expat-rs's: a contract in `verus_spec` under
    /// a `cfg_attr`, for a port whose MSRV is below vstd's) verifies a plain
    /// function outside `verus!`; `verus_verify(external_body)` is trusted.
    /// Negative: a `cfg_attr` of another attribute, a path that only
    /// passes through a `verus_spec` module, or a `cfg_attr` whose
    /// predicate Verus's build never meets (`any()`, `test`, a predicate
    /// with a test atom, the negated Verus feature), is not the attribute
    /// form.
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
fn plain() {}
#[cfg(feature = "verus")]
verus! {
    spec fn lead(b: u8) -> nat { 0 }
    fn verus_only() {}
}
"#;
        let (t, errors) = count(src);
        assert_eq!(errors, 0, "errors: got = {errors}, wanted = 0");
        // spec_attr, direct, by_path verified; trusted_attr and assumes
        // trusted; other_attr, through_a_module, never, in_tests,
        // with_a_test_atom, not_for_verus and plain outside. The
        // `verus!` block is built only for Verus (`feature = "verus"`), so
        // its exec function and its spec function leave the tally.
        let want = [3, 2, 7, 0];
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
            shares(&t, (2, 30)),
            Floor([Some(600), Some(600), Some(800), Some(900)])
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
        let _ = fs::remove_dir_all(&root);
    }
}
