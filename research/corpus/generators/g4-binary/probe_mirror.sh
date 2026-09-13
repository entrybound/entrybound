#!/usr/bin/env bash
# Authoring aid: check whether an alternative official mirror serves the same file (HTTP status and
# byte count of a 20 MB range within 20 s).  Feasibility only; not a benchmark.
for u in "$@"; do
  curl -s -o /dev/null -r 0-20000000 --max-time 20 -w '%{http_code} %{size_download} %{url_effective}\n' "$u"
done
