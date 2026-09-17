<!--
generated-by: python -m ebr report --spec research/experiments/EXP-HARNESS-SMOKE-NETEM/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 4593671e2aecad82fe4f547d1ba874d1aa3f177f
experiment: EXP-HARNESS-SMOKE-NETEM
input: research/normalized/EXP-HARNESS-SMOKE-NETEM/per_candidate.csv sha256=1f0789fa11155151fad43c6ec9cb5c939890d686b01aadbf6ca15d49382e8060
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-HARNESS-SMOKE-NETEM: per-candidate summary

Unit of analysis per `analysis.pairing`; CI = percentile bootstrap of the median. Timing-family rows with decision_grade = no are informational only.

| env_id | metric | candidate | unit | n_units | median | p90 | iqr | mad | ci_lower | ci_upper | decision_grade | ci_status |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | wall_s | origin-roundtrip | s | 1 | 0.11 | 0.11 | 0 | 0 |  |  | no | insufficient_units |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | cpu_s | origin-roundtrip | s | 1 | 0.04809 | 0.04809 | 0 | 0 |  |  | no | insufficient_units |
