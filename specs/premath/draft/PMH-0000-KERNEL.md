---
slug: draft
shortname: PMH-0000-KERNEL
title: workingdoge.com/premath/PMH-0000-KERNEL
name: Premath Meta-Harness Kernel Axioms
status: draft
category: Standards Track
tags:
  - premath
  - pmh
  - harness
  - kernel
  - receipts
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

Premath Meta-Harness (PMH) is not a harness. PMH is the admitted experience
calculus by which harnesses become lawful, comparable, replayable, and
disposable.

This document defines the kernel axioms for that calculus. It is the normative
front door for the `PMH-0001` through `PMH-0011` object specifications and the
kernel refinement of `draft/HARNESS-OPTIMIZATION`.

## 2. Core distinction

A harness is a mutable specimen. The harness kernel is the durable law over
specimens.

Harness specimens perform actions. The kernel observes actions. Only the kernel
or kernel-delegated runner/archive components write evidence.

The core nonforgeability invariant is:

```text
A candidate may cause evidence to be produced,
but may not produce evidence itself.
```

In type shape:

```text
HarnessAction -> KernelObservation -> Receipt
```

not:

```text
HarnessAction -> HarnessReceipt
```

## 3. Fundamental judgment

The first PMH behavior is receipt-bearing evaluation:

```text
Candidate x RuntimeClosure x EvalPolicy -> EvalReceipt | Obstruction
```

The judgment says that a typed candidate is realized under a declared runtime
closure and evaluation policy, producing either a valid evaluation receipt or a
typed obstruction explaining why no valid receipt exists.

## 4. Kernel axioms

PMH-K1: Every candidate is typed by a `HarnessSpec`.

PMH-K2: Every realization is bound to a `RuntimeClosure`.

PMH-K3: Every evaluation emits an `EvalReceipt`, or an `Obstruction`
explaining why no valid receipt exists.

PMH-K4: Every score is subordinate to an `EvalReceipt`.

PMH-K5: Every `ImprovementClaim` is scoped by candidate, baseline, model
profile, runtime closure, eval policy, task set, and metric order.

PMH-K6: Every archive read by a learning or evaluated actor is mediated and
receipted. Internal kernel reads for validation MAY be separately audited, but
they are not the leakage boundary.

PMH-K7: Every failed or invalid run is either explained by an `Obstruction` or
marked incomplete.

PMH-K8: Every `TraceBundle` MUST pass a declared coherence check before
becoming admissible evidence.

PMH-K9: No non-kernel actor may directly mutate authority state or append
kernel evidence.

PMH-K10: Every retired harness remains replayable, or carries an
`UnreplayableReceipt` explaining the missing model, closure, provider,
verifier, policy, or dependency reason.

PMH-K11: Harness specimens MAY call only mediated capabilities exposed by their
`RuntimeClosure` and `EvalPolicy`.

PMH-K12: A denied access attempt is itself evidence and MUST be represented as
a `TraceEvent` plus `VisibilityReceipt`.

## 5. Actor classes

PMH distinguishes these actor classes:

- `kernel actor`: owns authority state and evidence append rights,
- `runner actor`: executes admitted runs on behalf of the kernel,
- `archive actor`: mediates artifact access and stores kernel-owned evidence,
- `grader actor`: computes outcome/score evidence under `EvalPolicy`,
- `evaluated actor`: the harness specimen under evaluation,
- `learning actor`: a proposer or optimizer that reads allowed experience,
- `operator actor`: a human or control-plane agent that selects work.

Only kernel actors and kernel-delegated runner/archive/grader components MAY
append kernel evidence. Evaluated and learning actors MUST NOT receive direct
evidence-writer capabilities.

## 6. Harness-visible capabilities

The v0 evaluated specimen receives a capability-scoped facade:

```text
list_allowed(task_id)
read_artifact(task_id, artifact_id)
submit_answer(task_id, answer_json)
```

The specimen MUST NOT receive:

```text
append_trace_event
append_visibility_receipt
append_eval_receipt
append_obstruction
append_score_bundle
append_outcome_bundle
```

Those methods are kernel-owned.

## 7. Kernel-owned evidence operations

The kernel-owned side MAY expose operations equivalent to:

```text
append_trace_event(...)
append_visibility_receipt(...)
append_score_bundle(...)
append_outcome_bundle(...)
append_obstruction(...)
append_eval_receipt(...)
```

These operations are authority-state mutations. They MUST be unavailable to the
evaluated candidate and to learning actors except through explicit,
kernel-admitted control paths.

## 8. Mediation rule

Archive mediation is the PMH leakage boundary.

For a mediated artifact read, the kernel observes:

```text
read_artifact(task_id, artifact_id)
  -> check VisibilityLaw
  -> append TraceEvent(kind=file_read_attempt)
  -> append VisibilityReceipt(decision=allowed | denied)
  -> return ArtifactView | AccessDenied
```

Denied reads MUST NOT be invisible. A denied read is evidence even when no
artifact bytes are returned.

## 9. Evidence rule

Scores are not facts. Receipts are facts.

A score bundle MAY be useful evidence only when it is referenced by a valid
`EvalReceipt` that binds the candidate, runtime closure, evaluation policy,
trace bundle, visibility bundle, outcome bundle, verifier, and result normal
form.

## 10. Scope rule

Improvement claims are always scoped. A PMH implementation MUST NOT emit an
unqualified "better harness" claim unless the referenced `EvalPolicy` defines a
global scope and the cited receipts satisfy that scope.

Tiny or v0 fixture comparisons SHOULD use `confidence: descriptive_only`.

## 11. Replay rule

A retired harness MUST either remain replayable or carry an
`UnreplayableReceipt`. Replay failure is not silent deletion; it is a typed
fact about the missing model, closure, provider, verifier, policy, artifact, or
dependency.

## 12. Invalidity rule

Invalid runs MAY be informative, but they are not admissible improvement
evidence unless an `EvalPolicy` explicitly admits the obstruction class for the
claim being made.

## 13. V0 conformance target

The first PMH conformance target is deliberately small:

- three manual harness candidates,
- one deterministic benchmark,
- mediated archive access only,
- typed trace events,
- typed visibility receipts,
- deterministic grading,
- typed score and outcome bundles,
- typed eval receipts,
- obstruction emission for invalid runs,
- trace coherence checks,
- no mutation and no LLM proposer.

The v0 milestone is:

```text
candidate x runtime closure x eval policy -> valid receipt
```

Once this exists, Meta-Harness-style mutation becomes a learning actor that
consumes lawful archive material and proposes mutation intents under admission.
