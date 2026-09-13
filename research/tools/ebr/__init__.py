"""ebr: Entrybound research experiment runner and statistics framework.

Modules
-------
spec       experiment spec schema, validation, held-out guard
corpus     corpus selection, generated items, fingerprints
run        per-sample execution, resource measurement, quiet-machine guard, raw JSONL
stats      robust descriptive statistics, bootstrap CIs, noise rule, Pareto, sensitivity
thresholds machine-readable thresholds (research/methods/thresholds.json)
normalize  raw JSONL -> normalized CSV + summary JSON
report     markdown tables and matplotlib figures from normalized CSV
rawio      raw JSONL writer/reader and verify-raw
cli        ``python -m ebr {run,normalize,report,verify-raw,validate,plan}``
"""

__version__ = "0.1.0"

RAW_SCHEMA = "ebr.raw.v1"
META_SCHEMA = "ebr.run-meta.v1"
SPEC_SCHEMA = "ebr.spec.v1"
SUMMARY_SCHEMA = "ebr.summary.v1"
THRESHOLDS_SCHEMA = "ebr.thresholds.v1"
