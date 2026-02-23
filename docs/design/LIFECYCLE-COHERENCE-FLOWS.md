# Lifecycle and Coherence Flows

Status: draft
Scope: design-level, non-normative

Normative anchors:

- `specs/premath/draft/CONTROL-PLANE-CONTRACT.json`
- `specs/premath/draft/PREMATH-COHERENCE.md`
- `specs/premath/draft/UNIFICATION-DOCTRINE.md`
- `specs/process/SCHEMA-LIFECYCLE-GOVERNANCE.md`

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
     -> enforce governance mode contract:
        - mode = rollover|freeze
        - decisionRef + owner required
        - rollover: cadence required, aliases required, runway <= cadence
        - freeze: freezeReason required, aliases forbidden
  -> CI client validators consume canonical kinds only
  -> emitted required/instruction witness payloads stay canonical
```

Deterministic reject reasons include:

- malformed or missing `schemaLifecycle` fields,
- unknown kind family or kind,
- expired alias window,
- mixed alias rollover epochs,
- non-positive or overlong rollover runway.
- governance-mode violations (missing/invalid mode fields, rollover/freeze
  mismatch with alias state).

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
2. keep governance mode `rollover` with explicit `rolloverCadenceMonths`,
3. keep one shared alias `supportUntilEpoch` across families,
4. keep rollover runway within `1..rolloverCadenceMonths` (max 12),
5. remove aliases that should no longer be accepted,
6. append/update decision log entry in `specs/process/decision-log.md`,
7. update `schemaLifecycle.governance.decisionRef`.

Freeze update checklist:

1. set governance mode `freeze`,
2. set `freezeReason`,
3. remove all compatibility aliases,
4. append/update decision log entry in `specs/process/decision-log.md`,
5. update `schemaLifecycle.governance.decisionRef`.
