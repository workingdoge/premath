---
slug: draft
shortname: BIDIR-DESCENT
title: workingdoge.com/premath/BIDIR-DESCENT
name: Bidirectional Synthesis/Checking with Descent Obligations
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - gate
  - bidirectional
  - descent
  - refinement
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## Language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**,
**SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this
specification are to be interpreted as described in RFC 2119 (and RFC 8174 for
capitalization).

## 1. Overview

This specification defines the operational execution model for **full-profile**
Premath verification:

- **Synthesis** on authored contexts produces values plus provenance.
- **Checking** on target contexts produces **obligations** (what must be shown
  for admissibility under the Gate).
- **Discharge** validates obligations (or rejects with witnesses), using a
  deterministic **normalizer** in `normalized` mode.

This document is architecture-level:
it does not change KCIR wire format, commitment profiles, NF grammars, or opcode
contracts. It defines how full-profile verifiers orchestrate those components to
enforce `draft/GATE`.

## 2. Context model and mode discipline

Let `C` be the full context space and `S ⊂ C` the authored context subset
(base + selected modifier contexts).

Implementations MUST enforce:

1. Positions in `S` are evaluated in **synthesis mode**.
2. Positions in `C` are evaluated in **checking mode** (unless they are in `S`).
3. Implementations MUST NOT silently treat derived/checking results as authored inputs.

A claimed derived value MUST be traceable to:

- synthesized authored facts,
- obligation discharge steps, and
- the declared normalization/refinement policy (if used).

### 2.1 Context API (required)

A full-profile implementation MUST have a deterministic Context API sufficient to:

- enumerate the authored subset `S`,
- identify a target context `c ∈ C`,
- determine declared cover families for contexts (via the Base API and cover store),
- and serialize contexts deterministically for witness IDs (as required by `draft/WITNESS-ID`).

This spec does not mandate a specific context representation.

## 3. Normalizer interface (required)

Gate discharge in `normalized` mode depends on a deterministic normalizer.

### 3.1 Normalizer ID

The implementation MUST define a stable identifier:

- `normalizerId: string`

This MUST change if any normalization behavior that affects comparisons changes
(e.g. fusion rules, canonical ordering, cover normalization behavior).

### 3.2 Policy digest

If the implementation supports multiple refinement/normalization policies, it MUST define:

- `policyDigest: bytes32`

that commits to all policy parameters affecting normalization and comparisons.

`policyDigest` and `normalizerId` MUST be emitted in any `normalized`-mode witness/certificate output.

### 3.3 Normalizer function

A full-profile implementation MUST provide:

- `normalize(kind, valueRef, envSig, uid, policy) -> Normalized`

Where:

- `kind ∈ {"obj","mor"}` indicates whether `valueRef` is an ObjNF or MorNF output.
- `valueRef` is a committed reference to the value (often `out: Ref` from a KCIR node).
- `envSig, uid` are the KCIR DAG invariants of the verification run.
- `policy` selects refinement/normalization mode.

`Normalized` MUST include:

- `cmpRef: Ref` (comparison key)
- `normalizerId`
- `policyDigest`

`cmpRef` MUST be computed via the backend-generic binding interface in `draft/REF-BINDING`:
it MUST equal `project_ref(domain, payload_bytes(normBytes))` for the corresponding NF domain.

Normalization MUST be deterministic given fixed inputs, store content, profile
params, anchors, `normalizerId`, and `policyDigest`.

## 4. Judgments

### 4.1 Synthesis

Synthesis judgment:

- `Γ ⊢ t@s ↑ τ ▷ v, p`

Where:

- `s ∈ S`,
- `τ` is the synthesized type (implementation-defined; often {OBJ,MOR}),
- `v` is the synthesized value reference (typically a KCIR output Ref),
- `p` is provenance sufficient to identify authored source(s).

Synthesis MUST be deterministic for fixed inputs, profile parameters, and
policy/refinement settings.

### 4.2 Checking

Checking judgment:

- `Γ ⊢ t@c ↓ τ ⇝ (v?, O)`

Where:

- `c ∈ C`,
- `v?` is an OPTIONAL candidate value ref at `c`,
- `O` is an obligation set to be discharged.

Checking MUST NOT fabricate authored definitions.

### 4.3 Obligation discharge

Discharge judgment:

- `Γ ⊢ O ✓` (accepted), or
- `Γ ⊢ O ✗ W` (rejected with witnesses `W`).

Witness identifiers and ordering MUST be deterministic.

### 4.4 LLM proposal ingestion (checking-only)

When LLM-generated proposal artifacts are used, implementations MUST treat them
as untrusted checking inputs (see `draft/LLM-PROPOSAL-CHECKING`).

Implementations MUST enforce:

1. LLM proposal payloads enter checking mode only,
2. LLM proposal payloads MUST NOT be inserted directly into authored subset `S`,
3. proposal claims MUST compile to obligations before discharge,
4. acceptance remains discharge-determined (`Γ ⊢ O ✓`), never proposal-determined.

For proposal checks in `normalized` mode, deterministic binding to
`(normalizerId, policyDigest)` is REQUIRED as in §3 and §7.

## 5. Refinement / comparison modes

Implementations MUST expose at least one mode:

- `normalized` mode (REQUIRED): compare post-normalization committed outputs

An implementation MAY additionally expose:

- `semantic` mode (OPTIONAL): compare structured intent before full normalization

Any emitted witness/certificate set under `normalized` mode MUST bind to:

- `normalizerId`
- `policyDigest`

so results cannot be replayed across policy/normalizer changes.

## 6. Obligation kinds (normative)

A conforming full-profile implementation MUST support obligations covering at least:

1. `stability` — functorial reindexing (GATE-3.1)
2. `locality` — cover restriction existence (GATE-3.2)
3. `descent_exists` — gluing existence (GATE-3.3)
4. `descent_contractible` — contractible glue space (GATE-3.4)
5. `adjoint_triple` — Sigma/f*/Pi coherence (GATE-3.5) **only if advertised**

Implementations MAY use the following operational obligations, which MUST map into Gate classes deterministically:

6. `ext_gap` — no derivation/transport path for a required target context
7. `ext_ambiguous` — multiple incomparable maximal derivations (non-contractible choice)

### 6.1 Obligation record format (required)

Each obligation MUST have a deterministic serialization sufficient to compute
a stable ID. At minimum:

- `kind`
- `ctx` (serialized)
- `subject` (what value is being checked; at minimum `kind` + `Ref`)
- `details` (kind-specific data)

Implementations MAY add fields, but MUST keep canonical serialization stable.

## 7. Discharge requirements (normative)

Discharge MUST be deterministic and MUST either accept or reject with witnesses.

### 7.1 Discharge in `normalized` mode (required)

In `normalized` mode, the verifier MUST discharge obligations by:

- normalizing any compared values via §3.3, and
- comparing the resulting `cmpRef: Ref` values for equality.

If equality fails, discharge MUST reject with a witness of the correct Gate class.

If a discharge step compares two values under `normalized` mode, the implementation MUST ensure
the same `(normalizerId, policyDigest)` are used on both sides. If they differ, discharge MUST reject
deterministically (verifier contract violation).

### 7.2 Discharge in `semantic` mode (optional)

In `semantic` mode, the verifier MAY compare pre-normalized structure,
but MUST still be able to emit valid Gate failure classes and MUST record
that `semantic` mode was used.

## 8. Witnessing and mapping to Gate

### 8.1 Required mapping (normative)

The following mapping is normative for full-profile implementations:

- `stability` failures -> `stability_failure` (`GATE-3.1`)
- `locality` failures -> `locality_failure` (`GATE-3.2`)
- `descent_exists` / `ext_gap` -> `descent_failure` (`GATE-3.3`)
- `descent_contractible` / `ext_ambiguous` -> `glue_non_contractible` (`GATE-3.4`)
- `adjoint_triple` -> `adjoint_triple_coherence_failure` (`GATE-3.5`)

### 8.2 Witness format

Rejected checks MUST emit Gate witness payloads as specified by `draft/GATE` §4.1.

`witnessId` values MUST be computed per `draft/WITNESS-ID`.
If the implementation adds details, it MUST do so under `details` without breaking
schema compatibility.

Witness arrays MUST be deterministically ordered as required by `draft/GATE`.

## 9. Conformance requirements

Full-profile implementations MUST:

1. enforce mode discipline in §2,
2. implement normalization in §3,
3. expose deterministic discharge outcomes (`✓` / `✗ W`),
4. emit Gate rejection classes using the mapping in §8.

## 10. Security and robustness

Implementations MUST treat authored inputs, certificates, stores, and witness
payloads as untrusted.

Implementations SHOULD:

- bound recursion/graph depth and obligation expansion,
- fail closed on malformed or incomplete mode/provenance metadata,
- produce deterministic machine-readable error codes for CI.

## 11. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.refine.context`
- `dm.refine.cover`
- `dm.profile.evidence` (for fixed semantic inputs + fixed bindings)
- `dm.policy.rebind` (normalized-mode binding checks are explicit)

Not preserved:

- `dm.transport.world` (handled by `raw/SQUEAK-CORE`)
- `dm.transport.location` (handled by `raw/SQUEAK-SITE`)
- `dm.profile.execution` (handled by runtime/CI layer)
- `dm.presentation.projection` (handled by projection layer)
