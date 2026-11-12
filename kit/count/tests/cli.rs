//! The counter run as a port runs it, on a scratch repository: the
//! configuration check `--base-rev` makes (kit/count/README.md,
//! "Configuration").

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PLAIN: &str = "[crates]\ndir = \"crates\"\n";

const GATED: &str = "[crates]\ndir = \"crates\"\n[reach]\npath = \"verus/REACH\"\n[mints]\npath = \"verus/MINTS\"\nmint = \"c_mint\"\ntoken_trait = \"CToken\"\nhome = \"crates/a/src/lib.rs\"\n";

/// One crate of one function, committed four times under a tag each:
/// `none` without a configuration, `bad` with one that does not parse,
/// `gated` with `[reach]` and `[mints]`, and `plain` with neither.
fn repository(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("verus-count-cli-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("crates/a/src")).expect("mkdir");
    fs::create_dir_all(dir.join("verus")).expect("mkdir");
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n",
    )
    .expect("write");
    fs::write(dir.join("crates/a/src/lib.rs"), "pub fn f() {}\n").expect("write");
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "crates"]);
    git(&dir, &["commit", "-q", "-m", "none"]);
    git(&dir, &["tag", "none"]);
    for (tag, text) in [("bad", "[crates"), ("gated", GATED), ("plain", PLAIN)] {
        fs::write(dir.join("verus/count.toml"), text).expect("write");
        git(&dir, &["add", "verus/count.toml"]);
        git(&dir, &["commit", "-q", "-m", tag]);
        git(&dir, &["tag", tag]);
    }
    dir
}

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
        ])
        .args(args)
        .status()
        .is_ok_and(|s| s.success());
    assert!(ok, "git {args:?}: got = failure, wanted = success");
}

/// The counter's exit code, standard output and standard error on `root`.
fn count(root: &Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_verus-count"))
        .args(args)
        .arg(root)
        .output()
        .expect("runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A head that drops `[mints]` (and `[reach]`) from the base's
/// configuration exits 1 with `--base-rev`, and 0 without it. A base
/// with no configuration, or the same one, passes; a base whose copy
/// does not parse exits 2.
#[test]
fn a_gate_dropped_since_the_base_fails_the_run() {
    let dir = repository("drop");
    for (args, code, want) in [
        (
            &["--base-rev", "gated"][..],
            1,
            "FAIL configuration: verus/count.toml: the [mints] section is gone",
        ),
        (&[][..], 0, ""),
        (
            &["--base-rev", "none"][..],
            0,
            "ok configuration: none has no verus/count.toml to compare with",
        ),
        (
            &["--base-rev", "plain"][..],
            0,
            "ok configuration: verus/count.toml keeps every gate plain:verus/count.toml runs",
        ),
        (
            &["--base-rev", "bad"][..],
            2,
            "FAIL configuration: bad:verus/count.toml:",
        ),
    ] {
        let (got, out, err) = count(&dir, args);
        assert!(
            got == code && (out.contains(want) || err.contains(want)),
            "{args:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit {code}, {want:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// A narrowing named by a `[[narrowed]]` line with a reason passes, and the
/// run prints the reason. Negative: a line that names no narrowing of the
/// base fails, and so does a narrowing left unnamed beside a named one.
#[test]
fn a_named_narrowing_passes_and_prints_its_reason() {
    let dir = repository("named");
    let line = |change: &str| {
        format!("[[narrowed]]\nchange = {change:?}\nreason = \"no C entry points yet\"\n")
    };
    let reach = "the [reach] section is gone";
    let mints = "the [mints] section is gone";
    for (text, base, code, want) in [
        (
            format!("{PLAIN}{}{}", line(reach), line(mints)),
            "gated",
            0,
            "ok configuration: verus/count.toml: the [mints] section is gone, so its gate no longer runs, with a reason: no C entry points yet",
        ),
        (
            format!("{PLAIN}{}", line(reach)),
            "gated",
            1,
            "FAIL configuration: verus/count.toml: the [mints] section is gone, so its gate no longer runs, where gated:verus/count.toml runs it: restore it, or give it a new [[narrowed]] line with change = \"the [mints] section is gone\" and a reason",
        ),
        (
            format!("{PLAIN}{}", line(mints)),
            "plain",
            1,
            "FAIL configuration: verus/count.toml: the [[narrowed]] line for \"the [mints] section is gone\" names no change from plain:verus/count.toml",
        ),
    ] {
        fs::write(dir.join("verus/count.toml"), &text).expect("write");
        let (got, out, err) = count(&dir, &["--base-rev", base]);
        assert!(
            got == code && (out.contains(want) || err.contains(want)),
            "{text:?} against {base}: got = exit {got}, {out:?}, {err:?}; wanted = exit {code}, {want:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// `--floor FILE` must be ROOT's `[floor] path`, and with `--base-rev` the
/// base's FLOOR is read at the base configuration's `[floor] path`: a head
/// that points `path` and `--floor` at a new file with a lowered figure
/// fails for want of a `lowered` line. Negative: a `--floor` that is not
/// `[floor] path` (the table deleted, or a typo) exits 2, and so does a
/// `[floor] path` that is a symlink, even to the file `--floor` names.
#[test]
fn the_floor_file_is_the_configured_one_and_held_to_the_base() {
    let dir = std::env::temp_dir().join(format!("verus-count-cli-floor-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("crates/a/src")).expect("mkdir");
    fs::create_dir_all(dir.join("verus")).expect("mkdir");
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n",
    )
    .expect("write");
    // One verified function and one outside `verus!`: every share is 50%.
    fs::write(
        dir.join("crates/a/src/lib.rs"),
        "verus! {\npub fn v() {}\n}\npub fn o() {}\n",
    )
    .expect("write");
    let floor = |path: &str| format!("{PLAIN}[floor]\npath = {path:?}\n");
    let figures = |functions: &str| {
        format!("functions {functions}\nlines 50.0\nchecked-functions 50.0\nchecked-lines 50.0\n")
    };
    fs::write(dir.join("verus/count.toml"), floor("verus/FLOOR")).expect("write");
    fs::write(dir.join("verus/FLOOR"), figures("50.0")).expect("write");
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "crates", "verus"]);
    git(&dir, &["commit", "-q", "-m", "floor"]);
    fs::write(dir.join("verus/FLOOR.old"), figures("40.0")).expect("write");
    std::os::unix::fs::symlink("FLOOR", dir.join("verus/LINK")).expect("ln");
    for (config, file, code, want) in [
        (
            floor("verus/FLOOR"),
            "verus/FLOOR",
            0,
            "ok coverage floor: functions verified 50.0%, at the floor",
        ),
        (
            floor("verus/FLOOR.old"),
            "verus/FLOOR.old",
            1,
            "FAIL coverage floor: verus/FLOOR.old lowers functions from 50.0% to 40.0% with no `lowered functions 40.0 <reason>` line",
        ),
        (
            PLAIN.to_owned(),
            "verus/FLOOR.old",
            2,
            "'s [floor] path verus/FLOOR: give `path` the file the run holds",
        ),
        (
            floor("verus/FLOR"),
            "verus/FLOOR",
            2,
            "'s [floor] path verus/FLOR: give `path` the file the run holds",
        ),
        (
            floor("verus/LINK"),
            "verus/LINK",
            2,
            "FAIL coverage floor: [floor] path verus/LINK passes through verus/LINK, a symlink",
        ),
    ] {
        fs::write(dir.join("verus/count.toml"), &config).expect("write");
        let (got, out, err) = count(
            &dir,
            &[
                "--floor",
                &dir.join(file).to_string_lossy(),
                "--base-rev",
                "HEAD",
            ],
        );
        assert!(
            got == code && (out.contains(want) || err.contains(want)),
            "{config:?} with --floor {file}: got = exit {got}, {out:?}, {err:?}; wanted = exit {code}, {want:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// A crate whose own Cargo.toml gains `publish = false` leaves the shipped
/// tally, so with `--base-rev` it is a narrowing: it fails without a
/// `[[narrowed]]` line and passes with one, for a `dir` crate and for a
/// member without `ships`. Negative: a crate losing or keeping `publish =
/// false`, and a member whose `ships = true` the configuration keeps,
/// pass; where the configuration's `ships` decides before or after, only
/// the configuration's change is reported.
#[test]
fn a_crate_that_gains_publish_false_is_a_narrowing() {
    let dir = repository("publish");
    let cargo = dir.join("crates/a/Cargo.toml");
    let manifest = "[package]\nname = \"a\"\nversion = \"0.0.0\"\n";
    let unpublished = format!("{manifest}publish = false\n");
    let member = |ships: &str| {
        format!("[crates]\nmembers = [{{ name = \"a\", path = \"crates/a\"{ships} }}]\n")
    };
    let commit = |config: &str, cargo_text: &str, tag: &str| {
        fs::write(dir.join("verus/count.toml"), config).expect("write");
        fs::write(&cargo, cargo_text).expect("write");
        git(&dir, &["add", "verus/count.toml", "crates"]);
        git(&dir, &["commit", "-q", "--allow-empty", "-m", tag]);
        git(&dir, &["tag", tag]);
    };
    commit(PLAIN, &unpublished, "unpub");
    commit(&member(""), manifest, "member");
    commit(&member(", ships = true"), manifest, "shipped");
    let dir_change = "[crates] crate \"a\" at \"crates/a\" gains publish = false";
    let line =
        format!("[[narrowed]]\nchange = {dir_change:?}\nreason = \"a is a test harness now\"\n");
    for (config, cargo_text, base, code, want) in [
        (
            PLAIN.to_owned(),
            &unpublished,
            "plain",
            1,
            format!(
                "FAIL configuration: verus/count.toml: {dir_change}, so it no longer counts as shipped"
            ),
        ),
        (
            format!("{PLAIN}{line}"),
            &unpublished,
            "plain",
            0,
            format!(
                "ok configuration: verus/count.toml: {dir_change}, so it no longer counts as shipped, with a reason: a is a test harness now"
            ),
        ),
        (
            member(""),
            &unpublished,
            "member",
            1,
            format!("FAIL configuration: verus/count.toml: {dir_change}"),
        ),
        (
            PLAIN.to_owned(),
            &manifest.to_owned(),
            "unpub",
            0,
            "ok configuration: verus/count.toml keeps every gate unpub:verus/count.toml runs"
                .to_owned(),
        ),
        (
            member(", ships = true"),
            &unpublished,
            "shipped",
            0,
            "ok configuration: verus/count.toml keeps every gate shipped:verus/count.toml runs"
                .to_owned(),
        ),
        (
            PLAIN.to_owned(),
            &unpublished,
            "unpub",
            0,
            "ok configuration: verus/count.toml keeps every gate unpub:verus/count.toml runs"
                .to_owned(),
        ),
        // Where the configuration's `ships` decides, before or after, the
        // configuration names the change, and the Cargo.toml adds none.
        (
            member(""),
            &unpublished,
            "shipped",
            1,
            "[crates] member \"a\" changes ships from true to unset".to_owned(),
        ),
        (
            member(", ships = false"),
            &unpublished,
            "member",
            1,
            "[crates] member \"a\" changes ships from unset to false".to_owned(),
        ),
    ] {
        fs::write(dir.join("verus/count.toml"), &config).expect("write");
        fs::write(&cargo, cargo_text).expect("write");
        let (got, out, err) = count(&dir, &["--base-rev", base]);
        let doubled = err.contains("gains publish") && !want.contains("gains publish");
        assert!(
            got == code && !doubled && (out.contains(&want) || err.contains(&want)),
            "{config:?} against {base}: got = exit {got}, {out:?}, {err:?}; wanted = exit {code}, {want:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// A scratch repository whose commit tagged `base` holds the files `base`
/// (path, text), with the working tree then holding only `head`.
fn changed(name: &str, base: &[(&str, &str)], head: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("verus-count-cli-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("mkdir");
    let write = |files: &[(&str, &str)]| {
        for (path, text) in files {
            let p = dir.join(path);
            fs::create_dir_all(p.parent().expect("a parent")).expect("mkdir");
            fs::write(p, text).expect("write");
        }
    };
    write(base);
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    git(&dir, &["tag", "base"]);
    for e in fs::read_dir(&dir).expect("ls").flatten() {
        if e.file_name() != ".git" {
            let p = e.path();
            if p.is_dir() {
                fs::remove_dir_all(p).expect("rm");
            } else {
                fs::remove_file(p).expect("rm");
            }
        }
    }
    write(head);
    dir
}

/// A crate is followed when it moves, and judged where the base read it:
/// a member moved that gains `publish = false` reports the gain beside the
/// move; a crate of `dir` that the head reads at no directory and under no
/// name is no longer read, as a member dropped is, so a renamed one is the
/// old crate dropped and a new crate; one the head reads as a member
/// elsewhere moves, and a `ships = false` or a `publish = false` there is
/// its own narrowing; a root crate (`path = "."`) is compared at the root's
/// Cargo.toml. Negative: a `dir` renamed reports only the `dir` change, and
/// a crate new under its name and directory nothing.
#[test]
fn a_crate_is_followed_when_it_moves() {
    const LIB: &str = "pub fn f() {}\n";
    const MANIFEST: &str = "[package]\nname = \"a\"\nversion = \"0.0.0\"\n";
    const UNPUB: &str = "[package]\nname = \"a\"\nversion = \"0.0.0\"\npublish = false\n";
    const CONFIG: &str = "verus/count.toml";
    let line = |change: &str| {
        format!("[[narrowed]]\nchange = {change:?}\nreason = \"directory renamed\"\n")
    };
    let move_line = line("[crates] member \"a\" moves from \"crates/a\" to \"crates/b\"");
    let rename_line = line("[crates] crate \"a\" at \"crates/a\" is no longer read");
    let member_a = "[crates]\nmembers = [{ name = \"a\", path = \"crates/a\" }]\n";
    let member_b =
        format!("[crates]\nmembers = [{{ name = \"a\", path = \"crates/b\" }}]\n{move_line}");
    let renamed = format!("{PLAIN}{rename_line}");
    let attic =
        |ships: &str| format!("{PLAIN}members = [{{ name = \"a\", path = \"attic/a\"{ships} }}]\n");
    let (attic_false, attic_unset) = (attic(", ships = false"), attic(""));
    let in_dir = [
        (CONFIG, PLAIN),
        ("crates/a/Cargo.toml", MANIFEST),
        ("crates/a/src/lib.rs", LIB),
        ("crates/z/src/lib.rs", LIB),
    ];
    let moved_to_attic = |config: &str, manifest: &str| -> Vec<(String, String)> {
        vec![
            (CONFIG.into(), config.into()),
            ("attic/a/Cargo.toml".into(), manifest.into()),
            ("attic/a/src/lib.rs".into(), LIB.into()),
            ("crates/z/src/lib.rs".into(), LIB.into()),
        ]
    };
    type Files = Vec<(String, String)>;
    /// A case: its name, the base's files and the head's, the exit code,
    /// and the text the run must print and must not.
    type Case<'a> = (&'a str, Files, Files, i32, Vec<&'a str>, Vec<&'a str>);
    let owned = |f: &[(&str, &str)]| -> Files {
        f.iter()
            .map(|(p, t)| (p.to_string(), t.to_string()))
            .collect()
    };
    let cases: Vec<Case> = vec![
        (
            "member-moved",
            owned(&[
                (CONFIG, member_a),
                ("crates/a/Cargo.toml", MANIFEST),
                ("crates/a/src/lib.rs", LIB),
            ]),
            owned(&[
                (CONFIG, &member_b),
                ("crates/b/Cargo.toml", UNPUB),
                ("crates/b/src/lib.rs", LIB),
            ]),
            1,
            vec![
                "with a reason: directory renamed",
                "FAIL configuration: verus/count.toml: [crates] crate \"a\" at \"crates/b\" gains publish = false, so it no longer counts as shipped",
            ],
            vec![],
        ),
        (
            "dir-renamed-crate",
            owned(&in_dir),
            owned(&[
                (CONFIG, PLAIN),
                ("crates/b/Cargo.toml", UNPUB),
                ("crates/b/src/lib.rs", LIB),
                ("crates/z/src/lib.rs", LIB),
            ]),
            1,
            vec![
                "FAIL configuration: verus/count.toml: [crates] crate \"a\" at \"crates/a\" is no longer read, so it is no longer read",
            ],
            vec!["gains publish", "moves"],
        ),
        (
            "dir-renamed-crate-named",
            owned(&in_dir),
            owned(&[
                (CONFIG, &renamed),
                ("crates/b/Cargo.toml", UNPUB),
                ("crates/b/src/lib.rs", LIB),
                ("crates/z/src/lib.rs", LIB),
            ]),
            0,
            vec![
                "[crates] crate \"a\" at \"crates/a\" is no longer read, so it is no longer read, with a reason: directory renamed",
            ],
            vec!["gains publish"],
        ),
        (
            "dir-crate-to-member-unshipped",
            owned(&in_dir),
            moved_to_attic(&attic_false, MANIFEST),
            1,
            vec![
                "[crates] crate \"a\" moves from \"crates/a\" to \"attic/a\", so the crate it read may no longer be read",
                "[crates] member \"a\" at \"attic/a\" changes ships from unset to false",
            ],
            vec!["gains publish"],
        ),
        (
            "dir-crate-to-member-unpublished",
            owned(&in_dir),
            moved_to_attic(&attic_unset, UNPUB),
            1,
            vec![
                "[crates] crate \"a\" moves from \"crates/a\" to \"attic/a\"",
                "[crates] crate \"a\" at \"attic/a\" gains publish = false",
            ],
            vec!["changes ships"],
        ),
        (
            "dir-crate-to-member-kept",
            owned(&in_dir),
            moved_to_attic(&attic_unset, MANIFEST),
            1,
            vec!["[crates] crate \"a\" moves from \"crates/a\" to \"attic/a\""],
            vec!["changes ships", "gains publish"],
        ),
        // Onto a base member's path, the configuration compares `ships`
        // with that member's, and the move adds no second line for it.
        (
            "dir-crate-onto-member",
            owned(&[
                (
                    CONFIG,
                    "[crates]\ndir = \"crates\"\nmembers = [{ name = \"m\", path = \"m\", ships = true }]\n",
                ),
                ("crates/a/src/lib.rs", LIB),
                ("crates/z/src/lib.rs", LIB),
                ("m/src/lib.rs", LIB),
            ]),
            owned(&[
                (
                    CONFIG,
                    "[crates]\ndir = \"crates\"\nmembers = [{ name = \"a\", path = \"m\", ships = false }]\n",
                ),
                ("crates/z/src/lib.rs", LIB),
                ("m/src/lib.rs", LIB),
            ]),
            1,
            vec![
                "[crates] crate \"a\" moves from \"crates/a\" to \"m\"",
                "[crates] member \"a\" at \"m\" changes ships from true to false",
            ],
            vec!["changes ships from unset"],
        ),
        // The `dir` line covers its crates: one moved with it, and `z`,
        // which it no longer holds.
        (
            "dir-renamed",
            owned(&in_dir),
            owned(&[
                (CONFIG, "[crates]\ndir = \"pkgs\"\n"),
                ("pkgs/a/Cargo.toml", MANIFEST),
                ("pkgs/a/src/lib.rs", LIB),
            ]),
            1,
            vec!["[crates] dir is no longer \"crates\""],
            vec!["moves", "no longer read,", "gains publish"],
        ),
        (
            "root-crate",
            owned(&[
                (
                    CONFIG,
                    "[crates]\nmembers = [{ name = \"top\", path = \".\" }]\n",
                ),
                ("Cargo.toml", MANIFEST),
                ("src/lib.rs", LIB),
            ]),
            owned(&[
                (
                    CONFIG,
                    "[crates]\nmembers = [{ name = \"top\", path = \".\" }]\n",
                ),
                ("Cargo.toml", UNPUB),
                ("src/lib.rs", LIB),
            ]),
            1,
            vec!["[crates] crate \"top\" at \".\" gains publish = false"],
            vec![],
        ),
    ];
    fn as_str(f: &[(String, String)]) -> Vec<(&str, &str)> {
        f.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect()
    }
    for (name, base, head, code, want, unwanted) in &cases {
        let dir = changed(name, &as_str(base), &as_str(head));
        let (got, out, err) = count(&dir, &["--base-rev", "base"]);
        let all = format!("{out}{err}");
        assert!(
            got == *code
                && want.iter().all(|w| all.contains(w))
                && !unwanted.iter().any(|u| all.contains(u)),
            "{name}: got = exit {got}, {out:?}, {err:?}; wanted = exit {code}, {want:?}, none of {unwanted:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

/// A `[reach]` whose REACH file is missing fails at the head, so no base
/// configuration can name a REACH file that no run read.
#[test]
fn a_reach_section_needs_its_file() {
    let dir = repository("reach");
    let text = format!("{PLAIN}[reach]\npath = \"verus/REACH\"\n");
    fs::write(dir.join("verus/count.toml"), &text).expect("write");
    let (got, out, err) = count(&dir, &[]);
    let want = "FAIL entry reach: verus/REACH:";
    assert!(
        got == 1 && err.contains(want),
        "no REACH: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
    );
    fs::write(dir.join("verus/REACH"), "").expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 0,
        "an empty REACH: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A head that turns verus/count.toml, or the directory above it, into a
/// symlink to a configuration without `[reach]` or `[mints]` exits 2,
/// rather than comparing a path the base does not have.
#[test]
fn a_symlinked_configuration_fails_the_run() {
    let dir = repository("link");
    fs::write(dir.join("verus/next.toml"), PLAIN).expect("write");
    fs::remove_file(dir.join("verus/count.toml")).expect("rm");
    std::os::unix::fs::symlink("next.toml", dir.join("verus/count.toml")).expect("ln");
    let (got, out, err) = count(&dir, &["--base-rev", "gated"]);
    let want = "FAIL configuration: verus/count.toml is a symlink";
    assert!(
        got == 2 && err.contains(want),
        "file: got = exit {got}, {out:?}, {err:?}; wanted = exit 2, {want:?}"
    );
    fs::rename(dir.join("verus"), dir.join("next")).expect("mv");
    fs::remove_file(dir.join("next/count.toml")).expect("rm");
    fs::write(dir.join("next/count.toml"), PLAIN).expect("write");
    std::os::unix::fs::symlink("next", dir.join("verus")).expect("ln");
    let (got, out, err) = count(&dir, &["--base-rev", "gated"]);
    let want = "FAIL configuration: verus is a symlink";
    assert!(
        got == 2 && err.contains(want),
        "directory: got = exit {got}, {out:?}, {err:?}; wanted = exit 2, {want:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// One verified function `a::v` and one outside `verus!`, so every share is
/// 50%: the sources, configuration, FLOOR and RLIMIT a ratchet case starts
/// from, and the base it is compared with.
const RATCHET_LIB: &str = "verus! {\npub fn v() {}\n}\npub fn o() {}\n";
const RATCHET_CONFIG: &str = "[crates]\ndir = \"crates\"\n[floor]\npath = \"verus/FLOOR\"\n[rlimit]\npath = \"verus/RLIMIT\"\n";
const RATCHET_FLOOR: &str =
    "functions 50.0\nlines 50.0\nchecked-functions 50.0\nchecked-lines 50.0\n";
const RATCHET_RLIMIT: &str = "max-share 65\n";

/// Verus's `--time-expanded --output-json` statistics for crate `a`, with
/// `a::v` using `used` of its rlimit count (30,000,000 at the default 10).
fn ratchet_stats(used: u64) -> String {
    format!(
        r#"{{"times-ms":{{"smt":{{"smt-run-module-times":[{{"module":"m","rlimit":0,"function-breakdown":[{{"function":"a::v","rlimit":{used}}}]}}]}}}},"verification-results":{{"success":true,"verified":1,"errors":0}}}}"#
    )
}

/// The ratchets a port's verus check runs, through the command line it
/// runs them with (`--floor verus/FLOOR --rlimit STATS --base-rev REV`):
/// a coverage figure lowered with and without a `lowered` line
/// (`Figures::lowering`), a function over its headroom (`rlimit::check`),
/// an RLIMIT file loosened (`rlimit::check_ratchet`), an rlimit attribute
/// raised with and without an `rlimit-raised` line (`rlimit::check_raised`),
/// and an `[rlimit] path` moved to a loosened file, which is still held to
/// the base's. `--list` adds its rows and every check still runs.
#[test]
fn the_ratchets_hold_through_the_command_line() {
    let dir = std::env::temp_dir().join(format!("verus-count-cli-ratchet-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("crates/a/src")).expect("mkdir");
    fs::create_dir_all(dir.join("verus")).expect("mkdir");
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n",
    )
    .expect("write");
    let lib = dir.join("crates/a/src/lib.rs");
    let base = |dir: &Path| {
        fs::write(&lib, RATCHET_LIB).expect("write");
        fs::write(dir.join("verus/count.toml"), RATCHET_CONFIG).expect("write");
        fs::write(dir.join("verus/FLOOR"), RATCHET_FLOOR).expect("write");
        fs::write(dir.join("verus/RLIMIT"), RATCHET_RLIMIT).expect("write");
        let _ = fs::remove_file(dir.join("verus/RLIMIT2"));
    };
    base(&dir);
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "crates", "verus"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    let stats = dir.join("stats.json");
    let raised = "verus! {\n#[verifier::rlimit(20)]\npub fn v() {}\n}\npub fn o() {}\n";
    let moved = RATCHET_CONFIG.replace("verus/RLIMIT", "verus/RLIMIT2");
    let row = "a\tVerified\t1\tcrates/a/src/lib.rs:2\tv";
    for (case, edits, used, list, code, wants) in [
        (
            "as committed",
            &[][..],
            1_000_000,
            false,
            0,
            &[
                "ok coverage floor: functions verified 50.0%, at the floor",
                "ok rlimit headroom: 1 functions, max-share 65.0%",
            ][..],
        ),
        (
            "a figure lowered with no lowered line",
            &[(
                "verus/FLOOR",
                RATCHET_FLOOR.replace("functions 50.0\nlines", "functions 40.0\nlines"),
            )][..],
            1_000_000,
            false,
            1,
            &[
                "FAIL coverage floor: verus/FLOOR lowers functions from 50.0% to 40.0% with no `lowered functions 40.0 <reason>` line",
            ][..],
        ),
        (
            "a figure lowered with a lowered line",
            &[(
                "verus/FLOOR",
                RATCHET_FLOOR.replace("functions 50.0\nlines", "functions 40.0\nlines")
                    + "lowered functions 40.0 o left verus! for a reason\n",
            )][..],
            1_000_000,
            false,
            0,
            &["ok coverage floor: functions lowered from 50.0% to 40.0%, with a reason"][..],
        ),
        (
            "a function over its headroom",
            &[][..],
            30_000_000,
            false,
            1,
            &[
                "FAIL rlimit headroom: a::v uses 100.0% of its rlimit (30000000 of 30000000), above 65.0% (verus/RLIMIT's max-share)",
            ][..],
        ),
        (
            "max-share raised",
            &[("verus/RLIMIT", "max-share 70\n".to_owned())][..],
            1_000_000,
            false,
            1,
            &["FAIL rlimit headroom: verus/RLIMIT raises max-share from 65.0% to 70.0%"][..],
        ),
        (
            "an allow line added",
            &[("verus/RLIMIT", "max-share 65\nallow a::v 90\n".to_owned())][..],
            24_000_000,
            false,
            1,
            &[
                "FAIL rlimit headroom: verus/RLIMIT allows a::v, which the base does not; restore its headroom instead",
            ][..],
        ),
        (
            "max-share lowered",
            &[("verus/RLIMIT", "max-share 60\n".to_owned())][..],
            1_000_000,
            false,
            0,
            &["ok rlimit headroom: 1 functions, max-share 60.0%"][..],
        ),
        (
            "an rlimit raised with no rlimit-raised line",
            &[("crates/a/src/lib.rs", raised.to_owned())][..],
            1_000_000,
            false,
            1,
            &[
                "FAIL rlimit headroom: crates/a/src/lib.rs:3 raises a::v's rlimit from 10 to 20 with no new `rlimit-raised a::v 20 <reason>` line in verus/RLIMIT",
            ][..],
        ),
        (
            "an rlimit raised with an rlimit-raised line",
            &[
                ("crates/a/src/lib.rs", raised.to_owned()),
                (
                    "verus/RLIMIT",
                    "max-share 65\nrlimit-raised a::v 20 it uses 40M on its own prover\n"
                        .to_owned(),
                ),
            ][..],
            1_000_000,
            false,
            0,
            &[
                "ok rlimit headroom: crates/a/src/lib.rs:3 raises a::v's rlimit from 10 to 20, with a reason",
            ][..],
        ),
        (
            "[rlimit] path moved to a file that raises max-share",
            &[
                ("verus/count.toml", moved.clone()),
                ("verus/RLIMIT2", "max-share 70\n".to_owned()),
            ][..],
            1_000_000,
            false,
            1,
            &["FAIL rlimit headroom: verus/RLIMIT2 raises max-share from 65.0% to 70.0%"][..],
        ),
        (
            "--list over a function over its headroom",
            &[][..],
            30_000_000,
            true,
            1,
            &[row, "FAIL rlimit headroom: a::v uses 100.0% of its rlimit"][..],
        ),
        (
            "--list over a figure lowered with no lowered line",
            &[(
                "verus/FLOOR",
                RATCHET_FLOOR.replace("functions 50.0\nlines", "functions 40.0\nlines"),
            )][..],
            1_000_000,
            true,
            1,
            &[
                row,
                "FAIL coverage floor: verus/FLOOR lowers functions from 50.0%",
            ][..],
        ),
        (
            "--list as committed",
            &[][..],
            1_000_000,
            true,
            0,
            &[
                row,
                "ok coverage floor: functions verified 50.0%",
                "ok rlimit headroom: 1 functions",
            ][..],
        ),
    ] {
        base(&dir);
        for (path, text) in edits {
            fs::write(dir.join(path), text).expect("write");
        }
        fs::write(&stats, ratchet_stats(used)).expect("write");
        let mut args = vec![
            "--floor".to_owned(),
            dir.join("verus/FLOOR").to_string_lossy().into_owned(),
            "--rlimit".to_owned(),
            stats.to_string_lossy().into_owned(),
            "--base-rev".to_owned(),
            "HEAD".to_owned(),
        ];
        if list {
            args.insert(0, "--list".to_owned());
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let (got, out, err) = count(&dir, &args);
        let missing: Vec<&&str> = wants
            .iter()
            .filter(|w| !out.contains(*w) && !err.contains(*w))
            .collect();
        assert!(
            got == code && missing.is_empty(),
            "{case}: got = exit {got}, {out:?}, {err:?}; wanted = exit {code}, with {missing:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}
