package main

import (
	"path/filepath"
	"slices"
	"sort"
	"strings"
	"testing"
)

const resolveRules = "command\tseams -write\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\nguess\tC\tkind=.\ntest-file\ttests.rs\n" +
	"cite-tag\tKani\ncite-tag\tcallers\ncite-tag\tconst assert\n" +
	"cite-fn\tKani\tkani::proof\tkani\ncite-fn\tcallers\ttest\tfeature = \"std\"\ncite-module\tkani_proofs\tsrc/kani\ncite-module\text\tproofs\n" +
	"cite-assert\tconst assert\n" +
	"cite-row\tcallers\tverus/README.md\tGuard tests\tMutants killed:\tKnown escapes:\n"

var resolveFiles = map[string]string{
	"src/a.rs": lines("verus! {", "#[verifier::external_body]", "fn f() {}", "}"),
	"src/kani/ptr.rs": lines(
		"#[kani::proof]",
		"fn ptr_add() {}",
		"",
		"#[kani::proof] // a trailing comment",
		"// a comment line, at any indentation",
		"        // and one deeper",
		"#[kani::unwind(3)]",
		"/* a block",
		"   comment */",
		"fn ptr_sub<T>() {}",
		"",
		"#[kani::proof]",
		"",
		"fn gap() {}",
		"",
		"/*",
		"#[kani::proof]",
		"*/",
		"fn commented() {}",
		"",
		"#[test]",
		"fn a_test() {}",
		"",
		"#[kani::proof]",
		"pub(crate) fn visible() {}",
		"",
		"#[kani::proof]",
		"async fn qualified() {}",
		"",
		"#[kani::proof]",
		"#[cfg(any())]",
		"fn gated() {}",
		"",
		"#[cfg(kani)]",
		"// gated above the attribute too",
		"#[kani::proof]",
		"fn gated_above() {}",
		"",
		"#[kani::proof]",
		"#[cfg_attr(kani, kani::unwind(2))]",
		"fn gated_attr() {}",
		"",
		"#[cfg(kani)]",
		"fn before() {}",
		"#[kani::proof]",
		"#[ignore_not]",
		"fn after_gated_fn() {}",
		"",
		// A cfg rustfmt wraps over lines, one on a line after another
		// attribute, or one whose `#[` stands alone, still gates the fn.
		"#[cfg(all(",
		"    feature = \"a\",",
		"    target_os = \"linux\"",
		"))]",
		"#[kani::proof]",
		"fn wrapped_cfg() {}",
		"",
		"#[kani::proof]",
		"#[kani::unwind(2)] #[cfg(any())]",
		"fn same_line() {}",
		"",
		"#[",
		"    cfg(any())",
		"]",
		"#[kani::proof]",
		"fn split_open() {}",
		"",
		// A wrapped attribute that is no gate keeps the block, and text
		// in a literal is no attribute.
		"#[kani::proof]",
		"#[kani::stub(",
		"    a::b,",
		"    c::d",
		")]",
		"fn wrapped_stub() {}",
		"",
		"#[kani::proof]",
		"#[doc = \"#[cfg(x)] [\"]",
		"fn doc_text() {}",
		"",
		"#[cfg(kani)] #[cfg(any())]",
		"#[kani::proof]",
		"fn allowed_not_whole() {}",
		"",
		"#[cfg(any())]",
		"",
		"#[kani::proof]",
		"fn blank_cfg() {}",
		"",
		"  #[cfg(any())]",
		"#[kani::proof]",
		"fn other_indent_cfg() {}",
		"",
		"#[kani::proof]",
		"  #[inline]",
		"fn odd_indent() {}",
		"",
		// `#` and `[` are two tokens: blanks between them.
		"# [cfg(any())]",
		"#[kani::proof]",
		"fn spaced_cfg() {}",
		"",
		"#[kani::unwind(2)] #\t[cfg(any())]",
		"#[kani::proof]",
		"fn spaced_shared() {}",
		"",
		"#",
		"// a comment between the tokens",
		"[cfg(any())]",
		"#[kani::proof]",
		"fn hash_alone() {}",
		"",
		// A lone `#` before other code is no attribute: the next fn
		// still resolves.
		"#",
		"fn not_attr() {}",
		"#[kani::proof]",
		"fn after_hash() {}"),
	"src/kani.rs": lines("#[kani::proof]", "fn top() {}"),
	"src/heap/mod.rs": lines(
		"mod kani_x {",
		"    #[kani::proof]",
		"    pub(crate) fn inner() {}",
		"}",
		"#[kani::proof]",
		"    fn indented() {}",
		"#[kani::proof]",
		"mod carry {",
		"    fn g() {}",
		"}",
		"pub mod a {",
		"    #[kani::proof]",
		"    fn outside() {}",
		"    pub(super) mod b {",
		"        #[kani::proof]",
		"        fn deep() {}",
		"    }",
		"}",
		"/*",
		"mod fake {",
		"    #[kani::proof]",
		"    fn f() {}",
		"}",
		"*/",
		"#[cfg(test)]",
		"mod tests {",
		"    #[test]",
		"    fn guard() {}",
		"    #[test]",
		"    fn unlisted() {}",
		"    #[test]",
		"    fn other_section() {}",
		"    #[test]",
		"    fn one_part() {}",
		"    #[test]",
		"    #[ignore = \"slow\"]",
		"    fn ignored() {}",
		// An allowed cfg with a literal is matched as written: `abc`
		// lexes as `std` does.
		"    #[cfg(feature = \"std\")]",
		"    #[test]",
		"    fn std_only() {}",
		"    #[cfg(feature = \"abc\")]",
		"    #[test]",
		"    fn abc_only() {}",
		"}"),
	// src/lock.rs is read, not src/lock/mod.rs.
	"src/lock/mod.rs": lines("#[kani::proof]", "fn in_mod_rs() {}"),
	"src/lock.rs": lines(
		"mod tests {",
		"}",
		"#[test]",
		"fn late() {}"),
	// Outside the source directory, so only the citation check reads it.
	"proofs/open.rs": lines("#[kani::proof]", "fn f() {}", "// `O`", "const _: () = assert!(true);", "/* never closed"),
	"proofs/shut.rs": lines("#[kani::proof]", "fn f() {}", "/* closed */", "// `P`", "const _: () = assert!(true);"),
	// An assertion counts only in a .rs file of the crate, not a test
	// file.
	"docs/notes.md": lines("// `N`", "const _: () = assert!(false);"),
	"proofs.rs":     lines("// `Q`", "const _: () = assert!(true);"),
	"src/notes.txt": lines("// `N`", "const _: () = assert!(false);"),
	"src/tests.rs":  lines("// `N`", "const _: () = assert!(false);"),
	"src/layout.rs": lines(
		"// `L1` the layout",
		"// and more",
		"const _: () = assert!(true);",
		"#[cfg(unix)]",
		"",
		"// `L2` gated",
		"const _: () = assert!(",
		"    true",
		");",
		"// `L3` twice",
		"const _: () = assert!(true);",
		"// `L3` again",
		"const _: () = assert!(true);",
		"/*",
		"// `L4` in a comment",
		"const _: () = assert!(true);",
		"*/",
		"// `L5`",
		"const _: () = assert!(true); let x = 1;",
		"// `L6`",
		"",
		"const _: () = assert!(true);",
		"// see `L7`",
		"const _: () = assert!(true);",
		"// `L8`",
		"const _: () = assert!(true);",
		// The attribute is still the last code above the label across a
		// block comment, or a comment line and a blank line.
		"#[cfg(any())]",
		"/* layout check */",
		"// `L9`",
		"const _: () = assert!(false);",
		"#[cfg(any())]",
		"// a note",
		"",
		"// `L10`",
		"const _: () = assert!(false);",
		"#[cfg(all(",
		"    unix,",
		"))]",
		"// `L11`",
		"const _: () = assert!(false);"),
	"verus/README.md": lines(
		"## Guard tests",
		"",
		"| test | what it pins |",
		"|---|---|",
		"| `heap::tests::guard` | x. Mutants killed: 3. Known escapes: none |",
		"| `heap::tests::one_part` | x. Mutants killed: 3. |",
		"| `heap::tests::ignored` | x. Mutants killed: 3. Known escapes: none |",
		"| `heap::tests::std_only` | x. Mutants killed: 3. Known escapes: none |",
		"| `heap::tests::abc_only` | x. Mutants killed: 3. Known escapes: none |",
		"| `kani_proofs::ptr::a_test` | x. Mutants killed: 1. Known escapes: none |",
		"| other | see `heap::tests::unlisted`. Mutants killed: 1. Known escapes: none |",
		"- | `heap::tests::unlisted` | Mutants killed: 1. Known escapes: none |",
		"",
		"## Other",
		"",
		"| `heap::tests::other_section` | x. Mutants killed: 3. Known escapes: none |"),
}

// resolveRun writes the tree with seam f citing each check in status,
// and returns the citation lines -check prints, sorted.
func resolveRun(t *testing.T, status string) []string {
	t.Helper()
	files := map[string]string{"verus/seams.rules": resolveRules, "verus/ledger/src/a.rs.overrides": "f\tC\tT\t" + status + "\n"}
	for k, v := range resolveFiles {
		files[k] = v
	}
	d := tree(t, files)
	if code, _, e := runIn(t, d, "-write"); code != 0 {
		t.Fatalf("-write: %d %s", code, e)
	}
	code, _, e := runIn(t, d, "-check")
	var got []string
	for _, l := range strings.Split(strings.TrimSuffix(e, "\n"), "\n") {
		if l = strings.TrimPrefix(l, "seams: verus/ledger/src/a.rs.overrides: f: "); l != "" && !strings.HasPrefix(l, "seams:") {
			got = append(got, l)
		}
	}
	if (code != 0) != (len(got) > 0) {
		t.Errorf("exit %d with lines %q (stderr %q)", code, got, e)
	}
	sort.Strings(got)
	return got
}

func TestResolveFn(t *testing.T) {
	ok := []string{"kani_proofs::ptr::ptr_add", "kani_proofs::ptr::ptr_sub", "kani_proofs::ptr::visible",
		"kani_proofs::top", "heap::kani_x::inner", "heap::a::b::deep", "ext::shut::f", "kani_proofs::ptr::after_gated_fn",
		"kani_proofs::ptr::wrapped_stub", "kani_proofs::ptr::doc_text",
		"kani_proofs::ptr::gated_above", // #[cfg(kani)], which the rules allow
		"kani_proofs::ptr::after_hash"}
	// Found, but under a gate: maybe never built.
	gated := []string{
		"kani_proofs::ptr::gated", // a cfg between
		"kani_proofs::ptr::gated_attr",
		"kani_proofs::ptr::wrapped_cfg",
		"kani_proofs::ptr::same_line",
		"kani_proofs::ptr::split_open",
		"kani_proofs::ptr::allowed_not_whole", // #[cfg(kani)] and another on its line
		"kani_proofs::ptr::blank_cfg",         // a blank line between the cfg and the attribute
		"kani_proofs::ptr::other_indent_cfg",  // a cfg at another indentation
		"kani_proofs::ptr::spaced_cfg",        // `# [cfg(..)]`
		"kani_proofs::ptr::spaced_shared",     // the same after another attribute on its line
		"kani_proofs::ptr::hash_alone",        // `#` / comment / `[cfg(..)]`
	}
	bad := []string{
		"kani::ptr::ptr_add",           // reaches src/kani not by its module name
		"kani_proofs::ptr::gap",        // a blank line between
		"kani_proofs::ptr::commented",  // the attribute is in a comment
		"kani_proofs::ptr::a_test",     // #[test], not #[kani::proof]
		"kani_proofs::ptr::qualified",  // `async fn` is not read
		"kani_proofs::ptr::odd_indent", // an attribute at another indentation between
		"kani_proofs::ptr::nothing",
		"ptr_add",                // one segment
		"heap::inner",            // not at the top level
		"heap::indented",         // not at the top level's indentation
		"heap::fake::f",          // the mod is in a comment
		"heap::kani_x::inner::x", // no such mod
		"heap::kani.x::inner",    // no identifier
		"kani::top",              // src/kani.rs, not by its module name
		"heap::carry::g",         // the attribute is above the mod
		"heap::kani_x::outside",  // in another mod at that indentation
		"heap::kani_x::f",        // left the mod before an f
		"ext::open::f",           // the file ends inside a comment
		"nowhere::f",             // no file
		"lock::in_mod_rs",        // src/lock.rs comes first
		"kani_proofs::ptr::ptr_add_x",
	}
	got := resolveRun(t, "trusted (Kani: "+strings.Join(slices.Concat(ok, gated, bad), ", ")+")")
	var want []string
	for _, b := range bad {
		want = append(want, "Kani: "+b+" names no #[kani::proof] fn")
	}
	for _, g := range gated {
		want = append(want, "Kani: "+g+" names a #[kani::proof] fn under a cfg, cfg_attr or ignore attribute, which may keep it out of every build that checks it (a cite-fn CFG field allows `#[cfg(CFG)]`)")
	}
	sort.Strings(want)
	if strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Errorf("got:\n%s\nwant:\n%s", strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

func TestResolveCallers(t *testing.T) {
	got := resolveRun(t, "trusted (callers: heap::tests::guard, kani_proofs::ptr::a_test, heap::tests::unlisted, heap::tests::other_section, heap::tests::one_part, heap::tests::ignored, heap::tests::std_only, heap::tests::abc_only, lock::tests::late, kani_proofs::ptr::ptr_add)")
	row := " has no row in the `## Guard tests` table of verus/README.md that names it in backquotes in its first cell and holds `Mutants killed:` and `Known escapes:`"
	want := []string{
		"callers: heap::tests::abc_only names a #[test] fn under a cfg, cfg_attr or ignore attribute, which may keep it out of every build that checks it (a cite-fn CFG field allows `#[cfg(CFG)]`)",
		"callers: heap::tests::ignored names a #[test] fn under a cfg, cfg_attr or ignore attribute, which may keep it out of every build that checks it (a cite-fn CFG field allows `#[cfg(CFG)]`)",
		"callers: heap::tests::one_part" + row,
		"callers: heap::tests::other_section" + row,
		"callers: heap::tests::unlisted" + row,
		"callers: kani_proofs::ptr::ptr_add names no #[test] fn",
		"callers: lock::tests::late names no #[test] fn",
	}
	if strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Errorf("got:\n%s\nwant:\n%s", strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

func TestResolveAssertion(t *testing.T) {
	bad := []string{"`L2` gated in src/layout.rs", "`L3` in src/layout.rs", "`L4` in src/layout.rs", "`L5` in src/layout.rs",
		"`L6` in src/layout.rs", "`L7` in src/layout.rs", "`L1` in src/a.rs", "`L1` in ../layout.rs", "`L1` in /src/layout.rs", "`L1`", "`L1` in src/none.rs", "`O` in proofs/open.rs",
		"`N` in docs/notes.md", "`N` in src/notes.txt", "`N` in src/tests.rs",
		"`L9` in src/layout.rs", "`L10` in src/layout.rs", "`L11` in src/layout.rs"}
	got := resolveRun(t, "trusted (const assert: `L1` in src/layout.rs, `L1` the layout in src/layout.rs, `L8` in src/layout.rs, `P` in proofs/shut.rs, `Q` in proofs.rs,"+strings.Join(bad, ", ")+")")
	var want []string
	for _, b := range bad {
		want = append(want, "const assert: "+b+" names no one labeled assertion (a top-level `const _: () = assert!(` right under a `// LABEL` comment, no attribute above it, in FILE)")
	}
	sort.Strings(want)
	if strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Errorf("got:\n%s\nwant:\n%s", strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

// A seam whose name serves several seams prints a failing citation once,
// and a tag with no cite-fn, cite-assert or cite-row line is not
// resolved.
func TestResolveOnce(t *testing.T) {
	r, err := parseRules("r", resolveRules+"cite-tag\tloom\ncite-tag\tA\ncite-assert\tA\ncite-row\tA\tR.md\tH\n")
	if err != nil {
		t.Fatal(err)
	}
	// A's row is checked only once its assertion resolves.
	st := "trusted (Kani: x::y; loom: a::b; A: `X` in none.rs)"
	es := []*entry{{file: "src/a.rs", okey: "src/a.rs:f", stat: st}, {file: "src/a.rs", okey: "src/a.rs:f", stat: st}}
	got := resolveCites(t.TempDir(), r, es)
	want := []string{"seams: verus/ledger/src/a.rs.overrides: f: Kani: x::y names no #[kani::proof] fn",
		"seams: verus/ledger/src/a.rs.overrides: f: A: `X` in none.rs names no one labeled assertion (a top-level `const _: () = assert!(` right under a `// LABEL` comment, no attribute above it, in FILE)"}
	if strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Errorf("got %q", got)
	}
}

// An assertion's FILE may not leave the tree, though the file it names
// is there by that path too.
func TestResolveEscape(t *testing.T) {
	r, err := parseRules("r", resolveRules)
	if err != nil {
		t.Fatal(err)
	}
	d := tree(t, resolveFiles)
	for _, file := range []string{"src/layout.rs", "src/../src/layout.rs", "../" + filepath.Base(d) + "/src/layout.rs", filepath.ToSlash(filepath.Join(d, "src/layout.rs"))} {
		got := resolveCites(d, r, []*entry{{file: "src/a.rs", okey: "src/a.rs:f", stat: "trusted (const assert: `L1` in " + file + ")"}})
		if inside := !strings.HasPrefix(file, "../") && !filepath.IsAbs(file); inside != (len(got) == 0) {
			t.Errorf("%s: got %q", file, got)
		}
	}
}

// A LABEL of blanks alone names nothing, though the file holds one
// assertion under a `// ` comment.
func TestResolveEmptyLabel(t *testing.T) {
	r, err := parseRules("r", resolveRules)
	if err != nil {
		t.Fatal(err)
	}
	d := tree(t, resolveFiles)
	for _, st := range []string{"trusted (const assert: `Q` in proofs.rs)", "trusted (const assert:  in proofs.rs)", "trusted (const assert:    in proofs.rs)"} {
		got := resolveCites(d, r, []*entry{{file: "src/a.rs", okey: "src/a.rs:f", stat: st}})
		if want := strings.Contains(st, "`Q`"); want != (len(got) == 0) {
			t.Errorf("%s: got %q", st, got)
		}
	}
}

func TestResolveRules(t *testing.T) {
	base := "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\ncite-tag\tK\n"
	for _, c := range []struct{ text, want string }{
		{base + "cite-fn\tX\tkani::proof\n", "r:6: cite-fn X: not a cite-tag listed above it"},
		{base + "cite-fn\tK\n", "r:6: want cite-fn<TAB>TAG<TAB>ATTR[<TAB>CFG...]"},
		{base + "cite-fn\tK\ttest\t\n", "r:6: want cite-fn<TAB>TAG<TAB>ATTR[<TAB>CFG...], with no empty field"},
		{base + "cite-fn\tK\t#[test]\n", "r:6: cite-fn K: ATTR #[test]: want the attribute's path"},
		{base + "cite-fn\tK\ttest\ncite-fn\tK\ttest\n", "r:7: cite-fn K: the tag already has a cite-fn or cite-assert line"},
		{base + "cite-fn\tK\ttest\ncite-assert\tK\n", "r:7: cite-assert K: the tag already has"},
		{base + "cite-assert\tK\ncite-fn\tK\ttest\n", "r:7: cite-fn K: the tag already has"},
		{base + "cite-assert\tK\tx\n", "r:6: want cite-assert<TAB>TAG"},
		{base + "cite-row\tK\tf.md\n", "r:6: want cite-row<TAB>TAG<TAB>FILE<TAB>HEADING<TAB>PART..."},
		{base + "cite-row\tK\t../f.md\tH\n", "r:6: cite-row ../f.md: want a path inside the tree"},
		{base + "cite-row\tK\tf.md\tH\ncite-row\tK\tg.md\tH\n", "r:7: cite-row K is given twice"},
		{base + "cite-row\tK\tf.md\tH\t\n", "with no empty field"},
		{base + "cite-module\tk-p\tsrc/kani\n", "r:6: cite-module k-p: want an identifier"},
		{base + "cite-module\tkp\t/src\n", "r:6: cite-module /src: want a path inside the tree"},
		{base + "cite-module\tkp\tsrc/k\ncite-module\tkp\tsrc/j\n", "r:7: cite-module kp is given twice"},
		{base + "cite-module\tkp\n", "r:6: want cite-module<TAB>NAME<TAB>DIR"},
	} {
		_, err := parseRules("r", c.text)
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("rules %q: want error %q, got %v", c.text, c.want, err)
		}
	}
	r, err := parseRules("r", base+"cite-fn\tK\tkani::proof\tkani\tall(kani, x)\ncite-module\tkp\tsrc/kani/\ncite-row\tK\tv/R.md\tGuard tests\tA:\tB:\n")
	if err != nil || r.CiteFn["K"] != "kani::proof" || strings.Join(r.CiteFnCfg["K"], "|") != "#[cfg(kani)]|#[cfg(all(kani, x))]" || r.CiteModule["kp"] != "src/kani" || r.CiteRow["K"].file != "v/R.md" || r.CiteRow["K"].heading != "Guard tests" || strings.Join(r.CiteRow["K"].parts, "|") != "A:|B:" {
		t.Errorf("good rules: %v %+v", err, r)
	}
	r, err = parseRules("r", base+"cite-assert\tK\ncite-row\tK\tR.md\tH\n")
	if err != nil || !r.CiteAssert["K"] || len(r.CiteRow["K"].parts) != 0 {
		t.Errorf("good rules: %v %+v", err, r)
	}
}
