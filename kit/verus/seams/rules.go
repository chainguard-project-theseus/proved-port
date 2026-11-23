package main

import (
	"fmt"
	"os"
	"path"
	"regexp"
	"slices"
	"sort"
	"strings"
)

// Rules is a port's rules file: what seams.sh hard-coded for challoc.
type Rules struct {
	Ledger      string   // ledger directory, relative to the root
	Source      string   // source directory scanned
	Command     string   // the command the ledger header names
	SeamsDoc    string   // the SEAMS.md each ledger file links to
	TestFiles   []string // find-style patterns of test files
	Tools       []string // the tools, in table order
	Categories  []string // the categories, in table order
	ToolOf      map[string]string
	Guesses     []Guess
	PairMacro   string // the export macro (export-macro or pair), or ""
	PairFile    string
	PairForm    string   // "slash" (`fn a / b(`) or "list" (`fn f [a, b](`)
	PairVars    []string // one template VAR for every name, or one per name
	UnsupDoc    string
	CiteTags    []string            // tags a status cites a check with (`Kani: ...`)
	Partial     map[string]bool     // cite tags a discharged status may not cite
	CiteFn      map[string]string   // tag -> the attribute its fn carries (cite-fn)
	CiteFnCfg   map[string][]string // tag -> the `#[cfg(CFG)]` lines its fn may carry
	CiteAssert  map[string]bool     // tags whose citations are labeled assertions
	CiteRow     map[string]citeRow  // tag -> the table row each citation needs
	CiteTest    map[string]citeTest // tag -> the test crate its citations name tests in
	CiteCase    map[string]string   // tag -> the directory whose cases file its citations name cases in
	CiteModule  map[string]string   // a path's first segment -> the directory it reads
	NotCopyFile string              // the file of the tracked types' not-Copy checks, or ""
	NotCopyTag  string              // the cite-assert tag a tracked-type seam cites them with
	Pins        []string            // the pins files (source lines a copy runs), in rules order
	Assumption  string              // the tool of a seam no check applies to
	VulnClasses string              // the vulnerability-class catalog -check reads, or ""
	testRes     []*regexp.Regexp
	testOnPath  []bool
}

// Guess is one `guess` line: the category when every condition holds.
type Guess struct {
	Category string // "" for `-`
	Conds    []Cond
}

// Cond is `field=RE` (matches) or `field!RE` (does not match).
type Cond struct {
	Field  string
	Negate bool
	Re     *regexp.Regexp
}

func (g Guess) holds(e *entry) bool {
	for _, c := range g.Conds {
		var v string
		switch c.Field {
		case "kind":
			v = e.kind
		case "name":
			v = e.name
		case "file":
			v = e.file
		case "body":
			v = e.body
		}
		if c.Re.MatchString(v) == c.Negate {
			return false
		}
	}
	return true
}

func (r *Rules) guess(e *entry) string {
	for _, g := range r.Guesses {
		if g.holds(e) {
			return g.Category
		}
	}
	return ""
}

// citesCheck says whether a status cites a check: it holds `TAG: ` for
// one of the rules' cite tags.
func (r *Rules) citesCheck(stat string) bool {
	for _, t := range r.CiteTags {
		if strings.Contains(stat, t+": ") {
			return true
		}
	}
	return false
}

// toolOf is the default tool of a category, "?" for none.
func (r *Rules) toolOf(cat string) string {
	if t, ok := r.ToolOf[cat]; ok {
		return t
	}
	return "?"
}

// globRe turns a find-style pattern (`*` and `?` match `/` too, as in
// find -path) into an anchored regular expression.
func globRe(p string) (*regexp.Regexp, error) {
	var b strings.Builder
	b.WriteString("^")
	for i := 0; i < len(p); i++ {
		switch c := p[i]; c {
		case '*':
			b.WriteString(".*")
		case '?':
			b.WriteString(".")
		case '[':
			j := strings.IndexByte(p[i+1:], ']')
			if j < 0 {
				return nil, fmt.Errorf("unclosed [ in %q", p)
			}
			cls := p[i+1 : i+1+j]
			if strings.HasPrefix(cls, "!") {
				cls = "^" + cls[1:]
			}
			b.WriteString("[" + cls + "]")
			i += j + 1
		default:
			b.WriteString(regexp.QuoteMeta(string(c)))
		}
	}
	b.WriteString("$")
	return regexp.Compile(b.String())
}

// isTest says whether the source file p (a path from the root, under
// Source) is a test file: a pattern with a `/` is matched against the
// whole path, any other against the base name.
func (r *Rules) isTest(p string) bool {
	for i, t := range r.testRes {
		s := path.Base(p)
		if r.testOnPath[i] {
			s = p
		}
		if t.MatchString(s) {
			return true
		}
	}
	return false
}

var reIdent = regexp.MustCompile(`^[A-Za-z_][A-Za-z0-9_]*$`)

var guessFields = map[string]bool{"kind": true, "name": true, "file": true, "body": true}

// readRules reads a rules file; the errors name its lines.
func readRules(file string) (*Rules, error) {
	data, err := os.ReadFile(file)
	if err != nil {
		return nil, fmt.Errorf("cannot read the rules file: %v", err)
	}
	return parseRules(file, string(data))
}

func parseRules(file, data string) (*Rules, error) {
	r := &Rules{Ledger: "verus/ledger", Source: "src", SeamsDoc: "verus/SEAMS.md", ToolOf: map[string]string{}, Partial: map[string]bool{},
		CiteFn: map[string]string{}, CiteFnCfg: map[string][]string{}, CiteAssert: map[string]bool{}, CiteRow: map[string]citeRow{}, CiteTest: map[string]citeTest{}, CiteCase: map[string]string{}, CiteModule: map[string]string{}}
	tools := map[string]bool{}
	seen := map[string]bool{}
	var errs []string
	lines := strings.Split(data, "\n")
	if len(lines) > 0 && lines[len(lines)-1] == "" {
		lines = lines[:len(lines)-1]
	}
	for ln, l := range lines {
		bad := func(format string, a ...any) {
			errs = append(errs, fmt.Sprintf("%s:%d: %s", file, ln+1, fmt.Sprintf(format, a...)))
		}
		if strings.HasPrefix(l, "#") || strings.TrimSpace(l) == "" {
			continue
		}
		f := strings.Split(l, "\t")
		want := func(n int, form string) bool {
			if len(f) != n {
				bad("want %s", form)
				return false
			}
			for _, x := range f[1:] {
				if x == "" {
					bad("want %s, with no empty field", form)
					return false
				}
			}
			return true
		}
		// A rule that holds one value may be given once: a second
		// would silently replace the first.
		switch f[0] {
		case "ledger", "source", "command", "seams-doc", "unsupported-doc", "assumption-tool", "pair", "export-macro", "vuln-classes":
			// pair is the slash form of export-macro: the two name
			// the one macro the scan reads, so either is given once.
			k := f[0]
			if k == "pair" {
				k = "export-macro"
			}
			if seen[k] {
				bad("%s given twice", k)
				continue
			}
			seen[k] = true
		}
		// A path is relative to the root and inside it, and is not the
		// root itself: -write removes every *.rs.md under the ledger.
		inTree := func(p string) (string, bool) {
			c := path.Clean(p)
			if path.IsAbs(c) || c == "." || c == ".." || strings.HasPrefix(c, "../") {
				bad("%s %s: want a path inside the tree, below its root", f[0], p)
				return "", false
			}
			return c, true
		}
		switch f[0] {
		case "ledger":
			if want(2, "ledger<TAB>DIR") {
				if c, ok := inTree(f[1]); ok {
					r.Ledger = c
				}
			}
		case "source":
			if want(2, "source<TAB>DIR") {
				if c, ok := inTree(f[1]); ok {
					r.Source = c
				}
			}
		case "command":
			if want(2, "command<TAB>TEXT") {
				r.Command = f[1]
			}
		case "seams-doc":
			if want(2, "seams-doc<TAB>PATH") {
				if c, ok := inTree(f[1]); ok {
					r.SeamsDoc = c
				}
			}
		case "unsupported-doc":
			if want(2, "unsupported-doc<TAB>TEXT") {
				r.UnsupDoc = f[1]
			}
		case "cite-tag":
			form := "cite-tag<TAB>TAG[<TAB>partial]"
			if len(f) == 3 && f[2] != "partial" {
				bad("want %s, not %q in the third field", form, f[2])
				continue
			}
			if !want(min(max(len(f), 2), 3), form) {
				continue
			}
			// A citation runs from `TAG: ` to the first `;` or `)`, and a
			// part holding `: ` starts the next tag: a tag holding any of
			// them could not be read back.
			if strings.ContainsAny(f[1], ";):") {
				bad("cite-tag %s holds `;`, `)` or `:`, which end a citation", f[1])
				continue
			}
			if slices.Contains(r.CiteTags, f[1]) {
				bad("cite-tag %s is given twice", f[1])
				continue
			}
			r.CiteTags = append(r.CiteTags, f[1])
			if len(f) == 3 {
				r.Partial[f[1]] = true
			}
		case "cite-fn", "cite-assert", "cite-row", "cite-test", "cite-case":
			forms := map[string]string{"cite-fn": "cite-fn<TAB>TAG<TAB>ATTR[<TAB>CFG...]", "cite-assert": "cite-assert<TAB>TAG", "cite-row": "cite-row<TAB>TAG<TAB>FILE<TAB>HEADING<TAB>PART...", "cite-test": "cite-test<TAB>TAG<TAB>DIR[<TAB>CFG...]", "cite-case": "cite-case<TAB>TAG<TAB>DIR"}
			n := map[string]int{"cite-fn": max(len(f), 3), "cite-assert": 2, "cite-row": max(len(f), 4), "cite-test": max(len(f), 3), "cite-case": 3}[f[0]]
			if !want(n, forms[f[0]]) {
				continue
			}
			if !slices.Contains(r.CiteTags, f[1]) {
				bad("%s %s: not a cite-tag listed above it", f[0], f[1])
				continue
			}
			switch f[0] {
			case "cite-fn", "cite-assert", "cite-test", "cite-case":
				_, fn := r.CiteFn[f[1]]
				_, test := r.CiteTest[f[1]]
				_, cs := r.CiteCase[f[1]]
				if fn || test || cs || r.CiteAssert[f[1]] {
					bad("%s %s: the tag already has a cite-fn, cite-assert, cite-test or cite-case line", f[0], f[1])
					continue
				}
				if f[0] == "cite-case" {
					if c, ok := inTree(f[2]); ok {
						r.CiteCase[f[1]] = c
					}
					continue
				}
				if f[0] == "cite-test" {
					c, ok := inTree(f[2])
					if !ok {
						continue
					}
					ct := citeTest{dir: c}
					for _, g := range f[3:] {
						// The `mod` line's attributes are compared lexed,
						// which blanks a literal's contents, so a CFG holding
						// one could stand for another of its length.
						if strings.ContainsAny(g, "\"'") {
							bad("cite-test %s: CFG %s holds a literal, which the check cannot compare", f[1], g)
							ok = false
							break
						}
						ct.cfgs = append(ct.cfgs, "#[cfg("+g+")]")
					}
					if ok {
						r.CiteTest[f[1]] = ct
					}
					continue
				}
				if f[0] == "cite-assert" {
					r.CiteAssert[f[1]] = true
					continue
				}
				if strings.ContainsAny(f[2], "[] ") {
					bad("cite-fn %s: ATTR %s: want the attribute's path without `#[`, `]` or blanks", f[1], f[2])
					continue
				}
				r.CiteFn[f[1]] = f[2]
				// Each CFG allows the one attribute `#[cfg(CFG)]`, whole
				// on its line, on the fn: the cfg the tool builds under.
				for _, c := range f[3:] {
					r.CiteFnCfg[f[1]] = append(r.CiteFnCfg[f[1]], "#[cfg("+c+")]")
				}
			case "cite-row":
				if _, ok := r.CiteRow[f[1]]; ok {
					bad("cite-row %s is given twice", f[1])
					continue
				}
				if c, ok := inTree(f[2]); ok {
					r.CiteRow[f[1]] = citeRow{file: c, heading: f[3], parts: f[4:]}
				}
			}
		case "notcopy":
			if !want(3, "notcopy<TAB>FILE<TAB>TAG") {
				continue
			}
			if r.NotCopyFile != "" {
				bad("notcopy given twice")
				continue
			}
			if !r.CiteAssert[f[2]] {
				bad("notcopy %s: not a tag with a cite-assert line above it", f[2])
				continue
			}
			if c, ok := inTree(f[1]); ok {
				r.NotCopyFile, r.NotCopyTag = c, f[2]
			}
		case "vuln-classes":
			if !want(2, "vuln-classes<TAB>FILE") {
				continue
			}
			if c, ok := inTree(f[1]); ok {
				r.VulnClasses = c
			}
		case "pins":
			if !want(2, "pins<TAB>FILE") {
				continue
			}
			c, ok := inTree(f[1])
			if !ok {
				continue
			}
			if slices.Contains(r.Pins, c) {
				bad("pins %s given twice", f[1])
				continue
			}
			r.Pins = append(r.Pins, c)
		case "cite-module":
			if !want(3, "cite-module<TAB>NAME<TAB>DIR") {
				continue
			}
			if name := f[1]; !reIdent.MatchString(name) {
				bad("cite-module %s: want an identifier", name)
				continue
			}
			if _, ok := r.CiteModule[f[1]]; ok {
				bad("cite-module %s is given twice", f[1])
				continue
			}
			if c, ok := inTree(f[2]); ok {
				r.CiteModule[f[1]] = c
			}
		case "assumption-tool":
			if want(2, "assumption-tool<TAB>TOOL") {
				if !tools[f[1]] {
					bad("assumption-tool %s is not listed above it", f[1])
					continue
				}
				r.Assumption = f[1]
			}
		case "test-file":
			if want(2, "test-file<TAB>PATTERN") {
				g, err := globRe(f[1])
				if err != nil {
					bad("%v", err)
					continue
				}
				r.TestFiles = append(r.TestFiles, f[1])
				r.testRes = append(r.testRes, g)
				r.testOnPath = append(r.testOnPath, strings.Contains(f[1], "/"))
			}
		case "tool":
			if want(2, "tool<TAB>NAME") {
				if tools[f[1]] {
					bad("tool %s listed twice", f[1])
					continue
				}
				tools[f[1]] = true
				r.Tools = append(r.Tools, f[1])
			}
		case "category":
			if want(3, "category<TAB>NAME<TAB>TOOL") {
				if _, ok := r.ToolOf[f[1]]; ok {
					bad("category %s listed twice", f[1])
					continue
				}
				if !tools[f[2]] {
					bad("category %s: tool %s is not listed above it", f[1], f[2])
					continue
				}
				r.Categories = append(r.Categories, f[1])
				r.ToolOf[f[1]] = f[2]
			}
		case "guess":
			if len(f) < 3 {
				bad("want guess<TAB>CATEGORY|-<TAB>COND...")
				continue
			}
			g := Guess{Category: f[1]}
			if g.Category == "-" {
				g.Category = ""
			} else if _, ok := r.ToolOf[g.Category]; !ok {
				bad("guess: category %s is not listed above it", f[1])
				continue
			}
			ok := true
			for _, c := range f[2:] {
				i := strings.IndexAny(c, "=!")
				if i < 0 || !guessFields[c[:i]] {
					bad("guess: want a condition FIELD=RE or FIELD!RE with FIELD kind, name, file or body, got %q", c)
					ok = false
					break
				}
				x, err := regexp.Compile(c[i+1:])
				if err != nil {
					bad("guess: %v", err)
					ok = false
					break
				}
				g.Conds = append(g.Conds, Cond{Field: c[:i], Negate: c[i] == '!', Re: x})
			}
			if ok {
				r.Guesses = append(r.Guesses, g)
			}
		case "pair":
			if want(5, "pair<TAB>MACRO<TAB>FILE<TAB>VAR<TAB>VAR") {
				if c, ok := inTree(f[2]); ok {
					r.PairMacro, r.PairFile, r.PairForm = f[1], c, "slash"
					r.PairVars = []string{f[3], f[4]}
				}
			}
		case "export-macro":
			if len(f) < 5 {
				bad("want export-macro<TAB>MACRO<TAB>FILE<TAB>slash|list<TAB>VAR...")
				continue
			}
			if !want(len(f), "export-macro<TAB>MACRO<TAB>FILE<TAB>slash|list<TAB>VAR...") {
				continue
			}
			if !reIdent.MatchString(f[1]) {
				bad("export-macro %s: want a macro name (an identifier, no path or !)", f[1])
				continue
			}
			if f[3] != "slash" && f[3] != "list" {
				bad("export-macro %s: form %s: want slash or list", f[1], f[3])
				continue
			}
			// slash names two exports: one VAR for both or one each.
			if f[3] == "slash" && len(f) > 6 {
				bad("export-macro %s: the slash form names two exports: want one VAR or two", f[1])
				continue
			}
			vok := true
			for _, v := range f[4:] {
				if !reIdent.MatchString(v) {
					bad("export-macro %s: VAR %s: want an identifier (the name after $ in the definition's `fn $VAR(`)", f[1], v)
					vok = false
					break
				}
			}
			if !vok {
				continue
			}
			if c, ok := inTree(f[2]); ok {
				r.PairMacro, r.PairFile, r.PairForm = f[1], c, f[3]
				r.PairVars = append([]string(nil), f[4:]...)
			}
		default:
			bad("unknown rule %q", f[0])
		}
	}
	// A cite-module DIR is exempt from the reach check, so it may not be
	// SOURCE or hold it.
	var mods []string
	for name := range r.CiteModule {
		mods = append(mods, name)
	}
	sort.Strings(mods)
	for _, name := range mods {
		if d := r.CiteModule[name]; strings.HasPrefix(r.Source+"/", d+"/") {
			errs = append(errs, fmt.Sprintf("%s: cite-module %s: DIR %s is the source directory or holds it", file, name, d))
		}
	}
	if r.Command == "" {
		errs = append(errs, file+": no command line (the command that writes the ledger, for its header)")
	}
	if r.Assumption == "" {
		errs = append(errs, file+": no assumption-tool line (the tool of a seam no check applies to)")
	}
	if len(r.Categories) == 0 {
		errs = append(errs, file+": no category lines")
	}
	if len(errs) > 0 {
		return nil, fmt.Errorf("%s", strings.Join(errs, "\n"))
	}
	return r, nil
}
