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
	if res := generate(r, input{sources: []namedText{{"src/a/b.rs", src}}}, true); res.code != 0 {
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
}

func TestPair(t *testing.T) {
	mod := lines("macro_rules! verified_export_pair {",
		"    ($(#[$m:meta])* fn $name:ident / $je:ident ($($a:tt)*)) => {",
		"        $(#[$m])*", "        #[unsafe(no_mangle)]", "        pub extern \"C\" fn $name() {}",
		"        #[unsafe(export_name =", "            \"je\")]", "        pub extern \"C\" fn $je() {}", "    };", "}",
		"verified_export_pair! {", "    fn malloc / je_malloc(size: usize)", "}")
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
		{`fn f<'a>(x: &'a u8) {`, `fn f<'a>(x: &'a u8) {`},
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
		{"tool\tT\ncategory\tC\tT\nassumption-tool\tT\n", "r: no command line"},
		{"command\tc\ntool\tT\nassumption-tool\tT\n", "r: no category lines"},
		{"command\tc\ntool\tT\ncategory\tC\tT\n", "r: no assumption-tool line"},
	} {
		_, err := parseRules("r", c.text)
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("rules %q: want error %q, got %v", c.text, c.want, err)
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
}
