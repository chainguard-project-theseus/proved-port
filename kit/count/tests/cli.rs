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
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n[package.metadata.verus]\nverify = true\n",
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

/// [`count`] with the environment variables `env` set.
fn count_env(root: &Path, env: &[(&str, &str)], args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_verus-count"))
        .envs(env.iter().copied())
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

/// The counter run with `args` alone, no ROOT: its exit code, standard
/// output and standard error.
fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_verus-count"))
        .args(args)
        .output()
        .expect("runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// `--version` prints the kit release (Cargo.toml's version, never the
/// 0.0.0 placeholder) and exits 0; `--help` and `-h` print the usage,
/// naming every option, and exit 0; a misspelt `--helps` still exits 2,
/// and so does `--help`, `-h` or `--version` beside a checking option or
/// a ROOT.
#[test]
fn version_and_help_exit_zero() {
    let (code, out, err) = run(&["--version"]);
    assert_eq!(code, 0, "--version: got = {code}, wanted = 0 ({err})");
    assert_eq!(out, format!("verus-count {}\n", env!("CARGO_PKG_VERSION")));
    assert_ne!(
        env!("CARGO_PKG_VERSION"),
        "0.0.0",
        "Cargo.toml's version: got = 0.0.0, wanted = the kit release"
    );
    for help in ["--help", "-h"] {
        let (code, out, err) = run(&[help]);
        assert_eq!(code, 0, "{help}: got = {code}, wanted = 0 ({err})");
        assert!(
            out.starts_with("usage: verus-count "),
            "{help}: got = {out:?}"
        );
        for option in [
            "--config",
            "--list",
            "--floor",
            "--base-rev",
            "--no-base",
            "--rlimit",
            "--badge",
            "--version",
            "--help",
        ] {
            assert!(out.contains(option), "{help}: got = no {option} in {out:?}");
        }
    }
    let (code, _, err) = run(&["--helps"]);
    assert_eq!(code, 2, "--helps: got = {code}, wanted = 2");
    assert!(
        err.contains("unknown option --helps"),
        "--helps: got = {err:?}"
    );
    // Help or version beside a check prints nothing and passes nothing:
    // a check's command line that gained one exits 2, not 0.
    for args in [
        &["--floor", "verus/FLOOR", "--base-rev", "HEAD", "-h", "."][..],
        &["--floor", "verus/FLOOR", "--no-base", "--help", "."][..],
        &["--list", "--help"][..],
        &["--version", "--floor", "verus/FLOOR", "--no-base", "."][..],
        &["--help", "."][..],
    ] {
        let (code, out, err) = run(args);
        assert_eq!(code, 2, "{args:?}: got = {code}, wanted = 2 ({out})");
        assert!(
            out.is_empty(),
            "{args:?}: got = {out:?}, wanted = nothing printed"
        );
        assert!(
            err.contains("--help and --version print and check nothing"),
            "{args:?}: got = {err:?}"
        );
    }
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

/// A head that moves its configuration with `--config` to a path the base
/// lacks is held to the one configuration file of the base's tree
/// (`verus/count.toml` or another, whatever Git's quoting): moving it with
/// every gate passes, and narrowing a gate on the way fails. A base with
/// several such files fails, naming them, even when one is
/// `verus/count.toml`; one that does not parse, or whose name is not
/// UTF-8, fails with exit 2. A base with none passes, unless it has
/// a FLOOR (at the head's `[floor] path` or `verus/FLOOR`), whose gates
/// came from a configuration the run cannot find: that exits 1, with
/// `--config` or without. A configuration outside ROOT, which the base
/// cannot hold at its path, is compared with the base's
/// `verus/count.toml`, or with the base's one configuration file (several
/// fail, as above), or, with none, with nothing even beside a FLOOR.
#[test]
fn a_moved_configuration_is_held_to_the_base_one() {
    let dir = repository("moved");
    let commit = |tag: &str, rm: &[&str], add: &[(&str, &str)]| {
        for f in rm {
            fs::remove_file(dir.join(f)).expect("rm");
        }
        for (f, text) in add {
            fs::write(dir.join(f), text).expect("write");
        }
        git(&dir, &["add", "-A", "verus"]);
        git(&dir, &["commit", "-q", "-m", tag]);
        git(&dir, &["tag", tag]);
    };
    commit(
        "floored",
        &["verus/count.toml"],
        &[("verus/FLOOR", "functions 0.0\n")],
    );
    commit(
        "floored2",
        &["verus/FLOOR"],
        &[("verus/F2", "functions 0.0\n")],
    );
    commit("elsewhere", &["verus/F2"], &[("verus/other.toml", GATED)]);
    commit("twice", &[], &[("verus/third.toml", PLAIN)]);
    // A name Git quotes in its output unless asked for it verbatim.
    commit(
        "quoted",
        &["verus/other.toml", "verus/third.toml"],
        &[("verus/café FLOOR.toml", GATED)],
    );
    // A base that ran verus/a.toml and kept an older verus/count.toml.
    commit(
        "stale",
        &["verus/café FLOOR.toml"],
        &[("verus/count.toml", PLAIN), ("verus/a.toml", GATED)],
    );
    // A base configuration with a key this release does not read.
    let unknown = format!("dropped_key = 1\n{GATED}");
    commit(
        "unknown",
        &["verus/count.toml", "verus/a.toml"],
        &[("verus/other.toml", unknown.as_str())],
    );
    // A base configuration that is not TOML: a `[crates]` table twice,
    // with comments on the headers.
    commit(
        "malformed",
        &["verus/other.toml"],
        &[(
            "verus/m.toml",
            "[crates] # one\ndir = \"crates\"\n[crates] # two\ndir = \"crates\"\n",
        )],
    );
    let outside =
        std::env::temp_dir().join(format!("verus-count-outside-{}.toml", std::process::id()));
    let outside = outside.to_str().expect("utf-8").to_owned();
    let moved = dir.join("verus/count2.toml");
    let moved = moved.to_str().expect("utf-8").to_owned();
    let default = dir.join("verus/count.toml");
    let default = default.to_str().expect("utf-8").to_owned();
    let f2 = format!("{PLAIN}[floor]\npath = \"verus/F2\"\n");
    // A row with no exit code passes the configuration check and fails
    // later, on GATED's [reach] file, which this repository lacks. A row
    // whose file is the default one runs without --config.
    for (file, text, base, code, want) in [
        (
            &moved,
            PLAIN,
            "gated",
            Some(1),
            "FAIL configuration: verus/count2.toml: the [mints] section is gone, so its gate no longer runs, where gated:verus/count.toml runs it",
        ),
        (
            &moved,
            PLAIN,
            "gated",
            Some(1),
            "ok configuration: gated has no verus/count2.toml, so verus/count2.toml is held to gated:verus/count.toml, the one configuration gated has",
        ),
        (
            &moved,
            GATED,
            "gated",
            None,
            "ok configuration: verus/count2.toml keeps every gate gated:verus/count.toml runs",
        ),
        (
            &moved,
            PLAIN,
            "none",
            Some(0),
            "ok configuration: none has no verus/count2.toml or other configuration to compare with",
        ),
        (
            &moved,
            PLAIN,
            "floored",
            Some(1),
            "FAIL configuration: floored has verus/FLOOR but no configuration",
        ),
        (
            &moved,
            &f2,
            "floored",
            Some(1),
            "FAIL configuration: floored has verus/FLOOR but no configuration",
        ),
        (
            &moved,
            &f2,
            "floored2",
            Some(1),
            "FAIL configuration: floored2 has verus/F2 but no configuration",
        ),
        (
            &moved,
            PLAIN,
            "elsewhere",
            Some(1),
            "FAIL configuration: verus/count2.toml: the [mints] section is gone, so its gate no longer runs, where elsewhere:verus/other.toml runs it",
        ),
        (
            &moved,
            PLAIN,
            "elsewhere",
            Some(1),
            "ok configuration: elsewhere has no verus/count2.toml, so verus/count2.toml is held to elsewhere:verus/other.toml, the one configuration elsewhere has",
        ),
        (
            &moved,
            GATED,
            "elsewhere",
            None,
            "ok configuration: verus/count2.toml keeps every gate elsewhere:verus/other.toml runs",
        ),
        (
            &moved,
            PLAIN,
            "twice",
            Some(1),
            "FAIL configuration: twice has no verus/count2.toml, and 2 configuration files (verus/other.toml, verus/third.toml)",
        ),
        (
            &moved,
            PLAIN,
            "stale",
            Some(1),
            "FAIL configuration: stale has no verus/count2.toml, and 2 configuration files (verus/a.toml, verus/count.toml)",
        ),
        (
            &moved,
            PLAIN,
            "unknown",
            Some(2),
            "FAIL configuration: unknown:verus/other.toml: ",
        ),
        (
            &moved,
            PLAIN,
            "malformed",
            Some(2),
            "FAIL configuration: malformed:verus/m.toml: ",
        ),
        (
            &moved,
            PLAIN,
            "quoted",
            Some(1),
            "FAIL configuration: verus/count2.toml: the [mints] section is gone, so its gate no longer runs, where quoted:verus/café FLOOR.toml runs it",
        ),
        (
            &default,
            PLAIN,
            "floored",
            Some(1),
            "FAIL configuration: floored has verus/FLOOR but no configuration",
        ),
        (
            &outside,
            PLAIN,
            "floored",
            Some(0),
            "ok configuration: floored has no verus/count.toml to compare with",
        ),
        (
            &outside,
            PLAIN,
            "elsewhere",
            Some(1),
            "ok configuration: elsewhere has no verus/count.toml, so the configuration outside ROOT is held to elsewhere:verus/other.toml, the one configuration elsewhere has",
        ),
        (
            &outside,
            PLAIN,
            "elsewhere",
            Some(1),
            "the [mints] section is gone, so its gate no longer runs, where elsewhere:verus/other.toml runs it",
        ),
        (
            &outside,
            PLAIN,
            "twice",
            Some(1),
            "FAIL configuration: twice has no verus/count.toml, and 2 configuration files (verus/other.toml, verus/third.toml)",
        ),
    ] {
        fs::write(file, text).expect("write");
        let mut args = vec!["--base-rev", base];
        if *file != default {
            args.extend(["--config", file.as_str()]);
        }
        let (got, out, err) = count(&dir, &args);
        let exit = code.map_or(!err.contains("FAIL configuration"), |c| got == c);
        assert!(
            exit && (out.contains(want) || err.contains(want)),
            "{base} with {file} {text:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit {code:?}, {want:?}"
        );
        if *file == default {
            fs::remove_file(file).expect("rm");
        }
    }
    // Git's pathspec variables do not reach the search (#117): under each,
    // the base's verus/other.toml is still found, and the dropped
    // `[mints]` still fails. GIT_ICASE_PATHSPECS, which this base cannot
    // show, has a test of its own below.
    fs::write(&moved, PLAIN).expect("write");
    for var in [
        "GIT_GLOB_PATHSPECS",
        "GIT_LITERAL_PATHSPECS",
        "GIT_NOGLOB_PATHSPECS",
    ] {
        let (got, out, err) = count_env(
            &dir,
            &[(var, "1")],
            &["--base-rev", "elsewhere", "--config", moved.as_str()],
        );
        let want = "where elsewhere:verus/other.toml runs it";
        assert!(
            got == 1 && err.contains(want),
            "{var}=1: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
        );
    }
    // A base configuration whose name is not UTF-8 cannot be read back,
    // so the run fails (exit 2) rather than pass it over.
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::OsStr::from_bytes(b"verus/\xff.toml");
        fs::remove_file(dir.join("verus/m.toml")).expect("rm");
        fs::remove_file(&moved).expect("rm");
        fs::write(dir.join(name), GATED).expect("write");
        git(&dir, &["add", "-A", "verus"]);
        git(&dir, &["commit", "-q", "-m", "bytes"]);
        git(&dir, &["tag", "bytes"]);
        fs::write(&moved, PLAIN).expect("write");
        let (got, out, err) = count(&dir, &["--base-rev", "bytes", "--config", &moved]);
        let want = "is not a UTF-8 path under the commit";
        assert!(
            got == 2 && err.contains(want),
            "bytes: got = exit {got}, {out:?}, {err:?}; wanted = exit 2, {want:?}"
        );
    }
    let _ = fs::remove_file(&outside);
    let _ = fs::remove_dir_all(&dir);
}

/// GIT_ICASE_PATHSPECS does not reach the search (#117): a base whose one
/// configuration is `verus/cargo.toml`, which only a case-folded match
/// takes for a `Cargo.toml` and excludes, is still found under it, so its
/// dropped `[mints]` fails, as it does with no variable set.
#[test]
fn a_case_folded_pathspec_does_not_hide_a_base_configuration() {
    let dir = repository("icase");
    fs::remove_file(dir.join("verus/count.toml")).expect("rm");
    fs::write(dir.join("verus/cargo.toml"), GATED).expect("write");
    git(&dir, &["add", "-A", "verus"]);
    git(&dir, &["commit", "-q", "-m", "lower"]);
    git(&dir, &["tag", "lower"]);
    let moved = dir.join("verus/count2.toml");
    fs::write(&moved, PLAIN).expect("write");
    let moved = moved.to_str().expect("utf-8");
    let icase: &[(&str, &str)] = &[("GIT_ICASE_PATHSPECS", "1")];
    for env in [&[][..], icase] {
        let (got, out, err) = count_env(&dir, env, &["--base-rev", "lower", "--config", moved]);
        let want = "where lower:verus/cargo.toml runs it";
        assert!(
            got == 1 && err.contains(want),
            "{env:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// A base configuration whose blob cannot be read fails the run (exit 2)
/// rather than be passed over: the only configuration of the base, an
/// unreadable gated one beside a readable stale one (where `git grep`
/// exits 0 or 1 and reports the blob only on stderr), and the base's copy
/// at the head's own path (whose tree entry exists, so it is not read as
/// absent).
#[test]
fn an_unreadable_base_configuration_fails_closed() {
    let dir = repository("unreadable");
    let commit = |tag: &str, rm: &[&str], add: &[(&str, String)]| {
        for f in rm {
            fs::remove_file(dir.join(f)).expect("rm");
        }
        for (f, text) in add {
            fs::write(dir.join(f), text).expect("write");
        }
        git(&dir, &["add", "-A", "verus"]);
        git(&dir, &["commit", "-q", "-m", tag]);
        git(&dir, &["tag", tag]);
    };
    // Each configuration is a blob of its own, so deleting it touches no
    // other commit's.
    commit(
        "lone",
        &["verus/count.toml"],
        &[("verus/a.toml", format!("{GATED}# lone\n"))],
    );
    commit(
        "beside",
        &["verus/a.toml"],
        &[
            ("verus/b.toml", format!("{GATED}# beside\n")),
            ("verus/count.toml", format!("{PLAIN}# stale\n")),
        ],
    );
    commit(
        "headpath",
        &["verus/b.toml"],
        &[("verus/count.toml", format!("{GATED}# headpath\n"))],
    );
    for object in [
        "lone:verus/a.toml",
        "beside:verus/b.toml",
        "headpath:verus/count.toml",
    ] {
        let out = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["rev-parse", object])
            .output()
            .expect("runs");
        let oid = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        assert!(out.status.success() && oid.len() == 40, "{object}: {oid:?}");
        fs::remove_file(dir.join(".git/objects").join(&oid[..2]).join(&oid[2..]))
            .expect("rm object");
    }
    let moved = dir.join("verus/count2.toml");
    fs::write(&moved, PLAIN).expect("write");
    let moved = moved.to_str().expect("utf-8").to_owned();
    fs::write(dir.join("verus/count.toml"), PLAIN).expect("write");
    for (base, config, want) in [
        (
            "lone",
            Some(moved.as_str()),
            "FAIL configuration: git grep crates ",
        ),
        (
            "beside",
            Some(moved.as_str()),
            "FAIL configuration: git grep crates ",
        ),
        ("headpath", None, "FAIL configuration: git show "),
    ] {
        let mut args = vec!["--base-rev", base];
        args.extend(config.iter().flat_map(|c| ["--config", c]));
        let (got, out, err) = count(&dir, &args);
        assert!(
            got == 2 && err.contains(want),
            "{base}: got = exit {got}, {out:?}, {err:?}; wanted = exit 2, {want:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// A ROOT below the repository's top level: `git grep` there lists the
/// base's configuration relative to ROOT, and the base reads find it
/// there (#114), so a moved configuration is held to it.
#[test]
fn a_moved_configuration_below_the_top_level_is_held_to_the_base_one() {
    let top = std::env::temp_dir().join(format!("verus-count-cli-sub-{}", std::process::id()));
    let _ = fs::remove_dir_all(&top);
    let dir = top.join("sub");
    fs::create_dir_all(dir.join("crates/a/src")).expect("mkdir");
    fs::create_dir_all(dir.join("verus")).expect("mkdir");
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n[package.metadata.verus]\nverify = true\n",
    )
    .expect("write");
    fs::write(dir.join("crates/a/src/lib.rs"), "pub fn f() {}\n").expect("write");
    fs::write(dir.join("verus/a.toml"), GATED).expect("write");
    git(&top, &["init", "-q"]);
    git(&top, &["add", "sub"]);
    git(&top, &["commit", "-q", "-m", "base"]);
    git(&top, &["tag", "base"]);
    let moved = dir.join("verus/count2.toml");
    fs::write(&moved, PLAIN).expect("write");
    let (got, out, err) = count(
        &dir,
        &[
            "--base-rev",
            "base",
            "--config",
            moved.to_str().expect("utf-8"),
        ],
    );
    let want = "FAIL configuration: verus/count2.toml: the [mints] section is gone, so its gate no longer runs, where base:verus/a.toml runs it";
    assert!(
        got == 1 && err.contains(want),
        "got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
    );
    let _ = fs::remove_dir_all(&top);
}

/// A ROOT below the repository's top level reads the base at ROOT's
/// paths: a gate the base's verus/count.toml there runs, dropped at the
/// head, fails the run, and an unchanged configuration passes. Before
/// the base reads resolved from ROOT, both read nothing and passed.
#[test]
fn a_root_below_the_top_level_reads_its_base() {
    let top = std::env::temp_dir().join(format!("verus-count-cli-subroot-{}", std::process::id()));
    let _ = fs::remove_dir_all(&top);
    let dir = top.join("sub");
    fs::create_dir_all(dir.join("crates/a/src")).expect("mkdir");
    fs::create_dir_all(dir.join("verus")).expect("mkdir");
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n[package.metadata.verus]\nverify = true\n",
    )
    .expect("write");
    fs::write(dir.join("crates/a/src/lib.rs"), "pub fn f() {}\n").expect("write");
    fs::write(dir.join("verus/count.toml"), GATED).expect("write");
    git(&top, &["init", "-q"]);
    git(&top, &["add", "sub"]);
    git(&top, &["commit", "-q", "-m", "gated"]);
    git(&top, &["tag", "gated"]);
    fs::write(dir.join("verus/count.toml"), PLAIN).expect("write");
    let (got, out, err) = count(&dir, &["--base-rev", "gated"]);
    let want = "FAIL configuration: verus/count.toml: the [mints] section is gone, so its gate no longer runs, where gated:verus/count.toml runs it";
    assert!(
        got == 1 && err.contains(want),
        "dropped gate: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
    );
    git(&top, &["commit", "-q", "-a", "-m", "plain"]);
    git(&top, &["tag", "plain"]);
    let (got, out, err) = count(&dir, &["--base-rev", "plain"]);
    let want = "ok configuration: verus/count.toml keeps every gate plain:verus/count.toml runs";
    assert!(
        got == 0 && out.contains(want),
        "unchanged: got = exit {got}, {out:?}, {err:?}; wanted = exit 0, {want:?}"
    );
    let _ = fs::remove_dir_all(&top);
}

/// A repository whose port is in `sub/`: its commit tagged `base` holds
/// `files` (path under `sub/`, bytes), each a path Git may quote. Returns
/// the top level and ROOT (`sub/`).
fn sub_repository(name: &str, files: &[(&[u8], &str)]) -> (PathBuf, PathBuf) {
    use std::os::unix::ffi::OsStrExt;
    let top =
        std::env::temp_dir().join(format!("verus-count-cli-sub-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&top);
    let dir = top.join("sub");
    for (path, text) in files {
        let p = dir.join(std::ffi::OsStr::from_bytes(path));
        fs::create_dir_all(p.parent().expect("a parent")).expect("mkdir");
        fs::write(p, text).expect("write");
    }
    git(&top, &["init", "-q"]);
    git(&top, &["add", "-A"]);
    git(&top, &["commit", "-q", "-m", "base"]);
    git(&top, &["tag", "base"]);
    (top, dir)
}

/// Every base read resolves from a ROOT below the top level (#114): the
/// crates of a base `dir` (a crate the head deletes is a narrowing), the
/// base's FLOOR (a lowered figure fails), the base crate's sources (a
/// raised `rlimit` fails) and its workspace manifest (an inherited edition
/// reads the same on both sides), and the base sources' names, verbatim
/// (one Git quotes reads back; one that is not UTF-8 fails, exit 2).
#[test]
#[cfg(unix)]
fn a_root_below_the_top_level_reads_every_base_file() {
    let manifest = |name: &str| {
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\n[package.metadata.verus]\nverify = true\n"
        )
    };
    let check = |what: &str, got: i32, out: &str, err: &str, code: i32, want: &str| {
        assert!(
            got == code && (out.contains(want) || err.contains(want)),
            "{what}: got = exit {got}, {out:?}, {err:?}; wanted = exit {code}, {want:?}"
        );
    };
    // A crate of `dir` the head deletes.
    let (ma, mb) = (manifest("a"), manifest("b"));
    let (top, dir) = sub_repository(
        "deleted",
        &[
            (b"verus/count.toml", PLAIN),
            (b"crates/a/Cargo.toml", &ma),
            (b"crates/a/src/lib.rs", "pub fn f() {}\n"),
            (b"crates/b/Cargo.toml", &mb),
            (b"crates/b/src/lib.rs", "pub fn g() {}\n"),
        ],
    );
    fs::remove_dir_all(dir.join("crates/b")).expect("rm");
    let (got, out, err) = count(&dir, &["--base-rev", "base"]);
    check(
        "deleted crate",
        got,
        &out,
        &err,
        1,
        "[crates] crate \"b\" at \"crates/b\" is no longer read",
    );
    let _ = fs::remove_dir_all(&top);
    // A lowered FLOOR, and a limit raised in the base crate's source, in a
    // file Git quotes beside the one it reads.
    let split =
        |attr: &str| format!("use vstd::prelude::*;\nverus! {{\n{attr} pub fn f() {{}}\n}}\n");
    let lib = "mod split;\npub fn o() {}\n";
    let (top, dir) = sub_repository(
        "ratchets",
        &[
            (b"verus/count.toml", RATCHET_CONFIG),
            (b"verus/FLOOR", RATCHET_FLOOR),
            (b"verus/RLIMIT", RATCHET_RLIMIT),
            (b"crates/a/Cargo.toml", &ma),
            (b"crates/a/src/lib.rs", lib),
            (b"crates/a/src/split.rs", &split("#[verifier::rlimit(5)]")),
            ("crates/a/src/café.rs".as_bytes(), "// No functions.\n"),
        ],
    );
    let stats = top.join("stats.json");
    fs::write(&stats, stats_naming(&[("a::split::f", 1_000_000)])).expect("write");
    let args = |floor: &Path| {
        vec![
            "--floor".to_owned(),
            floor.to_string_lossy().into_owned(),
            "--rlimit".to_owned(),
            stats.to_string_lossy().into_owned(),
            "--base-rev".to_owned(),
            "base".to_owned(),
        ]
    };
    let run_at = |a: Vec<String>| {
        let a: Vec<&str> = a.iter().map(String::as_str).collect();
        count(&dir, &a)
    };
    let (got, out, err) = run_at(args(&dir.join("verus/FLOOR")));
    check(
        "as committed",
        got,
        &out,
        &err,
        0,
        "ok rlimit headroom: 1 functions",
    );
    fs::write(
        dir.join("verus/FLOOR"),
        RATCHET_FLOOR.replace("functions 50.0\nlines", "functions 40.0\nlines"),
    )
    .expect("write");
    let (got, out, err) = run_at(args(&dir.join("verus/FLOOR")));
    check(
        "lowered floor",
        got,
        &out,
        &err,
        1,
        "lowers functions from 50.0% to 40.0%",
    );
    fs::write(dir.join("verus/FLOOR"), RATCHET_FLOOR).expect("write");
    fs::write(dir.join("crates/a/src/split.rs"), split("")).expect("write");
    let (got, out, err) = run_at(args(&dir.join("verus/FLOOR")));
    check(
        "raised rlimit",
        got,
        &out,
        &err,
        1,
        "raises a::split::f's rlimit from 5 to 10",
    );
    let _ = fs::remove_dir_all(&top);
    // A base source whose name is not UTF-8 cannot be read back.
    let (top, dir) = sub_repository(
        "bytes",
        &[
            (b"verus/count.toml", RATCHET_CONFIG),
            (b"verus/FLOOR", RATCHET_FLOOR),
            (b"verus/RLIMIT", RATCHET_RLIMIT),
            (b"crates/a/Cargo.toml", &ma),
            (b"crates/a/src/lib.rs", lib),
            (b"crates/a/src/split.rs", &split("#[verifier::rlimit(5)]")),
            (b"crates/a/src/\xff.rs", "// No functions.\n"),
        ],
    );
    let stats = top.join("stats.json");
    fs::write(&stats, stats_naming(&[("a::split::f", 1_000_000)])).expect("write");
    let s = stats.to_string_lossy().into_owned();
    let floor = dir.join("verus/FLOOR").to_string_lossy().into_owned();
    let (got, out, err) = count(
        &dir,
        &["--floor", &floor, "--rlimit", &s, "--base-rev", "base"],
    );
    check("not UTF-8", got, &out, &err, 2, "is not a UTF-8 path");
    let _ = fs::remove_dir_all(&top);
    // The edition a crate inherits from the workspace at the base.
    let (top, dir) = sub_repository(
        "workspace",
        &[
            (
                b"Cargo.toml",
                "[workspace]\nmembers = [\"crates/a\"]\n[workspace.package]\nedition = \"2024\"\n",
            ),
            (
                b"crates/a/Cargo.toml",
                "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition.workspace = true\n[package.metadata.verus]\nverify = true\n",
            ),
            (
                b"crates/a/src/lib.rs",
                "::vstd::prelude::verus! {\n#[verifier::rlimit(10)]\npub fn f() {}\n}\n",
            ),
            (
                b"verus/count.toml",
                &format!("{PLAIN}[rlimit]\npath = \"verus/RLIMIT\"\n"),
            ),
            (b"verus/RLIMIT", "max-share 65\n"),
        ],
    );
    let stats = top.join("stats.json");
    fs::write(&stats, ratchet_stats(3_000_000).replace("a::v", "a::f")).expect("write");
    let s = stats.to_string_lossy().into_owned();
    let (got, out, err) = count(&dir, &["--rlimit", &s, "--base-rev", "base"]);
    check("workspace", got, &out, &err, 0, "ok rlimit headroom");
    let _ = fs::remove_dir_all(&top);
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
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n[package.metadata.verus]\nverify = true\n",
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
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n[package.metadata.verus]\nverify = true\n",
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

/// The badge line kit/templates/PORT-README.md starts a port with.
const ZERO_BADGE: &str = "[![Verus: 0.0% of functions, 0.0% of lines](https://img.shields.io/badge/Verus-0.0%25%20of%20functions%2C%200.0%25%20of%20lines-blue)](verus/README.md#coverage)";

/// A new port's first commit, before any proof: `make verus`'s counter
/// command (`--floor --rlimit STATS`) passes on FLOOR 0.0, the 0.0 badge
/// and Verus statistics with nothing verified (empty, as cargo verus
/// writes them for a port with no `verus!` code, or with no module
/// times), for a crate with one
/// function and for one with none (every share `-`), with `--no-base`,
/// with a base that has the same FLOOR, and with a base from before
/// verus/ (the change that adds the first FLOOR). A figure above 0.0
/// fails, so the pass is the floor's, not a check that ran nothing.
#[test]
fn a_port_with_no_proofs_passes_at_floor_zero() {
    for (name, lib) in [("one", "pub fn f() {}\n"), ("none", "")] {
        let dir = std::env::temp_dir().join(format!(
            "verus-count-cli-zero-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("crates/a/src")).expect("mkdir");
        fs::create_dir_all(dir.join("verus")).expect("mkdir");
        fs::write(
            dir.join("crates/a/Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"0.0.0\"\n",
        )
        .expect("write");
        fs::write(dir.join("crates/a/src/lib.rs"), lib).expect("write");
        fs::write(dir.join("README.md"), "# a\n").expect("write");
        git(&dir, &["init", "-q"]);
        git(&dir, &["add", "crates", "README.md"]);
        git(&dir, &["commit", "-q", "-m", "before verus"]);
        git(&dir, &["tag", "before"]);
        fs::write(
            dir.join("verus/count.toml"),
            format!("{PLAIN}[floor]\npath = \"verus/FLOOR\"\n[rlimit]\npath = \"verus/RLIMIT\"\n[badge]\npath = \"README.md\"\n"),
        )
        .expect("write");
        fs::write(
            dir.join("verus/FLOOR"),
            "functions 0.0\nlines 0.0\nchecked-functions 0.0\nchecked-lines 0.0\n",
        )
        .expect("write");
        fs::write(dir.join("verus/RLIMIT"), "max-share 65\n").expect("write");
        fs::write(dir.join("README.md"), format!("# a\n{ZERO_BADGE}\n")).expect("write");
        // `cargo verus focus --output-json` (Verus 0.2026.10.18) writes
        // nothing when no crate opts into verification, as on a port with
        // no `verus!` code; a crate that opts in with nothing to verify
        // writes the statistics below.
        fs::write(dir.join("stats-empty.json"), "").expect("write");
        fs::write(
            dir.join("stats.json"),
            r#"{"times-ms":{"smt":{"smt-run-module-times":[]}},"verification-results":{"success":true,"verified":0,"errors":0}}"#,
        )
        .expect("write");
        git(&dir, &["add", "verus", "README.md"]);
        git(&dir, &["commit", "-q", "-m", "floor 0.0"]);
        let floor = dir.join("verus/FLOOR").to_string_lossy().into_owned();
        let stats = dir.join("stats.json").to_string_lossy().into_owned();
        let empty = dir.join("stats-empty.json").to_string_lossy().into_owned();
        let run = ["--floor", floor.as_str(), "--rlimit", stats.as_str()];
        for statistics in [stats.as_str(), empty.as_str()] {
            for base in [
                &["--no-base"][..],
                &["--base-rev", "HEAD"],
                &["--base-rev", "before"],
            ] {
                let args = [
                    &["--floor", floor.as_str(), "--rlimit", statistics][..],
                    base,
                ]
                .concat();
                let (got, out, err) = count(&dir, &args);
                for want in [
                    "ok coverage floor: functions verified 0.0%, at the floor",
                    "ok coverage floor: checked-lines verified or discharged 0.0%, at the floor",
                    "ok badge: README.md:2 gives verus/FLOOR's functions and lines figures",
                    "ok rlimit headroom: 0 functions",
                ] {
                    assert!(
                        got == 0 && out.contains(want),
                        "{name} {statistics} {base:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0, {want:?}"
                    );
                }
            }
        }
        fs::write(
            dir.join("verus/FLOOR"),
            "functions 0.1\nlines 0.0\nchecked-functions 0.0\nchecked-lines 0.0\n",
        )
        .expect("write");
        let (got, out, err) = count(&dir, &[&run[..], &["--no-base"]].concat());
        assert!(
            got == 1 && out.contains("FAIL coverage floor: functions"),
            "{name} at functions 0.1: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, a functions FAIL"
        );
        let _ = fs::remove_dir_all(&dir);
    }
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

/// A crate's edition sets how its macros match: in edition 2021 an
/// `$e:expr` begins at neither `_` nor `const`, so `m!(_)` takes the
/// fallback and its function counts; in 2024 it takes the first rule. The
/// edition is the library's (`[lib]`'s own, else the package's), never a
/// test's, bench's or example's.
/// A `thread_local!` not shown to be std's is read as any other macro of
/// its name (#68): the crate's own `macro_rules! thread_local` is
/// expanded, so the function it writes is counted, and another crate's,
/// by its path or by a `use`, counts as one Outside `thread_local!`
/// function. std's, by `::std::` or by the bare name where nothing else
/// may bind it, writes statics alone and adds no row.
#[test]
fn a_thread_local_not_std_s_is_counted() {
    for (i, (lib, want)) in [
        (
            "macro_rules! thread_local { () => { pub fn made() {} }; }\nthread_local!();\n",
            vec![("Outside", "crates/a/src/lib.rs:2", "made")],
        ),
        (
            "elsewhere::thread_local! {}\n",
            vec![("Outside", "crates/a/src/lib.rs:1", "thread_local!")],
        ),
        (
            "use elsewhere::thread_local;\nthread_local! { static A: u8 = 1; }\n",
            vec![("Outside", "crates/a/src/lib.rs:2", "thread_local!")],
        ),
        (
            "::std::thread_local! { static A: u8 = 1; }\nthread_local! { static B: u8 = 2; }\n",
            vec![],
        ),
        // A bare one only a `*` may bind to another macro is std's in
        // nearly every port (beside `vstd::prelude::*`): it counts
        // nothing, as before, though its module's globs place no type.
        (
            "use vstd::prelude::*;\nthread_local! { static C: u8 = 3; }\n",
            vec![],
        ),
        ("std::thread_local! { static D: u8 = 4; }\n", vec![]),
        // std's imported by name: the `use` binds the bare name, so it is
        // not shown to be std's, but its path is std's, so it counts
        // nothing.
        (
            "use std::thread_local;\nthread_local! { static E: u8 = 5; }\n",
            vec![],
        ),
        // A module of the crate named `core` shadows the extern prelude's:
        // `core::thread_local!` is the crate's own macro, expanded.
        (
            "mod core {\n    macro_rules! thread_local { () => { pub fn made() {} }; }\n    pub(crate) use thread_local;\n}\ncore::thread_local!();\n",
            vec![("Outside", "crates/a/src/lib.rs:5", "made")],
        ),
        // With a Cargo dependency renamed to `std`, `std::` and a `use
        // std::thread_local;` may name another crate's macro: each counts.
        (
            "RENAMED\nstd::thread_local! { static F: u8 = 6; }\n",
            vec![("Outside", "crates/a/src/lib.rs:1", "thread_local!")],
        ),
        (
            "RENAMED\nuse std::thread_local;\nthread_local! { static G: u8 = 7; }\n",
            vec![("Outside", "crates/a/src/lib.rs:2", "thread_local!")],
        ),
        // A bare one beside a `*` or a `#[macro_use]` that may bring
        // another crate's `thread_local` counts as one (#122): a glob of
        // another crate, of a module of this crate that imports the name or
        // globs another crate in turn, or a `#[macro_use]` of a crate other
        // than std's or Verus's. std's, core's and Verus's globs, a glob of
        // a module that brings nothing in, and `#[macro_use]` of std or
        // vstd do not.
        (
            "use elsewhere::*;\nthread_local! { static H: u8 = 8; }\n",
            vec![("Outside", "crates/a/src/lib.rs:2", "thread_local!")],
        ),
        (
            "mod m {\n    pub(crate) use elsewhere::*;\n}\nuse m::*;\nthread_local! { static J: u8 = 10; }\n",
            vec![("Outside", "crates/a/src/lib.rs:5", "thread_local!")],
        ),
        (
            "#[macro_use]\nextern crate elsewhere;\nthread_local! { static K: u8 = 11; }\n",
            vec![("Outside", "crates/a/src/lib.rs:3", "thread_local!")],
        ),
        (
            "use std::io::prelude::*;\nuse core::cell::*;\nuse verus_builtin_macros::*;\nthread_local! { static L: u8 = 12; }\n",
            vec![],
        ),
        (
            "mod m {\n    pub(crate) use std::cell::*;\n    pub fn f() {}\n}\nuse m::*;\nmod n {\n    use super::*;\n    thread_local! { static M: u8 = 13; }\n}\n",
            vec![("Outside", "crates/a/src/lib.rs:3", "f")],
        ),
        (
            "#[macro_use]\nextern crate std;\n#[macro_use]\nextern crate vstd;\nthread_local! { static N: u8 = 14; }\n",
            vec![],
        ),
        // A glob only tests build is not in a shipped build.
        (
            "#[cfg(test)]\nuse elsewhere::*;\nthread_local! { static O: u8 = 15; }\n",
            vec![],
        ),
        // From 2018 on, a glob's plain first segment in a module other
        // than the root names a module in scope there or a crate, not the
        // root's module of that name: `elsewhere` here is the crate.
        (
            "mod elsewhere {}\nmod n {\n    use elsewhere::*;\n    thread_local! { static P: u8 = 16; }\n}\n",
            vec![("Outside", "crates/a/src/lib.rs:4", "thread_local!")],
        ),
        (
            "mod n {\n    mod elsewhere {}\n    use elsewhere::*;\n    thread_local! { static Q: u8 = 17; }\n}\n",
            vec![],
        ),
        // A name a `use` rebinds, or a Cargo dependency renamed to Verus's
        // crate, is not that crate.
        (
            "use elsewhere as std;\nuse std::*;\nthread_local! { static R: u8 = 18; }\n",
            vec![("Outside", "crates/a/src/lib.rs:3", "thread_local!")],
        ),
        (
            "RENAMED vstd\nuse vstd::prelude::*;\nthread_local! { static S: u8 = 19; }\n",
            vec![("Outside", "crates/a/src/lib.rs:2", "thread_local!")],
        ),
        (
            "RENAMED vstd\n#[macro_use]\nextern crate vstd;\nthread_local! { static T: u8 = 20; }\n",
            vec![("Outside", "crates/a/src/lib.rs:3", "thread_local!")],
        ),
        (
            "extern crate elsewhere as core;\nuse core::*;\nthread_local! { static Y: u8 = 25; }\n",
            vec![("Outside", "crates/a/src/lib.rs:3", "thread_local!")],
        ),
        // A module under a `cfg` the counter cannot settle may be left
        // out, and the glob then names the crate.
        (
            "#[cfg(feature = \"stub\")]\nmod elsewhere {}\nuse elsewhere::*;\nthread_local! { static AH: u8 = 34; }\n",
            vec![("Outside", "crates/a/src/lib.rs:4", "thread_local!")],
        ),
        // So may one inside it, or one a `cfg`-gated `verus!` block
        // declares.
        (
            "#[cfg(feature = \"stub\")]\nmod e {\n    pub mod inner {}\n}\nuse e::inner::*;\nthread_local! { static AI: u8 = 35; }\n",
            vec![("Outside", "crates/a/src/lib.rs:6", "thread_local!")],
        ),
        (
            "#[cfg(feature = \"stub\")]\nverus! {\n    mod elsewhere {}\n}\nuse elsewhere::*;\nthread_local! { static AJ: u8 = 36; }\n",
            vec![("Outside", "crates/a/src/lib.rs:6", "thread_local!")],
        ),
        // A `cfg` may select a module of that name or the alias: either
        // may be what the glob names.
        (
            "#[cfg(feature = \"local\")]\nmod core {}\n#[cfg(not(feature = \"local\"))]\nextern crate elsewhere as core;\nuse core::*;\nthread_local! { static AB: u8 = 28; }\n",
            vec![("Outside", "crates/a/src/lib.rs:6", "thread_local!")],
        ),
        // From 2018 on a leading `::` names a crate, never a module in
        // scope; `::std::` is still std's.
        (
            "mod n {\n    mod elsewhere {}\n    use ::elsewhere::*;\n    thread_local! { static X: u8 = 24; }\n}\n",
            vec![("Outside", "crates/a/src/lib.rs:4", "thread_local!")],
        ),
        (
            "use ::std::cell::*;\nthread_local! { static Z: u8 = 26; }\n",
            vec![],
        ),
        (
            "RENAMED verus_builtin\n#[macro_use]\nextern crate verus_builtin;\nthread_local! { static W: u8 = 23; }\n",
            vec![("Outside", "crates/a/src/lib.rs:3", "thread_local!")],
        ),
        // A `std` or `core` that a `use` or an `extern crate .. as` gives
        // another crate names that crate in a named import and in a path
        // (#137).
        (
            "extern crate elsewhere as core;\nuse core::thread_local;\nthread_local! { static AT: u8 = 46; }\n",
            vec![("Outside", "crates/a/src/lib.rs:3", "thread_local!")],
        ),
        (
            "use elsewhere as core;\ncore::thread_local! { static AU: u8 = 47; }\n",
            vec![("Outside", "crates/a/src/lib.rs:2", "thread_local!")],
        ),
        (
            "extern crate elsewhere as std;\nmod n {\n    std::thread_local! { static AV: u8 = 48; }\n}\n",
            vec![("Outside", "crates/a/src/lib.rs:3", "thread_local!")],
        ),
        (
            "mod n {\n    use elsewhere as core;\n    use core::thread_local;\n    thread_local! { static AW: u8 = 49; }\n}\n",
            vec![("Outside", "crates/a/src/lib.rs:4", "thread_local!")],
        ),
        // From 2018 on a `use` path's `core` in `n` is the crate, not the
        // root's module of that name (#140).
        (
            "mod core {\n    pub(crate) use elsewhere::thread_local;\n}\nmod n {\n    use core::thread_local;\n    thread_local! { static BC: u8 = 55; }\n}\n",
            vec![],
        ),
        // A root `use` does not rebind a path in another module from 2018
        // on: `core` in `n` is core.
        (
            "use elsewhere as core;\nmod n {\n    core::thread_local! { static AX: u8 = 50; }\n    use core::thread_local;\n    thread_local! { static AY: u8 = 51; }\n}\n",
            vec![],
        ),
    ]
    .into_iter()
    .enumerate()
    {
        // A `RENAMED` first line stands for a Cargo.toml that renames a
        // dependency to `std`, `RENAMED vstd` to `vstd`.
        let (renamed, lib) = match lib.split_once('\n') {
            Some(("RENAMED", rest)) => (Some("std"), rest),
            Some((first, rest)) if first.starts_with("RENAMED ") => {
                (Some(&first["RENAMED ".len()..]), rest)
            }
            _ => (None, lib),
        };
        let dir = crate_of(&format!("thread-local-{i}"), &[("lib.rs", lib)]);
        if let Some(name) = renamed {
            fs::write(
                dir.join("crates/a/Cargo.toml"),
                format!("[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[package.metadata.verus]\nverify = true\n[dependencies]\n{name} = {{ package = \"other\", path = \"x\" }}\n"),
            )
            .expect("write");
        }
        let (got, out, err) = count(&dir, &["--list"]);
        let rows: Vec<(String, String, String)> = list_rows(&out)
            .into_iter()
            .map(|r| (r.0, r.1, r.2))
            .collect();
        let want: Vec<(String, String, String)> = want
            .iter()
            .map(|(c, at, n)| ((*c).to_owned(), (*at).to_owned(), (*n).to_owned()))
            .collect();
        assert!(
            got == 0 && rows == want,
            "{lib:?}: got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // A glob of a module of this crate that brings the name in by a `use`
    // may bring another crate's: the invocation resolves as any other
    // macro's, and a re-export the counter does not follow fails the run.
    let dir = crate_of(
        "thread-local-binds",
        &[(
            "lib.rs",
            "mod m {\n    pub(crate) use elsewhere::thread_local;\n}\nuse m::*;\nthread_local! { static I: u8 = 9; }\n",
        )],
    );
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains("thread_local! may come through `m::*`"),
        "binds: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
    );
    let _ = fs::remove_dir_all(&dir);
    // In 2015 a `use` path starts at the crate root, so `core` in `n` is
    // the root's module, not core (#140): its re-export may be another
    // crate's macro, and one the counter does not follow fails the run.
    let dir = crate_of(
        "thread-local-2015-module",
        &[(
            "lib.rs",
            "extern crate elsewhere;\nmod core {\n    pub(crate) use elsewhere::thread_local;\n}\nmod n {\n    use core::thread_local;\n    thread_local! { static BB: u8 = 54; }\n}\n",
        )],
    );
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2015\"\n[package.metadata.verus]\nverify = true\n",
    )
    .expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1
            && err.contains(
                "thread_local! is imported from this crate by a path the counter does not follow"
            ),
        "2015 module: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
    );
    let _ = fs::remove_dir_all(&dir);
    // From 2018 on (the edition boundary of the rule) `core` in `n` is the
    // crate, not the root's module: the bare one is std's.
    let dir = crate_of(
        "thread-local-2018-module",
        &[(
            "lib.rs",
            "mod core {\n    pub(crate) use elsewhere::thread_local;\n}\nmod n {\n    use core::thread_local;\n    thread_local! { static BE: u8 = 57; }\n}\n",
        )],
    );
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2018\"\n[package.metadata.verus]\nverify = true\n",
    )
    .expect("write");
    let (got, out, err) = count(&dir, &["--list"]);
    assert!(
        got == 0 && list_rows(&out).is_empty(),
        "2018 module: got = exit {got}, {out:?}, {err:?}; wanted = exit 0, no rows"
    );
    let _ = fs::remove_dir_all(&dir);
    // In 2015 a `use` path starts at the crate root: `elsewhere` in `n` is
    // the root's empty module, so the bare one stays std's; with no such
    // module at the root it is the crate, though `n` has a child
    // `elsewhere`.
    for (case, lib, want) in [
        (
            "the root's module",
            "mod elsewhere {}\nmod n {\n    use elsewhere::*;\n    thread_local! { static U: u8 = 21; }\n}\n",
            vec![],
        ),
        (
            "a `cfg`-gated module beside the crate",
            "#[cfg(feature = \"stub\")]\nmod elsewhere {}\n#[cfg(not(feature = \"stub\"))]\nextern crate elsewhere;\nuse elsewhere::*;\nthread_local! { static AG: u8 = 33; }\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:6".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root glob that brings a module named core",
            "extern crate elsewhere;\nmod outer {\n    pub mod core {\n        pub use elsewhere::*;\n    }\n}\nuse outer::*;\nmod n {\n    use core::*;\n    thread_local! { static AM: u8 = 39; }\n}\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:10".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root glob through an alias that brings a module named core",
            "extern crate elsewhere;\nmod outer {\n    pub mod core {\n        pub use elsewhere::*;\n    }\n}\nuse outer as alias;\nuse alias::*;\nmod n {\n    use core::*;\n    thread_local! { static AO: u8 = 41; }\n}\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:11".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root glob through an alias that brings a module named vstd",
            "extern crate elsewhere;\nmod outer {\n    pub mod vstd {\n        pub mod prelude {\n            pub use elsewhere::*;\n        }\n    }\n}\nuse outer as alias;\nuse alias::*;\nmod n {\n    use vstd::prelude::*;\n    thread_local! { static AP: u8 = 42; }\n}\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:13".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root glob of an alias named core that brings vstd",
            "extern crate elsewhere;\nmod outer {\n    pub mod vstd {\n        pub mod prelude {\n            pub use elsewhere::*;\n        }\n    }\n}\nuse outer as core;\nuse core::*;\nmod n {\n    use vstd::prelude::*;\n    thread_local! { static AQ: u8 = 43; }\n}\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:13".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root glob of an extern crate alias named core",
            "extern crate elsewhere as core;\nuse core::*;\nmod n {\n    use vstd::prelude::*;\n    thread_local! { static AS: u8 = 45; }\n}\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:5".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root glob of a module named core that brings vstd",
            "extern crate elsewhere;\nmod core {\n    pub mod vstd {\n        pub mod prelude {\n            pub use elsewhere::*;\n        }\n    }\n}\nuse core::*;\nmod n {\n    use vstd::prelude::*;\n    thread_local! { static AR: u8 = 44; }\n}\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:12".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root glob that brings a module named vstd",
            "extern crate elsewhere;\nmod outer {\n    pub mod vstd {\n        pub mod prelude {\n            pub use elsewhere::*;\n        }\n    }\n}\nuse outer::*;\nmod n {\n    use vstd::prelude::*;\n    thread_local! { static AN: u8 = 40; }\n}\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:12".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root `use` alias of vstd",
            "extern crate elsewhere;\nuse elsewhere as vstd;\nmod n {\n    use vstd::prelude::*;\n    thread_local! { static AC: u8 = 29; }\n}\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:5".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root `use` alias of core",
            "extern crate elsewhere;\nuse elsewhere as core;\nmod n {\n    use core::*;\n    thread_local! { static AD: u8 = 30; }\n}\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:5".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root `mod vstd` that re-exports a crate's module",
            "mod vstd {\n    pub(crate) use elsewhere::prelude;\n}\nmod n {\n    use vstd::prelude::*;\n    thread_local! { static AE: u8 = 31; }\n}\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:6".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root `mod core` that re-exports a crate's module",
            "mod core {\n    pub(crate) use elsewhere::cell;\n}\nuse core::cell::*;\nthread_local! { static AF: u8 = 32; }\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:5".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root `mod core` that globs a crate",
            "mod core {\n    pub use elsewhere::*;\n}\nuse core::*;\nthread_local! { static AA: u8 = 27; }\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:5".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root `use` alias of core, in a named import (#137)",
            "extern crate elsewhere;\nuse elsewhere as core;\nmod n {\n    use core::thread_local;\n    thread_local! { static AZ: u8 = 52; }\n}\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:5".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
        (
            "a root `use` alias of core, beside a path in another module",
            "extern crate elsewhere;\nuse elsewhere as core;\nmod n {\n    core::thread_local! { static BA: u8 = 53; }\n}\n",
            vec![],
        ),
        (
            "the crate, beside a child module",
            "extern crate elsewhere;\nmod n {\n    mod elsewhere {}\n    use elsewhere::*;\n    thread_local! { static V: u8 = 22; }\n}\n",
            vec![(
                "Outside".to_owned(),
                "crates/a/src/lib.rs:5".to_owned(),
                "thread_local!".to_owned(),
            )],
        ),
    ] {
        let dir = crate_of("thread-local-2015", &[("lib.rs", lib)]);
        fs::write(
            dir.join("crates/a/Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2015\"\n[package.metadata.verus]\nverify = true\n",
        )
        .expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        let rows: Vec<(String, String, String)> = list_rows(&out)
            .into_iter()
            .map(|r| (r.0, r.1, r.2))
            .collect();
        assert!(
            got == 0 && rows == want,
            "2015, {case}: got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // A module file whose own `#![cfg]` may leave it out: the glob names
    // the crate then, in either edition.
    for (edition, lib, line) in [
        (
            "2024",
            "mod elsewhere;\nuse elsewhere::*;\nthread_local! { static AK: u8 = 37; }\n",
            3,
        ),
        (
            "2015",
            "mod elsewhere;\n#[cfg(not(feature = \"stub\"))]\nextern crate elsewhere;\nuse elsewhere::*;\nthread_local! { static AL: u8 = 38; }\n",
            5,
        ),
    ] {
        let dir = crate_of(
            "thread-local-file-cfg",
            &[
                ("lib.rs", lib),
                ("elsewhere.rs", "#![cfg(feature = \"stub\")]\n"),
            ],
        );
        fs::write(
            dir.join("crates/a/Cargo.toml"),
            format!("[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"{edition}\"\n[package.metadata.verus]\nverify = true\n"),
        )
        .expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        let rows: Vec<(String, String, String)> = list_rows(&out)
            .into_iter()
            .map(|r| (r.0, r.1, r.2))
            .collect();
        let want = vec![(
            "Outside".to_owned(),
            format!("crates/a/src/lib.rs:{line}"),
            "thread_local!".to_owned(),
        )];
        assert!(
            got == 0 && rows == want,
            "file #![cfg], {edition}: got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn the_edition_sets_what_an_expr_matches() {
    let lib = "macro_rules! m { ($e:expr) => {}; ($t:tt) => { pub fn unproved() {} }; }\nm!(_);\n";
    for (i, (targets, want)) in [
        ("edition = \"2021\"\n", vec!["unproved".to_owned()]),
        ("edition = \"2024\"\n", vec![]),
        (
            "edition = \"2024\"\n[[test]]\nname = \"t\"\nedition = \"2021\"\n[[bench]]\nname = \"b\"\nedition = \"2021\"\n[[example]]\nname = \"e\"\nedition = \"2021\"\n",
            vec![],
        ),
        (
            "edition = \"2021\"\n[[bench]]\nname = \"b\"\nedition = \"2024\"\n",
            vec!["unproved".to_owned()],
        ),
        ("edition = \"2021\"\n[lib]\nedition = \"2024\"\n", vec![]),
    ]
    .into_iter()
    .enumerate()
    {
        let dir = crate_of(&format!("edition-{i}"), &[("lib.rs", lib)]);
        fs::write(
            dir.join("crates/a/Cargo.toml"),
            format!("[package]\nname = \"a\"\nversion = \"0.0.0\"\n{targets}"),
        )
        .expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        let names: Vec<String> = list_rows(&out).into_iter().map(|r| r.2).collect();
        assert!(
            got == 0 && names == want,
            "{targets:?}: got = exit {got}, {names:?}, {err:?}; wanted = exit 0, {want:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // Below edition 2021 a `pat` takes no top-level `|`, so `m!(A | B)`
    // takes the fallback; with no `edition` key the crate is 2015.
    let lib =
        "macro_rules! m { ($p:pat) => {}; ($($t:tt)*) => { pub fn unproved() {} }; }\nm!(A | B);\n";
    for (i, (targets, want)) in [
        ("edition = \"2018\"\n", vec!["unproved".to_owned()]),
        ("", vec!["unproved".to_owned()]),
        ("edition = \"2021\"\n", vec![]),
    ]
    .into_iter()
    .enumerate()
    {
        let dir = crate_of(&format!("edition-pat-{i}"), &[("lib.rs", lib)]);
        fs::write(
            dir.join("crates/a/Cargo.toml"),
            format!("[package]\nname = \"a\"\nversion = \"0.0.0\"\n{targets}"),
        )
        .expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        let names: Vec<String> = list_rows(&out).into_iter().map(|r| r.2).collect();
        assert!(
            got == 0 && names == want,
            "pat {targets:?}: got = exit {got}, {names:?}, {err:?}; wanted = exit 0, {want:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

/// Targets whose files are read on both sides of 2024 read `expr`
/// differently, and on both sides of 2021 `pat`, and the counter does not
/// split the files it reads by target: the run fails. The binaries are the
/// `[[bin]]` tables and those cargo finds (`src/main.rs`,
/// `src/bin/NAME.rs`, `src/bin/NAME/main.rs`) at the package's edition,
/// unless `autobins = false` or a `[[bin]]` of that name gives its own. A
/// `[[test]]`, `[[bench]]` or `[[example]]` whose `path` is under `src/`
/// is read too; one outside it is not.
#[test]
fn shipped_targets_across_2024_fail() {
    let lib = "macro_rules! m { ($e:expr) => {}; ($t:tt) => { pub fn unproved() {} }; }\nm!(_);\n";
    let split = "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[lib]\nedition = \"2021\"\n";
    let package = "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2024\"\n";
    let mut cases = vec![
        (
            "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[[bin]]\nname = \"b\"\npath = \"src/main.rs\"\nedition = \"2021\"\n".to_owned(),
            None,
            1,
        ),
        (split.to_owned(), Some("main.rs"), 1),
        (split.to_owned(), Some("bin/x.rs"), 1),
        (split.to_owned(), Some("bin/y/main.rs"), 1),
        (split.replace("[lib]", "autobins = false\n[lib]"), Some("main.rs"), 0),
        (
            format!("{split}[[bin]]\nname = \"a\"\nedition = \"2021\"\n"),
            Some("main.rs"),
            0,
        ),
        // `pat`: 2018 and 2021 differ, 2015 and 2018 do not.
        (
            "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2018\"\n[[bin]]\nname = \"b\"\npath = \"src/main.rs\"\nedition = \"2021\"\n".to_owned(),
            Some("main.rs"),
            1,
        ),
        (
            "[package]\nname = \"a\"\nversion = \"0.0.0\"\n[[bin]]\nname = \"b\"\npath = \"src/main.rs\"\nedition = \"2018\"\n".to_owned(),
            Some("main.rs"),
            0,
        ),
    ];
    // An example with no `edition` of its own takes the package's 2024
    // beside a 2021 `[lib]`: the split fails too.
    cases.push((
        format!("{split}[[example]]\nname = \"demo\"\npath = \"src/legacy.rs\"\n"),
        Some("legacy.rs"),
        1,
    ));
    // A 2021 test, bench or example at `src/legacy.rs` beside a 2024
    // library is read at the library's edition, where `m!(_)` would take
    // the first rule: the run fails. One outside `src/` is never read.
    for kind in ["test", "bench", "example"] {
        cases.push((
            format!(
                "{package}[[{kind}]]\nname = \"l\"\npath = \"src/legacy.rs\"\nedition = \"2021\"\n"
            ),
            Some("legacy.rs"),
            1,
        ));
        cases.push((
            format!("{package}[[{kind}]]\nname = \"l\"\npath = \"other/legacy.rs\"\nedition = \"2021\"\n"),
            None,
            0,
        ));
    }
    for (i, (manifest, bin, want)) in cases.into_iter().enumerate() {
        let dir = crate_of(&format!("edition-split-{i}"), &[("lib.rs", lib)]);
        fs::write(dir.join("crates/a/Cargo.toml"), &manifest).expect("write");
        if let Some(bin) = bin {
            let path = dir.join("crates/a/src").join(bin);
            fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            let text = if bin == "legacy.rs" {
                lib
            } else {
                "fn main() {}\n"
            };
            fs::write(path, text).expect("write");
        }
        let other = dir.join("crates/a/other");
        fs::create_dir_all(&other).expect("mkdir");
        fs::write(other.join("legacy.rs"), lib).expect("write");
        let (got, _, err) = count(&dir, &["--list"]);
        let split = err.contains("read a macro's `expr` or `pat` fragment differently");
        assert!(
            got == want && split == (want == 1),
            "{manifest:?} with {bin:?}: got = exit {got}, {err:?}; wanted = exit {want}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // A binary-only package (no `src/lib.rs`) has no library's edition: a
    // 2021 `[[bin]]` beside the package's 2024 is its one target, by
    // `autobins = false` or `autolib = false`, and the run passes.
    for (i, keys) in ["autobins = false\n", "autolib = false\nautobins = false\n"]
        .into_iter()
        .enumerate()
    {
        let dir = crate_of(&format!("edition-bin-only-{i}"), &[("lib.rs", lib)]);
        fs::remove_file(dir.join("crates/a/src/lib.rs")).expect("remove");
        fs::write(dir.join("crates/a/src/main.rs"), "fn main() {}\n").expect("write");
        let manifest = format!(
            "{package}{keys}[[bin]]\nname = \"a\"\npath = \"src/main.rs\"\nedition = \"2021\"\n"
        );
        fs::write(dir.join("crates/a/Cargo.toml"), &manifest).expect("write");
        let (got, _, err) = count(&dir, &["--list"]);
        assert!(
            got == 0,
            "{manifest:?}: got = exit {got}, {err:?}; wanted = exit 0"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

/// Every shipped file is scanned for export blocks, whether or not it
/// holds a function, and an invocation is found by its tokens: with an
/// empty REACH, an export block of a verified body passes; one in a file
/// of `use`s alone that exports a body defined elsewhere fails as
/// unmatched; and `width::exports !` (a space before the `!`) roots its
/// unverified body, which then fails as reached.
#[test]
fn every_shipped_file_is_scanned_for_exports() {
    let config = format!("{PLAIN}[reach]\npath = \"verus/REACH\"\nexports_macro = \"exports\"\n");
    for (lib, exp, want) in [
        (
            "mod exp;\n",
            "width::exports! { fn body [a, b, c] (); }\nverus! { pub fn body() {} }\n",
            None,
        ),
        (
            "mod exp;\npub fn body() {}\n",
            "use super::body;\nwidth::exports! { fn body [a, b, c] (); }\n",
            Some(
                "an exports! block exports `body`, but the file defines no free function of that name",
            ),
        ),
        (
            "mod exp;\n",
            "width::exports ! { fn body [a, b, c] (); }\npub fn body() {}\n",
            Some("`body` is reachable from a C entry point"),
        ),
    ] {
        let dir = crate_of("exports-files", &[("lib.rs", lib), ("exp.rs", exp)]);
        fs::write(dir.join("verus/count.toml"), &config).expect("write");
        fs::write(dir.join("verus/REACH"), "").expect("write");
        let (got, out, err) = count(&dir, &[]);
        match want {
            None => assert!(
                got == 0,
                "{exp}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
            ),
            Some(want) => assert!(
                got == 1 && err.contains(want),
                "{exp}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
            ),
        }
        let _ = fs::remove_dir_all(&dir);
    }
}

/// An export a `cfg_attr` applies roots the walk where a shipped build
/// can meet the predicate (#153): with an empty
/// REACH, an unverified `cfg_attr(feature = "export-symbols",
/// unsafe(export_name = prefix!(..)))` function and the helper it calls
/// fail as reached, with the feature in `verus_sets` or not. Negative:
/// under `cfg_attr(test, ..)` neither is an entry point, so the run
/// passes.
#[test]
fn an_export_under_cfg_attr_roots_the_walk() {
    let sets = "verus_sets = ['feature = \"export-symbols\"']\n";
    let reach = "[reach]\npath = \"verus/REACH\"\n";
    let lib = |p: &str| {
        format!(
            "fn helper() {{}}\n#[cfg_attr({p}, unsafe(export_name = prefix!(BZ2_x)))]\npub extern \"C\" fn api() {{ helper() }}\n"
        )
    };
    for (config, src, reached) in [
        (
            format!("{PLAIN}{sets}{reach}"),
            lib("feature = \"export-symbols\""),
            true,
        ),
        (
            format!("{PLAIN}{reach}"),
            lib("feature = \"export-symbols\""),
            true,
        ),
        (format!("{PLAIN}{sets}{reach}"), lib("test"), false),
    ] {
        let dir = crate_of("cfg-attr-export", &[("lib.rs", &src)]);
        fs::write(dir.join("verus/count.toml"), &config).expect("write");
        fs::write(dir.join("verus/REACH"), "").expect("write");
        let (got, out, err) = count(&dir, &[]);
        if reached {
            assert!(
                got == 1
                    && err.contains("`api` is reachable from a C entry point")
                    && err.contains("`helper` is reachable from a C entry point"),
                "{config}{src}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, api and helper reached"
            );
        } else {
            assert!(
                got == 0,
                "{config}{src}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }
}

/// A `macro_rules!` under a `cfg` that `verus_sets` names is sure in
/// Verus's build but not in a shipped one, which may compile the earlier
/// definition of its name instead: both are candidates (#26), so a pair
/// that counts differently fails the run and one that counts alike
/// passes, written or emitted, as the same pair under `cfg(unix)` does.
#[test]
fn a_macro_a_shipped_build_may_skip_does_not_shadow_the_one_before() {
    let jit = "verus_sets = ['feature = \"jit\"']\n";
    let arch = "verus_sets = ['target_arch = \"x86_64\"', 'target_arch = \"aarch64\"']\n";
    let differ = Some("m! could be any of 2 definitions");
    for (config, lib, want) in [
        (
            jit,
            "macro_rules! m { () => { fn a() {} fn b() {} }; }\n#[cfg(feature = \"jit\")]\nmacro_rules! m { () => { fn a() {} }; }\nm!();\n",
            differ,
        ),
        (
            jit,
            "macro_rules! m { () => { fn a() {} }; }\n#[cfg(feature = \"jit\")]\nmacro_rules! m { () => { fn a() {} }; }\nm!();\n",
            None,
        ),
        (
            jit,
            "macro_rules! m { () => { fn a() {} fn b() {} }; }\n#[cfg(feature = \"jit\")]\nmod j {\n    macro_rules! m { () => { fn a() {} }; }\n    m!();\n}\n",
            None,
        ),
        (
            jit,
            "macro_rules! m { () => { fn a() {} fn b() {} }; }\nmacro_rules! def { () => { #[cfg(feature = \"jit\")] macro_rules! m { () => { fn a() {} }; } }; }\ndef!();\nm!();\n",
            differ,
        ),
        // The `cfg` is on what holds the definition: a `#[macro_use]`
        // module, a `verus!` block, or a `verus!` block an expansion
        // writes it in.
        (
            jit,
            "macro_rules! m { () => { fn a() {} fn b() {} }; }\n#[cfg(feature = \"jit\")]\n#[macro_use]\nmod j {\n    macro_rules! m { () => { fn a() {} }; }\n}\nm!();\n",
            differ,
        ),
        (
            jit,
            "macro_rules! m { () => { fn a() {} fn b() {} }; }\n#[cfg(feature = \"jit\")]\nverus! {\n    macro_rules! m { () => { fn a() {} }; }\n}\nm!();\n",
            differ,
        ),
        (
            jit,
            "macro_rules! m { () => { fn a() {} fn b() {} }; }\nmacro_rules! def { () => { macro_rules! m { () => { fn a() {} }; } }; }\n#[cfg(feature = \"jit\")]\nverus! {\n    def!();\n}\nm!();\n",
            differ,
        ),
        (
            arch,
            "#[cfg(target_arch = \"x86_64\")]\nmacro_rules! m { () => { fn a() {} fn b() {} }; }\n#[cfg(target_arch = \"aarch64\")]\nmacro_rules! m { () => { fn a() {} }; }\nm!();\n",
            differ,
        ),
        (
            "",
            "macro_rules! m { () => { fn a() {} fn b() {} }; }\nmacro_rules! m { () => { fn a() {} }; }\nm!();\n",
            None,
        ),
        // A `cfg` the counter cannot read may be false: the definition
        // under it does not shadow.
        (
            jit,
            "macro_rules! m { () => { fn a() {} fn b() {} }; }\n#[cfg(version(\"1.80\"))]\nmacro_rules! m { () => { fn a() {} }; }\nm!();\n",
            differ,
        ),
        // In Verus-only code the rule reads Verus's build, which surely
        // compiles the `jit` definition: it alone is the one, and the
        // earlier one, which does not expand, is never tried.
        (
            jit,
            "macro_rules! m { () => { fn }; }\n#[cfg(feature = \"jit\")]\nmacro_rules! m { () => {}; }\n#[cfg(verus_keep_ghost)]\nmod j {\n    m!();\n}\n",
            None,
        ),
    ] {
        let dir = crate_of("macro-shadow", &[("lib.rs", lib)]);
        fs::write(dir.join("verus/count.toml"), format!("{PLAIN}{config}")).expect("write");
        let (got, out, err) = count(&dir, &[]);
        match want {
            None => assert!(
                got == 0,
                "{config:?} {lib:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
            ),
            Some(want) => assert!(
                got == 1 && err.contains(want),
                "{config:?} {lib:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
            ),
        }
        let _ = fs::remove_dir_all(&dir);
    }
}

/// A module declaration whose file the walk does not read fails the run
/// (#124): a `#[path]` to a file not named `*.rs` (`src/gen`), to a
/// `.rs` file outside `src/`, or a `cfg_attr` path to either. A test-only
/// module may sit anywhere, since its functions are left out anyway.
#[test]
fn a_module_file_the_walk_does_not_read_fails_the_run() {
    let fns = "verus! { pub fn f() {} }\n";
    for (lib, want) in [
        (
            "#[path = \"gen\"]\nmod gen;\n",
            Some("src/gen, which the counter does not read"),
        ),
        (
            "#[path = \"../extra.rs\"]\nmod extra;\n",
            Some("crates/a/extra.rs, which the counter does not read"),
        ),
        (
            "#[cfg_attr(unix, path = \"gen\")]\nmod gen;\n",
            Some("src/gen, which the counter does not read"),
        ),
        ("#[cfg(test)]\n#[path = \"gen\"]\nmod gen;\n", None),
        (
            "#[cfg(test)]\n#[path = \"../extra.rs\"]\nmod extra;\n",
            None,
        ),
        ("#[path = \"gen.rs\"]\nmod gen;\n", None),
    ] {
        let dir = crate_of(
            "unread-module",
            &[("lib.rs", lib), ("gen", fns), ("gen.rs", fns)],
        );
        fs::write(dir.join("crates/a/extra.rs"), fns).expect("write");
        let (got, out, err) = count(&dir, &[]);
        match want {
            None => assert!(
                got == 0,
                "{lib:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
            ),
            Some(want) => assert!(
                got == 1 && err.contains(want),
                "{lib:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
            ),
        }
        let _ = fs::remove_dir_all(&dir);
    }
}

/// A lib.rs that is a module (`a::lib`) declares its modules under
/// `lib/`, the other reading of where its modules live: a `#[path]` in
/// an inline module there reaches `src/a/lib/inl/gen`, which the walk
/// does not read, and fails the run as a primary reading's would. Under
/// `cfg(test)` it may sit anywhere.
#[test]
fn an_unread_module_file_of_the_other_reading_fails_the_run() {
    let fns = "verus! { pub fn f() {} }\n";
    for (inl, want) in [
        ("mod inl {\n    #[path = \"gen\"]\n    mod g;\n}\n", true),
        (
            "#[cfg(test)]\nmod inl {\n    #[path = \"gen\"]\n    mod g;\n}\n",
            false,
        ),
    ] {
        let dir = crate_of("unread-alternate", &[("lib.rs", "mod a;\n")]);
        let src = dir.join("crates/a/src");
        fs::create_dir_all(src.join("a/lib/inl")).expect("mkdir");
        fs::write(src.join("a/mod.rs"), "mod lib;\n").expect("write");
        fs::write(src.join("a/lib.rs"), inl).expect("write");
        fs::write(src.join("a/lib/inl/gen"), fns).expect("write");
        let (got, out, err) = count(&dir, &[]);
        if want {
            assert!(
                got == 1 && err.contains("src/a/lib/inl/gen, which the counter does not read"),
                "{inl:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
            );
        } else {
            assert!(
                got == 0,
                "{inl:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }
    // A descendant: `a/lib/child.rs`, which the shipped build reaches by
    // the other reading of `a/lib.rs` and which the one declaration of the
    // primary reading names under `cfg(test)`, declares `gen`. Its
    // declaring file is shipped by the other reading, so `gen` is too.
    let dir = crate_of(
        "unread-descendant",
        &[(
            "lib.rs",
            "mod a;\n#[cfg(test)]\n#[path = \"a/lib/child.rs\"]\nmod test_child;\n",
        )],
    );
    let src = dir.join("crates/a/src");
    fs::create_dir_all(src.join("a/lib")).expect("mkdir");
    fs::write(src.join("a/mod.rs"), "mod lib;\n").expect("write");
    fs::write(src.join("a/lib.rs"), "mod child;\n").expect("write");
    fs::write(src.join("a/lib/child.rs"), "#[path = \"gen\"]\nmod g;\n").expect("write");
    fs::write(src.join("a/lib/gen"), fns).expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains("src/a/lib/gen, which the counter does not read"),
        "descendant: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
    );
    let _ = fs::remove_dir_all(&dir);
    // A crate root is built on its own: `src/main.rs`, a binary, names
    // `gen` (directly, or through `src/m.rs`), though the library declares
    // `main` only under `cfg(test)`.
    for (main, m, want) in [
        ("#[path = \"gen\"]\nmod g;\nfn main() {}\n", None, "src/gen"),
        (
            "mod m;\nfn main() {}\n",
            Some("#[path = \"gen\"]\nmod g;\n"),
            "src/gen",
        ),
    ] {
        let dir = crate_of(
            "unread-root",
            &[
                ("lib.rs", "#[cfg(test)]\nmod main;\n"),
                ("main.rs", main),
                ("gen", fns),
            ],
        );
        if let Some(m) = m {
            fs::write(dir.join("crates/a/src/m.rs"), m).expect("write");
        }
        let (got, out, err) = count(&dir, &[]);
        assert!(
            got == 1 && err.contains(&format!("{want}, which the counter does not read")),
            "{main:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // A file no declaration names may be a target the counter does not
    // know, so it is an entry point; one whose own `#![cfg(test)]` keeps
    // it out of shipped builds declares only test-only modules.
    for (lib, other, want) in [
        ("", "#[path = \"gen\"]\nmod g;\n", 1),
        (
            "mod other;\n",
            "#![cfg(test)]\n#[path = \"gen\"]\nmod g;\n",
            0,
        ),
    ] {
        let dir = crate_of(
            "unread-entry",
            &[("lib.rs", lib), ("other.rs", other), ("gen", fns)],
        );
        let (got, out, err) = count(&dir, &[]);
        assert!(
            got == want && (want == 0 || err.contains("src/gen, which the counter does not read")),
            "{other:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit {want}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // A test-only step earlier on the path: `t` is declared only under
    // `cfg(test)`, and `t.rs` (or `u.rs`, which `t.rs` declares) names
    // `gen` with no guard of its own. No shipped build reaches it.
    for (t, u) in [
        ("#[path = \"gen\"]\nmod g;\n", None),
        ("mod u;\n", Some("#[path = \"../gen\"]\nmod g;\n")),
    ] {
        let dir = crate_of(
            "unread-test-path",
            &[
                ("lib.rs", "#[cfg(test)]\nmod t;\n"),
                ("t.rs", t),
                ("gen", fns),
            ],
        );
        if let Some(u) = u {
            fs::create_dir_all(dir.join("crates/a/src/t")).expect("mkdir");
            fs::write(dir.join("crates/a/src/t/u.rs"), u).expect("write");
        }
        let (got, out, err) = count(&dir, &[]);
        assert!(
            got == 0,
            "{t:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // A link under src/ to a file not named `*.rs`, which the walk passes
    // over (#71), reaches the check by the file it names.
    let dir = crate_of("unread-link", &[("lib.rs", "#[path = \"gen\"]\nmod g;\n")]);
    fs::write(dir.join("crates/a/extra"), fns).expect("write");
    std::os::unix::fs::symlink("../extra", dir.join("crates/a/src/gen")).expect("ln");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains("crates/a/extra, which the counter does not read"),
        "link: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A file is classed from every reading that may compile it (#47): one
/// that only the other reading of a lib.rs that is a module (`a::lib`,
/// whose `mod child;` is `a/lib/child.rs`) reaches without a guard is
/// shipped, though the primary reading names it only under `cfg(test)`;
/// and a binary root is shipped, though the library declares it as a
/// module under `cfg(test)`.
#[test]
fn a_file_is_classed_from_every_reading_that_compiles_it() {
    let tree = |name: &str| {
        let dir = crate_of(name, &[]);
        let src = dir.join("crates/a/src");
        (dir, src)
    };
    let (dir, src) = tree("every-reading-alternate");
    fs::create_dir_all(src.join("a/lib")).expect("mkdir");
    fs::write(
        src.join("lib.rs"),
        "mod a;\n#[cfg(test)]\n#[path = \"a/lib/child.rs\"]\nmod test_child;\n",
    )
    .expect("write");
    fs::write(src.join("a/mod.rs"), "mod lib;\n").expect("write");
    fs::write(src.join("a/lib.rs"), "mod child;\n").expect("write");
    fs::write(src.join("a/lib/child.rs"), "pub fn shipped_fn() {}\n").expect("write");
    let (got, out, err) = count(&dir, &["--list"]);
    let names: Vec<String> = list_rows(&out).into_iter().map(|r| r.2).collect();
    assert!(
        got == 0 && names == ["shipped_fn"],
        "alternate: got = exit {got}, {names:?}, {err:?}; wanted = exit 0, [\"shipped_fn\"]"
    );
    let _ = fs::remove_dir_all(&dir);
    let (dir, src) = tree("every-reading-root");
    fs::write(src.join("lib.rs"), "#[cfg(test)]\nmod main;\n").expect("write");
    fs::write(src.join("main.rs"), "pub fn helper() {}\nfn main() {}\n").expect("write");
    let (got, out, err) = count(&dir, &["--list"]);
    let mut names: Vec<String> = list_rows(&out).into_iter().map(|r| r.2).collect();
    names.sort();
    assert!(
        got == 0 && names == ["helper", "main"],
        "root: got = exit {got}, {names:?}, {err:?}; wanted = exit 0, [\"helper\", \"main\"]"
    );
    let _ = fs::remove_dir_all(&dir);
    // The other reading may be one no target makes: a stray `src/lib/foo.rs`
    // beside a root lib.rs's `mod foo;`, or `src/b.rs` beside `src/a.rs`'s
    // `mod b;` where lib.rs declares `b` only for tests. Each is counted as
    // unsure through that reading, never verified.
    for (case, files) in [
        (
            "stray lib/foo.rs",
            &[
                ("lib.rs", "mod foo;\n"),
                ("foo.rs", ""),
                ("lib/foo.rs", "verus! { pub fn b() {} }\n"),
            ][..],
        ),
        (
            "a.rs's mod b; read beside a.rs",
            &[
                ("lib.rs", "mod a;\n#[cfg(test)]\nmod b;\n"),
                ("a.rs", "mod b;\n"),
                ("a/b.rs", ""),
                ("b.rs", "verus! { pub fn b() {} }\n"),
            ][..],
        ),
        // Named by no primary declaration, and by the other reading only
        // under `cfg(test)`: it keeps the fallback of a file nothing
        // declares (here `include!` compiles it).
        (
            "only a test-only alternate names it",
            &[
                ("lib.rs", "mod parser;\ninclude!(\"tables.rs\");\n"),
                ("parser.rs", "#[cfg(test)]\nmod tables;\n"),
                ("parser/tables.rs", ""),
                ("tables.rs", "pub fn b() {}\n"),
            ][..],
        ),
    ] {
        let (dir, src) = tree("every-reading-stray");
        for (path, text) in files {
            if let Some(parent) = src.join(path).parent() {
                fs::create_dir_all(parent).expect("mkdir");
            }
            fs::write(src.join(path), text).expect("write");
        }
        let (got, out, err) = count(&dir, &["--list"]);
        let rows: Vec<(String, String)> = list_rows(&out).into_iter().map(|r| (r.0, r.2)).collect();
        assert!(
            got == 0
                && rows.iter().any(|(_, name)| name == "b")
                && rows.iter().all(|(class, _)| class != "Verified"),
            "{case}: got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, `b` counted, nothing Verified"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // One the primary reading declares only for tests, and the other
    // reading too, stays test-only: the fallback is for a file the
    // primary reading does not declare.
    let (dir, src) = tree("every-reading-test-only");
    fs::create_dir_all(src.join("parser")).expect("mkdir");
    for (path, text) in [
        ("lib.rs", "mod parser;\n#[cfg(test)]\nmod tables;\n"),
        ("parser.rs", "#[cfg(test)]\nmod tables;\n"),
        ("parser/tables.rs", ""),
        ("tables.rs", "pub fn t() {}\n"),
    ] {
        fs::write(src.join(path), text).expect("write");
    }
    let (got, out, err) = count(&dir, &["--list"]);
    let names: Vec<String> = list_rows(&out).into_iter().map(|r| r.2).collect();
    assert!(
        got == 0 && names.is_empty(),
        "test-only in both readings: got = exit {got}, {names:?}, {err:?}; wanted = exit 0, no rows"
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

/// cargo verus has Verus check only a package whose Cargo.toml sets
/// `[package.metadata.verus] verify = true`, so a crate with code Verus
/// checks (a verified function, or a proof built only for Verus) that
/// does not opt in fails the run, whatever else the manifest says, and
/// every such crate is named: a second one, `b-c` (spelled `b_c` in the
/// names Verus gives), with no table, no Cargo.toml or one that does not
/// parse. A crate with no Verus code needs no opt-in.
#[test]
fn a_crate_with_verus_code_must_opt_into_verification() {
    let verified = "use vstd::prelude::*;\nverus! {\npub fn v() {}\n}\n";
    let ghost = "pub fn o() {}\n#[cfg(verus_keep_ghost)]\nverus! {\nproof fn p() {}\n}\n";
    let plain = "pub fn o() {}\n";
    let head = "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2024\"\n";
    let opt_in = format!("{head}[package.metadata.verus]\nverify = true\n");
    let fail = "FAIL verification opt-in: ";
    // The second crate's Cargo.toml: no table, none at all, or not TOML.
    let second = [
        Some(head.replace("\"a\"", "\"b-c\"")),
        None,
        Some(format!(
            "{}[package.metadata.verus\nverify = true\n",
            head.replace("\"a\"", "\"b-c\"")
        )),
    ];
    for (i, (lib, manifest, workspace, want, named)) in [
        (verified, opt_in.clone(), "", 0, 0),
        (ghost, opt_in.clone(), "", 0, 0),
        (plain, head.to_owned(), "", 0, 0),
        (verified, head.to_owned(), "", 1, 1),
        (ghost, head.to_owned(), "", 1, 1),
        (
            verified,
            format!("{head}[package.metadata.verus]\nverify = false\n"),
            "",
            1,
            1,
        ),
        (
            verified,
            format!("{head}[package.metadata.verus]\nverify = \"true\"\n"),
            "",
            1,
            1,
        ),
        (
            verified,
            format!("{head}[package.metadata]\nverify = true\n"),
            "",
            1,
            1,
        ),
        (
            verified,
            head.to_owned(),
            "[workspace.metadata.verus]\nverify = true\n",
            1,
            1,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let dir = crate_of("opt-in", &[("lib.rs", lib)]);
        fs::write(dir.join("crates/a/Cargo.toml"), &manifest).expect("write");
        fs::write(dir.join("Cargo.toml"), workspace).expect("write");
        // A second crate with Verus code that does not opt in, named
        // alongside.
        let b = &second[i % second.len()];
        if named > 0 {
            fs::create_dir_all(dir.join("crates/b-c/src")).expect("mkdir");
            if let Some(text) = b {
                fs::write(dir.join("crates/b-c/Cargo.toml"), text).expect("write");
            }
            fs::write(dir.join("crates/b-c/src/lib.rs"), verified).expect("write");
        }
        let (got, out, err) = count(&dir, &[]);
        let lines = err.matches(fail).count();
        let want_lines = if named > 0 { 2 } else { 0 };
        assert!(
            got == want
                && lines == want_lines
                && (want == 0
                    || (err.contains("crates/a has code") && err.contains("crates/b-c has code"))),
            "{lib:?} {manifest:?} {workspace:?} {b:?}: got = exit {got}, {lines} lines, {out:?}, {err:?}; wanted = exit {want}, {want_lines} lines"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

/// The opt-in rule holds the head alone: a port adds the table in the
/// pull request that moves to the release with this rule, and its verus
/// check runs against a base whose crate has Verus code and no table. That
/// run passes; the same head without the table fails.
#[test]
fn the_opt_in_rule_holds_the_head_not_the_base() {
    let dir = crate_of("opt-in-base", &[("lib.rs", RATCHET_LIB)]);
    let manifest = dir.join("crates/a/Cargo.toml");
    let plain = "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2024\"\n";
    fs::write(&manifest, plain).expect("write");
    fs::write(dir.join("verus/count.toml"), RATCHET_CONFIG).expect("write");
    fs::write(dir.join("verus/FLOOR"), RATCHET_FLOOR).expect("write");
    fs::write(dir.join("verus/RLIMIT"), RATCHET_RLIMIT).expect("write");
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "crates", "verus"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    let stats = dir.join("stats.json");
    fs::write(&stats, ratchet_stats(3_000_000)).expect("write");
    for (head, want) in [
        (
            format!("{plain}[package.metadata.verus]\nverify = true\n"),
            0,
        ),
        (plain.to_owned(), 1),
    ] {
        fs::write(&manifest, &head).expect("write");
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
        assert_eq!(
            got, want,
            "{head:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit {want}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Ordinary Rust may name a field, a parameter or a `let` binding with a
/// word Verus reserves (`exec`, `tracked`, `ghost`); outside `verus!` the
/// counter reads such a file with no `stand_ins`, every function counted
/// at its own line, while `verus!` keeps its own syntax (`let tracked`).
/// A macro's metavariable named so is not rewritten: the function its
/// expansion writes, which `assume`s through it, stays trusted. Nor is a
/// `macro_rules!` matcher's literal `ghost:`, whose invocation must still
/// match it rather than fall to a catch-all arm that writes nothing.
#[test]
fn verus_keywords_name_bindings_outside_verus() {
    let lib = "use vstd::prelude::*;\npub struct S {\n    pub exec: u8,\n    tracked:&'static u8,\n}\npub fn f(ghost: u8) -> u8 {\n    let tracked = ghost;\n    let mut exec = tracked;\n    exec += 1;\n    exec\n}\nverus! {\nproof fn p() {\n    let tracked x = 1u8;\n}\npub fn v() {}\n}\npub fn g(s: &S) -> u8 {\n    s.exec + *s.tracked\n}\nmacro_rules! checked { ($name:ident, $proof:expr) => { #[verus_verify] fn $name() { proof!($proof); } } }\nchecked!(h, assume(false));\nmacro_rules! literal { (ghost: $n:ident) => { #[verus_verify] fn $n() { proof!(assume(false)); } }; ($($t:tt)*) => {}; }\nliteral!(ghost: k);\n";
    let dir = crate_of("verus-keywords", &[("lib.rs", lib)]);
    let (got, out, err) = count(&dir, &["--list"]);
    let rows: Vec<String> = list_rows(&out)
        .into_iter()
        .map(|r| format!("{} {} {}", r.0, r.1, r.2))
        .collect();
    let want = [
        "Outside crates/a/src/lib.rs:6 f",
        "Verified crates/a/src/lib.rs:16 v",
        "Outside crates/a/src/lib.rs:18 g",
        "Trusted crates/a/src/lib.rs:22 h",
        "Trusted crates/a/src/lib.rs:24 k",
    ];
    assert!(
        got == 0 && rows == want && err.is_empty(),
        "got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
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

/// A crate whose only Verus code is a proof built only for Verus
/// (`#[cfg(verus_keep_ghost)]`) counts outside functions alone, but Verus
/// checks the proof, so its headroom is held: one using 90% of its rlimit
/// fails `max-share 65`, where one using 30% passes.
#[test]
fn a_crate_of_verus_only_proofs_is_held_to_its_headroom() {
    let lib = "pub fn o() {}\n#[cfg(verus_keep_ghost)]\nverus! {\nproof fn p() {}\n}\n";
    let dir = crate_of("rlimit-ghost", &[("lib.rs", lib)]);
    fs::write(
        dir.join("verus/count.toml"),
        format!("{PLAIN}[rlimit]\npath = \"verus/RLIMIT\"\n"),
    )
    .expect("write");
    fs::write(dir.join("verus/RLIMIT"), "max-share 65\n").expect("write");
    let stats = dir.join("stats.json");
    for (used, want, line) in [
        (
            27_000_000,
            1,
            "FAIL rlimit headroom: a::p uses 90.0% of its rlimit",
        ),
        (9_000_000, 0, "ok rlimit headroom: 1 functions"),
    ] {
        fs::write(&stats, ratchet_stats(used).replace("a::v", "a::p")).expect("write");
        let s = stats.to_string_lossy().into_owned();
        let (got, out, err) = count(&dir, &["--rlimit", &s]);
        assert!(
            got == want && (out.contains(line) || err.contains(line)),
            "{used}: got = exit {got}, {out:?}, {err:?}; wanted = exit {want}, {line:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// One crate's statistics laid out as Verus prints them, a line per
/// top-level key between a `{` line and a `}` line: `name` using `used`
/// of its rlimit count.
fn pretty_stats(name: &str, used: u64) -> Vec<String> {
    vec![
        "{".to_owned(),
        format!(
            r#"  "times-ms": {{"smt":{{"smt-run-module-times":[{{"module":"m","rlimit":0,"function-breakdown":[{{"function":"{name}","rlimit":{used}}}]}}]}}}},"#
        ),
        r#"  "verification-results": {"success":true,"verified":1,"errors":0}"#.to_owned(),
        "}".to_owned(),
    ]
}

/// Two crates' statistics pass back to back and fail, exit 2, naming the
/// interleave, when cargo's line-at-a-time re-print of two crates checked
/// in parallel mixed them.
#[test]
fn interleaved_statistics_fail_the_run() {
    let dir = crate_of(
        "rlimit-interleave",
        &[("lib.rs", "verus! {\nfn v() {}\n}\n")],
    );
    fs::write(
        dir.join("verus/count.toml"),
        format!("{PLAIN}[rlimit]\npath = \"verus/RLIMIT\"\n"),
    )
    .expect("write");
    fs::write(dir.join("verus/RLIMIT"), "max-share 65\n").expect("write");
    let (a, v) = (pretty_stats("a::v", 3_000_000), pretty_stats("vstd::x", 1));
    let mixed: Vec<String> = [&a[0], &v[0], &a[1], &v[1], &v[2], &a[2], &v[3], &a[3]]
        .into_iter()
        .cloned()
        .collect();
    let stats = dir.join("stats.json");
    let s = stats.to_string_lossy().into_owned();
    for (lines, want, line) in [
        (
            [a.clone(), v.clone()].concat(),
            0,
            "ok rlimit headroom: 1 functions",
        ),
        (
            mixed,
            2,
            "stats.json: documents interleave: line 2 starts a document before the one line 1 starts ends.",
        ),
    ] {
        fs::write(&stats, lines.join("\n") + "\n").expect("write");
        let (got, out, err) = count(&dir, &["--rlimit", &s]);
        assert!(
            got == want && (out.contains(line) || err.contains(line)),
            "exit {want}: got = exit {got}, {out:?}, {err:?}; wanted = exit {want}, {line:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// Verus's statistics for crate `a`: each function in `fns` with the
/// rlimit count it used.
fn stats_naming(fns: &[(&str, u64)]) -> String {
    let rows: Vec<String> = fns
        .iter()
        .map(|(n, used)| format!(r#"{{"function":"{n}","rlimit":{used}}}"#))
        .collect();
    format!(
        r#"{{"times-ms":{{"smt":{{"smt-run-module-times":[{{"module":"m","rlimit":0,"function-breakdown":[{}]}}]}}}},"verification-results":{{"success":true,"verified":{},"errors":0}}}}"#,
        rows.join(","),
        fns.len()
    )
}

/// What a method with `#[verifier::rlimit(20)]` uses in
/// [`a_method_is_named_by_its_type_s_module`]: 60% of that limit, inside
/// max-share 65, but 120% of the default 10. A limit put on another
/// function leaves this one at the default, and the run fails.
const HELD: u64 = 36_000_000;

/// Verus names a method by the path of the item its `impl`'s type
/// resolves to, not by the `impl`'s module (rust_verify's
/// `register_friendly_path_as_rust_name`), so a method's
/// `#[verifier::rlimit]` is named that way, the type found as rustc's
/// scoping finds it: by its path (`super::Top`, an alias
/// `crate::state::Alias`, `inner::Kid` through a module the `impl`'s
/// module declares, `state::Deep` through `use crate::x::state;` though
/// the crate root has a `state` with a `Deep`); defined in the `impl`'s module (`Local`, and
/// `Shadow`, which a macro there writes), ahead of a glob import of the
/// same name; imported by name (`Renamed`); or brought in by the one
/// glob of this crate's modules that has it (`S`, though a test-only `S`
/// sits in the `impl`'s module, and `Twice`, though the crate root has
/// one too). A trait's default method keeps the trait's module, though a
/// struct the glob brings in shares its name. Each statistics file also
/// names the function each wrong place would give (using 1), so a limit
/// put there leaves the right function at the default, over max-share.
///
/// A type the counter cannot place keeps the `impl`'s module, so its
/// limit names nothing the statistics have and the run fails, each such
/// limit named, though a function of the same name sits in a module the
/// crate defines a type of that name in: an `impl`'s own type parameter
/// (`S`); a type another crate's `use` imports by name (`Gone`); one an
/// external glob brings in (`Seq`, from `vstd::prelude`), beside a glob
/// of a sibling whose `Seq` is private; one a glob's module re-exports
/// (`Re`, through `prelude`), beside a glob of a module whose `Re` only it
/// sees; one two `use`s import under exclusive `cfg`s (`Cond`, `Ghosty`);
/// one a `use` Verus's build might not compile imports, ahead of a glob
/// that also brings it in (`Mixed`); and one a glob's module defines
/// under a `cfg` Verus's build might not meet, beside another crate's
/// glob (`Flip`); one only a glob of a sibling brings in whose type of
/// that name is `pub(in crate::b)`, beside another crate's glob (`Set`);
/// one only a glob under a `cfg` Verus's build might not meet brings in,
/// beside another crate's glob (`Gseq`); and one a path, a glob or a
/// `use` reaches through a first segment, `state2`, that only the crate
/// root declares as a module, which below the root names another crate
/// (`state2::T`, `U`, `V`); and, in modules of their own, a name a binding
/// the walk does not see may take from the glob: a type another crate's
/// macro may write, though its tokens never name it (`Made`, beside
/// `elsewhere::mk2! {}`), one another crate's `thread_local!` may write
/// (`Tl`, beside `elsewhere::thread_local! {}`), one a bare
/// `thread_local!` not shown to be std's may write (`Tl2`, beside the
/// crate's own `macro_rules! thread_local`; `Tl3`, beside `use
/// elsewhere::thread_local;`; `Tl4`, std's but beside a `*` that may
/// bring another; `Tl5`, beside a glob and another crate's
/// `thread_local` that only Verus's build compiles), and a `use` a local
/// macro's expansion writes (`Foo`,
/// from `imp!()`'s `use crate::other::Foo;`). std's `thread_local!` by
/// `::std::` writes statics alone, and split.rs's globs still place their
/// types beside it.
#[test]
fn a_method_is_named_by_its_type_s_module() {
    let lib = "use vstd::prelude::*;\nmod state;\nmod split;\nmod other;\nmod prelude;\nmod b;\nmod alt;\nmod alt2;\nmod x;\nmod state2;\nmod opaque;\nmod imp;\nmod tl;\nmod tl2;\nmod tl3;\nmod tl4;\nmod tl5;\npub struct Top;\npub struct Twice;\n";
    let state = "pub struct S;\npub struct Named;\npub struct Local;\npub struct Walk;\npub struct Twice;\npub struct Shadow;\npub struct Gone;\npub struct Cond;\npub struct Deep;\npub struct Made;\npub struct Foo;\npub struct Tl;\npub struct Tl2;\npub struct Tl3;\npub struct Tl4;\npub struct Tl5;\npub type Alias = S;\npub trait Tr {\n    fn step(&self);\n}\n";
    let x = "pub mod state {\n    pub struct Deep;\n}\n";
    let state2 = "pub struct T;\npub struct U;\npub struct V;\n";
    let other = "pub struct Seq;\npub struct Re;\npub struct Mixed;\npub struct Ghosty;\npub struct Cond;\npub struct Foo;\n";
    // Modules apart from split.rs, each of which may bind a name the walk
    // does not see, so that none of split.rs's globs is refused with them:
    // another crate's macro, whose tokens never name what it writes, and
    // a `use` a local macro's expansion writes.
    let opaque = "use vstd::prelude::*;\nuse super::state::*;\nelsewhere::mk2! {}\n\
verus! {\nimpl Made { #[verifier::rlimit(20)] fn made(&self) {} }\n}\n";
    let imp = "use vstd::prelude::*;\nuse super::state::*;\n\
macro_rules! imp { () => { use crate::other::Foo; }; }\nimp!();\n\
verus! {\nimpl Foo { #[verifier::rlimit(20)] fn foo(&self) {} }\n}\n";
    // Another crate's macro named `thread_local`, which may write any
    // item, unlike std's.
    let tl = "use vstd::prelude::*;\nuse super::state::*;\nelsewhere::thread_local! {}\n\
verus! {\nimpl Tl { #[verifier::rlimit(20)] fn tl(&self) {} }\n}\n";
    // A bare `thread_local!` at file level that is not shown to be std's:
    // the crate's own macro of the name, one a `use` imports, and std's
    // beside a `*` (here `vstd::prelude::*`'s) that may bring another.
    let tl2 = "use vstd::prelude::*;\nuse super::state::*;\n\
macro_rules! thread_local { ($($t:tt)*) => {}; }\nthread_local! {}\n\
verus! {\nimpl Tl2 { #[verifier::rlimit(20)] fn tl2(&self) {} }\n}\n";
    let tl3 = "use vstd::prelude::*;\nuse super::state::*;\nuse elsewhere::thread_local;\n\
thread_local! { static THREE: u8 = 3; }\n\
verus! {\nimpl Tl3 { #[verifier::rlimit(20)] fn tl3(&self) {} }\n}\n";
    let tl4 = "use vstd::prelude::*;\nuse super::state::*;\nthread_local! { static FOUR: u8 = 4; }\n\
verus! {\nimpl Tl4 { #[verifier::rlimit(20)] fn tl4(&self) {} }\n}\n";
    // The glob and another crate's `thread_local` only Verus's build
    // compiles: no shipped binding breaks std's bare rule, but Verus's
    // build, whose names the limits take, has both. It runs in a crate of
    // its own, with no `macro_rules! thread_local` (tl2's), which makes
    // every bare `thread_local!` in its crate not std's.
    let tl5 = "use vstd::prelude::verus;\n#[cfg(verus_keep_ghost)]\nuse super::state::*;\n\
#[cfg(verus_keep_ghost)]\nuse elsewhere::thread_local;\nthread_local! { static FIVE: u8 = 5; }\n\
verus! {\nimpl Tl5 { #[verifier::rlimit(20)] fn tl5(&self) {} }\n}\n";
    let prelude = "pub use crate::other::Re;\n";
    let b = "struct Seq;\npub(in crate::b) struct Re;\npub(in crate::b) struct Set;\n";
    let alt2 = "pub struct Gseq;\n";
    let alt = "pub struct Mixed;\npub struct Ghosty;\n#[cfg(feature = \"alternate\")]\npub struct Flip;\n";
    let split = |uses: &str, extra: &str| {
        format!(
            "use vstd::prelude::*;\nuse super::state::*;\nuse crate::state::Named as Renamed;\nuse crate::x::state;\n{uses}\
#[cfg(test)]\npub struct S;\n\
pub mod inner {{ pub struct Kid; }}\n\
::std::thread_local! {{ static ONE: u8 = 1; }}\n\
::std::thread_local! {{ static TWO: u8 = 2; }}\n\
macro_rules! shadow {{ () => {{ pub struct Shadow; }}; }}\nshadow!();\n\
verus! {{\npub struct Local;\n\
impl S {{ #[verifier::rlimit(20)] fn bare(&self) {{}} }}\n\
impl super::Top {{ #[verifier::rlimit(20)] fn pathed(&self) {{}} }}\n\
impl Renamed {{ #[verifier::rlimit(20)] fn imported(&self) {{}} }}\n\
impl Local {{ #[verifier::rlimit(20)] fn here(&self) {{}} }}\n\
impl crate::state::Alias {{ #[verifier::rlimit(20)] fn aliased(&self) {{}} }}\n\
impl Twice {{ #[verifier::rlimit(20)] fn two(&self) {{}} }}\n\
impl Shadow {{ #[verifier::rlimit(20)] fn shadowed(&self) {{}} }}\n\
impl state::Deep {{ #[verifier::rlimit(20)] fn deep(&self) {{}} }}\n\
impl inner::Kid {{ #[verifier::rlimit(20)] fn kid(&self) {{}} }}\n\
trait Walk {{ #[verifier::rlimit(20)] fn walk(&self) {{}} }}\n\
{extra}}}\n"
        )
    };
    let placed: Vec<(&str, u64)> = vec![
        ("a::state::S::bare", HELD),
        ("a::Top::pathed", HELD),
        ("a::state::Named::imported", HELD),
        ("a::split::Local::here", HELD),
        ("a::state::Alias::aliased", HELD),
        ("a::state::Twice::two", HELD),
        ("a::split::Shadow::shadowed", HELD),
        ("a::split::Walk::walk", HELD),
        ("a::x::state::Deep::deep", HELD),
        ("a::split::inner::Kid::kid", HELD),
        // The wrong places.
        ("a::state::Deep::deep", 1),
        ("a::split::S::bare", 1),
        ("a::state::Local::here", 1),
        ("a::Twice::two", 1),
        ("a::state::Shadow::shadowed", 1),
        ("a::state::Walk::walk", 1),
    ];
    let unplaced = "impl<S: Copy> Tr for S { #[verifier::rlimit(20)] fn step(&self) {} }\n\
impl Tr for Gone { #[verifier::rlimit(20)] fn step(&self) {} }\n\
impl<A> Tr for Seq<A> { #[verifier::rlimit(20)] fn step(&self) {} }\n\
impl Re { #[verifier::rlimit(20)] fn re(&self) {} }\n\
impl Mixed { #[verifier::rlimit(20)] fn mixed(&self) {} }\n\
impl Ghosty { #[verifier::rlimit(20)] fn ghosty(&self) {} }\n\
impl Cond { #[verifier::rlimit(20)] fn cond(&self) {} }\n\
impl Flip { #[verifier::rlimit(20)] fn flip(&self) {} }\n\
impl<A> Tr for Set<A> { #[verifier::rlimit(20)] fn step(&self) {} }\n\
impl Tr for Gseq { #[verifier::rlimit(20)] fn step(&self) {} }\n\
impl state2::T { #[verifier::rlimit(20)] fn t(&self) {} }\n\
impl U { #[verifier::rlimit(20)] fn u(&self) {} }\n\
impl V { #[verifier::rlimit(20)] fn v(&self) {} }\n";
    let unplaced_uses = "use elsewhere::Gone;\nuse elsewhere::*;\nuse state2::*;\nuse state2::V;\nuse crate::prelude::*;\nuse crate::b::*;\nuse crate::alt::*;\n\
#[cfg(feature = \"alternate\")]\nuse crate::alt2::*;\n\
#[cfg(feature = \"alternate\")]\nuse crate::state::Cond;\n#[cfg(not(feature = \"alternate\"))]\nuse crate::other::Cond;\n\
#[cfg(feature = \"alternate\")]\nuse crate::other::Mixed;\n\
#[cfg(not(verus_keep_ghost))]\nuse crate::other::Ghosty;\n#[cfg(verus_keep_ghost)]\nuse crate::alt::Ghosty;\n";
    let wrong = [
        ("a::state::S::step", 1),
        ("a::state::Gone::step", 1),
        ("a::other::Seq::step", 1),
        ("a::b::Seq::step", 1),
        ("a::other::Re::re", 1),
        ("a::b::Re::re", 1),
        ("a::other::Mixed::mixed", 1),
        ("a::alt::Mixed::mixed", 1),
        ("a::other::Ghosty::ghosty", 1),
        ("a::alt::Ghosty::ghosty", 1),
        ("a::state::Cond::cond", 1),
        ("a::other::Cond::cond", 1),
        ("a::alt::Flip::flip", 1),
        ("a::b::Set::step", 1),
        ("a::alt2::Gseq::step", 1),
        ("a::state2::T::t", 1),
        ("a::state2::U::u", 1),
        ("a::state2::V::v", 1),
        ("a::state::Made::made", 1),
        ("a::state::Foo::foo", 1),
        ("a::state::Tl::tl", 1),
        ("a::state::Tl3::tl3", 1),
        ("a::state::Tl4::tl4", 1),
    ];
    let miss =
        |name: &str| format!("#[verifier::rlimit] on {name}, which Verus's statistics do not name");
    for (uses, extra, apart, fns, want, lines) in [
        (
            "",
            "",
            ["", "", "", "", "", "", ""],
            placed.clone(),
            0,
            vec!["ok rlimit headroom: 16 functions".to_owned()],
        ),
        (
            unplaced_uses,
            unplaced,
            [opaque, imp, tl, "", tl3, tl4, ""],
            [&placed[..], &wrong[..]].concat(),
            2,
            vec![
                miss("a::split::S::step"),
                miss("a::split::Gone::step"),
                miss("a::split::Seq::step"),
                miss("a::split::Re::re"),
                miss("a::split::Mixed::mixed"),
                miss("a::split::Ghosty::ghosty"),
                miss("a::split::Cond::cond"),
                miss("a::split::Flip::flip"),
                miss("a::split::Set::step"),
                miss("a::split::Gseq::step"),
                miss("a::split::T::t"),
                miss("a::split::U::u"),
                miss("a::split::V::v"),
                miss("a::opaque::Made::made"),
                miss("a::imp::Foo::foo"),
                miss("a::tl::Tl::tl"),
                miss("a::tl3::Tl3::tl3"),
                miss("a::tl4::Tl4::tl4"),
            ],
        ),
        (
            "",
            "",
            ["", "", "", "", "", "", tl5],
            [&placed[..], &[("a::state::Tl5::tl5", 1)][..]].concat(),
            2,
            vec![miss("a::tl5::Tl5::tl5")],
        ),
        // tl2's `macro_rules! thread_local` makes every bare
        // `thread_local!` in its crate not std's, so it runs alone too.
        (
            "",
            "",
            ["", "", "", tl2, "", "", ""],
            [&placed[..], &[("a::state::Tl2::tl2", 1)][..]].concat(),
            2,
            vec![miss("a::tl2::Tl2::tl2")],
        ),
    ] {
        let dir = crate_of(
            "rlimit-type-module",
            &[
                ("lib.rs", lib),
                ("state.rs", state),
                ("other.rs", other),
                ("prelude.rs", prelude),
                ("b.rs", b),
                ("alt.rs", alt),
                ("alt2.rs", alt2),
                ("x.rs", x),
                ("state2.rs", state2),
                ("opaque.rs", apart[0]),
                ("imp.rs", apart[1]),
                ("tl.rs", apart[2]),
                ("tl2.rs", apart[3]),
                ("tl3.rs", apart[4]),
                ("tl4.rs", apart[5]),
                ("tl5.rs", apart[6]),
                ("split.rs", &split(uses, extra)),
            ],
        );
        fs::write(
            dir.join("verus/count.toml"),
            format!("{PLAIN}[rlimit]\npath = \"verus/RLIMIT\"\n"),
        )
        .expect("write");
        fs::write(dir.join("verus/RLIMIT"), "max-share 65\n").expect("write");
        let stats = dir.join("stats.json");
        fs::write(&stats, stats_naming(&fns)).expect("write");
        let s = stats.to_string_lossy().into_owned();
        let (got, out, err) = count(&dir, &["--rlimit", &s]);
        for line in &lines {
            assert!(
                got == want && (out.contains(line) || err.contains(line)),
                "{extra:?}: got = exit {got}, {out:?}, {err:?}; wanted = exit {want}, {line:?}"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }
}

/// A method's limit is held to the base revision's under the name Verus
/// gives it, its type's module, on both sides: dropping a sub-default
/// `#[verifier::rlimit(5)]` from a method in an `impl` split from its type
/// raises it to the default, which needs an `rlimit-raised` line.
#[test]
fn a_split_impl_s_limit_is_held_to_the_base() {
    let lib = "mod state;\nmod split;\npub fn o() {}\n";
    let state = "pub struct S;\n";
    let split = |attr: &str| {
        format!(
            "use vstd::prelude::*;\nuse super::state::S;\nverus! {{\nimpl S {{\n{attr} pub fn f(&self) {{}}\n}}\n}}\n"
        )
    };
    let dir = crate_of(
        "split-rlimit-base",
        &[
            ("lib.rs", lib),
            ("state.rs", state),
            ("split.rs", &split("#[verifier::rlimit(5)]")),
        ],
    );
    fs::write(dir.join("verus/count.toml"), RATCHET_CONFIG).expect("write");
    fs::write(dir.join("verus/FLOOR"), RATCHET_FLOOR).expect("write");
    fs::write(dir.join("verus/RLIMIT"), RATCHET_RLIMIT).expect("write");
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "crates", "verus"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    let stats = dir.join("stats.json");
    fs::write(&stats, stats_naming(&[("a::state::S::f", 1_000_000)])).expect("write");
    for (case, attr, code, want) in [
        (
            "as committed",
            "#[verifier::rlimit(5)]",
            0,
            "ok rlimit headroom: 1 functions",
        ),
        (
            "dropped",
            "",
            1,
            "raises a::state::S::f's rlimit from 5 to 10 with no new `rlimit-raised a::state::S::f 10 <reason>` line",
        ),
    ] {
        fs::write(dir.join("crates/a/src/split.rs"), split(attr)).expect("write");
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

/// A run against a base that has verus/RLIMIT must hold the head to it:
/// without `--rlimit` it fails (exit 1), naming the base's file and
/// VERUS_RLIMIT. A base without the file, or `--no-base`, does not need
/// `--rlimit`.
#[test]
fn a_base_with_rlimit_needs_rlimit() {
    let lib = "use vstd::prelude::*;\nverus! {\npub fn f() {}\n}\n";
    let dir = crate_of("rlimit-required", &[("lib.rs", lib)]);
    fs::write(dir.join("verus/count.toml"), RATCHET_CONFIG).expect("write");
    fs::write(dir.join("verus/FLOOR"), RATCHET_FLOOR).expect("write");
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "crates", "verus"]);
    git(&dir, &["commit", "-q", "-m", "no RLIMIT"]);
    fs::write(dir.join("verus/RLIMIT"), RATCHET_RLIMIT).expect("write");
    git(&dir, &["add", "verus"]);
    git(&dir, &["commit", "-q", "-m", "RLIMIT"]);
    let stats = dir.join("stats.json");
    fs::write(&stats, stats_naming(&[("a::f", 1_000_000)])).expect("write");
    let (floor, s) = (
        dir.join("verus/FLOOR").to_string_lossy().into_owned(),
        stats.to_string_lossy().into_owned(),
    );
    let needs = "FAIL rlimit headroom: HEAD:verus/RLIMIT holds each verified function to its rlimit share, but this run has no --rlimit";
    for (case, args, code, want) in [
        (
            "no --rlimit, base has RLIMIT",
            vec!["--floor", &floor, "--base-rev", "HEAD"],
            1,
            needs,
        ),
        (
            "--rlimit, base has RLIMIT",
            vec!["--floor", &floor, "--rlimit", &s, "--base-rev", "HEAD"],
            0,
            "ok rlimit headroom: 1 functions",
        ),
        (
            "no --rlimit, base without RLIMIT",
            vec!["--floor", &floor, "--base-rev", "HEAD^"],
            0,
            "ok coverage floor",
        ),
        (
            "no --rlimit, --no-base",
            vec!["--floor", &floor, "--no-base"],
            0,
            "ok coverage floor",
        ),
    ] {
        let (got, out, err) = count(&dir, &args);
        assert!(
            got == code
                && (out.contains(want) || err.contains(want))
                && (code == 0) != err.contains("VERUS_RLIMIT"),
            "{case}: got = exit {got}, {out:?}, {err:?}; wanted = exit {code}, {want:?}"
        );
    }
    // The head moves `[rlimit] path` and drops --rlimit: the run is held
    // to the path the base's configuration gives, which the base has.
    fs::write(
        dir.join("verus/count.toml"),
        RATCHET_CONFIG.replace("verus/RLIMIT", "verus/RLIMIT2"),
    )
    .expect("write");
    let (got, out, err) = count(&dir, &["--floor", &floor, "--base-rev", "HEAD"]);
    assert!(
        got == 1 && err.contains(needs) && err.contains("VERUS_RLIMIT"),
        "moved [rlimit] path, no --rlimit: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {needs:?}"
    );
    let _ = fs::remove_dir_all(&dir);
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
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n[package.metadata.verus]\nverify = true\n",
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
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n[package.metadata.verus]\nverify = true\n",
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

/// A zlib port's `export!`, `forward!` and `twins!` shapes (one definition each,
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
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\n[package.metadata.verus]\nverify = true\n",
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

/// The nested-loop check (`[loops]`, #76): with the section, a shipped
/// function that nests a loop, or calls one that loops from inside a
/// loop, fails the run until the table names its file and quotes
/// `fn NAME`; a crate that does not ship is not read; without the section
/// nothing is checked.
#[test]
fn a_nested_loop_needs_a_work_bound_row() {
    let lib = "pub fn f(a: &[u8]) -> usize {\n    let mut n = 0;\n    for i in 0..a.len() {\n        for _ in 0..i {\n            n += 1;\n        }\n    }\n    n\n}\nfn g() { for _ in 0..2 {} }\npub fn h() {\n    loop { g(); break; }\n}\n";
    let dir = crate_of("loops", &[("lib.rs", lib)]);
    let (got, _, err) = count(&dir, &[]);
    assert!(
        got == 0 && !err.contains("FAIL loops"),
        "off: got = exit {got}, {err:?}"
    );
    fs::write(
        dir.join("verus/count.toml"),
        format!("{PLAIN}[loops]\ntable = \"verus/WORK1.md\"\n"),
    )
    .expect("write");
    let (got, _, err) = count(&dir, &[]);
    assert!(
        got == 1
            && err.contains("FAIL loops: crates/a/src/lib.rs:1: `f` has a loop at line 4 inside another")
            && err.contains("FAIL loops: crates/a/src/lib.rs:11: `h` has a call to `g`, which loops, inside the loop at line 12")
            && err.contains("verus/WORK1.md's table has no row naming its file and quoting `fn f`"),
        "no rows: got = exit {got}, {err:?}"
    );
    // One row: the other still fails.
    fs::write(
        dir.join("verus/WORK1.md"),
        "| function | file | bound |\n|---|---|---|\n| `fn f` | `crates/a/src/lib.rs` | n^2 |\n",
    )
    .expect("write");
    let (got, _, err) = count(&dir, &[]);
    assert!(
        got == 1 && !err.contains("`f` has") && err.contains("`h` has"),
        "one row: got = exit {got}, {err:?}"
    );
    fs::write(
        dir.join("verus/WORK1.md"),
        "| function | file | bound |\n|---|---|---|\n| `fn f` | `crates/a/src/lib.rs` | n^2 |\n| `fn h` | `crates/a/src/lib.rs` | 2 |\n",
    )
    .expect("write");
    let (got, _, err) = count(&dir, &[]);
    assert!(
        got == 0 && !err.contains("FAIL loops"),
        "both rows: got = exit {got}, {err:?}"
    );
    // A crate that does not ship is not read.
    fs::write(dir.join("verus/WORK1.md"), "").expect("write");
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n[package.metadata.verus]\nverify = true\n",
    )
    .expect("write");
    let (got, _, err) = count(&dir, &[]);
    assert!(
        got == 0 && !err.contains("FAIL loops"),
        "unshipped: got = exit {got}, {err:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// `[floor] room_functions` and `room_lines` (#100): the figure a raise
/// records leaves that room for unverified code, or no raise where the
/// room's share is not above the floor; the pass is the share's alone.
#[test]
fn a_floor_raise_leaves_the_configured_room() {
    // Three verified functions of one line each, one outside `verus!`.
    let lib = "use vstd::prelude::*;\nverus! {\npub fn a() {}\npub fn b() {}\npub fn c() {}\n}\npub fn o() {}\n";
    let dir = crate_of("floor-room", &[("lib.rs", lib)]);
    fs::write(
        dir.join("verus/FLOOR"),
        "functions 50.0\nlines 50.0\nchecked-functions 50.0\nchecked-lines 50.0\n",
    )
    .expect("write");
    fs::create_dir_all(dir.join("verus/ledger")).expect("mkdir");
    fs::write(dir.join("verus/ledger/lib.rs.md"), "").expect("write");
    let run = |room: &str| {
        fs::write(
            dir.join("verus/count.toml"),
            format!("{PLAIN}[floor]\npath = \"verus/FLOOR\"\n{room}"),
        )
        .expect("write");
        let floor = dir.join("verus/FLOOR");
        count(&dir, &["--floor", &floor.to_string_lossy(), "--no-base"])
    };
    // No room: the share itself, 3 of 4.
    let (got, out, err) = run("");
    assert!(
        got == 0 && out.contains("raise verus/FLOOR to `functions 75.0`"),
        "no room: got = exit {got}, {out:?}, {err:?}"
    );
    // Room for 1 function: 3 of 5.
    let (got, out, err) = run("room_functions = 1\n");
    assert!(
        got == 0 && out.contains("raise verus/FLOOR to `functions 60.0`"),
        "room 1: got = exit {got}, {out:?}, {err:?}"
    );
    // Room for 4: 3 of 8, not above 50.0: no raise, and still a pass.
    let (got, out, err) = run("room_functions = 4\nroom_lines = 0\n");
    assert!(
        got == 0
            && out.contains("functions verified 75.0%, above the floor's 50.0% but within [floor]'s room for unverified code (37.5%): no raise")
            && !out.contains("raise verus/FLOOR to `functions"),
        "room 4: got = exit {got}, {out:?}, {err:?}"
    );
    // `room_lines` from the configuration: the lines are one each, so 3 of
    // 5 lines.
    let (got, out, err) = run("room_lines = 1\n");
    assert!(
        got == 0
            && out.contains("raise verus/FLOOR to `lines 60.0`")
            && out.contains("raise verus/FLOOR to `functions 75.0`"),
        "room_lines 1: got = exit {got}, {out:?}, {err:?}"
    );
    // A port with `[ledger]` reports through the ledger's tables: the room
    // holds there too.
    let (got, out, err) =
        run("room_functions = 1\nroom_lines = 1\n[ledger]\ndir = \"verus/ledger\"\n");
    assert!(
        got == 0
            && out.contains("raise verus/FLOOR to `functions 60.0`")
            && out.contains("raise verus/FLOOR to `lines 60.0`"),
        "ledger: got = exit {got}, {out:?}, {err:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A crate `a` with `files` (path under `src/` and text) and `PLAIN`.
fn crate_of(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("verus-count-cli-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("crates/a/src")).expect("mkdir");
    fs::create_dir_all(dir.join("verus")).expect("mkdir");
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[package.metadata.verus]\nverify = true\n",
    )
    .expect("write");
    fs::write(dir.join("verus/count.toml"), PLAIN).expect("write");
    for (path, text) in files {
        fs::write(dir.join("crates/a/src").join(path), text).expect("write");
    }
    dir
}

/// `assume_unsupported_attributes` trusts an export's linkage, not its
/// body: both exports stay verified, the report counts the ledger's
/// `unsupported-attribute` seams (a static's included) and lists each
/// function, a function without an entry is a warning, and an entry whose
/// status is not `trusted` fails the run.
#[test]
fn an_export_s_linkage_is_a_seam_of_its_own() {
    let lib = "verus! {\n#[unsafe(no_mangle)]\n#[verifier::assume_unsupported_attributes]\npub extern \"C\" fn e1() {}\n#[unsafe(no_mangle)]\n#[verifier::assume_unsupported_attributes]\npub extern \"C\" fn e2() {}\n}\n";
    let dir = crate_of("unsupported-attribute", &[("lib.rs", lib)]);
    fs::write(
        dir.join("verus/count.toml"),
        format!("{PLAIN}[ledger]\ndir = \"verus/ledger\"\n"),
    )
    .expect("write");
    fs::create_dir_all(dir.join("verus/ledger")).expect("mkdir");
    let ledger = |status: &str| {
        format!(
            "crates/a/src/lib.rs:4. Kind: unsupported-attribute. Category: C ABI export.\nTool: ABI tests. Status: {status}\n\ncrates/a/src/lib.rs:20. Kind: unsupported-attribute. Category: C ABI export.\nTool: ABI tests. Status: trusted\n"
        )
    };
    fs::write(dir.join("verus/ledger/lib.rs.md"), ledger("trusted")).expect("write");
    let (got, out, err) = count(&dir, &["--list"]);
    let classes: Vec<String> = list_rows(&out).into_iter().map(|r| r.0).collect();
    assert!(
        got == 0 && classes == ["Verified", "Verified"],
        "classes: got = exit {got}, {classes:?}, {err:?}; wanted = exit 0, two Verified"
    );
    for line in [
        "unsupported-attribute seams (linkage trusted, bodies keep their class)\t2",
        "unsupported-attribute\tcrates/a/src/lib.rs:4",
        "unsupported-attribute\tcrates/a/src/lib.rs:7",
    ] {
        assert!(
            out.lines().any(|l| l == line),
            "summary: got = {out:?}, wanted = a line {line:?}"
        );
    }
    let warning = "warning: function at crates/a/src/lib.rs:7 assumes its attributes and has no unsupported-attribute ledger entry";
    assert!(
        err.lines().any(|l| l == warning),
        "warnings: got = {err:?}, wanted = {warning:?}"
    );
    fs::write(
        dir.join("verus/ledger/lib.rs.md"),
        ledger("discharged (ABI tests)"),
    )
    .expect("write");
    let (got, _, err) = count(&dir, &[]);
    assert!(
        got == 2
            && err.contains(
                "crates/a/src/lib.rs:4: an unsupported-attribute seam's status is `trusted`"
            ),
        "a discharged entry: got = exit {got}, {err:?}; wanted = exit 2 naming the entry"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// An export a macro writes is listed once, at the invocation's line (the
/// line a ledger entry for it takes), also when the walk first compares
/// two candidate definitions under `cfg`s the counter cannot settle.
#[test]
fn an_export_a_macro_writes_is_listed_at_its_invocation() {
    let def = "macro_rules! export { ($n:ident) => { verus! {\n#[unsafe(no_mangle)]\n#[verifier::assume_unsupported_attributes]\npub extern \"C\" fn $n() {}\n} }; }\n";
    let lib = format!("#[cfg(unix)]\n{def}#[cfg(not(unix))]\n{def}export!(e1);\nexport!(e2);\n");
    let dir = crate_of("unsupported-attribute-macro", &[("lib.rs", &lib)]);
    let (got, out, err) = count(&dir, &[]);
    let lines: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("unsupported-attribute"))
        .collect();
    let want = [
        "unsupported-attribute seams (linkage trusted, bodies keep their class)\t2",
        "unsupported-attribute\tcrates/a/src/lib.rs:13",
        "unsupported-attribute\tcrates/a/src/lib.rs:14",
    ];
    assert!(
        got == 0 && lines == want,
        "summary: got = exit {got}, {lines:?}, {err:?}; wanted = exit 0, {want:?}"
    );
    let _ = fs::remove_dir_all(&dir);
    // Candidates that differ only in the attribute, either one carrying
    // it, count differently: the run fails rather than report one
    // build's linkage seams and drop the other's.
    let bare = def.replace("#[verifier::assume_unsupported_attributes]\n", "");
    for (unix, other) in [(def, bare.as_str()), (bare.as_str(), def)] {
        let lib = format!("#[cfg(unix)]\n{unix}#[cfg(not(unix))]\n{other}export!(e1);\n");
        let dir = crate_of("unsupported-attribute-macro-differs", &[("lib.rs", &lib)]);
        let (got, _, err) = count(&dir, &[]);
        assert!(
            got != 0 && err.contains("export! could be any of 2 definitions"),
            "differing candidates: got = exit {got}, {err:?}; wanted = a failure naming export!"
        );
        let _ = fs::remove_dir_all(&dir);
    }
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

/// challoc's `verified_export_pair!` shape: a macro that writes its
/// `extern "C"` pairs into `::vstd::prelude::verus! { .. }`.
const EXPORT_PAIR: &str = "use vstd::prelude::*;
macro_rules! verified_export_pair {
    ($($name:ident / $je:ident($($arg:ident: $ty:ty),*) { $($body:tt)* })*) => {
        ::vstd::prelude::verus! {
        $(
            #[unsafe(no_mangle)]
            pub extern \"C\" fn $name($($arg: $ty),*) { $($body)* }
            #[unsafe(no_mangle)]
            pub extern \"C\" fn $je($($arg: $ty),*) { $($body)* }
        )*
        } // verus!
    };
}
verified_export_pair! {
    free / je_free(p: usize) { let _ = p; }
    sdallocx / je_sdallocx(p: usize, n: usize) { let _ = (p, n); }
}
::vstd::prelude::verus! { pub fn b() {} }
::verus_builtin_macros::verus! { pub fn c() {} }
::verus_builtin_macros::verus! { pub fn d() {} }
verus! { pub fn e() {} }
other::verus! { pub fn f() {} }
::verus! { pub fn g() {} }
::vstd::prelude::verus! { macro_rules! inside { () => { pub fn h() {} }; } }
inside!();
vstd::prelude::verus! { pub mod inner; }
";

/// `verus!` is Verus's by any path that names it, written by the crate
/// or by a macro it expands (challoc's `verified_export_pair!`), so the
/// functions in it are verified, a `macro_rules!` in it defines its macro
/// (`inside!`), and a module it declares is read; another crate's
/// `other::verus!` and `::verus!` each count as one function outside. A
/// `verus!` that writes a function exits 1 by any path without a leading
/// `::`; with one, where a root `extern crate .. as` of the name (in the
/// sources, or in either candidate of a macro's expansion), edition 2015
/// or a renamed dependency may make it another; bare, where a binding of
/// `vstd`, `verus_builtin_macros` or `verus` or another crate's
/// `#[macro_use]` may. Every shape the review rounds named is a row; with
/// a leading `::`, the crate's items, `use`s and `*`s change nothing, nor
/// do test-only bindings, nor anything for a block that writes no
/// function.
#[test]
fn a_path_qualified_verus_is_verus() {
    let dir = crate_of(
        "verus-path",
        &[
            ("lib.rs", EXPORT_PAIR),
            ("inner.rs", "pub fn i() {}\nverus! { pub fn j() {} }\n"),
        ],
    );
    let (got, out, err) = count(&dir, &["--list"]);
    let rows: Vec<(String, String)> = list_rows(&out)
        .into_iter()
        .map(|(class, _, name, _)| (class, name))
        .collect();
    let want: Vec<(String, String)> = [
        ("Outside", "i"),
        ("Verified", "j"),
        ("Verified", "free"),
        ("Verified", "je_free"),
        ("Verified", "sdallocx"),
        ("Verified", "je_sdallocx"),
        ("Verified", "b"),
        ("Verified", "c"),
        ("Verified", "d"),
        ("Verified", "e"),
        ("Outside", "verus!"),
        ("Outside", "verus!"),
        ("Outside", "h"),
    ]
    .iter()
    .map(|(c, n)| ((*c).to_owned(), (*n).to_owned()))
    .collect();
    assert!(
        got == 0 && rows == want,
        "rows: got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
    );
    let lib = dir.join("crates/a/src/lib.rs");
    fs::remove_file(dir.join("crates/a/src/inner.rs")).expect("remove");
    const SHADOWED: &str = "may not be Verus's `verus!`";
    const EMITTED: &str = "may make this macro another than the Verus one";
    #[rustfmt::skip]
    let cases = [
        // A bare `verus!`: a crate `macro_rules! verus`, a `use` of another
        // `verus`, a binding of `vstd`, a `#[macro_use]` of another crate
        // (directly or through a `cfg_attr`).
        ("macro_rules! verus { ($($t:tt)*) => { $($t)* }; }\nverus! { pub fn f() {} }\n", SHADOWED),
        ("use other::verus;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("use other as vstd;\nuse vstd::prelude::verus;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("use other as vstd;\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("#[macro_use]\nextern crate other;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("#[cfg_attr(unix, macro_use)]\nextern crate other;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("extern crate other as vstd;\nmod m { use vstd::prelude::verus; verus! { pub fn f() {} } }\n", SHADOWED),
        // Another crate's `*` in the module, which may bring its own
        // `verus`, with no import of Verus's there to settle the name (one
        // in another module, or a `#[macro_use]`, does not).
        ("use other::*;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("#[macro_use]\nextern crate vstd;\nuse other::*;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("use vstd::prelude::*;\nmod m { use other::*; verus! { pub fn f() {} } }\n", SHADOWED),
        ("#[cfg(verus_keep_ghost)]\nuse other::*;\nverus! { pub fn f() {} }\n", SHADOWED),
        // An import of Verus's that Verus's build only may compile settles
        // nothing.
        ("use other::*;\n#[cfg(feature = \"x\")]\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("use other::*;\n#[cfg(feature = \"x\")]\nuse vstd::prelude::verus;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("use other::*;\n#[cfg(unix)]\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("mod q { pub use other::*; #[cfg(feature = \"x\")] pub use vstd::prelude::*; }\nuse q::*;\nverus! { pub fn f() {} }\n", SHADOWED),
        // A `*` of a module of the crate where such a `*` is.
        ("use other::*;\nmod s { use super::*; verus! { pub fn f() {} } }\n", SHADOWED),
        ("mod q { pub use other::*; }\nuse crate::q::*;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("mod q { pub use self::r::*; mod r { pub use other::*; } }\nuse q::*;\nverus! { pub fn f() {} }\n", SHADOWED),
        // Only `use super::*;` is followed: any other `*` of a module of the
        // crate settles nothing, even one whose module imports Verus's.
        ("mod q { pub use vstd::prelude::*; }\nuse q::*;\nverus! { pub fn f() {} }\n", SHADOWED),
        // A `::` glob names an extern crate, whatever module of the crate
        // shares its name.
        ("mod other {}\nuse ::other::*;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("mod other { pub use vstd::prelude::*; }\nuse ::other::*;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("mod s { mod other {} use ::other::*; verus! { pub fn f() {} } }\n", SHADOWED),
        // A path: another crate's `*`, a binding of `vstd` anywhere (a
        // module, in a `verus!` block too; a `use` or `extern crate`, at
        // the root or in another module; one a crate module's `*` carries),
        // a crate `macro_rules! verus`, a renamed `core` globbed, `::vstd`
        // beside any of them.
        ("use other::*;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("mod m { pub use other as vstd; }\nuse m::*;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("mod m { pub use n::*; mod n { pub mod vstd {} } }\nuse m::*;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("mod vstd { pub mod prelude {} }\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("use other as vstd;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("mod m { extern crate other as vstd; vstd::prelude::verus! { pub fn f() {} } }\n", SHADOWED),
        ("extern crate vstd;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("verus_builtin_macros::verus! { pub fn f() {} }\n", SHADOWED),
        // A leading `::` names an extern crate: only an `extern crate .. as`
        // of the name makes it another.
        ("extern crate other as vstd;\n::vstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("extern crate other as verus_builtin_macros;\n::verus_builtin_macros::verus! { pub fn f() {} }\n", SHADOWED),
        ("extern crate self as vstd;\n::vstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("extern crate other as vstd;\nmod m { ::vstd::prelude::verus! { pub fn f() {} } }\n", SHADOWED),
        ("extern crate other as vstd;\nmod m { vstd::prelude::verus! { pub fn f() {} } }\n", SHADOWED),
        ("mod m { use other as core; pub use core::*; }\nuse m::*;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("verus! { mod vstd { pub mod prelude {} } }\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("enum Op { vstd }\nuse Op::*;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        // A `::` glob names an extern crate, whatever module or type of the
        // crate shares its name.
        ("mod other {}\nuse ::other::*;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("enum other { A }\nuse ::other::*;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("mod hidden { enum other { A } }\nuse other::*;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("mod q { pub use other as bridge; }\nuse q::bridge::*;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        // Verus's build decides: a binding only it compiles counts.
        ("#[cfg(verus_keep_ghost)]\nuse other as vstd;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        ("#[cfg(verus_keep_ghost)]\nuse other::verus;\nverus! { pub fn f() {} }\n", SHADOWED),
        ("#[cfg(verus_keep_ghost)]\nuse other::*;\nvstd::prelude::verus! { pub fn f() {} }\n", SHADOWED),
        // What a macro expansion writes: a path without `::` as above, and
        // an `extern crate .. as vstd;` beside a `::vstd` block, before or
        // after it, in the same expansion or another, in either candidate
        // of a definition under a `cfg` the counter cannot settle.
        ("macro_rules! mk { () => { use other as vstd; vstd::prelude::verus! { pub fn f() {} } }; }\nmk!();\n", SHADOWED),
        ("macro_rules! mk { () => { mod vstd { pub mod prelude { pub use other::verus; } } vstd::prelude::verus! { pub fn f() {} } }; }\nmk!();\n", SHADOWED),
        ("macro_rules! mk { () => { mod m { use other as vstd; vstd::prelude::verus! { pub fn f() {} } } }; }\nmk!();\n", SHADOWED),
        ("macro_rules! mk { () => { extern crate other as vstd; ::vstd::prelude::verus! { pub fn f() {} } }; }\nmk!();\n", EMITTED),
        ("::vstd::prelude::verus! { pub fn f() {} }\nmacro_rules! mk { () => { extern crate other as vstd; }; }\nmk!();\n", EMITTED),
        ("#[cfg(unix)]\nmacro_rules! mk { () => { extern crate other as vstd; ::vstd::prelude::verus! { pub fn f() {} } }; }\n#[cfg(not(unix))]\nmacro_rules! mk { () => { ::vstd::prelude::verus! { pub fn f() {} } }; }\nmk!();\n", EMITTED),
        ("#[cfg(not(unix))]\nmacro_rules! mk { () => { ::vstd::prelude::verus! { pub fn f() {} } }; }\n#[cfg(unix)]\nmacro_rules! mk { () => { extern crate other as vstd; ::vstd::prelude::verus! { pub fn f() {} } }; }\nmk!();\n", EMITTED),
        ("verus! { pub fn f() {} }\nmacro_rules! mk { () => { use other::verus; }; }\nmk!();\n", EMITTED),
    ];
    for (src, want) in cases {
        fs::write(&lib, src).expect("write");
        let (got, out, err) = count(&dir, &[]);
        assert!(
            got == 1 && err.contains(want),
            "{src}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
        );
    }
    // A `verus!` that writes no function needs no precondition.
    fs::write(&lib, "use other::*;\nvstd::prelude::verus! { struct S; }\n").expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 0,
        "no function: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
    );
    // With a leading `::`, the crate's items and `*`s cannot make the path
    // another: rustc reads `::vstd` from the extern prelude alone.
    for src in [
        "extern crate vstd;\n::vstd::prelude::verus! { pub fn f() {} }\n",
        "mod vstd { pub mod prelude {} }\n::vstd::prelude::verus! { pub fn f() {} }\n",
        "macro_rules! verus { ($($t:tt)*) => { $($t)* }; }\n::vstd::prelude::verus! { pub fn f() {} }\n",
        "use other::*;\nuse other as vstd;\n::vstd::prelude::verus! { pub fn f() {} }\n",
        "mod other {}\nmod s { use other::*; ::vstd::prelude::verus! { pub fn f() {} } }\n",
        "mod hidden { enum other { A } }\nuse other::*;\n::vstd::prelude::verus! { pub fn f() {} }\n",
        "#[macro_use]\nextern crate other;\n::vstd::prelude::verus! { pub fn f() {} }\n",
        "#[cfg_attr(unix, macro_use)]\nextern crate core;\n::vstd::prelude::verus! { pub fn f() {} }\n",
        "#[macro_use]\nextern crate vstd;\nverus! { pub fn f() {} }\n",
        "#[macro_use]\nextern crate std;\nverus! { pub fn f() {} }\n",
        "use other::*;\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n",
        "use other::*;\nuse vstd::prelude::verus;\nverus! { pub fn f() {} }\n",
        "mod s { use other::*; }\nverus! { pub fn f() {} }\n",
        "use vstd::prelude::*;\nmod s { use super::*; verus! { pub fn f() {} } }\n",
        "use other::*;\nuse vstd::prelude::*;\nmod s { use super::*; verus! { pub fn f() {} } }\n",
        "use other::*;\n#[cfg(verus_keep_ghost)]\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n",
        "#[cfg(test)]\nuse other::*;\nverus! { pub fn f() {} }\n",
        "#[cfg(test)]\n#[macro_use]\nextern crate other;\nverus! { pub fn f() {} }\n",
        "#[cfg(test)]\nmacro_rules! verus { ($($t:tt)*) => { $($t)* }; }\nverus! { pub fn f() {} }\n",
        "#[cfg(test)]\nextern crate other as vstd;\n::vstd::prelude::verus! { pub fn f() {} }\n",
    ] {
        fs::write(&lib, src).expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        let rows: Vec<(String, String)> = list_rows(&out)
            .into_iter()
            .map(|(class, _, name, _)| (class, name))
            .collect();
        let want = vec![("Verified".to_owned(), "f".to_owned())];
        assert!(
            got == 0 && rows == want,
            "{src}: got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
        );
    }
    // In edition 2015 a leading `::` is this crate's root, and a
    // dependency of the name may be another package: each exits 1.
    // A target's own 2015 edition, a renamed dependency inherited from the
    // workspace, and (for a bare `verus!`, which comes from Verus's
    // crates) any renamed `vstd`: each exits 1. An inherited dependency of
    // the same package is Verus's.
    let manifest = dir.join("crates/a/Cargo.toml");
    let workspace = dir.join("Cargo.toml");
    let path_form = "::vstd::prelude::verus! { pub fn f() {} }\n";
    let package = "[package]\nname = \"a\"\nversion = \"0.0.0\"\n";
    #[rustfmt::skip]
    let cases = [
        ("", package.to_string(), path_form, Some("the crate's edition is 2015")),
        ("", format!("{package}edition = \"2015\"\n"), path_form, Some("the crate's edition is 2015")),
        ("", format!("{package}edition = \"2024\"\n[lib]\nedition = \"2015\"\n"), path_form, Some("the crate's edition is 2015")),
        ("", format!("{package}edition = \"2024\"\n[dependencies]\nvstd = {{ package = \"other\", path = \"x\" }}\n"), path_form, Some("Cargo.toml's dependency `vstd` is the package `other`")),
        ("", format!("{package}edition = \"2024\"\n[target.'cfg(unix)'.dependencies]\nvstd = {{ package = \"other\", path = \"x\" }}\n"), path_form, Some("Cargo.toml's dependency `vstd` is the package `other`")),
        ("[workspace.dependencies]\nvstd = { package = \"fakevstd\", path = \"fake\" }\n", format!("{package}edition = \"2024\"\n[dependencies]\nvstd = {{ workspace = true }}\n"), path_form, Some("Cargo.toml's dependency `vstd` is the package `fakevstd`")),
        ("[workspace.dependencies]\nvstd = { version = \"1\" }\n", format!("{package}edition.workspace = true\n[dependencies]\nvstd.workspace = true\n"), path_form, Some("the crate's edition is 2015")),
        ("[workspace.package]\nedition = \"2024\"\n[workspace.dependencies]\nvstd = { version = \"1\" }\n", format!("{package}edition.workspace = true\n[dependencies]\nvstd.workspace = true\n[package.metadata.verus]\nverify = true\n"), path_form, None),
        ("", format!("{package}edition = \"2024\"\n[dependencies]\nvstd = {{ package = \"other\", path = \"x\" }}\n"), "use vstd::prelude::*;\nverus! { pub fn f() {} }\n", Some("Cargo.toml's dependency `vstd` is the package `other`")),
    ];
    for (ws, toml, src, want) in cases {
        fs::write(&workspace, ws).expect("write");
        fs::write(&manifest, &toml).expect("write");
        fs::write(&lib, src).expect("write");
        let (got, out, err) = count(&dir, &[]);
        match want {
            Some(want) => assert!(
                got == 1 && err.contains(want),
                "{ws}{toml}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
            ),
            None => assert!(
                got == 0,
                "{ws}{toml}: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
            ),
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

/// With both of challoc's architectures in `verus_sets`, an import of
/// Verus's beside `use other::*;` settles a bare `verus!` only where
/// every proof run that compiles the module compiles the import. Stacked
/// guards for the two architectures (in either order, on the import,
/// through a `cfg_attr`, or one on an enclosing module) hold in no build.
/// One architecture's guard on the import itself, or on a `verus!` block
/// around it, holds in one run only, and the other run compiles the
/// invocation with `other`'s `verus`, as it does under a module guard
/// every run meets (`any(target_arch = "x86_64", not(target_arch =
/// "x86_64"))`), which forces no architecture. Each exits 1. An unguarded import,
/// one under an atom every run sets, and one whose module's guard forces
/// its own still settle the name. With both oses declared too, one os and
/// one architecture together need two runs' atoms, and settle nothing.
#[test]
fn a_confirming_import_holds_under_its_combined_guards() {
    let dir = crate_of("confirm-arches", &[("lib.rs", "")]);
    fs::write(
        dir.join("verus/count.toml"),
        format!("{PLAIN}verus_sets = ['target_arch = \"x86_64\"', 'target_arch = \"aarch64\"']\n"),
    )
    .expect("write");
    let lib = dir.join("crates/a/src/lib.rs");
    const SHADOWED: &str = "may not be Verus's `verus!`";
    #[rustfmt::skip]
    let never = [
        "use other::*;\n#[cfg(target_arch = \"x86_64\")]\n#[cfg(target_arch = \"aarch64\")]\nuse vstd::prelude::*;\nverus! { fn unproved() {} }\n",
        "use other::*;\n#[cfg(target_arch = \"x86_64\")]\n#[cfg(target_arch = \"aarch64\")]\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n",
        "use other::*;\n#[cfg(target_arch = \"aarch64\")]\n#[cfg(target_arch = \"x86_64\")]\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n",
        "use other::*;\n#[cfg(target_arch = \"x86_64\")]\n#[cfg(target_arch = \"aarch64\")]\nuse vstd::prelude::verus;\nverus! { pub fn f() {} }\n",
        "use other::*;\n#[cfg(target_arch = \"x86_64\")]\n#[cfg_attr(target_arch = \"x86_64\", cfg(target_arch = \"aarch64\"))]\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n",
        "#[cfg(target_arch = \"x86_64\")]\nmod m { use other::*; #[cfg(target_arch = \"aarch64\")] use vstd::prelude::*; verus! { pub fn f() {} } }\n",
        "#[cfg(target_arch = \"aarch64\")]\nmod m { use other::*; #[cfg(target_arch = \"x86_64\")] use vstd::prelude::*; verus! { pub fn f() {} } }\n",
        // One run's import beside an invocation every run compiles: the
        // other run takes `other`'s `verus`.
        "use other::*;\n#[cfg(target_arch = \"x86_64\")]\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n",
        "use other::*;\n#[cfg(target_arch = \"aarch64\")]\n#[cfg(target_arch = \"aarch64\")]\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n",
        "use other::*;\n#[cfg(target_arch = \"x86_64\")]\nuse vstd::prelude::verus;\nverus! { pub fn f() {} }\n",
        "mod m { use other::*; #[cfg(target_arch = \"x86_64\")] use vstd::prelude::*; verus! { pub fn f() {} } }\n",
        "use other::*;\n#[cfg(target_arch = \"x86_64\")]\nverus! { use vstd::prelude::*; }\nverus! { pub fn f() {} }\n",
        "mod q { pub use other::*; #[cfg(target_arch = \"x86_64\")] pub use vstd::prelude::*; }\nuse q::*;\nverus! { pub fn f() {} }\n",
        // A module guard that every run meets forces no architecture.
        "#[cfg(any(target_arch = \"x86_64\", not(target_arch = \"x86_64\")))]\nmod m {\n    use other::*;\n    #[cfg(target_arch = \"x86_64\")]\n    use vstd::prelude::verus;\n    verus! { pub fn f() {} }\n}\n",
        // Two inline modules of one name, one per architecture, share a
        // path but not an instance: one's import of Verus's does not settle
        // the other's `*`, which the other run compiles.
        "#[cfg(target_arch = \"x86_64\")]\nmod q { pub use vstd::prelude::*; }\n#[cfg(target_arch = \"aarch64\")]\nmod q { pub use other::*; }\nuse q::*;\nverus! { pub fn f() {} }\n",
        "#[cfg(target_arch = \"aarch64\")]\nmod q { pub use other::*; }\n#[cfg(target_arch = \"x86_64\")]\nmod q { pub use vstd::prelude::*; }\nuse q::*;\nverus! { pub fn f() {} }\n",
        "#[cfg(target_arch = \"x86_64\")]\nmod arch { use other::*; use vstd::prelude::*; }\n#[cfg(target_arch = \"aarch64\")]\nmod arch { use other::*; verus! { pub fn f() {} } }\n",
        // A `*` of a module of the crate is not followed, even where each
        // instance of that module imports Verus's: only `use super::*;` is.
        "#[cfg(target_arch = \"x86_64\")]\nmod q { pub use vstd::prelude::*; }\n#[cfg(target_arch = \"aarch64\")]\nmod q { pub use other::*; pub use vstd::prelude::*; }\nuse q::*;\nverus! { pub fn f() {} }\n",
    ];
    for src in never {
        fs::write(&lib, src).expect("write");
        let (got, out, err) = count(&dir, &[]);
        assert!(
            got == 1 && err.contains(SHADOWED),
            "{src}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {SHADOWED:?}"
        );
    }
    #[rustfmt::skip]
    let settled = [
        "use other::*;\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n",
        "use other::*;\n#[cfg(verus_keep_ghost)]\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n",
        "#[cfg(target_arch = \"x86_64\")]\nmod m { use other::*; use vstd::prelude::*; verus! { pub fn f() {} } }\n",
        "#[cfg(target_arch = \"x86_64\")]\nmod m { use other::*; #[cfg(target_arch = \"x86_64\")] use vstd::prelude::*; verus! { pub fn f() {} } }\n",
        "#[cfg(target_arch = \"x86_64\")]\nmod m { use other::*; verus! { use vstd::prelude::*; pub fn f() {} } }\n",
        "#[cfg(any(target_arch = \"x86_64\", test))]\nmod m { use other::*; #[cfg(target_arch = \"x86_64\")] use vstd::prelude::verus; verus! { pub fn f() {} } }\n",
        // Each instance of a same-named module settles its own `*`s.
        "#[cfg(target_arch = \"x86_64\")]\nmod arch { use other::*; use vstd::prelude::*; }\n#[cfg(target_arch = \"aarch64\")]\nmod arch { use other::*; use vstd::prelude::*; verus! { pub fn f() {} } }\n",
        // A `*` a macro writes is in the instance it is invoked in, which
        // that instance's import of Verus's settles: an inline `mod`, or
        // the file itself.
        "macro_rules! mk { () => { use other::*; }; }\n#[cfg(target_arch = \"x86_64\")]\nmod m { use vstd::prelude::*; mk!(); verus! { pub fn f() {} } }\n",
        "macro_rules! mk { () => { use other::*; }; }\nuse vstd::prelude::*;\nmk!();\nverus! { pub fn f() {} }\n",
    ];
    for src in settled {
        fs::write(&lib, src).expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        let rows: Vec<(String, String)> = list_rows(&out)
            .into_iter()
            .map(|(class, _, name, _)| (class, name))
            .collect();
        let want = vec![("Verified".to_owned(), "f".to_owned())];
        assert!(
            got == 0 && rows == want,
            "{src}: got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
        );
    }
    // The review's case through an expansion: the `*` a macro writes is
    // in the instance it is invoked in, the aarch64 `q` that holds the
    // invocation, not the x86_64 one beside it. The bindings an expansion
    // writes are held by their own check, which names the `*` as the
    // expansion's.
    const EMITTED: &str = "which a macro expansion writes) may bring another crate's `verus`";
    fs::write(
        &lib,
        "macro_rules! mk { () => { pub use other::*; }; }\n#[cfg(target_arch = \"x86_64\")]\nmod q { pub use vstd::prelude::*; }\n#[cfg(target_arch = \"aarch64\")]\nmod q { mk!(); verus! { pub fn f() {} } }\n",
    )
    .expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains(EMITTED),
        "an expansion's `*`: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {EMITTED:?}"
    );
    // A `mod` an expansion writes carries the spans of the macro's
    // definition: here `q` in m.rs, at line 4, column 12, where lib.rs's
    // own `mod q` also is. It is still an instance of its own, so the
    // source `q`'s import of Verus's does not settle the `*` beside the
    // invocation the expansion's `q` holds.
    const ANOTHER: &str = "may bring another crate's `verus`";
    let m = dir.join("crates/a/src/m.rs");
    fs::write(
        &m,
        "macro_rules! mk {\n    () => {\n        #[cfg(target_arch = \"aarch64\")]\n        mod q { pub use other::*; verus! { pub fn f() {} } }\n    };\n}\n",
    )
    .expect("write");
    fs::write(
        &lib,
        "#[macro_use]\nmod m;\n#[cfg(target_arch = \"x86_64\")]\n        mod q { pub use vstd::prelude::*; }\nmk!();\n",
    )
    .expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains(ANOTHER),
        "a mod an expansion writes: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {ANOTHER:?}"
    );
    fs::remove_file(&m).expect("remove");
    // A `*` of a module of the crate other than `use super::*;` (bare, in
    // edition 2015 or 2018 on, or through `crate::` or `self::`) is never
    // followed, so none settles: in some target or run it may name another
    // crate's module, or an alias of one, whose `verus` may be its own.
    // These are the inputs of the module-existence findings (a test-only,
    // one-architecture, undeclared or other-target module; a `#[path]`
    // file; a `crate::` alias) and the local globs that settled before.
    let src_dir = dir.join("crates/a/src");
    #[rustfmt::skip]
    let absent: [(&str, &[(&str, &str)]); 28] = [
        ("mod a { mod other; }\nuse a::other::*;\nverus! { pub fn f() {} }\n", &[("a/other.rs", "")]),
        ("#[cfg(target_arch = \"x86_64\")]\nmod m {\n    mod other {}\n    use other::*;\n    verus! { pub fn f() {} }\n}\n", &[]),
        ("#[cfg(target_arch = \"x86_64\")]\nmod m;\n", &[("m.rs", "mod other;\nuse other::*;\nverus! { pub fn f() {} }\n"), ("m/other.rs", "")]),
        ("mod other {}\nuse other::*;\nverus! { pub fn f() {} }\n", &[]),
        ("mod other;\nuse other::*;\nverus! { pub fn f() {} }\n", &[("other.rs", "")]),
        ("#[cfg(target_arch = \"x86_64\")]\nmod other {}\n#[cfg(target_arch = \"aarch64\")]\nmod other {}\nuse other::*;\nverus! { pub fn f() {} }\n", &[]),
        ("mod other;\nuse other::*;\nverus! { pub fn f() {} }\n", &[("other.rs", ""), ("main.rs", "mod other;\n")]),
        ("mod p;\nuse p::other::*;\nverus! { pub fn f() {} }\n", &[("p.rs", "pub mod other {}\n")]),
        // A main.rs that is a module (`a::main`, from src/main.rs) has its
        // `mod y;` at src/a/main/y.rs, which the library reaches as an
        // inline `a::main::y`; and a `#[path]`-named p.rs that src/main.rs
        // declares by its place has its `mod z;` at src/p/z.rs. In both,
        // main's `crate::q` is `otherdep`.
        ("mod a;\nmod q {}\n", &[("a.rs", "mod main { mod y; }\n"), ("main.rs", "mod a { mod main; }\nuse otherdep as q;\nfn main() {}\n"), ("a/main.rs", "mod y;\n"), ("a/main/y.rs", "use crate::q::*;\nverus! { pub fn f() {} }\n")]),
        ("#[path = \"p.rs\"]\nmod imp;\nmod p { mod z; }\nmod q {}\n", &[("main.rs", "mod p;\nuse otherdep as q;\nfn main() {}\n"), ("p.rs", "mod z;\n"), ("p/z.rs", "use crate::q::*;\nverus! { pub fn f() {} }\n")]),
        // A file another target also compiles is checked there too: here
        // src/main.rs compiles lib.rs as `crate::lib`, where `crate::q`
        // is `otherdep`; and the library compiles src/bin/foo.rs as
        // `bin::foo`, which the bin target compiles as its root.
        ("mod q {}\nuse crate::q::*;\nverus! { pub fn f() {} }\n", &[("main.rs", "mod lib;\nuse otherdep as q;\n")]),
        ("mod bin { mod foo; }\n", &[("bin/foo.rs", "mod foo { mod other; }\nuse other::*;\nverus! { pub fn f() {} }\n"), ("bin/foo/other.rs", ""), ("bin/foo/foo/other.rs", "")]),
        // A module file named main.rs or lib.rs is `a::main` or `a::lib`,
        // where no `other` is, though its place under src/ reads as `a`.
        ("mod a;\n", &[("a.rs", "mod main;\nmod other {}\n"), ("a/main.rs", "use other::*;\nverus! { pub fn f() {} }\n")]),
        ("mod a;\n", &[("a.rs", "mod lib;\nmod other {}\n"), ("a/lib.rs", "use other::*;\nverus! { pub fn f() {} }\n")]),
        // A src/bin root names its modules from its own `crate::`: here
        // `crate::foo::other`, while src/bin/foo/other.rs reads as
        // `bin::foo::other`.
        ("", &[("bin/foo.rs", "mod foo { mod other; }\nuse other::*;\nverus! { pub fn f() {} }\n"), ("bin/foo/other.rs", "")]),
        // A file declared in an inline module whose directory a path
        // attribute chose holds `a::other`, not the `foo::other` its
        // location would.
        ("#[path = \"foo\"]\nmod a { mod other; }\nuse foo::other::*;\nverus! { pub fn f() {} }\n", &[("foo/other.rs", "")]),
        ("#[cfg_attr(not(test), path = \"foo\")]\nmod a { mod other; }\nuse foo::other::*;\nverus! { pub fn f() {} }\n", &[("foo/other.rs", "")]),
        // A `#[path]` file holds the module its declaration names (`imp`),
        // not the one its location would.
        ("#[path = \"other.rs\"]\nmod imp;\nuse other::*;\nverus! { pub fn f() {} }\n", &[("other.rs", "")]),
        // Through `crate::`, a test-only `mod q` leaves `q` the extern
        // crate's alias in the proof build.
        ("#[cfg(not(test))]\npub use other as q;\n#[cfg(test)]\nmod q {}\nuse crate::q::*;\nverus! { pub fn f() {} }\n", &[]),
        ("#[cfg(test)]\nmod other {}\nuse other::*;\nverus! { pub fn f() {} }\n", &[]),
        ("#[cfg(target_arch = \"x86_64\")]\nmod other {}\nuse other::*;\nverus! { pub fn f() {} }\n", &[]),
        ("#[cfg(test)]\nmod other;\nuse other::*;\nverus! { pub fn f() {} }\n", &[("other.rs", "")]),
        ("use other::*;\nverus! { pub fn f() {} }\n", &[("other.rs", "")]),
        ("mod other;\nuse other::*;\nverus! { pub fn f() {} }\n", &[("other.rs", "#![cfg(test)]\n")]),
        ("mod other;\nuse other::*;\nverus! { pub fn f() {} }\n", &[("other.rs", "#![cfg(target_arch = \"x86_64\")]\n")]),
        ("use other::*;\nverus! { pub fn f() {} }\n", &[("other.rs", ""), ("main.rs", "mod other;\n")]),
        ("use p::other::*;\nverus! { pub fn f() {} }\n", &[("p.rs", "pub mod other {}\n")]),
        ("mod p;\nuse p::other::*;\nverus! { pub fn f() {} }\n", &[("p.rs", "#[cfg(test)]\npub mod other {}\n")]),
    ];
    for (src, files) in absent {
        for (name, text) in files {
            let path = src_dir.join(name);
            fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            fs::write(path, text).expect("write");
        }
        fs::write(&lib, src).expect("write");
        let (got, out, err) = count(&dir, &[]);
        assert!(
            got == 1 && err.contains(SHADOWED),
            "{src} ({files:?}): got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {SHADOWED:?}"
        );
        for (name, _) in files {
            fs::remove_file(src_dir.join(name)).expect("remove");
        }
    }
    // `use super::*;` is followed only where its file surely is the module
    // its place under src/ names, so that place gives its parent. Each of
    // these places the `*` in a file whose module, in some target, is not
    // the one its place names, with that target's real parent holding
    // another crate's `*` and the parent its place names holding none:
    // - a `#[path]`-named file (inner.rs is `imp`, its parent the root);
    // - a file declared in an inline module whose directory a `#[path]`
    //   chose (inner.rs is `a::inner`, its parent `a`);
    // - a file another target reads under the other reading of where a
    //   main.rs's modules live (y.rs is main's `a::main::y`), with each
    //   declaration under one architecture;
    // - a file another target reaches inside an inline module whose
    //   directory a `#[path]` chose (x.rs is main's `m::x`);
    // - a `#[path]`-named file only the other reading reaches, whose own
    //   `mod y;` is beside it (y.rs is main's `a::main::m::x::y`);
    // - a file one root declares twice, by its place and inside an inline
    //   module whose directory a `#[path]` chose (inner.rs is both
    //   `foo::inner` and `a::inner`), or by its place and under the other
    //   reading (y.rs is both an inline `a::main`'s and a/main.rs's), each
    //   compiled once per declaration;
    // - a file a macro in a mod.rs (which has no other reading) declares
    //   inside an inline module whose directory a `#[path]` chose
    //   (inner.rs is `p::a::inner`), or a mod.rs's `#[path]`-named file
    //   (inner.rs is `p::imp`);
    // - a file declared both by its place and from a file whose own
    //   module its place does not name (leaf.rs is both `foo::leaf` and
    //   the `#[path]`-named `a`'s `a::leaf`), or two levels down (mid.rs
    //   is both `foo::mid` and `a::mid`, so leaf.rs is `a::mid::leaf` too).
    #[rustfmt::skip]
    let uncertain: [(&str, &[(&str, &str)]); 11] = [
        ("use other::*;\n#[path = \"x/inner.rs\"]\nmod imp;\nmod x;\n", &[("x.rs", "use vstd::prelude::*;\n"), ("x/inner.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
        ("#[path = \"foo\"]\nmod a { use other::*; mod inner; }\nmod foo;\n", &[("foo.rs", "use vstd::prelude::*;\n"), ("foo/inner.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
        ("mod a;\n", &[("a.rs", "#[cfg(target_arch = \"x86_64\")]\nmod main { mod y; }\n"), ("main.rs", "#[cfg(target_arch = \"aarch64\")]\nmod a { mod main; }\n"), ("a/main.rs", "use other::*;\nmod y;\n"), ("a/main/y.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
        ("mod x;\n", &[("main.rs", "#[path = \".\"]\nmod m { use other::*; mod x; }\n"), ("x.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
        ("mod a;\n", &[("a.rs", "mod main { mod m { mod y; } }\n"), ("main.rs", "mod a { mod main; }\n"), ("a/main.rs", "mod m { #[path = \"foo.rs\"] mod x; }\n"), ("a/main/m/foo.rs", "use other::*;\nmod y;\n"), ("a/main/m/y.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
        ("#[path = \"foo\"]\nmod a { use other::*; mod inner; }\nmod foo;\n", &[("foo.rs", "use vstd::prelude::*;\nmod inner;\n"), ("foo/inner.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
        ("mod a;\n", &[("a.rs", "#[cfg(target_arch = \"x86_64\")]\nmod main;\n#[cfg(target_arch = \"aarch64\")]\nmod main { use vstd::prelude::*; mod y; }\n"), ("a/main.rs", "use other::*;\nmod y;\n"), ("a/main/y.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
        ("mod p;\n", &[("p/mod.rs", "macro_rules! mk { () => { #[path = \"foo\"] mod a { use other::*; mod inner; } }; }\nmk!();\nmod foo;\n"), ("p/foo.rs", "use vstd::prelude::*;\n"), ("p/foo/inner.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
        ("mod p;\n", &[("p/mod.rs", "use other::*;\n#[path = \"x/inner.rs\"]\nmod imp;\nmod x;\n"), ("p/x.rs", "use vstd::prelude::*;\n"), ("p/x/inner.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
        ("#[path = \"foo/strange.rs\"]\nmod a;\nmod foo;\n", &[("foo.rs", "use vstd::prelude::*;\nmod leaf;\n"), ("foo/strange.rs", "use other::*;\nmod leaf;\n"), ("foo/leaf.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
        ("#[path = \"foo/strange.rs\"]\nmod a;\nmod foo;\n", &[("foo.rs", "use vstd::prelude::*;\nmod mid;\n"), ("foo/strange.rs", "use other::*;\nmod mid;\n"), ("foo/mid.rs", "use super::*;\nmod leaf;\n"), ("foo/mid/leaf.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
    ];
    for (src, files) in uncertain {
        for (name, text) in files {
            let path = src_dir.join(name);
            fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            fs::write(path, text).expect("write");
        }
        fs::write(&lib, src).expect("write");
        let (got, out, err) = count(&dir, &[]);
        assert!(
            got == 1 && err.contains(SHADOWED),
            "{src} ({files:?}): got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {SHADOWED:?}"
        );
        for (name, _) in files {
            fs::remove_file(src_dir.join(name)).expect("remove");
        }
    }
    // Where every target names the file by its place, `use super::*;`
    // settles through the parent's import of Verus's: a file the library
    // alone declares, or one the library and src/main.rs both declare. A
    // local module's `*` still settles beside an import of Verus's in the
    // same instance, as any `*` does.
    #[rustfmt::skip]
    let certain: [(&str, &[(&str, &str)]); 3] = [
        ("mod other {}\nuse other::*;\nuse vstd::prelude::verus;\nverus! { pub fn f() {} }\n", &[]),
        ("use vstd::prelude::*;\nmod x;\n", &[("x.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
        ("use vstd::prelude::*;\nmod x;\n", &[("main.rs", "use vstd::prelude::*;\nmod x;\n"), ("x.rs", "use super::*;\nverus! { pub fn f() {} }\n")]),
    ];
    for (src, files) in certain {
        for (name, text) in files {
            let path = src_dir.join(name);
            fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            fs::write(path, text).expect("write");
        }
        fs::write(&lib, src).expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        let rows: Vec<(String, String)> = list_rows(&out)
            .into_iter()
            .map(|(class, _, name, _)| (class, name))
            .collect();
        let want = vec![("Verified".to_owned(), "f".to_owned())];
        assert!(
            got == 0 && rows == want,
            "{src} ({files:?}): got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
        );
        for (name, _) in files {
            fs::remove_file(src_dir.join(name)).expect("remove");
        }
    }
    // In edition 2015 a bare path is from the crate root, and names the
    // crate's own module only where every proof run has it: a stray
    // src/other.rs no `mod` declares leaves `use other::*;` the extern
    // crate's.
    let manifest = dir.join("crates/a/Cargo.toml");
    let edition = fs::read_to_string(&manifest).expect("read");
    fs::write(
        &manifest,
        edition.replace("edition = \"2024\"", "edition = \"2015\""),
    )
    .expect("write");
    fs::write(src_dir.join("other.rs"), "").expect("write");
    fs::write(
        &lib,
        "extern crate other;\nuse other::*;\nverus! { pub fn f() {} }\n",
    )
    .expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains(SHADOWED),
        "edition 2015, a stray src/other.rs: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {SHADOWED:?}"
    );
    fs::remove_file(src_dir.join("other.rs")).expect("remove");
    // A `[[bin]]` root at src/tool.rs gets no sure tree, so a file it and
    // the library both declare is checked against it: there `crate::q` is
    // `otherdep`, whatever the library's `mod q` is.
    fs::write(
        &manifest,
        format!("{edition}[[bin]]\nname = \"tool\"\npath = \"src/tool.rs\"\n"),
    )
    .expect("write");
    fs::write(src_dir.join("tool.rs"), "mod x;\nuse otherdep as q;\n").expect("write");
    fs::write(
        src_dir.join("x.rs"),
        "use crate::q::*;\nverus! { pub fn f() {} }\n",
    )
    .expect("write");
    fs::write(&lib, "mod x;\nmod q {}\n").expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains(SHADOWED),
        "a [[bin]] at src/tool.rs sharing src/x.rs: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {SHADOWED:?}"
    );
    // The same file's `use super::*;` is the tool root's `*` in that
    // target, so it settles nothing, though in the library it is the
    // root's and the root holds no `*`: the tool root names its modules
    // from its own `crate::`, and so names no file by its place.
    fs::write(src_dir.join("tool.rs"), "use other::*;\nmod x;\n").expect("write");
    fs::write(
        src_dir.join("x.rs"),
        "use super::*;\nverus! { pub fn f() {} }\n",
    )
    .expect("write");
    fs::write(&lib, "mod x;\n").expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains(SHADOWED),
        "`use super::*;` in src/x.rs, shared with a [[bin]] root: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {SHADOWED:?}"
    );
    // The library reads the root src/tool.rs as `mod tool;`, whose `mod x;`
    // is src/tool/x.rs there; src/x.rs is the tool target's alone, where
    // its `super` is the tool root and its `*`.
    fs::write(&lib, "mod tool;\n").expect("write");
    fs::create_dir_all(src_dir.join("tool")).expect("mkdir");
    fs::write(src_dir.join("tool/x.rs"), "").expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains(SHADOWED),
        "`use super::*;` in src/x.rs, the [[bin]] root's: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {SHADOWED:?}"
    );
    fs::remove_file(src_dir.join("tool/x.rs")).expect("remove");
    fs::write(src_dir.join("tool.rs"), "mod x;\nuse otherdep as q;\n").expect("write");
    // Read as an ordinary module (src/main.rs's `mod tool;`), the root
    // src/tool.rs has its `mod x;` at src/tool/x.rs, where main's
    // `crate::q` is `otherdep`; as the bin's root, at src/x.rs.
    fs::write(&lib, "mod tool { mod x; }\nmod q {}\n").expect("write");
    fs::write(
        src_dir.join("main.rs"),
        "mod tool;\nuse otherdep as q;\nfn main() {}\n",
    )
    .expect("write");
    fs::write(src_dir.join("tool.rs"), "mod x;\nfn main() {}\n").expect("write");
    fs::write(src_dir.join("x.rs"), "").expect("write");
    fs::create_dir_all(src_dir.join("tool")).expect("mkdir");
    fs::write(
        src_dir.join("tool/x.rs"),
        "use crate::q::*;\nverus! { pub fn f() {} }\n",
    )
    .expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains(SHADOWED),
        "src/tool.rs read as a module by src/main.rs: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {SHADOWED:?}"
    );
    for name in ["main.rs", "tool.rs", "x.rs", "tool/x.rs"] {
        fs::remove_file(src_dir.join(name)).expect("remove");
    }
    // The library's `mod tool;` reads the root src/tool.rs as a module,
    // whose `mod x;` is `tool::x` at src/tool/x.rs; read as the bin's root
    // it is src/x.rs, which the library's sure walk must not take as
    // `crate::x`, so `use x::*;` in the library is another crate's.
    fs::write(&lib, "mod tool;\nuse x::*;\nverus! { pub fn f() {} }\n").expect("write");
    fs::write(src_dir.join("tool.rs"), "mod x;\n").expect("write");
    fs::write(src_dir.join("x.rs"), "").expect("write");
    fs::write(src_dir.join("tool/x.rs"), "").expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains(SHADOWED),
        "the library reaching the root src/tool.rs: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {SHADOWED:?}"
    );
    for name in ["tool.rs", "x.rs", "tool/x.rs"] {
        fs::remove_file(src_dir.join(name)).expect("remove");
    }
    fs::write(&manifest, edition).expect("write");
    // With both oses declared too, an import under one os and one arch
    // needs two runs' atoms at once: it may be built only on a host no
    // run is, so it settles nothing, as a function there is not sure,
    // even where both come from the modules around it.
    fs::write(
        dir.join("verus/count.toml"),
        format!("{PLAIN}verus_sets = ['target_arch = \"x86_64\"', 'target_arch = \"aarch64\"', 'target_os = \"linux\"', 'target_os = \"macos\"']\n"),
    )
    .expect("write");
    for src in [
        "use other::*;\n#[cfg(target_os = \"macos\")]\n#[cfg(target_arch = \"x86_64\")]\nuse vstd::prelude::*;\nverus! { pub fn f() {} }\n",
        "#[cfg(target_os = \"macos\")]\nmod m { use other::*; #[cfg(target_arch = \"x86_64\")] use vstd::prelude::*; verus! { pub fn f() {} } }\n",
        "#[cfg(target_os = \"macos\")]\nmod m { #[cfg(target_arch = \"x86_64\")] mod n { use other::*; use vstd::prelude::*; verus! { pub fn f() {} } } }\n",
    ] {
        fs::write(&lib, src).expect("write");
        let (got, out, err) = count(&dir, &[]);
        assert!(
            got == 1 && err.contains(SHADOWED),
            "{src}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {SHADOWED:?}"
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

/// The base revision's walk reads the edition a crate inherits from the
/// workspace at that revision: with `--base-rev`, a `::vstd` block under
/// an inherited 2024 edition reads the same on the base as on the head.
#[test]
fn the_base_walk_reads_the_workspace_manifest() {
    let dir = crate_of(
        "base-workspace",
        &[(
            "lib.rs",
            "::vstd::prelude::verus! {\n#[verifier::rlimit(10)]\npub fn f() {}\n}\n",
        )],
    );
    fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/a\"]\n[workspace.package]\nedition = \"2024\"\n",
    )
    .expect("write");
    fs::write(
        dir.join("crates/a/Cargo.toml"),
        "[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition.workspace = true\n[package.metadata.verus]\nverify = true\n",
    )
    .expect("write");
    fs::write(
        dir.join("verus/count.toml"),
        format!("{PLAIN}[rlimit]\npath = \"verus/RLIMIT\"\n"),
    )
    .expect("write");
    fs::write(dir.join("verus/RLIMIT"), "max-share 65\n").expect("write");
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "Cargo.toml", "crates", "verus"]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    let stats = dir.join("stats.json");
    fs::write(&stats, ratchet_stats(3_000_000).replace("a::v", "a::f")).expect("write");
    let s = stats.to_string_lossy().into_owned();
    let (got, out, err) = count(&dir, &["--rlimit", &s, "--base-rev", "HEAD"]);
    assert!(
        got == 0,
        "base walk: got = exit {got}, {out:?}, {err:?}; wanted = exit 0"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// challoc's crate root: `compile_error!`s under a `vstd::prelude::*`.
const STD_MACROS_LIB: &str = "use vstd::prelude::*;
mod tls;
mod lglob;
mod sync;
mod asm;
#[cfg(not(target_pointer_width = \"64\"))]
compile_error!(\"challoc needs a 64-bit target\");
core::compile_error!(\"never\");
::std::compile_error!(\"never\");
other::compile_error!(\"\");
include!(\"gen.rs\");
verus! { pub fn v() {} }
";

/// `compile_error!`, which writes nothing that ships, counts nothing as
/// `::std::compile_error!` (unless an `extern crate .. as std` breaks the
/// rule), or bare where no binding of the name, `#[macro_use]` of another
/// crate, or `*` in its module is (so challoc's root, under `use
/// vstd::prelude::*;`, counts its four, as `sys/tls.rs` beside `use
/// core::ffi::*;` counts its one). `core::compile_error!` without `::`
/// always counts; a breaking binding a macro expansion writes fails the
/// run. `global_asm!` always counts (it may write code), as
/// challoc's `sys/tls.rs` imports it, and by every path; so does
/// `include!`, which writes what its file holds.
#[test]
fn std_item_macros_write_no_function() {
    let dir = crate_of(
        "std-macros",
        &[
            ("lib.rs", STD_MACROS_LIB),
            (
                "tls.rs",
                "use core::arch::{asm, global_asm};\nuse core::ffi::*;\nglobal_asm!(\".globl y\");\n#[cfg(not(unix))]\ncompile_error!(\"unix only\");\n",
            ),
            (
                "lglob.rs",
                "mod q { pub struct X; }\nuse q::*;\ncompile_error!(\"\");\n",
            ),
            (
                "sync.rs",
                "#[cfg(loom)]\npub use loom::sync::*;\ncompile_error!(\"\");\n",
            ),
            (
                "asm.rs",
                "core::arch::global_asm!(\"\");\n::std::arch::global_asm!(\"\");\nuse core::arch::*;\nglobal_asm!(\"\");\n",
            ),
        ],
    );
    let (got, out, err) = count(&dir, &["--list"]);
    let mut rows: Vec<(String, String, String)> = list_rows(&out)
        .into_iter()
        .map(|(class, place, name, _)| (class, place, name))
        .collect();
    rows.sort();
    let row = |class: &str, place: &str, name: &str| (class.into(), place.into(), name.into());
    let want: Vec<(String, String, String)> = vec![
        row("Outside", "crates/a/src/asm.rs:1", "global_asm!"),
        row("Outside", "crates/a/src/asm.rs:2", "global_asm!"),
        row("Outside", "crates/a/src/asm.rs:4", "global_asm!"),
        row("Outside", "crates/a/src/lglob.rs:3", "compile_error!"),
        row("Outside", "crates/a/src/lib.rs:10", "compile_error!"),
        row("Outside", "crates/a/src/lib.rs:11", "include!"),
        row("Outside", "crates/a/src/lib.rs:7", "compile_error!"),
        row("Outside", "crates/a/src/lib.rs:8", "compile_error!"),
        row("Outside", "crates/a/src/sync.rs:3", "compile_error!"),
        row("Outside", "crates/a/src/tls.rs:3", "global_asm!"),
        row("Outside", "crates/a/src/tls.rs:5", "compile_error!"),
        row("Verified", "crates/a/src/lib.rs:12", "v"),
    ];
    assert!(
        got == 0 && rows == want,
        "rows: got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
    );
    let _ = fs::remove_dir_all(&dir);
    // A crate whose root holds one binding, then a bare `compile_error!`
    // there, and in module `t` one by `core::` (always a row: without a
    // leading `::` its first segment may be the crate's), one by
    // `::std::` and one by `::core::`. The bare one counts where any
    // binding of the name, a `#[macro_use]` of another crate, or a `*` in
    // its module is; a leading `::` path counts where an `extern crate ..
    // as` takes its name.
    let rows_at = |lib: Option<&str>, t: &[u8]| {
        let mut v: Vec<(String, String, String)> = lib
            .into_iter()
            .map(|l| {
                row(
                    "Outside",
                    &format!("crates/a/src/lib.rs:{l}"),
                    "compile_error!",
                )
            })
            .collect();
        v.extend(t.iter().map(|l| {
            row(
                "Outside",
                &format!("crates/a/src/t.rs:{l}"),
                "compile_error!",
            )
        }));
        v
    };
    #[rustfmt::skip]
    let cases = [
        ("use asmkit::compile_error;", rows_at(Some("3"), &[1])),
        ("use core::compile_error;", rows_at(Some("3"), &[1])),
        ("mod core {}", rows_at(None, &[1])),
        ("use asmkit as core;", rows_at(None, &[1])),
        ("extern crate asmkit as core;", rows_at(None, &[1, 3])),
        ("extern crate asmkit as std;", rows_at(None, &[1, 2])),
        ("use asmkit as std;\nuse std::*;", rows_at(Some("4"), &[1])),
        ("#[macro_use]\nextern crate asmkit;", rows_at(Some("4"), &[1])),
        ("#[cfg_attr(unix, macro_use)]\nextern crate asmkit;", rows_at(Some("4"), &[1])),
        ("::vstd::prelude::verus! { #[macro_use] extern crate asmkit; }", rows_at(Some("3"), &[1])),
        ("use asmkit::*;", rows_at(Some("3"), &[1])),
        ("use core::ptr::*;", rows_at(Some("3"), &[1])),
        ("mod q { pub use asmkit::*; }\nuse q::*;", rows_at(Some("4"), &[1])),
        ("mod q { pub use asmkit as bridge; }\nuse q::bridge::*;", rows_at(Some("4"), &[1])),
        ("mod asmkit {}\nmod s { use asmkit::*; compile_error!(\"\"); }", {
            let mut v = rows_at(None, &[1]);
            v.push(row("Outside", "crates/a/src/lib.rs:2", "compile_error!"));
            v.sort();
            v
        }),
        ("#[macro_use]\nextern crate std;", rows_at(None, &[1])),
        ("#[cfg_attr(unix, macro_use)]\nextern crate core;", rows_at(None, &[1])),
        ("extern crate std;", rows_at(None, &[1])),
        ("#[cfg(test)]\nuse asmkit::*;", rows_at(None, &[1])),
    ];
    for (binding, want) in cases {
        let lib = format!("{binding}\nmod t;\ncompile_error!(\"x\");\n");
        let t = "core::compile_error!(\"\");\n::std::compile_error!(\"\");\n::core::compile_error!(\"\");\n";
        let dir = crate_of("std-macro-use", &[("lib.rs", &lib), ("t.rs", t)]);
        let (got, out, err) = count(&dir, &["--list"]);
        let mut rows: Vec<(String, String, String)> = list_rows(&out)
            .into_iter()
            .map(|(class, place, name, _)| (class, place, name))
            .collect();
        rows.sort();
        assert!(
            got == 0 && rows == want,
            "{lib}: got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // A dependency renamed to `std`, `core` or `alloc` (directly or
    // inherited from the workspace) may bring its own prelude: a bare
    // `compile_error!` and the `::std` / `::core` forms count.
    for (ws, deps) in [
        ("", "std = { package = \"other\", path = \"x\" }"),
        ("", "core = { package = \"other\", path = \"x\" }"),
        ("", "alloc = { package = \"other\", path = \"x\" }"),
        (
            "[workspace.dependencies]\nstd = { package = \"other\", path = \"x\" }\n",
            "std = { workspace = true }",
        ),
    ] {
        let dir = crate_of(
            "std-renamed",
            &[(
                "lib.rs",
                "compile_error!(\"x\");\n::std::compile_error!(\"y\");\n",
            )],
        );
        fs::write(dir.join("Cargo.toml"), ws).expect("write");
        fs::write(
            dir.join("crates/a/Cargo.toml"),
            format!("[package]\nname = \"a\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[dependencies]\n{deps}\n"),
        )
        .expect("write");
        let (got, out, err) = count(&dir, &["--list"]);
        let rows: Vec<String> = list_rows(&out).into_iter().map(|r| r.1).collect();
        let want = if deps.starts_with("std") {
            vec!["crates/a/src/lib.rs:1", "crates/a/src/lib.rs:2"]
        } else {
            vec!["crates/a/src/lib.rs:1"]
        };
        assert!(
            got == 0 && rows == want,
            "{deps}: got = exit {got}, {rows:?}, {err:?}; wanted = exit 0, {want:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // A shadowing binding a macro expansion writes, once a `compile_error!`
    // was counted as nothing: the run fails.
    let lib = "compile_error!(\"x\");\nmacro_rules! mk { () => { use asmkit::compile_error; }; }\nmk!();\n";
    let dir = crate_of("std-emitted", &[("lib.rs", lib)]);
    let (got, out, err) = count(&dir, &[]);
    let want = "may make this macro another than the std one";
    assert!(
        got == 1 && err.contains(want),
        "emitted: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
    );
    let _ = fs::remove_dir_all(&dir);
    // A `#[macro_use] mod` only a `cfg_attr` marks, under a predicate only
    // test builds meet, carries nothing into the shipped walk: the `n!` the
    // shipped build takes from `other` stays a row, and the crate's own
    // `n` beside it fails the run rather than stand in for it.
    let lib =
        "#[macro_use]\nextern crate other;\n#[cfg_attr(test, macro_use)]\nmod helpers;\nn!();\n";
    let dir = crate_of(
        "macro-use-mod",
        &[
            ("lib.rs", lib),
            ("helpers.rs", "macro_rules! n { () => {} }\n"),
        ],
    );
    let (got, out, err) = count(&dir, &["--list"]);
    let rows: Vec<(String, String)> = list_rows(&out)
        .into_iter()
        .map(|(class, _, name, _)| (class, name))
        .collect();
    let want = "n! resolves to none of this crate's definitions of `n`";
    let row_n = vec![("Outside".to_owned(), "n!".to_owned())];
    assert!(
        got == 1 && rows == row_n && err.contains(want),
        "macro_use mod: got = exit {got}, {rows:?}, {err:?}; wanted = exit 1, {row_n:?}, {want:?}"
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
            "diff! could be any of 2 definitions (crates/a/src/pairs.rs:8, crates/a/src/pairs.rs:6), which count differently",
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

/// A definition at the top of a file a `#[macro_use] mod name;` declares
/// is in the declaring file's textual scope from that `mod` on, as rustc
/// reads it (#128): after an earlier definition there it shadows it, and
/// before a later one it is shadowed, also two files out (`j` carrying
/// `k`). An invocation before the `mod` does not see it.
#[test]
fn a_macro_use_file_s_definition_is_in_scope_from_its_mod() {
    let one = "macro_rules! m { () => { pub fn a() {} }; }\n";
    let three = "macro_rules! m { () => { pub fn a() {} pub fn b() {} pub fn c() {} }; }\n";
    let carry_k = "#[macro_use]\nmod k;\n";
    // `j`'s definition after its `mod k;`, and before it; `k`'s on a later
    // line of its own file than `j`'s, so the lines alone would misorder.
    let j_after_k = format!("{carry_k}{three}");
    let j_before_k = format!("{three}{carry_k}");
    let k_late = format!("{}{one}", "\n".repeat(9));
    let inline = format!("#[macro_use]\nmod inner {{\n{three}}}\n");
    let in_verus = format!("verus! {{\n{three}}}\n");
    for (case, lib, j, k, want) in [
        (
            "after an earlier one",
            format!("{one}#[macro_use]\nmod j;\nm!();\n"),
            three,
            None,
            &["a", "b", "c"][..],
        ),
        (
            "before a later one",
            format!("#[macro_use]\nmod j;\n{one}m!();\n"),
            three,
            None,
            &["a"][..],
        ),
        (
            "an invocation before the mod",
            format!("{one}m!();\n#[macro_use]\nmod j;\n"),
            three,
            None,
            &["a"][..],
        ),
        (
            "two files out",
            format!("{one}#[macro_use]\nmod j;\nm!();\n"),
            carry_k,
            Some(three),
            &["a", "b", "c"][..],
        ),
        (
            "j's after its mod k shadows k's",
            "#[macro_use]\nmod j;\nm!();\n".to_owned(),
            &j_after_k,
            Some(&k_late),
            &["a", "b", "c"][..],
        ),
        (
            "k's shadows j's before its mod k",
            "#[macro_use]\nmod j;\nm!();\n".to_owned(),
            &j_before_k,
            Some(&k_late),
            &["a"][..],
        ),
        (
            "in an inline #[macro_use] module of the carried file",
            format!("{one}#[macro_use]\nmod j;\nm!();\n"),
            &inline,
            None,
            &["a", "b", "c"][..],
        ),
        (
            "in a verus! block of the carried file",
            format!("{one}#[macro_use]\nmod j;\nm!();\n"),
            &in_verus,
            None,
            &["a", "b", "c"][..],
        ),
    ] {
        let dir = crate_of("macro-use-mod", &[("lib.rs", &lib)]);
        let src = dir.join("crates/a/src");
        match k {
            None => fs::write(src.join("j.rs"), j).expect("write"),
            Some(k) => {
                fs::create_dir_all(src.join("j")).expect("mkdir");
                fs::write(src.join("j/mod.rs"), j).expect("write");
                fs::write(src.join("j/k.rs"), k).expect("write");
            }
        }
        let (got, out, err) = count(&dir, &["--list"]);
        let names: Vec<String> = list_rows(&out).into_iter().map(|r| r.2).collect();
        assert!(
            got == 0 && names == want,
            "{case}: got = exit {got}, {names:?}, {err:?}; wanted = exit 0, {want:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // A definition on the line of the `mod` that carries the other in: the
    // order on the line is not read, so both are candidates, and as they
    // count differently the run fails.
    let dir = crate_of(
        "macro-use-mod-line",
        &[
            (
                "lib.rs",
                &format!("{} #[macro_use] mod j;\nm!();\n", one.trim_end()),
            ),
            ("j.rs", three),
        ],
    );
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains("m! could be any of 2 definitions"),
        "same line: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
    );
    let _ = fs::remove_dir_all(&dir);
    // Two `mod`s on one line carry two files in: their order on the line
    // is not read, whatever lines the definitions sit on in their own
    // files, so both are candidates.
    let dir = crate_of(
        "macro-use-mod-siblings",
        &[
            ("lib.rs", "#[macro_use] mod j; #[macro_use] mod k;\nm!();\n"),
            ("j.rs", &format!("{}{one}", "\n".repeat(9))),
            ("k.rs", three),
        ],
    );
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains("m! could be any of 2 definitions"),
        "sibling mods: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
    );
    let _ = fs::remove_dir_all(&dir);
    // A `mod` on the invocation's own line: its order against the
    // invocation is not read either, so j's is a candidate beside the
    // earlier one.
    let dir = crate_of(
        "macro-use-mod-invocation-line",
        &[
            ("lib.rs", &format!("{one}#[macro_use] mod j; m!();\n")),
            ("j.rs", three),
        ],
    );
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains("m! could be any of 2 definitions"),
        "mod on the invocation's line: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
    );
    let _ = fs::remove_dir_all(&dir);
    // One module declared by two `mod`s under `cfg`s, one before a
    // definition and one after: each build places the carried file at
    // its own `mod`, so the two are not ordered, and as they count
    // differently the run fails. Also when only one `mod` carries, when
    // the definition is inside an inline `#[macro_use]` module or a
    // `verus!` block of the carried file, and when the module declared
    // twice is one file further in (`k`, carried out through `j`).
    let ghost = "#[cfg(verus_keep_ghost)]";
    let real = "#[cfg(not(verus_keep_ghost))]";
    let twice = format!("{ghost} #[macro_use] mod j;\n{one}{real} #[macro_use] mod j;\nm!();\n");
    let k_twice = format!("{ghost} #[macro_use] mod k;\n{one}{real} #[macro_use] mod k;\n");
    for (case, lib, files) in [
        (
            "the carried one is the larger",
            twice.clone(),
            vec![("j.rs", three.to_owned())],
        ),
        (
            "the written one is the larger",
            format!("{real} #[macro_use] mod j;\n{three}{ghost} #[macro_use] mod j;\nm!();\n"),
            vec![("j.rs", one.to_owned())],
        ),
        (
            "one mod after the invocation",
            format!("{one}{ghost} #[macro_use] mod j;\nm!();\n{real} #[macro_use] mod j;\n"),
            vec![("j.rs", three.to_owned())],
        ),
        (
            "only one mod carries",
            format!("{ghost} #[macro_use] mod j;\n{one}{real} mod j;\nm!();\n"),
            vec![("j.rs", three.to_owned())],
        ),
        (
            "in an inline #[macro_use] module",
            twice.clone(),
            vec![("j.rs", inline.clone())],
        ),
        (
            "in a verus! block",
            twice.clone(),
            vec![("j.rs", in_verus.clone())],
        ),
        (
            "one file further in",
            "#[macro_use]\nmod j;\nm!();\n".to_owned(),
            vec![("j/mod.rs", k_twice), ("j/k.rs", three.to_owned())],
        ),
    ] {
        let dir = crate_of("macro-use-mod-twice", &[("lib.rs", &lib)]);
        for (path, text) in files {
            let p = dir.join("crates/a/src").join(path);
            fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
            fs::write(p, text).expect("write");
        }
        let (got, out, err) = count(&dir, &[]);
        assert!(
            got == 1 && err.contains("m! could be any of 2 definitions"),
            "declared twice, {case}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

/// Two more shapes of a carried definition (#134). A file `mod` inside an
/// inline `#[macro_use]` module carries its definition out through that
/// module too, so it shadows an earlier one after the module; inside an
/// inline module without `#[macro_use]` it does not. A `macro_rules!` an
/// expansion writes at the top of a carried file is carried as one
/// written there; when the counter reads that file after the declaring
/// one, an invocation it may reach fails the run.
#[test]
fn a_carried_definition_passes_inline_macro_use_modules_and_expansions() {
    let one = "macro_rules! m { () => { pub fn a() {} }; }\n";
    let three = "macro_rules! m { () => { pub fn a() {} pub fn b() {} pub fn c() {} }; }\n";
    let gen_three = "macro_rules! gen { () => { macro_rules! m { () => { pub fn a() {} pub fn b() {} pub fn c() {} }; } }; }\ngen!();\n";
    // A `gen` that writes the three-function `m` inside `mod inner`, with
    // `attr` on it.
    let gen_in_inner = |attr: &str| {
        format!(
            "macro_rules! gen {{ () => {{ {attr} mod inner {{ macro_rules! m {{ () => {{ pub fn a() {{}} pub fn b() {{}} pub fn c() {{}} }}; }} }} }}; }}\n"
        )
    };
    let in_outer = |macro_use: &str| {
        format!("{one}{macro_use}mod outer {{\n#[macro_use]\nmod j;\n}}\nm!();\n")
    };
    for (case, lib, files, want) in [
        (
            "through an inline #[macro_use] module",
            in_outer("#[macro_use]\n"),
            vec![("outer/j.rs", three)],
            &["a", "b", "c"][..],
        ),
        (
            "not through an inline module without #[macro_use]",
            in_outer(""),
            vec![("outer/j.rs", three)],
            &["a"][..],
        ),
        (
            "through an inline #[macro_use] module of a carried file",
            format!("{one}#[macro_use]\nmod j;\nm!();\n"),
            vec![
                (
                    "j/mod.rs",
                    "#[macro_use]\nmod inner {\n#[macro_use]\nmod k;\n}\n",
                ),
                ("j/inner/k.rs", three),
            ],
            &["a", "b", "c"][..],
        ),
        (
            "emitted at the top of a carried file read first",
            format!("{one}#[macro_use]\nmod j;\nm!();\n"),
            vec![("j.rs", gen_three)],
            &["a", "b", "c"][..],
        ),
        (
            "emitted in a carried file read later, after the invocation",
            format!("{one}m!();\n#[macro_use]\nmod z;\n"),
            vec![("z.rs", gen_three)],
            &["a"][..],
        ),
        (
            "emitted in a carried file read later, outside its scope",
            format!("{one}mod outer {{\n#[macro_use]\nmod z;\n}}\nm!();\n"),
            vec![("outer/z.rs", gen_three)],
            &["a"][..],
        ),
        // One an expansion writes in an inline `#[macro_use]` module is
        // carried out of it, as one written there is; without the
        // attribute its scope ends with the module.
        (
            "emitted in an inline #[macro_use] module",
            format!("{one}{}gen!();\nm!();\n", gen_in_inner("#[macro_use]")),
            vec![],
            &["a", "b", "c"][..],
        ),
        (
            "emitted in an inline module without #[macro_use]",
            format!("{one}{}gen!();\nm!();\n", gen_in_inner("")),
            vec![],
            &["a"][..],
        ),
    ] {
        let dir = crate_of("carried-shapes", &[("lib.rs", &lib)]);
        let src = dir.join("crates/a/src");
        for (path, text) in files {
            let p = src.join(path);
            fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
            fs::write(p, text).expect("write");
        }
        let (got, out, err) = count(&dir, &["--list"]);
        let names: Vec<String> = list_rows(&out).into_iter().map(|r| r.2).collect();
        assert!(
            got == 0 && names == want,
            "{case}: got = exit {got}, {names:?}, {err:?}; wanted = exit 0, {want:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // The carried file is read after the declaring one: the definition its
    // expansion writes is not known when the invocation resolves. A `mod`
    // on the invocation's line may carry it in too.
    for (case, lib, line) in [
        (
            "after the mod",
            format!("{one}#[macro_use]\nmod z;\nm!();\n"),
            4,
        ),
        (
            "on the mod's line",
            format!("{one}#[macro_use] mod z; m!();\n"),
            2,
        ),
    ] {
        let dir = crate_of(
            "carried-shapes-late",
            &[("lib.rs", &lib), ("z.rs", gen_three)],
        );
        let (got, out, err) = count(&dir, &[]);
        let want = format!(
            "lib.rs:{line}: m! may expand the `macro_rules! m` that an expansion writes at crates/a/src/z.rs:2"
        );
        assert!(
            got == 1 && err.contains(&want),
            "read later, {case}: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
    // An invocation in a sibling file the `mod` after it declares, read
    // before the carried file: the resolution would reach the emitted
    // definition as one of the others, so the run fails there too.
    let dir = crate_of(
        "carried-shapes-sibling",
        &[
            ("lib.rs", &format!("{one}#[macro_use]\nmod z;\nmod y;\n")),
            ("y.rs", "m!();\n"),
            ("z.rs", gen_three),
        ],
    );
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1
            && err.contains(
                "y.rs:1: m! may expand the `macro_rules! m` that an expansion writes at crates/a/src/z.rs:2"
            ),
        "sibling read earlier: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
    );
    let _ = fs::remove_dir_all(&dir);
    // Not carried at all: one an expansion writes at the top of z/mod.rs
    // is in scope in z/a.rs, which `mod a;` after it declares and the
    // counter reads first.
    let dir = crate_of(
        "carried-shapes-child",
        &[("lib.rs", &format!("{one}mod z;\n"))],
    );
    fs::create_dir_all(dir.join("crates/a/src/z")).expect("mkdir");
    fs::write(
        dir.join("crates/a/src/z/mod.rs"),
        format!("{gen_three}mod a;\n"),
    )
    .expect("write");
    fs::write(dir.join("crates/a/src/z/a.rs"), "m!();\n").expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1
            && err.contains(
                "z/a.rs:1: m! may expand the `macro_rules! m` that an expansion writes at crates/a/src/z/mod.rs:2"
            ),
        "child read earlier: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
    );
    let _ = fs::remove_dir_all(&dir);
    // An inline module declared twice under `cfg`s, with `#[macro_use]`
    // in one build only: the file `mod` both hold is declared twice, so
    // its definition is a candidate that ends no chain.
    let lib = format!(
        "{one}#[cfg(verus_keep_ghost)]\n#[macro_use]\nmod outer {{\n#[macro_use]\nmod j;\n}}\n#[cfg(not(verus_keep_ghost))]\nmod outer {{\n#[macro_use]\nmod j;\n}}\nm!();\n"
    );
    let dir = crate_of("carried-shapes-inline-twice", &[("lib.rs", &lib)]);
    fs::create_dir_all(dir.join("crates/a/src/outer")).expect("mkdir");
    fs::write(dir.join("crates/a/src/outer/j.rs"), three).expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 1 && err.contains("m! could be any of 2 definitions"),
        "inline module declared twice: got = exit {got}, {out:?}, {err:?}; wanted = exit 1"
    );
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
                "m! could be any of 2 definitions (crates/a/src/defs.rs:2, crates/a/src/defs.rs:1), and one of them does not count",
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

/// A reported reproducer: a function under a `cfg` Verus's
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
            "[package]\nname = \"b\"\nversion = \"0.0.0\"\nautobins = false\n\n[package.metadata.verus]\nverify = true\n\n[[bin]]\nname = \"tool\"\npath = \"src/bin/tool.rs\"\n",
        ),
        (
            "b/src/bin/tool.rs",
            "#[cfg(windows)]\nmod x;\nverus! {\npub fn in_tool() {}\n}\n",
        ),
        ("b/src/bin/x.rs", "verus! {\npub fn in_tool_child() {}\n}\n"),
        ("b/src/bin/stray.rs", "verus! {\npub fn in_stray() {}\n}\n"),
        (
            "c/Cargo.toml",
            "[package]\nname = \"c\"\nversion = \"0.0.0\"\n[package.metadata.verus]\nverify = true\n",
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

/// A file's inner `#![verifier::exec_allows_no_decreases_clause]` makes
/// its exec functions trusted. In a file that declares a module out of
/// line, whose file the counter reads on its own, it fails the run (exit
/// 1), naming the file.
#[test]
fn a_file_without_decreases_is_trusted() {
    let dir = repository("no-decreases");
    let lib = dir.join("crates/a/src/lib.rs");
    fs::write(
        &lib,
        "#![verifier::exec_allows_no_decreases_clause]\nverus! { fn f() {} fn g() {} }\n",
    )
    .expect("write");
    let (got, out, err) = count(&dir, &[]);
    assert!(
        got == 0 && out.contains("\nshipped\t0\t2\t0\t0\t"),
        "a leaf file: got = exit {got}, {out:?}, {err:?}; wanted = exit 0, 0 verified and 2 trusted"
    );
    fs::write(
        &lib,
        "#![verifier::exec_allows_no_decreases_clause]\nmod m;\nverus! { fn f() {} }\n",
    )
    .expect("write");
    fs::write(dir.join("crates/a/src/m.rs"), "verus! { fn h() {} }\n").expect("write");
    let (got, out, err) = count(&dir, &[]);
    let want = "crates/a/src/lib.rs:2: #[verifier::exec_allows_no_decreases_clause] covers `mod m;`, a module the counter reads file by file";
    assert!(
        got == 1 && err.contains(want),
        "a file declaring mod m;: got = exit {got}, {out:?}, {err:?}; wanted = exit 1, {want:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}
