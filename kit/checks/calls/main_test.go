package main

import (
	"bytes"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// write puts content in a file under a test directory and returns its path.
func write(t *testing.T, name, content string) string {
	t.Helper()
	p := filepath.Join(t.TempDir(), name)
	if err := os.WriteFile(p, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
	return p
}

// runCalls runs the command over a rules file and a workflow, and returns
// its exit code, stdout and stderr.
func runCalls(t *testing.T, rules, workflow string) (int, string, string) {
	t.Helper()
	var out, errb bytes.Buffer
	code := run([]string{"-rules", write(t, "rules", rules), "-"}, strings.NewReader(workflow), &out, &errb)
	return code, out.String(), errb.String()
}

// oneCheck is a workflow with one check, kit-installing, whose steps are
// the given JSON.
func oneCheck(steps string) string {
	return `{"jobs":{"c":{"environment":{"packages":["theseus-proved-port~0.4.13","make"]},"steps":` + steps + `,"timeout":"60s","triggers":["TRIGGER_PRESUBMIT"]}}}`
}

// The challoc fixture is `chainctl checks explain --output=json` on
// challoc 7138852, with two values no rule reads made neutral (the floor
// checks' FLOOR_SA service account and an egress storage host, now
// example.com names): its verus checks run the kit's targets, its advisory
// codegen check runs its own script, and no check runs the counter it
// once carried.
func TestChalloc(t *testing.T) {
	data, err := os.ReadFile(filepath.Join("testdata", "challoc-7138852.json"))
	if err != nil {
		t.Fatal(err)
	}
	rules := strings.Join([]string{
		"# challoc's kit calls",
		"call verus verus-headers verus",
		"call verus-main verus-headers verus",
		"call verus-arm64 verus-headers verus",
		"call kani kani with QUANTUM",
		"  call kani-ignoring kani-regular with QUANTUM SHARD SHARDS",
		"",
		"call codegen codegen",
		"call consumers consumers with DOCKER_HOST TASK TESTS RUBY_TEST_EXCLUDE_EXTRA",
		"call bench-unit bench-unit verus",
		"forbid verus/count",
		"forbid verus/kani/run.sh",
		"forbid ci/codegen.sh",
		"forbid ci/heap-noalias.sh",
		"forbid tests/genmc",
	}, "\n")
	code, out, errs := runCalls(t, rules, string(data))
	want := strings.Join([]string{
		"ok call verus verus-headers verus",
		"ok call verus-main verus-headers verus",
		"ok call verus-arm64 verus-headers verus",
		// challoc's kani runs only when its when.changed paths change.
		"FAIL call kani kani with QUANTUM: the check runs only when its when.changed paths change",
		// A Kani split cannot be held to cover every shard.
		"FAIL call kani-ignoring kani-regular with QUANTUM SHARD SHARDS: " +
			"no step runs make kani-regular: step 1 (harnesses, regular format) runs make with a variable that sets KANI_SHARD, which the call line does not allow (`with KANI_SHARD`)",
		`FAIL call codegen codegen: the check is advisory; its environment does not install theseus-proved-port; no step runs make codegen: step 1 runs "sh ci/codegen.sh", not make or sh -c`,
		`FAIL call consumers consumers with DOCKER_HOST TASK TESTS RUBY_TEST_EXCLUDE_EXTRA: the check runs only on request; the check runs on neither presubmit nor postsubmit; its environment starts services, which run beside the steps and could change what make runs; its environment does not install theseus-proved-port; its foreach vars: sets RUBY_TEST_EXCLUDE_EXTRA to "test/ruby/ test/rubygems/"` + notSafe + `; no step runs make consumers: step 1 runs "bash ci/extended.sh", not make or sh -c`,
		`FAIL call bench-unit bench-unit verus: its environment does not install theseus-proved-port; no step runs make verus: step 1 runs make bench-unit`,
		"ok forbid verus/count",
		"ok forbid verus/kani/run.sh",
		"FAIL forbid ci/codegen.sh: check codegen step 1 names ci/codegen.sh; check codegen-arm64 step 1 names ci/codegen.sh",
		"FAIL forbid ci/heap-noalias.sh: check heap-noalias step 1 names ci/heap-noalias.sh",
		"FAIL forbid tests/genmc: check genmc step 1 names tests/genmc; check genmc step 2 names tests/genmc; check genmc step 3 names tests/genmc",
		"",
	}, "\n")
	if code != 1 || out != want || errs != "" {
		t.Errorf("got code %d, stdout:\n%s\nstderr: %s\nwant code 1, stdout:\n%s", code, out, errs, want)
	}
}

// The templates fixture is `chainctl checks explain --output=json` on the
// kit's template check files filled with a sample port's values
// (kit/kit-calls-test.sh writes it with --update). The rules template
// must hold over it, and must have a call line for each kit target each
// check's steps name: a check whose step holds the word make and the
// name of one of proved-port.mk's targets needs a call line naming that
// check and target. That list comes from the steps, not the rules file,
// so a call line the template loses, or a template check that gains a
// kit target with no call line, fails here. KIT_CALLS_TEMPLATES names
// another file to read in its place: kit/kit-calls-test.sh runs this test
// over what the real chainctl prints for the templates as they are, so a
// fixture that lags them does not hide a missing call line.
func TestTemplateCalls(t *testing.T) {
	fixture := filepath.Join("testdata", "templates.json")
	if f := os.Getenv("KIT_CALLS_TEMPLATES"); f != "" {
		fixture = f
	}
	data, err := os.ReadFile(fixture)
	if err != nil {
		t.Fatal(err)
	}
	rulesPath := filepath.Join("..", "..", "templates", "ci", "kit-calls.rules")
	rules, err := readRules(rulesPath)
	if err != nil {
		t.Fatal(err)
	}
	mk, err := os.ReadFile(filepath.Join("..", "..", "make", "proved-port.mk"))
	if err != nil {
		t.Fatal(err)
	}
	m := regexp.MustCompile(`(?m)^proved_port_phony = (.*)$`).FindSubmatch(mk)
	if m == nil {
		t.Fatal("proved-port.mk has no proved_port_phony line")
	}
	kit := map[string]bool{"verify": true}
	for _, name := range strings.Fields(string(m[1])) {
		kit[name] = true
	}
	if !kit["kit-calls"] || !kit["verus"] || len(kit) < 20 {
		t.Fatalf("read %d kit targets from proved_port_phony, want at least 20 with verus and kit-calls", len(kit))
	}
	w, err := readWorkflow(data)
	if err != nil {
		t.Fatal(err)
	}
	called := map[string]bool{}
	for _, r := range rules {
		if r.kind == "call" {
			for _, target := range r.targets {
				called[r.check+" "+target] = true
			}
		}
	}
	n := 0
	for _, name := range w.names {
		for k, s := range w.jobs[name].steps {
			words := strings.Fields(strings.Join(s.Cmd, " "))
			hasMake := false
			for _, x := range words {
				hasMake = hasMake || x == "make"
			}
			if !hasMake {
				continue
			}
			for _, x := range words {
				if !kit[x] {
					continue
				}
				n++
				if !called[name+" "+x] {
					t.Errorf("check %s step %d runs make %s, and kit-calls.rules has no call line for it (call %s %s)", name, k+1, x, name, x)
				}
			}
		}
	}
	// verus and verus-main: verus-headers and verus; kani and kani-main:
	// kani; source-rules: lint-unsafe and kit-calls.
	if n < 8 {
		t.Errorf("found %d kit targets in the template's steps, want at least 8", n)
	}
	var out, errb bytes.Buffer
	code := run([]string{"-rules", rulesPath, fixture}, nil, &out, &errb)
	if code != 0 || errb.Len() != 0 || strings.Contains(out.String(), "FAIL") {
		t.Errorf("the rules template over the templates: got code %d, stdout:\n%s\nstderr: %s\nwant 0, every rule ok", code, out.String(), errb.String())
	}
}

func TestCallPasses(t *testing.T) {
	for _, steps := range []string{
		`[{"cmd":["make","t"]}]`,
		`[{"cmd":["make","a","t","b"]}]`,
		`[{"cmd":["make","-s","-k","-j","4","t","X=1"]}]`,
		`[{"cmd":["make","-j8","--jobs=2","--jobs","3","--silent","--quiet","--keep-going","t"]}]`,
		`[{"cmd":["make","-O","-Oline","--output-sync","--output-sync=target","-Orecurse","-Onone","t"]}]`,
		`[{"cmd":["make","-j","t"]}]`,
		`[{"cmd":["make","FOO:=x","t","BAR+=y","Q::=z","R:::=w","S?=v"]}]`,
		`[{"cmd":["make","A=azAZ09_.,/:+@%=-","t"]}]`,
		`[{"cmd":["make","u"]},{"name":"n","cmd":["make","t"]},{"cmd":["echo","hi"]}]`,
		`[{"cmd":["sh","-c","make t"]}]`,
		`[{"cmd":["bash","-c","make t","name","arg"]}]`,
		`[{"cmd":["sh","-c","A=1 make t B=x/y.z"]}]`,
		// A script value may be one quoted expansion of a variable the
		// check sets (Q and X here).
		`[{"cmd":["sh","-c","A=\"$Q\" make t B=\"${X}\" R=\"${Q:?no Q}\" S=\"${X?}\" U=\"${Q:-}\" V=\"${X-}\""]}]`,
		// The template's verus step: VERUS_THREADS as the guest's CPUs,
		// VERUS_BASE as the base Checks sets.
		`[{"cmd":["sh","-c","\n  make t VERUS_THREADS=\"${CG_CPUS:-}\" VERUS_BASE=\"${CHAINGUARD_BASE_REVISION:?no base revision to compare verus/FLOOR with}\"\n"]}]`,
		`[{"cmd":["sh","-c","VERUS_BASE=\"$CHAINGUARD_BASE_REVISION\" make t VERUS_BASE=\"${CHAINGUARD_BASE_REVISION}\" VERUS_THREADS=\"$CG_CPUS\""]}]`,
		`[{"cmd":["make","VERUS_THREADS=8","t","VERUS_THREADS="]}]`,
		`[{"cmd":["sh","-c","make t VERUS_THREADS=12"]}]`,
	} {
		job := `{"jobs":{"c":{"environment":{"packages":["theseus-proved-port"]},"variables":{"Q":"1","X":"a"},"steps":` + steps + `,"triggers":["TRIGGER_PRESUBMIT"]}}}`
		code, out, errs := runCalls(t, "call c t with A B BAR FOO Q R S U V X\n", job)
		if code != 0 || out != "ok call c t with A B BAR FOO Q R S U V X\n" || errs != "" {
			t.Errorf("steps %s: got code %d, stdout %q, stderr %q; want 0, ok", steps, code, out, errs)
		}
	}
}

// Each of these runs no make t that is read with certainty.
func TestCallStepRefused(t *testing.T) {
	for _, c := range []struct{ cmd, why string }{
		{`["/usr/bin/make","t"]`, `runs "/usr/bin/make t", not make or sh -c`},
		{`["env","make","t"]`, `runs "env make t", not make or sh -c`},
		{`["sh","ci/t.sh"]`, `runs "sh ci/t.sh", not make or sh -c`},
		{`["sh","-ec","make t"]`, `runs "sh -ec make t", not make or sh -c`},
		{`["sh","-c"]`, `runs "sh -c", not make or sh -c`},
		{`["make"]`, `runs make with no goal`},
		{`["make","VERUS_THREADS=1"]`, `runs make with no goal`},
		{`["make","-n","t"]`, `runs make with an option this tool does not read as running every goal: -n`},
		{`["make","-q","t"]`, `runs make with an option this tool does not read as running every goal: -q`},
		{`["make","-t","t"]`, `runs make with an option this tool does not read as running every goal: -t`},
		{`["make","-i","t"]`, `runs make with an option this tool does not read as running every goal: -i`},
		{`["make","-C","d","t"]`, `runs make with an option this tool does not read as running every goal: -C`},
		{`["make","-f","m","t"]`, `runs make with an option this tool does not read as running every goal: -f`},
		{`["make","-ks","t"]`, `runs make with an option this tool does not read as running every goal: -ks`},
		{`["make","--","t"]`, `runs make with an option this tool does not read as running every goal: --`},
		{`["make","-jx","t"]`, `runs make with an option this tool does not read as running every goal: -jx`},
		{`["make","--jobs=","t"]`, `runs make with an option this tool does not read as running every goal: --jobs=`},
		{`["make","-Oall","t"]`, `runs make with an option this tool does not read as running every goal: -Oall`},
		{`["make","--output-sync=x","t"]`, `runs make with an option this tool does not read as running every goal: --output-sync=x`},
		{`["make","--dry-run","t"]`, `runs make with an option this tool does not read as running every goal: --dry-run`},
		{`["make","MAKEFLAGS=n","t"]`, `runs make with a variable that sets MAKEFLAGS` + reads},
		{`["make","SHELL:=/bin/true","t"]`, `runs make with a variable that sets SHELL` + reads},
		{`["make",".SHELLFLAGS+=x","t"]`, `runs make with a variable that sets .SHELLFLAGS` + reads},
		{`["make","MAKEOVERRIDES=","t"]`, `runs make with a variable that sets MAKEOVERRIDES` + reads},
		// proved-port.mk's verify runs $(MAKE) for each target.
		{`["make","t","MAKE=true"]`, `runs make with a variable that sets MAKE` + reads},
		{`["make","t","CARGO_TARGET_DIR=x"]`, `runs make with a variable that sets CARGO_TARGET_DIR` + reads},
		{`["sh","-c","RUSTC_WRAPPER=true make t"]`, `has a script that sets RUSTC_WRAPPER` + reads},
		{`["make","t",".DEFAULT_GOAL=x"]`, `runs make with a variable that sets .DEFAULT_GOAL` + reads},
		// make trims blanks around a name and expands it.
		{`["make","SHELL =/bin/true","t"]`, `runs make with a variable that is not read: "SHELL =/bin/true"`},
		{`["make","MAKEFLAGS\t=n","t"]`, `runs make with a variable that is not read: "MAKEFLAGS\t=n"`},
		{`["make","SH$()ELL=/bin/true","t"]`, `runs make with a variable that is not read: "SH$()ELL=/bin/true"`},
		{`["make","=x","t"]`, `runs make with a variable that is not read: "=x"`},
		{`["make","A:+=x","t"]`, `runs make with a variable that is not read: "A:+=x"`},
		// Only the variables the call line allows, VERUS_THREADS and
		// VERUS_BASE as the base Checks sets.
		{`["make","Y=1","t"]`, "runs make with a variable that sets Y, which the call line does not allow (`with Y`)"},
		{`["make","PROVED_PORT_HOME=verus/kit","t"]`, "runs make with a variable that sets PROVED_PORT_HOME, which the call line does not allow (`with PROVED_PORT_HOME`)"},
		{`["make","VERUS_COUNT=true","t"]`, "runs make with a variable that sets VERUS_COUNT, which the call line does not allow (`with VERUS_COUNT`)"},
		{`["make","VERUS_BASE=none","t"]`, "runs make with a variable that " + base},
		{`["make","VERUS_BASE=${CHAINGUARD_BASE_REVISION}","t"]`, "runs make with a variable that " + base},
		{`["sh","-c","make t VERUS_BASE=none"]`, "runs make with a variable that " + base},
		{`["sh","-c","make t VERUS_BASE=\"none\""]`, "runs make with a variable that " + base},
		{`["sh","-c","make t VERUS_BASE='${CHAINGUARD_BASE_REVISION}'"]`, "runs make with a variable that " + base},
		{`["sh","-c","make t VERUS_BASE+=\"${CHAINGUARD_BASE_REVISION}\""]`, "runs make with a variable that " + base},
		{`["sh","-c","make t VERUS_BASE=\"${CHAINGUARD_BASE_REVISION:-none}\""]`, "runs make with a variable that " + base},
		{`["sh","-c","make t VERUS_BASE=\"${CHAINGUARD_BASE_REVISION} none\""]`, "runs make with a variable that " + base},
		{`["sh","-c","make t VERUS_BASE=\"$X\""]`, "runs make with a variable that " + base},
		{`["sh","-c","make t VERUS_BASE='${CHAINGUARD_BASE_REVISION:?'\"$X\"'}'"]`, "runs make with a variable that " + base},
		{`["sh","-c","make t VERUS_BASE=\"${CHAINGUARD_BASE_REVISION:?\\$}\""]`, "has a script that is not read: a `${..}` holding a quote, backslash, `$`, `{`, backquote or newline, or empty"},
		// "$@" and "${@}" are one word per positional parameter, quoted
		// or not: `sh -c 'make t V="$@"' x 1 -n` runs `make t V=1 -n`.
		{`["sh","-c","make t VERUS_THREADS=\"$@\"","checks","1","-n"]`, "has a script that is not read: a positional or special parameter (`$@`)"},
		{`["sh","-c","make t VERUS_THREADS=\"${@}\"","checks","1","-n"]`, "has a script that is not read: a `${@}` that is not a named parameter, alone or with :-, -, :=, =, :?, ?, :+ or +"},
		{`["sh","-c","make t VERUS_THREADS=\"${a[@]}\""]`, "has a script that is not read: a `${a[@]}` that is not a named parameter, alone or with :-, -, :=, =, :?, ?, :+ or +"},
		{`["sh","-c","make t VERUS_THREADS=\"${!a@}\""]`, "has a script that is not read: a `${!a@}` that is not a named parameter, alone or with :-, -, :=, =, :?, ?, :+ or +"},
		{`["sh","-c","make t VERUS_THREADS=\"${#a}\""]`, "has a script that is not read: a `${#a}` that is not a named parameter, alone or with :-, -, :=, =, :?, ?, :+ or +"},
		{`["sh","-c","make t VERUS_THREADS=\"${a%b}\""]`, "has a script that is not read: a `${a%b}` that is not a named parameter, alone or with :-, -, :=, =, :?, ?, :+ or +"},
		{`["sh","-c","make t VERUS_THREADS=\"$1\""]`, "has a script that is not read: a positional or special parameter (`$1`)"},
		{`["sh","-c","make t VERUS_THREADS=\"$*\""]`, "has a script that is not read: a positional or special parameter (`$*`)"},
		// make expands a `$` in a value: `$(eval override MAKEFLAGS:=n)`
		// turns on dry-run mode.
		{`["make","kani","VERUS_THREADS:=$(eval override MAKEFLAGS:=n)","t"]`, "runs make with a variable that " + threads},
		{`["sh","-c","make t VERUS_THREADS:='$(eval override MAKEFLAGS:=n)'"]`, "runs make with a variable that " + threads},
		{`["sh","-c","make t VERUS_THREADS:=\\$X"]`, "runs make with a variable that " + threads},
		{`["sh","-c","make t VERUS_THREADS:=\"\\$X\""]`, "runs make with a variable that " + threads},
		{`["sh","-c","VERUS_THREADS='$(eval x)' make t"]`, "has a script that " + threads},
		{`["sh","-c","VERUS_BASE=none make t"]`, "has a script that " + base},
		// One name split across a quote is no one expansion:
		// "$C"HAINGUARD_BASE_REVISION expands C.
		{`["sh","-c","make t VERUS_BASE=\"$C\"HAINGUARD_BASE_REVISION"]`, "runs make with a variable that " + base},
		{`["sh","-c","make t VERUS_BASE=$C\"HAINGUARD_BASE_REVISION\""]`, "runs make with a variable that is not read: \"VERUS_BASE=$CHAINGUARD_BASE_REVISION\""},
		{`["sh","-c","VERUS_BASE=\"$C\"HAINGUARD_BASE_REVISION make t"]`, "has a script that " + base},
		{`["sh","-c","VERUS_BASE=$CHAINGUARD_BASE'_REVISION' make t"]`, "has a script that " + base},
		{`["sh","-c","make t VERUS_BASE=\"$CHAINGUARD_BASE\"_REVISION"]`, "runs make with a variable that " + base},
		{`["sh","-c","make t VERUS_BASE=\"${CHAINGUARD_BASE_REVISION}\"x"]`, "runs make with a variable that " + base},
		// VERUS_THREADS is pasted unquoted into the cargo verus line.
		{`["make","VERUS_THREADS=1 >/dev/null; cp verus/planted.json target/verus-stats.json #","t"]`, "runs make with a variable that " + threads},
		{`["make","VERUS_THREADS=8 --no-verify","t"]`, "runs make with a variable that " + threads},
		{`["sh","-c","make t VERUS_THREADS='1 ; true #'"]`, "runs make with a variable that " + threads},
		{`["sh","-c","make t VERUS_THREADS=\"$Q\""]`, "runs make with a variable that " + threads},
		{`["sh","-c","VERUS_THREADS='8 x' make t"]`, "has a script that " + threads},
		// `!=` runs its value as a shell command.
		{`["make","t","VERUS_THREADS!=printf 't:;@true' > GNUmakefile"]`, `runs make with a variable that is not read: "VERUS_THREADS!=printf 't:;@true' > GNUmakefile"`},
		{`["sh","-c","make t X!=true"]`, `runs make with a variable that is not read: "X!=true"`},
		// Other values: plain, or one quoted expansion of a variable the
		// check sets.
		{`["make","X=a b","t"]`, `runs make with a variable that sets X to "a b"` + notSafe},
		{`["make","X=a;b","t"]`, `runs make with a variable that sets X to "a;b"` + notSafe},
		{`["sh","-c","make t X='a b'"]`, `runs make with a variable that sets X to "'a b'"` + notSafe},
		{`["sh","-c","make t X=\"a\""]`, `runs make with a variable that sets X to "\"a\""` + notSafe},
		{`["sh","-c","make t X=\"$HOME\""]`, "runs make with a variable that sets X to HOME, which the check's variables do not set"},
		{`["sh","-c","make t X=\"${Y:-1}\""]`, `runs make with a variable that sets X to "\"${Y:-1}\""` + notSafe},
		{`["sh","-c","make t X=\"$Y$Y\""]`, `runs make with a variable that sets X to "\"$Y$Y\""` + notSafe},
		{`["sh","-c","make t X=\"a\\x\""]`, `runs make with a variable that sets X to "\"a\\x\""` + notSafe},
		{`["sh","-c","X=a\\ b make t"]`, `has a script that sets X to "a\\ b"` + notSafe},
		{`["sh","-c","Q=1 make t"]`, "has a script that sets Q, which the call line does not allow (`with Q`)"},
		{`["make","t*"]`, `runs make with a goal that is not literal: "t*"`},
		{`["make",""]`, `runs make with a goal that is not literal: ""`},
		{`["sh","-c","cd d && make t"]`, `has a script that is not read: an operator ` + "`&`"},
		{`["sh","-c","make t || true"]`, `has a script that is not read: an operator ` + "`|`"},
		{`["sh","-c","make t; true"]`, `has a script that is not read: an operator ` + "`;`"},
		{`["sh","-c","make t > log"]`, `has a script that is not read: an operator ` + "`>`"},
		{`["sh","-c","(make t)"]`, `has a script that is not read: an operator ` + "`(`"},
		{`["sh","-c","make t # x"]`, `has a script that is not read: a comment`},
		{`["sh","-c","make t\ntrue"]`, `has a script that is not read: more than one command (a newline between words)`},
		{`["sh","-c","make t \\\n x"]`, `has a script that is not read: a line continuation`},
		{`["sh","-c","make t X=\"a\\\nb\""]`, `has a script that is not read: a line continuation`},
		{`["sh","-c","make $(echo t)"]`, "has a script that is not read: a command substitution or arithmetic (`$(`)"},
		{`["sh","-c","make ` + "`echo t`" + `"]`, `has a script that is not read: a command substitution (backquote)`},
		{`["sh","-c","make \"` + "`echo t`" + `\""]`, `has a script that is not read: a command substitution (backquote)`},
		{`["sh","-c","make t X='a"]`, `has a script that is not read: an unterminated single quote`},
		{`["sh","-c","make t X=\"a"]`, `has a script that is not read: an unterminated double quote`},
		{`["sh","-c","make t X=\"a\\"]`, `has a script that is not read: an unterminated double quote`},
		{`["sh","-c","make t \\"]`, `has a script that is not read: a backslash at the end of the script`},
		{`["sh","-c","make t $"]`, "has a script that is not read: a `$` at the end of the script"},
		{`["sh","-c","make t $'x'"]`, "has a script that is not read: a `$'..'` string"},
		{`["sh","-c","make t $%"]`, "has a script that is not read: a `$` before '%'"},
		{`["sh","-c","make t ${X"]`, "has a script that is not read: an unterminated `${`"},
		{`["sh","-c","make t ${}"]`, "has a script that is not read: a `${..}` holding a quote, backslash, `$`, `{`, backquote or newline, or empty"},
		{`["sh","-c","make t ${X:-\"}\"}"]`, "has a script that is not read: a `${..}` holding a quote, backslash, `$`, `{`, backquote or newline, or empty"},
		{`["sh","-c","make t ${X:-${Y}}"]`, "has a script that is not read: a `${..}` holding a quote, backslash, `$`, `{`, backquote or newline, or empty"},
		{`["sh","-c",""]`, `has a script that is not read: no command`},
		{`["sh","-c"," \n "]`, `has a script that is not read: no command`},
		{`["sh","-c","VERUS_THREADS=1"]`, `has a script that runs no command`},
		{`["sh","-c","MAKEFLAGS=n make t"]`, `has a script that sets MAKEFLAGS` + reads},
		{`["sh","-c","PATH=. make t"]`, `has a script that sets PATH` + reads},
		{`["sh","-c","SHELLOPTS=noexec make t"]`, `has a script that sets SHELLOPTS` + reads},
		{`["sh","-c","CHAINGUARD_BASE_REVISION=none make t"]`, `has a script that sets CHAINGUARD_BASE_REVISION` + reads},
		{`["sh","-c","BASH_FUNC_make%%=x make t"]`, `has a script that runs "BASH_FUNC_make%%=x", not make`},
		{`["sh","-c","1A=x make t"]`, `has a script that runs "1A=x", not make`},
		{`["sh","-c","A-B=x make t"]`, `has a script that runs "A-B=x", not make`},
		{`["sh","-c","=x make t"]`, `has a script that runs "=x", not make`},
		{`["sh","-c","make t X=\"$(x)\""]`, "has a script that is not read: a command substitution or arithmetic (`$(`)"},
		{`["sh","-c","\"make\" t"]`, `has a script that runs "make", not make`},
		{`["sh","-c","m\\ake t"]`, `has a script that runs "make", not make`},
		{`["sh","-c","\"X\"=1 make t"]`, `has a script that runs "X=1", not make`},
		{`["sh","-c","exec make t"]`, `has a script that runs "exec", not make`},
		{`["sh","-c","! make t"]`, `has a script that runs "!", not make`},
		{`["sh","-c","if make t"]`, `has a script that runs "if", not make`},
		{`["sh","-c","make \"t\""]`, `runs make with a goal that is not literal: "t"`},
		{`["sh","-c","make \\t"]`, `runs make with a goal that is not literal: "t"`},
		{`["sh","-c","make $T"]`, `runs make with a goal that is not literal: "$T"`},
		{`["sh","-c","make ~t"]`, `runs make with a goal that is not literal: "~t"`},
		{`["sh","-c","make {t,u}"]`, `runs make with a goal that is not literal: "{t,u}"`},
		{`["sh","-c","make t?"]`, `runs make with a goal that is not literal: "t?"`},
		{`["sh","-c","make [t]"]`, `runs make with a goal that is not literal: "[t]"`},
		{`["sh","-c","make \"-n\" t"]`, `runs make with an option that is not literal: "-n"`},
		{`["sh","-c","make -j \"4\" t"]`, `runs make with a goal that is not literal: "4"`},
		{`["sh","-c","make A=$X t"]`, `runs make with a variable that is not read: "A=$X"`},
		{`["sh","-c","make A=${X} t"]`, `runs make with a variable that is not read: "A=${X}"`},
		{`["sh","-c","make \"A=1\" t"]`, `runs make with a variable that is not read: "A=1"`},
		{`["sh","-c","make {A=x,-n} t"]`, `runs make with a variable that is not read: "{A=x,-n}"`},
		{`["sh","-c","make \\A=1 t"]`, `runs make with a variable that is not read: "A=1"`},
		{`["sh","-c","make MAKEFILES=x t"]`, `runs make with a variable that sets MAKEFILES` + reads},
	} {
		steps := `[{"cmd":` + c.cmd + `}]`
		code, out, errs := runCalls(t, "call c t with X\n", oneCheck(steps))
		want := "FAIL call c t with X: no step runs make t: step 1 " + c.why + "\n"
		if code != 1 || out != want || errs != "" {
			t.Errorf("cmd %s: got code %d, stdout %q, stderr %q; want 1, %q", c.cmd, code, out, errs, want)
		}
	}
}

// Each of these checks fails a call rule whatever its steps run.
func TestCallCheckRefused(t *testing.T) {
	steps := `"steps":[{"cmd":["make","t"]}],"trigger":"TRIGGER_PRESUBMIT"`
	bare := `"steps":[{"cmd":["make","t"]}]`
	kit := `"environment":{"packages":["theseus-proved-port"]}`
	for _, c := range []struct{ job, why string }{
		{`{` + kit + `,` + steps + `,"advisory":true}`, "the check is advisory"},
		{`{` + kit + `,` + steps + `,"when":{"requested":true}}`, "the check runs only on request"},
		{`{` + kit + `,` + steps + `,"when":{"changed":["never/**"]}}`, "the check runs only when its when.changed paths change"},
		{`{` + kit + `,` + steps + `,"when":{"requested":true,"changed":["src/**"]}}`, "the check runs only on request; the check runs only when its when.changed paths change"},
		{`{"environment":{"packages":["make"]},` + steps + `}`, "its environment does not install theseus-proved-port"},
		{`{"environment":{"packages":["theseus-proved-port-x","theseus-proved"]},` + steps + `}`, "its environment does not install theseus-proved-port"},
		{`{` + steps + `}`, "its environment does not install theseus-proved-port"},
		{`{` + kit + `,` + bare + `}`, "the check runs on neither presubmit nor postsubmit"},
		{`{` + kit + `,` + bare + `,"trigger":"TRIGGER_CRON","triggers":["TRIGGER_CRON"]}`, "the check runs on neither presubmit nor postsubmit"},
		{`{` + kit + `,` + bare + `,"when":{"requested":true}}`, "the check runs only on request; the check runs on neither presubmit nor postsubmit"},
		{`{` + kit + `,` + bare + `,"trigger":["TRIGGER_CRON"]}`, "its trigger field: json: cannot unmarshal array into Go value of type string"},
		{`{` + kit + `,` + bare + `,"triggers":"TRIGGER_PRESUBMIT"}`, "its triggers field: json: cannot unmarshal string into Go value of type []string"},
		{`{"environment":{"packages":["theseus-proved-port"],"variables":{"MAKEFLAGS":"n"}},` + steps + `}`, "its environment's variables: sets MAKEFLAGS" + reads},
		{`{` + kit + `,` + steps + `,"variables":{"A":"1","BASH_ENV":"x","PATH":"."}}`, "its variables: sets A, which the call line does not allow (`with A`); its variables: sets BASH_ENV" + reads + "; its variables: sets PATH" + reads},
		{`{` + kit + `,` + steps + `,"foreach":{"entries":[{"name":"a","vars":{"Q":"1"}},{"name":"b","vars":{"GNUMAKEFLAGS":"n","Q":"2","ENV":"x"}}]}}`, "its foreach vars: sets Q, which the call line does not allow (`with Q`); its foreach vars: sets ENV" + reads + "; its foreach vars: sets GNUMAKEFLAGS" + reads},
		{`{` + kit + `,` + steps + `,"variables":{"VERUS_BASE":"none","VERUS_THREADS":"2"}}`, "its variables: " + base},
		{`{` + kit + `,` + steps + `,"variables":{"VERUS_BASE":"$$x"}}`, "its variables: " + base},
		{`{` + kit + `,` + steps + `,"variables":{"VERUS_BASE":"x"}}`, "its variables: " + base},
		{`{"environment":{"packages":["theseus-proved-port"],"variables":{"VERUS_THREADS":"$(eval override MAKEFLAGS:=n)"}},` + steps + `}`, "its environment's variables: " + threads},
		{`{` + kit + `,` + steps + `,"variables":{"PROVED_PORT_HOME":"x"}}`, "its variables: sets PROVED_PORT_HOME, which the call line does not allow (`with PROVED_PORT_HOME`)"},
		// kani.sh runs no harness at shard 0 and passes.
		{`{` + kit + `,` + steps + `,"variables":{"KANI_SHARD":"0","KANI_SHARDS":"1"}}`, "its variables: sets KANI_SHARD, which the call line does not allow (`with KANI_SHARD`); its variables: sets KANI_SHARDS, which the call line does not allow (`with KANI_SHARDS`)"},
		{`{` + kit + `,` + steps + `,"variables":{"CI_SKIP_UPDATE":"1"}}`, "its variables: sets CI_SKIP_UPDATE, which the call line does not allow (`with CI_SKIP_UPDATE`)"},
		// git reads GIT_DIR when the counter reads the base revision.
		{`{` + kit + `,` + steps + `,"variables":{"GIT_DIR":"ci/base.git"}}`, "its variables: sets GIT_DIR" + reads},
		{`{"environment":{"packages":["theseus-proved-port"],"services":[{"name":"s","cmd":["sh","ci/prep.sh"],"workdir":"."}]},` + steps + `}`, "its environment starts services, which run beside the steps and could change what make runs"},
		{`{` + kit + `,` + steps + `,"needs":["x"],"allow":1}`, `the check has a field this tool does not read: "allow"; the check has a field this tool does not read: "needs"`},
		{`{"environment":{"packages":["theseus-proved-port"],"repositories":[]},` + steps + `}`, `its environment field: json: unknown field "repositories"`},
		{`{` + kit + `,` + steps + `,"when":{"branches":["main"]}}`, `its when field: json: unknown field "branches"`},
		{`{` + kit + `,` + steps + `,"foreach":{"entries":[{"name":"a","steps":[]}]}}`, `its foreach field: json: unknown field "steps"`},
		{`{` + kit + `,` + steps + `,"advisory":"yes"}`, `its advisory field: json: cannot unmarshal string into Go value of type bool`},
	} {
		code, out, errs := runCalls(t, "call c t\n", `{"jobs":{"c":`+c.job+`}}`)
		want := "FAIL call c t: " + c.why + "\n"
		if code != 1 || out != want || errs != "" {
			t.Errorf("job %s: got code %d, stdout %q, stderr %q; want 1, %q", c.job, code, out, errs, want)
		}
	}
	code, out, _ := runCalls(t, "call d t\n", oneCheck(`[{"cmd":["make","t"]}]`))
	if want := "FAIL call d t: no check d\n"; code != 1 || out != want {
		t.Errorf("a missing check: got %d %q, want 1 %q", code, out, want)
	}
	// A pinned kit package and the fields a check may have pass; an
	// advisory false and a when that is not requested gate.
	job := `{"environment":{"packages":["theseus-proved-port=0.4.13-r0"],"variables":{"A":"1"}},` + bare + `,"advisory":false,"when":{"requested":false},"foreach":{"entries":[{"name":"a","vars":{"Q":"1"},"resources":"large","network":{"egress":["x"]},"timeout":"9s"}]},"variables":{"B":"2"},"egress":[],"identity":"x","network":"NETWORK_MODE_EXTERNAL","resources":"large","schedule":"1 * * * *","timeout":"1s","trigger":"TRIGGER_CRON","triggers":["TRIGGER_CRON","TRIGGER_POSTSUBMIT"]}`
	code, out, errs := runCalls(t, "call c t with A B Q\n", `{"jobs":{"c":`+job+`}}`)
	if code != 0 || out != "ok call c t with A B Q\n" || errs != "" {
		t.Errorf("a check with every field read: got %d %q %q, want 0, ok", code, out, errs)
	}
	job = `{` + kit + `,` + bare + `,"trigger":"TRIGGER_POSTSUBMIT"}`
	code, out, errs = runCalls(t, "call c t\n", `{"jobs":{"c":`+job+`}}`)
	if code != 0 || out != "ok call c t\n" || errs != "" {
		t.Errorf("a postsubmit check: got %d %q %q, want 0, ok", code, out, errs)
	}
}

// A failing call names each step and why it runs no target.
func TestCallNamesSteps(t *testing.T) {
	steps := `[{"name":"one","cmd":["make","u"]},{"cmd":["sh","-c","make t; x"]}]`
	code, out, _ := runCalls(t, "call c u t v\n", oneCheck(steps))
	want := "FAIL call c u t v: no step runs make t: step 1 (one) runs make u, step 2 has a script that is not read: an operator `;`; " +
		"no step runs make v: step 1 (one) runs make u, step 2 has a script that is not read: an operator `;`\n"
	if code != 1 || out != want {
		t.Errorf("got %d %q, want 1 %q", code, out, want)
	}
}

// A step before the first that runs a target could write a GNUmakefile
// or replace the kit's files, so each must be a make step read with
// certainty; a step after it is free.
func TestCallStepsBefore(t *testing.T) {
	steps := `[{"name":"prep","cmd":["sh","ci/prep.sh"]},{"cmd":["make","u"]},{"cmd":["make","t"]},{"cmd":["sh","ci/after.sh"]},{"cmd":["make","t"]}]`
	code, out, _ := runCalls(t, "call c u t\n", oneCheck(steps))
	want := `FAIL call c u t: a step before the one that runs make u is not a make step read with certainty: step 1 (prep) runs "sh ci/prep.sh", not make or sh -c; ` +
		`a step before the one that runs make t is not a make step read with certainty: step 1 (prep) runs "sh ci/prep.sh", not make or sh -c` + "\n"
	if code != 1 || out != want {
		t.Errorf("got %d %q, want 1 %q", code, out, want)
	}
	steps = `[{"cmd":["make","u"]},{"cmd":["make","t"]},{"cmd":["sh","ci/after.sh"]}]`
	if code, out, _ := runCalls(t, "call c t u\n", oneCheck(steps)); code != 0 || out != "ok call c t u\n" {
		t.Errorf("make steps, then another: got %d %q, want 0, ok", code, out)
	}
}

func TestForbid(t *testing.T) {
	for _, c := range []struct {
		arg   string
		match bool
	}{
		{"ci/x.sh", true},
		{"./ci/x.sh", true},
		{"a/ci/x.sh", true},
		{"cd ci/x.sh && y", true},
		{"'ci/x.sh'", true},
		{"ci/x.sh/y", true},
		{"--manifest-path=ci/x.sh", true},
		{"ci/x.shell", false},
		{"ci/x.sh.bak", false},
		{"ci/x.sh-old", false},
		{"ci/x.sh_", false},
		{"ci/x.sh+", false},
		{"aci/x.sh", false},
		{"a.ci/x.sh", false},
		{"a-ci/x.sh", false},
		{"a_ci/x.sh", false},
		{"a+ci/x.sh", false},
		{"ci/x", false},
		{"aci/x.sh ci/x.sh", true},
		{"ci/x.shx ci/x.sh", true},
	} {
		wf := oneCheck(`[{"cmd":["sh","-c",` + quoteJSON(c.arg) + `]}]`)
		code, out, _ := runCalls(t, "forbid ci/x.sh\n", wf)
		if c.match {
			if want := "FAIL forbid ci/x.sh: check c step 1 names ci/x.sh\n"; code != 1 || out != want {
				t.Errorf("%q: got %d %q, want 1 %q", c.arg, code, out, want)
			}
		} else if code != 0 || out != "ok forbid ci/x.sh\n" {
			t.Errorf("%q: got %d %q, want 0, ok", c.arg, code, out)
		}
	}
	// Every element of every step of every check is read, advisory ones
	// too, and a step that names the path twice is named once.
	wf := `{"jobs":{"b":{"advisory":true,"steps":[{"cmd":["sh","ci/x.sh","ci/x.sh"]}]},"a":{"steps":[{"cmd":["true"]},{"cmd":["x","y","./ci/x.sh"]}]}}}`
	code, out, _ := runCalls(t, "forbid ci/x.sh\n", wf)
	if want := "FAIL forbid ci/x.sh: check a step 2 names ci/x.sh; check b step 1 names ci/x.sh\n"; code != 1 || out != want {
		t.Errorf("got %d %q, want 1 %q", code, out, want)
	}
	// A service's cmd and workdir are read too, each as written. A call
	// rule fails a check with services, which run beside the steps.
	wf = `{"jobs":{"c":{"environment":{"packages":["theseus-proved-port"],"services":[` +
		`{"name":"s","cmd":["sh","./ci/x.sh"],"timeout":"5m","workdir":"."},` +
		`{"name":"t","cmd":["sh","./run.sh"],"workdir":"ci/x.sh"},` +
		`{"name":"u","cmd":["sh","./x.sh"],"workdir":"ci"}]},"steps":[{"cmd":["make","t"]}],"triggers":["TRIGGER_PRESUBMIT"]}}}`
	code, out, _ = runCalls(t, "call c t\nforbid ci/x.sh\n", wf)
	if want := "FAIL call c t: its environment starts services, which run beside the steps and could change what make runs\nFAIL forbid ci/x.sh: check c service s names ci/x.sh; check c service t names ci/x.sh\n"; code != 1 || out != want {
		t.Errorf("services: got %d %q, want 1 %q", code, out, want)
	}
	// So are the variables' values, in name order, wherever a check sets
	// them; foreach entries with other fields and no vars are read too.
	wf = `{"jobs":{"c":{"steps":[{"cmd":["sh","-c","sh $B"]}],"variables":{"B":"./ci/x.sh","A":"ci/x.shell"},` +
		`"environment":{"packages":[],"variables":{"E":"ci/x.sh"}},` +
		`"foreach":{"entries":[{"name":"one","resources":"large"},{"name":"two","vars":{"Z":"x","F":"a/ci/x.sh"}}]}}}}`
	code, out, _ = runCalls(t, "forbid ci/x.sh\n", wf)
	if want := "FAIL forbid ci/x.sh: check c environment variable E names ci/x.sh; check c variable B names ci/x.sh; check c foreach entry two var F names ci/x.sh\n"; code != 1 || out != want {
		t.Errorf("variables: got %d %q, want 1 %q", code, out, want)
	}
}

// reads and base end the refusals of a banned name and of a VERUS_BASE
// that is not the base Checks sets.
const (
	reads   = ", which make, the shell, the loader or cargo reads, or Checks sets"
	threads = "sets VERUS_THREADS other than to digits or the guest's CPUs (`VERUS_THREADS=\"${CG_CPUS:-}\"` in a script)"
	notSafe = ", which is not one quoted expansion of a variable the check sets nor a plain value (letters, digits and _ . , / : + @ % = -)"
	base    = "sets VERUS_BASE other than to the base Checks sets (`VERUS_BASE=\"${CHAINGUARD_BASE_REVISION:?...}\"` in a script)"
)

func quoteJSON(s string) string {
	r := strings.NewReplacer(`\`, `\\`, `"`, `\"`, "\n", `\n`)
	return `"` + r.Replace(s) + `"`
}

func TestRulesErrors(t *testing.T) {
	wf := oneCheck(`[{"cmd":["make","t"]}]`)
	for _, c := range []struct{ rules, msg string }{
		{"", "no rule: it holds nothing"},
		{"# only a comment\n\n", "no rule: it holds nothing"},
		{"call c\n", ":1: a call line names a check and at least one target"},
		{"call\n", ":1: a call line names a check and at least one target"},
		{"\ncall c t t\n", ":2: target t twice"},
		{"call c t\ncall c u\n", ":2: check c has a call line already, line 1"},
		{"forbid\n", ":1: a forbid line names one path"},
		{"forbid a b\n", ":1: a forbid line names one path"},
		{"forbid /a\n", ":1: forbid /a: not a clean relative path"},
		{"forbid ./a\n", ":1: forbid ./a: not a clean relative path"},
		{"forbid a/\n", ":1: forbid a/: not a clean relative path"},
		{"forbid a//b\n", ":1: forbid a//b: not a clean relative path"},
		{"forbid a/../b\n", ":1: forbid a/../b: not a clean relative path"},
		{"forbid .\n", ":1: forbid .: not a clean relative path"},
		{"forbid ..\n", ":1: forbid ..: not a clean relative path"},
		{"forbid ../a\n", ":1: forbid ../a: not a clean relative path"},
		{"forbid a\nforbid a\n", ":2: forbid a twice, line 1"},
		{"calls c t\n", `:1: "calls" is not a rule (call or forbid)`},
		{"call c -n\n", ":1: target -n: not a goal a step could run (an option, a variable or not literal)"},
		{"call c t A=1\n", ":1: target A=1: not a goal a step could run (an option, a variable or not literal)"},
		{"call c t$\n", ":1: target t$: not a goal a step could run (an option, a variable or not literal)"},
		{"call c 't'\n", ":1: target 't': not a goal a step could run (an option, a variable or not literal)"},
		{"call c t with\n", ":1: `with` names no variable"},
		{"call c with A\n", ":1: a call line names a check and at least one target"},
		{"call c t with A A\n", ":1: variable A twice"},
		{"call c t with 1A\n", ":1: with 1A: not a variable name"},
		{"call c t with A-B\n", ":1: with A-B: not a variable name"},
		{"call c t with MAKEFLAGS\n", ":1: with MAKEFLAGS: make, the shell, the loader or cargo reads it, or Checks sets it"},
		{"call c t with MAKE\n", ":1: with MAKE: make, the shell, the loader or cargo reads it, or Checks sets it"},
		{"call c t with CARGO_TARGET_DIR\n", ":1: with CARGO_TARGET_DIR: make, the shell, the loader or cargo reads it, or Checks sets it"},
		{"call c t with RUSTFLAGS\n", ":1: with RUSTFLAGS: make, the shell, the loader or cargo reads it, or Checks sets it"},
		{"call c t with RUSTC_WRAPPER\n", ":1: with RUSTC_WRAPPER: make, the shell, the loader or cargo reads it, or Checks sets it"},
		{"call c t with LD_PRELOAD\n", ":1: with LD_PRELOAD: make, the shell, the loader or cargo reads it, or Checks sets it"},
		{"call c t with VPATH\n", ":1: with VPATH: make, the shell, the loader or cargo reads it, or Checks sets it"},
		{"call c t with GIT_DIR\n", ":1: with GIT_DIR: make, the shell, the loader or cargo reads it, or Checks sets it"},
		{"call c t with GIT_CONFIG_COUNT\n", ":1: with GIT_CONFIG_COUNT: make, the shell, the loader or cargo reads it, or Checks sets it"},
		{"call c t with HOME\n", ":1: with HOME: make, the shell, the loader or cargo reads it, or Checks sets it"},
		{"call c t with XDG_CONFIG_HOME\n", ":1: with XDG_CONFIG_HOME: make, the shell, the loader or cargo reads it, or Checks sets it"},
		{"call c t with VERUS_THREADS\n", ":1: with VERUS_THREADS: always allowed"},
		{"call c t with VERUS_BASE\n", ":1: with VERUS_BASE: always allowed, to the values the rules say"},
		{"call c t with VERUS_COUNT\n", ":1: with VERUS_COUNT: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with PROVED_PORT_HOME\n", ":1: with PROVED_PORT_HOME: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with KANI_EXCLUDE\n", ":1: with KANI_EXCLUDE: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with KANI_SHARD\n", ":1: with KANI_SHARD: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with KANI_SHARDS\n", ":1: with KANI_SHARDS: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with CI_SKIP_UPDATE\n", ":1: with CI_SKIP_UPDATE: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with CI_CELL\n", ":1: with CI_CELL: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with CONSUMER\n", ":1: with CONSUMER: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with LINT_UNSAFE_INNER\n", ":1: with LINT_UNSAFE_INNER: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with KANI_ONLY\n", ":1: with KANI_ONLY: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with KIT_CALLS_RULES\n", ":1: with KIT_CALLS_RULES: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with kit_calls\n", ":1: with kit_calls: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with verus_count_cmd\n", ":1: with verus_count_cmd: it chooses which kit, counter, ratchet or harness runs"},
		{"call c t with CG_CPUS\n", ":1: with CG_CPUS: make, the shell, the loader or cargo reads it, or Checks sets it"},
	} {
		code, out, errs := runCalls(t, c.rules, wf)
		if code != 2 || out != "" || !strings.Contains(errs, c.msg) || !strings.HasPrefix(errs, "calls: ") {
			t.Errorf("rules %q: got %d %q %q, want 2 and %q", c.rules, code, out, errs, c.msg)
		}
	}
}

func TestWorkflowErrors(t *testing.T) {
	for _, c := range []struct{ wf, msg string }{
		{``, "not a resolved workflow (JSON)"},
		{`[]`, "not a resolved workflow (JSON)"},
		{`null`, "not a resolved workflow: not a JSON object"},
		{`{}`, "no `jobs`"},
		// What chainctl checks explain --output=json prints (exit 1) for a
		// configuration it refuses.
		{`{"root":"/tmp/p","diagnostics":[{"file":".chainguard/ci/go.yaml","message":"not valid YAML: yaml: line 189: did not find expected node content"}]}`, "no `jobs`"},
		{`{"jobs":{},"environments":{}}`, `the workflow has a field this tool does not read: "environments"`},
		{`{"jobs":{}} {}`, "not a resolved workflow (JSON): data after the value"},
		// What explain prints for a configuration with no check is `{}`;
		// no check at all holds every forbid rule vacuously.
		{`{}`, "no `jobs`"},
		{`{"jobs":{}}`, "`jobs` holds no check"},
		{`{"jobs":[]}`, "`jobs` is not an object of checks"},
		{`{"jobs":null}`, "`jobs` is not an object of checks"},
		{`{"jobs":{"c":[]}}`, "check c is not an object"},
		{`{"jobs":{"c":null}}`, "check c is not an object"},
		{`{"jobs":{"c":{}}}`, "check c has no steps"},
		{`{"jobs":{"c":{"steps":[{"cmd":["a"],"dir":"x"}]}}}`, `check c: steps: json: unknown field "dir"`},
		{`{"jobs":{"c":{"steps":{}}}}`, "check c: steps: json: cannot unmarshal"},
		{`{"jobs":{"c":{"steps":[{"cmd":"make t"}]}}}`, "check c: steps: json: cannot unmarshal"},
		{`{"jobs":{"c":{"steps":[{"name":"x"}]}}}`, "check c: step 1 has no cmd"},
		{`{"jobs":{"c":{"steps":[{"cmd":[]}]}}}`, "check c: step 1 has no cmd"},
		{`{"jobs":{"c":{"steps":[{"cmd":["a"]}],"environment":[]}}}`, "check c: environment is not an object"},
		{`{"jobs":{"c":{"steps":[{"cmd":["a"]}],"environment":null}}}`, "check c: environment is not an object"},
		{`{"jobs":{"c":{"steps":[{"cmd":["a"]}],"environment":{"services":{}}}}}`, "check c: environment services: json: cannot unmarshal"},
		{`{"jobs":{"c":{"steps":[{"cmd":["a"]}],"environment":{"services":[{"cmd":["a"],"env":{}}]}}}}`, `check c: environment services: json: unknown field "env"`},
		{`{"jobs":{"b":{"steps":[]},"a":{}}}`, "check a has no steps"},
		{`{"jobs":{"c":{"steps":[],"variables":{"A":1}}}}`, "check c: variables: json: cannot unmarshal number"},
		{`{"jobs":{"c":{"steps":[],"variables":[]}}}`, "check c: variables: json: cannot unmarshal array"},
		{`{"jobs":{"c":{"steps":[],"environment":{"variables":{"A":true}}}}}`, "check c: environment variables: json: cannot unmarshal bool"},
		{`{"jobs":{"c":{"steps":[],"foreach":[]}}}`, "check c: foreach: json: cannot unmarshal array"},
		{`{"jobs":{"c":{"steps":[],"foreach":{"entries":[{"name":"e","vars":{"A":[]}}]}}}}`, "check c: foreach entry e vars: json: cannot unmarshal array"},
	} {
		code, out, errs := runCalls(t, "forbid a\n", c.wf)
		if code != 2 || out != "" || !strings.Contains(errs, c.msg) || !strings.HasPrefix(errs, "calls: -: ") {
			t.Errorf("workflow %q: got %d %q %q, want 2 and %q", c.wf, code, out, errs, c.msg)
		}
	}
}

func TestUsage(t *testing.T) {
	rules := write(t, "rules", "forbid a\n")
	wf := write(t, "wf", `{"jobs":{"c":{"steps":[{"cmd":["true"]}]}}}`)
	for _, args := range [][]string{
		nil,
		{wf},
		{"-rules", rules},
		{"-rules", rules, wf, wf},
		{"-rules", "", wf},
		{"-x", wf},
	} {
		var out, errb bytes.Buffer
		if code := run(args, strings.NewReader(""), &out, &errb); code != 2 || out.Len() != 0 || errb.String() != usage+"\n" {
			t.Errorf("args %q: got %d %q %q, want 2 and usage", args, code, out.String(), errb.String())
		}
	}
	var out, errb bytes.Buffer
	if code := run([]string{"-rules", rules, wf}, nil, &out, &errb); code != 0 || out.String() != "ok forbid a\n" {
		t.Errorf("a workflow file: got %d %q %q, want 0, ok", code, out.String(), errb.String())
	}
	out.Reset()
	errb.Reset()
	if code := run([]string{"-rules", rules, wf + ".none"}, nil, &out, &errb); code != 2 || !strings.HasPrefix(errb.String(), "calls: open ") {
		t.Errorf("a missing workflow: got %d %q, want 2", code, errb.String())
	}
	out.Reset()
	errb.Reset()
	if code := run([]string{"-rules", rules + ".none", wf}, nil, &out, &errb); code != 2 || !strings.HasPrefix(errb.String(), "calls: open ") {
		t.Errorf("a missing rules file: got %d %q, want 2", code, errb.String())
	}
}

// Every variable the kit's proved-port.mk assigns or defines, and every
// upper-case variable its scripts read, chooses what a target runs:
// `with` refuses each, so a variable the kit adds cannot be set from a
// check unseen.
func TestKitVariablesRefused(t *testing.T) {
	dir := filepath.Join("..", "..", "make")
	data, err := os.ReadFile(filepath.Join(dir, "proved-port.mk"))
	if err != nil {
		t.Fatal(err)
	}
	assign := regexp.MustCompile(`^(?:define\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*(?:\?=|::?=|\+=|=|$)`)
	n := 0
	for _, line := range strings.Split(string(data), "\n") {
		m := assign.FindStringSubmatch(line)
		if m == nil || m[1] == "endef" || m[1] == "export" || m[1] == "include" {
			continue
		}
		n++
		if withRefused(m[1]) == "" {
			t.Errorf("proved-port.mk assigns %s, which `with` allows", m[1])
		}
	}
	if n < 100 {
		t.Errorf("read %d assignments in proved-port.mk, want at least 100", n)
	}
	// Names a script sets itself before it reads them: awk's NF,
	// perf-budget.sh's self-test MUT and SYM, consumers.sh's P_* (the
	// documentation's name for the ${CONSUMERS_PREFIX}_* it passes each
	// run.sh), and `_`.
	own := map[string]bool{"NF": true, "MUT": true, "SYM": true, "_": true}
	read := regexp.MustCompile(`\$\{?([A-Z_][A-Z0-9_]*)`)
	scripts, err := filepath.Glob(filepath.Join(dir, "*.sh"))
	if err != nil || len(scripts) < 10 {
		t.Fatalf("kit/make/*.sh: %v, %d scripts", err, len(scripts))
	}
	n = 0
	for _, s := range scripts {
		data, err := os.ReadFile(s)
		if err != nil {
			t.Fatal(err)
		}
		for _, m := range read.FindAllStringSubmatch(string(data), -1) {
			if own[m[1]] || strings.HasPrefix(m[1], "P_") {
				continue
			}
			n++
			if withRefused(m[1]) == "" {
				t.Errorf("%s reads %s, which `with` allows", filepath.Base(s), m[1])
			}
		}
	}
	if n < 50 {
		t.Errorf("read %d variable reads in kit/make/*.sh, want at least 50", n)
	}
}
