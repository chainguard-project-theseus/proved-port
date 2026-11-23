package main

import (
	"os"
	"reflect"
	"strings"
	"testing"
)

// vcGood is a catalog in the template's layout whose citations resolve
// against vcTheoremFiles.
var vcGood = strings.Join([]string{
	"# Vulnerability classes",
	"",
	"Rows cite theorems as `ID (#n)`.",
	"",
	"## Catalog",
	"",
	"| id | year | class | upstream | counterpart | ruled out by | sources |",
	"|---|---|---|---|---|---|---|",
	"| CVE-2020-1 | 2020 | Heap over-read | `f` (a.c) | `a` | RD1 (#10), every read in bounds; RD2 (#11); RD1 (#10) again | NVD |",
	"| CVE-2021-2 | 2021 | integer overflow | `g` (b.c) | `b` | partial: SZ1 (#12) for `g`; gap: #20 | NVD \\| fix |",
	"| (class) | 2022 | Unbounded recursion | `h` (c.c) | none | not shipped; Fence (#3) has no digit | NVD |",
	"",
	"## Classes",
	"",
	"### Heap over-read",
	"",
	"**Property.** Every read in bounds (#99 is no theorem).",
	"",
	"**Proved.** RD1 (#10) for `f`,",
	"RD2 (#11) for `g`; re-RD9 (#2) is inside a word.",
	"",
	"A paragraph of its own: XY9 (#4) is no theorem.",
	"",
	"**Gap.** none.",
	"",
	"### Integer overflow",
	"",
	"**Proved.** SZ1 (#12).",
	"**Note.** QQ1 (#5) is no theorem.",
	"**Gap.** gap: #20.",
	"",
	"### Unbounded recursion",
	"**Proved.** none.",
	"**Gap.** the issue in Gaps,",
	"#21, is open: gap: #21",
	"",
	"```",
	"**Gap.** gap: #77",
	"### Not a class",
	"```",
	"",
	"## Gaps",
	"",
	"1. **#20 `vuln class: integer overflow (CVE-2021-2)`.** Prove",
	"   it for every input (#21 here is a continuation line).",
	"   - a nested point on #30 is no item.",
	"2. **#21 `vuln class: unbounded recursion ((class))`.** Prove it.",
	"",
	"## Keeping this current",
	"",
	"### Integer overflow",
	"",
	"- No issue here: a list outside Gaps.",
	"",
}, "\n")

var vcTheoremFiles = map[string]string{
	"verus/theorems/RD1.md": "# RD1. reads\n",
	"verus/theorems/RD2.md": "# RD2. reads\n",
	"verus/theorems/SZ1.md": "# SZ1. sizes\n",
}

func vcOut(t *testing.T, doc string, drop ...string) []string {
	t.Helper()
	files := map[string]string{"verus/VULN-CLASSES.md": doc}
	for k, v := range vcTheoremFiles {
		files[k] = v
	}
	for _, k := range drop {
		delete(files, k)
	}
	return vulnClassChecks(tree(t, files), &Rules{VulnClasses: "verus/VULN-CLASSES.md"})
}

func TestVulnClassesGood(t *testing.T) {
	if got := vcOut(t, vcGood); len(got) != 0 {
		t.Errorf("got %q", got)
	}
	// The theorems are beside the catalog, wherever it is.
	files := map[string]string{"docs/VC.md": vcGood}
	for k, v := range vcTheoremFiles {
		files["docs/theorems/"+strings.TrimPrefix(k, "verus/theorems/")] = v
	}
	if got := vulnClassChecks(tree(t, files), &Rules{VulnClasses: "docs/VC.md"}); len(got) != 0 {
		t.Errorf("docs/VC.md: got %q", got)
	}
	// Cells split at each `|` but an escaped one, the row's own closing
	// `|` dropped.
	for l, want := range map[string][]string{
		"| a | `b` |":     {"a", "`b`"},
		"| a \\| b | c |": {"a | b", "c"},
		"| a | b \\|":     {"a", "b |"},
		"a | b":           {"a", "b"},
	} {
		if got := vcCells(l); !reflect.DeepEqual(got, want) {
			t.Errorf("vcCells(%q) = %q, want %q", l, got, want)
		}
	}
	// No vuln-classes line: nothing is read.
	if got := vulnClassChecks(t.TempDir(), &Rules{}); got != nil {
		t.Errorf("no line: got %q", got)
	}
}

func TestVulnClassesStale(t *testing.T) {
	const f = "seams: verus/VULN-CLASSES.md"
	rep := func(old, new string) string {
		if !strings.Contains(vcGood, old) {
			t.Fatalf("fixture lacks %q", old)
		}
		return strings.Replace(vcGood, old, new, 1)
	}
	for _, c := range []struct {
		name string
		doc  string
		drop []string
		want []string
	}{
		{"theorem gone", vcGood, []string{"verus/theorems/RD2.md"}, []string{
			f + ":9: cites theorem RD2, which has no verus/theorems/RD2.md",
			f + ":19: cites theorem RD2, which has no verus/theorems/RD2.md"}},
		{"theorem renamed in a Proved line", rep("SZ1 (#12).", "SZ9 (#12)."), nil, []string{
			f + ":28: cites theorem SZ9, which has no verus/theorems/SZ9.md"}},
		{"theorem file is a directory", vcGood, nil, nil},
		{"class with no section", rep("| Unbounded recursion |", "| Excessive work |"), nil, []string{
			f + `:11: class "Excessive work" has no ### section under ## Classes`}},
		{"second section", rep("### Unbounded recursion", "### heap over-read"), nil, []string{
			f + `:32: class "heap over-read" has a second section (the first is line 15)`,
			f + `:11: class "Unbounded recursion" has no ### section under ## Classes`}},
		{"gap not listed", rep("2. **#21", "2. **#22"), nil, []string{
			f + ":34: gap: #21 is not an item of ## Gaps",
			f + ":47: Gaps item #22: no row or class section cites gap: #22"}},
		{"gap listed, no longer cited", strings.NewReplacer("**Gap.** gap: #20.", "**Gap.** none.", "; gap: #20 |", " |").Replace(vcGood), nil, []string{
			f + ":44: Gaps item #20: no row or class section cites gap: #20"}},
		{"gap listed twice", rep("2. **#21", "2. **#20"), nil, []string{
			f + ":47: Gaps lists #20 again (first on line 44)",
			f + ":34: gap: #21 is not an item of ## Gaps"}},
		{"item with no issue", rep("2. **#21", "2. **"), nil, []string{
			f + ":47: a Gaps item names no issue (#n) on its first line",
			f + ":34: gap: #21 is not an item of ## Gaps"}},
		{"row cell count", rep("| (class) | 2022 |", "| (class) |"), nil, []string{
			f + ":11: a row has 6 cells, the header 7"}},
		{"row with no class", rep("| Unbounded recursion |", "|  |"), nil, []string{
			f + ":11: a row names no class"}},
		{"no class column", rep("| id | year | class |", "| id | year | kind |"), nil, []string{
			f + ":7: the Catalog table has no class column"}},
		{"no ruled out by column", rep("| ruled out by |", "| ruling |"), nil, []string{
			f + ":7: the Catalog table has no ruled out by column"}},
		{"no separator", rep("|---|---|---|---|---|---|---|\n", ""), nil, []string{
			f + ":8: the Catalog table's header has no separator row under it"}},
		{"second table", rep("\n## Classes", "\n| x |\n|---|\n\n## Classes"), nil, []string{
			f + ":13: a second table under Catalog, which -check does not read"}},
		{"no table", rep("## Catalog", "## Catalogue"), nil, []string{f + ": no table under ## Catalog"}},
		{"a heading ends the table", rep("| NVD |\n\n## Classes", "| NVD |\n### Notes\n| x |\n|---|\n\n## Classes"), nil, []string{
			f + ":13: a second table under Catalog, which -check does not read"}},
		{"an uncited item listed twice", rep("\n## Keeping", "3. **#40** x.\n4. **#40** y.\n\n## Keeping"), nil, []string{
			f + ":49: Gaps lists #40 again (first on line 48)",
			f + ":48: Gaps item #40: no row or class section cites gap: #40"}},
		{"a cell citing nothing read", rep("| `a` | RD1 (#10), every read in bounds; RD2 (#11); RD1 (#10) again |", "| `a` | the fence |"), nil, []string{
			f + ":9: the ruled out by cell cites no theorem `ID (#n)` or `gap: #n` and is neither `not shipped` nor `by construction:`"}},
		{"a by construction cell", rep("| `a` | RD1 (#10), every read in bounds; RD2 (#11); RD1 (#10) again |", "| `a` | by construction: a type the counterpart cannot be given |"), nil, nil},
		{"a theorem citation wrapped in a Proved paragraph", rep("**Proved.** SZ1 (#12).", "**Proved.** SZ9\n(#12)."), nil, []string{
			f + ":28: cites theorem SZ9, which has no verus/theorems/SZ9.md"}},
		{"a gap citation wrapped in a Gap paragraph", rep("**Gap.** gap: #20.", "**Gap.** gap:\n#20."), nil, nil},
		{"a wrapped gap citation not in Gaps", rep("**Gap.** gap: #20.", "**Gap.** gap:\n#22."), nil, []string{
			f + ":30: gap: #22 is not an item of ## Gaps"}},
		{"a header a heading follows", "## Catalog\n| class | ruled out by |\n## Classes\n", nil, []string{
			f + ":3: the Catalog table's header has no separator row under it"}},
		{"a header at the end", "## Catalog\n| class | ruled out by |", nil, []string{
			f + ":3: the Catalog table's header has no separator row under it"}},
		{"a header a fence follows", "## Catalog\n| class | ruled out by |\n```\n```\n", nil, []string{
			f + ":3: the Catalog table's header has no separator row under it"}},
		{"a header and separator, no rows", "## Catalog\n| class | ruled out by |\n|---|---|\n", nil, nil},
		{"a row with no leading pipe", rep("| (class) | 2022 | Unbounded recursion | `h` (c.c) | none | not shipped; Fence (#3) has no digit | NVD |", "(class) | 2022 | Unbounded recursion | `h` (c.c) | none | XX1 (#3) | NVD"), nil, []string{
			f + ":11: cites theorem XX1, which has no verus/theorems/XX1.md"}},
		{"a line of text under the rows", rep("| NVD |\n\n## Classes", "| NVD |\nbar\n\n## Classes"), nil, []string{
			f + ":12: a row has 1 cells, the header 7"}},
		{"a list under the rows", rep("| NVD |\n\n## Classes", "| NVD |\n- a note | x\n\n## Classes"), nil, nil},
		{"a quote under the rows", rep("| NVD |\n\n## Classes", "| NVD |\n> a note | x\n\n## Classes"), nil, nil},
		{"a fence under the rows", rep("| NVD |\n\n## Classes", "| NVD |\n```\nx | y\n```\nz | w\n\n## Classes"), nil, nil},
		{"a section with no Proved paragraph", rep("**Proved.** SZ1 (#12).\n", ""), nil, []string{
			f + ":26: class \"Integer overflow\" has no **Proved.** paragraph (write `**Proved.** none.` when nothing is proved)"}},
		{"a misnamed Gap paragraph", rep("**Gap.** gap: #20.", "**Gap**. gap: #20."), nil, []string{
			f + ":26: class \"Integer overflow\" has no **Gap.** paragraph (write `**Gap.** none.` when there is none)"}},
		{"an id in code font", rep("RD2 (#11); RD1", "`RD2` (#11); RD1"), nil, []string{
			f + ":9: `RD2` (#11: write a theorem citation `ID (#n)`, the id in plain text"}},
		{"an id as a link", rep("RD2 (#11) for `g`;", "[RD2](theorems/RD2.md) (#11) for `g`;"), nil, []string{
			f + ":19: [RD2](theorems/RD2.md) (#11: write a theorem citation `ID (#n)`, the id in plain text"}},
		{"a Proved paragraph citing nothing read", rep("**Proved.** SZ1 (#12).", "**Proved.** the sizes,\nby `SZ1` alone."), nil, []string{
			f + ":28: the **Proved.** paragraph cites no theorem `ID (#n)` and is not `none`"}},
		{"a cell citing only a gap", rep("partial: SZ1 (#12) for `g`; gap: #20", "gap: #20"), nil, nil},
		{"a Proved paragraph of none", rep("**Proved.** SZ1 (#12).", "**Proved.** None."), nil, nil},
		{"a Gap paragraph citing nothing read", rep("**Gap.** none.", "**Gap.** #20 is open."), nil, []string{
			f + ":24: the **Gap.** paragraph cites no `gap: #n` and is not `none`"}},
		{"a Gap paragraph whose citation is on a later line", rep("**Gap.** none.", "**Gap.** the issue\ngap: #20."), nil, nil},
	} {
		if c.name == "theorem file is a directory" {
			files := map[string]string{"verus/VULN-CLASSES.md": vcGood, "verus/theorems/RD1.md/x": "", "verus/theorems/RD2.md": "", "verus/theorems/SZ1.md": ""}
			got := vulnClassChecks(tree(t, files), &Rules{VulnClasses: "verus/VULN-CLASSES.md"})
			want := []string{f + ":9: cites theorem RD1, which has no verus/theorems/RD1.md", f + ":19: cites theorem RD1, which has no verus/theorems/RD1.md"}
			if !reflect.DeepEqual(got, want) {
				t.Errorf("%s: got %q, want %q", c.name, got, want)
			}
			continue
		}
		if got := vcOut(t, c.doc, c.drop...); !reflect.DeepEqual(got, c.want) {
			t.Errorf("%s: got %q, want %q", c.name, got, c.want)
		}
	}
	if got := vulnClassChecks(t.TempDir(), &Rules{VulnClasses: "v.md"}); !reflect.DeepEqual(got, []string{"seams: v.md is gone"}) {
		t.Errorf("gone: got %q", got)
	}
}

// The template's own layout is the one -check reads: its Catalog table
// has the two columns, every section it names is read, and its one
// placeholder row's class has a section only once the port fills both.
func TestVulnClassesTemplate(t *testing.T) {
	b, err := os.ReadFile("../../templates/verus/VULN-CLASSES.md")
	if err != nil {
		t.Fatal(err)
	}
	d := parseVulnClasses("t", string(b))
	// The Gaps item's placeholder names no issue number yet.
	if len(d.errs) != 1 || !strings.HasSuffix(d.errs[0], ": a Gaps item names no issue (#n) on its first line") ||
		d.tableLine == 0 || len(d.classOf) != 1 || len(d.ruledOut) != 1 || len(d.sections) != 1 || len(d.provedAll) != 1 || len(d.gapAll) != 1 {
		t.Fatalf("template read as %+v", d)
	}
	if got := vcTheorems(d.ruledOut[0].text); !reflect.DeepEqual(got, []string(nil)) {
		t.Errorf("the placeholder row cites %q", got)
	}
}

func TestVulnClassesRules(t *testing.T) {
	base := "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\n"
	for _, c := range []struct{ text, want string }{
		{base + "vuln-classes\n", "r:5: want vuln-classes<TAB>FILE"},
		{base + "vuln-classes\ta\tb\n", "r:5: want vuln-classes<TAB>FILE"},
		{base + "vuln-classes\t../v.md\n", "r:5: vuln-classes ../v.md: want a path inside the tree"},
		{base + "vuln-classes\tv.md\nvuln-classes\tw.md\n", "r:6: vuln-classes given twice"},
	} {
		_, err := parseRules("r", c.text)
		if err == nil || !strings.Contains(err.Error(), c.want) {
			t.Errorf("rules %q: want error %q, got %v", c.text, c.want, err)
		}
	}
	r, err := parseRules("r", base+"vuln-classes\tverus/./VULN-CLASSES.md\n")
	if err != nil || r.VulnClasses != "verus/VULN-CLASSES.md" {
		t.Fatalf("good rules: %v %+v", err, r)
	}
}

// -check runs the catalog's checks with a vuln-classes line, and only
// then.
func TestVulnClassesEndToEnd(t *testing.T) {
	rules := "command\tc\ntool\tT\nassumption-tool\tT\ncategory\tC\tT\nguess\tC\tkind=.\nvuln-classes\tverus/VULN-CLASSES.md\n"
	files := map[string]string{"verus/seams.rules": rules, "src/a.rs": "fn f() {}\n", "verus/VULN-CLASSES.md": vcGood}
	for k, v := range vcTheoremFiles {
		files[k] = v
	}
	d := tree(t, files)
	runIn(t, d, "-write")
	if code, o, e := runIn(t, d, "-check"); code != 0 || e != "" || o != "verus/ledger is up to date\n" {
		t.Errorf("-check: %d %q %q", code, o, e)
	}
	delete(files, "verus/theorems/SZ1.md")
	d = tree(t, files)
	runIn(t, d, "-write")
	want := "seams: verus/VULN-CLASSES.md:10: cites theorem SZ1, which has no verus/theorems/SZ1.md\n" +
		"seams: verus/VULN-CLASSES.md:28: cites theorem SZ1, which has no verus/theorems/SZ1.md\n"
	if code, _, e := runIn(t, d, "-check"); code != 1 || e != want {
		t.Errorf("-check, a theorem gone: %d %q", code, e)
	}
	files["verus/seams.rules"] = strings.Replace(rules, "vuln-classes\tverus/VULN-CLASSES.md\n", "", 1)
	d = tree(t, files)
	runIn(t, d, "-write")
	if code, _, e := runIn(t, d, "-check"); code != 0 || e != "" {
		t.Errorf("-check, no vuln-classes line: %d %q", code, e)
	}
}
