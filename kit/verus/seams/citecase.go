package main

import (
	"fmt"
	"path"
	"regexp"
	"slices"
	"strings"
)

// reCaseCite is a cite-case citation, `CASE [MODEL ...]`: a case name
// and its models, one blank between the words.
var reCaseCite = regexp.MustCompile(`^([A-Za-z0-9_]+) \[([^ \[\]]+( [^ \[\]]+)*)\]$`)

// reCaseWord is a case or template name in a cases file.
var reCaseWord = regexp.MustCompile(`^[A-Za-z0-9_]+$`)

// caseCite says why the citation `CASE [MODELS]` of seam e does not
// resolve against dir/cases, "" when it does, seams.sh's genmctest():
//   - dir/cases (`name | template | models | constants | expected` a
//     line, as challoc's tests/genmc/run.sh reads it: a blank line or
//     one starting `#` is none, fields trimmed) has exactly one line
//     whose name is CASE, with no empty field;
//   - its expected verdict is `pass` alone (a `fail` case, with its
//     expected reports after it, is a control);
//   - its models are MODELS, the same words in the same order, so the
//     citation states every model the case runs under and no other;
//   - its template is a word and dir/TEMPLATE.rs is there and lexes
//     whole;
//   - that file names e's source file in backquotes (`SOURCE/a/b.rs`, or
//     `a/b.rs` for a file in a directory under SOURCE) and e's function
//     (seamFn, namesFn) anywhere, comments included, as genmctest()
//     reads it.
func (v *resolver) caseCite(dir string, e *entry, target string) string {
	m := reCaseCite.FindStringSubmatch(target)
	if m == nil {
		return "is no `CASE [MODEL ...]` citation (a word, a blank, and the models in brackets, one blank between them)"
	}
	name, models := m[1], strings.Fields(m[2])
	cases := path.Join(dir, "cases")
	if !v.exists(cases) {
		return cases + " is not there"
	}
	var rows []int
	// run.sh skips a blank line and one starting `#`; neither has a
	// word for its name, so neither names CASE.
	for i, l := range v.file(cases).raw {
		if f := strings.Split(l, "|"); strings.TrimSpace(f[0]) == name {
			rows = append(rows, i)
		}
	}
	switch len(rows) {
	case 0:
		return fmt.Sprintf("%s has no case %s", cases, name)
	case 1:
	default:
		return fmt.Sprintf("%s names the case %s on %d lines", cases, name, len(rows))
	}
	ln := rows[0] + 1
	f := strings.Split(v.file(cases).raw[rows[0]], "|")
	for i := range f {
		f[i] = strings.TrimSpace(f[i])
		if f[i] == "" {
			return fmt.Sprintf("%s:%d: the case %s has an empty field", cases, ln, name)
		}
	}
	if len(f) < 5 {
		return fmt.Sprintf("%s:%d: the case %s has %d fields, not `name | template | models | constants | expected`", cases, ln, name, len(f))
	}
	if len(f) != 5 || f[4] != "pass" {
		return fmt.Sprintf("%s:%d: the case %s expects %s, not `pass` alone (a `fail` case is a control)", cases, ln, name, strings.Join(f[4:], " | "))
	}
	if got := strings.Fields(f[2]); !slices.Equal(got, models) {
		return fmt.Sprintf("%s:%d: the case %s runs under [%s], not [%s]", cases, ln, name, strings.Join(got, " "), strings.Join(models, " "))
	}
	if !reCaseWord.MatchString(f[1]) {
		return fmt.Sprintf("%s:%d: the case %s's template %s is not a word", cases, ln, name, f[1])
	}
	tmpl := path.Join(dir, f[1]+".rs")
	t := v.file(tmpl)
	if !t.ok {
		return fmt.Sprintf("%s (the case %s's template) is not there or ends inside a literal or comment", tmpl, name)
	}
	if why := v.namesSrc(t, tmpl, e); why != "" {
		return why
	}
	fn := seamFn(e)
	for _, l := range t.raw {
		if namesFn(l, fn) {
			return ""
		}
	}
	return fmt.Sprintf("%s does not name `%s` in backquotes", tmpl, fn)
}
