package main

import (
	"fmt"
	"regexp"
	"strings"
)

// word is one shell word of a script: its text with quotes removed,
// whether any part of it was quoted, escaped or expanded (not literal),
// how many of its leading bytes were literal, whether it holds an
// expansion outside double quotes (which the shell splits into words),
// and its source as written (raw), which a variable's value is read
// from.
type word struct {
	text    string
	literal bool
	plain   int
	split   bool
	raw     string
}

// simpleCommand reads script as `sh -c` would, but only when it is
// exactly one simple command made of plain words: no operator (`;`,
// `&`, `|`, `<`, `>`, `(`, `)`), no comment, no command substitution
// (`$(`, backquote), no arithmetic, no `$'..'`, and a newline only before
// the first word or after the last. Quotes (single and double),
// backslash escapes and expansions of named parameters (`$NAME`,
// `${NAME}`, and `${NAME}` with one of `:-`, `-`, `:=`, `=`, `:?`, `?`,
// `:+`, `+` and a text holding no quote, backslash, `$`, `{`, backquote
// or newline) are read, and mark their word not literal; so do the glob
// characters `*`, `?`, `[`, a `{` or `}` and a leading `~`. A positional
// or special parameter (`$1`, `"$@"`, which is one word per parameter
// even quoted, `$*`, `$#`) or any other `${..}` (`${#X}`, `${X%y}`,
// `${a[@]}`) is an error, as is any other form, naming it: the caller
// counts no command it cannot read.
func simpleCommand(script string) ([]word, error) {
	var words []word
	i, n := 0, len(script)
	// trailing is set once a newline follows a word: only blanks and
	// newlines may follow.
	trailing := false
	for i < n {
		c := script[i]
		switch c {
		case ' ', '\t':
			i++
			continue
		case '\n':
			if len(words) > 0 {
				trailing = true
			}
			i++
			continue
		}
		if trailing {
			return nil, fmt.Errorf("more than one command (a newline between words)")
		}
		switch c {
		case ';', '&', '|', '<', '>', '(', ')':
			return nil, fmt.Errorf("an operator `%c`", c)
		case '#':
			return nil, fmt.Errorf("a comment")
		}
		w, next, err := readWord(script, i)
		if err != nil {
			return nil, err
		}
		words = append(words, w)
		i = next
	}
	if len(words) == 0 {
		return nil, fmt.Errorf("no command")
	}
	return words, nil
}

// readWord reads the word starting at i, which is no blank, newline or
// operator, and returns it and the index after it.
func readWord(s string, i int) (word, int, error) {
	var b strings.Builder
	literal := true
	n := len(s)
	start := i
	plain := 0
	split := false
	done := func(i int) (word, int, error) {
		if literal {
			plain = b.Len()
		}
		return word{b.String(), literal, plain, split, s[start:i]}, i, nil
	}
	for i < n {
		if literal {
			plain = b.Len()
		}
		c := s[i]
		switch c {
		case ' ', '\t', '\n', ';', '&', '|', '<', '>', '(', ')':
			return done(i)
		case '\'':
			end := strings.IndexByte(s[i+1:], '\'')
			if end < 0 {
				return word{}, 0, fmt.Errorf("an unterminated single quote")
			}
			b.WriteString(s[i+1 : i+1+end])
			literal = false
			i += end + 2
		case '"':
			j, err := readDouble(s, i+1, &b)
			if err != nil {
				return word{}, 0, err
			}
			literal = false
			i = j
		case '\\':
			if i+1 >= n {
				return word{}, 0, fmt.Errorf("a backslash at the end of the script")
			}
			if s[i+1] == '\n' {
				return word{}, 0, fmt.Errorf("a line continuation")
			}
			b.WriteByte(s[i+1])
			literal = false
			i += 2
		case '`':
			return word{}, 0, fmt.Errorf("a command substitution (backquote)")
		case '$':
			j, err := readDollar(s, i)
			if err != nil {
				return word{}, 0, err
			}
			b.WriteString(s[i:j])
			literal = false
			split = true
			i = j
		case '*', '?', '[', '{', '}':
			b.WriteByte(c)
			literal = false
			i++
		case '~':
			if i == start {
				literal = false
			}
			b.WriteByte(c)
			i++
		default:
			b.WriteByte(c)
			i++
		}
	}
	return done(i)
}

// readDouble reads a double-quoted string from just after its opening
// quote, writing its text to b, and returns the index after the closing
// quote.
func readDouble(s string, i int, b *strings.Builder) (int, error) {
	n := len(s)
	for i < n {
		c := s[i]
		switch c {
		case '"':
			return i + 1, nil
		case '\\':
			if i+1 >= n {
				return 0, fmt.Errorf("an unterminated double quote")
			}
			if s[i+1] == '\n' {
				return 0, fmt.Errorf("a line continuation")
			}
			switch s[i+1] {
			case '$', '`', '"', '\\':
				b.WriteByte(s[i+1])
			default:
				b.WriteByte('\\')
				b.WriteByte(s[i+1])
			}
			i += 2
		case '`':
			return 0, fmt.Errorf("a command substitution (backquote)")
		case '$':
			j, err := readDollar(s, i)
			if err != nil {
				return 0, err
			}
			b.WriteString(s[i:j])
			i = j
		default:
			b.WriteByte(c)
			i++
		}
	}
	return 0, fmt.Errorf("an unterminated double quote")
}

// readDollar reads the parameter expansion at s[i] == '$' and returns
// the index after it.
func readDollar(s string, i int) (int, error) {
	n := len(s)
	if i+1 >= n {
		return 0, fmt.Errorf("a `$` at the end of the script")
	}
	c := s[i+1]
	switch {
	case c == '(':
		return 0, fmt.Errorf("a command substitution or arithmetic (`$(`)")
	case c == '\'':
		return 0, fmt.Errorf("a `$'..'` string")
	case c == '{':
		end := strings.IndexByte(s[i+2:], '}')
		if end < 0 {
			return 0, fmt.Errorf("an unterminated `${`")
		}
		body := s[i+2 : i+2+end]
		if body == "" || strings.ContainsAny(body, "'\"\\$`{\n") {
			return 0, fmt.Errorf("a `${..}` holding a quote, backslash, `$`, `{`, backquote or newline, or empty")
		}
		if !namedBody.MatchString(body) {
			return 0, fmt.Errorf("a `${%s}` that is not a named parameter, alone or with :-, -, :=, =, :?, ?, :+ or +", body)
		}
		return i + 2 + end + 1, nil
	case isNameStart(c):
		j := i + 2
		for j < n && isNameChar(s[j]) {
			j++
		}
		return j, nil
	case c >= '0' && c <= '9', strings.IndexByte("@*#?-$!", c) >= 0:
		// "$@" is one word per positional parameter, even quoted.
		return 0, fmt.Errorf("a positional or special parameter (`$%c`)", c)
	}
	return 0, fmt.Errorf("a `$` before %q", c)
}

// namedBody is what a `${..}` this reads holds: a named parameter, alone
// or with one of the default, assign, error or alternative operators and
// its text. It is never more than one word when quoted.
var namedBody = regexp.MustCompile(`^[A-Za-z_][A-Za-z0-9_]*(:?[-=?+].*)?$`)

func isNameStart(c byte) bool {
	return c == '_' || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')
}

func isNameChar(c byte) bool {
	return isNameStart(c) || (c >= '0' && c <= '9')
}

// assignment returns the name a word assigns (`NAME=VALUE`, with NAME
// and `=` unquoted), or "" when the word is no assignment.
func assignment(w word) string {
	eq := strings.IndexByte(w.text, '=')
	if eq <= 0 || eq >= w.plain {
		return ""
	}
	name := w.text[:eq]
	if !isNameStart(name[0]) {
		return ""
	}
	for k := 1; k < len(name); k++ {
		if !isNameChar(name[k]) {
			return ""
		}
	}
	return name
}
