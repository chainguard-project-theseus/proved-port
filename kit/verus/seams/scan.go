package main

import (
	"fmt"
	"regexp"
	"strings"
)

// entry is one seam, as seams.sh's arrays held it.
type entry struct {
	file, kind, name string
	line             int
	attrs, doc       string
	body             string // code() of the body lines, each after a space
	note             string
	contract         string // the contract as printed, verbatim lines
	fmtKey           string // a fmt-trait spec impl's trait key
	hc, hb           string // contract and body without comments, for the hashes
	ind              int
	// set by render
	cat, tool, stat, okey, id, hash string
}

// scanner is the line-by-line state of seams.sh's awk program.
type scanner struct {
	rules   *Rules
	entries []*entry
	errs    []string
	// verifiedImpl[file \x1c traitKey]: a trait impl inside verus!.
	verifiedImpl map[string]bool
	// unsafeImpl[type]: `FILE:LINE (unsafe impl T)` items joined by `, `.
	unsafeImpl map[string]string
	pairTmpl   []pairTemplate // the export macro's template per VAR
	lex        lexer          // the current file's literals and comments

	file string
	ln   int
	raw  string // $0

	depth, skipd               int
	skip, cap                  bool
	attrs, hattrs, doc, uattrs string
	at, ah, au                 string
	aopen                      int
	pendtest                   int
	lastfn, pendimpl           string
	inverus                    bool
	verusd                     int
	pairln, paird              int
	pairAcc                    string // a list form's names read so far
	pairAccAt                  int
	pairAccAttrs               string       // the declaration's attributes
	fileToks                   []ptok       // the file's tokens, for pairUses
	pairUse                    map[int]bool // lines the line reader took as macro uses
	pairDefs                   int          // definitions read in the rules' FILE
	pairToks                   []ptok       // the open invocation's tokens
	pairGot                    []string     // the names its lines gave
	pairdef                    bool         // in the pair macro's macro_rules! body
	pairdefd                   int
	implname                   []string
	impld                      []int
	cur                        *entry
	phase, pd, bd, bodyd       int
	clauses, wholebody         bool

	// Where the open state began, for endFile.
	lexAt, attrAt, pendAt, skipAt int
}

func newScanner(r *Rules) *scanner {
	return &scanner{rules: r, verifiedImpl: map[string]bool{}, unsafeImpl: map[string]string{}}
}

func (s *scanner) qual(n string) string {
	if len(s.implname) > 0 {
		return s.implname[len(s.implname)-1] + "::" + n
	}
	return n
}

func unchecked(e *entry) bool { return strings.HasPrefix(e.kind, "external_body") }

// begin starts an entry at the current line.
func (s *scanner) begin(kind, name string) *entry {
	ind := len(s.raw) - len(sub(reLead, "", s.raw))
	e := &entry{file: s.file, line: s.ln, kind: kind, name: name, attrs: s.attrs, doc: s.doc, hc: s.hattrs, ind: ind}
	s.entries = append(s.entries, e)
	return e
}

func addline(e *entry, raw string) {
	if strings.Trim(substr(raw, 1, e.ind), " \t") == "" {
		raw = substrFrom(raw, e.ind+1)
	}
	if e.contract != "" {
		e.contract += "\n"
	}
	e.contract += raw
}

var reClause = re("^(requires|ensures|recommends|returns|decreases|opens_invariants|no_unwind|default_ensures)([ \t]|$)")

// feed feeds one line of an item being captured; it returns true once
// the item ends. Phase 1 is the signature and clauses (the contract),
// phase 2 the body.
func (s *scanner) feed(raw, c string) bool {
	cur := s.cur
	t := trim(c)
	r := strip(raw, c)
	if s.phase == 1 {
		if s.pd == 0 && s.bd == 0 && reClause.MatchString(t) {
			s.clauses = true
		}
		cut := 0
		for i := 1; i <= len(c); i++ {
			ch := c[i-1]
			if ch == '(' || ch == '[' {
				s.pd++
			} else if ch == ')' || ch == ']' {
				s.pd--
			} else if ch == '}' {
				s.bd--
			} else if ch == '{' {
				if s.pd == 0 && s.bd == 0 && !s.wholebody && (!s.clauses || trim(substr(c, 1, i-1)) == "") {
					cut = i
					break
				}
				s.bd++
			} else if ch == ';' && s.pd == 0 && s.bd == 0 {
				addline(cur, raw)
				cur.hc += " " + r
				return true
			}
		}
		if s.wholebody && s.bd == 0 && s.pd == 0 && strings.HasSuffix(t, "}") {
			addline(cur, raw)
			cur.hc += " " + r
			return true
		}
		if cut == 0 {
			addline(cur, raw)
			cur.hc += " " + r
			return false
		}
		if trim(substr(raw, 1, cut-1)) != "" {
			addline(cur, substr(raw, 1, cut-1))
		}
		// Split raw and c at the brace, both column for column, before
		// stripping comments: r is shorter than raw once a comment is
		// dropped, so cutting r at c's column would move the split.
		cur.hc += " " + strip(substr(raw, 1, cut-1), substr(c, 1, cut-1))
		cur.contract = sub(reTrail, "", cur.contract)
		s.phase = 2
		s.bodyd = 0
		r = strip(substrFrom(raw, cut), substrFrom(c, cut))
		c = substrFrom(c, cut)
	}
	s.bodyd += strings.Count(c, "{") - strings.Count(c, "}")
	cur.body += " " + c
	cur.hb += " " + r
	return s.bodyd <= 0
}

// unsupported starts an entry for an item marked
// `#[verifier::assume_unsupported_attributes]`, at line at: its contract
// is every outer attribute line of the item and the item's line.
func (s *scanner) unsupported(name string, at int, contract string) {
	e := s.begin("unsupported-attribute", name)
	e.line = at
	e.contract = contract
	e.hc = contract
	e.doc = s.rules.UnsupDoc
}

var (
	reDocLine = re("^///")
	reDocHead = re("^/// ?")
	// `#`, `!` and `[` are separate tokens, so blanks may stand between
	// them (`# [verifier::external_body]`).
	// (reOuterAttr, an outer attribute only, is resolve.go's.)
	reAttrStart = re("^#[ \t]*!?[ \t]*\\[")
	// (reCfgTest1 already reads blanks after `#`.)
	reCfgTest1 = re("^#[ \t]*\\[[ \t]*cfg[ \t]*\\([ \t]*(all[ \t]*\\([ \t]*)?test[ \t]*[,)]")
	reCfgTest2 = re("^#\\[cfg\\(test\\)\\]")
	reFnLine   = re("(^|[ \t])fn [A-Za-z_]")
	// The first line of an item that has a body (besides fn and impl):
	// after `#[cfg(test)]` the scan waits for that body on a later line.
	reItemHead     = re("^(pub(\\([^)]*\\))?[ \t]+)?((unsafe|auto)[ \t]+)*(mod|trait|struct|enum|union)[ \t]+[A-Za-z_]|^macro_rules![ \t]*[A-Za-z_]")
	reSemiEnd      = re(";$")
	reIsFn         = re("(^|[ \t])fn [A-Za-z_][A-Za-z0-9_]*")
	reFnHead       = re("^.*fn ")
	reUseHead      = re("^use ")
	reParens       = re("\\(.*\\)")
	reStructHead   = re("^.*struct ")
	reIdentTail    = re("[^A-Za-z0-9_].*$")
	reTraitHead    = re("^.*trait ")
	reSpecImplLine = re("^impl[< ].*SpecImpl(<[^>]*>)? for ")
	reBraceEnd     = re(" *\\{$")
	reModsHead     = re("^([a-z_]+::)+")
	reTypeLine     = re("(^|[ \t])(struct|enum) ")
	reTypeHead     = re("^.*(struct|enum) ")
	reConstLine    = re("(^|[ \t])(const|static) [A-Z_][A-Z0-9_]*[:<]")
	reConstName    = re("(const|static) [A-Z_][A-Z0-9_]*")
	reWordHead     = re("^[a-z]+ ")
	reAxiomFn      = re("(^|[ \t])axiom fn ")
	reProofFn      = re("(^|[ \t])proof fn ")
	reAxiomName    = re("^axiom_")
	reUnsupAttr    = re("verifier[ \t]*(::|\\()[ \t]*assume_unsupported_attributes")
	reExtBody      = re("verifier[ \t]*(::|\\()[ \t]*external_body")
	// An attribute that makes the item it is on a seam of some kind.
	reTrusting     = re("(^|[^A-Za-z0-9_])(external_body|external_fn_specification|external_type_specification|external_trait_specification|exec_allows_no_decreases_clause)([^A-Za-z0-9_]|$)")
	reUnsupItem    = re("(const|static)( mut)? [A-Z_a-z][A-Za-z0-9_]*")
	reLastWordHead = re("^.* ")
	rePairFn       = re("^fn [A-Za-z_][A-Za-z0-9_]* */ *[A-Za-z_][A-Za-z0-9_]*")
	reAssumeNew    = re("(Tracked|Ghost)::assume_new\\(\\)")
	reTrackedNew   = re("Tracked::assume_new")
	reGhostNew     = re("Ghost::assume_new\\(\\)")
	reTrackedNewC  = re("Tracked::assume_new\\(\\)")
	reFmtSpec      = re("^(Display|Debug|LowerHex|UpperHex|Octal|Binary|LowerExp|UpperExp|Pointer)SpecImpl")
	reImplLine     = re("^(pub(\\([a-z]+\\))? )?(unsafe )?impl[< ]")
	reVerusOpen    = re("^verus! *\\{")
	reUnsafeImpl   = re("(^|[^A-Za-z0-9_])unsafe impl(<.*>)? +(::)?((core|std)::marker::)?(Send|Sync) +for +(::)?[A-Za-z_][A-Za-z0-9_:]*")
	reForSpTail    = re(" +for .*$")
	reUpToSpColon  = re("^.*[ :]")
	reUpToFor      = re("^.* for +")
	reUpToColons   = re("^.*::")
)

// macroDef says whether t opens `macro_rules! NAME` for exactly that
// name: the name is followed by the line's end, a blank or the macro's
// opening bracket, not by any other byte (NAME_extra, NAMEé).
func macroDef(t, name string) bool {
	p := "macro_rules! " + name
	if !strings.HasPrefix(t, p) {
		return false
	}
	return len(t) == len(p) || strings.IndexByte(" \t{([", t[len(p)]) >= 0
}

// code lexes the current line with the file's lexer, noting the line
// where a string, raw string or block comment left open began.
func (s *scanner) code(raw string) string {
	open := s.lex.str != 0 || s.lex.depth > 0
	c := s.lex.code(raw)
	if !open && (s.lex.str != 0 || s.lex.depth > 0) {
		s.lexAt = s.ln
	}
	return c
}

// endFile fails a file the scan ends inside something in: a string,
// raw string or block comment, an attribute, an item being captured or
// skipped, or with its braces not balanced. Valid Rust ends with none
// open once literals and comments are blanked, so each is a misread
// that would otherwise leave seams out silently. source is false for a
// test file, read only for `unsafe impl`, where only the lexer counts.
func (s *scanner) endFile(source bool) {
	at := func(ln int, what string) {
		s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: the file ends inside %s that starts here, so the scan cannot read it whole", s.file, ln, what))
	}
	n := len(s.errs)
	switch {
	case s.lex.str == 1:
		at(s.lexAt, "a string")
	case s.lex.str == 2:
		at(s.lexAt, "a raw string")
	case s.lex.depth > 0:
		at(s.lexAt, "a block comment")
	}
	if !source {
		return
	}
	switch {
	case s.aopen > 0:
		at(s.attrAt, "an attribute")
	case s.cap:
		at(s.cur.line, "an item")
	case s.skip:
		at(s.skipAt, "a test-only item")
	case s.pendtest != 0:
		at(s.pendAt, "a test-only item")
	}
	if len(s.errs) == n && s.depth != 0 {
		s.errs = append(s.errs, fmt.Sprintf("seams: %s: the file's braces do not balance (depth %d at its end), so the scan cannot read it whole", s.file, s.depth))
	}
}

// hashEnd says whether the code line t, read outside any attribute, ends
// in a `#` or `#!` (blanks allowed between and after): the start of an
// attribute whose `[` is on a later line. Rust joins the tokens across
// the line break; the scan does not, so a trusting attribute there would
// leave a seam out. A raw string's closing `"#` is no such `#`.
func hashEnd(t string) bool {
	s := strings.TrimRight(t, " \t")
	s = strings.TrimRight(strings.TrimSuffix(s, "!"), " \t")
	if !strings.HasSuffix(s, "#") {
		return false
	}
	return !strings.HasSuffix(strings.TrimRight(s, "#"), "\"")
}

func (s *scanner) hashEndErr() {
	s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: a line that ends in an attribute's `#` (alone or after other code), its `[` on a later line: the scan does not read it, so a trusting attribute there would leave a seam out (write `#[` on one line)", s.file, s.ln))
}

// unread reports a trusting attribute on an item line t the scan cannot
// read as a seam.
func (s *scanner) unread(t, attr string) {
	s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: %s on an item the scan does not read as a seam: %s (write the item plainly, e.g. no macro-written name, an UPPER_CASE const or static, a struct or enum)", s.file, s.ln, attr, t))
}

// attrEnd is where an item starts after the attributes on the code line
// c, which opens inside an attribute `depth` brackets deep (0: the line
// starts with `#[` or `#![`); 0 when the line holds no item after them.
func attrEnd(c string, depth int) int {
	for i := 0; i < len(c); i++ {
		ch := c[i]
		switch {
		case depth > 0 && ch == '[':
			depth++
		case depth > 0 && ch == ']':
			depth--
		case depth > 0:
		case ch == ' ' || ch == '\t' || ch == '#' || ch == '!':
		case ch == '[':
			depth++
		default:
			return i
		}
	}
	return 0
}

// newFile resets the per-file state (the awk program's newfile rule).
func (s *scanner) newFile(file string) {
	s.file = file
	s.ln = 0
	s.lex = lexer{}
	s.depth = 0
	s.implname, s.impld = nil, nil
	s.skip, s.cap = false, false
	s.attrs, s.hattrs, s.doc, s.uattrs = "", "", "", ""
	s.aopen = 0
	s.pendtest = 0
	s.lastfn, s.pendimpl = "", ""
	s.inverus = false
	s.pairln = 0
	s.pairAcc, s.pairAccAttrs = "", ""
	s.pairdef = false
	s.fileToks, s.pairUse = nil, map[int]bool{}
}

// typeImpl records an `unsafe impl` of Send or Sync on the current line,
// for every line of every source and test file (c is the line's code).
func (s *scanner) typeImpl(c string) {
	m, ok := match(reUnsafeImpl, c)
	if !ok {
		return
	}
	tr := sub(reUpToSpColon, "", sub(reForSpTail, "", m))
	n := sub(reUpToColons, "", sub(reUpToFor, "", m))
	p := ""
	if v, ok := s.unsafeImpl[n]; ok {
		p = v + ", "
	}
	s.unsafeImpl[n] = p + fmt.Sprintf("%s:%d (unsafe impl %s)", s.file, s.ln, tr)
}

func (s *scanner) popImpls() {
	for len(s.implname) > 0 && s.depth <= s.impld[len(s.impld)-1] {
		s.implname = s.implname[:len(s.implname)-1]
		s.impld = s.impld[:len(s.impld)-1]
	}
}

func assumeKind(c string) string {
	if reTrackedNew.MatchString(c) {
		return "assume_new"
	}
	return "ghost_assume_new"
}

// line scans one source line (the awk program's main rule); c is its
// code, from the file's lexer.
func (s *scanner) line(raw, c string) {
	s.raw = raw
	t := trim(c)
	rawt := trim(raw)
	opens, closes := strings.Count(c, "{"), strings.Count(c, "}")

	if s.skip {
		s.depth += opens - closes
		if s.depth <= s.skipd {
			s.skip = false
		}
		return
	}

	if s.cap {
		cur := s.cur
		if s.phase == 2 && reAssumeNew.MatchString(c) && !unchecked(cur) {
			e := s.begin(assumeKind(c), cur.name)
			e.contract = rawt
			e.hc = strip(raw, c)
		}
		if s.phase == 2 && unchecked(cur) {
			if reGhostNew.MatchString(c) {
				cur.note += fmt.Sprintf("mints Ghost at line %d; ", s.ln)
			}
			if reTrackedNewC.MatchString(c) {
				cur.note += fmt.Sprintf("mints Tracked at line %d; ", s.ln)
			}
		}
		if s.feed(raw, c) {
			s.cap = false
		}
		s.depth += opens - closes
		s.popImpls()
		return
	}

	if reDocLine.MatchString(rawt) {
		d := sub(reDocHead, "", rawt)
		if d != "" {
			if s.doc != "" {
				s.doc += " "
			}
			s.doc += d
		}
		return
	}
	if t == "" {
		return
	}
	if s.aopen == 0 && hashEnd(t) {
		s.hashEndErr()
		return
	}
	// An attribute, read through its closing bracket when it spans lines.
	if s.aopen > 0 || reAttrStart.MatchString(t) {
		// An item after the last attribute on this line (`#[a] fn f()`,
		// `#[cfg(test)] mod t;`): the attributes are read, then the rest
		// as a line of its own at this line's indentation. seams.sh took
		// the whole line as the attribute and never read the item.
		if cut := attrEnd(c, s.aopen); cut > 0 && cut <= len(raw) {
			ind := raw[:len(raw)-len(strings.TrimLeft(raw, " \t"))]
			s.line(raw[:cut], c[:cut])
			s.line(ind+strings.TrimLeft(raw[cut:], " \t"), ind+strings.TrimLeft(c[cut:], " \t"))
			return
		}
		if s.aopen > 0 {
			st := trim(strip(raw, c))
			s.at += " " + t
			s.ah += " " + st
			s.au += " " + st
		} else {
			s.at = t
			s.ah = strip(raw, c)
			s.au = trim(strip(raw, c))
			s.attrAt = s.ln
		}
		s.aopen += strings.Count(c, "[") - strings.Count(c, "]")
		if s.aopen > 0 {
			return
		}
		s.aopen = 0
		if hashEnd(t) {
			s.hashEndErr()
		}
		s.attrs += " " + s.at
		s.hattrs += " " + s.ah
		if reOuterAttr.MatchString(s.at) {
			s.uattrs += s.au + "\n"
		}
		if reCfgTest1.MatchString(s.at) || reCfgTest2.MatchString(s.at) {
			s.pendtest = 1
			s.pendAt = s.ln
		}
		return
	}

	// A test-only item is skipped from its first line through its body,
	// which may open lines below its first (pendtest 2): a multi-line fn
	// signature, `mod t` then `{`, an impl with a `where` line. Only a
	// first line that starts an item with a body (fn, impl, mod, trait,
	// struct, enum, union, macro_rules!) and does not end in `;` waits;
	// any other first line (a field, variant, parameter or match arm) is
	// the whole item, its continuation lines read as code (that can
	// count too much, never too little). A line
	// that closes more braces than it opens ends the pending item and is
	// read as code (it closes the enclosing block: `B` / `}` after a
	// test-only last variant).
	if s.pendtest != 0 {
		s.attrs, s.hattrs, s.doc, s.uattrs = "", "", "", ""
		if opens > closes {
			s.pendtest = 0
			s.skip = true
			s.skipAt = s.ln
			s.skipd = s.depth
			s.depth += opens - closes
			return
		}
		if opens > 0 && opens == closes {
			// A one-line item (`fn g() {}`): it ends here. seams.sh read
			// it as a multi-line signature and skipped on to the next
			// line that opens a brace, dropping the seams between.
			s.pendtest = 0
			return
		}
		if closes > opens {
			s.pendtest = 0
		} else {
			head := reFnLine.MatchString(t) || reImplLine.MatchString(t) || reItemHead.MatchString(t)
			if s.pendtest == 1 && !reSemiEnd.MatchString(t) && head {
				s.pendtest = 2
			} else if s.pendtest == 1 || reSemiEnd.MatchString(t) {
				s.pendtest = 0
			}
			return
		}
	}

	kind, name := "", ""
	s.wholebody = false
	fnm, isfn := match(reIsFn, t)
	fname := ""
	if isfn {
		fname = sub(reFnHead, "", fnm)
	}
	attrs := s.attrs
	has := func(x string) bool { return strings.Contains(attrs, x) }
	// `#[verifier::external_body]` or `#[verifier(external_body)]`.
	extBody := reExtBody.MatchString(attrs)
	switch {
	case strings.Contains(t, "assume_specification") && !reUseHead.MatchString(t):
		kind, name = "assume_spec", specPath(t)
	case has("external_type_specification"):
		kind = "ext_type_spec"
		if m, ok := match(reParens, t); ok {
			name = m[1 : len(m)-1]
		} else {
			name = sub(reIdentTail, "", sub(reStructHead, "", t))
		}
	case has("external_trait_specification"):
		kind = "ext_trait_spec"
		name = sub(reIdentTail, "", sub(reTraitHead, "", t))
		s.wholebody = true
	case isfn && has("external_fn_specification"):
		// The older form of assume_specification: a wrapper fn whose
		// contract Verus takes for the external fn its body calls.
		// seams.sh did not read it.
		kind, name = "assume_spec", s.qual(fname)
	case reSpecImplLine.MatchString(t):
		kind = "spec_impl"
		name = sub(reImplHead, "", t)
		name = sub(reBraceEnd, "", name)
		name = sub(reModsHead, "", name)
		name = strings.Replace(name, " for crate::", " for ", 1)
		s.wholebody = true
	case extBody && reTypeLine.MatchString(t):
		kind = "external_body type"
		name = s.qual(sub(reIdentTail, "", sub(reTypeHead, "", t)))
	case !isfn && extBody && reConstLine.MatchString(t):
		m, _ := match(reConstName, t)
		kind = "external_body const"
		name = s.qual(sub(reWordHead, "", m))
	case isfn && reAxiomFn.MatchString(t):
		kind, name = "axiom", s.qual(fname)
	case isfn && extBody && reProofFn.MatchString(t):
		kind, name = "external_body proof", s.qual(fname)
	case isfn && extBody:
		kind, name = "external_body", s.qual(fname)
	case isfn && reAxiomName.MatchString(fname) && reProofFn.MatchString(t):
		kind, name = "axiom", s.qual(fname)
	case isfn && has("exec_allows_no_decreases_clause"):
		kind, name = "no_decreases", s.qual(fname)
	}
	if isfn {
		s.lastfn = s.qual(fname)
	}

	// Unchecked symbol and section attributes: an entry beside any other
	// seam of the item. A pair macro invocation writes two exports per
	// `fn a / b(..)` line.
	if reUnsupAttr.MatchString(attrs) {
		if isfn {
			s.unsupported(s.qual(fname), s.ln, s.uattrs+trim(strip(raw, c)))
		} else if m, ok := match(reUnsupItem, t); ok {
			s.unsupported(s.qual(sub(reLastWordHead, "", m)), s.ln, s.uattrs+trim(strip(raw, c)))
		} else if !s.pairdef {
			s.unread(t, "verifier::assume_unsupported_attributes")
		}
	}
	// A trusting attribute on an item no kind reads (a fn whose name a
	// macro writes, a lowercase const or static, a union, ...) would
	// leave a seam out of the ledger: fail instead (seams.sh dropped it).
	if kind == "" {
		if m, ok := match(reTrusting, attrs); ok {
			s.unread(t, strings.Trim(m, " \t#[]():,"))
		}
	}
	// A line directly inside an export macro invocation: each `fn` line
	// there names exports, in the rules' form. One the form does not
	// read would leave its exports out: fail instead.
	// A list form's names may run over lines (`fn f [` then one name a
	// line, as rustfmt writes them): read on through the `]`.
	// The declaration's own attributes (above its `fn` line) go with
	// the names, wherever the `]` falls.
	if s.pairAcc != "" {
		s.pairAcc += " " + t
		if strings.Contains(t, "]") {
			s.pairExports(s.pairAcc, s.pairAccAttrs)
			s.pairAcc, s.pairAccAttrs = "", ""
		}
	} else if s.pairln != 0 && s.depth == s.paird+1 && reFnLine.MatchString(t) {
		if s.rules.PairForm == "list" && reListOpen.MatchString(t) && !strings.Contains(t, "]") {
			s.pairAcc, s.pairAccAt, s.pairAccAttrs = t, s.ln, s.uattrs
		} else {
			s.pairExports(t, s.uattrs)
		}
	}

	if kind == "" && s.inverus && reAssumeNew.MatchString(c) {
		e := s.begin(assumeKind(c), s.lastfn)
		e.contract = rawt
		e.hc = strip(raw, c)
	}

	if kind != "" {
		s.cur = s.begin(kind, name)
		s.phase, s.pd, s.bd = 1, 0, 0
		s.clauses = false
		s.cap = !s.feed(raw, c)
		if kind == "spec_impl" && reFmtSpec.MatchString(name) {
			s.cur.fmtKey = traitKey(t)
		}
	}
	s.attrs, s.hattrs, s.doc, s.uattrs = "", "", "", ""

	if reImplLine.MatchString(t) {
		if s.inverus && strings.Contains(t, " for ") && !strings.Contains(t, "SpecImpl") {
			s.verifiedImpl[s.file+"\x1c"+traitKey(t)] = true
		}
		if opens > closes {
			s.implname = append(s.implname, implName(t))
			s.impld = append(s.impld, s.depth)
		} else if opens == 0 {
			s.pendimpl = implName(t)
		}
	} else if s.pendimpl != "" && opens > closes {
		s.implname = append(s.implname, s.pendimpl)
		s.impld = append(s.impld, s.depth)
		s.pendimpl = ""
	}
	if reVerusOpen.MatchString(t) {
		s.inverus = true
		s.verusd = s.depth
	}
	// The pair macro's own definition: its exports' attributes are read
	// at each invocation, so they are no unread item here, and a call
	// of itself in it (its first line included) is no invocation.
	if s.rules.PairMacro != "" && !s.pairdef && macroDef(t, s.rules.PairMacro) && opens > closes {
		s.pairdef = true
		s.pairdefd = s.depth
	}
	if s.rules.PairMacro != "" && !s.pairdef {
		u := pairInvocation(t, s.rules.PairMacro)
		if u != invNone {
			s.pairUse[s.ln] = true
		}
		switch u {
		case invOpen:
			if s.pairln != 0 {
				s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: a %s! invocation inside another, which the scan does not read: %s", s.file, s.ln, s.rules.PairMacro, t))
				break
			}
			s.pairln = s.ln
			s.paird = s.depth
		case invUnread:
			s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: a use of %s! the scan does not read as an invocation (write `%s! {` or `path::%s! {`, its fn lines below it): %s", s.file, s.ln, s.rules.PairMacro, s.rules.PairMacro, s.rules.PairMacro, t))
		}
	}
	// The definition ends at its closing brace, not at its line's end:
	// code after that brace on the line (`} exports! {`) would be read
	// as part of the definition, exempt. Refuse it.
	if s.pairdef {
		d := s.depth
		for i := 0; i < len(c); i++ {
			if c[i] == '{' {
				d++
			} else if c[i] == '}' {
				if d--; d <= s.pairdefd {
					if rest := trim(c[i+1:]); rest != "" && rest != ";" {
						s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: code after the closing brace of macro_rules! %s on its line, which the scan would read as part of the definition (start it on a line of its own): %s", s.file, s.ln, s.rules.PairMacro, rest))
					}
					break
				}
			}
		}
	}
	s.depth += opens - closes
	s.popImpls()
	if s.inverus && s.depth <= s.verusd {
		s.inverus = false
	}
	if s.pairln != 0 && s.depth <= s.paird {
		s.pairln = 0
		if s.pairAcc != "" {
			s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: the %s! invocation ends inside the list of names that starts here", s.file, s.pairAccAt, s.rules.PairMacro))
			s.pairAcc, s.pairAccAttrs = "", ""
		}
	}
	if s.pairdef && s.depth <= s.pairdefd {
		s.pairdef = false
	}
}

const (
	invNone   = iota // no use of the macro on the line
	invOpen          // the line opens an invocation the scan reads
	invUnread        // a use of the macro the scan cannot read
)

// pairInvocation reads the code line t for uses of the export macro.
// It opens an invocation when the line is `MACRO! {` with nothing after
// the brace, the name alone or at the end of a path (`width::MACRO! {`,
// `crate::width::MACRO! {`, `::m::MACRO! {`). Any other use (another
// delimiter, an expression, names on the line itself) would write
// exports the scan never reads, so it is invUnread. Blanks may stand
// between the name and its `!` (`MACRO ! {`; a comment there is blanks
// in the lexed line t).
func pairInvocation(t, macro string) int {
	found := false
	for i := 0; ; {
		j := strings.Index(t[i:], macro)
		if j < 0 {
			break
		}
		j += i
		i = j + len(macro)
		if j > 0 && isIdentByte(t[j-1]) {
			continue // another name that ends in this one
		}
		k := i
		for k < len(t) && (t[k] == ' ' || t[k] == '\t') {
			k++
		}
		if k >= len(t) || t[k] != '!' || k+1 < len(t) && t[k+1] == '=' {
			continue // the name, not a macro call (`use w::MACRO;`, MACRO_x)
		}
		i = k + 1
		found = true
		if p := t[:j]; p != "" && !rePathPrefix.MatchString(p) {
			return invUnread
		}
		rest := strings.TrimLeft(t[i:], " \t")
		if !strings.HasPrefix(rest, "{") || trim(rest[1:]) != "" {
			return invUnread
		}
	}
	if found {
		return invOpen
	}
	return invNone
}

func isIdentByte(c byte) bool {
	return c == '_' || c >= '0' && c <= '9' || c >= 'A' && c <= 'Z' || c >= 'a' && c <= 'z' || c >= 0x80
}

// pairExports reads the export names on an invocation's `fn` line t in
// the rules' form and writes an unsupported-attribute entry for each,
// with the declaration's own attribute lines (uattrs), the template's
// attributes and its item line; a line the form does not read, names
// that do not fit the VARs, or a template not found in the macro's file
// is an input error.
func (s *scanner) pairExports(t, uattrs string) {
	r := s.rules
	mac := r.PairMacro
	fail := func(format string, a ...any) {
		s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: ", s.file, s.ln)+fmt.Sprintf(format, a...))
	}
	var names []string
	switch r.PairForm {
	case "slash":
		if m, ok := match(rePairFn, t); ok {
			names = strings.Split(strings.ReplaceAll(m[3:], " ", ""), "/")
		}
	case "list":
		if m, ok := match(reListFn, t); ok {
			l := m[strings.IndexByte(m, '[')+1 : len(m)-1]
			for _, n := range strings.Split(l, ",") {
				if n = strings.Trim(n, " \t"); n != "" {
					names = append(names, n)
				}
			}
		}
	}
	if names == nil {
		form := "fn a / b(..)"
		if r.PairForm == "list" {
			form = "fn f [a, b, ...] (..)"
		}
		fail("a fn line in a %s! invocation that the %s form (%s, its names on that line) does not read: %s", mac, r.PairForm, form, t)
		return
	}
	if len(r.PairVars) != 1 && len(r.PairVars) != len(names) {
		fail("%s! names %d exports here, but the rules give %d template VARs (want one for every name, or one per name)", mac, len(names), len(r.PairVars))
		return
	}
	for i, n := range names {
		k := 0
		if len(r.PairVars) > 1 {
			k = i
		}
		tp := s.pairTmpl[k]
		if tp.attrs == "" {
			fail("no %s! attributes in %s", mac, r.PairFile)
		}
		item := tp.item + " fn "
		if tp.item == "" {
			item = "fn "
		}
		s.unsupported(n, s.pairln, uattrs+pairAttrs(tp.attrs)+item+n+" ("+mac+"!)")
		s.pairGot = append(s.pairGot, n)
	}
}

// ptok is a token of an invocation's code, with its line.
type ptok struct {
	s   string
	ln  int
	def bool // on a line of the export macro's definition
}

// pairUses reads a source file's tokens, so a name, its `!` and its
// `{` split over lines are seen as one use: a `macro_rules! MACRO` in
// a file other than the rules' FILE, or a second one in FILE, would be
// exempted while its exports take FILE's template; a `MACRO !` outside
// the definition on a line the line reader did not take as an
// invocation (or refuse), or `MACRO as`, would leave exports out. Each
// is an input error.
func (s *scanner) pairUses() {
	mac := s.rules.PairMacro
	t := s.fileToks
	at := func(k int) string {
		if k < len(t) {
			return t[k].s
		}
		return ""
	}
	bad := func(ln int, format string, a ...any) {
		s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: ", s.file, ln)+fmt.Sprintf(format, a...))
	}
	for k, x := range t {
		if x.s == "macro_rules" && at(k+1) == "!" && at(k+2) == mac {
			switch {
			case s.file != s.rules.PairFile:
				bad(x.ln, "a macro_rules! %s outside %s, the file the rules read its exports' attributes from (rename one, or name this file in the rules)", mac, s.rules.PairFile)
			case s.pairDefs > 0:
				bad(x.ln, "a second macro_rules! %s in %s: the rules read the attributes of the first only", mac, s.file)
			}
			if s.file == s.rules.PairFile {
				s.pairDefs++
			}
			continue
		}
		if x.def || x.s != mac || s.pairUse[x.ln] {
			continue
		}
		switch {
		case at(k+1) == "as":
			bad(x.ln, "%s renamed with `as`, so the scan cannot find its invocations (import and invoke it by its own name)", mac)
		case at(k+1) == "!" && at(k+2) != "=":
			bad(x.ln, "a use of %s! the scan does not read as an invocation (write `%s! {` with the name, its `!` and the `{` on one line, its fn lines below it)", mac, mac)
		}
	}
}

// tokens splits a lexed code line into identifiers (and numbers) and
// single punctuation bytes; a literal's blanked contents are an
// identifier between its quotes.
func tokens(c string, ln int) []ptok {
	var out []ptok
	for i := 0; i < len(c); {
		switch ch := c[i]; {
		case ch == ' ' || ch == '\t' || ch == '\r' || ch == '\f':
			i++
		case isIdentByte(ch):
			j := i
			for j < len(c) && isIdentByte(c[j]) {
				j++
			}
			out = append(out, ptok{s: c[i:j], ln: ln})
			i = j
		default:
			out = append(out, ptok{s: c[i : i+1], ln: ln})
			i++
		}
	}
	return out
}

// pairCollect follows an export macro invocation over lines: called
// after each source line with the invocation line before it, it keeps
// the tokens of the lines inside one and checks them when it closes.
func (s *scanner) pairCollect(before int, beforeDef bool, c string) {
	if s.rules.PairMacro == "" {
		return
	}
	// Every line's tokens, for pairUses; those of the definition's
	// lines marked.
	inDef := beforeDef || s.pairdef
	for _, k := range tokens(c, s.ln) {
		k.def = inDef
		s.fileToks = append(s.fileToks, k)
	}
	if before == 0 {
		if s.pairln == s.ln {
			s.pairToks, s.pairGot = nil, nil
		}
		return
	}
	s.pairToks = append(s.pairToks, tokens(c, s.ln)...)
	if s.pairln == 0 {
		s.pairCheck(before)
	}
}

// pairCheck reads a closed invocation's tokens as the macro's grammar
// in the rules' form, a sequence of declarations, each
//
//	#[attr]* fn NAME / NAME ...      (slash)
//	#[attr]* fn BODY [NAME, ...] ... (list)
//
// whose rest runs to a `;` outside brackets or through a `{...}` group
// (then an optional `;`), up to the invocation's closing `}`. The names
// it declares must be the names the line reader gave: a declaration the
// lines did not read (its `fn` and names split over lines, two on one
// line, a grammar with no `fn`, macro-written tokens) would otherwise
// leave exports out of the ledger.
func (s *scanner) pairCheck(at int) {
	t := s.pairToks
	mac := s.rules.PairMacro
	i := 0
	fail := func(what string) {
		ln := s.ln
		got := "the end of the file"
		if i < len(t) {
			ln, got = t[i].ln, "`"+t[i].s+"`"
		}
		s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: the %s! invocation at line %d does not read as %s declarations: want %s, got %s", s.file, ln, mac, at, s.rules.PairForm, what, got))
	}
	is := func(x string) bool { return i < len(t) && t[i].s == x }
	ident := func() (string, bool) {
		if i < len(t) && (t[i].s[0] == '_' || t[i].s[0] >= 'A' && t[i].s[0] <= 'Z' || t[i].s[0] >= 'a' && t[i].s[0] <= 'z') {
			i++
			return t[i-1].s, true
		}
		return "", false
	}
	var names []string
	for !is("}") {
		for is("#") {
			i++
			if is("!") {
				i++
			}
			if !is("[") {
				fail("`[` after `#`")
				return
			}
			d := 0
			for ; i < len(t); i++ {
				if t[i].s == "[" {
					d++
				} else if t[i].s == "]" {
					if d--; d == 0 {
						break
					}
				}
			}
			if i >= len(t) {
				fail("the attribute's `]`")
				return
			}
			i++
		}
		if !is("fn") {
			fail("`fn` (or `#[` or the invocation's `}`)")
			return
		}
		i++
		first, ok := ident()
		if !ok {
			fail("a name after `fn`")
			return
		}
		if s.rules.PairForm == "slash" {
			if !is("/") {
				fail("`/` after `fn " + first + "`")
				return
			}
			i++
			second, ok := ident()
			if !ok {
				fail("a second name after `/`")
				return
			}
			names = append(names, first, second)
		} else {
			if !is("[") {
				fail("`[` after `fn " + first + "`")
				return
			}
			i++
			for {
				n, ok := ident()
				if !ok {
					fail("a name in the list")
					return
				}
				names = append(names, n)
				if is(",") {
					i++
				} else if !is("]") {
					fail("`,` or `]` after a name")
					return
				}
				if is("]") {
					i++
					break
				}
			}
		}
		// The declaration's rest: to a `;` outside brackets, or through
		// a `{...}` group outside brackets.
		d := 0
		for {
			if i >= len(t) {
				fail("the declaration's end (`;` or a `{...}` body)")
				return
			}
			x := t[i].s
			i++
			if x == "(" || x == "[" || x == "{" {
				d++
				continue
			}
			if x == ")" || x == "]" || x == "}" {
				if d == 0 {
					i--
					fail("the declaration's end (`;` or a `{...}` body)")
					return
				}
				if d--; d == 0 && x == "}" {
					if is(";") {
						i++
					}
					break
				}
				continue
			}
			if x == ";" && d == 0 {
				break
			}
		}
	}
	if strings.Join(names, " ") != strings.Join(s.pairGot, " ") {
		s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: the %s! invocation declares %s, but its lines read as %s (write each declaration's `fn` and names on one line of their own)", s.file, at, mac, orNone(names), orNone(s.pairGot)))
	}
}

func orNone(n []string) string {
	if len(n) == 0 {
		return "no exports"
	}
	return strings.Join(n, ", ")
}

// pairAttrs is the `@@`-joined attribute lines one per line.
func pairAttrs(s string) string { return strings.ReplaceAll(s, "@@", "\n") }

// pairTemplate is an export's template in the macro's definition: the
// attribute lines above its `fn $VAR(` (`@@`-joined, "" when the macro
// or the line is missing) and the item's qualifiers before the `fn`
// (`pub extern "C"`, `pub unsafe extern "C"`).
type pairTemplate struct {
	attrs, item string
	found, dup  int // the template's line, and a second one's (0: none)
	split       int // a template whose head runs over lines (0: none)
}

// readPairAttrs is seams.sh's pair_attrs: the attribute lines the export
// macro writes directly above its `fn $VAR(` export, `$(#[$m])*`
// included, lines that hold only comments (`//` and block comments, a
// block comment over several lines too) or nothing skipped; any other
// line starts the list again. The file is read with the lexer, so a
// comment or literal never counts as code; an attribute's text has its
// comments dropped. An attribute that spans lines is read through its
// closing bracket.
func readPairAttrs(text, macro, v string) pairTemplate {
	brackets := func(c string) int { return strings.Count(c, "[") - strings.Count(c, "]") }
	// A template head is `fn $VAR` as tokens, followed by anything but a
	// `:` (a matcher's `fn $VAR:ident`) or more of a name (`fn $VARx`):
	// `(`, `<'a>(` and the line's end alike. Blanks may stand between
	// the tokens (a macro body is not rustfmt'd).
	fnRe := regexp.MustCompile(`(^|[^A-Za-z0-9_])(fn[ \t]+\$[ \t]*` + regexp.QuoteMeta(v) + `)[ \t]*([^A-Za-z0-9_: \t]|$)`)
	fnAt := func(c string) int {
		if m := fnRe.FindStringSubmatchIndex(c); m != nil {
			return m[4]
		}
		return -1
	}
	// The same across line ends, over the definition's whole code: a
	// template split over lines (`fn` then `$VAR`) is still one.
	fnAll := regexp.MustCompile(`(^|[^A-Za-z0-9_])(fn\s+\$\s*` + regexp.QuoteMeta(v) + `)\s*([^A-Za-z0-9_:\s]|$)`)
	var def strings.Builder
	var defLn []int // the line of each byte of def
	var lx lexer
	on := false
	open := 0
	a, cur := "", ""
	var tp pairTemplate
	// The definition's brace depth: the search ends where it closes, and
	// a second `fn $VAR(` in it (another arm) is reported, since only the
	// first one's attributes are read.
	depth, started := 0, false
	for n, l := range splitLines(text) {
		c := lx.code(l)
		if on {
			if started && depth <= 0 {
				break
			}
			depth += strings.Count(c, "{") - strings.Count(c, "}")
			if depth > 0 {
				started = true
			}
			def.WriteString(c + "\n")
			for range len(c) + 1 {
				defLn = append(defLn, n+1)
			}
		}
		if !on {
			on = macroDef(trim(c), macro)
			if on {
				depth = strings.Count(c, "{") - strings.Count(c, "}")
				started = depth > 0
				// A definition that closes on its own line holds no
				// template the search reads: stop, not run on into
				// whatever macro follows.
				if strings.Contains(c, "{") && depth <= 0 {
					break
				}
			}
			continue
		}
		if tp.found != 0 {
			continue
		}
		if open > 0 {
			cur += " " + trim(strip(l, c))
			open += brackets(c)
			if open <= 0 {
				a += cur + "@@"
				open = 0
			}
			continue
		}
		if i := fnAt(c); i >= 0 {
			tp = pairTemplate{attrs: a, item: trim(strip(l[:i], c[:i])), found: n + 1}
			continue
		}
		if rePairAttr.MatchString(c) {
			x := sub(reLead, "", strip(l, c))
			open = brackets(c)
			if open > 0 {
				cur = x
				continue
			}
			a += x + "@@"
			open = 0
			continue
		}
		if trim(c) != "" {
			a = ""
		}
	}
	// Every template in the definition, line ends included: a second one
	// is reported, and so is one the line-by-line read did not find (its
	// head split over lines), since its attributes cannot be read.
	ms := fnAll.FindAllStringSubmatchIndex(def.String(), -1)
	switch {
	case len(ms) >= 2:
		tp.found, tp.dup = defLn[ms[0][4]], defLn[ms[1][4]]
	case len(ms) == 1 && tp.found == 0:
		tp.split = defLn[ms[0][4]]
	}
	return tp
}

var (
	rePairAttr   = re("^[ \t]*(\\$\\()?#[ \t]*\\[")
	rePathPrefix = re("^(::)?([A-Za-z_][A-Za-z0-9_]*[ \t]*::[ \t]*)+$")
	reListOpen   = re("^fn [A-Za-z_][A-Za-z0-9_]*[ \t]*\\[")
	reListFn     = re("^fn [A-Za-z_][A-Za-z0-9_]*[ \t]*\\[[ \t]*[A-Za-z_][A-Za-z0-9_]*([ \t]*,[ \t]*[A-Za-z_][A-Za-z0-9_]*)*[ \t]*,?[ \t]*\\]")
)

// splitLines splits text into awk records: a final newline ends the
// last line rather than starting an empty one.
func splitLines(text string) []string {
	if text == "" {
		return nil
	}
	l := strings.Split(text, "\n")
	if l[len(l)-1] == "" {
		l = l[:len(l)-1]
	}
	return l
}
