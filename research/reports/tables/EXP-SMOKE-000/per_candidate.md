<!--
generated-by: python -m ebr report --spec research/experiments/EXP-SMOKE-000/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155
experiment: EXP-SMOKE-000
input: research/normalized/EXP-SMOKE-000/per_candidate.csv sha256=222841c3b54bb63d5689721080bf878afe399a04006805b31214e8fb02aa0d41
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-SMOKE-000: per-candidate summary

Unit of analysis per `analysis.pairing`; CI = percentile bootstrap of the median. Timing-family rows with decision_grade = no are informational only.

| env_id | metric | candidate | unit | n_units | median | p90 | iqr | mad | ci_lower | ci_upper | decision_grade | ci_status |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | cpu_s | gzip-1 | s | 3 | 0.01068 | 0.01479 | 0.0031 | 0.001055 | 0.009622 | 0.01582 | no | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | cpu_s | gzip-6 | s | 3 | 0.0782 | 0.08336 | 0.003476 | 0.0005 | 0.0777 | 0.08465 | no | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | cpu_s | gzip-9 | s | 3 | 0.1011 | 0.1025 | 0.004404 | 0.001631 | 0.09397 | 0.1028 | no | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | max_rss_bytes | gzip-1 | B | 3 | 3452928 | 3551232 | 90112 | 57344 | 3395584 | 3575808 | yes | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | max_rss_bytes | gzip-6 | B | 3 | 3538944 | 3568435 | 51200 | 36864 | 3473408 | 3575808 | yes | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | max_rss_bytes | gzip-9 | B | 3 | 3547136 | 3566797 | 55296 | 24576 | 3461120 | 3571712 | yes | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | scratch_peak_bytes | gzip-1 | B | 3 | 332210 | 332210 | 0 | 0 | 332210 | 332210 | yes | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | scratch_peak_bytes | gzip-6 | B | 3 | 268376 | 268376 | 0 | 0 | 268376 | 268376 | yes | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | scratch_peak_bytes | gzip-9 | B | 3 | 267014 | 267014 | 0 | 0 | 267014 | 267014 | yes | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | output_bytes | gzip-1 | B | 3 | 332210 | 332210 | 0 | 0 | 332210 | 332210 | yes | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | output_bytes | gzip-6 | B | 3 | 268376 | 268376 | 0 | 0 | 268376 | 268376 | yes | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | output_bytes | gzip-9 | B | 3 | 267014 | 267014 | 0 | 0 | 267014 | 267014 | yes | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | ratio | gzip-1 |  | 3 | 0.3168 | 0.3168 | 0 | 0 | 0.3168 | 0.3168 | yes | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | ratio | gzip-6 |  | 3 | 0.2559 | 0.2559 | 0 | 0 | 0.2559 | 0.2559 | yes | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
| 8576a4a4947ee57863c03ba9ebd455a4b41191d3a41a70fd8e35ae7d79db506e | ratio | gzip-9 |  | 3 | 0.2546 | 0.2546 | 0 | 0 | 0.2546 | 0.2546 | yes | ok:percentile-bootstrap:median:resamples=10000:seed=20260912 |
