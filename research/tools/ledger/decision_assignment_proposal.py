#!/usr/bin/env python3
"""Build a source-linked K01 assignment proposal; never mutates the live ledger."""

from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.util
import json
import re
from collections import defaultdict
from pathlib import Path

from constraint_crosswalk import BARE_INVARIANT, I_TO_HC, invariant_hcs


ASSIGNER = "K01 assignment session, 2026-10-01; no candidate-design role"
OD_PATTERNS: dict[str, str] = {
    "OD-01": r"semantic|single authority|coheren|canonical (?:model|interpretation)|one interpretation|duplicate authorit",
    "OD-02": r"lossless|byte.exact|exact round.trip|round.trip mismatch|identical original bytes|reproduc.*original bytes",
    "OD-03": r"fidelity|capture|restore|preserv.*metadata|xattr|(?:file|filesystem|metadata|restor|captur).{0,30}\bACL\b|\bACL\b.{0,30}(?:fidelity|restor|captur|metadata)|\b(?:file|filesystem|metadata)\b.{0,20}timestamp|timestamp.{0,20}\b(?:file|metadata|restor|captur)\b|ownership|sparse|hardlink",
    "OD-04": r"unique interpretation|reason.code|diagnostic|error class|outcome class|truncation|deterministic interpret",
    "OD-05": r"compress(?:ion|ed|or)? (?:ratio|efficiency|size|cost)|dedup (?:ratio|savings)|stored bytes|bytes saved|archive savings|storage efficiency|artifact size|fixed overhead|format overhead",
    "OD-06": r"pack (?:time|speed|throughput)\b|creation performance|encode (?:time|speed|throughput)\b|planner (?:cost|time|speed)\b|planning (?:CPU|time)\b",
    "OD-07": r"decode (?:time|speed|throughput|cost)|unpack (?:time|speed|throughput)|verify (?:time|speed|cost)|extract.*(?:latency|speed|time)",
    "OD-08": r"memory|scratch|buffer|spill|resident|\bRAM\b|working set",
    "OD-09": r"\bstreaming\b|\bpipe\b|\bsequential\b|non.seekable|single.pass|stream layout|stream reader|stream writer",
    "OD-10": r"random.access|random.read|seek|access granularity|first.entry|lookback|range.address",
    "OD-11": r"verified[- ]partial|partial[- ]retriev|partial[- ]read|partial verif|range proof|verified range|Merkle slice|proof granularity",
    "OD-12": r"\bremote\b|\bHTTP\b|\bETag\b|range request|network|remote origin|\bCDN\b|lazy.pull",
    "OD-13": r"parallel|concurren|multi.thread|thread count|speedup|multi.core",
    "OD-14": r"blast radius|corruption locality|salvage|damage local|unreadable.*corrupt",
    "OD-15": r"cryptograph|authenticat|signature|\bsigning\b|signing[- ]key|recipient|key management|keystore|\bHSM\b|PKCS#11|\bTSA\b|RFC 3161|timestamp (?:token|validity|trust|binding)|revocation|crypto suite|AEAD|KEM|tamper",
    "OD-16": r"privacy|leak|bucketed padding|encrypted padding|padding overhead|presence.confirm|plaintext structure|metadata exposure",
    "OD-17": r"resource|budget|decode limit|KDF cost|refusal cost|bomb|allocation|hostile.*CPU",
    "OD-18": r"cross.platform|platform fidelity|hostile name|filesystem metadata fidelity",
    "OD-19": r"legacy|\bZIP\b|\btar\b|\b7z\b|\bimport(?:er|ing|ed|s)?\b|\bexport(?:er|ing|ed|s)?\b|compatib|foreign format",
    "OD-20": r"migrat|repack|publish|publication|atomic commit|no.replace|conversion receipt|convert|sidecar",
    "OD-21": r"independent reader|independent implement|clean.room|normative spec|conformance corpus|specification completeness",
    "OD-22": r"dependenc|maintain|licen[cs]e|codec longevity|RustCrypto|crate|MSRV|supply.chain",
    "OD-23": r"spec(?:ification)? (?:complexity|size|surface)|normative words|wire items|format complexity",
    "OD-24": r"attack surface|fuzz|unsafe code|parser complexity|vulnerab|untrusted.input|confinement|path safety|safe extraction",
    "OD-25": r"CLI|usab|flags? (?:per|for|surface)|unsafe default|error action|discoverab|user workflow|comprehension|users? understand|task success",
    "OD-26": r"adopt|ecosystem|media type|MIME|extension|registration|zero.install|integration|packag(?:e|ing)",
}

# Required-evidence text often names comparison tools or implementation
# context.  Only direct measured outcomes add an OD from that field; broad
# keyword screens there would turn ZIP baselines into import objectives and
# lookback implementation details into random-access objectives.
EVIDENCE_OD_PATTERNS = {
    "OD-05": r"net bytes saved|archive savings|bytes lost|ratio (?:versus|vs)|break.even file size",
    "OD-06": r"planning CPU|planning time|encode wall|pack wall",
    "OD-08": r"peak memory|index memory|memory and CPU|working.set",
}


def od_matches(od: str, pattern: str, value: str) -> bool:
    if re.search(pattern, value, re.I):
        return True
    if od != "OD-18":
        return False
    # Platform names are evaluation conditions until the question asks about
    # metadata/name restoration or fidelity.  Lower-case codec windows never
    # count as an OS mention.
    host = re.search(r"\bWindows\b|\b(?:macOS|Linux|NTFS|APFS|ext4|XFS|btrfs|ReFS)\b", value)
    fidelity = re.search(r"fidel|restor|metadata|xattr|\bACL\b|security descriptor|resource fork|native name|stream syntax|logical.?path", value, re.I)
    return host is not None and fidelity is not None

CLUSTER_DEFAULT = {
    "access": "OD-10", "compression": "OD-05", "container": "OD-01",
    "crypto": "OD-15", "ecosystem": "OD-26", "integrity": "OD-04",
    "legacy": "OD-19", "model": "OD-01", "platform": "OD-18",
}

CANONICAL_PRIMARY = {
    "OD-03": "restore_fidelity_fraction",
    "OD-04": "reason_code_specificity_fraction",
    "OD-05": "artifact_bytes",
    "OD-06": "encode_wall_s",
    "OD-07": "decode_wall_s",
    "OD-08": "alloc_peak_bytes",
    "OD-09": "stream_unpack_wall_s",
    "OD-10": "random_entry_latency_s",
    "OD-11": "verified_range_fetch_bytes",
    "OD-12": "remote_task_latency_s",
    "OD-13": "parallel_speedup",
    "OD-14": "blast_logical_bytes_p95",
    "OD-16": "presence_advantage",
    "OD-17": "declared_tightness_ratio",
    "OD-18": "cross_platform_restore_fidelity_fraction",
    "OD-19": "import_acceptance_fraction",
    "OD-25": "error_actionability_fraction",
}

# Source-row-specific corrections are tied to exact ledger bytes.  If a row is
# edited, stop generation and review its OD/type route again rather than
# silently carrying an old exception forward.
LOCKED_ROW_SHA = {
    "DEC-ACC-063": "d84f98097040d0540e19423922cf37e3e5d1d605100a605bc05f76b68b9a283d",
    "DEC-CMP-004": "35f109ec303a2e0ac8db3d3e91f4ea0f914b806fcb59a6901b290ce218d6061a",
    "DEC-CMP-027": "c6bbed6df8fab9894fdb4f8128a7f750b7e7145dfdf0a53cdf26cefb5d0f9056",
    "DEC-CMP-032": "53bfd7430b6578910b55b7fb4c6dd4e61e8631e74b120253eee1b247e38cd263",
    "DEC-CRY-081": "5e4440a6b251576df416f8f7c8248cb6e9ec9377396c2a0560fb593d33ab75cd",
    "DEC-ECO-041": "712e19608faa70df0bafcf46b339aa55d5238af300560f32296aabb43eb59a07",
    "DEC-ECO-053": "4f545e8e354d161b74e0d8b0e44ad7c3a4e9297b64a1430a8258fa908f93b76b",
    "DEC-ECO-055": "691c01ec74e9e16565413a60495a0038995d74b7a4ce5df3e7664fdebd770166",
    "DEC-ECO-060": "2fcdcef7fe21bd9cb328abab8322b9f815ab741859b991097e9f814216a092fd",
    "DEC-ECO-068": "60082c413afa068f1f4d09d0f4b56791ff0e523b3d89d39add608fb043d77251",
    "DEC-ECO-069": "d8b8b8a88584e76890a681b2e99e7dadeeee4e3731963260e77ce8e571669ecc",
    "DEC-ECO-070": "60107c66c954aef758a48c9a5eab45666dbec6fae768a04cf21e8d7fb95aee91",
    "DEC-ECO-077": "781e512ca48653dee647b3b1e3ae416aa22ce6b2ab7995c004bf43a17b30ee7d",
    "DEC-ECO-080": "8e5ab8e404d2a923a617907724ae79e222f9dc130d4104bd96bfa616ba56de4c",
    "DEC-ECO-081": "a882a1d44d4f40ebb5d7ed8beeb2af0263e5980a8860e9c97c3e53f436da81e0",
    "DEC-ECO-082": "37ca82626655a8b44ba8212e3ca32bb208c01c7deeb69ab2825b944312438390",
    "DEC-ECO-083": "267ec9a39a7473330026896c143460ee1c6e174e35462905be3ac646dc9a3de6",
    "DEC-LEG-027": "e10a7b06cab2510753a2d8a28a79adc69c687079624fba064aa5fcb9c7792a07",
    "DEC-LEG-047": "1d53cf84bba60aadc2597e11b4fac8df028f0140e2ec7788a81fab89fb584624",
    "DEC-LEG-083": "4ac5b7ccad527a22c86842a5d7d7b0cd9bf8af6920877c6bb36eae528cfd34e7",
    "DEC-LEG-108": "6af81e86be6cec8cb91a2049b08f4a5c13de06c289a80be2fe2a39ba001c2e18",
    "DEC-MOD-006": "d1406893577019558398023418a7a348c676e02c0b57b0eb480525e9acab5cd7",
    "DEC-PLT-039": "c828fbfecc48e6c866a5b40b23d454e36e25780fb99d1ebb495ed23da7e5e03b",
    "DEC-PLT-054": "6eb316ef000a2a5eb9c6f18a91ce72328f04a5540c672503cc598111b5912a2e",
}

OD_OVERRIDES = {
    "DEC-CMP-004": {"OD-05": "Actual archive-byte delta chooses the planner rule.",
                    "OD-06": "The alternative full-cost evaluation has explicit pack CPU cost."},
    "DEC-CMP-027": {"OD-05": "Ratio gain is the benefit of lookback.",
                    "OD-08": "Prefix-window sweep states decoder-memory cost.",
                    "OD-10": "First-byte latency and read amplification are direct access outcomes.",
                    "OD-12": "Remote RTT and bandwidth are separate access conditions.",
                    "OD-13": "Parallel decode throughput loss is explicit.",
                    "OD-14": "Fault injection measures corruption propagation/blast radius."},
    "DEC-CMP-032": {"OD-05": "Regret against exhaustive search measures size quality.",
                    "OD-06": "Pack time per strategy is explicit required evidence.",
                    "OD-08": "Peak memory per strategy is explicit required evidence."},
    "DEC-CRY-081": {"OD-08": "Per-layout memory is explicit evidence.",
                    "OD-10": "Random-access fetched bytes and latency are explicit evidence.",
                    "OD-14": "Per-layout blast radius is explicit evidence.",
                    "OD-15": "The authenticated encrypted segment layout is the subject of the decision.",
                    "OD-16": "Entry/count inference against the observer is the privacy outcome."},
    "DEC-ECO-041": {"OD-07": "Decoder throughput compares native and pure-Rust paths.",
                    "OD-08": "Decoder memory delta is explicit.",
                    "OD-22": "Native dependency/build burden is explicit.",
                    "OD-24": "Untrusted-input native-code attack surface is explicit."},
    "DEC-ECO-053": {"OD-25": "Target-user message comprehension/choice is required.",
                    "OD-26": "The publishable headline is an adoption/positioning claim."},
    "DEC-LEG-027": {"OD-05": "Per-level output size is required.",
                    "OD-06": "Per-level compression time is required.",
                    "OD-07": "Per-level decompression time is required.",
                    "OD-08": "Per-level decoder memory is required.",
                    "OD-19": "The selected codec parameters govern legacy export acceptance."},
    "DEC-LEG-047": {"OD-17": "Dictionary-size bound/refusal is required.",
                    "OD-19": "Measured RAR5 migration demand is required.",
                    "OD-22": "UnRAR licence and permanent dependency cost are explicit.",
                    "OD-24": "Parser CVEs, fuzzing and unknown-record safety are explicit."},
    "DEC-LEG-083": {"OD-08": "Adversarial peak-memory measurement is explicit.",
                    "OD-17": "Bomb/extent refusal is the resource-safety outcome.",
                    "OD-19": "False-refusal rate on real archives is import applicability."},
    "DEC-LEG-108": {"OD-08": "Peak memory and temporary disk are explicit.",
                    "OD-09": "Non-seekable streaming export is a binary capability.",
                    "OD-19": "Reader acceptance governs ZIP export."},
    "DEC-MOD-006": {"OD-05": "Manifest bytes per entry and total are explicit.",
                    "OD-06": "Encoding throughput is explicit.",
                    "OD-07": "Decoding throughput is explicit.",
                    "OD-08": "Peak memory is explicit.",
                    "OD-09": "Streaming emission feasibility is explicit.",
                    "OD-21": "Independent reader effort and ambiguity log are explicit."},
}

TYPE_OVERRIDES = {
    "DEC-CMP-004": ("EMPIRICAL", ["EMPIRICAL"], "Archive-size delta and pack CPU compare measured planner alternatives."),
    "DEC-ECO-053": ("HUMAN-FACING", ["HUMAN-FACING", "EXTERNAL", "EMPIRICAL"], "Target-user message testing needs external human review; product superiority substantiation takes an empirical route."),
    "DEC-LEG-027": ("EMPIRICAL", ["EMPIRICAL"], "Codec level selection compares measured size, encode/decode time and memory."),
    "DEC-LEG-047": ("EXTERNAL", ["EXTERNAL", "EMPIRICAL"], "UnRAR licence requires qualified legal review; demand and safety outcomes are measured."),
    "DEC-PLT-054": ("EMPIRICAL", ["EMPIRICAL", "FORMAL"], "Real prevalence and kernel/corpus evidence are measured; ACL authority also needs mechanical proof."),
}

GOVERNED_NO_PRIMARY = {
    "DEC-ACC-063": "Remote emulation validity is established by the fixed §4.15 network profiles, calibration to a real endpoint and rank-stability checks, not selection on remote_task_latency_s.",
    "DEC-ECO-068": "Capability disposition is governed by conformance, fuzzing, independent-reader and release-admission criteria in §8 and §14, not a graded product utility metric.",
    "DEC-ECO-070": "Fairness/completeness of the public claim is an evidence audit; underlying per-profile metrics need their own pre-registration, not selection weight here.",
    "DEC-ECO-077": "Corpus split, independence and workload-mix validity are governed by §5 and Appendix B; this is no dependency-performance or graded utility claim.",
    "DEC-ECO-080": "Every experiment's outcome and negative-result disclosure are governed by §10 and §14 item 8, not a graded selection metric.",
    "DEC-ECO-081": "Exact-source reproduction, hash pins and a clean rerun are governed by §12, not a graded selection metric.",
    "DEC-ECO-082": "Protocol and calibration criteria are governed by §4.7 and §14 items 1–4, not a graded selection metric.",
}

# These four method-admission decisions have no product objective to grade.
# The public method permits a reviewed empty OD assignment for these exact
# governance questions; a source-row digest above prevents silent carryover
# if any question changes.  Their evidence routes and every other gate remain.
GOVERNED_EMPTY_OD = {
    "DEC-ECO-077": "This question audits corpus split, independence and workload-mix validity under §5 and Appendix B; no product OD applies. It admits no graded utility or R4 selection, while HC/F screening, cost and mixed evidence, status and applicable execution gates remain required.",
    "DEC-ECO-080": "This question audits the complete experiment-outcome and negative-result census under §10 and §14 item 8; no product OD applies. It admits no graded utility or R4 selection, while HC/F screening, cost and mixed evidence, status and applicable execution gates remain required.",
    "DEC-ECO-081": "This question audits exact-source reproduction, hash pins and a clean rerun under §12; no product OD applies. It admits no graded utility or R4 selection, while HC/F screening, cost and mixed evidence, status and applicable execution gates remain required.",
    "DEC-ECO-082": "This question audits protocol and calibration criteria under §4.7 and §14 items 1–4; no product OD applies. It admits no graded utility or R4 selection, while HC/F screening, cost and mixed evidence, status and applicable execution gates remain required.",
}


def sha_bytes(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def objective_module(path: Path):
    spec = importlib.util.spec_from_file_location("objective_screen_for_k01", path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def requirement_od_screen(path: Path, screen) -> dict[str, set[str]]:
    mapping = {}
    with path.open(encoding="utf-8", newline="") as handle:
        for row in csv.DictReader(handle):
            if row["kind"] not in ("INVARIANT", "REQUIREMENT"):
                continue
            rid, description = row["req_id"], row["requirement_or_question"]
            tags = set()
            for invariant in screen.explicit_invariants(description):
                tags.update(screen.I2HC[invariant])
            tags.update(h for h, pat in screen.HC_KW.items() if re.search(pat, description, re.I))
            tags.update(screen.MANUAL.get(rid, []))
            tags = {tag for tag in tags if (rid, tag) not in screen.EXCLUDE}
            if not any(tag.startswith("HC-") for tag in tags):
                tags.update(od for od, pat in screen.OD_KW.items() if re.search(pat, description, re.I))
            mapping[rid] = {tag for tag in tags if tag.startswith("OD-")}
    return mapping


def metric_for(od: str, row: dict) -> tuple[str | None, str]:
    """Transcribe an existing banded metric, or explain why none is proposed."""
    text = " ".join((row["question"], row["archetypal_objective"])).lower()
    if od == "OD-05":
        if "fixed overhead" in text or "tiny archive" in text:
            return "fixed_overhead_bytes", "T-21/M05.3 small-tier component named by the question."
        if "metadata" in text and "per entry" in text:
            return "metadata_bytes_per_entry", "T-02b/M05.2 component named by the question."
    if od == "OD-07" and "verify" in text and "decode" not in text:
        return "verify_wall_s", "T-04/M07.2 verification component named by the question."
    if od == "OD-09" and re.search(r"\b(?:export|publish)\b", row["question"].lower()) and not re.search(r"unpack|reader|extract", row["question"].lower()):
        return None, "Question concerns streaming export/publish; Appendix A.2 has no OD-09 stream-export primary metric."
    if od == "OD-09" and not re.search(r"(?:speed|time|performance|throughput|overhead)", text):
        return None, "Streaming capability is binary (HC-06/F-09); no graded primary declared."
    if od == "OD-10" and "granularity" in text and "latency" not in text:
        return "access_granularity_bytes", "T-09 bounds the declared granularity in this question."
    if od == "OD-11" and "overhead" in text and "byte" not in text:
        return "verify_overhead_pp", "T-13/M11.1 verifies overhead in percentage points."
    if od == "OD-12" and "request" in text and "latency" not in text:
        return "http_requests", "T-12/M12.1 request count is the named remote cost."
    if od == "OD-16" and "padding overhead" in text and "leak" not in text:
        return "padding_overhead_pp", "T-16/M16.3 measures the named padding cost."
    if od == "OD-17" and "refusal" in text:
        return "benign_false_refusal_fraction", "T-20/M17.3 measures false refusal on a fixed case set."
    if od == "OD-19":
        question = row["question"].lower()
        if "export" in question and ("import" not in question or "publish" in question):
            return "export_acceptance_fraction", "T-20/M19.4 covers the questioned export/publish operation; ancillary re-import does not change its primary route."
        if "export" in question and "import" in question:
            return None, "Question combines import and export; independent review must select the operation or split the OD-19 claim."
    metric = CANONICAL_PRIMARY.get(od)
    if metric:
        return metric, "Existing banded canonical metric in decision-method.md Appendix A.2; experiment-specific choice remains unapproved."
    return None, "This OD has a binary, cost-input, descriptive, or heuristic route; no banded primary is declared by this proposal."


def decision_type(row: dict) -> tuple[str, list[str], str]:
    if row["decision_id"] in TYPE_OVERRIDES:
        return TYPE_OVERRIDES[row["decision_id"]]
    blocker = row["blocker_class"]
    text = " ".join((row["question"], row["archetypal_objective"],
                     " ".join(row["required_evidence"]))).lower()
    measured = bool(re.search(r"bench|measur|timing|latency|throughput|performance|(?:size|memory|overhead|leakage) (?:curve|comparison|experiment)|corpus.*compar|experiment|bounded.memory|memory.scal|scratch (?:use|growth)|compression ratio|access cost|decode cost|pack time|unpack time|bytes saved|break.even|recall and precision|peak memory|planning CPU", text))
    if re.search(r"first.time users? understand|comprehension study|users? predict.*presentation", text):
        return "HUMAN-FACING", ["HUMAN-FACING", "EXTERNAL"], "Explicit comprehension claim requires external human review under §1.2, in addition to mechanical human-facing checks."
    if row["decision_id"] == "DEC-ECO-055":
        return "HUMAN-FACING", ["HUMAN-FACING", "EXTERNAL", "EMPIRICAL"], "User profile-choice accuracy requires external human review; stated workload benchmarks also require measured evidence."
    if row["decision_id"] == "DEC-ECO-083":
        return "HUMAN-FACING", ["HUMAN-FACING", "EXTERNAL"], "Usability-evidence method cannot certify first-use comprehension without an external human route."
    if row["decision_id"] == "DEC-ECO-060":
        return "EXTERNAL", ["EXTERNAL"], "Identified adopter evidence requires external confirmation before language-binding selection."
    if row["decision_id"] == "DEC-ECO-069":
        return "EXTERNAL", ["EXTERNAL", "FORMAL", "EMPIRICAL"], "Published novelty/comparison claim requires independent external review, formal falsification and measured incumbent comparison."
    if row["decision_id"] == "DEC-PLT-039":
        return "EXTERNAL", ["EXTERNAL", "FORMAL"], "Comparative public extraction-safety claim requires an independent external wording and evidence review."
    if re.search(r"legal freedom.to.operate|patent search by qualified counsel", text):
        return "EXTERNAL", ["EXTERNAL"], "Legal freedom-to-operate claim requires a qualified independent expert dossier under §1.2/§8.3."
    if blocker == "HUMAN_PARTICIPANTS":
        routes = ["HUMAN-FACING", "EXTERNAL"]
        if measured:
            routes.append("EMPIRICAL")
        return "HUMAN-FACING", routes, "Human-participant blocker forces the human claim through the HUMAN-FACING/external route (§1.2)."
    if blocker == "EXTERNAL_REVIEW":
        routes = ["EXTERNAL"] + (["EMPIRICAL"] if measured else [])
        return "EXTERNAL", routes, "Independent expert dossier is required; any measured part also takes EMPIRICAL route (§1.2)."
    if measured:
        return "EMPIRICAL", ["EMPIRICAL"], "Question or required evidence invokes measured effects; §1.2 forces EMPIRICAL route."
    return "FORMAL", ["FORMAL"], "No measured or human preference claim in this row; proposed invariant/spec/vector route still requires independent counterexample review."


def assign_ods(row: dict, requirement_tags: dict[str, set[str]]) -> tuple[list[str], dict[str, list[str]], dict[str, str]]:
    if row["decision_id"] in OD_OVERRIDES:
        reasons = OD_OVERRIDES[row["decision_id"]]
        selected = sorted(reasons)
        linked = {od: sorted(rid for rid in row["requirement_ids"]
                             if od in requirement_tags.get(rid, set())) for od in selected}
        return selected, linked, {od: "exact source-row review: " + why for od, why in reasons.items()}
    scores = defaultdict(int)
    reasons = defaultdict(list)
    cluster = row["cluster"]
    default = CLUSTER_DEFAULT[cluster]
    question = row["question"]
    objective = row["archetypal_objective"]
    for od, pattern in OD_PATTERNS.items():
        if od_matches(od, pattern, question):
            scores[od] += 4
            reasons[od].append("question")
        if od_matches(od, pattern, objective):
            scores[od] += 1
            reasons[od].append("archetypal_objective")
        evidence_pattern = EVIDENCE_OD_PATTERNS.get(od)
        if evidence_pattern and any(re.search(evidence_pattern, evidence, re.I)
                                    for evidence in row["required_evidence"]):
            scores[od] += 4
            reasons[od].append("required_evidence")
    linked = defaultdict(list)
    for rid in row["requirement_ids"]:
        for od in requirement_tags.get(rid, set()):
            scores[od] += 1
            linked[od].append(rid)
            reasons[od].append(f"C.3:{rid}")
    selected = [od for od, score in sorted(scores.items(), key=lambda pair: (-pair[1], pair[0]))
                if score >= 4]
    if not selected:
        selected = [default]
        reasons[default].append(f"cluster-fallback:{cluster}; no direct question or linked requirement screen")
    selected = sorted(set(selected))
    return selected, {od: sorted(linked[od]) for od in selected}, {od: ", ".join(reasons[od]) for od in selected}


def read_ledger(path: Path) -> list[tuple[dict, str]]:
    result = []
    with path.open("rb") as handle:
        for raw in handle:
            if raw.strip():
                content = raw.rstrip(b"\r\n")
                result.append((json.loads(content), sha_bytes(content)))
    return result


def build(repo: Path) -> list[dict]:
    ledger_path = repo / "research/decision-ledger.jsonl"
    crosswalk_path = repo / "research/methods/constraint-crosswalk.csv"
    screen_path = repo / "research/tools/ledger/objective_screen.py"
    req_path = repo / "research/requirement-ledger.csv"
    with crosswalk_path.open(encoding="utf-8", newline="") as handle:
        crosswalk = {r["constraint_string"]: r for r in csv.DictReader(handle)}
    screen = objective_module(screen_path)
    requirement_tags = requirement_od_screen(req_path, screen)
    source_hashes = {
        "source_ledger_sha256": sha_bytes(ledger_path.read_bytes()),
        "source_crosswalk_sha256": sha_bytes(crosswalk_path.read_bytes()),
        "source_requirement_ledger_sha256": sha_bytes(req_path.read_bytes()),
        "source_objective_screen_sha256": sha_bytes(screen_path.read_bytes()),
        "source_decision_method_sha256": sha_bytes((repo / "research/decision-method.md").read_bytes()),
        "source_archetypal_objective_sha256": sha_bytes((repo / "research/archetypal-objective.md").read_bytes()),
    }
    canonical = set()
    method = (repo / "research/decision-method.md").read_text(encoding="utf-8")
    for match in re.finditer(r"^\| `([a-z][a-z0-9_]+)` \| [^|]+ \| (banded|diagnostic) \|", method, re.M):
        canonical.add(match.group(1))
    proposals = []
    for row, digest in read_ledger(ledger_path):
        locked = LOCKED_ROW_SHA.get(row["decision_id"])
        if locked is not None and digest != locked:
            raise ValueError(f"source row changed; re-review assignment override for {row['decision_id']}")
        hc = set()
        freezes = set()
        constraint_evidence = []
        for value in row["hard_constraints"]:
            if BARE_INVARIANT.fullmatch(value):
                ids = I_TO_HC[int(value[1:])]
                hc.update(ids)
                constraint_evidence.append({"constraint": value, "source": "objective Appendix A", "hc_ids": list(ids)})
            else:
                if value not in crosswalk:
                    raise ValueError(f"unmapped constraint in {row['decision_id']}: {value}")
                routed = crosswalk[value]
                targets = routed["target"].split(";") if routed["target"] else []
                if routed["class"] == "HC-xx":
                    hc.update(targets)
                elif routed["class"] == "F-xx":
                    freezes.update(targets)
                constraint_evidence.append({"constraint_sha256": routed["string_sha256"],
                                            "class": routed["class"], "target": routed["target"]})
        if row["decision_id"] in GOVERNED_EMPTY_OD:
            ods, od_reqs, od_rationale = [], {}, {}
        else:
            ods, od_reqs, od_rationale = assign_ods(row, requirement_tags)
        dtype, routes, type_reason = decision_type(row)
        primary = {}
        no_primary = {}
        primary_rationale = {}
        for od in ods:
            metric, why = metric_for(od, row)
            if "EMPIRICAL" not in routes:
                no_primary[od] = "No graded EMPIRICAL route is assigned; required formal or human/external evidence remains. " + why
            elif metric:
                if metric not in canonical:
                    raise ValueError(f"noncanonical metric {metric} for {row['decision_id']}")
                primary[od] = metric
                primary_rationale[od] = why
            else:
                no_primary[od] = why
        if row["decision_id"] in GOVERNED_NO_PRIMARY:
            governed = GOVERNED_NO_PRIMARY[row["decision_id"]]
            primary.clear()
            primary_rationale.clear()
            no_primary = {od: (governed + " No graded selection or utility result is admitted; "
                               "§0.2 requires a canonical band and role committed before the first decision-relevant result.")
                          for od in ods}
        proposals.append({
            "decision_id": row["decision_id"],
            "source_row_sha256": digest,
            **source_hashes,
            "hc_ids": sorted(hc),
            "freeze_ids_from_crosswalk": sorted(freezes),
            "constraint_evidence": constraint_evidence,
            "od_ids": ods,
            "od_requirement_screen_refs": od_reqs,
            "od_assignment_rationale": od_rationale,
            "od_applicability_rationale": GOVERNED_EMPTY_OD.get(row["decision_id"], ""),
            "decision_type": dtype,
            "evidence_route_types": routes,
            "decision_type_rationale": type_reason,
            "primary_metrics_by_od": primary,
            "primary_metric_rationale_by_od": primary_rationale,
            "no_primary_metric_reason_by_od": no_primary,
            "metric_registration_blockers_by_od": {},
            "assigning_session": ASSIGNER,
            "assignment_review_ref": "",
            "assignment_status": "PROPOSED_UNAPPROVED",
            "analysis_role_limit": "L7 identity exposure recorded; metric choices require independent review and unexposed analysis design where applicable families overlap exposure.",
        })
    return proposals


def render(rows: list[dict]) -> str:
    return "".join(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n" for row in rows)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--repo", type=Path, default=Path("."))
    ap.add_argument("--output", type=Path, default=Path("research/methods/decision-assignments-proposal.jsonl"))
    mode = ap.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = ap.parse_args()
    output = args.output if args.output.is_absolute() else args.repo / args.output
    rows = build(args.repo)
    if len(rows) != 596 or len({r["decision_id"] for r in rows}) != len(rows):
        raise ValueError("expected 596 unique decisions")
    expected = render(rows)
    if args.write:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(expected, encoding="utf-8", newline="")
        print(f"wrote {len(rows)} proposed decisions to {output}")
        return 0
    actual = output.read_text(encoding="utf-8") if output.exists() else ""
    if actual != expected:
        print(f"decision assignment proposal stale: {output}")
        return 1
    print(f"decision assignment proposal exact: {len(rows)} rows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
