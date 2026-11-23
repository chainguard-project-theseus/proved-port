#!/bin/sh
# kit/perf/box/rounds.sh: the measured-run skeleton of a bench box run
# (PLAN step 6). A port's box scripts repeated it in every phase:
#
#   rounds.sh -o RAW [-g GROUP] [-r ROUNDS] [-e EVENTS | -E SET [-f FILE] [-a ARCH]]
#             [-c CORE] [-w WIDTH] ARMS ROWS
#   rounds.sh -n [-r ROUNDS] [-E SET [-f FILE] [-a ARCH]] ARMS ROWS
#
# ARMS is a file with one arm name per line. ROWS is a file of
# "row<TAB>command" lines. For each round r = 1..ROUNDS (default 5), the
# arms rotated by r (with n arms, position i = 0..n-1 runs arm
# (i + r) mod n + 1 of the file, so no arm always runs first), and for
# each arm every row in file order, it runs
#
#   perf stat -x, -e EVENTS -o CSV -- [taskset -c CORE] sh -c COMMAND
#
# with ARM and ROUND exported to the command, and appends one line to
# RAW: row, arm, round, each event's count (the first CSV field of the
# line whose third field is the event, empty when there is none), and
# the command's output (stdout and stderr) with tabs and newlines turned
# into spaces, cut to WIDTH bytes (default 60). -g puts GROUP before the
# row; with it and the default EVENTS (instructions:u,cycles:u) the line
# is the layout `stats relabel` and `stats check` read (group, row, arm,
# round, instructions, cycles, output). Without -c there is no taskset.
#
# -E SET takes EVENTS from an events file of ARCH<TAB>SET<TAB>EVENTS
# lines (# lines and empty lines skipped): the one line for SET and
# ARCH. FILE defaults to events.tsv beside rounds.sh, and ARCH to
# `uname -m`. Event names differ per arch (aarch64's l1i_cache_refill
# has no x86-64 namesake), so a phase names a set and each box gets its
# own list. A set's lines have one more count column per extra event.
# stats reads two counts and then the output, so -g (stats's layout)
# takes exactly two events; a wider set such as frontend runs without
# -g, and its lines are for the port's own table (port A read its
# frontend phase with its own awk), not for stats. stats reads the two
# counts by place, as instructions then cycles, whatever their names.
#
# A command that exits non-zero still gets its line; after the last run
# rounds.sh names each such run on stderr and exits 1. Wrap a command
# whose non-zero exit is expected in `|| true` in ROWS.
#
# -n prints the runs in order, one "round<TAB>arm<TAB>row" line each,
# and runs nothing.
#
# Refused (exit 2, before any run): a missing or unknown option or
# operand, ROUNDS or WIDTH not a positive number, CORE not a CPU list
# (digits, commas and dashes, as taskset -c takes), no -o without -n, a
# -g GROUP that is empty or holds a space or tab, -g with other than two
# events (from -e, -E or the default), an empty event, an
# ARMS line that is empty or holds a space or tab, an
# arm named twice, a ROWS line without a tab, an empty row or command, a
# row named twice, and an ARMS or ROWS file with no lines; -e with -E,
# -f or -a without -E, an empty SET or ARCH, a missing FILE, a FILE line
# that is not three non-empty tab-separated fields or whose ARCH or SET
# holds a space, an ARCH and SET on two lines, and no line for SET and
# ARCH. -n checks the same, so it is a dry run of -E's lookup.
set -eu

usage() {
	echo "rounds.sh: $1" >&2
	echo "usage: rounds.sh -o RAW [-g GROUP] [-r ROUNDS] [-e EVENTS | -E SET [-f FILE] [-a ARCH]] [-c CORE] [-w WIDTH] ARMS ROWS" >&2
	echo "       rounds.sh -n [-r ROUNDS] [-E SET [-f FILE] [-a ARCH]] ARMS ROWS" >&2
	exit 2
}

number() {
	case $2 in
	'' | *[!0-9]*) usage "$1 must be a number, got '$2'" ;;
	esac
}

raw='' group='' grouped=0 rounds=5 events=instructions:u,cycles:u core='' width=60 dry=0
eset=0 set='' efile=$(dirname "$0")/events.tsv fset=0 arch='' aset=0 lit=0
while getopts o:g:r:e:E:f:a:c:w:n opt; do
	case $opt in
	o) raw=$OPTARG ;;
	g) group=$OPTARG grouped=1 ;;
	r) rounds=$OPTARG ;;
	e) events=$OPTARG lit=1 ;;
	E) set=$OPTARG eset=1 ;;
	f) efile=$OPTARG fset=1 ;;
	a) arch=$OPTARG aset=1 ;;
	c) core=$OPTARG ;;
	w) width=$OPTARG ;;
	n) dry=1 ;;
	*) usage "unknown option" ;;
	esac
done
shift $((OPTIND - 1))
[ $# -eq 2 ] || usage "want ARMS and ROWS, got $# operands"
arms=$1 rows=$2
number -r "$rounds"
[ "$rounds" -gt 0 ] || usage "-r must be positive"
number -w "$width"
[ "$width" -gt 0 ] || usage "-w must be positive"
case $core in
*[!0-9,-]*) usage "-c must be a CPU list (digits, commas, dashes), got '$core'" ;;
esac
if [ "$dry" = 0 ] && [ -z "$raw" ]; then
	usage "-o is required without -n"
fi
tab=$(printf '\t')
if [ "$grouped" = 1 ]; then
	case $group in
	'' | *' '* | *"$tab"*) usage "-g GROUP is one non-empty word" ;;
	esac
fi
if [ "$eset" = 1 ]; then
	[ "$lit" = 0 ] || usage "-e and -E together"
	[ -n "$set" ] || usage "-E SET is empty"
	if [ "$aset" = 0 ]; then
		arch=$(uname -m) || usage "uname -m failed; give -a ARCH"
	fi
	[ -n "$arch" ] || usage "ARCH is empty"
	[ -f "$efile" ] || usage "no events file $efile"
	# Check every line of the file, then take SET's for ARCH.
	n=0 seen="$tab" found=''
	while IFS= read -r l || [ -n "$l" ]; do
		n=$((n + 1))
		case $l in
		'' | '#'*) continue ;;
		*"$tab"*"$tab"*"$tab"*) usage "$efile:$n: want ARCH<TAB>SET<TAB>EVENTS" ;;
		*"$tab"*"$tab"*) ;;
		*) usage "$efile:$n: want ARCH<TAB>SET<TAB>EVENTS" ;;
		esac
		fa=${l%%"$tab"*} rest=${l#*"$tab"}
		fs=${rest%%"$tab"*} fe=${rest#*"$tab"}
		if [ -z "$fa" ] || [ -z "$fs" ] || [ -z "$fe" ]; then
			usage "$efile:$n: want ARCH<TAB>SET<TAB>EVENTS"
		fi
		case $fa$fs in
		*' '*) usage "$efile:$n: ARCH and SET are one word each" ;;
		esac
		case $seen in
		*"$tab$fa $fs$tab"*) usage "$efile:$n: $fa $fs named twice" ;;
		esac
		seen="$seen$fa $fs$tab"
		if [ "$fa" = "$arch" ] && [ "$fs" = "$set" ]; then
			found=$fe
		fi
	done < "$efile"
	[ -n "$found" ] || usage "no $set events for $arch in $efile"
	events=$found
else
	if [ "$fset" = 1 ] || [ "$aset" = 1 ]; then
		usage "-f and -a need -E"
	fi
fi
case ,$events, in
*,,*) usage "empty event in '$events'" ;;
esac
# stats reads two counts then the output, so a wider line under -g
# would put counts into the output.
if [ "$grouped" = 1 ]; then
	case $events in
	*,*,*) usage "-g writes stats's layout, which takes two events; got '$events'" ;;
	*,*) ;;
	*) usage "-g writes stats's layout, which takes two events; got '$events'" ;;
	esac
fi
[ -f "$arms" ] || usage "no file $arms"
[ -f "$rows" ] || usage "no file $rows"

# Check both files before anything runs.
n=0 seen=' '
while IFS= read -r a || [ -n "$a" ]; do
	n=$((n + 1))
	case $a in
	'' | *' '* | *"$tab"*) usage "$arms:$n: an arm is one non-empty word" ;;
	esac
	case $seen in
	*" $a "*) usage "$arms:$n: arm '$a' named twice" ;;
	esac
	seen="$seen$a "
done < "$arms"
[ $n -gt 0 ] || usage "$arms has no arms"
narms=$n
n=0 seen="$tab"
while IFS= read -r l || [ -n "$l" ]; do
	n=$((n + 1))
	case $l in
	*"$tab"*) ;;
	*) usage "$rows:$n: want row<TAB>command" ;;
	esac
	r=${l%%"$tab"*} c=${l#*"$tab"}
	if [ -z "$r" ] || [ -z "$c" ]; then
		usage "$rows:$n: empty row or command"
	fi
	case $seen in
	*"$tab$r$tab"*) usage "$rows:$n: row '$r' named twice" ;;
	esac
	seen="$seen$r$tab"
done < "$rows"
[ $n -gt 0 ] || usage "$rows has no rows"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failed=0
round=1
while [ "$round" -le "$rounds" ]; do
	i=0
	while [ $i -lt "$narms" ]; do
		k=$(((i + round) % narms + 1))
		arm=$(sed -n "${k}p" "$arms")
		while IFS= read -r l || [ -n "$l" ]; do
			r=${l%%"$tab"*} c=${l#*"$tab"}
			if [ "$dry" = 1 ]; then
				printf '%s\t%s\t%s\n' "$round" "$arm" "$r"
				continue
			fi
			rc=0
			if [ -n "$core" ]; then
				ARM=$arm ROUND=$round perf stat -x, -e "$events" -o "$tmp/p.csv" -- \
					taskset -c "$core" sh -c "$c" > "$tmp/o.txt" 2>&1 < /dev/null || rc=$?
			else
				ARM=$arm ROUND=$round perf stat -x, -e "$events" -o "$tmp/p.csv" -- \
					sh -c "$c" > "$tmp/o.txt" 2>&1 < /dev/null || rc=$?
			fi
			[ -f "$tmp/p.csv" ] || : > "$tmp/p.csv"
			# Each event's count: the first line whose third field it is.
			counts=$(awk -F, -v ev="$events" '
				BEGIN { n = split(ev, e, ",") }
				{ for (i = 1; i <= n; i++) if ($3 == e[i] && !(i in v)) v[i] = $1 }
				END { for (i = 1; i <= n; i++) printf "\t%s", v[i] }' "$tmp/p.csv")
			o=$(tr '\t\n' '  ' < "$tmp/o.txt" | cut -c "1-$width")
			if [ "$grouped" = 1 ]; then
				printf '%s\t' "$group" >> "$raw"
			fi
			printf '%s\t%s\t%s%s\t%s\n' "$r" "$arm" "$round" "$counts" "$o" >> "$raw"
			rm -f "$tmp/p.csv"
			if [ $rc -ne 0 ]; then
				echo "rounds.sh: round $round arm $arm row $r exited $rc" >&2
				failed=$((failed + 1))
			fi
		done < "$rows"
		i=$((i + 1))
	done
	round=$((round + 1))
done
if [ $failed -gt 0 ]; then
	echo "rounds.sh: $failed runs exited non-zero" >&2
	exit 1
fi
