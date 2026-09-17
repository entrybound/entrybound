# EXP-ECO-010: Authenticated hosting arms — publishing `.eb` to real object stores, CDNs and release hosts

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-010 |
| Domain | ecosystem (program §32; cross-cuts §12, §24) |
| Status | **BLOCKED** |
| Blocked by | (1) no cloud or hosting accounts (AWS S3 and CloudFront, Google Cloud Storage, Azure Blob Storage, Cloudflare R2, GitHub Releases on a writable repository); agents may not create accounts or authenticate, and publishing content requires owner approval; (2) Internet Information Services needs administrator rights on the Windows host, which this session does not have; (3) enterprise proxies and CDN caching tiers with private configuration are not available |
| Unblocked by | owner-provisioned, owner-authenticated test buckets or repositories with scoped credentials supplied through the environment (never typed by an agent), explicit owner approval to publish the test archives (synthetic content only), and an administrator session for IIS |
| Runner | the EXP-ECO-009 scripts with `--target real-<provider>` (no new tooling except upload helpers using the providers' official CLIs) |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |

## 1. Question

Do real hosting services behave like their local emulators for the remote-access requirements of the reader (HEAD, ETag strength, ranges, multipart ranges, `If-Match`, presigned-URL method binding, CDN cache revalidation, redirects), and do separation and staleness of companion files (signatures, sidecars, parity) occur when artifacts are published and fetched through those services?

## 2. Hypotheses and falsification

- **H1.** For each provider, the header behaviours recorded for its local emulator in EXP-ECO-009 (S10-S13) match the real service on the pre-registered request list. *Falsified* per provider by any differing status code or header semantics.
- **H2.** At least one real presigned-URL provider rejects HEAD on a GET-signed URL, so the status-quo strict HEAD policy fails on presigned hosting. *Falsified* if every provider accepts HEAD on GET-presigned URLs.
- **H3.** A CDN in front of an object store serves stale bytes for a replaced object within its cache TTL with an unchanged or weak validator in at least one configuration, and the reader's content verification detects it. *Falsified* if no staleness occurs or if staleness is not detected.
- **H4.** Uploading only the primary artifact through a provider's standard release workflow separates companion files silently (DEC-INT-011). *Falsified* if the workflow requires or reports the companions.

## 3. Decisions informed

DEC-ACC-015, DEC-ECO-057, DEC-INT-011.

## 4. Candidates

As EXP-ECO-009 §4 (client policies of DEC-ACC-015 with and without verification; publisher guidance candidates of DEC-ECO-057) and EXP-ECO-007 §4 for DEC-INT-011 binding variants, evaluated on real services: S3 (virtual-host and path style), S3 behind CloudFront, GCS (public and signed URL), Azure Blob (public and SAS), R2 (public bucket domain and presigned), GitHub Releases (asset download redirect chain), IIS on Windows (static file handler defaults).

## 5. Corpus

Synthetic content only: archives of generated trees (`text-v1` and `random-v1` generators, seeds 2026091710-1..4) plus the EXP-ECO-009 archive built from `f01-tuning-ripgrep-14-1-1-git` only if the owner approves publishing a public source tree archive. Validation look: a second set of generated trees (seeds 2026091710-5..8).

## 6. Environment and platform requirements

Owner-provisioned accounts and buckets per provider; scoped credentials in environment variables; outbound network; administrator Windows session for IIS; the WSL research VM as the client.

## 7. Procedure and commands

`research/tools/ecosystem/hosting/real_publish.sh <provider>` (official CLIs: `aws`, `gcloud storage`, `az storage`, `wrangler r2`, `gh release upload`) uploads the archive set and companions; `live_run.py --target real-<provider>` and `range_policy_probe.py` exactly as in EXP-ECO-009; `copy_channels.py --channel <provider-release-workflow>` from EXP-ECO-007; objects deleted at the end by the owner-held credentials.

## 8. Seeds, warmup, replication

Seeds `order` 2026091710, `bootstrap` 2026091710, `command` 2026091710; each probe twice on two different days; no warmups.

## 9-12. Metrics, aggregation, analysis, thresholds

As EXP-ECO-009 §9-§12 (`origin_protocol_safety` binary; `http_requests` T-12 and `http_bytes` T-11 within each provider condition; AGG-S by provider; no cross-provider averaging), and EXP-ECO-007 §9 for separation outcomes.

## 13. Sensitivity analysis

CDN cache TTL short versus default; HTTP/2 versus HTTP/1.1 client; regions nearest versus farthest (latency irrelevant; behaviour only).

## 14. Held-out (Phase D placeholder)

Conventions §8; if unblocked before the freeze, a fresh generated archive set with an owner-held seed is used after unlock.

## 15. Expected negative results worth recording

Emulator-real mismatches (e.g. ETag forms, multipart support), presigned HEAD failures, CDN staleness windows.

## 16. Threats to validity

Provider behaviour changes over time and by region and configuration; one bucket per provider is a thin sample.

## 17. Cost estimate

About 6 h machine time and about 10 GB transfer once unblocked; agent effort scripted; owner effort for provisioning and approval.

## 18. Gates and exposure

Conventions §1 and §2. This protocol exists so that `remaining-unknowns` can cite the exact missing access for DEC-ACC-015, DEC-ECO-057 and DEC-INT-011.
