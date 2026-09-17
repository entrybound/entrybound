"""Merge research/experiments/_index/*.jsonl into two derived, deterministic tables:

  research/experiments/index.csv            one row per experiment
  research/experiments/decision-coverage.csv one row per decision in the ledger

and validate cross-references between the experiment index, the experiment
directories on disk, and research/decision-ledger.jsonl.

Stdlib only. Deterministic: all iteration order is sorted before it affects
output, so re-running against unchanged inputs reproduces byte-identical CSVs.

Usage:
    python research/tools/experiments/build_index.py [--check]

Exit codes:
    0  no validation errors
    1  one or more validation errors (CSVs are still written, best-effort)

--check runs validation and prints the report without writing the CSVs.
"""

from __future__ import annotations

import argparse
import csv
import glob
import json
import os
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
INDEX_DIR = ROOT / "research" / "experiments" / "_index"
EXPERIMENTS_DIR = ROOT / "research" / "experiments"
LEDGER_PATH = ROOT / "research" / "decision-ledger.jsonl"
INDEX_CSV = ROOT / "research" / "experiments" / "index.csv"
COVERAGE_CSV = ROOT / "research" / "experiments" / "decision-coverage.csv"

LIST_SEP = "; "
MULTI_SEP = " || "

EXPERIMENT_REQUIRED_FIELDS = [
    "experiment_id",
    "domain",
    "title",
    "program_sections",
    "decision_ids",
    "status",
    "blocked_reason",
    "platforms",
    "compute_hours_estimate",
    "disk_gb_estimate",
    "requires_timing",
    "tooling_to_build",
]
UNCOVERED_REQUIRED_FIELDS = ["decision_id", "reason", "evidence_route"]
VALID_STATUSES = {"READY", "BLOCKED", "NEEDS_TOOLING"}


def rel(p: Path) -> str:
    try:
        return p.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(p)


def join_list(values) -> str:
    if not values:
        return ""
    return LIST_SEP.join(str(v) for v in values)


def dedup_preserve_order(items):
    seen = set()
    out = []
    for it in items:
        if it not in seen:
            seen.add(it)
            out.append(it)
    return out


class Report:
    def __init__(self):
        self.errors = []  # list of dict(kind, detail)

    def add(self, kind, **detail):
        self.errors.append({"kind": kind, **detail})

    def by_kind(self):
        c = Counter(e["kind"] for e in self.errors)
        return dict(sorted(c.items()))


def load_jsonl_records(report: Report):
    """Read every research/experiments/_index/*.jsonl file.

    Returns (experiments: list[dict], uncovered: list[dict]) in deterministic
    (sorted-filename, then file-order) sequence. Malformed lines and records
    missing required fields are reported and skipped rather than raising.
    """
    experiments = []
    uncovered = []
    seen_experiment_ids = {}  # experiment_id -> (file, lineno)
    seen_uncovered_in_file = defaultdict(set)  # file -> set(decision_id)

    files = sorted(glob.glob(str(INDEX_DIR / "*.jsonl")))
    if not files:
        report.add("no_index_files", path=rel(INDEX_DIR))

    for fpath in files:
        fname = os.path.basename(fpath)
        with open(fpath, "r", encoding="utf-8") as fh:
            for lineno, raw in enumerate(fh, 1):
                line = raw.strip()
                if not line:
                    continue
                try:
                    rec = json.loads(line)
                except json.JSONDecodeError as exc:
                    report.add("malformed_json", file=fname, line=lineno, error=str(exc))
                    continue
                if not isinstance(rec, dict):
                    report.add("unexpected_record_shape", file=fname, line=lineno, value=repr(rec)[:200])
                    continue

                if "experiment_id" in rec:
                    missing = [f for f in EXPERIMENT_REQUIRED_FIELDS if f not in rec]
                    if missing:
                        report.add(
                            "experiment_missing_fields",
                            file=fname,
                            line=lineno,
                            experiment_id=rec.get("experiment_id"),
                            fields=missing,
                        )
                    rec["_source_file"] = fname
                    eid = rec.get("experiment_id")
                    if eid in seen_experiment_ids:
                        report.add(
                            "duplicate_experiment_id",
                            experiment_id=eid,
                            first=seen_experiment_ids[eid],
                            second=(fname, lineno),
                        )
                    else:
                        seen_experiment_ids[eid] = (fname, lineno)
                    experiments.append(rec)

                elif "decision_id" in rec:
                    missing = [f for f in UNCOVERED_REQUIRED_FIELDS if f not in rec]
                    if missing:
                        report.add(
                            "uncovered_missing_fields",
                            file=fname,
                            line=lineno,
                            decision_id=rec.get("decision_id"),
                            fields=missing,
                        )
                    rec["_source_file"] = fname
                    did = rec.get("decision_id")
                    if did in seen_uncovered_in_file[fname]:
                        report.add(
                            "duplicate_uncovered_in_same_file",
                            file=fname,
                            decision_id=did,
                        )
                    seen_uncovered_in_file[fname].add(did)
                    uncovered.append(rec)

                else:
                    report.add(
                        "unrecognized_record",
                        file=fname,
                        line=lineno,
                        keys=sorted(rec.keys()),
                    )

    experiments.sort(key=lambda r: r.get("experiment_id") or "")
    uncovered.sort(key=lambda r: (r.get("decision_id") or "", r.get("_source_file") or ""))
    return experiments, uncovered


def load_ledger(report: Report):
    decisions = {}
    order = []
    if not LEDGER_PATH.is_file():
        report.add("ledger_missing", path=rel(LEDGER_PATH))
        return decisions, order
    with open(LEDGER_PATH, "r", encoding="utf-8") as fh:
        for lineno, raw in enumerate(fh, 1):
            line = raw.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError as exc:
                report.add("ledger_malformed_json", line=lineno, error=str(exc))
                continue
            did = rec.get("decision_id")
            if not did:
                report.add("ledger_record_missing_decision_id", line=lineno)
                continue
            if did in decisions:
                report.add("duplicate_ledger_decision_id", decision_id=did, line=lineno)
                continue
            decisions[did] = rec
            order.append(did)
    return decisions, order


def has_spec_glob(exp_dir: Path) -> bool:
    if not exp_dir.is_dir():
        return False
    return any(
        fn.name.startswith("spec") and fn.name.endswith((".yaml", ".yml")) and fn.is_file()
        for fn in exp_dir.iterdir()
    )


def validate_experiment_directories(experiments, report: Report):
    """protocol.md presence, and spec.yaml presence for runner-driven experiments.

    runner_driven is read from the record's explicit `runner_driven` field when
    present (only the platform domain currently declares it). When the field is
    absent, runner-driven status is inferred from whether a spec*.yaml file is
    already present in the experiment directory -- this never manufactures a
    false "missing spec" error for experiments that were designed without a
    runner (analytical/literature/external-review work), while still catching
    drift between a declared spec_files list and what is actually on disk.
    """
    rows_extra = {}
    for rec in experiments:
        eid = rec["experiment_id"]
        exp_dir = EXPERIMENTS_DIR / eid
        protocol_ok = False
        if not exp_dir.is_dir():
            report.add("missing_experiment_directory", experiment_id=eid, path=rel(exp_dir))
        else:
            protocol_ok = (exp_dir / "protocol.md").is_file()
            if not protocol_ok:
                report.add("missing_protocol_md", experiment_id=eid, path=rel(exp_dir / "protocol.md"))

        spec_files = rec.get("spec_files")
        explicit_runner_driven = rec.get("runner_driven")
        if explicit_runner_driven is not None:
            runner_driven = bool(explicit_runner_driven)
        else:
            runner_driven = has_spec_glob(exp_dir)

        spec_ok = True
        if spec_files:
            for sf in spec_files:
                if not (ROOT / sf).is_file():
                    report.add("spec_file_listed_but_missing", experiment_id=eid, spec_file=sf)
                    spec_ok = False
        if runner_driven:
            if spec_files:
                if not any((ROOT / sf).is_file() for sf in spec_files):
                    report.add("runner_driven_missing_spec_yaml", experiment_id=eid, checked=spec_files)
                    spec_ok = False
            elif not has_spec_glob(exp_dir):
                report.add("runner_driven_missing_spec_yaml", experiment_id=eid, checked=["spec*.yaml (glob)"])
                spec_ok = False

        if rec.get("status") not in VALID_STATUSES:
            report.add("invalid_status", experiment_id=eid, status=rec.get("status"))

        rows_extra[eid] = {
            "protocol_ok": protocol_ok,
            "spec_ok": spec_ok,
            "runner_driven_effective": runner_driven,
        }
    return rows_extra


def cross_reference_decisions(experiments, uncovered, ledger_decisions, report: Report):
    """Build decision_id -> covering experiment_ids, and decision_id -> uncovered notes.

    Also validates that every decision_id referenced from either source exists
    in the ledger.
    """
    dec_to_exps = defaultdict(list)
    for rec in experiments:
        eid = rec["experiment_id"]
        for did in rec.get("decision_ids", []) or []:
            if did not in ledger_decisions:
                report.add(
                    "unknown_decision_id_referenced",
                    decision_id=did,
                    referenced_by=eid,
                    source="experiment.decision_ids",
                    file=rec.get("_source_file"),
                )
            dec_to_exps[did].append(eid)

    dec_to_uncovered = defaultdict(list)
    for rec in uncovered:
        did = rec["decision_id"]
        if did not in ledger_decisions:
            report.add(
                "unknown_decision_id_referenced",
                decision_id=did,
                referenced_by=rec.get("_source_file"),
                source="uncovered.decision_id",
            )
        dec_to_uncovered[did].append(rec)

    for did, exps in dec_to_exps.items():
        dec_to_exps[did] = dedup_preserve_order(sorted(exps))

    return dec_to_exps, dec_to_uncovered


def build_coverage_rows(ledger_decisions, ledger_order, dec_to_exps, dec_to_uncovered, report: Report):
    rows = []
    for did in sorted(ledger_order):
        dec = ledger_decisions[did]
        exp_ids = dec_to_exps.get(did, [])
        notes = dec_to_uncovered.get(did, [])
        # Sort cross-domain notes by source file for determinism, dedup identical text.
        notes_sorted = sorted(notes, key=lambda r: r.get("_source_file") or "")
        reasons = dedup_preserve_order([n["reason"] for n in notes_sorted if n.get("reason")])
        routes = dedup_preserve_order([n["evidence_route"] for n in notes_sorted if n.get("evidence_route")])

        covered = bool(exp_ids)
        has_uncovered_reason = bool(reasons)
        if not covered and not has_uncovered_reason:
            report.add("decision_without_coverage_or_reason", decision_id=did)

        rows.append(
            {
                "decision_id": did,
                "program_sections": join_list(dec.get("program_sections")),
                "experiment_ids": join_list(exp_ids),
                "uncovered_reason": MULTI_SEP.join(reasons),
                "evidence_route": MULTI_SEP.join(routes),
                "covered": "Y" if covered else "N",
                "experiment_count": len(exp_ids),
                "cluster": dec.get("cluster", ""),
                "status": dec.get("status", ""),
                "blocker_class": dec.get("blocker_class", ""),
            }
        )
    return rows


def write_index_csv(experiments, rows_extra):
    fieldnames = [
        "experiment_id",
        "domain",
        "status",
        "title",
        "program_sections",
        "decision_ids",
        "decision_count",
        "platforms",
        "compute_hours_estimate",
        "disk_gb_estimate",
        "requires_timing",
        "runner_driven",
        "runner_driven_declared",
        "spec_files",
        "tooling_to_build",
        "blocked_reason",
        "protocol_ok",
        "spec_ok",
        "source_index_file",
    ]
    INDEX_CSV.parent.mkdir(parents=True, exist_ok=True)
    with open(INDEX_CSV, "w", encoding="utf-8", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=fieldnames, lineterminator="\n")
        w.writeheader()
        for rec in experiments:
            eid = rec["experiment_id"]
            extra = rows_extra.get(eid, {})
            declared = rec.get("runner_driven")
            w.writerow(
                {
                    "experiment_id": eid,
                    "domain": rec.get("domain", ""),
                    "status": rec.get("status", ""),
                    "title": rec.get("title", ""),
                    "program_sections": join_list(rec.get("program_sections")),
                    "decision_ids": join_list(rec.get("decision_ids")),
                    "decision_count": len(rec.get("decision_ids") or []),
                    "platforms": join_list(rec.get("platforms")),
                    "compute_hours_estimate": rec.get("compute_hours_estimate", ""),
                    "disk_gb_estimate": rec.get("disk_gb_estimate", ""),
                    "requires_timing": rec.get("requires_timing", ""),
                    "runner_driven": extra.get("runner_driven_effective", ""),
                    "runner_driven_declared": "" if declared is None else declared,
                    "spec_files": join_list(rec.get("spec_files")),
                    "tooling_to_build": join_list(rec.get("tooling_to_build")),
                    "blocked_reason": rec.get("blocked_reason", ""),
                    "protocol_ok": extra.get("protocol_ok", ""),
                    "spec_ok": extra.get("spec_ok", ""),
                    "source_index_file": rec.get("_source_file", ""),
                }
            )


def write_coverage_csv(rows):
    fieldnames = [
        "decision_id",
        "program_sections",
        "experiment_ids",
        "uncovered_reason",
        "evidence_route",
        "covered",
        "experiment_count",
        "cluster",
        "status",
        "blocker_class",
    ]
    COVERAGE_CSV.parent.mkdir(parents=True, exist_ok=True)
    with open(COVERAGE_CSV, "w", encoding="utf-8", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=fieldnames, lineterminator="\n")
        w.writeheader()
        for row in rows:
            w.writerow(row)


def summarize(experiments, ledger_decisions, coverage_rows, report: Report):
    status_counts = Counter(r.get("status") for r in experiments)
    domain_counts = Counter(r.get("domain") for r in experiments)

    def num(v, default=0.0):
        try:
            return float(v)
        except (TypeError, ValueError):
            return default

    total_compute_hours = sum(num(r.get("compute_hours_estimate")) for r in experiments)
    total_disk_gb = sum(num(r.get("disk_gb_estimate")) for r in experiments)
    compute_by_status = defaultdict(float)
    disk_by_status = defaultdict(float)
    for r in experiments:
        compute_by_status[r.get("status")] += num(r.get("compute_hours_estimate"))
        disk_by_status[r.get("status")] += num(r.get("disk_gb_estimate"))

    covered = [row for row in coverage_rows if row["covered"] == "Y"]
    uncovered = [row for row in coverage_rows if row["covered"] == "N"]
    with_explicit_reason = [row for row in coverage_rows if row["uncovered_reason"]]

    blocker_class_counts = Counter(row["blocker_class"] for row in coverage_rows)
    blocker_class_covered = Counter(row["blocker_class"] for row in coverage_rows if row["covered"] == "Y")
    blocker_class_uncovered = Counter(row["blocker_class"] for row in coverage_rows if row["covered"] == "N")

    return {
        "experiments_total": len(experiments),
        "experiments_by_status": dict(sorted(status_counts.items())),
        "experiments_by_domain": dict(sorted(domain_counts.items())),
        "compute_hours_total": total_compute_hours,
        "compute_hours_by_status": {k: v for k, v in sorted(compute_by_status.items())},
        "disk_gb_total": total_disk_gb,
        "disk_gb_by_status": {k: v for k, v in sorted(disk_by_status.items())},
        "decisions_total": len(ledger_decisions),
        "decisions_covered": len(covered),
        "decisions_uncovered": len(uncovered),
        "decisions_with_explicit_uncovered_reason": len(with_explicit_reason),
        "decisions_by_blocker_class": dict(sorted(blocker_class_counts.items())),
        "decisions_covered_by_blocker_class": dict(sorted(blocker_class_covered.items())),
        "decisions_uncovered_by_blocker_class": dict(sorted(blocker_class_uncovered.items())),
        "validation_errors_total": len(report.errors),
        "validation_errors_by_kind": report.by_kind(),
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="validate only, do not write CSVs")
    ap.add_argument("--json", action="store_true", help="print the summary report as JSON")
    args = ap.parse_args()

    report = Report()
    experiments, uncovered = load_jsonl_records(report)
    ledger_decisions, ledger_order = load_ledger(report)
    rows_extra = validate_experiment_directories(experiments, report)
    dec_to_exps, dec_to_uncovered = cross_reference_decisions(experiments, uncovered, ledger_decisions, report)
    coverage_rows = build_coverage_rows(ledger_decisions, ledger_order, dec_to_exps, dec_to_uncovered, report)

    if not args.check:
        write_index_csv(experiments, rows_extra)
        write_coverage_csv(coverage_rows)

    summary = summarize(experiments, ledger_decisions, coverage_rows, report)

    if args.json:
        print(json.dumps({"summary": summary, "errors": report.errors}, indent=2, sort_keys=True))
    else:
        print("=== build_index summary ===")
        print(f"experiments: {summary['experiments_total']}")
        print(f"  by status: {summary['experiments_by_status']}")
        print(f"  by domain: {summary['experiments_by_domain']}")
        print(f"compute hours total: {summary['compute_hours_total']:.1f}  (by status: {summary['compute_hours_by_status']})")
        print(f"disk GB total: {summary['disk_gb_total']:.1f}  (by status: {summary['disk_gb_by_status']})")
        print(f"decisions: {summary['decisions_total']}")
        print(f"  covered: {summary['decisions_covered']}")
        print(f"  uncovered (no experiment): {summary['decisions_uncovered']}")
        print(f"  with explicit uncovered_reason text: {summary['decisions_with_explicit_uncovered_reason']}")
        print(f"  by blocker_class: {summary['decisions_by_blocker_class']}")
        print(f"    covered by blocker_class: {summary['decisions_covered_by_blocker_class']}")
        print(f"    uncovered by blocker_class: {summary['decisions_uncovered_by_blocker_class']}")
        print(f"validation errors: {summary['validation_errors_total']}  {summary['validation_errors_by_kind']}")
        if report.errors:
            print("--- errors ---")
            for e in report.errors:
                print(json.dumps(e, sort_keys=True))
        if not args.check:
            print(f"wrote {rel(INDEX_CSV)} ({len(experiments)} rows)")
            print(f"wrote {rel(COVERAGE_CSV)} ({len(coverage_rows)} rows)")

    return 1 if report.errors else 0


if __name__ == "__main__":
    sys.exit(main())
