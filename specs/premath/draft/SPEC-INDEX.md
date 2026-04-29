---
slug: draft
shortname: SPEC-INDEX
title: workingdoge.com/premath/SPEC-INDEX
name: Premath Spec Index
status: draft
category: Best Current Practice
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

## 0. Scope Sentence

Premath is not the stack. Premath is the admissibility law that stacks compile
to.

Premath owns a small decision spine:

```text
context/site -> definable/claim -> obligation -> gate -> witness -> receipt/replay
```

Everything else is a profile, adjacent site, or implementation of that spine.

## 1. Purpose

This file is the front door for Premath scope. It answers:

- what belongs to Premath Core,
- what is optional profile material,
- what is raw or non-normative,
- and what belongs to adjacent sites rather than Premath.

Mutable implementation state is not authority here. Current project status lives
in issue memory and non-normative process/design notes.

## 2. Premath Core

Premath Core is the admissibility kernel and replayable witness interface:

- `draft/PREMATH-KERNEL` — contexts, covers, indexed definables, reindexing
  stability, locality, descent data, contractible gluing, refinement closure,
  and rejection criteria.
- `draft/OBLIGATION-DISCHARGE` — Core obligation records, deterministic
  discharge, and Gate failure-class mapping for descent-shaped evidence.
- `draft/GATE` — accepted/rejected gate outcomes and failure classes.
- `draft/WITNESS-ID` — deterministic witness identity.
- `draft/CONFORMANCE` §§1-2.1 — Core claim boundary and conformance
  requirements. Later profile/capability sections do not expand Core.

`draft/DOCTRINE-INF` is a doctrine-preservation guard for implementations that
carry Premath through infinity-layer or doctrine-preserving contexts. It is part
of the promoted contract surface, but it is not a control plane.

## 3. Optional Profiles

Optional profiles add representation, runtime, or application constraints. They
must not create a second admissibility authority.

- Interop profile: `profile/interop/README` maps `KCIR-CORE`, `NF`,
  `NORMALIZER`, `REF-BINDING`, `WIRE-FORMATS`, and `ERROR-CODES`.
- Control-plane profile: `PREMATH-COHERENCE`, `COHERENCE-CONTRACT.json`,
  `CONTROL-PLANE-CONTRACT.json`, `DOCTRINE-SITE*`, `LLM-*`, `HARNESS-*`,
  `CHANGE-MORPHISMS`, `SPAN-SQUARE-CHECKING`, and the control-plane portions of
  `UNIFICATION-DOCTRINE`; see `profile/control-plane/README`.
- Adjoints/sites profile: `profile/ADJOINTS-AND-SITES`.
- Identity/auth material: currently raw; see `profile/identity/README`.

Profiles may be strict. Their strictness is profile-local and claim-scoped.

## 4. Adjacent Sites

Premath does not own adjacent site semantics.

- Atlas owns site-of-sites placement and cross-site cover/factorization rules.
- Nerve owns its protocol/substrate semantics until a reusable substrate factor
  is explicitly extracted.
- Tusk owns operator instruments, projections, runtime bindings, and tracker
  workflow surfaces over accepted decisions.
- Work-state meaning is a candidate `work` site role. Premath may check
  work-claim admissibility, but Premath does not own generic work semantics.

## 5. Normative Map

### 5.1 Core authority

The smallest Premath authority path is:

1. `draft/PREMATH-KERNEL`
2. `draft/OBLIGATION-DISCHARGE`
3. `draft/GATE`
4. `draft/WITNESS-ID`
5. `draft/CONFORMANCE` §§1-2.1

An implementation claiming Premath Core must preserve the same accepted/rejected
kernel outcome and deterministic witness/replay behavior at the boundaries it
exposes.

### 5.2 Interop profile

The Interop profile is optional and representation-facing.

Interop Core, when claimed, uses:

- `draft/KCIR-CORE`
- `draft/REF-BINDING`
- `draft/NF`
- `draft/WIRE-FORMATS`
- `draft/ERROR-CODES`

Interop Full, when claimed, adds:

- `draft/NORMALIZER`
- `profile/interop/BIDIR-DESCENT`

It uses the Core `draft/OBLIGATION-DISCHARGE` and `draft/GATE` interfaces to
enforce obligations and admissibility over deterministic artifacts.

Interop may change artifact form. It must not change kernel meaning.

### 5.3 Control-plane profile

The control-plane profile is optional and implementation-facing.

It may define deterministic wrappers, coherence checks, schema lifecycle tables,
runtime-route parity, instruction/proposal typing, and harness behavior. Those
surfaces remain projections or governance over the Premath spine. They do not
own semantic admissibility.

The long-form control-plane doctrine currently remains in promoted draft files
while it is being factored. The target shape is a control-plane profile, not a
larger Premath Core.

### 5.4 Capability-scoped documents

Executable capability identifiers:

- `capabilities.normal_forms`
- `capabilities.kcir_witnesses`
- `capabilities.commitment_checkpoints`
- `capabilities.squeak_site`
- `capabilities.ci_witnesses`
- `capabilities.instruction_typing`
- `capabilities.adjoints_sites`
- `capabilities.change_morphisms`

Capability-specific document bindings:

- `raw/SQUEAK-SITE` (for `capabilities.squeak_site`)
- `raw/PREMATH-CI` (for `capabilities.ci_witnesses`)
- `draft/LLM-INSTRUCTION-DOCTRINE` (for `capabilities.instruction_typing`)
- `draft/LLM-PROPOSAL-CHECKING` (for `capabilities.instruction_typing`)
- `profile/ADJOINTS-AND-SITES` (for `capabilities.adjoints_sites`)
- `draft/CHANGE-MORPHISMS` (for `capabilities.change_morphisms`)
- `draft/HARNESS-TYPESTATE` (for `capabilities.change_morphisms`)

Capability requirements apply only when the corresponding capability is claimed.

### 5.5 Raw, informative, and default status

The entries below are informative/default status unless they are
explicitly claimed under §5.4 or §5.6.

Conditional clauses:

- `raw/SQUEAK-SITE` is normative only when `capabilities.squeak_site` is
  claimed.
- `raw/PREMATH-CI` is normative only when `capabilities.ci_witnesses` is
  claimed.
- `draft/LLM-INSTRUCTION-DOCTRINE` is normative only when
  `capabilities.instruction_typing` is claimed.
- `draft/LLM-PROPOSAL-CHECKING` is normative only when
  `capabilities.instruction_typing` is claimed.
- `profile/ADJOINTS-AND-SITES` is normative only when
  `capabilities.adjoints_sites` is claimed.
- `draft/CHANGE-MORPHISMS` is normative only when
  `capabilities.change_morphisms` is claimed.
- `draft/HARNESS-TYPESTATE` is normative when
  `capabilities.change_morphisms` is claimed.

Raw capability-spec lifecycle policy:

- Raw capability specs may have executable vectors without becoming Premath
  Core.
- Promotion from raw to draft for capability-scoped specs requires:
  1. deterministic golden/adversarial/invariance vectors for every claimed law
     boundary;
  2. deterministic witness/failure-class mapping through checker/run surfaces;
  3. issue-backed migration plan and decision-log entry for lifecycle change.

Current raw-retain posture:

- `raw/SQUEAK-SITE` — retained raw per Decision 0040.
- `raw/TUSK-CORE` — retained raw per Decision 0041.

### 5.6 Profile overlays

Profile overlays are additive and claim-scoped.

- `profile/ADJOINTS-AND-SITES` defines the adjoints/sites overlay.
- `profile.doctrine_inf_governance.v0` is an optional doctrine-governance
  overlay claim defined by `draft/DOCTRINE-INF` and `draft/CONFORMANCE`.

Lane ownership:

- Unified evidence factoring MUST route control-plane artifact families through
  one attested surface.
- CwF<->sig\Pi bridge mapping is normative in
  `profile/ADJOINTS-AND-SITES` §11 and must preserve existing obligation
  vocabularies.

## 6. Reading Order

For Premath Core:

1. `draft/PREMATH-KERNEL`
2. `draft/OBLIGATION-DISCHARGE`
3. `draft/GATE`
4. `draft/WITNESS-ID`
5. `draft/CONFORMANCE` §§1-2.1

For Interop:

1. Core reading order
2. `draft/REF-BINDING` + `draft/KCIR-CORE`
3. `draft/NF` + `draft/NORMALIZER`
4. `draft/WIRE-FORMATS` + `draft/ERROR-CODES`

For control-plane work:

1. Core reading order
2. The relevant control-plane profile files
3. `draft/UNIFICATION-DOCTRINE` only for canonical-boundary and projection
   discipline

For adjacent sites:

1. Read the adjacent site's own specs first.
2. Use Premath only where that site explicitly compiles claims to Premath
   admissibility, gates, witnesses, or replayable receipts.

## 7. Non-Goals

Premath Core does not own:

- MCP routes or issue mutation flows;
- CI wrappers or provider workflow details;
- harness session lifecycle;
- LLM proposal policy;
- JWT/JWKS runtime search behavior;
- Nerve protocol semantics;
- Atlas site-of-sites machinery;
- Tusk operator instruments or projections.

Those surfaces may depend on Premath, but they are not Premath Core.
