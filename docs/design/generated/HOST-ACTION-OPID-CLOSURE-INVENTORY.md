# Host-Action OperationId Closure Inventory (Generated)

Status: historical KR0 inventory snapshot (non-authoritative)
Generated: 2026-02-25
Source: `specs/premath/draft/CONTROL-PLANE-CONTRACT.json`

## Summary

- `hostActionSurface.requiredActions` at KR0 snapshot: 41
- rows with missing `operationId` at KR0 snapshot: 12
- KR1/KR2 closure status: canonical operation IDs/classes/routes live in
  `specs/premath/draft/DOCTRINE-OP-REGISTRY.json` and
  `specs/premath/draft/DOCTRINE-SITE.json`

## Unmapped Actions (KR0 Snapshot)

| Host action | Canonical CLI | Proposed operationId | Proposed operationClass | Route candidate | Notes |
| --- | --- | --- | --- | --- | --- |
| `coherence.check` | `premath coherence-check --contract <path> --repo-root <repo> --json` | `op/ci.coherence_check` | `tooling_only` | - | Core checker entrypoint; semantic authority already in core checker. |
| `harness.feature.check` | `premath harness-feature check --path <path> [--require-closure] --json` | `op/harness.feature_check` | `read_only_projection` | - | Projection/validation over feature ledger. |
| `harness.feature.next` | `premath harness-feature next --path <path> --json` | `op/harness.feature_next` | `read_only_projection` | - | Deterministic next-feature projection. |
| `harness.feature.read` | `premath harness-feature read --path <path> --json` | `op/harness.feature_read` | `read_only_projection` | - | Read/query surface. |
| `harness.feature.write` | `premath harness-feature write --path <path> ... --json` | `op/harness.feature_write` | `tooling_only` | - | Local ledger mutation utility; not yet route-bound. |
| `harness.trajectory.append` | `premath harness-trajectory append --path <path> ... --json` | `op/harness.trajectory_append` | `tooling_only` | - | Local append lane for trajectory rows. |
| `harness.trajectory.query` | `premath harness-trajectory query --path <path> --mode <mode> --limit <n> --json` | `op/harness.trajectory_query` | `read_only_projection` | - | Query/projection over trajectory lane. |
| `required.decision_verify` | `premath required-decision-verify --input <path> --json` | `op/ci.verify_required_decision` | `route_bound` | `route.required_decision_attestation` | Should align with CI witness attestation family. |
| `required.delta` | `premath required-delta --input <path> --json` | `op/ci.required_delta` | `tooling_only` | - | Deterministic delta derivation utility. |
| `required.gate_ref` | `premath required-gate-ref --input <path> --json` | `op/ci.required_gate_ref` | `tooling_only` | - | Deterministic gate ref builder utility. |
| `required.projection` | `premath required-projection --input <path> --json` | `op/ci.required_projection` | `tooling_only` | - | Deterministic projection utility. |
| `required.witness` | `premath required-witness --runtime <path> --json` | `op/ci.required_witness` | `tooling_only` | - | Witness assembly utility; not decision attestation endpoint itself. |

## KR1 Implementation Notes

- Add these operation IDs to:
  - `specs/premath/draft/CONTROL-PLANE-CONTRACT.json` (`hostActionSurface.requiredActions.*.operationId`)
  - `specs/premath/draft/DOCTRINE-OP-REGISTRY.json` (`operations[]`)
- Keep route binding changes scoped to explicit `route_bound` entries only.
- Leave class/routing for non-route utilities explicit to prevent implicit authority creep.
