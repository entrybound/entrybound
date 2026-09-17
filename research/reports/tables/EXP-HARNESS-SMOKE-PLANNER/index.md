<!--
generated-by: python -m ebr report --spec research/experiments/EXP-HARNESS-SMOKE-PLANNER/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 4593671e2aecad82fe4f547d1ba874d1aa3f177f
experiment: EXP-HARNESS-SMOKE-PLANNER
input: research/normalized/EXP-HARNESS-SMOKE-PLANNER/summary.json sha256=885bc24951f4992899ed20b21a2ec2cbbbdb9a8fca905aa1ec89f9c37080c873
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-HARNESS-SMOKE-PLANNER generated report index

Question: Does research/harness's ebr-planner binary run end to end through the ebr runner (run -> verify-raw -> normalize -> report) on a tiny generated item?


Hypothesis: ebr-planner's drift/exhaustive-search/greedy/tree analysis over the generated item exits 0, writes valid ebr.harness.raw.v1 JSON rows to stdout, and normalize/report complete without error.


Tables: [per candidate](per_candidate.md), [paired ratios](paired_ratios.md), [Pareto](pareto.md), [sensitivity](sensitivity.md)

Figures (PNG + plotted CSV):

- [wall_s_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-PLANNER/wall_s_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-PLANNER/wall_s_by_candidate.csv))
- [cpu_s_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-PLANNER/cpu_s_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-PLANNER/cpu_s_by_candidate.csv))
- [max_rss_bytes_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-PLANNER/max_rss_bytes_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-PLANNER/max_rss_bytes_by_candidate.csv))
