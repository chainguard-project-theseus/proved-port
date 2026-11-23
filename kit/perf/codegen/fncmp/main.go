// Command fncmp compares two `objdump -d --no-show-raw-insn` dumps
// function by function. It is the kit's port of challoc's
// scripts/codegen-fncmp.py and prints the same report byte for byte.
//
// Functions pair by symbol name. Each pair is one of:
//
//	exact         instruction text identical once addresses and <symbol>
//	              targets are stripped
//	reloc-only    identical once hex/decimal immediates are masked too, with
//	              an AArch64 offset-0 operand `[xN]` read as `[xN, #I]`
//	padding-only  identical once trailing alignment padding (nop, int3,
//	              `xchg %ax,%ax`) is dropped as well: the function moved, and
//	              the gap to the next aligned symbol changed
//	erratum-stub  GNU ld's Cortex-A53 erratum 843419 workaround: an `adrp`
//	              at page offset 0xff8/0xffc became `adr` of the same page,
//	              or a load or store 1-3 instructions after it became a
//	              branch into the stub; the stub after the body holds only
//	              padding, a branch past the stubs, and displaced loads and
//	              stores each followed by a branch back to the instruction
//	              after the original, at the address the body branches to
//	changed       anything else
//
// A function whose name is new pairs first with a baseline function whose
// name differs only in the crate hash or impl disambiguator (the demangled
// path without `[hash]`, when c++filt is on PATH; that pair is classified
// as above), then with an unpaired baseline function of the same
// instructions (exact, then reloc-only) as a rename. Changed counts the
// changed functions plus the baseline functions left with no counterpart.
// The report prints one line per function that is neither exact nor
// reloc-only under its own name, and the full masked diff of every
// changed one.
//
// Usage: fncmp OLD NEW
package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"os/exec"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"unicode"
	"unicode/utf8"
)

var (
	reHdr     = regexp.MustCompile(`^[0-9a-f]+ <(.*)>:$`)
	reIns     = regexp.MustCompile(`^ *([0-9a-f]+):\t(.*)$`)
	rePad     = regexp.MustCompile(`^((data16|cs|ds) )*(nop[a-z]*( .*)?|xchg %ax,%ax|int3)$`)
	reBranch  = regexp.MustCompile(`^b ([0-9a-f]+) <sym>$`)
	reSym     = regexp.MustCompile(`<[^>]*>`)
	reHex     = regexp.MustCompile(`0x[0-9a-f]+`)
	reImm     = regexp.MustCompile(`#-?[0-9]+`)
	reBareReg = regexp.MustCompile(`\[(x[0-9]+|sp)\]`)
	reTarget  = regexp.MustCompile(`[0-9a-f]{3,} <sym>`)
	reComment = regexp.MustCompile(`// #-?[0-9]+`)
	reAdrp    = regexp.MustCompile(`^adrp? `)
	reLdSt    = regexp.MustCompile(`^(ld|st)`)
	reAdrpOp  = regexp.MustCompile(`^adrp (x[0-9]+), ([0-9a-f]+) <sym>$`)
	reAdrOp   = regexp.MustCompile(`^adr (x[0-9]+), ([0-9a-f]+) <sym>$`)
	reDmHash  = regexp.MustCompile(`\[[0-9a-f]+\]`)
	reHashDis = regexp.MustCompile(`(Cs[0-9A-Za-z]+_|Ms[0-9A-Za-z]*_)`)
	reBackref = regexp.MustCompile(`B[0-9A-Za-z]*_`)
)

type insn struct {
	addr uint64
	text string
}

type fn = []insn

// dump is a parsed objdump: name -> instructions, in dump order.
type dump struct {
	names []string
	fns   map[string]fn
}

func hexval(s string) uint64 {
	v, err := strconv.ParseUint(s, 16, 64)
	if err != nil {
		fmt.Fprintf(os.Stderr, "fncmp: address %s: %v\n", s, err)
		os.Exit(1)
	}
	return v
}

// parse reads a dump; a repeated name gets #2, #3.
func parse(r io.Reader) (*dump, error) {
	raw, err := io.ReadAll(bufio.NewReader(r))
	if err != nil {
		return nil, err
	}
	d := &dump{fns: map[string]fn{}}
	seen := map[string]int{}
	var cur *string
	for _, line := range textLines(decodeReplace(raw)) {
		if m := reHdr.FindStringSubmatch(line); m != nil {
			name := m[1]
			seen[name]++
			if seen[name] > 1 {
				name = fmt.Sprintf("%s#%d", name, seen[name])
			}
			if _, ok := d.fns[name]; !ok {
				d.names = append(d.names, name)
			}
			d.fns[name] = fn{}
			n := name
			cur = &n
			continue
		}
		if m := reIns.FindStringSubmatch(line); m != nil && cur != nil {
			t := squeeze(reSym.ReplaceAllLiteralString(m[2], "<sym>"))
			d.fns[*cur] = append(d.fns[*cur], insn{hexval(m[1]), t})
		}
	}
	return d, nil
}

// mask1 masks one instruction's immediates.
func mask1(t string) string {
	t = reHex.ReplaceAllLiteralString(t, "0xI")
	t = reImm.ReplaceAllLiteralString(t, "#I")
	// AArch64 offset 0: objdump prints `[x9]` where any other offset
	// prints `[x9, #16]`, so a :lo12: relocation that resolves to 0
	// drops the immediate. Spell it `[x9, #I]` like the others. A
	// following `!` or `,` (pre- or post-index) keeps the bare form.
	t = replaceBareReg(t)
	t = replaceTarget(t)
	t = reComment.ReplaceAllLiteralString(t, "// #I")
	return t
}

// replaceBareReg is re.sub(r'\[(x[0-9]+|sp)\](?!\s*[!,])', r'[\1, #I]', t);
// RE2 has no lookahead, so the lookahead is checked by hand.
func replaceBareReg(t string) string {
	var sb strings.Builder
	last := 0
	for _, m := range reBareReg.FindAllStringSubmatchIndex(t, -1) {
		rest := strings.TrimLeftFunc(t[m[1]:], pySpace)
		if strings.HasPrefix(rest, "!") || strings.HasPrefix(rest, ",") {
			continue
		}
		sb.WriteString(t[last:m[0]])
		sb.WriteString("[" + t[m[2]:m[3]] + ", #I]")
		last = m[1]
	}
	sb.WriteString(t[last:])
	return sb.String()
}

// wordBefore reports whether the character ending at t[:s] is a word
// character of Python's str \w: '_' or str.isalnum().
func wordBefore(t string, s int) bool {
	r, _ := utf8.DecodeLastRuneInString(t[:s])
	return r == '_' || unicode.IsLetter(r) || unicode.IsNumber(r)
}

// replaceTarget is re.sub(r'\b[0-9a-f]{3,} <sym>', 'A <sym>', t). The
// word boundary is checked by hand: Python's \b counts non-ASCII
// letters and digits as word characters, RE2's counts only ASCII.
func replaceTarget(t string) string {
	var sb strings.Builder
	last, pos := 0, 0
	for pos < len(t) {
		loc := reTarget.FindStringIndex(t[pos:])
		if loc == nil {
			break
		}
		s, e := pos+loc[0], pos+loc[1]
		// The hex run is greedy from s; a candidate whose left side is
		// a word byte fails \b, and Python then tries the next start.
		if s > 0 && wordBefore(t, s) {
			pos = s + 1
			continue
		}
		sb.WriteString(t[last:s])
		sb.WriteString("A <sym>")
		last, pos = e, e
	}
	sb.WriteString(t[last:])
	return sb.String()
}

func texts(f fn) []string {
	out := make([]string, len(f))
	for i, x := range f {
		out[i] = x.text
	}
	return out
}

func mask(f fn) []string {
	out := make([]string, len(f))
	for i, x := range f {
		out[i] = mask1(x.text)
	}
	return out
}

func equal(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

func stripPad(f fn) fn {
	n := len(f)
	for n > 0 && rePad.MatchString(f[n-1].text) {
		n--
	}
	return f[:n]
}

func isErratumOffset(a uint64) bool { a &= 0xfff; return a == 0xff8 || a == 0xffc }

type stubKey struct {
	text string
	ret  uint64
}

// erratum returns a note if n is o plus the erratum 843419 rewrite and
// stub. o and n have their trailing padding stripped.
func erratum(o, n fn) (string, bool) {
	k := len(o)
	if len(n) < k {
		return "", false
	}
	body, tail := n[:k], n[k:]
	var end uint64
	if len(tail) > 0 {
		end = tail[len(tail)-1].addr + 4
	} else if len(body) > 0 {
		end = body[len(body)-1].addr + 4
	}
	// The adrp/adr at page offset 0xff8/0xffc 1-3 before j.
	erratumAdrp := func(j int) (uint64, bool) {
		for x := j - 1; x >= max(0, j-3); x-- {
			if reAdrp.MatchString(body[x].text) && isErratumOffset(body[x].addr) {
				return body[x].addr, true
			}
		}
		return 0, false
	}
	var sites []string
	// (masked displaced load or store, return address) -> the stub
	// address the body branches to, or none when the instruction stayed
	// in place (adr form) and the stub's copy is unreachable.
	type at struct {
		addr uint64
		ok   bool
	}
	stubbed := map[stubKey]at{}
	for i := 0; i < k; i++ {
		ot, na, nt := o[i].text, body[i].addr, body[i].text
		if mask1(ot) == mask1(nt) {
			if _, ok := erratumAdrp(i); ok && reLdSt.MatchString(nt) {
				key := stubKey{mask1(nt), na + 4}
				if _, have := stubbed[key]; !have {
					stubbed[key] = at{}
				}
			}
			continue
		}
		a, b := reAdrpOp.FindStringSubmatch(ot), reAdrOp.FindStringSubmatch(nt)
		if a != nil && b != nil && a[1] == b[1] && isErratumOffset(na) && hexval(b[2])&0xfff == 0 {
			sites = append(sites, fmt.Sprintf("adrp %s at %#x (page offset %#x) became adr", a[1], na, na&0xfff))
			continue
		}
		br := reBranch.FindStringSubmatch(nt)
		p, pok := erratumAdrp(i)
		if br != nil && pok && reLdSt.MatchString(ot) && len(tail) > 0 {
			tgt := hexval(br[1])
			if tail[0].addr <= tgt && tgt <= tail[len(tail)-1].addr {
				stubbed[stubKey{mask1(ot), na + 4}] = at{tgt, true}
				sites = append(sites, fmt.Sprintf("`%s` at %#x became a branch to the stub (adrp at %#x, page offset %#x)",
					ot, na, p, p&0xfff))
				continue
			}
		}
		return "", false
	}
	if len(sites) == 0 {
		return "", false
	}
	// The stub: padding, a branch past the stubs, and each displaced
	// load or store followed by a branch back to the instruction after
	// it, at the address the body branches to.
	found := map[stubKey]bool{}
	for j := 0; j < len(tail); {
		a, t := tail[j].addr, tail[j].text
		var pair *stubKey
		if j+1 < len(tail) {
			if b := reBranch.FindStringSubmatch(tail[j+1].text); b != nil {
				pair = &stubKey{mask1(t), hexval(b[1])}
			}
		}
		if rePad.MatchString(t) {
			j++
			continue
		}
		if b := reBranch.FindStringSubmatch(t); b != nil && hexval(b[1]) >= end {
			j++
			continue
		}
		if pair != nil {
			if s, ok := stubbed[*pair]; ok && (!s.ok || s.addr == a) {
				found[*pair] = true
				j += 2
				continue
			}
		}
		return "", false
	}
	for key, s := range stubbed {
		if s.ok && !found[key] {
			return "", false
		}
	}
	note := strings.Join(sites, "; ")
	if len(tail) > 0 {
		ts := make([]string, len(tail))
		for i, x := range tail {
			ts[i] = x.text
		}
		note += fmt.Sprintf("; %d-instruction stub after the body: ", len(tail)) + strings.Join(ts, "; ")
	}
	return note, true
}

// classify returns (kind, note) for baseline function o and new function n.
func classify(o, n fn) (string, string) {
	if equal(texts(o), texts(n)) {
		return "exact", ""
	}
	if equal(mask(o), mask(n)) {
		return "reloc-only", ""
	}
	so, sn := stripPad(o), stripPad(n)
	if equal(mask(so), mask(sn)) {
		return "padding-only", fmt.Sprintf("same %d instructions, trailing padding %d -> %d",
			len(so), len(o)-len(so), len(n)-len(sn))
	}
	if note, ok := erratum(so, sn); ok && note != "" {
		return "erratum-stub", "GNU ld erratum 843419 workaround: " + note
	}
	if note, ok := erratum(sn, so); ok && note != "" {
		return "erratum-stub", "GNU ld erratum 843419 workaround in the baseline: " + note
	}
	return "changed", ""
}

// demangle runs c++filt, when it is on PATH, over names and returns the
// names it changed.
func demangle(names []string) map[string]string {
	out := map[string]string{}
	tool, err := exec.LookPath("c++filt")
	if err != nil || len(names) == 0 {
		return out
	}
	cmd := exec.Command(tool)
	cmd.Stdin = strings.NewReader(strings.Join(names, "\n") + "\n")
	b, _ := cmd.Output() // stderr is captured and dropped, as capture_output does

	lines := splitLines(string(b))
	for i := 0; i < len(names) && i < len(lines); i++ {
		if lines[i] != names[i] {
			out[names[i]] = lines[i]
		}
	}
	return out
}

// line is one report line: kind, new name, baseline name (hasM false
// for none) and note.
type line struct {
	kind, n, m string
	hasM       bool
	note       string
}

// truthy is Python's `if m:` on a name that may be None or empty.
func (l line) truthy() bool { return l.hasM && l.m != "" }

func run(oldPath, newPath string, w io.Writer) error {
	open := func(p string) (*dump, error) {
		f, err := os.Open(p)
		if err != nil {
			return nil, err
		}
		defer f.Close()
		return parse(f)
	}
	old, err := open(oldPath)
	if err != nil {
		return err
	}
	nw, err := open(newPath)
	if err != nil {
		return err
	}
	// A dump with no functions is a failed objdump or the wrong file.
	// The Python reports on it; the kit fails closed instead. That is
	// the one place the two differ.
	for _, x := range []struct {
		path string
		d    *dump
	}{{oldPath, old}, {newPath, nw}} {
		if len(x.d.names) == 0 {
			return fmt.Errorf("%s: no functions (not an objdump -d dump?)", x.path)
		}
	}
	kinds := map[string]int{}
	var lines []line
	for _, n := range nw.names {
		if o, ok := old.fns[n]; ok {
			k, note := classify(o, nw.fns[n])
			kinds[k]++
			if k != "exact" && k != "reloc-only" {
				lines = append(lines, line{k, n, n, true, note})
			}
		}
	}
	var left, fresh []string
	for _, n := range old.names {
		if _, ok := nw.fns[n]; !ok {
			left = append(left, n)
		}
	}
	for _, n := range nw.names {
		if _, ok := old.fns[n]; !ok {
			fresh = append(fresh, n)
		}
	}
	// A v0 symbol's crate hash (Cs..._) and impl disambiguator (Ms<n>_)
	// change when the crate metadata changes or an impl is added next
	// to it; the demangled path without [hash] stays. Pair leftovers
	// whose key is unique on both sides, and classify them like
	// same-name pairs. Without c++filt the key is the mangled name with
	// those two dropped and every backref (B<base-62>_, a byte offset
	// into the symbol, which moves when an earlier hash or disambiguator
	// changes length) spelled B_. That is coarser than the demangled
	// path: two names that differ only in what a backref points at share
	// a key. A key shared within one side pairs nothing; a pair made
	// across the sides is still classified by its instructions.
	dm := demangle(append(append([]string{}, left...), fresh...))
	mangledKey := func(name string) string {
		name = reHashDis.ReplaceAllLiteralString(name, "")
		return reBackref.ReplaceAllLiteralString(name, "B_")
	}
	key := func(name string) string {
		if d, ok := dm[name]; ok {
			return reDmHash.ReplaceAllLiteralString(d, "")
		}
		return mangledKey(name)
	}
	remove := func(s []string, x string) []string {
		for i, y := range s {
			if y == x {
				return append(s[:i:i], s[i+1:]...)
			}
		}
		return s
	}
	pairByKey := func(key func(string) string) {
		ko, kn := map[string]int{}, map[string]int{}
		for _, o := range left {
			ko[key(o)]++
		}
		for _, n := range fresh {
			kn[key(n)]++
		}
		byKey := map[string]string{}
		for _, o := range left {
			if ko[key(o)] == 1 {
				byKey[key(o)] = o
			}
		}
		for _, n := range append([]string{}, fresh...) {
			m, ok := byKey[key(n)]
			if !ok || kn[key(n)] != 1 {
				continue
			}
			k, note := classify(old.fns[m], nw.fns[n])
			kinds[k]++
			full := "renamed from " + m
			if note != "" {
				full += "; " + note
			}
			lines = append(lines, line{k, n, m, true, full})
			left = remove(left, m)
			fresh = remove(fresh, n)
		}
	}
	pairByKey(key)
	// When c++filt demangles a name on one side only, its demangled
	// key cannot meet the other side's mangled key, and the content
	// pass below would pair the leftovers by body: two swapped bodies
	// would read as two exact renames. So the leftovers pair once more
	// by the mangled key alone before that pass. The Python stops after
	// the first key; this is the one report difference on purpose
	// (testdata/golden/impl-old_impl-swap.filt.txt).
	if len(dm) > 0 {
		pairByKey(mangledKey)
	}
	for _, n := range fresh {
		v := nw.fns[n]
		k, m, hasM := "exact", "", false
		for _, o := range left {
			if equal(texts(old.fns[o]), texts(v)) {
				m, hasM = o, true
				break
			}
		}
		if !hasM {
			k = "reloc-only"
			for _, o := range left {
				if equal(mask(old.fns[o]), mask(v)) {
					m, hasM = o, true
					break
				}
			}
		}
		if !hasM {
			// A renamed and changed function diffs against its closest
			// leftover.
			k = "changed"
			best, bestOK := 0.0, false
			mv := mask(v)
			for _, o := range left {
				r := newSeqMatcher(mask(old.fns[o]), mv).ratio()
				if !bestOK || r > best {
					best, bestOK, m = r, true, o
				}
			}
			hasM = bestOK && best >= 0.5
			if !hasM {
				m = ""
			}
		}
		if hasM {
			left = remove(left, m)
		}
		kinds[k]++
		note := ""
		if k != "changed" {
			note = "renamed from " + pyStr(m, hasM)
		}
		lines = append(lines, line{k, n, m, hasM, note})
	}

	more := map[string]bool{}
	for _, l := range lines {
		for _, x := range []struct {
			s  string
			ok bool
		}{{l.n, true}, {l.m, l.hasM}} {
			if x.ok && x.s != "" {
				if _, have := dm[x.s]; !have {
					more[x.s] = true
				}
			}
		}
	}
	extra := make([]string, 0, len(more))
	for x := range more {
		extra = append(extra, x)
	}
	sort.Strings(extra)
	for k, v := range demangle(extra) {
		dm[k] = v
	}
	show := func(name string) string {
		if d, ok := dm[name]; ok {
			return name + " = " + d
		}
		return name
	}

	// A baseline function with no counterpart is a change too: its code
	// is gone, or went into a caller.
	var changed []line
	for _, l := range lines {
		if l.kind == "changed" {
			changed = append(changed, l)
		}
	}
	// bufio.Writer keeps the first write error, so Flush returns it and
	// a report cut short by a full disk or closed pipe exits non-zero.
	bw := bufio.NewWriter(w)
	total := func(d *dump) int {
		s := 0
		for _, f := range d.fns {
			s += len(f)
		}
		return s
	}
	fmt.Fprintf(bw, "functions old %d new %d: exact %d, reloc-only %d, padding-only %d, erratum-stub %d, changed %d\n",
		len(old.fns), len(nw.fns), kinds["exact"], kinds["reloc-only"], kinds["padding-only"],
		kinds["erratum-stub"], len(changed)+len(left))
	fmt.Fprintf(bw, "insns old %d new %d\n", total(old), total(nw))
	for _, l := range lines {
		if l.kind == "changed" {
			base := "no baseline counterpart"
			if l.truthy() {
				base = fmt.Sprintf("baseline %s (%d insns)", l.m, len(old.fns[l.m]))
			}
			fmt.Fprintf(bw, "### changed: %s (%d insns) vs %s\n", l.n, len(nw.fns[l.n]), base)
		} else {
			fmt.Fprintf(bw, "### %s: %s (%d -> %d insns): %s\n", l.kind, l.n, len(old.fns[l.m]), len(nw.fns[l.n]), l.note)
		}
		if d, ok := dm[l.n]; ok {
			fmt.Fprintf(bw, "    = %s\n", d)
		}
	}
	if len(left) > 0 {
		fmt.Fprint(bw, "\nbaseline functions with no counterpart (counted as changed):\n")
		for _, n := range left {
			fmt.Fprintf(bw, "   %s (%d insns)\n", show(n), len(old.fns[n]))
		}
	}
	for _, l := range changed {
		against := ""
		if l.truthy() && l.m != l.n {
			against = " against baseline " + l.m
		}
		fmt.Fprintf(bw, "\n--- diff of %s%s, immediates masked\n", l.n, against)
		var a []string
		if l.truthy() {
			a = mask(old.fns[l.m])
		}
		for _, d := range unifiedDiff(a, mask(nw.fns[l.n]), "old", "new", 2) {
			fmt.Fprintf(bw, "   %s\n", d)
		}
	}
	return bw.Flush()
}

// pyStr is Python's f'{m}' for a name that may be None.
func pyStr(m string, ok bool) string {
	if !ok {
		return "None"
	}
	return m
}

func main() {
	if len(os.Args) != 3 {
		fmt.Fprintln(os.Stderr, "usage: fncmp OLD NEW")
		os.Exit(2)
	}
	if err := run(os.Args[1], os.Args[2], os.Stdout); err != nil {
		fmt.Fprintln(os.Stderr, "fncmp:", err)
		os.Exit(1)
	}
}
