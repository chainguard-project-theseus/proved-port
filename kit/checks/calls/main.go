// Command calls holds a port's checks to the kit's targets (PLAN step 8:
// "a lint that a port's checks call the kit's targets"). The
// theseus-proved-port package installs it as /usr/bin/proved-port-calls.
//
// It reads no check file. Its input is the resolved configuration that
// `chainctl checks explain --output=json DIR` prints (workflow.v1: every
// check with its steps, YAML anchors and aliases expanded, and its
// environment inline; the command makes no network or API call, and
// needs no login and no .git), so what it reads is what Checks plans:
//
//	chainctl checks explain --output=json . > workflow.json
//	proved-port-calls -rules ci/kit-calls.rules workflow.json
//
// Usage:
//
//	calls -rules RULES WORKFLOW
//
// WORKFLOW is a file, or - for standard input. RULES has one rule per
// line, its fields separated by blanks; blank lines and lines whose
// first non-blank character is `#` are skipped:
//
//	call CHECK TARGET... [with NAME...]
//	forbid PATH
//
// A call rule holds when the check CHECK exists; it is not advisory,
// does not run only on request (`when: {requested: true}`), and runs on
// presubmit or postsubmit (its trigger or triggers), since otherwise it
// gates no pull request or push; its environment installs
// theseus-proved-port (the package name alone or followed by a version
// constraint, `~`, `=`, `<` or `>`), which brings the kit's targets; it
// sets no variable but those allowed (below); it has no field but those
// `chainctl checks explain` prints today (advisory, egress, environment,
// foreach, identity, network, resources, schedule, steps, timeout,
// trigger, triggers, variables, when), and its environment, when and
// foreach fields hold only the fields read here (packages, variables,
// services; requested, changed; entries of name, vars, resources,
// network, timeout); its environment starts no services (a sidecar runs
// beside the steps in the same checkout, so it could write a GNUmakefile
// or the kit's files while make runs); and for each
// TARGET one of its steps runs make with TARGET among its goals, and
// every step before the first that does is a make step read with
// certainty too (any other could write a GNUmakefile, which make reads
// before the Makefile, or replace the kit's files; steps after it are
// free). A check's resources, network, timeout, schedule and
// when.changed paths are the port's to choose. (No target is named
// `with`.)
//
// Variables. Every variable a call check sets reaches make, from the
// environment or its command line, and may change which kit, counter or
// ratchet the target runs (PROVED_PORT_HOME, VERUS_COUNT, VERUS_BASE=none
// skip or replace the kit's; MAKEFLAGS or SHELL change make itself). So a
// call check, in its variables, its environment's variables, its foreach
// entries' vars, a script's assignments before make and make's command
// line, sets only these, to these values:
//   - VERUS_THREADS (which proved-port.mk pastes unquoted into the cargo
//     verus line): digits or empty, or in a script exactly
//     `"${CG_CPUS:-}"` or `"$CG_CPUS"`, the guest's CPUs;
//   - VERUS_BASE, only in a script, with `=`, to exactly one quoted
//     expansion of the base Checks sets (`"$CHAINGUARD_BASE_REVISION"`,
//     `"${CHAINGUARD_BASE_REVISION}"`, `"${CHAINGUARD_BASE_REVISION:?TEXT}"`),
//     which is never `none`;
//   - the names the call line's `with` lists, the port's own (QUANTUM),
//     which the rules file shows a reviewer, each to a plain value (letters, digits and
//     `_ . , / : + @ % = -`: no blank, quote, backslash, `$` or shell
//     character) or, in a script, exactly one quoted expansion (`"$X"`,
//     `"${X}"`, `"${X:?TEXT}"`, `"${X?TEXT}"`, `"${X:-}"`, `"${X-}"`)
//     of a name X the check itself sets, whose value is held the same
//     way.
//
// A value is read as written: in a script before the shell removes
// quotes, so `"$C"HAINGUARD_BASE_REVISION` is no expansion of
// CHAINGUARD_BASE_REVISION, and a quoted or escaped `$`, which make would
// expand (`$(eval override MAKEFLAGS:=n)` turns on dry-run mode), is no
// plain value. CG_CPUS and CHAINGUARD_BASE_REVISION are the guest's,
// which this tool does not see.
//
// `with` may not list a name make, the shell, the loader or cargo reads,
// or Checks sets: one starting MAKE (MAKE, which proved-port.mk's verify
// runs, MAKEFLAGS, MAKEFILES, MAKEOVERRIDES, MAKELEVEL), `.` (make's
// special variables: .SHELLFLAGS, .DEFAULT_GOAL), BASH_FUNC_, LD_, CARGO
// (CARGO_TARGET_DIR, CARGO_HOME), RUST (RUSTFLAGS, RUSTC,
// RUSTC_WRAPPER), GIT_ (GIT_DIR, GIT_REPLACE_REF_BASE, GIT_CONFIG_*,
// which choose the base revision the counter compares the ratchets with)
// or XDG_, or MFLAGS, GNUMAKEFLAGS, SHELL, VPATH, GPATH, SUFFIXES,
// BASH_ENV, ENV, PATH, SHELLOPTS, BASHOPTS, IFS, HOME (git's
// configuration), CHAINGUARD_BASE_REVISION or CG_CPUS; no check may set
// those. Nor may it list the kit's own
// variables, which choose which kit, counter, ratchet, harness or
// library a target runs (KANI_EXCLUDE, KANI_SHARD, which at 0 runs no
// harness and passes, VERUS_COUNT, PROVED_PORT_HOME, CI_SKIP_UPDATE, a
// lower-case internal like verus_count_cmd): a name starting, in any
// case, VERUS_, PROVED_PORT_, KANI_, UNSAFE_, NOALIAS_, CODEGEN_, PERF_,
// SKIP_, GLIBC_MAX, KNOWN_, HEADER_CHECK_, ABI_ELF_, DEVIATIONS_,
// CONSUMER, VERIFY, CI_ or LINT_. The port's Makefile sets those, where a
// reviewer reads them; a test holds every name proved-port.mk assigns,
// and every upper-case variable the kit's scripts read, to this list. A
// check that splits Kani between instances (a foreach of KANI_SHARD)
// cannot be a call check: this tool cannot tell that the instances
// cover every shard.
//
// A step runs make only when it is read with certainty; any other step
// runs no target here, and a failing rule says why for each step:
//   - Its cmd is `make ARG...`, or `sh -c SCRIPT [ARG...]` or `bash -c
//     SCRIPT [ARG...]` (`-c` alone, no other option) whose SCRIPT is
//     exactly one simple command, `[NAME=VALUE...] make ARG...`, with the
//     word make unquoted. The script may not hold an operator (`;`, `&`,
//     `|`, `<`, `>`, `(`, `)`), a comment, a command substitution, a line
//     continuation or a second line: `cd x && make y`, `make y || true`
//     and `make y | tee log` run no target here. simpleCommand in shell.go
//     says what it reads.
//   - Each ARG is a goal, a variable (a word holding `=`: a name of
//     letters, digits, `_` and `.`, with no blank or `$` for make to trim
//     or expand, then one of make's operators `=`, `:=`, `::=`, `:::=`,
//     `+=`, `?=`, not `!=`, whose value make runs as a command; in a
//     script, with the name and `=` unquoted and no
//     `$` outside double quotes, whose expansion the shell would split
//     into more words), or an option
//     that runs every goal and keeps make's exit status: -s, --silent,
//     --quiet, -k, --keep-going, -j, -jN, --jobs, --jobs=N (a bare -j or
//     --jobs may take the next word as N when it is all digits), -O,
//     -OTYPE, --output-sync, --output-sync=TYPE. Any other option (-n, -q,
//     -t, -i, -C, -f, --, combined flags) means the step runs no target
//     here.
//   - Each goal is literal: no quote, escape, expansion or glob
//     character.
//   - Each variable on make's command line or before make in the script
//     is one the rule allows.
//
// What the target then does is proved-port.mk's and the port's
// Makefile's: this tool reads neither, nor what an earlier make step's
// recipe writes.
//
// A forbid rule holds when no step or service of any check names PATH, a
// clean relative path (an in-tree copy the kit replaced:
// ci/lint-unsafe.sh, verus/count), and no variable a check sets holds it.
// Each element of each step's cmd, of each service's cmd and its workdir
// (each as written; a service's cmd is not joined to its workdir), and
// each value of the check's variables, its environment's variables and
// its foreach entries' vars, is read as text: PATH
// matches where it starts the element or follows a character that is no
// letter, digit, `.`, `_`, `+` or `-` (a `/` too, so `./ci/x.sh` and
// `a/ci/x.sh` match `ci/x.sh`), and ends it or comes before such a
// character or a `/` (so `verus/count/Cargo.toml` matches `verus/count`,
// and `verus/count.toml` does not). It reads the text, not the shell, so
// a mention in an echo matches too.
//
// Each rule prints one line, in the rules file's order: `ok RULE` or
// `FAIL RULE: why`. Exit 0: every rule holds; 1: one fails; 2: usage, a
// rules file that cannot be read or holds no rule, a line that is no
// rule, or a WORKFLOW that is not a resolved workflow. A line is no rule
// when its first word is neither call nor forbid; when a call names no
// target, a target twice, a target no step could run as a goal (one
// starting `-`, or holding `=` or a character a goal may not), or a check
// an earlier call line names, or when its `with` lists no name, a name
// twice, one that is no shell variable name, VERUS_THREADS or VERUS_BASE
// (always allowed) or a name it may not list; and when a forbid names other than one
// clean path, or a path an earlier forbid line names. A WORKFLOW is not
// a resolved workflow unless it is one JSON object whose only field is
// `jobs`, holding at least one check (explain prints `{}` for none, over
// which a forbid rule would hold vacuously), each check has steps and each step a cmd and no field but cmd
// and name, and each check's environment, if any, is an object whose
// services, if any, are a list of objects of name, cmd, timeout and
// workdir, and each variables object (the check's, its environment's,
// a foreach entry's vars) maps names to strings.
package main

import (
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"path"
	"regexp"
	"sort"
	"strings"
)

func main() {
	os.Exit(run(os.Args[1:], os.Stdin, os.Stdout, os.Stderr))
}

const usage = "usage: calls -rules RULES WORKFLOW"

// rule is one line of the rules file.
type rule struct {
	line    int
	kind    string // "call" or "forbid"
	check   string
	targets []string
	with    []string        // the variables the call line allows, as written
	allow   map[string]bool // the same, as a set
	path    string
}

func (r rule) String() string {
	if r.kind == "call" {
		s := "call " + r.check + " " + strings.Join(r.targets, " ")
		if len(r.with) > 0 {
			s += " with " + strings.Join(r.with, " ")
		}
		return s
	}
	return "forbid " + r.path
}

func run(args []string, stdin io.Reader, stdout, stderr io.Writer) int {
	fs := flag.NewFlagSet("calls", flag.ContinueOnError)
	fs.SetOutput(io.Discard)
	rulesPath := fs.String("rules", "", "")
	if err := fs.Parse(args); err != nil || *rulesPath == "" || fs.NArg() != 1 {
		fmt.Fprintln(stderr, usage)
		return 2
	}
	rules, err := readRules(*rulesPath)
	if err != nil {
		fmt.Fprintf(stderr, "calls: %v\n", err)
		return 2
	}
	var data []byte
	if name := fs.Arg(0); name == "-" {
		data, err = io.ReadAll(stdin)
	} else {
		data, err = os.ReadFile(name)
	}
	if err != nil {
		fmt.Fprintf(stderr, "calls: %v\n", err)
		return 2
	}
	w, err := readWorkflow(data)
	if err != nil {
		fmt.Fprintf(stderr, "calls: %s: %v\n", fs.Arg(0), err)
		return 2
	}
	code := 0
	for _, r := range rules {
		var why []string
		if r.kind == "call" {
			why = checkCall(w, r)
		} else {
			why = checkForbid(w, r)
		}
		if len(why) == 0 {
			fmt.Fprintf(stdout, "ok %s\n", r)
			continue
		}
		code = 1
		fmt.Fprintf(stdout, "FAIL %s: %s\n", r, strings.Join(why, "; "))
	}
	return code
}

// readRules reads and checks the rules file.
func readRules(name string) ([]rule, error) {
	data, err := os.ReadFile(name)
	if err != nil {
		return nil, err
	}
	var rules []rule
	calls := map[string]int{}
	forbids := map[string]int{}
	for k, text := range strings.Split(string(data), "\n") {
		line := k + 1
		f := strings.Fields(text)
		if len(f) == 0 || strings.HasPrefix(f[0], "#") {
			continue
		}
		bad := func(msg string, a ...any) error {
			return fmt.Errorf("%s:%d: %s", name, line, fmt.Sprintf(msg, a...))
		}
		switch f[0] {
		case "call":
			targets, with := f[1:], []string(nil)
			for k, x := range targets {
				if x == "with" {
					targets, with = f[1:k+1], f[k+2:]
					if len(with) == 0 {
						return nil, bad("`with` names no variable")
					}
					break
				}
			}
			if len(targets) < 2 {
				return nil, bad("a call line names a check and at least one target")
			}
			check := targets[0]
			targets = targets[1:]
			if prev, ok := calls[check]; ok {
				return nil, bad("check %s has a call line already, line %d", check, prev)
			}
			seen := map[string]bool{}
			for _, t := range targets {
				if seen[t] {
					return nil, bad("target %s twice", t)
				}
				if strings.HasPrefix(t, "-") || strings.ContainsAny(t, notLiteral+"=") {
					return nil, bad("target %s: not a goal a step could run (an option, a variable or not literal)", t)
				}
				seen[t] = true
			}
			allow := map[string]bool{}
			for _, v := range with {
				if allow[v] {
					return nil, bad("variable %s twice", v)
				}
				if why := withRefused(v); why != "" {
					return nil, bad("with %s: %s", v, why)
				}
				allow[v] = true
			}
			calls[check] = line
			rules = append(rules, rule{line: line, kind: "call", check: check, targets: targets, with: with, allow: allow})
		case "forbid":
			if len(f) != 2 {
				return nil, bad("a forbid line names one path")
			}
			p := f[1]
			if strings.HasPrefix(p, "/") || path.Clean(p) != p || p == "." || p == ".." || strings.HasPrefix(p, "../") {
				return nil, bad("forbid %s: not a clean relative path", p)
			}
			if prev, ok := forbids[p]; ok {
				return nil, bad("forbid %s twice, line %d", p, prev)
			}
			forbids[p] = line
			rules = append(rules, rule{line: line, kind: "forbid", path: p})
		default:
			return nil, bad("%q is not a rule (call or forbid)", f[0])
		}
	}
	if len(rules) == 0 {
		return nil, fmt.Errorf("%s: no rule: it holds nothing", name)
	}
	return rules, nil
}

// banned reports whether make or the shell reads the variable name
// before a target runs, so setting it may change what make runs, or
// Checks sets it.
func banned(name string) bool {
	switch name {
	case "MFLAGS", "GNUMAKEFLAGS", "SHELL", "VPATH", "GPATH", "SUFFIXES",
		"BASH_ENV", "ENV", "PATH", "SHELLOPTS", "BASHOPTS", "IFS", "HOME", "CHAINGUARD_BASE_REVISION", "CG_CPUS":
		return true
	}
	// make's own (MAKE, MAKEFLAGS, MAKEFILES, MAKEOVERRIDES, MAKELEVEL and
	// the rest) and its special variables (.SHELLFLAGS, .DEFAULT_GOAL);
	// the loader's; and cargo's and rustc's, which choose the binaries,
	// flags and target directory cargo verus and cargo kani run with.
	// git's (GIT_DIR, GIT_REPLACE_REF_BASE, GIT_CONFIG_*, and the HOME and
	// XDG_ directories it reads its configuration from), which choose the
	// base revision the counter compares the ratchets with.
	for _, p := range []string{"MAKE", ".", "BASH_FUNC_", "LD_", "CARGO", "RUST", "GIT_", "XDG_"} {
		if strings.HasPrefix(name, p) {
			return true
		}
	}
	return false
}

// kitPrefixes are the prefixes of the variables the kit's proved-port.mk
// and scripts read (in any case: its own internal names are lower case):
// they choose which kit, counter, ratchet, harness or library a target
// runs, which the port's Makefile sets, not a check. The Kani split
// knobs are among them: KANI_SHARD=0 runs no harness and passes, and a
// KANI_SHARDS above the instances leaves shards unrun. So are the
// scripts' environment knobs (CI_SKIP_UPDATE, CI_CELL, CONSUMER,
// LINT_UNSAFE_*).
var kitPrefixes = []string{
	"VERUS_", "PROVED_PORT_", "KANI_", "UNSAFE_", "NOALIAS_", "CODEGEN_", "PERF_",
	"SKIP_", "GLIBC_MAX", "KNOWN_", "HEADER_CHECK_", "ABI_ELF_", "DEVIATIONS_",
	"CONSUMER", "VERIFY", "CI_", "LINT_",
}

// kitName reports whether name is one of the kit's variables, which a
// check may not set.
func kitName(name string) bool {
	up := strings.ToUpper(name)
	for _, p := range kitPrefixes {
		if strings.HasPrefix(up, p) {
			return true
		}
	}
	return false
}

// withRefused returns why a call line may not allow the variable name,
// or "".
func withRefused(name string) string {
	if !isNameStart(name[0]) {
		return "not a variable name"
	}
	for k := 1; k < len(name); k++ {
		if !isNameChar(name[k]) {
			return "not a variable name"
		}
	}
	switch {
	case banned(name):
		return "make, the shell, the loader or cargo reads it, or Checks sets it"
	case name == "VERUS_THREADS", name == "VERUS_BASE":
		return "always allowed, to the values the rules say"
	case kitName(name):
		return "it chooses which kit, counter, ratchet or harness runs, which the port's Makefile sets, not a check"
	}
	return ""
}

// A value's form. A check's variables and argv-form make arguments are
// read as make reads them; a script's assignments as written, before the
// shell removes quotes and expands.
const (
	fromVars = iota
	fromArgv
	fromScript
)

// safeValue is a literal value: nothing a shell or make would split,
// quote, expand or run (no blank, quote, backslash, `$`, `;`, `#` and
// the like).
var safeValue = regexp.MustCompile(`^[A-Za-z0-9_.,/:+@%=-]*$`)

// oneExpansion is a script value that is exactly one quoted expansion of
// a named parameter: `"$X"`, `"${X}"`, `"${X:?TEXT}"`, `"${X?TEXT}"`,
// `"${X:-}"` or `"${X-}"`, TEXT holding no quote, backslash, `$`, `}` or
// backquote. Its value is X's or empty, or the shell fails.
var oneExpansion = regexp.MustCompile("^\"\\$(?:([A-Za-z_][A-Za-z0-9_]*)|\\{([A-Za-z_][A-Za-z0-9_]*)(?::?\\?[^\"$`\\\\}]*|:?-)?\\})\"$")

// callVars is what a call rule allows a check to set: the rule's `with`
// names, and the names the check itself sets in its variables (whose
// values are held to safeValue), which a script may expand.
type callVars struct {
	r   rule
	set map[string]bool
}

// setRefused returns why a call check may not set the variable name with
// make's operator op to value (as written: raw for a script), or "".
func (c callVars) setRefused(name, op, value string, form int) string {
	switch {
	case banned(name):
		return "sets " + name + ", which make, the shell, the loader or cargo reads, or Checks sets"
	case name != "VERUS_THREADS" && name != "VERUS_BASE" && !c.r.allow[name]:
		return "sets " + name + ", which the call line does not allow (`with " + name + "`)"
	}
	// The one parameter the value expands, or "" for a literal.
	from := ""
	if form == fromScript {
		if m := oneExpansion.FindStringSubmatch(value); m != nil {
			from = m[1] + m[2]
		}
	}
	switch name {
	case "VERUS_BASE":
		// The base Checks sets, which `none` never is.
		if from == "CHAINGUARD_BASE_REVISION" && op == "=" {
			return ""
		}
		return "sets VERUS_BASE other than to the base Checks sets (`VERUS_BASE=\"${CHAINGUARD_BASE_REVISION:?...}\"` in a script)"
	case "VERUS_THREADS":
		// proved-port.mk pastes it unquoted into the cargo verus line.
		if from == "CG_CPUS" || (from == "" && allDigitsOrEmpty(value)) {
			return ""
		}
		return "sets VERUS_THREADS other than to digits or the guest's CPUs (`VERUS_THREADS=\"${CG_CPUS:-}\"` in a script)"
	}
	if from != "" {
		if c.set[from] {
			return ""
		}
		return "sets " + name + " to " + from + ", which the check's variables do not set"
	}
	if safeValue.MatchString(value) {
		return ""
	}
	return fmt.Sprintf("sets %s to %q, which is not one quoted expansion of a variable the check sets nor a plain value (letters, digits and _ . , / : + @ %% = -)", name, value)
}

func allDigitsOrEmpty(s string) bool {
	return s == "" || allDigits(s)
}

const kitPackage = "theseus-proved-port"

// checkCall returns why a call rule fails, or nothing.
func checkCall(w *workflow, r rule) []string {
	j, ok := w.jobs[r.check]
	if !ok {
		return []string{"no check " + r.check}
	}
	var why []string
	var keys []string
	for k := range j.fields {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		if !callFields[k] {
			why = append(why, fmt.Sprintf("the check has a field this tool does not read: %q", k))
		}
	}
	if len(why) > 0 {
		return why
	}
	var advisory bool
	var env environment
	var cond when
	var vars map[string]string
	var each foreach
	var trigger string
	var triggers []string
	for _, err := range []error{
		j.field("advisory", &advisory),
		j.field("environment", &env),
		j.field("when", &cond),
		j.field("variables", &vars),
		j.field("foreach", &each),
		j.field("trigger", &trigger),
		j.field("triggers", &triggers),
	} {
		if err != nil {
			why = append(why, err.Error())
		}
	}
	if len(why) > 0 {
		return why
	}
	if advisory {
		why = append(why, "the check is advisory")
	}
	if cond.Requested {
		why = append(why, "the check runs only on request")
	}
	if !gates(trigger, triggers) {
		why = append(why, "the check runs on neither presubmit nor postsubmit")
	}
	// A sidecar runs beside the steps in the same checkout, so it could
	// write a GNUmakefile or the kit's files while make runs.
	if len(env.Services) > 0 {
		why = append(why, "its environment starts services, which run beside the steps and could change what make runs")
	}
	if !hasKit(env.Packages) {
		why = append(why, "its environment does not install "+kitPackage)
	}
	// Every variable a check sets reaches make from the environment.
	// Each refusal is said once, though several foreach entries set the
	// name.
	c := callVars{r: r, set: map[string]bool{}}
	all := []map[string]string{env.Variables, vars}
	for _, e := range each.Entries {
		all = append(all, e.Vars)
	}
	for _, m := range all {
		for name := range m {
			c.set[name] = true
		}
	}
	said := map[string]bool{}
	refuse := func(where string, vars map[string]string) {
		for _, name := range sortedKeys(vars) {
			if bad := c.setRefused(name, "=", vars[name], fromVars); bad != "" && !said[where+bad] {
				said[where+bad] = true
				why = append(why, where+bad)
			}
		}
	}
	refuse("its environment's variables: ", env.Variables)
	refuse("its variables: ", vars)
	for _, e := range each.Entries {
		refuse("its foreach vars: ", e.Vars)
	}
	// goals[k] is the goals step k runs, or nil with reasons[k] why it
	// runs none here.
	goals := make([][]string, len(j.steps))
	reasons := make([]string, len(j.steps))
	for k, s := range j.steps {
		goals[k], reasons[k] = c.stepGoals(s.Cmd)
	}
	describe := func(k int) string {
		desc := fmt.Sprintf("step %d", k+1)
		if j.steps[k].Name != "" {
			desc += fmt.Sprintf(" (%s)", j.steps[k].Name)
		}
		if goals[k] != nil {
			return desc + " runs make " + strings.Join(goals[k], " ")
		}
		return desc + " " + reasons[k]
	}
	for _, t := range r.targets {
		first := -1
		for k, g := range goals {
			for _, x := range g {
				if x == t && first < 0 {
					first = k
				}
			}
		}
		if first < 0 {
			var steps []string
			for k := range j.steps {
				steps = append(steps, describe(k))
			}
			why = append(why, fmt.Sprintf("no step runs make %s: %s", t, strings.Join(steps, ", ")))
			continue
		}
		// A step before it could write a GNUmakefile, which make reads
		// before the Makefile, or replace the kit's files: each must be
		// a make step read with certainty too.
		for k := 0; k < first; k++ {
			if goals[k] == nil {
				why = append(why, fmt.Sprintf("a step before the one that runs make %s is not a make step read with certainty: %s", t, describe(k)))
			}
		}
	}
	return why
}

// gates reports whether a check's trigger or triggers hold presubmit or
// postsubmit.
func gates(trigger string, triggers []string) bool {
	for _, t := range append([]string{trigger}, triggers...) {
		if t == "TRIGGER_PRESUBMIT" || t == "TRIGGER_POSTSUBMIT" {
			return true
		}
	}
	return false
}

// hasKit reports whether packages installs the kit's package.
func hasKit(packages []string) bool {
	for _, p := range packages {
		rest, ok := strings.CutPrefix(p, kitPackage)
		if ok && (rest == "" || strings.ContainsRune("~=<>", rune(rest[0]))) {
			return true
		}
	}
	return false
}

// stepGoals returns the goals a step's cmd runs make with, or nil and
// why it runs no target here.
func (c callVars) stepGoals(cmd []string) ([]string, string) {
	var args []word
	form := fromArgv
	switch {
	case cmd[0] == "make":
		for _, a := range cmd[1:] {
			args = append(args, word{text: a, literal: true, plain: len(a), raw: a})
		}
	case (cmd[0] == "sh" || cmd[0] == "bash") && len(cmd) >= 3 && cmd[1] == "-c":
		form = fromScript
		words, err := simpleCommand(cmd[2])
		if err != nil {
			return nil, "has a script that is not read: " + err.Error()
		}
		k := 0
		for ; k < len(words); k++ {
			name := assignment(words[k])
			if name == "" {
				break
			}
			if bad := c.setRefused(name, "=", words[k].raw[len(name)+1:], fromScript); bad != "" {
				return nil, "has a script that " + bad
			}
		}
		if k == len(words) {
			return nil, "has a script that runs no command"
		}
		if !words[k].literal || words[k].text != "make" {
			return nil, fmt.Sprintf("has a script that runs %q, not make", words[k].text)
		}
		args = words[k+1:]
	default:
		return nil, fmt.Sprintf("runs %q, not make or sh -c", strings.Join(cmd, " "))
	}
	goals, err := c.makeGoals(args, form)
	if err != nil {
		return nil, "runs make " + err.Error()
	}
	if len(goals) == 0 {
		return nil, "runs make with no goal"
	}
	return goals, ""
}

// notLiteral are the characters no goal holds, as written in a cmd or
// in a script: quotes, escapes, expansions and globs.
const notLiteral = "*?[]{}~$`'\"\\"

// makeOps are the assignment operators read on make's command line. `!=`
// is not one: make runs its value as a shell command while it reads its
// command line.
var makeOps = map[string]bool{"=": true, ":=": true, "::=": true, ":::=": true, "+=": true, "?=": true}

// makeGoals reads make's arguments, as form gives them, and returns its
// goals.
func (c callVars) makeGoals(args []word, form int) ([]string, error) {
	var goals []string
	for k := 0; k < len(args); k++ {
		a := args[k]
		if strings.HasPrefix(a.text, "-") {
			if !a.literal {
				return nil, fmt.Errorf("with an option that is not literal: %q", a.text)
			}
			switch {
			case a.text == "-j" || a.text == "--jobs":
				if k+1 < len(args) && args[k+1].literal && allDigits(args[k+1].text) {
					k++
				}
			case okOption(a.text):
			default:
				return nil, fmt.Errorf("with an option this tool does not read as running every goal: %s", a.text)
			}
			continue
		}
		if eq := strings.IndexByte(a.text, '='); eq >= 0 {
			// The name and `=` unquoted, and no expansion the shell
			// splits into more words (`A=$X` with X = `1 -n`).
			if eq >= a.plain || a.split {
				return nil, fmt.Errorf("with a variable that is not read: %q", a.text)
			}
			// make trims blanks around the name and expands it, so a
			// name is read only when it is a plain one.
			name := strings.TrimRight(a.text[:eq], ":+?!")
			op := a.text[len(name) : eq+1]
			if !plainName(name) || !makeOps[op] {
				return nil, fmt.Errorf("with a variable that is not read: %q", a.text)
			}
			if bad := c.setRefused(name, op, a.raw[eq+1:], form); bad != "" {
				return nil, errors.New("with a variable that " + bad)
			}
			continue
		}
		if !a.literal || strings.ContainsAny(a.text, notLiteral) || a.text == "" {
			return nil, fmt.Errorf("with a goal that is not literal: %q", a.text)
		}
		goals = append(goals, a.text)
	}
	return goals, nil
}

// okOption reports whether a make option (other than a bare -j or
// --jobs) runs every goal and keeps make's exit status.
func okOption(o string) bool {
	switch o {
	case "-s", "--silent", "--quiet", "-k", "--keep-going", "-O", "--output-sync":
		return true
	}
	if n, ok := strings.CutPrefix(o, "-j"); ok {
		return allDigits(n)
	}
	if n, ok := strings.CutPrefix(o, "--jobs="); ok {
		return allDigits(n)
	}
	if t, ok := strings.CutPrefix(o, "-O"); ok {
		return syncType(t)
	}
	if t, ok := strings.CutPrefix(o, "--output-sync="); ok {
		return syncType(t)
	}
	return false
}

// plainName reports whether name is a variable name of letters, digits,
// `_` and `.` (.SHELLFLAGS), with no blank or `$` for make to trim or
// expand.
func plainName(name string) bool {
	if name == "" {
		return false
	}
	for k := 0; k < len(name); k++ {
		if !isNameChar(name[k]) && name[k] != '.' {
			return false
		}
	}
	return true
}

func syncType(t string) bool {
	switch t {
	case "none", "line", "target", "recurse":
		return true
	}
	return false
}

func allDigits(s string) bool {
	if s == "" {
		return false
	}
	for k := 0; k < len(s); k++ {
		if s[k] < '0' || s[k] > '9' {
			return false
		}
	}
	return true
}

// checkForbid returns why a forbid rule fails, or nothing.
func checkForbid(w *workflow, r rule) []string {
	var why []string
	for _, name := range w.names {
		j := w.jobs[name]
		for k, s := range j.steps {
			if anyNames(s.Cmd, r.path) {
				why = append(why, fmt.Sprintf("check %s step %d names %s", name, k+1, r.path))
			}
		}
		for _, s := range j.services {
			if anyNames(append([]string{s.Workdir}, s.Cmd...), r.path) {
				why = append(why, fmt.Sprintf("check %s service %s names %s", name, s.Name, r.path))
			}
		}
		for _, v := range j.values {
			if names(v.text, r.path) {
				why = append(why, fmt.Sprintf("check %s %s names %s", name, v.where, r.path))
			}
		}
	}
	return why
}

// anyNames reports whether any of texts names p.
func anyNames(texts []string, p string) bool {
	for _, t := range texts {
		if names(t, p) {
			return true
		}
	}
	return false
}

// pathChar reports whether c continues a path word.
func pathChar(c byte) bool {
	return isNameChar(c) || c == '.' || c == '+' || c == '-'
}

// names reports whether text names p at path-word boundaries.
func names(text, p string) bool {
	for from := 0; ; {
		i := strings.Index(text[from:], p)
		if i < 0 {
			return false
		}
		i += from
		end := i + len(p)
		before := i == 0 || !pathChar(text[i-1])
		after := end == len(text) || !pathChar(text[end])
		if before && after {
			return true
		}
		from = i + 1
	}
}

func sortedKeys(m map[string]string) []string {
	var keys []string
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}
