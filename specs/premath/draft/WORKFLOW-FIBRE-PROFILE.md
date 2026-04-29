---
slug: draft
shortname: WORKFLOW-FIBRE-PROFILE
title: workingdoge.com/premath/WORKFLOW-FIBRE-PROFILE
name: Workflow Fibre-Space Profile
status: draft
category: Standards Track
tags:
  - premath
  - workflow
  - fibre-space
  - harness
  - projection
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

This specification defines `premath.fibre_space.workflow_profile.v1`, a
Premath-owned meaning contract for projecting governed workflow evidence into a
fibre-space shape.

The profile gives workflow runtimes a common vocabulary for:

- base/site context,
- fibre/local workflow context,
- coverage/support,
- sections,
- gluing,
- admissible motion,
- realizations,
- receipts,
- deterministic projections.

This specification does not define a scheduler, tracker, terminal, process
runner, or agent protocol. Those are implementation or consumer surfaces.

## 2. Authority Boundary

Premath owns the meaning contract:

- the base/fibre vocabulary,
- the admissibility and gluing criteria,
- the projection row shape,
- the boundary between semantic law and consumer evidence.

Consumers such as Tusk MAY project local ledger, receipt, lane, tracker, or
runtime evidence into this contract. Such projections are evidence views only.
They MUST NOT introduce independent semantic authority.

Carriers such as Kurma MAY later check or transport this profile, but only after
this Premath contract and a consumer projection envelope are stable. A Kurma
carrier/checker MUST NOT own Tusk scheduling authority or Premath law.

Meta-Harness object doctrine remains outside this specification. Premath
supplies upstream fibre-space, descent, Gate, instruction, and evidence doctrine
that Meta-Harness sites MAY depend on.

## 3. Profile Kind

The canonical profile kind is:

```text
premath.fibre_space.workflow_profile.v1
```

Rows claiming this profile MUST use:

```text
schema: 1
profileKind: "premath.fibre_space.workflow_profile.v1"
```

The profile is a deterministic projection shape. It does not mutate source
state.

## 4. Fibre-Space Reading

Let `C_work` be a workflow context category. Objects are workflow base contexts,
for example repository state, issue state, lane state, runtime state, or a
bounded product of those contexts. Morphisms are context maps such as:

- checkpoint advance,
- tracker-state refinement,
- lane handoff,
- workspace realization,
- retry/reconciliation update,
- evidence-prefix inclusion.

Let `E_work -> C_work` be the workflow fibre space. A fibre over a base context
contains local workflow material admissible over that context, for example:

- selected work-item evidence,
- section/evidence rows,
- realized runtime artifacts,
- receipt references,
- projection references,
- obstruction reports.

The term "fibre space" is canonical in this profile. "Grothendieck
construction" MAY be used as construction language when discussing an indexed
family as a total space, but it is not the profile name.

## 5. Vocabulary

### 5.1 Base or site

`base` identifies the workflow context object in `C_work`.

A base row SHOULD contain:

- `baseId`: stable identifier for the base context,
- `baseKind`: context family or site name,
- `contextRefs`: deterministic references to source context material,
- `policyRefs`: OPTIONAL policy/profile references that constrain the context.

### 5.2 Fibre or local context

`fibre` identifies local workflow material over one base.

A fibre row SHOULD contain:

- `fibreId`: stable identifier for the local material,
- `fibreKind`: local material family,
- `overBaseId`: base identifier,
- `evidenceRefs`: deterministic source evidence references,
- `witnessRefs`: witness references,
- `lineageRefs`: lineage references needed to replay the projection.

### 5.3 Coverage and support

`support` identifies the cover or support family used to justify local
reasoning.

A support row SHOULD contain:

- `supportId`: stable support identifier,
- `supportKind`: cover/support family,
- `baseId`: covered base context,
- `memberRefs`: deterministic references to cover members,
- `refinementRefs`: OPTIONAL refinement lineage.

### 5.4 Section

`section` identifies a selected local workflow row over a base or support
member. Sections are not automatically globally valid.

A section row SHOULD contain:

- `sectionId`: stable section identifier,
- `sectionKind`: selected evidence family,
- `baseId` or `supportId`,
- `fibreId`: local fibre material,
- `resultClass`: deterministic class such as `accepted`, `rejected`,
  `observed`, `incomplete`, or `obstructed`,
- `witnessRefs`,
- `lineageRefs`.

### 5.5 Gluing

`gluing` identifies a deterministic glue-or-obstruction result over compatible
sections.

A gluing row SHOULD contain:

- `gluingId`: stable gluing identifier,
- `supportId`: support family being glued,
- `sectionRefs`: selected section identifiers,
- `resultClass`: `glued` or `obstructed`,
- `obstructionRefs`: REQUIRED when `resultClass = obstructed`,
- `witnessRefs`,
- `lineageRefs`.

`glued` MUST NOT be emitted unless the selected local sections are compatible
under the active cover/support policy and the projection carries enough lineage
to replay the selection boundary.

### 5.6 Admissible motion

`motion` identifies a context map plus the corresponding total-space movement.

A motion row SHOULD contain:

- `motionId`: stable motion identifier,
- `motionKind`: transition family,
- `fromBaseId`,
- `toBaseId`,
- `contextMapRef`,
- `totalMapRef`,
- `admissible`: boolean,
- `failureClasses`: deterministic string list,
- `witnessRefs`,
- `lineageRefs`.

If `admissible = true`, the motion MUST preserve the projection law for the
declared base/fibre relationship. If it does not, the motion MUST be rejected or
reported as obstructed.

### 5.7 Realization

`realization` identifies an execution or materialization of a section, gluing,
or motion in a concrete runtime.

A realization row SHOULD contain:

- `realizationId`: stable realization identifier,
- `realizationKind`: runtime/materialization family,
- `sourceRefs`: section, gluing, or motion refs being realized,
- `runtimeRefs`: implementation-specific runtime refs,
- `resultClass`,
- `witnessRefs`,
- `lineageRefs`.

Realization rows are runtime evidence. They MUST NOT by themselves establish
semantic admissibility.

### 5.8 Receipt

`receipt` identifies a durable attestation or completion artifact.

A receipt row SHOULD contain:

- `receiptId`: stable receipt identifier,
- `receiptKind`: receipt family,
- `sourceRefs`: realization, section, gluing, or motion refs,
- `receiptRefs`: external or local receipt references,
- `resultClass`,
- `witnessRefs`,
- `lineageRefs`.

Receipts are evidence. They become admissibility evidence only when they are
bound through the appropriate Gate/discharge or consumer projection contract.

### 5.9 Projection

`projection` identifies a deterministic view derived from source authority
surfaces.

A projection row SHOULD contain:

- `projectionId`: stable projection identifier,
- `projectionKind`: projection family,
- `sourceRefs`,
- `normalizerId` or `projectionPolicyRef`,
- `resultClass`,
- `writes`: an empty list for read-only projections,
- `witnessRefs`,
- `lineageRefs`.

Consumer projections into this profile SHOULD use `writes: []` unless a separate
mutation authority contract explicitly governs the write.

## 6. Canonical Envelope

A projection into this profile SHOULD use the following envelope shape:

```text
WorkflowFibreProfileProjection {
  schema: 1,
  profileKind: "premath.fibre_space.workflow_profile.v1",
  projectionKind: string,
  projectionId: string,
  source: {
    kind: string,
    refs: list<string>
  },
  base: list<BaseRow>,
  fibres: list<FibreRow>,
  supports: list<SupportRow>,
  sections: list<SectionRow>,
  gluings: list<GluingRow>,
  motions: list<MotionRow>,
  realizations: list<RealizationRow>,
  receipts: list<ReceiptRow>,
  projections: list<ProjectionRow>,
  readiness: {
    classification: "accepted" | "rejected" | "incomplete" | "obstructed",
    missing: list<string>,
    obstructions: list<string>
  },
  witnessRefs: list<string>,
  lineageRefs: list<string>,
  writes: list<never>
}
```

All row identifiers within a projection MUST be deterministic for the same
normalized source material and profile policy. Reference arrays MUST be trimmed,
sorted, and deduplicated unless a row explicitly declares an order-sensitive
field.

## 7. Gluing and Obstruction Discipline

The profile inherits the Premath kernel discipline:

- context change MUST be stable,
- local sections MUST restrict along declared support maps,
- compatible local sections MUST glue when the active law says they should,
- failed gluing MUST emit a deterministic obstruction,
- refinement MUST NOT change accepted meaning except by declared equivalence.

Operational consumers MAY report `incomplete` when the source material is not
yet sufficient to decide gluing or admissible motion. They MUST NOT upgrade
`incomplete` to `accepted` without the missing evidence.

## 8. SigPi Placement

SigPi obligations (`\Sigma_f -| f* -| \Pi_f`) belong to the semantic/profile
lane, not to a workflow consumer.

When a workflow profile projection claims that a context map supports dependent
aggregation, restriction, or quantification, it MUST route that claim through the
existing SigPi/profile surfaces:

- `profile/ADJOINTS-AND-SITES`,
- `draft/SPAN-SQUARE-CHECKING` when pullback/base-change commutation evidence is
  required,
- `draft/BIDIR-DESCENT` and `draft/GATE` for discharge and admissibility
  verdicts.

Consumer rows MAY cite SigPi witness or span/square refs. They MUST NOT define a
parallel SigPi law.

## 9. Tusk Consumer Boundary

Tusk MAY project local governed-runtime evidence into this profile. Typical
source material includes:

- run-ledger prefixes,
- receipt rows,
- lane state,
- tracker issue state,
- session/runtime state,
- realization and closeout receipts.

Tusk-owned projections MUST:

- remain read-only unless governed by a separate mutation contract,
- preserve source authority boundaries,
- treat ledger/receipt material as evidence, not sole authority state,
- carry `witnessRefs` and `lineageRefs` for external Kurma/Nerve material
  without reinterpreting that material,
- report missing source material as `incomplete` or `obstructed`, not accepted.

Tusk MUST NOT claim that a local projection proves Premath law. It may only
claim that its local evidence has been deterministically shaped for this
Premath profile.

## 10. Kurma Carrier Boundary

Kurma MAY define a carrier/checker for this profile after the Premath/Tusk edge
is stable.

Such a carrier/checker SHOULD consume:

- this profile contract,
- a Tusk projection envelope,
- declared witness and lineage refs,
- any required normalization or policy refs.

Kurma MAY report pass/fail/obstruction for carried projection shape, ref
integrity, and method-specific checks. It MUST NOT own Tusk scheduling,
workspace, tracker, or lane authority.

## 11. Rejection Classes

A projection or checker consuming this profile SHOULD use deterministic failure
classes. Recommended classes:

- `workflow_profile_missing_base`
- `workflow_profile_missing_fibre`
- `workflow_profile_missing_support`
- `workflow_profile_section_outside_support`
- `workflow_profile_gluing_obstructed`
- `workflow_profile_motion_not_admissible`
- `workflow_profile_receipt_unbound`
- `workflow_profile_projection_not_replayable`
- `workflow_profile_consumer_boundary_violation`

Implementations MAY add more specific classes, but unknown classes MUST be
treated conservatively by consumers.

## 12. Compatibility With Existing Harness Trajectory Rows

`premath.harness.step.v1` rows are valid source evidence for this profile. They
SHOULD project as sections, realizations, or receipts depending on their source
action and result class.

This specification does not replace `draft/HARNESS-RUNTIME`. The harness
runtime contract remains the operational row contract for `boot/step/stop` and
trajectory capture. This profile defines the fibre-space meaning view that a
consumer projection can derive from such evidence.
