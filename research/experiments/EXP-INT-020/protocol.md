# EXP-INT-020: Real cloud object stores, CDNs and sync clients as carriers and remote origins (BLOCKED)

| Field | Value |
|---|---|
| Status | **BLOCKED** |
| Blocked reason | Requires external service accounts and credentials (for example AWS S3 and CloudFront, Google Cloud Storage, Azure Blob Storage, Cloudflare R2 and CDN, OneDrive and Dropbox desktop sync clients), owner approval for spending, and entry of credentials, which agents may not perform (credentials must be supplied by the owner). Real network paths are also needed: only application-level emulation exists here (`sch_netem` absent; harness constraint R1-14 on slow start). Local S3-compatible behaviour is covered by the MinIO arm of EXP-INT-011; that emulation cannot show provider-specific metadata, ETag, range-caching or 200-fallback behaviour. |
| Unblocks when | the owner provisions test buckets, a CDN distribution and sync-client test accounts, supplies credentials through the environment (never typed by agents), and approves the cost |
| Domain | integrity (program §24, cross-reference remote §12 and ecosystem §32) |
| Decisions informed | DEC-INT-011, DEC-INT-034, DEC-CRY-073, DEC-INT-026, DEC-INT-010 |
| Gates, method, author | conventions C0 |
| Spec | none now: a spec is generated from EXP-INT-011's `spec.yaml` (carriers) and EXP-INT-016's remote arm when services exist, replacing `ebr-origin` with service endpoints |

## 1. Question

Through real object stores, CDNs and sync clients: which out-of-band carriers (parity sets,
receipts, wrapper indexes, markers, sidecar names, object metadata) survive upload, download,
server-side copy and sync; do CDNs and object stores exhibit the origin misbehaviours that HC-18
readers must refuse (weak or missing ETags, 200 whole-body fallback, stale cached ranges, revision
change mid-session, compressed transfer encodings); and what do remote signature verification and
remote whole-archive verification cost in requests and bytes on real paths?

## 2. Hypotheses and falsification

| ID | Hypothesis | Falsified by |
|---|---|---|
| H1 | Object-store user metadata survives single-object copy within a provider but not cross-provider copies with common tools; xattr and ADS markers never survive upload. | Markers survive upload. |
| H2 | At least one CDN configuration serves a 200 whole-body response or a stale cached range for a changed object; `origin_protocol_safety` of the status quo HTTP range source refuses it with a stable code. | No misbehaviour observed (then HC-18(b) evidence rests on emulation only), or a refusal failure (defect). |
| H3 | Measured request counts on real paths equal the counts from the emulated origin for the same operations (exact), so modelled latency transfers. | Any count difference not explained by provider redirects or retries. |

## 3. Candidates

As EXP-INT-011 (carriers for DEC-INT-011, DEC-INT-034), EXP-INT-016 (remote candidates for
DEC-CRY-073, DEC-INT-010) and EXP-INT-008 (fetch policies for DEC-INT-026).

## 4. Corpus

Archives packed from `f05-tuning-gutenberg-books`, `f19-tuning-zoo`, `f07-tuning-silesia-xray`
(plain, signed and encrypted), validation equivalents as in the referenced experiments.

## 5. Environment and platform requirements

Owner-provisioned accounts and buckets in two regions, one CDN distribution with configurable cache
policies, sync clients on the Windows host (D: only) and on a macOS host (EXP-INT-019), network
access from WSL and Windows, credentials via environment variables set by the owner.

## 6. Procedure and commands (designed)

Carrier matrix with provider tools (`aws s3 cp/sync`, `gsutil` or `gcloud storage`, `azcopy`,
`rclone`), CDN range-request probes through the EXP-INT-016 remote driver with exact request logs,
sync-client placement and retrieval of carriers.

## 7. Seeds, warmups, replication

Seeds reserved: 2026092001/02/03.
2 repetitions on different days (cache states differ; reported per repetition, not averaged).

## 8. Metrics

`carrier_survival_fraction` (C11), `origin_protocol_safety` (HC-18), `http_requests` (T-12),
`http_bytes` (T-11), `remote_task_latency_s` (T-10, measured on real paths, reported beside the
modelled value).

## 9. Normalization

Exact counts per provider and path; providers and network paths are evaluation conditions.

## 10. Statistical analysis and sensitivity

Counts exact; latency per §4.15 only where a profile-like path is characterized; sensitivity by
provider, region and cache policy.

## 11. Expected negative results worth recording

Metadata carriers lost across providers; CDN misbehaviours that the status quo must refuse.

## 12. Threats to validity

Provider behaviour changes over time (dates and API versions recorded); results are snapshots.

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12.

## 14. Cost estimates

About 10 CPU-hours locally; data transfer about 50 GB total; service costs to be estimated by the
owner. Agent effort: scripted execution after owner setup; analysis judgment.

## 15. Deviations (append-only)

None.
