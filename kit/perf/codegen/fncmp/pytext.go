package main

// Python text semantics the report depends on: reading a file with
// open(path, errors='replace') (UTF-8, universal newlines), the
// whitespace of re's \s and str.strip(), and str.splitlines().

import (
	"strings"
	"unicode"
	"unicode/utf8"
)

// decodeReplace decodes b as UTF-8 the way Python's errors='replace'
// does: one U+FFFD per maximal invalid subpart (an invalid lead byte
// alone, or a valid lead byte with the valid continuation bytes that
// follow it before the sequence breaks).
func decodeReplace(b []byte) string {
	var sb strings.Builder
	sb.Grow(len(b))
	for i := 0; i < len(b); {
		c := b[i]
		if c < 0x80 {
			sb.WriteByte(c)
			i++
			continue
		}
		if r, size := utf8.DecodeRune(b[i:]); r != utf8.RuneError || size > 1 {
			sb.Write(b[i : i+size])
			i += size
			continue
		}
		// Invalid here: find the maximal subpart.
		need, lo, hi := 0, byte(0x80), byte(0xbf)
		switch {
		case c >= 0xc2 && c <= 0xdf:
			need = 1
		case c == 0xe0:
			need, lo = 2, 0xa0
		case c == 0xed:
			need, hi = 2, 0x9f
		case c >= 0xe1 && c <= 0xef:
			need = 2
		case c == 0xf0:
			need, lo = 3, 0x90
		case c == 0xf4:
			need, hi = 3, 0x8f
		case c >= 0xf1 && c <= 0xf3:
			need = 3
		}
		j := i + 1
		for k := 0; k < need && j < len(b); k++ {
			if k > 0 {
				lo, hi = 0x80, 0xbf
			}
			if b[j] < lo || b[j] > hi {
				break
			}
			j++
		}
		sb.WriteRune(utf8.RuneError)
		i = j
	}
	return sb.String()
}

// textLines splits decoded text into lines as Python's text mode reads
// them (universal newlines: \r\n, \r and \n each end a line), without
// the line ends.
func textLines(s string) []string {
	s = strings.ReplaceAll(s, "\r\n", "\n")
	s = strings.ReplaceAll(s, "\r", "\n")
	lines := strings.Split(s, "\n")
	if lines[len(lines)-1] == "" {
		lines = lines[:len(lines)-1]
	}
	return lines
}

// pySpace is str.isspace() for one character, which is also what re's
// \s matches in a str pattern.
func pySpace(r rune) bool {
	return unicode.IsSpace(r) || (r >= 0x1c && r <= 0x1f)
}

// squeeze is re.sub(r'\s+', ' ', t).strip().
func squeeze(t string) string {
	var sb strings.Builder
	sp := false
	for _, r := range t {
		if pySpace(r) {
			sp = true
			continue
		}
		if sp && sb.Len() > 0 {
			sb.WriteByte(' ')
		}
		sp = false
		sb.WriteRune(r)
	}
	return sb.String()
}

// splitLines is str.splitlines().
func splitLines(s string) []string {
	var out []string
	start := 0
	rs := []rune(s)
	for i := 0; i < len(rs); i++ {
		switch rs[i] {
		case '\n', '\v', '\f', 0x1c, 0x1d, 0x1e, 0x85, 0x2028, 0x2029:
			out = append(out, string(rs[start:i]))
			start = i + 1
		case '\r':
			out = append(out, string(rs[start:i]))
			if i+1 < len(rs) && rs[i+1] == '\n' {
				i++
			}
			start = i + 1
		}
	}
	if start < len(rs) {
		out = append(out, string(rs[start:]))
	}
	return out
}
