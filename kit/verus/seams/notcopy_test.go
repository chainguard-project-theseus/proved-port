package main

import (
	"sort"
	"strings"
	"testing"
)

const ncRules = "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\ntest-file\ttests.rs\n" +
	"cite-tag\tKani\ncite-tag\tconst assert\ncite-assert\tconst assert\nnotcopy\tsrc/nc.rs\tconst assert\n"

// The checks file: seams.sh's self-test fixture (nc.rs), with the second
// impls, and the cases the lexer adds.
var ncFile = lines(
	"impl<T: Copy> NotCopy<IfCopy> for T {}",
	"impl<T: Clone> NotClone<IfClone> for T {}",
	"// `F` checked.",
	"const _: () = assert!(<F as NotCopy<_>>::HOLDS && <F as NotClone<_>>::HOLDS);",
	"// `path` checked.",
	"const _: () = assert!(",
	"    <crate::m::F as NotCopy<_>>::HOLDS && <crate::m::F as NotClone<_>>::HOLDS",
	");",
	"// `G` another type.",
	"const _: () = assert!(<G as NotCopy<_>>::HOLDS && <G as NotClone<_>>::HOLDS);",
	"// `FF` a longer name.",
	"const _: () = assert!(<FF as NotCopy<_>>::HOLDS && <FF as NotClone<_>>::HOLDS);",
	"// `copyonly` Copy alone.",
	"const _: () = assert!(<F as NotCopy<_>>::HOLDS);",
	"// `comment` in a comment.",
	"const _: () = assert!(",
	"    true // <F as NotCopy<_>>::HOLDS && <F as NotClone<_>>::HOLDS",
	");",
	"// `block` in a block comment.",
	"const _: () = assert!(true /* <F as NotCopy<_>>::HOLDS && <F as NotClone<_>>::HOLDS */);",
	"// `split` over three lines.",
	"const _: () = assert!(",
	"    <F as NotCopy<_>>::HOLDS",
	"        && <F as NotClone<_>>::HOLDS",
	");",
	"// `dup` once.",
	"const _: () = assert!(<F as NotCopy<_>>::HOLDS && <F as NotClone<_>>::HOLDS);",
	"// `dup` twice.",
	"const _: () = assert!(<F as NotCopy<_>>::HOLDS && <F as NotClone<_>>::HOLDS);",
	"// `mixed` NotCopy of a longer name.",
	"const _: () = assert!(<FF as NotCopy<_>>::HOLDS && <F as NotClone<_>>::HOLDS);",
	"// `cloneonly` Clone alone.",
	"const _: () = assert!(<F as NotClone<_>>::HOLDS);",
	"// `W` an assertion, but no not-Copy check.",
	"const _: () = assert!(true);",
	"// `K` checked at an argument.",
	"const _: () = assert!(<K<u8> as NotCopy<_>>::HOLDS && <K<u8> as NotClone<_>>::HOLDS);",
	"#[cfg(any())]",
	"// `Q` gated.",
	"const _: () = assert!(<Q as NotCopy<_>>::HOLDS && <Q as NotClone<_>>::HOLDS);",
	"/*",
	"// `R` in a block comment.",
	"const _: () = assert!(<R as NotCopy<_>>::HOLDS && <R as NotClone<_>>::HOLDS);",
	"*/")

// notCopy is seams.sh's notcopy(): the cited assertion checks the type.
func TestNotCopy(t *testing.T) {
	r, err := parseRules("r", ncRules)
	if err != nil {
		t.Fatal(err)
	}
	noImpls := strings.Join(strings.Split(ncFile, "\n")[2:], "\n")
	implsInComment := "/*\n" + strings.Join(strings.Split(ncFile, "\n")[:2], "\n") + "\n*/\n" + noImpls
	impls := strings.Split(ncFile, "\n")[:2]
	one := lines(impls[0], impls[1], "// `F` checked.", "const _: () = assert!(<F as NotCopy<_>>::HOLDS && <F as NotClone<_>>::HOLDS);")
	d := tree(t, map[string]string{"src/nc.rs": ncFile, "src/nc0.rs": noImpls, "src/nc1.rs": implsInComment, "src/notes.md": ncFile,
		"src/one.rs":   one,
		"src/open.rs":  one + "/* never closed\n",
		"src/clone.rs": strings.Replace(one, impls[0]+"\n", "", 1),
		"src/copy.rs":  strings.Replace(one, impls[1]+"\n", "", 1),
		// The second impls under a cfg the build may not meet, directly
		// or across a comment and a blank line, are none.
		"src/gated.rs":  "#[cfg(any())]\n" + one,
		"src/gated2.rs": strings.Replace(one, impls[1]+"\n", "#[cfg(any())]\n// note\n\n"+impls[1]+"\n", 1),
		// The check, or the impls, in a block (at column 0) are not the
		// file's top level.
		"src/blk.rs":     lines(impls[0], impls[1], "mod blk {", "// `F` checked.", "const _: () = assert!(<F as NotCopy<_>>::HOLDS && <F as NotClone<_>>::HOLDS);", "}"),
		"src/blkimpl.rs": "mod m {\n" + impls[0] + "\n" + impls[1] + "\n}\n" + strings.Join(strings.Split(one, "\n")[2:], "\n")})
	v := &resolver{root: d, r: r, files: map[string]*srcFile{}}
	for _, c := range []struct {
		ty, cite string
		want     bool
	}{
		{"F", "`F` in src/nc.rs", true},
		{"F", "`path` in src/nc.rs", true},
		{"K", "`K` in src/nc.rs", true},
		{"F", "`G` in src/nc.rs", false},        // another type
		{"F", "`FF` in src/nc.rs", false},       // a longer name
		{"F", "`copyonly` in src/nc.rs", false}, // Copy alone
		{"F", "`comment` in src/nc.rs", false},  // in a line comment
		{"F", "`block` in src/nc.rs", false},    // in a block comment
		{"R", "`R` in src/nc.rs", false},        // the whole check in a block comment
		{"F", "`F` in src/nc0.rs", false},       // no second impls
		{"F", "`F` in src/nc1.rs", false},       // the second impls in a block comment
		{"F", "`F` in src/notes.md", false},     // not a crate .rs file
		{"F", " in src/nc.rs", false},           // a blank label
		{"F", "`F`", false},                     // no FILE
		{"F", "`split` in src/nc.rs", true},     // over three lines
		{"F", "`dup` in src/nc.rs", false},      // two under the label
		{"F", "`mixed` in src/nc.rs", false},    // NotCopy of FF
		{"F", "`cloneonly` in src/nc.rs", false},
		{"F", "`F` in src/one.rs", true},
		{"F", " in src/one.rs", false},        // a blank label, though one assertion is labeled
		{"F", "`F` in src/open.rs", false},    // the file ends inside a comment
		{"F", "`F` in src/clone.rs", false},   // only the NotClone impl
		{"F", "`F` in src/copy.rs", false},    // only the NotCopy impl
		{"F", "`F` in src/gated.rs", false},   // the NotCopy impl gated
		{"F", "`F` in src/gated2.rs", false},  // the NotClone impl gated across a comment and a blank line
		{"F", "`F` in src/blk.rs", false},     // the check in a block
		{"F", "`F` in src/blkimpl.rs", false}, // the impls in a block
	} {
		if got := v.notCopy(c.ty, c.cite); got != c.want {
			t.Errorf("notCopy(%s, %s) = %v, want %v", c.ty, c.cite, got, c.want)
		}
	}
}

// Every tracked type needs its check (seams.sh's trackedcheck): whatever
// its visibility or indentation, a generic one checked at an argument;
// a check gated by an attribute is none; a second type of a name fails;
// neither an untracked struct nor one in a comment or literal needs one.
func TestTrackedCheck(t *testing.T) {
	r, err := parseRules("r", ncRules)
	if err != nil {
		t.Fatal(err)
	}
	files := map[string]string{
		"src/nc.rs":  ncFile,
		"src/lib.rs": "mod nc;\n",
		"Cargo.toml": "[package]\nname = \"t\"\n",
		"src/a.rs": lines("pub tracked struct F { x: () }", "    tracked struct K<T> { x: T }",
			"pub(crate) tracked enum E { A }", "    pub(in crate::m) tracked struct P { x: () }",
			"tracked struct M { x: () }", "tracked struct Q { x: () }", "tracked struct W { x: () }",
			"#[verifier::reject_recursive_types(T)] pub tracked struct Tok<T> { x: T }",
			// Every declaration on a line; one whose name a macro writes
			// cannot be checked by name.
			"tracked struct A2 { x: () } tracked enum B2 { X }",
			"macro_rules! token { ($n:ident) => { pub tracked struct $n { _p: () } } }",
			"// tracked struct C, in a comment", "/*", "tracked struct D { x: () }", "*/",
			"const S: &str = \"", "tracked struct L { x: () }", "\";", "pub struct U;"),
		"src/b.rs":     lines("tracked struct F { x: () }"),
		"src/tests.rs": lines("tracked struct N { x: () }"),
	}
	d := tree(t, files)
	got := notCopyChecks(d, r, nil, []string{"src/a.rs", "src/b.rs", "src/nc.rs", "src/tests.rs"})
	var tys []string
	for _, l := range got {
		if strings.Contains(l, " (src/") {
			tys = append(tys, strings.Fields(strings.TrimPrefix(l, "seams: src/nc.rs: "))[0])
		}
	}
	sort.Strings(tys)
	if strings.Join(tys, " ") != "A2 B2 E F M N P Q Tok W" {
		t.Errorf("failing types %q, want A2 B2 E F M N P Q Tok W:\n%s", tys, strings.Join(got, "\n"))
	}
	if w := "seams: src/nc.rs: src/a.rs:10: a tracked type whose name the scan cannot read (a macro-written name such as `$n`, or the name on a later line)"; !strings.Contains(strings.Join(got, "\n"), w) {
		t.Errorf("want %q in:\n%s", w, strings.Join(got, "\n"))
	}
	want := []string{
		"seams: src/nc.rs: E (src/a.rs:3): a tracked type with no check in src/nc.rs that it is neither Copy nor Clone (`<E as NotCopy<_>>::HOLDS` and `<E as NotClone<_>>::HOLDS` under a `// `E` ...` comment, with no attribute above it)",
		"seams: src/nc.rs: F (src/b.rs:1): a second tracked type of this name, which the one check of the name cannot stand for: rename one",
	}
	for _, w := range want {
		if !strings.Contains(strings.Join(got, "\n"), w) {
			t.Errorf("want %q in:\n%s", w, strings.Join(got, "\n"))
		}
	}
	// No notcopy line, no checks.
	r2, _ := parseRules("r", strings.Replace(ncRules, "notcopy\tsrc/nc.rs\tconst assert\n", "", 1))
	if got := notCopyChecks(d, r2, nil, []string{"src/a.rs"}); len(got) != 0 {
		t.Errorf("no notcopy line: %q", got)
	}
}

// A tracked-type seam that cites the tag cites its type's check among
// them (seams.sh's notcopycheck); an untracked one, or one citing no
// assertion, is not held to it.
func TestNotCopyCite(t *testing.T) {
	r, err := parseRules("r", ncRules)
	if err != nil {
		t.Fatal(err)
	}
	d := tree(t, map[string]string{"src/nc.rs": ncFile, "src/lib.rs": "mod nc;\n", "Cargo.toml": "[package]\nname = \"t\"\n"})
	tracked := "#[verifier::external_body] pub tracked struct F { _no_copy: () }"
	untracked := "#[verifier::external_body] pub struct F { _no_copy: () }"
	for _, c := range []struct {
		hc, cites string
		fail      bool
	}{
		{tracked, "`F` in src/nc.rs", false},
		{tracked, "`path` in src/nc.rs", false},
		{tracked, "`G` in src/nc.rs and `F` in src/nc.rs", false},
		{tracked, "`F` in src/nc.rs and `G` in src/nc.rs", false},
		{tracked, "`G` in src/nc.rs", true},
		{tracked, "`FF` in src/nc.rs", true},
		{tracked, "`copyonly` in src/nc.rs", true},
		{tracked, "`comment` in src/nc.rs", true},
		{untracked, "`G` in src/nc.rs", false},
	} {
		e := &entry{file: "src/t.rs", line: 3, okey: "src/t.rs:F", hc: c.hc, stat: "trusted (partly checked, const assert: " + c.cites + "; not checked: x)"}
		got := notCopyChecks(d, r, []*entry{e}, nil)
		want := "seams: verus/ledger/src/t.rs.overrides: F (src/t.rs:3): a tracked type, whose status cites no const assert that checks it is neither Copy nor Clone (`<F as NotCopy<_>>::HOLDS` and `<F as NotClone<_>>::HOLDS`, src/nc.rs)"
		if c.fail != (len(got) == 1 && got[0] == want) || (!c.fail && len(got) != 0) {
			t.Errorf("%s / %s: got %q", c.hc, c.cites, got)
		}
	}
	// Only the tag's citations count: another tag naming the check does
	// not stand for it.
	e0 := &entry{file: "src/t.rs", line: 3, okey: "src/t.rs:F", hc: tracked, stat: "trusted (partly checked, Kani: `F` in src/nc.rs; const assert: `G` in src/nc.rs; not checked: x)"}
	if got := notCopyChecks(d, r, []*entry{e0}, nil); len(got) != 1 {
		t.Errorf("another tag: %q", got)
	}
	// A status citing no assertion is not held to it here.
	e := &entry{file: "src/t.rs", line: 3, okey: "src/t.rs:F", hc: tracked, stat: "trusted"}
	if got := notCopyChecks(d, r, []*entry{e}, nil); len(got) != 0 {
		t.Errorf("trusted: %q", got)
	}
}

// -check runs the checks over the source and test files.
func TestNotCopyEndToEnd(t *testing.T) {
	d := tree(t, map[string]string{"verus/seams.rules": ncRules + "guess\tC\tkind=.\n", "src/nc.rs": ncFile, "src/lib.rs": "mod a;\nmod nc;\n", "Cargo.toml": "[package]\nname = \"t\"\n",
		"src/a.rs":     lines("verus! {", "#[verifier::external_body]", "pub tracked struct F { x: () }", "}"),
		"src/tests.rs": lines("tracked struct N { x: () }")})
	if code, _, e := runIn(t, d, "-write"); code != 0 {
		t.Fatalf("-write: %d %s", code, e)
	}
	code, _, e := runIn(t, d, "-check")
	if code != 1 || !strings.Contains(e, "seams: src/nc.rs: N (src/tests.rs:1): a tracked type with no check") || strings.Contains(e, ": F (") {
		t.Errorf("-check: %d %s", code, e)
	}
}

func TestNotCopyRules(t *testing.T) {
	base := "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\ncite-tag\tK\ncite-assert\tK\ncite-tag\tL\n"
	for _, c := range []struct{ text, want string }{
		{base + "notcopy\tsrc/nc.rs\n", "r:8: want notcopy<TAB>FILE<TAB>TAG"},
		{base + "notcopy\tsrc/nc.rs\tL\n", "r:8: notcopy L: not a tag with a cite-assert line above it"},
		{base + "notcopy\tsrc/nc.rs\tX\n", "r:8: notcopy X: not a tag with a cite-assert line above it"},
		{base + "notcopy\t../nc.rs\tK\n", "r:8: notcopy ../nc.rs: want a path inside the tree"},
		{base + "notcopy\tsrc/nc.rs\tK\nnotcopy\tsrc/nd.rs\tK\n", "r:9: notcopy given twice"},
	} {
		_, err := parseRules("r", c.text)
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("rules %q: want error %q, got %v", c.text, c.want, err)
		}
	}
	r, err := parseRules("r", base+"notcopy\tsrc/./nc.rs\tK\n")
	if err != nil || r.NotCopyFile != "src/nc.rs" || r.NotCopyTag != "K" {
		t.Errorf("good rules: %v %+v", err, r)
	}
}
