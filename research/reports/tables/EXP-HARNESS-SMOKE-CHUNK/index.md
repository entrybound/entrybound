<!--
generated-by: python -m ebr report --spec research/experiments/EXP-HARNESS-SMOKE-CHUNK/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 4593671e2aecad82fe4f547d1ba874d1aa3f177f
experiment: EXP-HARNESS-SMOKE-CHUNK
input: research/normalized/EXP-HARNESS-SMOKE-CHUNK/summary.json sha256=a1a04fc463476ea48108247fbad158a60cfa8732e6f05eec10bcd4d672e353e9
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-HARNESS-SMOKE-CHUNK generated report index

Question: Does research/harness's ebr-chunk binary run end to end through the ebr runner (run -> verify-raw -> normalize -> report) on a tiny generated item?


Hypothesis: ebr-chunk's boundaries subcommand chunks the generated item with gear-norm-v1, exits 0, writes a valid ebr.harness.raw.v1 JSON row to stdout, and normalize/report complete without error.


Tables: [per candidate](per_candidate.md), [paired ratios](paired_ratios.md), [Pareto](pareto.md), [sensitivity](sensitivity.md)

Figures (PNG + plotted CSV):

- [wall_s_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-CHUNK/wall_s_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-CHUNK/wall_s_by_candidate.csv))
- [cpu_s_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-CHUNK/cpu_s_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-CHUNK/cpu_s_by_candidate.csv))
- [max_rss_bytes_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-CHUNK/max_rss_bytes_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-CHUNK/max_rss_bytes_by_candidate.csv))
