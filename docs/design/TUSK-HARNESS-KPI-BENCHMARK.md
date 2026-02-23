# Tusk Harness KPI Benchmark

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Define one canonical multithread acceleration KPI and deterministic benchmark
procedure using existing harness projections.

## 2. Canonical KPI

KPI kind: `premath.multithread.throughput.v1`

Formula:

`kpi = throughput_per_worker_per_day * gate_pass_rate`

where:

- `throughput_per_worker_per_day = completed_rows_per_day / max(active_workers, 1)`
- `completed_rows_per_day = completed_rows * (24 / window_hours)`
- `gate_pass_rate = completed_rows / window_rows`

Row source: `.premath/harness_trajectory.jsonl` windowed by `finishedAt`.

## 3. Deterministic Benchmark Procedure

1. choose deterministic window (`window_hours`, default `24`),
2. load trajectory rows and sort descending by `(finishedAt, stepId, action)`,
3. classify success rows with canonical success classes,
4. compute counts/ratios/KPI,
5. evaluate thresholds and emit one decision state.

Command surface:

```sh
python3 tools/harness/benchmark_kpi.py --json
```

Mise alias:

```sh
mise run harness-kpi-report
```

## 4. Thresholds and Rollback Trigger

Default thresholds:

- target KPI: `0.8`
- rollback KPI: `0.4`
- minimum sample rows: `3`

Decision states:

- `pass`: KPI >= target,
- `watch`: rollback <= KPI < target,
- `rollback`: KPI < rollback,
- `insufficient_data`: window rows < minimum sample rows.

Rollback trigger:

- if decision state is `rollback`, treat as deterministic regression and pause
  multithread expansion until remediation is recorded.

## 5. Evidence and Lane Boundary

Benchmark references:

- trajectory projection path,
- harness session path,
- issue-memory authority path.

KPI output is control-plane operational evidence. It does not alter semantic
authority, checker verdicts, or issue-memory mutation authority.
