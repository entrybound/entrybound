#!/usr/bin/env bash
# Diagnostic (metadata only): count unwritten extents and SEEK_DATA extents of sparse image files in
# g4-binary tuning/validation F16 items and scratch files given as arguments.
for f in "$@"; do
  [[ -f $f ]] || { echo "missing $f"; continue; }
  total=$(filefrag -v "$f" 2>/dev/null | grep -cE '^ +[0-9]+:')
  unwritten=$(filefrag -v "$f" 2>/dev/null | grep -c unwritten)
  seek=$(/root/eb-research/venv/bin/python -c 'import os,sys
fd=os.open(sys.argv[1],os.O_RDONLY); n=os.fstat(fd).st_size; o=0; ex=0; b=0
while o<n:
  try: d=os.lseek(fd,o,os.SEEK_DATA)
  except OSError: break
  h=os.lseek(fd,d,os.SEEK_HOLE); ex+=1; b+=h-d; o=h
print(ex, b)' "$f")
  echo "$(basename "$f"): filefrag_extents=$total unwritten=$unwritten seek_data_extents/bytes=$seek"
done
