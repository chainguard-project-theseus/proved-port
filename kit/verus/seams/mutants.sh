#!/bin/sh
# kit/verus/seams/mutants.sh: hand mutants of the seam ledger generator
# that its tests must kill (the seams-mutants check; PLAN step 3's
# seams-mutants.sh row). Run it from the repository root:
#
#   sh kit/verus/seams/mutants.sh [LIST]
#
# LIST (default kit/verus/seams/testdata/mutants.txt) holds one mutant a
# line, tab-separated: FILE (a .go file of kit/verus/seams), FROM (text
# that occurs exactly once in FILE, on one line), TO (its replacement,
# possibly empty), and an optional fourth field `equivalent: REASON` for
# a mutant no test can kill because it changes nothing a valid input
# can show. `#` lines and blank lines are comments. FILE written
# `FILE#N/M` names the Nth of M occurrences of a FROM that occurs more
# than once (1 <= N <= M, M >= 2, each at most 9 digits with no leading
# zero): FROM must occur
# exactly M times, counted left to right, line by line, not
# overlapping, and only the Nth is replaced; the result lines name
# FILE#N/M. The count catches an occurrence added or removed, not one
# moved, so a FILE#N/M row may not be marked equivalent (a mark names
# one place: widen FROM until it occurs once).
#
# It copies the whole kit/ (the tests read ../../templates) to a
# temporary directory, checks that the unmutated copy passes go test,
# then for each mutant replaces FROM with TO in the copy, checks that it
# builds (go test -run '^$'), runs go test there and puts the file back.
# Each mutant prints one line: KILLED, SURVIVED, or EQUIVALENT (an
# equivalent one that survived). It fails (exit 1) on a survivor not
# marked equivalent, on a marked one that was killed (its reason no
# longer holds: drop the mark), on a mutant that does not build (no test
# could have killed it), on a FROM that does not occur exactly once, or
# exactly M times for FILE#N/M (the code moved: update the line), on a
# malformed FILE#N/M, and on a mutant that leaves the file unchanged;
# exit 2 is usage, an unreadable list or a failing unmutated copy.
#
# MUTANTS_KIT (default kit) and MUTANTS_PKG (default verus/seams) name
# the tree copied and the package tested inside it; the self-test sets
# them to a small stand-in package.
set -u

kit=${MUTANTS_KIT:-kit}
pkg=${MUTANTS_PKG:-verus/seams}
list=${1:-$kit/$pkg/testdata/mutants.txt}
[ $# -le 1 ] || { echo "usage: sh kit/verus/seams/mutants.sh [LIST]" >&2; exit 2; }
[ -d "$kit/$pkg" ] || { echo "mutants: no $kit/$pkg (run it from the repository root)" >&2; exit 2; }
[ -f "$list" ] && [ -r "$list" ] || { echo "mutants: cannot read $list" >&2; exit 2; }
command -v go >/dev/null 2>&1 || { echo "mutants: no go on PATH" >&2; exit 2; }

export GOTOOLCHAIN=local GOFLAGS=-mod=mod
tmp=$(mktemp -d) || exit 2
trap 'rm -rf "$tmp"' EXIT
cp -R "$kit" "$tmp/kit" || exit 2
src=$(cd "$kit/$pkg" && pwd) || exit 2
work=$tmp/kit/$pkg

gotest() {
	(cd "$work" && go test -count=1 -timeout 120s . >/dev/null 2>&1)
}

# gobuild: the package and its tests compile (and pass go test's vet),
# running no test; a mutant that does not is no kill.
gobuild() {
	(cd "$work" && go test -count=1 -run '^$' . >/dev/null 2>&1)
}

gotest || { echo "mutants: go test fails on the unmutated copy" >&2; exit 2; }

tab=$(printf '\t')
killed=0 survived=0 equiv=0 bad=0 n=0
while IFS= read -r line || [ -n "$line" ]; do
	n=$((n + 1))
	case $line in '#'* | '') continue ;; esac
	file=${line%%"$tab"*}
	rest=${line#*"$tab"}
	if [ "$rest" = "$line" ]; then
		echo "BAD $list:$n: want FILE<TAB>FROM<TAB>TO[<TAB>equivalent: REASON]"
		bad=$((bad + 1))
		continue
	fi
	from=${rest%%"$tab"*}
	rest2=${rest#*"$tab"}
	if [ "$rest2" = "$rest" ]; then
		echo "BAD $list:$n: want FILE<TAB>FROM<TAB>TO[<TAB>equivalent: REASON]"
		bad=$((bad + 1))
		continue
	fi
	to=${rest2%%"$tab"*}
	why=
	[ "$to" = "$rest2" ] || why=${rest2#*"$tab"}
	case $why in
	'' | 'equivalent: '?*) ;;
	*)
		echo "BAD $list:$n: a fourth field is \`equivalent: REASON\`"
		bad=$((bad + 1))
		continue
		;;
	esac
	# at: FILE as written, `FILE#N/M` naming the Nth of M occurrences
	# of FROM (#250); occ and occs are N and M, 1 and 1 for a plain FILE.
	at=$file occ=1 occs=1
	case $file in
	*'#'*)
		nm=${file#*#}
		file=${file%%#*}
		case $nm in
		[1-9]*/[1-9]*) ;;
		*) nm=bad ;;
		esac
		occ=${nm%%/*} occs=${nm#*/}
		# Digits only, at most 9 each, so test's integer comparisons
		# below cannot fail on range (a failing test reads as false).
		case $occ$occs in
		*[!0-9]*) occ=0 ;;
		esac
		[ ${#occ} -le 9 ] && [ ${#occs} -le 9 ] || occ=0
		if ! { [ "$occ" -ge 1 ] && [ "$occ" -le "$occs" ] && [ "$occs" -ge 2 ]; }; then
			echo "BAD $list:$n: want FILE#N/M, the Nth of M occurrences (1 <= N <= M, M >= 2; a FROM that occurs once takes FILE alone): $at"
			bad=$((bad + 1))
			continue
		fi
		# The count catches an occurrence added or removed, not one
		# moved (one dropped and one added, or two reordered), which
		# would carry an equivalent mark to another place unseen.
		if [ -n "$why" ]; then
			echo "BAD $list:$n: a row marked equivalent names one place: widen FROM until it occurs once and write FILE alone: $at"
			bad=$((bad + 1))
			continue
		fi
		;;
	esac
	case $file in
	*/* | .* | '' | *[!A-Za-z0-9_.-]*)
		echo "BAD $list:$n: FILE is a file name of the package: $file"
		bad=$((bad + 1))
		continue
		;;
	esac
	if [ -z "$from" ] || [ ! -f "$src/$file" ]; then
		echo "BAD $list:$n: no FROM, or no $file in the package"
		bad=$((bad + 1))
		continue
	fi
	count=$(FROM=$from awk 'BEGIN { f = ENVIRON["FROM"] } { s = $0; while ((i = index(s, f)) > 0) { c++; s = substr(s, i + length(f)) } } END { print c + 0 }' "$src/$file")
	if [ "$count" != "$occs" ]; then
		if [ "$occs" = 1 ]; then
			echo "BAD $list:$n: FROM occurs $count times in $file, want once: $from"
		else
			echo "BAD $list:$n: FROM occurs $count times in $file, want $occs ($at): $from"
		fi
		bad=$((bad + 1))
		continue
	fi
	# Replace the occ-th occurrence, counted as count counts them (left
	# to right, line by line, not overlapping).
	FROM=$from TO=$to N=$occ awk 'BEGIN { f = ENVIRON["FROM"]; t = ENVIRON["TO"]; n = ENVIRON["N"] + 0 }
		{ out = ""; s = $0
		  while ((i = index(s, f)) > 0) { c++; out = out substr(s, 1, i - 1) (c == n ? t : f); s = substr(s, i + length(f)) }
		  print out s }' "$src/$file" > "$work/$file" || {
		echo "mutants: cannot write the mutant of $file in the copy" >&2
		exit 2
	}
	# A mutant that leaves the file as it was (TO equal to FROM, or no
	# occurrence replaced) is no mutant: no test could kill it.
	if cmp -s "$src/$file" "$work/$file"; then
		echo "BAD $list:$n: the mutant changes nothing in $file: $at: $from -> $to"
		bad=$((bad + 1))
		continue
	fi
	if ! gobuild; then
		echo "BAD $list:$n: the mutant does not build (or fails go test's vet), so no test can kill it: $at: $from -> $to"
		bad=$((bad + 1))
	elif gotest; then
		if [ -n "$why" ]; then
			echo "EQUIVALENT $at: $from -> $to ($why)"
			equiv=$((equiv + 1))
		else
			echo "SURVIVED $at: $from -> $to"
			survived=$((survived + 1))
		fi
	else
		if [ -n "$why" ]; then
			echo "BAD $list:$n: marked equivalent but killed (drop the mark): $at: $from -> $to"
			bad=$((bad + 1))
		else
			echo "KILLED $at: $from -> $to"
			killed=$((killed + 1))
		fi
	fi
	# A mutant left in place would colour every line after it: a restore
	# that fails stops the run.
	if ! cp "$src/$file" "$work/$file" || ! cmp -s "$src/$file" "$work/$file"; then
		echo "mutants: cannot restore $file in the copy" >&2
		exit 2
	fi
done < "$list"

echo "mutants: $killed killed, $equiv equivalent, $survived survived, $bad bad lines"
[ "$survived" -eq 0 ] && [ "$bad" -eq 0 ] || exit 1
[ $((killed + equiv)) -gt 0 ] || { echo "mutants: no mutant in $list" >&2; exit 1; }
exit 0
