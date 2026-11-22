// Command seams generates and checks a port's seam ledger: every item
// the Verus proofs take on trust. It is the kit's Go form of challoc's
// verus/seams.sh (kit/PLAN.md step 3), whose ledgers it writes byte for
// byte given a rules file equal to challoc's heuristics.
//
//	seams [-C DIR] [-rules FILE]            print the totals tables
//	seams [-C DIR] [-rules FILE] -write     regenerate the ledger
//	seams [-C DIR] [-rules FILE] -check     exit 1 if the ledger is stale
//	                                        or an overrides line disagrees
//	seams [-C DIR] [-rules FILE] -entries   one tab-separated line per seam
//
// -C is the tree's root (default .); -rules is the rules file, relative
// to the root unless absolute (default verus/seams.rules).
//
// The ledger is one file per source file with seams: the seams of
// src/a/b.rs are in LEDGER/src/a/b.rs.md. The scan reads the source
// directory's .rs files but its test files (and skips `#[cfg(test)]`
// items) for the items Verus takes on trust: external_body (exec fn,
// proof fn, type, const), axiom, assume_spec, ext_type_spec,
// ext_trait_spec, spec_impl (every PartialEqSpecImpl, and a fmt-trait one
// unless its type's trait impl is inside `verus!` in the same file),
// no_decreases, assume_new and ghost_assume_new (inside `verus!`, outside
// tests and external_body bodies, whose mints are noted on their entry),
// and unsupported-attribute (`#[verifier::assume_unsupported_attributes]`
// on a fn, const or static, and each export a pair macro writes). Each
// entry gives its place, the contract Verus assumes (verbatim), a
// category and a tool from the rules' heuristics or the overrides, a
// status, and its hashes `contract C, body B`: the POSIX cksum CRCs of
// its contract and body, `//` comments dropped and whitespace runs
// outside string and char literals made one space (inside a literal
// every byte counts).
//
// The seams of src/a/b.rs may have an overrides file,
// LEDGER/src/a/b.rs.overrides: one `name<TAB>category<TAB>tool[<TAB>
// status[<TAB>hashes]]` per line, `#` comments, the name as printed in
// that file's ledger or `name#N` for the Nth seam of that name. -check
// fails when the ledger differs from what -write would write, when an
// overrides file's source file has no seams or a line matches no seam,
// when a seam has no category or an unknown tool, and on the hash rules:
// a status that cites a check (`discharged ...`, `trusted (partly checked
// ...`) or claims nothing (`trusted (claims nothing: <why>)`) records the
// item's hashes in the fifth column and they equal the item's now; any
// other status records none. A claims nothing status is only for an
// opaque type or an external_body constant, external_body, with no
// requires, ensures, returns or _recursive_types attribute, a reason, no
// cited check, the assumption tool, not a `tracked` type, and no `unsafe
// impl` of Send or Sync for the type anywhere in the source directory,
// test files included.
//
// -entries prints, per seam: source file, line, kind, id (the name, with
// #N when shared in its file), category, tool, status, overrides file (or
// -), and the contract normalized as its hash reads it. It is for a
// port's own checks of the checks its statuses cite (Kani harnesses,
// assertions and the like), which stay in the port.
//
// The rules file (tab-separated fields, `#` comments) holds what is the
// port's:
//
//	ledger<TAB>DIR             the ledger directory (default verus/ledger)
//	source<TAB>DIR             the directory scanned (default src)
//	command<TAB>TEXT           the command the ledger header names (required)
//	seams-doc<TAB>PATH         the SEAMS.md each ledger file links to
//	                           (default verus/SEAMS.md)
//	test-file<TAB>PATTERN      a test file: a pattern with a / matches the
//	                           path from the root, any other the base name;
//	                           * and ? match / too, as in find -path
//	tool<TAB>NAME              a tool, in table order
//	category<TAB>NAME<TAB>TOOL a category and its default tool, in table order
//	assumption-tool<TAB>TOOL   the tool of a seam no check applies to (required)
//	cite-tag<TAB>TAG           a status holding `TAG: ` cites a check
//	guess<TAB>CAT|-<TAB>COND.. a category heuristic: the first guess whose
//	                           conditions all hold names the category (-
//	                           for none); COND is FIELD=RE or FIELD!RE,
//	                           FIELD one of kind, name, file and body (the
//	                           body with comments and literals blanked), RE
//	                           an RE2 expression
//	pair<TAB>MACRO<TAB>FILE<TAB>VAR<TAB>VAR
//	                           a macro_rules! MACRO in FILE whose invocation
//	                           lines `fn a / b(..)` write two C exports with
//	                           the attributes it puts above `fn $VAR(`
//	unsupported-doc<TAB>TEXT   the note on each unsupported-attribute entry
//
// Each rule that holds one value (ledger, source, command, seams-doc,
// unsupported-doc, assumption-tool, pair) is given at most once, and
// each path is relative to the root and inside it, not the root itself.
//
// Exit status: 0 done, 1 a check failed, 2 usage, rules or input errors.
package main

import (
	"flag"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"
)

func main() { os.Exit(run(os.Args[1:], os.Stdout, os.Stderr)) }

func run(args []string, stdout, stderr io.Writer) int {
	fl := flag.NewFlagSet("seams", flag.ContinueOnError)
	fl.SetOutput(stderr)
	root := fl.String("C", ".", "the tree's root")
	rulesFile := fl.String("rules", "verus/seams.rules", "the rules file, relative to the root unless absolute")
	write := fl.Bool("write", false, "regenerate the ledger")
	check := fl.Bool("check", false, "exit 1 if the ledger is stale or an overrides line disagrees")
	entries := fl.Bool("entries", false, "print one tab-separated line per seam")
	fl.Usage = func() {
		fmt.Fprintln(stderr, "usage: seams [-C DIR] [-rules FILE] [-write | -check | -entries]")
	}
	if err := fl.Parse(args); err != nil {
		return 2
	}
	n := 0
	for _, b := range []bool{*write, *check, *entries} {
		if b {
			n++
		}
	}
	if fl.NArg() != 0 || n > 1 {
		fl.Usage()
		return 2
	}
	rf := *rulesFile
	if !filepath.IsAbs(rf) {
		rf = filepath.Join(*root, rf)
	}
	r, err := readRules(rf)
	if err != nil {
		fmt.Fprintf(stderr, "seams: %v\n", err)
		return 2
	}
	in, err := readTree(*root, r)
	if err != nil {
		fmt.Fprintf(stderr, "seams: %v\n", err)
		return 2
	}
	res := generate(r, in, *write || *check || *entries)
	for _, e := range res.errs {
		fmt.Fprintln(stderr, e)
	}
	if res.code != 0 {
		return res.code
	}
	switch {
	case *write:
		for _, d := range res.drift {
			fmt.Fprintln(stderr, d)
		}
		if err := writeLedger(*root, r, res.ledger); err != nil {
			fmt.Fprintf(stderr, "seams: %v\n", err)
			return 2
		}
		return 0
	case *check:
		return checkLedger(*root, r, in, res, stdout, stderr)
	case *entries:
		for _, e := range res.entries {
			of := "-"
			if e.okey != "" {
				of = path.Join(r.Ledger, e.file) + ".overrides"
			}
			fmt.Fprintf(stdout, "%s\t%d\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n", e.file, e.line, e.kind, e.id, e.cat, e.tool, e.stat, of, norm(e.hc))
		}
		return 0
	}
	fmt.Fprint(stdout, res.summary)
	return 0
}

// walk lists the regular files (and links) under root/dir whose names
// end in suffix, as paths from the root, in byte order; none when dir
// does not exist.
func walk(root, dir, suffix string) ([]string, error) {
	var out []string
	base := filepath.Join(root, dir)
	if _, err := os.Lstat(base); os.IsNotExist(err) {
		return nil, nil
	}
	err := filepath.WalkDir(base, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() || !strings.HasSuffix(d.Name(), suffix) {
			return nil
		}
		rel, err := filepath.Rel(root, p)
		if err != nil {
			return err
		}
		out = append(out, filepath.ToSlash(rel))
		return nil
	})
	sort.Strings(out)
	return out, err
}

// readTree reads the overrides files, the source and test files and the
// pair macro's file.
func readTree(root string, r *Rules) (input, error) {
	var in input
	files, err := walk(root, r.Source, ".rs")
	if err != nil {
		return in, err
	}
	ovs, err := walk(root, r.Ledger, ".rs.overrides")
	if err != nil {
		return in, err
	}
	read := func(names []string, into *[]namedText) {
		for _, f := range names {
			b, err := os.ReadFile(filepath.Join(root, f))
			if err != nil {
				in.unreadable = append(in.unreadable, f)
				continue
			}
			*into = append(*into, namedText{f, string(b)})
		}
	}
	var src, tests []string
	for _, f := range files {
		if r.isTest(f) {
			tests = append(tests, f)
		} else {
			src = append(src, f)
		}
	}
	if len(src) == 0 {
		return in, fmt.Errorf("no sources under %s/", r.Source)
	}
	read(ovs, &in.overrides)
	read(src, &in.sources)
	read(tests, &in.tests)
	if r.PairMacro != "" {
		if b, err := os.ReadFile(filepath.Join(root, r.PairFile)); err == nil {
			in.pairText = string(b)
		}
	}
	return in, nil
}

// writeLedger replaces every *.rs.md under the ledger directory with the
// files given.
func writeLedger(root string, r *Rules, ledger map[string]string) error {
	dir := filepath.Join(root, r.Ledger)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	old, err := walk(root, r.Ledger, ".rs.md")
	if err != nil {
		return err
	}
	for _, f := range old {
		if err := os.Remove(filepath.Join(root, f)); err != nil {
			return err
		}
	}
	for f, text := range ledger {
		p := filepath.Join(dir, f)
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			return err
		}
		if err := os.WriteFile(p, []byte(text), 0o644); err != nil {
			return err
		}
	}
	return nil
}

// checkLedger compares the ledger on disk with the generated one, and
// reports leftover overrides files and the drift lines.
func checkLedger(root string, r *Rules, in input, res *result, stdout, stderr io.Writer) int {
	stale := false
	have, err := walk(root, r.Ledger, ".rs.md")
	if err != nil {
		fmt.Fprintf(stderr, "seams: %v\n", err)
		return 2
	}
	want := make([]string, 0, len(res.ledger))
	for f := range res.ledger {
		want = append(want, path.Join(r.Ledger, f))
	}
	sort.Strings(want)
	if strings.Join(want, "\n") != strings.Join(have, "\n") {
		stale = true
	}
	for f, text := range res.ledger {
		b, err := os.ReadFile(filepath.Join(root, r.Ledger, f))
		if err != nil || string(b) != text {
			stale = true
		}
	}
	// An overrides file whose source file has no seams is left over.
	for _, o := range in.overrides {
		md := strings.TrimSuffix(strings.TrimPrefix(o.name, r.Ledger+"/"), ".overrides") + ".md"
		if _, ok := res.ledger[md]; !ok {
			fmt.Fprintf(stderr, "seams: %s: its source file has no seams\n", o.name)
			stale = true
		}
	}
	for _, d := range res.drift {
		fmt.Fprintln(stderr, d)
	}
	if !stale && len(res.drift) == 0 {
		fmt.Fprintf(stdout, "%s is up to date\n", r.Ledger)
		return 0
	}
	if stale {
		fmt.Fprintf(stderr, "%s is stale: run %s\n", r.Ledger, r.Command)
	}
	return 1
}
