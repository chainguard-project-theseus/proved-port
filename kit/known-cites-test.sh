#!/bin/sh
# kit/known-cites-test.sh: tests of kit/make/known-cites.sh and
# kit/make/ledger-active.sh (the known-cites-test check). Each case
# writes a table source and an ABI ledger by hand, runs the script, and
# checks its exit status and its whole output, spelled out by hand (none
# is taken from a run of the script under test): rows citing active
# entries (one ledger file, a ledger directory), entries removed,
# planned or missing, cites of no known form, lines starting with the
# macro in another form, a table with no row, lines that are no row
# (the macro's #define, a longer name, a row behind a comment opener),
# another macro (known!), TODO(#n) cites against a stand-in gh (open,
# closed, unreachable), the closing keywords of a pull request's
# description in each form GitHub reads and look-alikes it does not,
# the history part in a full clone, a shallow one and outside git, a
# malformed ledger, names with `=`, missing inputs and usage; then
# ledger-active.sh on its own (its IDs and exit statuses). Run it from
# the repository root. It needs a POSIX shell, BusyBox's tools and git.
set -eu

script=$(pwd)/kit/make/known-cites.sh
la=$(pwd)/kit/make/ledger-active.sh
[ -f "$script" ] || { echo "FAIL: run it from the repository root (no kit/make/known-cites.sh)" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# A stand-in gh first on PATH: `gh issue view N -R REPO --json state -q
# .state` prints the state gh-states names for N (`N STATE` lines) and
# fails for an issue it does not name, as gh does when it cannot reach
# GitHub. Each call is logged to gh-calls.
mkdir bin
cat > bin/gh <<'EOF'
#!/bin/sh
echo "$*" >> "$GH_DIR/gh-calls"
# GitHub is reachable, for the repository gh-reach names, only while
# that file exists.
reach=$(cat "$GH_DIR/gh-reach" 2> /dev/null) || exit 1
case "$1 $2" in
  'repo view') [ "$3" = "$reach" ] ;;
  'issue view')
    s=$(awk -v n="$3" '$1 == n { print $2 }' "$GH_DIR/gh-states")
    [ -n "$s" ] || { echo "GraphQL: Could not resolve to an issue or pull request with the number of $3. (repository.issue)" >&2; exit 1; }
    echo "$s" ;;
  *) exit 1 ;;
esac
EOF
chmod +x bin/gh
PATH=$tmp/bin:$PATH
GH_DIR=$tmp
export PATH GH_DIR
: > gh-states
echo o/port > gh-reach
REPO='o/port old/name.x'

# run [ARGS...]: the script with ARGS; its output (stdout and stderr)
# in out, its status in rc.
run() {
  if sh "$script" "$@" > out 2>&1; then rc=0; else rc=$?; fi
}

# want WHAT RC: the run exited RC and printed exactly the file want.
want() {
  if [ "$rc" -ne "$2" ] || ! cmp -s want out; then
    printf 'got (exit %s):\n' "$rc" >&2
    cat out >&2
    printf 'want (exit %s):\n' "$2" >&2
    cat want >&2
    fail "$1"
  fi
  echo "ok $1"
}

# The NOTE lines of a run with no description bound, outside git.
notes() {
  echo "NOTE: no pull request description bound (KNOWN_CITES_PR_BODY unset); its closing keywords were not checked"
  echo "NOTE: $1 is not in a full git clone; commits closing an issue were not checked"
}

# The ledger: ABI-001 and ABI-004 active, ABI-002 removed, ABI-003
# planned, and a fenced template naming every Status.
cat > abi.md <<'EOF'
# Known differences

```markdown
### <ID>: <one-line summary>

- **Status:** active | planned | removed (<date>, PR #n)
```

### ABI-001: limits trip at other counts

- **Status:** active

### ABI-002: an old difference

- **Status:** removed (2026-10-23, PR #149): fixed

### ABI-003: a coming difference

- **Status:** planned

### ABI-004: sizes are the port's own

- **Status:** active, narrowed (2026-11-02)
EOF
mkdir abi
printf '# Known differences\n\n### ABI-003: an example\n\n- **Status:** active\n' > abi/README.md
printf '### ABI-001: limits\n\n- **Status:** active\n' > abi/ABI-001.md
printf '### ABI-002: old\n\n- **Status:** removed (2026-10-23, PR #149)\n' > abi/ABI-002.md
printf '### ABI-004: sizes\n\n- **Status:** active (renamed 2026-11-14)\n' > abi/ABI-004.md

# A table as the PCRE2 port writes one: the macro's #define, the array
# name that begins with the macro's, a comment naming the macro without
# a `(`, and rows (one tab-indented).
cat > known.c <<'EOF'
/* Each class is listed in KNOWN with its citation. */
#define KNOWN(cite, predicate) {cite " " #predicate, predicate}
static const struct known KNOWN_CLASSES[] = {
    KNOWN("ABI-001", too_large),
	KNOWN("ABI-004", sizes),
    {0},
};
EOF

# --- rows citing active entries ---------------------------------------------
run known.c abi.md "$REPO"
{ notes known.c; echo "PASS: 2 rows of known.c, each citing an active entry of abi.md or an open issue"; } > want
want "rows citing active entries, the ledger one file" 0
run known.c abi "$REPO"
{ notes known.c; echo "PASS: 2 rows of known.c, each citing an active entry of abi or an open issue"; } > want
want "rows citing active entries, the ledger a directory" 0

# A row that starts its line inside a block comment is read as any
# other: a missing entry there fails.
cat > k1.c <<'EOF'
    KNOWN("ABI-001", a),
/*
    KNOWN("ABI-003", c),
*/
    KNOWN("ABI-005", e),
EOF
run k1.c abi.md "$REPO"
cat > want <<'EOF'
FAIL: k1.c:3: ABI-003 is not an active entry of abi.md
FAIL: k1.c:5: ABI-005 is not an active entry of abi.md
FAIL: a row of k1.c cites nothing, an entry not active in abi.md, or a closed issue
EOF
want "entries planned and missing" 1
cat > k1b.c <<'EOF'
KNOWN("ABI-002", gone),
EOF
run k1b.c abi "$REPO"
cat > want <<'EOF'
FAIL: k1b.c:1: ABI-002 is not an active entry of abi
FAIL: a row of k1b.c cites nothing, an entry not active in abi, or a closed issue
EOF
want "a removed entry, the ledger a directory" 1

# --- cites of no known form ------------------------------------------------
cat > k2.c <<'EOF'
KNOWN("ABI-", a),
KNOWN("ABI-01x", b),
KNOWN("TODO(#)", c),
KNOWN("TODO(#12a)", d),
KNOWN("see ABI-001", e),
KNOWN(" ABI-001", f),
KNOWN("ABI-001 ", g),
KNOWN("", h),
KNOWN("TODO(12)", i),
KNOWN("abi-001", j),
KNOWN("ABI-00", k),
KNOWN("TODO(#012)", l),
EOF
run k2.c abi.md "$REPO"
cat > want <<'EOF'
FAIL: k2.c:1: ABI- is not an ABI-nnn or TODO(#n) cite
FAIL: k2.c:2: ABI-01x is not an ABI-nnn or TODO(#n) cite
FAIL: k2.c:3: TODO(#) is not an ABI-nnn or TODO(#n) cite
FAIL: k2.c:4: TODO(#12a) is not an ABI-nnn or TODO(#n) cite
FAIL: k2.c:5: see ABI-001 is not an ABI-nnn or TODO(#n) cite
FAIL: k2.c:6:  ABI-001 is not an ABI-nnn or TODO(#n) cite
FAIL: k2.c:7: ABI-001  is not an ABI-nnn or TODO(#n) cite
FAIL: k2.c:8:  is not an ABI-nnn or TODO(#n) cite
FAIL: k2.c:9: TODO(12) is not an ABI-nnn or TODO(#n) cite
FAIL: k2.c:10: abi-001 is not an ABI-nnn or TODO(#n) cite
FAIL: k2.c:11: ABI-00 is not an active entry of abi.md
FAIL: k2.c:12: TODO(#012) is not an ABI-nnn or TODO(#n) cite
FAIL: a row of k2.c cites nothing, an entry not active in abi.md, or a closed issue
EOF
want "cites of no known form" 1

# --- the macro in another form, no row ---------------------------------------
cat > k3.c <<'EOF'
KNOWN("ABI-001", a),
KNOWN(ABI_001, b),
  KNOWN ("ABI-001", c),
KNOWN(
    "ABI-001", d),
KNOWN("ABI-001
KNOWN	("ABI-001", e),
KNOWN(x"ABI-001", f),
EOF
run k3.c abi.md "$REPO"
cat > want <<'EOF'
error: k3.c:2: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN(ABI_001, b),
error: k3.c:3: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN ("ABI-001", c),
error: k3.c:4: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN(
error: k3.c:6: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN("ABI-001
error: k3.c:7: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN	("ABI-001", e),
error: k3.c:8: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN(x"ABI-001", f),
EOF
want "the macro in another form" 1

# The cite is one string literal, then `,` or `)`: C joins a literal
# after it, and a macro after it can add more, so such a row is
# malformed. Blanks before the `,` and a `)` straight after are fine.
cat > k3f.c <<'EOF'
KNOWN("ABI-001", a),
KNOWN("ABI-001" "0", b),
KNOWN("ABI-001"/**/"0", c),
KNOWN("ABI-001"
  "2", d),
KNOWN("ABI-001" SUFFIX, e),
KNOWN("ABI-001"),
KNOWN("ABI-001"	, g),
EOF
run k3f.c abi.md "$REPO"
cat > want <<'EOF'
error: k3f.c:2: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN("ABI-001" "0", b),
error: k3f.c:3: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN("ABI-001"/**/"0", c),
error: k3f.c:4: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN("ABI-001"
error: k3f.c:6: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN("ABI-001" SUFFIX, e),
EOF
want "a cite joined with what follows it" 1

cat > k4.c <<'EOF'
#define KNOWN(cite, predicate) {cite, predicate}
static const struct known KNOWN_CLASSES[] = {
    /* each row: KNOWN with a cite */
    KNOWNS("ABI-001", b),
    UNKNOWN("ABI-001", c), MY_KNOWN ("ABI-001"), KNOWN_X("ABI-001"),
    KNOWN[1], KNOWN {0}, x = KNOWN[2],
#define KNOWN_ALIAS 1
#define KNOWN_Y_ALIAS MY_KNOWN_Y
};
EOF
run k4.c abi.md "$REPO"
echo 'error: k4.c has no KNOWN("<cite>", ...) row' > want
want "a table with no row" 1

# Any use of the macro but a row at a line's start and the macro a
# #define defines fails: a second row on a line, a row after other
# text, behind a comment opener, or in another macro's definition.
cat > k3b.c <<'EOF'
#  define KNOWN(cite, p) {cite, p}
#define OTHER(c) KNOWN(c, x)
    KNOWN("ABI-001", a),
    KNOWN("ABI-001", a), KNOWN("ABI-002", d),
    {0}, KNOWN("ABI-002", e),
    // KNOWN("ABI-002", b),
    /* KNOWN ("ABI-002", f) */
    KNOWN("ABI-001", g), /* was KNOWN("ABI-002", g) */
EOF
run k3b.c abi.md "$REPO"
cat > want <<'EOF'
error: k3b.c:2: KNOWN used other than as a row at its line's start (one row per line): #define OTHER(c) KNOWN(c, x)
error: k3b.c:4: KNOWN used other than as a row at its line's start (one row per line): KNOWN("ABI-001", a), KNOWN("ABI-002", d),
error: k3b.c:5: KNOWN used other than as a row at its line's start (one row per line): {0}, KNOWN("ABI-002", e),
error: k3b.c:6: KNOWN used other than as a row at its line's start (one row per line): // KNOWN("ABI-002", b),
error: k3b.c:7: KNOWN used other than as a row at its line's start (one row per line): /* KNOWN ("ABI-002", f) */
error: k3b.c:8: KNOWN used other than as a row at its line's start (one row per line): KNOWN("ABI-001", g), /* was KNOWN("ABI-002", g) */
EOF
want "other uses of the macro" 1

# What the C preprocessor still reads as an invocation: the macro alone
# on its line, before a `\`, or before a comment, with the `(` later;
# and an alias #define, whose rows this would not read. An #undef of
# the macro, and its name before other text, are no use.
cat > k3c.c <<'EOF'
KNOWN("ABI-001", a),
KNOWN
  ("ABI-002", b),
KNOWN /* c */ ("ABI-002", c),
KNOWN // d
KNOWN \
  ("ABI-002", e),
#define K KNOWN
K("ABI-002", f),
#define K2(c) KNOWN
#define K3 KNOWN + 1
#ifdef KNOWN
#undef KNOWN
/* each row: KNOWN with a cite; KNOWN. */
EOF
run k3c.c abi.md "$REPO"
cat > want <<'EOF'
error: k3c.c:2: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN
error: k3c.c:4: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN /* c */ ("ABI-002", c),
error: k3c.c:5: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN // d
error: k3c.c:6: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN \
error: k3c.c:8: KNOWN used other than as a row at its line's start (one row per line): #define K KNOWN
error: k3c.c:10: KNOWN used other than as a row at its line's start (one row per line): #define K2(c) KNOWN
error: k3c.c:11: KNOWN used other than as a row at its line's start (one row per line): #define K3 KNOWN + 1
error: k3c.c:12: KNOWN used other than as a row at its line's start (one row per line): #ifdef KNOWN
EOF
want "the macro before a later (, and an alias" 1
: > k5.c
run k5.c abi.md "$REPO"
echo 'error: k5.c has no KNOWN("<cite>", ...) row' > want
want "an empty table" 1

# --- another macro ------------------------------------------------------------
cat > known.rs <<'EOF'
macro_rules! known { ($c:literal, $p:ident) => { ($c, $p) }; }
const CLASSES: &[Class] = &[
    known!("ABI-001", too_large),
    known!("ABI-002", gone),
    KNOWN("ABI-003", not_this_macro),
];
EOF
KNOWN_CITES_MACRO='known!' run known.rs abi.md "$REPO"
cat > want <<'EOF'
FAIL: known.rs:4: ABI-002 is not an active entry of abi.md
FAIL: a row of known.rs cites nothing, an entry not active in abi.md, or a closed issue
EOF
want "KNOWN_CITES_MACRO=known!" 1
printf 'known! ("ABI-001", a),\nknown!("ABI-001", b),\n' > k6.rs
KNOWN_CITES_MACRO='known!' run k6.rs abi.md "$REPO"
echo 'error: k6.rs:1: a known! line not of the form known!("<cite>", ...): known! ("ABI-001", a),' > want
want "KNOWN_CITES_MACRO=known!, a blank before (" 1
# A Rust macro takes [ and { as delimiters too: such a row, at a line's
# start or after other text, fails. Its name before other text is no
# use, nor is the macro_rules! definition.
cat > k6b.rs <<'EOF'
macro_rules! known { ($c:literal, $p:ident) => { ($c, $p) }; }
known!("ABI-001", a),
known!["ABI-002", b],
known!{"ABI-002", c},
    x(), known! ["ABI-002", d],
// each class: a known! row
EOF
KNOWN_CITES_MACRO='known!' run k6b.rs abi.md "$REPO"
cat > want <<'EOF'
error: k6b.rs:3: a known! line not of the form known!("<cite>", ...): known!["ABI-002", b],
error: k6b.rs:4: a known! line not of the form known!("<cite>", ...): known!{"ABI-002", c},
error: k6b.rs:5: known! used other than as a row at its line's start (one row per line): x(), known! ["ABI-002", d],
EOF
want "KNOWN_CITES_MACRO=known!, [ and { delimiters" 1
# Rust reads blanks between the name and its `!`, and between the `!`
# and the delimiter, and a comment or a line end there: each such use
# fails. The name alone (a variable, a word in a comment), a longer
# name and the `!` before other text are no use.
cat > k6c.rs <<'EOF'
known!("ABI-001", a),
known !("ABI-002", b),
    x, known ! ["ABI-002", c],
known! /* c */ ("ABI-002", d),
known!
    ("ABI-002", e),
// a row is known; known! rows cite
let known = 1; known_x!(); unknown!("x");
pub(crate) use known as k;
use crate::{known as k2, other};
use crate::known;
// it is known assuredly
EOF
KNOWN_CITES_MACRO='known!' run k6c.rs abi.md "$REPO"
cat > want <<'EOF'
error: k6c.rs:2: a known! line not of the form known!("<cite>", ...): known !("ABI-002", b),
error: k6c.rs:3: known! used other than as a row at its line's start (one row per line): x, known ! ["ABI-002", c],
error: k6c.rs:4: a known! line not of the form known!("<cite>", ...): known! /* c */ ("ABI-002", d),
error: k6c.rs:5: a known! line not of the form known!("<cite>", ...): known!
error: k6c.rs:9: known! used other than as a row at its line's start (one row per line): pub(crate) use known as k;
error: k6c.rs:10: known! used other than as a row at its line's start (one row per line): use crate::{known as k2, other};
EOF
want "KNOWN_CITES_MACRO=known!, blanks, comments and line ends around the !" 1

# CRLF line ends: a carriage return is a blank (as a form feed is),
# one is still a use, and a CRLF table reads as an LF one.
printf 'KNOWN("ABI-001", a),\r\nKNOWN\r\n  ("ABI-002", b),\r\nKNOWN\f("ABI-002", c),\r\n' > k3d.c
run k3d.c abi.md "$REPO"
printf 'error: k3d.c:2: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN \nerror: k3d.c:4: a KNOWN line not of the form KNOWN("<cite>", ...): KNOWN\f("ABI-002", c), \n' > want
want "CRLF: the macro alone before a carriage return" 1
printf '/* t */\r\n    KNOWN("ABI-001", a),\r\n\tKNOWN("ABI-004", b),\r\n' > k3e.c
run k3e.c abi.md "$REPO"
{ notes k3e.c; echo "PASS: 2 rows of k3e.c, each citing an active entry of abi.md or an open issue"; } > want
want "CRLF: a table of rows" 0
for m in '' '1known' 'kn own' 'kn!own' 'known!!' 'a.b' 'K*'; do
  KNOWN_CITES_MACRO=$m run known.c abi.md "$REPO"
  if [ -z "$m" ]; then
    # Empty is unset: the default macro.
    { notes known.c; echo "PASS: 2 rows of known.c, each citing an active entry of abi.md or an open issue"; } > want
    want "KNOWN_CITES_MACRO empty is the default" 0
    continue
  fi
  echo "error: KNOWN_CITES_MACRO '$m' is not a macro name (KNOWN, or known! in Rust)" > want
  want "KNOWN_CITES_MACRO='$m'" 2
done

# --- TODO(#n) against the stand-in gh -------------------------------------------
cat > k7.c <<'EOF'
KNOWN("TODO(#12)", a),
KNOWN("TODO(#13)", b),
KNOWN("TODO(#14)", c),
KNOWN("TODO(#15)", d),
KNOWN("TODO(#14)", e),
KNOWN("TODO(#16)", f),
EOF
printf '12 OPEN\n13 CLOSED\n15 MERGED\n' > gh-states
: > gh-calls
run k7.c abi.md "$REPO"
cat > want <<'EOF'
FAIL: k7.c:2: TODO(#13): issue #13 of o/port is CLOSED
FAIL: k7.c:3: TODO(#14): gh reaches o/port but cannot view issue #14: GraphQL: Could not resolve to an issue or pull request with the number of 14. (repository.issue)
FAIL: k7.c:4: TODO(#15): issue #15 of o/port is MERGED
FAIL: k7.c:5: TODO(#14): gh reaches o/port but cannot view issue #14: GraphQL: Could not resolve to an issue or pull request with the number of 14. (repository.issue)
FAIL: k7.c:6: TODO(#16): gh reaches o/port but cannot view issue #16: GraphQL: Could not resolve to an issue or pull request with the number of 16. (repository.issue)
FAIL: a row of k7.c cites nothing, an entry not active in abi.md, or a closed issue
EOF
want "TODO cites: open, closed, merged, no such issue" 1
cat > want <<'EOF'
repo view o/port --json name
issue view 12 -R o/port --json state -q .state
issue view 13 -R o/port --json state -q .state
issue view 14 -R o/port --json state -q .state
issue view 15 -R o/port --json state -q .state
issue view 14 -R o/port --json state -q .state
issue view 16 -R o/port --json state -q .state
EOF
cmp -s want gh-calls || { cat gh-calls >&2; fail "gh asked about each issue of the first repository"; }
echo "ok gh asked about each issue of the first repository"
# Where gh cannot reach the repository (asked once), each TODO is
# unverified, the run fails naming each once, and no issue is asked about.
rm gh-reach
printf 'KNOWN("TODO(#12)", a),\nKNOWN("TODO(#13)", b),\nKNOWN("TODO(#16)", f),\nKNOWN("TODO(#13)", e),\n' > k8.c
: > gh-calls
run k8.c abi.md "$REPO"
cat > want <<'EOF'
FAIL: the live state of #12 #13 #16 was not checked: gh is missing, or it cannot reach o/port; give the check gh with a credential for it, or set KNOWN_CITES_OFFLINE=1 for a run without
FAIL: a row of k8.c cites nothing, an entry not active in abi.md, or a closed issue
EOF
want "an unreachable repository fails, its issues named once" 1
echo 'repo view o/port --json name' > want
cmp -s want gh-calls || { cat gh-calls >&2; fail "gh asked only whether it reaches the repository"; }
echo "ok gh asked only whether it reaches the repository"
# KNOWN_CITES_OFFLINE=1 (a local run without gh) makes it a NOTE, and
# the PASS line does not call the issues open.
KNOWN_CITES_OFFLINE=1 run k8.c abi.md "$REPO"
{
  notes k8.c
  echo "NOTE: KNOWN_CITES_OFFLINE=1: the live state of #12 #13 #16 was not checked"
  echo "PASS: 4 rows of k8.c, each citing an active entry of abi.md or an issue not found closed"
} > want
want "KNOWN_CITES_OFFLINE=1: an unreachable repository is a NOTE" 0
for o in 0 yes ' 1'; do
  KNOWN_CITES_OFFLINE=$o run k8.c abi.md "$REPO"
  echo "error: KNOWN_CITES_OFFLINE is '$o': set it to 1 for a run without gh, or leave it empty" > want
  want "KNOWN_CITES_OFFLINE='$o'" 2
done
echo o/port > gh-reach

# --- the pull request's description ----------------------------------------------
printf 'KNOWN("ABI-001", a),\nKNOWN("TODO(#123)", b),\n' > k9.c
printf '123 OPEN\n' > gh-states
# closes LINE: the description LINE closes #123.
closes() {
  KNOWN_CITES_PR_BODY=$(printf 'A change.\r\n\r\n%s\r\n' "$1") run k9.c abi.md "$REPO"
  cat > want <<'EOF'
FAIL: k9.c:2: TODO(#123): this pull request's description closes #123: delete the row (and its tests)
FAIL: a row of k9.c cites nothing, an entry not active in abi.md, or a closed issue
EOF
  want "the description closes #123: $1" 1
}
# keeps LINE: it does not.
keeps() {
  KNOWN_CITES_PR_BODY=$(printf 'A change.\n\n%s\n' "$1") run k9.c abi.md "$REPO"
  {
    echo "NOTE: k9.c is not in a full git clone; commits closing an issue were not checked"
    echo "PASS: 2 rows of k9.c, each citing an active entry of abi.md or an open issue"
  } > want
  want "the description keeps #123: $1" 0
}
closes 'Fixes #123'
closes 'closes #123.'
closes 'Resolved: #123'
closes 'FIXED #123'
closes 'close #123'
closes 'Fixes o/port#123'
closes 'fixes https://github.com/o/port/issues/123'
closes 'Fixes old/name.x#123'
closes 'resolves https://github.com/old/name.x/issues/123)'
closes 'This (fixes #123)'
keeps 'Fixes #1234'
keeps 'Fixes #12'
keeps 'Refs #123'
keeps 'prefixes #123'
keeps 'Fixes other/port#123'
keeps 'Fixes old/nameXx#123'
keeps 'fixes https://github.com/o/port/pull/123'
keeps 'Fixes#123'
keeps ''
KNOWN_CITES_PR_BODY='' run k9.c abi.md "$REPO"
{
  echo "NOTE: k9.c is not in a full git clone; commits closing an issue were not checked"
  echo "PASS: 2 rows of k9.c, each citing an active entry of abi.md or an open issue"
} > want
want "an empty description is bound" 0

# --- the history ---------------------------------------------------------------
g() { git -C repo -c user.name=t -c user.email=t@example.com -c init.defaultBranch=main "$@"; }
mkdir repo repo/fuzz
git -c init.defaultBranch=main init -q repo
printf 'KNOWN("ABI-001", a),\n' > repo/fuzz/known.c
g add fuzz/known.c
g commit -q -m 'add the table' -m 'Fixes #123 (before the row: not counted)'
printf 'KNOWN("ABI-001", a),\n    KNOWN("TODO(#123)", b),\n' > repo/fuzz/known.c
g commit -q -a -m 'a row for #123'
echo x > repo/other
g add other
g commit -q -m 'unrelated' -m 'Refs #123'
run repo/fuzz/known.c abi.md "$REPO"
{
  echo "NOTE: no pull request description bound (KNOWN_CITES_PR_BODY unset); its closing keywords were not checked"
  echo "PASS: 2 rows of repo/fuzz/known.c, each citing an active entry of abi.md or an open issue"
} > want
want "the history: no commit closes #123 since the row" 0
g commit -q --allow-empty -m 'fix the thing' -m 'Closes o/port#123'
fix=$(g log -n 1 --format=%h)
row=$(g log -n 1 --format=%h HEAD~2)
run repo/fuzz/known.c abi.md "$REPO"
cat > want <<EOF
FAIL: repo/fuzz/known.c:2: TODO(#123): commit $fix fix the thing closes #123 after $row, since which repo/fuzz/known.c has cited it
FAIL: a row of repo/fuzz/known.c cites nothing, an entry not active in abi.md, or a closed issue
EOF
want "the history: a commit closes #123 since the row" 1
# Only the quoted cite marks the row's change: a later comment naming
# TODO(#123) bare does not move "since" past the fix.
printf '/* TODO(#123) is the row below */\nKNOWN("ABI-001", a),\nKNOWN("TODO(#123)", b),\n' > repo/fuzz/known.c
g commit -q -a -m 'a comment'
run repo/fuzz/known.c abi.md "$REPO"
cat > want <<EOF
FAIL: repo/fuzz/known.c:3: TODO(#123): commit $fix fix the thing closes #123 after $row, since which repo/fuzz/known.c has cited it
FAIL: a row of repo/fuzz/known.c cites nothing, an entry not active in abi.md, or a closed issue
EOF
want "the history: a bare mention of the cite is not the row's change" 1
# A second row with the same cite after the fix, and a comment quoting
# the cite, do not move the cutoff: the file has held a row of it
# without a gap since the first row.
printf '/* "TODO(#123)" is the row below */\nKNOWN("ABI-001", a),\nKNOWN("TODO(#123)", b),\nKNOWN("TODO(#123)", c),\n' > repo/fuzz/known.c
g commit -q -a -m 'a second row'
run repo/fuzz/known.c abi.md "$REPO"
cat > want <<EOF
FAIL: repo/fuzz/known.c:3: TODO(#123): commit $fix fix the thing closes #123 after $row, since which repo/fuzz/known.c has cited it
FAIL: repo/fuzz/known.c:4: TODO(#123): commit $fix fix the thing closes #123 after $row, since which repo/fuzz/known.c has cited it
FAIL: a row of repo/fuzz/known.c cites nothing, an entry not active in abi.md, or a closed issue
EOF
want "the history: a second row and a quoted comment do not move the cutoff" 1
# Rows dropped and added again after the fix: the cutoff is the
# re-adding commit, and the fix before it is not counted. A comment
# still quoting the cite between them is no row.
printf '/* "TODO(#123)" is gone */\nKNOWN("ABI-001", a),\n' > repo/fuzz/known.c
g commit -q -a -m 'drop the rows'
printf 'KNOWN("ABI-001", a),\nKNOWN("TODO(#123)", b),\n' > repo/fuzz/known.c
g commit -q -a -m 'the row again'
run repo/fuzz/known.c abi.md "$REPO"
{
  echo "NOTE: no pull request description bound (KNOWN_CITES_PR_BODY unset); its closing keywords were not checked"
  echo "PASS: 2 rows of repo/fuzz/known.c, each citing an active entry of abi.md or an open issue"
} > want
want "the history: a gap moves the cutoff past the fix" 0
# The file deleted and added again: absent at the deleting commit, so
# the cutoff is the re-adding one, and a fix before the deletion is not
# counted.
g commit -q --allow-empty -m 'another fix' -m 'Closes #123'
g rm -q fuzz/known.c
g commit -q -m 'drop the table'
mkdir -p repo/fuzz
printf 'KNOWN("ABI-001", a),\nKNOWN("TODO(#123)", b),\n' > repo/fuzz/known.c
g add fuzz/known.c
g commit -q -m 'the table again'
run repo/fuzz/known.c abi.md "$REPO"
want "the history: a deleted file ends the cutoff's run" 0
# A shallow clone skips the history.
git clone -q --depth 1 "file://$tmp/repo" shallow
run shallow/fuzz/known.c abi.md "$REPO"
{ notes shallow/fuzz/known.c; echo "PASS: 2 rows of shallow/fuzz/known.c, each citing an active entry of abi.md or an open issue"; } > want
want "a shallow clone skips the history" 0
# A snapshot git cannot read (its blob lost) while the history queries
# succeed is an error, not the row's absence: read as absent it would
# move the cutoff past the fix.
mkdir repo2 repo2/fuzz
git -c init.defaultBranch=main init -q repo2
g2() { git -C repo2 -c user.name=t -c user.email=t@example.com "$@"; }
printf 'KNOWN("TODO(#123)", b),\n/* v1 */\n' > repo2/fuzz/known.c
g2 add fuzz/known.c
g2 commit -q -m 'the row'
g2 commit -q --allow-empty -m 'the fix' -m 'Fixes #123'
printf 'KNOWN("TODO(#123)", b),\n/* v2 */\n' > repo2/fuzz/known.c
g2 commit -q -a -m 'v2'
first=$(g2 rev-parse --short HEAD~2)
blob=$(g2 rev-parse HEAD~2:fuzz/known.c)
rm "repo2/.git/objects/$(printf '%.2s' "$blob")/${blob#??}"
run repo2/fuzz/known.c abi.md "$REPO"
# git's own message (its wording is git's) comes first, then this one.
if [ "$rc" -ne 1 ] || [ "$(tail -n 1 out)" != "error: git cannot read repo2/fuzz/known.c at $first" ] || ! grep -q '^fatal: ' out; then
  cat out >&2
  fail "the history: an unreadable snapshot is an error"
fi
echo "ok the history: an unreadable snapshot is an error"

# --- a malformed ledger, names with = --------------------------------------------
printf '### ABI-001: x\n\n- **Status:** active\n- **Status:** removed\n' > bad.md
run known.c bad.md "$REPO"
cat > want <<'EOF'
error: ledger: bad.md: ABI-001 has 2 Status lines; want one
error: the ABI ledger bad.md is malformed; no cite was checked
EOF
want "a malformed ledger" 1
cp known.c 'k=v.c'
cp abi.md 'a=b.md'
run 'k=v.c' 'a=b.md' "$REPO"
{ notes 'k=v.c'; echo "PASS: 2 rows of k=v.c, each citing an active entry of a=b.md or an open issue"; } > want
want "names with =" 0

# --- missing inputs and usage -------------------------------------------------------
run no.c abi.md "$REPO"
echo "error: no table source no.c" > want
want "a missing table" 1
run known.c no-abi "$REPO"
echo "error: no ABI ledger no-abi" > want
want "a missing ledger" 1
echo "usage: known-cites.sh FILE LEDGER 'OWNER/NAME [OLD/NAME...]'" > want
run known.c abi.md
want "two arguments" 2
run known.c abi.md o/port extra
want "four arguments" 2
run known.c abi.md ' '
echo "error: no repository: name the port's issues' OWNER/NAME" > want
want "a blank repository" 2
# o2/* names a file once globbed (o2/x): it stays itself.
mkdir -p o2/x
for r in port o/p/x /port o/ 'o/p;x' 'o/port bad' 'o2/*'; do
  run known.c abi.md "$r"
  b=${r##* }
  [ "$r" = 'o/port bad' ] || b=$r
  echo "error: '$b' is not an OWNER/NAME repository" > want
  want "the repository '$r'" 2
done

# --- ledger-active.sh on its own ---------------------------------------------------
# lrun ARGS...: ledger-active.sh with ARGS, as run does.
lrun() {
  if sh "$la" "$@" > out 2>&1; then rc=0; else rc=$?; fi
}
lrun abi.md
printf 'ABI-001\nABI-004\n' > want
want "ledger-active.sh: the active IDs of a ledger file" 0
lrun abi
printf 'ABI-001\nABI-004\n' > want
want "ledger-active.sh: the active IDs of a ledger directory" 0
lrun bad.md
echo "error: ledger: bad.md: ABI-001 has 2 Status lines; want one" > want
want "ledger-active.sh: a malformed ledger" 1
lrun no-abi
echo "error: no ABI ledger no-abi" > want
want "ledger-active.sh: a missing ledger" 3
lrun
echo "usage: ledger-active.sh LEDGER" > want
want "ledger-active.sh: no argument" 2

echo "PASS: known-cites.sh"
