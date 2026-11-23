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
//	seams [-C DIR] [-rules FILE] -cites     one tab-separated line per
//	                                        citation a status makes
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
// on a fn, const or static, and each export a pair macro writes). An
// attribute may hold blanks between `#`, `!` and `[` (`# [..]`); a line
// whose code (outside an open attribute) ends in `#` or `#!`, its `[` on
// a later line, is an input error. Each
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
// test files included. On the citations: -check fails a discharged or
// partly checked status that cites no check, a `TAG: ` (on any status)
// that names nothing, and a discharged status that cites a tag the
// rules mark partial.
//
// A status cites a check with `TAG: ` for a cite tag: the citation list
// runs from there to the first `;` or `)` (or the status's end), split
// at a comma and the blanks after it and at ` and ` (with or without a
// comma before it), each nonempty part a citation, up to a part that
// holds `: ` (the next tag, read at its own place). It is seams.sh's
// cites(). -cites prints one line per citation: overrides file, source
// file, line, the overrides line's name, tag, and the part as written,
// escaped as -entries escapes; seams in -entries order, each status's
// citations in the order they stand in it (a name that serves several
// seams prints its citations once per seam). A port's checks resolve
// each line of a tag the rules give no check (challoc's Isla citation
// reads `NAME in TARGET`, its emptyasm one `NAME @ TARGET`).
//
// -check resolves each citation of a tag with a cite-fn, cite-assert,
// cite-test, cite-case or cite-row line, and fails each that does not
// resolve (once, though a name serves several seams), as seams.sh's
// checkcite() resolved Kani, callers, const assert, loom and GenMC
// citations.
// A cite-fn citation is a path
// `a::b::m::f`: the file is the longest leading run of the segments but
// the last that names SOURCE/a/b.rs or else SOURCE/a/b/mod.rs (a first
// segment a cite-module line names reads under its DIR, and a path that
// reaches such a DIR any other way names nothing); each segment after
// the file's but the last is an inline mod, one per level of rustfmt's
// four-blank indentation, its line `mod m {` (`pub` or `pub(x)` before
// it allowed); the first line at the innermost level that starts `fn f(`
// or `fn f<` (visibility allowed) is the fn, and it resolves when the
// line `#[ATTR]` stands at its indentation above it with only other
// attributes (each on its lines, a wrapped one to its closing `]`) and
// comment lines between. It then fails, as gated, when an attribute
// that applies to the fn (any above it up to the previous line of other
// code, across blank lines, comment lines and indentation, as Rust
// applies them) is `#[cfg(..)]`, `#[cfg_attr(..)]` or `#[ignore]`,
// wherever it stands on its line and with or without blanks (or a line
// break) between `#` and `[`, other than `#[cfg(CFG)]` whole on its
// line, as written (a literal in CFG byte for byte), for a CFG field of
// the tag's cite-fn line: under such an
// attribute no build may compile or run the fn. A cfg on a mod or on
// the file's `mod` line is not read. A cite-assert citation is `LABEL
// in FILE` (FILE after the last ` in `, a .rs file under SOURCE or a
// cite-module DIR and not a test file; LABEL not blank): exactly one
// top-level (at depth 0 of braces, brackets and parentheses, so not in
// an inline mod) `const _: () = assert!(`, alone on its line or the whole
// assertion on it, right under a block of `//` lines whose first starts
// `// LABEL`, where the last line of code above the block (across
// blank lines, comment lines and block comments) is no attribute (a
// line starting `#[` or ending `]`). A cite-test citation is
// `MOD::TEST`, a test of the test crate in DIR (challoc's loom models in
// tests/loom/src; seams.sh's loomtest()): DIR/MOD.rs names the seam's
// source file in backquotes (`SOURCE/a/b.rs`, or `a/b.rs` for a file in
// a directory under SOURCE) and holds a top-level line `fn TEST() {` (a
// visibility allowed) whose attributes (up to the previous line of
// other code, across blank and comment lines, those before it on its
// line too, and the last top-level run after other code or an item's
// closing delimiter on that line) hold `#[test]`, no `#[should_panic]`
// (a control fails by design) and no cfg, cfg_attr or ignore attribute
// but `#[cfg(CFG)]` whole on its line for a CFG field; and the `///`
// lines among those attributes, or the body (to the delimiter that
// closes it), name the seam's
// function in backquotes: the name without `#N`, and for a name with no
// `::` `m::name`, m the source file's module (its base name, the
// directory's for a mod.rs, `lib` for lib.rs), alone or after a path of
// words (`a::m::f`). The test crate's manifest (DIR/../Cargo.toml, read
// as the reach check reads the main crate's) must keep DIR/lib.rs its
// tested root: DIR is the package's src, no `[lib] path` names another
// root, no `autolib = false`, no `[lib]` `test = false` or `harness =
// false`. DIR/lib.rs must declare `mod MOD;` at its top level under
// no cfg, cfg_attr or ignore attribute but `#[cfg(CFG)]` and no
// `#[path]`, read as the reach check reads a `mod` line, and neither file
// may hold an inner cfg attribute but `#![cfg(CFG)]` whole on its line.
// A `#[doc = ".."]` attribute is not read as a doc comment. A cite-case
// citation is `CASE [MODEL ...]` (one blank between the words), a case
// of DIR/cases (challoc's GenMC cases in tests/genmc; seams.sh's
// genmctest()), read as challoc's tests/genmc/run.sh reads it: lines
// `name | template | models | constants | expected`, a blank line or
// one starting `#` none, fields trimmed. Exactly one line names CASE,
// with no empty field; it expects `pass` alone (a `fail` case is a
// control); its models, split on blanks, are the cited ones in the same
// order; and its template is a word whose DIR/TEMPLATE.rs (read whole)
// names the seam's source file and the seam's function in backquotes,
// as a cite-test file and test do, anywhere in the file. A cite-row
// citation, once its cite-fn, cite-assert, cite-test or cite-case check
// passes, needs a line starting `| ` in
// FILE's table under the line `## HEADING` (to the next `## ` line)
// whose first cell holds the citation in backquotes and whose text holds
// every PART. Each file is read with the lexer: a line in a block
// comment or literal is no attribute, mod, fn, label or assertion, and
// a file that ends inside one resolves nothing.
//
// With a notcopy line, -check also holds the tracked types (`tracked
// struct` or `tracked enum`, ghost tokens whose axioms may rely on their
// not being Copy) to challoc's not-Copy checks, seams.sh's trackedcheck
// and notcopycheck. A not-Copy check of TYPE, cited `LABEL in FILE`
// (read as a cite-assert citation), is exactly one labeled top-level
// `const _: () = assert!(` whose text, to the line that ends in `);`,
// read as code, names `<TYPE as NotCopy<_>>::HOLDS` and `<TYPE as
// NotClone<_>>::HOLDS` (TYPE alone or after a path, with or without
// arguments that hold no `<` or `>`), in a FILE that also holds the
// whole top-level lines `impl<T: Copy> NotCopy<..> for T {}` and
// `impl<T: Clone> NotClone<..> for T {}`, each with no attribute as the
// last line of code above it, without which the check compiles for any
// type. -check fails each tracked type declared in a source or test
// file (seam or not; `tracked struct` or `tracked enum` anywhere on a
// code line, after attributes on it too) with no not-Copy check in the
// notcopy FILE under the label TYPE in backquotes that is also a
// cite-assert assertion (no attribute above it), each second tracked
// type of a name
// already seen (one check cannot stand for both), and each tracked-type
// seam whose status cites TAG with none of those citations a not-Copy
// check of its type.
//
// A cited assertion's FILE, and the notcopy FILE, must be compiled in
// every build: -check fails a const assert citation (or the notcopy line,
// once) when a file from FILE up to the crate root (SOURCE/lib.rs, or
// SOURCE/main.rs when there is no lib.rs: with both, a library build
// compiles lib.rs's tree alone; a Cargo.toml beside SOURCE that moves
// that target or turns off its discovery fails it, as do a `[lib]` or
// any `[[bin]]` table with main.rs as the root and the target settings
// as dotted keys or inline tables; the manifest must be there with a
// `[package]` table, and one holding a multi-line string, a line starting
// with `[` that is no whole header, an escape, or an absolute path, which
// its line reader does not follow, fails, as does a `[` line inside a
// value still open over lines; a root not at Cargo's default src/lib.rs
// or src/main.rs counts only when a `[lib] path` names it) holds an inner
// `#![cfg(..)]` or a
// `#![cfg_attr(..)]` that adds a cfg, or is not declared by a `mod
// NAME;` line in its parent (the root for a file directly under SOURCE,
// else DIR.rs, unless that is a root or a mod.rs, or DIR/mod.rs; a
// mod.rs stands for its directory; a line-ending `#` joins the next
// line's `[`), or is
// declared under a cfg, cfg_attr or ignore attribute or a `#[path]`
// (not followed); only a `mod NAME;` at the parent's top level counts,
// not one inside any delimiters (a macro's body or arguments, `cfg_if!`,
// an inline mod); an attribute after other code on the line before
// attaches to it; and an inner attribute counts only at a file's top
// level. A
// file under a cite-module DIR (or DIR.rs outside SOURCE) is held to a
// weaker rule: it is compiled when the tool's build runs, and neither
// its inner attributes nor the `#[path]` line reaching the DIR are read;
// a cite-module DIR may not be SOURCE or hold it. An item's head split
// over lines (`pub` / `mod NAME;`) is one item.
//
// With pins lines, -check also holds each pins FILE's rows in place,
// seams.sh's pins() and copies(). FILE lists the source lines a check
// runs a copy of (challoc's verus/kani/pins.txt: Kani cannot run an
// inline `asm!` block, so a harness runs a copy of the body without
// it), so a change to an original or its copy fails until the copy, the
// list and the status citing the check are revisited. A row is
// `SRC<TAB>LINE[<TAB>NAME:ROLE]` (`#` lines and lines of blanks are
// comments; a row of one field or more than three, an empty field, and
// a SRC that is no clean path inside the tree fail). SRC's rows must
// appear in it in their order, each on the line after the one before
// (lines compared with leading and trailing blanks and tabs removed;
// SRC's blank lines and lines starting `//` skipped), but where a `...`
// row allows any lines between; a `...` row stands between two rows of
// its SRC and has no third field (a tagged one would count as a row of a
// side, keeping it consecutive across a gap). The first row of SRC, and each after a `...`, starts a run,
// which must occur in SRC exactly once, so a second copy (in a block
// comment) cannot stand in for it. A third field ties a copy to its
// original: for each NAME, the `NAME:orig` rows in order must equal the
// `NAME:copy` rows, and each side's rows (`orig` and `orig-only`, `copy`
// and `copy-only`) must be consecutive rows of FILE; another ROLE, or a
// NAME with no rows of a side, fails. -check also fails a FILE that is
// gone or has no rows, a SRC that is gone, and an overrides file whose text names FILE
// with no row in FILE for its source file.
//
// -entries prints, per seam: source file, line, kind, id (the name, with
// #N when shared in its file), category, tool, status, overrides file (or
// -), and the contract normalized as its hash reads it. Each seam is
// one line of nine fields: in every field a backslash, newline, tab and
// carriage return are written `\\`, `\n`, `\t` and `\r` (an
// unsupported-attribute contract holds a newline after each attribute
// line, and a literal may hold a tab), so a reader splits lines and
// tabs, then undoes those four to get the text its hash reads. It is for a
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
//	cite-tag<TAB>TAG[<TAB>partial]
//	                           a status holding `TAG: ` cites a check;
//	                           `partial`: one that checks only part of a
//	                           contract, which a discharged status may not
//	                           cite. TAG holds no `;`, `)` or `:` and is
//	                           given once
//	cite-fn<TAB>TAG<TAB>ATTR[<TAB>CFG...]
//	                           each TAG citation names a fn with the
//	                           attribute line `#[ATTR]` (ATTR without `[`,
//	                           `]` or blanks), under no cfg, cfg_attr or
//	                           ignore attribute but `#[cfg(CFG)]` (the
//	                           cfg the tool builds under, `kani`)
//	cite-assert<TAB>TAG        each TAG citation names a labeled assertion
//	cite-test<TAB>TAG<TAB>DIR[<TAB>CFG...]
//	                           each TAG citation names a test `MOD::TEST`
//	                           in DIR/MOD.rs naming the seam's function,
//	                           under no gate but `#[cfg(CFG)]` (CFG with
//	                           no literal)
//	cite-case<TAB>TAG<TAB>DIR  each TAG citation names a pass case `CASE
//	                           [MODEL ...]` of DIR/cases whose template
//	                           names the seam's function; a tag takes one
//	                           of cite-fn, cite-assert, cite-test and
//	                           cite-case, once
//	cite-row<TAB>TAG<TAB>FILE<TAB>HEADING<TAB>PART...
//	                           each TAG citation needs a row in FILE's
//	                           `## HEADING` table holding every PART (once
//	                           per tag)
//	cite-module<TAB>NAME<TAB>DIR
//	                           a cite-fn path starting `NAME::` reads under
//	                           DIR (once per NAME); the TAG of cite-fn,
//	                           cite-assert, cite-test, cite-case and cite-row
//	                           is a cite-tag above the line
//	notcopy<TAB>FILE<TAB>TAG   the tracked types' not-Copy checks are in
//	                           FILE, and a tracked-type seam cites them with
//	                           TAG, a tag with a cite-assert line above (once)
//	pins<TAB>FILE              FILE lists source lines a check runs a copy
//	                           of, which -check holds in place (each FILE
//	                           once)
//	guess<TAB>CAT|-<TAB>COND.. a category heuristic: the first guess whose
//	                           conditions all hold names the category (-
//	                           for none); COND is FIELD=RE or FIELD!RE,
//	                           FIELD one of kind, name, file and body (the
//	                           body with comments and literals blanked), RE
//	                           an RE2 expression
//	export-macro<TAB>MACRO<TAB>FILE<TAB>FORM<TAB>VAR...
//	                           a macro_rules! MACRO in FILE whose invocations
//	                           (`MACRO! {` or `path::MACRO! {`, blanks
//	                           allowed before the `!`, its lines below)
//	                           write C exports: each `fn` line directly
//	                           inside names them in FORM, `slash` (`fn a /
//	                           b(..)`) or `list` (`fn f [a, b, ..] (..)`,
//	                           the names on that line or on the lines up
//	                           to the `]`). Each export is an
//	                           unsupported-attribute entry with the
//	                           attribute lines directly above `fn $VAR(`
//	                           in the definition (lines of comments only
//	                           skipped; the file read with the lexer) and
//	                           that line's qualifiers; one VAR serves
//	                           every name, or there is one per name, in
//	                           order. The invocation's tokens must read as
//	                           declarations (`#[..]`* `fn` and the names,
//	                           then a rest to a `;` or through a `{..}`
//	                           body) naming exactly the exports its lines
//	                           gave. A fn line the form does not read,
//	                           names the VARs do not fit, tokens that are
//	                           no such declarations or name other exports,
//	                           a renamed import (`MACRO as NAME`), any
//	                           other use of MACRO! (each file's tokens are
//	                           read too, so a name and its `!` split over
//	                           lines count), a macro_rules! MACRO outside
//	                           FILE or a second one in it, and a second
//	                           `fn $VAR(` for one VAR in the definition
//	                           are input errors.
//	pair<TAB>MACRO<TAB>FILE<TAB>VAR<TAB>VAR
//	                           export-macro MACRO FILE slash VAR VAR
//	unsupported-doc<TAB>TEXT   the note on each unsupported-attribute entry
//
// Each rule that holds one value (ledger, source, command, seams-doc,
// unsupported-doc, assumption-tool, and export-macro or pair, which
// name the one export macro) is given at most once, and
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
	cites := fl.Bool("cites", false, "print one tab-separated line per citation a status makes")
	fl.Usage = func() {
		fmt.Fprintln(stderr, "usage: seams [-C DIR] [-rules FILE] [-write | -check | -entries | -cites]")
	}
	if err := fl.Parse(args); err != nil {
		return 2
	}
	n := 0
	for _, b := range []bool{*write, *check, *entries, *cites} {
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
	res := generate(r, in, *write || *check || *entries || *cites)
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
		res.drift = append(res.drift, resolveCites(*root, r, res.entries)...)
		var files []string
		for _, f := range append(append([]namedText{}, in.sources...), in.tests...) {
			files = append(files, f.name)
		}
		res.drift = append(res.drift, notCopyChecks(*root, r, res.entries, files)...)
		res.drift = append(res.drift, pinChecks(*root, r, in.overrides)...)
		return checkLedger(*root, r, in, res, stdout, stderr)
	case *cites:
		x := entryField.Replace
		// A seam with no overrides line is `trusted` and cites nothing.
		for _, e := range res.entries {
			of := path.Join(r.Ledger, e.file) + ".overrides"
			name := strings.TrimPrefix(e.okey, e.file+":")
			cs, _ := r.citations(e.stat)
			for _, c := range cs {
				fmt.Fprintf(stdout, "%s\t%s\t%d\t%s\t%s\t%s\n", x(of), x(e.file), e.line, x(name), x(c.tag), x(c.target))
			}
		}
		return 0
	case *entries:
		for _, e := range res.entries {
			of := "-"
			if e.okey != "" {
				of = path.Join(r.Ledger, e.file) + ".overrides"
			}
			x := entryField.Replace
			fmt.Fprintf(stdout, "%s\t%d\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n", x(e.file), e.line, x(e.kind), x(e.id), x(e.cat), x(e.tool), x(e.stat), x(of), x(norm(e.hc)))
		}
		return 0
	}
	fmt.Fprint(stdout, res.summary)
	return 0
}

// entryField writes an -entries field on one line, tabs only between
// fields. The replacer makes one pass and a source `\` becomes `\\`, so
// the escapes read back unambiguously.
var entryField = strings.NewReplacer(`\`, `\\`, "\n", `\n`, "\t", `\t`, "\r", `\r`)

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
