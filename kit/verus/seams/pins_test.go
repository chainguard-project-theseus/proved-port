package main

import (
	"reflect"
	"strings"
	"testing"
)

// pinsOut runs readPins, pinsHeld and copiesMatch on a pins file in a
// tree of files, as -check does.
func pinsOut(t *testing.T, files map[string]string) []string {
	t.Helper()
	d := tree(t, files)
	rows, errs, ok := readPins(d, "p.txt")
	if !ok {
		return errs
	}
	return append(append(errs, pinsHeld(d, "p.txt", rows)...), copiesMatch("p.txt", rows)...)
}

// rowsOf writes `FILE<TAB>LINE[<TAB>TAG]` rows, one a string with its
// fields split by `|`.
func rowsOf(rows ...string) string {
	var b strings.Builder
	for _, r := range rows {
		b.WriteString(strings.ReplaceAll(r, "|", "\t") + "\n")
	}
	return b.String()
}

// seams.sh's self-test of pins(): lines in place pass (with a `...` row
// between two of them); a changed line, a skipped line, lines a `...`
// row is missing between, a second copy of the lines earlier in the file
// (in a block comment), a row with four fields, and a pins file that is
// gone fail.
func TestPins(t *testing.T) {
	pin := lines("fn f() {", "    // a comment", "", "    let a = 1;", "    a", "}", "fn g() {}")
	decoy := lines("/*", "fn f() {", "    let a = 1;", "    a", "}", "*/") + pin
	ok := []string{"a.rs|fn f() {", "a.rs|let a = 1;", "a.rs|a", "a.rs|...", "a.rs|fn g() {}"}
	for _, c := range []struct {
		name  string
		files map[string]string
		want  string // "" passes; else a line of the output
	}{
		{"in place", map[string]string{"a.rs": pin, "p.txt": "# a comment\n\n  \n" + rowsOf(ok...)}, ""},
		{"changed", map[string]string{"a.rs": pin, "p.txt": rowsOf("a.rs|fn f() {", "a.rs|let a = 2;")}, "seams: p.txt: a.rs no longer has, in place: let a = 2;"},
		{"skipped", map[string]string{"a.rs": pin, "p.txt": rowsOf("a.rs|fn f() {", "a.rs|a")}, "seams: p.txt: a.rs no longer has, in place: a"},
		{"gap", map[string]string{"a.rs": pin, "p.txt": rowsOf("a.rs|a", "a.rs|fn g() {}")}, "seams: p.txt: a.rs no longer has, in place: fn g() {}"},
		{"decoy", map[string]string{"a.rs": decoy, "p.txt": rowsOf(ok...)}, "seams: p.txt: a.rs has 2 lines fn f() {, want 1"},
		{"a later run's start twice", map[string]string{"a.rs": pin + "fn g() {}\n", "p.txt": rowsOf(ok...)}, "seams: p.txt: a.rs has 2 lines fn g() {}, want 1"},
		{"four fields", map[string]string{"a.rs": pin, "p.txt": rowsOf("a.rs|fn f() {|w:orig|x")}, "seams: p.txt: a row has more than three fields: a.rs\tfn f() {\tw:orig\tx"},
		{"pins gone", map[string]string{"a.rs": pin}, "seams: p.txt is gone"},
		{"source gone", map[string]string{"p.txt": rowsOf("b.rs|x")}, "seams: p.txt: b.rs is gone"},
		// A run's first line that the file lacks: not in place, and held
		// 0 times (seams.sh printed both).
		{"start missing", map[string]string{"a.rs": pin, "p.txt": rowsOf("a.rs|zzz")}, "seams: p.txt: a.rs has 0 lines zzz, want 1"},
		// Where seams.sh was lax, fail closed: a row of one field, an
		// empty field, a path outside the tree or not clean, a `...` row
		// that does not stand between two lines of its file.
		{"one field", map[string]string{"a.rs": pin, "p.txt": rowsOf(ok...) + "fn g() {}\n"}, "seams: p.txt:6: a row has one field, want FILE<TAB>LINE[<TAB>NAME:ROLE]: fn g() {}"},
		{"empty field", map[string]string{"a.rs": pin, "p.txt": rowsOf("a.rs|fn f() {|")}, "seams: p.txt:1: a row has an empty field: a.rs\tfn f() {\t"},
		{"outside", map[string]string{"a.rs": pin, "p.txt": rowsOf("../a.rs|fn f() {")}, "seams: p.txt:1: ../a.rs is not a clean path inside the tree"},
		{"absolute", map[string]string{"a.rs": pin, "p.txt": rowsOf("/a.rs|fn f() {")}, "seams: p.txt:1: /a.rs is not a clean path inside the tree"},
		{"unclean", map[string]string{"a.rs": pin, "p.txt": rowsOf("./a.rs|fn f() {")}, "seams: p.txt:1: ./a.rs is not a clean path inside the tree"},
		{"... first", map[string]string{"a.rs": pin, "p.txt": rowsOf("a.rs|...", "a.rs|fn g() {}")}, "seams: p.txt: a `...` row for a.rs does not stand between two of its lines"},
		{"... last", map[string]string{"a.rs": pin, "p.txt": rowsOf("a.rs|fn f() {", "a.rs|...")}, "seams: p.txt: a `...` row for a.rs does not stand between two of its lines"},
		{"... twice", map[string]string{"a.rs": pin, "p.txt": rowsOf("a.rs|fn f() {", "a.rs|...", "a.rs|...", "a.rs|fn g() {}")}, "seams: p.txt: a `...` row for a.rs does not stand between two of its lines"},
		// A file of no rows holds nothing in place.
		{"no rows", map[string]string{"a.rs": pin, "p.txt": "# all\n# commented out\n\n"}, "seams: p.txt has no rows, so it holds no line in place"},
		{"empty", map[string]string{"a.rs": pin, "p.txt": ""}, "seams: p.txt has no rows, so it holds no line in place"},
		// A tagged `...` would keep a side consecutive across a gap.
		{"... tagged", map[string]string{"a.rs": pin, "p.txt": rowsOf("a.rs|fn f() {|w:orig", "a.rs|...|w:orig-only", "a.rs|fn g() {}|w:orig")}, "seams: p.txt:2: a `...` row has a third field, which would let a side's rows span a gap: a.rs\t...\tw:orig-only"},
		// Rows of two files interleave; each file's rows are its own run.
		{"two files", map[string]string{"a.rs": pin, "b.rs": lines("x", "y"), "p.txt": rowsOf("a.rs|fn f() {", "b.rs|x", "a.rs|let a = 1;", "b.rs|y")}, ""},
	} {
		got := pinsOut(t, c.files)
		if c.want == "" && len(got) != 0 || c.want != "" && !contains(got, c.want) {
			t.Errorf("%s: got %q, want %q", c.name, got, c.want)
		}
	}
}

// A file whose only row cannot be read fails for that row alone, not
// again as a file of no rows.
func TestPinsBadRowOnly(t *testing.T) {
	got := pinsOut(t, map[string]string{"p.txt": "x\n"})
	if !reflect.DeepEqual(got, []string{"seams: p.txt:1: a row has one field, want FILE<TAB>LINE[<TAB>NAME:ROLE]: x"}) {
		t.Errorf("got %q", got)
	}
}

func contains(l []string, s string) bool {
	for _, x := range l {
		if x == s {
			return true
		}
	}
	return false
}

// seams.sh's self-test of copies(): a matching copy passes, with its
// pins in place; a copy changed with its rows updated (pins still
// passes), an unmarked row inside the original or inside the copy, a row
// with an unknown role, and a NAME with no copy rows fail.
func TestCopies(t *testing.T) {
	cp := lines("fn o() {", "    let a = 1;", "    hint(a);", "    a", "}", "fn c() {", "    let a = 1;", "    a", "}")
	// The copy's `let a = 1;` (after `fn c() {`) becomes `let a = 2;`.
	i := strings.Index(cp, "fn c() {")
	changed := cp[:i] + strings.Replace(cp[i:], "let a = 1;", "let a = 2;", 1)
	// cprows FILE COPY-FIRST-LINE [ORIG-HINT-TAG] [NAME], as seams.sh's.
	cprows := func(first, hint, name string) string {
		if name == "" {
			name = "w"
		}
		h := "a.rs|hint(a);"
		if hint != "" {
			h += "|" + hint
		}
		return rowsOf("a.rs|fn o() {|"+name+":orig-only", "a.rs|let a = 1;|"+name+":orig", h, "a.rs|a|"+name+":orig", "a.rs|}|"+name+":orig", "a.rs|...",
			"a.rs|fn c() {|w:copy-only", "a.rs|"+first+"|w:copy", "a.rs|a|w:copy", "a.rs|}|w:copy")
	}
	okRows := cprows("let a = 1;", "w:orig-only", "")
	if got := pinsOut(t, map[string]string{"a.rs": cp, "p.txt": okRows}); len(got) != 0 {
		t.Errorf("a matching copy: got %q", got)
	}
	// The changed copy's pins pass; copies fails it.
	d := tree(t, map[string]string{"a.rs": changed, "p.txt": cprows("let a = 2;", "w:orig-only", "")})
	rows, errs, _ := readPins(d, "p.txt")
	if got := append(errs, pinsHeld(d, "p.txt", rows)...); len(got) != 0 {
		t.Errorf("the changed copy's pins: got %q", got)
	}
	if got := copiesMatch("p.txt", rows); !reflect.DeepEqual(got, []string{"seams: p.txt: w: the copy has let a = 2; where the original has let a = 1;"}) {
		t.Errorf("the changed copy: got %q", got)
	}
	copygap := strings.Replace(okRows, "a.rs\ta\tw:copy\n", "a.rs\ta\n", 1)
	notRole := " is not NAME:orig, NAME:orig-only, NAME:copy or NAME:copy-only"
	for _, c := range []struct {
		name, rows string
		want       []string // the whole output
	}{
		{"unmarked", cprows("let a = 1;", "", ""), []string{"seams: p.txt: w: the orig rows are not consecutive"}},
		{"copy gap", copygap, []string{"seams: p.txt: w: the copy rows are not consecutive", "seams: p.txt: w: the copy has 2 shared rows, the original 3"}},
		{"role", cprows("let a = 1;", "w:body", ""), []string{"seams: p.txt: row 3: w:body" + notRole, "seams: p.txt: w: the orig rows are not consecutive"}},
		// A NAME with no rows of one side fails that alone, not again
		// for its shared rows.
		{"no copy", cprows("let a = 1;", "v:orig-only", "v"), []string{"seams: p.txt: v has no copy rows", "seams: p.txt: w has no orig rows"}},
		{"no orig", rowsOf("a.rs|fn c() {|u:copy"), []string{"seams: p.txt: u has no orig rows"}},
		{"no name", strings.Replace(okRows, "w:orig-only", ":orig-only", 1), []string{"seams: p.txt: row 1: :orig-only" + notRole}},
		{"no colon", strings.Replace(okRows, "w:orig-only", "orig", 1), []string{"seams: p.txt: row 1: orig" + notRole}},
		{"only -only rows", rowsOf("a.rs|fn o() {|x:orig-only", "a.rs|...", "a.rs|fn c() {|x:copy-only"), []string{"seams: p.txt: x has no NAME:orig rows"}},
		{"count", strings.Replace(okRows, "a.rs\ta\tw:copy\n", "a.rs\ta\tw:copy-only\n", 1), []string{"seams: p.txt: w: the copy has 2 shared rows, the original 3"}},
	} {
		if got := pinsOut(t, map[string]string{"a.rs": cp, "p.txt": c.rows}); !reflect.DeepEqual(got, c.want) {
			t.Errorf("%s: got %q, want %q", c.name, got, c.want)
		}
	}
}

// -check runs the pins checks of each pins line, and a status that names
// a pins file needs a row in it for its source file.
func TestPinsEndToEnd(t *testing.T) {
	rules := "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\nguess\tC\tkind=.\npins\tverus/kani/pins.txt\n"
	src := lines("#[verifier::external_body]", "fn f() {", "    let a = 1;", "}")
	// src/c.rs's status does not name the pins file, so it needs no row.
	files := map[string]string{"verus/seams.rules": rules, "src/a.rs": src, "src/b.rs": src, "src/c.rs": src,
		"verus/ledger/src/c.rs.overrides": "f\tC\tT\ttrusted (no copy)\n",
		"verus/kani/pins.txt":             rowsOf("src/a.rs|fn f() {", "src/a.rs|let a = 1;"),
		"verus/ledger/src/a.rs.overrides": "f\tC\tT\ttrusted (copy pinned in verus/kani/pins.txt)\n",
		"verus/ledger/src/b.rs.overrides": "f\tC\tT\ttrusted (copy pinned in verus/kani/pins.txt)\n"}
	d := tree(t, files)
	if code, _, e := runIn(t, d, "-write"); code != 0 {
		t.Fatalf("-write: %d %s", code, e)
	}
	code, _, e := runIn(t, d, "-check")
	if code != 1 || e != "seams: verus/ledger/src/b.rs.overrides: a status names verus/kani/pins.txt, which has no row for src/b.rs\n" {
		t.Errorf("-check: %d %q", code, e)
	}
	files["verus/kani/pins.txt"] += rowsOf("src/b.rs|let a = 1;")
	d = tree(t, files)
	runIn(t, d, "-write")
	if code, o, e := runIn(t, d, "-check"); code != 0 || e != "" || o != "verus/ledger is up to date\n" {
		t.Errorf("-check: %d %q %q", code, o, e)
	}
	files["src/a.rs"] = strings.Replace(src, "let a = 1;", "let a = 2;", 1)
	d = tree(t, files)
	runIn(t, d, "-write")
	if code, _, e := runIn(t, d, "-check"); code != 1 || e != "seams: verus/kani/pins.txt: src/a.rs no longer has, in place: let a = 1;\n" {
		t.Errorf("-check, a changed line: %d %q", code, e)
	}
	// A pins file that is gone fails once, not again for each status
	// naming it.
	gone := map[string]string{}
	for k, v := range files {
		if k != "verus/kani/pins.txt" {
			gone[k] = v
		}
	}
	d = tree(t, gone)
	runIn(t, d, "-write")
	if code, _, e := runIn(t, d, "-check"); code != 1 || e != "seams: verus/kani/pins.txt is gone\n" {
		t.Errorf("-check, the pins file gone: %d %q", code, e)
	}
	// Without a pins line nothing is read.
	files["verus/seams.rules"] = strings.Replace(rules, "pins\tverus/kani/pins.txt\n", "", 1)
	d = tree(t, files)
	runIn(t, d, "-write")
	if code, _, e := runIn(t, d, "-check"); code != 0 || e != "" {
		t.Errorf("-check, no pins line: %d %q", code, e)
	}
}

func TestPinsRules(t *testing.T) {
	base := "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\n"
	for _, c := range []struct{ text, want string }{
		{base + "pins\n", "r:5: want pins<TAB>FILE"},
		{base + "pins\ta\tb\n", "r:5: want pins<TAB>FILE"},
		{base + "pins\t\n", "r:5: want pins<TAB>FILE, with no empty field"},
		{base + "pins\t../p.txt\n", "r:5: pins ../p.txt: want a path inside the tree"},
		{base + "pins\tp.txt\npins\t./p.txt\n", "r:6: pins ./p.txt given twice"},
	} {
		_, err := parseRules("r", c.text)
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("rules %q: want error %q, got %v", c.text, c.want, err)
		}
	}
	r, err := parseRules("r", base+"pins\tverus/./p.txt\npins\tq.txt\n")
	if err != nil || !reflect.DeepEqual(r.Pins, []string{"verus/p.txt", "q.txt"}) {
		t.Fatalf("good rules: %v %+v", err, r)
	}
}
