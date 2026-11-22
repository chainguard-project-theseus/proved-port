#!/bin/sh
# kit/header-check-test.sh: tests of kit/make/header-check.sh (the
# header-check-test check). Each case writes an upstream include
# directory and the port's headers by hand, runs the script, and checks
# its exit status and its output, spelled out by hand from the PCRE2
# port's ci/header-check.sh's format (none is taken from a run of the
# script under test; the diff's two file-name lines, which name scratch
# files, are dropped before comparing): matching headers (blanks,
# comments, order, the port's own macros and other prefixes ignored), a
# changed value, a missing or extra definition, two headers with one
# differing, a prefix matched only at a name's start, no definition under
# the prefixes, a prefix that matches in no header beside one that does,
# a missing header (ours or upstream's) and usage. Run it
# from the repository root. It needs a POSIX shell and BusyBox's tools.
set -eu

script=$(pwd)/kit/make/header-check.sh
[ -f "$script" ] || { echo "FAIL: run it from the repository root (no kit/make/header-check.sh)" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"
mkdir up include

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# run ARGS...: the script with ARGS; its output (stdout and stderr) in
# out, without the diff's `--- ` and `+++ ` file-name lines, its status
# in rc.
run() {
  if sh "$script" "$@" > raw 2>&1; then rc=0; else rc=$?; fi
  grep -v -e '^--- ' -e '^+++ ' raw > out || true
}

# want WHAT RC: the run exited RC and printed exactly the file want.
want() {
  if [ "$rc" -ne "$2" ] || ! cmp -s want out; then
    printf 'got (exit %s):\n' "$rc" >&2
    cat raw >&2
    printf 'want (exit %s):\n' "$2" >&2
    cat want >&2
    fail "$1"
  fi
  echo "ok $1"
}

# The C library's header, and the port's copy: blanks and a tab around
# the name and value differ, a one-line comment is dropped, the order
# differs, the port adds a macro under its own prefix, and a macro under
# no prefix (FOO_VERSION_STRING is FOO_, so it is compared) and one
# under another prefix (BAR_) differ freely.
cat > up/foo.h <<'EOF'
#ifndef FOOLIB
#define FOOLIB
#define FOO_CASELESS   0x00000008u  /* Caseless */
#define FOO_ERROR_NOMATCH (-1)
#  define FOO_MAJOR 10
#define BAR_ONLY_UPSTREAM 1
#define FOO_VERSION_STRING "10.47"
#endif
EOF
cat > include/foo.h <<'EOF'
#ifndef FOOLIB
#define FOOLIB
#define FOO_ERROR_NOMATCH (-1)
#define	FOO_CASELESS 0x00000008u
#define FOO_MAJOR   10   /* major */
#define FOO_VERSION_STRING "10.47"
#define PORT_FOO_EXTRA 1
#define BAR_ONLY_OURS 2
#endif
EOF
run up FOO_ include/foo.h
cat > want <<'EOF'
ok   foo.h: 4 definitions match up/foo.h
EOF
want "matching header: blanks, comments, order and other prefixes ignored" 0

# A changed value fails with the diff, upstream's line first.
cp include/foo.h foo.good
cat > include/foo.h <<'EOF'
#define FOO_ERROR_NOMATCH (-2)
#define FOO_CASELESS 0x00000008u
#define FOO_MAJOR 10
#define FOO_VERSION_STRING "10.47"
EOF
run up FOO_ include/foo.h
cat > want <<'EOF'
@@ -1,4 +1,4 @@
 #define FOO_CASELESS 0x00000008u
-#define FOO_ERROR_NOMATCH (-1)
+#define FOO_ERROR_NOMATCH (-2)
 #define FOO_MAJOR 10
 #define FOO_VERSION_STRING "10.47"
FAIL foo.h differs from up/foo.h
EOF
want "a changed value fails" 1

# A suffix or parentheses are part of the value.
cat > include/foo.h <<'EOF'
#define FOO_ERROR_NOMATCH -1
#define FOO_CASELESS 0x00000008
#define FOO_MAJOR 10
#define FOO_VERSION_STRING "10.47"
EOF
run up FOO_ include/foo.h
cat > want <<'EOF'
@@ -1,4 +1,4 @@
-#define FOO_CASELESS 0x00000008u
-#define FOO_ERROR_NOMATCH (-1)
+#define FOO_CASELESS 0x00000008
+#define FOO_ERROR_NOMATCH -1
 #define FOO_MAJOR 10
 #define FOO_VERSION_STRING "10.47"
FAIL foo.h differs from up/foo.h
EOF
want "a dropped suffix and parentheses fail" 1

# A definition missing from ours, or one ours adds under the prefix.
cat > include/foo.h <<'EOF'
#define FOO_CASELESS 0x00000008u
#define FOO_ERROR_NOMATCH (-1)
#define FOO_MAJOR 10
#define FOO_VERSION_STRING "10.47"
#define FOO_NEW 1
EOF
run up FOO_ include/foo.h
cat > want <<'EOF'
@@ -1,4 +1,5 @@
 #define FOO_CASELESS 0x00000008u
 #define FOO_ERROR_NOMATCH (-1)
 #define FOO_MAJOR 10
+#define FOO_NEW 1
 #define FOO_VERSION_STRING "10.47"
EOF
printf 'FAIL foo.h differs from up/foo.h\n' >> want
want "an extra definition under the prefix fails" 1
cat > include/foo.h <<'EOF'
#define FOO_CASELESS 0x00000008u
#define FOO_MAJOR 10
#define FOO_VERSION_STRING "10.47"
EOF
run up FOO_ include/foo.h
cat > want <<'EOF'
@@ -1,4 +1,3 @@
 #define FOO_CASELESS 0x00000008u
-#define FOO_ERROR_NOMATCH (-1)
 #define FOO_MAJOR 10
 #define FOO_VERSION_STRING "10.47"
FAIL foo.h differs from up/foo.h
EOF
want "a missing definition fails" 1

# A continued definition is compared whole: its later lines too.
cp up/foo.h up/foo.save
cp foo.good include/foo.h
printf '#define FOO_ALL \\\n  (0x1|0x2|0x4)\n' >> up/foo.h
printf '#define FOO_ALL \\\n  (0x1|0x2)\n' >> include/foo.h
run up FOO_ include/foo.h
cat > want <<'EOF'
@@ -1,4 +1,4 @@
-#define FOO_ALL (0x1|0x2|0x4)
+#define FOO_ALL (0x1|0x2)
 #define FOO_CASELESS 0x00000008u
 #define FOO_ERROR_NOMATCH (-1)
 #define FOO_MAJOR 10
EOF
printf 'FAIL foo.h differs from up/foo.h\n' >> want
want "a continued definition's later line differing fails" 1
cp up/foo.save up/foo.h
printf '#define FOO_ALL (0x1|0x2|0x4)\n' >> up/foo.h
cp foo.good include/foo.h
printf '#define FOO_ALL \\\n\t(0x1|0x2|0x4)\n' >> include/foo.h
run up FOO_ include/foo.h
cat > want <<'EOF'
ok   foo.h: 5 definitions match up/foo.h
EOF
want "a definition continued where upstream's is not matches" 0

# A continuation as C reads it, on a header of one definition: a CR (a
# CRLF header) or blanks after the backslash still continue the line,
# and the backslash and line end leave nothing in their place.
printf '#define FOO_ALL \\\r\n  (0x1| \\\r\n  0x2|0x4)\r\n' > up/c.h
printf '#define FOO_ALL \\\r\n  (0x1| \\\r\n  0x2)\r\n' > include/c.h
run up FOO_ include/c.h
cat > want <<'EOF'
@@ -1 +1 @@
-#define FOO_ALL (0x1| 0x2|0x4)
+#define FOO_ALL (0x1| 0x2)
FAIL c.h differs from up/c.h
EOF
want "a CRLF definition continued twice, its last line differing, fails" 1
printf '#define FOO_ALL \\ \t\n  (0x1|0x2|0x4)\n' > up/c.h
printf '#define FOO_ALL \\ \n  (0x1|0x2)\n' > include/c.h
run up FOO_ include/c.h
cat > want <<'EOF'
@@ -1 +1 @@
-#define FOO_ALL (0x1|0x2|0x4)
+#define FOO_ALL (0x1|0x2)
FAIL c.h differs from up/c.h
EOF
want "blanks after a continuation's backslash, the later line differing, fails" 1
printf '#define FOO_ALL (0x1|0x2|0x4)\n' > up/c.h
printf '#define FOO_ALL \\\r\n  (0x1|0x2|0x4) \r\n' > include/c.h
run up FOO_ include/c.h
cat > want <<'EOF'
ok   c.h: 1 definitions match up/c.h
EOF
want "a CRLF continued definition equal to upstream's one line matches" 0
printf '#define FOO_S "abcdef"\n' > up/c.h
printf '#define FOO_S "abc\\\ndef"\n' > include/c.h
run up FOO_ include/c.h
cat > want <<'EOF'
ok   c.h: 1 definitions match up/c.h
EOF
want "a string continued across lines is the string joined" 0
printf '#define FOO_S "abc def"\n' > up/c.h
run up FOO_ include/c.h
cat > want <<'EOF'
@@ -1 +1 @@
-#define FOO_S "abc def"
+#define FOO_S "abcdef"
FAIL c.h differs from up/c.h
EOF
want "a continued string gets no blank at the join" 1
# Blanks inside a literal are kept as they are; outside, squeezed.
printf '#define  FOO_S\t"a b"  /* x */\n' > up/c.h
printf '#define FOO_S "a  b"\n' > include/c.h
run up FOO_ include/c.h
cat > want <<'EOF'
@@ -1 +1 @@
-#define FOO_S "a b"
+#define FOO_S "a  b"
FAIL c.h differs from up/c.h
EOF
want "blanks inside a string literal are compared as they are" 1
printf '#define FOO_S "a  b"\n' > up/c.h
printf '#define \tFOO_S   "a  b"\r\n' > include/c.h
run up FOO_ include/c.h
cat > want <<'EOF'
ok   c.h: 1 definitions match up/c.h
EOF
want "blanks outside a literal are squeezed, inside kept" 0
rm up/c.h include/c.h

# Two comments on a line: the text between them is compared, and a
# comment left open at the end of a line is dropped.
cp up/foo.save up/foo.h
printf '#define FOO_B /* a */ 5 /* b */\n#define FOO_C 3 /* open\n   comment */\n' >> up/foo.h
cp foo.good include/foo.h
printf '#define FOO_B /* a */ 6 /* b */\n#define FOO_C 3\n' >> include/foo.h
run up FOO_ include/foo.h
cat > want <<'EOF'
@@ -1,4 +1,4 @@
-#define FOO_B 5
+#define FOO_B 6
 #define FOO_C 3
 #define FOO_CASELESS 0x00000008u
 #define FOO_ERROR_NOMATCH (-1)
EOF
printf 'FAIL foo.h differs from up/foo.h\n' >> want
want "the value between two comments differing fails" 1
cp up/foo.save up/foo.h

# A comment marker inside a string literal is no comment.
printf '#define FOO_PATTERN "/*old"\n' >> up/foo.h
cp foo.good include/foo.h
printf '#define FOO_PATTERN "/*new"\n' >> include/foo.h
run up FOO_ include/foo.h
cat > want <<'EOF'
@@ -1,5 +1,5 @@
 #define FOO_CASELESS 0x00000008u
 #define FOO_ERROR_NOMATCH (-1)
 #define FOO_MAJOR 10
-#define FOO_PATTERN "/*old"
+#define FOO_PATTERN "/*new"
 #define FOO_VERSION_STRING "10.47"
FAIL foo.h differs from up/foo.h
EOF
want "a string holding /* differing fails" 1
# A character literal and an escaped quote in a string: neither opens a
# comment, so a later definition is still compared.
printf "#define FOO_Q '/*'\n#define FOO_X 1\n#define FOO_Y 2 /* */\n" > up/c.h
printf "#define FOO_Q '/*'\n#define FOO_X 9\n#define FOO_Y 2 /* */\n" > include/c.h
run up FOO_ include/c.h
cat > want <<'EOF'
@@ -1,3 +1,3 @@
 #define FOO_Q '/*'
-#define FOO_X 1
+#define FOO_X 9
 #define FOO_Y 2
FAIL c.h differs from up/c.h
EOF
want "a character literal holding /*, a later definition differing, fails" 1
printf '#define FOO_E "\\"/*"\n#define FOO_X 1\n#define FOO_Y 2 /* */\n' > up/c.h
printf '#define FOO_E "\\"/*"\n#define FOO_X 9\n#define FOO_Y 2 /* */\n' > include/c.h
run up FOO_ include/c.h
cat > want <<'EOF'
@@ -1,3 +1,3 @@
 #define FOO_E "\"/*"
-#define FOO_X 1
+#define FOO_X 9
 #define FOO_Y 2
FAIL c.h differs from up/c.h
EOF
want "an escaped quote then /* in a string, a later definition differing, fails" 1
rm up/c.h include/c.h
cp up/foo.save up/foo.h

# A definition inside a comment over several lines, and a `//` comment,
# are not compared; an `#undef` under the prefix is.
cp foo.good include/foo.h
printf '/* old:\n#define FOO_GONE 1\n*/\n#define FOO_MAJOR2 10 // the same\n' >> include/foo.h
printf '#define FOO_MAJOR2 10\n' >> up/foo.h
run up FOO_ include/foo.h
cat > want <<'EOF'
ok   foo.h: 5 definitions match up/foo.h
EOF
want "a definition inside a comment, and a // comment, are not compared" 0
printf '#undef FOO_MAJOR\n' >> include/foo.h
run up FOO_ include/foo.h
cat > want <<'EOF'
@@ -3,3 +3,4 @@
 #define FOO_MAJOR 10
 #define FOO_MAJOR2 10
 #define FOO_VERSION_STRING "10.47"
+#undef FOO_MAJOR
FAIL foo.h differs from up/foo.h
EOF
want "an #undef the port adds fails" 1
cp up/foo.save up/foo.h

# A prefix matches only at the start of the name: XFOO_ is not FOO_.
cp foo.good include/foo.h
printf '#define XFOO_A 1\n' >> include/foo.h
run up FOO_ include/foo.h
cat > want <<'EOF'
ok   foo.h: 4 definitions match up/foo.h
EOF
want "a prefix matches at the name's start only" 0

# Two headers and two prefixes: each is compared; one differing fails
# the run after both are checked.
cp foo.good include/foo.h
cat > up/foo-posix.h <<'EOF'
#define REG_ICASE 0x0001
#define FOO_POSIX 1
EOF
cat > include/foo-posix.h <<'EOF'
#define REG_ICASE 0x0002
#define FOO_POSIX 1
EOF
run up "FOO_ REG_" include/foo-posix.h include/foo.h
cat > want <<'EOF'
@@ -1,2 +1,2 @@
 #define FOO_POSIX 1
-#define REG_ICASE 0x0001
+#define REG_ICASE 0x0002
FAIL foo-posix.h differs from up/foo-posix.h
ok   foo.h: 4 definitions match up/foo.h
EOF
want "two headers, the first differing" 1
cp up/foo-posix.h include/foo-posix.h
run up "FOO_ REG_" include/foo.h include/foo-posix.h
cat > want <<'EOF'
ok   foo.h: 4 definitions match up/foo.h
ok   foo-posix.h: 2 definitions match up/foo-posix.h
EOF
want "two headers and two prefixes, both matching" 0

# No definition under the prefixes on either side compares nothing.
run up "QUX_" include/foo.h
cat > want <<'EOF'
FAIL foo.h: no definition under QUX_ in include/foo.h or up/foo.h
FAIL: no definition under QUX_ in any header
EOF
want "no definition under the prefixes fails" 1
# One prefix of two matches nothing in any header (REGX_ for REG_):
# foo.h's FOO_ lines match, and the dead prefix still fails the run.
run up "FOO_ REGX_" include/foo.h include/foo-posix.h
cat > want <<'EOF'
ok   foo.h: 4 definitions match up/foo.h
ok   foo-posix.h: 1 definitions match up/foo-posix.h
FAIL: no definition under REGX_ in any header
EOF
want "a prefix that matches no header fails, beside one that matches" 1
# A prefix that matches in one header alone (REG_, in foo-posix.h only)
# passes.
run up "FOO_ REG_" include/foo.h include/foo-posix.h
cat > want <<'EOF'
ok   foo.h: 4 definitions match up/foo.h
ok   foo-posix.h: 2 definitions match up/foo-posix.h
EOF
want "a prefix that matches in one header of two passes" 0

# A missing header, upstream's or ours.
run up FOO_ include/bar.h
cat > want <<'EOF'
FAIL: up/bar.h not found (install the C library's -dev package)
EOF
want "a header upstream lacks fails" 1
cp up/foo.h up/baz.h
run up FOO_ include/baz.h
cat > want <<'EOF'
FAIL: include/baz.h not found
EOF
want "a header of ours that is missing fails" 1

# Usage.
run up FOO_
cat > want <<'EOF'
usage: header-check.sh UPSTREAM PREFIXES HEADER...
EOF
want "no header is usage" 2
run up "" include/foo.h
cat > want <<'EOF'
usage: header-check.sh UPSTREAM PREFIXES HEADER...: PREFIXES is empty
EOF
want "empty prefixes are usage" 2
run up "FOO_ a|b" include/foo.h
cat > want <<'EOF'
usage: header-check.sh UPSTREAM PREFIXES HEADER...: PREFIXES holds a|b, not a C identifier prefix
EOF
want "a prefix that is no identifier is usage" 2
echo PASS
