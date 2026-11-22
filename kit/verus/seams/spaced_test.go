package main

import (
	"strings"
	"testing"
)

// `#`, `!` and `[` are separate tokens in Rust's grammar, so an
// attribute may hold blanks between them; the scan reads it as it reads
// `#[` (#233).
func TestSpacedAttr(t *testing.T) {
	for _, c := range []struct{ what, src, want string }{
		{"blank after #", lines("verus! {", "# [verifier::external_body]", "fn f() {}", "}"), "external_body f"},
		{"tab and blanks inside", lines("verus! {", "#\t[ verifier :: external_body ]", "fn f() {}", "}"), "external_body f"},
		{"spaced and wrapped", lines("verus! {", "# [verifier::", "  external_body]", "fn f() {}", "}"), "external_body f"},
		{"spaced after another attribute", lines("verus! {", "#[allow(dead_code)] # [verifier::external_body] fn f() {}", "}"), "external_body f"},
		{"spaced before an item on its line", lines("verus! {", "# [verifier::external_body] fn f() {}", "}"), "external_body f"},
		{"spaced axiom attribute block", lines("verus! {", "# [allow(dead_code)]", "pub axiom fn a();", "}"), "axiom a"},
		{"spaced inner attribute is read as one", lines("# ! [allow(dead_code)]", "verus! {", "#[verifier::external_body]", "fn f() {}", "}"), "external_body f"},
		{"spaced cfg(test) mod is skipped", lines("# [cfg(test)]", "mod t {", "    #[verifier::external_body]", "    fn g() {}", "}", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		{"spaced cfg(test) on its own", lines("# [cfg (test)]", "fn t() {", "    #[verifier::external_body]", "    fn g() {}", "}", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
	} {
		if got := names(scan(t, c.src)); got != c.want {
			t.Errorf("%s: got %q, want %q", c.what, got, c.want)
		}
	}
	// An unsupported-attribute item's contract keeps its attribute lines
	// as written.
	es := scan(t, lines("verus! {", "# [verifier::assume_unsupported_attributes]", "# [unsafe(no_mangle)]", "pub extern \"C\" fn x() {}", "}"))
	if len(es) != 1 || es[0].kind != "unsupported-attribute" || es[0].contract != "# [verifier::assume_unsupported_attributes]\n# [unsafe(no_mangle)]\npub extern \"C\" fn x() {}" {
		t.Errorf("unsupported attribute: %+v", es)
	}
}

// A trusting attribute in the spaced form on an item the scan cannot
// read fails as `#[` does, and a line that ends in a `#` (or `#!`),
// alone or after other code, its `[` on a later line, fails: the scan
// does not join them.
func TestSpacedAttrFails(t *testing.T) {
	const ends = "a line that ends in an attribute's `#`"
	for _, c := range []struct{ what, src, want string }{
		{"spaced on a lowercase const", lines("# [verifier::external_body]", "const max_u8: u8 = 255;"), "seams: src/a/b.rs:2: external_body on an item"},
		{"lone #", lines("verus! {", "#", "[verifier::external_body]", "fn f() {}", "}"), "seams: src/a/b.rs:2: " + ends},
		{"lone # with blanks", lines("  #  ", "[verifier::external_body]", "fn f() {}"), "seams: src/a/b.rs:1: " + ends},
		{"lone #!", lines("#!", "[allow(dead_code)]"), "seams: src/a/b.rs:1: " + ends},
		{"lone # !", lines("# !", "[allow(dead_code)]"), "seams: src/a/b.rs:1: " + ends},
		{"# after an attribute", lines("verus! {", "#[allow(dead_code)] #", "[verifier::external_body]", "fn f() {}", "}"), "seams: src/a/b.rs:2: " + ends},
		{"#! after an attribute", lines("#[allow(dead_code)] #!", "[allow(unused)]"), "seams: src/a/b.rs:1: " + ends},
		{"# after a wrapped attribute closes", lines("verus! {", "#[allow(", "    dead_code)] #", "[verifier::external_body]", "fn f() {}", "}"), "seams: src/a/b.rs:3: " + ends},
		{"# after code", lines("verus! {", "struct S; #", "[verifier::external_body]", "fn f() {}", "}"), "seams: src/a/b.rs:2: " + ends},
	} {
		res := generate(testRules(t), input{sources: []namedText{{"src/a/b.rs", c.src}}}, true)
		if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), c.want) {
			t.Errorf("%s: code %d, errs %v; want %q", c.what, res.code, res.errs, c.want)
		}
	}
	// A `#` inside an attribute that spans lines is part of it, and a raw
	// string's closing `"#` (or `"##`) is no attribute's `#`.
	if got := names(scan(t, lines("verus! {", "#[doc(", "#", ")]", "#[verifier::external_body]", "fn f() {}", "}"))); got != "external_body f" {
		t.Errorf("# inside an open attribute: %q", got)
	}
	if got := names(scan(t, lines("const S: &str =", "    r#\"x\"#", ";", "const T: &str =", "    r##\"x\"##", ";", "#[verifier::external_body]", "fn f() {}"))); got != "external_body f" {
		t.Errorf("raw strings ending lines: %q", got)
	}
	// A spaced inner attribute does not hide a trusting attribute after
	// it on its line.
	if got := names(scan(t, lines("verus! {", "# ! [allow(dead_code)] # [verifier::external_body] fn f() {}", "}"))); got != "external_body f" {
		t.Errorf("spaced inner then trusting: %q", got)
	}
}

// An export macro's template attributes may be spaced too.
func TestSpacedPairAttr(t *testing.T) {
	def := strings.Replace(listDef, "                #[unsafe(no_mangle)]", "                # [unsafe(no_mangle)]", 1)
	use := lines("width::exports! {", "    fn create [p_create_8, p_create_16] (n: usize);", "}")
	in := input{sources: []namedText{{"src/a.rs", use}, {"src/width.rs", def}}, pairText: def}
	res := generate(listRules(t, "name"), in, true)
	want := "$(#[$attr])*\n#[verifier::assume_unsupported_attributes]\n# [unsafe(no_mangle)]\npub unsafe extern \"C\" fn p_create_8 (exports!)"
	if res.code != 0 || len(res.entries) != 2 || res.entries[0].contract != want {
		t.Errorf("code %d %v %+v", res.code, res.errs, res.entries)
	}
}
