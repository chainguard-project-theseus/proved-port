package main

import (
	"bytes"
	"errors"
	"os"
	"os/exec"
	"strings"
	"testing"
)

// The verdict lines of challoc's ci/codegen-fncmp-test.sh, on its
// fixtures (testdata/, copied from challoc's
// scripts/testdata/codegen-fncmp).
var fixtures = []struct {
	old, new, want string
	hideFilt       bool
}{
	{"old", "reloc", "exact 0, reloc-only 1, padding-only 0, erratum-stub 0, changed 0", false},
	{"old", "post", "exact 0, reloc-only 0, padding-only 0, erratum-stub 0, changed 1", false},
	{"pad-old", "pad-new", "exact 0, reloc-only 0, padding-only 1, erratum-stub 0, changed 0", false},
	{"erratum-old", "erratum-new", "exact 0, reloc-only 0, padding-only 0, erratum-stub 1, changed 0", false},
	{"erratum-new", "erratum-old", "exact 0, reloc-only 0, padding-only 0, erratum-stub 1, changed 0", false},
	{"erratum-old", "erratum-far", "exact 0, reloc-only 0, padding-only 0, erratum-stub 0, changed 1", false},
	{"pair-old", "pair-swap", "exact 0, reloc-only 0, padding-only 0, erratum-stub 0, changed 2", false},
	{"erratum-old", "erratum-branch", "exact 0, reloc-only 0, padding-only 0, erratum-stub 1, changed 0", false},
	{"erratum-old", "erratum-loose", "exact 0, reloc-only 0, padding-only 0, erratum-stub 0, changed 1", false},
	{"erratum-old", "erratum-out", "exact 0, reloc-only 0, padding-only 0, erratum-stub 0, changed 1", false},
	{"erratum-old", "erratum-misdirect", "exact 0, reloc-only 0, padding-only 0, erratum-stub 0, changed 1", false},
	{"hash-old", "hash-swap", "exact 0, reloc-only 0, padding-only 0, erratum-stub 0, changed 2", true},
	{"hash-old", "hash-swap", "exact 0, reloc-only 0, padding-only 0, erratum-stub 0, changed 2", false},
	{"impl-old", "impl-swap", "exact 0, reloc-only 0, padding-only 0, erratum-stub 0, changed 2", true},
	// Not in challoc's list: with c++filt on PATH only the new names
	// demangle, and the swapped bodies must still be changes (the
	// Python reports exact 2 here).
	{"impl-old", "impl-swap", "exact 0, reloc-only 0, padding-only 0, erratum-stub 0, changed 2", false},
	{"pair-old", "pair-rename", "exact 2, reloc-only 0, padding-only 0, erratum-stub 0, changed 0", false},
	{"pair-old", "pair-removed", "exact 1, reloc-only 0, padding-only 0, erratum-stub 0, changed 1", false},
}

// report runs the comparer on two fixtures, with c++filt on PATH or
// hidden. Without c++filt on PATH, a case that needs it is skipped, or
// fails when FNCMP_REQUIRE_CXXFILT=1 (the fncmp-test check sets it).
func report(t *testing.T, old, new string, hideFilt bool) string {
	t.Helper()
	if hideFilt {
		t.Setenv("PATH", t.TempDir())
	} else if _, err := exec.LookPath("c++filt"); err != nil {
		if os.Getenv("FNCMP_REQUIRE_CXXFILT") == "1" {
			t.Fatal("c++filt is not on PATH and FNCMP_REQUIRE_CXXFILT=1")
		}
		t.Skip("c++filt is not on PATH")
	}
	var out bytes.Buffer
	if err := run("testdata/"+old+".txt", "testdata/"+new+".txt", &out); err != nil {
		t.Fatal(err)
	}
	return out.String()
}

func caseName(old, new string, hideFilt bool) string {
	if hideFilt {
		return old + "_" + new + ".nofilt"
	}
	return old + "_" + new + ".filt"
}

func TestFixtures(t *testing.T) {
	for _, f := range fixtures {
		t.Run(caseName(f.old, f.new, f.hideFilt), func(t *testing.T) {
			got, _, _ := strings.Cut(report(t, f.old, f.new, f.hideFilt), "\n")
			_, got, _ = strings.Cut(got, ": ")
			if got != f.want {
				t.Errorf("got %q, want %q", got, f.want)
			}
		})
	}
}

// TestGolden compares each fixture case's whole report, with c++filt
// on PATH and hidden, with challoc 053c930's codegen-fncmp.py report
// on the same pair under Python 3.13 (testdata/golden/OLD_NEW.filt.txt
// and .nofilt.txt; the .filt reports came from GNU c++filt 2.47). One
// golden is this tool's own report: impl-old_impl-swap.filt.txt, where
// the mangled-key pass makes the swapped bodies changes and the Python
// reports two exact renames.
func TestGolden(t *testing.T) {
	seen := map[string]bool{}
	for _, f := range fixtures {
		for _, hide := range []bool{false, true} {
			name := caseName(f.old, f.new, hide)
			if seen[name] {
				continue
			}
			seen[name] = true
			t.Run(name, func(t *testing.T) {
				want, err := os.ReadFile("testdata/golden/" + name + ".txt")
				if err != nil {
					t.Fatal(err)
				}
				if got := report(t, f.old, f.new, hide); got != string(want) {
					t.Errorf("report differs from the Python's\ngot:\n%s\nwant:\n%s", got, want)
				}
			})
		}
	}
}

// A dump with no functions is a failed objdump or the wrong file: the
// comparer fails rather than report on it.
func TestNoFunctions(t *testing.T) {
	empty := t.TempDir() + "/empty.txt"
	if err := os.WriteFile(empty, []byte("not a dump\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	for _, p := range [][2]string{{empty, "testdata/old.txt"}, {"testdata/old.txt", empty}, {empty, empty}} {
		var out bytes.Buffer
		if err := run(p[0], p[1], &out); err == nil || out.Len() != 0 {
			t.Errorf("run(%s, %s) = %v with %d bytes of report, want an error and no report", p[0], p[1], err, out.Len())
		}
	}
}

func TestMask1(t *testing.T) {
	for in, want := range map[string]string{
		"ldr q0, [x9]":                "ldr q0, [x9, #I]",
		"ldr q0, [x9, #3952]":         "ldr q0, [x9, #I]",
		"ldr q0, [x9], #3952":         "ldr q0, [x9], #I",
		"ldr x0, [sp] !":              "ldr x0, [sp] !",
		"bl 401a20 <sym>":             "bl A <sym>",
		"mov $0x10,%eax":              "mov $0xI,%eax",
		"b.ne x401a20 <sym>":          "b.ne x401a20 <sym>",
		"cbz w0, 1f0 <sym>":           "cbz w0, A <sym>",
		"add x0, x0, #0xff8 // #4088": "add x0, x0, #IxI // #I", // the hex mask runs first, as in the Python
		"call éabc <sym>":             "call éabc <sym>",
		"call �abc <sym>":             "call �A <sym>",
	} {
		if got := mask1(in); got != want {
			t.Errorf("mask1(%q) = %q, want %q", in, got, want)
		}
	}
}

func TestDecodeReplace(t *testing.T) {
	for in, want := range map[string]string{
		"a\xe2\x82b":       "a�b",
		"\xff\xfe":         "��",
		"\xed\xa0\x80":     "���",
		"\xf0\x9f\x98":     "�",
		"\xf0\x9f\x98\x80": "\U0001F600",
	} {
		if got := decodeReplace([]byte(in)); got != want {
			t.Errorf("decodeReplace(%q) = %q, want %q", in, got, want)
		}
	}
}

// Expected values from CPython 3.13's difflib.
func TestUnifiedDiff(t *testing.T) {
	a := strings.Split("a b c d e f g h i j", " ")
	b := strings.Split("a b X d e f g h i j k", " ")
	got := strings.Join(unifiedDiff(a, b, "old", "new", 2), "\n")
	want := strings.Join([]string{
		"--- old", "+++ new",
		"@@ -1,5 +1,5 @@", " a", " b", "-c", "+X", " d", " e",
		"@@ -9,2 +9,3 @@", " i", " j", "+k",
	}, "\n")
	if got != want {
		t.Errorf("got\n%s\nwant\n%s", got, want)
	}
	if d := unifiedDiff(a, a, "old", "new", 2); len(d) != 0 {
		t.Errorf("equal sequences gave %q", d)
	}
	if got := strings.Join(unifiedDiff(nil, []string{"x"}, "old", "new", 2), "\n"); got != "--- old\n+++ new\n@@ -0,0 +1 @@\n+x" {
		t.Errorf("insert into empty gave %q", got)
	}
}

func TestRatio(t *testing.T) {
	if r := newSeqMatcher(strings.Split("abcd", ""), strings.Split("bcde", "")).ratio(); r != 0.75 {
		t.Errorf("ratio = %v, want 0.75", r)
	}
	if r := newSeqMatcher(nil, nil).ratio(); r != 1.0 {
		t.Errorf("empty ratio = %v, want 1", r)
	}
}

type failWriter struct{}

func (failWriter) Write([]byte) (int, error) { return 0, errors.New("no space left on device") }

// A failed write fails the run. The old/reloc report fits in bufio's
// buffer, so the error only surfaces at the final Flush: a deferred,
// unchecked Flush would pass a truncated report with exit 0.
func TestWriteError(t *testing.T) {
	t.Setenv("PATH", t.TempDir())
	if err := run("testdata/old.txt", "testdata/reloc.txt", failWriter{}); err == nil {
		t.Fatal("run returned nil on a failed write")
	}
}
