# EXP-ECO-009: Hosting, CDN and object-store range-behaviour compatibility

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-009 |
| Domain | ecosystem (program §32; cross-cuts §12 remote access) |
| Status | READY (local servers and emulators installable; public read-only probes need only network; authenticated arms are EXP-ECO-010, BLOCKED) |
| Runner | `spec.yaml` (server configuration × item, local); `public_probe.py` script for the public read-only arm |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |
| Timing | none (request and byte counts are exact; latency is the remote domain's) |

## 1. Question

Which HTTP server, CDN and object-store behaviours that publishers actually get (HEAD support, strong or weak ETags, `Accept-Ranges`, exact `Content-Length`, `If-Match`/`If-Range`, multipart ranges, redirects, compression of range responses, presigned-URL method binding) does the implemented remote reader accept, refuse or mishandle; how would each relaxed-validator policy candidate of DEC-ACC-015 behave on the same transcripts, including under mixed-revision attacks; what must a publisher configure (DEC-ECO-057); and is HTTP/3 ever used at runtime (DEC-ACC-016)?

## 2. Hypotheses and falsification

- **H1 (strict status quo is incompatible with some mainstream hosting).** The status-quo `strict-head` policy refuses at least one server or emulator configuration that serves ranges correctly (candidates: weak ETags by default, HEAD not signed for presigned URLs, missing ETag). *Falsified* if the status quo accepts every configuration that serves correct ranges.
- **H2 (relaxed validators need content verification).** Every relaxed policy (`last-modified-validator`, `weak-etag-with-digest-verification`, `accept-200-whole-body`, `get-range-probe`) is safe against the mixed-revision cases only when the reader verifies content digests before reporting bytes as verified; `origin_protocol_safety` fails for any relaxed policy evaluated without verification. *Falsified* if a relaxed policy without verification passes every mixed-revision case, or a policy with verification fails one.
- **H3 (public CDNs vary).** Among the pre-registered public hosting categories, at least two differ in at least one of: ETag strength, HEAD/GET header parity, multipart range support, `If-Match` enforcement. *Falsified* if all categories behave identically on those four.
- **H4 (no HTTP/3 at runtime).** With a server that advertises HTTP/3 (`Alt-Svc`), the status-quo reader still uses HTTP/1.1 or HTTP/2. *Falsified* if the server log shows an HTTP/3 request.

## 3. Decisions informed

DEC-ACC-015, DEC-ECO-057, DEC-ACC-016 (runtime HTTP/3 question).

## 4. Candidates

- **Client policies (DEC-ACC-015):** `status-quo-strict-head` (the implemented reader, run live); `get-range-probe`, `last-modified-validator`, `weak-etag-with-digest-verification`, `final-url-pinning`, `accept-200-whole-body` — evaluated by `range_policy_probe.py`, a reference implementation of each policy's acceptance rules replayed on the recorded request/response transcripts of the live run plus the probe requests the policy itself would issue (issued live against local servers). Each policy is run with and without per-range content verification.
- **Publisher guidance (DEC-ECO-057):** `status-quo-url-commands`; `hosting-checklist-and-probe` (the probe script as a publisher-side checker: steps and flags counted); `explicit-full-download-fallback` (policy `accept-200-whole-body` with bounded size); `guidance-only` (no measurement beyond the matrix). `media-type-registration` is EXP-ECO-012.
- **Server configurations (local, the evaluation conditions):**

| Id | Server | Configuration |
|---|---|---|
| S01 | nginx (apt) | default static serving |
| S02 | nginx | `gzip on` for all types including `application/octet-stream` |
| S03 | nginx | `etag off` |
| S04 | Apache httpd (apt) | default; `FileETag` default |
| S05 | Apache httpd | `mod_deflate` on everything |
| S06 | Caddy (pinned release binary) | default file server (HTTP/1.1 and HTTP/2) |
| S07 | Caddy | HTTP/3 enabled, TLS with a scratch CA; the client trusts it through `SSL_CERT_FILE` if the reader honours it, otherwise inside a private mount namespace (`unshare -m`) in which the VM trust bundle is bind-mounted with the scratch CA appended (no persistent trust change on the VM or the Windows host) |
| S08 | lighttpd (apt) | default |
| S09 | Python `http.server` | default (no range support) |
| S10 | MinIO (pinned) | anonymous public-read bucket |
| S11 | MinIO | presigned GET URL (HEAD not covered by the signature) |
| S12 | Azurite (pinned npm package) | blob with SAS token |
| S13 | fake-gcs-server (pinned release) | public object |
| S14-S21 | `ebr-origin` behaviour modes | strict; weak ETag; no HEAD (405); 200 whole body; ETag churn between requests (mixed revision); 302 redirect to another port; multipart ranges refused (single ranges only); `Content-Length` off by one on HEAD |

Exclusions: IIS (needs admin on Windows), real S3/CloudFront/GCS/Azure/R2/GitHub-Releases uploads and presigned URLs from real providers (accounts; EXP-ECO-010 BLOCKED).

## 5. Corpus

Archives produced by dev-HEAD `ebound pack --profile balanced --layout indexed` from tuning items `f01-tuning-ripgrep-14-1-1-git`, `f04-tuning-sourcelike` (many entries: metadata fetch), `f18-tuning-lz4-releases-1-9-to-1-10`, `f07-tuning-silesia-mr` (one large entry); and one encrypted archive of `f01-tuning-ripgrep-14-1-1-git` (X-Wing recipient in scratch). Validation look: `f01-validation-redis-7-2-16-git`, `f04-validation-objstore`, `f18-validation-cjson-releases`. Operations per archive: `ebound list <URL>`, `ebound inspect <URL> --json`, `ebound read <URL> <path>` for three pre-registered paths (first, middle and last entry in logical order).

Public read-only arm: pre-registered hosting **categories**, each resolved to one concrete URL by `public_probe.py --resolve` at run time from a public index, and the resolved list committed before any probe request: (P1) GitHub release asset of `BurntSushi/ripgrep`'s latest release via the public releases API; (P2) an object listed in the AWS Open Data registry; (P3) an object in a Google Cloud public dataset bucket; (P4) an Azure Open Datasets blob; (P5) a PyPI file on `files.pythonhosted.org` for the package `requests`; (P6) an npm tarball on `registry.npmjs.org` for `left-pad`; (P7) a crate file on `static.crates.io` for `serde`; (P8) an Alpine package on `dl-cdn.alpinelinux.org`; (P9) a Debian package on `deb.debian.org`; (P10) a SourceForge download redirect. A category whose index cannot be resolved is recorded as unresolved.

## 6. Environment and platform requirements

WSL2 Ubuntu 24.04 (`wsl-ubuntu`), servers bound to 127.0.0.1 only; downloads of pinned server binaries and packages (approved); outbound HTTPS for the public arm (≤ 10 requests and ≤ 256 KiB per public object, `User-Agent` naming the research project without personal data). Not privileged.

## 7. Procedure and commands

Scripts in `research/tools/ecosystem/hosting/` (to be written):

1. `servers.sh up <Sxx>` / `down` — start a configuration with a committed config file under `research/experiments/EXP-ECO-009/servers/`; record server version and config SHA-256.
2. `capture_proxy.py` — a transparent recording HTTP proxy (HTTP/1.1 and HTTP/2 via `h2`/`mitmproxy`-free stdlib + `h11`; for TLS configurations the server log is the record) writing request/response header transcripts.
3. `live_run.py --server <Sxx> --archive <path> --ebound <bin>` — publish the archive to the server, run the operations of §5 through the proxy, record outcome class, reason code, exit status, stderr, bytes read and whether the reader reported the bytes as verified; compare bytes with the local archive entries.
4. `range_policy_probe.py --transcripts <dir> --policy <id> --verify on|off` — replay and live-probe each policy; for S18 (ETag churn) and a crafted mixed-revision server (two archive revisions swapped between range requests), record whether mixed-revision bytes would be delivered as verified.
5. `http3_check.sh` — S07 with Alt-Svc; run `ebound read`; parse Caddy access log protocol field.
6. `public_probe.py --resolve` then `public_probe.py --probe` — HEAD, `GET Range: bytes=0-65535`, multipart `bytes=0-99,200-299`, `If-Match` with the returned ETag and with a modified ETag, `If-Range`, range beyond end; record status codes and headers (ETag form, Accept-Ranges, Content-Length parity between HEAD and GET, Content-Encoding on range responses, redirect chain, cache headers).
7. `ebr run --spec spec.yaml`; `verify-raw`; `normalize`; `hosting_matrix.py` writes `research/normalized/EXP-ECO-009/{live,policies,public}.csv`.

## 8. Seeds, warmup, replication

Seeds `order` 2026091709, `bootstrap` 2026091709, `command` 2026091709. Request and byte counts are deterministic: 2 repetitions per (server, archive) + comparison of transcripts (differences other than dates and connection identifiers make the cell nondeterministic and are reported). Public probes run once per object on two different days (recorded as two observations; differences reported as volatility). No warmups.

## 9. Metrics and measurement

| Metric | Canonical | Role |
|---|---|---|
| `origin_protocol_safety` | yes (HC-18, M12.4) | binary per (policy, verification, server) |
| `http_requests` | yes (T-12, M12.1) | banded; primary for policy comparisons within a server condition |
| `http_bytes` | yes (T-11, M12.2) | banded; secondary |
| `error_actionability_fraction` | yes (T-20, M25.3) | for refusals (rubric EXP-ECO-015 §9) |
| `flags_per_task`, `migration_steps_count` | yes (C6, M26.2) | publisher checklist and probe steps |
| `hosting_accept`, `header_features`, `http_protocol_used` | no | record fields |

## 10. Normalization and aggregation

AGG-S: each server configuration and each public category is an evaluation condition; headline is the worst condition; no averaging across conditions. Policy comparisons are made within a condition on exact counts.

## 11. Statistical analysis and use in the decision rule

Deterministic outcomes. `origin_protocol_safety` = 0 eliminates the policy-with-verification-setting pair at R1 (HC-18). Among safe policies, R4 on `http_requests` per condition with the T-12 band (exact counts: item verdicts compare exact values against band and floor, §4.13); scoped winners per hosting condition are considered at R9 only if the product can express the scope (e.g. a probe-driven fallback). Public-arm results are `EXTERNALLY_SOURCED` observations at the access date and carry no decision weight beyond describing the conditions.

## 12. Practical-significance thresholds

T-11, T-12, T-20 as defined in §3.2 and Appendix A.2; binary per §3.3.

## 13. Sensitivity analysis

Conditions with TLS versus without; HTTP/2 versus HTTP/1.1 where the server supports both; `ebr-origin` modes with the recording proxy removed (proxy influence check on counts).

## 14. Held-out (Phase D placeholder)

Conventions §8; the live matrix is re-run once on held-out archives after unlock. Public categories are re-resolved after the freeze (fresh objects) and reported beside the tuning-time observations.

## 15. Expected negative results worth recording

Presigned URLs rejecting HEAD; CDNs returning weak ETags or gzip-encoded range responses; Python's `http.server` lacking ranges (a common "quick hosting" path failing); the status quo refusing hosting that serves correct ranges.

## 16. Threats to validity

Emulators (MinIO, Azurite, fake-gcs-server) are not the real services; public-arm observations are volatile and cover one object per category; the recording proxy may alter connection reuse; TLS certificate handling uses a test trust file, not host trust.

## 17. Cost estimate

About 4 h compute; about 5 GB disk (server binaries, archives, transcripts); agent effort scripted plus judgment to author the server configuration files and review `range_policy_probe.py` against the ledger candidate descriptions (one session, candidate owner review).

## 18. Gates and exposure

Conventions §1 and §2. Applicable families F01, F04, F07, F18; F04 is in the exposure record; analysis by an unexposed session.
