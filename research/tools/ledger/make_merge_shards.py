"""Build compact, topic-sharded merge inputs for Phase A ledger merging.

Reads research/audit/extract/*.jsonl and, for each cluster still awaiting a
merge, assigns every primary record to exactly one topic shard (ordered regex
rules over statement and tags; first match wins, the last rule is the
default). Writes one compact text file per shard, one record per line, which
keeps merge-agent contexts small:

  <local_key> | <kind> | <source>@<location> | <release> | <statement>
      [|| R: research_needed] [|| D: decision_needed] [|| I: implementation_hint]
      [|| E: evidence_in_source] [|| S: refines_or_supersedes] [|| T: tags]

Also writes shards.json mapping shard -> record keys, so coverage can be
verified mechanically after merging. Outputs are derived and not committed.

Usage: python research/tools/ledger/make_merge_shards.py [--clusters c1,c2]
"""

import argparse
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[2]
EXTRACT = ROOT / "audit" / "extract"
OUT = ROOT / "audit" / "by-cluster" / "shards"

I = re.IGNORECASE
RULES = {
    "container": [
        ("container-index-manifest", re.compile(r"manifest|index|merkle|outboard|proof|section|footer|preamble|locator|segment", I)),
        ("container-evolution", re.compile(r".", I)),
    ],
    "compression": [
        ("compression-chunking", re.compile(r"chunk(ing|er)|cdc|gear|dedup|similar|cohort|bottom-k|shingle|dictionar|lookback|chunkgroup|group", I)),
        ("compression-codecs-planner", re.compile(r".", I)),
    ],
    "platform": [
        ("platform-paths-extraction", re.compile(r"path|filename|name|unicode|utf|case|normaliz|reserved|travers|confine|extract|collision|symlink|junction|reparse|capabilit", I)),
        ("platform-metadata", re.compile(r".", I)),
    ],
    "crypto": [
        ("crypto-signatures-trust", re.compile(r"signat|sign(er|ing)|ebsig|ed25519|timestamp|rfc ?3161|tsa|trust|binding|stale|forward", I)),
        ("crypto-keys-recipients", re.compile(r"recipient|x-?wing|kem|password|argon|identity file|ebk|key[- ]management|rotat|afk|kdf|hkdf|commit", I)),
        ("crypto-leakage-privacy", re.compile(r"pad|leak|boundar|phte|secret gear|keyed|privacy|metadata|size|length|traffic", I)),
        ("crypto-suite-wire", re.compile(r".", I)),
    ],
    "legacy": [
        ("legacy-export-migration", re.compile(r"export|target[- ]profile|lossy|refused|publish|sidecar|receipt|migration|provenance|adapter priority|\boci\b|\bnar\b|rar5?|cpio|wim|squashfs|lzip|lz4 frame", I)),
        ("legacy-zip", re.compile(r"\bzip|central director|local header|zip64|deflate|crc", I)),
        ("legacy-tar-7z-streams", re.compile(r".", I)),
    ],
    "ecosystem": [
        ("ecosystem-conformance-deps", re.compile(r"conformance|corpus|vector|independent|minimal reader|fuzz|depend|crate|licen|longevity|msrv|unsafe|incumbent|benchmark", I)),
        ("ecosystem-cli-adoption", re.compile(r".", I)),
    ],
}


def compact(record):
    head = " | ".join([
        record["local_key"],
        record.get("kind", ""),
        f"{record.get('source', '')}@{record.get('location', '')}",
        record.get("release_relevance", ""),
        " ".join(record.get("statement", "").split()),
    ])
    parts = [head]
    for label, field, skip in (
        ("R", "research_needed", {""}),
        ("D", "decision_needed", {""}),
        ("I", "implementation_hint", {"", "N/A", "UNKNOWN"}),
        ("E", "evidence_in_source", {"", "none", "None"}),
        ("S", "refines_or_supersedes", {""}),
    ):
        value = " ".join(str(record.get(field, "")).split())
        if value not in skip:
            parts.append(f"{label}: {value}")
    tags = record.get("tags", [])
    if tags:
        parts.append("T: " + ",".join(tags))
    return " || ".join(parts)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--clusters", default=",".join(RULES))
    args = parser.parse_args()
    wanted = [c for c in args.clusters.split(",") if c]

    records = []
    for path in sorted(EXTRACT.glob("*.jsonl")):
        with path.open(encoding="utf-8") as handle:
            records.extend(json.loads(line) for line in handle if line.strip())

    OUT.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for cluster in wanted:
        buckets = {name: [] for name, _ in RULES[cluster]}
        for record in records:
            if record.get("primary_cluster") != cluster:
                continue
            haystack = " ".join([record.get("statement", "")] + record.get("tags", []))
            for name, pattern in RULES[cluster]:
                if pattern.search(haystack):
                    buckets[name].append(record)
                    break
        for name, rows in buckets.items():
            text = "\n".join(compact(r) for r in rows) + "\n"
            (OUT / f"{name}.txt").write_text(text, encoding="utf-8", newline="\n")
            manifest[name] = {
                "cluster": cluster,
                "records": len(rows),
                "bytes": len(text.encode("utf-8")),
                "keys": [r["local_key"] for r in rows],
            }
    (OUT / "shards.json").write_text(json.dumps(manifest, indent=1) + "\n", encoding="utf-8")
    for name, info in manifest.items():
        print(f"{name:32} {info['cluster']:12} records={info['records']:5} kb={info['bytes'] // 1024}")


if __name__ == "__main__":
    main()
