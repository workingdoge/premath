---
slug: draft
shortname: SPEC-TRACEABILITY
title: workingdoge.com/premath/SPEC-TRACEABILITY
name: Draft Spec Traceability Matrix
status: draft
category: Informational
tags:
  - premath
  - checker
  - traceability
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Scope

This matrix maps promoted draft specs under `specs/premath/draft/` to their
current executable checker/test surfaces.

Purpose:

- keep spec coverage auditable from one place,
- classify coverage maturity consistently,
- make unmapped areas explicit as concrete coverage targets.

Coverage rows are repository-root claims. They reference executable checker
commands, fixtures, and crates in a full Premath checkout. A specs-only release
bundle may include this matrix without those executable surfaces; such a bundle
is not self-verifying unless paired with the repository checkout.

## 2. Coverage Status Classes

- `covered`: canonical executable vectors/checks exist in direct Cargo or
  Premath checker command surfaces.
- `instrumented`: deterministic checks/tests exist, but no dedicated canonical
  executable vector suite for the full spec contract.
- `gap`: no dedicated deterministic executable surface for the claimed contract.

## 3. Traceability Matrix (Draft Specs)

Authority classes are defined in `../AUTHORITY-MAP.json`.

KCIR substrate authority lives in `fish/sites/kcir`. Rows for `KCIR-CORE`,
`REF-BINDING`, `NF`, `NORMALIZER`, `WIRE-FORMATS`, and `ERROR-CODES` currently
track the Premath-side KCIR adapter/profile checks that remain in this repo for
path stability.

| Draft spec | Authority class | Primary executable surface | Status | Coverage target |
| --- | --- | --- | --- | --- |
| `DOCTRINE-INF.md` | `doctrine-preservation` | `premath coherence-check` (declaration-set, edge coherence, reachability, and doctrine-preservation boundary checks) | covered | - |
| `PREMATH-KERNEL.md` | `core` | `cargo test -p premath-kernel`; `cargo test -p premath-kernel --test toy_vectors`; `python3 tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures`; `python3 tools/kcir_toy/run_kcir_toy_vectors.py --fixtures tests/kcir_toy/fixtures` | covered | - |
| `KERNEL-STATEMENT-BINDINGS.json` | `kernel-index` | `premath traceability-check`; kernel statement-binding fixtures retained under `tests/checker/fixtures/statement-bindings` | instrumented | - |
| `KCIR-CORE.md` | `interop` | KCIR adapter fixtures retained under `tests/checker/fixtures/interop-core`; `premath ref project`; `premath ref verify`; `cargo test -p premath-cli` | instrumented | - |
| `REF-BINDING.md` | `interop` | `premath ref project`; `premath ref verify`; `cargo test -p premath-cli` | covered | - |
| `NF.md` | `interop` | normal-form fixtures retained under `tests/checker/fixtures/capabilities/capabilities.normal_forms`; kernel tests | instrumented | - |
| `NORMALIZER.md` | `interop` | normal-form fixtures and normalized comparison cases in `tests/checker/fixtures/capabilities/capabilities.instruction_typing`; kernel tests | instrumented | - |
| `WIRE-FORMATS.md` | `interop` | KCIR wire-format fixtures retained under `tests/checker/fixtures/interop-core`; `cargo test -p premath-cli` | instrumented | - |
| `ERROR-CODES.md` | `interop` | KCIR error-code fixtures retained under `tests/checker/fixtures/interop-core`; `cargo test -p premath-cli` | instrumented | - |
| `WITNESS-ID.md` | `core` | `cargo test -p premath-kernel` witness-id unit tests | covered | - |
| `OBLIGATION-DISCHARGE.md` | `core` | `premath coherence-check` (`scope_noncontradiction` obligation vocabulary parity); obligation fixtures retained under capability fixture directories | covered | - |
| `GATE.md` | `core` | `cargo test -p premath-kernel`; `cargo test -p premath-kernel --test toy_vectors`; `python3 tools/toy/run_toy_vectors.py --fixtures tests/toy/fixtures` | covered | - |
| `CHECKER-CLAIMS.md` | `checker-claims` | `premath traceability-check`; `premath coherence-check` | instrumented | - |
| `CAPABILITY-VECTORS.md` | `capability-registry` | `premath coherence-check` capability parity; capability fixtures retained under `tests/checker/fixtures/capabilities` | instrumented | - |
| `CHANGE-MORPHISMS.md` | `control-plane` | change-morphism fixture subset retained under `tests/checker/fixtures/capabilities/capabilities.change_morphisms`; required-projection Rust tests | instrumented | - |
| `DOCTRINE-SITE.md` | `control-plane` | `premath coherence-check`; `premath traceability-check` | covered | - |
| `DOCTRINE-SITE.json` | `control-plane` | `premath coherence-check`; `premath traceability-check` | covered | - |
| `DOCTRINE-SITE-INPUT.json` | `control-plane` | `premath coherence-check`; `premath traceability-check` | covered | - |
| `DOCTRINE-OP-REGISTRY.json` | `control-plane` | `premath coherence-check`; `premath traceability-check` | covered | - |
| `LLM-INSTRUCTION-DOCTRINE.md` | `control-plane` | `premath coherence-check`; instruction-typing fixtures retained under `tests/checker/fixtures/capabilities/capabilities.instruction_typing` | instrumented | - |
| `LLM-PROPOSAL-CHECKING.md` | `control-plane` | `premath proposal-check`; `cargo test -p premath-coherence proposal::tests` | covered | - |
| `PREMATH-COHERENCE.md` | `control-plane` | `premath coherence-check`; `cargo test -p premath-coherence`; CLI smoke tests | covered | - |
| `COHERENCE-CONTRACT.json` | `control-plane` | `premath coherence-check`; CLI smoke tests | covered | - |
| `CAPABILITY-REGISTRY.json` | `capability-registry` | `premath coherence-check`; `premath drift-budget-check` | covered | - |
| `CONTROL-PLANE-CONTRACT.json` | `control-plane` | `premath coherence-check`; `premath drift-budget-check`; required-projection Rust tests | covered | - |
| `UNIFICATION-DOCTRINE.md` | `control-plane` | `premath coherence-check` (`gate_chain_parity` Stage 1+Stage 2 parity/rollback/authority checks and Stage 3 typed-first closure mapping checks); `premath drift-budget-check`; decision-log traceability via `premath traceability-check` | covered | - |
| `SPAN-SQUARE-CHECKING.md` | `control-plane` | `premath coherence-check` (`span_square_commutation` via site vectors, including composition-law vectors) | covered | - |
| `SPEC-INDEX.md` | `authority-index` | `premath traceability-check` | covered | - |
| `SPEC-TRACEABILITY.md` | `authority-index` | `premath traceability-check` | covered | - |

## 4. Coverage Targets (Open Gaps/Upgrades)

- MH coverage belongs to its owning downstream site and downstream Kurma/Tusk
  wires, not to the promoted Premath draft matrix.

## 5. Maintenance Rules

- Every promoted draft spec MUST have exactly one matrix row in this document.
- `gap` rows MUST reference a concrete target ID.
- Coverage target implementation work SHOULD be tracked as discovered issues
  linked from the active traceability issue chain.
