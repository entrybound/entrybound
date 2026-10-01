# Independent C4–C7 cost assessment template

Status: **UNASSESSED**. Copy this template for one candidate. It is not a tier or an acceptance record.

Source: `research/decision-method.md` §7.1–7.4 and `research/methods/method-review-round1.md` I02. Complete this before the candidate's first validation look. A session independent of candidate design signs it. A disputed tier takes the higher tier.

## Identity and source custody

- Decision ID:
- Candidate ID and exact definition commit:
- Governing method commit:
- C1–C3 output path and SHA-256:
- Assessor session and independence statement:
- Review session and date:

## C4 — attack surface and assurance

| Required census | Value | Evidence path and SHA-256 |
|---|---:|---|
| New untrusted parser entry points | UNASSESSED | |
| New attacker-sized allocations and their declared bounds | UNASSESSED | |
| New reachable `unsafe` or native paths | UNASSESSED | |
| New fuzz targets and coverage | UNASSESSED | |
| New key, nonce or associated-data constructions | UNASSESSED | |
| Recurring external security review obligation and cadence | UNASSESSED | |

Reader design and fuzz-target inventory cross-check:

## C5 — independent implementability

- Independent normative specification and stable external references:
- Independent vectors and minimal decoder evidence:
- Reader fragmentation for optional or extended features:
- Independent reviewer and review evidence:

## C6 — user complexity

- New flags, modes and concepts:
- First-use tasks and counted decision points, with `research/decision-method.md` §4.16 instrument:
- Unsafe defaults, if any:

## C7 — longevity and churn

- Planner/chunker IDs and permanent golden-vector obligations:
- Pinned encoder versions and writer-path churn:
- Library-version-defined decode steps and permanent decoder obligations:
- Existing pre-v1 archive migration cost:
- Single-upstream reliance:
- **For a defer candidate:** add-later incompatible feature bits, migration steps and identity effects:

## Tier computation and dispute

Record each T0–T4 trigger from §7.2 with a C1–C7 evidence citation. Final tier is the maximum triggered tier; no unassessed element may be treated as zero.

| Cost element | Triggered tier | Evidence |
|---|---|---|
| C1 | UNASSESSED | |
| C2 | UNASSESSED | |
| C3 | UNASSESSED | |
| C4 | UNASSESSED | |
| C5 | UNASSESSED | |
| C6 | UNASSESSED | |
| C7 | UNASSESSED | |

- Computed maximum tier: **UNASSESSED**
- Dispute and higher-tier resolution:
- Independent sign-off:
