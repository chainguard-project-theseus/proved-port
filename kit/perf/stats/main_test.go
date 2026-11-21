package main

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// runStats runs the command and returns its exit code, stdout and stderr.
func runStats(t *testing.T, args ...string) (int, string, string) {
	t.Helper()
	var out, errb bytes.Buffer
	code := run(args, &out, &errb)
	return code, out.String(), errb.String()
}

// write puts content in a file under a test directory and returns its path.
func write(t *testing.T, name, content string) string {
	t.Helper()
	p := filepath.Join(t.TempDir(), name)
	if err := os.WriteFile(p, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
	return p
}

func golden(t *testing.T, name string) string {
	t.Helper()
	b, err := os.ReadFile(filepath.Join("testdata", "golden", name))
	if err != nil {
		t.Fatal(err)
	}
	return string(b)
}

// cmpCases are the comparisons testdata/ref/gen.sh writes goldens for.
var cmpCases = []struct{ t, r string }{
	{"A0", "C0"}, {"B0", "C0"}, {"D0", "C0"}, {"A0", "W0"},
	{"B1", "C1"}, {"A1", "C1"}, {"D1", "C1"}, {"B1", "W1"}, {"C1", "W0"},
}

const reps = "testdata/reps.tsv"

// The goldens come from the reference awk (testdata/ref), not from stats.
func TestGoldenMed(t *testing.T) {
	code, out, errs := runStats(t, "med", reps)
	if code != 0 || errs != "" {
		t.Fatalf("exit %d, stderr %q", code, errs)
	}
	if want := golden(t, "med.tsv"); out != want {
		t.Errorf("med differs from the reference:\n got:\n%s\nwant:\n%s", out, want)
	}
}

func TestGoldenCmp(t *testing.T) {
	for _, c := range cmpCases {
		code, out, _ := runStats(t, "cmp", "-t", c.t, "-r", c.r, "-per", "perpat=20000,perpat2=20000", reps)
		if code != 0 {
			t.Fatalf("%s-%s: exit %d", c.t, c.r, code)
		}
		if want := golden(t, "cmp-"+c.t+"-"+c.r+".tsv"); out != want {
			t.Errorf("cmp %s %s differs from the reference:\n got:\n%s\nwant:\n%s", c.t, c.r, out, want)
		}
	}
}

func TestGoldenAhead(t *testing.T) {
	code, out, _ := runStats(t, "ahead", "-pairs", "A1:W1,B1:W1,C1:W1,D1:W1,A0:W0,B0:W0,C0:W0,D0:W0", "-skip", "none-1", "testdata/golden/med.tsv")
	if code != 0 {
		t.Fatalf("exit %d", code)
	}
	if want := golden(t, "ahead.tsv"); out != want {
		t.Errorf("ahead differs from the reference:\n got:\n%s\nwant:\n%s", out, want)
	}
}

// The fixture holds the shapes the goldens are meant to cover.
func TestFixtureShapes(t *testing.T) {
	rs, err := readReps(reps)
	if err != nil {
		t.Fatal(err)
	}
	counts := map[[3]string]int{}
	for _, r := range rs {
		counts[[3]string{r.group, r.row, r.arm}]++
	}
	for _, c := range []struct {
		k [3]string
		n int
	}{
		{[3]string{"plain", "s1-1", "A0"}, 4},     // an even count
		{[3]string{"perpat", "s4-64", "C1"}, 2},   // two rounds
		{[3]string{"perpat", "s5-512", "A1"}, 1},  // one round
		{[3]string{"plain", "s2-512", "W0"}, 0},   // a missing arm
		{[3]string{"perpat", "none-1", "W1"}, 5},  // a skipped row
		{[3]string{"perpat2", "s6-256", "B1"}, 5}, // the second per-pattern group
	} {
		if counts[c.k] != c.n {
			t.Errorf("%v: %d rounds, want %d", c.k, counts[c.k], c.n)
		}
	}
}

// A row only one arm has is left out with a notice.
func TestCmpMissingArmNotice(t *testing.T) {
	code, out, errs := runStats(t, "cmp", "-t", "A0", "-r", "W0", reps)
	if code != 0 {
		t.Fatalf("exit %d", code)
	}
	if want := "stats cmp: notice: plain s2-512 has no W0; left out\n"; errs != want {
		t.Errorf("stderr %q, want %q", errs, want)
	}
	if strings.Contains(out, "s2-512") {
		t.Errorf("s2-512 printed:\n%s", out)
	}
	// perpat2 has no A0 anywhere: its rows are left out with no notice.
	if strings.Contains(out, "perpat2") {
		t.Errorf("perpat2 printed:\n%s", out)
	}
}

// The bands read the rounds in file order: the same rounds in another
// order give another band. R [100 100 200 200]: median 100; halves
// [100 100] 100 and [200 200] 200: 100%. R [100 200 200 100]: odd
// [100 200] 100, even [200 100] 100; halves [100 200] and [200 100];
// pairs the same as the halves: 0. T is 100 throughout (band 0).
func TestCmpBandFileOrder(t *testing.T) {
	for _, c := range []struct{ cycles, want string }{
		{"100 100 200 200", "100.000"},
		{"100 200 200 100", "0.000"},
		{"100 200 100 200", "100.000"}, // odd [100 100], even [200 200]
	} {
		var b strings.Builder
		for i, v := range strings.Fields(c.cycles) {
			r := string(rune('1' + i))
			b.WriteString("g\tr\tT\t" + r + "\t10\t100\n")
			b.WriteString("g\tr\tR\t" + r + "\t10\t" + v + "\n")
		}
		p := write(t, "reps.tsv", b.String())
		_, out, _ := runStats(t, "cmp", "-t", "T", "-r", "R", p)
		if f := strings.Split(strings.TrimSuffix(out, "\n"), "\t"); len(f) != 10 || f[8] != c.want {
			t.Errorf("%s: %q, want cycles band %s", c.cycles, out, c.want)
		}
	}
}

// An arm on no line is an error, not an empty comparison.
func TestCmpArmOnNoLine(t *testing.T) {
	for _, args := range [][]string{{"-t", "X0", "-r", "C0"}, {"-t", "A0", "-r", "X0"}} {
		code, out, errs := runStats(t, append(append([]string{"cmp"}, args...), reps)...)
		if code != 1 || out != "" || !strings.Contains(errs, `arm "X0" is on no line`) {
			t.Errorf("%v: exit %d, stdout %q, stderr %q", args, code, out, errs)
		}
	}
}

func TestMedian(t *testing.T) {
	for _, c := range []struct {
		in   []float64
		want float64
	}{
		{nil, 0},
		{[]float64{7}, 7},
		{[]float64{5, 1, 3}, 3},
		{[]float64{4, 1, 3, 2}, 2}, // the lower middle value
		{[]float64{9, 9, 1, 1}, 1},
	} {
		in := append([]float64(nil), c.in...)
		if got := median(c.in); got != c.want {
			t.Errorf("median(%v) = %v, want %v", c.in, got, c.want)
		}
		for i := range in {
			if in[i] != c.in[i] {
				t.Errorf("median reordered its input: %v", c.in)
			}
		}
	}
}

// Bands worked by hand from the definition.
func TestBand(t *testing.T) {
	for _, c := range []struct {
		in   []float64
		want float64
	}{
		// median 100; odd [100 100 100] 100, even [110 110] 110: 10%;
		// halves [100 110] 100 and [100 110 100] 100; pairs (1,2,5)
		// [100 110 100] 100 and (3,4) [100 110] 100.
		{[]float64{100, 110, 100, 110, 100}, 10},
		// median 20; odd [10 30] 10, even [20 40] 20: 50%; halves
		// [10 20] 10, [30 40] 30: 100%; pairs the same as the halves.
		{[]float64{10, 20, 30, 40}, 100},
		// One round: every split has an empty half, read as median 0.
		{[]float64{50}, 100},
		// Two rounds: the pairs split puts both in one half.
		{[]float64{50, 50}, 100},
		{[]float64{0, 0, 0}, 0}, // a zero median gives 0
		// A zero median with a non-zero half (pairs [0 0] and [5]) still
		// gives 0, not 5 / 0.
		{[]float64{0, 0, 5}, 0},
		{[]float64{100, 100, 100}, 0},
	} {
		if got := band(c.in); got != c.want {
			t.Errorf("band(%v) = %v, want %v", c.in, got, c.want)
		}
	}
}

// Comparison lines worked by hand. T insns 1000 x3, R insns 900 x3: dI
// +100, +11.111%, band 0 floored to 0.020. Cycles T 500 x3, R [500 510
// 500]: dC 0, band 2% (R's odd/even split, |500 - 510| / 500).
const handReps = "g\tr\tT\t1\t1000\t500\n" +
	"g\tr\tT\t2\t1000\t500\n" +
	"g\tr\tT\t3\t1000\t500\n" +
	"g\tr\tR\t1\t900\t500\n" +
	"g\tr\tR\t2\t900\t510\n" +
	"g\tr\tR\t3\t900\t500\n" +
	// One round each: T's cycles 600 and R's 500 give dC +100, +20%, but
	// every band is 100% (each split has an empty half), so flat.
	"h\tq\tT\t1\t1000\t600\n" +
	"h\tq\tR\t1\t1000\t500\n"

func TestCmpHandWorked(t *testing.T) {
	p := write(t, "reps.tsv", handReps)
	for _, c := range []struct {
		args []string
		want string
	}{
		{nil, "g\tr\t+100\t-\t+11.111\t0.020\t+0\t+0.000\t2.000\tI+\n" +
			"h\tq\t+0\t-\t+0.000\t100.000\t+100\t+20.000\t100.000\tflat\n"},
		{[]string{"-per", "g=10"}, "g\tr\t+100\t+10.0\t+11.111\t0.020\t+0.0\t+0.000\t2.000\tI+\n" +
			"h\tq\t+0\t-\t+0.000\t100.000\t+100\t+20.000\t100.000\tflat\n"},
		// A floor above |dI %| makes it flat.
		{[]string{"-insns-floor", "12"}, "g\tr\t+100\t-\t+11.111\t12.000\t+0\t+0.000\t2.000\tflat\n" +
			"h\tq\t+0\t-\t+0.000\t100.000\t+100\t+20.000\t100.000\tflat\n"},
	} {
		args := append([]string{"cmp", "-t", "T", "-r", "R"}, c.args...)
		code, out, errs := runStats(t, append(args, p)...)
		if code != 0 || errs != "" {
			t.Fatalf("%v: exit %d, stderr %q", c.args, code, errs)
		}
		if out != c.want {
			t.Errorf("%v:\n got %q\nwant %q", c.args, out, c.want)
		}
	}
	// |dI %| exactly at its band is flat: a mark needs it to exceed the
	// band. T 1010 against R 1000 is +1% (exact in binary floating point).
	q := write(t, "eq.tsv", "g\tr\tT\t1\t1010\t5\ng\tr\tT\t2\t1010\t5\ng\tr\tT\t3\t1010\t5\n"+
		"g\tr\tR\t1\t1000\t5\ng\tr\tR\t2\t1000\t5\ng\tr\tR\t3\t1000\t5\n")
	for _, c := range []struct {
		floor, want string
	}{
		{"1", "g\tr\t+10\t-\t+1.000\t1.000\t+0\t+0.000\t0.000\tflat\n"},
		{"0.999", "g\tr\t+10\t-\t+1.000\t0.999\t+0\t+0.000\t0.000\tI+\n"},
	} {
		code, out, _ := runStats(t, "cmp", "-t", "T", "-r", "R", "-insns-floor", c.floor, q)
		if code != 0 || out != c.want {
			t.Errorf("floor %s: exit %d\n got %q\nwant %q", c.floor, code, out, c.want)
		}
	}
	// Swapped, the signs swap; the bands do not change.
	code, out, _ := runStats(t, "cmp", "-t", "R", "-r", "T", p)
	if want := "g\tr\t-100\t-\t-10.000\t0.020\t+0\t+0.000\t2.000\tI-\n" +
		"h\tq\t+0\t-\t+0.000\t100.000\t-100\t-16.667\t100.000\tflat\n"; code != 0 || out != want {
		t.Errorf("swapped: exit %d\n got %q\nwant %q", code, out, want)
	}
}

// Both marks, with a zero reference: the percentage reads 0.
func TestCmpMarksAndZero(t *testing.T) {
	var b strings.Builder
	for i, v := range [][4]string{{"2000", "900", "1000", "1000"}, {"2000", "900", "1000", "1000"}, {"2000", "900", "1000", "1000"}} {
		r := string(rune('1' + i))
		b.WriteString("g\tr\tT\t" + r + "\t" + v[0] + "\t" + v[1] + "\n")
		b.WriteString("g\tr\tR\t" + r + "\t" + v[2] + "\t" + v[3] + "\n")
		b.WriteString("z\tr\tT\t" + r + "\t5\t5\n")
		b.WriteString("z\tr\tR\t" + r + "\t0\t0\n")
	}
	p := write(t, "reps.tsv", b.String())
	code, out, _ := runStats(t, "cmp", "-t", "T", "-r", "R", p)
	want := "g\tr\t+1000\t-\t+100.000\t0.020\t-100\t-10.000\t0.000\tI+ C-\n" +
		"z\tr\t+5\t-\t+0.000\t0.020\t+5\t+0.000\t0.000\tflat\n"
	if code != 0 || out != want {
		t.Errorf("exit %d\n got %q\nwant %q", code, out, want)
	}
}

func TestAheadHandWorked(t *testing.T) {
	med := "g\tr1\tA\t10\t10\n" +
		"g\tr1\tW\t20\t5\n" + // A ahead on instructions only
		"g\tr2\tA\t20\t20\n" +
		"g\tr2\tW\t20\t30\n" + // a tie on instructions; ahead on cycles
		"g\tr4\tA\t30\t7\n" +
		"g\tr4\tW\t20\t7\n" + // behind on instructions; a tie on cycles
		"g\tskip\tA\t1\t1\n" +
		"g\tskip\tW\t9\t9\n" + // skipped
		"g\tr3\tA\t1\t1\n" + // no W
		"h\tr1\tW\t1\t1\n" + // no A: h prints nothing for A
		"h\tr1\tB\t0\t2\n"
	p := write(t, "med.tsv", med)
	code, out, errs := runStats(t, "ahead", "-pairs", "A:W,B:W", "-skip", "skip", p)
	if want := "g\tA\t3\t1\t1\nh\tB\t1\t1\t0\n"; code != 0 || out != want || errs != "" {
		t.Errorf("exit %d, stderr %q\n got %q\nwant %q", code, errs, out, want)
	}
}

// Input stats refuses, where the awk read on: exit 1, naming the line.
func TestRefusedInput(t *testing.T) {
	ok := "g\tr\tA\t1\t10\t10\tout\n"
	for _, c := range []struct {
		name, cmd, in, want string
	}{
		{"empty instructions", "med", ok + "g\tr\tA\t2\t\t10\n", "reps.tsv:2: empty instructions"},
		{"empty cycles", "med", "g\tr\tA\t2\t10\t\n", "reps.tsv:1: empty cycles"},
		{"not counted", "med", "g\tr\tA\t2\t<not counted>\t10\n", `instructions "<not counted>" is not a decimal number`},
		{"exponent", "med", "g\tr\tA\t2\t1e3\t10\n", `instructions "1e3" is not a decimal number`},
		{"negative", "med", "g\tr\tA\t2\t-1\t10\n", "not a decimal number"},
		{"hex", "med", "g\tr\tA\t2\t0x10\t10\n", "not a decimal number"},
		{"trailing point", "med", "g\tr\tA\t2\t10.\t10\n", "not a decimal number"},
		{"short line", "med", ok + "g\tr\tA\t2\t10\n", "reps.tsv:2: 5 columns, want at least 6"},
		{"empty group", "med", ok + "\tr\tA\t2\t10\t10\n", "reps.tsv:2: empty group"},
		{"empty row", "med", ok + "g\t\tA\t2\t10\t10\n", "reps.tsv:2: empty row"},
		{"empty arm", "med", "g\tr\t\t2\t10\t10\n", "empty arm"},
		{"empty round", "med", "g\tr\tA\t\t10\t10\n", "empty round"},
		{"repeated round", "med", ok + ok, "reps.tsv:2: repeated group, row, arm and round g r A 1"},
		{"cmp reads all lines", "cmp -t A -r B", ok + "g\tr\tC\t1\tx\t1\n", "reps.tsv:2:"},
		{"ahead short", "ahead -pairs A:B", "g\tr\tA\t1\n", "med.tsv:1: 4 columns, want 5"},
		// ahead reads its own lines: empty names there are refused too.
		{"ahead empty group", "ahead -pairs A:B", "g\tr\tA\t1\t1\n\tr\tB\t2\t2\n", "med.tsv:2: empty group"},
		{"ahead empty row", "ahead -pairs A:B", "g\t\tA\t1\t1\ng\t\tB\t2\t2\n", "med.tsv:1: empty row"},
		{"ahead empty arm", "ahead -pairs A:B", "g\tr\tA\t1\t1\ng\tr\t\t2\t2\n", "med.tsv:2: empty arm"},
		// A repetitions line (one round) passed to ahead by mistake.
		{"ahead long", "ahead -pairs A:B", "g\tr\tA\t1\t10\t10\n", "med.tsv:1: 6 columns, want 5"},
		// Names the flags give that are on no line.
		{"cmp -per group", "cmp -t A -r B -per g=2,gx=2", "g\tr\tA\t1\t1\t1\ng\tr\tB\t1\t1\t1\n", `-per group "gx" is on no line`},
		{"ahead arm", "ahead -pairs X:B", "g\tr\tA\t1\t1\ng\tr\tB\t1\t1\n", `arm "X" is on no line`},
		{"ahead reference", "ahead -pairs A:W2", "g\tr\tA\t1\t1\ng\tr\tB\t1\t1\n", `arm "W2" is on no line`},
		{"ahead -skip row", "ahead -pairs A:B -skip r,rx", "g\tr\tA\t1\t1\ng\tr\tB\t1\t1\n", `-skip row "rx" is on no line`},
		{"ahead repeated", "ahead -pairs A:B", "g\tr\tA\t1\t1\ng\tr\tA\t2\t2\n", "med.tsv:2: repeated group, row and arm g r A"},
		{"ahead counter", "ahead -pairs A:B", "g\tr\tA\t1\tn/a\n", `cycles "n/a"`},
	} {
		name := "reps.tsv"
		if strings.HasPrefix(c.cmd, "ahead") {
			name = "med.tsv"
		}
		p := write(t, name, c.in)
		code, out, errs := runStats(t, append(strings.Fields(c.cmd), p)...)
		if code != 1 || out != "" || !strings.Contains(errs, c.want) {
			t.Errorf("%s: exit %d, stdout %q, stderr %q; want exit 1 and %q", c.name, code, out, errs, c.want)
		}
	}
}

func TestMissingFile(t *testing.T) {
	code, _, errs := runStats(t, "med", filepath.Join(t.TempDir(), "none.tsv"))
	if code != 1 || !strings.Contains(errs, "none.tsv") {
		t.Errorf("exit %d, stderr %q", code, errs)
	}
}

func TestEmptyFile(t *testing.T) {
	p := write(t, "reps.tsv", "")
	code, out, errs := runStats(t, "med", p)
	if code != 0 || out != "" || errs != "" {
		t.Errorf("exit %d, stdout %q, stderr %q", code, out, errs)
	}
}

// A last line with no newline is read.
func TestNoFinalNewline(t *testing.T) {
	p := write(t, "reps.tsv", "g\tr\tA\t1\t10\t20\ng\tr\tA\t2\t30\t40")
	code, out, _ := runStats(t, "med", p)
	if want := "g\tr\tA\t10\t20\n"; code != 0 || out != want {
		t.Errorf("exit %d, got %q, want %q", code, out, want)
	}
}

func TestUsageErrors(t *testing.T) {
	p := write(t, "x.tsv", "")
	for _, c := range []struct {
		args []string
		want string
	}{
		{nil, "usage:"},
		{[]string{"tab"}, `unknown command "tab"`},
		{[]string{"med"}, "want one input file, got 0"},
		{[]string{"med", p, p}, "want one input file, got 2"},
		{[]string{"med", "-x", p}, "flag provided but not defined"},
		{[]string{"cmp", "-r", "B", p}, "-t and -r are required"},
		{[]string{"cmp", "-t", "A", p}, "-t and -r are required"},
		{[]string{"cmp", "-t", "A", "-r", "A", p}, `-t and -r are both "A"`},
		{[]string{"cmp", "-t", "A", "-r", "B", "-per", "g", p}, `"g" is not GROUP=N`},
		{[]string{"cmp", "-t", "A", "-r", "B", "-per", "=5", p}, `"=5" is not GROUP=N`},
		{[]string{"cmp", "-t", "A", "-r", "B", "-per", "g=0", p}, "N must be a positive integer"},
		{[]string{"cmp", "-t", "A", "-r", "B", "-per", "g=x", p}, "N must be a positive integer"},
		{[]string{"cmp", "-t", "A", "-r", "B", "-per", "g=1,g=2", p}, `group "g" named twice`},
		{[]string{"cmp", "-t", "A", "-r", "B", "-insns-floor", "-1", p}, "-insns-floor must be a non-negative number"},
		{[]string{"cmp", "-t", "A", "-r", "B", "-insns-floor", "NaN", p}, "-insns-floor must be a non-negative number"},
		{[]string{"ahead", p}, "-pairs is required"},
		{[]string{"ahead", "-pairs", "A", p}, `pair "A" is not ARM:REF`},
		{[]string{"ahead", "-pairs", "A:A", p}, `pair "A:A" is not ARM:REF`},
		{[]string{"ahead", "-pairs", "A:B:C", p}, `pair "A:B:C" is not ARM:REF`},
		{[]string{"ahead", "-pairs", ":B", p}, `pair ":B" is not ARM:REF`},
		{[]string{"ahead", "-pairs", "A:B,A:C", p}, `arm "A" is in two pairs`},
		{[]string{"ahead", "-pairs", "A:B,", p}, "empty name"},
		{[]string{"ahead", "-pairs", "A:B", "-skip", "", p}, "empty name"},
	} {
		code, out, errs := runStats(t, c.args...)
		if code != 2 || out != "" || !strings.Contains(errs, c.want) || !strings.Contains(errs, "usage:") {
			t.Errorf("%v: exit %d, stdout %q, stderr %q; want exit 2 and %q", c.args, code, out, errs, c.want)
		}
	}
}

type failWriter struct{}

func (failWriter) Write([]byte) (int, error) { return 0, errors.New("disk full") }

func TestWriteFailure(t *testing.T) {
	var errb bytes.Buffer
	if code := run([]string{"med", reps}, failWriter{}, &errb); code != 1 || !strings.Contains(errb.String(), "disk full") {
		t.Errorf("exit %d, stderr %q", code, errb.String())
	}
}
