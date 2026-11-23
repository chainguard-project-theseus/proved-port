package main

import (
	"fmt"
	"os"
	"path"
	"path/filepath"
	"slices"
	"sort"
	"strings"
)

// pinRow is a row of a pins file: `FILE<TAB>LINE[<TAB>NAME:ROLE]`.
type pinRow struct {
	file, text, tag string
}

// readPins reads the pins file pf (a path from root): its rows, in
// order, and why some row cannot be read. A file that is gone is one
// error and no rows. `#` lines and lines of blanks are comments; a row
// has two or three tab-separated fields, none empty, and its FILE is a
// path inside the tree (seams.sh's pins() skipped a row of one field and
// read any path; a row of more than three fields failed there too). A
// `...` row stands between two lines of its FILE: not first among
// FILE's rows, not last, and not right after another `...` row of FILE;
// and it has no third field, which would make it a row of a side. A file
// with no rows (empty, or only comments) fails.
func readPins(root, pf string) ([]pinRow, []string, bool) {
	b, err := os.ReadFile(filepath.Join(root, pf))
	if err != nil {
		return nil, []string{fmt.Sprintf("seams: %s is gone", pf)}, false
	}
	var rows []pinRow
	var errs []string
	lines := strings.Split(string(b), "\n")
	if len(lines) > 0 && lines[len(lines)-1] == "" {
		lines = lines[:len(lines)-1]
	}
	for i, l := range lines {
		if strings.HasPrefix(l, "#") || strings.TrimSpace(l) == "" {
			continue
		}
		f := strings.Split(l, "\t")
		switch {
		case len(f) > 3:
			errs = append(errs, fmt.Sprintf("seams: %s: a row has more than three fields: %s", pf, l))
			continue
		case len(f) < 2:
			errs = append(errs, fmt.Sprintf("seams: %s:%d: a row has one field, want FILE<TAB>LINE[<TAB>NAME:ROLE]: %s", pf, i+1, l))
			continue
		case slices.Contains(f, ""):
			errs = append(errs, fmt.Sprintf("seams: %s:%d: a row has an empty field: %s", pf, i+1, l))
			continue
		}
		if c := path.Clean(f[0]); path.IsAbs(c) || c == "." || c == ".." || strings.HasPrefix(c, "../") || c != f[0] {
			errs = append(errs, fmt.Sprintf("seams: %s:%d: %s is not a clean path inside the tree", pf, i+1, f[0]))
			continue
		}
		r := pinRow{file: f[0], text: f[1]}
		if len(f) == 3 {
			// A tagged `...` would count as a row of its side, keeping
			// the side consecutive across a gap no row pins.
			if r.text == "..." {
				errs = append(errs, fmt.Sprintf("seams: %s:%d: a `...` row has a third field, which would let a side's rows span a gap: %s", pf, i+1, l))
				continue
			}
			r.tag = f[2]
		}
		rows = append(rows, r)
	}
	// A `...` row's place, per file.
	last := map[string]string{}
	for _, r := range rows {
		prev, seen := last[r.file]
		if r.text == "..." && (!seen || prev == "...") {
			errs = append(errs, fmt.Sprintf("seams: %s: a `...` row for %s does not stand between two of its lines", pf, r.file))
		}
		last[r.file] = r.text
	}
	for f, t := range last {
		if t == "..." {
			errs = append(errs, fmt.Sprintf("seams: %s: a `...` row for %s does not stand between two of its lines", pf, f))
		}
	}
	// A pins line claims something is pinned: a file of no rows (empty,
	// or only comments) holds nothing in place.
	if len(rows) == 0 && len(errs) == 0 {
		errs = append(errs, fmt.Sprintf("seams: %s has no rows, so it holds no line in place", pf))
	}
	sort.Strings(errs)
	return rows, errs, true
}

// pinsHeld is seams.sh's pins() for the rows of pf: for each FILE the
// rows name (in byte order), the first of its rows that FILE does not
// hold in place, and each first line of a run that FILE does not hold
// exactly once. FILE's rows appear in it in their order, each on the
// line after the one before (lines compared with leading and trailing
// blanks and tabs removed; FILE's blank lines and lines starting `//`
// skipped), but where a `...` row allows any lines between. The first
// row, and each after a `...`, starts a run, which may start anywhere
// and must occur in FILE exactly once, so a second copy of the lines (in
// a block comment, say) cannot stand in for the real one.
func pinsHeld(root, pf string, rows []pinRow) []string {
	var files []string
	byFile := map[string][]string{}
	for _, r := range rows {
		if _, ok := byFile[r.file]; !ok {
			files = append(files, r.file)
		}
		byFile[r.file] = append(byFile[r.file], r.text)
	}
	sort.Strings(files)
	var out []string
	for _, f := range files {
		b, err := os.ReadFile(filepath.Join(root, f))
		if err != nil {
			out = append(out, fmt.Sprintf("seams: %s: %s is gone", pf, f))
			continue
		}
		p := byFile[f]
		n := len(p)
		// starts: the first line of each run, and how often FILE has it.
		starts := map[string]int{}
		for k := range p {
			if p[k] != "..." && (k == 0 || p[k-1] == "...") {
				starts[p[k]] = 0
			}
		}
		k, gap, done := 0, true, false
		for _, l := range strings.Split(string(b), "\n") {
			t := strings.Trim(l, " \t")
			if t == "" || strings.HasPrefix(t, "//") {
				continue
			}
			if _, ok := starts[t]; ok {
				starts[t]++
			}
			if done || k >= n {
				continue
			}
			if t == p[k] {
				k, gap = k+1, false
				if k < n && p[k] == "..." {
					k, gap = k+1, true
				}
				continue
			}
			if !gap {
				done = true
			}
		}
		if k < n {
			out = append(out, fmt.Sprintf("seams: %s: %s no longer has, in place: %s", pf, f, p[k]))
		}
		var ts []string
		for t, c := range starts {
			if c != 1 {
				ts = append(ts, t)
			}
		}
		sort.Strings(ts)
		for _, t := range ts {
			out = append(out, fmt.Sprintf("seams: %s: %s has %d lines %s, want 1", pf, f, starts[t], t))
		}
	}
	return out
}

// copiesMatch is seams.sh's copies() for the rows of pf: for each NAME
// the third fields name (`NAME:orig`, `NAME:orig-only`, `NAME:copy`,
// `NAME:copy-only`), in the order NAMEs first appear, the `NAME:orig`
// rows, in order, must equal the `NAME:copy` rows, and each side's rows
// (orig and orig-only, copy and copy-only) must be consecutive rows, so
// a row added inside either side must say which side it is on. As
// pinsHeld holds every row in place, equal rows mean the copy repeats
// the original's lines but for its `-only` rows.
func copiesMatch(pf string, rows []pinRow) []string {
	type span struct{ first, last, cnt int }
	var out, names []string
	sides := map[string]*span{}
	orig, cp := map[string][]string{}, map[string][]string{}
	for i, r := range rows {
		if r.tag == "" {
			continue
		}
		j := strings.LastIndex(r.tag, ":")
		nm, role := "", r.tag
		if j >= 0 {
			nm, role = r.tag[:j], r.tag[j+1:]
		}
		if nm == "" || role != "orig" && role != "orig-only" && role != "copy" && role != "copy-only" {
			out = append(out, fmt.Sprintf("seams: %s: row %d: %s is not NAME:orig, NAME:orig-only, NAME:copy or NAME:copy-only", pf, i+1, r.tag))
			continue
		}
		if _, ok := sides[nm+"\x00orig"]; !ok {
			if _, ok := sides[nm+"\x00copy"]; !ok {
				names = append(names, nm)
			}
		}
		side := "copy"
		if strings.HasPrefix(role, "orig") {
			side = "orig"
		}
		s := sides[nm+"\x00"+side]
		if s == nil {
			s = &span{first: i}
			sides[nm+"\x00"+side] = s
		}
		s.last, s.cnt = i, s.cnt+1
		switch role {
		case "orig":
			orig[nm] = append(orig[nm], r.text)
		case "copy":
			cp[nm] = append(cp[nm], r.text)
		}
	}
	for _, nm := range names {
		both := true
		for _, side := range []string{"orig", "copy"} {
			s := sides[nm+"\x00"+side]
			if s == nil {
				out = append(out, fmt.Sprintf("seams: %s: %s has no %s rows", pf, nm, side))
				both = false
				continue
			}
			if s.last-s.first+1 != s.cnt {
				out = append(out, fmt.Sprintf("seams: %s: %s: the %s rows are not consecutive", pf, nm, side))
			}
		}
		if !both {
			continue
		}
		o, c := orig[nm], cp[nm]
		if len(o) == 0 {
			out = append(out, fmt.Sprintf("seams: %s: %s has no NAME:orig rows", pf, nm))
			continue
		}
		if len(o) != len(c) {
			out = append(out, fmt.Sprintf("seams: %s: %s: the copy has %d shared rows, the original %d", pf, nm, len(c), len(o)))
			continue
		}
		for j := range o {
			if o[j] != c[j] {
				out = append(out, fmt.Sprintf("seams: %s: %s: the copy has %s where the original has %s", pf, nm, c[j], o[j]))
				break
			}
		}
	}
	return out
}

// pinChecks runs the checks of each pins file the rules name: its rows
// read, held in place (pinsHeld) and their copies matched (copiesMatch);
// and an overrides file whose text names the pins file (a status that
// relies on it) needs a row there for its source file.
func pinChecks(root string, r *Rules, overrides []namedText) []string {
	var out []string
	for _, pf := range r.Pins {
		rows, errs, ok := readPins(root, pf)
		out = append(out, errs...)
		if !ok {
			continue
		}
		out = append(out, pinsHeld(root, pf, rows)...)
		out = append(out, copiesMatch(pf, rows)...)
		for _, o := range overrides {
			if !strings.Contains(o.text, pf) {
				continue
			}
			src := strings.TrimSuffix(strings.TrimPrefix(o.name, r.Ledger+"/"), ".overrides")
			if !slices.ContainsFunc(rows, func(p pinRow) bool { return p.file == src }) {
				out = append(out, fmt.Sprintf("seams: %s: a status names %s, which has no row for %s", o.name, pf, src))
			}
		}
	}
	return out
}
