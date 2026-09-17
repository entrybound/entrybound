"""Fill the generated AUTO blocks of every EXP-CRYPTO-* protocol (design review DR1-07; amendment PA-07).

The crypto designer left `<!-- BEGIN AUTO:meta -->`, `AUTO:candidates` and `AUTO:common` blocks empty and no generator
existed, so the protocols lacked status, decision ids, per-decision candidate tables (status quo, SPEC design, ledger
alternatives, incumbent, defer, critic slot, exclusions) and the shared disclosure and gate provisions. This script
fills the three blocks deterministically from:

  research/experiments/_index/crypto.jsonl               status, decisions, platforms, estimates, tooling
  research/decision-ledger.jsonl                         question, candidate_set, candidate_exclusion_reasons
  research/experiments/EXP-*/protocol.md                 which candidate ids each protocol names (arm mapping)
  research/experiments/index.csv                         which experiments cover each decision
  research/experiments/_program/look-plan.csv            look ownership (optional)
  research/experiments/_index/crypto-conventions.md      referenced, not parsed

Only text between the BEGIN and END markers is replaced; the designer's prose is untouched.
Usage:  python research/tools/experiments/fill_crypto_auto_blocks.py [--check]
--check exits 1 if any block would change (use after editing inputs). Stdlib only, deterministic.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EXP = ROOT / "research" / "experiments"
FRAGMENT = EXP / "_index" / "crypto.jsonl"
LEDGER = ROOT / "research" / "decision-ledger.jsonl"
INDEX = EXP / "index.csv"
LOOK_PLAN = EXP / "_program" / "look-plan.csv"

BLOCK_RX = r"(<!-- BEGIN AUTO:{name} -->\n)(.*?)(<!-- END AUTO:{name} -->)"
DEFER_RX = re.compile(r"(?i)\b(defer|post[- ]?v1|never|not[- ]in[- ]v1|later|document[- ]only|non[- ]goal)\b")
INCUMBENT_RX = re.compile(r"(?i)\b(age|gpg|openpgp|restic|borg|kopia|7-?zip|7z|tarsnap|libsodium|tls|cosign|sigstore|minisign|signify|zip|jarsigner|in-toto|dsse|pkcs|hsm|keychain|keystore|scrypt|pbkdf2|wycheproof|acvp|rfc ?\d{3,5})\b")
SPEC_RX = re.compile(r"(?i)\bSPEC\b|as written|spec-")
RESPECIFY_RX = re.compile(r"(?i)re-?specify|remove from the default|normative|executable-matrix|vector-defined")


def load_jsonl(path):
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def protocol_text(eid):
    p = EXP / eid / "protocol.md"
    return p.read_text(encoding="utf-8") if p.is_file() else ""


def strip_auto(text):
    return re.sub(r"<!-- BEGIN AUTO:(\w+) -->.*?<!-- END AUTO:\1 -->", "", text, flags=re.S)


def names_candidate(text, cid):
    if f"`{cid}`" in text:
        return True
    if len(cid) >= 12 and re.search(r"(?<![\w-])" + re.escape(cid) + r"(?![\w-])", text):
        return True
    return False


def md(s):
    return str(s).replace("|", "\\|").replace("\n", " ")


def meta_block(rec, look_owned, look_part):
    specs = sorted(p.name for p in (EXP / rec["experiment_id"]).glob("spec*.yaml"))
    tooling = rec.get("tooling_to_build")
    tooling = "; ".join(tooling) if isinstance(tooling, list) else (tooling or "none")
    rows = [
        ("Index status", f"**{rec['status']}**" + (f" — {rec['blocked_reason']}" if rec.get("blocked_reason") else "")),
        ("Kind", rec.get("kind") or ("decision" if rec["decision_ids"] else "instrument (calibrates an instrument; informs no decision directly)")),
        ("Domain", "crypto (program §" + ", §".join(rec.get("program_sections", [])) + ")"),
        ("Decisions informed", ", ".join(rec["decision_ids"]) or "none (instrument)"),
        ("Platforms", rec.get("platforms", "")),
        ("Requires timing", str(rec.get("requires_timing"))),
        ("Estimates", f"{rec.get('compute_hours_estimate')} machine-hours; {rec.get('disk_gb_estimate')} GB disk"
         + (f" (peak {rec['disk_peak_gb']} GB, retained {rec.get('disk_retained_gb', '?')} GB)" if rec.get("disk_peak_gb") else "")),
        ("Tooling to build", tooling),
        ("Environment prerequisites", ", ".join(rec.get("requires_env", [])) or "none beyond the harness and the ebr venv"),
        ("Blocked arms", rec.get("blocked_arms") or "none"),
        ("Specs", ", ".join(f"`{s}`" for s in specs) or "none (non-runner experiment)"),
        ("Validation looks", (("owns look 1 for " + ", ".join(look_owned)) if look_owned else "owns no validation look")
         + (("; participates in looks owned by other experiments for " + ", ".join(look_part)) if look_part else "")),
        ("Gates", "G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04)"),
        ("Conventions", "`research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md`"),
    ]
    out = ["| Field | Value |", "|---|---|"]
    out += [f"| {k} | {md(v)} |" for k, v in rows]
    return "\n".join(out) + "\n"


def candidates_block(rec, ledger, cover, texts):
    eid = rec["experiment_id"]
    own = strip_auto(texts.get(eid, ""))
    if not rec["decision_ids"]:
        return ("This experiment informs no ledger decision directly (instrument or calibration). Its outputs are inputs "
                "to the decisions of the experiments that cite it; no candidate table applies.\n")
    parts = []
    for did in rec["decision_ids"]:
        d = ledger.get(did)
        if d is None:
            parts.append(f"#### {did}\n\nNot in the ledger (index error).\n")
            continue
        cands = d.get("candidate_set") or []
        others = [e for e in cover.get(did, []) if e != eid]
        lines = [f"#### {did} — {md(d.get('question', ''))}", "",
                 f"Ledger status `{d.get('status')}`, blocker class `{d.get('blocker_class')}`. Other experiments informing it: "
                 + (", ".join(others) if others else "none") + ".", "",
                 "| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |",
                 "|---|---|---|---|---|"]
        roles = {}
        unmapped = []
        for c in cands:
            cid = c.get("candidate_id", "")
            desc = c.get("description", "")
            if cid.startswith("status-quo") or desc.lower().startswith("status quo"):
                role = "status quo"
            elif SPEC_RX.search(cid) or SPEC_RX.search(desc):
                role = "SPEC design"
            elif DEFER_RX.search(cid) or DEFER_RX.search(desc):
                role = "defer / no change"
            else:
                role = "ledger alternative"
            roles.setdefault(role, []).append(cid)
            here = "yes" if names_candidate(own, cid) else "no"
            elsewhere = [e for e in others if names_candidate(strip_auto(texts.get(e, "")), cid)]
            if here == "no" and not elsewhere:
                unmapped.append(cid)
            lines.append(f"| `{md(cid)}` | {md(desc)} | {role} | {here} | {', '.join(elsewhere) or '—'} |")
        excl = d.get("candidate_exclusion_reasons") or []
        lines.append("")
        lines.append("**Ledger exclusions:** " + ("; ".join(md(x if isinstance(x, str) else json.dumps(x, ensure_ascii=False)) for x in excl) if excl else "none recorded in the ledger") + ".")
        protocol_excl = sorted(set(re.findall(r"`([a-z0-9][a-z0-9.-]+)`[^`\n]{0,40}?excluded at R1", own)))
        if protocol_excl:
            lines.append("**Exclusions proposed in this protocol (need independent sign-off, CR2):** " + ", ".join(f"`{x}`" for x in protocol_excl) + ".")
        incumbent = [c.get("candidate_id") for c in cands if INCUMBENT_RX.search(c.get("description", "") + " " + c.get("candidate_id", ""))]
        respecify = [c.get("candidate_id") for c in cands if RESPECIFY_RX.search(c.get("description", "") + " " + c.get("candidate_id", ""))]
        checklist = [
            ("1 status quo", ", ".join(f"`{x}`" for x in roles.get("status quo", [])) or "no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`"),
            ("2 SPEC design", ", ".join(f"`{x}`" for x in roles.get("SPEC design", [])) or "not separately identified (often the status quo); critic pass confirms"),
            ("3 ledger alternatives", f"{len(cands)} ledger candidates listed above"),
            ("4 strongest incumbent", ", ".join(f"`{x}`" for x in incumbent) or "no candidate names an incumbent technique; critic pass checks SPEC §22"),
            ("5 defer / not in v1", ", ".join(f"`{x}`" for x in roles.get("defer / no change", [])) or "absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost)"),
            ("6 critic-proposed", "**OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`)"),
            ("7 re-specify", ", ".join(f"`{x}`" for x in respecify) or "not applicable unless the critic pass finds an objective §2.0 rule 5 defect"),
        ]
        lines.append("")
        lines.append("**R0 checklist:** " + "; ".join(f"{k}: {v}" for k, v in checklist) + ".")
        if unmapped:
            lines.append("")
            lines.append("**R0 gap — ledger candidates named by no covering protocol:** " + ", ".join(f"`{x}`" for x in unmapped)
                         + ". Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.")
        parts.append("\n".join(lines) + "\n")
    return "\n".join(parts)


def common_block(rec):
    eid = rec["experiment_id"]
    lines = [
        "**Shared provisions (binding; `research/experiments/_index/crypto-conventions.md`).**",
        "",
        "- **Author and L7 exposure (CR1):** designed by the Phase C-design crypto session, which read `research/corpus/coverage.md` and `research/PROGRESS.md` under the shared design instructions and is recorded as L7-exposed for all families (`research/experiments/_program/l7-exposures-design-phase.jsonl`). Its candidate operationalizations, exclusions and analysis plans are re-signed by an unexposed session before G-A (PA-01); decision analyses are executed by unexposed sessions only.",
        "- **Gates (CR0):** runs before G-A/G-B are `NOT_DECISION_GRADE`; specs with non-empty `decision_ids` launch only through `research/tools/experiments/run_guarded.py` (PA-13).",
        "- **Candidates (CR2):** the tables above are generated; R0 item 6 is open until the crypto-cluster critic pass (PA-12).",
        "- **Security claims (CR3):** attack, leakage and tamper results are lower bounds; selections resting on a security claim, sufficiency of a mitigation or acceptance of leakage are provisional until the EXP-CRYPTO-021 external review is received.",
        "- **Metrics (CR6):** component throughput uses `__component_<name>` strata; chunk-stage throughput is owned by EXP-CHUNK-008 T1 (PA-17); HC oracle counts are binary under their MVT ids pending the PA-02 metric-registry disposition.",
        "- **Timing (CR5):** affinity and guard per §4.2-§4.4 (lint-checked), staged helpers (PA-16), calibration identity and instrument-class calibration (PA-04, PA-15).",
        "- **Split discipline (CR7):** committed specs are tuning-only or binary HC screens; graded looks use look specs derived at look time with candidates restricted to W ∪ S, registered by the owner in `research/experiments/_program/look-plan.csv` (PA-03, PA-09).",
        "- **Held-out (CR8):** generated only by EXP-EVAL-012 at Commit A (PA-20).",
        "- **Power (PA-06):** a banded comparison enters its full tuning run only after `research/tools/experiments/mde_feasibility.py` projects an MDE of at most 1 band unit on a tuning proxy.",
        "",
        f"_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for {eid}; edit the inputs, not this block._",
    ]
    return "\n".join(lines) + "\n"


def replace_block(text, name, body):
    rx = re.compile(BLOCK_RX.format(name=name), re.S)
    if not rx.search(text):
        return text, False
    return rx.sub(lambda m: m.group(1) + body + m.group(3), text, count=1), True


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    recs = load_jsonl(FRAGMENT)
    ledger = {r["decision_id"]: r for r in load_jsonl(LEDGER)}
    cover = {}
    with open(INDEX, encoding="utf-8", newline="") as fh:
        for r in csv.DictReader(fh):
            for d in [x for x in r["decision_ids"].split("; ") if x]:
                cover.setdefault(d, []).append(r["experiment_id"])
    for rec in recs:  # the fragment may be newer than index.csv
        for d in rec["decision_ids"]:
            cover.setdefault(d, [])
            if rec["experiment_id"] not in cover[d]:
                cover[d].append(rec["experiment_id"])
    owned, part = {}, {}
    if LOOK_PLAN.is_file():
        with open(LOOK_PLAN, encoding="utf-8", newline="") as fh:
            for r in csv.DictReader(line for line in fh if not line.startswith("#")):
                if r["look_number"] != "1":
                    continue
                owned.setdefault(r["owning_experiment"], []).append(r["decision_id"])
                for e in [x for x in r["participating_experiments"].split("; ") if x]:
                    part.setdefault(e, []).append(r["decision_id"])
    all_ids = set(cover_e for v in cover.values() for cover_e in v)
    texts = {e: protocol_text(e) for e in sorted(all_ids | {r["experiment_id"] for r in recs})}
    changed = 0
    for rec in recs:
        eid = rec["experiment_id"]
        path = EXP / eid / "protocol.md"
        if not path.is_file():
            print(f"missing {path}", file=sys.stderr)
            continue
        text = path.read_text(encoding="utf-8")
        new = text
        for name, body in (("meta", meta_block(rec, sorted(owned.get(eid, [])), sorted(part.get(eid, [])))),
                           ("candidates", candidates_block(rec, ledger, cover, texts)),
                           ("common", common_block(rec))):
            new, ok = replace_block(new, name, body)
            if not ok:
                print(f"{eid}: no AUTO:{name} block", file=sys.stderr)
        if new != text:
            changed += 1
            if not args.check:
                path.write_text(new, encoding="utf-8", newline="\n")
    print(f"protocols {'needing change' if args.check else 'updated'}: {changed} of {len(recs)}")
    return 1 if (args.check and changed) else 0


if __name__ == "__main__":
    sys.exit(main())
