# EXP-REMOTE-007: Verified byte-range proof granularity and verification-structure remote cost

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T9 `proofsim.py` structure encoders and proof planners, T5 `fetchsim` coalescing over proof layouts, T2, T1 `layout`; hashing microbenchmark binary `ebr-remote hashbench`) |
| Domain / program section | remote / §12 (with §11 container, §24 integrity, §22.1 model/crypto) |
| decision_ids | DEC-ACC-024, DEC-ACC-040, DEC-INT-029, DEC-CON-020, DEC-MOD-007, DEC-MOD-034 |
| Decision type (provisional) | EMPIRICAL for costs; FORMAL for soundness of each binding (DEC-MOD-007) and for authority when flat digests and a tree disagree (DEC-INT-029); EXTERNAL review for security of proof bindings and length-exposure ordering (DEC-ACC-024, `decision-method.md` §8.3) |
| OD / HC (provisional until G-A) | OD-11 (M11.1-M11.3), OD-12 (M12.3), OD-10 (M10.2 amplification), OD-08 (streaming-output memory), OD-05 guard (outboard bytes; container domain primary); HC-15 (a range is reported verified only against its authority), HC-10 (Merkle construction rules: domain separation, no padding, largest power of two split, length bound), HC-16 (new structures are new identifiers) |
| Author / L7 / gates | As EXP-REMOTE-001. Consumes EXP-REMOTE-002 layouts and WL-RANGE status-quo traces. The FORMAL soundness sketches and the external-review dossier are outside this experiment (integrity and crypto domains), but every structure measured here is paired with the binding it assumes so those reviews apply to exactly these candidates. |

## 1. Question

For a verified read of a logical byte range [a, b) inside a ContentObject, how many proof bytes, fetched bytes, requests, digest operations and how much modelled remote latency and client memory does each proof granularity and verification structure cost, at Chunk counts from 10^3 to 10^9, across range-size strata and network profiles; how cacheable are proofs across ranges; and what do intra-entry range reads and streaming output cost relative to today's whole-object verification?

## 2. Hypotheses

- **H1 (DEC-ACC-024, DEC-INT-029).** For the 1 B, 4 KiB and 1 MiB strata inside objects of ≥ 64 MiB, every sub-object structure (per-Chunk digest with leaf list, Merkle outboard, Bao slice) is `DISTINGUISHABLE_BETTER` than `status-quo-whole-object` on `verified_range_fetch_bytes` by at least the T3 minimum-gain band (a new section type, §7.3), because the status quo fetches the whole object.
- **H2 (structure ranking).** Among sub-object structures, the flat leaf list costs O(n) proof bytes per object and loses to O(log n) outboards on `verified_range_fetch_bytes` above a crossover Chunk count n× per object that lies between 10^3 and 10^5 Chunks at production Chunk sizes; below n× the flat list is `EQUIVALENT` or better on `remote_task_latency_s` because it needs no extra proof request.
- **H3 (layout, DEC-CON-020).** Paged subtree blocks aligned to 64 KiB have the highest `proof_bytes_reused_fraction` for WL-RANGE sequences within one object and need at most one extra request per range at NP-INTER after coalescing; Bao pre-order and combined interleaving need fewer requests but lower reuse.
- **H4 (DEC-ACC-040).** Chunk-verified streaming output bounds client memory by the largest Chunk plus its dependency closure (formula) and reduces time to first verified byte for ≥ 256 MiB entries by at least 1 band unit at every profile; stored logical offsets remove O(n) prefix-sum work that at 10^6 Chunks is below the T-08 in-process floor (so offsets are not needed for CPU, only for request planning).
- **H5 (DEC-MOD-034, proof leg).** k-ary trees with k = 16 cut proof depth but raise proof bytes per level so that binary trees are `NON_INFERIOR` on `verified_range_fetch_bytes` at every n; tree-native BLAKE3/Bao needs fewer digest operations per verified MiB than SHA-256 structured trees (microbenchmark), which matters only for CPU, not remote bytes.
- **H0 / falsification.** Each Hi falsified by the absence of its stated verdict or relation in its stated scope; H2 additionally if no crossover exists in [10^2, 10^7].

## 3. Decisions informed

| Decision | Supplied here | Remaining |
|---|---|---|
| DEC-ACC-024 (V1_BLOCKING) | Proof bytes, CPU (digest operations × measured per-operation cost), requests and cacheability per verified range per design over CDC Chunk lists at 10^3-10^7 Chunks | Security review of binding each proof to signed PCR/LAI and of length-exposure ordering (Bao final-chunk rule): EXTERNAL (crypto dossier); use-case study of sub-file range demand: literature/adoption (ecosystem, no experiment); wire-cost analysis: container domain + FORMAL tier assessment |
| DEC-ACC-040 | Latency and memory for multi-GiB entries per candidate (status quo measured on large tuning entries over HTTP; streaming output by formula and prototype-free simulation); offset-resolution cost with and without stored logical offsets at 10^6 Chunks (bytes and in-process CPU) | Report semantics of partial-object verification: integrity domain (HC-15) and the FORMAL/API review |
| DEC-INT-029 (V1_BLOCKING) | Bytes, requests, RAM and CPU per verified range at 10^6-10^9 Chunks (10^8-10^9 computed only) for each structure; proof size and cacheability under signature | Disagreement corpus (flat vs tree) with reader outcomes: integrity/conformance domains; specification and implementation complexity (C1/C2 cost scripts, §14 item 7) |
| DEC-CON-020 | Proof bytes per verified range and verification CPU at 10^3-10^7 Chunks; range request counts; cacheability (block alignment); bytes fetched per byte verified; outboard size as a fraction of archive size across Chunk sizes | Rebuild cost and independent-implementation reproduction of roots: container and conformance domains |
| DEC-MOD-007 | Proof bytes and request count for remote range reads under each root binding; wire cost per ContentObject at large entry counts | Formal soundness sketch against truncated/extended lists: FORMAL (model domain) with independent sign-off |
| DEC-MOD-034 | Proof size and digest-operation counts for random ranges at 10^3-10^7 Chunks per construction (including k-ary and hashsplit trees); hashing throughput microbenchmark including parallel leaf hashing | Second-preimage and domain-separation analysis: FORMAL + EXTERNAL; independent-implementation root reproduction: conformance domain |

## 4. Candidates

Each candidate is a (structure, physical layout, binding) triple; the byte encoding of every node and record is declared in `proofsim.py`'s committed layout description (field widths, digest sizes: 32 B for SHA-256 and BLAKE3; lengths 8 B), so sizes are exact for the declared encoding.

### 4.1 Structures

| candidate_id | Source decisions | Definition |
|---|---|---|
| `status-quo-whole-object` (reference) | ACC-024, ACC-040, INT-029, CON-020 (`no-outboard-status-quo`), MOD-034 (`external-rfc6962-status-quo`) | fetch every Chunk of the ContentObject, verify each digest, recompute `chunk_root` from the full leaf list (largest-power-of-two split, domain-labelled SHA-256), release bytes after whole-object verification |
| `per-chunk-digest-with-leaf-list` | ACC-024, INT-029 (`two-stage-index-then-chunk` shares its cost profile), MOD-034 `flat-digest-list` | fetch the object's full ordered leaf list (already in the Manifest ContentObject record), verify the root, then fetch and verify only covering Chunks (plus their dependency closures) |
| `subrange-with-leaf-list` | ACC-040 | as above, releasing only [a, b) |
| `merkle-outboard-balanced` | ACC-024, INT-029, CON-020, MOD-007 `merkle-outboard-with-signed-count` | binary Merkle over Chunk digests in a MERKLE_OUTBOARD section; proof = O(log n) sibling digests |
| `merkle-outboard-k<k>`, k ∈ {4, 16} | MOD-034 `k-ary-tree` | k-ary variant |
| `bao-slice` / `blake3-bao` | ACC-024, INT-029 | BLAKE3 tree over fixed 1 KiB leaves (Bao), slice = leaves covering [a, b) plus parent nodes; independent of CDC Chunks; payload stored uncompressed in the slice sense, so the stored-bytes guard is reported from the container domain |
| `bao-combined-interleaved` | CON-020 | Bao combined encoding (tree nodes interleaved with payload) |
| `segmented-proofs` | ACC-024 | per-encryption-segment proofs aligned to segment boundaries (cost computed on `balanced-enc-bucketed` layouts) |
| `sizes-sister-list-dag` | ACC-024, INT-029 | UnixFS-style nodes carrying child sizes, enabling offset computation and child verification |
| `per-object-outboard-large-<τ>`, τ ∈ {64 MiB, 256 MiB} | INT-029, CON-020 | outboard only for ContentObjects above τ; whole-object otherwise |
| `hash-chain` | INT-029 | AEA-style chained digests; range verification walks from the object start (linear) |
| `content-defined-hashsplit-tree` | MOD-034 | bup-style tree whose interior boundaries are chosen by extra rolling-hash bits of Chunk digests (fan-out target 16) |
| `merkle-over-tree-hash-chunk-ids`, `c2sp-sequencehash`, `rfc6962-byte-prefix`, `tree-native-hash-leaves` | MOD-034 | proof-size-identical to the binary outboard except node input framing; measured for digest-op counts and node bytes only |

### 4.2 Physical layouts of outboard nodes (DEC-CON-020)

`level-order-array` (one section, levels contiguous), `bao-preorder-outboard`, `bab-rfc9162-shape`, `paged-subtree-blocks-<P>` (P ∈ {4 KiB, 64 KiB}), `fs-verity-bottom-up-levels` (4 KiB blocks), `per-content-object-outboard`, `combined-interleaved-encoding` (= `bao-combined-interleaved`). Each structure in 4.1 that needs an outboard is evaluated in every applicable layout.

### 4.3 Root bindings (DEC-MOD-007)

`status-quo-bare-root-via-pcr`, `object-descriptor-root` (+ 32 B digest + 16 B counts per ContentObject in the authenticated descriptor list), `length-in-root-node` (no extra bytes; one extra 16 B hash input), `manifest-carried-counts` (+ 16 B per ContentObject record), `merkle-outboard-with-signed-count` (+ 16 B per outboard header). Remote cost differences are the extra bytes on the proof path and at open; soundness is FORMAL.

Exclusions: none. `hash-chain` is kept although its linear walk is expected to lose everywhere (R0 completeness).

## 5. Corpus selectors

- Real Chunk lists: tuning archives from EXP-REMOTE-002 (`balanced-plain`, `extreme-plain` small+medium, `balanced-enc-bucketed` small+medium for `segmented-proofs`), objects with ≥ 4 MiB logical size for the range strata; the largest tuning objects (F07 safetensors and GGUF, F15/F16 raw images, F05 cluster traces, F06 GH Archive) supply up to a few thousand production-size Chunks per object.
- Chunk-count scaling: synthetic Chunk lists with n = 10^3, 10^4, 10^5, 10^6, 10^7 whose Chunk sizes are drawn from the empirical Chunk-size distribution of the tuning balanced-plain archives (seeded bootstrap over the layout maps; no content needed) — computed, not packed; n = 10^8 and 10^9 by closed-form proof-size formulas cross-checked against the enumerated values for n ≤ 10^7 (exact agreement required).
- Access patterns: WL-RANGE strata {1 B, 4 KiB, 1 MiB, whole} (T-22), 16 seeded offsets per stratum per object, plus a 256-range sequential scan within one object for reuse; validation look for W vs S; held-out section 15.

## 6. Environment and platform requirements

`wsl-ubuntu`; deterministic computation for sizes, requests, digest-op counts and modelled latency; status-quo whole-object reads measured through the production client over http-loopback (EXP-REMOTE-002 traces) and scoped client memory on the ≥ 256 MiB tuning objects; hashing microbenchmark (`ebr-remote hashbench`: SHA-256 via `sha2 = 0.11.0` as pinned in production, BLAKE3 via a research-only pinned `blake3` crate in the harness; node-size and leaf-size inputs; 1 and 4 threads) in a quiet window on `st-v` and `mt-v-4` (VIRTUALIZED; also a Windows-host `st-p` arm for the per-operation costs, because verification CPU claims span both platforms and §4.1 requires same-direction support). Linux and Windows x86-64; no privileges; no large storage.

**Platform matrix.** Linux x86-64 required (WSL2); Windows x86-64 for the hashbench st-p arm; macOS: not applicable; ARM64: hashing throughput PLATFORM_BLOCKED (emulated ARM64 timing is never used); network: none/loopback; privileged: no; large storage: no; external review: yes for proof-binding security (crypto dossier).

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound; PY=/root/eb-research/venv/bin/python; export PYTHONPATH=research/tools
python research/orchestration/disk_guard.py
$PY -m ebr run --spec research/experiments/EXP-REMOTE-007/spec.yaml                   # proofsim over real layouts (deterministic)
$PY -m ebr verify-raw --spec research/experiments/EXP-REMOTE-007/spec.yaml && $PY -m ebr normalize --spec research/experiments/EXP-REMOTE-007/spec.yaml
$PY research/tools/remote/proofsim.py scale --chunk-size-source /root/eb-research/cache/remote-archives --n 1e3,1e4,1e5,1e6,1e7 \
    --formula-check 1e8,1e9 --structures research/experiments/EXP-REMOTE-007/structures.json \
    --connection-model research/experiments/EXP-REMOTE-001/connection-model.json --out research/raw/EXP-REMOTE-007/scale/
$PY -m ebr run --spec research/experiments/EXP-REMOTE-007/spec-hashbench.yaml --timing   # generated from spec.yaml (candidate: hash x threads)
pwsh -File research/experiments/EXP-REMOTE-007/run_hashbench_windows.ps1                 # Windows st-p arm, same binary built for x86_64-pc-windows-msvc
$PY -m decide analyze --experiment EXP-REMOTE-007
```

## 8. Seeds, warmups, repetitions

Seeds `order` 20261005, `bootstrap` 20261006, `command` 20261007 (range offsets, synthetic Chunk-size bootstrap). Deterministic arms: 2 repetitions + output repeat-hash. Hashbench (per-operation latency, T-08 rule): in-process batches of ≥ 1,000 operations, warmups 2, rounds min 10, step 10, cap 30, precision rule.

## 9. Metrics

| Metric | OD | Role | Threshold |
|---|---|---|---|
| `verified_range_fetch_bytes` per stratum | OD-11 | **primary** | T-11 |
| `remote_task_latency_s` for the WL-RANGE stratum (modelled, time to verified range) | OD-12 | **primary** | T-10 |
| `access_amplification` per stratum | OD-10 | **primary** for DEC-ACC-040 | T-22 |
| `alloc_peak_bytes` (client, streaming output vs whole object; formula for simulated candidates, scoped RSS for the status quo until §14 item 17) | OD-08 | **primary** for DEC-ACC-040 | T-06 |
| `verification_digest_ops` per verified range | OD-11 | diagnostic (secondary; one primary per OD) | T-12 |
| `verify_overhead_pp` (modelled verified vs unverified range latency including measured digest CPU) | OD-11 | secondary | T-13 |
| `http_requests` per verified range | OD-12 | secondary | T-12 |
| `proof_bytes`, `proof_bytes_reused_fraction`, outboard `side_data_bytes` and fraction of `artifact_bytes` | OD-11 / OD-05 | diagnostic | T-02 / T-01 |
| `remote_first_entry_latency_s` (time to first verified byte for ≥ 256 MiB entries) | OD-12 | secondary | T-10 |
| hashbench: per-node and per-MiB digest time (in-process) | OD-07 | secondary input | T-08 floor |
| `formula_mismatches` (closed form vs enumeration) | – | binary gate | §3.3 |

## 10. Normalization and statistical analysis

- Item = object (within archive); groups and families from the archive's corpus item; strata per range size, per Chunk-count decade (synthetic series analysed separately as family `SCALE-PROOF`), per profile; never pooled across strata.
- Tuning: utility winner W over the primaries; validation W vs S per §4.11-§4.12, MDE gate.
- Scaling view: per structure, proof bytes and requests vs n on log-log axes with exact closed forms reported (FORMALLY_DERIVED); crossover n× located by exact comparison.
- R6 is decisive here: structures that add sections, records or identity/digest roots are T3/T4 (§7.2) and must clear the corresponding minimum-gain band over the best lower-tier survivor (the status quo or the leaf-list reader, which needs no wire change because leaf lists are already in the Manifest) — tiers assessed by an independent session before the first look.
- R9: partition by object size (expressible if `per-object-outboard-large` thresholds are declared) and by local vs remote source.

## 11. Practical significance

T-11, T-10, T-22, T-06, T-12, T-13, T-02, T-01, T-08 by reference; minimum gains per computed tier (§7.3: a new section type needs the k = 5 band).

## 12. Sensitivity analysis

Slow-start and connection-model arms (proof fetches are small, so slow start matters mainly for covering-Chunk fetches); Chunk size target ×0.5 and ×2 for the synthetic series (proof cost vs Chunk granularity); dependency-closure inclusion on dense/extreme layouts vs lookback-0 layouts; digest size 32 B vs 64 B (SHA-512/BLAKE3-512 variants, reported); §6.3-§6.7 arms.

## 13. Expected negative results worth recording

- At production Chunk sizes most corpus objects have too few Chunks for O(log n) proofs to beat the flat leaf list that the Manifest already carries; outboards may not clear the T3 minimum gain on the corpus even if they win at 10^6 Chunks.
- Bao's 1 KiB leaves decouple proofs from CDC Chunks but force reading uncompressed-layout bytes, which interacts badly with compression (a container-domain guard likely fails).
- `hash-chain` loses everywhere for mid-object ranges.
- Streaming output helps memory but not time-to-all-bytes.

## 14. Threats to validity

- Structure sizes follow declared encodings, not implementations; real framing may add bytes.
- Synthetic Chunk lists bootstrap real Chunk sizes but not real object size distributions beyond the corpus.
- Digest-op counts ignore cache effects and hardware acceleration differences; hashbench measures them only on this host (x86-64; ARM64 PLATFORM_BLOCKED for performance).
- Security properties are not assessed here (EXTERNAL); a cheaper structure that fails review is eliminated later at R1/R2.

## 15. Held-out (Phase D placeholder)

Frozen W and S rerun once on held-out archives' layouts (deterministic proofsim and status-quo traces) and the hashbench re-run if `env_id` changed, after Commit C (`decision-method.md` §5.5), report-only per §5.6.

## 16. Estimates

- Compute: about 8 machine-hours (proofsim enumeration up to 10^7 Chunks per structure and layout is CPU-heavy but parallel; hashbench 1 h per platform in quiet windows).
- Disk: under 10 GB (enumerated proof plans for the scale series, compressed).
- Agent effort: scripted; judgment for tier assessment requests, cross-domain coordination (container, integrity, model) and analysis.
