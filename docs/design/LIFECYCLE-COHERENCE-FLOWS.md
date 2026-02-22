# Lifecycle and Coherence Flows

Status: draft
Scope: design-level, non-normative

Normative anchors:

- `specs/premath/draft/CONTROL-PLANE-CONTRACT.json`
- `specs/premath/draft/PREMATH-COHERENCE.md`
- `specs/premath/draft/UNIFICATION-DOCTRINE.md`

## 1. Schema Lifecycle Flow (Control Plane)

```text
CONTROL-PLANE-CONTRACT.schemaLifecycle
  -> tools/ci/control_plane_contract.py::load_control_plane_contract
     -> validate activeEpoch format (YYYY-MM)
     -> validate required kind families
     -> resolve alias kinds -> canonical kinds
     -> enforce epoch discipline:
        - one shared rollover epoch across aliases
        - runway = supportUntilEpoch - activeEpoch
        - 0 < runway <= 12 months
  -> CI client validators consume canonical kinds only
  -> emitted required/instruction witness payloads stay canonical
```

Deterministic reject reasons include:

- malformed or missing `schemaLifecycle` fields,
- unknown kind family or kind,
- expired alias window,
- mixed alias rollover epochs,
- non-positive or overlong rollover runway.

## 2. Coherence Gate-Chain Flow

```text
premath coherence-check
  -> load COHERENCE-CONTRACT + CONTROL-PLANE-CONTRACT
  -> obligation: gate_chain_parity
     -> evaluate_control_plane_schema_lifecycle
        - require schemaLifecycle presence
        - enforce required kind families
        - resolve contract/projection/witness/policy kinds
        - fail closed on malformed/expired/unsupported lifecycle entries
  -> emit premath.coherence.v1 witness
```

Deterministic failure class:

- `coherence.gate_chain_parity.schema_lifecycle_invalid`

## 3. Operator Loop

```text
edit CONTROL-PLANE-CONTRACT.json
  -> mise run ci-pipeline-test
  -> mise run coherence-check
  -> mise run ci-drift-budget-check
  -> mise run baseline
```

Rollover update checklist:

1. update `schemaLifecycle.activeEpoch`,
2. keep one shared alias `supportUntilEpoch` across families,
3. keep rollover runway within 1..12 months,
4. remove aliases that should no longer be accepted,
5. append/update decision log entry in `specs/process/decision-log.md`.
