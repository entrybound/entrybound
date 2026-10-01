#!/usr/bin/env python3
"""Fail-closed admission for decision-method.md §14 integration prerequisites.

The public register records accepted evidence, never inferred success from a
file's presence.  Until an item has a committed, hash-bound receipt, independent
review, and an implemented live verifier, its state is OPEN.  This tool reads no
held-out content.  It does not change the decision ledger or the register.
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


ROOT = Path(__file__).resolve().parents[3]
REGISTER = Path("research/methods/integration-gate-state.json")
METHOD = Path("research/decision-method.md")
HEX40 = re.compile(r"[0-9a-f]{40}\Z")
HEX64 = re.compile(r"[0-9a-f]{64}\Z")
SCHEMA = "entrybound.method-gates.v1"

# The timing strings are transcribed from §14; owner_scope names a public area
# of work, not a private execution package or an approval identity.
ITEMS = {
    1: ("G-B", "thresholds and ebr"),
    2: ("G-B", "decision analysis"),
    3: ("G-B for timing", "experiment runner"),
    4: ("G-B for timing and memory", "calibration"),
    5: ("G-B; G-C", "held-out custody"),
    6: ("G-D", "decision ledger validation"),
    7: ("before the first validation look", "cost accounting"),
    8: ("every experiment", "outcome records"),
    9: ("before the first validation look; G-D for DEC-ECO-078", "rule simulation"),
    10: ("G-A", "constraint classification"),
    11: ("with item 10", "objective screen"),
    12: ("G-A", "decision ledger assignments"),
    13: ("G-B", "workload mix"),
    14: ("G-A", "validation look registry"),
    15: ("G-C", "held-out custody and role separation"),
    16: ("before any OD-16 result", "privacy attack evaluation"),
    17: ("before a candidate's first HC-06 record", "allocation measurement"),
    18: ("before any remote result", "remote calibration"),
    19: ("before any CC outcome", "specialist baselines"),
}

# An item without an implemented verifier cannot become PASS from a declared
# receipt.  Additional verifiers are added alongside each future deliverable.
REQUIRED_ARTIFACTS = {
    10: {"research/methods/constraint-crosswalk.csv", "research/archetypal-objective.md",
         "research/tools/ledger/constraint_crosswalk.py",
         "research/tools/ledger/tests/test_constraint_crosswalk.py"},
    11: {"research/tools/ledger/objective_screen.py", "research/archetypal-objective.md",
         "research/requirement-ledger.csv", "research/decision-ledger.jsonl",
         "research/tools/ledger/tests/test_objective_screen.py"},
    12: {"research/decision-ledger.jsonl", "research/tools/ledger/decision-ledger.schema.json",
         "research/tools/ledger/validate_decisions.py", "research/tools/ledger/assemble.py",
         "research/methods/thresholds.json", "research/methods/constraint-crosswalk.csv",
         "research/tools/experiments/validation_looks.py",
         "research/tools/tests/test_decision_method_schema.py"},
    14: {"research/decisions/validation-looks.jsonl",
         "research/tools/experiments/validation_looks.py", "research/experiments/_program/look-plan.csv",
         "research/decision-ledger.jsonl", "research/tools/tests/test_validation_looks.py"},
}

GA = {10, 11, 12, 14}
GB = {1, 2, 5, 13}
TIMING = {3, 4}
MEMORY = {4}
RESULT_EVENTS = {"first_result", "first_validation", "first_od16_result",
                 "first_remote_result", "first_cc_outcome"}


def sha(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def safe_path(root: Path, ref: str) -> Path | None:
    if (not isinstance(ref, str) or not ref or
            any(char in ref for char in ("\\", ":", "\x00", "\n", "\r"))):
        return None
    relative = Path(ref)
    if relative.is_absolute() or any(part in {".", ".."} for part in relative.parts):
        return None
    def protected(parts: tuple[str, ...]) -> bool:
        if any(part in {".git", ".codex", ".agents", "target", "private"} for part in parts):
            return True
        # Public custody reports and the lock may be admitted after eligible
        # review.  Protected payload and fingerprint trees never enter this
        # generic receipt reader, including through an in-repository symlink.
        return (parts[:3] in {("research", "corpus", "heldout"),
                              ("research", "corpus", "fingerprints")} or
                parts[:3] == ("research", "corpus", "items") and "heldout" in parts)

    parts = tuple(part.casefold() for part in relative.parts)
    if protected(parts):
        return None
    target = (root / relative).resolve()
    base = root.resolve()
    if not target.is_relative_to(base):
        return None
    resolved_parts = tuple(part.casefold() for part in target.relative_to(base).parts)
    return None if protected(resolved_parts) else target


def git(root: Path, *args: str) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(["git", *args], cwd=root, capture_output=True, check=False)


def committed_file(root: Path, commit: str, ref: str, expected: str) -> bool:
    # Both the attested source commit and the current HEAD must contain the
    # current bytes.  A dirty or replaced artifact cannot inherit old proof.
    current = safe_path(root, ref)
    if current is None or not current.is_file() or sha(current.read_bytes()) != expected:
        return False
    for revision in (commit, "HEAD"):
        shown = git(root, "show", f"{revision}:{ref}")
        if shown.returncode != 0 or sha(shown.stdout) != expected:
            return False
    return True


def command(root: Path, argv: list[str]) -> tuple[bool, str, str]:
    try:
        run = subprocess.run(argv, cwd=root, capture_output=True, check=False)
    except OSError as exc:
        return False, "", str(exc)
    output = run.stdout + b"\n-- stderr --\n" + run.stderr
    # Test runtimes vary even on identical source.  The receipt binds the
    # command's deterministic verdict text while every admission reruns it.
    normalized = re.sub(rb"\bin [0-9]+(?:\.[0-9]+)?s\b", b"in <duration>s", output)
    return run.returncode == 0, sha(normalized), output.decode("utf-8", errors="replace")[:400]


def generated_objective_matches(root: Path, output: str) -> bool:
    objective = (root / "research/archetypal-objective.md").read_text(encoding="utf-8")
    begin = "<!-- BEGIN GENERATED: objective_screen.py -->\n"
    end = "<!-- END GENERATED: objective_screen.py -->"
    if objective.count(begin) != 1 or objective.count(end) != 1:
        return False
    actual = objective.split(begin, 1)[1].split(end, 1)[0].rstrip("\r\n")
    return actual == output.replace("\r\n", "\n").rstrip("\r\n")


def crosswalk_signed(root: Path) -> tuple[bool, str]:
    path = root / "research/methods/constraint-crosswalk.csv"
    ledger = root / "research/decision-ledger.jsonl"
    objective = root / "research/archetypal-objective.md"
    if not all(p.is_file() for p in (path, ledger, objective)):
        return False, "crosswalk, ledger or objective missing"
    with path.open(encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle))
    strings = {value for line in ledger.read_text(encoding="utf-8").splitlines() if line
               for value in json.loads(line)["hard_constraints"]
               if not re.fullmatch(r"I(?:[1-9]|[12][0-9]|3[01])", value)}
    if len(rows) != len(strings) or {r.get("constraint_string") for r in rows} != strings:
        return False, "crosswalk does not cover exact distinct constraint strings"
    frozen = set(re.findall(r"^\| (F-\d{2}) \|", objective.read_text(encoding="utf-8"), re.M))
    for row in rows:
        if not row.get("reviewer") or row["reviewer"] == row.get("classifier"):
            return False, "crosswalk classification lacks independent row sign-off"
        if row["class"] == "F-xx" and not set(row["target"].split(";")) <= frozen:
            return False, "crosswalk targets unregistered objective freeze"
    return True, f"{len(rows)} signed rows and registered freeze targets"


def live_checks(root: Path, item: int) -> dict[str, tuple[bool, str, str]]:
    py = sys.executable
    if item == 10:
        exact = command(root, [py, "research/tools/ledger/constraint_crosswalk.py", "--check"])
        signed, detail = crosswalk_signed(root)
        return {"crosswalk_exact": exact,
                "crosswalk_signed": (signed, sha(detail.encode()), detail),
                "crosswalk_test": command(root, [py, "-m", "unittest",
                                                 "research.tools.ledger.tests.test_constraint_crosswalk", "-q"])}
    if item == 11:
        run = subprocess.run([py, "research/tools/ledger/objective_screen.py", "--repo", "."],
                             cwd=root, capture_output=True, check=False)
        out = run.stdout.decode("utf-8", errors="replace")
        generated = run.returncode == 0 and generated_objective_matches(root, out)
        test = command(root, [py, "-m", "unittest",
                              "research.tools.ledger.tests.test_objective_screen", "-q"])
        return {"objective_generated": (generated, sha(run.stdout),
                                         "Appendix C matches generator" if generated else "Appendix C mismatch or generator failure"),
                "objective_test": test}
    if item == 12:
        return {"ledger_gate_a": command(root, [py, "research/tools/ledger/validate_decisions.py", "--gate-a"]),
                "ledger_schema_test": command(root, [py, "-m", "pytest", "-p", "no:cacheprovider",
                                                      "research/tools/tests/test_decision_method_schema.py", "-q"])}
    if item == 14:
        return {"look_registry": command(root, [py, "research/tools/experiments/validation_looks.py", "validate"]),
                "look_registry_test": command(root, [py, "-m", "pytest", "-p", "no:cacheprovider",
                                                     "research/tools/tests/test_validation_looks.py", "-q"])}
    return {}


def check_receipt(root: Path, item: int, receipt: object,
                  checks: dict[str, tuple[bool, str, str]]) -> list[str]:
    if item not in REQUIRED_ARTIFACTS:
        return ["item verifier not implemented; an artifact declaration cannot grant PASS"]
    if not isinstance(receipt, dict):
        return ["accepted evidence receipt missing"]
    required = {"item", "method_sha256", "source_commit", "artifact_sha256_by_path",
                "check_output_sha256", "review_ref", "review_sha256",
                "producer_session", "reviewer_session"}
    if set(receipt) != required or receipt.get("item") != item:
        return ["accepted evidence receipt shape/item mismatch"]
    errors = []
    method = root / METHOD
    if receipt["method_sha256"] != sha(method.read_bytes()):
        errors.append("method digest differs from current public method")
    commit = receipt["source_commit"]
    if not isinstance(commit, str) or not HEX40.fullmatch(commit):
        errors.append("source commit must be a full Git SHA")
    elif git(root, "merge-base", "--is-ancestor", commit, "HEAD").returncode != 0:
        errors.append("source commit is not an ancestor of HEAD")
    artifacts = receipt["artifact_sha256_by_path"]
    required_artifacts = REQUIRED_ARTIFACTS[item] | {METHOD.as_posix()}
    if not isinstance(artifacts, dict) or not required_artifacts <= set(artifacts):
        errors.append("required exact-source artifacts missing from receipt")
    else:
        for ref, digest in artifacts.items():
            if not isinstance(digest, str) or not HEX64.fullmatch(digest) or not committed_file(root, commit, ref, digest):
                errors.append(f"artifact missing, dirty or hash mismatch: {ref}")
    outputs = receipt["check_output_sha256"]
    if not checks:
        errors.append("live verifier was not run")
    if not isinstance(outputs, dict) or set(outputs) != set(checks):
        errors.append("live test receipt does not cover the exact item checks")
    else:
        for name, (passed, digest, _) in checks.items():
            if not passed or outputs[name] != digest:
                errors.append(f"live verification failed or changed: {name}")
    producer, reviewer = receipt["producer_session"], receipt["reviewer_session"]
    if not isinstance(producer, str) or not producer or not isinstance(reviewer, str) or not reviewer or producer == reviewer:
        errors.append("independent producer/reviewer sessions are required")
    if item == 10 and isinstance(reviewer, str) and reviewer:
        with (root / "research/methods/constraint-crosswalk.csv").open(encoding="utf-8", newline="") as handle:
            if any(reviewer not in row.get("reviewer", "") for row in csv.DictReader(handle)):
                errors.append("crosswalk row sign-offs do not name the receipt reviewer")
    review_ref, review_sha = receipt["review_ref"], receipt["review_sha256"]
    if not isinstance(review_sha, str) or not HEX64.fullmatch(review_sha) or not committed_file(root, commit, review_ref, review_sha):
        errors.append("independent review artifact missing, dirty or hash mismatch")
    else:
        review = (root / review_ref).read_text(encoding="utf-8", errors="replace")
        if reviewer not in review or str(item) not in review or not all(d in review for d in artifacts.values()):
            errors.append("review does not bind reviewer, item and exact artifact digests")
    return errors


def evaluate(root: Path, register: object, run_live: bool = True) -> dict:
    errors = []
    if not isinstance(register, dict) or set(register) != {"schema", "method_sha256", "items"}:
        return {"errors": ["gate register shape invalid"], "items": {}}
    if register["schema"] != SCHEMA:
        errors.append("gate register schema mismatch")
    method = root / METHOD
    if not method.is_file() or register["method_sha256"] != sha(method.read_bytes()):
        errors.append("gate register method digest stale")
    if method.is_file():
        section = method.read_text(encoding="utf-8").split("## 14. Integration prerequisites and gates", 1)
        if len(section) != 2:
            errors.append("public method §14 missing")
        else:
            table = section[1].split("\n---", 1)[0]
            gates = {int(n): timing.strip() for n, timing in re.findall(
                r"^\| (\d{1,2}) \|.*\| ([^|]+) \|$", table, re.M)}
            if gates != {n: timing for n, (timing, _) in ITEMS.items()}:
                errors.append("public method §14 timing differs from admission map")
    rows = register["items"]
    if not isinstance(rows, list) or len(rows) != 19:
        return {"errors": errors + ["gate register must contain exactly 19 items"], "items": {}}
    by_number = {}
    for row in rows:
        if not isinstance(row, dict) or set(row) != {"number", "timing", "owner_scope", "status", "open_reason", "evidence"}:
            errors.append("gate register item shape invalid")
            continue
        number = row["number"]
        if type(number) is not int or number not in ITEMS or number in by_number:
            errors.append(f"gate register unknown/duplicate item {number!r}")
            continue
        by_number[number] = row
    if set(by_number) != set(ITEMS):
        errors.append("gate register item membership mismatch")
    result = {}
    for item in sorted(by_number):
        row = by_number[item]
        if (row["timing"], row["owner_scope"]) != ITEMS[item]:
            errors.append(f"item {item}: timing or owner scope differs from §14 register")
        status = row["status"]
        checks = live_checks(root, item) if run_live and item in REQUIRED_ARTIFACTS else {}
        if status == "OPEN":
            if not isinstance(row["open_reason"], str) or not row["open_reason"] or row["evidence"] is not None:
                errors.append(f"item {item}: OPEN must have reason and no accepted evidence")
            why = [row["open_reason"]] + [f"{name}: {detail}" for name, (ok, _, detail) in checks.items() if not ok]
            result[item] = {"status": "OPEN", "reasons": why,
                            "live_checks": {name: ok for name, (ok, _, _) in checks.items()}}
        elif status == "PASS":
            reasons = check_receipt(root, item, row["evidence"], checks)
            if row["open_reason"]:
                reasons.append("PASS item retains an open reason")
            result[item] = {"status": "PASS" if not reasons else "OPEN", "reasons": reasons,
                            "live_checks": {name: ok for name, (ok, _, _) in checks.items()}}
        else:
            errors.append(f"item {item}: unknown status {status!r}")
            result[item] = {"status": "OPEN", "reasons": ["invalid status"], "live_checks": {}}
    return {"errors": errors, "items": result}


def required_items(event: str, *, decision_ids: bool = False, measurement: str = "unknown",
                   decision_id: str = "", routes: set[str] | None = None,
                   has_validation_looks: bool = False) -> set[int]:
    if event not in {"record_admission", "every_experiment", "first_result", "first_validation",
                     "freeze", "decided", "first_od16_result", "first_hc06_record",
                     "first_remote_result", "first_cc_outcome"}:
        raise ValueError(f"unknown admission event: {event}")
    if measurement not in {"unknown", "timing", "memory", "both", "other"}:
        raise ValueError(f"unknown measurement kind: {measurement}")
    if event == "record_admission":
        return set(GA)
    if event == "every_experiment":
        return {8} | (GA if decision_ids else set())
    if event == "first_hc06_record":
        return GA | {8, 17}
    required = GA | GB | {8}
    if measurement in {"unknown", "timing", "both"}:
        required |= TIMING
    elif measurement == "memory":
        required |= MEMORY
    if event == "first_validation":
        return required | {7, 9}
    if event == "freeze":
        return required | {15}
    if event == "first_od16_result":
        return required | {16}
    if event == "first_remote_result":
        return required | {18}
    if event == "first_cc_outcome":
        return required | {19}
    if event == "decided":
        required |= {6}
        if not decision_id:
            raise ValueError("decided admission requires decision_id")
        if not routes or {"EMPIRICAL", "HUMAN-FACING"} & routes:
            required |= {15}
        if has_validation_looks:
            required |= {7, 9}
        if decision_id == "DEC-ECO-078":
            required |= {9}
    return required


def graded_route_errors(ledger_assignments: dict[str, dict],
                        metric_claims: dict[str, dict]) -> list[str]:
    """Reject graded selection/utility use of an OD with no primary route.

    The caller must supply a pre-registered, exact-source metric-use record;
    this function checks its declared mapping against reviewed live v2 rows.
    It is deliberately not a mechanism for adding a late primary metric.
    """
    errors = []
    if not isinstance(metric_claims, dict) or not metric_claims:
        return ["decision-relevant result lacks a nonempty metric-use declaration"]
    for decision, claim in metric_claims.items():
        row = ledger_assignments.get(decision)
        if row is None:
            errors.append(f"{decision}: absent from live ledger")
            continue
        blockers = row.get("metric_registration_blockers_by_od") or {}
        if blockers:
            errors.append(f"{decision}: unresolved canonical metric registration for {sorted(blockers)}")
        if not isinstance(claim, dict) or set(claim) != {"graded", "primary_metrics_by_od", "utility_od_ids", "non_graded_reason"}:
            errors.append(f"{decision}: metric-use declaration shape invalid")
            continue
        graded = claim["graded"]
        primary = claim["primary_metrics_by_od"]
        utility = claim["utility_od_ids"]
        if (type(graded) is not bool or not isinstance(primary, dict) or
                not isinstance(utility, list) or
                not isinstance(claim["non_graded_reason"], str) or
                any(not isinstance(od, str) for od in utility) or
                any(not isinstance(od, str) or not isinstance(metric, str)
                    for od, metric in primary.items())):
            errors.append(f"{decision}: metric-use declaration types invalid")
            continue
        if not graded:
            if primary or utility or not claim["non_graded_reason"]:
                errors.append(f"{decision}: non-graded result needs reason and no selection metric/weight")
            continue
        if not primary or claim["non_graded_reason"]:
            errors.append(f"{decision}: graded result needs declared primary metrics and no non-graded reason")
        assigned = row.get("primary_metrics_by_od") or {}
        no_primary = row.get("no_primary_metric_reason_by_od") or {}
        for od, metric in primary.items():
            if od in no_primary:
                errors.append(f"{decision}: {od} has reviewed no-primary disposition; graded selection is forbidden")
            elif assigned.get(od) != metric:
                errors.append(f"{decision}: {od} metric differs from pre-registered live assignment")
        for od in utility:
            if od in no_primary or od not in primary:
                errors.append(f"{decision}: {od} cannot receive selection/utility weight")
        if len(utility) != len(set(utility)):
            errors.append(f"{decision}: duplicate weighted OD")
    return errors


def admission(event: str, item_results: dict[int, dict], *,
              ledger_assignments: dict[str, dict] | None = None,
              metric_claims: dict[str, dict] | None = None, **context: object) -> dict:
    required = sorted(required_items(event, **context))
    blocked = [number for number in required
               if item_results.get(number, {}).get("status") != "PASS"]
    metric_errors = []
    if event in {"record_admission", "every_experiment"}:
        # The gate receipts cover global prerequisites, including item 8 for
        # every experiment. They cannot establish that this particular
        # committed record was admitted against exact reviewed assignments.
        # That integration verifier is a separate, required future handoff.
        metric_errors.append("committed experiment-to-decision assignment verifier is not implemented")
        if ledger_assignments is not None:
            for decision_id, row in ledger_assignments.items():
                blockers = row.get("metric_registration_blockers_by_od") or {}
                if blockers:
                    metric_errors.append(
                        f"{decision_id}: unresolved canonical metric registration for {sorted(blockers)}"
                    )
    elif event in RESULT_EVENTS:
        # A caller-supplied dict is useful for route diagnostics, but it is
        # not the committed, preregistered spec/result pair.  The exact-source
        # record verifier is added with the experiment integration work.
        metric_errors.append("committed preregistered experiment metric-use verifier is not implemented")
        if ledger_assignments is None or metric_claims is None:
            metric_errors.append("exact-source experiment metric-use declaration not supplied")
        else:
            metric_errors.extend(graded_route_errors(ledger_assignments, metric_claims))
    elif event == "decided":
        decision_id = context.get("decision_id")
        if ledger_assignments is None or decision_id not in ledger_assignments:
            metric_errors.append("reviewed live assignment not supplied for DECIDED blocker check")
        else:
            blockers = ledger_assignments[decision_id].get("metric_registration_blockers_by_od") or {}
            if blockers:
                metric_errors.append(f"{decision_id}: unresolved canonical metric registration for {sorted(blockers)}")
    return {"event": event, "required_items": required, "blocked_items": blocked,
            "metric_errors": metric_errors,
            "status": "PASS" if not blocked and not metric_errors else "BLOCKED"}


def gate_d_status(root: Path) -> dict:
    """Return item 6 admission without invoking the ledger validator.

    The ledger validator may call this for a DECIDED row.  Item 6 remains OPEN
    until its own nonrecursive verifier is implemented with exact evidence.
    """
    try:
        register = json.loads((root / REGISTER).read_text(encoding="utf-8"))
        evaluated = evaluate(root, register, run_live=False)
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as exc:
        return {"status": "OPEN", "reasons": [f"gate register unreadable: {exc}"]}
    reasons = list(evaluated["errors"])
    item = evaluated["items"].get(6, {"status": "OPEN", "reasons": ["item 6 missing"]})
    reasons.extend(item["reasons"])
    return {"status": "PASS" if item["status"] == "PASS" and not reasons else "OPEN",
            "reasons": reasons}


def decision_context(root: Path, decision_id: str) -> tuple[set[str], bool, dict]:
    ledger = root / "research/decision-ledger.jsonl"
    for line in ledger.read_text(encoding="utf-8").splitlines():
        if line.strip():
            row = json.loads(line)
            if row.get("decision_id") == decision_id:
                return (set(row.get("evidence_route_types") or []),
                        bool(row.get("validation_look_refs")), row)
    raise ValueError(f"decision_id absent from ledger: {decision_id}")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("action", choices=("audit", "admit"))
    ap.add_argument("--root", type=Path, default=ROOT)
    ap.add_argument("--event", choices=("record_admission", "every_experiment", "first_result",
                                         "first_validation", "freeze", "decided", "first_od16_result",
                                         "first_hc06_record", "first_remote_result", "first_cc_outcome"))
    ap.add_argument("--decision-id", default="")
    ap.add_argument("--decision-ids-present", action="store_true")
    ap.add_argument("--measurement", choices=("unknown", "timing", "memory", "both", "other"), default="unknown")
    args = ap.parse_args()
    root = args.root.resolve()
    try:
        register = json.loads((root / REGISTER).read_text(encoding="utf-8"))
        evaluated = evaluate(root, register)
        if args.action == "audit":
            print(json.dumps(evaluated, indent=2, ensure_ascii=False))
            return 0 if not evaluated["errors"] else 1
        if not args.event:
            raise ValueError("admit requires --event")
        routes, looks, decision_row = (decision_context(root, args.decision_id)
                                      if args.event == "decided" else (set(), False, None))
        result = admission(args.event, evaluated["items"], decision_ids=args.decision_ids_present,
                           measurement=args.measurement, decision_id=args.decision_id,
                           routes=routes, has_validation_looks=looks,
                           ledger_assignments=({args.decision_id: decision_row}
                                               if decision_row is not None else None))
        result["register_errors"] = evaluated["errors"]
        if evaluated["errors"]:
            result["status"] = "BLOCKED"
        print(json.dumps(result, indent=2, ensure_ascii=False))
        return 0 if result["status"] == "PASS" else 1
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as exc:
        print(f"gate admission: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
