#!/bin/sh
# kit/seams-mutants-test.sh: tests of kit/verus/seams/mutants.sh, the
# seam ledger generator's mutant runner, on a small stand-in Go package
# (MUTANTS_KIT and MUTANTS_PKG point the runner at it): a killed mutant,
# a survivor (fails), a marked equivalent that survives (passes) or is
# killed (fails), a FROM that occurs twice or not at all, malformed
# lines, a FILE outside the package, an empty list, a missing list, a
# failing unmutated package and usage. The expected lines are written
# by hand. Run it from the repository root; it needs go.
set -eu

runner=$(pwd)/kit/verus/seams/mutants.sh
[ -f "$runner" ] || { echo "FAIL: run it from the repository root (no kit/verus/seams/mutants.sh)" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cd "$tmp"
export GOTOOLCHAIN=local GOFLAGS=-mod=mod GOCACHE="${GOCACHE:-$tmp/gocache}"

fail() {
	echo "FAIL: $*" >&2
	exit 1
}

# The stand-in: Max is tested, Twice is not.
mkdir -p k/toy
cat > k/toy/go.mod <<'EOF'
module toy

go 1.22
EOF
cat > k/toy/toy.go <<'EOF'
package toy

func Max(a, b int) int {
	if a > b {
		return a
	}
	return b
}

func Twice(a int) int { return a + a }

// pad: a + a
EOF
cat > k/toy/toy_test.go <<'EOF'
package toy

import "testing"

func TestMax(t *testing.T) {
	if Max(1, 2) != 2 || Max(3, 2) != 3 {
		t.Fatal("max")
	}
}
EOF
export MUTANTS_KIT=k MUTANTS_PKG=toy
T=$(printf '\t')

# run NAME WANT_CODE LIST_TEXT WANT_LINE...: write the list, run, check
# the exit status and that each wanted line is in the output.
run() {
	name=$1 want=$2 text=$3
	shift 3
	printf '%s' "$text" > list
	set +e
	sh "$runner" list > out 2>&1
	code=$?
	set -e
	[ "$code" = "$want" ] || { cat out >&2; fail "$name: exit $code, want $want"; }
	for w in "$@"; do
		grep -qF -- "$w" out || { cat out >&2; fail "$name: no line $w"; }
	done
	echo "ok $name"
}

run "killed" 0 "toy.go${T}a > b${T}a < b
" "KILLED toy.go: a > b -> a < b" "mutants: 1 killed, 0 equivalent, 0 survived, 0 bad lines"

run "survivor fails" 1 "toy.go${T}a > b${T}a < b
toy.go${T}return a + a${T}return a
" "SURVIVED toy.go: return a + a -> return a" "mutants: 1 killed, 0 equivalent, 1 survived, 0 bad lines"

run "equivalent survivor" 0 "# a comment

toy.go${T}return a + a${T}return a${T}equivalent: Twice is not part of this test
toy.go${T}a > b${T}a < b
" "EQUIVALENT toy.go: return a + a -> return a (equivalent: Twice is not part of this test)" "mutants: 1 killed, 1 equivalent, 0 survived, 0 bad lines"

run "equivalent killed" 1 "toy.go${T}a > b${T}a < b${T}equivalent: wrong
" "BAD list:1: marked equivalent but killed (drop the mark): toy.go: a > b -> a < b"

run "FROM twice" 1 "toy.go${T}a + a${T}a
toy.go${T}a > b${T}a < b
" "BAD list:1: FROM occurs 2 times in toy.go, want once: a + a"

run "FROM absent" 1 "toy.go${T}a >= b${T}a < b
toy.go${T}a > b${T}a < b
" "BAD list:1: FROM occurs 0 times in toy.go, want once: a >= b"

run "two fields" 1 "toy.go${T}a > b
toy.go${T}a > b${T}a < b
" "BAD list:1: want FILE<TAB>FROM<TAB>TO[<TAB>equivalent: REASON]"

run "one field" 1 "toy.go
toy.go${T}a > b${T}a < b
" "BAD list:1: want FILE<TAB>FROM<TAB>TO[<TAB>equivalent: REASON]"

run "fourth field" 1 "toy.go${T}a > b${T}a < b${T}because
" "BAD list:1: a fourth field is \`equivalent: REASON\`"

run "empty reason" 1 "toy.go${T}a > b${T}a < b${T}equivalent:
" "BAD list:1: a fourth field is \`equivalent: REASON\`"

run "file outside the package" 1 "../toy/toy.go${T}a > b${T}a < b
toy.go${T}a > b${T}a < b
" "BAD list:1: FILE is a file name of the package: ../toy/toy.go"

run "no such file" 1 "nope.go${T}a > b${T}a < b
toy.go${T}a > b${T}a < b
" "BAD list:1: no FROM, or no nope.go in the package"

run "empty FROM" 1 "toy.go${T}${T}x
toy.go${T}a > b${T}a < b
" "BAD list:1: no FROM, or no toy.go in the package"

run "no mutant" 1 "# only a comment
" "mutants: no mutant in list"

# Each mutant is undone before the next: a mutant of another file runs
# on the original toy.go (left as `a < b`, the message-only mutant of
# the test would be killed and reported as a marked one killed).
run "file restored" 0 "toy.go${T}a > b${T}a < b
toy_test.go${T}t.Fatal(\"max\")${T}t.Fatal(\"MAX\")${T}equivalent: only the message changes
" "KILLED toy.go: a > b -> a < b" "EQUIVALENT toy_test.go: t.Fatal(\"max\") -> t.Fatal(\"MAX\") (equivalent: only the message changes)" "mutants: 1 killed, 1 equivalent, 0 survived, 0 bad lines"

# A restore that fails stops the run: a cp first on PATH copies the kit
# (cp -R) but fails every other copy.
realcp=$(command -v cp)
mkdir shim
cat > shim/cp <<EOF
#!/bin/sh
case "\$1" in -R) exec "$realcp" "\$@" ;; esac
exit 1
EOF
chmod +x shim/cp
printf 'toy.go\ta > b\ta < b\n' > list
set +e
PATH=$tmp/shim:$PATH sh "$runner" list > out 2>&1
code=$?
set -e
if [ "$code" != 2 ] || ! grep -q "mutants: cannot restore toy.go in the copy" out; then cat out >&2; fail "failed restore: exit $code"; fi
echo "ok failed restore"

# A mutant that does not build is no kill.
run "does not build" 1 "toy.go${T}return b${T}return
toy.go${T}a > b${T}a < b
" "BAD list:1: the mutant does not build (or fails go test's vet), so no test can kill it: toy.go: return b -> return"

# The package itself is left as it was.
grep -q 'if a > b {' k/toy/toy.go || fail "the package was changed"
echo "ok package unchanged"

set +e
sh "$runner" missing > out 2>&1
code=$?
set -e
if [ "$code" != 2 ] || ! grep -q "mutants: cannot read missing" out; then cat out >&2; fail "missing list: exit $code"; fi
echo "ok missing list"

set +e
sh "$runner" a b > out 2>&1
code=$?
set -e
if [ "$code" != 2 ] || ! grep -q "usage:" out; then cat out >&2; fail "usage: exit $code"; fi
echo "ok usage"

printf 'toy.go\ta > b\ta < b\n' > list
cat > k/toy/toy_test.go <<'EOF'
package toy

import "testing"

func TestBroken(t *testing.T) { t.Fatal("broken") }
EOF
set +e
sh "$runner" list > out 2>&1
code=$?
set -e
if [ "$code" != 2 ] || ! grep -q "mutants: go test fails on the unmutated copy" out; then cat out >&2; fail "failing package: exit $code"; fi
echo "ok failing unmutated package"

echo "all ok"
