#!/usr/bin/env bash
# Reproduce the Linux-only failure of
# cross_file_compression::small_cohort_stays_independent_and_v2_v3_remain_readable
# observed at the research baseline (9e44608) during the research-internals identity check.
set -euo pipefail
cd /mnt/d/Projects/entrybound/entrybound
export CARGO_TARGET_DIR=/root/eb-research/target/repro-cross-file
/root/.cargo/bin/cargo +1.98.1 test --release --locked -p entrybound --test cross_file_compression \
  small_cohort_stays_independent_and_v2_v3_remain_readable -- --exact --nocapture 2>&1 | tail -40
