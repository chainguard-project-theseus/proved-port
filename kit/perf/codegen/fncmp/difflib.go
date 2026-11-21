package main

// A transcription of the parts of CPython's difflib (3.13) that
// challoc's codegen-fncmp.py uses: SequenceMatcher with isjunk=None and
// autojunk=True (ratio, get_opcodes, get_grouped_opcodes) and
// unified_diff. The report must match the Python byte for byte, so the
// matcher keeps difflib's exact choices: the popular-element purge for
// a b of 200 or more elements, the first-found longest match, and the
// LIFO queue in get_matching_blocks.

import (
	"fmt"
	"sort"
)

type match struct{ i, j, k int }

type opcode struct {
	tag            string
	i1, i2, j1, j2 int
}

type seqMatcher struct {
	a, b []string
	b2j  map[string][]int
}

func newSeqMatcher(a, b []string) *seqMatcher {
	m := &seqMatcher{a: a, b: b, b2j: map[string][]int{}}
	for i, elt := range b {
		m.b2j[elt] = append(m.b2j[elt], i)
	}
	// autojunk: an element of more than 1% of a b of 200 or more is
	// popular and leaves b2j. isjunk is None, so bjunk is empty and
	// difflib's junk-extension loops never move.
	if n := len(b); n >= 200 {
		ntest := n/100 + 1
		for elt, idxs := range m.b2j {
			if len(idxs) > ntest {
				delete(m.b2j, elt)
			}
		}
	}
	return m
}

func (m *seqMatcher) findLongestMatch(alo, ahi, blo, bhi int) match {
	a, b := m.a, m.b
	besti, bestj, bestsize := alo, blo, 0
	j2len := map[int]int{}
	for i := alo; i < ahi; i++ {
		newj2len := map[int]int{}
		for _, j := range m.b2j[a[i]] {
			if j < blo {
				continue
			}
			if j >= bhi {
				break
			}
			k := j2len[j-1] + 1
			newj2len[j] = k
			if k > bestsize {
				besti, bestj, bestsize = i-k+1, j-k+1, k
			}
		}
		j2len = newj2len
	}
	for besti > alo && bestj > blo && a[besti-1] == b[bestj-1] {
		besti, bestj, bestsize = besti-1, bestj-1, bestsize+1
	}
	for besti+bestsize < ahi && bestj+bestsize < bhi && a[besti+bestsize] == b[bestj+bestsize] {
		bestsize++
	}
	return match{besti, bestj, bestsize}
}

func (m *seqMatcher) matchingBlocks() []match {
	la, lb := len(m.a), len(m.b)
	queue := [][4]int{{0, la, 0, lb}}
	var blocks []match
	for len(queue) > 0 {
		q := queue[len(queue)-1]
		queue = queue[:len(queue)-1]
		alo, ahi, blo, bhi := q[0], q[1], q[2], q[3]
		x := m.findLongestMatch(alo, ahi, blo, bhi)
		if x.k > 0 {
			blocks = append(blocks, x)
			if alo < x.i && blo < x.j {
				queue = append(queue, [4]int{alo, x.i, blo, x.j})
			}
			if x.i+x.k < ahi && x.j+x.k < bhi {
				queue = append(queue, [4]int{x.i + x.k, ahi, x.j + x.k, bhi})
			}
		}
	}
	sort.Slice(blocks, func(p, q int) bool {
		x, y := blocks[p], blocks[q]
		if x.i != y.i {
			return x.i < y.i
		}
		if x.j != y.j {
			return x.j < y.j
		}
		return x.k < y.k
	})
	var out []match
	i1, j1, k1 := 0, 0, 0
	for _, x := range blocks {
		if i1+k1 == x.i && j1+k1 == x.j {
			k1 += x.k
		} else {
			if k1 > 0 {
				out = append(out, match{i1, j1, k1})
			}
			i1, j1, k1 = x.i, x.j, x.k
		}
	}
	if k1 > 0 {
		out = append(out, match{i1, j1, k1})
	}
	return append(out, match{la, lb, 0})
}

// ratio is difflib's 2.0*M/T, with 1.0 for two empty sequences.
func (m *seqMatcher) ratio() float64 {
	matches := 0
	for _, x := range m.matchingBlocks() {
		matches += x.k
	}
	if t := len(m.a) + len(m.b); t > 0 {
		return 2.0 * float64(matches) / float64(t)
	}
	return 1.0
}

func (m *seqMatcher) opcodes() []opcode {
	i, j := 0, 0
	var out []opcode
	for _, x := range m.matchingBlocks() {
		tag := ""
		switch {
		case i < x.i && j < x.j:
			tag = "replace"
		case i < x.i:
			tag = "delete"
		case j < x.j:
			tag = "insert"
		}
		if tag != "" {
			out = append(out, opcode{tag, i, x.i, j, x.j})
		}
		i, j = x.i+x.k, x.j+x.k
		if x.k > 0 {
			out = append(out, opcode{"equal", x.i, i, x.j, j})
		}
	}
	return out
}

func (m *seqMatcher) groupedOpcodes(n int) [][]opcode {
	codes := m.opcodes()
	if len(codes) == 0 {
		codes = []opcode{{"equal", 0, 1, 0, 1}}
	}
	if c := codes[0]; c.tag == "equal" {
		codes[0] = opcode{c.tag, max(c.i1, c.i2-n), c.i2, max(c.j1, c.j2-n), c.j2}
	}
	if c := codes[len(codes)-1]; c.tag == "equal" {
		codes[len(codes)-1] = opcode{c.tag, c.i1, min(c.i2, c.i1+n), c.j1, min(c.j2, c.j1+n)}
	}
	var groups [][]opcode
	var group []opcode
	for _, c := range codes {
		i1, j1 := c.i1, c.j1
		if c.tag == "equal" && c.i2-c.i1 > 2*n {
			group = append(group, opcode{c.tag, c.i1, min(c.i2, c.i1+n), c.j1, min(c.j2, c.j1+n)})
			groups = append(groups, group)
			group = nil
			i1, j1 = max(c.i1, c.i2-n), max(c.j1, c.j2-n)
		}
		group = append(group, opcode{c.tag, i1, c.i2, j1, c.j2})
	}
	if len(group) > 0 && !(len(group) == 1 && group[0].tag == "equal") {
		groups = append(groups, group)
	}
	return groups
}

func formatRangeUnified(start, stop int) string {
	beginning, length := start+1, stop-start
	if length == 1 {
		return fmt.Sprint(beginning)
	}
	if length == 0 {
		beginning--
	}
	return fmt.Sprintf("%d,%d", beginning, length)
}

// unifiedDiff is difflib.unified_diff(a, b, from, to, n=n, lineterm="").
func unifiedDiff(a, b []string, from, to string, n int) []string {
	var out []string
	for gi, group := range newSeqMatcher(a, b).groupedOpcodes(n) {
		if gi == 0 {
			out = append(out, "--- "+from, "+++ "+to)
		}
		first, last := group[0], group[len(group)-1]
		out = append(out, fmt.Sprintf("@@ -%s +%s @@",
			formatRangeUnified(first.i1, last.i2), formatRangeUnified(first.j1, last.j2)))
		for _, c := range group {
			if c.tag == "equal" {
				for _, l := range a[c.i1:c.i2] {
					out = append(out, " "+l)
				}
				continue
			}
			if c.tag == "replace" || c.tag == "delete" {
				for _, l := range a[c.i1:c.i2] {
					out = append(out, "-"+l)
				}
			}
			if c.tag == "replace" || c.tag == "insert" {
				for _, l := range b[c.j1:c.j2] {
					out = append(out, "+"+l)
				}
			}
		}
	}
	return out
}
