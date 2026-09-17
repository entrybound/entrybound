# EXP-REMOTE-013: Real object-store and CDN path confirmation

| Field | Value |
|---|---|
| Status | **BLOCKED** |
| Blocked reason (exact) | (1) No object-store or CDN accounts: needs owner-provisioned buckets on at least Amazon S3 and Cloudflare R2 (optionally Google Cloud Storage and Azure Blob Storage) and one CDN distribution (for example CloudFront) in at least two regions, with credentials held and entered by the owner (agents may not enter credentials or create accounts); (2) owner approval to publish corpus-derived test archives (only tuning items whose manifest licence is `redistributable: true`) to those buckets; (3) internet egress from the measurement host and an owner-approved egress budget; (4) a single client vantage point (this host) is all that exists, so path diversity is limited to endpoint regions. Docker/containerd is not required. |
| Domain / program section | remote / §12 (§7 emulation validity; §35 claims) |
| decision_ids | DEC-ACC-063, DEC-ACC-041, DEC-ACC-046, DEC-ACC-043, DEC-ACC-015, DEC-ACC-045, DEC-ACC-047 |
| Decision type (provisional) | EMPIRICAL (real-path confirmation); DEC-ACC-047 has HUMAN_PARTICIPANTS blocker for its demand part |
| OD / HC (provisional until G-A) | OD-12 (M12.1-M12.4); HC-18 (real origins' behaviour against the client's refusal rules), HC-15 |
| Author / L7 / gates | As EXP-REMOTE-001. Designed now so remaining-unknowns can cite it; its record is instantiated when the blocking access exists. No `spec.yaml` is committed: endpoint URLs, regions and bucket names do not exist yet, and a spec with placeholder endpoints would pre-register nothing checkable. `make_spec.py` (to be written with access) generates the spec from `endpoints.json` supplied by the owner, and that commit precedes any run. |

## 1. Question

On real object stores and CDNs reached over the internet from this host, do the remote-domain conclusions reached with the calibrated emulator and the latency model hold — request counts and bytes (which should be identical), latency direction of the selected candidates (coalescing, discovery/revalidation, cache, concurrency), server protocol features (HEAD vs presigned GET, multi-range, suffix ranges, `If-Match`, ETag strength), rate and connection limits under concurrency, CDN range caching, and the benefit of native object-store APIs over presigned HTTP ranges?

## 2. Hypotheses

- **H1 (count identity).** Request and byte counts of the production client and prototypes against real endpoints equal those recorded against `ebr-origin` for the same archive and workload, except for requests caused by server behaviour differences (redirects, 403 on HEAD with GET-presigned URLs), which are enumerated.
- **H2 (model and ranking on a real path).** With RTT and throughput measured per endpoint (probe P1/P2 of EXP-REMOTE-001 run against the endpoint), the SS-RFC6928 model's per-operation latency agrees in direction with measured latency for every rank-stability pair RP-1..RP-5 (EXP-REMOTE-001), and no selected candidate of EXP-REMOTE-003/004/005/006 reverses against its reference by more than 1 band unit of `remote_task_latency_s`.
- **H3 (server features, DEC-ACC-015).** S3 GET-presigned URLs reject HEAD (so the strict HEAD contract fails on presigned S3 URLs), S3 does not honour multi-range requests with multipart responses, and R2/CloudFront behaviours differ in at least one feature; these are observations to record, not assumptions.
- **H4 (concurrency limits, DEC-ACC-043).** At least one endpoint throttles (429/503) or queues above some connection count ≤ 32 from one client, bounding the useful N.
- **H5 (CDN cache, DEC-ACC-045).** Repeated identical range requests through the CDN produce cache hits that reduce latency, while distinct ranges of the same object do not share cache entries.
- **H6 (DEC-ACC-047).** Native API range reads (S3 SDK GetObject with Range) cost the same number of round trips as presigned HTTP ranges plus signing CPU, so their benefit is credentials handling, not performance.
- **H0 / falsification.** Each Hi falsified by one counterexample to its statement in the endpoints measured.

## 3. Decisions informed

| Decision | Supplied here (when unblocked) | Otherwise |
|---|---|---|
| DEC-ACC-063 | Calibration of the chosen emulation method against a real remote endpoint for the same traces; rank stability of coalescing and concurrency candidates across emulation and a real path | EXP-REMOTE-001 arms A1-A3 only |
| DEC-ACC-041 | Real S3/R2/GitHub Releases/CDN latency for the claim configurations | claims restricted to `emulated-network` wording |
| DEC-ACC-046 | Real-path confirmation of the coalescing winner (harness review use constraint 7 alternative to the slow-start sensitivity analysis) | slow-start sensitivity analysis and A3 only |
| DEC-ACC-043 | Server/CDN rate-limit and connection-limit behaviour | emulated caps only |
| DEC-ACC-015 | Presigned URL HEAD vs GET signature behaviour; R2 and CloudFront compatibility rows | EXP-REMOTE-009 public anonymous probes |
| DEC-ACC-045 | CDN range-cache behaviour for repeated reads | none |
| DEC-ACC-047 | Benefit measurement of native APIs vs presigned HTTP ranges | demand survey is HUMAN_PARTICIPANTS (blocked) |

## 4. Candidates

- Client configurations: `status-quo` (production client), the tuning winners W of EXP-REMOTE-003 (coalescing), -004 (discovery/revalidation), -005 (cache, retrieval), -006 (concurrency; N ∈ {1, 4, 8, 16, 32} for the limit probe), `get-range-probe` and `final-url-pinning` (EXP-REMOTE-009) for presigned URLs, incumbent `zip-cd-range-reader` (EXP-REMOTE-012).
- Access modes: public-read URL; GET-presigned URL (1 h expiry); CDN URL; native API range read (S3 SDK, research script, owner-provided credentials via environment set by the owner).
- Endpoints (owner-provided `endpoints.json`): S3 near region and far region, R2, CDN in front of the S3 near bucket; optional GCS and Azure.

## 5. Corpus selectors

The E12 subset of EXP-REMOTE-003 restricted to items with `license.redistributable: true`, packed `balanced-plain` (plus the W archive variant if an ordering candidate won), and the ZIP incumbent archives of the same items; tuning split only for tuning analysis, validation items for a confirmatory look with the same restriction. Held-out: section 15.

## 6. Environment and platform requirements

Windows host or WSL client (WSL preferred for tooling parity; the host's network stack is shared), internet egress, owner-provided credentials entered by the owner, object-store and CDN accounts, egress budget. Measurement blocks are interleaved (randomized blocks across candidates and endpoints) and repeated in at least three time-of-day windows over two days to expose diurnal variation. No quiet-machine guarantee exists for the network path; the local guard (§4.4) still applies to the client host.

**Platform matrix.** Linux x86-64 (WSL2) or Windows x86-64 client; macOS: not applicable; ARM64: not required; network: internet egress to owner-provided cloud endpoints (BLOCKED); privileged: no; large storage: no; external accounts and credentials: owner only (BLOCKED); external review: no.

## 7. Commands (to be generated with access)

```sh
# owner: create buckets/distribution, set credentials in the environment, approve uploads
$PY research/experiments/EXP-REMOTE-013/upload.py --items research/experiments/EXP-REMOTE-013/items.txt --endpoints endpoints.json   # owner-run
$PY research/experiments/EXP-REMOTE-013/make_spec.py --endpoints endpoints.json --out research/experiments/EXP-REMOTE-013/spec.yaml       # committed before runs
$PY -m ebr run --spec research/experiments/EXP-REMOTE-013/spec.yaml --timing
$PY -m ebr verify-raw --spec research/experiments/EXP-REMOTE-013/spec.yaml && $PY -m ebr normalize --spec research/experiments/EXP-REMOTE-013/spec.yaml
$PY -m decide analyze --experiment EXP-REMOTE-013 --real-path
```

## 8. Seeds, warmups, repetitions

Seeds fixed in the generated spec at creation. Per (endpoint, candidate, item, workload): warmups 2, rounds min 10, step 10, cap 30, the §4.6 precision rule on the median paired log ratio vs `status-quo` within each time-of-day block; endpoint RTT/throughput probes before and after every block.

## 9. Metrics

`http_requests`, `http_bytes` (T-12, T-11; exact from client logs), measured `remote_task_latency_s` and `remote_random_entry_latency_s` (T-10, label `real-path`), modelled latency with per-endpoint measured RTT/throughput (co-evidence), `origin_protocol_safety` (binary, HC-18) per endpoint and access mode, `benign_false_refusal_fraction` over legitimate endpoint configurations (T-20), throttling events per N, CDN cache hit ratio (descriptive).

## 10. Normalization and statistical analysis

- H1 exact comparison of counts with EXP-REMOTE-002/003 logs.
- H2 per endpoint: paired log ratios within time-of-day blocks, exact order-statistic item intervals, corpus-level Welch-Satterthwaite intervals over the E12 groups; direction agreement with model and with emulated verdicts; any reversal beyond 1 band unit is counterevidence for the affected selection (and triggers §11 reopen rule 1 if the decision were already decided).
- Endpoints and time-of-day blocks are strata, never pooled for verdicts.

## 11. Practical significance

T-10, T-11, T-12, T-20 by reference.

## 12. Sensitivity analysis

Endpoint region (near/far), time-of-day block, access mode (public, presigned, CDN, native), connection model arm (HTTP/1.1 vs HTTP/2 where the endpoint negotiates it), client on WSL vs Windows host.

## 13. Expected negative results worth recording

- The strict HEAD contract fails on presigned S3 URLs (HEAD signature mismatch), a real availability gap for status-quo remote access.
- Real-path variance at far regions exceeds the T-10 band for single operations, leaving some comparisons `INCONCLUSIVE` at the repetition cap.
- CDN caching does not help distinct ranges.

## 14. Threats to validity

Single vantage point; uncontrolled internet cross traffic; provider behaviour changes over time (dated observations); egress budget may cap repetitions; public-read test objects may be subject to provider throttling unrelated to the candidates.

## 15. Held-out (Phase D placeholder)

If unblocked before the design freeze, the frozen claim configurations are rerun once on held-out redistributable items uploaded after Commit C (`decision-method.md` §5.5), report-only per §5.6.

## 16. Estimates

- Compute: about 20 machine-hours spread over at least two days (network-dominated).
- Disk / upload: about 10 GB uploaded per endpoint; local disk under 5 GB.
- Agent effort: judgment for endpoint setup coordination with the owner and analysis; execution scripted.
