package main

import (
	"fmt"
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
	pairName   string // pair_attrs of the macro's first and second fn
	pairJe     string
	lex        lexer // the current file's literals and comments

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
	pairdef                    bool // in the pair macro's macro_rules! body
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
	reDocLine      = re("^///")
	reDocHead      = re("^/// ?")
	reAttrStart    = re("^#!?\\[")
	reCfgTest1     = re("^#[ \t]*\\[[ \t]*cfg[ \t]*\\([ \t]*(all[ \t]*\\([ \t]*)?test[ \t]*[,)]")
	reCfgTest2     = re("^#\\[cfg\\(test\\)\\]")
	reFnLine       = re("(^|[ \t])fn [A-Za-z_]")
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
	s.pairdef = false
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
		s.attrs += " " + s.at
		s.hattrs += " " + s.ah
		if strings.HasPrefix(s.at, "#[") {
			s.uattrs += s.au + "\n"
		}
		if reCfgTest1.MatchString(s.at) || reCfgTest2.MatchString(s.at) {
			s.pendtest = 1
			s.pendAt = s.ln
		}
		return
	}

	// A test-only item is skipped from its first line through its body,
	// which may open lines below a multi-line fn signature (pendtest 2).
	if s.pendtest != 0 {
		s.attrs, s.hattrs, s.doc, s.uattrs = "", "", "", ""
		if opens > closes {
			s.pendtest = 0
			s.skip = true
			s.skipAt = s.ln
			s.skipd = s.depth
			s.depth += opens - closes
		} else if opens > 0 && opens == closes {
			// A one-line item (`fn g() {}`): it ends here. seams.sh read
			// it as a multi-line signature and skipped on to the next
			// line that opens a brace, dropping the seams between.
			s.pendtest = 0
		} else if s.pendtest == 1 && reFnLine.MatchString(t) && !reSemiEnd.MatchString(t) {
			s.pendtest = 2
		} else if s.pendtest == 1 || reSemiEnd.MatchString(t) || closes > opens {
			s.pendtest = 0
		}
		return
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
	if s.pairln != 0 {
		if m, ok := match(rePairFn, t); ok {
			p := strings.ReplaceAll(m[3:], " ", "")
			pr := strings.Split(p, "/")
			mac := s.rules.PairMacro
			if s.pairName == "" || s.pairJe == "" {
				s.errs = append(s.errs, fmt.Sprintf("seams: %s:%d: no %s! attributes in %s", s.file, s.ln, mac, s.rules.PairFile))
			}
			s.unsupported(pr[0], s.pairln, s.uattrs+pairAttrs(s.pairName)+"pub extern \"C\" fn "+pr[0]+" ("+mac+"!)")
			s.unsupported(pr[1], s.pairln, s.uattrs+pairAttrs(s.pairJe)+"pub extern \"C\" fn "+pr[1]+" ("+mac+"!)")
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
	if s.rules.PairMacro != "" && strings.HasPrefix(t, s.rules.PairMacro+"!") && pairOpen(t[len(s.rules.PairMacro)+1:]) {
		s.pairln = s.ln
		s.paird = s.depth
	}
	// The pair macro's own definition: its exports' attributes are read
	// at each invocation, so they are no unread item here.
	if s.rules.PairMacro != "" && !s.pairdef && macroDef(t, s.rules.PairMacro) && opens > closes {
		s.pairdef = true
		s.pairdefd = s.depth
	}
	s.depth += opens - closes
	s.popImpls()
	if s.inverus && s.depth <= s.verusd {
		s.inverus = false
	}
	if s.pairln != 0 && s.depth <= s.paird {
		s.pairln = 0
	}
	if s.pairdef && s.depth <= s.pairdefd {
		s.pairdef = false
	}
}

// pairOpen: the rest of a `MACRO!` line is ` *\{`.
func pairOpen(rest string) bool {
	return strings.HasPrefix(strings.TrimLeft(rest, " "), "{")
}

// pairAttrs is the `@@`-joined attribute lines one per line.
func pairAttrs(s string) string { return strings.ReplaceAll(s, "@@", "\n") }

// readPairAttrs is seams.sh's pair_attrs: the attribute lines the pair
// macro writes above its `fn $VAR(` export, `$(#[$m])*` included, joined
// by `@@`; "" when the macro or the line is missing. An attribute that
// spans lines is read through its closing bracket.
func readPairAttrs(text, macro, v string) string {
	brackets := func(x string) int { return strings.Count(x, "[") - strings.Count(x, "]") }
	fn := "fn $" + v + "("
	on := false
	open := 0
	a, cur := "", ""
	for _, l := range splitLines(text) {
		if macroDef(l, macro) {
			on = true
			continue
		}
		if !on {
			continue
		}
		if open > 0 {
			x := trim(l)
			cur += " " + x
			open += brackets(x)
			if open <= 0 {
				a += cur + "@@"
				open = 0
			}
			continue
		}
		if strings.Contains(l, fn) {
			return a
		}
		if rePairAttr.MatchString(l) {
			x := sub(reLead, "", l)
			open = brackets(x)
			if open > 0 {
				cur = sub(reTrail, "", x)
				continue
			}
			a += x + "@@"
			open = 0
			continue
		}
		if reExternFn.MatchString(l) {
			a = ""
		}
	}
	return ""
}

var (
	rePairAttr = re("^[ \t]*(\\$\\()?#\\[")
	reExternFn = re("^[ \t]*pub extern \"C\" fn ")
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
