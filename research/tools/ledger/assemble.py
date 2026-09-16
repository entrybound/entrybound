#!/usr/bin/env python3
"""Deterministic Phase A ledger assembler for the Entrybound research program.

Reads merged requirement and decision rows (research/audit/merged), addenda
(research/audit/addenda) and extraction records (research/audit/extract), and
writes:

  research/requirement-ledger.csv           RFC 4180, UTF-8, sorted by req_id
  research/decision-ledger.jsonl            sorted by decision_id
  research/supersession-ledger.md           per-cluster supersession table
  research/source-inventory.md              every source with SHA-256 and use
  research/audit/id-map.json                stable, never-reused IDs + aliases
  research/audit/assembly-log.md            every merge, fix and resolution
  research/audit/coverage-report.md         mechanical extraction-key coverage
  research/tools/ledger/*.schema.json       JSON Schemas for both ledgers

Standard library only. Rerunnable: identical inputs give byte-identical
outputs. Exit status 0 when validation passes, 1 when validation errors remain
(outputs are still written so the errors can be inspected). See README.md.
"""

from __future__ import annotations

import argparse
import collections
import csv
import datetime as dt
import hashlib
import io
import itertools
import json
import math
import os
import pathlib
import re
import subprocess
import sys

TOOL_DIR = pathlib.Path(__file__).resolve().parent
RESEARCH = TOOL_DIR.parents[1]
REPO = RESEARCH.parent
AUDIT = RESEARCH / "audit"
EXTRACT = AUDIT / "extract"
MERGED = AUDIT / "merged"
ADDENDA = AUDIT / "addenda"
ID_MAP = AUDIT / "id-map.json"
ASSEMBLY_LOG = AUDIT / "assembly-log.md"
COVERAGE_REPORT = AUDIT / "coverage-report.md"
REQ_CSV = RESEARCH / "requirement-ledger.csv"
DEC_JSONL = RESEARCH / "decision-ledger.jsonl"
SUPERSESSION = RESEARCH / "supersession-ledger.md"
INVENTORY = RESEARCH / "source-inventory.md"
NEAR_DUPLICATES = TOOL_DIR / "near-duplicates.json"
REQ_SCHEMA = TOOL_DIR / "requirement-ledger.schema.json"
DEC_SCHEMA = TOOL_DIR / "decision-ledger.schema.json"

BASELINE_COMMIT = "9e44608"
DEFAULT_EXTERNAL_ROOT = "D:/Projects/entrybound" if os.name == "nt" else "/mnt/d/Projects/entrybound"
DEFAULT_SOURCES_ROOT = "D:/eb-research/sources" if os.name == "nt" else "/mnt/d/eb-research/sources"

CLUSTER_CODES = {
    "model": "MOD", "container": "CON", "compression": "CMP", "access": "ACC", "platform": "PLT",
    "crypto": "CRY", "legacy": "LEG", "integrity": "INT", "ecosystem": "ECO",
}
KINDS = ["INVARIANT", "REQUIREMENT", "FROZEN_DECISION", "DEFERRED_QUESTION", "OPEN_QUESTION",
         "CLAIM_NEEDING_EVIDENCE", "NON_GOAL", "CAPABILITY"]
IMPLEMENTATION_STATES = ["IMPLEMENTED", "PARTIAL", "MISSING", "BLOCKED_PLATFORM", "DEFERRED_APPROVED", "REJECTED"]
# Strictest (least support claimed) first; merges keep the stricter state.
EVIDENCE_STATES = ["CONTRADICTED", "EXTERNAL_REVIEW_REQUIRED", "UNTESTED", "INSUFFICIENT",
                   "SUPPORTED_WITH_LIMITATIONS", "EMPIRICALLY_SUPPORTED", "PROVEN"]
# Most severe first; merges keep the more severe relevance.
RELEASE_RELEVANCE = ["V1_BLOCKING", "V1_IMPORTANT", "POST_V1", "INFORMATIONAL"]
# Strictest first; merges keep the stricter status.
STATUSES = ["EXTERNAL_REVIEW_REQUIRED", "INSUFFICIENT_EVIDENCE", "DECIDED"]
BLOCKER_CLASSES = ["NONE", "EVIDENCE", "PLATFORM", "EXTERNAL_REVIEW", "HUMAN_PARTICIPANTS", "RESOURCES"]
PROGRESS_ORDER = {"MISSING": 0, "PARTIAL": 1, "IMPLEMENTED": 2}

SECTIONS = (["2", "3", "5", "6", "7"] + [f"8.{i}" for i in range(1, 9)] + ["9", "10"]
            + [str(i) for i in range(11, 22)] + [f"22.{i}" for i in range(1, 6)]
            + [str(i) for i in range(23, 36)])
REQUIRED_DECISION_SECTIONS = ([f"8.{i}" for i in range(1, 9)] + [str(i) for i in range(9, 22)]
                              + [f"22.{i}" for i in range(1, 6)] + [str(i) for i in range(23, 36)])

SYNONYMS = {
    "kind": {"NOT_IMPLEMENTED": "CAPABILITY", "EMPIRICAL_FINDING": "CLAIM_NEEDING_EVIDENCE",
             "IMPLEMENTATION_FACT": "REQUIREMENT", "QUESTION": "OPEN_QUESTION", "CLAIM": "CLAIM_NEEDING_EVIDENCE"},
    "implementation_state": {"NOT_IMPLEMENTED": "MISSING", "UNIMPLEMENTED": "MISSING", "ABSENT": "MISSING",
                             "PARTIALLY_IMPLEMENTED": "PARTIAL", "DONE": "IMPLEMENTED",
                             "BLOCKED": "BLOCKED_PLATFORM", "PLATFORM_BLOCKED": "BLOCKED_PLATFORM",
                             "DEFERRED": "DEFERRED_APPROVED"},
    "evidence_state": {"SUPPORTED": "SUPPORTED_WITH_LIMITATIONS", "EMPIRICAL": "EMPIRICALLY_SUPPORTED",
                       "NONE": "UNTESTED", "UNKNOWN": "UNTESTED", "EXTERNAL_REVIEW": "EXTERNAL_REVIEW_REQUIRED",
                       "REFUTED": "CONTRADICTED", "INSUFFICIENT_EVIDENCE": "INSUFFICIENT"},
    "release_relevance": {"BLOCKING": "V1_BLOCKING", "IMPORTANT": "V1_IMPORTANT", "POSTV1": "POST_V1",
                          "INFO": "INFORMATIONAL"},
    "initial_status": {"INSUFFICIENT": "INSUFFICIENT_EVIDENCE", "EXTERNAL_REVIEW": "EXTERNAL_REVIEW_REQUIRED",
                       "OPEN": "INSUFFICIENT_EVIDENCE"},
    "blocker_class": {"EXTERNAL": "EXTERNAL_REVIEW", "HUMAN": "HUMAN_PARTICIPANTS", "PLATFORM_BLOCKED": "PLATFORM",
                      "RESOURCE": "RESOURCES"},
}
ENUMS = {
    "kind": KINDS, "implementation_state": IMPLEMENTATION_STATES, "evidence_state": EVIDENCE_STATES,
    "release_relevance": RELEASE_RELEVANCE, "initial_status": STATUSES, "blocker_class": BLOCKER_CLASSES,
}
REQ_ENUM_FIELDS = ["kind", "implementation_state", "evidence_state", "release_relevance"]
DEC_ENUM_FIELDS = ["initial_status", "release_relevance", "blocker_class"]

REQ_FIELDS = ["record_type", "cluster", "canonical_key", "kind", "requirement_or_question", "source",
              "source_location", "additional_sources", "superseding_authority", "supersession_note",
              "implementation_state", "implementation_evidence", "evidence_state", "research_needed",
              "decision_needed", "decision_keys", "release_relevance", "program_sections"]
DEC_FIELDS = ["record_type", "cluster", "decision_key", "question", "archetypal_objective", "hard_constraints",
              "candidate_set", "candidate_exclusion_reasons", "required_evidence", "program_sections",
              "requirement_keys", "initial_status", "external_review_requirement", "release_relevance"]
REQ_REQUIRED_NEW = ["cluster", "canonical_key", "kind", "requirement_or_question", "source", "source_location",
                    "implementation_state", "evidence_state", "release_relevance"]
DEC_REQUIRED_NEW = ["cluster", "decision_key", "question", "candidate_set", "requirement_keys"]
REQ_LIST_FIELDS = {"additional_sources", "decision_keys", "program_sections"}
DEC_LIST_FIELDS = {"hard_constraints", "candidate_set", "candidate_exclusion_reasons", "required_evidence",
                   "program_sections", "requirement_keys"}

CSV_COLUMNS = ["req_id", "cluster", "kind", "requirement_or_question", "source", "source_location",
               "additional_sources", "superseding_authority", "supersession_note", "implementation_state",
               "implementation_evidence", "evidence_state", "research_needed", "decision_needed", "decision_ids",
               "release_relevance", "program_sections"]
DECISION_EVIDENCE_FIELDS = ["experiment_ids", "raw_result_refs", "normalized_result_refs", "heldout_result_refs",
                            "counterevidence", "sensitivity_analysis", "complexity_cost", "dependency_cost",
                            "security_cost", "wire_cost", "selected_decision", "decision_scope", "confidence",
                            "known_limitations", "reopen_trigger", "remaining_unknowns_ref"]
DECISION_LIST_EVIDENCE_FIELDS = {"experiment_ids", "raw_result_refs", "normalized_result_refs", "heldout_result_refs"}
DECISION_COLUMNS = (["decision_id", "status", "blocker_class", "cluster", "question", "requirement_ids",
                     "program_sections", "release_relevance", "archetypal_objective", "hard_constraints",
                     "candidate_set", "candidate_exclusion_reasons", "required_evidence"]
                    + DECISION_EVIDENCE_FIELDS[:15]
                    + ["external_review_requirement", "remaining_unknowns_ref", "history"])
CREATION_CHANGE = "created by Phase A audit"
AUDIT_HISTORY_PREFIXES = (CREATION_CHANGE, "assembler:")

# Evidence that needs people (usability studies, interviews, demand surveys), not literature or tool surveys.
HUMAN_PARTICIPANTS_RX = re.compile(
    r"\b(participants?|user (?:stud(?:y|ies)|tests?|testing|interviews?|surveys?)|usability (?:study|studies|tests?|testing)|"
    r"interviews?|think-aloud|human subjects?|gatekeeper conversations?|adopter tests?|"
    r"(?:demand|consumer|user|developer|integrator|stakeholder|maintainer|decision-maker|adopter)s? (?:requirements? )?surveys?|"
    r"surveys?\b[^.;]{0,40}\b(?:users|developers|maintainers|publishers|integrators|stakeholders|adopters))\b", re.IGNORECASE)
LOCAL_KEY_RX = re.compile(r"\b([a-z0-9]+(?:-[a-z0-9]+)*-\d{4})\b")
BRACKET_RX = re.compile(r"\[([^\[\]]*)\]")
REQ_ID_RX = re.compile(r"^REQ-([A-Z]{3})-(\d{4})$")
DEC_ID_RX = re.compile(r"^DEC-([A-Z]{3})-(\d{3})$")

KNOWN_EXTERNAL_SHA256 = {
    "design/2026-08-29-entrybound-product-architecture.md": "1f881c7b13f193b41353b90066d33ca6abcb7d4c3dcd4f58e22834801f0fe29c",
    "design/research-appendix/entrybound-architecture-research-appendix.tgz": "47a3240a685300206353c84c348ac504266b2e3e335bc05c441369a9e7d4db99",
    "research/2026-08-29-archive-category-research.md": "bc524da26e85e20bce515b81a9e84ba1a412599ca66b08bdda7ab3005c4b8cfe",
    "research/2026-08-29-entrybound-opportunity-validation.md": "8c3575a463e492d99298400c2bf77f0f8b6fa260368671b23398e00c9c1bd301",
    "research/2026-08-29-entrybound-research-iii-final-gate.md": "bb209e0c74bf3aac4067efdf54aeb70efd0cee232945903e6b11d1bdfee495c3",
    "research/entrybound-research-iii-instrumentation.tgz": "d221f434dd92641156003fe4b1fa9d50ba7fa10d4c848374cb37cd14017810c8",
}

STOPWORDS = frozenset(
    "a an the of to and or in on for with by is are be must not no as at from that this which it its each every any "
    "all only when whether than per into under over via vs versus does do can should may has have without other "
    "their there these those such same one two v1 entrybound what how".split())


# --------------------------------------------------------------------------- helpers

def ws(text):
    return " ".join(str(text).split())


def section_key(section):
    return tuple(int(part) for part in str(section).split("."))


def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def line_count(data):
    if b"\x00" in data[:8192]:
        return "binary"
    return data.count(b"\n") + (1 if data and not data.endswith(b"\n") else 0)


def nonempty(value):
    if value is None:
        return False
    if isinstance(value, str):
        return value.strip() != ""
    if isinstance(value, (list, dict)):
        return len(value) > 0
    return True


def md_cell(value):
    return ws(value).replace("|", "\\|") if value not in (None, "") else ""


def node_str(node):
    return f"{node[0]}/{node[1]}"


def parse_node(text):
    cluster, _, key = text.partition("/")
    return (cluster, key)


def join_text(first, second, sep=" | "):
    first = (first or "").strip()
    second = (second or "").strip()
    if not second or second in first:
        return first
    if not first or first in second:
        return second
    return f"{first}{sep}{second}"


def union_strings(first, second):
    out = list(first)
    seen = set(out)
    for item in second:
        if item not in seen:
            out.append(item)
            seen.add(item)
    return out


def union_by(first, second, keyfn):
    out = list(first)
    seen = {keyfn(item) for item in out}
    for item in second:
        key = keyfn(item)
        if key not in seen:
            out.append(item)
            seen.add(key)
    return out


def strip_key_brackets(location):
    return ws(re.sub(r"\s*\[[^\[\]]*\]", "", location or ""))


def stricter(order, a, b):
    if a not in order:
        return b
    if b not in order:
        return a
    return a if order.index(a) <= order.index(b) else b


def normalize_enum(field, value):
    allowed = ENUMS[field]
    if value in allowed:
        return value
    canon = re.sub(r"[\s\-]+", "_", str(value).strip().upper())
    if canon in allowed:
        return canon
    synonyms = SYNONYMS.get(field, {})
    fixed = synonyms.get(canon) or synonyms.get(canon.replace("_", ""))
    return fixed if fixed in allowed else None


def read_jsonl(path, errors):
    rows = []
    with open(path, encoding="utf-8") as handle:
        for lineno, line in enumerate(handle, 1):
            if not line.strip():
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError as exc:
                errors.append(f"{rel(path)}:{lineno}: invalid JSON ({exc.msg})")
                continue
            if not isinstance(obj, dict):
                errors.append(f"{rel(path)}:{lineno}: JSON line is not an object")
                continue
            rows.append((lineno, obj))
    return rows


def rel(path):
    try:
        return pathlib.Path(path).resolve().relative_to(REPO).as_posix()
    except ValueError:
        return pathlib.Path(path).as_posix()


def write_text(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(text)


def file_sha256(path):
    return sha256_bytes(pathlib.Path(path).read_bytes())


# --------------------------------------------------------------------------- similarity

def _stem(word):
    for suffix in ("ations", "ation", "ings", "ing", "ies", "es", "ed", "s"):
        if len(word) > len(suffix) + 3 and word.endswith(suffix):
            return word[: -len(suffix)]
    return word


def tokens(text):
    return [_stem(w) for w in re.findall(r"[a-z0-9]+", str(text).lower()) if w not in STOPWORDS and len(w) > 1]


def tfidf(docs):
    df = collections.Counter(t for doc in docs for t in set(doc))
    n = len(docs)
    vectors = []
    for doc in docs:
        tf = collections.Counter(doc)
        vec = {t: (1 + math.log(c)) * math.log(n / df[t]) for t, c in sorted(tf.items())}
        norm = math.sqrt(sum(x * x for x in vec.values())) or 1.0
        vectors.append({t: x / norm for t, x in vec.items()})
    return vectors


def cosine(u, v):
    if len(u) > len(v):
        u, v = v, u
    return sum(x * v.get(t, 0.0) for t, x in sorted(u.items()))


# --------------------------------------------------------------------------- mini JSON Schema validator

def schema_validate(instance, schema, root=None, path="$"):
    """Validate a subset of JSON Schema 2020-12 used by the ledger schemas."""
    root = root if root is not None else schema
    errors = []
    if "$ref" in schema:
        ref = schema["$ref"]
        target = root
        for part in ref.lstrip("#/").split("/"):
            target = target[part]
        return schema_validate(instance, target, root, path)
    if "anyOf" in schema:
        if not any(not schema_validate(instance, sub, root, path) for sub in schema["anyOf"]):
            errors.append(f"{path}: does not match any allowed form")
        return errors
    expected = schema.get("type")
    if expected is not None:
        types = expected if isinstance(expected, list) else [expected]
        checks = {"string": lambda v: isinstance(v, str), "array": lambda v: isinstance(v, list),
                  "object": lambda v: isinstance(v, dict), "boolean": lambda v: isinstance(v, bool),
                  "integer": lambda v: isinstance(v, int) and not isinstance(v, bool), "null": lambda v: v is None}
        if not any(checks[t](instance) for t in types):
            return [f"{path}: expected {expected}"]
    if "const" in schema and instance != schema["const"]:
        errors.append(f"{path}: expected constant {schema['const']!r}")
    if "enum" in schema and instance not in schema["enum"]:
        errors.append(f"{path}: {instance!r} not in enumeration")
    if isinstance(instance, str):
        if "minLength" in schema and len(instance) < schema["minLength"]:
            errors.append(f"{path}: shorter than {schema['minLength']}")
        if "pattern" in schema and not re.search(schema["pattern"], instance):
            errors.append(f"{path}: {instance[:60]!r} does not match {schema['pattern']}")
    if isinstance(instance, list):
        if "minItems" in schema and len(instance) < schema["minItems"]:
            errors.append(f"{path}: fewer than {schema['minItems']} items")
        if schema.get("uniqueItems"):
            seen = [json.dumps(i, sort_keys=True) for i in instance]
            if len(seen) != len(set(seen)):
                errors.append(f"{path}: items not unique")
        if "items" in schema:
            for index, item in enumerate(instance):
                errors.extend(schema_validate(item, schema["items"], root, f"{path}[{index}]"))
    if isinstance(instance, dict):
        for name in schema.get("required", []):
            if name not in instance:
                errors.append(f"{path}: missing {name}")
        props = schema.get("properties", {})
        for name, value in instance.items():
            if name in props:
                errors.extend(schema_validate(value, props[name], root, f"{path}.{name}"))
            elif schema.get("additionalProperties") is False:
                errors.append(f"{path}: unexpected property {name}")
            elif isinstance(schema.get("additionalProperties"), dict):
                errors.extend(schema_validate(value, schema["additionalProperties"], root, f"{path}.{name}"))
    return errors


def build_schemas():
    string = {"type": "string"}
    section = {"type": "string", "enum": SECTIONS}
    cluster = {"type": "string", "enum": sorted(CLUSTER_CODES)}
    relevance = {"type": "string", "enum": RELEASE_RELEVANCE}
    source_pattern = r"^(SPEC|R1|R2|R3|R3-INSTR|PROGRAM|APPX/.+|docs/.+|code:.+)$"
    req_schema = {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://github.com/entrybound/entrybound/research/tools/ledger/requirement-ledger.schema.json",
        "title": "Entrybound requirement ledger row",
        "description": ("One decoded row of research/requirement-ledger.csv (RFC 4180, UTF-8, header row, CRLF). "
                        "CSV encoding: additional_sources is a JSON array; decision_ids and program_sections are "
                        "'; '-separated lists (empty string for none). Rows are sorted by req_id."),
        "type": "object",
        "additionalProperties": False,
        "required": CSV_COLUMNS,
        "properties": {
            "req_id": {"type": "string", "pattern": r"^REQ-(MOD|CON|CMP|ACC|PLT|CRY|LEG|INT|ECO)-\d{4}$"},
            "cluster": cluster,
            "kind": {"type": "string", "enum": KINDS},
            "requirement_or_question": {"type": "string", "minLength": 1},
            "source": {"type": "string", "pattern": source_pattern},
            "source_location": {"type": "string", "minLength": 1},
            "additional_sources": {"type": "array", "items": {"$ref": "#/$defs/source_ref"}},
            "superseding_authority": string,
            "supersession_note": string,
            "implementation_state": {"type": "string", "enum": IMPLEMENTATION_STATES},
            "implementation_evidence": string,
            "evidence_state": {"type": "string", "enum": EVIDENCE_STATES},
            "research_needed": string,
            "decision_needed": string,
            "decision_ids": {"type": "array", "uniqueItems": True,
                             "items": {"type": "string", "pattern": r"^DEC-(MOD|CON|CMP|ACC|PLT|CRY|LEG|INT|ECO)-\d{3}$"}},
            "release_relevance": relevance,
            "program_sections": {"type": "array", "uniqueItems": True, "items": section},
        },
        "$defs": {
            "source_ref": {
                "anyOf": [
                    {"type": "object", "additionalProperties": False, "required": ["source", "location", "local_key"],
                     "properties": {"source": {"type": "string", "pattern": r"^(SPEC|R1|R2|R3|R3-INSTR|APPX/.+|docs(/.+)?|code:.+)$"},
                                    "location": {"type": "string"},
                                    "local_key": {"type": "string", "pattern": r"^[a-z0-9]+(-[a-z0-9]+)*-\d{4}$"}}},
                    {"type": "object", "additionalProperties": False, "required": ["source", "location", "local_key"],
                     "properties": {"source": {"const": "PROGRAM"}, "location": {"type": "string"},
                                    "local_key": {"const": ""}}},
                ]
            }
        },
    }
    dec_schema = {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://github.com/entrybound/entrybound/research/tools/ledger/decision-ledger.schema.json",
        "title": "Entrybound decision ledger row",
        "description": "One line of research/decision-ledger.jsonl; lines are sorted by decision_id.",
        "type": "object",
        "additionalProperties": False,
        "required": DECISION_COLUMNS,
        "properties": {
            "decision_id": {"type": "string", "pattern": r"^DEC-(MOD|CON|CMP|ACC|PLT|CRY|LEG|INT|ECO)-\d{3}$"},
            "status": {"type": "string", "enum": STATUSES},
            "blocker_class": {"type": "string", "enum": BLOCKER_CLASSES},
            "cluster": cluster,
            "question": {"type": "string", "minLength": 1},
            "requirement_ids": {"type": "array", "minItems": 1, "uniqueItems": True,
                                "items": {"type": "string", "pattern": r"^REQ-(MOD|CON|CMP|ACC|PLT|CRY|LEG|INT|ECO)-\d{4}$"}},
            "program_sections": {"type": "array", "uniqueItems": True, "items": section},
            "release_relevance": relevance,
            "archetypal_objective": string,
            "hard_constraints": {"type": "array", "items": string},
            "candidate_set": {"type": "array", "minItems": 1, "items": {
                "type": "object", "required": ["candidate_id", "description"],
                "properties": {"candidate_id": {"type": "string", "minLength": 1}, "description": string}}},
            "candidate_exclusion_reasons": {"type": "array", "items": {
                "type": "object", "required": ["candidate_id", "reason"],
                "properties": {"candidate_id": {"type": "string", "minLength": 1}, "reason": string,
                               "invariant_violated": string}}},
            "required_evidence": {"type": "array", "items": string},
            "experiment_ids": {"type": "array", "items": string},
            "raw_result_refs": {"type": "array", "items": string},
            "normalized_result_refs": {"type": "array", "items": string},
            "heldout_result_refs": {"type": "array", "items": string},
            "counterevidence": string,
            "sensitivity_analysis": string,
            "complexity_cost": string,
            "dependency_cost": string,
            "security_cost": string,
            "wire_cost": string,
            "selected_decision": string,
            "decision_scope": string,
            "confidence": string,
            "known_limitations": string,
            "reopen_trigger": string,
            "external_review_requirement": string,
            "remaining_unknowns_ref": string,
            "history": {"type": "array", "minItems": 1, "items": {
                "type": "object", "additionalProperties": False, "required": ["date", "change"],
                "properties": {"date": {"type": "string", "pattern": r"^\d{4}-\d{2}-\d{2}$"},
                               "change": {"type": "string", "minLength": 1}}}},
        },
    }
    return req_schema, dec_schema


# --------------------------------------------------------------------------- assembler state

class Assembler:
    def __init__(self, args):
        self.args = args
        self.date = args.date
        self.errors = []
        self.warnings = []
        self.log = collections.defaultdict(list)
        self.extract = {}
        self.extract_slice = {}
        self.reqs = {}
        self.decs = {}
        self.req_alias = {}
        self.dec_alias = {}
        self.inputs = []
        self.id_map = None
        self.near = {"merges": [], "distinct": []}
        self.edges = set()
        self.req_ids = {}
        self.dec_ids = {}
        self.deferred_merges = {}

    # ----------------------------------------------------------------- loading
    def load_extract(self):
        for path in sorted(EXTRACT.glob("*.jsonl")):
            slice_name = path.stem
            self.inputs.append(path)
            for _, rec in read_jsonl(path, self.errors):
                key = rec.get("local_key")
                if not key:
                    self.errors.append(f"{rel(path)}: extraction record without local_key")
                    continue
                if key in self.extract:
                    self.errors.append(f"duplicate extraction local_key {key}")
                self.extract[key] = rec
                self.extract_slice[key] = slice_name

    def load_id_map(self):
        if ID_MAP.exists():
            self.id_map = json.loads(ID_MAP.read_text(encoding="utf-8"))
        else:
            self.id_map = {}
        for field, default in (("schema", "entrybound-id-map/v1"), ("requirements", {}), ("decisions", {}),
                               ("requirement_aliases", {}), ("decision_aliases", {}), ("retired_ids", {}),
                               ("next", {})):
            self.id_map.setdefault(field, default)

    def load_near_duplicates(self):
        if NEAR_DUPLICATES.exists():
            self.near = json.loads(NEAR_DUPLICATES.read_text(encoding="utf-8"))
            self.inputs.append(NEAR_DUPLICATES)
        self.near.setdefault("merges", [])
        self.near.setdefault("distinct", [])
        self.review_threshold = float(self.near.get("review_threshold", 0.30))
        self.auto_threshold = float(self.near.get("auto_merge_threshold", 0.70))

    def load_merged(self):
        raw_reqs, raw_decs = [], []
        for path in sorted(MERGED.glob("*-requirements.jsonl")):
            job = path.name[: -len("-requirements.jsonl")]
            self.inputs.append(path)
            for lineno, row in read_jsonl(path, self.errors):
                row["_job"], row["_origin"] = job, f"{rel(path)}:{lineno}"
                raw_reqs.append(row)
        for path in sorted(MERGED.glob("*-decisions.jsonl")):
            job = path.name[: -len("-decisions.jsonl")]
            self.inputs.append(path)
            for lineno, row in read_jsonl(path, self.errors):
                row["_job"], row["_origin"] = job, f"{rel(path)}:{lineno}"
                raw_decs.append(row)
        for row in raw_reqs:
            self.prepare_row(row, "requirement")
        for row in raw_decs:
            self.prepare_row(row, "decision")
        self.resolve_collisions(raw_reqs, "requirement", raw_decs)
        self.resolve_collisions(raw_decs, "decision", raw_reqs)

    def prepare_row(self, row, record_type, partial=False):
        fields = REQ_FIELDS if record_type == "requirement" else DEC_FIELDS
        list_fields = REQ_LIST_FIELDS if record_type == "requirement" else DEC_LIST_FIELDS
        if not partial:
            for field in fields:
                if field not in row:
                    row[field] = [] if field in list_fields else ""
            row["record_type"] = record_type
        for field in list_fields:
            if field in row and not isinstance(row[field], list):
                self.log["enum_fixes"].append(f"{row.get('_origin')}: {field} was not a list; wrapped")
                row[field] = [row[field]] if nonempty(row[field]) else []
        enum_fields = REQ_ENUM_FIELDS if record_type == "requirement" else DEC_ENUM_FIELDS
        if record_type == "decision" and "status" in row and nonempty(row["status"]):
            row["initial_status"] = row.pop("status")
        for field in enum_fields:
            if field not in row or not nonempty(row[field]):
                continue
            value = row[field]
            fixed = normalize_enum(field, value)
            if fixed is None:
                self.errors.append(f"{row.get('_origin')}: invalid {field} {value!r} (no minimal fix)")
            elif fixed != value:
                self.log["enum_fixes"].append(f"{row.get('_origin')}: {field} {value!r} -> {fixed!r}")
                row[field] = fixed
        if "program_sections" in row:
            fixed_sections = []
            for item in row["program_sections"]:
                canon = str(item).strip().lstrip("§").strip()
                if canon in SECTIONS:
                    fixed_sections.append(canon)
                    if canon != item:
                        self.log["enum_fixes"].append(f"{row.get('_origin')}: program section {item!r} -> {canon!r}")
                else:
                    self.log["enum_fixes"].append(f"{row.get('_origin')}: dropped invalid program section {item!r}")
            row["program_sections"] = sorted(set(fixed_sections), key=section_key)
        if record_type == "requirement" and "additional_sources" in row:
            cleaned = []
            for entry in row["additional_sources"]:
                if not isinstance(entry, dict):
                    self.errors.append(f"{row.get('_origin')}: additional_sources entry is not an object")
                    continue
                cleaned.append({"source": str(entry.get("source", "")), "location": str(entry.get("location", "")),
                                "local_key": str(entry.get("local_key", "") or "")})
            row["additional_sources"] = cleaned
        if record_type == "decision" and not partial and not nonempty(row.get("initial_status")):
            row["initial_status"] = "INSUFFICIENT_EVIDENCE"
        if record_type == "decision" and "blocker_class" in row:
            row["_blocker_override"] = row.pop("blocker_class")
        if partial:
            row.pop("record_type", None)

    # ----------------------------------------------------------------- keys and merging
    def key_of(self, row, record_type):
        return row.get("canonical_key") if record_type == "requirement" else row.get("decision_key")

    def table(self, record_type):
        return self.reqs if record_type == "requirement" else self.decs

    def aliases(self, record_type):
        return self.req_alias if record_type == "requirement" else self.dec_alias

    def primary_key(self, row):
        location = row.get("source_location", "")
        for inner in reversed(BRACKET_RX.findall(location)):
            for key in LOCAL_KEY_RX.findall(inner):
                if key in self.extract:
                    return key
        slk = row.get("source_local_key")
        if slk and slk in self.extract:
            return slk
        bare = strip_key_brackets(location)
        entries = row.get("additional_sources", [])
        for entry in entries:
            if ws(entry["location"]) == bare and entry["source"] == row.get("source") and entry["local_key"] in self.extract:
                return entry["local_key"]
        for entry in entries:
            if ws(entry["location"]) == bare and entry["local_key"] in self.extract:
                return entry["local_key"]
        return None

    def text_of(self, row, record_type):
        return row.get("requirement_or_question") if record_type == "requirement" else row.get("question")

    def resolve_collisions(self, rows, record_type, other_rows):
        table = self.table(record_type)
        groups = collections.defaultdict(list)
        for row in rows:
            groups[(row.get("cluster"), self.key_of(row, record_type))].append(row)
        vectors = tfidf([tokens(self.text_of(r, record_type)) for r in rows])
        index = {id(r): i for i, r in enumerate(rows)}
        distinct_collisions = {(d.get("record_type"), d.get("cluster"), tuple(d.get("keys", [])))
                               for d in self.near.get("distinct_collisions", [])}
        for node in sorted(groups, key=lambda n: (str(n[0]), str(n[1]))):
            members = sorted(groups[node], key=lambda r: (r["_job"], r["_origin"]))
            first = members[0]
            if node[0] not in CLUSTER_CODES:
                self.errors.append(f"{first['_origin']}: unknown cluster {node[0]!r}; row skipped")
                continue
            if not node[1]:
                self.errors.append(f"{first['_origin']}: missing {record_type} key")
                continue
            table[node] = first
            for other in members[1:]:
                score = cosine(vectors[index[id(first)]], vectors[index[id(other)]])
                forced_distinct = (record_type, node[0], (node[1],)) in distinct_collisions
                if score >= self.review_threshold and not forced_distinct:
                    table[node] = first
                    self.merge(record_type, node, other, reason=f"key collision with matching meaning (text cosine {score:.3f})",
                               stage="key-collision", score=score, merged_node=None)
                else:
                    suffix = other["_job"][len(node[0]) + 1:] if other["_job"].startswith(node[0] + "-") else other["_job"]
                    new_key = f"{node[1]}--{suffix or 'dup'}"
                    counter = 2
                    while (node[0], new_key) in groups or (node[0], new_key) in table:
                        new_key = f"{node[1]}--{suffix or 'dup'}-{counter}"
                        counter += 1
                    self.rename_within_job(other_rows, other, record_type, node[1], new_key)
                    table[(node[0], new_key)] = other
                    self.log["collisions"].append(
                        f"{record_type} `{node_str(node)}` from {other['_origin']} renamed to `{new_key}` "
                        f"(meaning differs from {first['_origin']}; text cosine {score:.3f})")

    def rename_within_job(self, other_rows, row, record_type, old, new):
        if record_type == "requirement":
            row["canonical_key"] = new
        else:
            row["decision_key"] = new
        ref_field = "requirement_keys" if record_type == "requirement" else "decision_keys"
        # References from rows of the other record type in the same merge job follow the rename.
        for other in other_rows:
            if other.get("_job") == row["_job"] and other.get("cluster") == row.get("cluster"):
                other[ref_field] = [new if ref == old else ref for ref in other.get(ref_field, [])]

    def source_refs(self, row):
        refs = []
        key = self.primary_key(row)
        if key:
            refs.append(key)
        refs.extend(entry["local_key"] for entry in row.get("additional_sources", []) if entry["local_key"])
        return refs

    def merge(self, record_type, survivor_node, merged_row, reason, stage, score=None, merged_node=None):
        table = self.table(record_type)
        survivor = table[survivor_node]
        if merged_node is not None:
            table.pop(merged_node, None)
            self.aliases(record_type)[merged_node] = (survivor_node, reason)
        if record_type == "requirement":
            self.merge_requirement(survivor, merged_row, survivor_node, merged_node)
        else:
            self.merge_decision(survivor, merged_row, survivor_node, merged_node)
        survivor.setdefault("_merged", []).append(merged_node or (survivor_node[0], self.key_of(merged_row, record_type)))
        self.log["merges"].append({
            "record_type": record_type, "stage": stage, "survivor": survivor_node,
            "merged": merged_node or (survivor_node[0], self.key_of(merged_row, record_type)),
            "merged_origin": merged_row.get("_origin"), "survivor_origin": survivor.get("_origin"),
            "score": score, "reason": reason, "merged_text": self.text_of(merged_row, record_type)})

    def merge_requirement(self, survivor, merged, survivor_node, merged_node):
        own_bracket = next((k for inner in BRACKET_RX.findall(survivor["source_location"])
                            for k in LOCAL_KEY_RX.findall(inner)), None)
        entries = list(survivor["additional_sources"])
        merged_key = self.primary_key(merged)
        if merged_key:
            entries.append({"source": merged["source"], "location": strip_key_brackets(merged["source_location"]),
                            "local_key": merged_key})
        elif merged.get("source") == "PROGRAM":
            entries.append({"source": "PROGRAM", "location": ws(merged["source_location"]), "local_key": ""})
        entries.extend(merged["additional_sources"])
        survivor["additional_sources"] = [
            e for e in union_by(entries, [], lambda e: e["local_key"] or ("PROGRAM", ws(e["location"])))
            if not (own_bracket and e["local_key"] == own_bracket)
        ]
        if survivor.get("kind") != merged.get("kind"):
            self.log["merge_conflicts"].append(
                f"`{node_str(survivor_node)}` kind {survivor.get('kind')} kept over {merged.get('kind')} from merged row")
        survivor["evidence_state"] = stricter(EVIDENCE_STATES, survivor["evidence_state"], merged["evidence_state"])
        a, b = survivor["implementation_state"], merged["implementation_state"]
        if a != b:
            if a in PROGRESS_ORDER and b in PROGRESS_ORDER:
                survivor["implementation_state"] = a if PROGRESS_ORDER[a] <= PROGRESS_ORDER[b] else b
            else:
                self.log["merge_conflicts"].append(
                    f"`{node_str(survivor_node)}` implementation_state {a} kept over {b} from merged row")
        survivor["release_relevance"] = stricter(RELEASE_RELEVANCE, survivor["release_relevance"], merged["release_relevance"])
        survivor["program_sections"] = sorted(set(survivor["program_sections"]) | set(merged["program_sections"]), key=section_key)
        merged_cluster = merged_node[0] if merged_node else survivor_node[0]
        refs = [ref if ("/" in ref or merged_cluster == survivor_node[0]) else f"{merged_cluster}/{ref}"
                for ref in merged["decision_keys"]]
        survivor["decision_keys"] = union_strings(survivor["decision_keys"], refs)
        survivor["superseding_authority"] = join_text(survivor["superseding_authority"], merged["superseding_authority"], "; ")
        for field in ("supersession_note", "implementation_evidence", "research_needed", "decision_needed"):
            survivor[field] = join_text(survivor[field], merged[field])

    def merge_decision(self, survivor, merged, survivor_node, merged_node):
        survivor["hard_constraints"] = union_strings(survivor["hard_constraints"], merged["hard_constraints"])
        survivor["candidate_set"] = union_by(survivor["candidate_set"], merged["candidate_set"],
                                             lambda c: c.get("candidate_id") if isinstance(c, dict) else json.dumps(c))
        survivor["candidate_exclusion_reasons"] = union_by(
            survivor["candidate_exclusion_reasons"], merged["candidate_exclusion_reasons"],
            lambda c: json.dumps(c, sort_keys=True))
        survivor["required_evidence"] = union_strings(survivor["required_evidence"], merged["required_evidence"])
        survivor["program_sections"] = sorted(set(survivor["program_sections"]) | set(merged["program_sections"]), key=section_key)
        merged_cluster = merged_node[0] if merged_node else survivor_node[0]
        refs = [ref if ("/" in ref or merged_cluster == survivor_node[0]) else f"{merged_cluster}/{ref}"
                for ref in merged["requirement_keys"]]
        survivor["requirement_keys"] = union_strings(survivor["requirement_keys"], refs)
        survivor["initial_status"] = stricter(STATUSES, survivor["initial_status"], merged["initial_status"])
        survivor["external_review_requirement"] = join_text(survivor["external_review_requirement"],
                                                            merged["external_review_requirement"])
        survivor["release_relevance"] = stricter(RELEASE_RELEVANCE, survivor["release_relevance"], merged["release_relevance"])
        survivor["archetypal_objective"] = join_text(survivor["archetypal_objective"], merged["archetypal_objective"], " ")
        if "_blocker_override" in merged and "_blocker_override" not in survivor:
            survivor["_blocker_override"] = merged["_blocker_override"]

    def follow(self, record_type, node):
        aliases = self.aliases(record_type)
        seen = set()
        while node in aliases and node not in seen:
            seen.add(node)
            node = aliases[node][0]
        return node

    # ----------------------------------------------------------------- near duplicates
    def near_duplicates(self, record_type, added=None):
        """Merge near-duplicate rows of different jobs within one cluster.

        First pass (added is None): all pairs of merged-job rows. Addenda pass (added is a set of nodes
        added by addenda): only pairs involving an added row, scored over the post-addenda table.
        """
        table = self.table(record_type)
        suffix = "" if added is None else "_addenda"
        stage_tag = "" if added is None else ", addenda"
        nodes = sorted(table)
        docs = []
        for node in nodes:
            row = table[node]
            docs.append(tokens(self.text_of(row, record_type)) + tokens(node[1].replace("-", " ")) * 2)
        vectors = dict(zip(nodes, tfidf(docs)))
        candidates = []
        by_cluster = collections.defaultdict(list)
        for node in nodes:
            by_cluster[node[0]].append(node)
        for cluster in sorted(by_cluster):
            for a, b in itertools.combinations(by_cluster[cluster], 2):
                if table[a]["_job"] == table[b]["_job"]:
                    continue
                if added is not None and a not in added and b not in added:
                    continue
                score = cosine(vectors[a], vectors[b])
                if score >= self.review_threshold:
                    candidates.append((a, b, score))
        adjudicated_groups = {}
        for entry in self.near["merges"]:
            if entry.get("record_type") != record_type:
                continue
            cluster = entry["cluster"]
            survivor = (cluster, entry["survivor"])
            group = {survivor} | {(cluster, k) for k in entry["merged"]}
            for member in group:
                adjudicated_groups[member] = survivor
        distinct = set()
        for entry in self.near["distinct"]:
            if entry.get("record_type") == record_type:
                keys = entry["keys"]
                distinct.add((entry["cluster"], min(keys), max(keys)))
        score_of = {(a, b): s for a, b, s in candidates}
        score_of.update({(b, a): s for a, b, s in candidates})
        # Adjudicated merges. Entries naming rows that only addenda add are deferred to the addenda pass.
        if added is None:
            work = [(entry, key) for entry in sorted((e for e in self.near["merges"] if e.get("record_type") == record_type),
                                                     key=lambda e: (e["cluster"], e["survivor"]))
                    for key in sorted(entry["merged"])]
        else:
            work = self.deferred_merges.pop(record_type, [])
        deferred = []
        for entry, key in work:
            survivor = self.follow(record_type, (entry["cluster"], entry["survivor"]))
            node = (entry["cluster"], key)
            if survivor not in table or node not in table:
                missing = node_str(survivor) if survivor not in table else node_str(node)
                if added is None:
                    deferred.append((entry, key))
                else:
                    self.warnings.append(f"stale near-duplicate adjudication: {record_type} {missing} not found")
                continue
            score = score_of.get((survivor, node))
            self.merge(record_type, survivor, table[node], reason=entry.get("reason", "adjudicated near-duplicate"),
                       stage=f"near-duplicate (adjudicated{stage_tag})", score=score, merged_node=node)
        if added is None:
            self.deferred_merges[record_type] = deferred
        # Unadjudicated candidates: auto-merge above the auto threshold, otherwise report.
        unadjudicated = []
        for a, b, score in sorted(candidates, key=lambda c: (-round(c[2], 12), c[0], c[1])):
            if (a[0], min(a[1], b[1]), max(a[1], b[1])) in distinct:
                continue
            if a in adjudicated_groups and b in adjudicated_groups and adjudicated_groups[a] == adjudicated_groups[b]:
                continue
            fa, fb = self.follow(record_type, a), self.follow(record_type, b)
            if fa == fb:
                continue
            if score >= self.auto_threshold and fa in table and fb in table:
                survivor, merged = sorted([fa, fb], key=lambda n: self.survivor_rank(record_type, n))
                self.merge(record_type, survivor, table[merged], reason=f"auto-merged near-duplicate (cosine {score:.3f})",
                           stage=f"near-duplicate (auto{stage_tag})", score=score, merged_node=merged)
            else:
                unadjudicated.append((a, b, score))
        for a, b, score in unadjudicated:
            self.warnings.append(f"unadjudicated near-duplicate candidate ({record_type}{stage_tag}, cosine {score:.3f}): "
                                 f"{node_str(a)} <> {node_str(b)}")
        self.log[f"unadjudicated{suffix}_{record_type}"] = unadjudicated
        self.log[f"candidates{suffix}_{record_type}"] = len(candidates)
        return unadjudicated

    def survivor_rank(self, record_type, node):
        registry = self.id_map["requirements" if record_type == "requirement" else "decisions"]
        existing = registry.get(node_str(node))
        row = self.table(record_type)[node]
        refs = len(self.source_refs(row)) if record_type == "requirement" else len(row.get("requirement_keys", []))
        return (0 if existing else 1, existing or "", -refs, row["_job"], node[1])

    # ----------------------------------------------------------------- addenda
    def apply_addenda(self):
        files = sorted(ADDENDA.glob("*.jsonl")) if ADDENDA.exists() else []
        for path in files:
            self.inputs.append(path)
            for lineno, row in read_jsonl(path, self.errors):
                origin = f"{rel(path)}:{lineno}"
                row["_origin"], row["_job"] = origin, f"addenda:{path.stem}"
                record_type = row.get("record_type") or (
                    "requirement" if ("canonical_key" in row or "req_id" in row) else
                    "decision" if ("decision_key" in row or "decision_id" in row) else None)
                if record_type not in ("requirement", "decision"):
                    self.errors.append(f"{origin}: cannot determine record_type")
                    continue
                node = self.addendum_node(row, record_type)
                if node is None:
                    self.errors.append(f"{origin}: addendum row lacks cluster and key")
                    continue
                target = self.follow(record_type, node)
                if target not in self.table(record_type):
                    previous = self.id_map[f"{record_type}_aliases"].get(node_str(node))
                    if previous:
                        target = self.follow(record_type, parse_node(previous["alias_of"]))
                table = self.table(record_type)
                if target in table:
                    self.update_row(record_type, target, row, origin, redirected=(target != node))
                else:
                    self.add_row(record_type, node, row, origin)

    def addendum_node(self, row, record_type):
        key_field = "canonical_key" if record_type == "requirement" else "decision_key"
        if row.get("cluster") and row.get(key_field):
            return (row["cluster"], row[key_field])
        id_field = "req_id" if record_type == "requirement" else "decision_id"
        if row.get(id_field):
            registry = self.id_map["requirements" if record_type == "requirement" else "decisions"]
            for key, value in registry.items():
                if value == row[id_field]:
                    return parse_node(key)
        return None

    def update_row(self, record_type, node, row, origin, redirected):
        target = self.table(record_type)[node]
        update = dict(row)
        self.prepare_row(update, record_type, partial=True)
        allowed = set(REQ_FIELDS if record_type == "requirement" else DEC_FIELDS) - {"record_type", "cluster", "canonical_key", "decision_key"}
        changed = []
        dropped = {}
        for field in sorted(update):
            value = update[field]
            if field.startswith("_") and field != "_blocker_override":
                continue
            if field in ("correction_reason", "req_id", "decision_id", "cluster", "canonical_key", "decision_key", "record_type"):
                continue
            if field == "additional_sources":
                before = len(target["additional_sources"])
                target["additional_sources"] = union_by(target["additional_sources"], value,
                                                        lambda e: e["local_key"] or ("PROGRAM", ws(e["location"])))
                if len(target["additional_sources"]) != before:
                    changed.append("additional_sources")
                continue
            if field == "_blocker_override":
                if nonempty(value):
                    fixed = normalize_enum("blocker_class", value)
                    if fixed is None:
                        self.errors.append(f"{origin}: invalid blocker_class {value!r} (no minimal fix)")
                        continue
                    if fixed != value:
                        self.log["enum_fixes"].append(f"{origin}: blocker_class {value!r} -> {fixed!r}")
                    target["_blocker_override"] = fixed
                    target["_status_addendum"] = f"addendum {origin}: {ws(row.get('correction_reason', ''))}".rstrip(": ")
                    changed.append("blocker_class")
                continue
            if field not in allowed:
                self.warnings.append(f"{origin}: ignored unknown field {field!r}")
                continue
            if nonempty(value) and target.get(field) != value:
                if record_type == "decision" and field == "initial_status":
                    target["_status_addendum"] = f"addendum {origin}: {ws(row.get('correction_reason', ''))}".rstrip(": ")
                if record_type == "requirement" and field == "implementation_state":
                    target.setdefault("_impl_addenda", []).append(origin)
                old = target.get(field)
                if isinstance(old, list) and isinstance(value, list):
                    def keyfn(item, field=field):
                        if field in ("decision_keys", "requirement_keys"):
                            ref_type = "decision" if field == "decision_keys" else "requirement"
                            resolved, _ = self.resolve(ref_type, item, node[0])
                            return node_str(resolved) if resolved else str(item)
                        if isinstance(item, dict) and "candidate_id" in item:
                            return item["candidate_id"]
                        return item if isinstance(item, str) else json.dumps(item, sort_keys=True, ensure_ascii=False)
                    new_keys = {keyfn(item) for item in value}
                    lost = [keyfn(item) for item in old if keyfn(item) not in new_keys]
                    if lost:
                        dropped[field] = lost
                target[field] = value
                changed.append(field)
        target.setdefault("_addenda", []).append(origin)
        self.log["addenda_updates"].append({
            "record_type": record_type, "node": node, "origin": origin, "fields": changed, "dropped": dropped,
            "reason": ws(row.get("correction_reason", "")), "redirected": redirected})

    def add_row(self, record_type, node, row, origin):
        required = REQ_REQUIRED_NEW if record_type == "requirement" else DEC_REQUIRED_NEW
        missing = [field for field in required if not nonempty(row.get(field))]
        if missing:
            self.errors.append(f"{origin}: new {record_type} {node_str(node)} missing required fields {missing}")
            return
        if node[0] not in CLUSTER_CODES:
            self.errors.append(f"{origin}: unknown cluster {node[0]!r}")
            return
        new = {k: v for k, v in row.items() if k not in ("correction_reason",)}
        self.prepare_row(new, record_type)
        new["_addenda"] = [origin]
        self.table(record_type)[node] = new
        self.log["addenda_additions"].append({"record_type": record_type, "node": node, "origin": origin,
                                              "reason": ws(row.get("correction_reason", ""))})

    # ----------------------------------------------------------------- cross-cluster dedupe
    def dedupe_primary_keys(self):
        claims = collections.defaultdict(list)
        for node in sorted(self.reqs):
            key = self.primary_key(self.reqs[node])
            if key:
                claims[key].append(node)
        for key in sorted(claims):
            nodes = claims[key]
            clusters = {n[0] for n in nodes}
            if len(nodes) < 2:
                continue
            if len(clusters) < 2:
                self.warnings.append(f"primary local_key {key} is claimed by {len(nodes)} rows of one cluster: "
                                     + ", ".join(node_str(n) for n in nodes))
                continue
            home = self.extract.get(key, {}).get("primary_cluster")
            ordered = sorted(nodes, key=lambda n: (0 if n[0] == home else 1, self.survivor_rank("requirement", n)))
            survivor = ordered[0]
            for other in ordered[1:]:
                if other[0] == survivor[0]:
                    continue
                self.merge("requirement", survivor, self.reqs[other],
                           reason=f"rows in clusters {survivor[0]} and {other[0]} claim primary local_key {key}",
                           stage="cross-cluster primary-key dedupe", merged_node=other)

    # ----------------------------------------------------------------- links
    def resolve(self, record_type, ref, from_cluster):
        table = self.table(record_type)
        ref = str(ref).strip()
        id_rx = REQ_ID_RX if record_type == "requirement" else DEC_ID_RX
        if id_rx.match(ref):
            registry = self.id_map["requirements" if record_type == "requirement" else "decisions"]
            for key, value in registry.items():
                if value == ref:
                    node = self.follow(record_type, parse_node(key))
                    return (node, "id") if node in table else (None, "unresolved")
            return None, "unresolved"
        if "/" in ref:
            node = self.follow(record_type, parse_node(ref))
            if node in table:
                return node, "qualified"
            ref = parse_node(ref)[1]
        node = self.follow(record_type, (from_cluster, ref))
        if node in table:
            return node, "local" if node == (from_cluster, ref) else "alias"
        found = set()
        for cluster in CLUSTER_CODES:
            candidate = self.follow(record_type, (cluster, ref))
            if candidate in table:
                found.add(candidate)
        if len(found) == 1:
            return found.pop(), "cross-cluster"
        if len(found) > 1:
            return None, "ambiguous"
        return None, "unresolved"

    def build_links(self):
        from_req, from_dec = set(), set()
        for node in sorted(self.reqs):
            row = self.reqs[node]
            for ref in row["decision_keys"]:
                target, how = self.resolve("decision", ref, node[0])
                if target is None:
                    self.errors.append(f"requirement {node_str(node)}: {how} decision reference {ref!r}")
                    continue
                if how != "local":
                    self.log["resolutions"].append(f"requirement `{node_str(node)}` -> decision `{node_str(target)}` ({how}: {ref})")
                from_req.add((node, target))
        for node in sorted(self.decs):
            row = self.decs[node]
            for ref in row["requirement_keys"]:
                target, how = self.resolve("requirement", ref, node[0])
                if target is None:
                    self.errors.append(f"decision {node_str(node)}: {how} requirement reference {ref!r}")
                    continue
                if how != "local":
                    self.log["resolutions"].append(f"decision `{node_str(node)}` -> requirement `{node_str(target)}` ({how}: {ref})")
                from_dec.add((target, node))
        self.edges = from_req | from_dec
        self.log["reverse_links"] = {"requirement_side_only": len(from_req - from_dec),
                                     "decision_side_only": len(from_dec - from_req),
                                     "both": len(from_req & from_dec)}

    # ----------------------------------------------------------------- integrity
    def check_integrity(self):
        dec_links = collections.defaultdict(set)
        req_links = collections.defaultdict(set)
        for req, dec in self.edges:
            dec_links[dec].add(req)
            req_links[req].add(dec)
        self.dec_links, self.req_links = dec_links, req_links
        for node in sorted(self.decs):
            if not dec_links.get(node):
                self.errors.append(f"integrity: decision {node_str(node)} references no requirement")
        for node in sorted(self.reqs):
            if nonempty(self.reqs[node]["decision_needed"]) and not req_links.get(node):
                self.errors.append(f"integrity: requirement {node_str(node)} has decision_needed but references no decision")
        covered_sections = {s for row in self.decs.values() for s in row["program_sections"]}
        self.missing_sections = [s for s in REQUIRED_DECISION_SECTIONS if s not in covered_sections]
        for section in self.missing_sections:
            self.errors.append(f"integrity: program section {section} appears in no decision")
        for node in sorted(self.reqs):
            row = self.reqs[node]
            for entry in row["additional_sources"]:
                if entry["local_key"] and entry["local_key"] not in self.extract:
                    self.errors.append(f"requirement {node_str(node)}: unknown additional local_key {entry['local_key']}")
                if not entry["local_key"] and entry["source"] != "PROGRAM":
                    self.errors.append(f"requirement {node_str(node)}: additional source without local_key ({entry['source']})")
            if row["source"] != "PROGRAM" and not self.primary_key(row):
                self.warnings.append(f"requirement {node_str(node)}: no primary local_key resolvable from source_location")

    def coverage(self):
        referenced = collections.defaultdict(set)
        primary_refs = collections.Counter()
        for node, row in self.reqs.items():
            key = self.primary_key(row)
            if key:
                referenced[key].add(node)
                primary_refs[key] += 1
            for entry in row["additional_sources"]:
                if entry["local_key"]:
                    referenced[entry["local_key"]].add(node)
        self.referenced = referenced
        self.primary_refs = primary_refs
        self.uncovered = sorted(k for k in self.extract if k not in referenced)
        return self.uncovered

    def write_coverage_addenda(self):
        if not self.uncovered:
            return None
        path = ADDENDA / f"assembler-coverage-round{self.args.round}.jsonl"
        existing = set()
        lines = []
        if path.exists():
            for _, row in read_jsonl(path, self.errors):
                existing.add(row.get("canonical_key"))
                lines.append(json.dumps(row, ensure_ascii=False))
        kind_map = {"IMPLEMENTATION_FACT": "REQUIREMENT", "EMPIRICAL_FINDING": "CLAIM_NEEDING_EVIDENCE",
                    "NOT_IMPLEMENTED": "CAPABILITY"}
        for key in self.uncovered:
            rec = self.extract[key]
            canonical = f"coverage-{key}"
            if canonical in existing:
                continue
            hint = rec.get("implementation_hint", "")
            row = {
                "record_type": "requirement", "cluster": rec.get("primary_cluster"), "canonical_key": canonical,
                "kind": kind_map.get(rec.get("kind"), rec.get("kind")),
                "requirement_or_question": ws(rec.get("statement", "")),
                "source": rec.get("source"), "source_location": f"{rec.get('location', '')} [{key}]",
                "additional_sources": [], "superseding_authority": "",
                "supersession_note": "Assembler coverage addendum: extraction record not claimed by any merged row; review and fold into a canonical row.",
                "implementation_state": hint if hint in IMPLEMENTATION_STATES else "MISSING",
                "implementation_evidence": "none found (coverage addendum; implementation not audited)",
                "evidence_state": "UNTESTED", "research_needed": rec.get("research_needed", "") or "Review this unclaimed extraction record.",
                "decision_needed": rec.get("decision_needed", ""), "decision_keys": [],
                "release_relevance": rec.get("release_relevance", "V1_IMPORTANT"), "program_sections": [],
                "correction_reason": "mechanical coverage: uncovered extraction key",
            }
            lines.append(json.dumps(row, ensure_ascii=False))
        write_text(path, "\n".join(lines) + "\n")
        return path

    # ----------------------------------------------------------------- IDs
    def assign_ids(self):
        for record_type, registry_name, width, prefix in (("requirement", "requirements", 4, "REQ"),
                                                           ("decision", "decisions", 3, "DEC")):
            table = self.table(record_type)
            registry = self.id_map[registry_name]
            nxt = self.id_map["next"]
            rx = REQ_ID_RX if record_type == "requirement" else DEC_ID_RX
            for value in registry.values():
                match = rx.match(value)
                if match:
                    name = f"{prefix}-{match.group(1)}"
                    nxt[name] = max(nxt.get(name, 1), int(match.group(2)) + 1)
            live = sorted(node_str(n) for n in table)
            for key in live:
                if key in registry:
                    continue
                code = CLUSTER_CODES[parse_node(key)[0]]
                name = f"{prefix}-{code}"
                number = nxt.get(name, 1)
                if number >= 10 ** width:
                    self.errors.append(f"ID space exhausted for {name}")
                    continue
                registry[key] = f"{name}-{number:0{width}d}"
                nxt[name] = number + 1
                self.log["new_ids"].append(registry[key])
            aliases = self.id_map[f"{record_type}_aliases"]
            for node, (survivor, reason) in sorted(self.aliases(record_type).items()):
                final = self.follow(record_type, node)
                if final not in table:
                    continue
                aliases[node_str(node)] = {"alias_of": node_str(final), "id": registry[node_str(final)], "reason": reason}
            for alias_key, info in sorted(aliases.items()):
                if parse_node(alias_key) in table:
                    del aliases[alias_key]  # live again: no longer an alias
                    continue
                final = self.follow(record_type, parse_node(info["alias_of"]))
                if final in table:
                    info["alias_of"], info["id"] = node_str(final), registry[node_str(final)]
            retired = self.id_map["retired_ids"]
            for key, value in sorted(registry.items()):
                if rx.match(value) is None:
                    continue
                if key in aliases:
                    retired[value] = aliases[key]["id"]
                elif parse_node(key) in table:
                    retired.pop(value, None)
                else:
                    self.warnings.append(f"{value} ({key}) has no live row and is not an alias; ID stays reserved")
            ids = {node: registry[node_str(node)] for node in table if node_str(node) in registry}
            if record_type == "requirement":
                self.req_ids = ids
            else:
                self.dec_ids = ids
        self.id_map["next"] = dict(sorted(self.id_map["next"].items()))
        self.id_map["requirements"] = dict(sorted(self.id_map["requirements"].items()))
        self.id_map["decisions"] = dict(sorted(self.id_map["decisions"].items()))
        self.id_map["requirement_aliases"] = dict(sorted(self.id_map["requirement_aliases"].items()))
        self.id_map["decision_aliases"] = dict(sorted(self.id_map["decision_aliases"].items()))
        self.id_map["retired_ids"] = dict(sorted(self.id_map["retired_ids"].items()))

    # ----------------------------------------------------------------- outputs
    def visible_source_location(self, node, row):
        """source_location with the primary local_key in brackets when the ledger would not otherwise show it."""
        location = ws(row["source_location"])
        key = self.primary_key(row)
        if not key:
            return location
        in_brackets = any(key in LOCAL_KEY_RX.findall(inner) for inner in BRACKET_RX.findall(location))
        in_additional = any(entry["local_key"] == key for entry in row["additional_sources"])
        if in_brackets or in_additional:
            return location
        self.log["primary_key_visible"].append(f"`{node_str(node)}`: source_location {location!r} + [{key}] (from source_local_key)")
        return f"{location} [{key}]"

    def requirement_rows(self):
        rows = []
        self.log["primary_key_visible"] = []
        for node, row in sorted(self.reqs.items()):
            decision_ids = sorted({self.dec_ids[d] for d in self.req_links.get(node, set()) if d in self.dec_ids})
            rows.append({
                "req_id": self.req_ids[node], "cluster": node[0], "kind": row["kind"],
                "requirement_or_question": ws(row["requirement_or_question"]), "source": row["source"],
                "source_location": self.visible_source_location(node, row),
                "additional_sources": [{"source": e["source"], "location": ws(e["location"]), "local_key": e["local_key"]}
                                       for e in row["additional_sources"]],
                "superseding_authority": ws(row["superseding_authority"]), "supersession_note": ws(row["supersession_note"]),
                "implementation_state": row["implementation_state"],
                "implementation_evidence": ws(row["implementation_evidence"]), "evidence_state": row["evidence_state"],
                "research_needed": ws(row["research_needed"]), "decision_needed": ws(row["decision_needed"]),
                "decision_ids": decision_ids, "release_relevance": row["release_relevance"],
                "program_sections": sorted(set(row["program_sections"]), key=section_key),
            })
        rows.sort(key=lambda r: r["req_id"])
        return rows

    def write_requirement_csv(self, rows):
        buffer = io.StringIO()
        writer = csv.writer(buffer, lineterminator="\r\n", quoting=csv.QUOTE_MINIMAL)
        writer.writerow(CSV_COLUMNS)
        for row in rows:
            out = []
            for column in CSV_COLUMNS:
                value = row[column]
                if column == "additional_sources":
                    value = json.dumps(value, ensure_ascii=False)
                elif isinstance(value, list):
                    value = "; ".join(value)
                out.append(value)
            writer.writerow(out)
        with open(REQ_CSV, "w", encoding="utf-8", newline="") as handle:
            handle.write(buffer.getvalue())

    def read_back_csv(self):
        decoded = []
        with open(REQ_CSV, encoding="utf-8", newline="") as handle:
            reader = csv.DictReader(handle)
            if reader.fieldnames != CSV_COLUMNS:
                self.errors.append("requirement-ledger.csv header does not match the column contract")
            for row in reader:
                row = dict(row)
                try:
                    row["additional_sources"] = json.loads(row["additional_sources"])
                except json.JSONDecodeError:
                    self.errors.append(f"{row.get('req_id')}: additional_sources is not JSON")
                for column in ("decision_ids", "program_sections"):
                    row[column] = [part for part in row[column].split("; ")] if row[column] else []
                decoded.append(row)
        return decoded

    def derive_blocker(self, node, status):
        row = self.decs[node]
        if nonempty(row.get("_blocker_override")):
            return row["_blocker_override"]
        if status == "DECIDED":
            return "NONE"
        if status == "EXTERNAL_REVIEW_REQUIRED":
            return "EXTERNAL_REVIEW"
        text = " ".join([row["question"]] + [str(x) for x in row["required_evidence"]])
        if HUMAN_PARTICIPANTS_RX.search(text):
            return "HUMAN_PARTICIPANTS"
        if any(self.reqs[r]["implementation_state"] == "BLOCKED_PLATFORM" for r in self.dec_links.get(node, ())):
            return "PLATFORM"
        return "EVIDENCE"

    def linked_state_addenda(self, node):
        origins = sorted({o for r in self.dec_links.get(node, ()) for o in self.reqs[r].get("_impl_addenda", [])})
        return f"implementation_state addenda on linked requirements: {', '.join(origins)}" if origins else ""

    def decision_rows(self):
        existing = {}
        if DEC_JSONL.exists():
            for lineno, row in read_jsonl(DEC_JSONL, self.errors):
                if "decision_id" in row:
                    existing[row["decision_id"]] = row
        live_ids = {v: n for n, v in self.dec_ids.items()}
        retired = self.id_map["retired_ids"]
        absorbed = collections.defaultdict(list)
        orphans = []
        for decision_id in sorted(existing):
            if decision_id in live_ids:
                continue
            if decision_id in retired and retired[decision_id] in live_ids:
                absorbed[retired[decision_id]].append(existing[decision_id])
            else:
                orphans.append(existing[decision_id])
                self.errors.append(f"decision-ledger row {decision_id} has no live decision and no alias; row kept unchanged")
        rows = []
        for node in sorted(self.decs, key=lambda n: self.dec_ids[n]):
            dec = self.decs[node]
            decision_id = self.dec_ids[node]
            prev = existing.get(decision_id)
            derived_status = dec["initial_status"]
            row = collections.OrderedDict()
            row["decision_id"] = decision_id
            row["status"] = derived_status
            row["blocker_class"] = ""
            row["cluster"] = node[0]
            row["question"] = ws(dec["question"])
            row["requirement_ids"] = sorted({self.req_ids[r] for r in self.dec_links.get(node, set())})
            row["program_sections"] = sorted(set(dec["program_sections"]), key=section_key)
            row["release_relevance"] = dec["release_relevance"]
            row["archetypal_objective"] = ws(dec["archetypal_objective"])
            row["hard_constraints"] = [ws(x) for x in dec["hard_constraints"]]
            row["candidate_set"] = dec["candidate_set"]
            row["candidate_exclusion_reasons"] = dec["candidate_exclusion_reasons"]
            row["required_evidence"] = [ws(x) for x in dec["required_evidence"]]
            for field in DECISION_EVIDENCE_FIELDS[:15]:
                row[field] = [] if field in DECISION_LIST_EVIDENCE_FIELDS else ""
            row["external_review_requirement"] = ws(dec["external_review_requirement"])
            row["remaining_unknowns_ref"] = ""
            origin_note = f" (addendum {dec['_job'][len('addenda:'):]}.jsonl)" if str(dec.get("_job", "")).startswith("addenda:") else ""
            row["history"] = [{"date": self.date, "change": CREATION_CHANGE + origin_note}]
            if prev:
                history = prev.get("history") or row["history"]
                audit_owned = all(str(h.get("change", "")).startswith(AUDIT_HISTORY_PREFIXES) for h in history)
                for field in DECISION_EVIDENCE_FIELDS:
                    if nonempty(prev.get(field)):
                        row[field] = prev[field]
                row["history"] = list(history)
                if audit_owned:
                    new_blocker = self.derive_blocker(node, derived_status)
                    if prev.get("status") != derived_status or prev.get("blocker_class") != new_blocker:
                        why = dec.get("_status_addendum") or self.linked_state_addenda(node) or "reassembled inputs"
                        row["history"].append({"date": self.date, "change":
                            f"assembler: status {prev.get('status')}/{prev.get('blocker_class')} -> {derived_status}/{new_blocker} ({why})"})
                    row["status"], row["blocker_class"] = derived_status, new_blocker
                else:
                    row["status"] = prev.get("status", derived_status)
                    row["blocker_class"] = prev.get("blocker_class") or self.derive_blocker(node, row["status"])
                    if prev.get("status") != derived_status:
                        self.warnings.append(f"{decision_id}: kept ledger status {prev.get('status')} (row owned by a later phase); inputs say {derived_status}")
            else:
                row["blocker_class"] = self.derive_blocker(node, derived_status)
            for old in absorbed.get(decision_id, []):
                for field in DECISION_EVIDENCE_FIELDS:
                    if not nonempty(old.get(field)):
                        continue
                    if field in DECISION_LIST_EVIDENCE_FIELDS:
                        row[field] = union_strings(row[field], old[field])
                    else:
                        row[field] = join_text(row[field], old[field])
                row["history"].append({"date": self.date, "change": f"assembler: absorbed {old['decision_id']} (merged into this decision)"})
                self.log["absorbed_ledger_rows"].append(f"{old['decision_id']} -> {decision_id}")
            rows.append(row)
        rows.extend(orphans)
        rows.sort(key=lambda r: r["decision_id"])
        return rows

    def write_decision_ledger(self, rows):
        text = "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in rows)
        write_text(DEC_JSONL, text)

    # ----------------------------------------------------------------- reports
    def write_supersession(self, req_rows):
        lines = ["# Supersession ledger", "",
                 "Generated by `research/tools/ledger/assemble.py`; do not edit by hand. One row per requirement whose",
                 "authority was frozen, refined, contradicted or superseded by a later authority. Precedence used by",
                 "the audit: repository docs and code at `9e44608` supersede the SPEC where they freeze or refine it;",
                 "the SPEC supersedes Research I-III and the research appendix; PROGRAM rows add research obligations",
                 "only and never govern product semantics. *Governing authority* is the first non-PROGRAM superseding",
                 "authority, or the original authority when only PROGRAM obligations were added.", ""]
        by_cluster = collections.defaultdict(list)
        for row in req_rows:
            authority = row["superseding_authority"].strip()
            if authority and authority.lower() not in ("none", "n/a", "-"):
                by_cluster[row["cluster"]].append(row)
        total = sum(len(v) for v in by_cluster.values())
        lines.append(f"Rows: {total} of {len(req_rows)} requirements.")
        lines.append("")
        for cluster in sorted(CLUSTER_CODES):
            rows = by_cluster.get(cluster, [])
            lines.append(f"## {cluster} ({CLUSTER_CODES[cluster]}, {len(rows)} rows)")
            lines.append("")
            if not rows:
                lines.append("No supersession recorded.")
                lines.append("")
                continue
            lines.append("| req_id | original authority@location | superseding authority@location | nature of change | governing authority |")
            lines.append("|---|---|---|---|---|")
            for row in rows:
                parts = [p.strip() for p in row["superseding_authority"].split(";") if p.strip()]
                governing = next((p for p in parts if not p.upper().startswith("PROGRAM")), None)
                if governing is None:
                    governing = f"{row['source']}@{strip_key_brackets(row['source_location'])}"
                nature = self.nature_of_change(row)
                lines.append(f"| {row['req_id']} | {md_cell(row['source'] + '@' + row['source_location'])} | "
                             f"{md_cell(row['superseding_authority'])} | {md_cell(nature)} | {md_cell(governing)} |")
            lines.append("")
        write_text(SUPERSESSION, "\n".join(lines))

    @staticmethod
    def nature_of_change(row):
        note = row["supersession_note"]
        text = f"{note} {row['superseding_authority']}".lower()
        labels = []
        for label, rx in (("contradiction/divergence", r"contradict|conflict|diverg|inconsisten|disagree|mismatch|drift"),
                          ("supersedes", r"supersed|replac|obsolet|retir"),
                          ("freezes/refines", r"freez|frozen|refin|concret|specif|pins?\b|pinned|defin|answer"),
                          ("extends", r"\badd(?:s|ed|ing)?\b|extend|introduc"),
                          ("narrows/defers", r"defer|narrow|restrict|out of v1|not implemented|drop"),
                          ("implementation", r"implement|code\b|code:"),
                          ("research obligation", r"program")):
            if re.search(rx, text):
                labels.append(label)
        prefix = "+".join(labels) if labels else "authority change"
        return f"{prefix}: {note}" if note else prefix

    def write_coverage_report(self, req_rows):
        per_slice = collections.defaultdict(lambda: [0, 0])
        for key in self.extract:
            slot = per_slice[self.extract_slice[key]]
            slot[0] += 1
            if key in self.referenced:
                slot[1] += 1
        multi = sorted((k, v) for k, v in self.referenced.items() if len(v) > 1)
        lines = ["# Extraction-key coverage report", "",
                 "Generated by `research/tools/ledger/assemble.py`. A key is covered when at least one requirement",
                 "row references it as its primary key (bracketed key in `source_location`, or the `additional_sources`",
                 "entry matching `source_location`) or in `additional_sources`. The assembler checks that every covered",
                 "key is visible in `research/requirement-ledger.csv` itself (a bracket in `source_location` or an",
                 "`additional_sources` entry).", "",
                 f"- Extraction records: {len(self.extract)}",
                 f"- Covered: {len(self.extract) - len(self.uncovered)}",
                 f"- Uncovered: {len(self.uncovered)}",
                 f"- Keys referenced as a primary key: {len(self.primary_refs)}",
                 f"- Keys referenced by more than one requirement row: {len(multi)}", "",
                 "## Per extraction slice", "", "| slice | records | covered | uncovered |", "|---|---:|---:|---:|"]
        for name in sorted(per_slice):
            total, covered = per_slice[name]
            lines.append(f"| {name} | {total} | {covered} | {total - covered} |")
        lines += ["", "## Uncovered extraction keys", ""]
        if self.uncovered:
            lines += [f"- `{k}` ({self.extract[k].get('source')}@{md_cell(self.extract[k].get('location'))})" for k in self.uncovered]
        else:
            lines.append("None.")
        lines += ["", "## Keys referenced by more than one requirement row", "",
                  "Shared supporting references are expected (one source statement can support several requirements);",
                  "rows are merged only when they share a primary key across clusters.", ""]
        if multi:
            lines += ["| local_key | requirement rows |", "|---|---|"]
            for key, nodes in multi:
                lines.append(f"| {key} | {', '.join(sorted(self.req_ids[n] for n in nodes))} |")
        else:
            lines.append("None.")
        lines.append("")
        write_text(COVERAGE_REPORT, "\n".join(lines))

    def write_assembly_log(self, req_rows, dec_rows, unadjudicated):
        L = ["# Ledger assembly log", "",
             f"Generated by `research/tools/ledger/assemble.py` (round {self.args.round}, assembly date {self.date}).",
             "Every merge, key collision, enumeration fix, addendum application, cross-cluster reference resolution",
             "and integrity result of the latest run is recorded here. The log is regenerated on every run from the",
             "inputs listed below, so it always describes the current ledgers.", "",
             "## Inputs", "", "| input | rows or bytes | SHA-256 |", "|---|---:|---|"]
        for path in sorted(set(self.inputs), key=lambda p: rel(p)):
            data = pathlib.Path(path).read_bytes()
            count = sum(1 for line in data.splitlines() if line.strip()) if path.suffix == ".jsonl" else len(data)
            L.append(f"| `{rel(path)}` | {count} | `{sha256_bytes(data)}` |")
        L += ["", "## Result", "",
              f"- Requirement rows: {len(req_rows)}; decision rows: {len(dec_rows)}",
              f"- Uncovered extraction keys: {len(self.uncovered)}",
              f"- Validation errors: {len(self.errors)}; warnings: {len(self.warnings)}",
              f"- Stable IDs registered: {len(self.id_map['requirements'])} requirement, {len(self.id_map['decisions'])} decision; "
              f"aliases: {len(self.id_map['requirement_aliases'])} requirement, {len(self.id_map['decision_aliases'])} decision", ""]
        L += ["## Enumeration and format fixes", ""]
        L += [f"- {x}" for x in self.log["enum_fixes"]] or ["None."]
        L += ["", "## Primary keys made visible in source_location", "",
              "Rows whose primary extraction key was carried only in the merge-input field `source_local_key` (not a",
              "ledger column) get it appended in brackets to `source_location`, so coverage is checkable from the CSV alone.", ""]
        L += [f"- {x}" for x in self.log["primary_key_visible"]] or ["None."]
        L += ["", "## Key collisions within a cluster", ""]
        collision_merges = [m for m in self.log["merges"] if m["stage"] == "key-collision"]
        L += [f"- {x}" for x in self.log["collisions"]]
        L += [f"- merged {m['record_type']} `{node_str(m['merged'])}` from {m['merged_origin']} into {m['survivor_origin']} ({m['reason']})"
              for m in collision_merges]
        if not self.log["collisions"] and not collision_merges:
            L.append("None.")
        for record_type in ("requirement", "decision"):
            merges = [m for m in self.log["merges"] if m["record_type"] == record_type and m["stage"].startswith("near-duplicate")]
            L += ["", f"## Near-duplicate {record_type} merges across merge jobs", "",
                  f"Candidates: {self.log.get('candidates_' + record_type, 0)} pairs at TF-IDF cosine >= {self.review_threshold:.2f} "
                  f"between rows of different merge jobs in one cluster, plus {self.log.get('candidates_addenda_' + record_type, 0)} "
                  f"pairs involving a row added by addenda (scored after addenda are applied); adjudications in "
                  f"`research/tools/ledger/near-duplicates.json`; unadjudicated pairs at >= {self.auto_threshold:.2f} merge automatically.", ""]
            if merges:
                ids = self.req_ids if record_type == "requirement" else self.dec_ids
                L += ["| survivor | merged-away key | stage | cosine | reason | merged-away text |", "|---|---|---|---:|---|---|"]
                for m in merges:
                    score = f"{m['score']:.3f}" if m["score"] is not None else "-"
                    L.append(f"| {ids.get(m['survivor'], '?')} `{node_str(m['survivor'])}` | `{node_str(m['merged'])}` "
                             f"({m['merged_origin']}) | {m['stage']} | {score} | {md_cell(m['reason'])} | {md_cell(m['merged_text'])} |")
            else:
                L.append("None.")
            pending = self.log.get(f"unadjudicated_{record_type}", []) + self.log.get(f"unadjudicated_addenda_{record_type}", [])
            L += ["", f"Unadjudicated {record_type} candidates kept distinct: {len(pending)}", ""]
            L += [f"- {s:.3f} `{node_str(a)}` <> `{node_str(b)}`" for a, b, s in pending]
        L += ["", "## Addenda applied", ""]
        if self.log["addenda_updates"] or self.log["addenda_additions"]:
            for item in self.log["addenda_additions"]:
                L.append(f"- added {item['record_type']} `{node_str(item['node'])}` from {item['origin']}: {item['reason'] or '(no reason given)'}")
            for item in self.log["addenda_updates"]:
                redirect = " (redirected from alias)" if item["redirected"] else ""
                fields = ", ".join(item["fields"]) or "no field changed"
                verb = "corrected" if item["reason"] else "updated"
                L.append(f"- {verb} {item['record_type']} `{node_str(item['node'])}`{redirect} from {item['origin']} "
                         f"[{fields}]: {item['reason'] or '(no correction_reason)'}")
                for field, lost in sorted(item.get("dropped", {}).items()):
                    L.append(f"  - replaced `{field}` dropped {len(lost)} item(s): " + "; ".join(md_cell(x) for x in lost))
        else:
            L.append("None.")
        L += ["", "## Cross-cluster primary-key dedupe", ""]
        dedupes = [m for m in self.log["merges"] if m["stage"] == "cross-cluster primary-key dedupe"]
        L += [f"- `{node_str(m['merged'])}` merged into {self.req_ids.get(m['survivor'], '?')} `{node_str(m['survivor'])}`: {m['reason']}"
              for m in dedupes] or ["None."]
        L += ["", "## Merge field conflicts", ""]
        L += [f"- {x}" for x in self.log["merge_conflicts"]] or ["None."]
        L += ["", "## Reference resolution", "",
              f"- Requirement-decision links declared on both sides: {self.log['reverse_links']['both']}",
              f"- Declared only by the requirement (added to the decision's requirement_ids): {self.log['reverse_links']['requirement_side_only']}",
              f"- Declared only by the decision (added to the requirement's decision_ids): {self.log['reverse_links']['decision_side_only']}",
              f"- Non-local resolutions (cross-cluster, alias or qualified): {len(self.log['resolutions'])}", ""]
        L += [f"- {x}" for x in self.log["resolutions"]]
        L += ["", "## Absorbed decision-ledger rows", ""]
        L += [f"- {x}" for x in self.log["absorbed_ledger_rows"]] or ["None."]
        L += ["", "## Validation errors", ""]
        L += [f"- {x}" for x in self.errors] or ["None."]
        L += ["", "## Warnings", ""]
        L += [f"- {x}" for x in self.warnings] or ["None."]
        L.append("")
        write_text(ASSEMBLY_LOG, "\n".join(L))

    # ----------------------------------------------------------------- source inventory
    def write_source_inventory(self, req_rows):
        external_root = pathlib.Path(self.args.external_root)
        sources_root = pathlib.Path(self.args.sources_root)
        appendix_dir, r3_dir = sources_root / "appendix", sources_root / "r3-instrumentation"
        appendix_members = sorted(p.relative_to(appendix_dir).as_posix() for p in appendix_dir.rglob("*") if p.is_file()) if appendix_dir.exists() else []
        r3_members = sorted(p.relative_to(r3_dir).as_posix() for p in r3_dir.rglob("*") if p.is_file()) if r3_dir.exists() else []
        if not appendix_members:
            self.warnings.append(f"appendix members not found under {appendix_dir}")
        if not r3_members:
            self.warnings.append(f"R3 instrumentation members not found under {r3_dir}")

        def r3_resolve(location):
            found = set()
            for token in re.findall(r"[A-Za-z0-9_./-]+\.(?:py|json|java|go|mod|tar|zip|txt|md)\b", location):
                token = token.lstrip("./")
                matches = [m for m in r3_members if m == token or m.endswith("/" + token)]
                if len(matches) == 1:
                    found.add(matches[0])
            return found

        def ids_for(source, location):
            source = (source or "").strip()
            location = location or ""
            if source in ("SPEC", "R1", "R2", "R3"):
                return {f"ext:{source}"}
            if source.startswith("APPX/"):
                return {f"appx:{source[5:]}", "archive:APPX"}
            if source == "R3-INSTR":
                return {"archive:R3-INSTR"} | {f"r3i:{m}" for m in r3_resolve(location)}
            if source.startswith("docs/"):
                return {f"repo:{source}"}
            if source == "docs":
                token = location.split()[0] if location.startswith("docs/") else "docs"
                return {f"repo:{token}"}
            if source.startswith("code:"):
                return {f"repo:{source[5:]}"}
            if source == "PROGRAM":
                return {"PROGRAM"}
            return {f"unknown:{source}"}

        slices = collections.defaultdict(collections.Counter)
        for key, rec in self.extract.items():
            for sid in ids_for(rec.get("source"), rec.get("location")):
                slices[sid][self.extract_slice[key]] += 1
        citing = collections.defaultdict(set)
        for row in req_rows:
            pairs = [(row["source"], row["source_location"])] + [(e["source"], e["location"]) for e in row["additional_sources"]]
            for source, location in pairs:
                for sid in ids_for(source, location):
                    citing[sid].add(row["req_id"])

        def slice_text(sid):
            counter = slices.get(sid)
            if not counter:
                return ""
            return ", ".join(f"{name} ({count})" for name, count in sorted(counter.items()))

        known = set()
        sections = []
        external = [
            ("ext:SPEC", "design/2026-08-29-entrybound-product-architecture.md", "Product Architecture Specification v1.2 (SPEC; normative product architecture)", 2),
            ("ext:R1", "research/2026-08-29-archive-category-research.md", "Research I: archive category research (evidence base)", 3),
            ("ext:R2", "research/2026-08-29-entrybound-opportunity-validation.md", "Research II: opportunity validation (evidence base)", 3),
            ("ext:R3", "research/2026-08-29-entrybound-research-iii-final-gate.md", "Research III: final gate (evidence base)", 3),
            ("archive:APPX", "design/research-appendix/entrybound-architecture-research-appendix.tgz", "Architecture research appendix archive (members listed below)", 3),
            ("archive:R3-INSTR", "research/entrybound-research-iii-instrumentation.tgz", "Research III instrumentation archive (members listed below)", 3),
        ]
        table = []
        for sid, path, role, rank in external:
            known.add(sid)
            full = external_root / path
            if full.exists():
                data = full.read_bytes()
                digest, size, lines = sha256_bytes(data), len(data), line_count(data)
                expected = KNOWN_EXTERNAL_SHA256.get(path)
                if expected and expected != digest:
                    self.errors.append(f"external input {path} SHA-256 {digest} differs from recorded {expected}")
            else:
                digest, size, lines = "UNAVAILABLE", "", ""
                self.warnings.append(f"external input not found: {full}")
            if path.endswith(".tgz"):
                lines = "binary"
            table.append((path, digest, size, lines, role, rank, slice_text(sid), len(citing.get(sid, ()))))
        sections.append(("External unpublished inputs (outside the repository; paths relative to the parent directory `entrybound/`)", table))

        for title, base, members, prefix, role_fn in (
                ("Research appendix members (extracted from the appendix archive to `eb-research/sources/appendix`)",
                 appendix_dir, appendix_members, "appx", lambda m: "Research appendix index" if m == "README.md" else f"Research appendix: {pathlib.PurePosixPath(m).stem}"),
                ("Research III instrumentation members (extracted to `eb-research/sources/r3-instrumentation`)",
                 r3_dir, r3_members, "r3i", self.r3_role)):
            rows = []
            for member in members:
                sid = f"{prefix}:{member}"
                known.add(sid)
                data = (base / member).read_bytes()
                rows.append((member, sha256_bytes(data), len(data), line_count(data), role_fn(member), 3,
                             slice_text(sid), len(citing.get(sid, ()))))
            sections.append((title, rows))

        repo_rows = []
        try:
            commit = subprocess.run(["git", "-C", str(REPO), "rev-parse", "--verify", f"{BASELINE_COMMIT}^{{commit}}"],
                                    capture_output=True, text=True, check=True).stdout.strip()
            listing = subprocess.run(["git", "-C", str(REPO), "ls-tree", "-r", "-z", "--full-tree", commit],
                                     capture_output=True, check=True).stdout.decode("utf-8")
            entries = []
            for item in listing.split("\0"):
                if not item:
                    continue
                meta, path = item.split("\t", 1)
                _, kind, oid = meta.split()
                if kind == "blob":
                    entries.append((path, oid))
            batch = subprocess.run(["git", "-C", str(REPO), "cat-file", "--batch"],
                                   input="".join(oid + "\n" for _, oid in entries).encode(), capture_output=True, check=True).stdout
            offset = 0
            for path, oid in entries:
                header_end = batch.index(b"\n", offset)
                size = int(batch[offset:header_end].split()[2])
                data = batch[header_end + 1: header_end + 1 + size]
                offset = header_end + 1 + size + 1
                sid = f"repo:{path}"
                known.add(sid)
                repo_rows.append((path, sha256_bytes(data), size, line_count(data), self.repo_role(path), 1,
                                  slice_text(sid), len(citing.get(sid, ()))))
            repo_rows.sort()
        except (subprocess.CalledProcessError, FileNotFoundError, ValueError, IndexError) as exc:
            commit = BASELINE_COMMIT
            self.warnings.append(f"could not read repository tree at {BASELINE_COMMIT}: {exc}")
        sections.append((f"Repository files at `{BASELINE_COMMIT}` (commit `{commit}`): docs, crate sources and tests, tools, Cargo manifests", repo_rows))

        unresolved = sorted(sid for sid in set(slices) | set(citing) if sid not in known and sid != "PROGRAM")
        total_files = sum(len(rows) for _, rows in sections)
        L = ["# Source inventory", "",
             "Generated by `research/tools/ledger/assemble.py`; do not edit by hand.", "",
             "**The Product Architecture Specification (SPEC) and Research I-III, together with the research appendix and",
             "the Research III instrumentation archives, are external unpublished inputs. They are referenced here by",
             "SHA-256 and are not copied into this repository.** Ledger rows paraphrase and cite them by section and line.", "",
             "Authority rank (1 = highest): **1** repository docs, code, tests, tools and manifests at the research baseline",
             f"`{BASELINE_COMMIT}` (they supersede the SPEC where they freeze or refine it); **2** the SPEC (supersedes Research",
             "I-III and the appendix); **3** Research I-III, the research appendix and the Research III instrumentation",
             "(evidence base). PROGRAM (the research program brief) is not a file input: it adds research obligations only",
             f"and is cited by {len(citing.get('PROGRAM', ()))} requirement rows.", "",
             "*Extraction slice keys* lists the `research/audit/extract` slices whose records cite the source, with record",
             "counts. *Citing rows* counts requirement-ledger rows that cite the source as primary or additional source.",
             "Lines is the newline count (a final unterminated line counts); binary files show `binary`.", "",
             f"Files inventoried: {total_files}.", ""]
        for title, rows in sections:
            L += [f"## {title}", "", "| path | SHA-256 | bytes | lines | role | authority rank | extraction slice keys | citing rows |",
                  "|---|---|---:|---:|---|---:|---|---:|"]
            for path, digest, size, lines, role, rank, slice_keys, cites in rows:
                L.append(f"| `{path}` | `{digest}` | {size} | {lines} | {md_cell(role)} | {rank} | {md_cell(slice_keys)} | {cites} |")
            if not rows:
                L.append("| (unavailable) | | | | | | | |")
            L.append("")
        L += ["## Unresolved source references", "",
              "Sources named by extraction records or ledger rows that do not resolve to an inventoried file.", ""]
        if unresolved:
            L += ["| source reference | extraction slice keys | citing rows |", "|---|---|---:|"]
            for sid in unresolved:
                L.append(f"| `{sid}` | {md_cell(slice_text(sid))} | {len(citing.get(sid, ()))} |")
        else:
            L.append("None.")
        L.append("")
        write_text(INVENTORY, "\n".join(L))
        self.unresolved_sources = unresolved

    @staticmethod
    def r3_role(member):
        if member.startswith("out/"):
            return "recorded instrumentation output"
        if member.endswith("manifest.json"):
            return "adversarial case manifest"
        if member.startswith(("cases/", "cases2/", "malo/")):
            return "adversarial test case"
        if member.startswith("diff/"):
            return "differential harness driver"
        return "instrumentation script"

    @staticmethod
    def repo_role(path):
        parts = path.split("/")
        if path.startswith("docs/"):
            return "repo test vectors" if path.endswith(".txt") else "repo normative design doc"
        if parts[0] == "crates" and len(parts) > 2:
            crate = parts[1]
            if parts[2] == "Cargo.toml":
                return f"Cargo manifest ({crate})"
            if parts[2] == "tests":
                return f"crate test data ({crate})" if "data" in parts[3:-1] else f"crate test ({crate})"
            if parts[2] == "src":
                return f"crate source ({crate})"
            return f"crate file ({crate})"
        if parts[0] == "tools":
            if parts[-1] in ("Cargo.toml", "Cargo.lock"):
                return f"Cargo manifest (tool {parts[1]})"
            return f"repo tool ({parts[1]})"
        if path == "Cargo.toml":
            return "Cargo manifest (workspace)"
        if path == "Cargo.lock":
            return "Cargo lockfile (workspace)"
        if path == "rust-toolchain.toml":
            return "toolchain pin"
        return "repository metadata"

    # ----------------------------------------------------------------- run
    def run(self):
        self.load_extract()
        self.load_id_map()
        self.load_near_duplicates()
        self.load_merged()
        self.near_duplicates("requirement")
        self.near_duplicates("decision")
        self.apply_addenda()
        for record_type in ("requirement", "decision"):
            added = {item["node"] for item in self.log["addenda_additions"]
                     if item["record_type"] == record_type and item["node"] in self.table(record_type)}
            self.near_duplicates(record_type, added=added)
        self.dedupe_primary_keys()
        self.build_links()
        self.check_integrity()
        self.coverage()
        if self.uncovered and self.args.write_coverage_addenda:
            path = self.write_coverage_addenda()
            self.warnings.append(f"wrote coverage addendum rows to {rel(path)}; review them and rerun")
        self.assign_ids()
        req_schema, dec_schema = build_schemas()
        write_text(REQ_SCHEMA, json.dumps(req_schema, indent=2, ensure_ascii=False) + "\n")
        write_text(DEC_SCHEMA, json.dumps(dec_schema, indent=2, ensure_ascii=False) + "\n")
        req_rows = self.requirement_rows()
        self.write_requirement_csv(req_rows)
        visible_keys = set()
        for row in self.read_back_csv():
            for problem in schema_validate(row, req_schema):
                self.errors.append(f"requirement-ledger.csv {row.get('req_id')}: {problem}")
            for inner in BRACKET_RX.findall(row["source_location"]):
                visible_keys.update(LOCAL_KEY_RX.findall(inner))
            if isinstance(row["additional_sources"], list):
                visible_keys.update(e.get("local_key") for e in row["additional_sources"] if isinstance(e, dict))
        for key in sorted(set(self.extract) - visible_keys):
            if key in self.referenced:
                self.errors.append(f"coverage: extraction key {key} is covered but not visible in requirement-ledger.csv")
        dec_rows = self.decision_rows()
        for row in dec_rows:
            for problem in schema_validate(row, dec_schema):
                self.errors.append(f"decision-ledger.jsonl {row.get('decision_id')}: {problem}")
        self.write_decision_ledger(dec_rows)
        write_text(ID_MAP, json.dumps(self.id_map, indent=2, ensure_ascii=False, sort_keys=False) + "\n")
        self.write_supersession(req_rows)
        self.write_coverage_report(req_rows)
        if not self.args.skip_inventory:
            self.write_source_inventory(req_rows)
        self.write_assembly_log(req_rows, dec_rows, None)
        summary = {
            "requirement_rows": len(req_rows),
            "decision_rows": len(dec_rows),
            "uncovered_extraction_keys": len(self.uncovered),
            "validation_errors": self.errors,
            "warnings": len(self.warnings),
            "counts_by_implementation_state": dict(sorted(collections.Counter(r["implementation_state"] for r in req_rows).items())),
            "counts_by_evidence_state": dict(sorted(collections.Counter(r["evidence_state"] for r in req_rows).items())),
            "counts_by_status": dict(sorted(collections.Counter(r["status"] for r in dec_rows).items())),
            "counts_by_blocker_class": dict(sorted(collections.Counter(r["blocker_class"] for r in dec_rows).items())),
            "merges": dict(sorted(collections.Counter(f"{m['record_type']}:{m['stage']}" for m in self.log["merges"]).items())),
        }
        return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--round", type=int, default=0, help="assembly round (names generated coverage addenda)")
    parser.add_argument("--date", default=dt.date.today().isoformat(), help="history date for new ledger events (YYYY-MM-DD)")
    parser.add_argument("--external-root", default=DEFAULT_EXTERNAL_ROOT, help="parent directory holding design/ and research/ inputs")
    parser.add_argument("--sources-root", default=DEFAULT_SOURCES_ROOT, help="directory with extracted appendix and r3-instrumentation")
    parser.add_argument("--write-coverage-addenda", action="store_true",
                        help="write draft addendum rows for uncovered extraction keys (review before rerunning)")
    parser.add_argument("--skip-inventory", action="store_true", help="do not regenerate source-inventory.md")
    args = parser.parse_args()
    if not re.match(r"^\d{4}-\d{2}-\d{2}$", args.date):
        parser.error("--date must be YYYY-MM-DD")
    summary = Assembler(args).run()
    sys.stdout.reconfigure(encoding="utf-8")
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    return 1 if summary["validation_errors"] else 0


if __name__ == "__main__":
    sys.exit(main())
