package main

import (
	"strings"
	"testing"
)

// tsv turns "|" into tabs, so the cases below read as columns.
func tsv(s string) string { return strings.ReplaceAll(s, "|", "\t") }

// Every expected line below is worked by hand from the inputs.
func TestGate(t *testing.T) {
	for _, c := range []struct {
		name, reps, rules, out string
		code                   int
		errs                   string
	}{
		{
			// Port A's shape: F below 1/20 of M on cD-512 (1000*20 =
			// 20000 < 20001), F within 1% of M on lit-1 (99*100 = 9900
			// < 10000). Cycles and the other group's rows play no part.
			name: "pass",
			reps: `g|cD-512|F|1|1000|5|out one
g|cD-512|M|1|20001|9|out one
g|lit-1|F|1|10099|5|o2
g|lit-1|M|1|10000|5|o2
h|lit-1|F|1|1|5|other
`,
			rules: `# port A's gate

g|cD-512|F|below|1/20|M
g|lit-1|F|within|1/100|M
`,
			out: `g|cD-512|F|below|1/20|M|1000|20001|pass
g|lit-1|F|within|1/100|M|10099|10000|pass
`,
		},
		{
			// Equal to the bound fails (strict): 1000*20 = 20000, and
			// |10100-10000|*100 = 10000; below the reference counts too
			// (|9900-10000|*100 = 10000), and 9901 is inside.
			name: "strict",
			reps: `g|cD-512|F|1|1000|5|o
g|cD-512|M|1|20000|5|o
g|hi|F|1|10100|5|o
g|hi|M|1|10000|5|o
g|lo|F|1|9900|5|o
g|lo|M|1|10000|5|o
g|in|F|1|9901|5|o
g|in|M|1|10000|5|o
`,
			rules: `g|cD-512|F|below|1/20|M
g|hi|F|within|1/100|M
g|lo|F|within|1/100|M
g|in|F|within|1/100|M
`,
			out: `g|cD-512|F|below|1/20|M|1000|20000|bound
g|hi|F|within|1/100|M|10100|10000|bound
g|lo|F|within|1/100|M|9900|10000|bound
g|in|F|within|1/100|M|9901|10000|pass
`,
			code: 1, errs: "stats gate: 3 of 4 rules failed\n",
		},
		{
			// Medians are the lower middle: F's 5,1,3,2 sort to 1 2 3 5
			// and give 2 (the upper middle, 3, is not below 3/1 of 1);
			// M's 1,1,9 give 1. 2*1 < 3*1.
			name: "median",
			reps: `g|r|F|1|5|0|o
g|r|M|1|1|0|o
g|r|F|2|1|0|o
g|r|M|2|9|0|o
g|r|F|3|3|0|o
g|r|M|3|1|0|o
g|r|F|4|2|0|o
`,
			rules: "g|r|F|below|3/1|M\n",
			out:   "g|r|F|below|3/1|M|2|1|pass\n",
		},
		{
			// Exact arithmetic: 5 * 1801439850948199 = 9007199254740995,
			// below 9007199254740996. In float64 the product rounds up
			// to 9007199254740996 and the rule would fail.
			name:  "exact",
			reps:  "g|r|F|1|1801439850948199|0|o\ng|r|M|1|9007199254740996|0|o\n",
			rules: "g|r|F|below|1/5|M\n",
			out:   "g|r|F|below|1/5|M|1801439850948199|9007199254740996|pass\n",
		},
		{
			// Missing arms print - for both medians; a row in another
			// group does not count.
			name: "missing",
			reps: `g|r|M|1|10|0|o
g|s|F|1|10|0|o
h|t|F|1|1|0|o
h|t|M|1|100|0|o
`,
			rules: `g|r|F|below|1/2|M
g|s|F|below|1/2|M
g|t|F|below|1/2|M
`,
			out: `g|r|F|below|1/2|M|-|-|no-arm
g|s|F|below|1/2|M|-|-|no-ref
g|t|F|below|1/2|M|-|-|no-arm no-ref
`,
			code: 1, errs: "stats gate: 3 of 3 rules failed\n",
		},
		{
			// A zero median fails even where the bound holds (0*2 <
			// 1*10); a zero reference fails the bound too (0 < 0 is
			// false). A non-integer count prints as perf printed it.
			name: "zero",
			reps: `g|r|F|1|0|0|o
g|r|M|1|10|0|o
g|s|F|1|0.5|0|o
g|s|M|1|0|0|o
`,
			rules: `g|r|F|below|1/2|M
g|s|F|within|1/2|M
`,
			out: `g|r|F|below|1/2|M|0|10|zero
g|s|F|within|1/2|M|0.5|0|zero bound
`,
			code: 1, errs: "stats gate: 2 of 2 rules failed\n",
		},
		{
			// Outputs: F's second round differs from its first (row
			// r); M differs from F (row s, each arm agreeing with
			// itself); a third arm's output on the row is not compared
			// (row u); an output of several columns compares whole
			// (row v: "a|b" against "a|c"). The bound is checked too.
			name: "output",
			reps: `g|r|F|1|1|0|x
g|r|F|2|1|0|y
g|r|M|1|10|0|x
g|s|F|1|1|0|x
g|s|M|1|10|0|y
g|s|M|2|10|0|y
g|u|F|1|1|0|x
g|u|M|1|10|0|x
g|u|C|1|10|0|z
g|v|F|1|1|0|a|b
g|v|M|1|10|0|a|c
g|w|F|1|10|0|a
g|w|M|1|10|0|b
`,
			rules: `g|r|F|below|1/2|M
g|s|F|below|1/2|M
g|u|F|below|1/2|M
g|v|F|below|1/2|M
g|w|F|below|1/2|M
`,
			out: `g|r|F|below|1/2|M|1|10|output-diff
g|s|F|below|1/2|M|1|10|output-diff
g|u|F|below|1/2|M|1|10|pass
g|v|F|below|1/2|M|1|10|output-diff
g|w|F|below|1/2|M|10|10|output-diff bound
`,
			code: 1, errs: "stats gate: 4 of 5 rules failed\n",
		},
		{
			// Empty outputs fail even where the outputs agree: none at
			// all (row a, six columns), one arm's (row b, with the
			// diff), only empty columns on both (row c, the same "\t"
			// on both), and one round of ARM (row d).
			name: "empty",
			reps: `g|a|F|1|1|0
g|a|M|1|10|0
g|b|F|1|1|0|x
g|b|M|1|10|0|
g|c|F|1|1|0||
g|c|M|1|10|0||
g|d|F|1|1|0|x
g|d|F|2|1|0|
g|d|M|1|10|0|x
`,
			rules: `g|a|F|below|1/2|M
g|b|F|below|1/2|M
g|c|F|below|1/2|M
g|d|F|below|1/2|M
`,
			out: `g|a|F|below|1/2|M|1|10|empty-output
g|b|F|below|1/2|M|1|10|empty-output output-diff
g|c|F|below|1/2|M|1|10|empty-output
g|d|F|below|1/2|M|1|10|empty-output output-diff
`,
			code: 1, errs: "stats gate: 4 of 4 rules failed\n",
		},
		{
			// Rules print in file order, not sorted; the same rows with
			// another op or another ref are not a repeat.
			name: "order",
			reps: `g|b|F|1|1|0|o
g|b|M|1|10|0|o
g|b|N|1|10|0|o
g|a|F|1|1|0|o
g|a|M|1|10|0|o
`,
			rules: `g|b|F|below|1/2|M
g|b|F|within|10/1|M
g|b|F|below|1/2|N
g|a|F|below|1/2|M
`,
			out: `g|b|F|below|1/2|M|1|10|pass
g|b|F|within|10/1|M|1|10|pass
g|b|F|below|1/2|N|1|10|pass
g|a|F|below|1/2|M|1|10|pass
`,
		},
	} {
		reps := write(t, "reps.tsv", tsv(c.reps))
		rules := write(t, "rules.tsv", tsv(c.rules))
		code, out, errs := runStats(t, "gate", "-rules", rules, reps)
		if code != c.code || errs != c.errs {
			t.Errorf("%s: exit %d, stderr %q; want exit %d, stderr %q", c.name, code, errs, c.code, c.errs)
		}
		if want := tsv(c.out); out != want {
			t.Errorf("%s:\n got:\n%s\nwant:\n%s", c.name, out, want)
		}
	}
}

func TestGateRefuses(t *testing.T) {
	good := "g|r|F|1|1|0|o\ng|r|M|1|10|0|o\n"
	for _, c := range []struct {
		name, reps, rules, want string
	}{
		{"five columns", good, "g|r|F|below|1/2\n", "rules.tsv:1: 5 columns, want 6 (group, row, arm, op, P/Q, ref)"},
		{"seven columns", good, "# x\ng|r|F|below|1/2|M|x\n", "rules.tsv:2: 7 columns, want 6"},
		{"empty group", good, "|r|F|below|1/2|M\n", "rules.tsv:1: empty group"},
		{"empty row", good, "g||F|below|1/2|M\n", "rules.tsv:1: empty row"},
		{"empty arm", good, "g|r||below|1/2|M\n", "rules.tsv:1: empty arm"},
		{"empty op", good, "g|r|F||1/2|M\n", "rules.tsv:1: empty op"},
		{"empty bound", good, "g|r|F|below||M\n", "rules.tsv:1: empty bound"},
		{"empty ref", good, "g|r|F|below|1/2|\n", "rules.tsv:1: empty ref"},
		{"op", good, "g|r|F|above|1/2|M\n", `rules.tsv:1: op "above" is not below or within`},
		{"bound decimal", good, "g|r|F|below|0.05|M\n", `rules.tsv:1: bound "0.05" is not P/Q`},
		{"bound no q", good, "g|r|F|below|1/|M\n", `bound "1/" is not P/Q`},
		{"bound no p", good, "g|r|F|below|/2|M\n", `bound "/2" is not P/Q`},
		{"bound sign", good, "g|r|F|below|-1/2|M\n", `bound "-1/2" is not P/Q`},
		{"bound three", good, "g|r|F|below|1/2/3|M\n", `bound "1/2/3" is not P/Q`},
		{"bound p zero", good, "g|r|F|below|0/2|M\n", `bound "0/2" wants P and Q from 1 to 18446744073709551615`},
		{"bound q zero", good, "g|r|F|below|1/0|M\n", `bound "1/0" wants P and Q from 1`},
		{"bound too big", good, "g|r|F|below|1/18446744073709551616|M\n", `wants P and Q from 1`},
		{"own ref", good, "g|r|F|below|1/2|F\n", `rules.tsv:1: arm "F" is its own ref`},
		{"twice", good, "g|r|F|below|1/2|M\ng|r|F|below|1/3|M\n", "rules.tsv:2: rule g r F below M given twice"},
		{"no rules", good, "# only\n\n", "rules.tsv has no rules"},
		{"empty rules", good, "", "rules.tsv has no rules"},
		{"reps columns", "g|r|F|1|1\n", "g|r|F|below|1/2|M\n", "reps.tsv:1: 5 columns, want at least 6"},
		{"reps counter", "g|r|F|1|<not counted>|0|o\n", "g|r|F|below|1/2|M\n", "reps.tsv:1: instructions \"<not counted>\" is not a decimal number"},
		{"reps repeated", good + "g|r|F|1|1|0|o\n", "g|r|F|below|1/2|M\n", "reps.tsv:3: repeated group, row, arm and round"},
	} {
		reps := write(t, "reps.tsv", tsv(c.reps))
		rules := write(t, "rules.tsv", tsv(c.rules))
		code, out, errs := runStats(t, "gate", "-rules", rules, reps)
		if code != 1 || out != "" || !strings.Contains(errs, c.want) {
			t.Errorf("%s: exit %d, stdout %q, stderr %q; want exit 1, no stdout, %q", c.name, code, out, errs, c.want)
		}
	}
	reps := write(t, "reps.tsv", tsv(good))
	rules := write(t, "rules.tsv", tsv("g|r|F|below|1/2|M\n"))
	for _, c := range []struct {
		name string
		args []string
		want string
	}{
		{"no rules flag", []string{reps}, "stats gate: -rules is required"},
		{"no reps", []string{"-rules", rules}, "stats gate: want one input file, got 0"},
		{"two reps", []string{"-rules", rules, reps, reps}, "stats gate: want one input file, got 2"},
		{"unknown flag", []string{"-x", reps}, "stats gate: flag provided but not defined: -x"},
	} {
		code, out, errs := runStats(t, append([]string{"gate"}, c.args...)...)
		if code != 2 || out != "" || !strings.HasPrefix(errs, c.want+"\n") {
			t.Errorf("%s: exit %d, stdout %q, stderr %q; want exit 2, %q", c.name, code, out, errs, c.want)
		}
	}
}
