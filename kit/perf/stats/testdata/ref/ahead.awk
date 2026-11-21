# ahead.awk med.tsv: the reference tallies (stats ahead -pairs
# A1:W1,B1:W1,C1:W1,D1:W1,A0:W0,B0:W0,C0:W0,D0:W0 -skip none-1; the awk
# also skips none, which the fixture has no row of). Per group, the rows
# where each arm (suffix 1 or 0) is ahead of the reference W1 or W0 on
# instructions and on cycles. Prints group, arm, rows, ahead insns, ahead
# cycles. Unsorted; gen.sh sorts.
BEGIN { FS=OFS="\t" }
{ I[$1,$2,$3]=$4; C[$1,$2,$3]=$5; R[$1,$2]=1; G[$1]=1 }
END { split("A1 B1 C1 D1 A0 B0 C0 D0", A, " ")
  for (g in G) for (i=1;i<=8;i++) { a=A[i]; w=(a ~ /0$/)?"W0":"W1"; n=0; ni=0; nc=0
    for (k in R) { split(k, q, SUBSEP); if (q[1]!=g || q[2]=="none-1" || q[2]=="none" || !((g,q[2],a) in I) || !((g,q[2],w) in I)) continue
      n++; if (I[g,q[2],a]<I[g,q[2],w]) ni++; if (C[g,q[2],a]<C[g,q[2],w]) nc++ }
    if (n) print g, a, n, ni, nc } }
