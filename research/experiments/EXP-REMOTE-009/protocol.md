# EXP-REMOTE-009: Origin protocol safety, server compatibility, transient faults, and HTTP client features

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T3 origin fault modes and forward-proxy/TLS test origins; T4 `RetryingSource`, `RevalidateOnceSource`, pin-checking and redirect-policy client prototypes; server-matrix and public-endpoint probe scripts; feature-gated `ebound` build patch in a scratch clone for size/dependency counts) |
| Domain / program section | remote / §12 (§29 outcome classes, §31 dependency, §22.4 trust) |
| decision_ids | DEC-ACC-014, DEC-ACC-015, DEC-ACC-016, DEC-MOD-032, DEC-CRY-073 |
| Decision type (provisional) | EMPIRICAL (behaviour matrices, costs); FORMAL (outcome-class mapping, redirect/downgrade policy from pinned client source); EXTERNAL for the security review of redirect and plain-http policies and for licence interpretation, if any, of the HTTP stack (`decision-method.md` §8.3) |
| OD / HC (provisional until G-A) | OD-12 (M12.4 binary; M12.1 retry overhead), OD-17 (M17.3), OD-04 (M04.2), OD-22/OD-24 cost inputs (DEC-ACC-016); HC-18 (MVT-18(b) origin misbehaviour suite: the R1 screen for every remote-domain client candidate), HC-15 (distinguishable failure, nothing unverified reported verified), HC-05 (caller-owned policy: credentials and trust roots are caller inputs), HC-09 (verification never needs live infrastructure) |
| Author / L7 / gates | As EXP-REMOTE-001. This experiment also runs the MVT-18(b) screen for every protocol-changing candidate of EXP-REMOTE-003..006 and -010 (`revalidate-once-per-session`, `suffix-range-open`, speculative windows, persistent caches, multi-range and concurrent executors); a failure eliminates that candidate at R1 there. |

## 1. Question

Which origin behaviours must remote access require and which fallbacks are safe; which client features (timeouts and bounded retries, credentials and custom headers, proxies and custom trust roots, redirect and plain-http policy, URL redaction) are needed to survive transient faults and real hosting without masking revision changes or leaking credentials; how should environmental, transport and server faults map to outcome classes and reason codes; how do candidate client packagings compare in dependency and binary cost; and does plain-http remote access without a pin accept a substituted archive?

## 2. Hypotheses

- **H1 (HC-18 screen).** The status-quo client refuses every misbehaviour case of the suite (weak or missing ETag, 200 whole-body fallback, ETag or length change mid-session, short or mismatched 206, Content-Range disagreement, non-identity encoding) with a stable code and releases no bytes as verified: `origin_protocol_safety` = 1. Every candidate that keeps strong-validator pinning and `If-Match` also scores 1; `accept-200-whole-body`, `weak-etag-with-digest-verification` and `last-modified-validator` score 0 on at least one mixed-revision case.
- **H2 (DEC-ACC-014).** Under the transient-fault set (connection reset mid-body, 502/503, 429 with `Retry-After`, 5 s stall), `status-quo-minimal` fails (or hangs until an external timeout) on every case, while `timeouts-and-bounded-retries` completes every case with `benign_false_refusal_fraction` = 0, never retries across a 412 or ETag change, and adds at most the retried requests.
- **H3 (DEC-ACC-015 compatibility).** Ubuntu-packaged nginx, Apache httpd and lighttpd serving a static archive satisfy the strict HEAD contract; Python `http.server` does not (no Range) and is refused with `HttpRangeUnsupported`; public cloud objects probed anonymously differ in multi-range support.
- **H4 (DEC-MOD-032).** The status-quo mapping (I/O as `POLICY_REFUSED EB_IO`, transport faults and malformed framing as `CORRUPT EB_HTTP_RANGE_INVALID`) conflates distinct causes: `reason_code_specificity_fraction` < 1 over the fault case list, and `distinct-codes-existing-classes` reaches 1 without a seventh class.
- **H5 (DEC-CRY-073).** Over plain http, a proxy that substitutes a different valid archive for the whole session produces `OK` with no warning under the status quo; `require-pin-for-plain-http` refuses it.
- **H6 (DEC-ACC-016).** A build without the remote HTTP stack is smaller and has fewer decode-path dependencies with native code (counts from `cargo tree`), and a minimal HTTP/1.1 range client prototype passes the same H1/H3 suites.
- **H0 / falsification.** Each Hi falsified by one counterexample to its stated outcome (H6 falsified if the minimal client fails any suite case the status quo passes).

## 3. Decisions informed

| Decision | Supplied here | Remaining |
|---|---|---|
| DEC-ACC-014 | Failure-injection tests (resets, 5xx, stalls, 429) with and without retry; redirect-limit and cross-origin/downgrade behaviour (status quo from pinned reqwest source plus a TLS-capable test origin); plain-http acceptance; audit of URL echo (userinfo, query tokens) in text and JSON outputs and error messages | Private hosting auth survey: literature/adoption (ecosystem §32, no experiment); security review of redirect and plain-http policies: EXTERNAL |
| DEC-ACC-015 | Compatibility matrix across local nginx, Apache httpd, lighttpd, Caddy (if its package source is approved), Python http.server and ebr-origin; anonymous probes of public objects on GitHub Releases, S3, GCS, Azure Blob and a CloudFront-served public object (protocol features only); mixed-revision adversarial tests for each relaxed validator | IIS: BLOCKED (enabling IIS needs admin on the Windows host); R2 and presigned-URL HEAD vs GET signature behaviour: EXP-REMOTE-013 (BLOCKED, credentials) |
| DEC-ACC-016 | Binary size, direct/transitive dependency counts, native-code and `unsafe` presence for status quo vs feature-gated vs no-HTTP builds; parity of a minimal HTTP/1.1 range client on the suites; whether HTTP/3 is active at runtime (enabled features plus observation of UDP use against an h3-capable local server) | Advisory history, maintainer counts, licence and build portability audit: ecosystem domain (§31 dependency audit) |
| DEC-MOD-032 | Fault-injection census (disk full, permission denied, closed output pipe, connection reset, proxy mangling, server incompatibility) recording reported class and code; specificity of each candidate mapping | Usability review of diagnostics: HUMAN-FACING (§4.16 message census possible in ecosystem §33; human comprehension claims external) |
| DEC-CRY-073 | Substitution attack demonstration over plain http; refusal by pin candidates | Request and byte cost of remote signature verification: EXP-REMOTE-010; signature semantics: crypto domain |

## 4. Candidates

### 4.1 Server requirements and fallbacks (DEC-ACC-015)

| candidate_id | Definition | Screen status |
|---|---|---|
| `status-quo-strict-head` (reference) | HEAD with strong ETag, `Accept-Ranges: bytes`, exact Content-Length; ranges carry `If-Match` (`random_access.rs:180-357`) | measured |
| `get-range-probe` | if HEAD fails or lacks fields, a ranged GET establishes total length (Content-Range) and a strong ETag | measured (prototype) |
| `final-url-pinning` | resolve redirects once at open and pin the final URL for all later requests | measured (prototype) |
| `last-modified-validator` | accept Last-Modified + length as revision when no strong ETag | **proposed L0 exclusion**: HC-18's statement requires a weak or missing ETag to yield a stable refusal; exercised only to produce the counterexample artifact (mixed-revision case with unchanged Last-Modified second) for the independent reviewer |
| `weak-etag-with-digest-verification` | accept weak/no validator relying on archive digests with revision-change detection | **proposed L0 exclusion** (HC-18), exercised as above |
| `accept-200-whole-body` | accept 200 responses as whole-body fallback | **proposed L0 exclusion** (HC-18; `docs/http-range-access-v1.md` L20), exercised as above |

### 4.2 Client features (DEC-ACC-014)

`status-quo-minimal` (reference), `timeouts-and-bounded-retries` (connect 10 s, idle-read 30 s, at most 3 retries of idempotent ranged GETs with exponential backoff 0.5/1/2 s and seeded ±20% jitter, honours `Retry-After` up to 30 s, never retries after 412, ETag or length change), `auth-headers` (bearer or basic credentials and custom headers from caller configuration or environment; never logged), `proxy-and-ca-config` (honours `HTTP(S)_PROXY`/`NO_PROXY`; caller-supplied CA bundle), `refuse-downgrade-cross-origin` (refuse https→http and origin-changing redirects), `https-only-default` (refuse http:// unless explicitly allowed), `url-redaction` (redact userinfo and query in every report, trace and error). Each is evaluated alone and as the combined bundle `hardened-client`.

### 4.3 Outcome mapping (DEC-MOD-032)

`status-quo-mapping` (reference), `new-environment-class` (seventh class `ENVIRONMENT`), `distinct-codes-existing-classes` (`EB_IO_*`, `EB_TRANSPORT_*`, `EB_SOURCE_UNSTABLE` with documented classes), `unsupported-for-server-faults` (server incompatibility → `UNSUPPORTED`, transient faults → retryable qualifier). Mappings other than the status quo are evaluated as deterministic mapping tables applied to the observed fault causes (the cause of each injected fault is known by construction), so no implementation is needed for the census; `new-environment-class` changes the SPEC §20.5 outcome vocabulary (objective HC-15 rule 6) and is flagged for FORMAL review.

### 4.4 Packaging (DEC-ACC-016)

`status-quo-unconditional-reqwest` (reference), `cargo-feature-gate` (`remote-http` feature, measured default-on and default-off builds from a scratch clone patch that never touches the repository's `crates/`), `separate-crate` (dependency counts computed for the CLI+core with the HTTP source moved; equal to feature-gate-off for the core), `minimal-http11-client` (harness prototype over `std::net::TcpStream` + `rustls` implementing exactly the §4.1 contract), `caller-supplied-source-only` (core without any HTTP client).

### 4.5 Plain-http pin (DEC-CRY-073, substitution part)

`status-quo-no-signatures-remote` (reference), `require-pin-for-plain-http` (caller supplies expected PCI or LAI digest; checked before trusting metadata), `https-only-default` (shared with 4.2), `expected-pci-before-parse` (pin checked against the fetched footer/descriptor before parsing further). `verify-signatures-on-range-open` cost: EXP-REMOTE-010.

## 5. Corpus and case selectors

- Archives: 4 small tuning archives from EXP-REMOTE-002 (`balanced-plain`, drawn by seed from items with 10-1,000 regular files, ≥ 3 families) plus 1 `balanced-enc-bucketed` archive; a sibling archive packed from a seeded mutation of the same tree for substitution and mixed-revision cases.
- Case list (`cases.json`, committed before any run): MVT-18(b) misbehaviour cases M1-M12 (weak ETag, missing ETag, 200 fallback, ETag change between requests, length change, short 206 body, over-long body, Content-Range mismatch, `Content-Encoding: gzip`, chunked transfer without Content-Length, multiple Content-Length, stale-ETag served with changed bytes); transient cases T1-T6 (reset mid-body at 3 seeded offsets, 502, 503, 429 + Retry-After 2 s, 5 s stall, slow body at 1 KiB/s); redirect cases R1-R5 (3-hop chain, 4-hop chain, cross-origin, https→http via the TLS test origin, relative redirect); environment cases E1-E4 (output filesystem full on a 16 MiB loop-mounted ext4, permission denied on output directory, stdout closed after 1 byte, source file deleted mid-session for local sources); substitution S1-S2 (whole-session substitution, metadata-consistent substitution after open); auth cases A1-A2 (origin requires bearer; credentials in URL userinfo and query).
- Servers: `ebr-origin` (all cases), Ubuntu-packaged nginx, apache2, lighttpd (static file service, default configuration, then with compression modules enabled as a variant), Python `http.server`, Caddy (only if the owner approves its third-party apt source, otherwise recorded as unmeasured); a forward proxy (`tinyproxy`, Ubuntu package) for proxy cases; a TLS test origin (nginx with a research-generated self-signed CA used only by prototype clients configured with that CA; the status-quo client's behaviour for TLS cases is derived from pinned reqwest/rustls source and recorded as FORMALLY_DERIVED because it cannot be given a custom root).
- Public endpoints (probe arm P): 5 existing, publicly readable objects named in `public-endpoints.json` before the run (one each on GitHub Releases, Amazon S3, Google Cloud Storage, Azure Blob Storage and a CloudFront-served host); anonymous HEAD, single-range, suffix-range, multi-range and `If-Match` requests only; at most 20 requests per endpoint; no credentials; no uploads.

## 6. Environment and platform requirements

`wsl-ubuntu` root for local servers (apt packages from the Ubuntu archive under the program's download approval), loop-mounted small ext4 image for E1, `tinyproxy`; Windows host for the E-cases of the Windows CLI build (disk full via a small VHD is BLOCKED: VHD creation needs admin; permission-denied and closed-pipe cases run); **internet egress** for probe arm P (read-only anonymous requests to public objects; if egress is not permitted in the execution session, arm P is recorded as not run and DEC-ACC-015 cloud evidence routes to EXP-REMOTE-013). IIS: BLOCKED (admin). No timing except retry wall time (informational; bounded retries are judged by counts and outcomes).

**Platform matrix.** Linux x86-64 required (WSL2, root inside the VM for local servers and a loop-mounted filesystem); Windows x86-64 for environment fault cases (IIS and VHD disk-full BLOCKED: admin); macOS: not applicable; ARM64: not required; network: loopback plus optional internet egress for anonymous public-object probes; large storage: no; external review: yes for redirect/plain-http security policy.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound; PY=/root/eb-research/venv/bin/python; export PYTHONPATH=research/tools
python research/orchestration/disk_guard.py && python research/harness/lock_parity.py
bash research/experiments/EXP-REMOTE-009/setup_servers.sh          # nginx/apache2/lighttpd/tinyproxy configs under /root/eb-research/servers (no system service enablement)
$PY -m ebr run --spec research/experiments/EXP-REMOTE-009/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-REMOTE-009/spec.yaml && $PY -m ebr normalize --spec research/experiments/EXP-REMOTE-009/spec.yaml
$PY research/experiments/EXP-REMOTE-009/probe_public_endpoints.py --endpoints research/experiments/EXP-REMOTE-009/public-endpoints.json \
    --out research/raw/EXP-REMOTE-009/public-probe.jsonl                 # arm P, only with egress
bash research/experiments/EXP-REMOTE-009/build_packaging_variants.sh      # scratch clone at the pinned commit; cargo tree/cargo metadata counts; binary sizes
$PY research/experiments/EXP-REMOTE-009/redaction_audit.py --raw research/raw/EXP-REMOTE-009 --out research/normalized/EXP-REMOTE-009/decision/redaction.json
$PY -m decide analyze --experiment EXP-REMOTE-009
```

The misbehaviour screen for other experiments' candidates runs through the same spec with `--candidates-from research/experiments/EXP-REMOTE-00{3,4,5,6}/strategies.json` (generated `spec-screen.yaml`).

## 8. Seeds, warmups, repetitions

Seeds `order` 20261011, `bootstrap` 20261012, `command` 20261013 (fault offsets, backoff jitter). Behaviour cases are deterministic outcomes: 3 repetitions per (case, candidate, server) — a case whose outcome differs across repetitions is itself a finding (nondeterministic failure handling) and is committed as an artifact; public probes: 1 repetition (rate courtesy), recorded with UTC time.

## 9. Metrics

| Metric | OD | Role | Threshold |
|---|---|---|---|
| `origin_protocol_safety` (all M-cases refused with the stable code, zero bytes released as verified) | OD-12 | binary (HC-18, R1 screen) | §3.3 |
| `benign_false_refusal_fraction` over transient cases T1-T6 plus compliant-server cases | OD-17 | **primary** (DEC-ACC-014, DEC-ACC-015) | T-20 |
| `reason_code_specificity_fraction` over all injected fault causes | OD-04 | **primary** (DEC-MOD-032) | T-20 |
| `wrong_bytes_reported_verified` | – | binary (HC-15) | §3.3 |
| `http_requests` added by retries per completed case | OD-12 | secondary | T-12 |
| `substitution_accepted` (S1, S2) | – | reported (DEC-CRY-073); a candidate that claims pinning and accepts is eliminated | – |
| `credential_echo_count` (userinfo, query tokens or auth headers appearing in any output, trace or error) | – | secondary (non-canonical; secondary for every decision per §0.2 item 4) | – |
| `decode_path_dependency_count`, `decode_path_native_unsafe_count`, binary bytes of `ebound` | OD-22 / OD-24 | cost inputs (C3) | – |
| `compat_pass` per (server, candidate) and public-endpoint feature flags | OD-12 | descriptive | – |
| `http3_used` (UDP traffic or h3 ALPN observed) | – | descriptive | – |

## 10. Normalization and statistical analysis

- All outcome metrics are deterministic coverage counts over fixed case lists (AGG-C, minimum over case families as headline); no intervals; a one-case difference exceeds the T-20 band.
- R1: any candidate with `origin_protocol_safety` = 0 or `wrong_bytes_reported_verified` > 0 is eliminated (and, for proposed L0 exclusions, the failing case is the counterexample artifact given to the independent reviewer).
- Survivors compared by R4 on the primaries; cost tiers (independent assessment): timeouts/retries T0-T1; new CLI/config options T2 (C6); new reason codes T2; a seventh outcome class touches SPEC §20.5 vocabulary (at least T2, possibly higher, FORMAL); packaging changes C3-driven.
- Public probe arm P results are descriptive observations of third-party behaviour at a date (EXTERNALLY observed, EMPIRICALLY_MEASURED at access time), not decision-grade performance data.

## 11. Practical significance

T-20 (both primaries) and T-12 by reference; binaries §3.3; cost inputs feed §7 tiers only.

## 12. Sensitivity analysis

Retry parameter variants (1 and 5 retries; backoff ×2); server compression modules enabled (weak ETags expected on some servers); redirect chain length 3 vs 4; transient fault rate on every request vs once; E-cases on Windows CLI build vs WSL.

## 13. Expected negative results worth recording

- Status quo has no timeouts: a stalled server can hang a read indefinitely (from code; H2 measures).
- Some real servers serve weak ETags when compression modules are enabled, so strict HEAD refuses legitimate hosting configurations (availability cost of HC-18).
- Anonymous S3 objects do not return multipart responses for multi-range requests (to be observed, not assumed).
- Plain-http substitution is accepted without a pin (H5), a documented but unmitigated risk.
- Redaction audit finds URL echo in error messages.

## 14. Threats to validity

- Case lists are finite enumerations (regression evidence).
- Local servers in default configurations may differ from production deployments; public probes capture one moment of third-party behaviour.
- TLS behaviour of the status-quo client is source-derived, not observed, because it cannot trust a research CA without code changes.
- Packaging size and dependency counts come from a research patch whose feature boundaries may differ from a real implementation.

## 15. Held-out (Phase D placeholder)

Behaviour suites are not corpus-dependent; at Phase D the frozen client candidates are rerun once against the same case list and servers plus held-out archives as payloads (§5.5), report-only per §5.6; public probes are repeated with fresh UTC stamps.

## 16. Estimates

- Compute: about 4 machine-hours (case matrix × servers × candidates is small; stalls and backoffs dominate wall time; packaging builds about 1 h).
- Disk: under 5 GB (server roots, loop image, build outputs in the scratch target directory on D:/WSL ext4).
- Agent effort: scripted; judgment for case-list completeness, L0 counterexample packaging for independent review, FORMAL redirect-policy source reading and analysis.
