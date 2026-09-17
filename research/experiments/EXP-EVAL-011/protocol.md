# EXP-EVAL-011: Sealed supplementary held-out tranche (method review G01): selection, sealing and custody procedure, fixed now and executed after the design freeze

| Field | Value |
|---|---|
| Domain | evaluation (program sections 6, 7, 34, 35) |
| Status | NEEDS_TOOLING (selection, sealing, public-manifest and audit scripts; ebr support for an opaque-id sealed manifest). Also needs **program-owner actions** (seed generation and custody, execution in a non-agent terminal) and approved network downloads. Execution window: after Commit A (design freeze), before Commit C (unlock). |
| Kind | Corpus construction under the sealing route of `decision-method.md` §5.4 ("sealed tranche", which lifts the `identity_aware_heldout` cap). Pre-registered as an experiment because its procedure must be fixed, auditable and reproducible from a later-revealed seed. |
| Resolves | Method review G01 (option (a) as the cap-lifting route beside the adopted option (b)); PROGRESS "Open integration items (G01)"; corpus critique round 2 held-out verdict item 1 |
| Method | `research/decision-method.md` §4.9, §4.12, §5.3-§5.8, R5 at `14b977c`; `research/corpus/methodology.md` §3 split rules |

## Author, independence and exposure

Designed by the Phase C-design evaluation-domain session, which has a recorded L7 exposure to identifiers of the **existing** held-out set (EXP-EVAL-001 protocol). That exposure cannot contaminate the sealed tranche, for three reasons. The tranche's items are drawn after the freeze by a keyed random process from an owner-held seed this session never sees. The procedure excludes every lineage already in the corpus by a mechanical check. And the eligibility rules below are content-blind. No design or analysis session may execute, observe or read any output of the selection except `manifest.public.json`.

## Question

Can a supplementary held-out tranche be built whose item identities, sources, selection lists and seeds stay unknown to every design and analysis session until after unlock, and which still meets the §4.9 minimum support and the R5 held-out MDE limit? The tranche must also be verifiable after the fact: from the revealed seed, anyone can re-derive exactly which items were drawn and why every rejected candidate was rejected.

## Hypotheses and falsification

- **H1 (feasibility).** Every family slot planned under the EXP-EVAL-007 sizing rule is filled within 1,000 draw attempts per slot, except F15-F17-style derived families. Those are filled by keyed generator parameter draws. Falsified by any real-item slot left unfilled. An unfilled slot is recorded, and the tranche's minimum-support check re-runs. If support drops below 10 groups across 5 families, the tranche cannot serve corpus-level R5 alone, and the `identity_aware_heldout` cap is not lifted for any decision.
- **H2 (sealing).** Before Commit C, the transcript audit (EXP-EVAL-010) and the custody audit below find zero agent-session reads of the seed, the selection log, the listing snapshots, the private path map or the plaintext items. Falsified by any read. The affected tranche is then identity-aware, the cap stays, and the event is recorded as L7, or L1 if content was read.
- **H3 (independence).** Every sealed item has content overlap of at most 0.01 against every existing split per family (§5.3 measure) and a lineage absent from `research/corpus/manifest.json`. Falsified per item, which triggers rejection and redraw inside the selection loop. After sealing, a post-seal audit finding is an integrity relock: new draw, same procedure, attempt counter continued.
- **H4 (verifiability).** After all held-out runs, revealing S and the listing snapshots lets `sealed_select.py --verify` regenerate a byte-identical selection log and public manifest. Falsified by any difference, which invalidates the tranche's evidence (L4-class event).

## Decisions informed

`DEC-ECO-076` (the "sealed held-out tranches" and "fresh post-freeze collection" candidates made concrete), `DEC-ECO-077` (post-freeze fresh collection; real-only held-out question per family), `DEC-ECO-031` (held-out provenance and sealing record). Gate: G01 cap-lifting route for every EMPIRICAL decision's R5.

## Candidates (procedure variants considered; the selected procedure is P-SEAL)

| id | Procedure | Disposition |
|---|---|---|
| `P-NONE` | Status quo: identity-aware held-out only (§5.4 option (b)) | kept as the baseline evaluation. Every R5 pass carries the MEDIUM cap |
| `P-SPLIT` | Split the existing held-out into tranches | rejected: 1-6 groups per family cannot meet §4.9 per tranche (§5.5), and the items are exposed anyway |
| `P-SEAL` | Post-freeze keyed random draw from pre-registered public pools plus keyed generator draws, encrypted at rest, opaque public manifest, seed revealed after runs | **selected design** |
| `P-FRESH-DATE` | Items published after the freeze date only | merged into `P-SEAL` as an eligibility preference where pools carry dates, not a separate procedure. Recency alone does not seal identity |
| `P-EXTERNAL` | Third party selects and holds items | not executable without an external party; recorded as an alternative for remaining unknowns |

## Timeline (binding)

1. **Now (design pre-registration).** This protocol, the pool definitions below and the scripts named in "Commands" (written before any use) are committed.
2. **Before Commit A.**
   - The owner generates the seed S, 32 bytes from a CSPRNG, **outside any agent session** (for example PowerShell `[Security.Cryptography.RandomNumberGenerator]::GetBytes(32)` in a plain terminal) and stores it outside the repository, outside `D:\Projects`, outside any `.claude` directory and outside the WSL distro. The owner commits only `research/decisions/sealed-seed-commitment.json` = {`scheme`: "sha256(ebrc-sealed-seed-v1 || S)", `commitment`: <hex>, `date`}.
   - EXP-EVAL-007 sizing output fixes the slot plan (k groups per family, F families), committed as `research/decisions/sealed-slot-plan.json`.
3. **After Commit A, before Commit C.**
   - The owner runs `sealed_select.py` in a non-agent terminal. Outputs: an encrypted bundle, a private path map, `manifest.public.json`, and `research/decisions/sealed-tranche-lock.json` {`design_freeze_sha`, `manifest_public_sha256`, `slot_plan_sha256`, `seed_commitment`, `selection_log_sha256`, `date`}, committed.
   - The corpus session runs the EXP-EVAL-010 overlap and public-benchmark audits in sealed mode, printing aggregates only.
   - The EXP-EVAL-012 held-out MDE gate uses the public group counts.
4. **After Commit C.** The owner decrypts into `/root/eb-research/heldout-sealed/` and verifies every `logical_tree_sha256`. EXP-EVAL-012 runs the sealed specs exactly once. Then the owner commits S (`sealed-seed-reveal.json`), and `sealed_select.py --verify` runs (H4).

## Pools and eligibility (pre-registered; the listings are fetched at execution time and hashed into the lock)

Pools are public registries large enough that a uniform draw is unguessable: each needs at least 1,000 eligible candidates after filters, otherwise the family is marked `pool_too_small` and filled by generator draws or left unfilled. Listing snapshots are sorted by a stable key (registry id) before drawing.

| Family | Pool (real items) | Eligibility (applied in order, each a logged reason code on rejection) |
|---|---|---|
| F01 source repositories | public GitHub repositories created at least 1 year before the draw, with an SPDX OSI licence, fetched as a git clone at the default branch head | lineage (host/owner/repo) not in manifest provenance; tier by `git archive` size |
| F02 build trees | crates.io crates with at least 10 reverse dependencies and a `cargo build --release --offline`-able vendored build inside 30 min | as F01; build succeeds in WSL with no network after vendoring |
| F03 dependency and vendor trees | npm packages with at least 1,000 weekly downloads and a lockfile; `npm ci --ignore-scripts` closure | lineage not in manifest; per-package licence audit record |
| F06 structured text | catalog.data.gov CKAN resources of format CSV, JSON or XML, US-government or public-domain licence | resource bytes within the slot tier; not a mirror of a manifest provenance URL |
| F07 numeric arrays | Zenodo records under CC0 or CC-BY containing `.nc`, `.h5`, `.hdf5`, `.npy`, `.npz` or `.safetensors` files | record DOI lineage not in manifest |
| F09 executables and F10 redundant binaries | binary packages of a Linux distribution drawn from the DistroWatch top-100 list whose name matches no manifest provenance; F09 uses `/usr/bin` ELF files and F10 uses static libraries and firmware packages | distribution lineage not in manifest |
| F11 already-compressed | GitHub release assets (`.tar.gz`, `.zip`, `.whl`, `.jar`, `.7z`) of repositories drawn as for F01 | as F01 |
| F12 JPEG images | Wikimedia Commons files in "Quality images" with CC0, PD or CC-BY licences, grouped by uploader | uploader lineage not in manifest; at least 8 JPEGs per group |
| F13 other media | Internet Archive items of mediatype `movies` or `audio` with a PD or CC0 `licenseurl` | collection lineage not in manifest |
| F14 archive-in-archive | Maven Central artifacts whose `.jar` contains at least one nested `.jar` or `.zip` | groupId lineage not in manifest |
| F16 VM and disk images | official cloud or ISO images of the distribution drawn for F09 | same distribution as F09 is allowed (same independence group recorded) |
| F18 versioned trees | GitHub repositories as F01 with at least 5 release tags; 3 consecutive release source trees | as F01 |
| F04, F05, F08, F15, F17, F19, F20 (generated or derived) | the existing committed generators (`research/corpus/generators/`), with every generator parameter drawn from its documented range by `HMAC-SHA256(S, "gen|<family>|<j>|<param>")`, and derived items (F17 duplicate trees, F04 many-small-file trees) built from a sealed real item | a new independence group `sealed-gen-<opaque>`; overlap audit applies |

**Common eligibility, all families:**
- the licence permits private research use (redistribution is not required, because the bundle is never published);
- the item is not tagged or derivable as a public benchmark (Silesia, Canterbury, Calgary, enwik, Kodak, Blender open movies and every other `public-benchmark` lineage in the manifest);
- logical size within the slot's scale tier;
- fetch digest verifiable (registry-published digest where one exists, otherwise TOFU pin recorded);
- content overlap at most 0.01 per family against every existing split (§5.3 measure, computed after materialization);
- total sealed logical bytes at most 60 GiB (slots drawn in family order; once the budget is exhausted, remaining large-tier slots are downgraded to medium, recorded).

## Draw algorithm (exact)

```
for tranche T, family f in sorted(slot_plan), slot j in 0..k_f-1:
    tier = slot_plan[f].tiers[j]
    for attempt a in 0..999:
        h   = HMAC-SHA256(key=S, msg="ebrc-sealed-v1|T|f|j|a")
        idx = int.from_bytes(h, "big") mod len(pool_snapshot[f])
        c   = pool_snapshot[f][idx]
        r   = first failing eligibility rule(c, tier)  -> log(T,f,j,a,idx,registry_id(c),r)
        if r is None: accept c; break
    else: log slot unfilled
opaque_id(item)  = "sealed-" + hex(HMAC-SHA256(S, "id|" + registry_id))[0:16]
opaque_group(g)  = "sealed-grp-" + hex(HMAC-SHA256(S, "grp|" + lineage_key))[0:12]
```

Pool snapshots, the selection log and the private path map are stored only inside the encrypted bundle (`age`, owner X25519 identity held outside the host's agent-accessible paths) until the reveal.

## Corpus selectors

No existing corpus items are selected. The tranche is new data. Its public manifest `research/corpus/sealed/manifest.public.json` exposes per item only: `opaque_id`, `family`, `scale`, `bytes`, `file_count`, `logical_tree_sha256`, `opaque_group`, `real_or_generated`, `license_class`. That matches the "fingerprint-only records" that §5.4 allows.

## Environment

- The owner's non-agent terminal on the Windows host, driving WSL (`wsl.exe -d Ubuntu -- ...`).
- Network: approved public downloads.
- Storage `/root/eb-research/sealed/` (D:-hosted distro), encrypted at rest.
- `disk_guard.py` must pass.

## Commands (TO BE WRITTEN before any use; scripts committed, then reviewed by a session outside design work)

```sh
# owner, before Commit A
pwsh -File research/corpus/tools/sealed_commit_seed.ps1 -SeedFile <owner path>      # writes research/decisions/sealed-seed-commitment.json
# owner, after Commit A (non-agent terminal)
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && /root/eb-research/venv/bin/python research/corpus/tools/sealed_select.py --tranche S1 --seed-file /mnt/<owner path> --slot-plan research/decisions/sealed-slot-plan.json --pools research/corpus/sealed/pools.json --budget-gib 60 --age-recipient <owner.pub> --out-bundle /root/eb-research/sealed/S1.age --out-public research/corpus/sealed/manifest.public.json --out-lock research/decisions/sealed-tranche-lock.json'
# corpus session (aggregates only)
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && /root/eb-research/venv/bin/python research/corpus/tools/overlap_audit.py --sealed research/corpus/sealed/manifest.public.json --sealed-fingerprints /root/eb-research/sealed/fp.age --identity <owner-provided at audit time> --out research/decisions/heldout-audit/sealed-overlap.csv'
# owner, after Commit C and after EXP-EVAL-012 sealed runs
wsl.exe -d Ubuntu -- bash -lc '/root/eb-research/venv/bin/python research/corpus/tools/sealed_select.py --verify --seed-file ... --bundle /root/eb-research/sealed/S1.age --lock research/decisions/sealed-tranche-lock.json'
# ebr support (TO BE WRITTEN): corpus.manifest may point at manifest.public.json with a private path map; split name "heldout-sealed" guarded exactly like "heldout" (EB_HELDOUT_UNLOCK == design_freeze_sha)
```

`research/corpus/sealed/pools.json` is committed with this protocol's first tooling commit and holds the machine-readable form of the pool table: registry endpoints, filters and the stable sort key. After that commit it is frozen. Any change before Commit A is a logged protocol deviation. Any change after Commit A invalidates the tranche.

## Seeds

S is owner-held, committed by hash before Commit A and revealed after the held-out runs. No other randomness exists: every tie-break is by sorted registry id.

## Warmup

Not applicable.

## Measurement method and metrics

- Fill rate per family (slots filled / planned).
- Attempts per slot.
- Rejection counts by reason code (published after the reveal).
- Sealed-tranche group counts per family (public).
- Overlap fractions (§5.3).
- Custody audit result.
- H4 regeneration equality.

No Appendix A.2 metric applies. The tranche's own evaluation metrics are those of each decision experiment re-run in EXP-EVAL-012.

## Replication and adaptive rule

One tranche S1. A second tranche S2 is drawn only for §5.6 fresh confirmatory data after an unlock (re-selection or fixed candidates), with the same procedure, a new owner seed, a new commitment, and draws disjoint from S1 lineages.

## Normalization

None.

## Statistical analysis

Minimum-support check (§4.9) and held-out MDE per supporting comparison (§4.12, R5: at most 2 band units) computed from the public group counts before Commit C. These run in EXP-EVAL-012's pre-unlock record.

## Practical-significance thresholds

Content overlap at most 0.01 (§5.3). Held-out MDE at most 2 band units (R5; `thresholds.json` `statistics.mde_max_bands`). Minimum support of 10 groups across 5 families; family guard of at least 2 groups (§4.9).

## Sensitivity analysis

EXP-EVAL-012 reports held-out results three ways: sealed only, identity-aware only, and combined, with the cap applying to any result that includes identity-aware items (§5.4). Pool licence classes are reported, and one sensitivity view uses only redistributable sealed items.

## Expected negative results worth recording

- Families whose public pools are too small or too licence-restricted (likely F08 real databases and F16 VM images), leaving generator-only coverage.
- Large-tier slots downgraded by the 60 GiB budget.
- A sealed tranche too small to lift the cap for decisions that need timing MDE at 2 bands.
- Custody failures if any agent session touches the sealed paths.

## Threats to validity

- **Pool definitions are public.** Designers know the populations, not the items. A design tuned to be good on, say, every popular npm package is not leakage under §5.4 but population-level fitting. The report states it as a limitation.
- **Registry drift** between drawing and reveal is neutralized by the hashed snapshots.
- **Owner-executed steps** depend on the owner following the procedure exactly. The committed lock hashes and the post-reveal H4 verification make deviations detectable, but not preventable.
- **Agent filesystem access.** Agents on this host have broad filesystem access, so sealing against a *deliberately* misbehaving agent is not possible. The protection is procedural (no session reads sealed paths) plus detective (transcript audit, custody audit).

## Platform requirements

WSL2 on the D:-hosted distro; network (approved public downloads; registry APIs); large storage (at most 60 GiB sealed plus encrypted copy); program-owner action for seed custody and execution; no macOS or ARM64.

## Estimates

- Machine time: about 30 h (downloads, builds for F02 and F03, generation, fingerprinting, encryption, overlap audit).
- Disk: about 130 GiB peak (plaintext during materialization plus encrypted bundle), about 70 GiB steady (encrypted plus decrypted after unlock).
- Agent effort: none for execution (owner-run scripts); scripted audits by the corpus session; judgment only for the pre-use script review.
