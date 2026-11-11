//! The port's configuration, `verus/count.toml` (kit/count/README.md):
//! which crates to read and which ship, which `cfg`s are test-only, and
//! where each table the checks read lives. A check whose section is
//! missing does not run, and with `--base-rev` a change that removes or
//! narrows one the base revision's copy has fails
//! ([`Config::narrowed_from`]); every path is relative to the repository
//! root.
//!
//! Unknown keys are an error, so a misspelled section or key fails the
//! run rather than turning a check off.

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    pub(crate) crates: Crates,
    #[serde(default)]
    pub(crate) floor: Floor,
    #[serde(default)]
    pub(crate) rlimit: Rlimit,
    #[serde(default)]
    pub(crate) kani: Kani,
    #[serde(default)]
    pub(crate) seams: Seams,
    /// The recursion check (`recursive`), on when present.
    pub(crate) recursion: Option<Recursion>,
    /// The entry-point reachability gate ([`crate::reach`]), on when
    /// present.
    pub(crate) reach: Option<Reach>,
    /// The compile-inputs check ([`crate::inputs`]), on when present.
    pub(crate) inputs: Option<Inputs>,
    /// The token-mint gate ([`crate::mints`]), on when present.
    pub(crate) mints: Option<Mints>,
    /// The generated seam ledger ([`crate::ledger`]): when present, it and
    /// not the Kani marker says which trusted functions are discharged.
    pub(crate) ledger: Option<crate::ledger::Config>,
}

/// The crates the counter reads.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Crates {
    /// A directory each of whose subdirectories with a `src/` is a crate,
    /// named for the subdirectory (`crates`).
    pub(crate) dir: Option<String>,
    /// Crates named explicitly, read after `dir`'s.
    #[serde(default)]
    pub(crate) members: Vec<Member>,
    /// `cfg` names that only test builds set (`test`, `kani`): an item
    /// that needs one is left out.
    #[serde(default = "default_test_cfgs")]
    pub(crate) test_cfgs: Vec<String>,
    /// Features that only tests enable: an item under `feature = "F"` is
    /// left out, and a build of the workspace that enables one fails the
    /// run (`test_util_enablers`).
    #[serde(default)]
    pub(crate) test_features: Vec<String>,
    /// Features that only Verus's build enables (expat-rs's `verus`, which
    /// pulls in vstd for a port whose MSRV is below vstd's): an item under
    /// `feature = "F"` is built only for Verus, as one under
    /// `verus_keep_ghost` is.
    #[serde(default)]
    pub(crate) verus_features: Vec<String>,
    /// Whether the ghost column counts the `spec` and `proof` functions of
    /// code built only for Verus too (challoc's counter does); by default
    /// it counts shipped code's alone. No FLOOR figure reads it.
    #[serde(default)]
    pub(crate) verus_only_ghost: bool,
    /// Text to replace before parsing, each with a stand-in of the same
    /// length so lines keep their numbers: an identifier Verus keeps as a
    /// keyword (`["assert::", "assrtx::"]`).
    #[serde(default)]
    pub(crate) stand_ins: Vec<[String; 2]>,
}

/// One crate: its name, as cargo spells it, its directory, which holds
/// `src/` and `Cargo.toml`, and whether it ships. Without `ships`, a
/// crate ships unless its Cargo.toml says `publish = false`, as every
/// crate `dir` finds; with it, as it says (challoc's one crate is
/// `publish = false`, since it is not a crates.io crate, and ships).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Member {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) ships: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Floor {
    /// The FLOOR file a base revision's copy is read from (`--base-rev`).
    #[serde(default = "default_floor")]
    pub(crate) path: String,
    /// The prefix of the two verified-or-discharged figures: `checked`
    /// (`checked-functions`) or `combined`.
    #[serde(default = "default_checked")]
    pub(crate) checked: String,
    /// A prefix the base revision's copy may still give those two figures
    /// under (challoc's `combined`), read there as `checked` names them, so
    /// the change that renames them compares with its base. The file under
    /// test takes `checked` alone.
    #[serde(default)]
    pub(crate) renamed_from: Option<String>,
    /// Whether the configuration has a `[floor]` table: a base revision
    /// whose copy has one must hold the FLOOR file it names.
    #[serde(skip_deserializing, default = "present")]
    pub(crate) declared: bool,
}

impl Default for Floor {
    fn default() -> Self {
        Floor {
            path: default_floor(),
            checked: default_checked(),
            renamed_from: None,
            declared: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Rlimit {
    /// The RLIMIT file `--rlimit` holds the statistics to.
    #[serde(default = "default_rlimit")]
    pub(crate) path: String,
    /// Whether the configuration has an `[rlimit]` table, as
    /// [`Floor::declared`].
    #[serde(skip_deserializing, default = "present")]
    pub(crate) declared: bool,
}

impl Default for Rlimit {
    fn default() -> Self {
        Rlimit {
            path: default_rlimit(),
            declared: false,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Kani {
    /// The directory of the `kani-*.md` records that cover the functions
    /// marked `discharged_by = "kani"`. With none, a marked function fails
    /// the run.
    pub(crate) records: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Seams {
    /// Files holding a seams table (`verus/SEAMS.md`).
    #[serde(default)]
    pub(crate) tables: Vec<String>,
    /// A directory whose every `*.md` holds a theorem's seams table.
    pub(crate) theorems: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Recursion {
    /// The file whose table names each verified recursive function.
    pub(crate) table: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reach {
    /// The REACH file.
    pub(crate) path: String,
    /// The last segment of a macro whose `fn BODY [..]` entries name
    /// bodies it writes exported wrappers for (`exports`).
    pub(crate) exports_macro: Option<String>,
    /// A `macro_rules!` of one rule that writes a function-pointer
    /// wrapper struct's impls, expanded from its definition (`fn_ptr`).
    pub(crate) fn_ptr_macro: Option<String>,
    /// A one-argument associated function the exported wrappers apply to
    /// their arguments, rooted in every impl (`from_c`).
    pub(crate) conversion: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Inputs {
    /// The file whose table names each allowed read.
    pub(crate) table: String,
    /// The crates the walk follows calls through.
    pub(crate) crates: Vec<String>,
    /// The functions the walk starts from.
    pub(crate) roots: Vec<Root>,
    /// Whether every function that calls `get_or_init` is a root too.
    #[serde(default)]
    pub(crate) lazy: bool,
    /// The roots in words, for the report: "`Regex::new` and the lazy
    /// builders".
    pub(crate) describe: String,
}

/// A root of the compile-inputs walk: the free function or method `name`
/// (of `owner`, when given) in a file whose path ends in `file`.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Root {
    pub(crate) file: String,
    pub(crate) owner: Option<String>,
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Mints {
    /// The MINTS file.
    pub(crate) path: String,
    /// The one trusted function that makes a token (`c_mint`).
    pub(crate) mint: String,
    /// The trait each kind of token implements (`CToken`).
    pub(crate) token_trait: String,
    /// The file that defines the mint function.
    pub(crate) home: String,
}

fn default_test_cfgs() -> Vec<String> {
    vec!["test".into(), "kani".into()]
}

fn default_floor() -> String {
    "verus/FLOOR".into()
}

fn default_checked() -> String {
    "checked".into()
}

fn default_rlimit() -> String {
    "verus/RLIMIT".into()
}

fn present() -> bool {
    true
}

impl Config {
    /// Reads `path`, failing on a file it cannot read or parse, on a key
    /// it does not know, and on a configuration that names no crate.
    pub(crate) fn read(path: &Path) -> Result<Config, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Config::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub(crate) fn parse(text: &str) -> Result<Config, String> {
        let c: Config = toml::from_str(text).map_err(|e| e.to_string())?;
        if c.crates.dir.is_none() && c.crates.members.is_empty() {
            return Err("[crates] names no crate: give `dir`, `members`, or both".into());
        }
        let read = c.read_names();
        for [from, to] in &c.crates.stand_ins {
            if let Some(name) = read.iter().find(|n| rewrites(from, to, n)) {
                return Err(format!(
                    "stand-in [{from:?}, {to:?}] would rewrite `{name}`, which the counter reads to class code; a stand-in may only change text the classifier does not read"
                ));
            }
        }
        Ok(c)
    }

    /// The names the counter's classes and checks read in the source,
    /// which no stand-in may rewrite: a stand-in that turned
    /// `external_body` or `assume` into other text would class a trusted
    /// function verified, one that rewrote `cfg` or a test cfg would move
    /// code in or out of the shipped tally, and one that rewrote `fn`
    /// would drop functions from it. Verus reads the original text, so it
    /// still passes. The configured macro, conversion and mint names are
    /// among them.
    fn read_names(&self) -> Vec<String> {
        const FIXED: [&str; 24] = [
            "external_body",
            "external",
            "assume",
            "admit",
            "assume_new",
            "verus_verify",
            "verus_spec",
            "discharged_by",
            "kani",
            "verifier",
            "rlimit",
            "cfg",
            "cfg_attr",
            "feature",
            "verus_keep_ghost",
            "verus",
            "proof",
            "spec",
            "axiom",
            "fn",
            "no_mangle",
            "export_name",
            "thread_local",
            "macro_rules",
        ];
        let reach = self.reach.iter().flat_map(|r| {
            [&r.exports_macro, &r.fn_ptr_macro, &r.conversion]
                .into_iter()
                .flatten()
                .cloned()
        });
        let mints = self
            .mints
            .iter()
            .flat_map(|m| [m.mint.clone(), m.token_trait.clone()]);
        FIXED
            .iter()
            .map(|n| n.to_string())
            .chain(self.crates.test_cfgs.iter().cloned())
            .chain(self.crates.test_features.iter().cloned())
            .chain(self.crates.verus_features.iter().cloned())
            .chain(reach)
            .chain(mints)
            .collect()
    }

    /// The `cfg` atoms that only test builds meet, as `cfg_eval` compares
    /// them (whitespace removed): each test cfg name, and
    /// `feature="F"` for each test feature.
    pub(crate) fn test_atoms(&self) -> Vec<String> {
        self.crates
            .test_cfgs
            .iter()
            .cloned()
            .chain(
                self.crates
                    .test_features
                    .iter()
                    .map(|f| format!("feature=\"{f}\"")),
            )
            .collect()
    }

    /// `text` with each stand-in applied.
    pub(crate) fn stand_in(&self, text: &str) -> String {
        stand_in(&self.crates.stand_ins, text)
    }

    /// How this configuration turns off or narrows a gate that `base`, the
    /// base revision's, runs: one line per change. A gate whose section is
    /// left out does not run and prints nothing, so deleting its section
    /// would pass every check it makes; the run fails instead, as FLOOR
    /// and RLIMIT fail a loosened figure. The changes are a `[recursion]`,
    /// `[reach]`, `[inputs]` or `[mints]` section removed; a `[reach]`
    /// exports macro, function-pointer macro or conversion removed or
    /// renamed (each adds entry points); an `[inputs]` crate or root
    /// removed, or `lazy` turned off (each narrows the walk); a `[crates]`
    /// `dir` removed or changed, a member removed, or a crate's `ships`
    /// changed to anything but `true` (each can take a crate out of the
    /// shipped tally and the walks); a `test_cfgs` or `verus_features`
    /// name added (each takes the items under it out of the shipped
    /// tally). Renaming a table, a mint, or the FLOOR, RLIMIT or REACH
    /// file is not a narrowing: the gate still runs, on the files and
    /// names the change gives it, and the base's copy of each file is read
    /// at the path the base's configuration gives it.
    pub(crate) fn narrowed_from(&self, base: &Config) -> Vec<String> {
        let mut out = Vec::new();
        let (h, b) = (&self.crates, &base.crates);
        if let Some(d) = &b.dir
            && h.dir.as_ref() != Some(d)
        {
            out.push(format!(
                "[crates] dir is no longer {d:?}, so the crates under it may no longer be read"
            ));
        }
        let ships = |s: Option<bool>| s.map_or("unset".to_owned(), |s| s.to_string());
        for m in &b.members {
            match h.members.iter().find(|n| n.name == m.name) {
                None => out.push(format!(
                    "[crates] members drops {:?}, so it is no longer read",
                    m.name
                )),
                Some(n) if n.ships != m.ships && n.ships != Some(true) => out.push(format!(
                    "[crates] member {:?} changes ships from {} to {}, so it may no longer count as shipped",
                    m.name,
                    ships(m.ships),
                    ships(n.ships)
                )),
                Some(_) => {}
            }
        }
        for (key, here, there) in [
            ("test_cfgs", &h.test_cfgs, &b.test_cfgs),
            ("verus_features", &h.verus_features, &b.verus_features),
        ] {
            for n in here.iter().filter(|n| !there.contains(n)) {
                out.push(format!(
                    "[crates] {key} adds {n:?}, so the items under it leave the shipped tally"
                ));
            }
        }
        for (section, here, there) in [
            (
                "recursion",
                self.recursion.is_some(),
                base.recursion.is_some(),
            ),
            ("reach", self.reach.is_some(), base.reach.is_some()),
            ("inputs", self.inputs.is_some(), base.inputs.is_some()),
            ("mints", self.mints.is_some(), base.mints.is_some()),
        ] {
            if there && !here {
                out.push(format!(
                    "the [{section}] section is gone, so its gate no longer runs"
                ));
            }
        }
        if let (Some(h), Some(b)) = (&self.reach, &base.reach) {
            for (key, here, there) in [
                ("exports_macro", &h.exports_macro, &b.exports_macro),
                ("fn_ptr_macro", &h.fn_ptr_macro, &b.fn_ptr_macro),
                ("conversion", &h.conversion, &b.conversion),
            ] {
                match (there, here) {
                    (Some(_), None) => out.push(format!(
                        "[reach] {key} is gone, so the entry points it named are no longer walked"
                    )),
                    (Some(t), Some(n)) if t != n => out.push(format!(
                        "[reach] {key} changes from {t:?} to {n:?}, so the entry points {t:?} named are no longer walked"
                    )),
                    _ => {}
                }
            }
        }
        if let (Some(h), Some(b)) = (&self.inputs, &base.inputs) {
            for k in b.crates.iter().filter(|k| !h.crates.contains(k)) {
                out.push(format!(
                    "[inputs] crates drops {k:?}, so the walk no longer follows calls through it"
                ));
            }
            for r in b.roots.iter().filter(|r| !h.roots.contains(r)) {
                let owner = r.owner.as_ref().map_or(String::new(), |o| format!("{o}::"));
                out.push(format!(
                    "[inputs] roots drops {owner}{} in {}, so the walk no longer starts there",
                    r.name, r.file
                ));
            }
            if b.lazy && !h.lazy {
                out.push(
                    "[inputs] lazy is turned off, so the lazy builders are no longer roots".into(),
                );
            }
        }
        out
    }
}

/// Whether replacing `from` with `to` can change an occurrence of `name`:
/// whether `name` can sit in some text so that it overlaps a character
/// of `from` that `to` changes, agreeing with `from` wherever the two
/// overlap. Text `from` shares with `to` at its start or end is left as
/// it was, so a name that touches only that part is safe (challoc's
/// `let tracked =` keeps `let track` and ` =`).
fn rewrites(from: &str, to: &str, name: &str) -> bool {
    if !from.is_empty() && from == to {
        return false;
    }
    let f: Vec<char> = from.chars().collect();
    let t: Vec<char> = to.chars().collect();
    let n: Vec<char> = name.chars().collect();
    let prefix = f.iter().zip(&t).take_while(|(a, b)| a == b).count();
    let suffix = f
        .iter()
        .rev()
        .zip(t.iter().rev())
        .take_while(|(a, b)| a == b)
        .count()
        .min(f.len() - prefix);
    // The changed characters of `from`, half open; empty when `to` only
    // inserts, at the point between the kept parts.
    let (lo, hi) = (prefix as isize, (f.len() - suffix) as isize);
    let (f_len, n_len) = (f.len() as isize, n.len() as isize);
    // `name` placed with its first character at `from`'s offset `o`.
    (1 - n_len..f_len.max(1)).any(|o| {
        let end = o + n_len;
        let touches = if lo == hi {
            o < lo && lo < end
        } else {
            o < hi && lo < end
        };
        touches
            && n.iter().enumerate().all(|(k, ch)| {
                let at = o + k as isize;
                at < 0 || at >= f_len || f[at as usize] == *ch
            })
    })
}

/// `text` with each of `pairs`' first strings replaced by its second.
pub(crate) fn stand_in(pairs: &[[String; 2]], text: &str) -> String {
    pairs
        .iter()
        .fold(text.to_owned(), |t, [from, to]| t.replace(from, to))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_config_reads() {
        let c = Config::parse("[crates]\ndir = \"crates\"\n[floor]\n").expect("parse");
        assert_eq!(c.crates.dir.as_deref(), Some("crates"));
        assert_eq!(c.floor.path, "verus/FLOOR");
        assert_eq!(c.floor.checked, "checked");
        assert_eq!(c.floor.renamed_from, None);
        assert_eq!(c.rlimit.path, "verus/RLIMIT");
        // `[floor]` is given, with every key defaulted; `[rlimit]` is not.
        assert!(c.floor.declared && !c.rlimit.declared);
        assert_eq!(c.test_atoms(), ["test", "kani"]);
        assert!(c.recursion.is_none() && c.reach.is_none());
        assert!(c.inputs.is_none() && c.mints.is_none() && c.ledger.is_none());
    }

    #[test]
    fn test_features_are_atoms() {
        let c = Config::parse(
            "[crates]\ndir = \"crates\"\ntest_cfgs = [\"test\"]\ntest_features = [\"test-util\"]\n[floor]\n",
        )
        .expect("parse");
        assert_eq!(c.test_atoms(), ["test", "feature=\"test-util\""]);
    }

    #[test]
    fn bad_configs_fail() {
        for (text, want) in [
            ("[floor]\n", "missing field `crates`"),
            ("[crates]\n[floor]\n", "names no crate"),
            (
                "[crates]\ndir = \"c\"\n[floor]\n[reach]\npth = \"x\"\n",
                "unknown field",
            ),
            (
                "[crates]\ndir = \"c\"\n[floor]\n[recursoin]\ntable = \"x\"\n",
                "unknown field",
            ),
            (
                "[crates]\ndir = \"c\"\n[floor]\n[mints]\npath = \"x\"\n",
                "missing field",
            ),
            ("[crates", "TOML parse error"),
        ] {
            let got = Config::parse(text).map(|_| ()).unwrap_err();
            assert!(got.contains(want), "{text:?}: got = {got}, want = {want}");
        }
    }

    /// pcre3's and challoc's stand-ins pass. Negative: a stand-in that
    /// rewrites a name the classifier reads, by containing it, by being
    /// part of it, or by overlapping its end, is refused, and so is one of
    /// a configured name or an empty `from`.
    #[test]
    fn stand_ins_may_not_rewrite_what_the_classifier_reads() {
        let with = |pairs: &str, extra: &str| {
            Config::parse(&format!(
                "[crates]\ndir = \"crates\"\ntest_features = [\"test-util\"]\nstand_ins = [{pairs}]\n{extra}"
            ))
        };
        for ok in [
            r#"["assert::", "assrtx::"]"#,
            r#"["let tracked =", "let trackd_ ="]"#,
            r#"["same", "same"]"#,
        ] {
            if let Err(e) = with(ok, "") {
                panic!("{ok}: got = {e}, wanted = ok");
            }
        }
        for (bad, extra, name) in [
            (r#"["external_body", "externalxbody"]"#, "", "external_body"),
            (r#"["assume(", "assum_("]"#, "", "assume"),
            (r#"["admit", "admix"]"#, "", "admit"),
            (
                r#"["Tracked::assume_new", "Tracked::assume_neW"]"#,
                "",
                "assume_new",
            ),
            (r#"["verus_verify", "verus_verifx"]"#, "", "verus_verify"),
            (r#"["verus_spec", "verus_spex"]"#, "", "verus_spec"),
            (r#"["discharged_by", "discharged_bx"]"#, "", "discharged_by"),
            (r#"["body]", "bodx]"]"#, "", "external_body"),
            (r#"["y]", "x]"]"#, "", "external_body"),
            (r##"["#[cfg(", "#[cfx("]"##, "", "cfg"),
            (r#"["(test)", "(tesx)"]"#, "", "test"),
            (r#"["\"test-util\"", "\"test-utix\""]"#, "", "test-util"),
            (r#"["fn ", "fx "]"#, "", "fn"),
            (r#"["", "x"]"#, "", "external_body"),
            (
                r#"["c_mint(", "c_minx("]"#,
                "[mints]\npath = \"m\"\nmint = \"c_mint\"\ntoken_trait = \"CToken\"\nhome = \"h\"\n",
                "c_mint",
            ),
        ] {
            match with(bad, extra) {
                Ok(_) => panic!("{bad}: got = ok, wanted = refused"),
                Err(e) => assert!(
                    e.contains(&format!("would rewrite `{name}`")),
                    "{bad}: got = {e}, wanted = rewrites `{name}`"
                ),
            }
        }
    }

    /// pcre3's configuration, which every gate section has, with a member
    /// and a Verus feature so that every `[crates]` key it holds is set.
    const FULL: &str = r#"
[crates]
dir = "crates"
members = [{ name = "extra", path = "extra", ships = true }]
test_cfgs = ["test", "kani"]
verus_features = ["verus"]
[recursion]
table = "verus/theorems/RN1.md"
[reach]
path = "verus/REACH"
exports_macro = "exports"
fn_ptr_macro = "fn_ptr"
conversion = "from_c"
[inputs]
table = "verus/theorems/CD1.md"
crates = ["pcre3", "pcre3-syntax", "pcre3-unicode"]
roots = [
  { file = "crates/pcre3/src/regex.rs", owner = "Regex", name = "new" },
  { file = "crates/pcre3/src/serialize.rs", name = "encode_as" },
]
lazy = true
describe = "Regex::new"
[mints]
path = "verus/MINTS"
mint = "c_mint"
token_trait = "CToken"
home = "crates/pcre3-capi/src/object.rs"
"#;

    /// A configuration that keeps every gate, or widens or renames one,
    /// is not narrowed. Negative: removing each gate section, a `[reach]`
    /// key, an `[inputs]` crate or root, or `lazy`, renaming a `[reach]`
    /// key's macro or conversion, removing or changing `[crates] dir`,
    /// dropping a member or turning its `ships` off, and adding a
    /// `test_cfgs` or `verus_features` name, each names what it turned
    /// off.
    #[test]
    fn a_gate_removed_or_narrowed_since_the_base_fails() {
        let base = Config::parse(FULL).expect("parses");
        let edit = |from: &str, to: &str| {
            assert!(FULL.contains(from), "{from:?} is not in the fixture");
            Config::parse(&FULL.replacen(from, to, 1)).expect("parses")
        };
        for (from, to) in [
            ("", ""),
            ("\"pcre3-unicode\"]", "\"pcre3-unicode\", \"pcre3-capi\"]"),
            ("verus/theorems/RN1.md", "verus/theorems/RN2.md"),
            ("mint = \"c_mint\"", "mint = \"c_make\""),
            ("path = \"verus/REACH\"", "path = \"verus/reach/REACH\""),
            (
                "[recursion]",
                "[floor]\npath = \"verus/SHARES\"\n[rlimit]\npath = \"verus/HEADROOM\"\n[recursion]",
            ),
            (
                "ships = true }]",
                "ships = true }, { name = \"more\", path = \"more\" }]",
            ),
            ("\"test\", \"kani\"]", "\"test\"]"),
            ("verus_features = [\"verus\"]\n", ""),
        ] {
            let got = edit(from, to).narrowed_from(&base);
            assert!(got.is_empty(), "{to:?}: got = {got:?}, wanted = none");
        }
        let section = |name: &str| {
            let start = FULL.find(&format!("[{name}]")).expect("section");
            let end = FULL[start + 1..]
                .find("\n[")
                .map_or(FULL.len(), |e| start + 1 + e + 1);
            FULL[start..end].to_owned()
        };
        // No base gate: nothing to narrow.
        let bare = Config::parse(&section("crates")).expect("parses");
        assert!(base.narrowed_from(&bare).is_empty());
        for name in ["recursion", "reach", "inputs", "mints"] {
            let got = edit(&section(name), "").narrowed_from(&base);
            let want = format!("the [{name}] section is gone, so its gate no longer runs");
            assert_eq!(
                got,
                std::slice::from_ref(&want),
                "[{name}]: got = {got:?}, wanted = {want:?}"
            );
        }
        for (from, to, want) in [
            (
                "exports_macro = \"exports\"\n",
                "",
                "[reach] exports_macro is gone",
            ),
            (
                "fn_ptr_macro = \"fn_ptr\"\n",
                "",
                "[reach] fn_ptr_macro is gone",
            ),
            (
                "conversion = \"from_c\"\n",
                "",
                "[reach] conversion is gone",
            ),
            (
                "exports_macro = \"exports\"",
                "exports_macro = \"exportz\"",
                "[reach] exports_macro changes from \"exports\" to \"exportz\"",
            ),
            (
                "fn_ptr_macro = \"fn_ptr\"",
                "fn_ptr_macro = \"fn_ptx\"",
                "[reach] fn_ptr_macro changes from \"fn_ptr\" to \"fn_ptx\"",
            ),
            (
                "conversion = \"from_c\"",
                "conversion = \"from_x\"",
                "[reach] conversion changes from \"from_c\" to \"from_x\"",
            ),
            (
                ", \"pcre3-unicode\"",
                "",
                "[inputs] crates drops \"pcre3-unicode\"",
            ),
            (
                "  { file = \"crates/pcre3/src/serialize.rs\", name = \"encode_as\" },\n",
                "",
                "[inputs] roots drops encode_as in crates/pcre3/src/serialize.rs",
            ),
            (
                "owner = \"Regex\", ",
                "",
                "[inputs] roots drops Regex::new in crates/pcre3/src/regex.rs",
            ),
            ("lazy = true\n", "", "[inputs] lazy is turned off"),
            (
                "dir = \"crates\"\n",
                "",
                "[crates] dir is no longer \"crates\"",
            ),
            (
                "dir = \"crates\"",
                "dir = \"vendor\"",
                "[crates] dir is no longer \"crates\"",
            ),
            (
                "{ name = \"extra\", path = \"extra\", ships = true }",
                "",
                "[crates] members drops \"extra\"",
            ),
            (
                "ships = true",
                "ships = false",
                "[crates] member \"extra\" changes ships from true to false",
            ),
            (
                ", ships = true",
                "",
                "[crates] member \"extra\" changes ships from true to unset",
            ),
            (
                "\"kani\"]",
                "\"kani\", \"unix\"]",
                "[crates] test_cfgs adds \"unix\"",
            ),
            (
                "[\"verus\"]",
                "[\"verus\", \"fast\"]",
                "[crates] verus_features adds \"fast\"",
            ),
        ] {
            let got = edit(from, to).narrowed_from(&base);
            assert!(
                got.len() == 1 && got[0].starts_with(want),
                "{from:?} to {to:?}: got = {got:?}, wanted = [{want}..]"
            );
        }
    }

    #[test]
    fn stand_ins_keep_length() {
        let pairs = [["assert::".to_string(), "assrtx::".to_string()]];
        let got = stand_in(&pairs, "use assert::x; assert::y();");
        assert_eq!(got, "use assrtx::x; assrtx::y();");
    }
}
