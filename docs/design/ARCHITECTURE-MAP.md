# Architecture Map (One Page)

Status: draft
Scope: design-level, non-normative

## 1. Layer Stack

`Doctrine` (what must be preserved):
- `specs/premath/draft/DOCTRINE-INF.md`
- `specs/premath/draft/DOCTRINE-SITE.md`
- `specs/premath/draft/LLM-INSTRUCTION-DOCTRINE.md`

`Kernel` (semantic authority):
- `specs/premath/draft/PREMATH-KERNEL.md`
- `specs/premath/draft/GATE.md`
- `specs/premath/draft/BIDIR-DESCENT.md`

`Runtime` (execution inside/between worlds):
- `specs/premath/raw/TUSK-CORE.md`
- `specs/premath/raw/SQUEAK-CORE.md`
- `specs/premath/raw/SQUEAK-SITE.md`

`CI/Control` (closure and attestation):
- `specs/premath/raw/PREMATH-CI.md`
- `specs/premath/raw/CI-TOPOS.md`

`Operational surfaces` (scripts/tasks):
- `tools/ci/run_instruction.py`
- `tools/ci/run_gate.sh`
- `hk.pkl`, `.mise.toml`, `justfile`

## 2. Doctrine to Operation Path

```text
DOCTRINE-INF
  -> DOCTRINE-SITE (nodes/covers/edges)
  -> LLM-INSTRUCTION-DOCTRINE
  -> PREMATH-CI / CI-TOPOS
  -> tools/ci/run_instruction.py
  -> tools/ci/run_gate.sh
  -> hk/mise tasks
  -> CIWitness artifacts
  -> conformance + doctrine-site checks
```

Authority rule:
- semantic admissibility comes from kernel/gate contracts, not from runners or hooks.
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

## 4. Conformance Closure

Baseline gate (`mise run baseline`) enforces:
- build/test/toy suites,
- doctrine-site coherence (`tools/conformance/check_doctrine_site.py`),
- executable capability vectors (`tools/conformance/run_capability_vectors.py`).

Instruction doctrine is executable via:
- `capabilities.instruction_typing`
- `capabilities.ci_witnesses`
