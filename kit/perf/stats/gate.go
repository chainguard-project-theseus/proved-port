package main

// gate -rules RULES REPS is the sanity gate a box runs before the full
// run (PLAN step 6, section 3 item 5): a few rows run once or a few
// times, and the run goes on only if each arm's instruction count sits
// where it should against a reference arm. Port A's gate wanted one
// arm below 1/20 of the other on one row, within 1% of it on another,
// both counts positive and both outputs the same; the rows, arms and
// bounds are a port's, so they come from RULES.
//
// RULES has one rule per line, tab-separated:
//
//	GROUP ROW ARM OP P/Q REF
//
// Lines that start with # and empty lines are skipped. OP is "below"
// (median(ARM) * Q < P * median(REF)) or "within" (|median(ARM) -
// median(REF)| * Q < P * median(REF)), both strict, as port A's awk
// was. P and Q are positive integers, so a bound such as 1/20 or 1/100
// is exact. The medians are of the instructions column of REPS's lines
// with that group and row, stats's median (the lower middle value).
//
// It prints one line per rule, in RULES order: group, row, arm, op,
// bound, ref, ARM's median, REF's median, and "pass" or the checks the
// rule fails, separated by spaces:
//
//	no-arm       ARM has no line on the row (medians print as -)
//	no-ref       REF has no line on the row
//	zero          a median is 0
//	empty-output  a line of ARM or REF on the row has no output, or
//	              only empty columns (check's empty-output), so its
//	              output proves nothing
//	output-diff   the lines of ARM and REF on the row do not all have
//	              the output of the first of them in file order (the
//	              columns after the sixth, as check reads them)
//	bound         the bound does not hold
//
// It exits 1 when any rule fails, after printing every rule. REPS is
// read as med reads it, and refused likewise. RULES is refused (exit 1,
// naming the file and line) for a line that is not six non-empty
// columns, an OP other than below and within, a bound that is not P/Q
// with P and Q positive integers, ARM equal to REF, a rule given twice
// (group, row, arm, op and ref), and a file with no rules (a gate that
// checks nothing would pass). A failed rule and a refusal both exit 1,
// as check and md5 do; a failed rule prints every rule's line on
// stdout, and a refusal prints nothing there, only its error on stderr.
// Both mean the full run must not start.
//
// gate reads counts and outputs only; a REPS line carries no exit
// status. Two arms whose commands fail alike (a missing input file, the
// same error text, nearly the same count) can pass a within rule. So
// the phase that runs the gate's rows must stop on the runner's own
// exit as well (rounds.sh exits 1 when a command exits non-zero), and
// run gate only after it passed.

import (
	"fmt"
	"math"
	"math/big"
	"strconv"
	"strings"
)

// rule is one line of a rules file.
type rule struct {
	group, row, arm, op, bound, ref string
	p, q                            *big.Rat
}

// readRules reads a rules file in file order.
func readRules(path string) ([]rule, error) {
	var rules []rule
	seen := map[[5]string]bool{}
	err := readTSV(path, func(f []string, _ int) error {
		if (len(f) == 1 && f[0] == "") || strings.HasPrefix(f[0], "#") {
			return nil
		}
		if len(f) != 6 {
			return fmt.Errorf("%d columns, want 6 (group, row, arm, op, P/Q, ref)", len(f))
		}
		for i, name := range []string{"group", "row", "arm", "op", "bound", "ref"} {
			if f[i] == "" {
				return fmt.Errorf("empty %s", name)
			}
		}
		r := rule{group: f[0], row: f[1], arm: f[2], op: f[3], bound: f[4], ref: f[5]}
		if r.op != "below" && r.op != "within" {
			return fmt.Errorf("op %q is not below or within", r.op)
		}
		ps, qs, ok := strings.Cut(r.bound, "/")
		if !ok || !digits(ps) || !digits(qs) {
			return fmt.Errorf("bound %q is not P/Q", r.bound)
		}
		p, perr := strconv.ParseUint(ps, 10, 64)
		q, qerr := strconv.ParseUint(qs, 10, 64)
		if perr != nil || qerr != nil || p == 0 || q == 0 {
			return fmt.Errorf("bound %q wants P and Q from 1 to %d", r.bound, uint64(math.MaxUint64))
		}
		r.p, r.q = new(big.Rat).SetUint64(p), new(big.Rat).SetUint64(q)
		if r.arm == r.ref {
			return fmt.Errorf("arm %q is its own ref", r.arm)
		}
		k := [5]string{r.group, r.row, r.arm, r.op, r.ref}
		if seen[k] {
			return fmt.Errorf("rule %s %s %s %s %s given twice", r.group, r.row, r.arm, r.op, r.ref)
		}
		seen[k] = true
		rules = append(rules, r)
		return nil
	})
	if err == nil && len(rules) == 0 {
		err = fmt.Errorf("stats gate: %s has no rules", path)
	}
	return rules, err
}

func runGate(args []string) ([]string, int, error) {
	fs := newFlags("gate")
	rulesPath := fs.String("rules", "", "RULES: GROUP ROW ARM OP P/Q REF lines")
	path, err := oneFile(fs, args)
	if err != nil {
		return nil, 0, err
	}
	if *rulesPath == "" {
		return nil, 0, errUsage{"stats gate: -rules is required"}
	}
	rules, err := readRules(*rulesPath)
	if err != nil {
		return nil, 0, err
	}
	reps, err := readReps(path)
	if err != nil {
		return nil, 0, err
	}
	var out []string
	failed := 0
	for _, r := range rules {
		var a, b []float64
		var first *string
		diff, empty := false, false
		for i := range reps {
			x := &reps[i]
			if x.group != r.group || x.row != r.row || (x.arm != r.arm && x.arm != r.ref) {
				continue
			}
			if x.arm == r.arm {
				a = append(a, x.insns)
			} else {
				b = append(b, x.insns)
			}
			if strings.Trim(x.output, "\t") == "" {
				empty = true
			}
			if first == nil {
				first = &x.output
			} else if x.output != *first {
				diff = true
			}
		}
		var why []string
		ma, mb := "-", "-"
		if len(a) == 0 {
			why = append(why, "no-arm")
		}
		if len(b) == 0 {
			why = append(why, "no-ref")
		}
		if len(a) > 0 && len(b) > 0 {
			x, y := median(a), median(b)
			ma, mb = num(x), num(y)
			if x == 0 || y == 0 {
				why = append(why, "zero")
			}
			if empty {
				why = append(why, "empty-output")
			}
			if diff {
				why = append(why, "output-diff")
			}
			// Exact: x and y are the counts as parsed, P and Q integers.
			l := new(big.Rat).SetFloat64(x)
			if r.op == "within" {
				l.Sub(l, new(big.Rat).SetFloat64(y))
				l.Abs(l)
			}
			l.Mul(l, r.q)
			rt := new(big.Rat).Mul(r.p, new(big.Rat).SetFloat64(y))
			if l.Cmp(rt) >= 0 {
				why = append(why, "bound")
			}
		}
		verdict := "pass"
		if len(why) > 0 {
			verdict = strings.Join(why, " ")
			failed++
		}
		out = append(out, strings.Join([]string{r.group, r.row, r.arm, r.op, r.bound, r.ref, ma, mb, verdict}, "\t"))
	}
	return out, failed, nil
}

// num prints a median as perf printed the count: no exponent, and no
// trailing zeros after a point.
func num(v float64) string {
	return strconv.FormatFloat(v, 'f', -1, 64)
}
