---
slug: draft
shortname: DOCTRINE-INF
title: workingdoge.com/premath/DOCTRINE-INF
name: Meta Doctrine and Infinity-Layer Preservation
status: draft
category: Standards Track
tags:
  - premath
  - doctrine
  - meta-institution
  - infinity
  - preservation
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

This specification defines Premath's meta layer (`infty` layer): a doctrine of
morphisms over institutions, obligations, runtime transport, and projections.

Purpose:

- keep architecture changes composable,
- require explicit preservation claims under change,
- prevent semantic drift when implementations evolve.

`draft/DOCTRINE-INF` does not replace kernel laws. It constrains how lower
layers claim they preserve meaning.

## 2. Layer stack

Conforming stacks SHOULD be read as:

1. Doctrine layer (`draft/DOCTRINE-INF`)
2. Institution/kernel layer (`draft/PREMATH-KERNEL`, `draft/GATE`)
3. Obligation/runtime layer (`raw/TUSK-CORE`, `raw/SQUEAK-CORE`, `raw/SQUEAK-SITE`)
4. CI/projection layer (`raw/PREMATH-CI`, `raw/CI-TOPOS`)

Lower layers MUST publish which doctrine morphism classes they preserve.

## 3. Doctrine morphism registry (v0)

v0 defines the following morphism classes.

- `dm.identity`: identity/pure relabeling morphism preserving semantic object
  and verdict class.
- `dm.refine.context`: context/lineage refinement morphism preserving meaning
  under reindex/refinement.
- `dm.refine.cover`: cover refinement morphism preserving admissibility class.
- `dm.transport.world`: inter-world transport morphism preserving declared
  payload meaning and non-bypass constraints.
- `dm.transport.location`: runtime-location/site transport morphism preserving
  run-frame semantics and classification boundaries.
- `dm.profile.execution`: execution-substrate/profile morphism preserving Gate
  class outcomes for fixed semantic inputs.
- `dm.profile.evidence`: evidence-representation/profile morphism preserving
  kernel verdict and Gate failure classes.
- `dm.policy.rebind`: explicit policy/normalizer rebinding morphism requiring a
  new run boundary and explicit witness attribution.
- `dm.presentation.projection`: projection/view/UI morphism that MAY change
  representation but MUST NOT change semantic authority.
- `dm.commitment.attest`: commitment/witness attestation morphism binding
  obligations or instruction envelopes to deterministic evidence objects.

## 4. Preservation claim format

Lower specs MUST include a declaration section with this shape:

```text
Doctrine Preservation Declaration (v0)
Preserved morphisms:
- dm....
- dm....
Not preserved:
- dm.... (reason)
```

A spec MUST NOT claim preservation for a morphism class unless this spec's
required invariants make that preservation checkable.

## 5. Satisfaction preservation rule

Let `Sat_W(phi)` mean statement `phi` is satisfied in world/profile `W`.

A declared doctrine morphism `m : W1 -> W2` preserves `phi` only if:

```text
Sat_W1(phi) => Sat_W2(m(phi))
```

where `m(phi)` is the mapped statement under the declared morphism class.

For profile morphisms, preserving verdict class and Gate failure class set is a
sufficient v0 criterion.

## 6. Unknown and partial classification

Unknown classification is first-class.

`unknown(reason)` MUST be modeled as an explicit state, not implicit failure.

Morphisms from `unknown(reason)` MAY be restricted by policy (for example:
clarify-only, plan-only, escalation-required), but this restriction MUST be
explicitly declared in runtime/profile policy material.

## 7. Conformance expectations

Conforming implementations SHOULD:

- publish doctrine-preservation declarations across lower specs,
- include doctrine morphism attribution in witness diagnostics where relevant,
- reject or downgrade claims when preservation cannot be established.

## 8. Security and robustness

Implementations MUST treat doctrine declarations as auditable contract material.

Implementations SHOULD:

- keep doctrine morphism IDs stable,
- version declaration changes with explicit decision-log entries,
- fail closed when a required preservation claim is missing.

## 9. Governance flywheel preservation profile (v0)

This section defines doctrine-level constraints for operational governance loops
that change policy/prompt/model behavior over time.

Purpose:

- keep policy-as-code governance composable with semantic authority boundaries,
- make promotion/retraining decisions auditable and reversible,
- prevent ungoverned self-optimization paths from bypassing checker authority.

Applicability (conditional normative):

- this profile is normative only for surfaces that explicitly claim
  `profile.doctrine_inf_governance.v0`,
- surfaces that do not claim this profile MAY omit §9 obligations and MUST NOT
  advertise governance-profile conformance.

### 9.1 Policy provenance binding

For surfaces claiming `profile.doctrine_inf_governance.v0`, any
governance-sensitive run or mutation that depends on policy material MUST bind
explicit policy provenance in runtime call/witness evidence.

Minimum required provenance fields:

- `policyProvenance.pinned` (boolean),
- `policyProvenance.packageRef` (non-empty policy package/version reference),
- `policyProvenance.expectedDigest` (declared pinned digest),
- `policyProvenance.boundDigest` (runtime-bound digest).

Fail-closed rules:

- if `pinned=false` or required provenance fields are missing, fail closed with
  `governance.policy_package_unpinned`,
- if `expectedDigest != boundDigest`, fail closed with
  `governance.policy_package_mismatch`.

### 9.2 Staged guardrail preservation

Guardrail stages MUST be modeled explicitly as ordered doctrine-relevant
decisions:

1. `pre_flight`
2. `input`
3. `output`

Missing or out-of-order required stages MUST fail closed.

Risk-tier policy (`low|moderate|high`) and observability mode
(`dashboard|internal_processor|disabled`) MUST be explicit for governance
surfaces that claim this profile.

### 9.3 Evaluation flywheel evidence

Promotion decisions for policy/prompt/model changes MUST carry measurable
evidence from an evaluation loop (analyze -> measure -> improve), including:

- dataset lineage/provenance,
- grader/evaluator configuration lineage,
- decision metrics and thresholds used for acceptance/rejection.

If required governance thresholds are unmet, promotion MUST fail closed or
escalate through declared runtime policy.

### 9.4 Controlled self-evolution bounds

Automated prompt/retraining loops MUST declare:

- bounded retry policy (max attempts / terminal condition),
- terminal escalation behavior when retries are exhausted,
- deterministic rollback/revert path with lineage attribution.

Default doctrine posture: high-risk governance tiers SHOULD require explicit
human checkpoint/approval before promotion.

When a claiming surface's active governance policy declares high-risk human
checkpoint as required, implementations MUST enforce that requirement and fail
closed when approval evidence is missing.

### 9.5 Doctrine morphism attribution for governance paths

Governance implementations claiming this profile SHOULD declare preservation for
the relevant doctrine morphisms:

- `dm.policy.rebind` (policy/prompt/model rebinding under explicit run boundary),
- `dm.commitment.attest` (decision/eval/promotion evidence attestation),
- `dm.presentation.projection` (dashboard/reporting layers that MUST NOT gain
  semantic authority).

Claims that cannot establish preservation MUST be rejected or downgraded.

### 9.6 Fail-closed expectation (minimum)

Conforming governance-profile implementations SHOULD expose deterministic
fail-closed classes for at least:

- `governance.policy_package_unpinned`,
- `governance.policy_package_mismatch`,
- `governance.guardrail_stage_missing`,
- `governance.guardrail_stage_order_invalid`,
- `governance.eval_gate_unmet`,
- `governance.eval_lineage_missing`,
- `governance.self_evolution_retry_missing`,
- `governance.self_evolution_escalation_missing`,
- `governance.self_evolution_rollback_missing`,
- `governance.trace_mode_violation`,
- `governance.risk_tier_profile_missing`.

## 10. Route and world-descent consolidation boundary (WDC-1)

This section binds doctrine-level transport/route consolidation to the same
single authority lane used by world/site contracts.

Purpose:

- prevent route-bound transport actions from drifting outside declared
  world-route bindings,
- keep doctrine boundary vectors aligned with kernel-backed route admission,
- avoid wrapper-local route authority.

Canonical contract identifier:

- `doctrine.world_descent.v1`

Contract surfaces for `doctrine.world_descent.v1`:

- doctrine morphism authority: `draft/DOCTRINE-INF`,
- constructor/route-world authority: `draft/WORLD-REGISTRY`,
- route-input total binding: `draft/DOCTRINE-SITE` +
  `draft/DOCTRINE-SITE-INPUT.json`,
- resolver/KCIR projection boundary: `draft/SITE-RESOLVE`.

Conforming doctrine-boundary evaluations MUST:

- validate transport route closure through kernel world-route semantics
  (`draft/WORLD-REGISTRY` + `draft/DOCTRINE-SITE` inputs),
- require declared transport dispatch bindings when route families are marked
  required by profile or vector contract,
- fail closed when required route families or operation bindings are missing.

Minimum deterministic fail-closed classes for this boundary:

- `world_route_unbound`,
- `world_route_morphism_drift`,
- `world_route_identity_missing`,
- `world_descent_data_missing`,
- `kcir_handoff_identity_missing`.
