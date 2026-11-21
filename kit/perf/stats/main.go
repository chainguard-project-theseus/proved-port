// Command stats summarizes a bench run's repetitions: per-row medians,
// one arm against a reference arm with half-split noise bands, and how
// many rows each arm is ahead of its reference. It is the kit's Go
// replacement for the awk family a port's bench pulls used (PLAN step 6),
// and prints what that awk printed, byte for byte, with the lines sorted.
//
// Input files are tab-separated. A repetitions file (reps.tsv) has one
// line per run: group, row, arm, round, instructions, cycles, and
// optionally further columns (the run's output), which stats ignores. A
// medians file (med.tsv) is what `stats med` prints.
//
// Usage:
//
//	stats med REPS
//	stats cmp -t ARM -r ARM [-per GROUP=N,...] [-insns-floor PCT] REPS
//	stats ahead -pairs ARM:REF,... [-skip ROW,...] MED
//	stats relabel -map MAP REPS
//	stats check [-same ARM,...]... REPS
//	stats md5 LOG...
//
// relabel, check and md5 are a pull's generic checks; pull.go documents
// them.
//
// med prints group, row, arm, median instructions, median cycles. The
// median of an even count is the lower middle value.
//
// cmp compares arm T (-t) against arm R (-r) per (group, row) both have.
// Columns: group, row, dI (median T - median R instructions), dI per
// pattern ("-" unless -per names the group), dI %, instructions band %,
// dC (per pattern when -per names the group), dC %, cycles band %, mark.
// A band is the larger of T's and R's half-split spread: the rounds, in
// file order (a pull writes them in round order), split three ways
// (odd/even positions, first/second half, alternate pairs), and the
// largest |median(a) - median(b)| / median. The instructions band is at
// least -insns-floor (default 0.02). The mark is "I+" or "I-" when
// |dI %| exceeds its band, "C+" or "C-" likewise for cycles, both
// separated by a space, or "flat". With fewer than three rounds one split
// has an empty half, read as median 0, so the band is 100% (a non-zero
// median): only a change above 100% marks. A row only one of the two
// arms has is left out, with a notice on stderr when the arm it lacks has
// rows elsewhere in the group; a group with no rows of one arm is left
// out silently, and an arm on no line of the file is an error.
//
// ahead prints, per group and pair, the rows (less those -skip names)
// that have both the arm and its reference, and how many of them the
// arm's median is below the reference's on instructions and on cycles.
// Columns: group, arm, rows, ahead instructions, ahead cycles. A pair with
// no such rows in a group prints nothing for that group. An arm is in at
// most one pair.
//
// Unlike the awk, stats refuses (exit 1, naming the file and line) an
// empty counter or one that is not a plain decimal number, an empty
// group, row, arm or round, a line with too few columns, and a
// repeated key: (group, row, arm, round) in a repetitions file, (group,
// row, arm) in a medians file. A medians line must have exactly 5
// columns. It also refuses (exit 1) a name its flags give that is on no
// line of the file, where the awk printed less with no sign: a -t or -r
// arm, a -per group, an -pairs arm or reference, a -skip row. Usage
// errors exit 2.
package main

import (
	"bufio"
	"errors"
	"flag"
	"fmt"
	"io"
	"math"
	"os"
	"sort"
	"strconv"
	"strings"
)

func main() {
	os.Exit(run(os.Args[1:], os.Stdout, os.Stderr))
}

const usage = `usage:
  stats med REPS
  stats cmp -t ARM -r ARM [-per GROUP=N,...] [-insns-floor PCT] REPS
  stats ahead -pairs ARM:REF,... [-skip ROW,...] MED
  stats relabel -map MAP REPS
  stats check [-same ARM,...]... REPS
  stats md5 LOG...
`

// errUsage marks an error that exits 2.
type errUsage struct{ msg string }

func (e errUsage) Error() string { return e.msg }

func run(args []string, stdout, stderr io.Writer) int {
	if len(args) == 0 {
		fmt.Fprint(stderr, usage)
		return 2
	}
	var lines []string
	var err error
	// fileOrder keeps the lines as the command returns them; findings
	// makes any line exit 1 once printed.
	fileOrder, findings := false, false
	switch args[0] {
	case "med":
		lines, err = runMed(args[1:])
	case "cmp":
		lines, err = runCmp(args[1:], stderr)
	case "ahead":
		lines, err = runAhead(args[1:])
	case "relabel":
		lines, err = runRelabel(args[1:])
		fileOrder = true
	case "check":
		lines, err = runCheck(args[1:])
		findings = true
	case "md5":
		lines, err = runMD5(args[1:])
		findings = true
	default:
		err = errUsage{fmt.Sprintf("stats: unknown command %q", args[0])}
	}
	if err != nil {
		fmt.Fprintf(stderr, "%v\n", err)
		var u errUsage
		if errors.As(err, &u) {
			fmt.Fprint(stderr, usage)
			return 2
		}
		return 1
	}
	if !fileOrder {
		sort.Strings(lines)
	}
	w := bufio.NewWriter(stdout)
	for _, l := range lines {
		w.WriteString(l)
		w.WriteByte('\n')
	}
	if err := w.Flush(); err != nil {
		fmt.Fprintf(stderr, "stats: write: %v\n", err)
		return 1
	}
	if findings && len(lines) > 0 {
		fmt.Fprintf(stderr, "stats %s: %d findings\n", args[0], len(lines))
		return 1
	}
	return 0
}

// newFlags returns a flag set that reports errors as usage errors.
func newFlags(name string) *flag.FlagSet {
	fs := flag.NewFlagSet(name, flag.ContinueOnError)
	fs.SetOutput(io.Discard)
	return fs
}

// oneFile parses fs and returns its single positional argument.
func oneFile(fs *flag.FlagSet, args []string) (string, error) {
	if err := fs.Parse(args); err != nil {
		return "", errUsage{fmt.Sprintf("stats %s: %v", fs.Name(), err)}
	}
	if fs.NArg() != 1 {
		return "", errUsage{fmt.Sprintf("stats %s: want one input file, got %d", fs.Name(), fs.NArg())}
	}
	return fs.Arg(0), nil
}

// rep is one line of a repetitions file.
type rep struct {
	group, row, arm string
	insns, cycles   float64
}

// key is a (group, row) pair.
type key struct{ group, row string }

// readTSV calls fn with each line's fields and its 1-based number. A
// final line with no newline counts; an empty file has no lines.
func readTSV(path string, fn func(f []string, n int) error) error {
	fh, err := os.Open(path)
	if err != nil {
		return fmt.Errorf("stats: %v", err)
	}
	defer fh.Close()
	sc := bufio.NewScanner(fh)
	sc.Buffer(make([]byte, 0, 1<<16), 1<<26)
	n := 0
	for sc.Scan() {
		n++
		if err := fn(strings.Split(sc.Text(), "\t"), n); err != nil {
			return fmt.Errorf("stats: %s:%d: %v", path, n, err)
		}
	}
	if err := sc.Err(); err != nil {
		return fmt.Errorf("stats: %s: %v", path, err)
	}
	return nil
}

// counter parses a counter column: digits, optionally a point and more
// digits (what perf stat -x prints for a count).
func counter(name, s string) (float64, error) {
	if s == "" {
		return 0, fmt.Errorf("empty %s", name)
	}
	whole, frac, dot := strings.Cut(s, ".")
	if !digits(whole) || (dot && !digits(frac)) {
		return 0, fmt.Errorf("%s %q is not a decimal number", name, s)
	}
	v, err := strconv.ParseFloat(s, 64)
	if err != nil || math.IsInf(v, 0) {
		return 0, fmt.Errorf("%s %q is out of range", name, s)
	}
	return v, nil
}

// digits reports whether s is one or more ASCII digits.
func digits(s string) bool {
	if s == "" {
		return false
	}
	for i := 0; i < len(s); i++ {
		if s[i] < '0' || s[i] > '9' {
			return false
		}
	}
	return true
}

// readReps reads a repetitions file in file order.
func readReps(path string) ([]rep, error) {
	var reps []rep
	seen := map[[4]string]bool{}
	err := readTSV(path, func(f []string, _ int) error {
		if len(f) < 6 {
			return fmt.Errorf("%d columns, want at least 6 (group, row, arm, round, instructions, cycles)", len(f))
		}
		for i, name := range []string{"group", "row", "arm", "round"} {
			if f[i] == "" {
				return fmt.Errorf("empty %s", name)
			}
		}
		k := [4]string{f[0], f[1], f[2], f[3]}
		if seen[k] {
			return fmt.Errorf("repeated group, row, arm and round %s %s %s %s", f[0], f[1], f[2], f[3])
		}
		seen[k] = true
		i, err := counter("instructions", f[4])
		if err != nil {
			return err
		}
		c, err := counter("cycles", f[5])
		if err != nil {
			return err
		}
		reps = append(reps, rep{f[0], f[1], f[2], i, c})
		return nil
	})
	return reps, err
}

// median returns the middle value of xs, the lower of the two middle
// values for an even count, and 0 for none. It does not modify xs.
func median(xs []float64) float64 {
	if len(xs) == 0 {
		return 0
	}
	s := append([]float64(nil), xs...)
	sort.Float64s(s)
	return s[(len(s)-1)/2]
}

// band is the half-split spread of xs (in file order) as a percentage of
// its median: over three splits (odd/even positions, first/second half,
// alternate pairs), the largest |median(a) - median(b)| / median. It is 0
// when the median is 0.
func band(xs []float64) float64 {
	m := median(xs)
	if m == 0 {
		return 0
	}
	n := len(xs)
	best := 0.0
	for k := 1; k <= 3; k++ {
		var a, b []float64
		for i := 1; i <= n; i++ {
			var inA bool
			switch k {
			case 1:
				inA = i%2 == 1
			case 2:
				inA = 2*i <= n
			default:
				inA = (i-1)%4 < 2
			}
			if inA {
				a = append(a, xs[i-1])
			} else {
				b = append(b, xs[i-1])
			}
		}
		if d := math.Abs(median(a)-median(b)) / math.Abs(m); d > best {
			best = d
		}
	}
	return best * 100
}

// series is one (group, row, arm)'s counters in file order.
type series struct{ insns, cycles []float64 }

// collect groups reps by (group, row, arm) in file order.
func collect(reps []rep) map[[3]string]*series {
	m := map[[3]string]*series{}
	for _, r := range reps {
		k := [3]string{r.group, r.row, r.arm}
		s := m[k]
		if s == nil {
			s = &series{}
			m[k] = s
		}
		s.insns = append(s.insns, r.insns)
		s.cycles = append(s.cycles, r.cycles)
	}
	return m
}

func runMed(args []string) ([]string, error) {
	path, err := oneFile(newFlags("med"), args)
	if err != nil {
		return nil, err
	}
	reps, err := readReps(path)
	if err != nil {
		return nil, err
	}
	var out []string
	for k, s := range collect(reps) {
		out = append(out, fmt.Sprintf("%s\t%s\t%s\t%.0f\t%.0f", k[0], k[1], k[2], median(s.insns), median(s.cycles)))
	}
	return out, nil
}

// perFlag is -per: GROUP=N pairs, N a positive integer.
type perFlag map[string]float64

func (p perFlag) String() string { return "" }

func (p perFlag) Set(v string) error {
	for _, item := range strings.Split(v, ",") {
		g, n, ok := strings.Cut(item, "=")
		if !ok || g == "" {
			return fmt.Errorf("%q is not GROUP=N", item)
		}
		x, err := strconv.ParseUint(n, 10, 32)
		if err != nil || x == 0 {
			return fmt.Errorf("%q: N must be a positive integer", item)
		}
		if _, dup := p[g]; dup {
			return fmt.Errorf("group %q named twice", g)
		}
		p[g] = float64(x)
	}
	return nil
}

func runCmp(args []string, stderr io.Writer) ([]string, error) {
	fs := newFlags("cmp")
	t := fs.String("t", "", "arm compared")
	r := fs.String("r", "", "reference arm")
	floor := fs.Float64("insns-floor", 0.02, "least instructions band, percent")
	per := perFlag{}
	fs.Var(per, "per", "GROUP=N,...: report the group's deltas per pattern, N patterns a run")
	path, err := oneFile(fs, args)
	if err != nil {
		return nil, err
	}
	if *t == "" || *r == "" {
		return nil, errUsage{"stats cmp: -t and -r are required"}
	}
	if *t == *r {
		return nil, errUsage{fmt.Sprintf("stats cmp: -t and -r are both %q", *t)}
	}
	if math.IsNaN(*floor) || math.IsInf(*floor, 0) || *floor < 0 {
		return nil, errUsage{"stats cmp: -insns-floor must be a non-negative number"}
	}
	reps, err := readReps(path)
	if err != nil {
		return nil, err
	}
	all := collect(reps)
	rows := map[key]bool{}
	// inGroup records the (group, arm) pairs with at least one row.
	inGroup := map[[2]string]bool{}
	for k := range all {
		if k[2] == *t || k[2] == *r {
			rows[key{k[0], k[1]}] = true
			inGroup[[2]string{k[0], k[2]}] = true
		}
	}
	for _, arm := range []string{*t, *r} {
		found := false
		for k := range inGroup {
			found = found || k[1] == arm
		}
		if !found {
			return nil, fmt.Errorf("stats cmp: %s: arm %q is on no line", path, arm)
		}
	}
	var perGroups []string
	for g := range per {
		perGroups = append(perGroups, g)
	}
	sort.Strings(perGroups)
	if err := allOnSomeLine("cmp", path, "-per group", perGroups, func(g string) bool {
		for k := range all {
			if k[0] == g {
				return true
			}
		}
		return false
	}); err != nil {
		return nil, err
	}
	var out, missing []string
	for k := range rows {
		ts, rs := all[[3]string{k.group, k.row, *t}], all[[3]string{k.group, k.row, *r}]
		if ts == nil || rs == nil {
			arm := *t
			if ts != nil {
				arm = *r
			}
			if inGroup[[2]string{k.group, arm}] {
				missing = append(missing, fmt.Sprintf("stats cmp: notice: %s %s has no %s; left out", k.group, k.row, arm))
			}
			continue
		}
		out = append(out, cmpRow(k, ts, rs, *floor, per))
	}
	sort.Strings(missing)
	for _, m := range missing {
		fmt.Fprintln(stderr, m)
	}
	return out, nil
}

// allOnSomeLine fails on the first of names (a flag's values) that onLine
// does not find in the file: a misspelled name would otherwise change
// nothing, silently.
func allOnSomeLine(cmd, path, what string, names []string, onLine func(string) bool) error {
	for _, n := range names {
		if !onLine(n) {
			return fmt.Errorf("stats %s: %s: %s %q is on no line", cmd, path, what, n)
		}
	}
	return nil
}

// cmpRow formats one comparison line.
func cmpRow(k key, ts, rs *series, floor float64, per perFlag) string {
	ti, ri := median(ts.insns), median(rs.insns)
	tc, rc := median(ts.cycles), median(rs.cycles)
	bi := math.Max(band(ts.insns), band(rs.insns))
	if bi < floor {
		bi = floor
	}
	bc := math.Max(band(ts.cycles), band(rs.cycles))
	di, dc := ti-ri, tc-rc
	pi, pc := 0.0, 0.0
	if ri != 0 {
		pi = 100 * di / ri
	}
	if rc != 0 {
		pc = 100 * dc / rc
	}
	ip, cp := "-", fmt.Sprintf("%+.0f", dc)
	if n, ok := per[k.group]; ok {
		ip, cp = fmt.Sprintf("%+.1f", di/n), fmt.Sprintf("%+.1f", dc/n)
	}
	var marks []string
	if math.Abs(pi) > bi {
		marks = append(marks, sign(pi, "I"))
	}
	if math.Abs(pc) > bc {
		marks = append(marks, sign(pc, "C"))
	}
	mark := "flat"
	if len(marks) > 0 {
		mark = strings.Join(marks, " ")
	}
	return fmt.Sprintf("%s\t%s\t%+.0f\t%s\t%+.3f\t%.3f\t%s\t%+.3f\t%.3f\t%s", k.group, k.row, di, ip, pi, bi, cp, pc, bc, mark)
}

func sign(x float64, p string) string {
	if x > 0 {
		return p + "+"
	}
	return p + "-"
}

// med is one line of a medians file.
type med struct{ insns, cycles float64 }

// listFlag is a comma-separated list of non-empty names.
type listFlag struct{ items []string }

func (l *listFlag) String() string { return strings.Join(l.items, ",") }

func (l *listFlag) Set(v string) error {
	for _, item := range strings.Split(v, ",") {
		if item == "" {
			return fmt.Errorf("empty name in %q", v)
		}
		l.items = append(l.items, item)
	}
	return nil
}

func runAhead(args []string) ([]string, error) {
	fs := newFlags("ahead")
	var pairs, skip listFlag
	fs.Var(&pairs, "pairs", "ARM:REF,...: count the rows ARM is ahead of REF")
	fs.Var(&skip, "skip", "ROW,...: rows left out of every count")
	path, err := oneFile(fs, args)
	if err != nil {
		return nil, err
	}
	if len(pairs.items) == 0 {
		return nil, errUsage{"stats ahead: -pairs is required"}
	}
	type pair struct{ arm, ref string }
	var ps []pair
	seenArm := map[string]bool{}
	for _, p := range pairs.items {
		a, r, ok := strings.Cut(p, ":")
		if !ok || a == "" || r == "" || a == r || strings.Contains(r, ":") {
			return nil, errUsage{fmt.Sprintf("stats ahead: pair %q is not ARM:REF with two different arms", p)}
		}
		// The output names the arm, not its reference, so one arm has one.
		if seenArm[a] {
			return nil, errUsage{fmt.Sprintf("stats ahead: arm %q is in two pairs", a)}
		}
		seenArm[a] = true
		ps = append(ps, pair{a, r})
	}
	skipped := map[string]bool{}
	for _, s := range skip.items {
		skipped[s] = true
	}
	meds := map[[3]string]med{}
	rows := map[key]bool{}
	err = readTSV(path, func(f []string, _ int) error {
		// Exactly 5: a repetitions file passed by mistake has 6 or more,
		// and its round would read as instructions.
		if len(f) != 5 {
			return fmt.Errorf("%d columns, want 5 (group, row, arm, instructions, cycles)", len(f))
		}
		for i, name := range []string{"group", "row", "arm"} {
			if f[i] == "" {
				return fmt.Errorf("empty %s", name)
			}
		}
		k := [3]string{f[0], f[1], f[2]}
		if _, dup := meds[k]; dup {
			return fmt.Errorf("repeated group, row and arm %s %s %s", f[0], f[1], f[2])
		}
		i, err := counter("instructions", f[3])
		if err != nil {
			return err
		}
		c, err := counter("cycles", f[4])
		if err != nil {
			return err
		}
		meds[k] = med{i, c}
		rows[key{f[0], f[1]}] = true
		return nil
	})
	if err != nil {
		return nil, err
	}
	groups := map[string]bool{}
	arms := map[string]bool{}
	for k := range meds {
		groups[k[0]] = true
		arms[k[2]] = true
	}
	for _, p := range ps {
		for _, a := range []string{p.arm, p.ref} {
			if !arms[a] {
				return nil, fmt.Errorf("stats ahead: %s: arm %q is on no line", path, a)
			}
		}
	}
	if err := allOnSomeLine("ahead", path, "-skip row", skip.items, func(s string) bool {
		for k := range rows {
			if k.row == s {
				return true
			}
		}
		return false
	}); err != nil {
		return nil, err
	}
	var out []string
	for g := range groups {
		for _, p := range ps {
			n, ni, nc := 0, 0, 0
			for k := range rows {
				if k.group != g || skipped[k.row] {
					continue
				}
				a, okA := meds[[3]string{g, k.row, p.arm}]
				r, okR := meds[[3]string{g, k.row, p.ref}]
				if !okA || !okR {
					continue
				}
				n++
				if a.insns < r.insns {
					ni++
				}
				if a.cycles < r.cycles {
					nc++
				}
			}
			if n > 0 {
				out = append(out, fmt.Sprintf("%s\t%s\t%d\t%d\t%d", g, p.arm, n, ni, nc))
			}
		}
	}
	return out, nil
}
