package main

import (
	"os"
	"strconv"
	"strings"
	"testing"
)

// The goldens come from the reference awk (testdata/ref), not from stats.
func TestGoldenRelabel(t *testing.T) {
	code, out, errs := runStats(t, "relabel", "-map", "testdata/pull/map.tsv", "testdata/pull/raw.tsv")
	if code != 0 || errs != "" {
		t.Fatalf("exit %d, stderr %q", code, errs)
	}
	if want := golden(t, "relabel.tsv"); out != want {
		t.Errorf("relabel differs from the reference:\n got:\n%s\nwant:\n%s", out, want)
	}
}

func TestGoldenCheck(t *testing.T) {
	for _, c := range []struct {
		golden string
		sets   []string
	}{
		{"check.tsv", []string{"A0,B0,C0,D0", "A1,B1,C1,D1"}},
		{"check-w1.tsv", []string{"A1,B1,C1,D1", "W1"}},
	} {
		var args []string
		for _, s := range c.sets {
			args = append(args, "-same", s)
		}
		want := golden(t, c.golden)
		n := strings.Count(want, "\n")
		code, out, errs := runStats(t, append(append([]string{"check"}, args...), "testdata/pull/check.tsv")...)
		if code != 1 || errs != "stats check: "+strconv.Itoa(n)+" findings\n" {
			t.Errorf("%s: exit %d, stderr %q, want exit 1 and %d findings", c.golden, code, errs, n)
		}
		if out != want {
			t.Errorf("%s: check differs from the reference:\n got:\n%s\nwant:\n%s", c.golden, out, want)
		}
	}
}

func TestGoldenMD5(t *testing.T) {
	code, out, errs := runStats(t, "md5", "testdata/pull/before.log", "testdata/pull/after.log")
	if code != 1 || errs != "stats md5: 2 findings\n" {
		t.Errorf("exit %d, stderr %q", code, errs)
	}
	if want := golden(t, "md5.tsv"); out != want {
		t.Errorf("md5 differs from the reference:\n got:\n%s\nwant:\n%s", out, want)
	}
}

// The pull fixtures hold the shapes the goldens are meant to cover.
func TestPullFixtureShapes(t *testing.T) {
	m := golden(t, "relabel.tsv")
	raw := readFile(t, "testdata/pull/raw.tsv")
	if m == raw {
		t.Error("relabel golden equals its input")
	}
	// Two labels name A1, and the input is not in sorted order.
	if !strings.Contains(raw, "\trev-a\t") || !strings.Contains(raw, "\trev-a:1\t") {
		t.Error("raw.tsv lacks two labels for one arm")
	}
	for _, g := range []string{"check.tsv", "check-w1.tsv"} {
		c := golden(t, g)
		for _, kind := range []string{"counter\t", "empty-output\t", "output-diff\t", "\tcycles\t<not counted>"} {
			if !strings.Contains(c, kind) {
				t.Errorf("%s has no %q finding", g, kind)
			}
		}
	}
	// W1's differences show only when a set names W1.
	for _, l := range strings.Split(golden(t, "check.tsv"), "\n") {
		if strings.HasPrefix(l, "output-diff\t") && strings.Contains(l, "\tW1\t") {
			t.Errorf("check.tsv compares W1, which no set names: %q", l)
		}
	}
	if !strings.Contains(golden(t, "check-w1.tsv"), "output-diff\tperpat\ts1-1\tW1\t2\tW1\t1\n") {
		t.Error("check-w1.tsv lacks W1's single-arm round comparison")
	}
}

func readFile(t *testing.T, p string) string {
	t.Helper()
	b, err := os.ReadFile(p)
	if err != nil {
		t.Fatal(err)
	}
	return string(b)
}

func TestRelabelHandWorked(t *testing.T) {
	m := write(t, "map.tsv", "x:0\tX0\nx\tX1\nx:1\tX1\n")
	// File order, other columns (an empty last one included) unchanged.
	r := write(t, "raw.tsv", "g\tb\tx\t2\t5\t6\tout a\ng\ta\tx:0\t1\t7\t8\t\ng\ta\tx:1\t1\t9\t9\tout\tmore")
	code, out, errs := runStats(t, "relabel", "-map", m, r)
	want := "g\tb\tX1\t2\t5\t6\tout a\ng\ta\tX0\t1\t7\t8\t\ng\ta\tX1\t1\t9\t9\tout\tmore\n"
	if code != 0 || errs != "" || out != want {
		t.Errorf("exit %d, stderr %q, out:\n%s\nwant:\n%s", code, errs, out, want)
	}
}

func TestRelabelRefused(t *testing.T) {
	raw := "g\tr\tx\t1\t5\t6\tout\n"
	for _, c := range []struct{ name, mp, raw, want string }{
		{"unmapped label", "y\tY\n", "g\tr\ty\t1\t5\t6\no\tr\tz\t1\t5\t6\n", `raw.tsv:2: arm label "z" is not in`},
		{"one-column map", "x\n", raw, "map.tsv:1: want two non-empty columns"},
		{"three-column map", "x\tX\textra\n", raw, "map.tsv:1: want two non-empty columns"},
		{"empty arm", "x\t\n", raw, "map.tsv:1: want two non-empty columns"},
		{"empty label", "\tX\n", raw, "map.tsv:1: want two non-empty columns"},
		{"label twice", "x\tX\nx\tY\n", raw, `map.tsv:2: label "x" named twice`},
		{"empty map", "", raw, "map.tsv: no labels"},
		{"unused label", "x\tX\nq\tQ\n", raw, `-map label "q" is on no line`},
		{"short line", "x\tX\n", "g\tx\n", "raw.tsv:1: 2 columns, want at least 3"},
	} {
		code, out, errs := runStats(t, "relabel", "-map", write(t, "map.tsv", c.mp), write(t, "raw.tsv", c.raw))
		if code != 1 || out != "" || !strings.Contains(errs, c.want) {
			t.Errorf("%s: exit %d, out %q, stderr %q, want exit 1 and %q", c.name, code, out, errs, c.want)
		}
	}
	code, _, errs := runStats(t, "relabel", write(t, "raw.tsv", raw))
	if code != 2 || !strings.Contains(errs, "-map is required") {
		t.Errorf("no -map: exit %d, stderr %q", code, errs)
	}
}

func TestCheckHandWorked(t *testing.T) {
	// Arms A and B must agree; C is in no set. The first line of a
	// (group, row) in file order is the reference, here B's round 2.
	reps := strings.Join([]string{
		"g\tr\tB\t2\t10\t20\tok",
		"g\tr\tA\t1\t10\t20\tok",
		"g\tr\tA\t2\t10.5\t20\tbad",
		"g\tr\tC\t1\t10\t20\tdifferent",
		"g\tr\tC\t2\t1.\t-3\t",
		"g\ts\tA\t1\t10\t20\tx\ty",
		"g\ts\tB\t1\t10\t20\tx\ty",
		"g\ts\tB\t2\t10\t20\tx y",
		"h\tr\tA\t1\t10\t20\t\t",
		"h\tr\tB\t1\t10\t20",
	}, "\n") + "\n"
	code, out, errs := runStats(t, "check", "-same", "A,B", write(t, "reps.tsv", reps))
	want := strings.Join([]string{
		"counter\tg\tr\tC\t2\tcycles\t-3",
		"counter\tg\tr\tC\t2\tinstructions\t1.",
		"empty-output\tg\tr\tC\t2",
		"empty-output\th\tr\tA\t1",
		"empty-output\th\tr\tB\t1",
		"output-diff\tg\tr\tA\t2\tB\t2",
		"output-diff\tg\ts\tB\t2\tA\t1",
		"output-diff\th\tr\tB\t1\tA\t1",
	}, "\n") + "\n"
	if code != 1 || out != want || errs != "stats check: 8 findings\n" {
		t.Errorf("exit %d, stderr %q, out:\n%s\nwant:\n%s", code, errs, out, want)
	}
	// A set of one arm compares its rounds; with no set, only counters
	// and empty outputs.
	code, out, _ = runStats(t, "check", "-same", "C", write(t, "reps.tsv", reps))
	if code != 1 || !strings.Contains(out, "output-diff\tg\tr\tC\t2\tC\t1\n") || strings.Contains(out, "\tA\t2\tB") {
		t.Errorf("-same C: exit %d, out:\n%s", code, out)
	}
	code, out, _ = runStats(t, "check", write(t, "reps.tsv", reps))
	if code != 1 || strings.Contains(out, "output-diff") || strings.Count(out, "\n") != 5 {
		t.Errorf("no set: exit %d, out:\n%s", code, out)
	}
}

func TestCheckClean(t *testing.T) {
	code, out, errs := runStats(t, "check", "-same", "A,B", write(t, "reps.tsv", "g\tr\tA\t1\t10\t20.25\tok\ng\tr\tB\t1\t11\t21\tok\n"))
	if code != 0 || out != "" || errs != "" {
		t.Errorf("exit %d, out %q, stderr %q", code, out, errs)
	}
}

func TestCheckRefused(t *testing.T) {
	for _, c := range []struct {
		name, reps string
		args       []string
		code       int
		want       string
	}{
		{"short line", "g\tr\tA\t1\t10\n", nil, 1, "reps.tsv:1: 5 columns, want at least 6"},
		{"empty round", "g\tr\tA\t\t10\t20\tok\n", nil, 1, "reps.tsv:1: empty round"},
		{"empty arm", "g\tr\t\t1\t10\t20\tok\n", nil, 1, "reps.tsv:1: empty arm"},
		{"repeated key", "g\tr\tA\t1\t10\t20\tok\ng\tr\tA\t1\t11\t21\tok\n", nil, 1, "reps.tsv:2: repeated group, row, arm and round g r A 1"},
		{"arm on no line", "g\tr\tA\t1\t10\t20\tok\n", []string{"-same", "A,Z"}, 1, `-same arm "Z" is on no line`},
		{"arm twice", "g\tr\tA\t1\t10\t20\tok\n", []string{"-same", "A", "-same", "B,A"}, 2, `arm "A" named twice`},
		{"empty arm in set", "g\tr\tA\t1\t10\t20\tok\n", []string{"-same", "A,"}, 2, `empty arm in "A,"`},
		{"no lines", "", nil, 1, "reps.tsv: no lines"},
	} {
		code, out, errs := runStats(t, append(append([]string{"check"}, c.args...), write(t, "reps.tsv", c.reps))...)
		if code != c.code || out != "" || !strings.Contains(errs, c.want) {
			t.Errorf("%s: exit %d, out %q, stderr %q, want exit %d and %q", c.name, code, out, errs, c.code, c.want)
		}
	}
}

func TestMD5HandWorked(t *testing.T) {
	h := func(c byte) string { return strings.Repeat(string(c), 32) }
	a := write(t, "a.log", "== before\n"+h('a')+"  /x/one file\n"+h('b')+"\t/x/two\n"+h('c')+"  /x/same\n")
	// The tab-separated /x/two in a.log is the same file as here.
	b := write(t, "b.log", "== after\n"+h('d')+" /x/one file\n"+h('9')+"  /x/two\n"+h('c')+" /x/same\n"+
		// Not md5sum lines: 31 digits, upper case, no separator, no file.
		strings.Repeat("e", 31)+"  /x/same\n"+strings.ToUpper(h('f'))+"  /x/same\n"+h('3')+"/x/same\n"+h('e')+"  \n")
	code, out, errs := runStats(t, "md5", a, b)
	want := "/x/one file\t" + h('a') + " " + h('d') + "\n/x/two\t" + h('9') + " " + h('b') + "\n"
	if code != 1 || out != want || errs != "stats md5: 2 findings\n" {
		t.Errorf("exit %d, stderr %q, out:\n%s\nwant:\n%s", code, errs, out, want)
	}
	// One log may hold both sides and every md5; they print sorted.
	var c, sorted []string
	for i, d := range "7fa0c3e9b1" {
		if i%5 == 0 {
			c = append(c, "== side "+strconv.Itoa(i))
		}
		c = append(c, h(byte(d))+"  f")
	}
	for _, d := range "01379abcef" {
		sorted = append(sorted, h(byte(d)))
	}
	code, out, _ = runStats(t, "md5", write(t, "c.log", strings.Join(c, "\n")+"\n"))
	if want := "f\t" + strings.Join(sorted, " ") + "\n"; code != 1 || out != want {
		t.Errorf("one log: exit %d, out %q, want %q", code, out, want)
	}
	// The same side twice is a pair with no change; so are two pairs
	// whose files differ between pairs (port A's sub-phase brackets).
	code, out, errs = runStats(t, "md5", a, a)
	if code != 0 || out != "" || errs != "" {
		t.Errorf("no change: exit %d, out %q, stderr %q", code, out, errs)
	}
	two := write(t, "two.log", "== b1\n"+h('1')+"  p\n== a1\n"+h('1')+"  p\n## other\n== b2\n"+h('2')+"  q\n"+h('3')+"  r\n== a2\n"+h('3')+"  r\n"+h('2')+"  q\n")
	code, out, errs = runStats(t, "md5", two)
	if code != 0 || out != "" || errs != "" {
		t.Errorf("two pairs: exit %d, out %q, stderr %q", code, out, errs)
	}
}

func TestMD5Refused(t *testing.T) {
	h := func(c byte) string { return strings.Repeat(string(c), 32) }
	good := write(t, "good.log", "== before\n"+h('a')+"  f\n")
	// The bracket checks, each exit 1 with no output.
	for _, c := range []struct{ name, log, want string }{
		{"before side only", "== before\n" + h('a') + "  f\n" + h('b') + "  g\n", `1 "== LABEL" sides; want before-and-after pairs`},
		{"three sides", "== b\n" + h('a') + "  f\n== a\n" + h('a') + "  f\n== b2\n" + h('a') + "  f\n", `3 "== LABEL" sides`},
		{"file only before", "== before\n" + h('a') + "  f\n" + h('b') + "  g\n== after\n" + h('a') + "  f\n",
			`side "after" (LOG:4) has no md5 of g, which side "before" (LOG:1) has`},
		{"file only after", "== before\n" + h('a') + "  f\n== after\n" + h('a') + "  f\n" + h('c') + "  h\n",
			`side "before" (LOG:1) has no md5 of h, which side "after" (LOG:3) has`},
		{"empty after side", "== before\n" + h('a') + "  f\n== after\n", `side "after" (LOG:3) has no md5 of f`},
		{"five files only before, all named in order", "== before\n" + h('a') + "  f\n" + h('1') + "  q5\n" + h('2') + "  q3\n" + h('3') + "  q1\n" + h('4') + "  q4\n" + h('5') + "  q2\n== after\n" + h('a') + "  f\n",
			`side "after" (LOG:8) has no md5 of q1, q2, q3, q4, q5, which`},
		{"second pair short", "== b1\n" + h('a') + "  f\n== a1\n" + h('a') + "  f\n== b2\n" + h('a') + "  f\n" + h('b') + "  g\n== a2\n" + h('a') + "  f\n",
			`side "a2" (LOG:8) has no md5 of g, which side "b2" (LOG:5) has`},
		{"md5 before any side", h('a') + "  f\n== before\n== after\n", `LOG:1: an md5sum line before any "== LABEL" side line`},
		{"escaped name", "== before\n\\" + h('a') + "  f\\\\x\n== after\n", `LOG:2: an escaped file name (md5sum's leading backslash) is not read`},
	} {
		p := write(t, "x.log", c.log)
		code, out, errs := runStats(t, "md5", p)
		if want := strings.ReplaceAll(c.want, "LOG", p); code != 1 || out != "" || !strings.Contains(errs, want) {
			t.Errorf("%s: exit %d, out %q, stderr %q, want %q", c.name, code, out, errs, want)
		}
	}
	// The sides pair across logs in argument order.
	code, _, errs := runStats(t, "md5", good, good, good)
	if code != 1 || !strings.Contains(errs, `3 "== LABEL" sides`) {
		t.Errorf("three logs of one side: exit %d, stderr %q", code, errs)
	}
	for _, l := range []string{"== after\ndone\n", "== after\n" + h('a') + "  \n" + h('b') + "\t\n"} {
		code, _, errs := runStats(t, "md5", good, write(t, "empty.log", l))
		if code != 1 || !strings.Contains(errs, "empty.log: no md5sum line") {
			t.Errorf("log without md5 (%q): exit %d, stderr %q", l, code, errs)
		}
	}
	// md5sum's error lines (GNU's and busybox's), alone or among
	// unchanged digests, so no file has two md5s: refused, naming the
	// line.
	for _, l := range []string{
		"== after\nmd5sum: /box/x.so: No such file or directory\n",
		"== after\n" + h('a') + "  f\nmd5sum: /box/x.so: No such file or directory\n",
		"== after\n" + h('a') + "  f\nmd5sum: can't open '/box/x.so': No such file or directory\n",
	} {
		code, out, errs := runStats(t, "md5", good, write(t, "err.log", l))
		n := strings.Count(l, "\n")
		if code != 1 || out != "" || !strings.Contains(errs, "err.log:"+strconv.Itoa(n)+": md5sum could not read a file: md5sum: ") {
			t.Errorf("md5sum error line (%q): exit %d, out %q, stderr %q", l, code, out, errs)
		}
	}
	code, _, errs = runStats(t, "md5")
	if code != 2 || !strings.Contains(errs, "want at least one log") {
		t.Errorf("no log: exit %d, stderr %q", code, errs)
	}
	code, _, errs = runStats(t, "md5", good, "testdata/no-such.log")
	if code != 1 || !strings.Contains(errs, "no-such.log") {
		t.Errorf("missing log: exit %d, stderr %q", code, errs)
	}
}
