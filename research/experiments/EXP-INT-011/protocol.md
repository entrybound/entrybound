# EXP-INT-011: Out-of-band carrier survival through copy, archive, HTTP and object-store transfers

| Field | Value |
|---|---|
| Status | NEEDS_TOOLING (T13 `ebr-int-carrier` driver; MinIO server and `mc` client download for the S3-compatible arm); cloud services and sync clients are EXP-INT-020 (BLOCKED); APFS and Finder are EXP-INT-019 (BLOCKED) |
| Domain | integrity (program §24, §32 cross-reference; objective HC-15, OD-26) |
| Decisions informed | DEC-INT-011, DEC-INT-034, DEC-LEG-056 |
| Gates, method, author | conventions C0 |
| Specs | `spec.yaml` (Linux transfers), `spec-windows.yaml` (Windows transfers) |

## 1. Question

When an archive travels with an out-of-band carrier (parity set and its binding, unverified
marker on salvaged output, sidecar discovered by naming convention, receipt or wrapper index),
which carriers survive which common transfer tools unchanged, which are silently separated or
stripped, and when a carrier is lost or the artifact is changed in transit, does the carrier's
verification report the separation or mismatch rather than silently treating the object as
unmarked or intact?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-INT-034 | Extended attributes and NTFS alternate data streams are stripped by at least half of the transfer tools (for example `cp` without `--preserve=xattr`, `zip`, HTTP download, S3 upload without explicit metadata), so an xattr or ADS marker is not a reliable unverified marker; filename suffixes and in-band salvage-to-new-archive survive every byte-preserving transfer. | xattr and ADS survive at least as often as suffixes. |
| H2 | DEC-INT-011 | Filename-convention parity discovery survives directory copies but not single-file HTTP or object-store transfers; content-digest bindings (PCI, wrapper index) detect in-transit modification in 100% of injected changes; filename-only binding detects none. | Filename-only binding detects a modification (it cannot by construction; this would indicate a harness defect). |
| H3 | DEC-LEG-056 | `<artifact>.eb` discovery by naming survives `rsync -a`, `cp -a`, tar and zip of the containing directory, and fails whenever only the legacy artifact is transferred. | Discovery survives single-file transfers. |

## 3. Decisions and candidates

| Decision | Candidates | Carrier implementation |
|---|---|---|
| DEC-INT-011 | `bind-pci`, `bind-section-chunk-digests`, `bind-lai-plus-signature`, `filename-convention-only`, `receipt-or-detached-signature`, `par-native-file-hash`, `wrapper-index-artifact` | par2 set named `<archive>.par2` and `<archive>.vol*.par2`; JSON binding receipt `<archive>.parity.json` carrying PCI and per-chunk digests; detached `.ebsig`; wrapper index `<archive>.index.json` (OCI-image-index-like, digests of archive and parity). Staleness under rewrite is EXP-INT-009. |
| DEC-INT-034 | `report-only`, `filename-suffix`, `xattr-or-ads-marker`, `marker-manifest-file`, `quarantine-directory`, `salvage-to-new-archive`, `no-marking` | Salvaged files with suffix `.unverified`; `user.entrybound.salvage` xattr (Linux) and `:entrybound.salvage` ADS (Windows); `ENTRYBOUND-SALVAGE.json` in the destination root; `quarantine/` directory layout; a new `.eb` archive (in-band, by construction). `no-marking` is proposed for L0 exclusion (HC-15: salvaged output must be marked unverified; SPEC §17.2 L1824 cited in the ledger); kept as a negative control whose survival is vacuous. `report-only` survives only as long as the report file is transferred (measured as a separate file carrier). |
| DEC-LEG-056 | `normative-naming-discovery` (and the status quo default `<artifact>.eb`) | Legacy artifact plus `<artifact>.eb` sidecar in a directory. |

## 4. Transfers (pre-registered)

- Linux (`wsl-ubuntu`, ext4 source and destination; cross-filesystem to an XFS loop image):
  `cp`, `cp -a`, `cp --preserve=xattr`, `mv` (same filesystem), `mv` (ext4 to XFS), `rsync -a`,
  `rsync -aX`, GNU tar default and `--xattrs`, bsdtar default, Info-ZIP `zip -r` and `unzip`,
  `7z a` and `7z x`, Python `shutil.copy` and `shutil.copy2`, `git add` and local `git clone`,
  `curl` download from `ebr-origin` (single file, filename from URL), `curl` download of a
  directory listing is not applicable (recorded), MinIO local S3-compatible server with `mc cp`
  single file and `mc mirror` directory (metadata flags default and with `--attr` where
  supported).
- Windows (`windows-host`, NTFS on D:): `Copy-Item`, `Move-Item` (same volume), `robocopy`
  default (`/COPY:DAT`), `robocopy /COPY:DATS` if permitted without admin (recorded otherwise),
  `xcopy /E`, `Compress-Archive` and `Expand-Archive`, `tar.exe` (bsdtar 3.8.8) create and
  extract, `curl.exe` and `Invoke-WebRequest` download from `ebr-origin`. Explorer copy is not
  scriptable without UI automation and is excluded (recorded). D: only.
- In-transit modification sub-arm: for each transfer, one seeded byte of the archive or legacy
  artifact is changed after the transfer; each carrier's verification is run.

## 5. Environment and platform requirements

WSL root for loop XFS images; `ebr-origin` on localhost; MinIO single binary and `mc` downloaded
at pinned versions (approved downloads, SHA-256 recorded), bound to 127.0.0.1 with generated test
credentials stored only in the work directory. Windows non-admin on D:. `timing: false`.

## 6. Procedure and commands

`ebr run` of `spec.yaml` and `spec-windows.yaml`: for each item, `ebr-int-carrier` packs the
archive (B-PROD), creates every carrier, performs every transfer into a fresh destination,
inspects carrier presence and byte equality, runs each carrier's verification with and without
in-transit modification, writes detail rows and prints one summary. Then `verify-raw`,
`verify_detail.py`, `normalize`, `report`.

## 7. Seeds, warmups, replication

Seeds 2026091811/12/13 (Linux), 2026091814/15/16 (Windows); modification offsets per C8.
Warmups 0; 2 repetitions; repeat-hash on the survival vector.

## 8. Metrics

| Metric | ID | OD | Role |
|---|---|---|---|
| `carrier_survival_fraction` (per carrier over the transfer set) | C11 proposed | OD-26 or as assigned | secondary until named; **primary candidate** for DEC-INT-034 and DEC-LEG-056 if added |
| silent-separation count (carrier lost and verification reports intact or unmarked without saying the carrier is missing) | none | HC-15 | binary-style count; any non-zero value for a candidate claiming it marks output is an HC-15 finding |
| in-transit modification detection per carrier | `undetected_tampers` (HC-14 M15.1 extended to bindings) | OD-15 floor | binary for carriers that claim integrity binding; descriptive for filename-only |
| `unsafe_defaults` | HC-05 | OD-25 floor | binary: default transfers after which salvaged output appears verified |

## 9. Normalization

Exact fractions per carrier over the pre-registered transfer set per host; hosts are evaluation
conditions (never pooled; headline = worst host).

## 10. Statistical analysis and sensitivity

Exact tallies, no intervals. Sensitivity: transfer-set weighting arm (all transfers equal versus
byte-preserving transfers only versus "common default commands" subset pre-registered in the T13
source), host arm.

## 11. Expected negative results worth recording

- xattr and ADS markers are lost by common tools (the ledger's `xattr-or-ads-marker` candidate is
  weak).
- par2 sets and receipts are separated by single-file transfers.
- Object-store uploads drop extended attributes and most metadata by default.

## 12. Threats to validity

- Tool versions and default flags matter; all are pinned and recorded.
- MinIO emulates S3 semantics locally; real cloud stores, CDNs and sync clients may differ
  (EXP-INT-020, BLOCKED).
- Windows Explorer, OneDrive and macOS Finder are not covered (excluded or BLOCKED).

## 13. Held-out evaluation (Phase D placeholder)

Conventions C12 (content-independent; held-out repeats use held-out archives only for completeness).

## 14. Cost estimates

- Compute: about 30 transfers x 12 carriers x 4 items x 2 repetitions: about 3 CPU-hours per host.
- Disk: about 5 GB per host.
- Agent effort: tooling scripted with review; execution none; analysis judgment.

## 15. Deviations (append-only)

None.
