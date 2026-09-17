<!--
generated-by: python -m ebr report --spec research/experiments/EXP-HARNESS-SMOKE-PLANNER/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 4593671e2aecad82fe4f547d1ba874d1aa3f177f
experiment: EXP-HARNESS-SMOKE-PLANNER
input: research/normalized/EXP-HARNESS-SMOKE-PLANNER/per_candidate.csv sha256=104c660feb1836915baf7a51dcbba54dbe0585ad6190a22930c8e30d7f906c1b
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-HARNESS-SMOKE-PLANNER: per-candidate summary

Unit of analysis per `analysis.pairing`; CI = percentile bootstrap of the median. Timing-family rows with decision_grade = no are informational only.

| env_id | metric | candidate | unit | n_units | median | p90 | iqr | mad | ci_lower | ci_upper | decision_grade | ci_status |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | wall_s | balanced | s | 1 | 0.1413 | 0.1413 | 0 | 0 |  |  | no | insufficient_units |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | cpu_s | balanced | s | 1 | 0.1437 | 0.1437 | 0 | 0 |  |  | no | insufficient_units |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | max_rss_bytes | balanced | B | 1 | 35373056 | 35373056 | 0 | 0 |  |  | yes | insufficient_units |
