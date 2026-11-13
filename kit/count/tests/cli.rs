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

/// A run that holds a FLOOR fails without a base (exit 1, naming
/// `--no-base`), so leaving `--base-rev` off cannot skip the base's
/// comparisons; it passes with `--base-rev` and with `--no-base`, and
/// giving both is a usage error (exit 2).
#[test]
fn a_floor_without_a_base_fails_unless_no_base_says_so() {
    let dir = std::env::temp_dir().join(format!("verus-count-cli-no-base-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("crates/a/src")).expect("mkdir");
    fs::create_dir_all(dir.join("verus")).expect("mkdir");
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n",
    )
    .expect("write");
    fs::write(
        dir.join("crates/a/src/lib.rs"),
        "verus! {\npub fn v() {}\n}\npub fn o() {}\n",
    )
    .expect("write");
    fs::write(
        dir.join("verus/count.toml"),
        format!("{PLAIN}[floor]\npath = \"verus/FLOOR\"\n"),
    )
    .expect("write");
    fs::write(
        dir.join("verus/FLOOR"),
        "functions 50.0\nlines 50.0\nchecked-functions 50.0\nchecked-lines 50.0\n",
    )
    .expect("write");
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "crates", "verus"]);
    git(&dir, &["commit", "-q", "-m", "floor"]);
    let floor = dir.join("verus/FLOOR").to_string_lossy().into_owned();
    for (args, code, want) in [
        (
            &["--floor", &floor][..],
            1,
            "FAIL coverage floor: --floor with no --base-rev, so a lowered figure or a dropped gate would pass: give --base-rev REV, or --no-base",
        ),
        (
            &["--floor", &floor, "--no-base"][..],
            0,
            "ok coverage floor: functions verified 50.0%, at the floor",
        ),
        (
            &["--floor", &floor, "--base-rev", "HEAD"][..],
            0,
            "ok coverage floor: functions verified 50.0%, at the floor",
        ),
        (
            &["--floor", &floor, "--no-base", "--base-rev", "HEAD"][..],
            2,
            "--no-base with --base-rev: give one",
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

/// The badge FLOOR's 50.0 figures give, as `--badge` prints it.
const BADGE: &str = "[![Verus: 50.0% of functions, 50.0% of lines](https://img.shields.io/badge/Verus-50.0%25%20of%20functions%2C%2050.0%25%20of%20lines-blue)](verus/README.md#coverage)";

/// With `[badge]`, `--floor` holds the file's one Verus badge to FLOOR's
/// `functions` and `lines` figures, and `--badge` prints the line to
/// write. With `--base-rev`, `[badge]` removed or its `path` moved is a
/// narrowing, which passes with a `[[narrowed]]` line. Negative: a badge
/// that drifted from FLOOR, none, two, a missing file, and a `path`
/// through a symlink or with `..` each fail; without `--floor` nothing is
/// checked.
#[test]
fn the_badge_is_held_to_the_floor() {
    let dir = std::env::temp_dir().join(format!("verus-count-cli-badge-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("crates/a/src")).expect("mkdir");
    fs::create_dir_all(dir.join("verus")).expect("mkdir");
    fs::create_dir_all(dir.join("docs")).expect("mkdir");
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
    let badge = |path: &str| format!("{PLAIN}[floor]\n[badge]\npath = {path:?}\n");
    let readme = |line: &str| format!("# a\n{line}\n");
    let stale = BADGE.replace("50.0%25%20of%20functions", "40.0%25%20of%20functions");
    fs::write(dir.join("verus/count.toml"), badge("README.md")).expect("write");
    fs::write(
        dir.join("verus/FLOOR"),
        "functions 50.0\nlines 50.0\nchecked-functions 50.0\nchecked-lines 50.0\n",
    )
    .expect("write");
    fs::write(dir.join("README.md"), readme(BADGE)).expect("write");
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "crates", "verus", "README.md"]);
    git(&dir, &["commit", "-q", "-m", "badge"]);
    std::os::unix::fs::symlink("README.md", dir.join("LINK.md")).expect("ln");
    fs::write(dir.join("docs/README.md"), readme(BADGE)).expect("write");
    let floor = dir.join("verus/FLOOR").to_string_lossy().into_owned();
    let held = ["--floor", floor.as_str(), "--no-base"];
    let based = ["--floor", floor.as_str(), "--base-rev", "HEAD"];
    let line = |change: &str| {
        format!("[[narrowed]]\nchange = {change:?}\nreason = \"the badge moved to docs\"\n")
    };
    let gone = "the [badge] section is gone";
    let moved = "[badge] path changes from \"README.md\" to \"docs/README.md\"";
    for (config, text, args, code, want) in [
        (
            badge("README.md"),
            readme(BADGE),
            &based[..],
            0,
            "ok badge: README.md:2 gives verus/FLOOR's functions and lines figures".to_owned(),
        ),
        (
            badge("README.md"),
            readme(&stale),
            &based[..],
            1,
            format!(
                "FAIL badge: README.md:2 is not the badge verus/FLOOR's functions and lines figures give; replace it with this line:\n{BADGE}"
            ),
        ),
        (
            badge("README.md"),
            "# a\n".to_owned(),
            &held[..],
            1,
            format!(
                "FAIL badge: README.md has no Verus badge; add this line, which gives verus/FLOOR's functions and lines figures:\n{BADGE}"
            ),
        ),
        (
            badge("README.md"),
            readme(&format!("{BADGE}\n{BADGE}")),
            &held[..],
            1,
            "FAIL badge: README.md has 2 Verus badges (lines 2, 3)".to_owned(),
        ),
        (
            badge("MISSING.md"),
            readme(BADGE),
            &held[..],
            1,
            "FAIL badge: MISSING.md: ".to_owned(),
        ),
        (
            badge("LINK.md"),
            readme(BADGE),
            &held[..],
            2,
            "FAIL badge: [badge] path LINK.md passes through LINK.md, a symlink".to_owned(),
        ),
        (
            badge("docs/../README.md"),
            readme(BADGE),
            &held[..],
            2,
            "FAIL badge: [badge] path docs/../README.md is not a relative path without `..`"
                .to_owned(),
        ),
        // Without `--floor` there are no figures to hold the badge to.
        (
            badge("README.md"),
            readme(&stale),
            &[][..],
            0,
            String::new(),
        ),
        (
            format!("{PLAIN}[floor]\n"),
            readme(&stale),
            &based[..],
            1,
            format!("FAIL configuration: verus/count.toml: {gone}, so its gate no longer runs"),
        ),
        (
            format!("{PLAIN}[floor]\n{}", line(gone)),
            readme(&stale),
            &based[..],
            0,
            format!(
                "ok configuration: verus/count.toml: {gone}, so its gate no longer runs, with a reason: the badge moved to docs"
            ),
        ),
        (
            badge("docs/README.md"),
            readme(&stale),
            &based[..],
            1,
            format!(
                "FAIL configuration: verus/count.toml: {moved}, so the badge the base held to FLOOR is no longer held"
            ),
        ),
        (
            format!("{}{}", badge("docs/README.md"), line(moved)),
            readme(&stale),
            &based[..],
            0,
            "ok badge: docs/README.md:2 gives".to_owned(),
        ),
    ] {
        fs::write(dir.join("verus/count.toml"), &config).expect("write");
        fs::write(dir.join("README.md"), &text).expect("write");
        let (got, out, err) = count(&dir, args);
        assert!(
            got == code && (out.contains(&want) || err.contains(&want)),
            "{config:?} with {text:?} and {args:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit {code}, {want:?}"
        );
    }
    fs::write(dir.join("verus/count.toml"), badge("README.md")).expect("write");
    let (got, out, err) = count(&dir, &["--badge"]);
    assert!(
        got == 0 && out == format!("{BADGE}\n"),
        "--badge: got = exit {got}, {out:?}, {err:?}; wanted = exit 0, {BADGE:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// chainz's `export!`, `forward!` and `twins!` shapes (one definition each,
/// as crates/zlib-capi writes them), and macros of another crate, one over
/// three lines and one on one.
const MACROS: &str = r#"verus! {
pub fn v() {}
}

macro_rules! export {
    ($(fn $name:ident / $zname:ident($($arg:ident: $ty:ty),*) -> $ret:ty $body:block)*) => {
        $(
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $name($($arg: $ty),*) -> $ret {
                guard(0, || $body)
            }

            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $zname($($arg: $ty),*) -> $ret {
                unsafe { $name($($arg),*) }
            }
        )*
    };
}

macro_rules! forward {
    ($(fn $name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? => $cname:ident, $fail:expr;)*) => {
        unsafe extern "C" {
            $(fn $cname($($arg: $ty),*) $(-> $ret)?;)*
        }
        $(
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $name($($arg: $ty),*) $(-> $ret)? {
                guard($fail, || unsafe { $cname($($arg),*) })
            }
        )*
    };
}

macro_rules! twins {
    ($(fn $name:ident / $zname:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)?;)*) => {
        $(
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $zname($($arg: $ty),*) $(-> $ret)? {
                unsafe { $name($($arg),*) }
            }
        )*
    };
}

export! {
    fn adler32 / z_adler32(adler: u64, buf: *const u8, len: u32) -> u64 {
        BODY
    }
    fn crc32 / z_crc32(crc: u64, buf: *const u8, len: u32) -> u64 {
        crc
    }
}

forward! {
    fn zlibVersion() -> *const u8 => z_zlibVersion, null();
    fn zError(err: i32) -> *const u8 => z_zError, null();
}

twins! {
    fn inflate / z_inflate(strm: *mut u8, flush: i32) -> i32;
    fn inflateEnd / z_inflateEnd(strm: *mut u8);
}

#[cfg(test)]
export! {
    fn t / z_t() -> u64 { 0 }
}

other::bitflags! {
    pub struct Flags: u32 { const A = 1; }
}

other::flags! { pub struct G: u8 { const B = 1; } }
"#;

/// Each function a crate-local `macro_rules!` writes is counted, outside
/// `verus!`, at its invocation's line; a line added to an invocation's body
/// adds a line to its function; an item macro of another crate counts as
/// one function outside `verus!` named for it, over the lines it spans
/// (one for a one-line invocation, its bang and delimiters alike); one
/// under a test-only cfg counts nothing. Negative: an invocation of a
/// crate-local macro that no rule matches fails the run.
#[test]
fn functions_a_macro_writes_are_counted() {
    let dir = std::env::temp_dir().join(format!("verus-count-cli-macros-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("crates/a/src")).expect("mkdir");
    fs::create_dir_all(dir.join("verus")).expect("mkdir");
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n",
    )
    .expect("write");
    fs::write(dir.join("verus/count.toml"), PLAIN).expect("write");
    let lib = dir.join("crates/a/src/lib.rs");
    let rows = |body: &str| {
        fs::write(&lib, MACROS.replace("BODY", body)).expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        assert!(
            got == 0,
            "--list: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
        );
        list_rows(&out)
    };
    let one = rows("adler");
    let at = |name: &str| {
        let line = MACROS
            .lines()
            .position(|l| l.starts_with(name))
            .expect("in MACROS")
            + 1;
        format!("crates/a/src/lib.rs:{line}")
    };
    let mut want: Vec<(String, String, String)> = vec![(
        "Verified".into(),
        "crates/a/src/lib.rs:2".into(),
        "v".into(),
    )];
    for (name, invocation) in [
        ("adler32", "export!"),
        ("z_adler32", "export!"),
        ("crc32", "export!"),
        ("z_crc32", "export!"),
        ("zlibVersion", "forward!"),
        ("zError", "forward!"),
        ("z_inflate", "twins!"),
        ("z_inflateEnd", "twins!"),
        ("bitflags!", "other::bitflags!"),
        ("flags!", "other::flags!"),
    ] {
        want.push(("Outside".into(), at(invocation), name.into()));
    }
    let got: Vec<(String, String, String)> = one
        .iter()
        .map(|(c, f, n, _)| (c.clone(), f.clone(), n.clone()))
        .collect();
    assert_eq!(got, want, "rows: got = {got:?}, wanted = {want:?}");
    let lines = |r: &[(String, String, String, String)], name: &str| {
        r.iter()
            .find(|x| x.2 == name)
            .map(|x| x.3.parse::<usize>().expect("a count"))
            .expect("listed")
    };
    for (name, want) in [("bitflags!", 3), ("flags!", 1)] {
        let got = lines(&one, name);
        assert!(got == want, "{name}'s lines: got = {got}, wanted = {want}");
    }
    // A second line in adler32's body: one more line for adler32 alone.
    let two = rows("let s = adler;\n        s");
    for (name, more) in [("adler32", 1), ("z_adler32", 0), ("crc32", 0)] {
        let (a, b) = (lines(&one, name), lines(&two, name));
        assert!(
            b == a + more,
            "{name}'s lines after a body line: got = {b}, wanted = {}",
            a + more
        );
    }
    fs::write(
        &lib,
        format!(
            "{}\nexport! {{ not a function }}\n",
            MACROS.replace("BODY", "adler")
        ),
    )
    .expect("write");
    let (got, out, err) = count(&dir, &[]);
    let want = "export! does not expand: no rule matches the invocation";
    assert!(
        got == 1 && err.contains(want),
        "no rule matches: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The function rows of `--list` output `out`: class, place, name and
/// lines.
fn list_rows(out: &str) -> Vec<(String, String, String, String)> {
    out.lines()
        .filter(|l| {
            l.split('\t')
                .nth(1)
                .is_some_and(|c| c.starts_with(char::is_uppercase))
        })
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (
                f[1].to_owned(),
                f[3].to_owned(),
                f[4].to_owned(),
                f[2].to_owned(),
            )
        })
        .collect()
}

/// A crate `a` with `files` (path under `src/` and text) and `PLAIN`.
fn crate_of(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("verus-count-cli-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("crates/a/src")).expect("mkdir");
    fs::create_dir_all(dir.join("verus")).expect("mkdir");
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n",
    )
    .expect("write");
    fs::write(dir.join("verus/count.toml"), PLAIN).expect("write");
    for (path, text) in files {
        fs::write(dir.join("crates/a/src").join(path), text).expect("write");
    }
    dir
}

/// An invocation expands the definition rustc would: one a shipped build
/// compiles (not one under `#[cfg(test)]`, in a `#[cfg(test)]` module or
/// `verus!` block, or in a test-only file), and one whose textual scope it is in (not one in
/// a module that has closed). Each would otherwise shadow the definition
/// that writes the function, or make the name ambiguous.
#[test]
fn an_invocation_expands_the_definition_in_scope() {
    let lib = "#[macro_use]\nmod helpers;\n#[cfg(test)]\nmod tests;\n\nmacro_rules! m { () => { pub fn f() {} }; }\n#[cfg(test)]\nmacro_rules! m { () => {}; }\nm!();\n\nn!();\n\nmacro_rules! k { () => { pub fn h() {} }; }\nmod closed { macro_rules! k { () => {}; } }\nk!();\n";
    let helpers = "macro_rules! n { () => { pub fn g() {} }; }\n#[cfg(test)]\nmod inner { macro_rules! n { () => {}; } }\n#[cfg(test)]\nverus! { macro_rules! n { () => {}; } }\n";
    let tests = "macro_rules! n { () => {}; }\n";
    let dir = crate_of(
        "scope",
        &[
            ("lib.rs", lib),
            ("helpers.rs", helpers),
            ("tests.rs", tests),
        ],
    );
    let (got, out, err) = count(&dir, &["--list"]);
    let names: Vec<String> = list_rows(&out).into_iter().map(|r| r.2).collect();
    let want = ["f", "g", "h"];
    assert!(
        got == 0 && names == want,
        "functions: got = exit {got}, {names:?}, {err:?}; wanted = exit 0, {want:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A function a macro writes into `verus!` under a `cfg` Verus's build
/// might not meet (`unix`) is shipped and trusted, never verified, as a
/// hand-written one under the same `cfg` is; without the `cfg` it is
/// verified.
#[test]
fn a_function_a_macro_writes_under_an_unsure_cfg_is_trusted() {
    let lib = "macro_rules! vone { ($n:ident) => { verus! { pub fn $n() {} } }; }\n#[cfg(unix)]\nvone!(v1);\nvone!(v2);\n#[cfg(unix)]\nverus! { pub fn h() {} }\n";
    let dir = crate_of("unsure-macro", &[("lib.rs", lib)]);
    let (got, out, err) = count(&dir, &["--list"]);
    let rows: Vec<(String, String)> = list_rows(&out)
        .into_iter()
        .map(|(class, _, name, _)| (class, name))
        .collect();
    let want: Vec<(String, String)> = [("Trusted", "v1"), ("Verified", "v2"), ("Trusted", "h")]
        .iter()
        .map(|(c, n)| ((*c).to_owned(), (*n).to_owned()))
        .collect();
    assert!(
        got == 0 && rows == want,
        "rows: got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A `vis` forwarded to another macro is one fragment there, empty or
/// `pub(crate)`: `inner!`'s first rule matches and writes the function,
/// where its catch-all would write nothing.
#[test]
fn a_forwarded_vis_is_one_fragment() {
    for (invocation, want) in [("outer!(fn a);", "a"), ("outer!(pub(crate) fn a);", "a")] {
        let lib = format!(
            "macro_rules! inner {{\n    ($v:vis fn $n:ident) => {{ $v fn $n() {{}} }};\n    ($($t:tt)*) => {{}};\n}}\nmacro_rules! outer {{ ($v:vis fn $n:ident) => {{ inner!($v fn $n); }}; }}\n{invocation}\n"
        );
        let dir = crate_of("vis", &[("lib.rs", &lib)]);
        let (got, out, err) = count(&dir, &["--list"]);
        let names: Vec<String> = list_rows(&out).into_iter().map(|r| r.2).collect();
        assert!(
            got == 0 && names == [want],
            "{invocation}: got = exit {got}, {names:?}, {err:?}; wanted = exit 0, [{want:?}]"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

/// The review's three invocations rustc expands, through the command
/// line: a `ty` forwarded to another macro stays a type (`inner!` writes
/// `f`, not its `(u8)` rule's nothing), `true` is a `literal`, and
/// `let x = 1` is a `stmt`.
#[test]
fn fragments_match_as_rustc_matches_them() {
    let lib = "macro_rules! inner { (u8) => {}; ($t:ty) => { pub fn f() {} }; }\nmacro_rules! outer { ($t:ty) => { inner!($t); }; }\nouter!(u8);\n\nmacro_rules! b { ($b:literal) => { pub fn g() -> bool { $b } }; }\nb!(true);\n\nmacro_rules! s { ($s:stmt) => { pub fn h() { $s; } }; }\ns!(let x = 1);\n";
    let dir = crate_of("fragments", &[("lib.rs", lib)]);
    let (got, out, err) = count(&dir, &["--list"]);
    let names: Vec<String> = list_rows(&out).into_iter().map(|r| r.2).collect();
    let want = ["f", "g", "h"];
    assert!(
        got == 0 && names == want,
        "functions: got = exit {got}, {names:?}, {err:?}; wanted = exit 0, {want:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The base revision's rlimits are read as the head's are, through the
/// crate's macros: a macro whose definition, in another file, writes
/// `verus! { #[verifier::rlimit(1)] .. }` holds its function to that
/// limit, so raising it, or dropping the attribute for the default, needs
/// an `rlimit-raised` line.
#[test]
fn a_limit_a_macro_writes_is_held_to_the_base() {
    let lib = "#[macro_use]\nmod defs;\nw!();\npub fn o() {}\n";
    let defs = |attr: &str| {
        format!("macro_rules! w {{ () => {{ verus! {{ {attr} pub fn w() {{}} }} }}; }}\n")
    };
    let dir = crate_of(
        "macro-rlimit",
        &[
            ("lib.rs", lib),
            ("defs.rs", &defs("#[verifier::rlimit(1)]")),
        ],
    );
    fs::write(dir.join("verus/count.toml"), RATCHET_CONFIG).expect("write");
    fs::write(dir.join("verus/FLOOR"), RATCHET_FLOOR).expect("write");
    fs::write(dir.join("verus/RLIMIT"), RATCHET_RLIMIT).expect("write");
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "crates", "verus"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    let stats = dir.join("stats.json");
    fs::write(&stats, ratchet_stats(1_000_000).replace("a::v", "a::w")).expect("write");
    for (case, attr, code, want) in [
        (
            "as committed",
            "#[verifier::rlimit(1)]",
            0,
            "ok rlimit headroom",
        ),
        (
            "raised",
            "#[verifier::rlimit(2)]",
            1,
            "raises a::w's rlimit from 1 to 2 with no new `rlimit-raised a::w 2 <reason>` line",
        ),
        (
            "dropped",
            "",
            1,
            "raises a::w's rlimit from 1 to 10 with no new `rlimit-raised a::w 10 <reason>` line",
        ),
    ] {
        fs::write(dir.join("crates/a/src/defs.rs"), defs(attr)).expect("write");
        let (got, out, err) = count(
            &dir,
            &[
                "--floor",
                &dir.join("verus/FLOOR").to_string_lossy(),
                "--rlimit",
                &stats.to_string_lossy(),
                "--base-rev",
                "HEAD",
            ],
        );
        assert!(
            got == code && (out.contains(want) || err.contains(want)),
            "{case}: got = exit {got}, {out:?}, {err:?}; wanted = exit {code}, {want:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Macros defined in a second file, line for line as the expansion uses
/// them: `one!`'s function's tokens sit on lines 3 to 5, the same numbers
/// as lines of the invocation in lib.rs.
const SHAPES_DEFS: &str = "macro_rules! one {
    ($name:ident) => {
        pub fn $name() {
            let a = 1;
        }
    };
}
macro_rules! shl { ($name:ident, $op:tt) => { pub fn $name(a: u32, b: u32) -> u32 { a $op b } }; ($($rest:tt)*) => {}; }
macro_rules! nest { ($n:ident) => { one! { $n } }; }
macro_rules! meth { ($n:ident) => { fn $n(&self) {} }; }
macro_rules! vone { ($n:ident) => { verus! { pub fn $n() {} } }; }
macro_rules! deep { () => { deep!(); }; }
macro_rules! bad { () => { pub fn }; }
";

/// Two definitions of each name in one module of a third file, under
/// `cfg`s the counter cannot settle: `pair!`'s write the same functions,
/// `diff!`'s do not.
const SHAPES_PAIRS: &str = "#[cfg(unix)]
macro_rules! pair { ($($n:ident),*) => { $(pub fn $n() {})* }; }
#[cfg(not(unix))]
macro_rules! pair { ($($n:ident),*) => { $(pub fn $n() {})* }; }
#[cfg(unix)]
macro_rules! diff { () => { pub fn du() {} }; }
#[cfg(not(unix))]
macro_rules! diff { () => { pub fn dw() {} }; }
";

const SHAPES_LIB: &str = "#[macro_use]
mod defs;
#[macro_use]
mod pairs;
one! {
    f
}
shl!(shl, <<);
nest!(n1);
pub struct S;
impl S {
    meth!(m1);
}
pub trait T {
    meth!(m2);
}
verus! {
    one! { inside }
}
vone!(v1);
pair!(a, b, c);
macro_rules! two { () => { pub fn first() {} }; }
macro_rules! two { () => { pub fn second() {} }; }
two!();
#[cfg(unix)]
macro_rules! here { () => { pub fn h1() {} }; }
#[cfg(not(unix))]
macro_rules! here { () => { pub fn h1() {} }; }
here!();
";

/// Each shape of invocation a port writes, through the command line: a
/// definition in another file (with `f`'s lines, 3 from the definition and
/// 1 from the invocation, though both files number them alike), a `tt`
/// that is one token of two characters (`<<`), an expansion that writes an
/// invocation (`nest!` writes `one!`), invocations in an `impl` and in a
/// trait, one inside `verus!` (outside, as `verus!` expands no macro) and
/// one that writes `verus! { .. }` (verified), two definitions in another
/// file's module that write the same functions (expanded), two in the
/// invoking file (the last before the invocation), and two in the
/// invoking file under `cfg(unix)` and `cfg(not(unix))` that write the
/// same functions (expanded). Negative: two definitions that write
/// different functions, in another file or under those `cfg`s in the
/// invoking file, an expansion deeper than rustc's recursion limit, and
/// one that does not parse each exit 1.
#[test]
fn every_invocation_shape_is_counted() {
    let dir = crate_of(
        "shapes",
        &[
            ("lib.rs", SHAPES_LIB),
            ("defs.rs", SHAPES_DEFS),
            ("pairs.rs", SHAPES_PAIRS),
        ],
    );
    let (got, out, err) = count(&dir, &["--list"]);
    let rows: Vec<(String, String, String)> = list_rows(&out)
        .into_iter()
        .map(|(class, _, name, lines)| (class, name, lines))
        .collect();
    let row = |class: &str, name: &str, lines: &str| (class.into(), name.into(), lines.into());
    let want: Vec<(String, String, String)> = vec![
        row("Outside", "f", "4"),
        row("Outside", "shl", "2"),
        row("Outside", "n1", "4"),
        row("Outside", "m1", "2"),
        row("Outside", "m2", "2"),
        row("Outside", "inside", "4"),
        row("Verified", "v1", "2"),
        row("Outside", "a", "2"),
        row("Outside", "b", "2"),
        row("Outside", "c", "2"),
        row("Outside", "second", "1"),
        row("Outside", "h1", "1"),
    ];
    assert!(
        got == 0 && rows == want,
        "rows: got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
    );
    for (invocation, want) in [
        (
            "diff!();",
            "diff! could be any of 2 definitions (crates/a/src/pairs.rs:6, crates/a/src/pairs.rs:8), which count differently",
        ),
        (
            "#[cfg(unix)]\nmacro_rules! pm { () => { pub fn ma() {} pub fn mb() {} }; }\n#[cfg(not(unix))]\nmacro_rules! pm { () => { pub fn ma() {} }; }\npm!();",
            "pm! could be any of 2 definitions (crates/a/src/lib.rs:33, crates/a/src/lib.rs:31), which count differently",
        ),
        ("deep!();", "deep! expands more than 128 macros deep"),
        ("bad!();", "bad! does not expand"),
    ] {
        fs::write(
            dir.join("crates/a/src/lib.rs"),
            format!("{SHAPES_LIB}{invocation}\n"),
        )
        .expect("write");
        let (got, out, err) = count(&dir, &[]);
        assert!(
            got == 1 && err.contains(want),
            "{invocation}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// A definition reaches an invocation in another file only along a path
/// rustc resolves: an enclosing module's textual scope (not a sibling
/// file's private definition), a `use NAME;` re-export named by a path
/// (`w::ex!`, `$crate::w::ex!`), or a definition an expansion writes
/// (a macro that defines a macro; of two one expansion writes, the later
/// shadows). Two definitions that may each be the one must count the same
/// rows, class and lines included: a pair whose functions differ only in
/// `external_body` exits 1, as does a pair one of which writes an
/// invocation that is itself ambiguous, and a same-file definition under
/// a `cfg` beside an enclosing module's unconditional one. An invocation
/// imported under a new name, or through a module imported under one,
/// resolves by the imported path (from `crate::` or from the root). One
/// that resolves to none of the crate's definitions of its name or of the
/// name it was imported as (through a glob of a re-export, a renamed
/// import of another module's re-export, or a definition a file read
/// later writes) exits 1 rather than counting as one function, as does a
/// module alias under `cfg`s one of whose paths names no definition. A
/// test-only `use` imports nothing.
#[test]
fn a_definition_reaches_an_invocation_as_rustc_resolves_it() {
    for (case, files, code, want) in [
        (
            "an enclosing module's, not a sibling file's",
            &[
                (
                    "lib.rs",
                    "macro_rules! m { () => { pub fn f() {} }; }\nmod caller;\nmod closed;\n",
                ),
                ("caller.rs", "m!();\n"),
                ("closed.rs", "macro_rules! m { () => {}; }\n"),
            ][..],
            0,
            &["f"][..],
        ),
        (
            "a re-export named by a path",
            &[
                (
                    "lib.rs",
                    "mod w;\nw::ex!();\nmacro_rules! twice { () => { $crate::w::ex2!(); }; }\ntwice!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { pub fn pe() {} }; }\npub(crate) use ex;\nmacro_rules! ex2 { () => { pub fn pe2() {} }; }\npub(crate) use ex2;\n",
                ),
            ][..],
            0,
            &["pe", "pe2"][..],
        ),
        (
            "a definition an expansion writes",
            &[(
                "lib.rs",
                "macro_rules! outer {\n    ($name:ident) => {\n        macro_rules! $name { ($x:ident) => { pub fn $x() {} pub fn b() {} }; }\n        $name!(a);\n    };\n}\nouter!(inner);\n",
            )][..],
            0,
            &["a", "b"][..],
        ),
        (
            "a pair that differs in class alone",
            &[
                ("lib.rs", "#[macro_use]\nmod pair;\nv!();\n"),
                (
                    "pair.rs",
                    "#[cfg(unix)]\nmacro_rules! v { () => { verus! { pub fn f() {} } }; }\n#[cfg(not(unix))]\nmacro_rules! v { () => { verus! { #[verifier::external_body] pub fn f() {} } }; }\n",
                ),
            ][..],
            1,
            &["v! could be any of 2 definitions"][..],
        ),
        (
            "a pair one of which writes an ambiguous invocation",
            &[
                ("lib.rs", "#[macro_use]\nmod defs;\nm!();\n"),
                (
                    "defs.rs",
                    "#[cfg(unix)] macro_rules! m { () => {}; }\n#[cfg(not(unix))] macro_rules! m { () => { n!(); }; }\n#[cfg(unix)] macro_rules! n { () => { fn a() {} }; }\n#[cfg(not(unix))] macro_rules! n { () => { fn b() {} }; }\n",
                ),
            ][..],
            1,
            &[
                "m! could be any of 2 definitions (crates/a/src/defs.rs:1, crates/a/src/defs.rs:2), and one of them does not count",
            ][..],
        ),
        (
            "definitions one expansion writes, in order",
            &[(
                "lib.rs",
                "macro_rules! outer {\n    () => {\n        macro_rules! inner { () => {}; }\n        inner!();\n        macro_rules! inner { () => { pub fn b() {} pub fn c() {} }; }\n        inner!();\n    };\n}\nouter!();\n",
            )][..],
            0,
            &["b", "c"][..],
        ),
        (
            "every same-file definition under a cfg, an enclosing one without",
            &[
                (
                    "lib.rs",
                    "macro_rules! m { () => { fn a() {} fn b() {} }; }\nmod caller;\n",
                ),
                (
                    "caller.rs",
                    "#[cfg(unix)]\nmacro_rules! m { () => { fn a() {} }; }\nm!();\n",
                ),
            ][..],
            1,
            &["m! could be any of 2 definitions"][..],
        ),
        (
            "a definition reached through a glob of a re-export",
            &[
                ("lib.rs", "mod w;\nmod prelude;\nmod caller;\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} fn b() {} }; }\npub(crate) use ex;\n",
                ),
                ("prelude.rs", "pub(crate) use crate::w::ex;\n"),
                ("caller.rs", "use crate::prelude::*;\nex!();\n"),
            ][..],
            0,
            &["a", "b"][..],
        ),
        (
            "a glob of a module whose re-export the counter cannot follow",
            &[
                (
                    "lib.rs",
                    "extern crate self as me;\nmod w;\nmod prelude;\nmod caller;\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} fn b() {} }; }\npub(crate) use ex;\n",
                ),
                ("prelude.rs", "pub(crate) use me::w::ex;\n"),
                ("caller.rs", "use crate::prelude::*;\nex!();\n"),
            ][..],
            1,
            &[
                "crates/a/src/caller.rs:2: ex! may come through `crate::prelude::*`, which re-exports `ex` by a path the counter does not follow",
            ][..],
        ),
        (
            "globs under cfgs of modules that re-export definitions counting differently",
            &[
                (
                    "lib.rs",
                    "mod w;\nmod v;\nmod p;\n#[cfg(unix)] use crate::w::*;\n#[cfg(not(unix))] use crate::p::*;\nbuild!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex as build;\n",
                ),
                (
                    "v.rs",
                    "macro_rules! ex { () => { fn a() {} fn b() {} }; }\npub(crate) use ex as build;\n",
                ),
                ("p.rs", "pub(crate) use crate::v::build;\n"),
            ][..],
            1,
            &["build! could be any of 2 definitions"][..],
        ),
        (
            "globs under cfgs of modules that re-export definitions counting the same",
            &[
                (
                    "lib.rs",
                    "mod w;\nmod v;\nmod p;\n#[cfg(unix)] use crate::w::*;\n#[cfg(not(unix))] use crate::p::*;\nbuild!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex as build;\n",
                ),
                (
                    "v.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex as build;\n",
                ),
                ("p.rs", "pub(crate) use crate::v::build;\n"),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a named use and a glob under cfgs, to definitions counting differently",
            &[
                (
                    "lib.rs",
                    "mod w;\nmod v;\nmod p;\n#[cfg(unix)] use crate::w::ex;\n#[cfg(not(unix))] use crate::p::*;\nex!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                (
                    "v.rs",
                    "macro_rules! ex { () => { fn a() {} fn b() {} }; }\npub(crate) use ex;\n",
                ),
                ("p.rs", "pub(crate) use crate::v::ex;\n"),
            ][..],
            1,
            &["ex! could be any of 2 definitions"][..],
        ),
        (
            "a rename in another module, by its path",
            &[
                ("lib.rs", "mod w;\nmod p;\np::make!();\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                ("p.rs", "pub(crate) use crate::w::ex as make;\n"),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a rename in another module, by its path from the crate",
            &[
                ("lib.rs", "mod w;\nmod p;\ncrate::p::make!();\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                ("p.rs", "pub(crate) use crate::w::ex as make;\n"),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a rename in another module, through a glob",
            &[
                ("lib.rs", "mod w;\nmod p;\nuse crate::p::*;\nmake!();\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                ("p.rs", "pub(crate) use crate::w::ex as make;\n"),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a two-step chain of renames",
            &[
                ("lib.rs", "mod w;\nmod p;\np::make!();\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex as build;\n",
                ),
                ("p.rs", "pub(crate) use crate::w::build as make;\n"),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a three-step chain of renames",
            &[
                ("lib.rs", "mod w;\nmod p;\nmod q;\nq::run!();\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex as build;\n",
                ),
                ("p.rs", "pub(crate) use crate::w::build as make;\n"),
                ("q.rs", "pub(crate) use crate::p::make as run;\n"),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a path through a module's rename the counter cannot follow",
            &[
                (
                    "lib.rs",
                    "extern crate self as me;\nmod w;\nmod q;\nq::run!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                ("q.rs", "pub(crate) use me::w::ex as run;\n"),
            ][..],
            1,
            &[
                "crates/a/src/lib.rs:4: q::run! names `run` in `a::q`, which brings it in by a `use` the counter does not follow",
            ][..],
        ),
        (
            "a named use under cfgs, one path of which names no definition",
            &[
                (
                    "lib.rs",
                    "extern crate self as me;\nmod w;\nmod v;\n#[cfg(unix)] use crate::w::ex;\n#[cfg(not(unix))] use me::v::ex;\nex!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                (
                    "v.rs",
                    "macro_rules! ex { () => { fn a() {} fn b() {} }; }\npub(crate) use ex;\n",
                ),
            ][..],
            1,
            &[
                "crates/a/src/lib.rs:6: ex! may stand for `me::v::ex`, which names none of this crate's definitions",
            ][..],
        ),
        (
            "a definition imported under a new name",
            &[
                ("lib.rs", "mod w;\nmod caller;\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                ("caller.rs", "use crate::w::ex as build;\nbuild!();\n"),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a definition imported under a new name by a path from the root",
            &[
                ("lib.rs", "mod w;\nuse w::ex as build;\nbuild!();\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} fn b() {} }; }\npub(crate) use ex;\n",
                ),
            ][..],
            0,
            &["a", "b"][..],
        ),
        (
            "a module imported under a new name",
            &[
                ("lib.rs", "mod w;\nmod caller;\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                ("caller.rs", "use crate::w as x;\nx::ex!();\n"),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a module alias under cfgs, to definitions that count differently",
            &[
                (
                    "lib.rs",
                    "mod w;\nmod v;\n#[cfg(unix)] use crate::w as x;\n#[cfg(not(unix))] use crate::v as x;\nx::ex!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                (
                    "v.rs",
                    "macro_rules! ex { () => { fn a() {} fn b() {} }; }\npub(crate) use ex;\n",
                ),
            ][..],
            1,
            &[
                "ex! could be any of 2 definitions (crates/a/src/w.rs:1, crates/a/src/v.rs:1), which count differently",
            ][..],
        ),
        (
            "a module alias under cfgs, to definitions that count the same",
            &[
                (
                    "lib.rs",
                    "mod w;\nmod v;\n#[cfg(unix)] use crate::w as x;\n#[cfg(not(unix))] use crate::v as x;\nx::ex!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                (
                    "v.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a module alias under cfgs, one path through a re-export",
            &[
                (
                    "lib.rs",
                    "mod w;\nmod v;\nmod p;\n#[cfg(unix)] use crate::w as x;\n#[cfg(not(unix))] use crate::p as x;\nx::ex!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                (
                    "v.rs",
                    "macro_rules! ex { () => { fn a() {} fn b() {} }; }\npub(crate) use ex;\n",
                ),
                ("p.rs", "pub(crate) use crate::v::ex;\n"),
            ][..],
            1,
            &["crates/a/src/lib.rs:6: ex! could be any of 2 definitions"][..],
        ),
        (
            "a module alias under cfgs, one path of which names no definition",
            &[
                (
                    "lib.rs",
                    "extern crate self as me;\nmod w;\nmod v;\nmod p;\n#[cfg(unix)] use crate::w as x;\n#[cfg(not(unix))] use crate::p as x;\nx::ex!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                (
                    "v.rs",
                    "macro_rules! ex { () => { fn a() {} fn b() {} }; }\npub(crate) use ex;\n",
                ),
                ("p.rs", "pub(crate) use me::v::ex;\n"),
            ][..],
            1,
            &[
                "crates/a/src/lib.rs:7: ex! may stand for `crate::p::ex`, which names none of this crate's definitions",
            ][..],
        ),
        (
            "a module alias a test-only use would shadow",
            &[
                (
                    "lib.rs",
                    "mod w;\nmod v;\n#[cfg(not(test))] use crate::w as x;\n#[cfg(test)] use crate::v as x;\nx::ex!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                (
                    "v.rs",
                    "macro_rules! ex { () => { fn a() {} fn b() {} }; }\npub(crate) use ex;\n",
                ),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a module alias or a module of the same name, counting differently",
            &[
                (
                    "lib.rs",
                    "mod w;\n#[cfg(unix)] mod x;\n#[cfg(not(unix))] use crate::w as x;\nx::ex!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                (
                    "x.rs",
                    "macro_rules! ex { () => { fn a() {} fn b() {} }; }\npub(crate) use ex;\n",
                ),
            ][..],
            1,
            &["ex! could be any of 2 definitions"][..],
        ),
        (
            "a module alias or a module of the same name, counting the same",
            &[
                (
                    "lib.rs",
                    "mod w;\n#[cfg(unix)] mod x;\n#[cfg(not(unix))] use crate::w as x;\nx::ex!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                (
                    "x.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a re-export under a new name, by its path",
            &[
                ("lib.rs", "mod w;\nw::build!();\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex as build;\n",
                ),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a re-export under a new name, through a glob",
            &[
                ("lib.rs", "mod w;\nuse crate::w::*;\nbuild!();\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex as build;\n",
                ),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a re-export under a new name, through a glob of another re-export",
            &[
                ("lib.rs", "mod w;\nmod p;\nuse crate::p::*;\nbuild!();\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex as build;\n",
                ),
                ("p.rs", "pub(crate) use crate::w::build;\n"),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a definition an expansion writes in another file, through a glob",
            &[
                (
                    "lib.rs",
                    "mod w;\nmod caller;\nmacro_rules! outer {\n    ($name:ident) => {\n        macro_rules! $name { () => { fn a() {} }; }\n    };\n}\nouter!(build);\n",
                ),
                ("w.rs", ""),
                ("caller.rs", "use crate::w::*;\nbuild!();\n"),
            ][..],
            1,
            &["crates/a/src/caller.rs:2: build! resolves to none of this crate's definitions"][..],
        ),
        (
            "a renamed import through a module's re-export",
            &[
                ("lib.rs", "mod w;\nmod p;\nuse p::ex as build;\nbuild!();\n"),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                ("p.rs", "pub(crate) use crate::w::ex;\n"),
            ][..],
            0,
            &["a"][..],
        ),
        (
            "a renamed import through a module's re-export the counter cannot follow",
            &[
                (
                    "lib.rs",
                    "extern crate self as me;\nmod w;\nmod p;\nuse p::ex as build;\nbuild!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
                ("p.rs", "pub(crate) use me::w::ex;\n"),
            ][..],
            1,
            &["build! is imported from this crate by a path the counter does not follow"][..],
        ),
        (
            "a renamed import by a path the counter cannot place",
            &[
                (
                    "lib.rs",
                    "extern crate self as me;\nmod w;\nuse me::w::ex as build;\nbuild!();\n",
                ),
                (
                    "w.rs",
                    "macro_rules! ex { () => { fn a() {} }; }\npub(crate) use ex;\n",
                ),
            ][..],
            1,
            &["crates/a/src/lib.rs:4: ex! resolves to none of this crate's definitions"][..],
        ),
        (
            "a definition an expansion in a file read later writes",
            &[
                (
                    "lib.rs",
                    "macro_rules! outer {\n    ($name:ident) => {\n        macro_rules! $name { ($x:ident) => { fn $x() {} fn b() {} }; }\n    };\n}\nouter!(inner);\nmod caller;\n",
                ),
                ("caller.rs", "inner!(a);\n"),
            ][..],
            1,
            &["inner! resolves to none of this crate's definitions"][..],
        ),
    ] {
        let dir = crate_of("resolve", files);
        let (got, out, err) = count(&dir, &["--list"]);
        let names: Vec<String> = list_rows(&out).into_iter().map(|r| r.2).collect();
        let ok = match code {
            0 => names == want,
            _ => want.iter().all(|w| err.contains(w)),
        };
        assert!(
            got == code && ok,
            "{case}: got = exit {got}, {names:?}, {err:?}; wanted = exit {code}, {want:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

/// wolfi-dev/proved-port#12's reproducer: a function under a `cfg` Verus's
/// build might not meet (`target_os = "windows"`, which a Linux proof run
/// never sees) ships with nothing to prove Verus checked it, so it counts
/// as trusted, not verified.
#[test]
fn a_platform_cfg_is_not_verified() {
    let dir = repository("windows");
    fs::write(
        dir.join("crates/a/src/lib.rs"),
        "#[cfg(target_os = \"windows\")]\n#[verus_verify]\npub fn f() {}\n\npub fn g() {}\n",
    )
    .expect("write");
    let (got, out, err) = count(&dir, &[]);
    let want = "a\t0\t1\t0\t1\t0.0%\t0.0%\t";
    assert!(
        got == 0 && out.lines().any(|l| l.starts_with(want)),
        "got = exit {got}, {out:?}, {err:?}; wanted = exit 0, a row {want:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// Each way a `cfg` reaches a function: its own attributes, an enclosing
/// `impl`, trait, inline `mod` or `verus!` block, an out-of-line
/// `#[cfg(..)] mod x;` whose file, and the files under it, it classes, and
/// an inline module whose `cfg` the `mod child;` inside it carries to
/// child's file and the files under that, or whose `mod child;` has its
/// own, a file's inner `#![cfg(..)]`, which guards the files under it too,
/// and `#[cfg_attr(P, cfg(Q))]`. Under `not(miri)` and `feature = "jit"`,
/// which the configuration
/// does not settle, every one of them is trusted; with `jit` in
/// `verus_sets` and `miri` in `verus_never_sets`, each is verified.
/// `windows` and the file under a `windows` inline module stay trusted,
/// `sure` and the file under an unguarded inline module verified, and
/// `plain` outside throughout.
#[test]
fn every_cfg_path_is_sure_only_when_settled() {
    const LIB: &str = r#"
#[cfg(target_os = "windows")]
#[verus_verify]
pub fn windows() {}
pub fn plain() {}
pub struct S;
verus! {
pub fn sure() {}
#[cfg(not(miri))]
pub fn own() {}
#[cfg(not(miri))]
impl S {
    pub fn in_impl(&self) {}
}
#[cfg(not(miri))]
pub trait T {
    fn in_trait(&self) {}
}
#[cfg(not(miri))]
pub mod inline {
    pub fn in_mod() {}
}
}
#[cfg(not(miri))]
verus! {
pub fn in_block() {}
}
#[cfg(all(feature = "jit", not(miri)))]
mod exec;
#[cfg(not(miri))]
mod inline_guarded {
    mod child;
}
mod inline_plain {
    #[cfg(not(miri))]
    mod child;
}
#[cfg(target_os = "windows")]
mod inline_windows {
    mod child;
}
mod inline_sure {
    mod child;
}
mod inner_attr;
verus! {
#[cfg_attr(unix, cfg(not(miri)))]
pub fn in_cfg_attr() {}
}
"#;
    let dir = repository("paths");
    let src = dir.join("crates/a/src");
    fs::write(src.join("lib.rs"), LIB).expect("write");
    let files = [
        ("exec.rs", "mod deeper;\nverus! {\npub fn in_file() {}\n}\n"),
        ("exec/deeper.rs", "verus! {\npub fn in_deeper() {}\n}\n"),
        (
            "inline_guarded/child.rs",
            "mod deeper;\nverus! {\npub fn in_inline_file() {}\n}\n",
        ),
        (
            "inline_guarded/child/deeper.rs",
            "verus! {\npub fn in_inline_deeper() {}\n}\n",
        ),
        (
            "inline_plain/child.rs",
            "verus! {\npub fn in_inline_decl() {}\n}\n",
        ),
        (
            "inline_windows/child.rs",
            "verus! {\npub fn windows_inline() {}\n}\n",
        ),
        (
            "inline_sure/child.rs",
            "verus! {\npub fn sure_inline() {}\n}\n",
        ),
        (
            "inner_attr.rs",
            "#![cfg(not(miri))]\nmod deeper;\nverus! {\npub fn in_inner_attr() {}\n}\n",
        ),
        (
            "inner_attr/deeper.rs",
            "verus! {\npub fn in_inner_deeper() {}\n}\n",
        ),
    ];
    for (path, text) in files {
        let p = src.join(path);
        fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        fs::write(p, text).expect("write");
    }
    let paths = [
        "own",
        "in_impl",
        "in_trait",
        "in_mod",
        "in_block",
        "in_file",
        "in_deeper",
        "in_inline_file",
        "in_inline_deeper",
        "in_inline_decl",
        "in_inner_attr",
        "in_inner_deeper",
        "in_cfg_attr",
    ];
    for (config, class) in [
        (PLAIN.to_owned(), "Trusted"),
        (
            format!("{PLAIN}verus_sets = ['feature = \"jit\"']\nverus_never_sets = [\"miri\"]\n"),
            "Verified",
        ),
    ] {
        fs::write(dir.join("verus/count.toml"), &config).expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        assert_eq!(
            got, 0,
            "{config:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
        );
        let class_of = |name: &str| {
            out.lines()
                .find(|l| l.ends_with(&format!("\t{name}")))
                .and_then(|l| l.split('\t').nth(1))
                .unwrap_or("missing")
                .to_owned()
        };
        for (name, want) in paths.iter().map(|n| (*n, class)).chain([
            ("windows", "Trusted"),
            ("windows_inline", "Trusted"),
            ("sure", "Verified"),
            ("sure_inline", "Verified"),
            ("plain", "Outside"),
        ]) {
            let got = class_of(name);
            assert_eq!(
                got, want,
                "{config:?}: {name}: got = {got}, wanted = {want}"
            );
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

/// The class of the function `name` in `--list`'s rows `out`, or
/// "missing" when no row names it.
fn class_in(out: &str, name: &str) -> String {
    out.lines()
        .find(|l| l.ends_with(&format!("\t{name}")))
        .and_then(|l| l.split('\t').nth(1))
        .unwrap_or("missing")
        .to_owned()
}

/// A crate root's inner `#![cfg(target_os = "windows")]` guards every file
/// of the crate, so nothing in it is verified; a module file's
/// `#![cfg(test)]`, and a `#[cfg(test)] mod t;` inside an inline module,
/// leave their files out.
#[test]
fn an_inner_cfg_guards_its_file_and_those_under_it() {
    let dir = repository("inner");
    let src = dir.join("crates/a/src");
    fs::create_dir_all(src.join("m")).expect("mkdir");
    for (path, text) in [
        (
            "lib.rs",
            "#![cfg(target_os = \"windows\")]\nmod below;\nmod tests_file;\nmod m {\n    #[cfg(test)]\n    mod t;\n}\nverus! {\npub fn at_root() {}\n}\n",
        ),
        ("below.rs", "verus! {\npub fn below() {}\n}\n"),
        (
            "tests_file.rs",
            "#![cfg(test)]\nverus! {\npub fn in_test_file() {}\n}\n",
        ),
        ("m/t.rs", "verus! {\npub fn in_inline_test() {}\n}\n"),
    ] {
        fs::write(src.join(path), text).expect("write");
    }
    let (got, out, err) = count(&dir, &["--list"]);
    assert_eq!(
        got, 0,
        "got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
    );
    for (name, want) in [
        ("at_root", "Trusted"),
        ("below", "Trusted"),
        ("in_test_file", "missing"),
        ("in_inline_test", "missing"),
    ] {
        let got = class_in(&out, name);
        assert_eq!(got, want, "{name}: got = {got}, wanted = {want}");
    }
    let _ = fs::remove_dir_all(&dir);
}

/// The module declarations the first pass must find and place. A file
/// declared twice is built where either declaration holds, so it stays in
/// the count: `unix.rs` (two `#[path]` declarations, linux and macos) and
/// `fast.rs` (x86_64 and aarch64) are trusted with nothing declared and
/// verified once the proof runs settle their atoms; `imp.rs`, under `std`
/// and `not(std)`, stays trusted, as the counter reads each atom on its
/// own. A file declared inside a `cfg_if!` body is trusted whatever is
/// declared, since the counter does not evaluate the macro (one a `#[path]`
/// there names too, with the files it declares beside it, whatever
/// visibility or other attributes stand between the `#[path]` and the
/// `mod`), and one
/// declared under a `cfg` inside `verus!` takes that `cfg`. A file a
/// `#[path]` names declares its modules beside it, as a `mod.rs` does,
/// and they take its guard; so does a file a `#[path]` on an inline module
/// in a module file places, beside that file.
#[test]
fn module_declarations_are_found_and_placed() {
    const LIB: &str = r#"
#[cfg(target_os = "linux")]
#[path = "unix.rs"]
mod sys;
#[cfg(target_os = "macos")]
#[path = "unix.rs"]
mod sys;
#[cfg(target_arch = "x86_64")]
pub mod fast;
#[cfg(target_arch = "aarch64")]
mod fast;
#[cfg(feature = "std")]
pub mod imp;
#[cfg(not(feature = "std"))]
mod imp;
cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        mod win;
        #[path = "windows_impl.rs"]
        mod platform;
        #[path = "vis_pub.rs"]
        #[cfg(unix)]
        pub mod named_pub;
        #[path = "vis_crate.rs"]
        pub(crate) mod named_crate;
        #[path = "vis_super.rs"]
        #[allow(dead_code)]
        pub(super) mod named_super;
        #[path = "vis_in.rs"]
        pub(in crate::x) mod named_in;
    } else {
        mod other;
    }
}
verus! {
#[cfg(target_os = "windows")]
mod verus_win;
}
#[cfg(target_os = "windows")]
#[path = "arch/win.rs"]
mod pwin;
#[cfg(target_os = "windows")]
mod host;
"#;
    let dir = repository("decls");
    let src = dir.join("crates/a/src");
    for (path, text) in [
        ("lib.rs", LIB),
        ("unix.rs", "verus! {\npub fn in_unix() {}\n}\n"),
        ("fast.rs", "verus! {\npub fn in_fast() {}\n}\n"),
        ("imp.rs", "verus! {\npub fn in_imp() {}\n}\n"),
        ("win.rs", "verus! {\npub fn in_cfg_if() {}\n}\n"),
        ("other.rs", ""),
        (
            "windows_impl.rs",
            "mod wchild;\nverus! {\npub fn in_cfg_if_path() {}\n}\n",
        ),
        ("wchild.rs", "verus! {\npub fn in_cfg_if_child() {}\n}\n"),
        ("vis_pub.rs", "verus! {\npub fn in_vis_pub() {}\n}\n"),
        ("vis_crate.rs", "verus! {\npub fn in_vis_crate() {}\n}\n"),
        ("vis_super.rs", "verus! {\npub fn in_vis_super() {}\n}\n"),
        ("vis_in.rs", "verus! {\npub fn in_vis_in() {}\n}\n"),
        ("verus_win.rs", "verus! {\npub fn in_verus_mod() {}\n}\n"),
        // arch/ sorts before lib.rs, so the first pass reads win.rs
        // before the `#[path]` that names it.
        (
            "arch/win.rs",
            "mod child;\n#[cfg(not(miri))]\nmod guarded;\n",
        ),
        ("arch/child.rs", "verus! {\npub fn in_path_child() {}\n}\n"),
        (
            "arch/guarded.rs",
            "verus! {\npub fn in_path_guarded() {}\n}\n",
        ),
        ("host.rs", "#[path = \"d\"]\nmod inline {\n    mod x;\n}\n"),
        ("d/x.rs", "verus! {\npub fn in_inline_path() {}\n}\n"),
    ] {
        let p = src.join(path);
        fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        fs::write(p, text).expect("write");
    }
    let settled = "verus_sets = ['target_os = \"linux\"', 'target_os = \"windows\"', 'target_arch = \"x86_64\"', 'target_arch = \"aarch64\"']\nverus_never_sets = [\"miri\"]\n";
    for (keys, wants) in [
        (
            "",
            [
                ("in_unix", "Trusted"),
                ("in_fast", "Trusted"),
                ("in_imp", "Trusted"),
                ("in_cfg_if", "Trusted"),
                ("in_cfg_if_path", "Trusted"),
                ("in_cfg_if_child", "Trusted"),
                ("in_vis_pub", "Trusted"),
                ("in_vis_crate", "Trusted"),
                ("in_vis_super", "Trusted"),
                ("in_vis_in", "Trusted"),
                ("in_verus_mod", "Trusted"),
                ("in_path_child", "Trusted"),
                ("in_path_guarded", "Trusted"),
                ("in_inline_path", "Trusted"),
            ],
        ),
        (
            settled,
            [
                ("in_unix", "Verified"),
                ("in_fast", "Verified"),
                ("in_imp", "Trusted"),
                ("in_cfg_if", "Trusted"),
                ("in_cfg_if_path", "Trusted"),
                ("in_cfg_if_child", "Trusted"),
                ("in_vis_pub", "Trusted"),
                ("in_vis_crate", "Trusted"),
                ("in_vis_super", "Trusted"),
                ("in_vis_in", "Trusted"),
                ("in_verus_mod", "Verified"),
                ("in_path_child", "Verified"),
                ("in_path_guarded", "Verified"),
                ("in_inline_path", "Verified"),
            ],
        ),
    ] {
        fs::write(dir.join("verus/count.toml"), format!("{PLAIN}{keys}")).expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        assert_eq!(
            got, 0,
            "{keys:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
        );
        for (name, want) in wants {
            let got = class_in(&out, name);
            assert_eq!(got, want, "{keys:?}: {name}: got = {got}, wanted = {want}");
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

/// `#[cfg_attr(P, path = "..")]` names the module's file where `P` holds,
/// and the default file where none does, so `windows.rs` is built only
/// where `windows` is and `sys.rs` only where neither atom is. A crate
/// file no declaration names (`orphan.rs`) is unsure, never verified.
/// With `unix` sure and `windows` never set in the proof run, only
/// `unix.rs` and the plainly declared `plain.rs` are verified.
#[test]
fn conditional_paths_and_undeclared_files_are_placed() {
    const LIB: &str = r#"
#[cfg_attr(windows, path = "windows.rs")]
#[cfg_attr(unix, path = "unix.rs")]
mod sys;
mod plain;
"#;
    let dir = repository("cfgpath");
    let src = dir.join("crates/a/src");
    for (path, text) in [
        ("lib.rs", LIB),
        ("windows.rs", "verus! {\npub fn in_windows() {}\n}\n"),
        ("unix.rs", "verus! {\npub fn in_unix() {}\n}\n"),
        ("sys.rs", "verus! {\npub fn in_default() {}\n}\n"),
        ("plain.rs", "verus! {\npub fn in_plain() {}\n}\n"),
        ("orphan.rs", "verus! {\npub fn in_orphan() {}\n}\n"),
    ] {
        fs::write(src.join(path), text).expect("write");
    }
    for (keys, wants) in [
        (
            "",
            [
                ("in_windows", "Trusted"),
                ("in_unix", "Trusted"),
                ("in_default", "Trusted"),
                ("in_plain", "Verified"),
                ("in_orphan", "Trusted"),
            ],
        ),
        (
            "verus_sets = [\"unix\"]\nverus_never_sets = [\"windows\"]\n",
            [
                ("in_windows", "Trusted"),
                ("in_unix", "Verified"),
                ("in_default", "Trusted"),
                ("in_plain", "Verified"),
                ("in_orphan", "Trusted"),
            ],
        ),
    ] {
        fs::write(dir.join("verus/count.toml"), format!("{PLAIN}{keys}")).expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        assert_eq!(
            got, 0,
            "{keys:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
        );
        for (name, want) in wants {
            let got = class_in(&out, name);
            assert_eq!(got, want, "{keys:?}: {name}: got = {got}, wanted = {want}");
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

/// rustc takes a module's first path attribute in effect, in source
/// order. With `unix` and `linux` sure in the proof run:
/// - `sys`: `cfg_attr(unix, path = "unix_sys.rs")` then a bare `#[path =
///   "generic.rs"]`, so `generic.rs` is the fallback where `unix` does not
///   hold, and is trusted;
/// - `overlap`: `cfg_attr(unix, ..)` then `cfg_attr(linux, ..)`, so
///   `second.rs` is built only where linux holds and unix does not, which
///   the proof run never sees, and is trusted, as is the default
///   `overlap.rs`;
/// - `bare`: a bare `#[path]` first ends the list, so the later
///   `cfg_attr`'s `never.rs` is no declaration of it, and counts as
///   undeclared.
#[test]
fn path_attributes_take_the_first_in_effect() {
    const LIB: &str = r#"
#[cfg_attr(unix, path = "unix_sys.rs")]
#[path = "generic.rs"]
mod sys;
#[cfg_attr(unix, path = "first.rs")]
#[cfg_attr(target_os = "linux", path = "second.rs")]
mod overlap;
#[path = "bare_first.rs"]
#[cfg_attr(unix, path = "never.rs")]
mod bare;
"#;
    let dir = repository("pathorder");
    let src = dir.join("crates/a/src");
    for (path, text) in [
        ("lib.rs", LIB),
        ("unix_sys.rs", "verus! {\npub fn in_unix_sys() {}\n}\n"),
        ("generic.rs", "verus! {\npub fn in_generic() {}\n}\n"),
        ("first.rs", "verus! {\npub fn in_first() {}\n}\n"),
        ("second.rs", "verus! {\npub fn in_second() {}\n}\n"),
        (
            "overlap.rs",
            "verus! {\npub fn in_overlap_default() {}\n}\n",
        ),
        ("bare_first.rs", "verus! {\npub fn in_bare_first() {}\n}\n"),
        ("never.rs", "verus! {\npub fn in_never() {}\n}\n"),
    ] {
        fs::write(src.join(path), text).expect("write");
    }
    fs::write(
        dir.join("verus/count.toml"),
        format!("{PLAIN}verus_sets = [\"unix\", 'target_os = \"linux\"']\n"),
    )
    .expect("write");
    let (got, out, err) = count(&dir, &["--list"]);
    assert_eq!(
        got, 0,
        "got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
    );
    for (name, want) in [
        ("in_unix_sys", "Verified"),
        ("in_generic", "Trusted"),
        ("in_first", "Verified"),
        ("in_second", "Trusted"),
        ("in_overlap_default", "Trusted"),
        ("in_bare_first", "Verified"),
        ("in_never", "Trusted"),
    ] {
        let got = class_in(&out, name);
        assert_eq!(got, want, "{name}: got = {got}, wanted = {want}");
    }
    let _ = fs::remove_dir_all(&dir);
}

/// `cfg_attr`s apply as rustc expands them, nested to any depth and with
/// several attributes each, for a module's path, a `cfg`, and Verus's
/// attribute form alike: each applies where every predicate around it
/// holds. With nothing declared each is trusted; with `unix`, `linux` and
/// `feature = "fast"` sure, `linux.rs` (two levels), `deep.rs` (three),
/// `multi.rs` (a path after another attribute) and the attribute forms
/// are verified, the default files stay trusted, and a function under a
/// `cfg(windows)` two levels down is trusted, as is `outer_off.rs`, whose
/// inner `linux` holds but whose outer `windows` the run does not settle.
#[test]
fn nested_cfg_attrs_apply_as_rustc_expands_them() {
    const LIB: &str = r#"
#[cfg_attr(unix, cfg_attr(target_os = "linux", path = "linux.rs"))]
mod sys;
#[cfg_attr(unix, cfg_attr(target_os = "linux", cfg_attr(feature = "fast", path = "deep.rs")))]
mod three;
#[cfg_attr(unix, allow(dead_code), path = "multi.rs")]
mod multi;
#[cfg_attr(windows, cfg_attr(target_os = "linux", path = "outer_off.rs"))]
mod outer;
verus! {
#[cfg_attr(unix, cfg_attr(target_os = "linux", cfg(windows)))]
pub fn nested_cfg() {}
}
#[cfg_attr(unix, cfg_attr(target_os = "linux", verus_verify))]
pub fn nested_attr() {}
#[cfg_attr(unix, allow(dead_code), verus_verify)]
pub fn multi_attr() {}
"#;
    let dir = repository("nested");
    let src = dir.join("crates/a/src");
    for (path, text) in [
        ("lib.rs", LIB),
        ("linux.rs", "verus! {\npub fn in_linux() {}\n}\n"),
        ("sys.rs", "verus! {\npub fn in_sys_default() {}\n}\n"),
        ("deep.rs", "verus! {\npub fn in_deep() {}\n}\n"),
        ("three.rs", "verus! {\npub fn in_three_default() {}\n}\n"),
        ("multi.rs", "verus! {\npub fn in_multi() {}\n}\n"),
        ("outer_off.rs", "verus! {\npub fn in_outer_off() {}\n}\n"),
    ] {
        fs::write(src.join(path), text).expect("write");
    }
    for (keys, wants) in [
        (
            "",
            [
                ("in_linux", "Trusted"),
                ("in_sys_default", "Trusted"),
                ("in_deep", "Trusted"),
                ("in_three_default", "Trusted"),
                ("in_multi", "Trusted"),
                ("nested_cfg", "Trusted"),
                ("nested_attr", "Trusted"),
                ("multi_attr", "Trusted"),
                ("in_outer_off", "Trusted"),
            ],
        ),
        (
            "verus_sets = [\"unix\", 'target_os = \"linux\"', 'feature = \"fast\"']\n",
            [
                ("in_linux", "Verified"),
                ("in_sys_default", "Trusted"),
                ("in_deep", "Verified"),
                ("in_three_default", "Trusted"),
                ("in_multi", "Verified"),
                ("nested_cfg", "Trusted"),
                ("nested_attr", "Verified"),
                ("multi_attr", "Verified"),
                ("in_outer_off", "Trusted"),
            ],
        ),
    ] {
        fs::write(dir.join("verus/count.toml"), format!("{PLAIN}{keys}")).expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        assert_eq!(
            got, 0,
            "{keys:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
        );
        for (name, want) in wants {
            let got = class_in(&out, name);
            assert_eq!(got, want, "{keys:?}: {name}: got = {got}, wanted = {want}");
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

/// An inline module's directory follows its path attributes as an
/// out-of-line module's file does: under `cfg_attr(unix, path = "u")`,
/// `u/x.rs` is the one built where unix holds and `inline/x.rs` the one
/// where it does not. A crate's roots are the targets its Cargo.toml
/// declares: with `autobins = false` and a `[[bin]]` at `src/bin/tool.rs`,
/// `tool.rs` is a root whose `#[cfg(windows)] mod x;` is `src/bin/x.rs`
/// beside it, and an undeclared `src/bin/stray.rs` is no root, so is never
/// verified; with the default `autobins`, `src/bin/solo.rs` is a root.
#[test]
fn inline_paths_and_declared_targets_place_files() {
    let dir = repository("targets");
    let crates = dir.join("crates");
    for (path, text) in [
        (
            "a/src/lib.rs",
            "#[cfg_attr(unix, path = \"u\")]\nmod inline {\n    mod x;\n}\n",
        ),
        ("a/src/u/x.rs", "verus! {\npub fn in_u() {}\n}\n"),
        (
            "a/src/inline/x.rs",
            "verus! {\npub fn in_default_dir() {}\n}\n",
        ),
        (
            "b/Cargo.toml",
            "[package]\nname = \"b\"\nversion = \"0.0.0\"\nautobins = false\n\n[[bin]]\nname = \"tool\"\npath = \"src/bin/tool.rs\"\n",
        ),
        (
            "b/src/bin/tool.rs",
            "#[cfg(windows)]\nmod x;\nverus! {\npub fn in_tool() {}\n}\n",
        ),
        ("b/src/bin/x.rs", "verus! {\npub fn in_tool_child() {}\n}\n"),
        ("b/src/bin/stray.rs", "verus! {\npub fn in_stray() {}\n}\n"),
        (
            "c/Cargo.toml",
            "[package]\nname = \"c\"\nversion = \"0.0.0\"\n",
        ),
        ("c/src/main.rs", "fn main() {}\n"),
        ("c/src/bin/solo.rs", "verus! {\npub fn in_solo() {}\n}\n"),
    ] {
        let p = crates.join(path);
        fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        fs::write(p, text).expect("write");
    }
    for (keys, wants) in [
        (
            "",
            [
                ("in_u", "Trusted"),
                ("in_default_dir", "Trusted"),
                ("in_tool", "Verified"),
                ("in_tool_child", "Trusted"),
                ("in_stray", "Trusted"),
                ("in_solo", "Verified"),
            ],
        ),
        (
            "verus_sets = [\"unix\", \"windows\"]\n",
            [
                ("in_u", "Verified"),
                ("in_default_dir", "Trusted"),
                ("in_tool", "Verified"),
                ("in_tool_child", "Verified"),
                ("in_stray", "Trusted"),
                ("in_solo", "Verified"),
            ],
        ),
    ] {
        fs::write(dir.join("verus/count.toml"), format!("{PLAIN}{keys}")).expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        assert_eq!(
            got, 0,
            "{keys:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
        );
        for (name, want) in wants {
            let got = class_in(&out, name);
            assert_eq!(got, want, "{keys:?}: {name}: got = {got}, wanted = {want}");
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

/// With two runs' atoms declared for two single-valued keys (an x86_64
/// Linux run and an aarch64 macOS run), a function is sure only under one
/// of those atoms: under two, from one `cfg` or several (its own, its
/// module's), it may ship on a host neither run is (x86_64 macOS), so it
/// is trusted. Under one it is verified, and with one key declared twice
/// and the other once, the other's atom does not count. An atom under two
/// `not`s counts as one under none, a `cfg_attr(not(P), cfg(Q))` included,
/// and each alternative of an `any` is a way of its own.
#[test]
fn several_runs_atoms_together_are_not_sure() {
    const LIB: &str = r#"
#[cfg(target_os = "macos")]
mod mac {
    #[cfg(target_arch = "x86_64")]
    #[verus_verify]
    pub fn enclosing() {}
}
verus! {
#[cfg(target_os = "macos")]
#[cfg(target_arch = "x86_64")]
pub fn stacked() {}
#[cfg(all(target_arch = "x86_64", any(target_arch = "aarch64", unix)))]
pub fn nested() {}
#[cfg(target_arch = "x86_64")]
pub fn one_arch() {}
#[cfg(target_os = "macos")]
pub fn one_os() {}
}
#[cfg_attr(all(target_os = "linux", target_arch = "aarch64"), verus_verify)]
pub fn attr_form() {}
#[cfg(not(not(target_os = "macos")))]
mod mac2 {
    #[cfg(target_arch = "x86_64")]
    #[verus_verify]
    pub fn double_not() {}
}
#[cfg_attr(not(target_os = "macos"), cfg(feature = "portable"))]
mod mac3 {
    #[cfg(target_arch = "x86_64")]
    #[verus_verify]
    pub fn attr_not() {}
}
verus! {
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
pub fn either_arch() {}
}
"#;
    let dir = repository("runs");
    fs::write(dir.join("crates/a/src/lib.rs"), LIB).expect("write");
    let both = "verus_sets = ['target_arch = \"x86_64\"', 'target_arch = \"aarch64\"', 'target_os = \"linux\"', 'target_os = \"macos\"']\n";
    let arches = "verus_sets = ['target_arch = \"x86_64\"', 'target_arch = \"aarch64\"', 'target_os = \"macos\"']\n";
    for (keys, wants) in [
        (
            both,
            [
                ("enclosing", "Trusted"),
                ("stacked", "Trusted"),
                ("nested", "Trusted"),
                ("one_arch", "Verified"),
                ("one_os", "Verified"),
                ("attr_form", "Trusted"),
                ("double_not", "Trusted"),
                ("attr_not", "Trusted"),
                ("either_arch", "Verified"),
            ],
        ),
        (
            arches,
            [
                ("enclosing", "Verified"),
                ("stacked", "Verified"),
                ("nested", "Trusted"),
                ("one_arch", "Verified"),
                ("one_os", "Verified"),
                ("attr_form", "Trusted"),
                ("double_not", "Verified"),
                ("attr_not", "Verified"),
                ("either_arch", "Verified"),
            ],
        ),
    ] {
        fs::write(dir.join("verus/count.toml"), format!("{PLAIN}{keys}")).expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        assert_eq!(
            got, 0,
            "{keys:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
        );
        for (name, want) in wants {
            let got = class_in(&out, name);
            assert_eq!(got, want, "{keys:?}: {name}: got = {got}, wanted = {want}");
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

/// With both of challoc's architectures in `verus_sets`, a `cfg` that
/// needs both at once holds in no build: a function under it is left
/// out, and Verus's attribute form under it is never applied, so the
/// function is outside. The same holds when the two values come from
/// different `cfg`s that must hold together: two on the function, or one
/// on it and one on its `impl`, inline module or module file. Each
/// architecture's own code stays verified, and `not` of one stays trusted,
/// since the counter reads each atom on its own.
#[test]
fn two_values_of_one_key_are_never_verified() {
    const LIB: &str = r#"
pub fn plain() {}
verus! {
#[cfg(target_arch = "x86_64")]
pub fn x86() {}
#[cfg(all(target_arch = "x86_64", target_arch = "aarch64"))]
pub fn both() {}
#[cfg(all(target_arch = "x86_64", any(target_arch = "aarch64", kani)))]
pub fn both_nested() {}
#[cfg(not(target_arch = "x86_64"))]
pub fn not_x86() {}
}
#[cfg_attr(all(target_arch = "x86_64", target_arch = "aarch64"), verus_verify)]
pub fn both_attr() {}
#[cfg(target_arch = "x86_64")]
#[cfg(target_arch = "aarch64")]
#[verus_verify]
pub fn stacked() {}
#[cfg(target_arch = "x86_64")]
#[cfg_attr(target_arch = "aarch64", verus_verify)]
pub fn stacked_attr() {}
pub struct S;
verus! {
#[cfg(target_arch = "x86_64")]
impl S {
    #[cfg(target_arch = "aarch64")]
    pub fn in_impl(&self) {}
    pub fn x86_impl(&self) {}
}
}
#[cfg(target_arch = "x86_64")]
mod inline {
    verus! {
    #[cfg(target_arch = "aarch64")]
    pub fn in_mod() {}
    }
}
#[cfg(target_arch = "x86_64")]
mod x86;
"#;
    let dir = repository("arches");
    fs::write(dir.join("crates/a/src/lib.rs"), LIB).expect("write");
    fs::write(
        dir.join("crates/a/src/x86.rs"),
        "verus! {\n#[cfg(target_arch = \"aarch64\")]\npub fn in_file() {}\npub fn x86_file() {}\n}\n",
    )
    .expect("write");
    fs::write(
        dir.join("verus/count.toml"),
        format!("{PLAIN}verus_sets = ['target_arch = \"x86_64\"', 'target_arch = \"aarch64\"']\n"),
    )
    .expect("write");
    let (got, out, err) = count(&dir, &["--list"]);
    assert_eq!(
        got, 0,
        "got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
    );
    for (name, want) in [
        ("plain", "Outside"),
        ("x86", "Verified"),
        ("both", "missing"),
        ("both_nested", "missing"),
        ("not_x86", "Trusted"),
        ("both_attr", "Outside"),
        ("stacked", "missing"),
        ("stacked_attr", "Outside"),
        ("in_impl", "missing"),
        ("x86_impl", "Verified"),
        ("in_mod", "missing"),
        ("in_file", "missing"),
        ("x86_file", "Verified"),
    ] {
        let got = out
            .lines()
            .find(|l| l.ends_with(&format!("\t{name}")))
            .and_then(|l| l.split('\t').nth(1))
            .unwrap_or("missing");
        assert_eq!(got, want, "{name}: got = {got}, wanted = {want}");
    }
    let _ = fs::remove_dir_all(&dir);
}

/// A configuration that would settle one atom two ways, or names
/// something other than one atom, does not parse.
#[test]
fn verus_atoms_must_be_atoms_and_settled_once() {
    let dir = repository("atoms");
    for (keys, want) in [
        (
            "verus_never_sets = [\"not(miri)\"]\n",
            "[crates] verus_never_sets entry \"not(miri)\" is not one cfg atom",
        ),
        (
            "verus_sets = [\"miri\"]\nverus_never_sets = [\"miri\"]\n",
            "[crates] verus_never_sets entry \"miri\" is already in verus_sets",
        ),
        (
            "verus_sets = [\"test\"]\n",
            "[crates] verus_sets entry \"test\" is already a test atom",
        ),
        (
            "verus_never_sets = [\"verus_keep_ghost\"]\n",
            "[crates] verus_never_sets entry \"verus_keep_ghost\" is already a Verus atom",
        ),
        (
            "verus_features = [\"verus\"]\nverus_sets = ['feature = \"verus\"']\n",
            "[crates] verus_sets entry \"feature = \\\"verus\\\"\" is already a Verus atom",
        ),
    ] {
        fs::write(dir.join("verus/count.toml"), format!("{PLAIN}{keys}")).expect("write");
        let (got, out, err) = count(&dir, &[]);
        assert!(
            got == 2 && err.contains(want),
            "{keys:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 2, {want:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}
