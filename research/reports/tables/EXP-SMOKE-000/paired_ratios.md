<!--
generated-by: python -m ebr report --spec research/experiments/EXP-SMOKE-000/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155
experiment: EXP-SMOKE-000
input: research/normalized/EXP-SMOKE-000/paired_ratios.csv sha256=d04f8aa71af2b8461fde2ce590fd81fde1bd7c1264d07db9671cec42fd9b1bd2
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-SMOKE-000: paired ratios vs baseline

Ratio = candidate / baseline. Verdict applies the noise rule (CI must lie wholly outside the band).

| env_id | metric | candidate | baseline | n_pairs | median_ratio | ci_lower | ci_upper | band_lower | band_upper | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | cpu_s | gzip-1 | gzip-6 | 3 | 0.1365 | 0.1137 | 0.2036 |  |  | not_decision_grade(informational:threshold_unset:families.time_cpu.practical_significance_band) |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | cpu_s | gzip-9 | gzip-6 | 3 | 1.209 | 1.195 | 1.314 |  |  | not_decision_grade(informational:threshold_unset:families.time_cpu.practical_significance_band) |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | max_rss_bytes | gzip-1 | gzip-6 | 3 | 0.9941 | 0.9595 | 1 |  |  | threshold_unset:families.memory_peak.practical_significance_band |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | max_rss_bytes | gzip-9 | gzip-6 | 3 | 1.009 | 0.9679 | 1.021 |  |  | threshold_unset:families.memory_peak.practical_significance_band |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | scratch_peak_bytes | gzip-1 | gzip-6 | 3 | 1.238 | 1.238 | 1.238 |  |  | threshold_unset:families.scratch_peak.practical_significance_band |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | scratch_peak_bytes | gzip-9 | gzip-6 | 3 | 0.9949 | 0.9949 | 0.9949 |  |  | threshold_unset:families.scratch_peak.practical_significance_band |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | output_bytes | gzip-1 | gzip-6 | 3 | 1.238 | 1.238 | 1.238 | 0.999 | 1.001 | distinguishable_above |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | output_bytes | gzip-9 | gzip-6 | 3 | 0.9949 | 0.9949 | 0.9949 | 0.999 | 1.001 | distinguishable_below |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | ratio | gzip-1 | gzip-6 | 3 | 1.238 | 1.238 | 1.238 | 0.999 | 1.001 | distinguishable_above |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | ratio | gzip-9 | gzip-6 | 3 | 0.9949 | 0.9949 | 0.9949 | 0.999 | 1.001 | distinguishable_below |
