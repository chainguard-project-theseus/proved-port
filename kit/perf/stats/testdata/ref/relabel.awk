# relabel.awk -v MAP=map.tsv raw.tsv: the reference relabelling (stats
# relabel). Port A's pull did it with one sed rule per label; this reads
# the rules from MAP (label, tab, arm) and rewrites the third column, in
# file order. Valid input only: an unmapped label passes through.
BEGIN { FS = OFS = "\t"; while ((getline l < MAP) > 0) { split(l, m, "\t"); arm[m[1]] = m[2] } }
{ if ($3 in arm) $3 = arm[$3]; print }
