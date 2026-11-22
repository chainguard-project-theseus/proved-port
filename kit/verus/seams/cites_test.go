package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// citeRules: challoc's cite tags, loom and GenMC partial.
func citeRules(t *testing.T) *Rules {
	t.Helper()
	r, err := parseRules("r", "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\n"+
		"cite-tag\tKani\ncite-tag\tconst assert\ncite-tag\tIsla\ncite-tag\tloom\tpartial\ncite-tag\tGenMC\tpartial\n")
	if err != nil {
		t.Fatal(err)
	}
	return r
}

func fmtCites(cs []cite, empty []string) string {
	var b strings.Builder
	for _, c := range cs {
		b.WriteString(c.tag + "=" + c.target + "|")
	}
	if len(empty) > 0 {
		b.WriteString("empty:" + strings.Join(empty, ","))
	}
	return b.String()
}

// The parse is seams.sh's cites().
func TestCitations(t *testing.T) {
	r := citeRules(t)
	for _, c := range []struct{ stat, want string }{
		{"discharged (Kani: a::f)", "Kani=a::f|"},
		{"discharged (Kani: a::f, b::g and c::h; not checked: x)", "Kani=a::f|Kani=b::g|Kani=c::h|"},
		{"discharged (Kani: a, and b)", "Kani=a|Kani=b|"},
		{"discharged (Kani: a,b,  c)", "Kani=a|Kani=b|Kani=c|"},
		// The list runs to the end when no `;` or `)` closes it.
		{"discharged by Kani: a", "Kani=a|"},
		// A part holding `: ` starts the next tag, read at its place;
		// the citations come in text order whatever the rules' order.
		{"trusted (partly checked, const assert: `X` in f.rs, Kani: k::g; not checked: y)", "const assert=`X` in f.rs|Kani=k::g|"},
		{"discharged (Kani: a, see: b)", "Kani=a|"},
		{"discharged (Kani: a; Kani: b)", "Kani=a|Kani=b|"},
		{"discharged (loom: init::t; GenMC: init1 [rc11 imm])", "loom=init::t|GenMC=init1 [rc11 imm]|"},
		// Blanks after the tag's own are kept, as seams.sh kept them.
		{"discharged (Kani:  a)", "Kani= a|"},
		// `TAG:` without its blank, or another word, cites nothing.
		{"discharged (Kani:a; Kani)", ""},
		// A list that names nothing.
		{"discharged (Kani: ; x)", "empty:Kani"},
		{"discharged (Kani: )", "empty:Kani"},
		{"discharged (Kani: ", "empty:Kani"},
		{"discharged (Kani: , and ; Isla: f.md)", "Isla=f.md|empty:Kani"},
		{"discharged (Kani: x: y)", "empty:Kani"},
		{"trusted", ""},
	} {
		if got := fmtCites(r.citations(c.stat)); got != c.want {
			t.Errorf("%q: got %q, want %q", c.stat, got, c.want)
		}
	}
}

// -check on the citations: an empty list on any status, and a partial
// tag on a discharged status.
func TestCiteDrift(t *testing.T) {
	src := hsrc("x < 3", "x + 1")
	h := offered(t, hs(t, "discharged (Kani: k::f)", "", "", "", src), `^contract [0-9]+, body [0-9]+$`)
	want(t, "empty list", hs(t, "discharged (Kani: ; const assert: `A` in f.rs)", h, "", "", src), "seams: verus/ledger/src/t.rs.overrides: f: `Kani: ` names no check")
	want(t, "empty list on a trusted status", hs(t, "trusted (Kani: )", "", "", "", src), "f: `Kani: ` names no check")
	got := hs(t, "discharged (loom: a::t, a::u; GenMC: c [rc11])", h, "", "", src)
	want(t, "discharged by loom", got, "seams: verus/ledger/src/t.rs.overrides: f: discharged by a loom check, which checks only part of the contract (at most partly checked)")
	want(t, "discharged by GenMC", got, "f: discharged by a GenMC check")
	if n := strings.Count(got, "discharged by a loom check"); n != 1 {
		t.Errorf("one line per partial tag, got %d:\n%s", n, got)
	}
	wantNone(t, "partly checked by loom", hs(t, "trusted (partly checked, loom: a::t; not checked: x)", h, "", "", src))
	wantNone(t, "discharged by Kani and const assert", hs(t, "discharged (Kani: k::f; const assert: `A` in f.rs)", h, "", "", src))
}

func TestCiteTagRules(t *testing.T) {
	base := "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\n"
	for _, c := range []struct{ text, want string }{
		{base + "cite-tag\tKani\tfull\n", `r:5: want cite-tag<TAB>TAG[<TAB>partial], not "full" in the third field`},
		{base + "cite-tag\tKani\tpartial\tx\n", "r:5: want cite-tag<TAB>TAG[<TAB>partial]"},
		{base + "cite-tag\n", "r:5: want cite-tag<TAB>TAG[<TAB>partial]"},
		{base + "cite-tag\t\tpartial\n", "r:5: want cite-tag<TAB>TAG[<TAB>partial], with no empty field"},
		{base + "cite-tag\tKani\ncite-tag\tKani\tpartial\n", "r:6: cite-tag Kani is given twice"},
		{base + "cite-tag\tKani:\n", "r:5: cite-tag Kani: holds `;`, `)` or `:`"},
		{base + "cite-tag\ta;b\n", "r:5: cite-tag a;b holds"},
		{base + "cite-tag\ta)\n", "r:5: cite-tag a) holds"},
	} {
		_, err := parseRules("r", c.text)
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("rules %q: want error %q, got %v", c.text, c.want, err)
		}
	}
	r, err := parseRules("r", base+"cite-tag\tKani\ncite-tag\tloom\tpartial\n")
	if err != nil || strings.Join(r.CiteTags, ",") != "Kani,loom" || r.Partial["Kani"] || !r.Partial["loom"] {
		t.Errorf("good rules: %v %+v", err, r)
	}
}

// -cites: one escaped line of six fields per citation, seams in scan
// order, a shared name once per seam it serves, nothing for a seam with
// no overrides line.
func TestCitesMode(t *testing.T) {
	rules := "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\nguess\tC\tkind=.\ncite-tag\tKani\ncite-tag\tIsla\n"
	src := lines("verus! {",
		"#[verifier::external_body]", "fn g() {}",
		"#[verifier::external_body]", "fn f() {}",
		"#[verifier::external_body]", "fn h() {}",
		"}",
		"mod m { verus! {", "#[verifier::external_body]", "fn h() {}", "} }")
	d := tree(t, map[string]string{"verus/seams.rules": rules, "src/a.rs": src,
		"verus/ledger/src/a.rs.overrides": lines(
			"f\tC\tT\ttrusted (Kani: a::k1 and a::k2; Isla: v\\i.md)",
			"h\tC\tT\ttrusted (Kani: m::k)",
			"g\tC\tT\ttrusted"),
		"src/t\tb.rs":                        lines("#[verifier::external_body]", "fn x() {}"),
		"verus/ledger/src/t\tb.rs.overrides": "x\tC\tT\ttrusted (Kani: t::k)\n"})
	code, o, e := runIn(t, d, "-cites")
	if code != 0 || e != "" {
		t.Fatalf("-cites: %d %q", code, e)
	}
	wantOut := lines(
		"verus/ledger/src/a.rs.overrides\tsrc/a.rs\t5\tf\tKani\ta::k1",
		"verus/ledger/src/a.rs.overrides\tsrc/a.rs\t5\tf\tKani\ta::k2",
		"verus/ledger/src/a.rs.overrides\tsrc/a.rs\t5\tf\tIsla\tv\\\\i.md",
		"verus/ledger/src/a.rs.overrides\tsrc/a.rs\t7\th\tKani\tm::k",
		"verus/ledger/src/a.rs.overrides\tsrc/a.rs\t11\th\tKani\tm::k",
		"verus/ledger/src/t\\tb.rs.overrides\tsrc/t\\tb.rs\t2\tx\tKani\tt::k")
	if o != wantOut {
		t.Errorf("-cites:\n%s\nwant:\n%s", o, wantOut)
	}
	// An input error prints no citations.
	if err := os.WriteFile(filepath.Join(d, "verus/ledger/src/a.rs.overrides"), []byte("nope\tC\tT\ttrusted (Kani: k)\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if code, o, e := runIn(t, d, "-cites"); code != 1 || o != "" || !strings.Contains(e, "nope matches no seam") {
		t.Errorf("-cites, bad overrides: %d %q %q", code, o, e)
	}
	if code, _, e := runIn(t, d, "-cites", "-entries"); code != 2 || !strings.Contains(e, "-cites]") {
		t.Errorf("-cites -entries: %d %q", code, e)
	}
}

// challoc's golden tree: -cites gives testdata/challoc.cites, which
// equals seams.sh's cites() on those overrides once its forms are put
// together (`NAME in TARGET` for Isla and so on; the PR's parity run).
func TestChallocCites(t *testing.T) {
	rules, err := filepath.Abs("testdata/challoc.rules")
	if err != nil {
		t.Fatal(err)
	}
	want, err := os.ReadFile("testdata/challoc.cites")
	if err != nil {
		t.Fatal(err)
	}
	var o, e bytes.Buffer
	if code := run([]string{"-C", "testdata/challoc", "-rules", rules, "-cites"}, &o, &e); code != 0 || o.String() != string(want) || e.Len() != 0 {
		t.Errorf("-cites: %d\n%s\nstderr %q", code, o.String(), e.String())
	}
}
