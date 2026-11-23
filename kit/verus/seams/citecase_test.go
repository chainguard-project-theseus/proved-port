package main

import (
	"strings"
	"testing"
)

// citeCaseFiles is a GenMC directory after challoc's tests/genmc, with
// seams.sh's genmctest() self-test cases (g_ok to g_old, t.rs to x.rs)
// and more.
var citeCaseFiles = map[string]string{
	"tests/genmc/cases": lines(
		"# name | template | models | constants | expected",
		"g_ok | t | rc11 imm | const X: u32 = 0; | pass",
		"g_ctl | t | rc11 imm | const X: u32 = 1; | fail | text",
		"g_other | u | rc11 | const X: u32 = 0; | pass",
		"g_nosrc | v | rc11 | const X: u32 = 0; | pass",
		"g_method | w | imm arm8 | const X: u32 = 0; | pass",
		"g_lock | x | rc11 | const X: u32 = 0; | pass",
		"# g_old | t | rc11 | const X: u32 = 0; | pass",
		// Beyond genmctest(): what run.sh reads.
		"",
		"  g_spaced  |  t  |  rc11   imm+live  |  const X: u32 = 0;  |  pass  ",
		"g_twice | t | rc11 | const X: u32 = 0; | pass",
		"g_twice | t | rc11 imm | const X: u32 = 0; | pass",
		"g_twice_ctl | t | rc11 | const X: u32 = 0; | pass",
		"g_twice_ctl | t | rc11 | const X: u32 = 1; | fail | text",
		"g_empty | t | rc11 | | pass",
		"g_short | t | rc11 | pass",
		"g_passmore | t | rc11 | const X: u32 = 0; | pass | text",
		"g_fail5 | t | rc11 | const X: u32 = 1; | fail",
		"g_badtmpl | t.x | rc11 | const X: u32 = 0; | pass",
		"g_notmpl | none | rc11 | const X: u32 = 0; | pass",
		"g_open | open | rc11 | const X: u32 = 0; | pass",
		"g_both | both | rc11 | const X: u32 = 0; | pass",
		" # g_indented | t | rc11 | const X: u32 = 0; | pass",
	),
	"tests/genmc/t.rs":    lines("// A port of `vspec/count.rs`: `count::fetch_add`.", "fn harness() {}"),
	"tests/genmc/u.rs":    lines("// A port of `vspec/count.rs`: `count::fetch_add_split`, `fetch_add` and count::fetch_add."),
	"tests/genmc/v.rs":    lines("// A port of `count.rs`, not by its path: `count::fetch_add`."),
	"tests/genmc/w.rs":    lines("// A port of `src/slab.rs`: `init` and `SlabDesc::set_state`."),
	"tests/genmc/x.rs":    lines("// A port of `lock.rs`, not by its path: `RawLock::lock_slow`."),
	"tests/genmc/open.rs": lines("// A port of `vspec/count.rs`: `count::fetch_add`.", "/*"),
	// The source file and the function on different lines.
	"tests/genmc/both.rs": lines("// A port of `src/vspec/count.rs`.", "fn harness() {", "    // drives `vspec::count::fetch_add`", "}"),
}

const citeCaseRules = resolveRules + "cite-tag\tGenMC\tpartial\ncite-case\tGenMC\ttests/genmc\n"

// TestCiteCase holds seams.sh's genmctest() self-test cases, and the
// checks the kit adds (one row per case, no empty field, a template
// that lexes whole).
func TestCiteCase(t *testing.T) {
	r, err := parseRules("r", citeCaseRules)
	if err != nil {
		t.Fatal(err)
	}
	d := tree(t, citeCaseFiles)
	for _, c := range []struct {
		file, name, target, want string // want: "" resolves, else a part of the error
	}{
		// genmctest()'s self-test.
		{"src/vspec/count.rs", "fetch_add", "g_ok [rc11 imm]", ""},
		{"src/slab.rs", "SlabDesc::set_state", "g_method [imm arm8]", ""},
		{"src/vspec/count.rs", "fetch_add", "g_ctl [rc11 imm]", "tests/genmc/cases:3: the case g_ctl expects fail | text, not `pass` alone"},
		{"src/vspec/count.rs", "fetch_add", "g_ok [rc11]", "tests/genmc/cases:2: the case g_ok runs under [rc11 imm], not [rc11]"},
		{"src/vspec/count.rs", "fetch_add", "g_ok [rc11 imm arm8]", "runs under [rc11 imm], not [rc11 imm arm8]"},
		{"src/vspec/count.rs", "fetch_add", "g_ok [imm rc11]", "runs under [rc11 imm], not [imm rc11]"},
		{"src/vspec/count.rs", "fetch_add", "g_ok", "is no `CASE [MODEL ...]` citation"},
		{"src/vspec/count.rs", "fetch_add", "g_ok []", "is no `CASE [MODEL ...]` citation"},
		{"src/vspec/count.rs", "fetch_add", "no_such [rc11 imm]", "tests/genmc/cases has no case no_such"},
		{"src/vspec/count.rs", "store", "g_ok [rc11 imm]", "tests/genmc/t.rs does not name `count::store` in backquotes"},
		{"src/vspec/count.rs", "fetch_add", "g_other [rc11]", "tests/genmc/u.rs does not name `count::fetch_add`"},
		{"src/vspec/count.rs", "fetch_add", "g_nosrc [rc11]", "tests/genmc/v.rs names neither `src/vspec/count.rs` nor `vspec/count.rs`"},
		{"src/vspec/count.rs", "fetch_add", "g_old [rc11]", "has no case g_old"},
		{"src/vspec/count.rs", "fetch_add", "t [rc11 imm]", "has no case t"},
		{"src/vspec/byte.rs", "fetch_add", "g_ok [rc11 imm]", "tests/genmc/t.rs names neither `src/vspec/byte.rs` nor `vspec/byte.rs`"},
		{"src/slab.rs", "SlabDesc::init", "g_method [imm arm8]", "does not name `SlabDesc::init`"},
		{"src/lock.rs", "RawLock::lock_slow", "g_lock [rc11]", "tests/genmc/x.rs does not name `src/lock.rs`"},
		// Fields are trimmed and models split on blanks, as run.sh reads
		// them; the citation writes one blank between its words.
		{"src/vspec/count.rs", "fetch_add", "g_spaced [rc11 imm+live]", ""},
		{"src/vspec/count.rs", "fetch_add", "g_spaced [rc11  imm+live]", "is no `CASE [MODEL ...]` citation"},
		{"src/vspec/count.rs", "fetch_add", "g_spaced [ rc11 imm+live]", "is no `CASE [MODEL ...]` citation"},
		{"src/vspec/count.rs", "fetch_add", "g.ok [rc11 imm]", "is no `CASE [MODEL ...]` citation"},
		{"src/vspec/count.rs", "fetch_add", "g_ok [rc11 imm] x", "is no `CASE [MODEL ...]` citation"},
		// Beyond genmctest(), fail closed.
		{"src/vspec/count.rs", "fetch_add", "g_twice [rc11]", "tests/genmc/cases names the case g_twice on 2 lines"},
		{"src/vspec/count.rs", "fetch_add", "g_twice_ctl [rc11]", "names the case g_twice_ctl on 2 lines"},
		{"src/vspec/count.rs", "fetch_add", "g_empty [rc11]", "tests/genmc/cases:15: the case g_empty has an empty field"},
		{"src/vspec/count.rs", "fetch_add", "g_short [rc11]", "tests/genmc/cases:16: the case g_short has 4 fields"},
		{"src/vspec/count.rs", "fetch_add", "g_passmore [rc11]", "expects pass | text, not `pass` alone"},
		{"src/vspec/count.rs", "fetch_add", "g_fail5 [rc11]", "expects fail, not `pass` alone"},
		{"src/vspec/count.rs", "fetch_add", "g_badtmpl [rc11]", "the case g_badtmpl's template t.x is not a word"},
		{"src/vspec/count.rs", "fetch_add", "g_notmpl [rc11]", "tests/genmc/none.rs (the case g_notmpl's template) is not there"},
		{"src/vspec/count.rs", "fetch_add", "g_open [rc11]", "tests/genmc/open.rs (the case g_open's template) is not there or ends inside a literal or comment"},
		// A line starting with a blank before `#` is a row to run.sh.
		{"src/vspec/count.rs", "fetch_add", "# g_indented [rc11]", "is no `CASE [MODEL ...]` citation"},
		// The file and the function may be named on different lines, the
		// function behind a path.
		{"src/vspec/count.rs", "fetch_add", "g_both [rc11]", ""},
	} {
		at := "seams: verus/ledger/" + c.file + ".overrides: " + c.name + ": GenMC: " + c.target
		got := resolveCites(d, r, []*entry{{file: c.file, name: c.name, okey: c.file + ":" + c.name, stat: "trusted (partly checked, GenMC: " + c.target + "; not checked: x)"}})
		switch {
		case c.want == "" && len(got) != 0:
			t.Errorf("%s @ %s from %s: want resolved, got %q", c.name, c.target, c.file, got)
		case c.want != "" && (len(got) != 1 || !strings.HasPrefix(got[0], at+": ") || !strings.Contains(got[0], c.want)):
			t.Errorf("%s @ %s from %s: want an error holding %q, got %q", c.name, c.target, c.file, c.want, got)
		}
	}
}

// A `#N` seam (`fetch_add#2`) is matched by its name alone, and several
// citations of one tag resolve each.
func TestCiteCaseIndexed(t *testing.T) {
	r, err := parseRules("r", citeCaseRules)
	if err != nil {
		t.Fatal(err)
	}
	d := tree(t, citeCaseFiles)
	got := resolveCites(d, r, []*entry{{file: "src/vspec/count.rs", name: "fetch_add", okey: "src/vspec/count.rs:fetch_add#2", stat: "trusted (partly checked, GenMC: g_ok [rc11 imm] and g_spaced [rc11 imm+live]; not checked: x)"}})
	if len(got) != 0 {
		t.Errorf("got %q", got)
	}
}

// With no cases file every citation fails.
func TestCiteCaseNoCases(t *testing.T) {
	r, err := parseRules("r", citeCaseRules)
	if err != nil {
		t.Fatal(err)
	}
	d := tree(t, map[string]string{"tests/genmc/t.rs": citeCaseFiles["tests/genmc/t.rs"]})
	got := resolveCites(d, r, []*entry{{file: "src/vspec/count.rs", name: "fetch_add", okey: "src/vspec/count.rs:fetch_add", stat: "trusted (partly checked, GenMC: g_ok [rc11 imm]; not checked: x)"}})
	want := "seams: verus/ledger/src/vspec/count.rs.overrides: fetch_add: GenMC: g_ok [rc11 imm]: tests/genmc/cases is not there"
	if strings.Join(got, "\n") != want {
		t.Errorf("got %q, want %q", got, want)
	}
}

func TestCiteCaseRules(t *testing.T) {
	base := "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\ncite-tag\tK\n"
	for _, c := range []struct{ text, want string }{
		{base + "cite-case\tX\td\n", "r:6: cite-case X: not a cite-tag listed above it"},
		{base + "cite-case\tK\n", "r:6: want cite-case<TAB>TAG<TAB>DIR"},
		{base + "cite-case\tK\td\te\n", "r:6: want cite-case<TAB>TAG<TAB>DIR"},
		{base + "cite-case\tK\t\n", "with no empty field"},
		{base + "cite-case\tK\t../d\n", "r:6: cite-case ../d: want a path inside the tree"},
		{base + "cite-case\tK\td\ncite-case\tK\te\n", "r:7: cite-case K: the tag already has a cite-fn, cite-assert, cite-test or cite-case line"},
		{base + "cite-test\tK\td\ncite-case\tK\te\n", "r:7: cite-case K: the tag already has"},
		{base + "cite-case\tK\td\ncite-fn\tK\ttest\n", "r:7: cite-fn K: the tag already has"},
		{base + "cite-case\tK\td\ncite-assert\tK\n", "r:7: cite-assert K: the tag already has"},
		{base + "cite-case\tK\td\ncite-test\tK\td\n", "r:7: cite-test K: the tag already has"},
	} {
		_, err := parseRules("r", c.text)
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("rules %q: want error %q, got %v", c.text, c.want, err)
		}
	}
	r, err := parseRules("r", base+"cite-case\tK\ttests/genmc/\ncite-row\tK\tR.md\tH\n")
	if err != nil || r.CiteCase["K"] != "tests/genmc" || r.CiteRow["K"].file != "R.md" {
		t.Errorf("good rules: %v %+v", err, r)
	}
}
