package main

import (
	"fmt"
	"path"
	"strings"
)

// namedText is one file of the scan's input: its path from the root and
// its text.
type namedText struct{ name, text string }

// input is what the scan reads: the overrides files, the source files
// (test files left out), the test files (read only for unsafe impls),
// the pair macro's file, and the files that could not be read.
type input struct {
	overrides, sources, tests []namedText
	pairText                  string
	unreadable                []string
}

// result is what a scan gives.
type result struct {
	entries []*entry
	// errs go to stderr, in order; code is the exit status they call for
	// (0 when there are none).
	errs []string
	code int
	// drift: one line per seam whose overrides line disagrees with its
	// hashes or breaks a claims nothing rule (write and check modes).
	drift   []string
	summary string
	// ledger: file under the ledger directory (src/a/b.rs.md) to text.
	ledger map[string]string
}

type override struct {
	file, name, cat, tool, stat, hash string
	seen                              bool
}

var kinds = []string{"external_body", "external_body proof", "external_body type", "external_body const", "axiom", "assume_spec", "ext_type_spec", "ext_trait_spec", "spec_impl", "no_decreases", "assume_new", "ghost_assume_new", "unsupported-attribute"}

// generate scans in and renders the seams: the summary tables when
// hashes is false, else the ledger files with each seam's hashes and the
// drift lines.
func generate(r *Rules, in input, hashes bool) *result {
	res := &result{}
	bad := false
	// The overrides: name<TAB>category<TAB>tool[<TAB>status[<TAB>hashes]].
	ov := map[string]*override{}
	ovline := map[string]int{}
	var okeys []string
	prefix := r.Ledger + "/"
	for _, f := range in.overrides {
		src := strings.TrimSuffix(strings.TrimPrefix(f.name, prefix), ".overrides")
		for i, l := range splitLines(f.text) {
			if strings.HasPrefix(l, "#") || strings.Trim(l, " \t") == "" {
				continue
			}
			fl := strings.Split(l, "\t")
			if len(fl) < 3 {
				res.errs = append(res.errs, fmt.Sprintf("%s:%d: want name<TAB>category<TAB>tool", f.name, i+1))
				bad = true
				continue
			}
			k := src + ":" + fl[0]
			o := &override{file: f.name, name: fl[0], cat: fl[1], tool: fl[2]}
			if len(fl) >= 4 {
				o.stat = fl[3]
			}
			if len(fl) >= 5 {
				o.hash = fl[4]
			}
			if len(fl) > 5 {
				res.errs = append(res.errs, fmt.Sprintf("%s:%d: want at most name<TAB>category<TAB>tool<TAB>status<TAB>hashes", f.name, i+1))
				bad = true
			}
			// A second line for one name would silently replace the
			// first (seams.sh kept the last): refuse it.
			if first, dup := ovline[k]; dup {
				res.errs = append(res.errs, fmt.Sprintf("%s:%d: %s: a second line for this name (the first is line %d)", f.name, i+1, fl[0], first))
				bad = true
				continue
			}
			ovline[k] = i + 1
			ov[k] = o
			okeys = append(okeys, k)
		}
	}
	for _, f := range in.unreadable {
		res.errs = append(res.errs, "seams: cannot read "+f)
		bad = true
	}

	s := newScanner(r)
	if r.PairMacro != "" {
		for _, v := range r.PairVars {
			tp := readPairAttrs(in.pairText, r.PairMacro, v)
			if tp.dup != 0 {
				s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: a second `fn $%s(` in macro_rules! %s (the first is line %d): the rules read the first one's attributes only (give each template its own VAR)", r.PairFile, tp.dup, v, r.PairMacro, tp.found))
			}
			if tp.split != 0 {
				s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: the `fn $%s(` of macro_rules! %s runs over lines, so the scan cannot read its attributes (write `fn $%s(` on one line)", r.PairFile, tp.split, v, r.PairMacro, v))
			}
			s.pairTmpl = append(s.pairTmpl, tp)
		}
	}
	for _, f := range in.sources {
		s.newFile(f.name)
		for _, l := range splitLines(f.text) {
			s.ln++
			c := s.code(l)
			s.typeImpl(c)
			before, beforeDef := s.pairln, s.pairdef
			s.line(l, c)
			s.pairCollect(before, beforeDef, c)
		}
		s.endFile(true)
		if r.PairMacro != "" {
			s.pairUses()
		}
	}
	for _, f := range in.tests {
		s.file = f.name
		s.ln = 0
		s.lex = lexer{}
		for _, l := range splitLines(f.text) {
			s.ln++
			s.typeImpl(s.code(l))
		}
		s.endFile(false)
	}
	if len(s.errs) > 0 {
		res.errs = append(res.errs, s.errs...)
		bad = true
	}
	if bad {
		res.code = 2
		return res
	}

	// A fmt-trait spec impl (DisplaySpecImpl, ...) is no seam when its
	// type's trait impl is inside `verus!` in the same file.
	var es []*entry
	for _, e := range s.entries {
		if e.fmtKey != "" && s.verifiedImpl[e.file+"\x1c"+e.fmtKey] {
			continue
		}
		es = append(es, e)
	}
	res.entries = es

	catok := map[string]bool{}
	for _, c := range r.Categories {
		catok[c] = true
	}
	toolok := map[string]bool{}
	for _, t := range r.Tools {
		toolok[t] = true
	}
	nocc := map[string]int{}
	ncat, ntool, nkind, ct := map[string]int{}, map[string]int{}, map[string]int{}, map[string]int{}
	for _, e := range es {
		key := e.file + ":" + e.name
		nocc[key]++
		keyn := fmt.Sprintf("%s#%d", key, nocc[key])
		ok := ""
		if _, in := ov[keyn]; in {
			ok = keyn
		} else if _, in := ov[key]; in {
			ok = key
		}
		e.okey = ok
		if ok != "" {
			o := ov[ok]
			o.seen = true
			e.cat, e.tool, e.stat = o.cat, o.tool, o.stat
			if e.stat == "" {
				e.stat = "trusted"
			}
			if e.tool == "" {
				e.tool = r.toolOf(e.cat)
			}
		} else {
			e.cat = r.guess(e)
			e.tool = r.toolOf(e.cat)
			e.stat = "trusted"
		}
		if !catok[e.cat] {
			res.errs = append(res.errs, fmt.Sprintf("seams: %s:%d: %s: no category (add it to %s%s.overrides)", e.file, e.line, e.name, prefix, e.file))
			bad = true
		} else if !toolok[e.tool] {
			res.errs = append(res.errs, fmt.Sprintf("seams: %s:%d: %s: unknown tool %s", e.file, e.line, e.name, e.tool))
			bad = true
		}
		ncat[e.cat]++
		ntool[e.tool]++
		nkind[e.kind]++
		ct[e.cat+"\x1c"+e.tool]++
	}
	for _, k := range okeys {
		if !ov[k].seen {
			res.errs = append(res.errs, fmt.Sprintf("seams: %s: %s matches no seam", ov[k].file, k[strings.Index(k, ":")+1:]))
			bad = true
		}
	}
	if hashes {
		res.drift = hashDrift(r, es, ov, s.unsafeImpl)
	}
	if bad {
		res.code = 1
		return res
	}

	if !hashes {
		var b strings.Builder
		nf := filesWithSeams(es)
		fmt.Fprintf(&b, "%d %s, in %d %s\n\n", len(es), plural(len(es), "seam", "seams"), nf, plural(nf, "source file.", "source files."))
		b.WriteString("| category | seams |\n|---|---:|\n")
		for _, c := range r.Categories {
			if ncat[c] > 0 {
				fmt.Fprintf(&b, "| %s | %d |\n", c, ncat[c])
			}
		}
		b.WriteString("\n| tool | seams |\n|---|---:|\n")
		for _, t := range r.Tools {
			if ntool[t] > 0 {
				fmt.Fprintf(&b, "| %s | %d |\n", t, ntool[t])
			}
		}
		b.WriteString("\n| kind | seams |\n|---|---:|\n")
		for _, k := range kinds {
			if nkind[k] > 0 {
				fmt.Fprintf(&b, "| %s | %d |\n", k, nkind[k])
			}
		}
		b.WriteString("\n| category | tool | seams |\n|---|---|---:|\n")
		for _, c := range r.Categories {
			for _, t := range r.Tools {
				if n := ct[c+"\x1c"+t]; n > 0 {
					fmt.Fprintf(&b, "| %s | %s | %d |\n", c, t, n)
				}
			}
		}
		res.summary = b.String()
		return res
	}

	// One ledger file per source file; entries are grouped by file, in
	// source order. A name that several seams of one file share gets #N.
	nid := map[string]int{}
	for _, e := range es {
		key := e.file + ":" + e.name
		e.id = e.name
		if nocc[key] > 1 {
			nid[key]++
			e.id = fmt.Sprintf("%s#%d", e.name, nid[key])
		}
	}
	res.ledger = map[string]string{}
	for i := 0; i < len(es); {
		sf := es[i].file
		last := i
		for last+1 < len(es) && es[last+1].file == sf {
			last++
		}
		res.ledger[sf+".md"] = ledgerFile(r, sf, es[i:last+1])
		i = last + 1
	}
	return res
}

func plural(n int, one, many string) string {
	if n == 1 {
		return one
	}
	return many
}

func filesWithSeams(es []*entry) int {
	c, p := 0, ""
	for _, e := range es {
		if e.file != p {
			c++
			p = e.file
		}
	}
	return c
}

// relDoc is the path from the directory of the ledger file of sf to the
// rules' SEAMS.md.
func relDoc(r *Rules, sf string) string {
	from := strings.Split(path.Dir(path.Join(r.Ledger, sf)), "/")
	to := strings.Split(r.SeamsDoc, "/")
	i := 0
	for i < len(from) && i < len(to)-1 && from[i] == to[i] {
		i++
	}
	return strings.Repeat("../", len(from)-i) + strings.Join(to[i:], "/")
}

var (
	reFirstSentence = re("\\. .*$")
	reFirstWord     = re(" .*$")
	reNoteEnd       = re("; $")
)

func ledgerFile(r *Rules, sf string, es []*entry) string {
	var b strings.Builder
	base := ""
	if i := strings.LastIndex(sf, "/"); i >= 0 {
		base = sf[i+1:]
	}
	fmt.Fprintf(&b, "# Seams in `%s`\n\n", sf)
	fmt.Fprintf(&b, "Generated by `%s`; do not edit it by hand.\n", r.Command)
	fmt.Fprintf(&b, "[%s](%s) says what a seam, a category and a tool\n", path.Base(r.SeamsDoc), relDoc(r, sf))
	fmt.Fprintf(&b, "are. Set a category, tool or status in `%s.overrides` next to this\nfile.\n\n", base)
	b.WriteString("| item | kind | category | tool | status |\n|---|---|---|---|---|\n")
	for _, e := range es {
		tail := fmt.Sprintf(" | %s | %s | %s | %s |", e.kind, e.cat, e.tool, sub(reFirstWord, "", e.stat))
		fmt.Fprintf(&b, "| `%s`%s\n", md(short(e.id, 80-len(tail)-5)), tail)
	}
	for _, e := range es {
		fmt.Fprintf(&b, "\n## `%s`\n\n", e.id)
		fmt.Fprintf(&b, "%s:%d. Kind: %s. Category: %s.\n", sf, e.line, e.kind, e.cat)
		fmt.Fprintf(&b, "Tool: %s. Status: %s.\n", e.tool, e.stat)
		if d := sub(reFirstSentence, ".", e.doc); d != "" {
			fmt.Fprintf(&b, "\n> %s\n", d)
		}
		if e.note != "" {
			fmt.Fprintf(&b, "\nBody %s\n", sub(reNoteEnd, ".", e.note))
		}
		if e.hash != "" {
			fmt.Fprintf(&b, "\nHashes: %s.\n", e.hash)
		}
		fmt.Fprintf(&b, "\n```rust\n%s\n```\n", e.contract)
	}
	return b.String()
}

var (
	reClaimsNothing   = re("^trusted \\(claims nothing")
	reClaimsReason    = re("^trusted \\(claims nothing: [^ )].*\\)$")
	reDischarged      = re("^discharged")
	rePartly          = re("^trusted \\(partly checked")
	reExtBodyAttr     = re("#[ \t]*\\[[ \t]*verifier[ \t]*(::[ \t]*external_body|\\([ \t]*external_body[ \t]*\\))[ \t]*\\]")
	reClaimingClause  = re("(^|[^A-Za-z0-9_])(requires|ensures|returns|[a-z_]*_recursive_types[a-z_]*)([^A-Za-z0-9_]|$)")
	reTrackedType     = re("(^|[^A-Za-z0-9_])tracked (struct|enum) ")
	reOwnType         = re("(struct|enum) [A-Za-z_][A-Za-z0-9_]*")
	reUpToFirstComma  = re(",.*$")
	reGenericTailName = re("<.*$")
)

// hashDrift sets each seam's hashes, `contract C, body B`: the POSIX
// cksum CRCs of its contract (its attributes and the signature with its
// clauses) and its body, each without `//` comments and with whitespace
// runs made one space; `body none` for an item with no body. It returns
// one line per seam whose overrides line disagrees: a status that cites
// a check (discharged, or trusted and partly checked) or says the item
// claims nothing must record the hashes of the item in a fifth column,
// equal to the item's now; any other status records none. The claims
// nothing rules add theirs.
func hashDrift(r *Rules, es []*entry, ov map[string]*override, unsafeImpl map[string]string) []string {
	var drift []string
	for _, e := range es {
		c := fmt.Sprint(cksum(norm(e.hc)))
		b := "none"
		if hb := norm(e.hb); hb != "" {
			b = fmt.Sprint(cksum(hb))
		}
		e.hash = "contract " + c + ", body " + b
		var o *override
		rec := ""
		if e.okey != "" {
			o = ov[e.okey]
			rec = o.hash
		}
		cn := reClaimsNothing.MatchString(e.stat)
		if cn {
			drift = append(drift, claimsNothing(r, e, o, unsafeImpl)...)
		}
		chk := reDischarged.MatchString(e.stat) || rePartly.MatchString(e.stat) || cn
		// A status that says the item was checked names the check, by
		// one of the rules' cite tags, for the port's citation checks to
		// resolve (seams.sh's "cites no check").
		if chk && !cn && !r.citesCheck(e.stat) {
			drift = append(drift, fmt.Sprintf("seams: %s: %s: the status cites no check", o.file, o.name))
		}
		switch {
		case chk && rec == "":
			what := "cites a check"
			if cn {
				what = "claims nothing"
			}
			drift = append(drift, fmt.Sprintf("seams: %s: %s (%s:%d): the status %s but records no hashes; once it holds for the item as it is, add them as a fifth column: %s", o.file, o.name, e.file, e.line, what, e.hash))
		case chk && rec != e.hash:
			rc := sub(reUpToFirstComma, "", rec)
			nc := sub(reUpToFirstComma, "", e.hash)
			what := "body"
			if rc != nc {
				what = "contract"
				if substrFrom(rec, len(rc)+1) != substrFrom(e.hash, len(nc)+1) {
					what = "contract and body"
				}
			}
			drift = append(drift, fmt.Sprintf("seams: %s: %s (%s:%d): its %s changed since its status was checked (recorded %s; now %s): revisit the status, then record the hashes", o.file, o.name, e.file, e.line, what, rec, e.hash))
		case !chk && rec != "":
			drift = append(drift, fmt.Sprintf("seams: %s: %s: hashes on a status that cites no check", o.file, o.name))
		}
	}
	return drift
}

// claimsNothing checks a status `trusted (claims nothing: <why>)`: it is
// for an item that states nothing a check could test, an opaque type (an
// external type specification or an external_body struct or enum) or an
// external_body constant, with no `requires`, `ensures`, `returns` or
// `_recursive_types` attribute in its contract, a reason, and the tool
// `none (assumption)`. Not a function, a `tracked` type, or a type with
// an `unsafe impl` of Send or Sync anywhere under the source directory.
// One line per rule the seam breaks.
func claimsNothing(r *Rules, e *entry, o *override, unsafeImpl map[string]string) []string {
	var out []string
	at := fmt.Sprintf("seams: %s: %s (%s:%d): ", o.file, o.name, e.file, e.line)
	add := func(m string) { out = append(out, at+m) }
	if !reClaimsReason.MatchString(e.stat) {
		add("a claims nothing status gives no reason, want `trusted (claims nothing: <why>)`")
	}
	if e.kind != "ext_type_spec" && e.kind != "external_body type" && e.kind != "external_body const" {
		add("claims nothing, but its kind (" + e.kind + ") may not: only an opaque type or an external_body constant may")
	}
	if !reExtBodyAttr.MatchString(e.hc) {
		add("claims nothing, but the item is not external_body, so Verus sees inside it")
	}
	if reClaimingClause.MatchString(e.hc) {
		add("claims nothing, but its contract holds requires, ensures, returns or a _recursive_types attribute")
	}
	if r.citesCheck(e.stat) {
		add("claims nothing, but the status cites a check")
	}
	if e.tool != r.Assumption {
		add("claims nothing, but proposes the tool " + e.tool + ", want " + r.Assumption)
	}
	if reTrackedType.MatchString(e.hc) {
		add("claims nothing, but the item is a tracked type, a ghost token whose axioms may rely on it not being Copy")
	}
	if e.kind == "external_body const" {
		return out
	}
	nm := sub(reUpToColons, "", sub(reGenericTailName, "", e.name))
	own := ""
	if m, ok := match(reOwnType, e.hc); ok {
		own = sub(reWordHead, "", m)
	}
	if v, ok := unsafeImpl[nm]; ok {
		add("claims nothing, but the type " + nm + " has " + v + ", which rustc does not check")
	}
	if v, ok := unsafeImpl[own]; ok && own != "" && own != nm {
		add("claims nothing, but the type " + own + " has " + v + ", which rustc does not check")
	}
	return out
}
