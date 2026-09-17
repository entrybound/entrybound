<!--
generated-by: python -m ebr report --spec research/experiments/EXP-HARNESS-SMOKE-PACK/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 4593671e2aecad82fe4f547d1ba874d1aa3f177f
experiment: EXP-HARNESS-SMOKE-PACK
input: research/normalized/EXP-HARNESS-SMOKE-PACK/per_candidate.csv sha256=0d4a6e0502873daa573aff1de7644b8e71eec1b3118443245e710282ec553d8b
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-HARNESS-SMOKE-PACK: per-candidate summary

Unit of analysis per `analysis.pairing`; CI = percentile bootstrap of the median. Timing-family rows with decision_grade = no are informational only.

| env_id | metric | candidate | unit | n_units | median | p90 | iqr | mad | ci_lower | ci_upper | decision_grade | ci_status |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | wall_s | balanced-indexed | s | 1 | 0.03745 | 0.03745 | 0 | 0 |  |  | no | insufficient_units |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | cpu_s | balanced-indexed | s | 1 | 0.009384 | 0.009384 | 0 | 0 |  |  | no | insufficient_units |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | max_rss_bytes | balanced-indexed | B | 1 | 6426624 | 6426624 | 0 | 0 |  |  | yes | insufficient_units |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | output_bytes | balanced-indexed | B | 1 | 6287 | 6287 | 0 | 0 |  |  | yes | insufficient_units |
