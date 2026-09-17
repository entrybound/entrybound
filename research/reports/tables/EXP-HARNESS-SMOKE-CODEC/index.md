<!--
generated-by: python -m ebr report --spec research/experiments/EXP-HARNESS-SMOKE-CODEC/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 4593671e2aecad82fe4f547d1ba874d1aa3f177f
experiment: EXP-HARNESS-SMOKE-CODEC
input: research/normalized/EXP-HARNESS-SMOKE-CODEC/summary.json sha256=a8667072b63c1544e2705a5db3bb1ac12700301f6c1da017d57f47d73cb8e60e
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-HARNESS-SMOKE-CODEC generated report index

Question: Does research/harness's ebr-codec binary run end to end through the ebr runner (run -> verify-raw -> normalize -> report) on a tiny generated item?


Hypothesis: ebr-codec's default compare subcommand round-trips the generated item through production zstd at level 3 vs the external zstd crate, exits 0, writes a valid ebr.harness.raw.v1 JSON row to stdout, and normalize/report complete without error.


Tables: [per candidate](per_candidate.md), [paired ratios](paired_ratios.md), [Pareto](pareto.md), [sensitivity](sensitivity.md)

Figures (PNG + plotted CSV):

- [wall_s_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-CODEC/wall_s_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-CODEC/wall_s_by_candidate.csv))
- [cpu_s_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-CODEC/cpu_s_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-CODEC/cpu_s_by_candidate.csv))
- [max_rss_bytes_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-CODEC/max_rss_bytes_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-CODEC/max_rss_bytes_by_candidate.csv))
