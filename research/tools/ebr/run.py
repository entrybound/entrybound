"""Experiment execution: plan, guard, measure, and append raw rows.

Run order: every repetition round (warmup rounds first) visits each
(candidate, item) exactly once. With the default ``randomized_blocks`` order the
items are shuffled per round and candidates are shuffled within each item block,
so paired candidates for the same item run close together in time while the
position of each candidate drifts randomly across rounds (decorrelating machine
drift from candidate identity). ``randomized_rounds`` shuffles all pairs within a
round; ``sequential`` is for debugging only. The shuffle uses ``random.Random``
seeded with ``seeds.order`` (algorithm stable across CPython 3.x).
"""

from __future__ import annotations

import hashlib
import json
import os
import platform as _platform
import random
import shutil
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

from . import META_SCHEMA, __version__
from .corpus import FingerprintCache, Item, prepare_items, resolve_items, sha256_file
from .envinfo import resolve_env_id
from .execute import IS_LINUX, IS_WINDOWS, execute, path_size, render, run_check, utcnow
from .guard import QuietGuard
from .paths import Layout, git_dirty, git_sha
from .rawio import RawWriter, meta_path, raw_path, write_json
from .spec import HeldoutLocked, Spec, SpecError, check_heldout, design_freeze_sha
from .thresholds import TIMING_FAMILIES, Thresholds


class RunRefused(RuntimeError):
    """The runner refused to start or continue (guard, held-out, timing flags)."""


@dataclass
class RunOptions:
    timing: bool = False
    allow_untimed: bool = False
    run_id: Optional[str] = None
    compression: Optional[str] = None
    cpus: Optional[List[int]] = None
    keep_scratch: bool = False
    scratch_root: Optional[str] = None
    max_loadavg_1m: Optional[float] = None
    max_other_cpu_percent: Optional[float] = None
    dry_run: bool = False
    argv: List[str] = field(default_factory=list)
    quiet: bool = False


@dataclass
class PlanEntry:
    order_index: int
    phase: str
    round: int
    candidate: str
    item_key: str
    repetition: int
    seq: Optional[int]


@dataclass
class RunOutcome:
    run_id: str
    status: str
    raw_file: Optional[Path]
    meta_file: Optional[Path]
    planned: int
    recorded: int
    failed: int
    invalid: int
    plan: List[PlanEntry]


def os_name() -> str:
    if IS_LINUX:
        return "linux"
    if IS_WINDOWS:
        return "windows"
    return sys.platform


def arch_name() -> str:
    m = _platform.machine().lower()
    return {"amd64": "x86_64", "x64": "x86_64", "arm64": "aarch64"}.get(m, m)


def new_run_id() -> str:
    return datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + os.urandom(3).hex()


def build_plan(spec: Spec, items: List[Item], record_warmups: Optional[bool] = None) -> List[PlanEntry]:
    rng = random.Random(spec.seeds.order)
    record_warmups = spec.record_warmups if record_warmups is None else record_warmups
    cands = [c.name for c in spec.candidates]
    keys = [f"{it.split}/{it.item_id}" for it in items]
    rounds = [("warmup", r) for r in range(spec.warmups)] + [("measure", r) for r in range(spec.repetitions)]
    plan: List[PlanEntry] = []
    seq = 0
    for rnd, (phase, rep) in enumerate(rounds):
        if spec.order == "sequential":
            pairs = [(c, k) for k in keys for c in cands]
        elif spec.order == "randomized_rounds":
            pairs = [(c, k) for k in keys for c in cands]
            rng.shuffle(pairs)
        else:
            blocks = []
            for k in keys:
                cs = list(cands)
                rng.shuffle(cs)
                blocks.append([(c, k) for c in cs])
            rng.shuffle(blocks)
            pairs = [p for b in blocks for p in b]
        for c, k in pairs:
            recorded = phase == "measure" or record_warmups
            plan.append(PlanEntry(len(plan), phase, rnd, c, k, rep, seq if recorded else None))
            if recorded:
                seq += 1
    return plan


def sample_seed(spec: Spec, entry: PlanEntry) -> int:
    msg = f"{spec.seeds.command}|{entry.candidate}|{entry.item_key}|{entry.phase}|{entry.repetition}".encode()
    return int.from_bytes(hashlib.sha256(msg).digest()[:4], "little")


def _check_platform(spec: Spec, opts: RunOptions) -> List[int]:
    p = spec.platform
    if os_name() not in p.os:
        raise RunRefused(f"spec requires os in {p.os}; running on {os_name()}")
    if p.arch and arch_name() not in p.arch:
        raise RunRefused(f"spec requires arch in {p.arch}; running on {arch_name()}")
    ncpu = os.cpu_count() or 1
    if ncpu < p.min_cpus:
        raise RunRefused(f"spec requires >= {p.min_cpus} logical CPUs; have {ncpu}")
    missing = [t for t in p.requires if shutil.which(t) is None and not os.access(t, os.X_OK)]
    if IS_LINUX and not os.access("/usr/bin/time", os.X_OK):
        missing.append("/usr/bin/time")
    if missing:
        raise RunRefused(f"required executables missing: {missing}")
    affinity = opts.cpus if opts.cpus is not None else p.cpu_affinity
    if affinity:
        bad = [c for c in affinity if c >= ncpu]
        if bad:
            raise RunRefused(f"cpu_affinity indices {bad} out of range (ncpu={ncpu})")
    return list(affinity) if affinity else []


def _guard_for(spec: Spec, thresholds: Thresholds, opts: RunOptions) -> QuietGuard:
    tj = thresholds.guard()
    sources = {}
    vals = {}
    for key, cli in (("max_loadavg_1m", opts.max_loadavg_1m), ("max_other_cpu_percent", opts.max_other_cpu_percent)):
        spec_v = getattr(spec.guard, key)
        if cli is not None:
            vals[key], sources[key] = cli, "cli"
        elif spec_v is not None:
            vals[key], sources[key] = spec_v, "spec.guard"
        elif tj.get(key) is not None:
            vals[key], sources[key] = float(tj[key]), "research/methods/thresholds.json"
        else:
            vals[key], sources[key] = None, "unset"
    return QuietGuard(vals["max_loadavg_1m"], vals["max_other_cpu_percent"], spec.guard.sample_interval_s, sources)


def _numeric(v: Any) -> Optional[float]:
    if isinstance(v, bool) or v is None:
        return None if v is None else float(v)
    if isinstance(v, (int, float)):
        return float(v)
    return None


def _derive(op: str, a: Optional[float], b: Optional[float]) -> Optional[float]:
    if a is None or b is None:
        return None
    if op == "ratio":
        return None if b == 0 else a / b
    if op == "rate":
        return None if b == 0 else a / b
    if op == "difference":
        return a - b
    if op == "sum":
        return a + b
    if op == "product":
        return a * b
    raise ValueError(op)


def _json_key(line: Optional[str], key: str) -> Any:
    if not line:
        return None
    try:
        obj = json.loads(line)
    except ValueError:
        return None
    for part in key.split("."):
        if not isinstance(obj, dict) or part not in obj:
            return None
        obj = obj[part]
    return obj


def run_experiment(spec: Spec, layout: Layout, opts: RunOptions, log=None) -> RunOutcome:
    log = log or (lambda msg: None if opts.quiet else print(msg, file=sys.stderr, flush=True))

    # --- timing mode ---------------------------------------------------------
    if opts.timing and not spec.timing:
        raise SpecError(f"{spec.experiment_id}: --timing given but spec has timing: false")
    if spec.timing and not opts.timing and not opts.allow_untimed:
        raise RunRefused(
            f"{spec.experiment_id}: spec has timing: true; run in a quiet window with --timing, "
            "or pass --allow-untimed to record non-decision-grade samples"
        )
    timing_effective = bool(spec.timing and opts.timing)

    thresholds = Thresholds.load(layout.thresholds_path, spec.thresholds_override, spec.decision_ids)
    affinity = _check_platform(spec, opts)

    # --- held-out guard ---------------------------------------------------------
    heldout_ok = check_heldout(spec, layout.design_freeze_path)
    if not heldout_ok and layout.data_root is not None:
        # defense in depth: templates/env/params must not hard-code the held-out root
        hroot = layout.heldout_root.as_posix().rstrip("/")
        blobs = [json.dumps([c.command, c.env, c.params, c.cwd]) for c in spec.candidates]
        blobs += [json.dumps([m.command, m.path]) for m in spec.metrics]
        if any(hroot in b for b in blobs):
            raise HeldoutLocked(f"{spec.experiment_id}: a command/env/path references the held-out root {hroot}")
    items = resolve_items(spec, layout, heldout_ok)
    plan = build_plan(spec, items)
    run_id = opts.run_id or new_run_id()
    if opts.dry_run:
        return RunOutcome(run_id, "dry-run", None, None, len(plan), 0, 0, 0, plan)

    guard = _guard_for(spec, thresholds, opts)
    guard_initial = None
    if timing_effective:
        missing = guard.missing_limits()
        if missing:
            raise RunRefused(
                f"timing run refused: quiet-machine guard limits unset {missing} "
                "(set research/methods/thresholds.json quiet_machine_guard per decision-method.md, "
                "the spec guard block, or --max-load/--max-other-cpu)"
            )
        guard_initial = guard.check()
        if not guard_initial.ok:
            raise RunRefused(f"timing run refused: machine not quiet: {guard_initial.reasons}")

    scratch_root = Path(opts.scratch_root or spec.platform.scratch_root or layout.scratch_root)
    try:
        scratch_root.resolve().relative_to(layout.repo_root.resolve())
        raise RunRefused(f"scratch root {scratch_root} is inside the repository; use the data root")
    except ValueError:
        pass

    cache = FingerprintCache(layout.cache_dir / "fingerprints.sqlite" if layout.cache_dir else None)
    fingerprints = prepare_items(spec, layout, items, cache)
    item_by_key = {f"{it.split}/{it.item_id}": it for it in items}

    env_id, env_details = resolve_env_id(layout)
    sha = git_sha(layout.repo_root)
    dirty = git_dirty(layout.repo_root) if sha else None
    freeze_sha = design_freeze_sha(layout.design_freeze_path) if heldout_ok else None
    compression = opts.compression or spec.raw_compression

    raw_dir = layout.raw_dir(spec.experiment_id)
    rpath = raw_path(raw_dir, run_id, compression)
    mpath = meta_path(raw_dir, run_id)
    if mpath.exists():
        raise RunRefused(f"run id {run_id} already exists")
    writer = RawWriter(rpath)
    spec_file_sha = sha256_file(Path(spec.path)) if spec.path else spec.sha256
    meta: Dict[str, Any] = {
        "schema": META_SCHEMA,
        "experiment_id": spec.experiment_id,
        "run_id": run_id,
        "status": "running",
        "ebr_version": __version__,
        "argv": opts.argv,
        "spec_path": layout.rel(spec.path) if spec.path else None,
        "spec_sha256": spec_file_sha,
        "spec": spec.raw,
        "decision_ids": spec.decision_ids,
        "git_sha": sha,
        "git_dirty": dirty,
        "env_id": env_id,
        "env": env_details,
        "python": sys.version.split()[0],
        "platform": {"os": os_name(), "arch": arch_name(), "emulated": spec.platform.emulated},
        "timing": {
            "spec": spec.timing,
            "cli": opts.timing,
            "effective": timing_effective,
            "guard": guard.config(),
            "guard_initial": guard_initial.to_dict() if guard_initial else None,
        },
        "thresholds_source": thresholds.source,
        "cpu_affinity": affinity or None,
        "order": spec.order,
        "seeds": {"order": spec.seeds.order, "bootstrap": spec.seeds.bootstrap, "command": spec.seeds.command},
        "heldout": {"selected": spec.corpus.wants_heldout, "unlocked": heldout_ok, "unlock_sha": freeze_sha},
        "items": [
            {"item_key": k, "item_id": it.item_id, "split": it.split, "family": it.family,
             "path": it.path.as_posix(), "generated": it.generated, "fingerprint": fingerprints[k]}
            for k, it in sorted(item_by_key.items())
        ],
        "plan": [e.__dict__ for e in plan],
        "planned_samples": sum(1 for e in plan if e.seq is not None),
        "executed_samples": 0,
        "recorded_samples": 0,
        "raw_file": rpath.name,
        "raw_compression": compression,
        "started_utc": utcnow(),
        "finished_utc": None,
        "raw_sha256": None,
        "raw_bytes": None,
    }
    write_json(mpath, meta)

    run_scratch = scratch_root / spec.experiment_id / run_id
    io_dir = run_scratch / "_io"
    base_env = dict(os.environ)
    if not heldout_ok:
        base_env.pop("EB_HELDOUT_UNLOCK", None)

    failed = invalid = executed = 0
    status = "complete"
    last_guard = guard_initial
    try:
        for entry in plan:
            cand = spec.candidate(entry.candidate)
            item = item_by_key[entry.item_key]
            fp = fingerprints[entry.item_key]
            seed = sample_seed(spec, entry)

            guard_pre = None
            if timing_effective:
                guard_pre = last_guard if last_guard is not None else guard.check()
                waited = 0.0
                while not guard_pre.ok and waited < spec.guard.max_wait_s:
                    time.sleep(min(5.0, spec.guard.max_wait_s - waited))
                    waited += min(5.0, spec.guard.max_wait_s - waited)
                    guard_pre = guard.check()
                if not guard_pre.ok:
                    status = "aborted-guard"
                    log(f"[ebr] aborting: machine not quiet before sample {entry.order_index}: {guard_pre.reasons}")
                    break

            scratch = run_scratch / f"s{entry.order_index:06d}"
            if scratch.exists():
                shutil.rmtree(scratch)
            scratch.mkdir(parents=True)
            ctx = {
                "item_path": item.path.as_posix() if not IS_WINDOWS else str(item.path),
                "item_id": item.item_id,
                "split": item.split,
                "family": item.family or "",
                "scratch": scratch.as_posix() if not IS_WINDOWS else str(scratch),
                "candidate": cand.name,
                "rep": entry.repetition,
                "seed": seed,
                "input_bytes": fp["bytes"],
                **cand.params,
            }
            env = dict(base_env)
            env.update(cand.env)
            env.update({"EB_EXPERIMENT_ID": spec.experiment_id, "EB_RUN_ID": run_id, "EB_SCRATCH": ctx["scratch"]})
            rendered = render(cand.command, ctx, spec.shell)
            res = execute(
                rendered,
                cwd=cand.cwd,
                env=env,
                scratch=scratch,
                io_dir=io_dir,
                poll_interval_s=spec.platform.poll_interval_s,
                timeout_s=spec.platform.timeout_s,
                cpu_affinity=affinity or None,
            )
            executed += 1

            builtin: Dict[str, Any] = {**res.resource, **res.scratch, "input_bytes": fp["bytes"], "input_files": fp["files"]}
            metrics: Dict[str, Any] = {}
            checks: Dict[str, Any] = {}
            for m in spec.metrics:
                if m.source in ("resource", "scratch", "input"):
                    metrics[m.name] = builtin.get(m.name)
                elif m.source == "path_size":
                    metrics[m.name] = path_size(Path(m.path.format(**{k: str(v) for k, v in ctx.items()})))
                elif m.source == "path_sha256":
                    pth = Path(m.path.format(**{k: str(v) for k, v in ctx.items()}))
                    metrics[m.name] = sha256_file(pth) if pth.is_file() else None
                elif m.source == "check":
                    chk = run_check(render(m.command, ctx, spec.shell), cwd=cand.cwd, env=env, timeout_s=spec.platform.timeout_s)
                    checks[m.name] = chk
                    metrics[m.name] = 1 if chk["exit_code"] == 0 else 0
                elif m.source == "stdout_json":
                    metrics[m.name] = _json_key(res.last_stdout_line, m.key)
                elif m.source == "derived":
                    vals = [_numeric(metrics[a]) if a in metrics else _numeric(builtin.get(a)) for a in m.args]
                    metrics[m.name] = _derive(m.op, vals[0], vals[1])

            ok = res.exit_code == 0 and not res.timed_out and all(c["exit_code"] == 0 for c in checks.values())
            invalid_reasons: List[str] = []
            if not ok:
                invalid_reasons.append("timeout" if res.timed_out else "command_or_check_failed")
            if res.errors:
                invalid_reasons.extend(res.errors)

            guard_post = during = None
            if timing_effective:
                guard_post = guard.check()
                last_guard = guard_post
                if not guard_pre.ok:
                    invalid_reasons.append("guard_pre_failed")
                if not guard_post.ok:
                    invalid_reasons.append("guard_post_failed")
                during = guard.evaluate(None, res.cpu_window.get("other_cpu_percent"))
                enforce_during = res.cpu_window.get("window_s", 0) >= guard.sample_interval_s
                if enforce_during and not during.ok:
                    invalid_reasons.append("guard_during_failed")
            valid = ok and not invalid_reasons
            if not ok:
                failed += 1
            if not valid:
                invalid += 1

            if entry.seq is not None:
                row = {
                    "experiment_id": spec.experiment_id,
                    "run_id": run_id,
                    "seq": entry.seq,
                    "order_index": entry.order_index,
                    "round": entry.round,
                    "phase": entry.phase,
                    "candidate": cand.name,
                    "params": cand.params,
                    "item_id": item.item_id,
                    "split": item.split,
                    "family": item.family,
                    "repetition": entry.repetition,
                    "seed": seed,
                    "command": {"template": cand.command, "rendered": rendered["display"], "argv": res.measurement.get("argv"), "shell": spec.shell},
                    "exit_code": res.exit_code,
                    "timed_out": res.timed_out,
                    "ok": ok,
                    "valid": valid,
                    "invalid_reasons": invalid_reasons,
                    "stdout_sha256": res.stdout_sha256,
                    "stdout_bytes": res.stdout_bytes,
                    "stderr_sha256": res.stderr_sha256,
                    "stderr_bytes": res.stderr_bytes,
                    "stderr_excerpt": res.stderr_excerpt,
                    "metrics": metrics,
                    "metric_families": {m.name: m.family for m in spec.metrics},
                    "checks": checks,
                    "resource": res.resource,
                    "scratch": res.scratch,
                    "measurement": res.measurement,
                    "timestamps": {"start_utc": res.start_utc, "end_utc": res.end_utc},
                    "env_id": env_id,
                    "git_sha": sha,
                    "git_dirty": dirty,
                    "spec_sha256": spec_file_sha,
                    "item_fingerprint": fp,
                    "heldout": {"unlock_sha": freeze_sha} if item.is_heldout else None,
                    "timing": {
                        "spec": spec.timing,
                        "effective": timing_effective,
                        "decision_grade": timing_effective and valid,
                        "timing_families": sorted(TIMING_FAMILIES),
                        "guard_pre": guard_pre.to_dict() if guard_pre else None,
                        "guard_post": guard_post.to_dict() if guard_post else None,
                        "guard_during": during.to_dict() if during else None,
                        "cpu_window": res.cpu_window,
                    },
                    "cpu_affinity": affinity or None,
                }
                writer.append(row)
            log(
                f"[ebr] {entry.order_index + 1}/{len(plan)} {entry.phase} {cand.name} {entry.item_key} "
                f"rep={entry.repetition} exit={res.exit_code} ok={ok} valid={valid}"
            )
            if not opts.keep_scratch:
                shutil.rmtree(scratch, ignore_errors=True)
    except KeyboardInterrupt:
        status = "interrupted"
        raise
    except BaseException:
        status = "error"
        raise
    finally:
        meta["status"] = status
        meta["executed_samples"] = executed
        meta["recorded_samples"] = writer.count
        meta["failed_samples"] = failed
        meta["invalid_samples"] = invalid
        meta["finished_utc"] = utcnow()
        if rpath.exists():
            meta["raw_sha256"] = sha256_file(rpath)
            meta["raw_bytes"] = rpath.stat().st_size
        write_json(mpath, meta)
        if not opts.keep_scratch:
            shutil.rmtree(run_scratch, ignore_errors=True)
    if status != "complete":
        raise RunRefused(f"run {run_id} ended with status {status}")
    return RunOutcome(run_id, status, rpath, mpath, len(plan), writer.count, failed, invalid, plan)
