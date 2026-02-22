# Architecture Map (One Page)

Status: draft
Scope: design-level, non-normative

## 1. Layer Stack

`Doctrine` (what must be preserved):
- `specs/premath/draft/DOCTRINE-INF.md`
- `specs/premath/draft/DOCTRINE-SITE.md`
- `specs/premath/draft/LLM-INSTRUCTION-DOCTRINE.md`
- `specs/premath/draft/LLM-PROPOSAL-CHECKING.md`

`Kernel` (semantic authority):
- `specs/premath/draft/PREMATH-KERNEL.md`
- `specs/premath/draft/GATE.md`
- `specs/premath/draft/BIDIR-DESCENT.md`

`Runtime` (execution inside/between worlds):
- `specs/premath/raw/TUSK-CORE.md`
- `specs/premath/raw/SQUEAK-CORE.md`
- `specs/premath/raw/SQUEAK-SITE.md`

`CI/Control` (one layer, two roles):
- `specs/premath/raw/PREMATH-CI.md`
- `specs/premath/raw/CI-TOPOS.md`
- `specs/premath/draft/PREMATH-COHERENCE.md`
- `specs/premath/draft/COHERENCE-CONTRACT.json`

Role split inside CI/Control:
- check role: `PREMATH-COHERENCE` (`premath coherence-check`)
- execute/attest role: `PREMATH-CI` + `CI-TOPOS` (`pipeline_*`, `run_*`, verify/decide)

`Operational surfaces` (scripts/tasks):
- `tools/ci/pipeline_required.py`
- `tools/ci/pipeline_instruction.py`
- `tools/ci/run_required_checks.py`
- `tools/ci/verify_required_witness.py`
- `tools/ci/run_instruction.py` / `tools/ci/run_instruction.sh`
- `tools/ci/run_gate.sh`
- `tools/conformance/check_doctrine_site.py`
- `tools/conformance/run_doctrine_inf_vectors.py`
- `premath coherence-check` (`crates/premath-coherence` + `premath-cli`)
- `hk.pkl`, `.mise.toml`

## 2. Doctrine to Operation Path

```text
DOCTRINE-INF
  -> DOCTRINE-SITE (nodes/covers/edges)
  -> LLM-INSTRUCTION-DOCTRINE
  -> LLM-PROPOSAL-CHECKING
  -> Control Plane
     -> check role: PREMATH-COHERENCE / COHERENCE-CONTRACT
        -> premath coherence-check
     -> execute/attest role: PREMATH-CI / CI-TOPOS
        -> tools/ci/pipeline_required.py / tools/ci/pipeline_instruction.py
        -> tools/ci/run_required_checks.py
        -> tools/ci/verify_required_witness.py
        -> tools/ci/run_gate.sh
  -> tools/conformance/check_doctrine_site.py / run_doctrine_inf_vectors.py
  -> hk/mise tasks (.mise baseline + ci-required-attested)
  -> CIWitness artifacts
  -> conformance + doctrine-site checks
```

Authority rule:
- semantic admissibility comes from kernel/gate contracts, not from runners or hooks.
- coherence and CI are control-plane roles, not semantic authority layers.
- runners/profiles (`local`, `external`, infra bindings) change execution substrate only.

## 3. Instruction Runtime Loop

```text
InstructionEnvelope
  -> classify: typed(kind) | unknown(reason)
  -> apply typingPolicy.allowUnknown
  -> project requested checks
  -> execute checks via run_gate
  -> emit CI witness with:
     instructionDigest + instructionClassification + typingPolicy
```

Deterministic rejection path:
- `unknown(reason)` with `allowUnknown=false` rejects before check execution with
  `instruction_unknown_unroutable`.

## 4. Work-Memory Authority Loop

```text
WorkMemory (canonical JSONL substrate)
  -> InstructionMorphisms (typed, policy-bound mutation requests)
  -> Witnesses (instruction-bound + optional JJ snapshot linkage)
  -> QueryProjection (rebuildable read/index layer; non-authoritative)
```

Repository default profile:
- canonical memory: `.premath/issues.jsonl` (`premath-bd`)
- MCP mutation policy: `instruction-linked`
  - mutation authorization is policy-scoped and capability-scoped from accepted
    instruction witnesses (`policyDigest` + `capabilityClaims`)
- query backend default: `jsonl` (with optional `surreal` projection mode)

## 5. Refinement Loop

```text
issue_ready -> issue_blocked -> issue_claim -> instruction_run -> witness
  -> issue_lease_renew (long task) or issue_lease_release (handoff)
  -> issue_discover (when new work is found) -> issue_ready
```

Loop intent:
- keep sessions short and restartable,
- prevent lost/discarded discovered work,
- keep mutation authority instruction-mediated with auditable witnesses.

## 6. Conformance Closure

Baseline gate (`mise run baseline`) enforces:
- setup/lint/build/test/toy suites,
- conformance + traceability + coherence-check + docs-coherence + doctrine closure,
- CI/control-plane wiring, pipeline, observation, and instruction checks,
- executable fixture-suite closure (`mise run conformance-run`).

Operational source of truth for baseline composition is `.mise.toml`
(`[tasks.baseline]`).

Projected required gate (`mise run ci-required`) enforces:
- deterministic `Delta -> requiredChecks` projection,
- execution of projected checks only,
- CI closure witness emission (`artifacts/ciwitness/proj1_*.json`).

Authoritative verification (`mise run ci-verify-required`) enforces:
- projection/witness digest consistency,
- required/executed check-set consistency,
- verdict/failure-class consistency with check results.

Instruction doctrine is executable via:
- `capabilities.instruction_typing`
- `capabilities.ci_witnesses`
- `draft/LLM-PROPOSAL-CHECKING` proposal ingest/discharge path
- `capabilities.change_morphisms`
