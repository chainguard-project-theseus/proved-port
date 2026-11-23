package main

import (
	"fmt"
	"os"
	"path"
	"path/filepath"
	"regexp"
	"slices"
	"strings"
)

// citeRow is a cite-row line: the table in FILE under `## HEADING` where
// each citation of the tag needs a row holding every part.
type citeRow struct {
	file, heading string
	parts         []string
}

// srcFile is a file read for the citation checks: its lines, each line's
// code (lexed in order, literals and comments blanked) and whether the
// lexer was outside every literal and comment at the line's start. ok is
// false when the file could not be read or ends inside a literal or
// comment, which no check reads past.
type srcFile struct {
	raw, code []string
	clean     []bool
	ok        bool
}

// resolver reads the files the citations name, each once.
type resolver struct {
	root  string
	r     *Rules
	files map[string]*srcFile
}

func (v *resolver) file(p string) *srcFile {
	if f, ok := v.files[p]; ok {
		return f
	}
	f := &srcFile{}
	v.files[p] = f
	st, err := os.Stat(filepath.Join(v.root, p))
	if err != nil || !st.Mode().IsRegular() {
		return f
	}
	b, err := os.ReadFile(filepath.Join(v.root, p))
	if err != nil {
		return f
	}
	f.raw = splitLines(string(b))
	var lx lexer
	for _, l := range f.raw {
		f.clean = append(f.clean, lx.depth == 0 && lx.str == 0)
		f.code = append(f.code, strings.TrimRight(lx.code(l), " \t"))
	}
	f.ok = lx.depth == 0 && lx.str == 0
	return f
}

// exists says whether p is a regular file.
func (v *resolver) exists(p string) bool {
	st, err := os.Stat(filepath.Join(v.root, p))
	return err == nil && st.Mode().IsRegular()
}

// crateFile says whether p (cleaned) is a .rs file of the crate: under
// SOURCE or a cite-module DIR, and not one of the rules' test files
// (built only for tests). Whether a `mod` line reaches it, and under
// what cfg, is reach's (reach.go).
func (v *resolver) crateFile(p string) bool {
	if !strings.HasSuffix(p, ".rs") || v.r.isTest(p) {
		return false
	}
	if strings.HasPrefix(p, v.r.Source+"/") {
		return true
	}
	for _, d := range v.r.CiteModule {
		if p == d+".rs" || strings.HasPrefix(p, d+"/") {
			return true
		}
	}
	return false
}

const reVis = `(pub(\([a-z]+\))? )?`

// fn says whether PATH (`a::b::m::f`) names a fn with the attribute line
// `#[ATTR]` above it, seams.sh's harness(). The file is the longest
// leading run of segments, all but the last, that names SOURCE/a/b.rs or
// SOURCE/a/b/mod.rs; a first segment the rules' cite-module lines name
// reads under its DIR instead (challoc's `kani_proofs::m::f` is `f` in
// src/kani/m.rs), and a path that reaches such a DIR any other way names
// nothing. The segments after the file's, but the last, are inline
// `mod`s, one per level of rustfmt's four-blank indentation, each line
// `mod m {` (pub or pub(x) before it allowed); the fn is the first line
// at the innermost level that starts `fn f(` or `fn f<` (with the same
// visibility), and found is whether `#[ATTR]` stands alone at its
// indentation above it, with only other attribute lines and comment
// lines between. gated is whether an attribute that applies to the fn,
// across blank lines, comment lines and indentation as Rust applies it,
// is a cfg, cfg_attr or ignore attribute other than `#[cfg(CFG)]` for a
// CFG in allow. The lines are read as code: an attribute, mod or fn in
// a comment or literal is none, and a file that ends inside one names
// nothing.
func (v *resolver) fn(target, attr string, allow []string) (found, gated bool) {
	segs := strings.Split(target, "::")
	// One segment names no file (the file is the segments but the last).
	for _, s := range segs {
		if !reIdent.MatchString(s) {
			return false, false
		}
	}
	dir, alias := v.r.CiteModule[segs[0]]
	at := func(n int) string {
		if alias {
			return path.Join(append([]string{dir}, segs[1:n]...)...)
		}
		return path.Join(append([]string{v.r.Source}, segs[:n]...)...)
	}
	file, n := "", len(segs)-1
	for ; n >= 1; n-- {
		p := at(n)
		if v.exists(p + ".rs") {
			file = p + ".rs"
		} else if v.exists(p + "/mod.rs") {
			file = p + "/mod.rs"
		}
		if file != "" {
			break
		}
	}
	if file == "" {
		return false, false
	}
	if !alias {
		for _, d := range v.r.CiteModule {
			if file == d+".rs" || strings.HasPrefix(file, d+"/") {
				return false, false
			}
		}
	}
	mods, name := segs[n:len(segs)-1], segs[len(segs)-1]
	f := v.file(file)
	if !f.ok {
		return false, false
	}
	modRe := make([]*regexp.Regexp, len(mods))
	for k, m := range mods {
		modRe[k] = regexp.MustCompile("^" + strings.Repeat("    ", k) + reVis + "mod " + m + ` \{$`)
	}
	fnRe := regexp.MustCompile("^" + strings.Repeat("    ", len(mods)) + reVis + "fn " + name + "[(<]")
	// p: the fn's attribute block holds `#[ATTR]`; gate: it holds a
	// cfg, cfg_attr or ignore attribute, under which no build may compile
	// or run the fn; open: the brackets an attribute left open (rustfmt
	// wraps a long one over lines, to its `]`), whose lines belong to
	// the block.
	// isGate tests an attribute's lexed text; raw is its line as written
	// ("" for one wrapped over lines), which an allowed `#[cfg(CFG)]`
	// must equal (trimmed): the lexed text blanks a literal in CFG.
	isGate := func(text, raw string) bool {
		return reGateAttr.MatchString(text) && !slices.Contains(allow, strings.TrimSpace(raw))
	}
	d, ind, p, gate, open, text, hash := 0, "", false, false, 0, "", false
	for i, t := range f.code {
		if open > 0 {
			open += strings.Count(t, "[") - strings.Count(t, "]")
			if text += " " + t; open <= 0 {
				gate = gate || isGate(text, "")
			}
			continue
		}
		if d < len(mods) && modRe[d].MatchString(t) {
			d, ind, p, gate = d+1, ind+"    ", false, false
			continue
		}
		if d > 0 && t == ind[4:]+"}" {
			return false, false
		}
		if d == len(mods) && fnRe.MatchString(t) {
			return p, gate
		}
		if t == ind+"#["+attr+"]" {
			p = true
			continue
		}
		// A comment line (or one inside a block comment) keeps the
		// block. A blank line ends `#[ATTR]`'s hold but not a gate,
		// since Rust applies an attribute across it; any other code
		// ends both.
		if t == "" {
			if strings.TrimSpace(f.raw[i]) == "" {
				p = false
			}
			continue
		}
		at, raw := strings.TrimLeft(t, " \t"), f.raw[i]
		// `#` and `[` are two tokens, so a `#` alone on its line starts
		// an attribute whose `[` is on the next line of code: read the
		// two as one (never an allowed cfg, which is whole on a line).
		if hash {
			hash, at, raw = false, "#"+at, ""
		}
		if reLoneHash.MatchString(at) {
			hash = true
			continue
		}
		if !reOuterAttr.MatchString(at) {
			p, gate = false, false
			continue
		}
		// An attribute at another indentation still applies to the fn,
		// but `#[ATTR]` holds only across those at the fn's own.
		if !strings.HasPrefix(t, ind+"#[") {
			p = false
		}
		open, text = strings.Count(at, "[")-strings.Count(at, "]"), at
		if open <= 0 {
			gate = gate || isGate(text, raw)
		}
	}
	return false, false
}

var reLoneHash = regexp.MustCompile(`^#[ \t]*$`)

// reGateAttr finds, anywhere in an attribute line (an unformatted file
// may hold several on one line) or in the lines of one rustfmt wrapped,
// an attribute that may keep a fn from being built or run: `cfg(..)`,
// `cfg_attr(..)` (which may add one), and `ignore` or `ignore = ".."`.
// Literal contents are blanked, so text in a doc string is none.
// `#` and `[` are two tokens, so blanks may stand between them.
var reGateAttr = regexp.MustCompile(`#[ \t]*\[[ \t]*(cfg|cfg_attr)[ \t]*\(|#[ \t]*\[[ \t]*ignore[ \t]*(\]|=)`)

// reOuterAttr is a line (trimmed) that starts an outer attribute, `#[`
// or `# [`. An inner attribute (`#![..]`) is read as other code.
var reOuterAttr = regexp.MustCompile(`^#[ \t]*\[`)

var (
	reAssertLine = re(`^const _: \(\) = assert!\(($|.*\);$)`)
	reAttrAbove  = re(`^#\[|\]$`)
)

// assertion says whether `LABEL in FILE` names exactly one top-level
// `const _: () = assert!(` in FILE, alone on its line or with the whole
// assertion on that line, right under a block of `//` comment lines
// whose first starts `// LABEL`, seams.sh's assertion(). An attribute
// as the last line of code above the comment block (a line starting
// `#[` or ending `]`, as a multi-line `cfg` ends), across blank lines,
// comment lines and block comments, makes it count twice, so it fails:
// a gated assertion is not evaluated in every build.
// FILE is the text after the last ` in `. Comment and assertion lines
// count only where the line starts outside every literal and comment.
func (v *resolver) assertion(target string) bool {
	i := strings.LastIndex(target, " in ")
	if i < 0 {
		return false
	}
	// Cleaned, a FILE that leaves the tree (`../`, or an absolute path)
	// is under neither SOURCE nor a cite-module DIR.
	file := path.Clean(target[i+4:])
	if !v.crateFile(file) {
		return false
	}
	// A LABEL of blanks alone would match any `// ` comment.
	if strings.TrimSpace(target[:i]) == "" {
		return false
	}
	label := "// " + target[:i]
	af := v.file(file)
	if !af.ok {
		return false
	}
	// depth: the delimiters open before the line. A top-level assertion
	// is at depth 0, not in a (perhaps gated) inline mod or other block.
	n, inBlock, h, g, prev, depth := 0, false, false, false, "", 0
	for i, l := range af.raw {
		d0 := depth
		depth += nest(af.code[i])
		if af.clean[i] && strings.HasPrefix(l, "//") {
			if !inBlock {
				g = reAttrAbove.MatchString(prev)
				h = strings.HasPrefix(l, label)
			}
			inBlock = true
			continue
		}
		// The line after a comment line starts outside every literal
		// and comment.
		if h && d0 == 0 && reAssertLine.MatchString(l) {
			n++
			if g {
				n++
			}
		}
		inBlock, h = false, false
		// prev is the last line holding code: a line of comments alone
		// (a block comment between an attribute and the label too) is
		// not one, so it cannot hide the attribute.
		if af.code[i] != "" {
			prev = af.code[i]
		}
	}
	return n == 1
}

// row says whether the table of FILE under `## HEADING` (to the next
// `## ` line) has a row, a line starting `| `, whose first cell holds
// TARGET in backquotes and whose text holds every part, seams.sh's
// guardrow().
func (v *resolver) row(rw citeRow, target string) bool {
	f := v.file(rw.file)
	in := false
	for _, l := range f.raw {
		if strings.HasPrefix(l, "## ") {
			in = l == "## "+rw.heading
		}
		if !in || !strings.HasPrefix(l, "| ") {
			continue
		}
		if !strings.Contains(strings.Split(l, "|")[1], "`"+target+"`") {
			continue
		}
		all := true
		for _, p := range rw.parts {
			all = all && strings.Contains(l, p)
		}
		if all {
			return true
		}
	}
	return false
}

// resolveCites checks each citation of a tag the rules give a check
// (cite-fn, cite-assert, cite-test, cite-case, cite-row): one line per
// citation that does not resolve, once each.
func resolveCites(root string, r *Rules, es []*entry) []string {
	v := &resolver{root: root, r: r, files: map[string]*srcFile{}}
	var out []string
	seen := map[string]bool{}
	add := func(s string) {
		if !seen[s] {
			seen[s] = true
			out = append(out, s)
		}
	}
	for _, e := range es {
		// A seam with no overrides line is `trusted` and cites nothing.
		at := fmt.Sprintf("seams: %s: %s: ", path.Join(r.Ledger, e.file)+".overrides", strings.TrimPrefix(e.okey, e.file+":"))
		cs, _ := r.citations(e.stat)
		for _, c := range cs {
			if attr, ok := r.CiteFn[c.tag]; ok {
				found, gated := v.fn(c.target, attr, r.CiteFnCfg[c.tag])
				if !found {
					add(fmt.Sprintf("%s%s: %s names no #[%s] fn", at, c.tag, c.target, attr))
					continue
				}
				if gated {
					add(fmt.Sprintf("%s%s: %s names a #[%s] fn under a cfg, cfg_attr or ignore attribute, which may keep it out of every build that checks it (a cite-fn CFG field allows `#[cfg(CFG)]`)", at, c.tag, c.target, attr))
					continue
				}
			}
			if ct, ok := r.CiteTest[c.tag]; ok {
				if why := v.test(ct, e, c.target); why != "" {
					add(fmt.Sprintf("%s%s: %s: %s", at, c.tag, c.target, why))
					continue
				}
			}
			if dir, ok := r.CiteCase[c.tag]; ok {
				if why := v.caseCite(dir, e, c.target); why != "" {
					add(fmt.Sprintf("%s%s: %s: %s", at, c.tag, c.target, why))
					continue
				}
			}
			if r.CiteAssert[c.tag] && !v.assertion(c.target) {
				add(fmt.Sprintf("%s%s: %s names no one labeled assertion (a top-level `const _: () = assert!(` right under a `// LABEL` comment, no attribute above it, in FILE)", at, c.tag, c.target))
				continue
			}
			// The assertion holds only if every build compiles its file.
			if r.CiteAssert[c.tag] {
				file := path.Clean(c.target[strings.LastIndex(c.target, " in ")+4:])
				if why := v.reach(file); why != "" {
					add(fmt.Sprintf("%s%s: %s: %s may be left out of a build: %s", at, c.tag, c.target, file, why))
					continue
				}
			}
			if rw, ok := r.CiteRow[c.tag]; ok && !v.row(rw, c.target) {
				m := fmt.Sprintf("%s%s: %s has no row in the `## %s` table of %s that names it in backquotes in its first cell", at, c.tag, c.target, rw.heading, rw.file)
				if len(rw.parts) > 0 {
					m += " and holds `" + strings.Join(rw.parts, "` and `") + "`"
				}
				add(m)
			}
		}
	}
	return out
}
