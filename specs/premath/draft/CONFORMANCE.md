---
slug: draft
shortname: CONFORMANCE
title: workingdoge.com/premath/CONFORMANCE
name: Conformance and Claim Registry
status: draft
category: Standards Track
tags:
  - premath
  - kernel
  - conformance
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

Premath is **host-agnostic**. The kernel (`draft/PREMATH-KERNEL`) specifies semantic laws
(reindexing coherence + contractible descent + refinement invariance) but does not mandate
a single implementation architecture.

This document defines **conformance claims** and the **canonical vector suites**
for claims that exchange deterministic artifacts.

Core boundary:

- Section 2.1 defines the explicit Premath Core claim and the narrower
  Premath Kernel claim.
- Sections 2.2 and later define profile/capability claims. Those claims are
  normative only when explicitly asserted and do not expand Premath Core.

Conformance is established by **running code**: passing canonical test vectors, for the
Interop profiles described below.

Spec-level coverage tracking for promoted draft specs is maintained in
`draft/SPEC-TRACEABILITY`.

## 2. Conformance claims

An implementation MAY claim any of the following. It MUST satisfy the requirements of every
claim it asserts.

### 2.0 Claim tokens

Conformance claims have both a human-readable claim string and a canonical
machine token. Any witness, Gate output, or replay artifact that records a
claimed boundary MUST use the canonical claim token.

| Human claim | Canonical claim token |
| --- | --- |
| `Conforms to Premath Core` | `premath.core.v0` |
| `Conforms to Premath Kernel` | `premath.kernel.v0` |
| `Conforms to Premath Interop Core` | `premath.interop-core.v0` |
| `Conforms to Premath Interop Full` | `premath.interop-full.v0` |
| Governance flywheel preservation profile | `profile.doctrine_inf_governance.v0` |

### 2.1 Core and Kernel claims

- `Conforms to Premath Core`

This claim means the implementation exposes the minimal admissibility spine:
kernel laws, obligation/discharge, Gate verdicts, deterministic witnesses, and
replayable boundary behavior.

A `Premath Core` conforming implementation MUST:

1. satisfy the semantic laws in `draft/PREMATH-KERNEL` for its declared
   host/model,
2. expose deterministic `accepted`/`rejected` Gate outcomes per `draft/GATE`,
3. emit deterministic witness IDs per `draft/WITNESS-ID` for rejected checks,
4. preserve kernel verdict and Gate failure class under replay at exposed
   boundaries, and
5. implement the Core obligation/discharge interface in
   `draft/OBLIGATION-DISCHARGE`, or deterministically reject unsupported
   obligation forms.

A `Premath Core` implementation MUST support the base obligation classes for
any boundary where it accepts admissibility judgments. It MAY deterministically
reject unsupported optional obligation forms or unsupported host/model
surfaces, but it MUST NOT claim Core for a boundary whose accepted judgments
bypass stability, locality, descent existence, or contractible-gluing checks.

Core conformance does not imply any optional interop, control-plane, raw, or
identity profile claim.

- `Conforms to Premath Kernel`

This narrower semantic claim means the implementation's chosen model/host
satisfies the semantic laws in `draft/PREMATH-KERNEL`. It is not a substitute
for `Conforms to Premath Core`, because it does not by itself claim Gate,
obligation/discharge, or witness/replay behavior.

This bundle does not standardize a universal host-independent proof artifact for the kernel
claim alone. Implementations SHOULD substantiate kernel conformance by one of:

- a proof-assistant formalization of the kernel laws for the chosen host, or
- a published, reproducible test suite specific to the chosen `(𝒞, J, 𝒱, Def)` model.

This repository additionally publishes a canonical cross-model kernel profile vector
suite (`tests/conformance/fixtures/kernel-profile/`) for deterministic scenario
comparison across the semantic toy and KCIR toy evidence surfaces.

(Interop claims below *do* have canonical vectors.)

### 2.2 Interop profiles (deterministic artifact exchange)

Interop profiles are strict by design: they exist to make independent implementations converge.

An implementation MAY claim:

- `Conforms to Premath Interop Core`
- `Conforms to Premath Interop Full`

The unqualified claim `Conforms to Premath Interop` MUST be interpreted as
`Conforms to Premath Interop Full`.

### 2.3 Optional capability claims

Implementations MAY additionally claim optional capabilities.

Capabilities MUST be explicit: if a capability is not claimed, any vectors that exercise that
capability’s optional branches MUST be rejected deterministically or treated as out-of-scope
for conformance (as specified by the capability).

The capability registry and vector guidance are defined in:

- `draft/CAPABILITY-VECTORS`

### 2.4 Optional profile-overlay claims

Implementations MAY additionally claim doctrine profile overlays when supported.

Profile overlays are distinct from executable capability claims:

- capability claims express executable feature surfaces,
- profile-overlay claims express additional normative doctrine constraints that
  apply when explicitly asserted.

For governance doctrine profile conformance, implementations MAY claim:

- `profile.doctrine_inf_governance.v0`

Active repository-level profile-overlay claims MUST be declared in
`draft/CAPABILITY-REGISTRY.json` under `profileOverlayClaims`.

When this profile claim is asserted, §9 requirements in `draft/DOCTRINE-INF`
are normative for the claiming surface and MUST be validated through doctrine
conformance vectors.

## 3. Required behavior (profiles and capabilities)

### 3.1 Interop Core

A `Premath Interop Core` conforming verifier MUST:

1. Parse KCIR nodes and verify all referenced payloads (`draft/KCIR-CORE`, `draft/REF-BINDING`).
2. Enforce DAG invariants (`envSig, uid`) and acyclicity.
3. Parse NF bytes (`draft/NF`) and enforce opcode/constructor contracts (implementation-defined if
   `raw/OPCODES` is not adopted in the bundle).
4. Produce deterministic accept/reject results and stable error codes (`draft/ERROR-CODES`).
5. When emitting or consuming exchange artifacts, obey the registries in `draft/WIRE-FORMATS`.

### 3.2 Interop Full

A `Premath Interop Full` conforming verifier MUST satisfy all `Interop Core` requirements and MUST also:

6. Implement `draft/NORMALIZER` for `normalized` comparisons and stable comparison keys.
7. Implement `profile/interop/BIDIR-DESCENT` mode discipline, obligation
   emission, and discharge.
8. Enforce admissibility gate laws (`draft/GATE`) and emit Gate witness classes deterministically.

### 3.3 Semantic invariance across evidence profiles

If an implementation supports multiple evidence/representation profiles (for example
opaque witnesses, KCIR-linked witnesses, or commitment checkpoints), then for fixed
semantic inputs and fixed policy/normalizer bindings it:

- MUST preserve the same kernel accept/reject verdict, and
- MUST preserve the same Gate failure classes (when rejected).

Profile choice MAY change artifact shape, transport fields, and auxiliary evidence payloads.

### 3.4 Required behavior for change-morphism capability

If capability `capabilities.change_morphisms` is claimed, implementation MUST:

9. compute deterministic change projections from declared delta material to
   required gate checks with stable projection digest identity,
10. preserve projection/reference equivalence across provider wrapper mappings
    (local and mapped external env forms), and
11. enforce deterministic issue mutation transitions for claim/discover flows,
    including claim-lease lifecycle (`lease_id`, owner, expiry, renew/release)
    and stale/contended lease projection classification,
12. enforce paired invariance requirements (including kernel verdict/Gate class
    invariance claims) across local/external projection profiles.

Claim-scope note:

- `draft/HARNESS-TYPESTATE` closure/mutation-gate requirements are currently
  bundled under `capabilities.change_morphisms` (no separate typestate
  capability claim in `draft/CAPABILITY-REGISTRY.json` at this time).

Executable vectors for this claim MUST include same-owner active claim
preservation checks where existing lease ids are retained across deterministic
renew/claim transitions, plus adversarial override attempts and paired
local/external invariance rows.

### 3.5 Required behavior for SqueakSite capability

If capability `capabilities.squeak_site` is claimed, implementation MUST:

13. compute deterministic location descriptor identity material (`loc_id` or equivalent),
14. reject overlap disagreement deterministically for mismatched required checks or policy/projection bindings, and
15. preserve kernel verdict and Gate failure classes across paired runtime profiles in invariance vectors.

### 3.6 Required behavior for CI witness capability

If capability `capabilities.ci_witnesses` is claimed, implementation MUST:

16. bind each CI witness deterministically to instruction identity material
    (instruction digest/ref),
17. reject deterministic witness checks when the same instruction yields
    mismatched verdict class or required/executed check sets, and
18. verify required-gate witness payloads deterministically against projection
    bindings (including gate witness refs and native required-check bindings),
19. verify strict-delta and decision-attestation witness chains deterministically
    when those checks are requested, and
20. preserve kernel verdict and Gate failure classes across paired local/external
    CI witness-profile invariance vectors, including boundary-authority lineage
    parity (kernel obligation registry -> proposal discharge -> coherence scope ->
    CI semantic failure classes) and stale generated doctrine-site rejection.

Executable vectors for this claim MUST include harness v1 boot/stop/recovery
rows that verify deterministic bootstrap mode (`attach|resume`), lease-state
recovery action mapping, and stop-row lease-handoff witness linkage, with
adversarial mismatch coverage and paired local/external invariance rows.

### 3.7 Required behavior for instruction typing capability

If capability `capabilities.instruction_typing` is claimed, implementation MUST:

21. classify instruction handling explicitly as `typed(kind)` or
    `unknown(reason)`,
22. reject `unknown(reason)` deterministically when no explicit policy route is
    permitted, and
23. preserve kernel verdict and Gate failure classes across paired local/external
    instruction-typing profile invariance vectors,
24. ingest typed LLM proposal payloads as checking-only inputs (never authored
    synthesis inputs),
25. reject proposal payloads that are unbound to
    `(normalizerId, policyDigest)` deterministically, and
26. reject non-canonical or nondeterministic proposal-digest material
    deterministically.

### 3.8 Required behavior for adjoints/sites capability overlay

If capability `capabilities.adjoints_sites` is claimed, implementation MUST:

27. compile claimed refinement-plan semantic material into deterministic
    obligations including `adjoint_triangle`, `beck_chevalley_sigma`,
    `beck_chevalley_pi`, and `refinement_invariance`,
28. bind obligation discharge deterministically to
    `(normalizerId, policyDigest)`,
29. reject deterministically when required adjoint/site obligation evidence is
    missing, and
30. preserve kernel verdict and Gate failure classes across paired local/external
    adjoints-sites profile invariance vectors.

## 4. Vectors (informative guidance)

A repository SHOULD organize vectors as:

- `tests/conformance/fixtures/interop-core/{golden,adversarial}/`
- `tests/conformance/fixtures/gate/{golden,adversarial}/`
- `tests/conformance/fixtures/kernel-profile/{golden,adversarial}/`
- `tests/conformance/fixtures/capabilities/<capability-id>/{golden,adversarial,invariance}/`

This repository's merge-gated conformance surface executes:

- `python3 tools/conformance/run_fixture_suites.py`
- `premath coherence-check --contract specs/premath/draft/COHERENCE-CONTRACT.json --repo-root .`

The fixture-suite runner executes the executable suites:

- `python3 tools/conformance/run_interop_core_vectors.py`
- `python3 tools/conformance/run_gate_vectors.py`
- `python3 tools/conformance/run_witness_id_vectors.py`
- `python3 tools/conformance/run_kernel_profile_vectors.py`
- `python3 tools/conformance/run_capability_vectors.py`

Golden vectors MUST verify successfully.
Adversarial vectors MUST reject deterministically with stable witness classes/codes.

Repositories that publish doctrine preservation declarations SHOULD also publish
and validate a doctrine-to-operation site map (for example:
`draft/DOCTRINE-SITE` + `draft/DOCTRINE-SITE.json`) so operational gate
entrypoints remain auditable from doctrine root through runtime/CI layers.

## 5. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.profile.evidence` (profile invariance requirements in §3.3)
- `dm.presentation.projection` (claim/profile conformance is architecture-agnostic)
- `dm.commitment.attest` (when commitment/CI capabilities are claimed)

Not preserved:

- `dm.transport.world` / `dm.transport.location` (delegated to Squeak specs)
- `dm.refine.context` / `dm.refine.cover` (delegated to kernel/gate/runtime specs)
