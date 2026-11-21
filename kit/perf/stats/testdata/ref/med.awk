# med.awk reps.tsv: the reference medians (stats med). Per (group, row,
# arm), the median instructions and cycles over the rounds, the lower
# middle value for an even count. Unsorted; gen.sh sorts.
BEGIN { FS = OFS = "\t" }
function med(s,  n,i,j,t,x) { n=split(s,x," "); for (i=1;i<=n;i++) x[i]+=0; for (i=1;i<=n;i++) for (j=i+1;j<=n;j++) if (x[j]<x[i]) {t=x[i];x[i]=x[j];x[j]=t}; return x[int((n+1)/2)] }
{ k=$1 OFS $2 OFS $3; I[k]=I[k]" "$5; C[k]=C[k]" "$6 }
END { for (k in I) printf "%s\t%.0f\t%.0f\n", k, med(I[k]), med(C[k]) }
