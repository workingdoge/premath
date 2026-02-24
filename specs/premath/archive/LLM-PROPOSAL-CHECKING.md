---
slug: draft
shortname: LLM-PROPOSAL-CHECKING
title: workingdoge.com/premath/LLM-PROPOSAL-CHECKING
name: LLM Proposal Ingestion and Checking Contract
status: draft
category: Standards Track
tags:
  - premath
  - llm
  - proposal
  - checking
  - doctrine
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

## 1. Scope

This specification defines how LLM-produced proposal artifacts are ingested by
Premath checking/discharge pipelines.

It complements:

- `draft/LLM-INSTRUCTION-DOCTRINE` (authority split + typed/unknown handling),
- `draft/BIDIR-DESCENT` (obligation/discharge contract),
- `draft/GATE` (admissibility witness outcomes).

Normative intent:

- LLM output is proposal input, not semantic authority,
- proposals enter checking mode only,
- acceptance remains discharge-determined and witness-bound.

Repository profile note (v0): when attached to instruction envelopes, proposal
payloads are carried in `proposal` (legacy alias `llmProposal` MAY be accepted
for compatibility). In this profile, proposal `binding.normalizerId` and
`binding.policyDigest` MUST match top-level instruction envelope bindings.

## 2. Minimum proposal schema

Proposal encoding MUST stay minimal while retaining replay/checking power.

Canonical shape:

```text
LLMProposal {
  proposalKind        // value | derivation | refinementPlan
  targetCtxRef
  targetJudgment {
    kind              // obj | mor
    shape             // implementation-level expected shape/type descriptor
  }
  candidateRefs?[]
  steps?[]            // required for derivation
  binding {
    normalizerId
    policyDigest
  }
  proposalDigest?     // optional declared digest; MUST match canonical payload when present
  proposalKcirRef?    // optional declared KCIR ref; MUST match canonical proposal projection when present
}
```

Where derivation step entries are:

```text
Step {
  ruleId
  inputs[]
  outputs[]
  claim
}
```

`proposalKind` discipline:

- `derivation` MUST provide non-empty `steps`,
- `value` and `refinementPlan` MUST NOT provide `steps`.

## 3. Classification and fail-closed handling

Proposal handling MUST be classified explicitly as one of:

- `typed(kind)` where `kind` is a recognized proposal-handling route,
- `unknown(reason)` when classification is not established.

`unknown(reason)` MUST be first-class and MUST NOT be silently coerced into
execution.

Policy MAY permit explicit unknown routes (for example clarify-only), but this
MUST be explicit and auditable.

## 4. Checker contract

For typed proposal routes, the checker MUST:

1. treat proposal payloads as untrusted input,
2. compile proposal claims into obligations under `draft/BIDIR-DESCENT`,
3. discharge obligations deterministically,
4. map rejection outcomes into Gate witness classes,
5. preserve provenance (`source = llm_proposed`) without granting authority.

LLM proposals MUST NOT directly populate authored synthesis subset `S`.

Implementations SHOULD emit proposal-ingest witness material containing at
least deterministic `obligations[]` and deterministic `discharge` records so
accept/reject outcomes are auditable without replaying runtime logs.

When KCIR-linked evidence surfaces are exposed, implementations SHOULD emit a
deterministic proposal KCIR ref (`kcir1_*`) derived from canonical proposal
payload so instruction/proposal/coherence witnesses share one reference
boundary.

## 5. Determinism binding

Proposal checking in `normalized` mode MUST bind to:

- `normalizerId`,
- `policyDigest`.

Missing binding material MUST reject deterministically.

For fixed semantic payload and fixed binding material:

- canonical proposal digest MUST be deterministic,
- discharge verdict class and failure classes MUST be deterministic.

If `proposalDigest` is present, it MUST equal canonical digest output for the
same payload.

If `proposalKcirRef` is present, it MUST equal the canonical KCIR ref derived
from canonical proposal payload.

### 5.1 Canonical KCIR projection payload

The canonical proposal KCIR projection payload is:

```text
KCIRProposalProjection {
  kind: "kcir.proposal.v1",
  canonicalProposal: <canonical LLMProposal payload from Section 2>
}
```

`proposalKcirRef` MUST be derived as:

```text
"kcir1_" + SHA256(JCS(KCIRProposalProjection))
```

### 5.2 Witness lineage requirements

When instruction/proposal checking emits proposal-ingest witness material, that
material SHOULD include `proposalKcirRef` so instruction, proposal, and
coherence/parity surfaces can reference one deterministic proposal identity.

Migration guidance (minimum encoding):

- new witness/API surfaces SHOULD treat `proposalKcirRef` as the canonical
  external proposal key,
- `proposalDigest` MAY be retained as a compatibility digest during transition,
- when both are present they MUST bind to the same canonical proposal payload.

## 6. Deterministic failure classes

Proposal ingestion MUST produce deterministic machine-readable classes for
checker-level failures before Gate discharge. Minimum set:

- `proposal_invalid_kind`,
- `proposal_invalid_target`,
- `proposal_invalid_target_judgment`,
- `proposal_invalid_step`,
- `proposal_unbound_policy`,
- `proposal_binding_mismatch`,
- `proposal_nondeterministic`.
- `proposal_kcir_ref_mismatch`.

These classes do not replace Gate failure classes; they gate entry into
obligation discharge.

## 7. Security and robustness

Implementations MUST treat proposal payloads as untrusted.

Implementations SHOULD:

- cap step count and replay depth,
- fail closed on malformed or partially bound proposals,
- keep proposal-to-witness lineage records for audit.

## 8. Doctrine Preservation Declaration (v0)

Reference: `draft/DOCTRINE-INF`.

Preserved morphisms:

- `dm.identity`
- `dm.policy.rebind`
- `dm.presentation.projection`
- `dm.commitment.attest`

Not preserved:

- `dm.profile.execution`
- `dm.profile.evidence`
- `dm.refine.context`
- `dm.refine.cover`
- `dm.transport.location`
- `dm.transport.world`
