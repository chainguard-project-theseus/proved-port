package main

import (
	"regexp"
	"strings"
	"unicode/utf8"
)

// The helpers below keep the semantics of the awk seams.sh was written
// in: 1-based substr with clamping, byte strings (LC_ALL=C), and POSIX
// leftmost-longest regular expressions for sub and match.

// substr is awk's substr(s, m, n): n bytes from 1-based position m,
// clamped to the string.
func substr(s string, m, n int) string {
	start := m
	end := m + n // exclusive, 1-based
	if start < 1 {
		start = 1
	}
	if end > len(s)+1 {
		end = len(s) + 1
	}
	if end <= start {
		return ""
	}
	return s[start-1 : end-1]
}

// substrFrom is awk's substr(s, m): from 1-based position m to the end.
func substrFrom(s string, m int) string {
	if m < 1 {
		m = 1
	}
	if m > len(s) {
		return ""
	}
	return s[m-1:]
}

// index is awk's index(s, t): the 1-based position of t in s, or 0.
func index(s, t string) int {
	return strings.Index(s, t) + 1
}

func re(p string) *regexp.Regexp { return regexp.MustCompilePOSIX(p) }

// sub is awk's sub(re, repl, s) for a replacement with no `&`: the
// leftmost-longest match replaced once.
func sub(r *regexp.Regexp, repl, s string) string {
	loc := r.FindStringIndex(s)
	if loc == nil {
		return s
	}
	return s[:loc[0]] + repl + s[loc[1]:]
}

// match is awk's match(s, re), returning the matched text and whether
// it matched.
func match(r *regexp.Regexp, s string) (string, bool) {
	loc := r.FindStringIndex(s)
	if loc == nil {
		return "", false
	}
	return s[loc[0]:loc[1]], true
}

var (
	reLead  = re("^[ \t]+")
	reTrail = re("[ \t]+$")
	reWS    = re("[ \t]+")
)

func trim(s string) string { return sub(reTrail, "", sub(reLead, "", s)) }

// norm is the text a hash covers: whitespace runs outside string and
// char literals made one space, the ends trimmed. Inside a literal every
// byte is kept, so `"a  b"` and `"a b"` hash differently (seams.sh
// collapsed them too, so an edit to a literal's spacing kept the
// hashes). The literals are `"..."` with `\` escapes (b"..." too), raw
// strings `r"..."` and `r#"..."#` (br and cr too), and char literals as
// charLit reads them. In a block comment (`/* */`, nesting) whitespace
// runs are one space and a quote opens no literal (strip has already
// dropped the comments of the lines a hash covers).
func norm(s string) string {
	var b strings.Builder
	n := len(s)
	ws := false
	for i := 0; i < n; i++ {
		ch := s[i]
		if ch == ' ' || ch == '\t' {
			ws = true
			continue
		}
		// One space for the run before this byte; none at the start,
		// and a run at the end is never written.
		if ws && b.Len() > 0 {
			b.WriteByte(' ')
		}
		ws = false
		switch {
		case ch == 'r' && rawStart(s, i):
			// A raw string, r"..." or r#"..."# (br too): no escapes, it
			// ends at a quote followed by as many #s as opened it.
			j := i + 1
			for s[j] == '#' {
				j++
			}
			end := n
			if k := strings.Index(s[j+1:], "\""+s[i+1:j]); k >= 0 {
				end = j + 1 + k + 1 + (j - i - 1)
			}
			b.WriteString(s[i:end])
			i = end - 1
		case ch == '/' && i+1 < n && s[i+1] == '*':
			// A block comment (nesting): whitespace runs in it are one
			// space and a quote in it opens no literal.
			depth, j := 0, i
			for j < n {
				if j+1 < n && s[j] == '/' && s[j+1] == '*' {
					depth++
					b.WriteString("/*")
					j += 2
				} else if j+1 < n && s[j] == '*' && s[j+1] == '/' {
					depth--
					b.WriteString("*/")
					j += 2
					if depth == 0 {
						break
					}
				} else if s[j] == ' ' || s[j] == '\t' {
					for j < n && (s[j] == ' ' || s[j] == '\t') {
						j++
					}
					b.WriteByte(' ')
				} else {
					b.WriteByte(s[j])
					j++
				}
			}
			i = j - 1
		case ch == '"':
			j := i + 1
			for j < n && s[j] != '"' {
				if s[j] == '\\' {
					j++
				}
				j++
			}
			if j >= n {
				j = n - 1
			}
			b.WriteString(s[i : j+1])
			i = j
		case ch == '\'' && charLit(s, i) > 0:
			e := charLit(s, i)
			b.WriteString(s[i:e])
			i = e - 1
		default:
			b.WriteByte(ch)
		}
	}
	return b.String()
}

// charLit is the end (exclusive) of the char or byte literal whose
// opening quote is s[i], or 0 when none starts there (a lifetime, a
// label): `'` then one UTF-8 character (not `\` or `'`) or an escape
// (`\` and one byte, `\xHH`, or `\u{` 1 to 6 hex digits, `_` allowed
// among them, `}`), then `'`.
// seams.sh took one byte, or any escape closing within 9 bytes, so a
// non-ASCII char or `'\u{10FFFF}'` left its closing quote to open a
// false literal.
func charLit(s string, i int) int {
	n := len(s)
	if i+2 >= n {
		return 0
	}
	j := i + 1
	switch {
	case s[j] == '\\' && s[j+1] == 'u' && j+2 < n && s[j+2] == '{':
		// 1 to 6 hex digits; Rust also allows `_` among them
		// (`'\u{1_F600}'`), which do not count.
		k, digits := j+3, 0
		for k < n && (isHex(s[k]) || s[k] == '_') {
			if s[k] != '_' {
				digits++
			}
			k++
		}
		if digits == 0 || digits > 6 || k >= n || s[k] != '}' {
			return 0
		}
		j = k + 1
	case s[j] == '\\' && s[j+1] == 'x':
		if j+3 >= n || !isHex(s[j+2]) || !isHex(s[j+3]) {
			return 0
		}
		j += 4
	case s[j] == '\\':
		j += 2
	case s[j] == '\'':
		return 0
	default:
		_, sz := utf8.DecodeRuneInString(s[j:])
		j += sz
	}
	if j < n && s[j] == '\'' {
		return j + 1
	}
	return 0
}

func isHex(c byte) bool {
	return c >= '0' && c <= '9' || c >= 'a' && c <= 'f' || c >= 'A' && c <= 'F'
}

func identByte(c byte) bool {
	return c == '_' || c >= '0' && c <= '9' || c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z'
}

// rawStart says whether s[i], an `r`, opens a raw string: `r`, `br` or
// `cr` starting a token, then any #s, then a quote (`r#ident` is no
// string).
func rawStart(s string, i int) bool {
	if i > 0 && identByte(s[i-1]) && !((s[i-1] == 'b' || s[i-1] == 'c') && (i == 1 || !identByte(s[i-2]))) {
		return false
	}
	j := i + 1
	for j < len(s) && s[j] == '#' {
		j++
	}
	return j < len(s) && s[j] == '"'
}

var reCommentLine = re("^[ \t]*//")

// lexer reads a file's lines in order, carrying across lines what
// seams.sh's per-line reading lost: an open block comment (nesting), an
// open string and an open raw string.
type lexer struct {
	depth    int    // block comment nesting
	str      int    // 1 in a string, 2 in a raw string
	rawClose string // what closes the raw string: a quote and its #s
}

// code is the line with string and char literal contents and comments
// blanked, so braces and keywords in it are code: string contents as
// x (an escape as xx), char literals as 'x', raw strings (`r"..."`,
// `r#"..."#`, br and cr too, norm's rule) as x, `//` comments and block
// comments as spaces. A line that starts outside any literal or comment
// and is a `//` comment gives ""; one with no quote and no `/*` keeps
// the text before its comment. Unlike seams.sh, which read each line
// alone, a block comment, string or raw string that spans lines is
// blanked on every line it spans, and a raw string's `\` is no escape.
func (lx *lexer) code(s string) string {
	n := len(s)
	if lx.depth == 0 && lx.str == 0 {
		if reCommentLine.MatchString(s) {
			return ""
		}
		if !strings.ContainsAny(s, "\"'") && !strings.Contains(s, "/*") {
			if i := strings.Index(s, "//"); i >= 0 {
				return s[:i]
			}
			return s
		}
	}
	var out strings.Builder
	for i := 0; i < n; i++ {
		ch := s[i]
		switch {
		case lx.str == 1:
			if ch == '\\' {
				out.WriteString("xx")
				i++
			} else if ch == '"' {
				lx.str = 0
				out.WriteByte(ch)
			} else {
				out.WriteByte('x')
			}
		case lx.str == 2:
			if strings.HasPrefix(s[i:], lx.rawClose) {
				out.WriteString(lx.rawClose)
				i += len(lx.rawClose) - 1
				lx.str = 0
			} else {
				out.WriteByte('x')
			}
		case lx.depth > 0:
			if strings.HasPrefix(s[i:], "/*") {
				lx.depth++
				out.WriteString("  ")
				i++
			} else if strings.HasPrefix(s[i:], "*/") {
				lx.depth--
				out.WriteString("  ")
				i++
			} else {
				out.WriteByte(' ')
			}
		case ch == 'r' && rawStart(s, i):
			q := i + 1
			for s[q] == '#' {
				q++
			}
			lx.str, lx.rawClose = 2, "\""+s[i+1:q]
			out.WriteString(s[i : q+1])
			i = q
		case ch == '"':
			lx.str = 1
			out.WriteByte(ch)
		case ch == '/' && i+1 < n && s[i+1] == '/':
			out.WriteString(strings.Repeat(" ", n-i))
			i = n
		case ch == '/' && i+1 < n && s[i+1] == '*':
			lx.depth = 1
			out.WriteString("  ")
			i++
		case ch == '\'' && charLit(s, i) > 0:
			e := charLit(s, i)
			out.WriteString("'" + strings.Repeat("x", e-i-2) + "'")
			i = e - 1
		default:
			out.WriteByte(ch)
		}
	}
	return out.String()
}

// code is one line read alone (lexer.code from a clean state).
func code(s string) string { return (&lexer{}).code(s) }

// strip is the line raw without its comments, string literals kept: c is
// the lexer's code for raw, which blanks comments with spaces (and
// literals with x) column for column, so a byte c blanks to a space
// while raw has a non-blank there is comment and is dropped, and the
// text past c's last code byte (a `//` comment) is cut. seams.sh only
// cut at the `//`; this drops block comments too, so a quote in one
// cannot open a literal for norm.
func strip(raw, c string) string {
	end := len(sub(reTrail, "", c))
	if end > len(raw) {
		end = len(raw)
	}
	// Each dropped run becomes one blank, so the tokens either side of a
	// comment stay apart (`a/*c*/b` is not `ab`); norm folds it with
	// any blanks next to it.
	var b strings.Builder
	dropping := false
	for i := 0; i < end; i++ {
		if c[i] == ' ' && raw[i] != ' ' && raw[i] != '\t' {
			if !dropping {
				b.WriteByte(' ')
			}
			dropping = true
			continue
		}
		dropping = false
		b.WriteByte(raw[i])
	}
	return b.String()
}

// Name of the type an `impl` line implements.
func implName(t string) string {
	s := t
	if i := strings.LastIndex(s, "impl"); i >= 0 {
		s = s[i+4:]
	}
	if substr(s, 1, 1) == "<" {
		d := 0
		i := 1
		for ; i <= len(s); i++ {
			ch := s[i-1]
			if ch == '<' {
				d++
			} else if ch == '>' {
				d--
				if d == 0 {
					break
				}
			}
		}
		s = substrFrom(s, i+1)
	}
	s = trim(s)
	for k := strings.Index(s, " for "); k >= 0; k = strings.Index(s, " for ") {
		s = s[k+5:]
	}
	if i := strings.IndexAny(s, "< {"); i >= 0 {
		s = s[:i]
	}
	if i := strings.LastIndex(s, "::"); i >= 0 {
		s = s[i+2:]
	}
	return s
}

var (
	reForTail   = re(" for .*$")
	reImplHead  = re("^impl(<[^>]*>)? ")
	rePathHead  = re("^([A-Za-z0-9_]+::)+")
	reGenTail   = re("<.*$")
	reSpecImplT = re("SpecImpl$")
)

// traitKey is the trait and type of a trait impl line, as a key: `impl
// fmt::Display for T` and `impl vstd::std_specs::fmt::DisplaySpecImpl for
// crate::m::T` both give Display and T.
func traitKey(t string) string {
	tr := sub(reForTail, "", t)
	tr = sub(reImplHead, "", tr)
	tr = sub(rePathHead, "", tr)
	tr = sub(reGenTail, "", tr)
	tr = sub(reSpecImplT, "", tr)
	return tr + "\x1c" + implName(t)
}

// specPath is the bracketed path of an `assume_specification`.
func specPath(s string) string {
	k := index(s, "[")
	if k == 0 {
		return ""
	}
	s = substrFrom(s, k+1)
	d := 1
	i := 1
	for ; i <= len(s); i++ {
		ch := s[i-1]
		if ch == '[' {
			d++
		} else if ch == ']' {
			d--
			if d == 0 {
				break
			}
		}
	}
	return trim(substr(s, 1, i-1))
}

// short is item name x in at most w characters: the crate and `core`
// paths dropped, then the middle elided.
func short(x string, w int) string {
	x = strings.TrimPrefix(x, "crate::")
	x = strings.TrimPrefix(x, "core::sync::atomic::")
	x = strings.TrimPrefix(x, "core::")
	if w < 16 {
		w = 16
	}
	if len(x) <= w {
		return x
	}
	h := (w - 3) / 2
	return substr(x, 1, w-3-h) + "..." + substrFrom(x, len(x)-h+1)
}

func md(s string) string { return strings.ReplaceAll(s, "|", "\\|") }
