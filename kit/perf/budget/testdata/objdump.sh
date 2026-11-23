#!/bin/sh
# objdump.sh: the stand-in objdump of kit/perf-budget-test.sh's frozen
# case (PERF_BUDGET_OBJDUMP). It prints, for the flags the checker passes,
# GNU objdump's output for kit/perf/budget/fixture's default build as
# rustc 1.99.1 compiled it (the dumps trimmed to its four functions and
# their symbols), whatever library it is given, so table.txt's counts
# hold whatever compiler the check has.
d=$(dirname "$0")
case $1 in
  -f) printf '\nlibpbfixture.so:     file format elf64-x86-64\narchitecture: i386:x86-64, flags 0x00000150:\nHAS_SYMS, DYNAMIC, D_PAGED\nstart address 0x0000000000000000\n\n' ;;
  -T) cat "$d/objdump-T.txt" ;;
  -t) cat "$d/objdump-t.txt" ;;
  -d) cat "$d/objdump-d.txt" ;;
  *) echo "objdump.sh: unexpected arguments: $*" >&2; exit 2 ;;
esac
