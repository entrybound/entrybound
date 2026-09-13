#!/usr/bin/env python3
"""Synthetic batch-cluster log tree from a discrete-event simulation (F05, generated).
g3-data generator "logs C" -- deliberately a different model and code path from
gen_logs_ab.py so generated held-out items are not seeds of the tuning generator.

ebrc generator contract:
    python gen_logs_c.py --out <empty dir> --seed <int> --params <canonical JSON>
Pure function of (script bytes, seed, params).  Stdlib only.

params:
  start_epoch     int (default 1772323200 = 2026-03-01T00:00:00Z)
  nodes           int worker nodes (default 64)
  jobs            int submitted jobs (default 40000)
  metrics_every   int seconds between metric scrapes (default 30)
  max_bytes       int soft cap on total output; the simulation stops when exceeded (default 0 = off)

The simulation keeps state per job (queued -> scheduled -> running -> checkpoint* ->
succeeded | failed -> retried) and per node (load, memory, disk, GC pressure) and emits:
  scheduler/scheduler.log     ISO-8601 text lines with thread names, multi-line Python tracebacks
  nodes/<node>/agent.log      per-node agent log, bracketed levels, key=value tails
  nodes/<node>/gc.log         JVM unified-logging style GC pauses (only for JVM task nodes)
  audit/audit-YYYYMMDD.tsv    tab-separated audit trail with a header
  metrics/scrape-HHMM.prom    Prometheus text exposition snapshots (hourly files)
"""

import argparse
import heapq
import json
import os
import random
import time

QUEUES = ["etl", "ml-train", "reporting", "adhoc", "backfill"]
USERS = ["svc-etl", "svc-reports", "alice.k", "bo.chen", "carla.m", "dmitri.v", "e.okafor", "farah.n", "svc-ml"]
IMAGES = ["registry.local/etl-runner:4.12.1", "registry.local/spark-job:3.5.4", "registry.local/pytorch-train:2.7.0-cu124",
          "registry.local/report-gen:1.9.0", "registry.local/duckdb-batch:1.3.2"]
FAIL_REASONS = [("OOMKilled", "container exceeded memory limit"), ("ExitCode137", "killed by signal 9"),
                ("PreemptedByPriority", "preempted by higher priority job"), ("DataError", "input partition missing"),
                ("Timeout", "exceeded wall clock limit")]
PY_FRAMES = ["runner/main.py", "runner/stages.py", "etl/extract.py", "etl/transform.py", "io/objectstore.py",
             "io/parquet_writer.py", "ml/train_loop.py", "ml/dataloader.py"]


def ts_iso(t):
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime(t)) + ".%06d+00:00" % int((t % 1) * 1e6)


class Out:
    def __init__(self, root, max_bytes):
        self.root = root
        self.files = {}
        self.max_bytes = max_bytes
        self.total = 0

    def write(self, rel, text):
        f = self.files.get(rel)
        if f is None and rel.startswith("metrics/"):
            # hourly metric files are written strictly in time order: close the previous hour
            for old in [k for k in self.files if k.startswith("metrics/")]:
                self.files.pop(old).close()
        if f is None:
            path = os.path.join(self.root, rel)
            os.makedirs(os.path.dirname(path), exist_ok=True)
            f = open(path, "w", encoding="utf-8", newline="\n")
            self.files[rel] = f
            if rel.startswith("audit/"):
                header = "ts\tevent\tjob\tuser\tqueue\tnode\tattempt\tdetail\n"
                f.write(header)
                self.total += len(header)
        f.write(text)
        self.total += len(text)

    def full(self):
        return self.max_bytes and self.total >= self.max_bytes

    def close(self):
        for rel in sorted(self.files):
            self.files[rel].close()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    rng = random.Random(args.seed ^ 0x5EED_C0DE)
    t0 = int(p.get("start_epoch", 1772323200))
    n_nodes = int(p.get("nodes", 64))
    n_jobs = int(p.get("jobs", 40000))
    scrape = int(p.get("metrics_every", 30))
    out = Out(args.out, int(p.get("max_bytes", 0)))

    nodes = []
    for i in range(n_nodes):
        rack = i // 16
        nodes.append({"name": f"r{rack:02d}n{i:03d}", "cpu": rng.choice([32, 64, 96]), "mem": rng.choice([128, 256, 512]),
                      "jvm": rng.random() < 0.5, "busy": 0, "mem_used": 0.0, "disk": rng.uniform(0.1, 0.6),
                      "gc_heap": 0.0, "tasks_done": 0, "tasks_failed": 0})
    events = []  # (time, seq, kind, payload)
    seq = 0

    def push(t, kind, payload):
        nonlocal seq
        seq += 1
        heapq.heappush(events, (t, seq, kind, payload))

    t = float(t0)
    for j in range(n_jobs):
        # bursty arrivals: Poisson with occasional batch submissions from cron-like users
        t += rng.expovariate(1 / 4.0) if rng.random() < 0.9 else 0.001
        push(t, "submit", {"id": f"job-{j:07d}", "user": rng.choice(USERS), "queue": rng.choice(QUEUES),
                           "image": rng.choice(IMAGES), "cpus": rng.choice([1, 2, 4, 8, 16]),
                           "mem": rng.choice([2, 4, 8, 16, 32, 64]), "dur": rng.lognormvariate(5.0, 1.2), "attempt": 1})
    next_scrape = float(t0)
    queue = []
    counters = {"submitted": 0, "started": 0, "succeeded": 0, "failed": 0, "retried": 0}

    def sched_log(tt, level, thread, msg):
        out.write("scheduler/scheduler.log", f"{ts_iso(tt)} {level:<7} [{thread}] {msg}\n")

    def audit(tt, ev, job, node, detail):
        day = time.strftime("%Y%m%d", time.gmtime(tt))
        out.write(f"audit/audit-{day}.tsv", f"{ts_iso(tt)}\t{ev}\t{job['id']}\t{job['user']}\t{job['queue']}\t{node}\t{job['attempt']}\t{detail}\n")

    def try_schedule(tt):
        progressed = True
        while queue and progressed:
            progressed = False
            job = queue[0]
            cands = [n for n in nodes if n["cpu"] - n["busy"] >= job["cpus"] and n["mem"] - n["mem_used"] >= job["mem"]]
            if not cands:
                if rng.random() < 0.02:
                    sched_log(tt, "DEBUG", "placement-0", f"no capacity for {job['id']} cpus={job['cpus']} mem={job['mem']}Gi pending={len(queue)}")
                return
            node = min(cands, key=lambda n: (n["busy"] / n["cpu"], n["name"]))
            queue.pop(0)
            node["busy"] += job["cpus"]
            node["mem_used"] += job["mem"]
            counters["started"] += 1
            sched_log(tt, "INFO", "placement-0", f"scheduled {job['id']} attempt={job['attempt']} on {node['name']} score={1 - node['busy'] / node['cpu']:.3f}")
            audit(tt, "SCHEDULED", job, node["name"], f"cpus={job['cpus']};mem={job['mem']}Gi")
            push(tt + rng.uniform(0.5, 8.0), "start", (job, node))
            progressed = True

    while events and not out.full():
        tt, _, kind, payload = heapq.heappop(events)
        while next_scrape <= tt:
            hour = time.strftime("%Y%m%d-%H", time.gmtime(next_scrape))
            ms = int(next_scrape * 1000)
            lines = []
            for name in ("submitted", "started", "succeeded", "failed", "retried"):
                lines.append(f"# TYPE cluster_jobs_{name}_total counter\ncluster_jobs_{name}_total {counters[name]} {ms}\n")
            lines.append(f"cluster_queue_depth{{scope=\"global\"}} {len(queue)} {ms}\n")
            for n in nodes:
                lab = f'node="{n["name"]}",rack="{n["name"][:3]}"'
                lines.append(f"node_cpu_allocated_ratio{{{lab}}} {n['busy'] / n['cpu']:.4f} {ms}\n"
                             f"node_memory_allocated_bytes{{{lab}}} {int(n['mem_used'] * 2**30)} {ms}\n"
                             f"node_disk_used_ratio{{{lab}}} {n['disk']:.5f} {ms}\n"
                             f"node_tasks_completed_total{{{lab}}} {n['tasks_done']} {ms}\n")
            out.write(f"metrics/scrape-{hour}.prom", "".join(lines))
            next_scrape += scrape
        if kind == "submit":
            job = payload
            counters["submitted"] += 1
            queue.append(job)
            sched_log(tt, "INFO", "api-%d" % rng.randrange(4), f"accepted {job['id']} user={job['user']} queue={job['queue']} image={job['image']}")
            audit(tt, "SUBMITTED", job, "-", f"image={job['image']}")
            try_schedule(tt)
        elif kind == "start":
            job, node = payload
            nlog = f"nodes/{node['name']}/agent.log"
            out.write(nlog, f"[{ts_iso(tt)}] [INFO] task started job={job['id']} attempt={job['attempt']} image={job['image']} cgroup=/kubepods/burstable/pod{rng.getrandbits(64):016x}\n")
            dur = job["dur"]
            fail = None
            if rng.random() < 0.07:
                fail = rng.choice(FAIL_REASONS)
                dur *= rng.uniform(0.05, 0.9)
            for k in range(int(dur // 300)):
                push(tt + 300 * (k + 1), "checkpoint", (job, node, k))
            push(tt + dur, "finish", (job, node, fail))
        elif kind == "checkpoint":
            job, node, k = payload
            nbytes = int(rng.lognormvariate(18, 1.5))
            out.write(f"nodes/{node['name']}/agent.log",
                      f"[{ts_iso(tt)}] [DEBUG] checkpoint job={job['id']} seq={k} bytes={nbytes} dest=s3://checkpoints/{job['queue']}/{job['id']}/{k:05d}.ckpt\n")
            node["disk"] = min(0.99, node["disk"] + nbytes / 4e13)
            if node["jvm"]:
                pause = rng.lognormvariate(1.5, 0.8)
                before = rng.randrange(900, 3900)
                after = int(before * rng.uniform(0.2, 0.7))
                out.write(f"nodes/{node['name']}/gc.log",
                          f"[{ts_iso(tt)}][{tt - t0:.3f}s][info][gc] GC({node['tasks_done'] + k}) Pause Young (Normal) (G1 Evacuation Pause) {before}M->{after}M(4096M) {pause:.3f}ms\n")
        elif kind == "finish":
            job, node, fail = payload
            node["busy"] -= job["cpus"]
            node["mem_used"] -= job["mem"]
            nlog = f"nodes/{node['name']}/agent.log"
            if fail is None:
                node["tasks_done"] += 1
                counters["succeeded"] += 1
                out.write(nlog, f"[{ts_iso(tt)}] [INFO] task finished job={job['id']} exit=0 wall={job['dur']:.1f}s maxrss={int(job['mem'] * rng.uniform(0.2, 0.95) * 1024)}MiB\n")
                sched_log(tt, "INFO", "reaper-0", f"{job['id']} succeeded attempt={job['attempt']}")
                audit(tt, "SUCCEEDED", job, node["name"], "exit=0")
            else:
                code, text = fail
                node["tasks_failed"] += 1
                counters["failed"] += 1
                out.write(nlog, f"[{ts_iso(tt)}] [WARN] task failed job={job['id']} reason={code} detail=\"{text}\"\n")
                sched_log(tt, "ERROR", "reaper-0", f"{job['id']} failed attempt={job['attempt']} reason={code}")
                if code == "DataError":
                    tb = ["Traceback (most recent call last):"]
                    for _ in range(rng.randrange(3, 12)):
                        fr = rng.choice(PY_FRAMES)
                        tb.append(f'  File "/opt/runner/{fr}", line {rng.randrange(10, 700)}, in {rng.choice(["run", "_stage", "read_partition", "open", "__iter__", "write_batch"])}')
                        tb.append(f"    {rng.choice(['return self._inner(*args)', 'for batch in reader:', 'raise PartitionMissing(key)', 'blob = client.get_object(Bucket=b, Key=k)'])}")
                    tb.append(f"runner.errors.PartitionMissing: s3://lake/{job['queue']}/dt=2026-02-{rng.randrange(1, 29):02d}/part-{rng.randrange(1000):05d}.parquet")
                    out.write("scheduler/scheduler.log", "\n".join(tb) + "\n")
                audit(tt, "FAILED", job, node["name"], f"reason={code}")
                if job["attempt"] < 3 and code != "DataError":
                    job = dict(job, attempt=job["attempt"] + 1)
                    counters["retried"] += 1
                    push(tt + 30 * 2 ** job["attempt"], "submit", job)
            try_schedule(tt)
    out.close()


if __name__ == "__main__":
    main()
