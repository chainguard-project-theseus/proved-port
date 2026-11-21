package main

// A pull's generic checks: what a port's pull script did with sed and
// awk between fetching a run's results and summarizing them (PLAN step
// 6, the "pull scripts" row). Fetching the results and picking each
// source's columns stay with the port.
//
// relabel -map MAP REPS prints REPS with its third column (the arm)
// rewritten through MAP, in file order (cmp reads rounds in file order,
// so a pull sorts after relabelling, not before). MAP has one line per
// label the box wrote: the label, a tab, and the arm name. Two labels
// may name one arm (a box that wrote "rev" and "rev:1" for the same
// build). The other columns pass through unchanged. relabel refuses
// (exit 1) an arm label MAP does not name, a MAP line without exactly
// two non-empty columns, a label MAP names twice, and a MAP label on no
// line of REPS, where a sed left an unmatched label as it was.
//
// check [-same ARM,...]... REPS prints one tab-separated finding per
// line:
//
//	counter       group row arm round column value
//	empty-output  group row arm round
//	output-diff   group row arm round first-arm first-round
//
// A counter finding is an instructions or cycles column that is empty
// or not a plain decimal number (perf's "<not counted>"). The output is
// the columns after the sixth, joined by tabs; an empty-output finding
// is a line with none or with only empty ones. Each -same flag names a
// set of arms whose outputs must agree: per (group, row), every line of
// the set's arms against the first such line in file order, its round
// included, so a set of one arm checks that arm's rounds agree. An
// output-diff finding names the line that differs and the first line.
// Outputs of arms in no set are not compared. check exits 1 after
// printing any finding. It refuses (exit 1, naming the file and line) a
// line with fewer than 6 columns, an empty group, row, arm or round, and
// a repeated (group, row, arm, round), as med does; a REPS with no
// lines (an empty fetch would otherwise check clean); an arm a -same
// names that is on no line; and (exit 2) an arm named twice.
//
// md5 LOG... reads md5sum lines ("<32 lowercase hex digits><spaces or
// tabs><file>", the file being the rest of the line) from the logs,
// ignoring every other line, and prints each file that has more than
// one md5: the file, a tab, and its md5s sorted and separated by
// spaces. The spacing between md5 and file does not matter, where the
// awk compared whole lines.
//
// It exits 1 after printing any, and refuses (exit 1, naming the log and
// line) a line that starts "md5sum: " (md5sum's error for a file it
// could not read), and (exit 1) a log with no md5sum line. A pull read
// each of those as no change.

import (
	"fmt"
	"sort"
	"strings"
)

func runRelabel(args []string) ([]string, error) {
	fs := newFlags("relabel")
	mapPath := fs.String("map", "", "file of label, tab, arm lines")
	path, err := oneFile(fs, args)
	if err != nil {
		return nil, err
	}
	if *mapPath == "" {
		return nil, errUsage{"stats relabel: -map is required"}
	}
	arms := map[string]string{}
	var labels []string
	err = readTSV(*mapPath, func(f []string, _ int) error {
		if len(f) != 2 || f[0] == "" || f[1] == "" {
			return fmt.Errorf("want two non-empty columns (label, arm)")
		}
		if _, dup := arms[f[0]]; dup {
			return fmt.Errorf("label %q named twice", f[0])
		}
		arms[f[0]] = f[1]
		labels = append(labels, f[0])
		return nil
	})
	if err != nil {
		return nil, err
	}
	if len(arms) == 0 {
		return nil, fmt.Errorf("stats relabel: %s: no labels", *mapPath)
	}
	used := map[string]bool{}
	var out []string
	err = readTSV(path, func(f []string, _ int) error {
		if len(f) < 3 {
			return fmt.Errorf("%d columns, want at least 3 (group, row, arm)", len(f))
		}
		a, ok := arms[f[2]]
		if !ok {
			return fmt.Errorf("arm label %q is not in %s", f[2], *mapPath)
		}
		used[f[2]] = true
		f[2] = a
		out = append(out, strings.Join(f, "\t"))
		return nil
	})
	if err != nil {
		return nil, err
	}
	sort.Strings(labels)
	if err := allOnSomeLine("relabel", path, "-map label", labels, func(l string) bool { return used[l] }); err != nil {
		return nil, err
	}
	return out, nil
}

// sameFlag is -same: repeatable, each a comma-separated set of arms.
type sameFlag struct {
	sets [][]string
	set  map[string]int // arm -> index in sets
}

func (s *sameFlag) String() string { return "" }

func (s *sameFlag) Set(v string) error {
	if s.set == nil {
		s.set = map[string]int{}
	}
	var arms []string
	for _, a := range strings.Split(v, ",") {
		if a == "" {
			return fmt.Errorf("empty arm in %q", v)
		}
		if _, dup := s.set[a]; dup {
			return fmt.Errorf("arm %q named twice", a)
		}
		s.set[a] = len(s.sets)
		arms = append(arms, a)
	}
	s.sets = append(s.sets, arms)
	return nil
}

func runCheck(args []string) ([]string, error) {
	fs := newFlags("check")
	var same sameFlag
	fs.Var(&same, "same", "ARM,...: arms whose outputs must agree (repeatable)")
	path, err := oneFile(fs, args)
	if err != nil {
		return nil, err
	}
	type first struct{ arm, round, output string }
	// firsts is keyed by set index, group and row.
	firsts := map[[3]string]first{}
	seen := map[[4]string]bool{}
	onLine := map[string]bool{}
	var out []string
	err = readTSV(path, func(f []string, _ int) error {
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
		onLine[f[2]] = true
		id := strings.Join(f[:4], "\t")
		for i, name := range []string{"instructions", "cycles"} {
			if _, err := counter(name, f[4+i]); err != nil {
				out = append(out, fmt.Sprintf("counter\t%s\t%s\t%s", id, name, f[4+i]))
			}
		}
		output := strings.Join(f[6:], "\t")
		if strings.Trim(output, "\t") == "" {
			out = append(out, "empty-output\t"+id)
		}
		if s, ok := same.set[f[2]]; ok {
			fk := [3]string{fmt.Sprint(s), f[0], f[1]}
			if r, ok := firsts[fk]; !ok {
				firsts[fk] = first{f[2], f[3], output}
			} else if r.output != output {
				out = append(out, fmt.Sprintf("output-diff\t%s\t%s\t%s", id, r.arm, r.round))
			}
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	if len(seen) == 0 {
		return nil, fmt.Errorf("stats check: %s: no lines", path)
	}
	var named []string
	for _, set := range same.sets {
		named = append(named, set...)
	}
	if err := allOnSomeLine("check", path, "-same arm", named, func(a string) bool { return onLine[a] }); err != nil {
		return nil, err
	}
	return out, nil
}

// md5Line splits an md5sum line into its md5 and file; ok is false for
// any other line.
func md5Line(l string) (sum, file string, ok bool) {
	if len(l) < 34 {
		return "", "", false
	}
	for i := 0; i < 32; i++ {
		if c := l[i]; !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
			return "", "", false
		}
	}
	if l[32] != ' ' && l[32] != '\t' {
		return "", "", false
	}
	file = strings.TrimLeft(l[32:], " \t")
	if file == "" {
		return "", "", false
	}
	return l[:32], file, true
}

func runMD5(args []string) ([]string, error) {
	fs := newFlags("md5")
	if err := fs.Parse(args); err != nil {
		return nil, errUsage{fmt.Sprintf("stats md5: %v", err)}
	}
	if fs.NArg() == 0 {
		return nil, errUsage{"stats md5: want at least one log"}
	}
	sums := map[string]map[string]bool{}
	for _, path := range fs.Args() {
		n := 0
		err := readTSV(path, func(f []string, _ int) error {
			l := strings.Join(f, "\t")
			// md5sum's own error line: a file it could not read.
			if strings.HasPrefix(l, "md5sum: ") {
				return fmt.Errorf("md5sum could not read a file: %s", l)
			}
			if sum, file, ok := md5Line(l); ok {
				n++
				if sums[file] == nil {
					sums[file] = map[string]bool{}
				}
				sums[file][sum] = true
			}
			return nil
		})
		if err != nil {
			return nil, err
		}
		if n == 0 {
			return nil, fmt.Errorf("stats md5: %s: no md5sum line", path)
		}
	}
	var out []string
	for file, s := range sums {
		if len(s) < 2 {
			continue
		}
		var list []string
		for sum := range s {
			list = append(list, sum)
		}
		sort.Strings(list)
		out = append(out, file+"\t"+strings.Join(list, " "))
	}
	return out, nil
}
