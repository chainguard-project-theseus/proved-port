package main

import (
	"fmt"
	"path"
	"regexp"
	"slices"
	"strings"
)

// citeTest is a cite-test line: the directory of the test crate's
// sources (challoc's tests/loom/src, its loom models) and the
// `#[cfg(CFG)]` attributes (as written, `#[cfg(test)]`) its tests,
// their files and their `mod` lines may carry.
type citeTest struct {
	dir  string
	cfgs []string
}

// inner is the cfgs as inner attributes, `#![cfg(CFG)]`.
func (ct citeTest) inner() []string {
	var in []string
	for _, c := range ct.cfgs {
		in = append(in, "#!"+strings.TrimPrefix(c, "#"))
	}
	return in
}

// reTestDoc is a `///` doc comment line, as loomtest() reads one.
var reTestDoc = regexp.MustCompile(`^///( |$)`)

// reShouldPanic finds a should_panic attribute in an attribute's text.
var reShouldPanic = regexp.MustCompile(`#[ \t]*\[[ \t]*should_panic`)

// reTestAttr finds `#[test]` in an attribute's text (a line may hold
// several attributes).
var reTestAttr = regexp.MustCompile(`#[ \t]*\[[ \t]*test[ \t]*\]`)

// seamFn is the name a test names a seam's function by, seams.sh's
// loomtest(): the seam's name (which holds no `#N`: the overrides key
// does), and for a name with no `::` (no method) `m::name`, m the
// module of the seam's source file (its base name, the directory's for
// a mod.rs).
func seamFn(e *entry) string {
	name := e.name
	if strings.Contains(name, "::") {
		return name
	}
	m := strings.TrimSuffix(e.file, ".rs")
	if path.Base(m) == "mod" {
		m = path.Dir(m)
	}
	return path.Base(m) + "::" + name
}

// namesFn says whether a backquoted span of s is name, or a path ending
// in `::name` whose segments are words (`a::m::f`).
func namesFn(s, name string) bool {
	p := strings.Split(s, "`")
	for i := 1; i < len(p); i += 2 {
		sp := p[i]
		if sp == name {
			return true
		}
		if pre, ok := strings.CutSuffix(sp, "::"+name); ok && reWordPath.MatchString(pre) {
			return true
		}
	}
	return false
}

var reWordPath = regexp.MustCompile(`^[A-Za-z0-9_]+(::[A-Za-z0-9_]+)*$`)

// test says why the citation `MOD::TEST` of seam e does not resolve
// under ct, "" when it does, seams.sh's loomtest() and more:
//   - DIR/MOD.rs names e's source file in backquotes, `SOURCE/a/b.rs`
//     or, for a file in a directory under SOURCE, `a/b.rs`;
//   - it has a top-level line `fn TEST() {` (a visibility allowed; a
//     test in an inline mod, or in DIR/MOD/mod.rs, is not read) whose
//     attributes (up to the previous line of other code, across blank
//     and comment lines, those before it on its line too, each read
//     whole when wrapped, a `#` ending a line joined to the next line's
//     `[`, after other code on its line, an item's closing delimiter
//     included, the last top-level attribute run with no item after it
//     starting the block, tailBlock) hold `#[test]`, no `#[should_panic]` (a control, which fails by
//     design) and no cfg, cfg_attr or ignore attribute but one of ct's
//     `#[cfg(CFG)]` whole on its line;
//   - its `///` doc lines among those attributes, or its body (to the
//     delimiter that closes it), names e's function in backquotes (seamFn,
//     namesFn);
//   - DIR/lib.rs is the crate root the test crate's manifest (beside
//     DIR, read as targetsOf reads the main crate's) builds and tests:
//     DIR is the package's src, no `[lib] path` names another root, no
//     `autolib = false`, no `[lib]` `test = false` or `harness = false`;
//   - DIR/lib.rs declares `mod MOD;` at its top level under no gate but ct's cfgs
//     (modLine), and neither file holds an inner cfg attribute but
//     `#![cfg(CFG)]` whole on its line.
//
// Each file is read with the lexer, so a test, attribute or `mod` line in
// a block comment or literal is none, and a file that ends inside one
// resolves nothing.
func (v *resolver) test(ct citeTest, e *entry, target string) string {
	mod, t, ok := strings.Cut(target, "::")
	if !ok || !reIdent.MatchString(mod) || !reIdent.MatchString(t) {
		return "is no `MOD::TEST` path"
	}
	file := path.Join(ct.dir, mod+".rs")
	f := v.file(file)
	if !f.ok {
		return file + " is not there or ends inside a literal or comment"
	}
	// modLine finds no line in a lib.rs that is not there or ends inside
	// a literal or comment.
	lib := path.Join(ct.dir, "lib.rs")
	if why := v.targetsOf(ct.dir, lib, true); why != "" {
		return why
	}
	if ln, why := v.modLine(lib, mod, ct.cfgs); ln == 0 {
		return fmt.Sprintf("no `mod %s;` line in %s (read whole) declares %s", mod, lib, file)
	} else if why != "" {
		return fmt.Sprintf("%s:%d declares `mod %s;` %s", lib, ln, mod, why)
	}
	for _, p := range []string{lib, file} {
		if ln := innerGate(v.file(p), ct.inner()); ln > 0 {
			return fmt.Sprintf("%s:%d holds an inner cfg attribute, which may leave the file out", p, ln)
		}
	}
	if why := v.namesSrc(f, file, e); why != "" {
		return why
	}
	ln, why := v.testFn(f, t, ct.cfgs, seamFn(e))
	if ln == 0 {
		return fmt.Sprintf("%s has no top-level `fn %s() {`", file, t)
	}
	if why != "" {
		return fmt.Sprintf("%s:%d: `fn %s`: %s", file, ln, t, why)
	}
	return ""
}

// zeroAt is the index in the code line c, which starts d0 delimiters
// deep, just past the first delimiter that brings the depth to 0: the
// line's text from there is at the top level (an attribute in it,
// `#[ignore]`, opens and closes its own brackets). -1 when the depth
// never reaches 0 on the line.
func zeroAt(c string, d0 int) int {
	d := d0
	for i := 0; i < len(c); i++ {
		switch c[i] {
		case '{', '[', '(':
			d++
		case '}', ']', ')':
			d--
		default:
			continue
		}
		if d == 0 {
			return i + 1
		}
	}
	return -1
}

// tailBlock is the block the attributes after other code on a line t
// start (`x; #[should_panic]`): empty when t holds none, or when an item
// follows them on the line (`#[should_panic] const B: u8 = 0;`), which
// they attach to.
//
// Every attribute run at t's top level (t starts at depth 0; one inside
// a macro call's or block's delimiters, `m!(#[a] x)`, is not) is read
// in turn: a run with an item after it attaches to that item, and the
// last run, with none after it, starts the block (`const C: u8 = 0;
// #[inline] const D: u8 = 0; #[should_panic]`).
func tailBlock(t string) attrBlock {
	var b attrBlock
	for at := 0; at < len(t); {
		loc := reAttrAny.FindStringIndex(t[at:])
		if loc == nil {
			break
		}
		s := at + loc[0]
		if nest(t[:s]) != 0 {
			at = s + 1
			continue
		}
		end := attrEnd(t[s:], 0)
		if end == 0 {
			b.feed(t[s:])
			break
		}
		at = s + end
	}
	return b
}

// namesSrc says why the file f (at file) does not name e's source file
// in backquotes on some line, "" when it does: `SOURCE/a/b.rs`, or
// `a/b.rs` for a file in a directory under SOURCE.
func (v *resolver) namesSrc(f *srcFile, file string, e *entry) string {
	src := "`" + e.file + "`"
	short := src
	if rel, ok := strings.CutPrefix(e.file, v.r.Source+"/"); ok && strings.Contains(rel, "/") {
		short = "`" + rel + "`"
	}
	for _, l := range f.raw {
		if strings.Contains(l, src) || strings.Contains(l, short) {
			return ""
		}
	}
	if short != src {
		return fmt.Sprintf("%s names neither %s nor %s", file, src, short)
	}
	return fmt.Sprintf("%s does not name %s", file, src)
}

// testFn finds the first top-level line `fn TEST() {` of f: its line (0
// for none) and why it is no test naming name ("" when it is one).
func (v *resolver) testFn(f *srcFile, t string, allow []string, name string) (int, string) {
	fnRe := regexp.MustCompile("^" + reVis + "fn " + t + `\(\) \{$`)
	var b attrBlock
	// raws: per attribute of the block, its raw line (trimmed) when the
	// attribute is whole and alone on it, else "": an allowed
	// `#[cfg(CFG)]` must equal it. doc: the block's `///` lines. hash: a
	// line ended in an attribute's `#`, whose `[` is on the next line of
	// code (Rust joins the two tokens).
	var raws []string
	doc, depth, hash := "", 0, false
	sync := func() {
		for len(raws) < len(b.attrs) {
			raws = append(raws, "")
		}
	}
	for i, c := range f.code {
		d0 := depth
		depth += nest(c)
		// A line inside an item's delimiters (an attribute's wrapped
		// lines are fed below) is no line of the block, unless it closes
		// them: the text after its first return to depth 0 (`}
		// #[should_panic]`, `]; #[ignore]`, `} #`) is read as after other
		// code, so an attribute there starts the next block.
		if d0 != 0 && b.open == 0 {
			z := zeroAt(c, d0)
			if z < 0 {
				continue
			}
			rest := c[z:]
			b, raws, doc = tailBlock(rest), nil, ""
			sync()
			hash = hashEnd(rest)
			continue
		}
		if f.clean[i] && reTestDoc.MatchString(f.raw[i]) {
			doc += " " + f.raw[i]
			continue
		}
		// Blank lines and comment lines keep the block, as Rust attaches
		// attributes across them.
		if strings.TrimSpace(c) == "" {
			continue
		}
		raw := strings.TrimSpace(f.raw[i])
		if hash {
			c, raw, hash = "#"+strings.TrimLeft(c, " \t"), "", false
		}
		if b.open == 0 && reLoneHash.MatchString(strings.TrimSpace(c)) {
			hash = true
			continue
		}
		n := len(b.attrs)
		// Attributes before the item on its line belong to the block.
		if b.open == 0 && reOuterAttr.MatchString(strings.TrimLeft(c, " \t")) {
			if cut := attrEnd(c, 0); cut > 0 {
				b.feed(c[:cut])
				c, raw = c[cut:], ""
			}
		}
		attr := b.feed(c)
		sync()
		if len(b.attrs) == n+1 && raw != "" && b.attrs[n] == strings.TrimLeft(c, " \t") {
			raws[n] = raw
		}
		if attr {
			hash = hashEnd(c)
			continue
		}
		if !fnRe.MatchString(c) {
			// Other code ends the block; an attribute after it on its
			// line starts the next.
			b, raws, doc = tailBlock(c), nil, ""
			sync()
			hash = hashEnd(c)
			continue
		}
		test := false
		for k, a := range b.attrs {
			a = strings.TrimSpace(a)
			test = test || reTestAttr.MatchString(a)
			if reShouldPanic.MatchString(a) {
				return i + 1, "a #[should_panic] test (a control) is no check"
			}
			if reGateAttr.MatchString(a) && !slices.Contains(allow, raws[k]) {
				return i + 1, "under a cfg, cfg_attr or ignore attribute, which may keep it from running (a cite-test CFG field allows `#[cfg(CFG)]`)"
			}
		}
		if !test {
			return i + 1, "no `#[test]` line above it"
		}
		// The body: the lines after the fn line, to the one that closes it.
		text := doc
		// The closing line counts only up to the delimiter that closes
		// the body: code or a comment after it (`} fn helper() {`, `} //
		// ..`) is not the test's. The code view blanks column for column,
		// so its index holds in the raw line. (The fn line leaves depth
		// 1, so the body is open until that line.)
		for k, d := i+1, depth; k < len(f.code); k++ {
			if z := zeroAt(f.code[k], d); z >= 0 {
				text += " " + f.raw[k][:min(z, len(f.raw[k]))]
				break
			}
			d += nest(f.code[k])
			text += " " + f.raw[k]
		}
		if !namesFn(text, name) {
			return i + 1, fmt.Sprintf("neither its doc comment nor its body names `%s` in backquotes", name)
		}
		return i + 1, ""
	}
	return 0, ""
}
