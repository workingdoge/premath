# Tusk Harness Multithread Runbook

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Define one deterministic coordinator/worker loop for `N` worktrees:

`issue_ready -> claim -> work -> verify -> release/update`

Authority boundary remains unchanged:

- mutation authority: `.premath/issues.jsonl`,
- projection artifacts only: harness-session / harness-feature / harness-trajectory.

## 2. Script Surface

- worker loop: `python3 tools/harness/multithread_loop.py worker`
- coordinator loop: `python3 tools/harness/multithread_loop.py coordinator`
- mise aliases:
  - `mise run harness-worker-loop`
  - `mise run harness-coordinator-loop`

Policy gate:

- control-plane default remains `instruction-linked`,
- this direct CLI loop is allowed only under explicit `human-override`,
- override requires `--override-reason` and is bounded by
  `workerLaneAuthority.mutationPolicy.compatibilityOverrides` epoch support.

## 3. Coordinator Startup (N worktrees)

Example (deterministic round-robin over three worktrees):

```sh
mise run harness-coordinator-loop -- \
  --worktree ../premath-w1 \
  --worktree ../premath-w2 \
  --worktree ../premath-w3 \
  --rounds 4 \
  --worker-prefix lane \
  --max-steps-per-worker 1 \
  --mutation-mode human-override \
  --override-reason 'operator approved local multithread batch' \
  --work-cmd 'true' \
  --verify-cmd 'mise run ci-check'
```

Determinism contract:

- worktrees are processed in sorted path order,
- dependency integrity is checked with `dep_diagnostics(graph_scope=active)`
  before each scheduling pass,
- each worker takes at most one issue per dispatch by default,
- each round re-checks `issue_ready` before dispatch,
- no secondary issue-memory authority is introduced.

Operator flow example (dependency integrity preflight):

```sh
cargo run --package premath-cli -- dep diagnostics --issues .premath/issues.jsonl --graph-scope active --json
```

MCP tool call shape:

```json
{
  "tool": "dep_diagnostics",
  "arguments": {
    "issuesPath": ".premath/issues.jsonl",
    "graphScope": "active"
  }
}
```

For forensic review of historical closure cycles:

```sh
cargo run --package premath-cli -- dep diagnostics --issues .premath/issues.jsonl --graph-scope full --json
```

## 4. Worker Loop Contract

Each worker step does:

1. `issue claim-next` with deterministic lease TTL.
2. `harness-session write` (`active`) projection update.
3. `harness-feature write` (`in_progress`) projection update.
4. execute `work-cmd`.
5. execute `verify-cmd`.
6. derive stop/handoff lease state from canonical issue memory (`.premath/issues.jsonl`)
   after mutation outcome (`active` | `stale` | `contended` | `released` | invariant mismatch).
7. success path:
   - `issue update --status closed` (releases lease via closed transition),
   - assert stop invariant (`status=closed` and lease released) from issue memory,
   - append `harness-trajectory` row with witness refs including deterministic
     `lease://handoff/...` reference and deterministic site lineage refs
     (`ctx://...`, `cover://...`, `refinement://...`),
   - `harness-feature write` (`completed`),
   - `harness-session write` (`stopped`) with matching lineage refs.
8. failure path:
   - issue-memory-derived lease state determines recovery action
     (`issue_lease_renew` / `issue_lease_release` / reclaim / stop),
   - append `harness-trajectory` row with lease handoff witness ref,
   - `harness-feature write` (`blocked`) carrying lease state/action summary,
   - `harness-session write` (`stopped`) carrying deterministic next-step derived
     from issue-memory lease state and matching lineage refs.

## 5. Heartbeat / Renew Guidance

For long-running work exceeding lease TTL, renew from MCP mutation tools:

- `issue_lease_renew(id, assignee, lease_id, lease_ttl_seconds|lease_expires_at)`

Run renewal on a fixed interval (for example every 10-15 minutes) bounded by the
same worker identity and lease id.

## 6. Stuck-Worker Recovery Guidance

Recovery sequence:

1. inspect work frontier:
   - `issue_ready`
   - `dep_diagnostics(graph_scope=active)`
   - `issue.blocked` / `issue_lease_projection` (MCP)
2. if lease owner is known and cooperative:
   - `issue_lease_release(id, assignee?, lease_id?)`
3. if worker died:
   - wait for stale lease boundary (or explicit release by operator),
   - reclaim with `issue claim-next`.
4. record recovery in projection artifacts (`harness-session`, trajectory row).

Determinism rule:

- stop/recovery recommendations must be derived from issue-memory lease state,
  never inferred from projection artifacts alone.

## 7. Artifact Interpretation

Projection artifacts are operational evidence only:

- `.premath/harness_session.json`: compact restart handoff,
- `.premath/harness_feature_ledger.json`: per-issue feature projection,
- `.premath/harness_trajectory.jsonl`: append-only step trace.

They do not grant mutation authority and do not replace issue-memory state.
