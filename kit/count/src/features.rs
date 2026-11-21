//! Which builds of a workspace may compile a package with one of its
//! test-only features (`[crates] test_features`), read from the
//! workspace's manifests and its `Cargo.lock` alone, with no cargo (#136).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// A dependency a package's manifest declares, with its workspace entry
/// (`workspace = true`) merged in.
#[derive(Debug)]
struct Dep {
    /// The name the package's features use for it (the table key).
    key: String,
    /// The package it names (`package = ".."`, or else the key).
    package: String,
    /// Its directory, for a path dependency.
    path: Option<PathBuf>,
    /// A dev-dependency: only tests build it (under resolver 2 or later).
    dev: bool,
    /// An optional dependency, which gives the package an implicit
    /// feature of its key unless a feature names it as `dep:key`.
    optional: bool,
    /// The features it turns on. Its `default-features` is not read: every
    /// compiled package's `default` is taken as on.
    features: Vec<String>,
}

/// Dependencies, each as (dependent, dependency).
type Edges = Vec<(String, String)>;

/// A package whose manifest the workspace reaches.
#[derive(Debug)]
struct Package {
    name: String,
    manifest: PathBuf,
    /// A member of the workspace (the root manifest's own package, one
    /// `members` names, or a path dependency inside the workspace's
    /// directory), rather than a path dependency or `[patch]` alone.
    member: bool,
    features: BTreeMap<String, Vec<String>>,
    deps: Vec<Dep>,
}

impl Package {
    /// Whether the package has feature `f`: one `[features]` declares, or
    /// the implicit one of an optional dependency no `dep:` entry names.
    fn has(&self, f: &str) -> bool {
        self.features.contains_key(f)
            || (self.deps.iter().any(|d| d.optional && d.key == f)
                && !self
                    .features
                    .values()
                    .flatten()
                    .any(|e| e.strip_prefix("dep:") == Some(f)))
    }

    /// Every feature the package has ([`Package::has`]).
    fn all_features(&self) -> BTreeSet<&str> {
        self.features
            .keys()
            .map(String::as_str)
            .chain(
                self.deps
                    .iter()
                    .filter(|d| d.optional)
                    .map(|d| d.key.as_str()),
            )
            .filter(|f| self.has(f))
            .collect()
    }
}

/// The packages a build of the workspace at `root` may compile with one of
/// the test-only `test` features, each with the feature: those cargo could
/// resolve so in some build, read from the manifests and `Cargo.lock` with
/// no cargo, so that the check runs where cargo is not installed or cannot
/// resolve offline (#136).
///
/// A build compiles the workspace's members and what their normal and
/// build dependencies (on every target) reach; under resolver 1 (cargo's
/// for a virtual manifest with no `resolver`, or a root package before
/// edition 2021), which unifies a member's dev-dependencies' features
/// into its normal build, those too. Every member's every feature but the
/// test-only ones is taken as on, as a build such as `--features
/// pcre2-capi/hardened` may turn any of them on (an optional
/// dependency's implicit feature among them), with each compiled
/// package's `default` and the features each such dependency asks for.
/// A package with a source (a registry's or git's) that `Cargo.lock` has
/// depend on a local one (as a `[patch]` may make it) may ask for any
/// feature of it: each but the test-only ones is on, and one with a
/// test-only feature fails the run. From those, a feature's `name`,
/// `name/feature` and `name?/feature` entries are followed; `dep:name`
/// turns on no feature. Any test-only feature so reached is an enabler.
///
/// It fails closed on a form it does not read (a member glob other than a
/// trailing `/*`, a malformed entry, a `workspace = true` with no
/// workspace entry, a feature entry naming no dependency, an unknown
/// resolver), on a package with no source in `Cargo.lock` whose manifest
/// it does not find, on two manifests of one such name, and on a compiled
/// package `Cargo.lock` does not have (a stale lockfile).
pub(crate) fn test_util_enablers(
    root: &Path,
    test: &[String],
) -> Result<Vec<(String, String)>, String> {
    let lock = read_toml(&root.join("Cargo.lock"))?;
    let (local, sourced) = lock_packages(&lock)?;
    let (packages, resolver1) = read_packages(root)?;
    let mut by_name: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, p) in packages
        .iter()
        .enumerate()
        .filter(|(_, p)| local.contains(&p.name))
    {
        if let Some(other) = by_name.insert(&p.name, i) {
            return Err(format!(
                "two manifests name the package {}: {} and {}",
                p.name,
                packages[other].manifest.display(),
                p.manifest.display()
            ));
        }
    }
    if let Some(missing) = local.iter().find(|n| !by_name.contains_key(n.as_str())) {
        return Err(format!(
            "Cargo.lock in {} has the package {missing} with no source, but no manifest the workspace's members or their path dependencies reach names it",
            root.display()
        ));
    }
    let by_dir: BTreeMap<PathBuf, usize> = packages
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.manifest.parent().map(|d| (d.to_path_buf(), i)))
        .collect();
    // The package a dependency names: a path dependency's by its
    // directory, any other by its name where `Cargo.lock` has a local one
    // (a `[patch]` may make it so).
    let target = |d: &Dep| -> Option<usize> {
        match &d.path {
            Some(p) => by_dir.get(&normal(p)).copied(),
            None => by_name.get(d.package.as_str()).copied(),
        }
    };
    // Whether a build of `p` compiles dependency `d`.
    let built = |p: &Package, d: &Dep| !d.dev || (resolver1 && p.member);
    // Packages any of whose features a build may turn on: the members, and
    // the local packages a package with a source depends on.
    let mut open: BTreeSet<usize> = (0..packages.len())
        .filter(|&i| packages[i].member)
        .collect();
    for (from, to) in &sourced {
        let i = by_name[to.as_str()];
        if let Some(f) = packages[i]
            .all_features()
            .into_iter()
            .find(|f| test.iter().any(|t| t == f))
        {
            return Err(format!(
                "Cargo.lock in {} has {from}, a package with a source, depend on {to}, which has the test-only feature `{f}`; what {from} asks of it cannot be read from the manifests",
                root.display()
            ));
        }
        open.insert(i);
    }
    // The packages a build compiles.
    let mut active: BTreeSet<usize> = BTreeSet::new();
    let mut reach: Vec<usize> = open.iter().copied().collect();
    while let Some(i) = reach.pop() {
        if !active.insert(i) {
            continue;
        }
        let p = &packages[i];
        if !local.contains(&p.name) {
            return Err(format!(
                "{}: a build compiles the package {}, but Cargo.lock in {} does not have it; regenerate the lockfile",
                p.manifest.display(),
                p.name,
                root.display()
            ));
        }
        reach.extend(p.deps.iter().filter(|d| built(p, d)).filter_map(&target));
    }
    // Every (package, feature) a build may turn on, from the roots.
    let mut on: BTreeSet<(usize, String)> = BTreeSet::new();
    let mut todo: Vec<(usize, String)> = Vec::new();
    for &i in &active {
        let p = &packages[i];
        if open.contains(&i) {
            todo.extend(
                p.all_features()
                    .into_iter()
                    .filter(|f| !test.iter().any(|t| t == f))
                    .map(|f| (i, f.to_owned())),
            );
        }
        todo.push((i, "default".to_owned()));
        for d in p.deps.iter().filter(|d| built(p, d)) {
            if let Some(q) = target(d) {
                todo.extend(d.features.iter().map(|f| (q, f.clone())));
            }
        }
    }
    while let Some((i, feature)) = todo.pop() {
        if !on.insert((i, feature.clone())) {
            continue;
        }
        let p = &packages[i];
        for entry in p.features.get(&feature).into_iter().flatten() {
            if entry.starts_with("dep:") {
                continue;
            }
            let Some((key, f)) = entry.split_once('/') else {
                todo.push((i, entry.clone()));
                continue;
            };
            let key = key.strip_suffix('?').unwrap_or(key);
            let named: Vec<&Dep> = p.deps.iter().filter(|d| d.key == key).collect();
            if named.is_empty() {
                return Err(format!(
                    "{}: feature `{feature}` names `{entry}`, but {} has no dependency `{key}`",
                    p.manifest.display(),
                    p.name
                ));
            }
            for d in named.into_iter().filter(|d| built(p, d)) {
                if let Some(q) = target(d) {
                    todo.push((q, f.to_owned()));
                }
            }
        }
    }
    let mut enablers: Vec<(String, String)> = on
        .into_iter()
        .filter(|(i, f)| test.contains(f) && packages[*i].has(f))
        .map(|(i, f)| (packages[i].name.clone(), f))
        .collect();
    enablers.sort();
    enablers.dedup();
    Ok(enablers)
}

/// A TOML file, parsed.
fn read_toml(path: &Path) -> Result<toml::Table, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("cannot parse {}: {e}", path.display()))
}

/// `Cargo.lock`'s packages with no source (the workspace's own), by name,
/// and each dependency of a package with a source on a name such a
/// package has, as (dependent, dependency).
fn lock_packages(lock: &toml::Table) -> Result<(BTreeSet<String>, Edges), String> {
    let packages = lock
        .get("package")
        .map(|p| p.as_array().ok_or("Cargo.lock's `package` is not an array"))
        .transpose()?
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut local = BTreeSet::new();
    for p in packages {
        let name = p
            .get("name")
            .and_then(toml::Value::as_str)
            .ok_or("a Cargo.lock package has no name")?;
        if p.get("source").is_none() {
            local.insert(name.to_owned());
        }
    }
    let mut sourced = Vec::new();
    for p in packages.iter().filter(|p| p.get("source").is_some()) {
        let name = p
            .get("name")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        for d in p
            .get("dependencies")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            let d = d
                .as_str()
                .ok_or("a Cargo.lock dependency is not a string")?;
            // `name`, `name version` or `name version (source)`: one with a
            // source names a package with that source.
            let mut words = d.split(' ');
            let dep = words.next().unwrap_or_default();
            if local.contains(dep) && !d.contains(" (") {
                sourced.push((name.to_owned(), dep.to_owned()));
            }
        }
    }
    Ok((local, sourced))
}

/// Whether the workspace at `root` resolves features with cargo's
/// resolver 1, as cargo picks it: `[workspace] resolver`, else the root
/// package's `resolver`, else its edition (2021 on picks 2 or later), and
/// 1 for a virtual manifest with neither.
fn resolver1(top: &toml::Table, top_path: &Path) -> Result<bool, String> {
    let package = top.get("package");
    let named = top
        .get("workspace")
        .and_then(|w| w.get("resolver"))
        .or_else(|| package.and_then(|p| p.get("resolver")));
    if let Some(r) = named {
        return match r.as_str() {
            Some("1") => Ok(true),
            Some("2" | "3") => Ok(false),
            _ => Err(format!(
                "{}: resolver {r} is not one verus-count reads (\"1\", \"2\" or \"3\")",
                top_path.display()
            )),
        };
    }
    let Some(package) = package else {
        return Ok(true);
    };
    match package.get("edition") {
        None => Ok(true),
        Some(e) => match e.as_str() {
            Some("2015" | "2018") => Ok(true),
            Some("2021" | "2024") => Ok(false),
            // `edition.workspace = true`, or an edition this reader does
            // not know.
            _ => Err(format!(
                "{}: the root package's edition {e} does not settle the resolver; set `resolver` in [workspace]",
                top_path.display()
            )),
        },
    }
}

/// Every package whose manifest the workspace at `root` reaches: the root
/// manifest's own package, its members' (with `exclude`s left out), and
/// those of the path dependencies and `[patch]` paths they name, in turn;
/// and whether the workspace resolves with resolver 1 ([`resolver1`]).
fn read_packages(root: &Path) -> Result<(Vec<Package>, bool), String> {
    let top_path = root.join("Cargo.toml");
    let top = read_toml(&top_path)?;
    let resolver1 = resolver1(&top, &top_path)?;
    let ws = top.get("workspace").and_then(toml::Value::as_table);
    let ws_deps = ws
        .and_then(|w| w.get("dependencies"))
        .map(|d| {
            d.as_table()
                .ok_or("[workspace.dependencies] is not a table")
        })
        .transpose()?;
    // Each directory to read, with whether its package is a member.
    let mut dirs = vec![(root.to_path_buf(), true)];
    let excluded = match ws {
        Some(ws) => strings(ws.get("exclude"), &top_path, "workspace.exclude")?,
        None => Vec::new(),
    };
    let excluded = |d: &Path| {
        excluded
            .iter()
            .any(|x| normal(d).starts_with(normal(&root.join(x))))
    };
    if let Some(ws) = ws {
        for m in strings(ws.get("members"), &top_path, "workspace.members")? {
            dirs.extend(
                member_dirs(root, &m)?
                    .into_iter()
                    .filter(|d| !excluded(d))
                    .map(|d| (d, true)),
            );
        }
    }
    for patches in top
        .get("patch")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(|t| t.values())
    {
        for p in patches.as_table().into_iter().flat_map(|t| t.values()) {
            if let Some(path) = p.get("path").and_then(toml::Value::as_str) {
                dirs.push((root.join(path), false));
            }
        }
    }
    // A directory `members` names is a member's, however else it is
    // reached.
    let members: BTreeSet<PathBuf> = dirs
        .iter()
        .filter(|(_, m)| *m)
        .map(|(d, _)| normal(d))
        .collect();
    // Each directory read, with its package's index in `packages`.
    let mut seen: BTreeMap<PathBuf, Option<usize>> = BTreeMap::new();
    let mut packages: Vec<Package> = Vec::new();
    while let Some((dir, member)) = dirs.pop() {
        let dir = normal(&dir);
        let member = member || members.contains(&dir);
        if let Some(read) = seen.get(&dir) {
            // Reached again, now as a member's path dependency.
            if let Some(i) = *read {
                packages[i].member |= member;
            }
            continue;
        }
        seen.insert(dir.clone(), None);
        let manifest = dir.join("Cargo.toml");
        let table = if dir == normal(root) {
            top.clone()
        } else {
            read_toml(&manifest)?
        };
        let Some(package) = table.get("package") else {
            continue;
        };
        let name = package
            .get("name")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("{}: [package] has no name", manifest.display()))?
            .to_owned();
        let features = match table.get("features") {
            None => BTreeMap::new(),
            Some(f) => f
                .as_table()
                .ok_or_else(|| format!("{}: [features] is not a table", manifest.display()))?
                .iter()
                .map(|(k, v)| {
                    Ok((
                        k.clone(),
                        strings(Some(v), &manifest, &format!("features.{k}"))?,
                    ))
                })
                .collect::<Result<_, String>>()?,
        };
        let deps = deps_of(&table, &dir, &manifest, root, ws_deps)?;
        // A path dependency in the workspace's directory, and not excluded,
        // is a member too, as cargo makes it one.
        dirs.extend(deps.iter().filter_map(|d| d.path.clone()).map(|p| {
            let in_ws = normal(&p).starts_with(normal(root)) && !excluded(&p) && ws.is_some();
            (p, in_ws)
        }));
        seen.insert(dir, Some(packages.len()));
        packages.push(Package {
            name,
            manifest,
            member,
            features,
            deps,
        });
    }
    Ok((packages, resolver1))
}

/// The directories a `members` entry names: a path, or one whose last
/// segment alone is `*` (every directory there with a Cargo.toml).
fn member_dirs(root: &Path, member: &str) -> Result<Vec<PathBuf>, String> {
    let glob = |s: &str| s.contains(['*', '?', '[', ']', '{', '}']);
    match member.rsplit_once('/') {
        Some((parent, "*")) if !glob(parent) => {
            let dir = root.join(parent);
            let mut dirs: Vec<PathBuf> = fs::read_dir(&dir)
                .map_err(|e| format!("cannot read {}: {e}", dir.display()))?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.join("Cargo.toml").is_file())
                .collect();
            dirs.sort();
            Ok(dirs)
        }
        _ if glob(member) => Err(format!(
            "{}: workspace member `{member}` is a glob other than a trailing `/*`, which verus-count does not read",
            root.join("Cargo.toml").display()
        )),
        _ => Ok(vec![root.join(member)]),
    }
}

/// The dependencies a manifest declares, of every kind and on every
/// target.
fn deps_of(
    table: &toml::Table,
    dir: &Path,
    manifest: &Path,
    root: &Path,
    ws_deps: Option<&toml::Table>,
) -> Result<Vec<Dep>, String> {
    const KINDS: [(&str, bool); 5] = [
        ("dependencies", false),
        ("build-dependencies", false),
        ("build_dependencies", false),
        ("dev-dependencies", true),
        ("dev_dependencies", true),
    ];
    let mut sections: Vec<(&toml::Value, bool)> = KINDS
        .iter()
        .filter_map(|(k, dev)| table.get(*k).map(|v| (v, *dev)))
        .collect();
    if let Some(targets) = table.get("target") {
        let targets = targets
            .as_table()
            .ok_or_else(|| format!("{}: [target] is not a table", manifest.display()))?;
        for t in targets.values() {
            sections.extend(
                KINDS
                    .iter()
                    .filter_map(|(k, dev)| t.get(*k).map(|v| (v, *dev))),
            );
        }
    }
    let mut deps = Vec::new();
    for (section, dev) in sections {
        let section = section.as_table().ok_or_else(|| {
            format!(
                "{}: a dependency section is not a table",
                manifest.display()
            )
        })?;
        for (key, v) in section {
            let what = format!("dependency `{key}`");
            let mut dep = Dep {
                key: key.clone(),
                package: key.clone(),
                path: None,
                dev,
                optional: false,
                features: Vec::new(),
            };
            match v {
                toml::Value::String(_) => {}
                toml::Value::Table(t) => {
                    if t.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
                        let w = ws_deps.and_then(|w| w.get(key)).ok_or_else(|| {
                            format!(
                                "{}: {what} has `workspace = true`, but the workspace has no such dependency",
                                manifest.display()
                            )
                        })?;
                        dep.read(w, root, manifest)?;
                    }
                    dep.read(v, dir, manifest)?;
                }
                _ => {
                    return Err(format!(
                        "{}: {what} is neither a version nor a table",
                        manifest.display()
                    ));
                }
            }
            deps.push(dep);
        }
    }
    Ok(deps)
}

impl Dep {
    /// Merge in a dependency entry `v` (a member's, or its workspace's),
    /// whose `path` is relative to `dir`.
    fn read(&mut self, v: &toml::Value, dir: &Path, manifest: &Path) -> Result<(), String> {
        let Some(t) = v.as_table() else {
            return match v {
                toml::Value::String(_) => Ok(()),
                _ => Err(format!(
                    "{}: dependency `{}` is neither a version nor a table",
                    manifest.display(),
                    self.key
                )),
            };
        };
        let what = |field: &str| format!("dependency `{}`'s `{field}`", self.key);
        if let Some(p) = t.get("package") {
            p.as_str()
                .ok_or_else(|| {
                    format!(
                        "{}: {} is not a string",
                        manifest.display(),
                        what("package")
                    )
                })?
                .clone_into(&mut self.package);
        }
        if let Some(p) = t.get("path") {
            let p = p.as_str().ok_or_else(|| {
                format!("{}: {} is not a string", manifest.display(), what("path"))
            })?;
            self.path = Some(dir.join(p));
        }
        if let Some(o) = t.get("optional") {
            self.optional |= o.as_bool().ok_or_else(|| {
                format!(
                    "{}: {} is not a boolean",
                    manifest.display(),
                    what("optional")
                )
            })?;
        }
        self.features
            .extend(strings(t.get("features"), manifest, &what("features"))?);
        Ok(())
    }
}

/// A list of strings, or none.
fn strings(v: Option<&toml::Value>, manifest: &Path, what: &str) -> Result<Vec<String>, String> {
    let Some(v) = v else {
        return Ok(Vec::new());
    };
    v.as_array()
        .and_then(|a| a.iter().map(|s| s.as_str().map(str::to_owned)).collect())
        .ok_or_else(|| format!("{}: {what} is not a list of strings", manifest.display()))
}

/// `path` with its `.` and `..` segments folded, as written (no symlink is
/// followed).
fn normal(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir
                if matches!(out.components().next_back(), Some(Component::Normal(_))) =>
            {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}
