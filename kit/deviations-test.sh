#!/bin/sh
# kit/deviations-test.sh: tests of kit/make/deviations.sh (the
# deviations-test check). Each case writes a results file, a deviations
# list and an ABI ledger by hand, runs the script, and checks its exit
# status and its whole output, spelled out by hand (none is taken from a
# run of the script under test): a list that matches (one ledger file,
# a ledger directory, an empty list with no failure), a new failure, a
# listed key that passes, one that is not a key of the run, each kind
# of unexplained line (no reason, a reason not starting with ABI-nnn:,
# nothing after it, an entry that is removed, planned or missing, a key
# listed twice), the Status forms that count as active, more than 50 of
# a kind, malformed results (another word, a key twice, a third field,
# none), --update (drops passing and stale lines, keeps comments, blank
# lines and failing lines, never adds a line), DEVIATIONS_OUT's files,
# names with `|`, `&` and a backslash, and usage. Run it from the
# repository root. It needs a POSIX shell and BusyBox's tools.
set -eu

script=$(pwd)/kit/make/deviations.sh
[ -f "$script" ] || { echo "FAIL: run it from the repository root (no kit/make/deviations.sh)" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

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

# The ledger as one file: ABI-001 and ABI-004 active (one with a note
# after the word), ABI-002 removed, ABI-003 planned, and a template
# block, fenced as the kit template's is, whose Status line names every
# value under a heading that is no entry.
cat > abi.md <<'EOF'
# Known differences

## Entry format

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

- **Status:** active, narrowed (2026-11-02). Until then more.
EOF

# The same ledger as a directory of entry files (the kit template's
# docs/abi/), with a README that is no entry: only ABI-*.md files are
# read, so the ABI-003 example the README holds is not an entry.
mkdir abi
printf '# Known differences\n\n### ABI-003: an example\n\n- **Status:** active\n' > abi/README.md
printf '### ABI-001: limits\n\n- **Area:** match\n- **Status:** active\n' > abi/ABI-001.md
printf '### ABI-002: old\n\n- **Status:** removed (2026-10-23, PR #149)\n' > abi/ABI-002.md
printf '### ABI-004: sizes\n\n- **Status:** active (renamed 2026-11-14)\n' > abi/ABI-004.md

printf 'PASS t1:1\nFAIL t1:5\nPASS t1:9\nFAIL t2:3\nPASS t2:8\n' > res
cat > list <<'EOF'
# the deviations list
t1:5 ABI-001: the limit trips later

t2:3 ABI-004: a size differs
EOF
cp list list.orig

# --- matching lists -----------------------------------------------------
run res list abi.md
cat > want <<'EOF'
total: 3/5 pass
PASS: the failing keys are exactly the 2 listed in list, each with its reason
EOF
want "a list that matches, the ledger one file" 0

run res list abi
want "a list that matches, the ledger a directory" 0

# Tab-separated results read, and count, like blank-separated ones.
printf 'PASS\tt1:1\nFAIL\tt1:5\nPASS t1:9\nFAIL\tt2:3\nPASS t2:8\n' > restab
run restab list abi.md
cat > want <<'EOF'
total: 3/5 pass
PASS: the failing keys are exactly the 2 listed in list, each with its reason
EOF
want "tab-separated results" 0

printf 'PASS a\nPASS b\n' > res0
printf '# nothing yet\n\n' > list0
run res0 list0 abi.md
cat > want <<'EOF'
total: 2/2 pass
PASS: the failing keys are exactly the 0 listed in list0, each with its reason
EOF
want "no failure and an empty list" 0

# Blanks and tabs around fields, an indented comment and a line of blanks.
printf '  # indented comment\n \t \nt1:5\tABI-001:   later  \n   t2:3 ABI-004: size\n' > list1
run res list1 abi.md
cat > want <<'EOF'
total: 3/5 pass
PASS: the failing keys are exactly the 2 listed in list1, each with its reason
EOF
want "blanks around fields, an indented comment, a line of blanks" 0

# --- regressions and stale lines ----------------------------------------
printf 'PASS t1:1\nFAIL t1:5\nFAIL t1:9\nFAIL t2:3\nFAIL t2:8\n' > res1
run res1 list abi.md
cat > want <<'EOF'
total: 1/5 pass
FAIL: t1:9 fails and is not in list
FAIL: t2:8 fails and is not in list
FAIL: 2 failing keys are not in list (a regression: fix it, do not list it)
EOF
want "two new failures, in results order" 1

printf 'PASS t1:1\nPASS t1:5\nPASS t1:9\nFAIL t2:3\nPASS t2:8\n' > res2
run res2 list abi.md
cat > want <<'EOF'
total: 4/5 pass
FAIL: listed in list but passes: t1:5
FAIL: 1 listed keys pass and 0 are not keys of this run; run the ratchet with --update to drop them from list
EOF
want "a listed key that passes" 1

printf 'PASS t1:1\nPASS t1:9\nFAIL t2:3\nPASS t2:8\n' > res3
run res3 list abi.md
cat > want <<'EOF'
total: 3/4 pass
FAIL: listed in list but not a key of this run: t1:5
FAIL: 0 listed keys pass and 1 are not keys of this run; run the ratchet with --update to drop them from list
EOF
want "a listed key that is not a key of the run" 1

# All three at once, in the script's order: passes, stale, then new.
printf 'PASS t1:1\nPASS t1:9\nPASS t2:3\nFAIL t2:8\n' > res4
run res4 list abi.md
cat > want <<'EOF'
total: 3/4 pass
FAIL: listed in list but passes: t2:3
FAIL: listed in list but not a key of this run: t1:5
FAIL: 1 listed keys pass and 1 are not keys of this run; run the ratchet with --update to drop them from list
FAIL: t2:8 fails and is not in list
FAIL: 1 failing keys are not in list (a regression: fix it, do not list it)
EOF
want "a passing, a stale and a new key together" 1

# --- unexplained lines ----------------------------------------------------
printf 'FAIL k1\nFAIL k2\nFAIL k3\nFAIL k4\nFAIL k5\nFAIL k6\nFAIL k7\nFAIL k8\nFAIL k9\n' > resu
cat > listu <<'EOF'
k1 ABI-001: fine
k2
k3 TODO(#5): untriaged
k4 ABI-001:
k5 ABI-002: a removed entry
k6 ABI-003: a planned entry
k7 ABI-009: no such entry
k8 ABI-004 a missing colon
k9 ABI-004: fine
k1 ABI-004: listed again
EOF
run resu listu abi.md
cat > want <<'EOF'
total: 0/9 pass
FAIL: unexplained deviation: k2: no reason
FAIL: unexplained deviation: k3: the reason does not start with ABI-nnn:
FAIL: unexplained deviation: k4: the reason says nothing after ABI-001:
FAIL: unexplained deviation: k5: ABI-002 is not an active entry of abi.md
FAIL: unexplained deviation: k6: ABI-003 is not an active entry of abi.md
FAIL: unexplained deviation: k7: ABI-009 is not an active entry of abi.md
FAIL: unexplained deviation: k8: the reason does not start with ABI-nnn:
FAIL: unexplained deviation: k1: listed twice
EOF
want "each kind of unexplained line, the ledger one file" 1

run resu listu abi
cat > want <<'EOF'
total: 0/9 pass
FAIL: unexplained deviation: k2: no reason
FAIL: unexplained deviation: k3: the reason does not start with ABI-nnn:
FAIL: unexplained deviation: k4: the reason says nothing after ABI-001:
FAIL: unexplained deviation: k5: ABI-002 is not an active entry of abi
FAIL: unexplained deviation: k6: ABI-003 is not an active entry of abi
FAIL: unexplained deviation: k7: ABI-009 is not an active entry of abi
FAIL: unexplained deviation: k8: the reason does not start with ABI-nnn:
FAIL: unexplained deviation: k1: listed twice
EOF
want "each kind of unexplained line, the ledger a directory" 1

# Status forms: `active` must be the whole word, on its entry's one
# Status line outside fenced code (not another heading's).
cat > abi2.md <<'EOF'
### ABI-001: actively is not active

- **Status:** actively planned

### ABI-003: its own status

#pragma, a line that is no heading
`code` and ``more`` at a line's start, no fence
`` a run of two backticks with none after it, no fence either

---

- **Status:** active

## ABI-004: not an entry heading level

### ABI-005 no colon

### ABI-006: inactive

- **Status:** inactive

## Notes: a heading that ends ABI-006

### ABI-008: a code sample's #include is no heading

A sample, then a thematic break right after it:
```c
#include <foo.h>
- **Status:** removed
```
---

- **Status:** active

### ABI-009: a removed entry quoting an active one

- **Status:** removed (2026-10-23, PR #1)
- **Patch recipe:**

  ```markdown
  ### ABI-010: quoted
  - **Status:** active
  ```

~~~
- **Status:** active
~~~

### ABI-012: a four-backtick fence quoting a three-backtick one

````markdown
```
- **Status:** active
```
````

- **Status:** removed (2026-10-23, PR #1)

### ABI-013: a fence quoting an info-string fence line and a tilde run

```
```markdown
~~~
- **Status:** active
``` and more
```

- **Status:** removed (2026-10-23, PR #1)

### ABI-014: inline code of three backticks opens no fence

```inline``` at a line's start
- **Status:** removed (2026-10-23, PR #1)

```
- **Status:** active
```
EOF
printf 'FAIL a\nFAIL c\nFAIL d\nFAIL e\nFAIL f\nFAIL h\nFAIL i\nFAIL j\nFAIL k\nFAIL l\nFAIL m\n' > ress
printf 'a ABI-001: x\nc ABI-003: x\nd ABI-004: x\ne ABI-005: x\nf ABI-006: x\nh ABI-008: x\ni ABI-009: x\nj ABI-010: x\nk ABI-012: x\nl ABI-013: x\nm ABI-014: x\n' > lists
run ress lists abi2.md
cat > want <<'EOF'
total: 0/11 pass
FAIL: unexplained deviation: a: ABI-001 is not an active entry of abi2.md
FAIL: unexplained deviation: d: ABI-004 is not an active entry of abi2.md
FAIL: unexplained deviation: e: ABI-005 is not an active entry of abi2.md
FAIL: unexplained deviation: f: ABI-006 is not an active entry of abi2.md
FAIL: unexplained deviation: i: ABI-009 is not an active entry of abi2.md
FAIL: unexplained deviation: j: ABI-010 is not an active entry of abi2.md
FAIL: unexplained deviation: k: ABI-012 is not an active entry of abi2.md
FAIL: unexplained deviation: l: ABI-013 is not an active entry of abi2.md
FAIL: unexplained deviation: m: ABI-014 is not an active entry of abi2.md
EOF
want "Status forms that are not active, and fenced code" 1

# An entry holds exactly one Status line: none (the next heading, of
# any level, ends it first) or two (an old one left behind) is a
# malformed ledger.
cat > abi2b.md <<'EOF'
### ABI-002: the next heading ends an entry

### ABI-003: fine

- **Status:** active

### ABI-007: any heading ends an entry

## Notes

- **Status:** active

### ABI-011: an old Status line left behind

- **Status:** removed (2026-10-23, PR #1)
- **Status:** active

### ABI-015: a setext heading ends an entry

- **Area:** a list item, then a blank line

- **Surface:** a list item, then a fence
```c
code
```
A paragraph.
Notes
-----

- **Status:** active

### ABI-016: an indented ATX heading ends an entry

  ## Notes
- **Status:** active
### ABI-017: so does a setext heading of =, after a list item's heading
Notes
=====
- **Status:** active

## ABI-004: not an entry heading level

- **Status:** active

### ABI-005 no colon

- **Status:** active

### ABI-018: a --- under a list item is a thematic break

- **Status:** active
---
- **Status:** removed (2026-10-23, PR #1)

### ABI-019: so is one under a list item's lazy line or a quote

- **Area:** match
continued lazily
---
- **Surface:** inline code is no fence
```inline``` lazily
---

> a quote
---
- **Status:** active

### ABI-020: a blank line ends a list item, so a setext heading after it counts

- **Area:** match

Notes
-----

- **Status:** active
EOF
printf 'FAIL c\nFAIL s\n' > resb
printf 'c ABI-003: x\ns ABI-019: x\n' > listb
run resb listb abi2b.md
cat > want <<'EOF'
error: ledger: abi2b.md: ABI-002 has 0 Status lines; want one
error: ledger: abi2b.md: ABI-007 has 0 Status lines; want one
error: ledger: abi2b.md:11: a Status line outside any ABI-nnn entry
error: ledger: abi2b.md: ABI-011 has 2 Status lines; want one
error: ledger: abi2b.md: ABI-015 has 0 Status lines; want one
error: ledger: abi2b.md:30: a Status line outside any ABI-nnn entry
error: ledger: abi2b.md: ABI-016 has 0 Status lines; want one
error: ledger: abi2b.md:35: a Status line outside any ABI-nnn entry
error: ledger: abi2b.md: ABI-017 has 0 Status lines; want one
error: ledger: abi2b.md:39: a Status line outside any ABI-nnn entry
error: ledger: abi2b.md:43: a Status line outside any ABI-nnn entry
error: ledger: abi2b.md:47: a Status line outside any ABI-nnn entry
error: ledger: abi2b.md: ABI-018 has 2 Status lines; want one
error: ledger: abi2b.md: ABI-020 has 0 Status lines; want one
error: ledger: abi2b.md:75: a Status line outside any ABI-nnn entry
error: the ABI ledger abi2b.md is malformed; nothing was checked or updated
EOF
want "entries with no Status line or two" 1

# A fence never closed (here an indented one in a list item, which
# ends with its item when rendered) would hide the old Status line
# after it: the ledger is malformed.
cat > abi2c.md <<'EOF'
### ABI-001: x

- **Status:** active
- **Surface:** f
  ```c
  int f(void);
- **Status:** removed (2026-10-23, PR #1)
EOF
printf 'FAIL a\n' > resc
printf 'a ABI-001: x\n' > listc
run resc listc abi2c.md
cat > want <<'EOF'
error: ledger: abi2c.md:5: an indented code fence not closed before a line indented less
error: ledger: abi2c.md: ABI-001 has 2 Status lines; want one
error: the ABI ledger abi2c.md is malformed; nothing was checked or updated
EOF
want "an indented fence left open past its list item" 1

# The same, where a later indented fence would close the first one and
# hide the line between: it fails at that line, before the second.
cat > abi2d.md <<'EOF'
### ABI-001: x

- **Status:** active
- **Surface:** f
  ```c
  int f(void);
- **Status:** removed (2026-10-23, PR #1)
- **Recipe:**
  ```
  patch
EOF
run resc listc abi2d.md
cat > want <<'EOF'
error: ledger: abi2d.md:5: an indented code fence not closed before a line indented less
error: ledger: abi2d.md: ABI-001 has 2 Status lines; want one
error: ledger: abi2d.md:9: a code fence that is never closed
error: the ABI ledger abi2d.md is malformed; nothing was checked or updated
EOF
want "an indented fence a later fence would close" 1

# The same, where a later top-level fence, longer, would close it.
cat > abi2g.md <<'EOF'
### ABI-001: x

- **Status:** active
- **Surface:** f
  ```c
  int f(void);
- **Status:** removed (2026-10-23, PR #1)

````
```
````
EOF
run resc listc abi2g.md
cat > want <<'EOF'
error: ledger: abi2g.md:5: an indented code fence not closed before a line indented less
error: ledger: abi2g.md: ABI-001 has 2 Status lines; want one
error: the ABI ledger abi2g.md is malformed; nothing was checked or updated
EOF
want "an indented fence a later top-level fence would close" 1

# A closer indented otherwise than its opener: 4 spaces more is code
# in CommonMark (so a later real fence would reopen what renders as
# closed), and a tab is columns this reader does not count.
cat > abi2i.md <<'EOF'
### ABI-001: x

- **Status:** active

```
    ```
```
- **Status:** removed (2026-10-23, PR #1)
    ```
EOF
run resc listc abi2i.md
cat > want <<'EOF'
error: ledger: abi2i.md:6: a closing code fence indented otherwise than its opener (line 5)
error: ledger: abi2i.md:9: a closing code fence indented otherwise than its opener (line 7)
error: the ABI ledger abi2i.md is malformed; nothing was checked or updated
EOF
want "a closing fence indented 4 more than its opener" 1
# (The closer below is a tab, then three backticks.)
cat > abi2j.md <<'EOF'
### ABI-001: x

- **Status:** active

```
code
	```
EOF
run resc listc abi2j.md
cat > want <<'EOF'
error: ledger: abi2j.md:7: a closing code fence indented otherwise than its opener (line 5)
error: the ABI ledger abi2j.md is malformed; nothing was checked or updated
EOF
want "a tab-indented closing fence" 1

# A tab-indented opener: rendered, indented code (no fence), so the
# lines after it, indented 1 or 3, are visible Status lines.
cat > abi2k.md <<'EOF'
### ABI-001: x

- **Status:** active

	```
   - **Status:** removed (2026-10-23, PR #1)
 - **Status:** removed (2026-10-23, PR #1)
	```
EOF
run resc listc abi2k.md
cat > want <<'EOF'
error: ledger: abi2k.md:5: a code fence indented with a tab
error: the ABI ledger abi2k.md is malformed; nothing was checked or updated
EOF
want "a tab-indented opening fence" 1

# A top-level fence never closed hides every later line.
printf '### ABI-001: x\n\n- **Status:** active\n\n```\n- **Status:** removed (2026-10-23, PR #1)\n' > abi2e.md
run resc listc abi2e.md
cat > want <<'EOF'
error: ledger: abi2e.md:5: a code fence that is never closed
error: the ABI ledger abi2e.md is malformed; nothing was checked or updated
EOF
want "a fence that is never closed" 1

# A Status field in another form would go uncounted: malformed. Any
# line holding `**Status:**` outside a fence counts; inside one it is
# quoted text.
cat > abi2f.md <<'EOF'
### ABI-001: x

- **Status:** active
* **Status:** removed (2026-10-23, PR #1)
-   **Status:** removed
  - **Status:** removed

```
* **Status:** quoted, not counted
```
EOF
run resc listc abi2f.md
cat > want <<'EOF'
error: ledger: abi2f.md:4: a Status field not written `- **Status:**` at a line's start
error: ledger: abi2f.md:5: a Status field not written `- **Status:**` at a line's start
error: ledger: abi2f.md:6: a Status field not written `- **Status:**` at a line's start
error: the ABI ledger abi2f.md is malformed; nothing was checked or updated
EOF
want "a Status field in another form" 1

# Raw HTML at a line's start (a comment hides lines; in a <details>
# block a fence line opens nothing): malformed, outside a fence (inside
# one it is quoted text). An autolink at a line's start is no HTML.
cat > abi2h.md <<'EOF'
### ABI-001: x

- **Status:** removed (2026-10-23, PR #1)
<https://example.com/abi-001>

```html
<!-- quoted -->
<details>
```

  <!--
### ABI-002: retired

- **Status:** active
-->
EOF
printf 'FAIL b\n' > resh
printf 'b ABI-002: x\n' > listh
run resh listh abi2h.md
cat > want <<'EOF'
error: ledger: abi2h.md:11: raw HTML (rendered, its lines are not read as Markdown)
error: the ABI ledger abi2h.md is malformed; nothing was checked or updated
EOF
want "an HTML comment" 1

# A <details> block: rendered, the fence line inside it opens nothing,
# so the Status line after it is visible.
cat > abi2l.md <<'EOF'
### ABI-001: x

- **Status:** active

<details>
```
</details>

- **Status:** removed (2026-10-23, PR #1)
```
EOF
run resc listc abi2l.md
cat > want <<'EOF'
error: ledger: abi2l.md:5: raw HTML (rendered, its lines are not read as Markdown)
error: the ABI ledger abi2l.md is malformed; nothing was checked or updated
EOF
want "an HTML block" 1

# The same inside a list item, indented 4, and with a tab: refused at
# any indentation, as fence lines are read.
cat > abi2m.md <<'EOF'
### ABI-001: x

- **Status:** active
- **Recipe:**
    <details>
    ```
    </details>

    - **Status:** removed (2026-10-23, PR #1)
    ```
- **Notes:**
	<div>
EOF
run resc listc abi2m.md
cat > want <<'EOF'
error: ledger: abi2m.md:5: raw HTML (rendered, its lines are not read as Markdown)
error: ledger: abi2m.md:12: raw HTML (rendered, its lines are not read as Markdown)
error: the ABI ledger abi2m.md is malformed; nothing was checked or updated
EOF
want "an HTML block in a list item" 1

# An HTML block opening on a list or quote marker's line; an autolink
# there is no HTML.
cat > abi2n.md <<'EOF'
### ABI-001: x

- **Status:** active
- <details>
  ```
  </details>

  - **Status:** removed (2026-10-23, PR #1)
  ```
1. <div>
> <p>
- <https://example.com/abi-001>
EOF
run resc listc abi2n.md
cat > want <<'EOF'
error: ledger: abi2n.md:4: raw HTML (rendered, its lines are not read as Markdown)
error: ledger: abi2n.md:10: raw HTML (rendered, its lines are not read as Markdown)
error: ledger: abi2n.md:11: raw HTML (rendered, its lines are not read as Markdown)
error: the ABI ledger abi2n.md is malformed; nothing was checked or updated
EOF
want "an HTML block on a list or quote marker's line" 1

# A fence run anywhere but at a fence line's start: after a list or
# quote marker it opens a fence when rendered, hiding the heading the
# script would read.
cat > abi2o.md <<'EOF'
### ABI-001: x

- **Status:** removed (2026-10-23, PR #1)
- ```
  ### ABI-002: x
- **Status:** active
1. ~~~
> ```
Text, then ``` mid-line.
EOF
printf 'FAIL b\n' > reso
printf 'b ABI-002: x\n' > listo
run reso listo abi2o.md
cat > want <<'EOF'
error: ledger: abi2o.md:4: a code fence run on a line that is not a fence line
error: ledger: abi2o.md:7: a code fence run on a line that is not a fence line
error: ledger: abi2o.md:8: a code fence run on a line that is not a fence line
error: ledger: abi2o.md:9: a code fence run on a line that is not a fence line
error: the ABI ledger abi2o.md is malformed; nothing was checked or updated
EOF
want "a fence run after a list or quote marker, or mid-line" 1

# A directory entry's heading ends at its file's end: a Status line at
# the top of the next file is no entry's (not ABI-001's second), so
# the ledger is malformed there, and ABI-001 has its one Status line.
mkdir abi3
printf '### ABI-001: removed\n\n- **Status:** removed (2026-10-23, PR #1)\n' > abi3/ABI-001.md
printf -- '- **Status:** active\n### ABI-002: x\n\n- **Status:** active\n' > abi3/ABI-002.md
printf 'FAIL a\nFAIL b\n' > res5
printf 'a ABI-001: x\nb ABI-002: x\n' > list5
run res5 list5 abi3
cat > want <<'EOF'
error: ledger: abi3/ABI-002.md:1: a Status line outside any ABI-nnn entry
error: the ABI ledger abi3 is malformed; nothing was checked or updated
EOF
want "an entry ends at its file's end" 1

# --- malformed ledgers ------------------------------------------------------
# A copied heading: ABI-002.md heads itself ABI-001, an active copy of a
# removed entry; the list citing ABI-001 is not checked against it.
mkdir abi4
printf '### ABI-001: x\n\n- **Status:** removed (2026-10-23, PR #1)\n' > abi4/ABI-001.md
printf '### ABI-001: x\n\n- **Status:** active\n' > abi4/ABI-002.md
printf 'FAIL a\n' > res7
printf 'a ABI-001: x\n' > list7
cp list7 list7.orig
run res7 list7 abi4
cat > want <<'EOF'
error: ledger: abi4/ABI-002.md: holds 1 entry headings, the first ABI-001; want one, ### ABI-002:
error: ledger: abi4: ABI-001 has more than one entry heading
error: the ABI ledger abi4 is malformed; nothing was checked or updated
EOF
want "a directory entry headed with another ID" 1

# One file: a removed entry and an active entry with the same ID.
printf '### ABI-001: x\n\n- **Status:** removed (2026-10-23, PR #1)\n\n### ABI-001: x\n\n- **Status:** active\n' > abi5.md
run --update res7 list7 abi5.md
cat > want <<'EOF'
error: ledger: abi5.md: ABI-001 has more than one entry heading
error: the ABI ledger abi5.md is malformed; nothing was checked or updated
EOF
want "one file with an ID twice, under --update" 1
cmp -s list7.orig list7 || fail "--update changed the list with a malformed ledger"
echo "ok --update leaves the list alone with a malformed ledger"

# A directory: a file name that is no entry's, a file with two entry
# headings, a file with none.
mkdir abi6
printf '### ABI-001: x\n\n- **Status:** active\n' > abi6/ABI-001.md
printf '### ABI-003: x\n\n- **Status:** active\n' > abi6/ABI-notes.md
printf '### ABI-004: x\n\n- **Status:** active\n\n### ABI-005: y\n\n- **Status:** active\n' > abi6/ABI-004.md
printf '# ABI-006\n\n- **Status:** active\n' > abi6/ABI-006.md
run res7 list7 abi6
cat > want <<'EOF'
error: ledger: abi6/ABI-004.md: holds 2 entry headings, the first ABI-004; want one, ### ABI-004:
error: ledger: abi6/ABI-006.md:3: a Status line outside any ABI-nnn entry
error: ledger: abi6/ABI-006.md: holds 0 entry headings; want one, ### ABI-006:
error: ledger: abi6/ABI-notes.md: not an ABI-<digits>.md entry file name
error: the ABI ledger abi6 is malformed; nothing was checked or updated
EOF
want "a directory with a misnamed file, two headings and none" 1

# --- more than 50 of a kind -------------------------------------------------
: > res6
i=1
while [ "$i" -le 51 ]; do echo "FAIL n$i" >> res6; i=$((i + 1)); done
: > list6
run res6 list6 abi.md
{
  echo "total: 0/51 pass"
  i=1
  while [ "$i" -le 50 ]; do echo "FAIL: n$i fails and is not in list6"; i=$((i + 1)); done
  echo "FAIL: ... and 1 more"
  echo "FAIL: 51 failing keys are not in list6 (a regression: fix it, do not list it)"
} > want
want "51 new failures: 50 shown, then a count" 1

# --- DEVIATIONS_OUT -----------------------------------------------------------
mkdir o
printf 'stale from an earlier run\n' > o/fixed
printf 'stale from an earlier run\n' > o/stale
if DEVIATIONS_OUT=o sh "$script" res4 listu abi.md > out 2>&1; then rc=0; else rc=$?; fi
[ "$rc" -eq 1 ] || fail "DEVIATIONS_OUT run: got = exit $rc, want = exit 1"
printf 't2:8\n' > want
cmp -s want o/new-failures || { cat o/new-failures >&2; fail "DEVIATIONS_OUT new-failures"; }
: > want
cmp -s want o/fixed || { cat o/fixed >&2; fail "DEVIATIONS_OUT fixed: an earlier run's file is not emptied"; }
printf 'k1\nk2\nk3\nk4\nk5\nk6\nk7\nk8\nk9\n' > want
cmp -s want o/stale || { cat o/stale >&2; fail "DEVIATIONS_OUT stale (once each, list order)"; }
cat > want <<'EOF'
k2: no reason
k3: the reason does not start with ABI-nnn:
k4: the reason says nothing after ABI-001:
k5: ABI-002 is not an active entry of abi.md
k6: ABI-003 is not an active entry of abi.md
k7: ABI-009 is not an active entry of abi.md
k8: the reason does not start with ABI-nnn:
k1: listed twice
EOF
cmp -s want o/unexplained || { cat o/unexplained >&2; fail "DEVIATIONS_OUT unexplained"; }
echo "ok DEVIATIONS_OUT holds the full lists, an earlier run's emptied"

# --- --update -----------------------------------------------------------------
cat > listup <<'EOF'
# the deviations list

t1:5 ABI-001: the limit trips later
# t2 below
t2:3 ABI-004: a size differs
t9:1 ABI-004: gone from the run
t2:3 ABI-004: listed twice
EOF
chmod 640 listup
printf 'PASS t1:1\nPASS t1:5\nFAIL t2:3\nFAIL t2:8\n' > resup
run --update resup listup abi.md
cat > want <<'EOF'
updated listup
total: 2/4 pass
FAIL: unexplained deviation: t2:3: listed twice
FAIL: t2:8 fails and is not in listup
FAIL: 1 failing keys are not in listup (a regression: fix it, do not list it)
EOF
want "--update drops a passing and a stale line, then still fails" 1
cat > want <<'EOF'
# the deviations list

# t2 below
t2:3 ABI-004: a size differs
t2:3 ABI-004: listed twice
EOF
cmp -s want listup || { cat listup >&2; fail "--update: the list it wrote"; }
[ "$(stat -c %a listup)" = 640 ] || fail "--update: the list's mode changed: $(stat -c %a listup)"
echo "ok --update keeps comments, blank lines and failing lines, and the list's mode"

cp list.orig listok
run --update res2 listok abi.md
cat > want <<'EOF'
updated listok
total: 4/5 pass
PASS: the failing keys are exactly the 1 listed in listok, each with its reason
EOF
want "--update drops a fixed key and then passes" 0

cp list.orig listnew
run --update res1 listnew abi.md
cat > want <<'EOF'
updated listnew
total: 1/5 pass
FAIL: t1:9 fails and is not in listnew
FAIL: t2:8 fails and is not in listnew
FAIL: 2 failing keys are not in listnew (a regression: fix it, do not list it)
EOF
want "--update adds no line for a new failure" 1
cmp -s list.orig listnew || fail "--update changed a list it had nothing to drop from"
echo "ok --update leaves a list with nothing to drop as it was"

# --- malformed results ------------------------------------------------------
printf 'PASS a\nSKIP b\nFAIL c d\nFAIL\nPASS a\n\n' > resbad
run resbad list abi.md
cat > want <<'EOF'
error: resbad:2: not a `PASS KEY` or `FAIL KEY` line: SKIP b
error: resbad:3: not a `PASS KEY` or `FAIL KEY` line: FAIL c d
error: resbad:4: not a `PASS KEY` or `FAIL KEY` line: FAIL
error: resbad:5: a appears twice (first at line 1)
error: resbad:6: a blank line, not `PASS KEY` or `FAIL KEY`
EOF
want "malformed results" 1

: > resempty
run resempty list abi.md
echo "error: resempty is empty: the run wrote no results" > want
want "empty results" 1

cp list.orig listkeep
run --update resbad listkeep abi.md
cat > want <<'EOF'
error: resbad:2: not a `PASS KEY` or `FAIL KEY` line: SKIP b
error: resbad:3: not a `PASS KEY` or `FAIL KEY` line: FAIL c d
error: resbad:4: not a `PASS KEY` or `FAIL KEY` line: FAIL
error: resbad:5: a appears twice (first at line 1)
error: resbad:6: a blank line, not `PASS KEY` or `FAIL KEY`
EOF
want "--update on malformed results" 1
cmp -s list.orig listkeep || fail "--update rewrote the list from malformed results"
echo "ok --update leaves the list alone on malformed results"

# --- names with |, & and a backslash -------------------------------------------
mkdir 'we|ird&\d'
cp list.orig 'we|ird&\d/l'
run res1 'we|ird&\d/l' abi.md
cat > want <<'EOF'
total: 1/5 pass
FAIL: t1:9 fails and is not in we|ird&\d/l
FAIL: t2:8 fails and is not in we|ird&\d/l
FAIL: 2 failing keys are not in we|ird&\d/l (a regression: fix it, do not list it)
EOF
want "a list name with |, & and a backslash" 1
cp abi.md 'we|ird&\d/abi.md'
run resu listu 'we|ird&\d/abi.md'
grep -qxF 'FAIL: unexplained deviation: k5: ABI-002 is not an active entry of we|ird&\d/abi.md' out \
  || { cat out >&2; fail "a ledger name with |, & and a backslash"; }
echo "ok a ledger name with |, & and a backslash"
mkdir 'o\n'
if DEVIATIONS_OUT='o\n' sh "$script" res1 list abi.md > out 2>&1; then rc=0; else rc=$?; fi
printf 't1:9\nt2:8\n' > want
cmp -s want 'o\n/new-failures' || fail "a DEVIATIONS_OUT name with a backslash"
echo "ok a DEVIATIONS_OUT name with a backslash"

# --- names with = -------------------------------------------------------------
# awk would take a bare `NAME=VALUE` operand as an assignment and read
# nothing; each name must still be read as a file.
printf '# c\na ABI-001: known\nb ABI-001: known too\n' > 'dev=amd64.txt'
cp 'dev=amd64.txt' 'dev=amd64.orig'
printf 'PASS a\nFAIL b\n' > 'res=amd64'
mkdir 'abi=dir'
cp abi/ABI-001.md 'abi=dir/'
cp abi.md 'abi=file.md'
run 'res=amd64' 'dev=amd64.txt' 'abi=dir'
cat > want <<'EOF'
total: 1/2 pass
FAIL: listed in dev=amd64.txt but passes: a
FAIL: 1 listed keys pass and 0 are not keys of this run; run the ratchet with --update to drop them from dev=amd64.txt
EOF
want "results, list and ledger directory names with =" 1
run 'res=amd64' 'dev=amd64.txt' 'abi=file.md'
want "a ledger file name with =" 1
printf 'PASS a\nSKIP b\n' > 'bad=res'
run 'bad=res' 'dev=amd64.txt' 'abi=dir'
cat > want <<'EOF'
error: bad=res:2: not a `PASS KEY` or `FAIL KEY` line: SKIP b
EOF
want "malformed results named with =" 1
printf '### ABI-002: x\n\n- **Status:** active\n' > 'abi=dir/ABI-003.md'
run 'res=amd64' 'dev=amd64.txt' 'abi=dir'
cat > want <<'EOF'
error: ledger: abi=dir/ABI-003.md: holds 1 entry headings, the first ABI-002; want one, ### ABI-003:
error: the ABI ledger abi=dir is malformed; nothing was checked or updated
EOF
want "a malformed ledger directory named with =" 1
rm 'abi=dir/ABI-003.md'
cmp -s 'dev=amd64.orig' 'dev=amd64.txt' || fail "the list named with = changed without --update"
run --update 'res=amd64' 'dev=amd64.txt' 'abi=dir'
cat > want <<'EOF'
updated dev=amd64.txt
total: 1/2 pass
PASS: the failing keys are exactly the 1 listed in dev=amd64.txt, each with its reason
EOF
want "--update on a list named with =" 0
printf '# c\nb ABI-001: known too\n' > want
cmp -s want 'dev=amd64.txt' || { cat 'dev=amd64.txt' >&2; fail "--update on a list named with =: the list it wrote"; }
echo "ok --update keeps the comment and the failing line of a list named with ="

# A DEVIATIONS_OUT named with =: its files are still read for the
# per-key lines, and written.
if DEVIATIONS_OUT='o=1' sh "$script" res4 list abi.md > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<'EOF'
total: 3/4 pass
FAIL: listed in list but passes: t2:3
FAIL: listed in list but not a key of this run: t1:5
FAIL: 1 listed keys pass and 1 are not keys of this run; run the ratchet with --update to drop them from list
FAIL: t2:8 fails and is not in list
FAIL: 1 failing keys are not in list (a regression: fix it, do not list it)
EOF
want "a DEVIATIONS_OUT named with =" 1
printf 't2:8\n' > want
cmp -s want 'o=1/new-failures' || fail "a DEVIATIONS_OUT named with =: its new-failures file"
printf 'FAIL a\n' > res8
: > list8
if DEVIATIONS_OUT='o=2' sh "$script" res8 list8 abi.md > out 2>&1; then rc=0; else rc=$?; fi
cat > want <<'EOF'
total: 0/1 pass
FAIL: a fails and is not in list8
FAIL: 1 failing keys are not in list8 (a regression: fix it, do not list it)
EOF
want "a DEVIATIONS_OUT named with =, an empty list" 1
if DEVIATIONS_OUT='o=3' sh "$script" resu listu abi.md > out 2>&1; then rc=0; else rc=$?; fi
grep -qxF 'FAIL: unexplained deviation: k2: no reason' out || { cat out >&2; fail "a DEVIATIONS_OUT named with =: the unexplained lines"; }
echo "ok a DEVIATIONS_OUT named with =: the unexplained lines"

# --- missing inputs and usage ---------------------------------------------------
run no-res list abi.md
echo "error: no results file no-res" > want
want "a missing results file" 1
run res no-list abi.md
echo "error: no deviations list no-list" > want
want "a missing list" 1
run res list no-abi
echo "error: no ABI ledger no-abi" > want
want "a missing ledger" 1

echo "usage: deviations.sh [--update] RESULTS LIST LEDGER" > want
run res list
want "two arguments" 2
run res list abi.md extra
want "four arguments" 2
run --update res list
want "--update and two arguments" 2
run --check res list abi.md
want "an unknown option" 2

echo "PASS: deviations.sh"
