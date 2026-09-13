"""Split Phase A extraction records into per-cluster merge inputs.

For each cluster, writes research/audit/by-cluster/<cluster>.jsonl with every
record whose primary_cluster is that cluster, and <cluster>-secondary.jsonl
with records owned by other clusters whose tags overlap the cluster's
characteristic tags (tags whose occurrences are concentrated in that cluster).
Merge agents read these files instead of every extraction file; the output is
derived and regenerable, so it is not committed.

Usage: python research/tools/ledger/split_by_cluster.py
"""

import collections
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[2]
EXTRACT = ROOT / "audit" / "extract"
OUT = ROOT / "audit" / "by-cluster"
CLUSTERS = ["model", "container", "compression", "access", "platform", "crypto", "legacy", "integrity", "ecosystem"]
# A tag is characteristic of a cluster when at least this share of its
# occurrences carry that primary_cluster and it occurs at least MIN_COUNT times.
SHARE = 0.6
MIN_COUNT = 3
SHARDS = {
    "crypto": re.compile(
        r"signat|sign(er|ing)|ebsig|ed25519|timestamp|rfc ?3161|tsa|trust|recipient (add|remov)|key[- ]management|rotat|"
        r"change-password|stale|binding|forward|threat|adversar|review",
        re.IGNORECASE,
    ),
    "legacy": re.compile(
        r"export|target[- ]profile|lossy|refused|publish|sidecar|receipt|migration|provenance|adapter priority|"
        r"\boci\b|\bnar\b|rar5?|cpio|\bar\b|wim|squashfs|lzip|lz4 frame",
        re.IGNORECASE,
    ),
}


def main():
    records = []
    for path in sorted(EXTRACT.glob("*.jsonl")):
        with path.open(encoding="utf-8") as handle:
            for line in handle:
                if line.strip():
                    record = json.loads(line)
                    record["_file"] = path.name
                    records.append(record)

    tag_cluster = collections.defaultdict(collections.Counter)
    for record in records:
        for tag in record.get("tags", []):
            tag_cluster[tag.lower()][record.get("primary_cluster")] += 1
    characteristic = collections.defaultdict(set)
    for tag, counts in tag_cluster.items():
        total = sum(counts.values())
        cluster, count = counts.most_common(1)[0]
        if total >= MIN_COUNT and count / total >= SHARE:
            characteristic[cluster].add(tag)

    OUT.mkdir(parents=True, exist_ok=True)
    summary = {}
    for cluster in CLUSTERS:
        primary = [r for r in records if r.get("primary_cluster") == cluster]
        tags = characteristic[cluster]
        secondary = [
            r for r in records
            if r.get("primary_cluster") != cluster and tags.intersection(t.lower() for t in r.get("tags", []))
        ]
        for name, rows in ((f"{cluster}.jsonl", primary), (f"{cluster}-secondary.jsonl", secondary)):
            with (OUT / name).open("w", encoding="utf-8", newline="\n") as handle:
                for row in rows:
                    handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")
        summary[cluster] = {"primary": len(primary), "secondary": len(secondary), "characteristic_tags": len(tags)}
    # Oversized clusters are sharded deterministically so one merge agent never
    # has to hold more than about half a megabyte of records. Shard B takes
    # records whose statement or tags match the pattern; shard A takes the rest.
    for cluster, pattern in SHARDS.items():
        primary = [r for r in records if r.get("primary_cluster") == cluster]
        shard_b = [r for r in primary if pattern.search(" ".join([r.get("statement", "")] + r.get("tags", [])))]
        keys_b = {r["local_key"] for r in shard_b}
        shard_a = [r for r in primary if r["local_key"] not in keys_b]
        for suffix, rows in (("a", shard_a), ("b", shard_b)):
            with (OUT / f"{cluster}-{suffix}.jsonl").open("w", encoding="utf-8", newline="\n") as handle:
                for row in rows:
                    handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")
        summary[cluster]["shards"] = {"a": len(shard_a), "b": len(shard_b), "b_pattern": pattern.pattern}
    unknown = [r["local_key"] for r in records if r.get("primary_cluster") not in CLUSTERS]
    summary["_total_records"] = len(records)
    summary["_unknown_cluster_keys"] = unknown
    (OUT / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
