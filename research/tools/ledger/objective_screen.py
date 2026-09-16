#!/usr/bin/env python3
"""Screen requirement-ledger INVARIANT/REQUIREMENT rows against the hard constraints (HC-xx) and
optimization dimensions (OD-xx) of research/archetypal-objective.md.

Generates Appendix C of that document. Standard library only; deterministic for fixed inputs.

    C:/Python313/python.exe research/tools/ledger/objective_screen.py [--repo DIR] [--json OUT]

Method (stated in archetypal-objective.md Appendix C):
  1. Explicit references: I1-I31 in row text, plus the SPEC aliases P1-P8, C1-C2, E1-E3 (not T1-T2; see ALIAS),
     are mapped to I-numbers and then to HCs through I2HC (the Appendix A table).
  2. Keyword screen: HC_KW regular expressions over row text (case-insensitive).
  3. Manual assignments (MANUAL) for rows that 1-2 miss, each assigned by reading the row, and
     EXCLUDE for keyword false positives found on review.
  4. Rows with no HC tag are screened against OD_KW.
The keyword part is a heuristic screen, not an authoritative classification.
"""
import argparse
import collections
import csv
import hashlib
import json
import re
import sys

# SPEC aliases T1/T2 (now I30/I31) are deliberately not screened: in the ledger "T1" names the
# crypto-v1 tuple transcript encoding (docs/crypto-suite-v1.md), so every T1 match is a false positive.
ALIAS = {"P1": ["I5"], "P2": ["I4"], "P3": ["I4"], "P4": ["I3", "I6"], "P5": ["I4"],
         "P6": ["I26"], "P7": ["I27"], "P8": ["I4"], "C1": ["I29"], "C2": ["I28"],
         "E1": ["I1"], "E2": ["I7"], "E3": ["I8", "I28"]}

I2HC = {
    1: ["HC-01"], 2: ["HC-01"], 3: ["HC-17", "HC-03"], 4: ["HC-08", "HC-17"], 5: ["HC-17", "HC-08"],
    6: ["HC-17", "HC-03"], 7: ["HC-10"], 8: ["HC-10", "HC-13"], 9: ["HC-10", "HC-13", "HC-14"],
    10: ["HC-01"], 11: ["HC-07", "HC-04"], 12: ["HC-04"], 13: ["HC-07"], 14: ["HC-02", "HC-07"],
    15: ["HC-03", "HC-11", "HC-16"], 16: ["HC-05"], 17: ["HC-06"], 18: ["HC-07"], 19: ["HC-01", "HC-06"],
    20: ["HC-06"], 21: ["HC-15", "HC-04"], 22: ["HC-10", "HC-16"], 23: ["HC-01", "HC-17"], 24: ["HC-14"],
    25: ["HC-15"], 26: ["HC-17", "HC-03"], 27: ["HC-17", "HC-08"], 28: ["HC-10", "HC-02"],
    29: ["HC-09", "HC-06"], 30: ["HC-11", "HC-12", "HC-16"], 31: ["HC-02"],
}

HC_KW = {
    "HC-01": r"single[- ]authorit|one authorit|exactly one authorit|declared (exactly )?once|non-authoritative|second authorit|duplicated semantic header|never restate|group_ref|member list",
    "HC-02": r"byte-for-byte|byte-exact|lossless|round[- ]trip|lossy transform|byte equality",
    "HC-03": r"one interpretation|unique interpretation|canonical (form|serialis|serializ|order|encoding)|parser differential|strict pars|noncanonical|admissible",
    "HC-04": r"fail[s]?[- ]closed|unknown (critical|codec|feature|record|incompat|bit|stanza)|reserved (bit|field)|closed regist",
    "HC-05": r"extraction policy|extractionpolicy|caller[- ](owned|supplied|policy|provided|decides|constructed)|policy[- ]gated|setuid|confine|containment|openat2|no-follow|symlink escape|toctou|junction",
    "HC-06": r"budget|resource (limit|bound|declar)|before allocat|expansion ratio|decompression bomb|kdf cost|memory-hard|budgetdeclared|unbounded",
    "HC-07": r"fidelity|silent|fidelityreport|extractionreport|\blossy\b|omission|what (it|was) (dropped|restored|captured)|not preserved|ambiguit",
    "HC-08": r"case[- ]fold|nfc|nfd|reserved name|drive[- ]letter|\bunc\b|backslash|hostil|target filesystem|host filesystem|collision class",
    "HC-09": r"lookback|dependency (closure|chain|chunk)|independently decodable|external dependency|base archive|transitive",
    "HC-10": r"\blai\b|\bpcr\b|\baux\b|merkle|identity profile|logical_digest|chunk_root|identity_digest|descriptor digest",
    "HC-11": r"plans are data|decoder behavio|decoders never|thread count|locale|registry-gated|compile-time regist|never the planner",
    "HC-12": r"independent(ly)? implement|without entrybound source|conformance corpus|test vectors|minimal reader|specification and (test )?vectors",
    "HC-13": r"determinis|reproducib|byte-identical|source_date_epoch",
    "HC-14": r"aead|nonce|key commitment|commitment|hkdf|x-wing|argon2|constant[- ]time|zeroi[sz]|unauthenticated|post-quantum|cryptographic",
    "HC-15": r"unverified|not_verified|whole_archive_verified|truncat|distinguishab|salvage",
    "HC-16": r"frozen|permanent|byte-for-byte unchanged|historical bytes|new planner|new (layout )?feature bit|never reused",
    "HC-17": r"traversal|dot-dot|absolute path|duplicate (logical )?path|duplicate[- ]name|file-as-ancestor|proper prefix|explicit directory|hardlink is not|hard links are content",
}

OD_KW = {
    "OD-01": r"single authorit|one authorit|coheren|one interpretation|canonical",
    "OD-02": r"lossless|byte-exact|round[- ]trip|reconstruct",
    "OD-03": r"fidelity|preserv|xattr|acl|timestamp|ownership|sparse|hardlink",
    "OD-04": r"determinis|interpretation|differential|reason code",
    "OD-05": r"ratio|compress|codec|dictionar|dedup|chunk size|storage|padding overhead",
    "OD-06": r"pack time|creation|encode (speed|time|throughput)|write time|planner (cost|time|search)",
    "OD-07": r"decode (speed|time|throughput)|decompress|unpack time|extraction (speed|time)",
    "OD-08": r"memory|rss|scratch|spill|working set",
    "OD-09": r"stream",
    "OD-10": r"random access|random-access|seek|access cost|granularity",
    "OD-11": r"partial|range verif|verified range|merkle slice|bao",
    "OD-12": r"remote|http|range request|etag|cdn|lazy",
    "OD-13": r"parallel|thread|multicore|concurren",
    "OD-14": r"corruption|damage|blast radius|salvage|recovery|parity|truncat",
    "OD-15": r"encrypt|authenticat|signature|aead|recipient|key commit",
    "OD-16": r"leak|privacy|padding|metadata exposure|traffic",
    "OD-17": r"budget|bomb|resource|allocation|kdf cost|limit",
    "OD-18": r"platform|windows|macos|linux|ntfs|apfs|ext4|cross-platform|case-insensitive",
    "OD-19": r"\bzip\b|\btar\b|7z|legacy|import|export|compatib",
    "OD-20": r"migrat|repack|publish|conversion receipt|convert",
    "OD-21": r"independent (reader|implement)|conformance|specification|test vector",
    "OD-22": r"dependenc|crate|license|maintain|longevity|thirty years|30 years|decoder longevity",
    "OD-23": r"spec(ification)? (size|complexity|words)|record type|feature bit|wire cost|complexity",
    "OD-24": r"attack surface|fuzz|unsafe|parser|cve|hostile",
    "OD-25": r"cli|user|usabil|default|flag|error message|diagnostic",
    "OD-26": r"adopt|ecosystem|mime|media type|extension|binding|registration|switching",
}

MANUAL = {
    "REQ-ACC-0128": ["HC-15", "HC-03"], "REQ-ACC-0129": ["HC-03", "HC-01"], "REQ-ACC-0138": ["HC-15", "HC-05"],
    "REQ-ACC-0161": ["HC-09", "HC-03"], "REQ-CMP-0018": ["HC-01", "HC-03"], "REQ-CMP-0038": ["HC-08", "HC-13"],
    "REQ-CMP-0106": ["HC-10"], "REQ-CMP-0146": ["HC-01", "HC-11"], "REQ-CMP-0153": ["HC-02", "HC-11"],
    "REQ-CRY-0040": ["HC-04", "HC-14"], "REQ-CRY-0129": ["HC-10", "HC-14"], "REQ-CRY-0199": ["HC-14"],
    "REQ-CRY-0225": ["HC-14"], "REQ-CRY-0247": ["HC-09", "HC-14"], "REQ-ECO-0021": ["HC-12", "HC-03"],
    "REQ-INT-0005": ["HC-10", "HC-01"], "REQ-INT-0036": ["HC-10", "HC-07"], "REQ-INT-0037": ["HC-07", "HC-02"],
    "REQ-INT-0054": ["HC-15"], "REQ-LEG-0096": ["HC-07", "HC-01"], "REQ-LEG-0126": ["HC-05", "HC-01"],
    "REQ-LEG-0128": ["HC-13", "HC-01"], "REQ-LEG-0152": ["HC-07", "HC-03"], "REQ-LEG-0171": ["HC-07"],
    "REQ-LEG-0183": ["HC-17"], "REQ-LEG-0184": ["HC-07", "HC-03"], "REQ-LEG-0205": ["HC-08", "HC-07"],
    "REQ-MOD-0033": ["HC-17", "HC-01"], "REQ-MOD-0034": ["HC-16"], "REQ-MOD-0081": ["HC-10"],
    "REQ-PLT-0002": ["HC-03", "HC-07"], "REQ-PLT-0030": ["HC-15", "HC-05"], "REQ-PLT-0071": ["HC-07", "HC-05"],
    "REQ-PLT-0110": ["HC-05"], "REQ-PLT-0144": ["HC-04"], "REQ-CRY-0155": ["HC-14", "HC-06"],
    "REQ-CRY-0085": ["HC-14"], "REQ-INT-0003": ["HC-02", "HC-15"], "REQ-INT-0027": ["HC-15"],
    "REQ-INT-0032": ["HC-15"], "REQ-PLT-0031": ["HC-05"], "REQ-PLT-0034": ["HC-05"],
    "REQ-ACC-0122": ["OD-11", "OD-14"], "REQ-ACC-0125": ["OD-25"], "REQ-CMP-0099": ["OD-25"],
    "REQ-ECO-0108": ["OD-23"], "REQ-INT-0069": ["OD-25", "HC-15"],
    "REQ-ECO-0184": ["METHOD"], "REQ-ECO-0185": ["METHOD"],
}

EXCLUDE = {("REQ-CRY-0111", "HC-02"), ("REQ-CRY-0111", "HC-08"), ("REQ-MOD-0034", "HC-02"),
           ("REQ-CRY-0126", "HC-05"), ("REQ-PLT-0099", "HC-05"), ("REQ-MOD-0111", "HC-02")}

WEAK_EVIDENCE = ("CONTRADICTED", "INSUFFICIENT")


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def explicit_invariants(text):
    refs = set()
    for m in re.finditer(r"(?<![A-Za-z0-9_\-])I([1-9]|[12][0-9]|3[01])(?![0-9])", text):
        refs.add(int(m.group(1)))
    for m in re.finditer(r"(?<![A-Za-z0-9_\-])(P[1-8]|C[12]|E[123])(?![0-9A-Za-z])", text):
        refs.update(int(i[1:]) for i in ALIAS[m.group(1)])
    return refs


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--repo", default=".")
    ap.add_argument("--json", help="also write the full row-level mapping as JSON")
    args = ap.parse_args()
    req_path = f"{args.repo}/research/requirement-ledger.csv"
    dec_path = f"{args.repo}/research/decision-ledger.jsonl"
    with open(req_path, encoding="utf-8", newline="") as f:
        rows = [r for r in csv.DictReader(f) if r["kind"] in ("INVARIANT", "REQUIREMENT")]
    with open(dec_path, encoding="utf-8") as f:
        decisions = [json.loads(line) for line in f if line.strip()]

    inv_rows = collections.defaultdict(set)
    mapping = {}
    for r in rows:
        rid, text = r["req_id"], r["requirement_or_question"]
        tags = set()
        for i in explicit_invariants(text):
            inv_rows[i].add(rid)
            tags.update(I2HC[i])
        tags.update(h for h, pat in HC_KW.items() if re.search(pat, text, re.I))
        tags.update(MANUAL.get(rid, []))
        tags = {t for t in tags if (rid, t) not in EXCLUDE}
        if not any(t.startswith("HC-") for t in tags):
            tags.update(o for o, pat in OD_KW.items() if re.search(pat, text, re.I))
        mapping[rid] = (r, sorted(tags))

    inv_decisions = collections.Counter()
    for d in decisions:
        for h in d.get("hard_constraints", []):
            if re.fullmatch(r"I([1-9]|[12][0-9]|3[01])", h):
                inv_decisions[int(h[1:])] += 1

    out = []
    out.append(f"Inputs: `research/requirement-ledger.csv` SHA-256 `{sha256(req_path)}`; "
               f"`research/decision-ledger.jsonl` SHA-256 `{sha256(dec_path)}`.")
    out.append("")
    out.append("C.1 Explicit invariant references (rows whose text names the invariant or its SPEC alias) "
               "and decision-ledger rows listing the invariant in `hard_constraints`.")
    out.append("")
    out.append("| Invariant | Ledger rows naming it | Decisions citing it | HCs (Appendix A) |")
    out.append("|---|---|---|---|")
    for i in range(1, 32):
        ids = ", ".join(sorted(inv_rows[i])) or "none"
        out.append(f"| I{i} | {ids} | {inv_decisions[i]} | {', '.join(I2HC[i])} |")
    out.append("")
    out.append("C.2 HC screen. Weak = evidence_state CONTRADICTED or INSUFFICIENT, or implementation_state MISSING.")
    out.append("")
    out.append("| HC | Rows (INVARIANT / REQUIREMENT) | INVARIANT rows | Weak rows |")
    out.append("|---|---|---|---|")
    by_hc = collections.defaultdict(list)
    od_only = collections.Counter()
    method = []
    for rid, (r, tags) in sorted(mapping.items()):
        has_hc = any(t.startswith("HC-") for t in tags)
        for t in tags:
            if t.startswith("HC-"):
                by_hc[t].append(r)
            elif t.startswith("OD-") and not has_hc:
                od_only[t] += 1
        if "METHOD" in tags:
            method.append(rid)
    for h in sorted(by_hc):
        rs = by_hc[h]
        inv = [r["req_id"] for r in rs if r["kind"] == "INVARIANT"]
        weak = [r["req_id"] for r in rs
                if r["evidence_state"] in WEAK_EVIDENCE or r["implementation_state"] == "MISSING"]
        out.append(f"| {h} | {len(inv)} / {len(rs) - len(inv)} | {', '.join(inv)} | {len(weak)}: {', '.join(weak)} |")
    out.append("")
    out.append("C.3 REQUIREMENT rows with no HC tag, screened to ODs (a row may match several ODs).")
    out.append("")
    out.append("| OD | Rows |")
    out.append("|---|---|")
    for o in sorted(OD_KW):
        out.append(f"| {o} | {od_only[o]} |")
    total_hc = sum(1 for _, (r, t) in mapping.items() if any(x.startswith("HC-") for x in t))
    total_od = sum(1 for _, (r, t) in mapping.items()
                   if t and not any(x.startswith("HC-") for x in t) and "METHOD" not in t)
    untagged = sorted(rid for rid, (r, t) in mapping.items() if not t)
    inv_no_hc = sorted(rid for rid, (r, t) in mapping.items()
                       if r["kind"] == "INVARIANT" and not any(x.startswith("HC-") for x in t))
    out.append("")
    out.append(f"METHOD rows (obligations on the research method, discharged by this document and "
               f"`decision-method.md`): {', '.join(method)}.")
    out.append("")
    out.append(f"Totals: {len(mapping)} rows screened ({sum(1 for r in rows if r['kind'] == 'INVARIANT')} INVARIANT, "
               f"{sum(1 for r in rows if r['kind'] == 'REQUIREMENT')} REQUIREMENT); {total_hc} carry at least one HC tag; "
               f"{total_od} carry only OD tags; {len(method)} METHOD; {len(untagged)} untagged; "
               f"INVARIANT rows without an HC tag: {len(inv_no_hc)}.")
    sys.stdout.write("\n".join(out) + "\n")
    if args.json:
        with open(args.json, "w", encoding="utf-8", newline="\n") as f:
            json.dump({rid: tags for rid, (r, tags) in sorted(mapping.items())}, f, indent=1, sort_keys=True)
            f.write("\n")
    return 1 if (untagged or inv_no_hc) else 0


if __name__ == "__main__":
    sys.exit(main())
