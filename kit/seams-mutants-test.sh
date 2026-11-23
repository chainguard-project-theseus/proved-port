#!/bin/sh
# kit/seams-mutants-test.sh: tests of kit/verus/seams/mutants.sh, the
# seam ledger generator's mutant runner, on a small stand-in Go package
# (MUTANTS_KIT and MUTANTS_PKG point the runner at it): a killed mutant,
# a survivor (fails), a marked equivalent that survives (passes) or is
# killed (fails), a FROM that occurs twice or not at all, FILE#N/M (the
# Nth occurrence killed, replaced alone, a survivor, M not the count,
# malformed and oversized indexes, one marked equivalent), a mutant
# that changes nothing,
# malformed lines, a FILE outside the package, an empty list, a missing list, a
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

# FILE#N/M names the Nth of M occurrences (#250). `return a` is Max's
# (tested) then Twice's; `a + a` is Twice's then the comment's.
run "occurrence killed" 0 "toy.go#1/2${T}return a${T}return b
" "KILLED toy.go#1/2: return a -> return b" "mutants: 1 killed, 0 equivalent, 0 survived, 0 bad lines"

# Only the Nth is replaced: `a +` breaks the build in Twice (the 1st)
# but not in the comment (the 2nd).
run "occurrence index" 1 "toy.go#2/2${T}a + a${T}a +
toy.go#1/2${T}a + a${T}a +
" "SURVIVED toy.go#2/2: a + a -> a +" \
	"BAD list:2: the mutant does not build (or fails go test's vet), so no test can kill it: toy.go#1/2: a + a -> a +" \
	"mutants: 0 killed, 0 equivalent, 1 survived, 1 bad lines"

# An equivalent mark names one place; FILE#N/M can move unseen.
run "occurrence marked equivalent" 1 "toy.go#2/2${T}a + a${T}a - a${T}equivalent: the comment's
toy.go${T}a > b${T}a < b
" "BAD list:1: a row marked equivalent names one place: widen FROM until it occurs once and write FILE alone: toy.go#2/2"

run "occurrence survivor" 1 "toy.go#1/2${T}a + a${T}a - a
" "SURVIVED toy.go#1/2: a + a -> a - a" "mutants: 0 killed, 0 equivalent, 1 survived, 0 bad lines"

run "occurrences not M" 1 "toy.go#1/3${T}a + a${T}x
toy.go#1/2${T}a > b${T}a < b
toy.go${T}a > b${T}a < b
" "BAD list:1: FROM occurs 2 times in toy.go, want 3 (toy.go#1/3): a + a" \
	"BAD list:2: FROM occurs 1 times in toy.go, want 2 (toy.go#1/2): a > b"

# Oversized indexes too: past test's integer range a comparison fails,
# which must not read as in range.
for bad in 'toy.go#3/2' 'toy.go#0/2' 'toy.go#1/1' 'toy.go#1/02' 'toy.go#01/2' 'toy.go#a/2' 'toy.go#1' 'toy.go#1/2/3' 'toy.go#' 'toy.go#1/2x' 'toy.go#/2' \
	'toy.go#999999999999999999999999/2' 'toy.go#1/999999999999999999999999' 'toy.go#1000000000/2'; do
	run "malformed $bad" 1 "$bad${T}a + a${T}x
toy.go${T}a > b${T}a < b
" "BAD list:1: want FILE#N/M, the Nth of M occurrences (1 <= N <= M, M >= 2; a FROM that occurs once takes FILE alone): $bad"
done

# A mutant that leaves the file as it was is no mutant, though marked
# equivalent.
run "changes nothing" 1 "toy.go${T}a > b${T}a > b${T}equivalent: the same text
toy.go#2/2${T}a + a${T}a + a
toy.go${T}a > b${T}a < b
" "BAD list:1: the mutant changes nothing in toy.go: toy.go: a > b -> a > b" \
	"BAD list:2: the mutant changes nothing in toy.go: toy.go#2/2: a + a -> a + a" \
	"mutants: 1 killed, 0 equivalent, 0 survived, 2 bad lines"

run "occurrence file outside the package" 1 "../toy.go#1/2${T}a + a${T}x
toy.go${T}a > b${T}a < b
" "BAD list:1: FILE is a file name of the package: ../toy.go"

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
