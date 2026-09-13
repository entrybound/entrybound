#!/usr/bin/env bash
# Authoring helper: list first-parent commits of an upstream repo (blobless bare clone in the cache).
#   list_commits.sh <repo-url> <end-rev> <count>
set -euo pipefail
repo=$1; end=$2; n=$3
key=$(printf '%s' "$repo" | sha256sum | cut -c1-24)
d=/root/eb-research/cache/g1-code/authoring-git/$key.git
mkdir -p "$(dirname "$d")"
[[ -d $d ]] || git clone -q --bare --filter=blob:none "$repo" "$d"
git -C "$d" fetch -q --filter=blob:none origin '+refs/tags/*:refs/tags/*' '+refs/heads/*:refs/heads/*' || true
git -C "$d" log --first-parent --reverse -n "$n" --format='%H %cI %s' "$end"
