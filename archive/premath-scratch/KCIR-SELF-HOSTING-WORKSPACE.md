# KCIR Self-Hosting Workspace

Date: 2026-02-23
Lane: operations/planning
Status: active

## Goal

Move from parallel control-plane checker logic to one canonical
`compile -> KCIR -> Gate -> witness` decision spine for control-plane artifacts.

## Boundary rules

1. Keep semantic admissibility authority in kernel/gate contracts.
2. Keep wrappers/adapters orchestration-only.
3. Preserve fail-closed behavior and deterministic witness lineage.
4. Apply staged migration: parity first, authority cutover second.

## Scope (phase 2 self-hosting)

- Declare the control-plane bundle profile explicitly.
- Canonicalize control-plane artifacts into machine-readable KCIR mappings.
- Add a unified control-plane compile/gate/witness command path.
- Prove result + failure-class parity against current checker surfaces before cutover.
- Add conformance vectors for accept/reject/invariance, including wrapper-boundary adversarial cases.
- Capture cutover checkpoint + rollback evidence in operations lane.

## Issue chain

- Epic: `bd-262`
- Task A (bundle profile): `bd-263`
- Task B (KCIR mappings): `bd-264`
- Task C (unified decision core path): `bd-265`
- Task D (parity + adapter hard-boundary): `bd-266`
- Task E (conformance vectors): `bd-267`
- Task F (docs/traceability + decision closure): `bd-268`

## Exit criteria

1. One authoritative decision schema across control-plane checks.
2. Legacy checker wrappers remain transport/adapters only.
3. Conformance + docs coherence + traceability gates remain green.
