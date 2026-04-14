---
slug: raw
shortname: IDENTITY-ADMISSION
title: workingdoge.com/premath/IDENTITY-ADMISSION
name: Local Identity Admission Kernel
status: raw
category: Standards Track
tags:
  - premath
  - identity
  - admission
  - witnesses
  - obstructions
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

## 1. Purpose

This specification defines a reusable local identity-admission kernel for
Premath-shaped systems.

Its job is narrower than a full provider-neutral identity architecture.
It names the minimum reusable objects and laws needed when a system:

- receives principal presentations inside a local validity scope,
- checks assertions against freshness, revocation, and policy conditions,
- admits or rejects those assertions with witnesses,
- and glues compatible local admissions only under explicit overlap evidence.

This document is the kernel-side extraction of those ideas.
It is intended to sit between:

- `draft/PREMATH-KERNEL`, which owns general locality, reindexing, and descent
  law,
- `draft/GATE`, which owns deterministic acceptance and rejection witnessing,
  and
- domain profiles, which map the reusable kernel onto concrete objects such as
  request schemas, validator states, or secret-bound handoff surfaces.

## 2. Boundary

This specification defines:

- local identity-admission vocabulary over Premath contexts,
- the minimum restriction and gluing laws for identity-bearing presentations,
- admission and obstruction boundaries, and
- the witness discipline needed to keep acceptance and rejection explainable.

This specification does **not** define:

- one wire format,
- one cryptographic suite,
- one provider protocol such as OIDC, SAML, SCIM, mTLS, SSH certificates, or
  service tokens,
- one entitlement catalog,
- one domain-specific claim schema,
- one runtime validator lifecycle.

Those belong to profile or product-specific documents.

## 3. Relationship to Existing Premath Surfaces

This document specializes Premath doctrine; it does not replace it.

The ownership split is:

- `draft/PREMATH-KERNEL` owns context, coverage, reindexing, and descent law.
- `draft/GATE` owns the acceptance or rejection envelope and failure-class
  witnessing discipline.
- `raw/IDENTITY-ADMISSION` owns the reusable vocabulary for principal
  presentation, assertion, admission, entitlement narrowing, and typed
  obstruction.

So this document MUST NOT be read as a second kernel or a second Gate.
It is a reusable admission layer over the existing kernel.

## 4. Core Objects

### 4.1 Validity Scope

A local identity-admission scope is a Premath context `Gamma`.

`Gamma` MAY include:

- tenant or organization context,
- application or service context,
- machine or workload context,
- network or posture context,
- freshness or time-window context,
- audience or relying-party context.

This specification does not fix one canonical context decomposition.
It only requires that the chosen scope participate in the Premath context and
cover structure.

### 4.2 Principal Presentation

A principal presentation is the local appearance of a principal inside one
validity scope.

A presentation MUST provide at least:

- `issuer`,
- `subject`,
- `credential_class`,
- `scope`,
- `validity`, and
- `claims`.

`claims` MAY be issuer-native or normalized attributes.
A presentation is not yet operational authority.

### 4.3 Assertion

An assertion is a presentation together with the evidence needed for admission.

An assertion MUST provide at least:

- the presentation,
- supporting evidence,
- issuance metadata,
- audience or relying-party constraints when applicable,
- freshness or expiry data,
- revocation handles or status references when applicable.

### 4.4 Admitted Principal

An admitted principal is the local result of successful admission in one scope.

An admitted principal MUST carry:

- the admitting scope,
- a normalized local principal identity,
- the supporting presentation or compatible glued family of presentations,
- the admission witness,
- any residual obligations.

Residual obligations MAY include:

- step-up requirements,
- narrowed validity windows,
- local-only trust,
- required posture recheck,
- human approval gates.

### 4.5 Obstruction

An obstruction is a typed reason the presented material could not be admitted or
could not be glued.

At minimum, conforming systems SHOULD be able to distinguish:

- `unknown_issuer`,
- `freshness_failure`,
- `revocation_failure`,
- `audience_mismatch`,
- `scope_escape`,
- `normalization_failure`,
- `overlap_conflict`,
- `forbidden_join`,
- `policy_denial`.

Domain profiles MAY refine these classes further.

## 5. Restriction and Locality

Identity-admission data lives over scopes and therefore MUST restrict.

For every morphism `f : Delta -> Gamma`:

- a presentation in `Gamma` MUST restrict to a presentation in `Delta` when the
  narrower scope still supports that presentation,
- an assertion in `Gamma` MUST restrict to an assertion in `Delta` together
  with the evidence that remains valid under the narrower scope,
- an admitted principal in `Gamma` MUST restrict to an admitted principal in
  `Delta` or fail with an explicit obstruction.

If a required restriction does not exist, admission cannot be treated as
Premath-admissible in that scope.

## 6. Overlap Compatibility and Gluing

Presentations and admissions MAY glue only from compatible local data.

For a cover `U = {u_i : Gamma_i -> Gamma}`, compatibility on overlaps MUST
consider at least:

- issuer compatibility,
- alias or subject-linking witnesses when identifiers differ,
- claim consistency,
- validity-window compatibility,
- revocation coherence.

If compatible local presentations or admitted principals are supplied, a
conforming system MAY glue them to a global presentation or admitted principal.
If gluing fails, the system MUST return an explicit obstruction or witness of
failure rather than silently choosing one representative.

This document does not strengthen the Premath gluing law.
It applies that law to identity-bearing data.

## 7. Admission Judgment

The admission boundary is a local judgment of the shape:

`Gamma |- alpha admits p`

or an equivalent machine interface.

An implementation MAY realize this as a total function, a partial function, a
typed judgment, or a result type, provided the same boundary is preserved:

- success yields an admitted principal plus witness,
- failure yields a typed obstruction plus failure witness.

### 7.1 Mandatory Checks

A conforming admission operator MUST check at least:

- issuer admissibility,
- audience binding when applicable,
- freshness and expiry,
- revocation state,
- scope validity,
- claim normalization,
- policy predicates,
- entitlement narrowing.

Missing mandatory checks MUST be treated as rejection or explicit inability to
admit, not as silent acceptance.

### 7.2 No Silent Widening

Admission MAY narrow authority.
Admission MUST NOT silently widen authority.

In particular:

- combining multiple partial assertions MUST NOT increase authority unless an
  explicit policy rule authorizes the join,
- failed joins MUST become obstruction, not fallback acceptance,
- restriction into a narrower scope MUST NOT invent authority absent from the
  larger scope.

### 7.3 Residual Obligations

Admission MAY succeed with residual obligations, but those obligations MUST be:

- explicit,
- machine-checkable,
- carried by the admission witness.

## 8. Entitlement Boundary

This specification does not define one global entitlement algebra.
It only fixes the minimum reusable boundary.

For each scope `Gamma` and admitted principal `p`, a conforming system SHOULD
provide a local ordered entitlement space `Ent(Gamma, p)`.

If joins are partial or policy-dependent, undefined joins MUST be treated as
obstruction or policy failure, not as implicit escalation.

Restriction of entitlements along `f : Delta -> Gamma` MUST be monotone and MUST
preserve the no-silent-widening rule.

## 9. Witness Discipline

Acceptance and rejection MUST be witness-bearing.

An acceptance witness SHOULD include:

- the admitting scope,
- the supporting assertion or assertion family,
- normalization or alias-linking evidence when used,
- policy or rule references when used,
- the resulting admitted principal identity,
- the resulting entitlement bounds,
- residual obligations.

A rejection witness SHOULD include:

- obstruction class,
- affected scope,
- enough structured detail to support retry, escalation, or audit.

If witness payloads are externalized into deterministic artifacts, they SHOULD
reuse the Gate witness discipline and deterministic witness identifiers rather
than creating a parallel witness regime.

## 10. Domain Profile Boundary

Domain profiles instantiate this kernel.

A domain profile MAY decide, for example:

- what counts as a validity scope,
- which object class carries a presentation,
- how admission is represented operationally,
- which obstruction subclasses are exposed publicly,
- how entitlement bounds are encoded.

But domain profiles MUST NOT claim to change the kernel laws merely by renaming
the objects.

This is the intended relationship to systems such as bridge, Nerve, and later
consumer runtimes:

- Premath owns the reusable admission vocabulary and locality law,
- domain profiles own their concrete schema, validator, and lifecycle surfaces.

## 11. Conformance Summary

A system conforms to this raw kernel slice when it:

1. interprets identity-admission data over Premath contexts,
2. restricts presentations, assertions, and admissions along narrower scopes,
3. performs admission only through explicit checks and witnesses,
4. refuses silent authority widening,
5. glues only from compatible local data,
6. emits typed obstruction information on failure.

That is the minimum reusable identity-admission layer this document extracts.
