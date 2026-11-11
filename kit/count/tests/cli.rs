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
