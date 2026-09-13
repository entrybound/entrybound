<!--
generated-by: python -m ebr report --spec research/experiments/EXP-SMOKE-000/spec.yaml
tool: ebr 0.1.0 (research/tools/ebr)
tool-git-sha: 9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155
experiment: EXP-SMOKE-000
input: research/normalized/EXP-SMOKE-000/summary.json sha256=dcf2d785f7f08e69d9ea4dd96168b3bfd0770f3f6b3639a2490de77f2a0d8256
do-not-edit: regenerate with the generated-by command; no number here is hand-entered
-->
# EXP-SMOKE-000 generated report index

Question: Does the ebr pipeline (run -> verify-raw -> normalize -> report) work end to end on a generated 1 MiB file compressed with gzip inside WSL?


Hypothesis: All samples exit 0, round-trip checks pass, gzip output is byte-identical across repetitions (gzip -n), and normalize/report regenerate byte-identical outputs.


Tables: [per candidate](per_candidate.md), [paired ratios](paired_ratios.md), [Pareto](pareto.md), [sensitivity](sensitivity.md)

Figures (PNG + plotted CSV):

- [cpu_s_by_candidate.png](../../figures/EXP-SMOKE-000/cpu_s_by_candidate.png) ([csv](../../figures/EXP-SMOKE-000/cpu_s_by_candidate.csv))
- [max_rss_bytes_by_candidate.png](../../figures/EXP-SMOKE-000/max_rss_bytes_by_candidate.png) ([csv](../../figures/EXP-SMOKE-000/max_rss_bytes_by_candidate.csv))
- [scratch_peak_bytes_by_candidate.png](../../figures/EXP-SMOKE-000/scratch_peak_bytes_by_candidate.png) ([csv](../../figures/EXP-SMOKE-000/scratch_peak_bytes_by_candidate.csv))
- [output_bytes_by_candidate.png](../../figures/EXP-SMOKE-000/output_bytes_by_candidate.png) ([csv](../../figures/EXP-SMOKE-000/output_bytes_by_candidate.csv))
- [ratio_by_candidate.png](../../figures/EXP-SMOKE-000/ratio_by_candidate.png) ([csv](../../figures/EXP-SMOKE-000/ratio_by_candidate.csv))
- [paired_ratio_cpu_s.png](../../figures/EXP-SMOKE-000/paired_ratio_cpu_s.png) ([csv](../../figures/EXP-SMOKE-000/paired_ratio_cpu_s.csv))
- [paired_ratio_max_rss_bytes.png](../../figures/EXP-SMOKE-000/paired_ratio_max_rss_bytes.png) ([csv](../../figures/EXP-SMOKE-000/paired_ratio_max_rss_bytes.csv))
- [paired_ratio_scratch_peak_bytes.png](../../figures/EXP-SMOKE-000/paired_ratio_scratch_peak_bytes.png) ([csv](../../figures/EXP-SMOKE-000/paired_ratio_scratch_peak_bytes.csv))
- [paired_ratio_output_bytes.png](../../figures/EXP-SMOKE-000/paired_ratio_output_bytes.png) ([csv](../../figures/EXP-SMOKE-000/paired_ratio_output_bytes.csv))
- [paired_ratio_ratio.png](../../figures/EXP-SMOKE-000/paired_ratio_ratio.png) ([csv](../../figures/EXP-SMOKE-000/paired_ratio_ratio.csv))
- [pareto_output_bytes_vs_cpu_s.png](../../figures/EXP-SMOKE-000/pareto_output_bytes_vs_cpu_s.png) ([csv](../../figures/EXP-SMOKE-000/pareto_output_bytes_vs_cpu_s.csv))
