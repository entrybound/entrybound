<!--
generated-by: python -m ebr report --spec research/experiments/EXP-HARNESS-SMOKE-ACCESS/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 4593671e2aecad82fe4f547d1ba874d1aa3f177f
experiment: EXP-HARNESS-SMOKE-ACCESS
input: research/normalized/EXP-HARNESS-SMOKE-ACCESS/summary.json sha256=41f595f75bf4f9520166cbc6996becad38ba72c3cafd6a969eaea62d65bb8b72
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-HARNESS-SMOKE-ACCESS generated report index

Question: Does research/harness's ebr-access binary run end to end through the ebr runner (run -> verify-raw -> normalize -> report) on a tiny generated item?


Hypothesis: ebr-access's roundtrip mode plans/encodes the generated item and measures random access and stream open over it, exits 0, writes a valid ebr.harness.raw.v1 JSON row to stdout, and normalize/report complete without error.


Tables: [per candidate](per_candidate.md), [paired ratios](paired_ratios.md), [Pareto](pareto.md), [sensitivity](sensitivity.md)

Figures (PNG + plotted CSV):

- [wall_s_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-ACCESS/wall_s_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-ACCESS/wall_s_by_candidate.csv))
- [cpu_s_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-ACCESS/cpu_s_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-ACCESS/cpu_s_by_candidate.csv))
- [max_rss_bytes_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-ACCESS/max_rss_bytes_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-ACCESS/max_rss_bytes_by_candidate.csv))
