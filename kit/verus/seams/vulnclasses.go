package main

import (
	"fmt"
	"os"
	"path"
	"path/filepath"
	"regexp"
	"strings"
)

// vcText is a piece of the catalog to read citations in: a row's ruled
// out by cell, or a line of a class's **Proved.** or **Gap.** paragraph.
type vcText struct {
	line int
	text string
}

// vcGap is an item of the Gaps list and the issue its first line names.
type vcGap struct {
	line  int
	issue string
}

// vulnDoc is what -check reads of a catalog in the template's layout
// (kit/templates/verus/VULN-CLASSES.md).
type vulnDoc struct {
	tableLine int            // the Catalog table's header line, 0 for none
	classOf   []vcText       // each row's class cell
	ruledOut  []vcText       // each row's ruled out by cell
	sections  map[string]int // a class section's name, folded, and its line
	secs      []*vcSection   // the class sections, in order
	provedAll []vcText       // each **Proved.** paragraph whole (its lines joined), at its first line
	gapAll    []vcText       // each **Gap.** paragraph whole, at its first line
	gaps      []vcGap
	errs      []string
}

// vcSection is a class section and whether it has its **Proved.** and
// **Gap.** paragraphs.
type vcSection struct {
	name        string
	line        int
	proved, gap bool
}

var (
	// A theorem is cited `ID (#n)`: ID starts with a capital letter and
	// holds a digit (the digit is checked apart, as RE2 has no
	// lookahead).
	reVCTheorem = regexp.MustCompile(`(?:^|[^A-Za-z0-9_-])([A-Z][A-Za-z0-9_-]*) \(#[0-9]+`)
	reVCGap     = regexp.MustCompile(`gap: #([0-9]+)`)
	reVCIssue   = regexp.MustCompile(`#([0-9]+)`)
	reVCItem    = regexp.MustCompile(`^([0-9]+\.|[-*]) `)
	reVCSep     = regexp.MustCompile(`^:?-+:?$`)
	reVCPara    = regexp.MustCompile(`^\*\*[^*]+\.\*\*`)
	// A theorem id in code font or as a link before ` (#n)`, which the
	// citation form does not read: it fails rather than go unchecked.
	reVCNear = regexp.MustCompile("(`[A-Z][A-Za-z0-9_-]*`|\\[[^]]*\\]\\([^)]*\\)) \\(#[0-9]+")
	// A paragraph that cites nothing says "none".
	reVCNone = regexp.MustCompile(`^\*\*(Proved|Gap)\.\*\* *(?i:none)\.?$`)
)

// vcCells splits a table line into its cells, trimmed: every `|` but an
// escaped `\|` separates cells (as GitHub reads a table, code spans
// included), and the line's leading and trailing `|` close the row.
func vcCells(l string) []string {
	l = strings.TrimSpace(l)
	l = strings.TrimPrefix(l, "|")
	if strings.HasSuffix(l, "|") && !strings.HasSuffix(l, `\|`) {
		l = l[:len(l)-1]
	}
	var cells []string
	var b strings.Builder
	for i := 0; i < len(l); i++ {
		switch {
		case l[i] == '\\' && i+1 < len(l) && l[i+1] == '|':
			b.WriteByte('|')
			i++
		case l[i] == '|':
			cells = append(cells, strings.TrimSpace(b.String()))
			b.Reset()
		default:
			b.WriteByte(l[i])
		}
	}
	return append(cells, strings.TrimSpace(b.String()))
}

func vcFold(s string) string { return strings.ToLower(strings.TrimSpace(s)) }

// parseVulnClasses reads a catalog: the first table under `## Catalog`
// (its class and ruled out by columns found by header name), each `### `
// section under `## Classes` with its **Proved.** and **Gap.**
// paragraphs (a paragraph runs to a blank line, a heading or the next
// `**Name.**` paragraph), and each item of `## Gaps` (a line starting
// `N. `, `- ` or `* `). Lines inside ``` fences are skipped. Errors name
// file f and the line.
func parseVulnClasses(f, text string) *vulnDoc {
	d := &vulnDoc{sections: map[string]int{}}
	bad := func(ln int, format string, a ...any) {
		d.errs = append(d.errs, fmt.Sprintf("seams: %s:%d: %s", f, ln, fmt.Sprintf(format, a...)))
	}
	lines := strings.Split(text, "\n")
	// cur: the class section being read, so the paragraphs under Classes
	// are a class's (only lines under Classes read it).
	var cur *vcSection
	h2, fence := "", false
	para := "" // the paragraph being read in a class section
	table := 0 // 0 none yet, 1 header read, 2 rows, 3 ended, 4 a second table seen
	classCol, ruledCol, ncol := -1, -1, 0
	// A block that starts under the header leaves it with no separator
	// row; under the rows, it ends the table.
	endTable := func() {
		switch table {
		case 1:
			bad(d.tableLine+1, "the Catalog table's header has no separator row under it")
			table = 4
		case 2:
			table = 3
		}
	}
	for i, l := range lines {
		ln := i + 1
		if strings.HasPrefix(strings.TrimSpace(l), "```") {
			if !fence {
				endTable()
			}
			fence = !fence
			continue
		}
		if fence {
			continue
		}
		t := strings.TrimSpace(l)
		if strings.HasPrefix(l, "#") {
			level := len(l) - len(strings.TrimLeft(l, "#"))
			if level < len(l) && l[level] == ' ' {
				name := strings.TrimSpace(l[level:])
				para = ""
				endTable()
				switch {
				case level <= 2:
					h2 = name
				case level == 3 && h2 == "Classes":
					cur = &vcSection{name: name, line: ln}
					d.secs = append(d.secs, cur)
					k := vcFold(name)
					if first, ok := d.sections[k]; ok {
						bad(ln, "class %q has a second section (the first is line %d)", name, first)
					} else {
						d.sections[k] = ln
					}
				}
				continue
			}
		}
		switch h2 {
		case "Catalog":
			// Under the rows, a line that starts no other block is a
			// row, with or without its leading `|` (as GitHub reads a
			// table).
			isRow := strings.HasPrefix(t, "|") ||
				table == 2 && t != "" && !strings.HasPrefix(t, ">") && !reVCItem.MatchString(t)
			switch {
			case isRow && table == 0:
				hdr := vcCells(t)
				ncol = len(hdr)
				for j, c := range hdr {
					switch vcFold(c) {
					case "class":
						classCol = j
					case "ruled out by":
						ruledCol = j
					}
				}
				d.tableLine = ln
				if classCol < 0 {
					bad(ln, "the Catalog table has no class column")
				}
				if ruledCol < 0 {
					bad(ln, "the Catalog table has no ruled out by column")
				}
				table = 1
			case table == 1:
				sepOK := isRow
				for _, c := range vcCells(t) {
					if !reVCSep.MatchString(c) {
						sepOK = false
					}
				}
				if !sepOK {
					bad(ln, "the Catalog table's header has no separator row under it")
					table = 4 // its rows are not read, and not a second table either
					continue
				}
				table = 2
			case isRow && table == 2:
				cs := vcCells(t)
				if len(cs) != ncol {
					bad(ln, "a row has %d cells, the header %d", len(cs), ncol)
					continue
				}
				if classCol >= 0 {
					if cs[classCol] == "" {
						bad(ln, "a row names no class")
					} else {
						d.classOf = append(d.classOf, vcText{ln, cs[classCol]})
					}
				}
				if ruledCol >= 0 {
					d.ruledOut = append(d.ruledOut, vcText{ln, cs[ruledCol]})
				}
			case isRow && table == 3:
				bad(ln, "a second table under Catalog, which -check does not read")
				table = 4
			case !isRow && table == 2:
				table = 3
			}
		case "Classes":
			if cur == nil {
				continue
			}
			switch {
			case t == "":
				para = ""
			case strings.HasPrefix(t, "**Proved.**"):
				para = "proved"
				cur.proved = true
			case strings.HasPrefix(t, "**Gap.**"):
				para = "gap"
				cur.gap = true
			case reVCPara.MatchString(t):
				para = ""
			}
			// A paragraph's first line starts its whole text; each line
			// after it is added on, so a citation wrapped over two lines
			// reads whole.
			first := strings.HasPrefix(t, "**Proved.**") || strings.HasPrefix(t, "**Gap.**")
			switch para {
			case "proved":
				d.provedAll = vcAdd(d.provedAll, first, ln, t)
			case "gap":
				d.gapAll = vcAdd(d.gapAll, first, ln, t)
			}
		case "Gaps":
			if !reVCItem.MatchString(l) {
				continue
			}
			m := reVCIssue.FindStringSubmatch(l)
			if m == nil {
				bad(ln, "a Gaps item names no issue (#n) on its first line")
				continue
			}
			d.gaps = append(d.gaps, vcGap{ln, m[1]})
		}
	}
	endTable()
	// Every section has both paragraphs ("none" when empty), so one
	// misnamed (`**Proved**.`) fails rather than go unread.
	for _, s := range d.secs {
		if !s.proved {
			bad(s.line, "class %q has no **Proved.** paragraph (write `**Proved.** none.` when nothing is proved)", s.name)
		}
		if !s.gap {
			bad(s.line, "class %q has no **Gap.** paragraph (write `**Gap.** none.` when there is none)", s.name)
		}
	}
	return d
}

// vcAdd adds line t (line ln) to the paragraphs ps: as a new paragraph
// when it is a paragraph's first line, else onto the last.
func vcAdd(ps []vcText, first bool, ln int, t string) []vcText {
	if first {
		return append(ps, vcText{ln, t})
	}
	ps[len(ps)-1].text += " " + t
	return ps
}

// vcTheorems are the theorem ids text cites (`ID (#n)`).
func vcTheorems(text string) []string {
	var ids []string
	for _, m := range reVCTheorem.FindAllStringSubmatch(text, -1) {
		if strings.ContainsAny(m[1], "0123456789") {
			ids = append(ids, m[1])
		}
	}
	return ids
}

// vulnClassChecks are the vuln-classes line's checks of the catalog
// FILE: its Catalog table read; each row's class a `### ` section under
// Classes (compared with case folded); each theorem a row or a
// **Proved.** paragraph cites (`ID (#n)`) a file theorems/ID.md beside
// FILE; each `gap: #n` a row or a **Gap.** paragraph cites an item of
// Gaps, and each item of Gaps cited so (an item no row or class cites
// is left over from a class since proved). Whether an issue is open
// needs the network, so it is not checked.
func vulnClassChecks(root string, r *Rules) []string {
	f := r.VulnClasses
	if f == "" {
		return nil
	}
	b, err := os.ReadFile(filepath.Join(root, f))
	if err != nil {
		return []string{fmt.Sprintf("seams: %s is gone", f)}
	}
	d := parseVulnClasses(f, string(b))
	out := d.errs
	if d.tableLine == 0 {
		out = append(out, fmt.Sprintf("seams: %s: no table under ## Catalog", f))
	}
	for _, c := range d.classOf {
		if _, ok := d.sections[vcFold(c.text)]; !ok {
			out = append(out, fmt.Sprintf("seams: %s:%d: class %q has no ### section under ## Classes", f, c.line, c.text))
		}
	}
	// Each ruled out by cell cites something the check reads or is `not
	// shipped` or `by construction:` (whose check is named in prose and
	// stays true by review), and each **Proved.** and **Gap.** paragraph
	// cites one or says "none", so a citation in another form fails
	// rather than go unread.
	for _, c := range d.ruledOut {
		if vcTheorems(c.text) == nil && !reVCGap.MatchString(c.text) && !strings.Contains(c.text, "not shipped") && !strings.Contains(c.text, "by construction:") {
			out = append(out, fmt.Sprintf("seams: %s:%d: the ruled out by cell cites no theorem `ID (#n)` or `gap: #n` and is neither `not shipped` nor `by construction:`", f, c.line))
		}
	}
	for _, c := range d.provedAll {
		if vcTheorems(c.text) == nil && !reVCNone.MatchString(c.text) {
			out = append(out, fmt.Sprintf("seams: %s:%d: the **Proved.** paragraph cites no theorem `ID (#n)` and is not `none`", f, c.line))
		}
	}
	for _, c := range d.gapAll {
		if !reVCGap.MatchString(c.text) && !reVCNone.MatchString(c.text) {
			out = append(out, fmt.Sprintf("seams: %s:%d: the **Gap.** paragraph cites no `gap: #n` and is not `none`", f, c.line))
		}
	}
	thdir := path.Join(path.Dir(f), "theorems")
	for _, c := range append(append([]vcText{}, d.ruledOut...), d.provedAll...) {
		for _, m := range reVCNear.FindAllString(c.text, -1) {
			out = append(out, fmt.Sprintf("seams: %s:%d: %s: write a theorem citation `ID (#n)`, the id in plain text", f, c.line, m))
		}
		seen := map[string]bool{}
		for _, id := range vcTheorems(c.text) {
			if seen[id] {
				continue
			}
			seen[id] = true
			p := path.Join(thdir, id+".md")
			if st, err := os.Stat(filepath.Join(root, p)); err != nil || !st.Mode().IsRegular() {
				out = append(out, fmt.Sprintf("seams: %s:%d: cites theorem %s, which has no %s", f, c.line, id, p))
			}
		}
	}
	listed := map[string]int{}
	for _, g := range d.gaps {
		if first, ok := listed[g.issue]; ok {
			out = append(out, fmt.Sprintf("seams: %s:%d: Gaps lists #%s again (first on line %d)", f, g.line, g.issue, first))
			continue
		}
		listed[g.issue] = g.line
	}
	cited := map[string]bool{}
	for _, c := range append(append([]vcText{}, d.ruledOut...), d.gapAll...) {
		for _, m := range reVCGap.FindAllStringSubmatch(c.text, -1) {
			cited[m[1]] = true
			if _, ok := listed[m[1]]; !ok {
				out = append(out, fmt.Sprintf("seams: %s:%d: gap: #%s is not an item of ## Gaps", f, c.line, m[1]))
			}
		}
	}
	for _, g := range d.gaps {
		if listed[g.issue] == g.line && !cited[g.issue] {
			out = append(out, fmt.Sprintf("seams: %s:%d: Gaps item #%s: no row or class section cites gap: #%s", f, g.line, g.issue, g.issue))
		}
	}
	return out
}
