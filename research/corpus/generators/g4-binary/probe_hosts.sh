#!/usr/bin/env bash
# Authoring aid: check https availability of package hosts used by author_sources.py (no timing).
set -uo pipefail
for u in \
  https://archive.ubuntu.com/ubuntu/dists/noble/Release \
  https://dl.fedoraproject.org/pub/fedora/linux/releases/44/Everything/x86_64/os/repodata/repomd.xml \
  "https://archlinux.org/packages/search/json/?name=qt6-base" \
  https://archive.archlinux.org/packages/q/qt6-base/ \
  https://repo1.maven.org/maven2/org/jetbrains/kotlin/kotlin-compiler-embeddable/maven-metadata.xml \
  https://api.github.com/repos/uutils/coreutils/releases/latest \
  https://static.crates.io/crates/fd-find/fd-find-10.5.0.crate ; do
  code=$(curl -sS -o /dev/null -w '%{http_code} %{size_download}' --max-time 60 "$u" 2>&1)
  echo "$code $u"
done
curl -fsSL --max-time 60 "https://archlinux.org/packages/search/json/?name=qt6-base" | head -c 600; echo
curl -fsSL --max-time 60 https://api.github.com/repos/uutils/coreutils/releases/latest | grep -E '"(tag_name|name|size|digest)"' | grep -E 'tag_name|apple-darwin|windows-msvc|x86_64-unknown-linux' -A2 | head -40
