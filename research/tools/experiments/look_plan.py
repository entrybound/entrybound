"""Generate and check the program-wide validation-look plan (design review DR1-03).

decision-method.md section 5.2 allows at most two validation looks per decision and counts looks of
related decisions (shared candidate, primary metric or ledger cluster) against each other. Before this
plan, many experiments each planned their own look for the same decision. This tool assigns exactly one
look-owning experiment per decision, lists the experiments that may contribute validation specs to that
same look event (participants) and those demoted to tuning evidence, and reserves look 2 for the section
5.2 "new validation items from new independence groups" case.

Inputs (all committed):
  research/experiments/index.csv                      experiments, decision ids, status, domain
  research/decision-ledger.jsonl                      cluster and program sections per decision
  research/experiments/EXP-*/protocol.md              look-planning heuristics (regexes below)
  research/experiments/_program/look-ownership-overrides.csv   explicit owners and roles (review-assigned)

Output:
  research/experiments/_program/look-plan.csv         one row per (decision, look number)

Usage:
  python research/tools/experiments/look_plan.py                 # regenerate the plan
  python research/tools/experiments/look_plan.py --check-look --decision DEC-CMP-001 \
         --experiment EXP-CHUNK-004 --look 1                      # exit 0 only if the plan allows it

The validation-look registry writer (decision-method section 14 item 14, not yet built) MUST call
--check-look before appending a look and refuse the look on a non-zero exit. Stdlib only, deterministic.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
EXP = ROOT / "research" / "experiments"
PROGRAM = EXP / "_program"
LEDGER = ROOT / "research" / "decision-ledger.jsonl"
INDEX = EXP / "index.csv"
OVERRIDES = PROGRAM / "look-ownership-overrides.csv"
OUT = PROGRAM / "look-plan.csv"

# Wave = order of TUNING campaigns (later waves consume earlier waves' tuning winners W as fixed inputs).
# Looks never wait on another domain's look: see PA-03 in _program/design-revision-round1.md.
DOMAIN_WAVE = {
    "chunking": 1, "codecs": 1, "crypto": 1, "integrity": 1, "conformance": 1,
    "crossfile": 2, "reconstruction": 2, "container": 2, "remote": 2, "scale": 2, "platform": 2, "legacy": 2,
    "planner": 3,
    "evaluation": 4, "ecosystem": 4, "method": 0,
}
CLUSTER_HOME = {
    "access": {"remote", "scale"},
    "compression": {"chunking", "codecs", "crossfile", "reconstruction", "planner"},
    "container": {"container"},
    "crypto": {"crypto"},
    "ecosystem": {"ecosystem"},
    "integrity": {"integrity"},
    "legacy": {"legacy"},
    "model": {"platform", "container"},
    "platform": {"platform"},
}
# Experiments that never own or join a validation look: held-out placeholders, scoring, sealed tranche,
# calibration, method simulation.
NEVER_LOOK = {"EXP-EVAL-004", "EXP-EVAL-008", "EXP-EVAL-011", "EXP-EVAL-012", "EXP-PLANNER-012", "EXP-METHOD-001"}

GRADED_RX = re.compile(r"MDE|W ∪ S|W u S|W and S|confirmatory|look[- ]?1\b|validation look|validation-looks\.jsonl", re.I)
VALIDATION_RX = re.compile(r"\bvalidation\b", re.I)


def load_ledger():
    out = {}
    with open(LEDGER, encoding="utf-8") as fh:
        for line in fh:
            if line.strip():
                r = json.loads(line)
                out[r["decision_id"]] = r
    return out


def split_list(s):
    return [x for x in (s or "").split("; ") if x]


def load_overrides():
    rows = defaultdict(list)
    if not OVERRIDES.is_file():
        return rows
    with open(OVERRIDES, encoding="utf-8", newline="") as fh:
        for r in csv.DictReader(line for line in fh if not line.startswith("#")):
            rows[r["decision_id"]].append(r)
    return rows


def classify(index_rows):
    """Return {experiment_id: 'graded'|'hc_screen'|'none'} for non-BLOCKED experiments."""
    kinds = {}
    for r in index_rows:
        eid = r["experiment_id"]
        if r["status"] == "BLOCKED" or eid in NEVER_LOOK:
            kinds[eid] = "none"
            continue
        p = EXP / eid / "protocol.md"
        text = p.read_text(encoding="utf-8") if p.is_file() else ""
        if not VALIDATION_RX.search(text):
            kinds[eid] = "none"
        elif GRADED_RX.search(text):
            kinds[eid] = "graded"
        else:
            kinds[eid] = "hc_screen"
    return kinds


def rank_key(eid, d, idx, ledger):
    r = idx[eid]
    dec = ledger.get(d, {})
    cluster = dec.get("cluster", "")
    home = 0 if r["domain"] in CLUSTER_HOME.get(cluster, set()) else 1
    dsecs = set(dec.get("program_sections") or [])
    esecs = set(split_list(r["program_sections"]))
    overlap = -len(dsecs & esecs)
    decs = split_list(r["decision_ids"])
    same_cluster = sum(1 for x in decs if ledger.get(x, {}).get("cluster") == cluster)
    focus = -(same_cluster / max(1, len(decs)))
    evaluation_last = 1 if r["domain"] in ("evaluation", "ecosystem") and home else 0
    return (home, evaluation_last, overlap, focus, len(decs), eid)


def build(write=True):
    ledger = load_ledger()
    index_rows = list(csv.DictReader(open(INDEX, encoding="utf-8")))
    idx = {r["experiment_id"]: r for r in index_rows}
    kinds = classify(index_rows)
    overrides = load_overrides()
    problems = []

    dec_exps = defaultdict(list)
    for r in index_rows:
        for d in split_list(r["decision_ids"]):
            dec_exps[d].append(r["experiment_id"])

    out = []
    for d in sorted(dec_exps):
        exps = sorted(set(dec_exps[d]))
        graded = [e for e in exps if kinds.get(e) == "graded"]
        screens = [e for e in exps if kinds.get(e) == "hc_screen"]
        if not graded and d not in overrides:
            continue
        cluster = ledger.get(d, {}).get("cluster", "")
        basis = []
        owner = None
        participants = []
        tuning_only = []
        comparison_owners = []
        for ov in overrides.get(d, []):
            role = ov["role"]
            e = ov["experiment_id"]
            if e not in exps:
                problems.append(f"override {d} names {e}, which does not cover the decision")
                continue
            if role == "owner":
                owner = e
                basis.append(f"override:{ov['basis']}")
            elif role == "participant":
                participants.append(e)
            elif role == "tuning_only":
                tuning_only.append(e)
            if ov.get("comparison"):
                comparison_owners.append(f"{ov['comparison']}: {e}")
        if owner is None:
            pool = [e for e in graded if e not in tuning_only]
            if not pool:
                problems.append(f"{d}: no eligible look owner")
                continue
            owner = sorted(pool, key=lambda e: rank_key(e, d, idx, ledger))[0]
            basis.append("rule:home-domain>section-overlap>cluster-focus>fewest-decisions")
        for e in graded:
            if e != owner and e not in participants and e not in tuning_only:
                participants.append(e)
        wave = DOMAIN_WAVE.get(idx[owner]["domain"], 4)
        ws_sources = [f"research/experiments/{owner}/record.md"] + [f"research/experiments/{e}/record.md" for e in sorted(participants)]
        common = {
            "decision_id": d,
            "cluster": cluster,
            "look_group": f"LG-{cluster}",
            "tuning_wave": wave,
            "owning_experiment": owner,
            "participating_experiments": "; ".join(sorted(set(participants))),
            "tuning_only_experiments": "; ".join(sorted(set(tuning_only))),
            "hc_screen_experiments": "; ".join(screens),
            "comparison_owners": "; ".join(comparison_owners),
            "ws_source_records": "; ".join(ws_sources),
            "ownership_basis": "; ".join(basis),
        }
        out.append({**common, "look_number": 1, "look_state": "PLANNED",
                     "mde_record_path": f"research/normalized/{owner}/decision/{d}-look1-mde.json",
                     "look_condition": "after W and S of every decision in the look group are recorded on tuning (PA-03 rule 2); MDE gate section 4.12 passes"})
        out.append({**common, "look_number": 2, "look_state": "RESERVED",
                     "mde_record_path": f"research/normalized/{owner}/decision/{d}-look2-mde.json",
                     "look_condition": "only after a documented change, on new validation items from new independence groups committed and fingerprinted before use (section 5.2); otherwise labelled tuning"})

    fields = ["decision_id", "look_number", "look_state", "cluster", "look_group", "tuning_wave", "owning_experiment",
              "participating_experiments", "tuning_only_experiments", "hc_screen_experiments", "comparison_owners",
              "ws_source_records", "mde_record_path", "look_condition", "ownership_basis"]
    if write:
        PROGRAM.mkdir(parents=True, exist_ok=True)
        with open(OUT, "w", encoding="utf-8", newline="") as fh:
            fh.write("# generated by research/tools/experiments/look_plan.py; do not edit (edit look-ownership-overrides.csv)\n")
            w = csv.DictWriter(fh, fieldnames=fields, lineterminator="\n")
            w.writeheader()
            w.writerows(out)
    return out, problems, kinds


def check_look(decision, experiment, look):
    if not OUT.is_file():
        print("look plan missing; regenerate it", file=sys.stderr)
        return 2
    with open(OUT, encoding="utf-8", newline="") as fh:
        rows = list(csv.DictReader(line for line in fh if not line.startswith("#")))
    for r in rows:
        if r["decision_id"] == decision and int(r["look_number"]) == look:
            if experiment == r["owning_experiment"]:
                print(f"ALLOW {decision} look {look}: {experiment} owns the look")
                return 0
            if experiment in split_list(r["participating_experiments"]):
                print(f"ALLOW {decision} look {look}: {experiment} participates in the look owned by {r['owning_experiment']} (same look event only)")
                return 0
            print(f"REFUSE {decision} look {look}: {experiment} is not the owner ({r['owning_experiment']}) or a participant", file=sys.stderr)
            return 1
    print(f"REFUSE {decision} look {look}: not in the look plan", file=sys.stderr)
    return 1


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check-look", action="store_true")
    ap.add_argument("--decision")
    ap.add_argument("--experiment")
    ap.add_argument("--look", type=int)
    args = ap.parse_args()
    if args.check_look:
        if not (args.decision and args.experiment and args.look in (1, 2)):
            ap.error("--check-look needs --decision, --experiment and --look 1|2")
        return check_look(args.decision, args.experiment, args.look)
    out, problems, kinds = build(write=True)
    decisions = len({r["decision_id"] for r in out})
    print(f"look-plan rows={len(out)} decisions={decisions}")
    print("experiment kinds: " + ", ".join(f"{k}={sum(1 for v in kinds.values() if v == k)}" for k in ("graded", "hc_screen", "none")))
    for p in problems:
        print("PROBLEM " + p)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
