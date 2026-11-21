# cmp.awk -v T=ARM -v R=ARM reps.tsv: the reference comparison (stats cmp
# -t T -r R -per perpat=20000,perpat2=20000). T - R per (group, row).
# Columns: group, row, dI (median T - median R insns), dI per pattern
# (/20000 for the groups perpat and perpat2; else "-"), dI %, insns band
# %, dC per pattern (or dC), dC %, cycles band %, mark. The band is the
# larger of T's and R's half-split spread (three splits of the rounds:
# odd/even, first/second half, pairs; the largest |median(a) - median(b)|
# / median), floored at 0.02% for instructions. Mark: "I+"/"I-" when
# |dI %| exceeds its band, "C+"/"C-" likewise for cycles; "flat"
# otherwise. Unsorted; gen.sh sorts.
BEGIN { FS=OFS="\t" }
function med(s,  n,i,j,t,x) { n=split(s,x," "); for (i=1;i<=n;i++) x[i]+=0; for (i=1;i<=n;i++) for (j=i+1;j<=n;j++) if (x[j]<x[i]) {t=x[i];x[i]=x[j];x[j]=t}; return x[int((n+1)/2)] }
function ab(x) { return x<0 ? -x : x }
function bound(s,  n,i,x,a,b,m,best,d,h,k) { n=split(s,x," "); m=med(s); if (!m) return 0; best=0
  for (k=1;k<=3;k++) { a=""; b=""; for (i=1;i<=n;i++) { h=(k==1)?(i%2):((k==2)?(i<=n/2):(((i-1)%4)<2)); if (h) a=a" "x[i]; else b=b" "x[i] }; d=ab(med(a)-med(b))/ab(m); if (d>best) best=d }
  return best*100 }
$3==T || $3==R { k=$1 OFS $2; I[k,$3]=I[k,$3]" "$5; C[k,$3]=C[k,$3]" "$6; K[k]=$1 }
END { for (k in K) { if (I[k,T]=="" || I[k,R]=="") continue
    ti=med(I[k,T]); ri=med(I[k,R]); tc=med(C[k,T]); rc=med(C[k,R])
    bi=bound(I[k,T]); t=bound(I[k,R]); if (t>bi) bi=t; if (bi<0.02) bi=0.02
    bc=bound(C[k,T]); t=bound(C[k,R]); if (t>bc) bc=t
    di=ti-ri; dc=tc-rc; pi=ri?100*di/ri:0; pc=rc?100*dc/rc:0
    pp=(K[k] ~ /^(perpat|perpat2)$/); ip=pp ? sprintf("%+.1f", di/20000) : "-"; cp=pp ? sprintf("%+.1f", dc/20000) : sprintf("%+.0f", dc)
    mark=""; if (ab(pi)>bi) mark=mark (pi>0?"I+":"I-"); if (ab(pc)>bc) mark=mark (mark?" ":"") (pc>0?"C+":"C-"); if (!mark) mark="flat"
    printf "%s\t%+.0f\t%s\t%+.3f\t%.3f\t%s\t%+.3f\t%.3f\t%s\n", k, di, ip, pi, bi, cp, pc, bc, mark } }
