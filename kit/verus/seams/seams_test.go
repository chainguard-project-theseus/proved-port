package main

import (
	"bytes"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// testRules is challoc's rules file (testdata/challoc.rules), which
// reproduces its seams.sh at deda0db.
func testRules(t *testing.T) *Rules {
	t.Helper()
	r, err := readRules("testdata/challoc.rules")
	if err != nil {
		t.Fatal(err)
	}
	return r
}

func lines(xs ...string) string { return strings.Join(xs, "\n") + "\n" }

// hs is seams.sh's self-test helper: the drift lines for seam name
// (default f) of src as src/t.rs, with an overrides line giving it
// panic/abort, tool (default Kani), status and hashes.
func hs(t *testing.T, status, hashes, name, tool, src string) string {
	t.Helper()
	if name == "" {
		name = "f"
	}
	if tool == "" {
		tool = "Kani"
	}
	ov := name + "\tpanic/abort\t" + tool + "\t" + status
	if hashes != "" {
		ov += "\t" + hashes
	}
	in := input{
		overrides: []namedText{{"verus/ledger/src/t.rs.overrides", ov + "\n"}},
		sources:   []namedText{{"src/t.rs", src}},
	}
	res := generate(testRules(t), in, true)
	if res.code != 0 {
		return "scan failed"
	}
	return strings.Join(res.drift, "\n")
}

// hsrc: seam f, requiring req and returning body.
func hsrc(req, body string) string {
	return lines("verus! {", "#[verifier::external_body]", "fn f(x: u8) -> u8", "    requires "+req+",", "{", "    "+body, "}", "}")
}

// hty: an external type specification of F, with attr above it.
func hty(attr string) string {
	return lines("verus! {", "#[verifier::external_type_specification]", attr, "pub struct ExF(F);", "}")
}

func want(t *testing.T, what, got, sub string) {
	t.Helper()
	if !strings.Contains(got, sub) {
		t.Errorf("%s: want %q in:\n%s", what, sub, got)
	}
}

func wantNone(t *testing.T, what, got string) {
	t.Helper()
	if got != "" {
		t.Errorf("%s: want no drift, got:\n%s", what, got)
	}
}

var reOffered = regexp.MustCompile(`fifth column: (.*)$`)

func offered(t *testing.T, got, form string) string {
	t.Helper()
	m := reOffered.FindStringSubmatch(got)
	if m == nil || !regexp.MustCompile(form).MatchString(m[1]) {
		t.Fatalf("no hashes offered in the form %s:\n%s", form, got)
	}
	return m[1]
}

// The hash self-tests of seams.sh --check.
func TestHashes(t *testing.T) {
	dis := "discharged (Kani: k::f)"
	r := hs(t, dis, "", "", "", hsrc("x < 3", "x + 1"))
	want(t, "no hashes", r, "records no hashes")
	h := offered(t, r, `^contract [0-9]+, body [0-9]+$`)
	wantNone(t, "own hashes", hs(t, dis, h, "", "", hsrc("x < 3", "x + 1")))
	moved := lines("", "// moved down", "", "verus! {", "/// f.", "#[verifier::external_body]",
		"fn f(x: u8)  ->  u8 // a comment", "        requires x < 3, // and one", "{", "    // and one", "      x + 1", "}", "}")
	wantNone(t, "moved, reindented, recommented", hs(t, dis, h, "", "", moved))
	want(t, "edited requires", hs(t, dis, h, "", "", hsrc("x < 4", "x + 1")), "its contract changed")
	attr := strings.Replace(hsrc("x < 3", "x + 1"), "#[verifier::external_body]\n", "#[cfg(unix)] #[verifier::external_body]\n", 1)
	want(t, "added attribute", hs(t, dis, h, "", "", attr), "its contract changed")
	want(t, "edited body", hs(t, dis, h, "", "", hsrc("x < 3", "x + 2")), "its body changed")
	both := hs(t, dis, h, "", "", hsrc("x < 4", "x + 2"))
	want(t, "edited both", both, "its contract and body changed")
	want(t, "partly checked", hs(t, "trusted (partly checked, Kani: k::f; not checked: x)", "", "", "", hsrc("x < 3", "x + 1")), "records no hashes")
	want(t, "trusted with hashes", hs(t, "trusted", h, "", "", hsrc("x < 3", "x + 1")), "hashes on a status that cites no check")
	want(t, "discharged citing nothing", hs(t, "discharged (reviewed by hand)", h, "", "", hsrc("x < 3", "x + 1")), "seams: verus/ledger/src/t.rs.overrides: f: the status cites no check")
	want(t, "partly checked citing nothing", hs(t, "trusted (partly checked by reading; not checked: x)", h, "", "", hsrc("x < 3", "x + 1")), "the status cites no check")
	// A tag cites only as `TAG: `: naming the tool is no citation.
	want(t, "discharged naming a tag", hs(t, "discharged (Kani)", h, "", "", hsrc("x < 3", "x + 1")), "seams: verus/ledger/src/t.rs.overrides: f: the status cites no check")
	wantNone(t, "discharged citing a tag", hs(t, dis, h, "", "", hsrc("x < 3", "x + 1")))

	// Whitespace inside a literal is kept: "a  b".len() is 4 and
	// "a b".len() 3, so the edit changes the body hash. Whitespace
	// around the literal still does not count.
	r = hs(t, dis, "", "", "", hsrc("x < 3", `"a  b".len() as u8`))
	hl := offered(t, r, `^contract [0-9]+, body [0-9]+$`)
	wantNone(t, "literal, own hashes", hs(t, dis, hl, "", "", hsrc("x < 3", `"a  b".len()   as  u8`)))
	want(t, "literal spacing edited", hs(t, dis, hl, "", "", hsrc("x < 3", `"a b".len() as u8`)), "its body changed")
	want(t, "tab in literal", hs(t, dis, hl, "", "", hsrc("x < 3", "\"a\tb\".len() as u8")), "its body changed")
	r = hs(t, dis, "", "", "", hsrc("x < 3", `if c == '\t' { "a\"  b" } else { "" }.len() as u8`))
	he := offered(t, r, `^contract [0-9]+, body [0-9]+$`)
	want(t, "spacing after an escaped quote", hs(t, dis, he, "", "", hsrc("x < 3", `if c == '\t' { "a\" b" } else { "" }.len() as u8`)), "its body changed")
	r = hs(t, dis, "", "", "", hsrc("x < 3", `r#"a "  b"#.len() as u8 /* "  */`))
	hr := offered(t, r, `^contract [0-9]+, body [0-9]+$`)
	want(t, "raw string spacing edited", hs(t, dis, hr, "", "", hsrc("x < 3", `r#"a " b"#.len() as u8 /* "  */`)), "its body changed")
	// A block comment spanning lines, a quote in it: its text is out of
	// the hash, so the literal after it keeps its spacing rule.
	r = hs(t, dis, "", "", "", hsrc("x < 3", "/* see \"the\n   docs\" */ \"a  b\".len() as u8"))
	hm := offered(t, r, `^contract [0-9]+, body [0-9]+$`)
	want(t, "literal after a multi-line comment", hs(t, dis, hm, "", "", hsrc("x < 3", "/* see \"the\n   docs\" */ \"a b\".len() as u8")), "its body changed")
	// A comment in the signature before a body on the same line: the
	// split between contract and body stays at the brace.
	sig := func(lit string) string {
		return lines("verus! {", "#[verifier::external_body]", "fn f(/*c*/x: u8) -> usize { "+lit+".len() }", "}")
	}
	r = hs(t, dis, "", "", "", sig(`"a  b"`))
	hg := offered(t, r, `^contract [0-9]+, body [0-9]+$`)
	want(t, "literal after a signature comment", hs(t, dis, hg, "", "", sig(`"a b"`)), "its body changed")
	wantNone(t, "signature comment edited", hs(t, dis, hg, "", "", strings.Replace(sig(`"a  b"`), "/*c*/", "/* other */", 1)))
	// A comment between two tokens keeps them apart: a/*c*/b is not ab.
	r = hs(t, dis, "", "", "", hsrc("x < a/*c*/b", "return/*c*/x"))
	hj := offered(t, r, `^contract [0-9]+, body [0-9]+$`)
	want(t, "tokens joined in the contract", hs(t, dis, hj, "", "", hsrc("x < ab", "return/*c*/x")), "its contract changed")
	want(t, "tokens joined in the body", hs(t, dis, hj, "", "", hsrc("x < a/*c*/b", "returnx")), "its body changed")
	wantNone(t, "comment edited or spaced", hs(t, dis, hj, "", "", hsrc("x < a /* d */ b", "return /*e*/ x")))
	wantNone(t, "comment text edited", hs(t, dis, hm, "", "", hsrc("x < 3", "/* see 'a\n   b\" */ \"a  b\".len() as u8")))
	wantNone(t, "spacing in a block comment", hs(t, dis, hr, "", "", hsrc("x < 3", `r#"a "  b"#.len()  as u8 /*  " */`)))
	wantNone(t, "spacing outside, after char and escaped quote", hs(t, dis, he, "", "", hsrc("x < 3", `if c  ==  '\t' {  "a\"  b"  } else { "" }.len() as u8`)))
}

func TestNorm(t *testing.T) {
	for _, c := range [][2]string{
		{"  a \t b  ", "a b"},
		{`f("a  b",   'x', ' ',  '\n')  `, `f("a  b", 'x', ' ', '\n')`},
		{`"x\"  y"   z`, `"x\"  y" z`},
		{`'"'  "  "`, `'"' "  "`},
		{`'\''  "a  b"   '\\'  x`, `'\'' "a  b" '\\' x`},
		{`'«'  "a  b"   '\u{10FFFF}'  "c  d"`, `'«' "a  b" '\u{10FFFF}' "c  d"`},
		{`"open  `, `"open  `},
		{"", ""},
		{`r#"a "  b"#   x`, `r#"a "  b"#   x`[:11] + " x"},
		{`r"\"   "  c  "`, `r"\" "  c  "`},
		{`br"a  \"   z`, `br"a  \" z`},
		{`r##"q "# "  "##  y`, `r##"q "# "  "## y`},
		{`r#type   "a  b"`, `r#type "a  b"`},
		{`for   r"  "`, `for r"  "`},
		{`/* a "   b */   "c  d"`, `/* a " b */ "c  d"`},
		{`/* x /* "  */  " */   "e  f"`, `/* x /* " */ " */ "e  f"`},
		{`r"open  `, `r"open  `},
		// The byte right after a raw string or block comment is kept.
		{`r"a  b"x`, `r"a  b"x`},
		{`/* c */x`, `/* c */x`},
	} {
		if got := norm(c[0]); got != c[1] {
			t.Errorf("norm(%q) = %q, want %q", c[0], got, c[1])
		}
	}
}

// The claims nothing self-tests of seams.sh --check.
func TestClaimsNothing(t *testing.T) {
	st, tool := "trusted (claims nothing: an opaque type)", "none (assumption)"
	r := hs(t, st, "", "F", tool, hty("#[verifier::external_body]"))
	want(t, "no hashes", r, "claims nothing but records no hashes")
	h := offered(t, r, `^contract [0-9]+, body none$`)
	wantNone(t, "opaque type with its hashes", hs(t, st, h, "F", tool, hty("#[verifier::external_body]")))
	want(t, "tool", hs(t, st, h, "F", "Kani", hty("#[verifier::external_body]")), "proposes the tool Kani")
	only := func(what, r string) {
		t.Helper()
		if strings.Contains(r, "claims nothing, but") || strings.Contains(r, "gives no reason") || !strings.Contains(r, "claims nothing but records no hashes") {
			t.Errorf("%s: want only the hashes line, got:\n%s", what, r)
		}
	}
	only("external_body constant", hs(t, st, "", "F", tool, lines("verus! {", "#[verifier::external_body]", "pub exec const F: u8 = 1;", "}")))
	only("external_body struct", hs(t, st, "", "F", tool, lines("verus! {", "#[verifier::external_body]", "pub struct F(u8);", "}")))
	want(t, "no reason", hs(t, "trusted (claims nothing)", h, "F", "", hty("#[verifier::external_body]")), "gives no reason")
	want(t, "function", hs(t, st, "", "", "", hsrc("true", "x")), "its kind (external_body) may not")
	want(t, "not external_body", hs(t, st, "", "F", "", hty("#[allow(dead_code)]")), "not external_body")
	only("paren form external_body", hs(t, st, "", "F", tool, lines("verus! {", "#[verifier(external_body)]", "pub struct F(u8);", "}")))
	want(t, "ensures", hs(t, st, "", "F", "", lines("verus! {", "#[verifier::external_body]", "pub exec const F: u8", "    ensures F == 1,", "{ 1 }", "}")), "holds requires, ensures")
	want(t, "accept_recursive_types", hs(t, st, "", "F", "", hty("#[verifier::external_body] #[verifier::accept_recursive_types(T)]")), "holds requires, ensures")
	want(t, "cites a check", hs(t, "trusted (claims nothing: Kani: k::f)", h, "F", "", hty("#[verifier::external_body]")), "cites a check")
	want(t, "tracked", hs(t, st, "", "F", tool, lines("verus! {", "#[verifier::external_body]", "pub tracked struct F { _no_copy: () }", "}")), "the item is a tracked type")
	want(t, "unsafe impl Sync", hs(t, st, "", "F", tool, hty("#[verifier::external_body]")+"unsafe impl Sync for F {}\n"), "the type F has src/t.rs:6 (unsafe impl Sync)")
	want(t, "generic unsafe impl Send", hs(t, st, "", "F", tool, hty("#[verifier::external_body]")+"unsafe impl<'a> ::core::marker::Send for crate::m::F<'a> {}\n"), "the type F has src/t.rs:6 (unsafe impl Send)")
	want(t, "spec struct unsafe impl", hs(t, st, "", "F", tool, hty("#[verifier::external_body]")+"unsafe impl Sync for ExF {}\n"), "the type ExF has src/t.rs:6 (unsafe impl Sync)")
	only("other type, comment, derive", hs(t, st, "", "F", tool, lines("verus! {", "#[derive(Clone, Copy)]", "#[verifier::external_body]", "pub struct F(u8);", "}", "unsafe impl Send for FF {}", "// unsafe impl Send for F {}")))
}

// An unsafe impl in a test file counts; the test file has no seams.
func TestUnsafeImplInTestFile(t *testing.T) {
	in := input{
		overrides: []namedText{{"verus/ledger/src/t.rs.overrides", "F\tcore library\tnone (assumption)\ttrusted (claims nothing: opaque)\n"}},
		sources:   []namedText{{"src/t.rs", hty("#[verifier::external_body]")}},
		tests:     []namedText{{"src/tests.rs", lines("// x", "unsafe impl Send for F {}", "#[verifier::external_body]", "fn g() {}")}},
	}
	res := generate(testRules(t), in, true)
	if res.code != 0 || len(res.entries) != 1 {
		t.Fatalf("code %d, %d entries", res.code, len(res.entries))
	}
	want(t, "test file impl", strings.Join(res.drift, "\n"), "the type F has src/tests.rs:2 (unsafe impl Send)")
}

// scan gives the entries of one source file src/a/b.rs, or fails; a
// seam challoc's heuristics leave without a category is core library.
func scan(t *testing.T, src string) []*entry {
	t.Helper()
	r := testRules(t)
	r.Guesses = append(r.Guesses, Guess{Category: "core library", Conds: []Cond{{Field: "kind", Re: regexp.MustCompile("")}}})
	res := generate(r, input{sources: []namedText{{"src/a/b.rs", src}}}, true)
	if res.code != 0 {
		t.Fatalf("scan: code %d: %v", res.code, res.errs)
	}
	return res.entries
}

func names(es []*entry) string {
	var b []string
	for _, e := range es {
		b = append(b, e.kind+" "+e.id)
	}
	return strings.Join(b, "; ")
}

func TestScanner(t *testing.T) {
	cases := []struct{ what, src, want string }{
		{"multi-line attribute", lines("verus! {", "#[allow(", "  dead_code)]", "#[verifier::external_body]", "fn f() {}", "}"), "external_body f"},
		{"attribute path split over lines", lines("verus! {", "#[verifier::", "  external_body]", "fn f() {}", "}"), "external_body f"},
		{"attribute path with spaces", lines("verus! {", "#[verifier :: external_body]", "fn f() {}", "#[verifier( external_body )]", "fn g() {}", "}"), "external_body f; external_body g"},
		{"item on the attribute's line", lines("verus! {", "#[verifier::external_body] broadcast proof fn lemma_p(x: int) ensures f(x) > 0 {}", "}"), "external_body proof lemma_p"},
		{"item after two attributes on one line", lines("verus! {", "#[allow(dead_code)] #[verifier::external_body] fn f() {}", "fn g() {", "}", "}"), "external_body f"},
		{"cfg(test) mod declaration on one line", lines("verus! {", "#[cfg(test)] mod tests;", "pub axiom fn a();", "}"), "axiom a"},
		{"cfg(test) mod with its brace on the next line", lines("#[cfg(test)]", "mod t", "{", "    #[verifier::external_body]", "    fn g() {}", "}", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		{"cfg(test) impl with a where line", lines("#[cfg(test)]", "#[allow(dead_code)]", "impl<T> X for T", "where", "    T: Y,", "{", "    #[verifier::external_body]", "    fn g() {}", "}", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		{"cfg(test) fn whose first line ends in a comma", lines("#[cfg(test)]", "fn t(x: u8,", "     y: u8) {", "    #[verifier::external_body]", "    fn g() {}", "}", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		{"cfg(test) impl whose where clause ends in a comma", lines("#[cfg(test)]", "impl<T> X for T where T: Y,", "{", "    #[verifier::external_body]", "    fn g() {}", "}", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		{"cfg(test) last variant and field without a comma", lines("enum E {", "    A,", "    #[cfg(test)]", "    B", "}", "struct S {", "    a: u8,", "    #[cfg(test)]", "    b: u8 }", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		{"cfg(test) parameter with no comma", lines("verus! {", "fn p(a: u8,", "    #[cfg(test)]", "    b: u8", ") {", "    let t = Tracked::assume_new();", "}", "}"), "assume_new p"},
		{"cfg(test) match arm over lines", lines("verus! {", "fn q(x: E) {", "    match x {", "        #[cfg(test)]", "        A => f(", "            1),", "        B => {", "            let t = Tracked::assume_new();", "        }", "    }", "}", "}"), "assume_new q"},
		{"cfg(test) struct with its brace on the next line", lines("#[cfg(test)]", "pub(crate) struct S", "{", "    a: u8,", "}", "#[cfg(test)]", "unsafe trait T", "{", "    #[verifier::external_body]", "    fn g() {}", "}", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		// Each item head waits for its body; the fn inside stands for
		// anything the body holds that the scan would count.
		{"cfg(test) enum, union, auto trait and macro_rules! heads", lines("#[cfg(test)]", "enum E", "{", "    #[verifier::external_body] fn g1() {}", "}",
			"#[cfg(test)]", "union U", "{", "    #[verifier::external_body] fn g2() {}", "}",
			"#[cfg(test)]", "auto trait A", "{", "    #[verifier::external_body] fn g3() {}", "}",
			"#[cfg(test)]", "macro_rules! m", "{", "    () => {", "        #[verifier::external_body]", "        fn g4() {}", "    };", "}",
			"#[verifier::external_body]", "fn f() {}"), "external_body f"},
		{"cfg(test) field and variant", lines("struct S {", "    #[cfg(test)]", "    a: u8,", "    b: u8,", "}", "enum E {", "    #[cfg(test)]", "    A,", "    B,", "}", "#[verifier::external_body]", "fn f() {", "}"), "external_body f"},
		{"cfg(test) split over lines", lines("#[cfg(", "  test)]", "fn t() {", "    #[verifier::external_body]", "    fn g() {}", "}", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		{"non-ASCII char literal", lines("verus! {", "#[verifier::external_body]", "fn open(c: char) -> bool { matches!(c, '«'|'{') }", "pub axiom fn a();", `fn q(c: char) -> bool { matches!(c, 'é'|'"') }`, "pub axiom fn b();", "}"), "external_body open; axiom a; axiom b"},
		// Two literals a misread would each leave a false string after:
		// the second would close the first's, so the depth and the
		// lexer would end clean and only the seams between would go.
		{"unicode escapes with underscores, twice", lines("verus! {", "#[verifier::external_body]", `fn m(c: char) -> bool { matches!(c, '\u{1_F600}'|'"') }`, "pub axiom fn a();", "#[verifier::external_body]", "fn g() {}", `fn n(c: char) -> bool { matches!(c, '\u{1_F600}'|'"') }`, "pub axiom fn b();", "}"), "external_body m; axiom a; external_body g; axiom b"},
		{"6-digit unicode escape", lines("verus! {", "#[verifier::external_body]", `fn m(c: char) -> bool { matches!(c, '\u{10FFFF}'|'{') }`, "pub axiom fn a();", "}"), "external_body m; axiom a"},
		{"escaped apostrophe char", lines("verus! {", `fn m(c: char) -> bool { matches!(c, '\''|'"') }`, "#[verifier::external_body]", "fn f() {}", "pub axiom fn a();", "}"), "external_body f; axiom a"},
		{"cfg(test) item skipped", lines("verus! {", "#[cfg(test)]", "mod t {", "#[verifier::external_body]", "fn g() {}", "}", "#[verifier::external_body]", "fn f() {}", "}"), "external_body f"},
		{"cfg(test) multi-line fn signature", lines("#[cfg(test)]", "fn t(", "    x: u8,", ") {", "    #[verifier::external_body]", "    fn g() {}", "}", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		{"cfg(all(test, ...))", lines("#[cfg(all(test, unix))]", "#[verifier::external_body]", "fn g() {", "}", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		// seams.sh read a test fn whose body opens and closes on its first
		// line as a multi-line signature and skipped on to the next line
		// that opens a brace, dropping f and h; here the item ends there.
		{"cfg(test) one-line fn", lines("#[cfg(test)]", "fn g() {}", "#[verifier::external_body]", "fn f() {}", "#[verifier::external_body]", "fn h() {", "}", "#[verifier::external_body]", "fn k() {}"), "external_body f; external_body h; external_body k"},
		{"cfg(test) one-line axiom after it", lines("verus! {", "#[cfg(test)]", "fn g() { }", "pub axiom fn a1();", "proof fn axiom_x();", "}"), "axiom a1; axiom axiom_x"},
		{"cfg(test) multi-line signature, one-line body", lines("#[cfg(test)]", "fn t(", "    x: u8,", ") -> u8 { x }", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		{"raw string in a one-line body", lines("verus! {", "#[verifier::external_body]", `fn sep() -> &'static str { r"\" }`, "pub axiom fn a();", "#[verifier::external_body]", `fn brace() -> &'static str { r#"}"# }`, "pub axiom fn b();", "}"), "external_body sep; axiom a; external_body brace; axiom b"},
		{"quote in a block comment", lines("verus! {", "#[verifier::external_body]", `fn f() -> u8 { 1 /* " */ }`, "pub axiom fn a();", "}"), "external_body f; axiom a"},
		{"// in a block comment", lines("verus! {", "#[verifier::external_body]", "fn f() -> u8 { 1", "/* see https://x */ }", "pub axiom fn a();", "}"), "external_body f; axiom a"},
		{"brace in a multi-line comment of a test mod", lines("#[cfg(test)]", "mod t {", "    /* a {", "    */", "}", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		{"items in a multi-line comment", lines("/*", "#[verifier::external_body]", "fn g() {}", "*/", "#[verifier::external_body]", "fn f() {}"), "external_body f"},
		{"brace in a multi-line string", lines("verus! {", "#[verifier::external_body]", `fn f() -> &'static str { "a {`, `b" }`, "pub axiom fn a();", "}"), "external_body f; axiom a"},
		{"raw C string", lines("verus! {", "#[verifier::external_body]", `fn f() -> &'static core::ffi::CStr { cr"\" }`, "pub axiom fn a();", "}"), "external_body f; axiom a"},
		{"paren form external_body", lines("verus! {", "#[verifier(external_body)]", "fn f() {}", "#[verifier(external_body)]", "proof fn p() {}", "#[verifier(external_body)]", "pub struct S(u8);", "#[verifier(external_body)]", "pub const K: u8 = 1;", "}"), "external_body f; external_body proof p; external_body type S; external_body const K"},
		{"impl qualification", lines("impl<T: Copy> Foo<T> {", "    #[verifier::external_body]", "    fn f(&self) {}", "}", "#[verifier::external_body]", "fn g() {}"), "external_body Foo::f; external_body g"},
		{"impl on two lines", lines("impl Bar", "{", "    #[verifier::external_body]", "    fn f() {}", "}"), "external_body Bar::f"},
		{"trait impl", lines("impl core::fmt::Debug for crate::m::Baz {", "    #[verifier::external_body]", "    fn fmt(&self) {}", "}"), "external_body Baz::fmt"},
		{"assume_new inside verus!", lines("verus! {", "fn f() {", "    let t = Tracked::assume_new();", "    let g = Ghost::assume_new();", "}", "}"), "assume_new f#1; ghost_assume_new f#2"},
		{"assume_new outside verus!", lines("fn f() {", "    let t = Tracked::assume_new();", "}"), ""},
		{"assume_new in a test", lines("verus! {", "#[cfg(test)]", "fn t() {", "    let t = Tracked::assume_new();", "}", "}"), ""},
		{"#N ids", lines("verus! {", "#[verifier::external_body]", "fn f() {}", "mod m {", "#[verifier::external_body]", "fn f() {}", "}", "}"), "external_body f#1; external_body f#2"},
		{"axioms", lines("verus! {", "pub axiom fn a1();", "proof fn axiom_lock();", "#[verifier::external_body]", "proof fn p() {}", "}"), "axiom a1; axiom axiom_lock; external_body proof p"},
		{"external_body type and const", lines("#[verifier::external_body]", "pub struct S<T> { x: T }", "#[verifier::external_body]", "pub const K: u8 = 1;"), "external_body type S; external_body const K"},
		{"assume_specification", lines("pub assume_specification<T> [ core::ptr::null::<T> ] () -> *const T;", "use vstd::assume_specification;"), "assume_spec core::ptr::null::<T>"},
		{"ext specs", lines("#[verifier::external_type_specification]", "pub struct ExAtomicU8(core::sync::atomic::AtomicU8);", "#[verifier::external_trait_specification]", "pub trait ExT {", "    fn f();", "}"), "ext_type_spec core::sync::atomic::AtomicU8; ext_trait_spec ExT"},
		{"no_decreases", lines("#[verifier::exec_allows_no_decreases_clause]", "fn spin() { loop {} }"), "no_decreases spin"},
		{"external_fn_specification", lines("verus! {", "#[verifier::external_fn_specification]", "pub fn ex_foo(x: u8) -> (r: u8)", "    ensures r == x,", "{", "    foo(x)", "}", "pub axiom fn a();", "}"), "assume_spec ex_foo; axiom a"},
		{"unsupported attributes", lines("verus! {", "#[verifier::assume_unsupported_attributes]", "#[unsafe(no_mangle)]", "pub extern \"C\" fn malloc() {}", "#[verifier::assume_unsupported_attributes]", "pub static mut G: u8 = 0;", "}"), "unsupported-attribute malloc; unsupported-attribute G"},
		{"paren form unsupported attributes", lines("verus! {", "#[verifier(assume_unsupported_attributes)]", "#[unsafe(no_mangle)]", "pub extern \"C\" fn free() {}", "}"), "unsupported-attribute free"},
		{"string and comment braces", lines("verus! {", "#[verifier::external_body]", "fn f() { let s = \"}{\"; // }", "    let c = '{';", "}", "#[verifier::external_body]", "fn g() {}", "}"), "external_body f; external_body g"},
	}
	for _, c := range cases {
		if got := names(scan(t, c.src)); got != c.want {
			t.Errorf("%s: got %q, want %q", c.what, got, c.want)
		}
	}
}

// A trusting attribute on an item the scan cannot read is an input
// error naming the file and line, not a seam left out.
func TestUnreadItem(t *testing.T) {
	for _, c := range []struct{ what, src, want string }{
		{"macro-written fn name", lines("macro_rules! m {", "    ($name:ident) => {", "        #[verifier::external_body]", "        pub fn $name() -> u8 { 0 }", "    };", "}"), "seams: src/a/b.rs:4: external_body on an item the scan does not read as a seam: pub fn $name() -> u8 { 0 }"},
		{"lowercase static", lines("#[verifier::external_body]", "#[allow(non_upper_case_globals)]", "static page_size: usize = 4096;"), "seams: src/a/b.rs:3: external_body on an item"},
		{"lowercase-suffixed const", lines("#[verifier::external_body]", "const MAX_u8: u8 = 255;"), "seams: src/a/b.rs:2: external_body on an item"},
		{"union", lines("#[verifier::external_body]", "pub union U { a: u8 }"), "seams: src/a/b.rs:2: external_body on an item"},
		{"external_fn_specification on a non-fn", lines("#[verifier::external_fn_specification]", "const K: u8 = 1;"), "seams: src/a/b.rs:2: external_fn_specification on an item"},
		{"no_decreases on a non-fn", lines("#[verifier::exec_allows_no_decreases_clause]", "const K: u8 = 1;"), "seams: src/a/b.rs:2: exec_allows_no_decreases_clause on an item"},
		{"unsupported attributes on a non-fn", lines("#[verifier::assume_unsupported_attributes]", "pub union U { a: u8 }"), "seams: src/a/b.rs:2: verifier::assume_unsupported_attributes on an item"},
	} {
		r := testRules(t)
		res := generate(r, input{sources: []namedText{{"src/a/b.rs", c.src}}}, true)
		if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), c.want) {
			t.Errorf("%s: code %d, errs %v; want %q", c.what, res.code, res.errs, c.want)
		}
	}
	// A macro whose name only starts with the pair macro's is not exempt.
	r := testRules(t)
	ext := lines("macro_rules! verified_export_pair_extra {", "    ($name:ident) => {", "        #[verifier::assume_unsupported_attributes]", "        pub extern \"C\" fn $name() {}", "    };", "}")
	if res := generate(r, input{sources: []namedText{{"src/a/b.rs", ext}}}, true); res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "src/a/b.rs:4: verifier::assume_unsupported_attributes on an item") {
		t.Errorf("pair macro prefix: code %d, errs %v", res.code, res.errs)
	}
	uni := lines("macro_rules! verified_export_pairé {", "    ($name:ident) => {", "        #[verifier::assume_unsupported_attributes]", "        pub extern \"C\" fn $name() {}", "    };", "}")
	if res := generate(r, input{sources: []namedText{{"src/a/b.rs", uni}}}, true); res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "src/a/b.rs:4: verifier::assume_unsupported_attributes on an item") {
		t.Errorf("pair macro with a non-ASCII suffix: code %d, errs %v", res.code, res.errs)
	}
	for _, def := range []string{"macro_rules! verified_export_pair{", "macro_rules! verified_export_pair ("} {
		if !macroDef(def, "verified_export_pair") {
			t.Errorf("macroDef(%q) = false", def)
		}
	}
	// Inside the pair macro's own definition the export attributes are
	// read at each invocation, so they are no unread item there.
	src := lines("macro_rules! verified_export_pair {", "    ($name:ident) => {", "        #[verifier::assume_unsupported_attributes]", "        pub extern \"C\" fn $name() {}", "    };", "}")
	if res := generate(r, input{sources: []namedText{{"src/api/mod.rs", src}}}, true); res.code != 0 {
		t.Errorf("pair macro definition: code %d, errs %v", res.code, res.errs)
	}
}

// A file the scan ends inside something in fails, naming where it began.
func TestEndOfFile(t *testing.T) {
	for _, c := range []struct{ what, src, want string }{
		{"open string", lines("fn f() {", `    let s = "abc;`, "}"), "seams: src/a/b.rs:2: the file ends inside a string that starts here"},
		{"open raw string", lines(`const S: &str = r#"abc`, "}"), "seams: src/a/b.rs:1: the file ends inside a raw string"},
		{"open block comment", lines("fn f() {}", "/* a", "b"), "seams: src/a/b.rs:2: the file ends inside a block comment"},
		{"open attribute", lines("fn f() {}", "#[cfg(", "  unix"), "seams: src/a/b.rs:2: the file ends inside an attribute"},
		{"open item", lines("#[verifier::external_body]", "fn f() {", "    x"), "seams: src/a/b.rs:2: the file ends inside an item"},
		{"open test item", lines("#[cfg(test)]", "mod t {", "    x"), "seams: src/a/b.rs:2: the file ends inside a test-only item"},
		{"pending test attribute", lines("fn f() {}", "#[cfg(test)]"), "seams: src/a/b.rs:2: the file ends inside a test-only item"},
		{"unbalanced braces", lines("fn f() {", "    x"), "seams: src/a/b.rs: the file's braces do not balance (depth 1 at its end)"},
	} {
		res := generate(testRules(t), input{sources: []namedText{{"src/a/b.rs", c.src}}}, true)
		if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), c.want) {
			t.Errorf("%s: code %d, errs %v; want %q", c.what, res.code, res.errs, c.want)
		}
	}
	// A test file is read for `unsafe impl` alone: only its literals and
	// comments count.
	res := generate(testRules(t), input{sources: []namedText{{"src/a/b.rs", lines("fn f() {}")}}, tests: []namedText{{"src/a/tests.rs", lines("fn t() {", `    let s = "x;`)}}}, true)
	if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/a/tests.rs:2: the file ends inside a string") {
		t.Errorf("test file: code %d, errs %v", res.code, res.errs)
	}
	res = generate(testRules(t), input{sources: []namedText{{"src/a/b.rs", lines("fn f() {}")}}, tests: []namedText{{"src/a/tests.rs", lines("fn t() {")}}}, true)
	if res.code == 2 {
		t.Errorf("test file with open braces: %v", res.errs)
	}
	// A file that ends inside a string has its braces open too: the one
	// error is the string's, not a second about the braces.
	res = generate(testRules(t), input{sources: []namedText{{"src/a.rs", lines("fn f() {", `    let s = "x;`)}}}, true)
	if strings.Join(res.errs, "\n") != "seams: src/a.rs:2: the file ends inside a string that starts here, so the scan cannot read it whole" {
		t.Errorf("string and braces open: errs %q", res.errs)
	}
	// Each file starts with the lexer fresh: a file left inside a block
	// comment is its own error, and the next file is read as code.
	res = generate(testRules(t), input{
		sources: []namedText{{"src/a.rs", lines("/* open")}, {"src/b.rs", lines("fn g() {}")}},
		tests:   []namedText{{"src/tests.rs", lines("/* open")}, {"src/x/tests.rs", lines("fn t() {}")}}}, true)
	if strings.Join(res.errs, "\n") != "seams: src/a.rs:1: the file ends inside a block comment that starts here, so the scan cannot read it whole\nseams: src/tests.rs:1: the file ends inside a block comment that starts here, so the scan cannot read it whole" {
		t.Errorf("lexer per file: code %d, errs %q", res.code, res.errs)
	}
}

func TestMintsNote(t *testing.T) {
	es := scan(t, lines("verus! {", "#[verifier::external_body]", "fn f() {", "    let t = Tracked::assume_new();", "    let g = Ghost::assume_new();", "}", "}"))
	if len(es) != 1 || es[0].note != "mints Tracked at line 4; mints Ghost at line 5; " {
		t.Fatalf("got %q", names(es))
	}
	res := generate(testRules(t), input{sources: []namedText{{"src/a/b.rs", lines("verus! {", "#[verifier::external_body]", "fn f() {", "    let t = Tracked::assume_new();", "    let p = ptr::null();", "}", "}")}}}, true)
	want(t, "note", res.ledger["src/a/b.rs.md"], "\nBody mints Tracked at line 4.\n")
	want(t, "body guess", res.ledger["src/a/b.rs.md"], "Category: raw pointer (internal).\n")
}

// A block comment in an unsupported-attribute export's line, as the
// ledger's contract shows it: strip writes one blank for each run of the
// comment's non-blank bytes (`/*`, `c`, `*/`) and keeps the blanks
// between them, so `(/* c */)` reads `(     )` (norm folds them for the
// hash).
func TestStripCommentBlank(t *testing.T) {
	res := generate(testRules(t), input{sources: []namedText{{"src/a/b.rs", lines("verus! {", "#[verifier::assume_unsupported_attributes]", "pub extern \"C\" fn f(/* c */) {}", "}")}}}, true)
	want(t, "contract", res.ledger["src/a/b.rs.md"], "pub extern \"C\" fn f(     ) {}\n")
}

func TestFmtSpecImpl(t *testing.T) {
	src := lines("verus! {",
		"impl vstd::std_specs::fmt::DisplaySpecImpl for crate::m::T {", "    fn fmt_req() {}", "}",
		"impl fmt::Display for T {", "    fn fmt(&self) {}", "}",
		"impl vstd::std_specs::fmt::DebugSpecImpl for U {", "    fn fmt_req() {}", "}",
		"impl vstd::std_specs::cmp::PartialEqSpecImpl for T {", "    fn eq_req() {}", "}",
		"}")
	if got := names(scan(t, src)); got != "spec_impl DebugSpecImpl for U; spec_impl PartialEqSpecImpl for T" {
		t.Fatalf("got %q", got)
	}
	// Only a trait impl inside verus! (Verus checks it) drops the spec
	// impl: one outside verus! leaves it a seam.
	src = lines("verus! {",
		"impl vstd::std_specs::fmt::DisplaySpecImpl for T {", "    fn fmt_req() {}", "}",
		"}",
		"impl fmt::Display for T {", "    fn fmt(&self) {}", "}")
	if got := names(scan(t, src)); got != "spec_impl DisplaySpecImpl for T" {
		t.Fatalf("trait impl outside verus!: got %q", got)
	}
}

// A clause line's braces are the clause's (a struct literal), not the
// body's: the contract runs on to the body's own brace.
func TestClauseBraces(t *testing.T) {
	es := scan(t, lines("verus! {", "#[verifier::external_body]", "fn f() -> (r: S)", "    ensures r == S { a: 1 },", "{", "    S { a: 1 }", "}", "}"))
	if len(es) != 1 || es[0].contract != "fn f() -> (r: S)\n    ensures r == S { a: 1 }," {
		t.Fatalf("got %+v", es)
	}
}

// After the pair macro's invocation closes, a fn one level in (in a
// module) is no line of names; after its definition closes, an
// assume_unsupported_attributes item the scan cannot read fails again.
func TestPairEnds(t *testing.T) {
	def := lines("macro_rules! verified_export_pair {", "    ($name:ident / $je:ident) => {",
		"        #[verifier::assume_unsupported_attributes]", "        pub extern \"C\" fn $name() {}",
		"        #[verifier::assume_unsupported_attributes]", "        pub extern \"C\" fn $je() {}", "    };", "}")
	src := def + lines("verified_export_pair! {", "    fn a / b() {}", "}", "mod m {", "    fn c / d() {}", "}")
	res := generate(testRules(t), input{sources: []namedText{{"src/api/mod.rs", src}}, pairText: def}, true)
	if res.code != 0 || names(res.entries) != "unsupported-attribute a; unsupported-attribute b" {
		t.Errorf("after the invocation: code %d %v: %s", res.code, res.errs, names(res.entries))
	}
	src = def + lines("#[verifier::assume_unsupported_attributes]", "pub struct S;")
	res = generate(testRules(t), input{sources: []namedText{{"src/api/mod.rs", src}}, pairText: def}, true)
	if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/api/mod.rs:10: verifier::assume_unsupported_attributes on an item the scan does not read as a seam") {
		t.Errorf("after the definition: code %d %v", res.code, res.errs)
	}
}

// challoc's own verified_export_pair! definition (src/api/mod.rs at
// deda0db, verbatim) and one invocation (src/api/print.rs:1789-1793)
// give the contracts and hashes challoc's ledger records for them.
func TestPairChalloc(t *testing.T) {
	read := func(p string) string {
		b, err := os.ReadFile(filepath.Join("testdata/challoc-pair", p))
		if err != nil {
			t.Fatal(err)
		}
		return string(b)
	}
	mod, print := read("src/api/mod.rs"), read("src/api/print.rs")
	res := generate(testRules(t), input{sources: []namedText{{"src/api/mod.rs", mod}, {"src/api/print.rs", print}}, pairText: mod}, true)
	if res.code != 0 || names(res.entries) != "unsupported-attribute malloc_stats_print; unsupported-attribute je_malloc_stats_print" {
		t.Fatalf("code %d %v: %s", res.code, res.errs, names(res.entries))
	}
	// From verus/ledger/src/api/print.rs.md at deda0db.
	for i, w := range []struct {
		name string
		hash uint32
	}{{"malloc_stats_print", 1768398676}, {"je_malloc_stats_print", 3439469082}} {
		e := res.entries[i]
		want := "$(#[$m])*\n#[unsafe(no_mangle)]\n#[verifier::assume_unsupported_attributes]\npub extern \"C\" fn " + w.name + " (verified_export_pair!)"
		if e.contract != want || e.line != 1 || cksum(norm(e.hc)) != w.hash {
			t.Errorf("%s: line %d, contract %q, hash %d; want line 1, %q, %d", w.name, e.line, e.contract, cksum(norm(e.hc)), want, w.hash)
		}
	}
}

func TestPair(t *testing.T) {
	mod := lines("macro_rules! verified_export_pair {",
		"    ($(#[$m:meta])* fn $name:ident / $je:ident ($($a:tt)*)) => {",
		"        $(#[$m])*", "        #[unsafe(no_mangle)]", "        pub extern \"C\" fn $name() {}",
		"        #[unsafe(export_name =", "            \"je\")]", "        pub extern \"C\" fn $je() {}", "    };", "}",
		"verified_export_pair! {", "    fn malloc / je_malloc(size: usize) {}", "}")
	res := generate(testRules(t), input{sources: []namedText{{"src/api/mod.rs", mod}}, pairText: mod}, true)
	if res.code != 0 || names(res.entries) != "unsupported-attribute malloc; unsupported-attribute je_malloc" {
		t.Fatalf("code %d %v: %s", res.code, res.errs, names(res.entries))
	}
	// Each export's attributes are those above its own `fn $VAR(`: an
	// earlier export's line starts the list again.
	if c := res.entries[0].contract; c != "$(#[$m])*\n#[unsafe(no_mangle)]\npub extern \"C\" fn malloc (verified_export_pair!)" {
		t.Errorf("name contract %q", c)
	}
	if c := res.entries[1].contract; c != "#[unsafe(export_name = \"je\")]\npub extern \"C\" fn je_malloc (verified_export_pair!)" {
		t.Errorf("je contract %q", c)
	}
	if res.entries[0].line != 11 {
		t.Errorf("line %d, want the invocation's 11", res.entries[0].line)
	}
	// A macro whose name only starts with the pair macro's, defined
	// first, is not read for the attributes.
	inner := lines("macro_rules! verified_export_pair_inner {", "    ($name:ident) => {", "        #[inner_attr]", "        pub extern \"C\" fn $name() {}", "    };", "}") + mod
	res = generate(testRules(t), input{sources: []namedText{{"src/api/mod.rs", inner}}, pairText: inner}, true)
	if res.code != 0 || len(res.entries) != 2 || res.entries[0].contract != "$(#[$m])*\n#[unsafe(no_mangle)]\npub extern \"C\" fn malloc (verified_export_pair!)" {
		t.Errorf("longer-named macro first: code %d %v %+v", res.code, res.errs, res.entries)
	}
	res = generate(testRules(t), input{sources: []namedText{{"src/api/mod.rs", mod}}}, true)
	if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/api/mod.rs:12: no verified_export_pair! attributes in src/api/mod.rs") {
		t.Errorf("missing macro: code %d %v", res.code, res.errs)
	}
}

func TestShort(t *testing.T) {
	for _, c := range []struct {
		x    string
		w    int
		want string
	}{
		{"crate::a::b", 80, "a::b"},
		{"core::sync::atomic::AtomicU8::load", 80, "AtomicU8::load"},
		{"core::ptr::null", 80, "ptr::null"},
		{"abcdefghijklmnopqrstuvwxyz", 16, "abcdefg...uvwxyz"},
		{"abcdefghijklmnopqrstuvwxyz", 3, "abcdefg...uvwxyz"},
		{"abcdefghijklmnopqrstuvwxyz", 17, "abcdefg...tuvwxyz"},
		{"abcdefghijklmnop", 16, "abcdefghijklmnop"},
	} {
		if got := short(c.x, c.w); got != c.want {
			t.Errorf("short(%q, %d) = %q, want %q", c.x, c.w, got, c.want)
		}
	}
}

func TestCode(t *testing.T) {
	for _, c := range []struct{ s, want string }{
		{"  // all comment", ""},
		{"a { b } // c", "a { b } "},
		{`x("a{b") // }`, `x("xxx")     `},
		{`'{' '\n' '\u{7b}' 'a`, `'x' 'xx' 'xxxxxx' 'a`},
		{`"a\"b"`, `"xxxx"`},
		{`f() { r"\" }`, `f() { r"x" }`},
		{`r#"a " // x  y"#.len() // c`, `r#"xxxxxxxxxxx"#.len()     `},
		{`br##"{"#}"## {`, `br##"xxxx"## {`},
		{`r#type { "a" }`, `r#type { "x" }`},
		{`for r"}`, `for r"x`},
		{`x = r#"open {`, `x = r#"xxxxxx`},
		{`cr"\" }`, `cr"x" }`},
		{`f() { 1 /* " */ }`, `f() { 1         }`},
		{`/* see https://x */ }`, strings.Repeat(" ", 20) + "}"},
		{`a /* b /* { */ } */ c`, `a                   c`},
		{`/// a /* b`, ``},
		{`"/* {0} */" {`, `"xxxxxxxxx" {`},
		{`matches!(c, '\''|'"') {`, `matches!(c, 'xx'|'x') {`},
		{`'\u{7b}' '\\' {`, `'xxxxxx' 'xx' {`},
		{`'«'|'{' {`, `'xx'|'x' {`},
		{`'\u{10FFFF}'|'"' {`, `'xxxxxxxxxx'|'x' {`},
		{`'\x7f' b'a' '\u{}' {`, `'xxxx' b'x' '\u{}' {`},
		{`'\u{1_F600}' '\u{_7b_}' '\u{1234567}' {`, `'xxxxxxxxxx' 'xxxxxxxx' '\u{1234567}' {`},
		// Underscores do not count toward the six digits: seven digits
		// with them are no char literal, six with them are.
		{`'\u{1_0_F_F_F_F}' '\u{1_0_F_F_F_F_F}' {`, `'xxxxxxxxxxxxxxx' '\u{1_0_F_F_F_F_F}' {`},
		// An `r` inside an identifier opens no raw string: `xr"\" }` is
		// `xr` and a plain string whose `\"` is escaped, still open.
		{`xr"\" }`, `xr"xxxx`},
		{`fn f<'a>(x: &'a u8) {`, `fn f<'a>(x: &'a u8) {`},
		// A plain string after a raw one closes at its own quote.
		{`r#"x"# "y" {`, `r#"x"# "x" {`},
	} {
		if got := code(c.s); got != c.want {
			t.Errorf("code(%q) = %q, want %q", c.s, got, c.want)
		}
	}
}

// The lexer carries block comments, strings and raw strings across lines.
func TestLexer(t *testing.T) {
	for _, c := range []struct {
		what  string
		lines []string
		want  []string
	}{
		{"block comment", []string{"a /* {", "  } \" ", " */ b {"}, []string{"a     ", "      ", "    b {"}},
		{"nested block comment", []string{"/* /*", "*/ {", "*/ }"}, []string{"     ", "    ", "   }"}},
		{"string", []string{`s = "a {`, `b } // c`, `d" {`}, []string{`s = "xxx`, `xxxxxxxx`, `x" {`}},
		{"raw string", []string{`s = r#"a "{`, `// } "# {`}, []string{`s = r#"xxxx`, `xxxxx"# {`}},
		{"comment line after a closed comment", []string{"/* a */", "// b {", "c {"}, []string{"       ", "", "c {"}},
	} {
		var lx lexer
		for i, l := range c.lines {
			if got := lx.code(l); got != c.want[i] {
				t.Errorf("%s: line %d: code(%q) = %q, want %q", c.what, i+1, l, got, c.want[i])
			}
		}
	}
}

func TestCksum(t *testing.T) {
	for _, c := range []struct {
		s    string
		want uint32
	}{{"", 4294967295}, {"a", 1220704766}} {
		if got := cksum(c.s); got != c.want {
			t.Errorf("cksum(%q) = %d, want %d", c.s, got, c.want)
		}
	}
	if _, err := exec.LookPath("cksum"); err != nil {
		t.Skip("no cksum on PATH")
	}
	for _, s := range []string{"fn f(x: u8) -> u8 requires x < 3,", strings.Repeat("x", 70000)} {
		cmd := exec.Command("cksum")
		cmd.Stdin = strings.NewReader(s)
		out, err := cmd.Output()
		if err != nil {
			t.Fatal(err)
		}
		if f := strings.Fields(string(out)); f[0] != itoa(cksum(s)) {
			t.Errorf("cksum of %d bytes: system %s, ours %d", len(s), f[0], cksum(s))
		}
	}
}

func itoa(u uint32) string {
	var b [10]byte
	i := len(b)
	for {
		i--
		b[i] = byte('0' + u%10)
		u /= 10
		if u == 0 {
			return string(b[i:])
		}
	}
}

// listRules is challoc's rules with a list-form export macro in place
// of its pair line: `fn body [a, b, c] (..)`, each name from `fn $VAR(`.
func listRules(t *testing.T, vars ...string) *Rules {
	t.Helper()
	b, err := os.ReadFile("testdata/challoc.rules")
	if err != nil {
		t.Fatal(err)
	}
	var keep []string
	for _, l := range strings.Split(string(b), "\n") {
		if !strings.HasPrefix(l, "pair\t") {
			keep = append(keep, l)
		}
	}
	text := strings.Join(keep, "\n") + "\nexport-macro\texports\tsrc/width.rs\tlist\t" + strings.Join(vars, "\t") + "\n"
	r, err := parseRules("list.rules", text)
	if err != nil {
		t.Fatal(err)
	}
	return r
}

var listDef = lines("macro_rules! exports {",
	"    ($($(#[$attr:meta])* fn $body:ident [$n8:ident, $n16:ident] ($($a:tt)*);)*) => {",
	"        $crate::width::exports!(@w $($(#[$attr])* fn $body $n8;)*);",
	"    };",
	"    (@w $($(#[$attr:meta])* fn $body:ident $name:ident;)*) => {",
	"        mod m {",
	"            #[allow(unused_imports)]",
	"            use super::*;",
	"            $(",
	"                $(#[$attr])*",
	"                // Verus takes no_mangle on trust.",
	"",
	"                #[verifier::assume_unsupported_attributes]",
	"                #[unsafe(no_mangle)]",
	"                pub unsafe extern \"C\" fn $name() {}",
	"                #[unsafe(export_name = \"x\")]",
	"                pub extern \"C\" fn $other() {}",
	"            )*",
	"        }",
	"    };",
	"}")

func TestExportList(t *testing.T) {
	use := lines("width::exports! {",
		"    /// Docs.",
		"    fn create [p_create_8, p_create_16] (n: usize);",
		"    fn free [",
		"        p_free_8,",
		"        p_free_16,",
		"    ] (",
		"        p: *mut u8,",
		"    );",
		"}")
	in := input{sources: []namedText{{"src/a.rs", use}, {"src/width.rs", listDef}}, pairText: listDef}
	res := generate(listRules(t, "name"), in, true)
	if res.code != 0 || names(res.entries) != "unsupported-attribute p_create_8; unsupported-attribute p_create_16; unsupported-attribute p_free_8; unsupported-attribute p_free_16" {
		t.Fatalf("code %d %v: %s", res.code, res.errs, names(res.entries))
	}
	// The attributes directly above `fn $name(` (the comment between
	// them skipped, the `#[allow]` above the `$(` not), and the item's
	// own qualifiers.
	want := "$(#[$attr])*\n#[verifier::assume_unsupported_attributes]\n#[unsafe(no_mangle)]\npub unsafe extern \"C\" fn p_free_16 (exports!)"
	if c := res.entries[3].contract; c != want {
		t.Errorf("contract %q, want %q", c, want)
	}
	if l := res.entries[2].line; l != 1 {
		t.Errorf("line %d, want the invocation's 1", l)
	}
	// One VAR per name: the second name takes the second template.
	res = generate(listRules(t, "name", "other"), in, true)
	if res.code != 0 || res.entries[1].contract != "#[unsafe(export_name = \"x\")]\npub extern \"C\" fn p_create_16 (exports!)" {
		t.Errorf("two VARs: code %d %v %+v", res.code, res.errs, res.entries)
	}
	// Invocations by other paths are read; the definition's own
	// `$crate::width::exports!(` is not an invocation.
	for _, open := range []string{"exports! {", "crate::width::exports!{", "::w :: exports! {", "width::exports ! {", "exports/* c */! {"} {
		u := lines(open, "    fn f [a_8, a_16] ();", "}")
		res = generate(listRules(t, "name"), input{sources: []namedText{{"src/a.rs", u}, {"src/width.rs", listDef}}, pairText: listDef}, true)
		if res.code != 0 || names(res.entries) != "unsupported-attribute a_8; unsupported-attribute a_16" {
			t.Errorf("%q: code %d %v: %s", open, res.code, res.errs, names(res.entries))
		}
	}
	// `exports != x` compares a value; it is no use of the macro.
	// Nor is an import by its own name.
	u := lines("fn g(exports: u8) -> bool { exports != 3 }", "pub(crate) use exports;", "use w::{exports, asm};", "m!(exports asx);", "m!(exportsas);")
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/a.rs", u}, {"src/width.rs", listDef}}, pairText: listDef}, true)
	if res.code != 0 || len(res.entries) != 0 {
		t.Errorf("exports !=: code %d %v: %s", res.code, res.errs, names(res.entries))
	}
	// Comments between template attributes, block ones over one line or
	// several included, drop no attribute; a comment after an attribute
	// is no part of it.
	for _, cm := range [][]string{{"/* symbol docs */"}, {"/* symbol", "   docs */"}, {"// docs"}} {
		def := []string{"macro_rules! exports {", "    ($($b:ident [$n:ident];)*) => {", "        $(", `            #[unsafe(export_name = "x")] // the symbol`}
		def = append(def, cm...)
		def = append(def, "            #[verifier::assume_unsupported_attributes]", "            pub extern \"C\" fn $name() {}", "        )*", "    };", "}")
		d := lines(def...)
		u := lines("exports! {", "    fn f [a] ();", "}")
		res = generate(listRules(t, "name"), input{sources: []namedText{{"src/a.rs", u}, {"src/width.rs", d}}, pairText: d}, true)
		want := "#[unsafe(export_name = \"x\")]\n#[verifier::assume_unsupported_attributes]\npub extern \"C\" fn a (exports!)"
		if res.code != 0 || len(res.entries) != 1 || res.entries[0].contract != want {
			t.Errorf("comment %q in the template: code %d %v %+v", cm, res.code, res.errs, res.entries)
		}
	}
	// The template is read as code: an indented definition, `fn $name(`
	// in a comment, a comment before an attribute, a bracket in a
	// literal, a comment inside a multi-line attribute.
	d := lines("mod m {",
		"    macro_rules! exports {",
		"        ($($b:ident [$n:ident];)*) => {",
		"            // the export is fn $name( below",
		"            /* sym */ #[unsafe(export_name = \"x[\")]",
		"            #[cfg(",
		"                unix // only ]",
		"            )]",
		"            pub extern \"C\" fn $name() {}",
		"        };",
		"    }",
		"}")
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/a.rs", lines("exports! {", "    fn f [a] ();", "}")}, {"src/width.rs", d}}, pairText: d}, true)
	if want := "#[unsafe(export_name = \"x[\")]\n#[cfg( unix )]\npub extern \"C\" fn a (exports!)"; res.code != 0 || len(res.entries) != 1 || res.entries[0].contract != want {
		t.Errorf("lexed template: code %d %v %+v, want %q", res.code, res.errs, res.entries, want)
	}
	// Two invocations in a file, each read alone; a `;` inside brackets
	// does not end a declaration; inner attributes are attributes.
	u = lines("exports! {", "    #![allow(unused)]", "    fn f [a, b] (x: [u8; 4]);", "}", "exports! {", "    fn g [c, d] ();", "}")
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/a.rs", u}, {"src/width.rs", listDef}}, pairText: listDef}, true)
	if res.code != 0 || names(res.entries) != "unsupported-attribute a; unsupported-attribute b; unsupported-attribute c; unsupported-attribute d" {
		t.Errorf("two invocations: code %d %v: %s", res.code, res.errs, names(res.entries))
	}
	// A macro whose name ends in this one's is not an invocation.
	u = lines("my_exports! {", "    fn f [a_8, a_16] ();", "}")
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/a.rs", u}, {"src/width.rs", listDef}}, pairText: listDef}, true)
	if res.code != 0 || len(res.entries) != 0 {
		t.Errorf("my_exports!: code %d %v: %s", res.code, res.errs, names(res.entries))
	}
}

// The shape of the second port's export macro (written for this test,
// same layout): one arm takes `fn body [name_8, name_16, name_32]`
// declarations and calls the other once per width through
// `$crate::width::exports!(@width ..)`; that arm writes a private module
// per width with the exports inside `verus!`, each under the forwarded
// attributes, a comment and the two attributes Verus takes on trust.
// Invocations are rustfmt'd: docs, the names one a line, the arguments
// one a line.
func TestExportListPortShape(t *testing.T) {
	def := lines("/// Writes the _8, _16 and _32 exports of generic entry points.",
		"macro_rules! exports {",
		"    ($(",
		"        $(#[$attr:meta])*",
		"        fn $body:ident [$n8:ident, $n16:ident, $n32:ident]",
		"            ($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty)?;",
		"    )*) => {",
		"        $crate::width::exports!(@width w8, u8,",
		"            $($(#[$attr])* fn $body $n8 ($($arg: $ty),*) $(-> $ret)?;)*);",
		"        $crate::width::exports!(@width w16, u16,",
		"            $($(#[$attr])* fn $body $n16 ($($arg: $ty),*) $(-> $ret)?;)*);",
		"        $crate::width::exports!(@width w32, u32,",
		"            $($(#[$attr])* fn $body $n32 ($($arg: $ty),*) $(-> $ret)?;)*);",
		"        pub use w8::*;",
		"        pub use w16::*;",
		"        pub use w32::*;",
		"    };",
		"    (@width $m:ident, $unit:ty, $(",
		"        $(#[$attr:meta])*",
		"        fn $body:ident $name:ident ($($arg:ident: $ty:ty),*) $(-> $ret:ty)?;",
		"    )*) => {",
		"        mod $m {",
		"            #[allow(unused_imports)]",
		"            use super::*;",
		"",
		"            verus! {",
		"",
		"            /// The code unit of this width's exports.",
		"            type U = $unit;",
		"            $(",
		"                $(#[$attr])*",
		"                // Verus checks the body but takes `no_mangle` on trust.",
		"                #[verifier::assume_unsupported_attributes]",
		"                #[unsafe(no_mangle)]",
		"                pub unsafe extern \"C\" fn $name($($arg: $ty),*) $(-> $ret)? {",
		"                    // SAFETY: the caller's contract is the body's.",
		"                    unsafe { super::$body::<U>($($arg),*) }",
		"                }",
		"            )*",
		"",
		"            } // verus!",
		"        }",
		"    };",
		"}",
		"pub(crate) use exports;")
	use := lines("use crate::width;",
		"",
		"width::exports! {",
		"    /// `int thing_size(const thing *t)`",
		"    ///",
		"    /// # Safety",
		"    ///",
		"    /// `t` is NULL or a live thing of this width.",
		"    fn thing_size [p_thing_size_8, p_thing_size_16, p_thing_size_32] (t: *const Thing<U>) -> c_int;",
		"",
		"    /// `int thing_walk(const thing *t, walk_fn cb, void *data)`",
		"    fn thing_walk [",
		"        p_thing_walk_8,",
		"        p_thing_walk_16,",
		"        p_thing_walk_32",
		"    ] (",
		"        t: *const Thing<U>,",
		"        cb: Option<WalkPtr<U>>,",
		"        data: *mut c_void,",
		"    ) -> c_int;",
		"}")
	res := generate(listRules(t, "name"), input{sources: []namedText{{"src/a.rs", use}, {"src/width.rs", def}}, pairText: def}, true)
	want := "unsupported-attribute p_thing_size_8; unsupported-attribute p_thing_size_16; unsupported-attribute p_thing_size_32; unsupported-attribute p_thing_walk_8; unsupported-attribute p_thing_walk_16; unsupported-attribute p_thing_walk_32"
	if res.code != 0 || names(res.entries) != want {
		t.Fatalf("code %d %v: %s", res.code, res.errs, names(res.entries))
	}
	for _, e := range res.entries {
		c := "$(#[$attr])*\n#[verifier::assume_unsupported_attributes]\n#[unsafe(no_mangle)]\npub unsafe extern \"C\" fn " + e.name + " (exports!)"
		if e.contract != c || e.line != 3 {
			t.Errorf("%s: line %d, contract %q", e.name, e.line, e.contract)
		}
	}
}

func TestExportFailClosed(t *testing.T) {
	for _, c := range []struct {
		name, src, want string
		vars            []string
	}{
		{"fn line not in the form", lines("exports! {", "    fn f(a: u8);", "}"),
			"src/a.rs:2: a fn line in a exports! invocation that the list form (fn f [a, b, ...] (..), its names on that line) does not read: fn f(a: u8);", nil},
		{"unsafe fn line", lines("exports! {", "    unsafe fn f [a, b] ();", "}"),
			"src/a.rs:2: a fn line in a exports! invocation that the list form", nil},
		{"names do not fit the VARs", lines("exports! {", "    fn f [a, b, c] ();", "}"),
			"src/a.rs:2: exports! names 3 exports here, but the rules give 2 template VARs", []string{"name", "other"}},
		{"missing template", lines("exports! {", "    fn f [a, b] ();", "}"),
			"src/a.rs:2: no exports! attributes in src/width.rs", []string{"nope"}},
		{"list left open", lines("exports! {", "    fn f [", "        a,", "}"),
			"src/a.rs:2: the exports! invocation ends inside the list of names that starts here", nil},
		{"names on the invocation line", lines("exports! { fn f [a, b] (); }"),
			"src/a.rs:1: a use of exports! the scan does not read as an invocation", nil},
		{"other delimiter", lines("exports!(", "    fn f [a, b] ();", ");"),
			"src/a.rs:1: a use of exports! the scan does not read as an invocation", nil},
		{"in an expression", lines("fn g() {", "    let x = exports! {", "    };", "}"),
			"src/a.rs:2: a use of exports! the scan does not read as an invocation", nil},
		{"renamed import", lines("use crate::width::exports as ex;", "ex! {", "    fn f [a, b] ();", "}"),
			"src/a.rs:1: exports renamed with `as`, so the scan cannot find its invocations", nil},
		{"renamed in a group", lines("use crate::width::{", "    exports  as ex,", "};"),
			"src/a.rs:2: exports renamed with `as`", nil},
		// A name, its `!` or `as` split over lines is still a use.
		{"block comment over lines before the !", lines("exports/* comment", " */! {", "    fn f [a, b] ();", "}"),
			"src/a.rs:1: a use of exports! the scan does not read as an invocation (write `exports! {` with the name, its `!` and the `{` on one line", nil},
		{"! on the next line", lines("width::exports", "! {", "    fn f [a, b] ();", "}"),
			"src/a.rs:1: a use of exports! the scan does not read as an invocation", nil},
		{"rename split over lines", lines("use w::{", "    exports", "    as ex,", "};", "ex! {", "    fn f [a, b] ();", "}"),
			"src/a.rs:2: exports renamed with `as`", nil},
		// A definition the rules do not name would take FILE's template.
		{"definition in another file", lines("macro_rules! exports {", "    ($($b:ident [$n:ident];)*) => {", "        #[unsafe(link_section = \".init\")]", "        #[verifier::assume_unsupported_attributes]", "        pub unsafe extern \"C\" fn $name() {}", "    };", "}", "exports! {", "    fn g [n] ();", "}"),
			"src/a.rs:1: a macro_rules! exports outside src/width.rs, the file the rules read its exports' attributes from", nil},
		{"nested invocation", lines("exports! {", "    exports! {", "    }", "}"),
			"src/a.rs:2: a exports! invocation inside another", nil},
		// What the line reader does not see, the invocation's tokens do.
		{"grammar with no fn", lines("width::exports! {", "    create [p_8, p_16] (n: usize);", "}"),
			"src/a.rs:2: the exports! invocation at line 1 does not read as list declarations: want `fn` (or `#[` or the invocation's `}`), got `create`", nil},
		{"forwarded tokens", lines("exports! {", "    $($t)*", "}"),
			"src/a.rs:2: the exports! invocation at line 1 does not read as list declarations: want `fn` (or `#[` or the invocation's `}`), got `$`", nil},
		{"fn split from its names", lines("exports! {", "    fn", "    f [a, b] ();", "}"),
			"src/a.rs:1: the exports! invocation declares a, b, but its lines read as no exports", nil},
		{"two declarations on a line", lines("exports! {", "    fn f [a, b] (); fn g [c, d] ();", "}"),
			"src/a.rs:1: the exports! invocation declares a, b, c, d, but its lines read as a, b", nil},
		{"names without a comma", lines("exports! {", "    fn f [a b] ();", "}"),
			"src/a.rs:2: the exports! invocation at line 1 does not read as list declarations: want `,` or `]` after a name, got `b`", nil},
		{"declaration with no end", lines("exports! {", "    fn f [a, b] ()", "}"),
			"src/a.rs:3: the exports! invocation at line 1 does not read as list declarations: want the declaration's end (`;` or a `{...}` body), got `}`", nil},
		{"attribute with no bracket", lines("exports! {", "    # fn f [a, b] ();", "}"),
			"src/a.rs:2: the exports! invocation at line 1 does not read as list declarations: want `[` after `#`, got `fn`", nil},
	} {
		vars := c.vars
		if vars == nil {
			vars = []string{"name"}
		}
		res := generate(listRules(t, vars...), input{sources: []namedText{{"src/a.rs", c.src}, {"src/width.rs", listDef}}, pairText: listDef}, true)
		if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: "+c.want) {
			t.Errorf("%s: code %d, errs %v", c.name, res.code, res.errs)
		}
	}
	// The definition's own first and last lines may call it.
	rec := lines("macro_rules! exports { (@x) => { exports!(@y) };",
		"    ($($b:ident [$n:ident];)*) => {",
		"        #[verifier::assume_unsupported_attributes]",
		"        pub extern \"C\" fn $name() {}",
		"    };",
		"    (@y) => { exports!(@x) }; }")
	res := generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", rec}, {"src/a.rs", lines("exports! {", "    fn f [a] ();", "}")}}, pairText: rec}, true)
	if res.code != 0 || len(res.entries) != 1 {
		t.Errorf("calls on the definition's first and last lines: code %d, errs %v", res.code, res.errs)
	}
	// Code after the definition's closing brace on its line is refused,
	// not read as part of the definition; a `;` there is fine.
	after := strings.TrimSuffix(listDef, "}\n") + lines("} exports! {", "    fn f [a, b] ();", "}")
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", after}}, pairText: after}, true)
	if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/width.rs:21: code after the closing brace of macro_rules! exports on its line") {
		t.Errorf("invocation after the definition's brace: code %d, errs %v", res.code, res.errs)
	}
	semi := strings.TrimSuffix(listDef, "}\n") + lines("};")
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", semi}}, pairText: semi}, true)
	if res.code != 0 {
		t.Errorf("`};` closing the definition: code %d, errs %v", res.code, res.errs)
	}
	// A second template for one VAR (another arm) is refused: only the
	// first one's attributes would be read.
	arms := lines("macro_rules! exports {",
		"    (@w8 $name:ident) => {",
		"        #[unsafe(link_section = \".a\")]",
		"        #[verifier::assume_unsupported_attributes]",
		"        pub extern \"C\" fn $name() {}",
		"    };",
		"    (@w16 $name:ident) => {",
		"        #[verifier::assume_unsupported_attributes]",
		"        pub extern \"C\" fn $name() {}",
		"    };",
		"}")
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", arms}}, pairText: arms}, true)
	if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/width.rs:9: a second `fn $name(` in macro_rules! exports (the first is line 5)") {
		t.Errorf("two templates: code %d, errs %v", res.code, res.errs)
	}
	// Templates are matched as tokens: a blank before the `(` (either
	// arm) is the same template, and still a second one.
	for _, pair := range [][2]string{{"fn $name()", "fn $name ()"}, {"fn $name ()", "fn $ name()"}} {
		a := strings.Replace(arms, "fn $name()", pair[0], 1)
		i := strings.LastIndex(a, "fn $name()")
		a = a[:i] + pair[1] + a[i+len("fn $name()"):]
		res = generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", a}}, pairText: a}, true)
		if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/width.rs:9: a second `fn $name(` in macro_rules! exports (the first is line 5)") {
			t.Errorf("two templates %q: code %d, errs %v", pair, res.code, res.errs)
		}
	}
	// A template head split over lines is still a template: a second one
	// so written is refused, and a lone one cannot be read.
	i := strings.LastIndex(arms, "fn $name()")
	split := arms[:i] + "fn $name\n        () {}\n    };\n}\n"
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", split}}, pairText: split}, true)
	if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/width.rs:9: a second `fn $name(` in macro_rules! exports (the first is line 5)") {
		t.Errorf("second template split over lines: code %d, errs %v", res.code, res.errs)
	}
	j := strings.Index(arms, "fn $name()")
	// (Split between `fn` and `$name`, so the line-by-line read finds
	// only the second arm's; the first is still named as the first.)
	split = arms[:j] + "fn\n        $name() {}" + arms[j+len("fn $name() {}"):]
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", split}}, pairText: split}, true)
	if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/width.rs:10: a second `fn $name(` in macro_rules! exports (the first is line 5)") {
		t.Errorf("first template split over lines: code %d, errs %v", res.code, res.errs)
	}
	// Generics after the name: a template all the same, in either arm.
	for k, which := range []func(string) int{func(s string) int { return strings.Index(s, "fn $name()") }, func(s string) int { return strings.LastIndex(s, "fn $name()") }} {
		at := which(arms)
		g := arms[:at] + "fn $name<'a>(_p: &'a u8)" + arms[at+len("fn $name()"):]
		res = generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", g}}, pairText: g}, true)
		if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/width.rs:9: a second `fn $name(` in macro_rules! exports (the first is line 5)") {
			t.Errorf("generic template in arm %d: code %d, errs %v", k+1, res.code, res.errs)
		}
	}
	gen := lines("macro_rules! exports {", "    ($($b:ident [$name:ident];)*) => {", "        $(", "        #[unsafe(no_mangle)]", "        pub extern \"C\" fn $name<'a>(_p: &'a u8) {}", "        )*", "    };", "}")
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", gen}, {"src/a.rs", lines("exports! {", "    fn f [a] ();", "}")}}, pairText: gen}, true)
	if res.code != 0 || len(res.entries) != 1 || res.entries[0].contract != "#[unsafe(no_mangle)]\npub extern \"C\" fn a (exports!)" {
		t.Errorf("generic template read: code %d, errs %v, %+v", res.code, res.errs, res.entries)
	}
	// (`fn $name` then `()` on the next line is read on the head's line.)
	lone := lines("macro_rules! exports {", "    ($name:ident) => {", "        #[verifier::assume_unsupported_attributes]", "        pub extern \"C\" fn", "            $name() {}", "    };", "}")
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", lone}}, pairText: lone}, true)
	if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/width.rs:4: the `fn $name(` of macro_rules! exports runs over lines") {
		t.Errorf("lone template split over lines: code %d, errs %v", res.code, res.errs)
	}
	// A definition on one line holds no template the search reads, and
	// the search does not run on into the next macro.
	oneLine := lines("macro_rules! exports { ($(fn $b:ident [$name:ident];)*) => { $(#[unsafe(no_mangle)] pub extern \"C\" fn $name() {})* }; }",
		"macro_rules! other {", "    ($name:ident) => {", "        #[unsafe(link_section = \".x\")]", "        pub extern \"C\" fn $name() {}", "    };", "}")
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", oneLine}, {"src/a.rs", lines("exports! {", "    fn f [a] ();", "}")}}, pairText: oneLine}, true)
	if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/a.rs:2: no exports! attributes in src/width.rs") {
		t.Errorf("one-line definition: code %d, errs %v", res.code, res.errs)
	}
	// The search ends at the definition's close: a `fn $name(` in a
	// later macro is no template.
	later := lines("macro_rules! exports {", "}",
		"macro_rules! other {", "    ($name:ident) => {", "        #[verifier::assume_unsupported_attributes]", "        pub extern \"C\" fn $name() {}", "    };", "}")
	u := lines("exports! {", "    fn f [a] ();", "}")
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", later}, {"src/a.rs", u}}, pairText: later}, true)
	if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/a.rs:2: no exports! attributes in src/width.rs") {
		t.Errorf("template after the definition: code %d, errs %v", res.code, res.errs)
	}
	// A second definition in the rules' FILE is refused too.
	two := listDef + lines("macro_rules! exports {", "    () => {};", "}")
	res = generate(listRules(t, "name"), input{sources: []namedText{{"src/width.rs", two}}, pairText: two}, true)
	if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/width.rs:22: a second macro_rules! exports in src/width.rs") {
		t.Errorf("second definition: code %d, errs %v", res.code, res.errs)
	}
	// A list form's names over lines keep the declaration's own
	// attributes, as on one line.
	one := lines("exports! {", "    #[unsafe(link_section = \".text.hot\")]", "    fn free [p_free_8, p_free_16] (p: *mut u8);", "}")
	multi := lines("exports! {", "    #[unsafe(link_section = \".text.hot\")]", "    fn free [", "        p_free_8,", "        p_free_16,", "    ] (p: *mut u8);", "}")
	var got [2][]string
	for i, u := range []string{one, multi} {
		res := generate(listRules(t, "name"), input{sources: []namedText{{"src/a.rs", u}, {"src/width.rs", listDef}}, pairText: listDef}, true)
		if res.code != 0 || len(res.entries) != 2 {
			t.Fatalf("layout %d: code %d %v", i, res.code, res.errs)
		}
		for _, e := range res.entries {
			got[i] = append(got[i], e.contract)
		}
	}
	if got[0][1] != got[1][1] || got[0][0] != got[1][0] || !strings.HasPrefix(got[1][1], "#[unsafe(link_section = \".text.hot\")]\n") {
		t.Errorf("one line %q, over lines %q", got[0], got[1])
	}
	// A list left open is one error: the next invocation, or the next
	// file, starts afresh.
	for _, srcs := range [][]namedText{
		{{"src/a.rs", lines("exports! {", "    fn f [", "        a,", "}", "exports! {", "    fn g [b] ();", "}")}},
		{{"src/a.rs", lines("exports! {", "    fn f [")}, {"src/b.rs", lines("exports! {", "    fn g [b] ();", "}")}},
	} {
		res := generate(listRules(t, "name"), input{sources: append(srcs, namedText{"src/width.rs", listDef}), pairText: listDef}, true)
		want := "seams: src/a.rs:2: the exports! invocation ends inside the list of names that starts here\nseams: src/a.rs:4: the exports! invocation at line 1 does not read as list declarations: want a name in the list, got `}`"
		if len(srcs) == 2 {
			want = "seams: src/a.rs: the file's braces do not balance (depth 1 at its end), so the scan cannot read it whole"
		}
		if res.code != 2 || strings.Join(res.errs, "\n") != want {
			t.Errorf("list left open, then more: code %d, errs %q", res.code, res.errs)
		}
	}
	// The slash form fails a fn line it does not read too.
	mod := lines("macro_rules! verified_export_pair {", "    ($name:ident / $je:ident) => {",
		"        #[unsafe(no_mangle)]", "        pub extern \"C\" fn $name() {}",
		"        #[unsafe(no_mangle)]", "        pub extern \"C\" fn $je() {}", "    };", "}",
		"verified_export_pair! {", "    fn malloc(size: usize) {}", "}")
	res = generate(testRules(t), input{sources: []namedText{{"src/api/mod.rs", mod}}, pairText: mod}, true)
	if res.code != 2 || !strings.Contains(strings.Join(res.errs, "\n"), "seams: src/api/mod.rs:10: a fn line in a verified_export_pair! invocation that the slash form (fn a / b(..), its names on that line) does not read: fn malloc(size: usize) {}") {
		t.Errorf("slash form, fn line not read: code %d, errs %v", res.code, res.errs)
	}
	// A fn deeper in an invocation (inside an export's body) is not a
	// line of names.
	mod = lines("macro_rules! verified_export_pair {", "    ($name:ident / $je:ident) => {",
		"        #[unsafe(no_mangle)]", "        pub extern \"C\" fn $name() {}",
		"        #[unsafe(no_mangle)]", "        pub extern \"C\" fn $je() {}", "    };", "}",
		"verified_export_pair! {", "    fn malloc / je_malloc(size: usize) {", "        fn helper() {}", "    };", "    fn free / je_free() {}", "}")
	res = generate(testRules(t), input{sources: []namedText{{"src/api/mod.rs", mod}}, pairText: mod}, true)
	// A body may be followed by a `;`.
	if res.code != 0 || len(res.entries) != 4 {
		t.Errorf("slash form, nested fn: code %d, errs %v, %s", res.code, res.errs, names(res.entries))
	}
}

func TestRulesErrors(t *testing.T) {
	base := "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\n"
	for _, c := range []struct{ text, want string }{
		{base + "bogus\tx\n", `r:5: unknown rule "bogus"`},
		{base + "category\tD\tU\n", "r:5: category D: tool U is not listed above it"},
		{base + "category\tC\tT\n", "r:5: category C listed twice"},
		{base + "guess\tX\tkind=a\n", "r:5: guess: category X is not listed above it"},
		{base + "guess\tC\tsize=a\n", "r:5: guess: want a condition"},
		{base + "guess\tC\tkind=(\n", "r:5: guess: error parsing regexp"},
		{base + "guess\tC\n", "r:5: want guess<TAB>CATEGORY|-<TAB>COND..."},
		{base + "ledger\t\n", "r:5: want ledger<TAB>DIR, with no empty field"},
		{base + "pair\tm\tf\n", "r:5: want pair<TAB>MACRO<TAB>FILE<TAB>VAR<TAB>VAR"},
		{base + "test-file\t[a\n", "r:5: unclosed ["},
		{base + "command\td\n", "r:5: command given twice"},
		{base + "ledger\ta\nledger\tb\n", "r:6: ledger given twice"},
		{base + "assumption-tool\tT\n", "r:5: assumption-tool given twice"},
		{base + "ledger\t.\n", "r:5: ledger .: want a path inside the tree"},
		{base + "ledger\tverus/..\n", "r:5: ledger verus/..: want a path inside the tree"},
		{base + "ledger\t/tmp/l\n", "r:5: ledger /tmp/l: want a path inside the tree"},
		{base + "source\t../src\n", "r:5: source ../src: want a path inside the tree"},
		{base + "seams-doc\t../S.md\n", "r:5: seams-doc ../S.md: want a path inside the tree"},
		{base + "pair\tm\t/f\ta\tb\n", "r:5: pair /f: want a path inside the tree"},
		{base + "export-macro\tm\tf\tlist\n", "r:5: want export-macro<TAB>MACRO<TAB>FILE<TAB>slash|list<TAB>VAR..."},
		{base + "export-macro\tm\tf\tlist\t\n", "r:5: want export-macro<TAB>MACRO<TAB>FILE<TAB>slash|list<TAB>VAR..., with no empty field"},
		{base + "export-macro\tw::m\tf\tlist\tv\n", "r:5: export-macro w::m: want a macro name"},
		{base + "export-macro\tm!\tf\tlist\tv\n", "r:5: export-macro m!: want a macro name"},
		{base + "export-macro\tm\tf\tcomma\tv\n", "r:5: export-macro m: form comma: want slash or list"},
		{base + "export-macro\tm\tf\tslash\ta\tb\tc\n", "r:5: export-macro m: the slash form names two exports"},
		{base + "export-macro\tm\tf\tlist\t$v\n", "r:5: export-macro m: VAR $v: want an identifier"},
		{base + "export-macro\tm\t../f\tlist\tv\n", "r:5: export-macro ../f: want a path inside the tree"},
		{base + "export-macro\tm\tf\tlist\tv\nexport-macro\tn\tf\tlist\tv\n", "r:6: export-macro given twice"},
		{base + "pair\tm\tf\ta\tb\nexport-macro\tn\tf\tlist\tv\n", "r:6: export-macro given twice"},
		{"tool\tT\ncategory\tC\tT\nassumption-tool\tT\n", "r: no command line"},
		{"command\tc\ntool\tT\nassumption-tool\tT\n", "r: no category lines"},
		{"command\tc\ntool\tT\ncategory\tC\tT\n", "r: no assumption-tool line"},
	} {
		_, err := parseRules("r", c.text)
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("rules %q: want error %q, got %v", c.text, c.want, err)
		}
	}
	for _, c := range []struct{ line, form, vars string }{
		{"pair\tm\tsrc/f.rs\ta\tb", "slash", "a b"},
		{"export-macro\tm\tsrc/f.rs\tslash\ta", "slash", "a"},
		{"export-macro\tm\tsrc/f.rs\tlist\ta\tb\tc", "list", "a b c"},
	} {
		r, err := parseRules("r", base+c.line+"\n")
		if err != nil || r.PairMacro != "m" || r.PairFile != "src/f.rs" || r.PairForm != c.form || strings.Join(r.PairVars, " ") != c.vars {
			t.Errorf("%q: %v %+v", c.line, err, r)
		}
	}
	r, err := parseRules("r", base+"guess\t-\tkind=x\tname!y\n# comment\n\n")
	if err != nil || len(r.Guesses) != 1 || r.Guesses[0].Category != "" || !r.Guesses[0].Conds[1].Negate {
		t.Errorf("good rules: %v %+v", err, r)
	}
}

// The kit's template rules file parses, and categorizes a panic.
func TestTemplate(t *testing.T) {
	r, err := readRules("../../templates/verus/seams.rules")
	if err != nil {
		t.Fatal(err)
	}
	res := generate(r, input{sources: []namedText{{"src/a.rs", lines("verus! {", "#[verifier::external_body]", "fn f() { panic!() }", "}")}}}, false)
	if res.code != 0 || !strings.Contains(res.summary, "| panic/abort | Kani | 1 |") {
		t.Errorf("template: %d %v %q", res.code, res.errs, res.summary)
	}
}

func TestTestFiles(t *testing.T) {
	r := testRules(t)
	for p, want := range map[string]bool{
		"src/tests.rs": true, "src/a/linux_tests.rs": true, "src/testutil.rs": true,
		"src/a/tests/x.rs": true, "src/a/b/tests/x.rs": true, "src/a/tests/b/x.rs": true,
		"src/tests/x.rs": false, "src/a/tests.rs.bak.rs": false, "src/lib.rs": false,
	} {
		if got := r.isTest(p); got != want {
			t.Errorf("isTest(%s) = %v, want %v", p, got, want)
		}
	}
}

func TestRelDoc(t *testing.T) {
	r := testRules(t)
	for sf, want := range map[string]string{"src/a.rs": "../../SEAMS.md", "src/a/b/c.rs": "../../../../SEAMS.md"} {
		if got := relDoc(r, sf); got != want {
			t.Errorf("relDoc(%s) = %s, want %s", sf, got, want)
		}
	}
}

func TestOverridesErrors(t *testing.T) {
	src := hsrc("x < 3", "x + 1")
	for _, c := range []struct {
		ov   string
		code int
		want string
	}{
		{"f\tpanic/abort\n", 2, "verus/ledger/src/t.rs.overrides:1: want name<TAB>category<TAB>tool"},
		{"# c\nf\tpanic/abort\tKani\ttrusted\th\tx\n", 2, "verus/ledger/src/t.rs.overrides:2: want at most name<TAB>category<TAB>tool<TAB>status<TAB>hashes"},
		{"f\tpanic/abort\tKani\ng\tpanic/abort\tKani\n", 1, "seams: verus/ledger/src/t.rs.overrides: g matches no seam"},
		{"f\tnope\t\n", 1, "seams: src/t.rs:3: f: no category (add it to verus/ledger/src/t.rs.overrides)"},
		{"f\tpanic/abort\tHammer\n", 1, "seams: src/t.rs:3: f: unknown tool Hammer"},
		{"f\tpanic/abort\tKani\ttrusted\n# c\nf\tpanic/abort\tKani\tdischarged (Kani: k::f)\n", 2, "verus/ledger/src/t.rs.overrides:3: f: a second line for this name (the first is line 1)"},
	} {
		res := generate(testRules(t), input{overrides: []namedText{{"verus/ledger/src/t.rs.overrides", c.ov}}, sources: []namedText{{"src/t.rs", src}}}, true)
		if res.code != c.code || !strings.Contains(strings.Join(res.errs, "\n"), c.want) {
			t.Errorf("overrides %q: code %d (want %d), errs %v", c.ov, res.code, c.code, res.errs)
		}
	}
	res := generate(testRules(t), input{sources: []namedText{{"src/t.rs", lines("fn f() { let x = Tracked::assume_new(); }")}}, unreadable: []string{"src/u.rs"}}, true)
	if res.code != 2 || res.errs[0] != "seams: cannot read src/u.rs" {
		t.Errorf("unreadable: %d %v", res.code, res.errs)
	}
}

// tree writes a port tree under a temporary directory.
func tree(t *testing.T, files map[string]string) string {
	t.Helper()
	d := t.TempDir()
	for f, text := range files {
		p := filepath.Join(d, f)
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(p, []byte(text), 0o644); err != nil {
			t.Fatal(err)
		}
	}
	return d
}

func runIn(t *testing.T, dir string, args ...string) (int, string, string) {
	t.Helper()
	var o, e bytes.Buffer
	code := run(append([]string{"-C", dir, "-rules", "verus/seams.rules"}, args...), &o, &e)
	return code, o.String(), e.String()
}

// -entries writes each seam on one line of nine fields, though an
// unsupported-attribute contract holds newlines and a literal a tab,
// and the escapes undo to the text the hash reads.
func TestEntriesOneLine(t *testing.T) {
	rules, err := os.ReadFile("testdata/challoc.rules")
	if err != nil {
		t.Fatal(err)
	}
	rules = append(rules, "guess\tC ABI export\tkind=.\n"...)
	src := lines("verus! {",
		"#[unsafe(no_mangle)]",
		"#[verifier::assume_unsupported_attributes]",
		"pub extern \"C\" fn f() {}",
		"#[verifier::external_body]",
		"fn g() -> (r: &'static str)",
		"    ensures r == \"a\\\\n\tb\",",
		"{ \"x\" }",
		"}")
	// A file name may hold a tab too, and a status a backslash.
	d := tree(t, map[string]string{"verus/seams.rules": string(rules), "src/a.rs": src,
		"src/t\tb.rs":                        lines("#[verifier::external_body]", "fn h() {}"),
		"verus/ledger/src/t\tb.rs.overrides": "h\tpanic/abort\tKani\ttrusted (see a\\n)\n"})
	code, o, e := runIn(t, d, "-entries")
	if code != 0 || e != "" {
		t.Fatalf("entries: %d %q", code, e)
	}
	got := strings.Split(strings.TrimSuffix(o, "\n"), "\n")
	if len(got) != 3 {
		t.Fatalf("want 3 lines, got %d: %q", len(got), o)
	}
	unesc := strings.NewReplacer(`\\`, `\`, `\n`, "\n", `\t`, "\t", `\r`, "\r")
	res := generate(readRulesT(t, d), input{sources: []namedText{{"src/a.rs", src}}}, true)
	for i, l := range got {
		f := strings.Split(l, "\t")
		if len(f) != 9 {
			t.Errorf("line %d: %d fields: %q", i, len(f), l)
			continue
		}
		if i == 2 {
			if f[0] != `src/t\tb.rs` || f[6] != `trusted (see a\\n)` || f[7] != `verus/ledger/src/t\tb.rs.overrides` {
				t.Errorf("file, status and overrides fields %q %q %q", f[0], f[6], f[7])
			}
			continue
		}
		if c := unesc.Replace(f[8]); c != norm(res.entries[i].hc) {
			t.Errorf("line %d: contract %q, want the hash text %q", i, c, norm(res.entries[i].hc))
		}
	}
	if want := `#[unsafe(no_mangle)]\n#[verifier::assume_unsupported_attributes]\npub extern "C" fn f() {}`; !strings.HasSuffix(got[0], "\t"+want) {
		t.Errorf("unsupported-attribute line %q, want it to end in %q", got[0], want)
	}
	// The literal's backslash and tab, escaped.
	if want := `fn g() -> (r: &'static str) ensures r == "a\\\\n\tb",`; !strings.HasSuffix(got[1], " "+want) {
		t.Errorf("external_body line %q", got[1])
	}
}

func TestEntryField(t *testing.T) {
	if got := entryField.Replace("a\\n\nb\tc\rd\\"); got != `a\\n\nb\tc\rd\\` {
		t.Errorf("got %q", got)
	}
}

func readRulesT(t *testing.T, d string) *Rules {
	t.Helper()
	r, err := readRules(filepath.Join(d, "verus/seams.rules"))
	if err != nil {
		t.Fatal(err)
	}
	return r
}

func TestWriteAndCheck(t *testing.T) {
	rules, err := os.ReadFile("testdata/challoc.rules")
	if err != nil {
		t.Fatal(err)
	}
	d := tree(t, map[string]string{
		"verus/seams.rules":               string(rules),
		"src/a.rs":                        lines("verus! {", "/// Does f. More.", "#[verifier::external_body]", "fn f(p: *mut u8) {", "}", "}"),
		"src/b.rs":                        lines("fn plain() {}"),
		"src/tests.rs":                    lines("#[verifier::external_body]", "fn t() {}"),
		"verus/ledger/src/old.rs.md":      "stale\n",
		"verus/ledger/README.md":          "kept\n",
		"verus/ledger/src/a.rs.overrides": "f\tpanic/abort\tKani\n",
	})
	code, _, e := runIn(t, d, "-check")
	if code != 1 || !strings.HasSuffix(e, "verus/ledger is stale: run sh verus/seams.sh --write\n") {
		t.Fatalf("check before write: %d %q", code, e)
	}
	if code, _, e = runIn(t, d, "-write"); code != 0 || e != "" {
		t.Fatalf("write: %d %q", code, e)
	}
	if _, err := os.Stat(filepath.Join(d, "verus/ledger/src/old.rs.md")); !os.IsNotExist(err) {
		t.Errorf("write left the old ledger file")
	}
	if _, err := os.Stat(filepath.Join(d, "verus/ledger/README.md")); err != nil {
		t.Errorf("write removed a file that is not a ledger file")
	}
	b, _ := os.ReadFile(filepath.Join(d, "verus/ledger/src/a.rs.md"))
	wantLedger := lines("# Seams in `src/a.rs`", "",
		"Generated by `sh verus/seams.sh --write`; do not edit it by hand.",
		"[SEAMS.md](../../SEAMS.md) says what a seam, a category and a tool",
		"are. Set a category, tool or status in `a.rs.overrides` next to this", "file.", "",
		"| item | kind | category | tool | status |", "|---|---|---|---|---|",
		"| `f` | external_body | panic/abort | Kani | trusted |", "", "## `f`", "",
		"src/a.rs:4. Kind: external_body. Category: panic/abort.", "Tool: Kani. Status: trusted.", "",
		"> Does f.", "", "Hashes: contract "+itoa(cksum("#[verifier::external_body] fn f(p: *mut u8)"))+", body "+itoa(cksum("{ }"))+".", "",
		"```rust", "fn f(p: *mut u8)", "```")
	if string(b) != wantLedger {
		t.Errorf("ledger:\n%s\nwant:\n%s", b, wantLedger)
	}
	code, o, e := runIn(t, d, "-check")
	if code != 0 || o != "verus/ledger is up to date\n" || e != "" {
		t.Fatalf("check after write: %d %q %q", code, o, e)
	}
	code, o, _ = runIn(t, d, "-entries")
	if code != 0 || o != "src/a.rs\t4\texternal_body\tf\tpanic/abort\tKani\ttrusted\tverus/ledger/src/a.rs.overrides\t#[verifier::external_body] fn f(p: *mut u8)\n" {
		t.Errorf("entries: %d %q", code, o)
	}
	code, o, _ = runIn(t, d)
	if code != 0 || !strings.HasPrefix(o, "1 seam, in 1 source file.\n\n| category | seams |\n|---|---:|\n| panic/abort | 1 |\n") {
		t.Errorf("summary: %d %q", code, o)
	}
	// An edited ledger file, a missing one and an extra one are stale;
	// so is an overrides file whose source has no seams.
	for what, edit := range map[string]func(){
		"edited":   func() { os.WriteFile(filepath.Join(d, "verus/ledger/src/a.rs.md"), []byte("x\n"), 0o644) },
		"missing":  func() { os.Remove(filepath.Join(d, "verus/ledger/src/a.rs.md")) },
		"extra":    func() { os.WriteFile(filepath.Join(d, "verus/ledger/src/c.rs.md"), []byte("x\n"), 0o644) },
		"leftover": func() { os.WriteFile(filepath.Join(d, "verus/ledger/src/b.rs.overrides"), []byte("# none\n"), 0o644) },
	} {
		edit()
		code, _, e := runIn(t, d, "-check")
		if code != 1 || !strings.Contains(e, "is stale") {
			t.Errorf("%s: %d %q", what, code, e)
		}
		if what == "leftover" && !strings.Contains(e, "seams: verus/ledger/src/b.rs.overrides: its source file has no seams\n") {
			t.Errorf("leftover: %q", e)
		}
		os.Remove(filepath.Join(d, "verus/ledger/src/c.rs.md"))
		os.Remove(filepath.Join(d, "verus/ledger/src/b.rs.overrides"))
		runIn(t, d, "-write")
	}
	// A drift line fails -check with the ledger fresh; -write prints it
	// and writes all the same.
	os.WriteFile(filepath.Join(d, "verus/ledger/src/a.rs.overrides"), []byte("f\tpanic/abort\tKani\tdischarged (Kani: k::f)\n"), 0o644)
	code, _, e = runIn(t, d, "-write")
	if code != 0 || !strings.Contains(e, "records no hashes") {
		t.Errorf("write with drift: %d %q", code, e)
	}
	code, _, e = runIn(t, d, "-check")
	if code != 1 || !strings.Contains(e, "records no hashes") || strings.Contains(e, "is stale") {
		t.Errorf("check with drift: %d %q", code, e)
	}
}

// A golden from challoc: testdata/challoc holds nine of challoc's source
// files at deda0db (src/vspec/{panic,fmt,misc,zero}.rs,
// src/sys/{raw,pac}.rs, src/api/glibc.rs, src/heap/{slotstate,thread}.rs)
// with their ledger and overrides files, and testdata/challoc.summary
// is what challoc's verus/seams.sh printed over that tree; its
// --write left the ledger as it is. -check passes on it and the totals
// equal seams.sh's, so a change that breaks parity with seams.sh on
// real sources fails here.
func TestChallocGolden(t *testing.T) {
	rules, err := filepath.Abs("testdata/challoc.rules")
	if err != nil {
		t.Fatal(err)
	}
	var o, e bytes.Buffer
	if code := run([]string{"-C", "testdata/challoc", "-rules", rules, "-check"}, &o, &e); code != 0 || o.String() != "verus/ledger is up to date\n" || e.Len() != 0 {
		t.Errorf("-check: %d %q %q", code, o.String(), e.String())
	}
	want, err := os.ReadFile("testdata/challoc.summary")
	if err != nil {
		t.Fatal(err)
	}
	o.Reset()
	e.Reset()
	if code := run([]string{"-C", "testdata/challoc", "-rules", rules}, &o, &e); code != 0 || o.String() != string(want) || e.Len() != 0 {
		t.Errorf("summary: %d\n%s\nstderr %q", code, o.String(), e.String())
	}
}

func TestUsage(t *testing.T) {
	var o, e bytes.Buffer
	for _, args := range [][]string{{"-write", "-check"}, {"extra"}, {"-nope"}} {
		if code := run(args, &o, &e); code != 2 {
			t.Errorf("%v: code %d", args, code)
		}
	}
	d := tree(t, map[string]string{"verus/seams.rules": "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\n"})
	if code, _, e := runIn(t, d); code != 2 || e != "seams: no sources under src/\n" {
		t.Errorf("no sources: %d %q", code, e)
	}
	if code := run([]string{"-C", d, "-rules", "missing"}, &o, &e); code != 2 {
		t.Errorf("missing rules: %d", code)
	}
	// In a tree that would scan, two modes at once are still a usage
	// error, and write nothing.
	d = tree(t, map[string]string{"verus/seams.rules": "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\nguess\tC\tkind=.\n",
		"src/a.rs": lines("#[verifier::external_body]", "fn f() {}")})
	if code, o, e := runIn(t, d, "-write", "-check"); code != 2 || o != "" || !strings.HasPrefix(e, "usage: seams") {
		t.Errorf("-write -check: %d %q %q", code, o, e)
	}
	if _, err := os.Stat(filepath.Join(d, "verus/ledger")); !os.IsNotExist(err) {
		t.Errorf("-write -check wrote the ledger")
	}
	// A source the walk lists but cannot read (a dangling link) is an
	// input error, never a file with no seams.
	if err := os.Symlink("nowhere.rs", filepath.Join(d, "src/u.rs")); err != nil {
		t.Fatal(err)
	}
	if code, _, e := runIn(t, d); code != 2 || !strings.Contains(e, "seams: cannot read src/u.rs") {
		t.Errorf("unreadable source: %d %q", code, e)
	}
}

// Attributes before an item on their line are read whole: the hash
// text holds each as written.
func TestAttrsOnItemLine(t *testing.T) {
	es := scan(t, lines("verus! {", "#[allow(dead_code)] #[verifier::external_body] fn f() {}", "}"))
	if len(es) != 1 || norm(es[0].hc) != "#[allow(dead_code)] #[verifier::external_body] fn f()" {
		t.Fatalf("got %q", norm(es[0].hc))
	}
	// Brackets nested in an attribute stay inside it.
	es = scan(t, lines("verus! {", "#[x([a])] #[verifier::external_body] fn f() {}", "}"))
	if len(es) != 1 || norm(es[0].hc) != "#[x([a])] #[verifier::external_body] fn f()" {
		t.Fatalf("nested brackets: got %+v", es)
	}
}
