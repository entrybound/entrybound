#!/usr/bin/env python3
"""Append and audit actual validation looks; a look-plan row is never execution evidence.

The registry is an append-only event log. REGISTERED consumes a validation look before
validation data is opened. EXECUTED requires hashed raw records and is a separate event.
This tool reads validation item *identifiers* supplied by a caller, never item content.
"""
from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[3]
REGISTRY_REL = Path("research/decisions/validation-looks.jsonl")
PLAN_REL = Path("research/experiments/_program/look-plan.csv")
LEDGER_REL = Path("research/decision-ledger.jsonl")
HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
LOOK_ID = re.compile(r"^VL-[0-9]{6}$")
DECISION_ID = re.compile(r"^DEC-[A-Z]{3}-[0-9]{3}$")
EXPERIMENT_ID = re.compile(r"^EXP-[A-Z0-9]+-[0-9]{3}$")


def canonical(obj: object) -> str:
    return json.dumps(obj, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def digest(obj: object) -> str:
    return hashlib.sha256(canonical(obj).encode("utf-8")).hexdigest()


def require(ok: bool, message: str) -> None:
    if not ok:
        raise ValueError(message)


def unique_strings(value: object, label: str, nonempty: bool = True) -> list[str]:
    require(isinstance(value, list) and (bool(value) or not nonempty), f"{label}: expected {'nonempty ' if nonempty else ''}list")
    require(all(isinstance(x, str) and x for x in value), f"{label}: entries must be nonempty strings")
    require(len(value) == len(set(value)), f"{label}: duplicate entry")
    return value


def repo_path(root: Path, ref: str) -> Path:
    require(isinstance(ref, str) and ref and "heldout" not in ref.casefold(), "reference must be a non-held-out repository path")
    p = Path(ref)
    require(not p.is_absolute() and ".." not in p.parts, "reference must be relative and remain in the repository")
    resolved = (root / p).resolve()
    require(resolved.is_relative_to(root.resolve()), "reference escapes repository")
    return resolved


def committed_bytes(root: Path, commit: str, ref: str) -> bytes:
    require(isinstance(commit, str) and HEX40.fullmatch(commit) is not None,
            "full Git commit SHA required")
    result = subprocess.run(["git", "show", f"{commit}:{ref}"], cwd=root,
                            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False)
    require(result.returncode == 0, f"{ref} absent from Git commit {commit}")
    return result.stdout


def require_ancestor(root: Path, earlier: str, later: str) -> None:
    result = subprocess.run(["git", "merge-base", "--is-ancestor", earlier, later], cwd=root,
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
    require(result.returncode == 0, f"Git commit {earlier} does not precede {later}")


def read_jsonl(path: Path) -> list[dict]:
    if not path.is_file():
        raise ValueError(f"registry missing: {path}")
    result = []
    for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        obj = json.loads(line)
        require(isinstance(obj, dict), f"registry line {n}: expected object")
        result.append(obj)
    return result


def load_inputs(root: Path) -> tuple[dict, dict]:
    with (root / PLAN_REL).open(encoding="utf-8", newline="") as fh:
        plan_rows = list(csv.DictReader(line for line in fh if not line.startswith("#")))
    plan = {}
    for row in plan_rows:
        key = (row["decision_id"], int(row["look_number"]))
        require(key not in plan, f"duplicate look plan row: {key}")
        plan[key] = row
    ledger = {}
    for row in (json.loads(line) for line in (root / LEDGER_REL).read_text(encoding="utf-8").splitlines() if line.strip()):
        require(row["decision_id"] not in ledger, f"duplicate ledger decision: {row['decision_id']}")
        ledger[row["decision_id"]] = row
    return plan, ledger


def related_decisions(ledger: dict, decision_ids: list[str], candidate_ids: dict, primary_metrics: dict) -> list[str]:
    clusters = {ledger[d]["cluster"] for d in decision_ids}
    candidates = {c for values in candidate_ids.values() for c in values}
    metrics = {m for values in primary_metrics.values() for m in values.values()}
    result = []
    for did, row in ledger.items():
        row_candidates = {c["candidate_id"] for c in row["candidate_set"]}
        row_metrics = set((row.get("primary_metrics_by_od") or {}).values())
        if row["cluster"] in clusters or candidates & row_candidates or metrics & row_metrics:
            result.append(did)
    return sorted(result)


def enrich_registration(row: dict, ledger: dict) -> dict:
    row = dict(row)
    related = related_decisions(ledger, row["decision_ids"], row["candidate_ids"], row["primary_metrics"])
    row["related_decision_ids"] = related
    row["candidate_set_sha256_by_decision"] = {d: digest(ledger[d]["candidate_set"]) for d in related}
    row.setdefault("registered_at_utc", dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"))
    return row


def relation(prior: dict, current: dict) -> bool:
    return bool(set(prior["related_decision_ids"]) & set(current["decision_ids"]) or
                set(current["related_decision_ids"]) & set(prior["decision_ids"]))


def validate_registration(row: dict, prior: list[dict], plan: dict, ledger: dict, root: Path,
                          check_current_files: bool = False, check_current_snapshot: bool = False) -> None:
    required = {"schema", "event_type", "look_id", "look_number", "look_type", "decision_ids",
                "experiment_id", "participating_experiments", "candidate_ids", "primary_metrics",
                "validation_item_groups", "validation_split_evidence_ref", "validation_split_evidence_sha256",
                "spec_ref", "spec_sha256", "mde_refs", "mde_sha256_by_decision", "method_commit_sha",
                "source_commit_sha", "registered_at_utc", "change_ref", "related_prior_look_ids",
                "charged_prior_look_ids", "related_decision_ids", "candidate_set_sha256_by_decision"}
    require(set(row) == required, f"registration fields: missing={sorted(required-set(row))}, extra={sorted(set(row)-required)}")
    require(row["schema"] == "entrybound.validation-look.v1" and row["event_type"] == "REGISTERED", "invalid registration schema/event")
    require(isinstance(row["look_id"], str) and LOOK_ID.fullmatch(row["look_id"]) is not None, "invalid look_id")
    require(row["look_id"] not in {p["look_id"] for p in prior}, "look_id reused")
    require(type(row["look_number"]) is int and row["look_number"] in (1, 2), "look_number must be 1 or 2")
    require(row["look_type"] in ("CONFIRMATORY", "TUNING_ONLY"), "invalid look_type")
    decisions = unique_strings(row["decision_ids"], "decision_ids")
    require(all(DECISION_ID.fullmatch(d) and d in ledger for d in decisions), "unknown decision_id")
    require(isinstance(row["experiment_id"], str) and EXPERIMENT_ID.fullmatch(row["experiment_id"]) is not None, "invalid experiment_id")
    participants = unique_strings(row["participating_experiments"], "participating_experiments", False)
    require(all(EXPERIMENT_ID.fullmatch(e) for e in participants), "invalid participant experiment")
    require(isinstance(row["candidate_ids"], dict) and set(row["candidate_ids"]) == set(decisions), "candidate_ids must cover decisions")
    require(isinstance(row["primary_metrics"], dict) and set(row["primary_metrics"]) == set(decisions), "primary_metrics must cover decisions")
    for d in decisions:
        p = plan.get((d, row["look_number"]))
        require(p is not None, f"{d}: no planned look {row['look_number']}")
        blockers = ledger[d].get("metric_registration_blockers_by_od")
        require(isinstance(blockers, dict) and not blockers,
                f"{d}: missing or unresolved canonical metric registration blocks validation-look registration")
        require(p["owning_experiment"] == row["experiment_id"], f"{d}: experiment is not planned owner")
        allowed = {x for x in p["participating_experiments"].split("; ") if x}
        require(set(participants) <= allowed, f"{d}: unplanned participant")
        candidates = unique_strings(row["candidate_ids"][d], f"{d}.candidate_ids")
        known = {c["candidate_id"] for c in ledger[d]["candidate_set"]}
        require(set(candidates) <= known, f"{d}: candidate outside ledger candidate_set")
        metrics = row["primary_metrics"][d]
        require(isinstance(metrics, dict), f"{d}: primary metrics must map OD to metric")
        require(all(re.fullmatch(r"OD-[0-9]{2}", od) and isinstance(metric, str) and metric for od, metric in metrics.items()), f"{d}: malformed primary metric map")
        assigned = ledger[d].get("primary_metrics_by_od") or {}
        require(metrics == assigned, f"{d}: primary metrics differ from ledger assignment")
    groups = row["validation_item_groups"]
    require(isinstance(groups, dict) and groups and all(isinstance(k, str) and k and isinstance(v, str) and v for k, v in groups.items()), "validation_item_groups must map item IDs to independence groups")
    require(not any("heldout" in x.casefold() for pair in groups.items() for x in pair), "held-out identity in validation look")
    require(isinstance(row["validation_split_evidence_sha256"], str) and HEX64.fullmatch(row["validation_split_evidence_sha256"]), "validation split attestation SHA-256 required")
    split_path = repo_path(root, row["validation_split_evidence_ref"])
    if check_current_files:
        require(split_path.is_file() and hashlib.sha256(split_path.read_bytes()).hexdigest() == row["validation_split_evidence_sha256"], "validation split attestation missing or digest mismatch")
        split_record = json.loads(split_path.read_text(encoding="utf-8"))
        require(set(split_record) == {"schema", "split", "items"} and
                split_record.get("schema") == "entrybound.validation-split-attestation.v1" and
                split_record.get("split") == "validation" and split_record.get("items") == groups,
                "validation split attestation does not certify exact items/groups")
    require(isinstance(row["mde_refs"], dict) and set(row["mde_refs"]) == set(decisions), "mde_refs must cover decisions")
    require(isinstance(row["mde_sha256_by_decision"], dict) and set(row["mde_sha256_by_decision"]) == set(decisions), "MDE digest map must cover decisions")
    for d in decisions:
        require(row["mde_refs"][d] == plan[(d, row["look_number"])]["mde_record_path"], f"{d}: MDE ref differs from plan")
        require(isinstance(row["mde_sha256_by_decision"][d], str) and HEX64.fullmatch(row["mde_sha256_by_decision"][d]), f"{d}: MDE SHA-256 required")
        if check_current_files:
            mde_path = repo_path(root, row["mde_refs"][d])
            require(mde_path.is_file() and hashlib.sha256(mde_path.read_bytes()).hexdigest() == row["mde_sha256_by_decision"][d], f"{d}: MDE missing or digest mismatch")
    for field in ("method_commit_sha", "source_commit_sha"):
        require(isinstance(row[field], str) and HEX40.fullmatch(row[field]), f"{field}: full SHA required")
    require(isinstance(row["spec_sha256"], str) and HEX64.fullmatch(row["spec_sha256"]), "spec SHA-256 required")
    spec = repo_path(root, row["spec_ref"])
    spec_ref = spec.relative_to(root.resolve()).as_posix()
    require(spec_ref.startswith(f"research/experiments/{row['experiment_id']}/") and
            spec.suffix in {".yaml", ".yml"}, "spec must be a YAML file of the owning experiment")
    method_bytes = committed_bytes(root, row["method_commit_sha"], "research/decision-method.md")
    require(method_bytes, "method commit has empty governing method")
    require_ancestor(root, row["method_commit_sha"], row["source_commit_sha"])
    require_ancestor(root, row["source_commit_sha"], "HEAD")
    committed_spec = committed_bytes(root, row["source_commit_sha"], spec_ref)
    require(hashlib.sha256(committed_spec).hexdigest() == row["spec_sha256"],
            "spec SHA-256 differs from source commit")
    try:
        spec_doc = yaml.safe_load(committed_spec)
    except yaml.YAMLError as exc:
        raise ValueError("committed spec is invalid YAML") from exc
    spec_candidates = spec_doc.get("candidates") if isinstance(spec_doc, dict) else None
    require(isinstance(spec_doc, dict) and
            isinstance(spec_candidates, list) and
            all(isinstance(candidate, dict) for candidate in spec_candidates) and
            spec_doc.get("experiment_id") == row["experiment_id"] and
            set(spec_doc.get("decision_ids") or []) == set(decisions) and
            {c.get("name") for c in spec_candidates} ==
            {c for candidates in row["candidate_ids"].values() for c in candidates} and
            (spec_doc.get("corpus") or {}).get("splits") == ["validation"] and
            spec_doc.get("status") == f"look-{row['look_number']}",
            "committed spec experiment/decisions/candidates/split/look differ from registration")
    prereg_ref = f"research/experiments/{row['experiment_id']}/record.md"
    prereg = committed_bytes(root, row["source_commit_sha"], prereg_ref)
    require(row["experiment_id"].encode("utf-8") in prereg and
            b"pre-registration" in prereg.lower(),
            "source commit lacks matching experiment pre-registration record")
    if check_current_files:
        require(spec.is_file() and hashlib.sha256(spec.read_bytes()).hexdigest() == row["spec_sha256"], "spec missing or digest mismatch")
    require(isinstance(row["registered_at_utc"], str) and row["registered_at_utc"].endswith("Z"), "UTC registration timestamp required")
    dt.datetime.fromisoformat(row["registered_at_utc"].replace("Z", "+00:00"))
    related = unique_strings(row["related_decision_ids"], "related_decision_ids")
    require(set(decisions) <= set(related) and set(related) <= set(ledger), "related decision snapshot invalid")
    fingerprints = row["candidate_set_sha256_by_decision"]
    require(isinstance(fingerprints, dict) and set(fingerprints) == set(related) and all(isinstance(h, str) and HEX64.fullmatch(h) for h in fingerprints.values()), "candidate fingerprint snapshot invalid")
    if check_current_snapshot:
        require(related == related_decisions(ledger, decisions, row["candidate_ids"], row["primary_metrics"]), "related decision snapshot stale")
        require(all(fingerprints[d] == digest(ledger[d]["candidate_set"]) for d in related), "candidate fingerprint snapshot stale")
    related_prior = {p["look_id"] for p in prior if relation(p, row)}
    require(set(unique_strings(row["related_prior_look_ids"], "related_prior_look_ids", False)) == related_prior, "related prior looks omitted or invented")
    changed_prior = {p["look_id"] for p in prior if p["look_id"] in related_prior and
                     any(p["candidate_set_sha256_by_decision"].get(d) != fingerprints[d] for d in decisions)}
    require(set(unique_strings(row["charged_prior_look_ids"], "charged_prior_look_ids", False)) == changed_prior, "post-look candidate changes not charged exactly")
    for d in decisions:
        own = [p for p in prior if d in p["decision_ids"]]
        require(row["look_number"] == len(own) + 1, f"{d}: look number is not sequential")
        effective = {p["look_id"] for p in own} | changed_prior
        if row["look_type"] == "CONFIRMATORY":
            require(len(effective) + 1 <= 2, f"{d}: confirmatory look budget exceeded")
    if row["look_number"] == 2:
        require(isinstance(row["change_ref"], str) and row["change_ref"], "second look requires documented change")
        if row["look_type"] == "CONFIRMATORY":
            old_groups = {g for p in prior if p["look_id"] in related_prior for g in p["validation_item_groups"].values()}
            require(not (set(groups.values()) & old_groups), "second confirmatory look reuses an independence group")
    else:
        require(row["change_ref"] == "", "first look must not claim a second-look change")


def validate_execution(row: dict, registrations: dict, executions: set[str], root: Path,
                       check_current_files: bool = False) -> None:
    required = {"schema", "event_type", "look_id", "executed_at_utc", "raw_result_refs", "raw_result_sha256", "raw_meta_sha256"}
    require(set(row) == required, f"execution fields: missing={sorted(required-set(row))}, extra={sorted(set(row)-required)}")
    require(row["schema"] == "entrybound.validation-look.v1" and row["event_type"] == "EXECUTED", "invalid execution schema/event")
    require(row["look_id"] in registrations and row["look_id"] not in executions, "execution without unique registration")
    refs = unique_strings(row["raw_result_refs"], "raw_result_refs")
    require(isinstance(row["raw_result_sha256"], dict) and set(row["raw_result_sha256"]) == set(refs), "raw result digest map mismatch")
    require(isinstance(row["raw_meta_sha256"], dict) and set(row["raw_meta_sha256"]) == set(refs), "raw metadata digest map mismatch")
    registration = registrations[row["look_id"]]
    expected_candidates = {candidate for candidates in registration["candidate_ids"].values()
                           for candidate in candidates}
    for ref in refs:
        require(isinstance(row["raw_result_sha256"][ref], str) and HEX64.fullmatch(row["raw_result_sha256"][ref]), "raw result SHA-256 required")
        require(isinstance(row["raw_meta_sha256"][ref], str) and HEX64.fullmatch(row["raw_meta_sha256"][ref]), "raw metadata SHA-256 required")
        path = repo_path(root, ref)
        require(ref.startswith(f"research/raw/{registration['experiment_id']}/") and
                (ref.endswith(".jsonl") or ref.endswith(".jsonl.gz")),
                "raw result must use the registered experiment JSONL path")
        run_id = path.name.removesuffix(".jsonl.gz").removesuffix(".jsonl")
        meta_path = path.with_name(f"{run_id}.meta.json")
        if check_current_files:
            require(path.is_file() and hashlib.sha256(path.read_bytes()).hexdigest() == row["raw_result_sha256"][ref], "raw result missing or digest mismatch")
            require(meta_path.is_file() and hashlib.sha256(meta_path.read_bytes()).hexdigest() == row["raw_meta_sha256"][ref],
                    "raw metadata missing or digest mismatch")
            meta = json.loads(meta_path.read_text(encoding="utf-8"))
            require(isinstance(meta, dict), "raw metadata must be an object")
            require(meta.get("run_id") == run_id and meta.get("raw_file") == path.name and
                    meta.get("raw_sha256") == row["raw_result_sha256"][ref] and
                    meta.get("status") == "complete", "raw metadata does not bind complete result")
            require(meta.get("experiment_id") == registration["experiment_id"] and
                    set(meta.get("decision_ids") or []) == set(registration["decision_ids"]) and
                    meta.get("spec_path") == registration["spec_ref"] and
                    meta.get("spec_sha256") == registration["spec_sha256"],
                    "raw metadata does not bind registered experiment/spec/decisions")
            spec = meta.get("spec") or {}
            meta_candidates = spec.get("candidates") if isinstance(spec, dict) else None
            require(isinstance(meta_candidates, list) and
                    all(isinstance(candidate, dict) for candidate in meta_candidates) and
                    {c.get("name") for c in meta_candidates} == expected_candidates and
                    (spec.get("corpus") or {}).get("splits") == ["validation"],
                    "raw metadata candidates or validation split differ from registration")
            items = meta.get("items") or []
            require(isinstance(items, list) and all(isinstance(item, dict) for item in items) and
                    len(items) == len(registration["validation_item_groups"]) and
                    {item.get("item_id") for item in items} == set(registration["validation_item_groups"]) and
                    all(item.get("split") == "validation" for item in items),
                    "raw metadata validation items differ from registration")
            require(isinstance(meta.get("started_utc"), str) and
                    meta["started_utc"] >= registration["registered_at_utc"],
                    "raw run predates validation look registration")
            run_commit = meta.get("git_sha")
            require(meta.get("git_dirty") is False and isinstance(run_commit, str) and
                    HEX40.fullmatch(run_commit) is not None, "raw metadata lacks clean Git commit")
            require_ancestor(root, registration["source_commit_sha"], run_commit)
    require(isinstance(row["executed_at_utc"], str) and row["executed_at_utc"].endswith("Z"), "UTC execution timestamp required")
    dt.datetime.fromisoformat(row["executed_at_utc"].replace("Z", "+00:00"))
    require(row["executed_at_utc"] >= registrations[row["look_id"]]["registered_at_utc"], "execution predates registration")


def audit(events: list[dict], plan: dict, ledger: dict, root: Path) -> dict:
    prior = []
    registrations = {}
    executions: set[str] = set()
    for n, row in enumerate(events, 1):
        try:
            if row.get("event_type") == "REGISTERED":
                validate_registration(row, prior, plan, ledger, root, check_current_files=True)
                prior.append(row)
                registrations[row["look_id"]] = row
            elif row.get("event_type") == "EXECUTED":
                validate_execution(row, registrations, executions, root, check_current_files=True)
                executions.add(row["look_id"])
            else:
                raise ValueError("unknown event_type")
        except (ValueError, KeyError, TypeError) as exc:
            raise ValueError(f"registry event {n}: {exc}") from exc
    return {"registered_looks": len(prior), "executed_looks": len(executions),
            "registered_unexecuted": len(prior)-len(executions),
            "confirmatory_registered": sum(p["look_type"] == "CONFIRMATORY" for p in prior),
            "tuning_only_registered": sum(p["look_type"] == "TUNING_ONLY" for p in prior)}


def append_event(path: Path, row: dict) -> None:
    data = (canonical(row) + "\n").encode("utf-8")
    with path.open("ab") as fh:
        fh.write(data)
        fh.flush()
        os.fsync(fh.fileno())


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("action", choices=("validate", "register", "record-execution"))
    ap.add_argument("--root", type=Path, default=ROOT)
    ap.add_argument("--record", type=Path, help="JSON object for register or record-execution")
    args = ap.parse_args()
    root = args.root.resolve()
    try:
        plan, ledger = load_inputs(root)
        registry = root / REGISTRY_REL
        events = read_jsonl(registry)
        summary = audit(events, plan, ledger, root)
        if args.action != "validate":
            require(args.record is not None, "--record is required for append")
            record = json.loads(args.record.read_text(encoding="utf-8"))
            require(isinstance(record, dict), "record must be a JSON object")
            if args.action == "register":
                record = enrich_registration(record, ledger)
                prior = [e for e in events if e["event_type"] == "REGISTERED"]
                validate_registration(record, prior, plan, ledger, root,
                                      check_current_files=True, check_current_snapshot=True)
            else:
                require(record.get("event_type") == "EXECUTED", "record-execution needs EXECUTED event")
                registrations = {e["look_id"]: e for e in events if e["event_type"] == "REGISTERED"}
                executed = {e["look_id"] for e in events if e["event_type"] == "EXECUTED"}
                validate_execution(record, registrations, executed, root, check_current_files=True)
            append_event(registry, record)
            summary = audit(read_jsonl(registry), plan, ledger, root)
        print(canonical(summary))
        return 0
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as exc:
        print(f"validation-look registry: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
