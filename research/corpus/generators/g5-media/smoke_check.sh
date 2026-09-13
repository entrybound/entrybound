#!/usr/bin/env bash
# g5-media smoke test (tuning/validation items only; never touches held-out):
# every file in the JPEG matrix items must decode with djpeg (except CMYK/YCCK -> checked via ImageMagick identify),
# download items: list file type counts via libmagic.
set -uo pipefail
C=/root/eb-research/corpus
for it in $C/tuning/f12-tuning-kodak-jpeg-matrix-medium $C/validation/f12-validation-fsa-jpeg-matrix-medium; do
  bad=0; n=0
  while IFS= read -r -d '' f; do
    n=$((n+1))
    if ! djpeg -outfile /dev/null "$f" 2>/dev/null; then
      if ! identify -regard-warnings "$f" >/dev/null 2>&1; then bad=$((bad+1)); echo "UNDECODABLE $f"; fi
    fi
  done < <(find "$it" -type f -print0)
  echo "$(basename $it): $n files, $bad undecodable"
done
for it in $C/tuning/f12-tuning-kodak-jpeg-edge-cases-small $C/validation/f12-validation-fsa-jpeg-edge-cases-small; do
  ok=0; warn=0; n=0
  while IFS= read -r -d '' f; do
    n=$((n+1))
    if djpeg -outfile /dev/null "$f" >/dev/null 2>/tmp/dj.err; then
      if [[ -s /tmp/dj.err ]]; then warn=$((warn+1)); else ok=$((ok+1)); fi
    fi
  done < <(find "$it" -type f -print0)
  echo "$(basename $it): $n files, $ok clean-decode, $warn decode-with-warning, $((n-ok-warn)) djpeg-error (expected for malformed/CMYK cases)"
done
for it in $(ls -d $C/tuning/f1[23]-* $C/validation/f1[23]-*); do
  echo "== $(basename $it): $(find $it -type f | wc -l) files"
  find "$it" -type f -print0 | xargs -0 file -b --mime-type | sort | uniq -c | sort -rn | head -8
done
