---
slug: draft
shortname: SPEC-INDEX
title: workingdoge.com/premath/SPEC-INDEX
name: Spec Index and Conformance Profiles
status: draft
category: Informational
tags:
  - premath
  - kernel
  - conformance
  - index
editor: arj <arj@workingdoge.com>
contributors: []
---

## License

This specification is dedicated to the public domain under **CC0 1.0** (see
`../../../LICENSE`).

## Change Process

This document is governed by the process in `../../process/coss.md`.

## 1. Purpose

This file is the **front door**. It answers:

- what is normative vs. informative,
- what claims an implementation may make,
- and how the ecosystem compiles to a small, checkable kernel.

Premath is designed to be **host-agnostic**. We treat “typechecker” and “∞-cosmos”
as *examples of external host bases* `B` in which Premath meanings are realized and/or checked.
Whether `B` is presented as a “type”, “term”, or other meta-object is an implementation detail.

Normative conformance requirements live in `draft/CONFORMANCE`.

## 2. The Premath shape (one total, two bases)

Premath’s semantic kernel is the fibre-space projection:

- `p₀ : E → 𝒞` where `𝒞` is the context world (covers/refinements are declared on it),
  and `E` is the total space of definables-in-context.

Implementations additionally choose an external host base `B` and a realization `F`
into a Premath-shaped bundle over `B`. See `draft/PREMATH-KERNEL` for the exact diagram and roles.

## 3. Claims and profiles (capability-based)

Premath avoids prescribing a single internal architecture. Instead, conformance is
defined by **claims**. An implementation MUST satisfy the requirements of every claim it asserts.

Claims are grouped into profiles:

- **Kernel**: semantic law only (reindexing coherence + contractible descent + refinement invariance).
- **Interop Core**: deterministic, exchangeable artifacts (KCIR + NF + ref binding + wire/errors).
- **Interop Full**: `Interop Core` + deterministic normalization, obligations, and gate enforcement.

Implementations MAY additionally claim profile overlays published under
`specs/premath/profile/`. Profile overlays are additive to base claims and are
normative only when explicitly claimed.

Details and required vectors are defined in `draft/CONFORMANCE` and `draft/CAPABILITY-VECTORS`.

Interop profiles should be read as evidence/representation profiles over one kernel.
The unifying feature is the kernel law outcome, not the wire representation.
Profile choice may change portability and artifact form, but must not change kernel meaning.

## 4. Reference architecture (optional)

The diagram below is a **reference pathway** for implementations that want deterministic,
portable interop artifacts (`ObjNF/MorNF`, `cmpRef`, `project_ref`, wire formats). It is not
the only valid architecture.

Conformance is judged at the boundaries an implementation exposes (parsers, normalizers,
verifiers, obligation checkers), not by whether it contains these exact boxes internally.

```
   source syntax / IR
 (DSL, KCIR builders, etc.)
            |
            | elaborate / compile (optional)
            v
   obligations Ω  +  candidate meaning
            |              |
            | discharge     | canonicalize / key (optional)
            v              v
      host checker      NF/Normalizer/Refs
            \              /
             \            /
              v          v
            kernel laws (p₀:E→𝒞)
     (stability + contractible descent + refinement invariance)
```

## 5. What is normative (by claim)

### 5.1 Always normative (Kernel claim)

- `draft/DOCTRINE-INF` — doctrine/infinity-layer preservation contract.
- `draft/PREMATH-KERNEL` — semantic kernel (contexts/covers + contractible descent).

### 5.2 Normative for Interop Core (only if claimed)

- `draft/KCIR-CORE`
- `draft/REF-BINDING`
- `draft/NF`
- `draft/WIRE-FORMATS`
- `draft/ERROR-CODES`

### 5.3 Normative for Interop Full (only if claimed)

Everything in Interop Core, plus:

- `draft/NORMALIZER`
- `draft/BIDIR-DESCENT`
- `draft/GATE`

### 5.4 Normative for optional evidence capabilities (only if claimed)

For capability identifiers and vectors defined in `draft/CAPABILITY-VECTORS`:

- `capabilities.normal_forms`
- `capabilities.kcir_witnesses`
- `capabilities.commitment_checkpoints`
- `capabilities.squeak_site`
- `capabilities.ci_witnesses`
- `capabilities.instruction_typing`
- `capabilities.adjoints_sites`
- `capabilities.change_morphisms`

Capability-specific normative specs include:

- `raw/SQUEAK-SITE` (for `capabilities.squeak_site`)
- `raw/PREMATH-CI` (for `capabilities.ci_witnesses`)
- `draft/LLM-INSTRUCTION-DOCTRINE` (for `capabilities.instruction_typing`)
- `draft/LLM-PROPOSAL-CHECKING` (for `capabilities.instruction_typing`)
- `profile/ADJOINTS-AND-SITES` (for `capabilities.adjoints_sites`)
- `draft/CHANGE-MORPHISMS` (for `capabilities.change_morphisms`)

Normative requirements apply only when the corresponding capability is claimed.

### 5.5 Informative and optional

The entries below are informative/default reading surfaces unless they are
explicitly claimed under §5.4 or §5.6.

- `draft/DOCTRINE-SITE` — machine-checkable doctrine-to-operation site map
  (`draft/DOCTRINE-SITE.json`).
- `draft/SPEC-TRACEABILITY` — spec-to-check/vector coverage matrix with
  explicit gap targets.
- `draft/PREMATH-COHERENCE` — typed coherence-contract checker/witness model
  for repository control-plane surfaces (`draft/COHERENCE-CONTRACT.json`).
- `draft/COHERENCE-CONTRACT.json` — machine coherence contract artifact for
  deterministic checker execution.
- `draft/LLM-INSTRUCTION-DOCTRINE` — doctrine contract for typed LLM
  instruction flows (normative only when `capabilities.instruction_typing` is
  claimed).
- `draft/LLM-PROPOSAL-CHECKING` — proposal ingestion/checking contract for LLM
  proposal artifacts (normative only when
  `capabilities.instruction_typing` is claimed).
- `raw/SEMANTICS-INFTOPOS` — presentation-free model sketch (informational).
- `raw/HYPERDESCENT` — optional strengthening: hyperdescent.
- `raw/UNIVERSE` — optional extension: universe + comprehension (Tarski-style).
- `raw/SPLIT-PRESENTATION` — guidance: strict IR vs. semantic equality.
- `raw/TUSK-CORE` — single-world operational runtime contracts (informational/raw).
- `raw/SQUEAK-CORE` — inter-world transport/composition contracts (informational/raw).
- `raw/SQUEAK-SITE` — runtime-location site contracts for Squeak/Cheese
  (normative only when `capabilities.squeak_site` is claimed).
- `raw/PREMATH-CI` — higher-order CI/CD control-loop contract (normative only
  when `capabilities.ci_witnesses` is claimed).
- `raw/CI-TOPOS` — closure-style CI projection discipline (informational/raw).
- `docs/foundations/` — explanatory notes (non-normative).

### 5.6 Normative for profile overlays (only if claimed)

- `profile/ADJOINTS-AND-SITES` — capability-scoped adjoint/site overlay:
  admissible-map allowlist policy, Beck-Chevalley obligations, and deterministic
  `(normalizerId, policyDigest)` discharge binding for profile claims.

## 6. Suggested reading order

If you are proving semantics:
1) `draft/DOCTRINE-INF`
2) `draft/PREMATH-KERNEL`
3) `raw/SEMANTICS-INFTOPOS` (optional)
4) optional extensions (`HYPERDESCENT`, `UNIVERSE`)

If you are implementing Interop Full:
1) `draft/DOCTRINE-INF`
2) `draft/PREMATH-KERNEL`
3) `draft/REF-BINDING` + `draft/KCIR-CORE`
4) `draft/NF` → `draft/NORMALIZER`
5) `draft/BIDIR-DESCENT` + `draft/GATE`
6) `draft/WIRE-FORMATS` + `draft/ERROR-CODES`
7) `draft/CONFORMANCE` + `draft/CAPABILITY-VECTORS`
8) `draft/SPEC-TRACEABILITY`

If you are implementing change discipline:
1) `draft/CHANGE-MORPHISMS`
2) `draft/CAPABILITY-VECTORS` (`capabilities.change_morphisms`)
3) conformance fixtures under `tests/conformance/fixtures/capabilities/`

If you are implementing higher-order CI/CD:
1) `draft/DOCTRINE-INF`
2) `draft/DOCTRINE-SITE` (+ `draft/DOCTRINE-SITE.json`)
3) `draft/LLM-INSTRUCTION-DOCTRINE`
4) `draft/LLM-PROPOSAL-CHECKING`
5) `raw/PREMATH-CI`
6) `raw/CI-TOPOS`
7) `draft/PREMATH-COHERENCE` + `draft/COHERENCE-CONTRACT.json`
8) `raw/TUSK-CORE` + `raw/SQUEAK-CORE`
9) `raw/SQUEAK-SITE`

If you are implementing the adjoints-and-sites overlay:
1) `draft/PREMATH-KERNEL`
2) `draft/GATE`
3) `draft/BIDIR-DESCENT`
4) `profile/ADJOINTS-AND-SITES`

## 7. Notes on restrictiveness

- The kernel is intentionally small and closed.
- Interop is intentionally strict when claimed: it exists to make independent implementations converge.
- Implementations that do not exchange artifacts (e.g., proof-assistant-internal models) MAY omit
  interop machinery, and should simply refrain from making the corresponding interop claims.
