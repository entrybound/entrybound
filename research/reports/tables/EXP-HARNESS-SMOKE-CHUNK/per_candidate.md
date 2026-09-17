<!--
generated-by: python -m ebr report --spec research/experiments/EXP-HARNESS-SMOKE-CHUNK/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 4593671e2aecad82fe4f547d1ba874d1aa3f177f
experiment: EXP-HARNESS-SMOKE-CHUNK
input: research/normalized/EXP-HARNESS-SMOKE-CHUNK/per_candidate.csv sha256=c6c7ab6202591fde49b4261348021d79e770ca46cc2b1e2f996f2fa110f73a69
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-HARNESS-SMOKE-CHUNK: per-candidate summary

Unit of analysis per `analysis.pairing`; CI = percentile bootstrap of the median. Timing-family rows with decision_grade = no are informational only.

| env_id | metric | candidate | unit | n_units | median | p90 | iqr | mad | ci_lower | ci_upper | decision_grade | ci_status |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | wall_s | gear-norm-balanced | s | 1 | 0.01588 | 0.01588 | 0 | 0 |  |  | no | insufficient_units |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | cpu_s | gear-norm-balanced | s | 1 | 0.004739 | 0.004739 | 0 | 0 |  |  | no | insufficient_units |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | max_rss_bytes | gear-norm-balanced | B | 1 | 3477504 | 3477504 | 0 | 0 |  |  | yes | insufficient_units |
