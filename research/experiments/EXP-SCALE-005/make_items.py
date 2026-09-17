#!/usr/bin/env python3
"""Select items and write spec.yaml, spec-windows.yaml and items-win.jsonl for EXP-SCALE-005.

Run from the repository root:  python research/experiments/EXP-SCALE-005/make_items.py
Selection rule (protocol.md "Corpus selectors"): every small-scale tuning/validation item, plus
for each (split, family, independence_group) that has medium items the smallest medium item.
Held-out rows are never read beyond their split field.
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))

VARIATIONS = ["base", "aff1", "aff8", "aff32", "memcap-tight", "tmpdir-alt", "locale-tz", "creation-shuffled", "toolchain-1971"]
CONFIGS = ["bi", "di", "bs", "xi"]
CANDIDATES = [f"{v}-{c}" for v in VARIATIONS for c in CONFIGS] + ["stress20-bi", "stress20-di",
              "repack-roundtrip-bi", "replan-same-profile-bi", "encrypted-twice-bi"]
WIN_CANDIDATES = [f"{v}-{c}" for v in ("base", "aff1", "toolchain-1971") for c in ("bi", "di", "bs")] + ["stress20-bi", "stress20-di"]

METRICS = """metrics:
  - {name: applicable, source: stdout_json, key: payload.applicable, family: correctness, direction: none}
  - {name: outcome, source: stdout_json, key: payload.outcome, family: correctness, kind: string}
  - {name: reason_code, source: stdout_json, key: payload.reason_code, family: correctness, kind: string}
  - {name: output_sha256, source: stdout_json, key: payload.output_sha256, family: correctness, kind: string}
  - {name: lai, source: stdout_json, key: payload.lai, family: correctness, kind: string}
  - {name: pcr, source: stdout_json, key: payload.pcr, family: correctness, kind: string}
  - {name: aux, source: stdout_json, key: payload.aux, family: correctness, kind: string}
  - {name: pci, source: stdout_json, key: payload.pci, family: correctness, kind: string}
  - {name: distinct_pci, source: stdout_json, key: payload.distinct_pci, family: correctness, direction: none, unit: count}
  - {name: distinct_lai, source: stdout_json, key: payload.distinct_lai, family: correctness, direction: none, unit: count}
  - {name: repack_pci_equal_direct, source: stdout_json, key: payload.repack_pci_equal_direct, family: correctness, direction: none}
  - {name: repack_lai_pcr_aux_equal, source: stdout_json, key: payload.repack_lai_pcr_aux_equal, family: correctness, direction: none}
  - {name: replan_pcr_equal, source: stdout_json, key: payload.replan_pcr_equal, family: correctness, direction: none}
  - {name: replan_pci_equal, source: stdout_json, key: payload.replan_pci_equal, family: correctness, direction: none}
  - {name: enc_lai_equal, source: stdout_json, key: payload.enc_lai_equal, family: correctness, direction: none}
  - {name: enc_aux_equal, source: stdout_json, key: payload.enc_aux_equal, family: correctness, direction: none}
  - {name: enc_pcr_equal, source: stdout_json, key: payload.enc_pcr_equal, family: correctness, direction: none}
  - {name: enc_ciphertext_differs, source: stdout_json, key: payload.enc_ciphertext_differs, family: correctness, direction: none}
  - {name: replan_wall_s, source: stdout_json, key: payload.replan_wall_s, family: time_wall, direction: none, unit: s}
  - {name: replan_peak_rss_bytes, source: stdout_json, key: payload.replan_peak_rss_bytes, family: memory_peak, direction: none, unit: B}
"""

HEADER = """schema: ebr.spec.v1
experiment_id: {expid}
status: design-draft
notes: >
  {notes}
question: >
  Is the single-threaded production writer's output (PCI bytes and LAI/PCR/AUX) invariant under CPU
  availability, memory limit, TMPDIR, locale/time zone, creation order, background load, toolchain and
  host, are encrypted packs identity-stable with differing ciphertext, and do repack round trips and
  same-profile replans reproduce identities and bytes?
hypothesis: >
  Within one host class PCI and LAI never differ across variations; across hosts LAI and PCR agree and
  AUX differences are explained by FidelityReport; encrypted packs share LAI and AUX with different
  ciphertext; repack round trips and same-profile replans reproduce PCR and PCI. One two-output
  counterexample falsifies the corresponding claim.
decision_ids: [DEC-MOD-011, DEC-MOD-012, DEC-MOD-013, DEC-CMP-019, DEC-CMP-021, DEC-CMP-041, DEC-INT-046]

candidates:
"""


def main() -> None:
    m = json.load(open(os.path.join(REPO, "research", "corpus", "manifest.json"), encoding="utf-8"))
    items = [i for i in m["items"] if i.get("split") in ("tuning", "validation")]
    small = sorted(i["item_id"] for i in items if i.get("scale") == "small")
    groups = {}
    for i in items:
        if i.get("scale") != "medium":
            continue
        key = (i["split"], i["family"], i["independence_group"])
        if key not in groups or (i["bytes"], i["item_id"]) < (groups[key]["bytes"], groups[key]["item_id"]):
            groups[key] = i
    medium = sorted(g["item_id"] for g in groups.values())
    chosen = small + medium

    lin = HEADER.format(expid="EXP-SCALE-005", notes="Linux arm. Design draft until gate G-A. item_ids generated by make_items.py (small items plus the smallest medium item per independence group). Driver: research/tools/scale/det_cell.py.")
    lin += "".join(f"  - {{name: {c}}}\n" for c in CANDIDATES)
    lin += """command: >-
  /root/eb-research/venv/bin/python /mnt/d/Projects/entrybound/entrybound/research/tools/scale/det_cell.py
  --item {item_path} --item-id {item_id} --candidate {candidate} --rep {rep} --seed {seed}
  --scratch {scratch} --manifest /mnt/d/Projects/entrybound/entrybound/research/corpus/manifest.json
shell: bash

corpus:
  manifest: research/corpus/manifest.json
  splits: [tuning, validation]
  item_ids:
"""
    lin += "".join(f"    - {i}\n" for i in chosen)
    lin += "\n" + METRICS + """
repetitions: 3
warmups: 0
seeds: {order: 1305001, bootstrap: 1305002, command: 1305003}
timing: false
order: randomized_blocks
raw_compression: gzip

platform:
  os: [linux]
  arch: [x86_64]
  min_cpus: 32
  requires:
    - /root/eb-research/target/scale-prod/release/ebound
    - /root/eb-research/target/scale-prod-1971/release/ebound
    - /root/eb-research/target/harness/release/ebr-pack
    - /root/eb-research/venv/bin/python
    - taskset
  poll_interval_s: 1.0
  timeout_s: 21600

analysis:
  pairing: item_repetition
  report_metrics: [distinct_pci, repack_pci_equal_direct, replan_pci_equal, enc_ciphertext_differs]
"""
    with open(os.path.join(HERE, "spec.yaml"), "w", encoding="utf-8", newline="\n") as fh:
        fh.write(lin)

    split_of = {i["item_id"]: i["split"] for i in items}
    fam_of = {i["item_id"]: i["family"] for i in items}
    with open(os.path.join(HERE, "items-win.jsonl"), "w", encoding="utf-8", newline="\n") as fh:
        for iid in small:
            fh.write(json.dumps({"item_id": iid, "split": split_of[iid], "family": fam_of[iid],
                                 "path": f"D:/eb-research/scale/det-items/{iid}"}, sort_keys=True) + "\n")
    win = HEADER.format(expid="EXP-SCALE-005-WIN", notes="Windows NTFS arm (small items copied with robocopy /COPY:DT /DCOPY:T). Design draft until gate G-A. Driver: research/tools/scale/det_cell.py.")
    win += "".join(f"  - {{name: {c}}}\n" for c in WIN_CANDIDATES)
    win += """command: >-
  C:/Python313/python.exe D:/Projects/entrybound/entrybound/research/tools/scale/det_cell.py
  --item {item_path} --item-id {item_id} --candidate {candidate} --rep {rep} --seed {seed}
  --scratch {scratch} --manifest D:/Projects/entrybound/entrybound/research/corpus/manifest.json --host windows
shell: pwsh

corpus:
  manifest: research/experiments/EXP-SCALE-005/items-win.jsonl
  splits: [tuning, validation]

""" + METRICS + """
repetitions: 3
warmups: 0
seeds: {order: 1305101, bootstrap: 1305102, command: 1305103}
timing: false
order: randomized_blocks
raw_compression: gzip

platform:
  os: [windows]
  scratch_root: D:/eb-research/scale/scratch
  requires:
    - D:/eb-research/target/scale-prod-win/release/ebound.exe
    - D:/eb-research/target/scale-prod-win-1971/release/ebound.exe
  poll_interval_s: 1.0
  timeout_s: 21600

analysis:
  pairing: item_repetition
  report_metrics: [distinct_pci, pcr]
"""
    with open(os.path.join(HERE, "spec-windows.yaml"), "w", encoding="utf-8", newline="\n") as fh:
        fh.write(win)
    print(f"small={len(small)} medium={len(medium)} candidates={len(CANDIDATES)}")


if __name__ == "__main__":
    main()
