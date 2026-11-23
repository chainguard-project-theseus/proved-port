package main

import (
	"strings"
	"testing"
)

// reach: a cited file counts only when every file from it up to the
// crate root is reached ungated (#237).
func TestReach(t *testing.T) {
	r, err := parseRules("r", "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\ncite-module\tkp\tproofs\ncite-module\tkani_proofs\tsrc/kani\n")
	if err != nil {
		t.Fatal(err)
	}
	d := tree(t, map[string]string{
		"Cargo.toml": "[package]\nname = \"t\"\n",
		"src/lib.rs": lines(
			"#![cfg_attr(any(kani, x), deny(warnings))]",
			"mod good;",
			"#[cfg(any())]",
			"mod gated;",
			"#[cfg_attr(x, path = \"y.rs\")]",
			"mod attr;",
			"#[path = \"other.rs\"]",
			"mod pathed;",
			"pub(crate) mod sub;",
			"#[cfg(any())] mod sameline;",
			"#[",
			"    cfg(any())",
			"]",
			"// a comment between",
			"",
			"mod wrapped;",
			"// mod commented;",
			"mod lintonly;",
			"mod sub2;",
			"mod sub3;",
			"#[cfg(any())]",
			"fn other() {}",
			"mod after_item;",
			"# [cfg(any())]",
			"mod spaced;",
			"mod inline_only {",
			"}",
			// A `mod` line in a block is not read as reached.
			"cfg_if::cfg_if! {",
			"    if #[cfg(kani)] {",
			"        mod in_cfg_if;",
			"    }",
			"}",
			"macro_rules! m {",
			"    () => {",
			"        mod in_macro;",
			"    };",
			"}",
			"mod outer {",
			"    mod in_outer;",
			"}",
			"mod twice;",
			"mod outer2 {",
			"    mod twice;",
			"}",
			// An attribute after code on the line before attaches to the
			// next item; a macro's arguments in parentheses or brackets
			// hold no declaration; a block's inner attribute is its own.
			"mod a2; #[cfg(any())]",
			"mod trailing;",
			"macro_rules! discard { ($($t:tt)*) => {}; }",
			"discard!(",
			"    mod in_parens;",
			");",
			"discard![",
			"    mod in_brackets;",
			"];",
			"mod tests {",
			"    #![cfg(test)]",
			"}",
			// A `#` ending a line in a skipped test module, its `[` on
			// the next line, is one attribute.
			"#[cfg(test)]",
			"mod t2 {",
			"} #",
			"[cfg(any())]",
			"mod split_hash;",
			"mod x;",
			// An item's head split over lines keeps the attributes above.
			"#[cfg(any())]",
			"pub",
			"mod vis1;",
			"#[cfg(any())] pub",
			"mod vis2;",
			"#[cfg(any())]",
			"pub(crate)",
			"mod vis3;",
			"mod",
			"split_name;",
			// A trailing attribute and a `#` ending the same line.
			"#[cfg(test)]",
			"mod t3 {",
			"} #[cfg(any())] #",
			"[allow(dead_code)]",
			"mod both_hash;"),
		"src/good.rs":        "",
		"src/gated.rs":       "",
		"src/attr.rs":        "",
		"src/pathed.rs":      "",
		"src/missing.rs":     "",
		"src/commented.rs":   "",
		"src/sameline.rs":    "",
		"src/wrapped.rs":     "",
		"src/after_item.rs":  "",
		"src/spaced.rs":      "",
		"src/inline_only.rs": "",
		"src/in_cfg_if.rs":   "",
		"src/in_macro.rs":    "",
		"src/in_outer.rs":    "",
		"src/twice.rs":       "",
		"src/trailing.rs":    "",
		"src/in_parens.rs":   "",
		"src/in_brackets.rs": "",
		"src/split_hash.rs":  "",
		// lib.rs's `mod x;` declares src/x.rs, never src/lib/x.rs; a
		// mod.rs's `mod xm;` declares the file beside it.
		"src/x.rs":          "",
		"src/lib/x.rs":      "",
		"src/sub/xm.rs":     "",
		"src/sub/mod/xm.rs": "",
		"src/vis1.rs":       "",
		"src/vis2.rs":       "",
		"src/vis3.rs":       "",
		"src/split_name.rs": "",
		"src/both_hash.rs":  "",
		// A cite-module DIR.rs under SOURCE is an ordinary module file.
		"src/kani.rs":             "",
		"src/kani/k.rs":           "",
		"src/lintonly.rs":         lines("#![cfg_attr(not(x), allow(dead_code))]", "const A: u8 = 1;"),
		"src/sub/mod.rs":          lines("mod deep;", "mod inner;", "mod adds;", "mod winner;", "mod spaced_inner;", "mod xm;"),
		"src/sub/deep.rs":         "",
		"src/sub/none.rs":         "",
		"src/sub/inner.rs":        lines("//! docs", "#![cfg(any())]"),
		"src/sub/adds.rs":         lines("#![cfg_attr(x, cfg(y))]"),
		"src/sub/winner.rs":       lines("#![cfg_attr(", "    x,", "    cfg(y)", ")]"),
		"src/sub/spaced_inner.rs": lines("# ! [cfg(any())]"),
		"src/sub2.rs":             "mod leaf;\n",
		"src/sub2/leaf.rs":        "",
		"src/sub3/mod.rs":         lines("#![cfg(any())]", "mod d;"),
		"src/sub3/d.rs":           "",
		"src/open.rs":             "/* never closed\n",
		"proofs/x.rs":             "",
	})
	v := &resolver{root: d, r: r, files: map[string]*srcFile{}}
	for _, c := range []struct{ file, want string }{
		{"src/lib.rs", ""},
		{"src/good.rs", ""},
		{"src/sub/deep.rs", ""},
		{"src/sub2/leaf.rs", ""},
		{"src/lintonly.rs", ""},
		{"src/after_item.rs", ""}, // the cfg is on the fn above it
		{"proofs/x.rs", ""},       // a cite-module DIR is reached by #[path]
		{"src/gated.rs", "src/lib.rs:4 declares `mod gated;` under a cfg, cfg_attr or ignore attribute"},
		{"src/attr.rs", "src/lib.rs:6 declares `mod attr;` under a cfg, cfg_attr or ignore attribute"},
		{"src/pathed.rs", "src/lib.rs:8 declares `mod pathed;` under a `#[path]` attribute"},
		{"src/sameline.rs", "src/lib.rs:10 declares `mod sameline;` under a cfg"},
		{"src/wrapped.rs", "src/lib.rs:16 declares `mod wrapped;` under a cfg"},
		{"src/spaced.rs", "src/lib.rs:25 declares `mod spaced;` under a cfg"},
		{"src/missing.rs", "no `mod missing;` line in src/lib.rs reaches src/missing.rs"},
		{"src/commented.rs", "no `mod commented;` line in src/lib.rs"},
		{"src/inline_only.rs", "no `mod inline_only;` line"}, // an inline mod declares no file
		{"src/in_cfg_if.rs", "src/lib.rs:30 declares `mod in_cfg_if;` inside a block or macro"},
		{"src/in_macro.rs", "src/lib.rs:35 declares `mod in_macro;` inside a block or macro"},
		{"src/in_outer.rs", "src/lib.rs:39 declares `mod in_outer;` inside a block or macro"},
		{"src/twice.rs", ""}, // the top-level line counts, the nested one aside
		{"src/trailing.rs", "src/lib.rs:46 declares `mod trailing;` under a cfg"},
		{"src/in_parens.rs", "src/lib.rs:49 declares `mod in_parens;` inside a block or macro"},
		{"src/in_brackets.rs", "src/lib.rs:52 declares `mod in_brackets;` inside a block or macro"},
		{"src/split_hash.rs", "src/lib.rs:61 declares `mod split_hash;` under a cfg"},
		{"src/x.rs", ""},
		{"src/lib/x.rs", "no `mod x;` line in src/lib/mod.rs reaches src/lib/x.rs"},
		{"src/sub/xm.rs", ""},
		{"src/sub/mod/xm.rs", "no `mod xm;` line in src/sub/mod/mod.rs reaches src/sub/mod/xm.rs"},
		{"src/vis1.rs", "src/lib.rs:65 declares `mod vis1;` under a cfg"},
		{"src/vis2.rs", "src/lib.rs:67 declares `mod vis2;` under a cfg"},
		{"src/vis3.rs", "src/lib.rs:70 declares `mod vis3;` under a cfg"},
		{"src/split_name.rs", ""},
		{"src/both_hash.rs", "src/lib.rs:77 declares `mod both_hash;` under a cfg"},
		{"src/kani.rs", "no `mod kani;` line in src/lib.rs reaches src/kani.rs"},
		{"src/kani/k.rs", ""}, // under the cite-module DIR
		{"src/sub/inner.rs", "src/sub/inner.rs:2 holds an inner cfg attribute"},
		{"src/sub/adds.rs", "src/sub/adds.rs:1 holds an inner cfg attribute"},
		{"src/sub/winner.rs", "src/sub/winner.rs:1 holds an inner cfg attribute"},
		{"src/sub/spaced_inner.rs", "src/sub/spaced_inner.rs:1 holds an inner cfg attribute"},
		{"src/sub3/d.rs", "src/sub3/mod.rs:1 holds an inner cfg attribute"},
		{"src/sub/none.rs", "no `mod none;` line in src/sub.rs or src/sub/mod.rs reaches src/sub/none.rs"},
		{"src/open.rs", "src/open.rs cannot be read whole"},
	} {
		got := v.reach(c.file)
		if (c.want == "") != (got == "") || !strings.Contains(got, c.want) {
			t.Errorf("reach(%s) = %q, want %q", c.file, got, c.want)
		}
	}
	// A crate whose root is main.rs; one with no root.
	pkg := "[package]\nname = \"t\"\n"
	d = tree(t, map[string]string{"src/main.rs": "mod m;\n", "src/m.rs": "", "src/main/m.rs": "", "Cargo.toml": pkg})
	v = &resolver{root: d, r: r, files: map[string]*srcFile{}}
	if got := v.reach("src/m.rs"); got != "" {
		t.Errorf("main.rs root: %q", got)
	}
	if got := v.reach("src/main/m.rs"); got != "no `mod m;` line in src/main/mod.rs reaches src/main/m.rs" {
		t.Errorf("main.rs root, src/main/m.rs: %q", got)
	}
	// With both roots, a library build compiles lib.rs's tree alone.
	d = tree(t, map[string]string{"src/lib.rs": "mod l;\n", "src/main.rs": "mod m;\n", "src/l.rs": "", "src/m.rs": "", "Cargo.toml": pkg})
	v = &resolver{root: d, r: r, files: map[string]*srcFile{}}
	for file, want := range map[string]string{
		"src/l.rs":    "",
		"src/m.rs":    "no `mod m;` line in src/lib.rs reaches src/m.rs",
		"src/main.rs": "src/main.rs is the binary's root; with src/lib.rs there, a library build compiles src/lib.rs's tree alone",
	} {
		if got := v.reach(file); got != want {
			t.Errorf("both roots: reach(%s) = %q, want %q", file, got, want)
		}
	}
	d = tree(t, map[string]string{"src/x.rs": ""})
	v = &resolver{root: d, r: r, files: map[string]*srcFile{}}
	if got := v.reach("src/x.rs"); !strings.Contains(got, "there is no crate root (src/lib.rs or src/main.rs) to reach src/x.rs from") {
		t.Errorf("no root: %q", got)
	}
}

// The crate root is lib.rs or main.rs at their default paths: a
// Cargo.toml that moves the target, or turns off its discovery, leaves
// a leftover default root unbuilt.
func TestReachTargets(t *testing.T) {
	r, err := parseRules("r", "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\n")
	if err != nil {
		t.Fatal(err)
	}
	libTree := map[string]string{"src/lib.rs": "mod layout;\n", "src/layout.rs": ""}
	mainTree := map[string]string{"src/main.rs": "mod layout;\n", "src/layout.rs": ""}
	for _, c := range []struct {
		files    map[string]string
		manifest string
		want     string
	}{
		// The manifest must be a package's.
		{libTree, "", "there is no Cargo.toml beside the source directory"},
		{libTree, "![workspace]\nmembers = [\"crates/*\"]\n", "Cargo.toml has no [package] table (a workspace's virtual manifest"},
		{libTree, "!", "Cargo.toml has no [package] table"},
		{libTree, "![package]\nname = \"x\"\n", ""},
		// What the line reader cannot follow: a multi-line string, a line
		// of a multi-line value starting with `[`, an escape, an absolute
		// path.
		{libTree, "!description = \"\"\"\nC ABI shim.\n[experimental]\n\"\"\"\n[package]\nautolib = false\n", "Cargo.toml holds a multi-line string"},
		{libTree, "!x = '''\n'''\n[package]\n", "Cargo.toml holds a multi-line string"},
		{libTree, "[package.metadata]\nm = [\n[1, 2],\n]\n", "Cargo.toml has a line starting with `[` that is not a table header"},
		{libTree, "[lib] x\n", "Cargo.toml has a line starting with `[` that is not a table header"},
		{libTree, "[lib]\n\"p\\u0061th\" = \"src/ffi.rs\"\n", "Cargo.toml holds an escape"},
		{libTree, "[lib]\npath = \"/src/lib.rs\"\n", "Cargo.toml sets a target path \"/src/lib.rs\""},
		{libTree, "[package]\nname = \"x\"\n\n[lib]\ncrate-type = [\"cdylib\", \"staticlib\"]\n", ""},
		{libTree, "[lib]\npath = \"src/lib.rs\"\n", ""},
		{libTree, "[lib]\npath = \"src/ffi.rs\"\n", "Cargo.toml sets a target path \"src/ffi.rs\", so src/lib.rs may not be the crate root it builds"},
		{libTree, "[package]\nautolib = false\n", "Cargo.toml sets autolib = false, so src/lib.rs may not be built"},
		{libTree, "[[bin]]\npath = \"src/bin/x.rs\"\n", ""}, // a bin path leaves the library alone
		{mainTree, "[[bin]]\nname = \"x\"\npath = \"src/bin/x.rs\"\n", "Cargo.toml declares a [[bin]] target, so src/main.rs may not be built as the default binary"},
		{mainTree, "[package]\nautobins = false\n", "Cargo.toml sets autobins = false, so src/main.rs may not be built"},
		{mainTree, "[package]\nautolib = false\n", ""},
		// Any [[bin]] table, with main.rs as the root: it may move the
		// binary, gate it on features, or (edition 2015) turn discovery
		// off; the check reads none of it.
		{mainTree, "[[bin]]\npath = 'src/main.rs'\n", "Cargo.toml declares a [[bin]] target"},
		{mainTree, "[ [bin] ]\npath = 'src/bin/y.rs'\n", "Cargo.toml declares a [[bin]] target"},
		{mainTree, "[package]\nname = \"p\"\n\n[[bin]]\nname = \"p\"\nrequired-features = [\"kani\"]\n", "Cargo.toml declares a [[bin]] target"},
		{mainTree, "[package]\nedition = \"2015\"\n\n[[bin]]\nname = \"other\"\n", "Cargo.toml declares a [[bin]] target"},
		{libTree, "[package]\nautolib = true\n", ""},
		// A library elsewhere while main.rs is the root; comments after a
		// header; dotted keys and inline tables, which are refused.
		{mainTree, "[lib]\npath = \"src/ffi.rs\"\n", "Cargo.toml declares a [lib] target, so a library build does not compile src/main.rs's tree"},
		{libTree, "[lib]  # the C ABI\npath = \"src/ffi.rs\"\n", "Cargo.toml sets a target path \"src/ffi.rs\""},
		{libTree, "[package] # x\nautolib = false\n", "Cargo.toml sets autolib = false"},
		{libTree, "lib.path = \"src/ffi.rs\"\n", "Cargo.toml sets lib.path in a form the check does not read"},
		{libTree, "lib = { path = \"src/ffi.rs\" }\n", "Cargo.toml sets lib in a form the check does not read"},
		{libTree, "package.autolib = false\n", "Cargo.toml sets package.autolib in a form the check does not read"},
		{mainTree, "bin = [{ path = \"src/x.rs\" }]\n", "Cargo.toml sets bin in a form the check does not read"},
		{libTree, "[dependencies]\nlibc = \"0.2\"\nbinread = \"2\"\n", ""},
		{libTree, "[package.metadata.verus]\nverify = true\n", ""},
		{libTree, "package = { autolib = false }\n", "Cargo.toml sets package in a form the check does not read"},
		{libTree, "lib . path = \"src/ffi.rs\"\n", "Cargo.toml sets lib.path in a form the check does not read"},
		// Tabs and quoted keys and headers, as TOML allows.
		{libTree, "[lib]\npath\t= \"src/ffi.rs\"\n", "Cargo.toml sets a target path \"src/ffi.rs\""},
		{libTree, "[lib]\n\"path\" = \"src/ffi.rs\"\n", "Cargo.toml sets a target path \"src/ffi.rs\""},
		{libTree, "[\tlib]\npath = \"src/ffi.rs\"\n", "Cargo.toml sets a target path \"src/ffi.rs\""},
		{libTree, "[\"lib\"]\npath = \"src/ffi.rs\"\n", "Cargo.toml sets a target path \"src/ffi.rs\""},
		{libTree, "[package]\n'autolib' = false\n", "Cargo.toml sets autolib = false"},
		{libTree, "\"lib\".path = \"src/ffi.rs\"\n", "Cargo.toml sets lib.path in a form the check does not read"},
		{libTree, "[features]\nbin = []\nlib = []\n", ""}, // keys of those names in another table
	} {
		files := map[string]string{}
		for k, v := range c.files {
			files[k] = v
		}
		// Each manifest gets a [package] table at its end, but one
		// written with a leading `!`, which is used as it stands.
		switch {
		case strings.HasPrefix(c.manifest, "!"):
			files["Cargo.toml"] = c.manifest[1:]
		case c.manifest != "":
			files["Cargo.toml"] = c.manifest + "\n[package]\nname = \"x\"\n"
		}
		v := &resolver{root: tree(t, files), r: r, files: map[string]*srcFile{}}
		if got := v.reach("src/layout.rs"); (c.want == "") != (got == "") || !strings.Contains(got, c.want) {
			t.Errorf("manifest %q: got %q, want %q", c.manifest, got, c.want)
		}
	}
}

// A cite-module DIR, exempt from the reach check, may not be SOURCE or
// hold it.
func TestReachRules(t *testing.T) {
	base := "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\n"
	for _, c := range []struct{ text, want string }{
		{base + "cite-module\tm\tsrc\n", "r: cite-module m: DIR src is the source directory or holds it"},
		{base + "source\tcrates/a/src\ncite-module\tm\tcrates\n", "r: cite-module m: DIR crates is the source directory or holds it"},
		{base + "cite-module\tm\tcrates/a\nsource\tcrates/a/src\n", "r: cite-module m: DIR crates/a is the source directory or holds it"},
	} {
		if _, err := parseRules("r", c.text); err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("rules %q: want error %q, got %v", c.text, c.want, err)
		}
	}
	for _, text := range []string{base + "cite-module\tm\tsrc/kani\n", base + "cite-module\tm\tsrcx\n", base + "source\tcrates/a/src\ncite-module\tm\tcrates/ab\n"} {
		if _, err := parseRules("r", text); err != nil {
			t.Errorf("rules %q: %v", text, err)
		}
	}
}

// -check reports a cited assertion or a notcopy FILE that a build may
// leave out.
func TestReachCheck(t *testing.T) {
	r, err := parseRules("r", resolveRules)
	if err != nil {
		t.Fatal(err)
	}
	files := map[string]string{}
	for k, v := range resolveFiles {
		files[k] = v
	}
	files["src/lib.rs"] = lines("mod a;", "#[cfg(any())]", "mod layout;")
	d := tree(t, files)
	got := resolveCites(d, r, []*entry{{file: "src/a.rs", okey: "src/a.rs:f", stat: "trusted (const assert: `L1` in src/layout.rs)"}})
	want := "seams: verus/ledger/src/a.rs.overrides: f: const assert: `L1` in src/layout.rs: src/layout.rs may be left out of a build: src/lib.rs:3 declares `mod layout;` under a cfg, cfg_attr or ignore attribute, which may leave the file out"
	if len(got) != 1 || got[0] != want {
		t.Errorf("got %q", got)
	}
	rn, err := parseRules("r", ncRules)
	if err != nil {
		t.Fatal(err)
	}
	d = tree(t, map[string]string{"src/nc.rs": ncFile, "src/lib.rs": "#[cfg(any())]\nmod nc;\n", "Cargo.toml": "[package]\nname = \"t\"\n"})
	got = notCopyChecks(d, rn, nil, nil)
	if len(got) != 1 || !strings.HasPrefix(got[0], "seams: src/nc.rs: may be left out of a build: src/lib.rs:2 declares `mod nc;` under a cfg") {
		t.Errorf("notcopy: %q", got)
	}
}
