<!--
generated-by: python -m ebr report --spec research/experiments/EXP-HARNESS-SMOKE-CODEC/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 4593671e2aecad82fe4f547d1ba874d1aa3f177f
experiment: EXP-HARNESS-SMOKE-CODEC
input: research/normalized/EXP-HARNESS-SMOKE-CODEC/per_candidate.csv sha256=18ffbdae720b5df24317e8426861e1ef852d58354efe3e31467537149c971a5d
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-HARNESS-SMOKE-CODEC: per-candidate summary

Unit of analysis per `analysis.pairing`; CI = percentile bootstrap of the median. Timing-family rows with decision_grade = no are informational only.

| env_id | metric | candidate | unit | n_units | median | p90 | iqr | mad | ci_lower | ci_upper | decision_grade | ci_status |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | wall_s | compare-level3 | s | 1 | 0.02131 | 0.02131 | 0 | 0 |  |  | no | insufficient_units |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | cpu_s | compare-level3 | s | 1 | 0.007139 | 0.007139 | 0 | 0 |  |  | no | insufficient_units |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | max_rss_bytes | compare-level3 | B | 1 | 7204864 | 7204864 | 0 | 0 |  |  | yes | insufficient_units |
