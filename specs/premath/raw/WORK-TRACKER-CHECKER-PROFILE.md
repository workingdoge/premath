---
slug: raw
shortname: WORK-TRACKER-CHECKER-PROFILE
title: workingdoge.com/premath/WORK-TRACKER-CHECKER-PROFILE
name: Work Tracker Checker Profile
status: raw
category: Informational
tags:
  - premath
  - tracker
  - checker
  - work
  - atlas
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

This raw profile defines the provisional Premath checker endpoint for a
simplex-native work tracker.

It is the Premath endpoint of the Atlas cover:

```text
simplex.substrate -> work.semantic_state -> premath.work_checker -> tusk.tracker_instrument
```

Premath checks a canonical work-transition claim. Premath does not own the
meaning of work state.

This profile defines:

- checker input shape;
- admissibility decision output;
- failure classes;
- projection-separation checks;
- checker expectations for a future executable checker.

It does not define tracker runtime, UI, daemon behavior, storage, board layout,
or generic work semantics.

## 2. Authority Boundary

Premath owns only the checker/kernel-facing surface:

- `premath.work_checker.input_nf`
- `premath.work_checker.admissibility_check`
- `premath.work_checker.decision`
- `premath.work_checker.failure_classes`
- `premath.work_checker.checker_vectors`

Premath does not own:

- reusable simplex substrate;
- generic work-state meaning;
- acceptance, verification, or handoff semantics outside the checker input
  contract;
- Tusk CLI, MCP, daemon, worker-loop, or UI behavior;
- `bd` storage or graph-native authority;
- downstream product policy or live proof.

Until a durable `work` site exists, Atlas may use this profile as the
checker-facing placeholder for work-transition claims. That does not make
Premath the owner of `work.semantic_state`.

## 3. External Inputs

This profile assumes these external authority surfaces:

- `simplex.substrate`: simplex identity, boundary, patch, and local-to-global
  vocabulary. This is provisionally hosted by Nerve and factored through Atlas
  `ATLAS-0003-SIMPLEX-FACTOR-CANDIDATE.md`.
- `work.semantic_state`: work-state meaning, transition meaning, acceptance,
  verification, and handoff semantics. This is an Atlas candidate site role,
  not a Premath-owned surface.
- `tusk.tracker_instrument`: operator commands and runtime projection surfaces.
  These instrument accepted checker decisions rather than define them.

## 4. Checker Objects

### 4.1 `WorkClaimNF`

`WorkClaimNF` is the canonical checker input for one proposed work transition.

It is not the semantic owner of the work object. It is the normalized envelope
Premath can check.

A conforming `WorkClaimNF` MUST identify:

- cover reference;
- work semantic profile reference;
- simplex substrate references;
- work subject reference;
- operation class;
- prior work-state references;
- claimed output work-state references;
- boundary references required by the semantic profile;
- evidence references;
- actor or instrument reference, when relevant;
- projection references, if any projection is being checked.

Operation classes are interpreted by the referenced work semantic profile.
Premath may check that the class is declared and that required evidence exists;
Premath MUST NOT define the general meaning of the operation class.

### 4.2 `WorkCheckDecision`

`WorkCheckDecision` is the checker output for one `WorkClaimNF`.

A conforming decision MUST include:

- decision: `accept` or `reject`;
- checker profile reference;
- input claim reference or digest;
- accepted output reference, when accepted;
- failure classes, when rejected;
- evidence references used by the checker.

### 4.3 `WorkProjectionCheck`

`WorkProjectionCheck` is an optional checker input for a projection over
accepted work state.

It checks that a projection cites or deterministically derives from accepted
authority. It does not make the projection authoritative.

## 5. Checker Rules

### Rule C1 - Explicit Authority Inputs

A checker input MUST name the cover, semantic profile, substrate references,
and work-state references it depends on.

If those inputs are missing or profile-inconsistent, the checker MUST reject.

### Rule C2 - Boundary Evidence

A transition claim that depends on boundary work-state references MUST cite
accepted boundary evidence under the active cover.

Premath checks that the evidence is present and profile-compatible. The meaning
of the boundary relation belongs to the substrate/work semantic surfaces.

### Rule C3 - Deterministic Decision

For the same canonical input and checker profile, the checker decision MUST be
deterministic.

Different storage encodings, UI actions, or command paths MUST NOT change the
checker result.

### Rule C4 - Projection Is Not Authority

Ready lists, blocked lists, dependency graphs, boards, dashboards, command
transcripts, `bd` rows, and import/export files MUST NOT authorize mutation by
themselves.

They may be accepted only as projections or compatibility inputs that cite
accepted authority.

### Rule C5 - No Work Semantics By Accident

Premath MUST NOT infer generic work semantics from graph shape, issue metadata,
Tusk runtime state, or board layout.

If the checker needs a semantic relation, that relation must be declared in the
referenced work semantic profile or supplied as explicit evidence.

### Rule C6 - Handoff Check

When a handoff claim is checked, the checker MUST verify that the claim carries
the recovery evidence required by the referenced semantic profile.

The handoff artifact may be projected into Tusk session state, but the Tusk
projection is not the authority unless it cites the accepted checker decision.

## 6. Failure Classes

A conforming checker SHOULD classify rejected inputs with one or more of:

- `work_checker.missing_authority`
- `work_checker.invalid_boundary`
- `work_checker.stale_input`
- `work_checker.unsupported_operation`
- `work_checker.conflicting_transition`
- `work_checker.projection_as_authority`
- `work_checker.invalid_handoff`
- `work_checker.profile_mismatch`

Failure classes are checker output. Human-readable diagnostics, UI grouping,
retry policy, and operator shortcuts belong to Tusk or downstream projections
unless another Premath contract explicitly claims them.

## 7. Compatibility Rule

`bd` JSONL rows, Dolt state, issue metadata, and import/export files MAY seed,
compare, or project tracker state.

They MUST NOT define work authority. A compatibility import must become a
`WorkClaimNF` or projection check before Premath can accept or reject it.

## 8. Checker Sketch

A future checker suite for this profile should include vectors for:

- accepting a simple transition with explicit authority inputs;
- rejecting a transition with missing boundary evidence;
- rejecting a projection used as mutation authority;
- rejecting a checker input that relies on graph shape as work semantics;
- accepting a projection check that cites an accepted decision;
- checking a handoff claim with required recovery evidence.

No implementation claims checker compatibility with this raw profile until vectors and a
promotion path are declared.

## 9. Non-Goals

This profile does not define:

- a durable `work` site;
- a durable `simplex` site;
- work-state ontology;
- tracker daemon behavior;
- CLI, MCP, or UI commands;
- storage backend schema;
- board layout;
- issue ID format;
- worker scheduling policy;
- retry/escalation policy;
- migration of existing tracker data.

## 10. Next Promotion Step

Current fixture reference:

- `tests/checker/fixtures/work-tracker-checker/`
- `cargo run --package premath-cli -- work-tracker-check --input <input.json> --json`

Current Tusk instrument reference:

- `tusk/design/notes/tusk-work-tracker-instrument-profile.md`

Before promotion, this profile needs:

1. alignment with Atlas `work-tracker.v0`;
2. expanded checker fixture coverage beyond the initial raw vectors;
3. implementation guidance that continues to cite the Tusk instrument note
   rather than redefining mutation authority;
4. a compatibility rule for importing and exporting current `bd` state;
5. a decision on whether repeated semantic leakage justifies a durable `work`
   site outside Premath.
