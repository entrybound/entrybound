"""Experiment spec schema (YAML or JSON), validation and the held-out guard.

Minimal example (YAML)::

    schema: ebr.spec.v1
    experiment_id: EXP-EXAMPLE-001
    question: Does gzip -9 shrink text meaningfully vs -6?
    hypothesis: ...
    decision_ids: [DEC-001]
    candidates:
      - name: gzip-6
        params: {level: 6}
      - name: gzip-9
        params: {level: 9}
    command: "gzip -{level} -c -n {item_path} > {scratch}/out.gz"
    corpus: {splits: [dev], families: [text], manifest: /root/eb-research/corpus/manifest.jsonl}
    metrics:
      - {name: wall_s, source: resource, family: time_wall, direction: minimize, unit: s}
      - {name: output_bytes, source: path_size, path: "{scratch}/out.gz", family: size, direction: minimize, unit: B}
    repetitions: 5
    warmups: 1
    seeds: {order: 1, bootstrap: 2}
    timing: true
    platform: {os: [linux], arch: [x86_64], requires: [gzip]}

Placeholders in command templates: ``{item_path} {item_id} {split} {family} {scratch}
{candidate} {rep} {seed} {input_bytes}`` plus every candidate ``params`` key. In
string commands substituted values are shell-quoted automatically; do not quote
placeholders yourself.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import string
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Union

import yaml

from . import SPEC_SCHEMA
from .thresholds import METRIC_FAMILIES

ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")
HEX_RE = re.compile(r"^[0-9a-fA-F]{7,64}$")
HELDOUT_SPLIT = "heldout"

BUILTIN_PLACEHOLDERS = frozenset(
    {"item_path", "item_id", "split", "family", "scratch", "candidate", "rep", "seed", "input_bytes"}
)

RESOURCE_METRICS = frozenset(
    {
        "wall_s",
        "user_s",
        "sys_s",
        "cpu_s",
        "max_rss_bytes",
        "fs_inputs",
        "fs_outputs",
        "major_faults",
        "minor_faults",
        "voluntary_ctx",
        "involuntary_ctx",
        "timev_elapsed_s",
        "tree_peak_rss_sum_bytes",
        "io_read_bytes",
        "io_write_bytes",
    }
)
SCRATCH_METRICS = frozenset({"scratch_peak_bytes", "scratch_peak_alloc_bytes", "scratch_final_bytes"})
INPUT_METRICS = frozenset({"input_bytes", "input_files"})
METRIC_SOURCES = ("resource", "scratch", "input", "path_size", "path_sha256", "check", "stdout_json", "derived")
DERIVED_OPS = ("ratio", "difference", "sum", "product", "rate")
GENERATORS = ("text-v1", "random-v1", "zeros-v1")
ORDERS = ("randomized_blocks", "randomized_rounds", "sequential")
PAIRINGS = ("item", "item_repetition")


class SpecError(ValueError):
    pass


class HeldoutLocked(PermissionError):
    pass


Command = Union[str, List[str]]


@dataclass
class Candidate:
    name: str
    params: Dict[str, Any]
    command: Command
    env: Dict[str, str]
    cwd: Optional[str]


@dataclass
class GeneratedItem:
    item_id: str
    generator: str
    bytes: int
    seed: int
    family: Optional[str]
    sha256: Optional[str]


@dataclass
class CorpusSelector:
    splits: List[str]
    item_ids: List[str]
    families: List[str]
    manifest: Optional[str]
    root: Optional[str]
    generated: List[GeneratedItem]

    @property
    def wants_heldout(self) -> bool:
        return HELDOUT_SPLIT in self.splits


@dataclass
class Metric:
    name: str
    source: str
    family: str
    direction: str  # minimize | maximize | none
    unit: Optional[str]
    kind: str  # numeric | string
    path: Optional[str] = None
    command: Optional[Command] = None
    key: Optional[str] = None
    op: Optional[str] = None
    args: List[str] = field(default_factory=list)


@dataclass
class Seeds:
    order: int
    bootstrap: int
    command: int


@dataclass
class GuardConfig:
    max_loadavg_1m: Optional[float]
    max_other_cpu_percent: Optional[float]
    sample_interval_s: float
    max_wait_s: float


@dataclass
class Platform:
    os: List[str]
    arch: List[str]
    emulated: bool
    min_cpus: int
    cpu_affinity: Optional[List[int]]
    scratch_root: Optional[str]
    requires: List[str]
    poll_interval_s: float
    timeout_s: Optional[float]


@dataclass
class Analysis:
    baseline: Optional[str]
    pairing: str
    pareto_objectives: List[str]
    report_metrics: List[str]
    sensitivity: Optional[Dict[str, Any]]


@dataclass
class Spec:
    experiment_id: str
    question: str
    hypothesis: str
    decision_ids: List[str]
    candidates: List[Candidate]
    corpus: CorpusSelector
    metrics: List[Metric]
    repetitions: int
    warmups: int
    seeds: Seeds
    timing: bool
    platform: Platform
    guard: GuardConfig
    analysis: Analysis
    order: str
    raw_compression: str
    record_warmups: bool
    thresholds_override: Optional[Dict[str, Any]]
    shell: str
    raw: Dict[str, Any]
    path: Optional[str]
    sha256: str

    def metric(self, name: str) -> Metric:
        for m in self.metrics:
            if m.name == name:
                return m
        raise KeyError(name)

    def candidate(self, name: str) -> Candidate:
        for c in self.candidates:
            if c.name == name:
                return c
        raise KeyError(name)


# ----------------------------------------------------------------------------
# loading


def load_spec(path: Union[str, os.PathLike]) -> Spec:
    p = Path(path)
    raw_bytes = p.read_bytes()
    text = raw_bytes.decode("utf-8")
    if p.suffix.lower() == ".json":
        data = json.loads(text)
    else:
        data = yaml.safe_load(text)
    if not isinstance(data, dict):
        raise SpecError(f"{p}: top level must be a mapping")
    return parse_spec(data, path=str(p), sha256=hashlib.sha256(raw_bytes).hexdigest())


def _req(d: Dict[str, Any], key: str, where: str) -> Any:
    if key not in d or d[key] is None:
        raise SpecError(f"{where}: missing required field '{key}'")
    return d[key]


def _str_list(v: Any, where: str) -> List[str]:
    if v is None:
        return []
    if isinstance(v, str):
        return [v]
    if not isinstance(v, list) or not all(isinstance(x, str) for x in v):
        raise SpecError(f"{where}: must be a list of strings")
    return list(v)


def _int(v: Any, where: str, minimum: Optional[int] = None) -> int:
    if isinstance(v, bool) or not isinstance(v, int):
        raise SpecError(f"{where}: must be an integer")
    if minimum is not None and v < minimum:
        raise SpecError(f"{where}: must be >= {minimum}")
    return v


def _opt_float(v: Any, where: str) -> Optional[float]:
    if v is None:
        return None
    if isinstance(v, bool) or not isinstance(v, (int, float)):
        raise SpecError(f"{where}: must be a number or null")
    return float(v)


def _command(v: Any, where: str) -> Command:
    if isinstance(v, str) and v.strip():
        return v
    if isinstance(v, list) and v and all(isinstance(x, (str, int, float)) for x in v):
        return [str(x) for x in v]
    raise SpecError(f"{where}: command must be a non-empty string or list of strings")


def template_fields(cmd: Command) -> List[str]:
    parts = [cmd] if isinstance(cmd, str) else list(cmd)
    names = []
    for part in parts:
        for _, fname, _, _ in string.Formatter().parse(part):
            if fname is None:
                continue
            if fname == "" or not re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", fname):
                raise SpecError(f"unsupported placeholder '{{{fname}}}' in {part!r} (use named fields only)")
            names.append(fname)
    return names


def parse_spec(data: Dict[str, Any], path: Optional[str] = None, sha256: Optional[str] = None) -> Spec:
    where = path or "<spec>"
    schema = data.get("schema", SPEC_SCHEMA)
    if schema != SPEC_SCHEMA:
        raise SpecError(f"{where}: schema must be {SPEC_SCHEMA}")
    known = {
        "schema", "experiment_id", "question", "hypothesis", "decision_ids", "candidates", "command",
        "corpus", "metrics", "repetitions", "warmups", "seeds", "timing", "platform", "guard", "analysis",
        "order", "raw_compression", "record_warmups", "thresholds_override", "shell", "status", "notes",
        "env", "cwd",
    }
    unknown = set(data) - known
    if unknown:
        raise SpecError(f"{where}: unknown top-level fields {sorted(unknown)}")

    exp_id = _req(data, "experiment_id", where)
    if not isinstance(exp_id, str) or not ID_RE.match(exp_id):
        raise SpecError(f"{where}: experiment_id must match {ID_RE.pattern}")
    question = _req(data, "question", where)
    hypothesis = _req(data, "hypothesis", where)
    if not isinstance(question, str) or not isinstance(hypothesis, str):
        raise SpecError(f"{where}: question and hypothesis must be strings")
    decision_ids = _str_list(data.get("decision_ids", []), f"{where}.decision_ids")

    shell = data.get("shell", "bash")
    if shell not in ("bash", "sh", "cmd", "pwsh"):
        raise SpecError(f"{where}.shell: must be bash, sh, cmd or pwsh")

    default_cmd = data.get("command")
    default_env = data.get("env") or {}
    cands_raw = _req(data, "candidates", where)
    if not isinstance(cands_raw, list) or not cands_raw:
        raise SpecError(f"{where}.candidates: must be a non-empty list")
    candidates: List[Candidate] = []
    for i, c in enumerate(cands_raw):
        cw = f"{where}.candidates[{i}]"
        if not isinstance(c, dict):
            raise SpecError(f"{cw}: must be a mapping")
        name = _req(c, "name", cw)
        if not isinstance(name, str) or not ID_RE.match(name):
            raise SpecError(f"{cw}.name: must match {ID_RE.pattern}")
        params = c.get("params") or {}
        if not isinstance(params, dict):
            raise SpecError(f"{cw}.params: must be a mapping")
        for k in params:
            if not re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", str(k)):
                raise SpecError(f"{cw}.params: invalid key {k!r}")
            if k in BUILTIN_PLACEHOLDERS:
                raise SpecError(f"{cw}.params: key {k!r} collides with a built-in placeholder")
        cmd = c.get("command", default_cmd)
        if cmd is None:
            raise SpecError(f"{cw}: no command (set candidate.command or top-level command)")
        cmd = _command(cmd, f"{cw}.command")
        allowed = BUILTIN_PLACEHOLDERS | set(params)
        for f_ in template_fields(cmd):
            if f_ not in allowed:
                raise SpecError(f"{cw}.command: unknown placeholder '{{{f_}}}'")
        env = dict(default_env)
        env.update(c.get("env") or {})
        candidates.append(Candidate(name, dict(params), cmd, {str(k): str(v) for k, v in env.items()}, c.get("cwd", data.get("cwd"))))
    names = [c.name for c in candidates]
    if len(set(names)) != len(names):
        raise SpecError(f"{where}.candidates: duplicate names")

    corpus = _parse_corpus(_req(data, "corpus", where), f"{where}.corpus")
    metrics = _parse_metrics(_req(data, "metrics", where), f"{where}.metrics", candidates)

    repetitions = _int(_req(data, "repetitions", where), f"{where}.repetitions", 1)
    warmups = _int(data.get("warmups", 0), f"{where}.warmups", 0)

    seeds_raw = _req(data, "seeds", where)
    if isinstance(seeds_raw, int) and not isinstance(seeds_raw, bool):
        seeds = Seeds(seeds_raw, seeds_raw, seeds_raw)
    elif isinstance(seeds_raw, dict):
        order = _int(_req(seeds_raw, "order", f"{where}.seeds"), f"{where}.seeds.order", 0)
        boot = _int(_req(seeds_raw, "bootstrap", f"{where}.seeds"), f"{where}.seeds.bootstrap", 0)
        cmd_seed = _int(seeds_raw.get("command", order), f"{where}.seeds.command", 0)
        seeds = Seeds(order, boot, cmd_seed)
    else:
        raise SpecError(f"{where}.seeds: must be an integer or mapping with order/bootstrap[/command]")

    timing = data.get("timing")
    if not isinstance(timing, bool):
        raise SpecError(f"{where}.timing: must be true or false")

    platform = _parse_platform(data.get("platform") or {}, f"{where}.platform")
    guard = _parse_guard(data.get("guard") or {}, f"{where}.guard")
    analysis = _parse_analysis(data.get("analysis") or {}, f"{where}.analysis", names, metrics)

    order = data.get("order", "randomized_blocks")
    if order not in ORDERS:
        raise SpecError(f"{where}.order: must be one of {ORDERS}")
    raw_compression = data.get("raw_compression", "none")
    if raw_compression not in ("none", "gzip"):
        raise SpecError(f"{where}.raw_compression: none or gzip")
    record_warmups = data.get("record_warmups", True)
    if not isinstance(record_warmups, bool):
        raise SpecError(f"{where}.record_warmups: bool")
    override = data.get("thresholds_override")
    if override is not None:
        if not isinstance(override, dict):
            raise SpecError(f"{where}.thresholds_override: mapping")
        if decision_ids:
            raise SpecError(
                f"{where}.thresholds_override: not allowed when decision_ids is non-empty "
                "(decision experiments take thresholds from research/methods/thresholds.json)"
            )

    if sha256 is None:
        sha256 = hashlib.sha256(json.dumps(data, sort_keys=True, default=str).encode()).hexdigest()
    return Spec(
        experiment_id=exp_id,
        question=question,
        hypothesis=hypothesis,
        decision_ids=decision_ids,
        candidates=candidates,
        corpus=corpus,
        metrics=metrics,
        repetitions=repetitions,
        warmups=warmups,
        seeds=seeds,
        timing=timing,
        platform=platform,
        guard=guard,
        analysis=analysis,
        order=order,
        raw_compression=raw_compression,
        record_warmups=record_warmups,
        thresholds_override=override,
        shell=shell,
        raw=data,
        path=path,
        sha256=sha256,
    )


def _parse_corpus(c: Any, where: str) -> CorpusSelector:
    if not isinstance(c, dict):
        raise SpecError(f"{where}: must be a mapping")
    unknown = set(c) - {"splits", "item_ids", "families", "manifest", "root", "generated"}
    if unknown:
        raise SpecError(f"{where}: unknown fields {sorted(unknown)}")
    splits = _str_list(c.get("splits"), f"{where}.splits")
    item_ids = _str_list(c.get("item_ids"), f"{where}.item_ids")
    families = _str_list(c.get("families"), f"{where}.families")
    for s in splits + item_ids:
        if not ID_RE.match(s):
            raise SpecError(f"{where}: invalid split/item id {s!r}")
    generated = []
    for i, g in enumerate(c.get("generated") or []):
        gw = f"{where}.generated[{i}]"
        if not isinstance(g, dict):
            raise SpecError(f"{gw}: mapping")
        item_id = _req(g, "item_id", gw)
        if not isinstance(item_id, str) or not ID_RE.match(item_id):
            raise SpecError(f"{gw}.item_id invalid")
        gen = _req(g, "generator", gw)
        if gen not in GENERATORS:
            raise SpecError(f"{gw}.generator: one of {GENERATORS}")
        nbytes = _int(_req(g, "bytes", gw), f"{gw}.bytes", 0)
        seed = _int(g.get("seed", 0), f"{gw}.seed", 0)
        sha = g.get("sha256")
        if sha is not None and not re.match(r"^[0-9a-f]{64}$", str(sha)):
            raise SpecError(f"{gw}.sha256: 64 lowercase hex chars")
        generated.append(GeneratedItem(item_id, gen, nbytes, seed, g.get("family"), sha))
    if not (splits or item_ids or generated or c.get("manifest")):
        raise SpecError(f"{where}: select at least one of splits, item_ids, manifest, generated")
    if item_ids and not splits and not c.get("manifest"):
        raise SpecError(f"{where}: item_ids need splits or a manifest to locate items")
    if families and not c.get("manifest") and not generated:
        raise SpecError(f"{where}: families selection requires a manifest")
    return CorpusSelector(splits, item_ids, families, c.get("manifest"), c.get("root"), generated)


def _parse_metrics(ms: Any, where: str, candidates: Sequence[Candidate]) -> List[Metric]:
    if not isinstance(ms, list) or not ms:
        raise SpecError(f"{where}: non-empty list required")
    out: List[Metric] = []
    seen = set()
    all_params = set().union(*[set(c.params) for c in candidates]) if candidates else set()
    for i, m in enumerate(ms):
        mw = f"{where}[{i}]"
        if not isinstance(m, dict):
            raise SpecError(f"{mw}: mapping")
        name = _req(m, "name", mw)
        if not isinstance(name, str) or not re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", name):
            raise SpecError(f"{mw}.name invalid")
        if name in seen:
            raise SpecError(f"{mw}: duplicate metric {name}")
        seen.add(name)
        source = _req(m, "source", mw)
        if source not in METRIC_SOURCES:
            raise SpecError(f"{mw}.source: one of {METRIC_SOURCES}")
        family = _req(m, "family", mw)
        if family not in METRIC_FAMILIES:
            raise SpecError(f"{mw}.family: one of {sorted(METRIC_FAMILIES)}")
        direction = m.get("direction", "none")
        if direction not in ("minimize", "maximize", "none"):
            raise SpecError(f"{mw}.direction: minimize, maximize or none")
        kind = "string" if source == "path_sha256" else m.get("kind", "numeric")
        met = Metric(name=name, source=source, family=family, direction=direction, unit=m.get("unit"), kind=kind)
        if source == "resource":
            if name not in RESOURCE_METRICS:
                raise SpecError(f"{mw}: resource metric must be one of {sorted(RESOURCE_METRICS)}")
        elif source == "scratch":
            if name not in SCRATCH_METRICS:
                raise SpecError(f"{mw}: scratch metric must be one of {sorted(SCRATCH_METRICS)}")
        elif source == "input":
            if name not in INPUT_METRICS:
                raise SpecError(f"{mw}: input metric must be one of {sorted(INPUT_METRICS)}")
        elif source in ("path_size", "path_sha256"):
            met.path = _req(m, "path", mw)
            template_check(met.path, all_params, f"{mw}.path")
        elif source == "check":
            met.command = _command(_req(m, "command", mw), f"{mw}.command")
            template_check(met.command, all_params, f"{mw}.command")
        elif source == "stdout_json":
            met.key = _req(m, "key", mw)
        elif source == "derived":
            met.op = _req(m, "op", mw)
            if met.op not in DERIVED_OPS:
                raise SpecError(f"{mw}.op: one of {DERIVED_OPS}")
            args = _str_list(_req(m, "args", mw), f"{mw}.args")
            if len(args) != 2:
                raise SpecError(f"{mw}.args: exactly two metric names")
            builtin = RESOURCE_METRICS | SCRATCH_METRICS | INPUT_METRICS
            for a in args:
                if a == name or (a not in seen and a not in builtin):
                    raise SpecError(f"{mw}.args: {a!r} must name an earlier metric or a built-in measurement")
            met.args = args
        out.append(met)
    return out


def template_check(cmd: Command, params: set, where: str) -> None:
    allowed = BUILTIN_PLACEHOLDERS | set(params)
    for f_ in template_fields(cmd):
        if f_ not in allowed:
            raise SpecError(f"{where}: unknown placeholder '{{{f_}}}'")


def _parse_platform(p: Dict[str, Any], where: str) -> Platform:
    os_ = _str_list(p.get("os", ["linux"]), f"{where}.os")
    for o in os_:
        if o not in ("linux", "windows", "darwin"):
            raise SpecError(f"{where}.os: linux, windows or darwin")
    arch = _str_list(p.get("arch", []), f"{where}.arch")
    emulated = p.get("emulated", False)
    if not isinstance(emulated, bool):
        raise SpecError(f"{where}.emulated: bool")
    min_cpus = _int(p.get("min_cpus", 1), f"{where}.min_cpus", 1)
    aff = p.get("cpu_affinity")
    if aff is not None:
        if not isinstance(aff, list) or not aff or not all(isinstance(x, int) and x >= 0 for x in aff):
            raise SpecError(f"{where}.cpu_affinity: non-empty list of cpu indices")
    poll = _opt_float(p.get("poll_interval_s", 0.05), f"{where}.poll_interval_s")
    if poll is None or poll <= 0:
        raise SpecError(f"{where}.poll_interval_s: positive number")
    timeout = _opt_float(p.get("timeout_s"), f"{where}.timeout_s")
    return Platform(os_, arch, emulated, min_cpus, aff, p.get("scratch_root"), _str_list(p.get("requires", []), f"{where}.requires"), poll, timeout)


def _parse_guard(g: Dict[str, Any], where: str) -> GuardConfig:
    unknown = set(g) - {"max_loadavg_1m", "max_other_cpu_percent", "sample_interval_s", "max_wait_s"}
    if unknown:
        raise SpecError(f"{where}: unknown fields {sorted(unknown)}")
    interval = _opt_float(g.get("sample_interval_s", 1.0), f"{where}.sample_interval_s")
    wait = _opt_float(g.get("max_wait_s", 0.0), f"{where}.max_wait_s")
    if interval is None or interval <= 0:
        raise SpecError(f"{where}.sample_interval_s: positive")
    return GuardConfig(
        _opt_float(g.get("max_loadavg_1m"), f"{where}.max_loadavg_1m"),
        _opt_float(g.get("max_other_cpu_percent"), f"{where}.max_other_cpu_percent"),
        interval,
        wait or 0.0,
    )


def _parse_analysis(a: Dict[str, Any], where: str, cand_names: List[str], metrics: List[Metric]) -> Analysis:
    baseline = a.get("baseline")
    if baseline is not None and baseline not in cand_names:
        raise SpecError(f"{where}.baseline: unknown candidate {baseline!r}")
    pairing = a.get("pairing", "item")
    if pairing not in PAIRINGS:
        raise SpecError(f"{where}.pairing: one of {PAIRINGS}")
    mnames = {m.name: m for m in metrics}
    objectives = _str_list(a.get("pareto_objectives", []), f"{where}.pareto_objectives")
    for o in objectives:
        if o not in mnames:
            raise SpecError(f"{where}.pareto_objectives: unknown metric {o}")
        if mnames[o].direction == "none" or mnames[o].kind != "numeric":
            raise SpecError(f"{where}.pareto_objectives: {o} needs direction minimize/maximize and numeric kind")
    rep = _str_list(a.get("report_metrics", []), f"{where}.report_metrics")
    for r in rep:
        if r not in mnames:
            raise SpecError(f"{where}.report_metrics: unknown metric {r}")
    sens = a.get("sensitivity")
    if sens is not None:
        if not isinstance(sens, dict) or not isinstance(sens.get("weights"), dict):
            raise SpecError(f"{where}.sensitivity: mapping with 'weights' {{metric: weight}}")
        for k in sens["weights"]:
            if k not in mnames or mnames[k].direction == "none":
                raise SpecError(f"{where}.sensitivity.weights: {k} must be a metric with a direction")
    return Analysis(baseline, pairing, objectives, rep, sens)


# ----------------------------------------------------------------------------
# held-out guard


def design_freeze_sha(freeze_path: Path) -> Optional[str]:
    """SHA recorded in research/decisions/design-freeze.json, or None if absent."""
    if not freeze_path.is_file():
        return None
    with open(freeze_path, encoding="utf-8") as fh:
        data = json.load(fh)
    for key in ("design_freeze_sha", "commit_sha", "sha"):
        v = data.get(key)
        if isinstance(v, str) and HEX_RE.match(v):
            return v.lower()
    raise SpecError(f"{freeze_path}: no valid design_freeze_sha/commit_sha/sha field")


def heldout_unlocked(freeze_path: Path, env: Optional[Dict[str, str]] = None) -> bool:
    env = os.environ if env is None else env
    token = env.get("EB_HELDOUT_UNLOCK")
    if not token:
        return False
    recorded = design_freeze_sha(freeze_path)
    if recorded is None:
        return False
    return token.strip().lower() == recorded


def check_heldout(spec: Spec, freeze_path: Path, env: Optional[Dict[str, str]] = None) -> bool:
    """Raise HeldoutLocked if the spec selects held-out data without a valid unlock.

    Returns True when the held-out split is selected and unlocked.
    """
    if not spec.corpus.wants_heldout:
        return False
    if heldout_unlocked(freeze_path, env):
        return True
    recorded = design_freeze_sha(freeze_path)
    if recorded is None:
        reason = f"{freeze_path} does not exist (no design freeze recorded)"
    elif not (env if env is not None else os.environ).get("EB_HELDOUT_UNLOCK"):
        reason = "EB_HELDOUT_UNLOCK is not set"
    else:
        reason = "EB_HELDOUT_UNLOCK does not equal the recorded design-freeze SHA"
    raise HeldoutLocked(f"{spec.experiment_id}: split=heldout refused: {reason}")
