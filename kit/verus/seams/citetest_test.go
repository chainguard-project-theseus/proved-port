package main

import (
	"strings"
	"testing"
)

// citeTestFiles is a test crate after challoc's tests/loom/src, with
// seams.sh's loomtest() self-test models (m.rs, n.rs) and more.
var citeTestFiles = map[string]string{
	"tests/loom/Cargo.toml": citeTestManifest,
	"tests/loom/src/lib.rs": lines(
		"//! The models.",
		"#[cfg(test)]",
		"mod m;",
		"#[cfg(test)] mod n;",
		"pub mod inner;",
		"#[cfg(loom)]",
		"mod gated;",
		"#[path = \"x/moved.rs\"]",
		"mod moved;",
		"mod ignored;",
		"mod shut;",
		"mod open;",
		"mod modrs;",
		"// mod commented;",
		"cfg_if! {",
		"    mod nested;",
		"}",
	),
	"tests/loom/src/m.rs": lines(
		"//! A model of `src/vspec/count.rs`.",
		"",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_ok() {",
		"    run();",
		"}",
		"",
		"/// The control: `count::fetch_add` split.",
		"#[test]",
		"#[should_panic(expected = \"x\")]",
		"fn loom_control() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add_split` and count::fetch_add.",
		"#[test]",
		"fn loom_longer() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add`.",
		"fn loom_untested() {",
		"    run();",
		"}",
		"",
		"/// Checks `init` and `SlabDesc::set_state` (`src/slab.rs`).",
		"#[test]",
		"fn loom_method() {",
		"    run();",
		"}",
		"",
		"/// Checks `fetch_add`, not by its module.",
		"#[test]",
		"fn loom_bare() {",
		"    run();",
		"}",
		"",
		"/// Checks `entry::store`, of another module.",
		"#[test]",
		"fn loom_other_mod() {",
		"    run();",
		"}",
		"",
		// Beyond loomtest(): the body names the function, behind a path.
		"#[test]",
		"fn loom_body() {",
		"    // Runs `vspec::count::fetch_add`.",
		"    if x {",
		"        run();",
		"    }",
		"}",
		"",
		// Named only after the body closes.
		"#[test]",
		"fn loom_after() {",
		"    run();",
		"}",
		"// `count::fetch_add`",
		"",
		"#[test]",
		"#[ignore = \"slow\"]",
		"/// Checks `count::fetch_add`.",
		"fn loom_ignored() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"#[cfg(test)]",
		"fn loom_cfg_allowed() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"#[cfg(loom)]",
		"fn loom_cfg_other() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add`.",
		"#[test] #[should_panic]",
		"fn loom_same_line() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"",
		"// a comment between",
		"fn loom_gap() {",
		"    run();",
		"}",
		"",
		"//// Checks `count::fetch_add` (a plain comment).",
		"#[test]",
		"fn loom_four_slashes() {",
		"    run();",
		"}",
		"",
		"/*",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_commented() {",
		"    run();",
		"}",
		"*/",
		"",
		"mod deeper {",
		"    /// Checks `count::fetch_add`.",
		"    #[test]",
		"    fn loom_deep() {",
		"        run();",
		"    }",
		"}",
		"",
		"const X: u8 = 0; #[should_panic]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_trailing() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add`.",
		"#[test] fn loom_inline() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"#[cfg_attr(miri, ignore)]",
		"fn loom_cfg_attr() {",
		"    run();",
		"}",
		"",
		// A fn at column 0 inside a mod is not top-level.
		"mod flat {",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_flat() {",
		"    run();",
		"}",
		"}",
		"",
		// A doc line in a block comment is no doc.
		"/*",
		"/// Checks `count::fetch_add`.",
		"*/",
		"#[test]",
		"fn loom_doc_commented() {",
		"    run();",
		"}",
		"",
		// The name between two spans, outside backquotes.
		"/// Checks `x`count::fetch_add`y`.",
		"#[test]",
		"fn loom_outside() {",
		"    run();",
		"}",
		"",
		"/// Checks `a b::count::fetch_add`.",
		"#[test]",
		"fn loom_not_a_path() {",
		"    run();",
		"}",
		"",
		// `#` and `[` split over lines are one attribute.
		"#",
		"[ignore]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_split_ignore() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"#",
		"[ignore]",
		"fn loom_split_late() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"#",
		"[inline]",
		"fn loom_split_ok() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add`.",
		"#[test] #",
		"[ignore]",
		"fn loom_split_on_attr() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add`.",
		"#[inline]",
		"fn loom_not_test() {",
		"    run();",
		"}",
		"",
		"const Y: u8 = 0; #",
		"[ignore]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_split_after_code() {",
		"    run();",
		"}",
		"",
		// An attribute wrapped over lines after other code on its line:
		// the allowed cfg below it stays its own.
		"const Z: u8 = 0; #[doc(",
		"    hidden)]",
		"#[cfg(test)]",
		"#[cfg(loom)]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_wrapped_after_code() {",
		"    run();",
		"}",
		"",
		// rustfmt wraps a long attribute over lines.
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"#[should_panic(",
		"    expected = \"a long message\"",
		")]",
		"fn loom_wrapped_control() {",
		"    run();",
		"}",
		"",
		"/// Checks `count::fetch_add`.",
		"#[doc(",
		"    alias = \"x\"",
		")]",
		"#[test]",
		"fn loom_wrapped_ok() {",
		"    run();",
		"}",
		"",
		// An attribute after a multi-line item's closing delimiter on
		// its line attaches to the next item.
		"fn helper() {",
		"}  #[should_panic]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_after_close() {",
		"    run();",
		"}",
		"",
		"const A: [u8; 2] = [",
		"    0, 1,",
		"]; #[ignore]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_after_array() {",
		"    run();",
		"}",
		"",
		"m!(",
		"    x",
		"); #[cfg(loom)]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_after_macro() {",
		"    run();",
		"}",
		"",
		"fn helper2() {",
		"} #",
		"[ignore]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_after_close_split() {",
		"    run();",
		"}",
		"",
		"fn helper3() {",
		"} #[cfg(any(",
		"    loom,",
		"))]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_after_close_wrapped() {",
		"    run();",
		"}",
		"",
		// Code after the close ends the block as before.
		"fn helper4() {",
		"} #[should_panic] const B: u8 = 0;",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_after_close_code() {",
		"    run();",
		"}",
		"",
		"const C: u8 = 0; #[should_panic] const D: u8 = 0;",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_after_code_item() {",
		"    run();",
		"}",
		"",
		// The last attribute run on the line starts the block, past
		// runs with an item after them and runs in delimiters.
		"const E: u8 = 0; #[inline] const F: u8 = 0; #[should_panic]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_code_runs() {",
		"    run();",
		"}",
		"",
		"fn helper6() {",
		"} #[cfg(x)] const G: u8 = 0; #[ignore]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_close_runs() {",
		"    run();",
		"}",
		"",
		"m!(#[a] x); #[ignore]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_macro_runs() {",
		"    run();",
		"}",
		"",
		"fn helper7() {",
		"} m!(#[a] x); #[should_panic]",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_close_macro_runs() {",
		"    run();",
		"}",
		"",
		"const I: u8 = 0; m!(#[should_panic(",
		"    expected = \"x\")] x);",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_wrapped_in_macro() {",
		"    run();",
		"}",
		"",
		"const H: u8 = 0; m!(#[should_panic] x);",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_attr_in_macro() {",
		"    run();",
		"}",
		"",
		// The body ends at the delimiter that closes it: code or a
		// comment after it on its line is not the test's.
		"#[test]",
		"fn loom_body_then_fn() {",
		"    run();",
		"} fn helper8() {",
		"    // drives `count::fetch_add`",
		"}",
		"",
		"#[test]",
		"fn loom_body_then_comment() {",
		"    run();",
		"} // `count::fetch_add`",
		"",
		"#[test]",
		"fn loom_body_closing_line() {",
		"    run(); /* `count::fetch_add` */ }",
		"",
		// An attribute on a line still inside an item is the item's.
		"fn helper5() {",
		"    #[should_panic] let _ = 0;",
		"}",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn loom_inside_item() {",
		"    run();",
		"}",
	),
	"tests/loom/src/modrs.rs": lines(
		"//! A model of `src/vspec/count/mod.rs`.",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn ok() {",
		"}",
	),
	"tests/loom/src/n.rs": lines(
		"//! A model of `lock.rs`, not by its path.",
		"",
		"/// Checks `RawLock::lock_slow`.",
		"#[test]",
		"fn loom_lock() {",
		"    run();",
		"}",
	),
	"tests/loom/src/inner.rs": lines(
		"#![cfg(test)]",
		"//! `src/vspec/count.rs`",
		"/// Checks `count::fetch_add`.",
		"#[test]",
		"fn ok() {",
		"}",
	),
	"tests/loom/src/undeclared.rs": lines("//! `src/vspec/count.rs`", "/// Checks `count::fetch_add`.", "#[test]", "fn ok() {", "}"),
	"tests/loom/src/gated.rs":      lines("//! `src/vspec/count.rs`", "/// Checks `count::fetch_add`.", "#[test]", "fn ok() {", "}"),
	"tests/loom/src/moved.rs":      lines("//! `src/vspec/count.rs`", "/// Checks `count::fetch_add`.", "#[test]", "fn ok() {", "}"),
	"tests/loom/src/commented.rs":  lines("//! `src/vspec/count.rs`", "/// Checks `count::fetch_add`.", "#[test]", "fn ok() {", "}"),
	"tests/loom/src/nested.rs":     lines("//! `src/vspec/count.rs`", "/// Checks `count::fetch_add`.", "#[test]", "fn ok() {", "}"),
	"tests/loom/src/ignored.rs":    lines("#![cfg(loom)]", "//! `src/vspec/count.rs`", "/// Checks `count::fetch_add`.", "#[test]", "fn ok() {", "}"),
	"tests/loom/src/shut.rs":       lines("//! `vspec/count.rs`", "/// Checks `count::fetch_add`.", "#[test]", "fn ok() {", "}"),
	"tests/loom/src/open.rs":       lines("//! `src/vspec/count.rs`", "/// Checks `count::fetch_add`.", "#[test]", "fn ok() {", "}", "/*"),
}

const citeTestRules = resolveRules + "cite-tag\tloom\tpartial\ncite-test\tloom\ttests/loom/src\ttest\n"

// TestCiteTest holds seams.sh's loomtest() self-test cases, and the
// checks the kit adds (the file declared, no gate on it or the test).
func TestCiteTest(t *testing.T) {
	r, err := parseRules("r", citeTestRules)
	if err != nil {
		t.Fatal(err)
	}
	d := tree(t, citeTestFiles)
	for _, c := range []struct {
		file, name, target, want string // want: "" resolves, else a part of the error
	}{
		// loomtest()'s self-test.
		{"src/vspec/count.rs", "fetch_add", "m::loom_ok", ""},
		{"src/slab.rs", "SlabDesc::set_state", "m::loom_method", ""},
		{"src/lock.rs", "RawLock::lock_slow", "n::loom_lock", "tests/loom/src/n.rs does not name `src/lock.rs`"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_control", "a #[should_panic] test (a control) is no check"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_longer", "names `count::fetch_add` in backquotes"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_untested", "no `#[test]` line above it"},
		{"src/vspec/count.rs", "fetch_add", "m::no_such_test", "has no top-level `fn no_such_test() {`"},
		{"src/vspec/count.rs", "fetch_add", "no_such_mod::loom_ok", "tests/loom/src/no_such_mod.rs is not there"},
		{"src/vspec/count.rs", "fetch_add", "loom_ok", "is no `MOD::TEST` path"},
		{"src/vspec/count.rs", "store", "m::loom_ok", "names `count::store` in backquotes"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_bare", "names `count::fetch_add`"},
		{"src/vspec/count.rs", "store", "m::loom_other_mod", "names `count::store`"},
		{"src/vspec/byte.rs", "fetch_add", "m::loom_ok", "names neither `src/vspec/byte.rs` nor `vspec/byte.rs`"},
		{"src/slab.rs", "SlabDesc::init", "m::loom_method", "names `SlabDesc::init`"},
		// The body counts, to the line that closes it; text after it
		// does not.
		{"src/vspec/count.rs", "fetch_add", "m::loom_body", ""},
		{"src/vspec/count.rs", "fetch_add", "m::loom_after", "neither its doc comment nor its body"},
		// A mod.rs is its directory's module.
		{"src/vspec/count/mod.rs", "fetch_add", "m::loom_ok", "names neither `src/vspec/count/mod.rs`"},
		{"src/vspec/count/mod.rs", "fetch_add", "modrs::ok", ""},
		// MOD and TEST are words, not patterns.
		{"src/vspec/count.rs", "fetch_add", "m::loom_.k", "is no `MOD::TEST` path"},
		{"src/vspec/count.rs", "fetch_add", "a.b::loom_ok", "is no `MOD::TEST` path"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_flat", "has no top-level `fn loom_flat() {`"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_wrapped", "has no top-level `fn loom_wrapped() {`"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_doc_commented", "neither its doc comment nor its body"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_outside", "neither its doc comment nor its body"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_not_a_path", "neither its doc comment nor its body"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_split_ignore", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_split_after_code", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_split_late", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_split_ok", ""},
		{"src/vspec/count.rs", "fetch_add", "m::loom_split_on_attr", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_not_test", "no `#[test]` line above it"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_wrapped_after_code", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_wrapped_control", "a #[should_panic] test"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_wrapped_ok", ""},
		// After an item's closing delimiter on its line.
		{"src/vspec/count.rs", "fetch_add", "m::loom_after_close", "a #[should_panic] test"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_after_array", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_after_macro", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_after_close_split", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_after_close_wrapped", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_after_close_code", ""},
		{"src/vspec/count.rs", "fetch_add", "m::loom_inside_item", ""},
		{"src/vspec/count.rs", "fetch_add", "m::loom_after_code_item", ""},
		{"src/vspec/count.rs", "fetch_add", "m::loom_code_runs", "a #[should_panic] test"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_close_runs", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_macro_runs", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_close_macro_runs", "a #[should_panic] test"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_attr_in_macro", ""},
		{"src/vspec/count.rs", "fetch_add", "m::loom_wrapped_in_macro", ""},
		{"src/vspec/count.rs", "fetch_add", "m::loom_body_then_fn", "neither its doc comment nor its body names"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_body_then_comment", "neither its doc comment nor its body names"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_body_closing_line", ""},
		// Gates on the test: ignore and cfg but the allowed one.
		{"src/vspec/count.rs", "fetch_add", "m::loom_ignored", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_cfg_allowed", ""},
		{"src/vspec/count.rs", "fetch_add", "m::loom_cfg_other", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_cfg_attr", "under a cfg, cfg_attr or ignore attribute"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_same_line", "a #[should_panic] test"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_trailing", "a #[should_panic] test"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_inline", ""},
		// Blank and comment lines keep the attributes; `////` is no doc.
		{"src/vspec/count.rs", "fetch_add", "m::loom_gap", ""},
		{"src/vspec/count.rs", "fetch_add", "m::loom_four_slashes", "neither its doc comment nor its body"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_commented", "has no top-level `fn loom_commented() {`"},
		{"src/vspec/count.rs", "fetch_add", "m::loom_deep", "has no top-level `fn loom_deep() {`"},
		// The file must be declared, ungated, at lib.rs's top level.
		{"src/vspec/count.rs", "fetch_add", "inner::ok", ""},
		{"src/vspec/count.rs", "fetch_add", "undeclared::ok", "no `mod undeclared;` line in tests/loom/src/lib.rs"},
		{"src/vspec/count.rs", "fetch_add", "gated::ok", "tests/loom/src/lib.rs:7 declares `mod gated;` under a cfg"},
		{"src/vspec/count.rs", "fetch_add", "moved::ok", "under a `#[path]` attribute"},
		{"src/vspec/count.rs", "fetch_add", "commented::ok", "no `mod commented;` line"},
		{"src/vspec/count.rs", "fetch_add", "nested::ok", "inside a block or macro"},
		{"src/vspec/count.rs", "fetch_add", "ignored::ok", "tests/loom/src/ignored.rs:1 holds an inner cfg attribute"},
		{"src/vspec/count.rs", "fetch_add", "shut::ok", ""},
		{"src/vspec/count.rs", "fetch_add", "open::ok", "tests/loom/src/open.rs is not there or ends inside a literal or comment"},
	} {
		at := "seams: verus/ledger/" + c.file + ".overrides: " + c.name + ": loom: " + c.target
		got := resolveCites(d, r, []*entry{{file: c.file, name: c.name, okey: c.file + ":" + c.name, stat: "trusted (partly checked, loom: " + c.target + "; not checked: x)"}})
		switch {
		case c.want == "" && len(got) != 0:
			t.Errorf("%s @ %s from %s: want resolved, got %q", c.name, c.target, c.file, got)
		case c.want != "" && (len(got) != 1 || !strings.HasPrefix(got[0], at+": ") || !strings.Contains(got[0], c.want)):
			t.Errorf("%s @ %s from %s: want an error holding %q, got %q", c.name, c.target, c.file, c.want, got)
		}
	}
}

// A `#N` seam (`fetch_add#2`) is matched by its name alone.
func TestCiteTestIndexed(t *testing.T) {
	r, err := parseRules("r", citeTestRules)
	if err != nil {
		t.Fatal(err)
	}
	d := tree(t, citeTestFiles)
	got := resolveCites(d, r, []*entry{{file: "src/vspec/count.rs", name: "fetch_add", okey: "src/vspec/count.rs:fetch_add#2", stat: "trusted (partly checked, loom: m::loom_ok; not checked: x)"}})
	if len(got) != 0 {
		t.Errorf("got %q", got)
	}
}

// Without the CFG field, the `#[cfg(test)]` on lib.rs's `mod m;` gates
// the model.
func TestCiteTestNoCfg(t *testing.T) {
	r, err := parseRules("r", resolveRules+"cite-tag\tloom\ncite-test\tloom\ttests/loom/src\n")
	if err != nil {
		t.Fatal(err)
	}
	d := tree(t, citeTestFiles)
	got := resolveCites(d, r, []*entry{{file: "src/vspec/count.rs", name: "fetch_add", okey: "src/vspec/count.rs:fetch_add", stat: "trusted (partly checked, loom: m::loom_ok, n::loom_lock; not checked: x)"}})
	want := "seams: verus/ledger/src/vspec/count.rs.overrides: fetch_add: loom: m::loom_ok: tests/loom/src/lib.rs:3 declares `mod m;` under a cfg, cfg_attr or ignore attribute, which may leave the file out\n" +
		"seams: verus/ledger/src/vspec/count.rs.overrides: fetch_add: loom: n::loom_lock: tests/loom/src/lib.rs:4 declares `mod n;` under a cfg, cfg_attr or ignore attribute, which may leave the file out"
	if strings.Join(got, "\n") != want {
		t.Errorf("got:\n%s\nwant:\n%s", strings.Join(got, "\n"), want)
	}
}

func TestCiteTestRules(t *testing.T) {
	base := "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\ncite-tag\tK\n"
	for _, c := range []struct{ text, want string }{
		{base + "cite-test\tX\td\n", "r:6: cite-test X: not a cite-tag listed above it"},
		{base + "cite-test\tK\n", "r:6: want cite-test<TAB>TAG<TAB>DIR[<TAB>CFG...]"},
		{base + "cite-test\tK\td\t\n", "with no empty field"},
		{base + "cite-test\tK\t../d\n", "r:6: cite-test ../d: want a path inside the tree"},
		{base + "cite-test\tK\td\ncite-test\tK\te\n", "r:7: cite-test K: the tag already has a cite-fn, cite-assert or cite-test line"},
		{base + "cite-fn\tK\ttest\ncite-test\tK\td\n", "r:7: cite-test K: the tag already has"},
		{base + "cite-test\tK\td\ncite-assert\tK\n", "r:7: cite-assert K: the tag already has"},
		{base + "cite-test\tK\td\tfeature = \"loom\"\n", "r:6: cite-test K: CFG feature = \"loom\" holds a literal"},
	} {
		_, err := parseRules("r", c.text)
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("rules %q: want error %q, got %v", c.text, c.want, err)
		}
	}
	r, err := parseRules("r", base+"cite-test\tK\ttests/loom/src/\ttest\tany(test, loom)\ncite-row\tK\tR.md\tH\n")
	if err != nil || r.CiteTest["K"].dir != "tests/loom/src" || strings.Join(r.CiteTest["K"].cfgs, "|") != "#[cfg(test)]|#[cfg(any(test, loom))]" || r.CiteRow["K"].file != "R.md" {
		t.Errorf("good rules: %v %+v", err, r)
	}
}

// lib.rs's own inner cfg gates every model; with no lib.rs no model is
// declared.
func TestCiteTestLib(t *testing.T) {
	r, err := parseRules("r", citeTestRules)
	if err != nil {
		t.Fatal(err)
	}
	model := lines("//! `src/vspec/count.rs`", "/// Checks `count::fetch_add`.", "#[test]", "fn ok() {", "}")
	for _, c := range []struct {
		lib, want string
	}{
		{lines("#![cfg(loom)]", "mod m;"), "tests/loom/src/lib.rs:1 holds an inner cfg attribute"},
		{lines("#![cfg(test)]", "mod m;"), ""},
		{"", "no `mod m;` line in tests/loom/src/lib.rs (read whole) declares tests/loom/src/m.rs"},
		{lines("mod m;", "/*"), "no `mod m;` line in tests/loom/src/lib.rs (read whole)"},
	} {
		files := map[string]string{"tests/loom/src/m.rs": model, "tests/loom/Cargo.toml": citeTestManifest}
		if c.lib != "" {
			files["tests/loom/src/lib.rs"] = c.lib
		}
		got := resolveCites(tree(t, files), r, []*entry{{file: "src/vspec/count.rs", name: "fetch_add", okey: "src/vspec/count.rs:fetch_add", stat: "trusted (partly checked, loom: m::ok; not checked: x)"}})
		if c.want == "" && len(got) != 0 || c.want != "" && (len(got) != 1 || !strings.Contains(got[0], c.want)) {
			t.Errorf("lib.rs %q: want %q, got %q", c.lib, c.want, got)
		}
	}
}

// citeTestManifest is a test crate's manifest after challoc's
// tests/loom/Cargo.toml.
var citeTestManifest = lines(
	"[package]",
	"name = \"x-loom\"",
	"edition = \"2024\"",
	"",
	"[dependencies]",
	"loom = \"=0.7.2\"",
)

// The test crate's manifest must keep DIR/lib.rs the library root with
// its tests on, read as the reach check reads the main crate's.
func TestCiteTestManifest(t *testing.T) {
	r, err := parseRules("r", citeTestRules)
	if err != nil {
		t.Fatal(err)
	}
	model := lines("//! `src/vspec/count.rs`", "/// Checks `count::fetch_add`.", "#[test]", "fn ok() {", "}")
	for _, c := range []struct {
		man, want string // man "-": no manifest
	}{
		{citeTestManifest, ""},
		{citeTestManifest + lines("[lib]", "path = \"src/lib.rs\"", "test = true", "harness = true"), ""},
		{"-", "there is no tests/loom/Cargo.toml beside the source directory"},
		{citeTestManifest + lines("[lib]", "path = \"src/other.rs\""), "sets a target path \"src/other.rs\""},
		{citeTestManifest + lines("[lib]", "test = false"), "tests/loom/Cargo.toml sets test = false under [lib], so `cargo test` runs none of tests/loom/src/lib.rs's tests"},
		{citeTestManifest + lines("[lib]", "harness = false"), "sets harness = false under [lib]"},
		{lines("[package]", "name = \"x\"", "autolib = false"), "sets autolib = false"},
		{lines("[workspace]", "members = []"), "has no [package] table"},
		{citeTestManifest + lines("lib.test = false"), "a dotted key or an inline table"},
		// `test = false` under another table is not the library's.
		{citeTestManifest + lines("[[test]]", "name = \"t\"", "test = false"), ""},
	} {
		files := map[string]string{"tests/loom/src/m.rs": model, "tests/loom/src/lib.rs": lines("mod m;")}
		if c.man != "-" {
			files["tests/loom/Cargo.toml"] = c.man
		}
		got := resolveCites(tree(t, files), r, []*entry{{file: "src/vspec/count.rs", name: "fetch_add", okey: "src/vspec/count.rs:fetch_add", stat: "trusted (partly checked, loom: m::ok; not checked: x)"}})
		if c.want == "" && len(got) != 0 || c.want != "" && (len(got) != 1 || !strings.Contains(got[0], c.want)) {
			t.Errorf("manifest %q: want %q, got %q", c.man, c.want, got)
		}
	}
}
