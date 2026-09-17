<!--
generated-by: python -m ebr report --spec research/experiments/EXP-HARNESS-SMOKE-PACK/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 4593671e2aecad82fe4f547d1ba874d1aa3f177f
experiment: EXP-HARNESS-SMOKE-PACK
input: research/normalized/EXP-HARNESS-SMOKE-PACK/summary.json sha256=a093a60dc0f1ee31fc397260d6c7cf6740c703a73d9a59af0a6ff9176ef388cc
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-HARNESS-SMOKE-PACK generated report index

Question: Does research/harness's ebr-pack binary run end to end through the ebr runner (run -> verify-raw -> normalize -> report) on a tiny generated item?


Hypothesis: ebr-pack packs the generated item, exits 0, writes a valid ebr.harness.raw.v1 JSON row to stdout, and normalize/report complete without error.


Tables: [per candidate](per_candidate.md), [paired ratios](paired_ratios.md), [Pareto](pareto.md), [sensitivity](sensitivity.md)

Figures (PNG + plotted CSV):

- [wall_s_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-PACK/wall_s_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-PACK/wall_s_by_candidate.csv))
- [cpu_s_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-PACK/cpu_s_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-PACK/cpu_s_by_candidate.csv))
- [max_rss_bytes_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-PACK/max_rss_bytes_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-PACK/max_rss_bytes_by_candidate.csv))
- [output_bytes_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-PACK/output_bytes_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-PACK/output_bytes_by_candidate.csv))
