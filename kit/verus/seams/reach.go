package main

import (
	"fmt"
	"path"
	"regexp"
	"strings"
)

var (
	// An inner attribute (`#![..]`, blanks allowed between the tokens).
	reInnerAttr = regexp.MustCompile(`^#[ \t]*![ \t]*\[`)
	// A `cfg(` token in an attribute's text.
	reCfgToken = regexp.MustCompile(`cfg[ \t]*\(`)
	rePathAttr = regexp.MustCompile(`#[ \t]*\[[ \t]*path[ \t]*=`)
	// An outer attribute's start anywhere on a line.
	reAttrAny = regexp.MustCompile(`#[ \t]*\[`)
	// The head of a `mod` item alone on its line, its rest on a later
	// one: a visibility, `mod`, or `mod NAME`, or a visibility and those.
	reModHead = regexp.MustCompile(`^[ \t]*(pub([ \t]*\([^)]*\))?[ \t]*)?(mod([ \t]+[A-Za-z_][A-Za-z0-9_]*)?[ \t]*)?$`)
)

// nest is the delimiters a code line opens less those it closes.
func nest(c string) int {
	return strings.Count(c, "{") + strings.Count(c, "[") + strings.Count(c, "(") -
		strings.Count(c, "}") - strings.Count(c, "]") - strings.Count(c, ")")
}

// attrBlock reads a file's code lines as Rust attaches outer attributes:
// at each line it reports the attributes standing above it (up to the
// previous line of other code, across blank lines, comment lines and
// indentation), each read whole when rustfmt wraps it, joined with
// blanks.
type attrBlock struct {
	attrs []string // the attributes of the block so far
	open  int      // brackets the current attribute left open
	text  string   // the current attribute's text
}

// feed takes one code line; it reports whether the line is part of an
// attribute, so the block goes on. (modLine keeps the block across a
// blank or comment line too: reModHead matches an empty line.)
func (b *attrBlock) feed(t string) bool {
	if b.open > 0 {
		b.open += strings.Count(t, "[") - strings.Count(t, "]")
		if b.text += " " + t; b.open <= 0 {
			b.attrs = append(b.attrs, b.text)
		}
		return true
	}
	at := strings.TrimLeft(t, " \t")
	if !reOuterAttr.MatchString(at) {
		return false
	}
	b.open, b.text = strings.Count(at, "[")-strings.Count(at, "]"), at
	if b.open <= 0 {
		b.attrs = append(b.attrs, at)
	}
	return true
}

// innerGate is the line of a file's inner attribute that may compile the
// whole file out: `#![cfg(..)]`, or a `#![cfg_attr(..)]` that adds a cfg
// (one that adds a lint level is harmless). 0 for none.
func innerGate(f *srcFile) int {
	// depth: the delimiters open before the line. Only an inner
	// attribute at depth 0 is the file's; one in a block (`mod tests {
	// #![cfg(test)] }`) is the block's. ia: the brackets the inner
	// attribute being read left open.
	level, ia, text, at := 0, 0, "", 0
	for i, c := range f.code {
		l0 := level
		level += nest(c)
		t := strings.TrimLeft(c, " \t")
		if ia > 0 {
			ia += strings.Count(t, "[") - strings.Count(t, "]")
			text += " " + t
		} else if l0 == 0 && reInnerAttr.MatchString(t) {
			ia, text, at = strings.Count(t, "[")-strings.Count(t, "]"), t, i+1
		} else {
			continue
		}
		// `#![cfg(..)]` itself, or a cfg a cfg_attr adds (read as the
		// attribute's lines come): `cfg_attr(` is no `cfg(`, and a cfg
		// predicate holds none.
		if reCfgToken.MatchString(text) {
			return at
		}
	}
	return 0
}

// reach says why FILE (a cited assertion's, or the notcopy FILE) may be
// left out of a build, "" when every file from it up to the crate root
// is reached ungated: each file holds no inner cfg attribute (innerGate),
// and each but the root is declared by a `mod NAME;` line in its parent
// (SOURCE/lib.rs or SOURCE/main.rs for a file directly under SOURCE, else
// DIR.rs or DIR/mod.rs for a file in DIR, a mod.rs standing for its
// directory) under no cfg, cfg_attr or ignore attribute and no
// `#[path]`, which the check does not follow; the `mod NAME;` line at
// the parent's top level (modLine). A file under a cite-module DIR is
// held to a weaker rule: compiled when the tool's build runs (challoc's
// src/kani under cfg(kani)), it is reached by a `#[path]` line by its
// nature, so neither its inner attributes nor that line are read.
func (v *resolver) reach(file string) string {
	for p := file; ; {
		for _, d := range v.r.CiteModule {
			// DIR.rs under SOURCE is an ordinary module file, read in
			// full; one outside SOURCE is reached by `#[path]` too.
			if strings.HasPrefix(p, d+"/") || p == d+".rs" && !strings.HasPrefix(p, v.r.Source+"/") {
				return ""
			}
		}
		f := v.file(p)
		if !f.ok {
			return p + " cannot be read whole"
		}
		if ln := innerGate(f); ln > 0 {
			return fmt.Sprintf("%s:%d holds an inner cfg attribute, which may leave the file out", p, ln)
		}
		// The crate root: lib.rs, or main.rs when there is no lib.rs (with
		// both, a library build compiles lib.rs's tree alone).
		src := v.r.Source
		lib, main := src+"/lib.rs", src+"/main.rs"
		root := main
		if v.exists(lib) {
			root = lib
		} else if !v.exists(main) {
			return fmt.Sprintf("there is no crate root (%s or %s) to reach %s from", lib, main, p)
		}
		if why := v.targets(root); why != "" {
			return why
		}
		if p == root {
			return ""
		}
		if p == main {
			return fmt.Sprintf("%s is the binary's root; with %s there, a library build compiles %s's tree alone", main, lib, lib)
		}
		dir, base := path.Dir(p), path.Base(p)
		name, pd := strings.TrimSuffix(base, ".rs"), dir
		if base == "mod.rs" {
			name, pd = path.Base(dir), path.Dir(dir)
		}
		// DIR.rs declares DIR/NAME.rs, but not when DIR.rs is a crate
		// root or a mod.rs (their `mod NAME;` declares a file beside
		// them): Rust never reads src/lib/x.rs for lib.rs's `mod x;`.
		var parents []string
		if c := pd + ".rs"; c != lib && c != main && path.Base(pd) != "mod" {
			parents = append(parents, c)
		}
		parents = append(parents, pd+"/mod.rs")
		if pd == src {
			parents = []string{root}
		}
		next := ""
		for _, par := range parents {
			// A parent that is not there holds no line.
			ln, why := v.modLine(par, name)
			if ln == 0 {
				continue
			}
			if why != "" {
				return fmt.Sprintf("%s:%d declares `mod %s;` %s", par, ln, name, why)
			}
			next = par
			break
		}
		if next == "" {
			return fmt.Sprintf("no `mod %s;` line in %s reaches %s", name, strings.Join(parents, " or "), p)
		}
		p = next
	}
}

// tomlKey normalises a TOML key or table header for comparison: TOML
// allows blanks and tabs around its parts and quoted keys (`"path"`,
// `["lib"]`), so both go. A quoted key that holds a dot (`"lib.path"`)
// then reads as dotted, which fails closed.
var tomlKey = strings.NewReplacer(" ", "", "\t", "", `"`, "", "'", "")

// targets says why root (SOURCE/lib.rs or SOURCE/main.rs) may not be the
// crate root Cargo builds, "" when the package's Cargo.toml (beside
// SOURCE; none means the default paths) keeps the default target paths:
// for lib.rs, no `[lib] path` other than it and no `autolib = false`;
// for main.rs, no `[[bin]] path` other than it and no `autobins =
// false`. reach reads only the default crate roots.
func (v *resolver) targets(root string) string {
	dir := path.Dir(v.r.Source)
	man := path.Join(dir, "Cargo.toml")
	// The manifest must be there and be a package's: the check reads the
	// default roots of a package rooted beside SOURCE, and nothing else.
	if !v.exists(man) {
		return fmt.Sprintf("there is no %s beside the source directory, so the check cannot tell that a package builds %s", man, root)
	}
	f := v.file(man)
	lib := strings.HasSuffix(root, "/lib.rs")
	// open: the brackets a value left open, over its lines;
	// named: a [lib] path names root.
	table, pkg, open, named := "", false, 0, false
	for _, l := range f.raw {
		// The reader is line by line: a multi-line string, which may hold
		// a line like a header, is not read.
		if strings.Contains(l, `"""`) || strings.Contains(l, `'''`) {
			return fmt.Sprintf("%s holds a multi-line string, which the check does not read, so %s may not be the crate root it builds", man, root)
		}
		// The line's text before a comment and its brackets, both read
		// outside quoted strings; a string that does not close on its
		// line is not read.
		t, n, closed := tomlLine(l)
		if !closed {
			return fmt.Sprintf("%s has a string that does not close on its line, which the check does not read, so %s may not be the crate root it builds", man, root)
		}
		if strings.Contains(t, `\`) {
			return fmt.Sprintf("%s holds an escape (`\\`), which the check does not read, so %s may not be the crate root it builds", man, root)
		}
		// A line of a value still open (a multi-line array) is the
		// value's; one starting with `[` there may look like a header but
		// is none, so the reader cannot follow it. A bracket in a string
		// counts too, which fails closed.
		if open > 0 {
			if t != "" && t[0] == '[' {
				return fmt.Sprintf("%s has a line starting with `[` inside a multi-line value, which the check does not read, so %s may not be the crate root it builds", man, root)
			}
			open += n
			continue
		}
		if strings.HasPrefix(t, "[") {
			table = tomlKey.Replace(t)
			// Only a whole header: a line of a multi-line value that
			// starts with `[` is not read.
			if !reTomlHeader.MatchString(table) {
				return fmt.Sprintf("%s has a line starting with `[` that is not a table header (a multi-line value), which the check does not read, so %s may not be the crate root it builds", man, root)
			}
			pkg = pkg || table == "[package]"
			// A library the check does not read the root of: with main.rs
			// as the root, a [lib] table names one elsewhere.
			if !lib && table == "[lib]" {
				return fmt.Sprintf("%s declares a [lib] target, so a library build does not compile %s's tree (the check reads only the default roots)", man, root)
			}
			// With main.rs as the root, only the plain default binary is
			// read: a [[bin]] table may move it, gate it on features
			// (required-features) or, in edition 2015, turn discovery off.
			if !lib && table == "[[bin]]" {
				return fmt.Sprintf("%s declares a [[bin]] target, so %s may not be built as the default binary (with main.rs as the root the check reads no [[bin]] table)", man, root)
			}
			continue
		}
		k, val, _ := strings.Cut(t, "=")
		k, val = tomlKey.Replace(k), strings.TrimSpace(val)
		open = n
		// Spellings the check does not read: dotted keys and inline
		// tables for the targets and their discovery.
		if strings.HasPrefix(k, "lib.") || strings.HasPrefix(k, "package.auto") ||
			table == "" && (k == "lib" || k == "bin" || k == "package") {
			return fmt.Sprintf("%s sets %s in a form the check does not read (a dotted key or an inline table), so %s may not be the crate root it builds", man, k, root)
		}
		switch {
		// (With main.rs as the root a [lib] table has already failed.)
		case table == "[lib]" && k == "path":
			// An absolute path is no path beside the manifest, though
			// path.Join would clean it to one.
			if p := strings.Trim(val, `"'`); strings.HasPrefix(p, "/") || path.Join(dir, p) != root {
				return fmt.Sprintf("%s sets a target path %s, so %s may not be the crate root it builds (the check reads only the default roots)", man, val, root)
			}
			named = true
		case table == "[package]" && (lib && k == "autolib" || !lib && k == "autobins") && strings.HasPrefix(val, "false"):
			return fmt.Sprintf("%s sets %s = false, so %s may not be built (the check reads only the default roots)", man, k, root)
		}
	}
	if !pkg {
		return fmt.Sprintf("%s has no [package] table (a workspace's virtual manifest, or one the check cannot read), so no package builds %s", man, root)
	}
	// Cargo's default roots are src/lib.rs and src/main.rs beside the
	// manifest: another SOURCE's root counts only when a [lib] path names
	// it.
	if def := path.Join(dir, "src", path.Base(root)); root != def && !named {
		return fmt.Sprintf("%s is not Cargo's default root %s, and no [lib] path in %s names it", root, def, man)
	}
	return ""
}

// tomlLine reads a manifest line: its text before a comment (a `#`
// outside a quoted string), the brackets it opens less those it closes
// outside quoted strings (an inline table is one line in TOML 1.0, so its
// braces never stay open), and whether every string on it closes. An
// escape is refused by the caller, so a quote always ends its string.
func tomlLine(l string) (text string, nest int, closed bool) {
	q := byte(0)
	for i := 0; i < len(l); i++ {
		switch c := l[i]; {
		case q != 0:
			if c == q {
				q = 0
			}
		case c == '"' || c == '\'':
			q = c
		case c == '#':
			return strings.TrimSpace(l[:i]), nest, true
		case c == '[':
			nest++
		case c == ']':
			nest--
		}
	}
	return strings.TrimSpace(l), nest, q == 0
}

// reTomlHeader is a whole table header, normalised by tomlKey: `[name]`
// or `[[name]]`, nothing after its closing brackets.
var reTomlHeader = regexp.MustCompile(`^\[\[?[^\[\]]+\]\]?$`)

// modLine finds `mod NAME;` (a visibility allowed) in file: its line,
// and why it does not count ("" when it does): under a cfg, cfg_attr
// or ignore attribute, or a `#[path]`. 0 when there is none.
func (v *resolver) modLine(file, name string) (int, string) {
	f := v.file(file)
	if !f.ok {
		return 0, ""
	}
	re := regexp.MustCompile(`^[ \t]*(pub(\([^)]*\))?[ \t]+)?mod[ \t]+` + regexp.QuoteMeta(name) + `[ \t]*;`)
	var b attrBlock
	// depth: the delimiters (braces, brackets, parentheses) open before
	// the line (the code view blanks literals and comments, so they are
	// code). A `mod NAME;` inside any (a macro's body or arguments in
	// any of its three delimiters, `cfg_if!`, an inline mod, which
	// declares DIR/outer/NAME.rs) is not read as reached.
	depth, nested, hash, pre := 0, 0, false, ""
	for i, c := range f.code {
		d0 := depth
		depth += nest(c)
		// A `#` that ended an earlier line (hashEnd) and this line's `[`
		// are one attribute: Rust joins the tokens across the break.
		if hash && strings.TrimSpace(c) != "" {
			c, hash = "#"+strings.TrimLeft(c, " \t"), false
		}
		// An item's head split over lines (`pub` / `mod NAME;`, `mod` /
		// `NAME;`) is one item: the held head joins this line.
		if pre != "" && strings.TrimSpace(c) != "" {
			c, pre = pre+" "+strings.TrimLeft(c, " \t"), ""
		}
		// Attributes before the item on its line belong to the block.
		if b.open == 0 && reOuterAttr.MatchString(strings.TrimLeft(c, " \t")) {
			if cut := attrEnd(c, 0); cut > 0 {
				b.feed(c[:cut])
				c = c[cut:]
			}
		}
		if b.feed(c) {
			continue
		}
		if reModHead.MatchString(c) {
			pre = strings.TrimSpace(c)
			continue
		}
		if re.MatchString(c) && d0 != 0 {
			nested = i + 1
		} else if re.MatchString(c) {
			for _, a := range b.attrs {
				if reGateAttr.MatchString(a) {
					return i + 1, "under a cfg, cfg_attr or ignore attribute, which may leave the file out"
				}
				if rePathAttr.MatchString(a) {
					return i + 1, "under a `#[path]` attribute, which the check does not follow"
				}
			}
			return i + 1, ""
		}
		// An attribute after other code on the line attaches to the next
		// item: it starts the next block.
		b = attrBlock{}
		if loc := reAttrAny.FindStringIndex(c); loc != nil {
			b.feed(c[loc[0]:])
		}
		// A `#` ending the line (after a trailing attribute too) joins
		// the next line's `[`.
		if hashEnd(c) {
			hash = true
		}
	}
	if nested != 0 {
		return nested, "inside a block or macro (not at the file's top level), which the check does not read as reached"
	}
	return 0, ""
}
