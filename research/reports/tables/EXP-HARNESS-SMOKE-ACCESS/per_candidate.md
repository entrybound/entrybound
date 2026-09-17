<!--
generated-by: python -m ebr report --spec research/experiments/EXP-HARNESS-SMOKE-ACCESS/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 4593671e2aecad82fe4f547d1ba874d1aa3f177f
experiment: EXP-HARNESS-SMOKE-ACCESS
input: research/normalized/EXP-HARNESS-SMOKE-ACCESS/per_candidate.csv sha256=60b436933a798328f0526ce1565c50b8a8c85e8ba81d4145747b0dbb297ef052
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-HARNESS-SMOKE-ACCESS: per-candidate summary

Unit of analysis per `analysis.pairing`; CI = percentile bootstrap of the median. Timing-family rows with decision_grade = no are informational only.

| env_id | metric | candidate | unit | n_units | median | p90 | iqr | mad | ci_lower | ci_upper | decision_grade | ci_status |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | wall_s | memory-balanced | s | 1 | 0.02918 | 0.02918 | 0 | 0 |  |  | no | insufficient_units |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | cpu_s | memory-balanced | s | 1 | 0.01475 | 0.01475 | 0 | 0 |  |  | no | insufficient_units |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | max_rss_bytes | memory-balanced | B | 1 | 7933952 | 7933952 | 0 | 0 |  |  | yes | insufficient_units |
