#!/usr/bin/env python3
"""Validate decision-ledger schema v2 and refuse unsupported DECIDED states.

Structural validation is always strict. --gate-a additionally requires reviewed
method assignments on every row. A current v1 ledger correctly fails until the
coordinated migration is reviewed; this tool never changes a ledger.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

try:
    from .assemble import build_schemas, schema_validate
except ImportError:  # direct script execution
    from assemble import build_schemas, schema_validate

TOOLS = Path(__file__).resolve().parents[1]
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
from experiments import validation_looks
from methods import gate_admission
try:
    from .constraint_crosswalk import BARE_INVARIANT, I_TO_HC
except ImportError:  # direct script execution
    from constraint_crosswalk import BARE_INVARIANT, I_TO_HC

ROOT = Path(__file__).resolve().parents[3]
HC_RE = re.compile(r"^HC-(0[1-9]|1[0-8])$")
OD_RE = re.compile(r"^OD-\d{2}$")
SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SPLITS = ("conformance", "tuning", "validation", "heldout")
ROUTES = {"EMPIRICAL", "FORMAL", "EXTERNAL", "HUMAN-FACING"}
RULES = {f"R{i}" for i in range(11)}
DIGEST_RE = re.compile(r"^[0-9a-f]{64}$")
FREEZE_ROW_RE = re.compile(r"^\| (F-\d{2}) \|")
ASSIGNMENT_REVIEW_SCHEMA = "entrybound.assignment-review.v1"
ASSIGNMENT_REVIEW_REF = "research/methods/decision-assignments-independent-review.jsonl"
ASSIGNMENT_REVIEW_FIELDS = (
    "hc_ids", "hc_applicability_rationale", "od_ids", "od_applicability_rationale", "decision_type",
    "evidence_route_types", "primary_metrics_by_od", "no_primary_metric_reason_by_od",
    "primary_metric_justifications", "metric_registration_blockers_by_od",
)
METHOD_ADMISSION_ONLY_DECISIONS = {"DEC-ECO-077", "DEC-ECO-080", "DEC-ECO-081", "DEC-ECO-082"}


def registered_freezes(root: Path) -> set[str]:
    """Read the actual Appendix B register, rather than accepting any F-nn."""
    objective = (root / "research/archetypal-objective.md").read_text(encoding="utf-8")
    appendix = objective.split("## Appendix B", 1)[1].split("\n## Appendix ", 1)[0]
    freezes = {
        match.group(1) for line in appendix.splitlines()
        if (match := FREEZE_ROW_RE.match(line))
    }
    if not freezes:
        raise ValueError("objective Appendix B has no registered freezes")
    return freezes


def source_ref(root: Path, ref: str) -> Path | None:
    if not isinstance(ref, str) or not ref or "\\" in ref or ":" in ref or "\x00" in ref:
        return None
    p = Path(ref)
    if p.is_absolute() or ".." in p.parts or "." in p.parts:
        return None
    parts = tuple(part.casefold() for part in p.parts)
    if not parts or any(part in {".git", ".codex", ".agents", "target", "private"}
                        for part in parts):
        return None
    # Public reports, experiments, the lock and freeze receipts may name held-out
    # evidence. Corpus payloads, fingerprints and sample identities may not.
    if parts[:3] in {("research", "corpus", "heldout"),
                     ("research", "corpus", "fingerprints")}:
        return None
    if parts[:3] == ("research", "corpus", "items") and "heldout" in parts:
        return None
    result = (root / p).resolve()
    return result if result.is_relative_to(root.resolve()) else None


def checked_ref(root: Path, ref: str, errors: list[str], label: str) -> None:
    p = source_ref(root, ref)
    if p is None or not p.is_file():
        errors.append(f"{label}: missing or unsafe evidence reference {ref!r}")


def checked_pinned_ref(root: Path, row: dict, ref: str, errors: list[str], label: str,
                       expected_override: str | None = None) -> None:
    """Require existing, SHA-256 pinned bytes equal to committed HEAD bytes."""
    path = source_ref(root, ref)
    if path is None or not path.is_file():
        errors.append(f"{label}: missing or unsafe evidence reference {ref!r}")
        return
    expected = expected_override if expected_override is not None else row["evidence_sha256"].get(ref)
    if not isinstance(expected, str) or not DIGEST_RE.fullmatch(expected):
        errors.append(f"{label}: evidence {ref!r} lacks SHA-256 pin")
        return
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        errors.append(f"{label}: evidence {ref!r} SHA-256 mismatch")
        return
    committed = subprocess.run(["git", "show", f"HEAD:{ref}"], cwd=root,
                               stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False)
    if committed.returncode != 0 or hashlib.sha256(committed.stdout).hexdigest() != expected:
        errors.append(f"{label}: evidence {ref!r} is not identical to committed HEAD")


def reviewed_assignment_digest(row: dict) -> str:
    """Bind independent review to the exact governed assignment fields."""
    selected = {field: row[field] for field in ASSIGNMENT_REVIEW_FIELDS}
    payload = json.dumps(selected, ensure_ascii=False, sort_keys=True,
                         separators=(",", ":"), allow_nan=False).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def checked_assignment_review(root: Path, row: dict, errors: list[str], label: str) -> None:
    producer = row["assigning_session"].strip()
    reviewer = row["assignment_reviewer_session"].strip()
    if not producer or not reviewer or producer == reviewer:
        errors.append(f"{label}: independent assignment reviewer session missing")
    ref = row["assignment_review_ref"]
    if ref != ASSIGNMENT_REVIEW_REF:
        errors.append(f"{label}: assignment review must use the public independent review register")
    checked_pinned_ref(root, row, ref, errors, label,
                       expected_override=row["assignment_review_sha256"])
    path = source_ref(root, ref)
    if path is None or not path.is_file():
        return
    try:
        records = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()
                   if line.strip()]
    except (OSError, ValueError, TypeError):
        errors.append(f"{label}: assignment review is not valid JSONL")
        return
    matches = [record for record in records if isinstance(record, dict) and
               record.get("decision_id") == label]
    if len(matches) != 1:
        errors.append(f"{label}: assignment review requires exactly one decision record")
        return
    record = matches[0]
    if (record.get("schema") != ASSIGNMENT_REVIEW_SCHEMA or
            record.get("review_status") != "ACCEPTED_PROSPECTIVE_ASSIGNMENT" or
            record.get("reviewer_session") != reviewer):
        errors.append(f"{label}: assignment review does not bind reviewer and decision")
    expected_assignment = {field: row[field] for field in ASSIGNMENT_REVIEW_FIELDS}
    if record.get("reviewed_assignment") != expected_assignment:
        errors.append(f"{label}: assignment review payload differs from governed assignment")
    if record.get("reviewed_assignment_sha256") != reviewed_assignment_digest(row):
        errors.append(f"{label}: assignment review digest differs from governed assignment")


def checked_method_commit(root: Path, commit: str, errors: list[str], label: str) -> None:
    if not SHA_RE.fullmatch(commit):
        errors.append(f"{label}: governing method commit missing")
        return
    exists = subprocess.run(["git", "cat-file", "-e", f"{commit}:research/decision-method.md"],
                            cwd=root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
    ancestor = subprocess.run(["git", "merge-base", "--is-ancestor", commit, "HEAD"],
                              cwd=root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
    if exists.returncode or ancestor.returncode:
        errors.append(f"{label}: governing method commit lacks an ancestor method source")


def load_crosswalk(path: Path, errors: list[str], require_review: bool,
                   freeze_ids: set[str]) -> dict[str, set[str]]:
    if not path.is_file():
        errors.append(f"missing crosswalk: {path}")
        return {}
    mapping = {}
    for n, row in enumerate(csv.DictReader(path.open(encoding="utf-8-sig", newline="")), 2):
        text = row.get("constraint_string", "")
        if not text or text in mapping:
            errors.append(f"crosswalk line {n}: missing/duplicate constraint string")
            continue
        expected = hashlib.sha256(text.encode("utf-8")).hexdigest()
        if row.get("string_sha256") != expected:
            errors.append(f"crosswalk line {n}: string SHA mismatch")
        cls = row.get("class", "")
        target = {x.strip() for x in row.get("target", "").split(";") if x.strip()}
        if cls == "HC-xx":
            if not target or not all(HC_RE.fullmatch(t) for t in target):
                errors.append(f"crosswalk line {n}: invalid HC target")
        elif cls == "F-xx":
            if not target or not target <= freeze_ids:
                errors.append(f"crosswalk line {n}: invalid freeze target")
        elif cls not in {"NON_GOAL", "PROGRAM", "DECISION_INPUT"}:
            errors.append(f"crosswalk line {n}: invalid class {cls!r}")
        if any(t.startswith("F-") and t not in freeze_ids for t in target):
            errors.append(f"crosswalk line {n}: unregistered objective Appendix B freeze")
        if not row.get("classifier") or not row.get("rationale") or (require_review and not row.get("reviewer")):
            errors.append(f"crosswalk line {n}: unsigned or unexplained classification")
        mapping[text] = {t for t in target if HC_RE.fullmatch(t)}
    return mapping


def metric_check(row: dict, registry: dict, errors: list[str], label: str, require_every_od: bool) -> None:
    by_od = row.get("primary_metrics_by_od") or {}
    no_primary = row.get("no_primary_metric_reason_by_od") or {}
    blockers = row.get("metric_registration_blockers_by_od") or {}
    if not isinstance(by_od, dict):
        errors.append(f"{label}: primary_metrics_by_od must be a mapping")
        return
    if not isinstance(no_primary, dict):
        errors.append(f"{label}: no_primary_metric_reason_by_od must be a mapping")
        return
    if not isinstance(blockers, dict):
        errors.append(f"{label}: metric_registration_blockers_by_od must be a mapping")
        return
    od_ids = set(row.get("od_ids") or [])
    dispositions = (set(by_od), set(no_primary), set(blockers))
    if set.union(*dispositions) - od_ids:
        errors.append(f"{label}: metric disposition assigned outside od_ids")
    if any(dispositions[i] & dispositions[j] for i in range(3) for j in range(i + 1, 3)):
        errors.append(f"{label}: OD has overlapping primary, no-primary, or metric-registration blocker dispositions")
    if require_every_od and set.union(*dispositions) != od_ids:
        errors.append(f"{label}: every OD needs primary metric, reviewed no-primary rationale, or reviewed metric-registration blocker")
    justifications = row.get("primary_metric_justifications") or {}
    for od, name in by_od.items():
        metric = registry.get(name)
        if metric is None:
            errors.append(f"{label}: unregistered primary metric {name!r}")
            continue
        role = metric.get("role")
        if role not in {"banded", "diagnostic"}:
            errors.append(f"{label}: {name} has non-primary role {role}")
        if role == "diagnostic":
            rationale = justifications.get(od, "")
            objective_components = metric.get("od_metrics", [])
            if not rationale or name not in rationale or not any(code in rationale for code in objective_components) or "component" not in rationale.casefold():
                errors.append(f"{label}: diagnostic primary metric {name} lacks named component justification")
        mid = od.replace("OD-", "M") + "."
        if not any(m.startswith(mid) for m in metric.get("od_metrics", [])):
            errors.append(f"{label}: {name} does not measure {od} in metric registry")
    for od, reason in no_primary.items():
        if not isinstance(reason, str) or len(reason.strip()) < 20:
            errors.append(f"{label}: {od} no-primary rationale lacks reviewed detail")
    for od, reason in blockers.items():
        if (not isinstance(reason, str) or len(reason.strip()) < 40 or
                not any(term in reason.casefold() for term in ("instrument", "role", "band"))):
            errors.append(f"{label}: {od} metric-registration blocker lacks exact missing instrument/role/band detail")


def validate_row(row: dict, schema: dict, metrics: dict, crosswalk: dict[str, set[str]],
                 look_events: dict[str, dict], root: Path, gate_a: bool) -> list[str]:
    label = row.get("decision_id", "<missing decision_id>")
    errors = [f"{label}: {p}" for p in schema_validate(row, schema)]
    if errors:
        return errors
    hc_ids = row["hc_ids"]
    od_ids = row["od_ids"]
    if not all(HC_RE.fullmatch(x) for x in hc_ids):
        errors.append(f"{label}: invalid HC ID")
    if not all(OD_RE.fullmatch(x) for x in od_ids):
        errors.append(f"{label}: invalid OD ID")
    required_hc = set()
    for text in row["hard_constraints"]:
        if BARE_INVARIANT.fullmatch(text):
            required_hc.update(I_TO_HC[int(text[1:])])
        elif text not in crosswalk:
            if gate_a or row["status"] == "DECIDED":
                errors.append(f"{label}: uncrosswalked hard constraint {text!r}")
        else:
            required_hc.update(crosswalk[text])
    if not required_hc <= set(hc_ids):
        errors.append(f"{label}: hc_ids omit crosswalk targets {sorted(required_hc-set(hc_ids))}")
    types = set(row["evidence_route_types"])
    if row["decision_type"] != "UNASSIGNED" and row["decision_type"] not in types:
        errors.append(f"{label}: decision_type missing from evidence_route_types")
    if row["blocker_class"] == "HUMAN_PARTICIPANTS" and "HUMAN-FACING" not in types:
        errors.append(f"{label}: human-participant route cannot be dropped")
    if gate_a or row["status"] == "DECIDED":
        if row["decision_type"] == "UNASSIGNED" or not types:
            errors.append(f"{label}: G-A method assignment incomplete")
        if not od_ids:
            if (label not in METHOD_ADMISSION_ONLY_DECISIONS or
                    len(row["od_applicability_rationale"].strip()) < 20 or
                    row["primary_metrics_by_od"] or row["no_primary_metric_reason_by_od"] or
                    row["metric_registration_blockers_by_od"]):
                errors.append(f"{label}: empty product OD requires reviewed method-admission exception")
        elif row["od_applicability_rationale"].strip():
            errors.append(f"{label}: OD applicability exception cannot accompany assigned product ODs")
        if not hc_ids and not row["hc_applicability_rationale"].strip():
            errors.append(f"{label}: zero applicable HC requires reviewed applicability rationale")
        if not row["assigning_session"] or not row["assignment_review_ref"]:
            errors.append(f"{label}: independent assignment/review missing")
        checked_assignment_review(root, row, errors, label)
        checked_method_commit(root, row["method_commit_sha"], errors, label)
    metric_check(row, metrics, errors, label, require_every_od=gate_a or row["status"] == "DECIDED")
    assessments = row["hc_assessments"]
    assessed = {a["hc_id"] for a in assessments}
    if len(assessed) != len(assessments) or assessed - set(hc_ids):
        errors.append(f"{label}: duplicate or unassigned HC assessment")
    if row["status"] != "DECIDED":
        return errors
    if row["metric_registration_blockers_by_od"]:
        errors.append(f"{label}: DECIDED retains unresolved canonical metric registration for "
                      f"{sorted(row['metric_registration_blockers_by_od'])}")
    if row["blocker_class"] != "NONE":
        errors.append(f"{label}: DECIDED retains blocker_class {row['blocker_class']}")
    candidates = {c["candidate_id"] for c in row["candidate_set"]}
    if len(candidates) != len(row["candidate_set"]):
        errors.append(f"{label}: duplicate candidate ID")
    for exclusion in row["candidate_exclusion_reasons"]:
        if exclusion.get("candidate_id") not in candidates or not re.fullmatch(r"R[0-9]|R10|L0", exclusion.get("rule_id", "")):
            errors.append(f"{label}: candidate exclusion lacks valid candidate/rule")
        if not exclusion.get("evidence_ref"):
            errors.append(f"{label}: candidate exclusion lacks evidence ref")
        else:
            checked_pinned_ref(root, row, exclusion["evidence_ref"], errors, label)
        if exclusion.get("rule_id") == "R1":
            if not re.fullmatch(r"HC-\d{2}|F-\d{2}", exclusion.get("invariant_violated", "")) or not exclusion.get("mechanical_test_ref"):
                errors.append(f"{label}: R1 exclusion lacks HC/freeze and mechanical test")
            else:
                checked_pinned_ref(root, row, exclusion["mechanical_test_ref"], errors, label)
    if assessed != set(hc_ids):
        errors.append(f"{label}: every applicable HC needs split assessment")
    for assessment in assessments:
        for split in SPLITS:
            item = assessment[split]
            if item["status"] == "EXCLUDED_PLATFORM":
                platform = item.get("platform_exclusion", "").strip()
                if not platform or platform.casefold() not in row["decision_scope"].casefold():
                    errors.append(f"{label}: {assessment['hc_id']} {split} platform exclusion absent from decision_scope")
            elif item["status"] != "PASS":
                errors.append(f"{label}: {assessment['hc_id']} {split} lacks PASS evidence")
            if not item["evidence_refs"]:
                errors.append(f"{label}: {assessment['hc_id']} {split} lacks evidence")
            for ref in item["evidence_refs"]:
                checked_pinned_ref(root, row, ref, errors, label)
    if row["cost_tier"] is None or not row["cost_assessment_ref"]:
        errors.append(f"{label}: computed cost tier and assessment missing")
    else:
        checked_pinned_ref(root, row, row["cost_assessment_ref"], errors, label)
    if not row["gate_audit_refs"]:
        errors.append(f"{label}: gate-audit references missing")
    for ref in row["gate_audit_refs"]:
        if not re.fullmatch(rf"research/decisions/{re.escape(label)}/gate-audit-\d+\.md", ref):
            errors.append(f"{label}: gate audit must use the §8.6 decision path")
        checked_pinned_ref(root, row, ref, errors, label)
    if row["gate_audit_refs"] and not set(row["gate_audit_refs"]) & {
            ref for entry in row["history"] for ref in entry.get("evidence_refs", [])}:
        errors.append(f"{label}: history does not cite a gate-audit artifact")
    for field in ("selected_decision", "decision_scope", "known_limitations", "reopen_trigger",
                  "external_review_requirement", "confidence", "remaining_unknowns_ref"):
        if not row[field]:
            errors.append(f"{label}: {field} missing")
    if row["remaining_unknowns_ref"]:
        checked_pinned_ref(root, row, row["remaining_unknowns_ref"], errors, label)
    if row["confidence"] not in {"MEDIUM", "HIGH"}:
        errors.append(f"{label}: DECIDED requires MEDIUM or HIGH confidence")
    if row["cost_tier"] in {"T3", "T4"} and row["confidence"] != "HIGH":
        if not any(h.get("owner_acceptance_ref") for h in row["history"]):
            errors.append(f"{label}: T3/T4 needs HIGH confidence or owner acceptance of each cap")
    for field in ("complexity_cost", "dependency_cost", "security_cost", "wire_cost"):
        if not row[field]:
            errors.append(f"{label}: {field} missing")
    if not row["counterevidence"] or not re.search(r"research/experiments/[^\s]+/adversarial-search/", row["counterevidence"]):
        errors.append(f"{label}: counterevidence lacks committed adversarial-search artifact")
    if not re.fullmatch(r"research/experiments/EXP-[A-Z0-9]+-\d{3}/adversarial-search/[^/]+", row["adversarial_search_ref"]):
        errors.append(f"{label}: adversarial-search manifest reference missing")
    else:
        checked_pinned_ref(root, row, row["adversarial_search_ref"], errors, label)
        if row["adversarial_search_ref"] not in row["counterevidence"]:
            errors.append(f"{label}: counterevidence does not cite its pinned adversarial-search manifest")
    if not row["sensitivity_analysis"]:
        errors.append(f"{label}: sensitivity analysis missing")
    elif types == {"FORMAL"} and "not applicable" not in row["sensitivity_analysis"].casefold():
        errors.append(f"{label}: FORMAL sensitivity needs reasoned not-applicable record")
    if ("EMPIRICAL" in types or "HUMAN-FACING" in types) and not row["experiment_ids"]:
        errors.append(f"{label}: decision experiments missing")
    if row["experiment_ids"]:
        if len(row["experiment_ids"]) != len(set(row["experiment_ids"])):
            errors.append(f"{label}: duplicate experiment ID")
        if not row["raw_result_refs"] or not row["normalized_result_refs"]:
            errors.append(f"{label}: experiment raw/normalized evidence missing")
        if not row["raw_verification_refs"] or not row["analysis_trace_refs"]:
            errors.append(f"{label}: verify-raw and analysis trace evidence missing")
        for experiment_id in row["experiment_ids"]:
            if not re.fullmatch(r"EXP-[A-Z0-9]+-\d{3}", experiment_id):
                errors.append(f"{label}: invalid experiment id {experiment_id!r}")
                continue
            checked_pinned_ref(root, row, f"research/experiments/{experiment_id}/spec.yaml", errors, label)
            checked_pinned_ref(root, row, f"research/experiments/{experiment_id}/outcome.json", errors, label)
        for field in ("raw_result_refs", "normalized_result_refs", "raw_verification_refs", "analysis_trace_refs"):
            for ref in row[field]:
                checked_pinned_ref(root, row, ref, errors, label)
    if "EMPIRICAL" in types or "HUMAN-FACING" in types:
        if not row["heldout_result_refs"] or not row["freeze_ref"]:
            errors.append(f"{label}: empirical held-out/freeze evidence missing")
        if not row["validation_look_refs"]:
            errors.append(f"{label}: no validation look reference")
        for ref in row["heldout_result_refs"]:
            checked_pinned_ref(root, row, ref, errors, label)
        if row["freeze_ref"]:
            checked_pinned_ref(root, row, row["freeze_ref"], errors, label)
    if "EXTERNAL" in types:
        checked_pinned_ref(root, row, row["external_review_requirement"], errors, label)
    for look_id in row["validation_look_refs"]:
        event = look_events.get(look_id)
        if event is None or label not in event.get("decision_ids", []) or not event.get("executed"):
            errors.append(f"{label}: look {look_id} lacks executed registry evidence")
    history = [h for h in row["history"] if h.get("method_commit_sha") == row["method_commit_sha"]
               and set((h.get("rule_outcomes") or {})) == RULES and "deviations" in h and h.get("evidence_refs")]
    if not history:
        errors.append(f"{label}: history lacks R0-R10 outcomes, governing SHA and deviations")
    else:
        h = history[-1]
        if any(not value or any(block in value.upper() for block in ("BLOCKED", "UNVERIFIED", "NOT_RUN"))
               for value in h["rule_outcomes"].values()):
            errors.append(f"{label}: R0-R10 history retains blocked/unverified rule")
        if "PASS" not in h["rule_outcomes"]["R0"].upper() or not h.get("completeness_critic_ref"):
            errors.append(f"{label}: R0 candidate completeness critic missing")
        if "FORMAL" in types and (not h.get("formal_search_ref") or not h.get("formal_reviewer_ref")):
            errors.append(f"{label}: FORMAL counterexample search/reviewer missing")
    for entry in row["history"]:
        for ref in entry.get("evidence_refs", []):
            checked_pinned_ref(root, row, ref, errors, label)
        for field in ("completeness_critic_ref", "formal_search_ref", "formal_reviewer_ref", "owner_acceptance_ref"):
            if entry.get(field):
                checked_pinned_ref(root, row, entry[field], errors, label)
    return errors


def validate(root: Path, gate_a: bool = False) -> dict:
    errors = []
    _, schema = build_schemas()
    ledger_path = root / "research/decision-ledger.jsonl"
    rows = [json.loads(x) for x in ledger_path.read_text(encoding="utf-8").splitlines() if x.strip()]
    ids = [r.get("decision_id") for r in rows]
    if len(ids) != len(set(ids)):
        errors.append("decision-ledger: duplicate decision_id")
    if ids != sorted(ids):
        errors.append("decision-ledger: rows not sorted by decision_id")
    thresholds = json.loads((root / "research/methods/thresholds.json").read_text(encoding="utf-8"))
    try:
        freeze_ids = registered_freezes(root)
    except (ValueError, IndexError, OSError) as exc:
        errors.append(f"objective freeze register invalid: {exc}")
        freeze_ids = set()
    crosswalk = load_crosswalk(root / "research/methods/constraint-crosswalk.csv", errors,
                               require_review=gate_a or any(r.get("status") == "DECIDED" for r in rows),
                               freeze_ids=freeze_ids)
    if any(r.get("status") == "DECIDED" for r in rows):
        gate_d = gate_admission.gate_d_status(root)
        if gate_d["status"] != "PASS":
            errors.append("G-D integration gate OPEN: " + "; ".join(gate_d["reasons"]))
    looks_path = root / "research/decisions/validation-looks.jsonl"
    look_events = {}
    if looks_path.is_file():
        events = [json.loads(line) for line in looks_path.read_text(encoding="utf-8").splitlines() if line.strip()]
        try:
            look_plan, look_ledger = validation_looks.load_inputs(root)
            validation_looks.audit(events, look_plan, look_ledger, root)
        except (ValueError, OSError, KeyError, TypeError) as exc:
            errors.append(f"validation-look registry invalid: {exc}")
        for event in events:
            look_id = event.get("look_id")
            if event.get("event_type") == "REGISTERED":
                look_events[look_id] = {**event, "executed": False}
            elif event.get("event_type") == "EXECUTED" and look_id in look_events:
                look_events[look_id]["executed"] = True
    elif gate_a:
        errors.append("validation-look registry missing")
    for row in rows:
        errors.extend(validate_row(row, schema, thresholds.get("metric_thresholds", {}), crosswalk,
                                   look_events, root, gate_a))
    return {"rows": len(rows), "decided": sum(r.get("status") == "DECIDED" for r in rows),
            "errors": errors, "pass": not errors}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--root", type=Path, default=ROOT)
    ap.add_argument("--gate-a", action="store_true")
    args = ap.parse_args()
    result = validate(args.root.resolve(), args.gate_a)
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0 if result["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
