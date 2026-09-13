#!/usr/bin/env python3
"""Synthetic application/system log trees (F05, generated).  g3-data generator "logs A/B".

ebrc generator contract:
    python gen_logs_ab.py --out <empty dir> --seed <int> --params <canonical JSON>
Output is a pure function of (script bytes, seed, params).  Stdlib only.

params:
  start_epoch  int, UTC seconds of the first event (default 1767225600 = 2026-01-01)
  hosts        int, number of distinct hostnames (default 24)
  files        list of {"path": rel path, "profile": name, "bytes": target size,
                        "rate": mean events/second (default 5.0)}
               A file is generated until it reaches at least `bytes` (whole lines only).

profiles:
  syslog    RFC 3164-style lines from sshd, CRON, kernel, systemd, sudo, postfix
  jsonl     one JSON object per line: structured service logs with trace ids and http fields
  logfmt    key=value lines in the Go/logfmt style
  access    Apache/nginx "combined" access log
  java      log4j-style lines with occasional multi-line exception stack traces

Events arrive as a Poisson process with a diurnal rate modulation, values follow skewed
(Zipf-like) popularity so that templates, paths and users repeat realistically.  The data is
generated: its statistics reflect this model, not any real system.
"""

import argparse
import json
import math
import os
import random
import time

MONTHS = "Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec".split()
WORDS = ("order payment invoice session cart user account token cache shard replica index queue "
         "worker batch export import report upload download profile search catalog item price "
         "inventory shipment refund coupon review rating message thread channel").split()
SERVICES = ["checkout", "catalog", "auth", "search", "payments", "inventory", "notifications",
            "gateway", "recommendations", "shipping", "billing", "profile"]
METHODS = ["GET"] * 14 + ["POST"] * 4 + ["PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]
STATUSES = [200] * 60 + [201] * 4 + [204] * 3 + [301, 302, 304] * 3 + [400] * 2 + [401, 403] + [404] * 5 + [500, 502, 503]
UAS = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_6) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.2 Safari/605.1.15",
    "Mozilla/5.0 (X11; Linux x86_64; rv:141.0) Gecko/20100101 Firefox/141.0",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148",
    "curl/8.5.0", "Go-http-client/2.0", "python-requests/2.32.4",
    "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
    "Prometheus/3.2.1", "kube-probe/1.33",
]
JAVA_CLASSES = ["com.example.orders.OrderService", "com.example.orders.OrderRepository",
                "com.example.http.RequestFilter", "com.example.cache.RedisCacheManager",
                "com.example.payments.GatewayClient", "org.hibernate.engine.jdbc.spi.SqlExceptionHelper",
                "org.apache.kafka.clients.consumer.internals.ConsumerCoordinator",
                "com.zaxxer.hikari.pool.HikariPool", "org.springframework.web.servlet.DispatcherServlet"]
EXCEPTIONS = [("java.net.SocketTimeoutException", "Read timed out"),
              ("java.sql.SQLTransientConnectionException", "HikariPool-1 - Connection is not available, request timed out after 30000ms."),
              ("java.lang.IllegalStateException", "Order {id} is already in state SHIPPED"),
              ("java.lang.NullPointerException", "Cannot invoke \"String.length()\" because \"sku\" is null"),
              ("org.apache.kafka.common.errors.TimeoutException", "Topic orders-v2 not present in metadata after 60000 ms.")]


class Zipf:
    """Deterministic Zipf-like sampler over range(n)."""

    def __init__(self, rng, n, s=1.1):
        self.rng = rng
        w = [1.0 / (k + 1) ** s for k in range(n)]
        tot = 0.0
        self.cum = []
        for x in w:
            tot += x
            self.cum.append(tot)
        self.tot = tot

    def __call__(self):
        u = self.rng.random() * self.tot
        lo, hi = 0, len(self.cum) - 1
        while lo < hi:
            mid = (lo + hi) // 2
            if self.cum[mid] < u:
                lo = mid + 1
            else:
                hi = mid
        return lo


class World:
    def __init__(self, rng, n_hosts):
        self.rng = rng
        self.hosts = [f"{rng.choice(['web', 'app', 'db', 'cache', 'batch', 'edge'])}-{i:02d}.{rng.choice(['use1', 'euw1', 'aps2'])}.internal"
                      for i in range(n_hosts)]
        self.host_z = Zipf(rng, n_hosts, 0.8)
        self.users = [f"{rng.choice(WORDS)}{rng.randrange(10, 99999)}" for _ in range(5000)]
        self.user_z = Zipf(rng, len(self.users), 1.05)
        self.ips = [f"{rng.choice([10, 172, 192, 203, 198, 81, 94, 185])}.{rng.randrange(256)}.{rng.randrange(256)}.{rng.randrange(1, 255)}"
                    for _ in range(20000)]
        self.ip_z = Zipf(rng, len(self.ips), 1.0)
        self.paths = []
        for _ in range(1500):
            depth = rng.randrange(1, 5)
            segs = [rng.choice(WORDS) + ("s" if rng.random() < 0.3 else "") for _ in range(depth)]
            if rng.random() < 0.5:
                segs.insert(rng.randrange(len(segs) + 1), "{id}")
            self.paths.append("/" + rng.choice(["api/v1/", "api/v2/", "", "static/", "internal/"]) + "/".join(segs))
        self.path_z = Zipf(rng, len(self.paths), 1.15)
        self.svc_z = Zipf(rng, len(SERVICES), 0.9)

    def host(self):
        return self.hosts[self.host_z()]

    def user(self):
        return self.users[self.user_z()]

    def ip(self):
        return self.ips[self.ip_z()]

    def path(self):
        p = self.paths[self.path_z()]
        while "{id}" in p:
            p = p.replace("{id}", str(self.rng.randrange(1, 10 ** self.rng.randrange(2, 8))), 1)
        if self.rng.random() < 0.15:
            p += f"?page={self.rng.randrange(1, 50)}&sort={self.rng.choice(['asc', 'desc'])}"
        return p


def hexid(rng, nbytes):
    return "%0*x" % (nbytes * 2, rng.getrandbits(nbytes * 8))


def clock(rng, start, rate):
    t = float(start)
    while True:
        # diurnal modulation between 0.25x and 1.75x of the base rate
        day_frac = ((t % 86400) / 86400.0)
        r = rate * (1.0 + 0.75 * math.sin(2 * math.pi * (day_frac - 0.3)))
        t += rng.expovariate(max(r, 1e-3))
        yield t


def fmt_syslog_ts(t):
    tm = time.gmtime(t)
    return f"{MONTHS[tm.tm_mon - 1]} {tm.tm_mday:2d} {tm.tm_hour:02d}:{tm.tm_min:02d}:{tm.tm_sec:02d}"


def iso(t, ms=True):
    tm = time.gmtime(t)
    base = time.strftime("%Y-%m-%dT%H:%M:%S", tm)
    return base + (".%03dZ" % int((t % 1) * 1000) if ms else "Z")


def gen_syslog(rng, w, t):
    host = w.host().split(".")[0]
    pid = rng.randrange(300, 65000)
    k = rng.random()
    if k < 0.30:
        ip, port = w.ip(), rng.randrange(1024, 65535)
        m = rng.random()
        if m < 0.45:
            msg = f"sshd[{pid}]: Failed password for invalid user {rng.choice(['admin', 'test', 'oracle', 'ubuntu', 'git', 'pi', w.user()])} from {ip} port {port} ssh2"
        elif m < 0.75:
            msg = f"sshd[{pid}]: Accepted publickey for {rng.choice(['deploy', 'ops', 'backup', w.user()])} from {ip} port {port} ssh2: ED25519 SHA256:{hexid(rng, 16)}"
        elif m < 0.9:
            msg = f"sshd[{pid}]: pam_unix(sshd:session): session closed for user {rng.choice(['deploy', 'ops', 'backup'])}"
        else:
            msg = f"sshd[{pid}]: Received disconnect from {ip} port {port}:11: disconnected by user"
    elif k < 0.45:
        msg = f"CRON[{pid}]: ({rng.choice(['root', 'www-data', 'backup'])}) CMD ({rng.choice(['/usr/lib/php/sessionclean', 'run-parts --report /etc/cron.hourly', '/opt/backup/bin/snapshot --incremental', 'test -x /usr/sbin/anacron || run-parts /etc/cron.daily'])})"
    elif k < 0.62:
        up = rng.random() * 3000000
        m = rng.random()
        if m < 0.4:
            msg = f"kernel: [{up:12.6f}] [UFW BLOCK] IN=eth0 OUT= MAC={hexid(rng, 6)}:{hexid(rng, 6)}:08:00 SRC={w.ip()} DST=10.0.{rng.randrange(256)}.{rng.randrange(256)} LEN={rng.randrange(40, 1500)} TOS=0x00 PREC=0x00 TTL={rng.randrange(30, 250)} ID={rng.randrange(65536)} PROTO=TCP SPT={rng.randrange(1024, 65535)} DPT={rng.choice([22, 23, 80, 443, 3389, 5432, 6379, 8080])} WINDOW={rng.randrange(1024, 65535)} RES=0x00 SYN URGP=0"
        elif m < 0.7:
            msg = f"kernel: [{up:12.6f}] EXT4-fs (nvme0n1p{rng.randrange(1, 4)}): mounted filesystem {hexid(rng, 8)}-{hexid(rng, 2)}-{hexid(rng, 2)} r/w with ordered data mode. Quota mode: none."
        else:
            msg = f"kernel: [{up:12.6f}] TCP: request_sock_TCP: Possible SYN flooding on port {rng.choice([80, 443, 8443])}. Sending cookies.  Check SNMP counters."
    elif k < 0.8:
        unit = rng.choice(["logrotate.service", "apt-daily.service", "certbot.timer", "node-exporter.service", "docker.service", "systemd-tmpfiles-clean.service"])
        msg = f"systemd[1]: {rng.choice(['Starting', 'Started', 'Finished', 'Deactivated successfully:'])} {unit}{'' if rng.random() < 0.5 else ' - ' + rng.choice(['Rotate log files', 'Daily apt download activities', 'Cleanup of Temporary Directories'])}."
    elif k < 0.9:
        msg = f"sudo[{pid}]: {rng.choice(['ops', 'deploy'])} : TTY=pts/{rng.randrange(6)} ; PWD=/home/{rng.choice(['ops', 'deploy'])} ; USER=root ; COMMAND=/usr/bin/{rng.choice(['systemctl restart nginx', 'journalctl -u app -n 200', 'apt-get upgrade -y', 'docker ps'])}"
    else:
        qid = hexid(rng, 5).upper()
        msg = f"postfix/smtp[{pid}]: {qid}: to=<{w.user()}@{rng.choice(['example.com', 'example.org', 'mail.test'])}>, relay={rng.choice(['mx1', 'mx2'])}.example.net[{w.ip()}]:25, delay={rng.random() * 3:.2f}, delays=0.01/0/{rng.random():.2f}/{rng.random():.2f}, dsn=2.0.0, status=sent (250 2.0.0 OK)"
    return f"{fmt_syslog_ts(t)} {host} {msg}\n"


def gen_jsonl(rng, w, t):
    svc = SERVICES[w.svc_z()]
    status = rng.choice(STATUSES)
    level = "ERROR" if status >= 500 else ("WARN" if status >= 400 else rng.choice(["INFO"] * 9 + ["DEBUG"]))
    lat = round(rng.lognormvariate(2.5, 0.9), 3)
    obj = {
        "ts": iso(t), "level": level, "service": svc, "host": w.host(),
        "trace_id": hexid(rng, 16), "span_id": hexid(rng, 8),
        "msg": rng.choice(["request completed", "request failed", "cache miss", "upstream call", "db query", "auth ok"]),
        "http": {"method": rng.choice(METHODS), "path": w.path(), "status": status, "latency_ms": lat,
                 "bytes_out": rng.randrange(0, 200000)},
    }
    if rng.random() < 0.7:
        obj["user"] = {"id": w.user(), "tier": rng.choice(["free", "pro", "enterprise"])}
    if level == "ERROR":
        obj["error"] = {"kind": rng.choice(["Timeout", "ConnectionReset", "ValidationError", "Unavailable"]),
                        "retryable": rng.random() < 0.5, "attempt": rng.randrange(1, 4)}
    if rng.random() < 0.2:
        obj["db"] = {"statement": rng.choice(["SELECT", "UPDATE", "INSERT"]), "rows": rng.randrange(0, 500),
                     "duration_ms": round(rng.lognormvariate(1.0, 1.0), 3)}
    return json.dumps(obj, separators=(",", ":")) + "\n"


def gen_logfmt(rng, w, t):
    status = rng.choice(STATUSES)
    lvl = "error" if status >= 500 else ("warn" if status >= 400 else "info")
    caller = f"{rng.choice(['server', 'handler', 'store', 'client', 'queue'])}.go:{rng.randrange(20, 900)}"
    msg = rng.choice(["request handled", "upstream error", "retrying", "slow query", "flushed batch"])
    dur = rng.lognormvariate(2.2, 1.0)
    parts = [f"ts={iso(t)}", f"level={lvl}", f"caller={caller}", f'msg="{msg}"', f"svc={SERVICES[w.svc_z()]}",
             f"method={rng.choice(METHODS)}", f"path={w.path()}", f"status={status}", f"dur={dur:.3f}ms",
             f"request_id={hexid(rng, 12)}", f"remote={w.ip()}"]
    if msg == "flushed batch":
        parts.append(f"records={rng.randrange(1, 5000)} bytes={rng.randrange(100, 10_000_000)}")
    if lvl == "error":
        parts.append(f'err="{rng.choice(["context deadline exceeded", "connection refused", "EOF", "i/o timeout"])}"')
    return " ".join(parts) + "\n"


def gen_access(rng, w, t):
    tm = time.gmtime(t)
    ts = f"{tm.tm_mday:02d}/{MONTHS[tm.tm_mon - 1]}/{tm.tm_year}:{tm.tm_hour:02d}:{tm.tm_min:02d}:{tm.tm_sec:02d} +0000"
    status = rng.choice(STATUSES)
    size = 0 if status in (204, 304) else rng.randrange(120, 250000)
    ref = "-" if rng.random() < 0.6 else f"https://www.example.com{w.path()}"
    user = "-" if rng.random() < 0.95 else w.user()
    return f'{w.ip()} - {user} [{ts}] "{rng.choice(METHODS)} {w.path()} HTTP/{rng.choice(["1.1", "1.1", "2.0"])}" {status} {size} "{ref}" "{rng.choice(UAS)}"\n'


def gen_java(rng, w, t):
    tm = time.gmtime(t)
    ts = time.strftime("%Y-%m-%d %H:%M:%S", tm) + ",%03d" % int((t % 1) * 1000)
    cls = rng.choice(JAVA_CLASSES)
    thread = rng.choice(["http-nio-8080-exec-%d" % rng.randrange(1, 200), "main", "kafka-consumer-%d" % rng.randrange(1, 9),
                         "scheduling-1", "HikariPool-1 housekeeper"])
    r = rng.random()
    if r < 0.93:
        lvl = rng.choice(["INFO"] * 8 + ["DEBUG", "WARN"])
        msg = rng.choice([f"Processed order {rng.randrange(10**6, 10**7)} in {rng.randrange(1, 900)} ms",
                          f"Cache hit ratio {rng.random():.4f} over last 60s",
                          f"Committed offsets for partition orders-v2-{rng.randrange(12)}: {rng.randrange(10**8)}",
                          f"User {w.user()} logged in from {w.ip()}",
                          f"HikariPool-1 - Pool stats (total={rng.randrange(10, 50)}, active={rng.randrange(0, 10)}, idle={rng.randrange(0, 40)}, waiting=0)"])
        return f"{ts} {lvl:5s} [{thread}] {cls} - {msg}\n"
    exc, text = rng.choice(EXCEPTIONS)
    lines = [f"{ts} ERROR [{thread}] {cls} - Unhandled exception while processing request {hexid(rng, 8)}",
             f"{exc}: {text.replace('{id}', str(rng.randrange(10**6, 10**7)))}"]
    for _ in range(rng.randrange(6, 40)):
        c = rng.choice(JAVA_CLASSES)
        m = rng.choice(["invoke", "doFilter", "handle", "process", "execute", "call", "run", "lambda$submit$0"])
        lines.append(f"\tat {c}.{m}({c.rsplit('.', 1)[1]}.java:{rng.randrange(20, 1200)})")
    if rng.random() < 0.4:
        lines.append(f"Caused by: {rng.choice(EXCEPTIONS)[0]}: nested failure")
        lines.append(f"\t... {rng.randrange(5, 60)} more")
    return "\n".join(lines) + "\n"


PROFILES = {"syslog": gen_syslog, "jsonl": gen_jsonl, "logfmt": gen_logfmt, "access": gen_access, "java": gen_java}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    start = int(p.get("start_epoch", 1767225600))
    master = random.Random(args.seed)
    world = World(random.Random(master.getrandbits(64)), int(p.get("hosts", 24)))
    for spec in p["files"]:
        rel = spec["path"]
        gen = PROFILES[spec["profile"]]
        rng = random.Random(master.getrandbits(64))
        target = int(spec["bytes"])
        path = os.path.join(args.out, rel)
        os.makedirs(os.path.dirname(path) or args.out, exist_ok=True)
        ticks = clock(rng, start + rng.randrange(0, 3600), float(spec.get("rate", 5.0)))
        n = 0
        buf = []
        pending = 0
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            while n < target:
                line = gen(rng, world, next(ticks))
                buf.append(line)
                sz = len(line.encode("utf-8"))
                n += sz
                pending += sz
                if pending >= 1 << 20:
                    f.write("".join(buf))
                    buf.clear()
                    pending = 0
            f.write("".join(buf))


if __name__ == "__main__":
    main()
