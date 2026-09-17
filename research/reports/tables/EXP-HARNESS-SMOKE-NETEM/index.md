<!--
generated-by: python -m ebr report --spec research/experiments/EXP-HARNESS-SMOKE-NETEM/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 4593671e2aecad82fe4f547d1ba874d1aa3f177f
experiment: EXP-HARNESS-SMOKE-NETEM
input: research/normalized/EXP-HARNESS-SMOKE-NETEM/summary.json sha256=d31be01428fae5317061c24f32e9d53fc79bcb68cff28fb4ed4a1218b0faa11a
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-HARNESS-SMOKE-NETEM generated report index

Question: Does research/harness's ebr-netem crate (ebr-origin binary) run end to end through the ebr runner (run -> verify-raw -> normalize -> report) serving a tiny generated item?


Hypothesis: ebr-origin serves the generated item over a real loopback HTTP GET, byte-identical to the source, exits with the roundtrip verified, and normalize/report complete without error.


Tables: [per candidate](per_candidate.md), [paired ratios](paired_ratios.md), [Pareto](pareto.md), [sensitivity](sensitivity.md)

Figures (PNG + plotted CSV):

- [wall_s_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-NETEM/wall_s_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-NETEM/wall_s_by_candidate.csv))
- [cpu_s_by_candidate.png](../../figures/EXP-HARNESS-SMOKE-NETEM/cpu_s_by_candidate.png) ([csv](../../figures/EXP-HARNESS-SMOKE-NETEM/cpu_s_by_candidate.csv))
