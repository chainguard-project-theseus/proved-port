package main

import (
	"fmt"
	"path"
	"regexp"
	"strings"
)

var (
	// A `tracked` struct or enum (a ghost token) anywhere on a code line:
	// after indentation, a visibility, or attributes on its line, as
	// render.go's reTrackedType finds one in a contract. Its name follows.
	reTrackedDecl = regexp.MustCompile(`(^|[^A-Za-z0-9_])tracked[ \t]+(struct|enum)([^A-Za-z0-9_]|$)`)
	reIdentHead   = regexp.MustCompile(`^[A-Za-z_][A-Za-z0-9_]*`)
	// The second impls, without which a not-Copy check compiles for any
	// type: whole lines at the top level.
	reNotCopyImpl  = regexp.MustCompile(`^impl<T: Copy> NotCopy<[A-Za-z_]+> for T \{\}$`)
	reNotCloneImpl = regexp.MustCompile(`^impl<T: Clone> NotClone<[A-Za-z_]+> for T \{\}$`)
	reAssertOpen   = regexp.MustCompile(`^const _: \(\) = assert!\(`)
)

// notCopy says whether `LABEL in FILE` names a not-Copy check of TYPE,
// seams.sh's notcopy(): exactly one top-level `const _: () = assert!(`
// right under a block of `//` lines whose first starts `// LABEL` (as
// assertion finds it, attributes aside), whose text, from `const _` to
// the line that ends in `);`, read as code (comments and literal
// contents blanked), names both `<TYPE as NotCopy<_>>::HOLDS` and `<TYPE
// as NotClone<_>>::HOLDS`, TYPE alone or after a path, with or without
// arguments that hold no `<` or `>`; and FILE holds the second impls,
// `impl<T: Copy> NotCopy<..> for T {}` and `impl<T: Clone> NotClone<..>
// for T {}`, as whole top-level lines. FILE and LABEL are read as
// assertion reads them.
func (v *resolver) notCopy(ty, target string) bool {
	i := strings.LastIndex(target, " in ")
	if i < 0 || strings.TrimSpace(target[:i]) == "" {
		return false
	}
	file := path.Clean(target[i+4:])
	if !v.crateFile(file) {
		return false
	}
	label := "// " + target[:i]
	f := v.file(file)
	if !f.ok {
		return false
	}
	// prev: the last line holding code (comment lines and block comments
	// hold none), for an attribute above an impl.
	// depth: the delimiters open before the line; the assertion and the
	// impls count at depth 0, the file's top level, not in a block.
	n, inBlock, h, in, text, ic, il, prev, depth := 0, false, false, false, "", false, false, "", 0
	for i, l := range f.raw {
		c := f.code[i]
		d0 := depth
		depth += nest(c)
		if in {
			text += " " + c
			in = !strings.HasSuffix(c, ");")
			continue
		}
		if f.clean[i] && strings.HasPrefix(l, "//") {
			if !inBlock {
				h = strings.HasPrefix(l, label)
			}
			inBlock = true
			continue
		}
		if h && d0 == 0 && reAssertOpen.MatchString(l) {
			n++
			text, in = c, !strings.HasSuffix(c, ");")
		}
		inBlock, h = false, false
		// An impl under an attribute (a cfg the build may not meet), or in
		// a block, is none, as a gated assertion is none.
		ungated := d0 == 0 && !reAttrAbove.MatchString(prev)
		ic = ic || ungated && reNotCopyImpl.MatchString(c)
		il = il || ungated && reNotCloneImpl.MatchString(c)
		if c != "" {
			prev = c
		}
	}
	if n != 1 || !ic || !il {
		return false
	}
	q := regexp.QuoteMeta(ty) + `(<[^<>]*>)?`
	return regexp.MustCompile(`[<:]`+q+` as NotCopy<_>>::HOLDS`).MatchString(text) &&
		regexp.MustCompile(`[<:]`+q+` as NotClone<_>>::HOLDS`).MatchString(text)
}

// tracked checks one tracked type declared at file:line: not a second
// of its name, and with its not-Copy check in the notcopy FILE.
func (v *resolver) tracked(seen map[string]bool, ty, file string, line int, how func(string) string) []string {
	nc := v.r.NotCopyFile
	at := fmt.Sprintf("seams: %s: %s (%s:%d): ", nc, ty, file, line)
	if seen[ty] {
		return []string{at + "a second tracked type of this name, which the one check of the name cannot stand for: rename one"}
	}
	seen[ty] = true
	cite := "`" + ty + "` in " + nc
	if !v.notCopy(ty, cite) || !v.assertion(cite) {
		return []string{at + fmt.Sprintf("a tracked type with no check in %s that it is neither Copy nor Clone (%s under a `// `%s` ...` comment, with no attribute above it)", nc, how(ty), ty)}
	}
	return nil
}

// notCopyChecks are the rules' notcopy line's checks (seams.sh's
// trackedcheck and notcopycheck): every `tracked` struct or enum in
// files (the source and test files), seam or not, each on a line read,
// has a not-Copy check in FILE under the label NAME in backquotes with
// no attribute above it, and no second tracked type has a name already
// seen; a declaration whose name is no identifier (a macro's `$n`)
// fails, as no check can be held to it; and every tracked-type seam
// whose status cites TAG cites a not-Copy check of its type among
// them. One line per failure.
//
// A const assertion checks a generic type at the arguments it names
// only: a port writes it at Copy arguments (`K<u8>`), so that it fails
// to compile when the type is Copy for some Copy argument.
func notCopyChecks(root string, r *Rules, es []*entry, files []string) []string {
	if r.NotCopyFile == "" {
		return nil
	}
	v := &resolver{root: root, r: r, files: map[string]*srcFile{}}
	var out []string
	nc := r.NotCopyFile
	how := func(ty string) string {
		return fmt.Sprintf("`<%s as NotCopy<_>>::HOLDS` and `<%s as NotClone<_>>::HOLDS`", ty, ty)
	}
	// The checks hold only if every build compiles their file.
	if why := v.reach(nc); why != "" {
		out = append(out, fmt.Sprintf("seams: %s: may be left out of a build: %s", nc, why))
	}
	seen := map[string]bool{}
	for _, file := range files {
		f := v.file(file)
		for i, c := range f.code {
			// Read as code: a declaration in a comment or literal is none.
			// Every one on the line is read.
			for _, loc := range reTrackedDecl.FindAllStringIndex(c, -1) {
				ty := reIdentHead.FindString(strings.TrimLeft(c[loc[1]:], " \t"))
				if ty == "" {
					out = append(out, fmt.Sprintf("seams: %s: %s:%d: a tracked type whose name the scan cannot read (a macro-written name such as `$n`, or the name on a later line), so no check can be held to it: write `tracked struct NAME` on one line", nc, file, i+1))
					continue
				}
				out = append(out, v.tracked(seen, ty, file, i+1, how)...)
			}
		}
	}
	for _, e := range es {
		// A seam with no overrides line is `trusted` and cites nothing.
		if !reTrackedType.MatchString(e.hc) {
			continue
		}
		cs, _ := r.citations(e.stat)
		var cited []string
		for _, c := range cs {
			if c.tag == r.NotCopyTag {
				cited = append(cited, c.target)
			}
		}
		if len(cited) == 0 {
			continue
		}
		ty := ""
		if m, ok := match(reOwnType, e.hc); ok {
			ty = sub(reWordHead, "", m)
		}
		ok := false
		for _, t := range cited {
			ok = ok || v.notCopy(ty, t)
		}
		if !ok {
			out = append(out, fmt.Sprintf("seams: %s: %s (%s:%d): a tracked type, whose status cites no %s that checks it is neither Copy nor Clone (%s, %s)", path.Join(r.Ledger, e.file)+".overrides", strings.TrimPrefix(e.okey, e.file+":"), e.file, e.line, r.NotCopyTag, how(ty), nc))
		}
	}
	return out
}
