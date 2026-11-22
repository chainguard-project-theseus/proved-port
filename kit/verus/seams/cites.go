package main

import (
	"regexp"
	"sort"
	"strings"
)

// cite is one citation a status makes: a cite tag and the check it
// names, as written.
type cite struct{ tag, target string }

// citeSep splits a citation list: at `, and ` or ` and ` (a comma before
// `and` optional) or at a comma and the blanks after it. seams.sh split
// with awk's leftmost-longest match; Go's leftmost-first gives the same
// parts, since where both alternatives match (at a comma) the first is
// the longer.
var citeSep = regexp.MustCompile(`,? and |, *`)

// citations parses a status into its citations, seams.sh's cites(): at
// each `TAG: ` for a cite tag, the text up to the first `;` or `)` (or
// the end) split by citeSep, each nonempty part up to the first that
// holds `: ` (the next tag, which its own occurrence reads). The
// citations come in the order their tags stand in the status, each
// list's parts in order. empty lists the tags of the occurrences that
// name nothing (seams.sh's `none`), in the same order.
func (r *Rules) citations(stat string) (cites []cite, empty []string) {
	type occ struct {
		at    int
		tag   string
		parts []string
	}
	var occs []occ
	for _, t := range r.CiteTags {
		key := t + ": "
		for from := 0; ; {
			i := strings.Index(stat[from:], key)
			if i < 0 {
				break
			}
			at := from + i
			from = at + len(key)
			l := stat[from:]
			if j := strings.IndexAny(l, ";)"); j >= 0 {
				l = l[:j]
			}
			var parts []string
			for _, h := range citeSep.Split(l, -1) {
				if strings.Contains(h, ": ") {
					break
				}
				if h != "" {
					parts = append(parts, h)
				}
			}
			occs = append(occs, occ{at, t, parts})
		}
	}
	// Two tags cannot stand at one place (each holds no `:` and ends in
	// `: `), so the order is total.
	sort.Slice(occs, func(a, b int) bool { return occs[a].at < occs[b].at })
	for _, o := range occs {
		if len(o.parts) == 0 {
			empty = append(empty, o.tag)
		}
		for _, p := range o.parts {
			cites = append(cites, cite{o.tag, p})
		}
	}
	return cites, empty
}
