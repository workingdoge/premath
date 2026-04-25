---
slug: draft
shortname: HARNESS-OPTIMIZATION
title: workingdoge.com/premath/HARNESS-OPTIMIZATION
name: Harness Optimization as Admitted Experience Calculus
status: draft
category: Standards Track
tags:
  - premath
  - harness
  - optimization
  - admission
  - receipts
  - obstruction
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

## 1. Purpose and authority boundary

This specification defines harness optimization as an admitted,
receipt-bearing transition system over typed experience.

It recasts the meta-harness loop:

```text
read prior candidates / scores / traces
  -> propose harness mutation
  -> evaluate candidate harness
  -> store logs, scores, receipts, obstructions
  -> repeat
```

as a Premath control spine:

```text
Gamma
  -> intent
  -> admission
  -> realization
  -> receipt
  -> obstruction/coherence
  -> next Gamma
```

This spec is semantic/control-plane doctrine. It does not define a concrete
LLM provider, editor, filesystem layout, Nix closure implementation, or Tusk
runtime. Those are realizers or adapters of the typed construction.

Authoritative admissibility remains in `draft/PREMATH-KERNEL`,
`draft/BIDIR-DESCENT`, and `draft/GATE`. LLM proposal typing remains routed
through `draft/LLM-INSTRUCTION-DOCTRINE` and `draft/LLM-PROPOSAL-CHECKING`.
Harness operation surfaces remain partitioned by `draft/HARNESS-RUNTIME`,
`draft/HARNESS-TYPESTATE`, and `draft/HARNESS-RETRY-ESCALATION`.

## 2. Harness as a typed domain object

A harness candidate MUST be represented as a typed object before it is treated
as an executable program.

The canonical shape is:

```text
HarnessSpec :=
  Sigma {
    task_domain      : TaskDomain
    model_profile    : ModelProfile
    context_policy   : ContextPolicy
    memory_policy    : MemoryPolicy
    retrieval_policy : RetrievalPolicy
    tool_policy      : ToolPolicy
    stop_policy      : StopPolicy
    eval_contract    : EvalPolicy
  }
```

A candidate harness inhabits a typed judgment:

```text
Gamma |- H : HarnessSpec(D, M, E)
```

where:

- `Gamma` is witnessed context,
- `D` is the task domain,
- `M` is the model profile,
- `E` is the evaluation policy,
- `H` is the candidate harness.

Source code, scripts, prompts, templates, and runtime files are realizers of
the typed harness. They are not the typed harness by themselves.

## 3. Experience archive as Gamma-material

The optimization archive MUST be a typed experience object, not an untyped dump
folder.

The canonical archive shape is:

```text
ExperienceArchive :=
  Sigma {
    candidates     : List CandidateRecord
    eval_runs      : List EvalRun
    traces         : List TraceBundle
    receipts       : List Receipt
    obstructions   : List Obstruction
    improvements   : List ImprovementClaim
  }
```

The archive is admissible context material for later proposal and admission
judgments only when its rows are typed, content-addressed, and visibility-bound.

### 3.1 Candidate records

```text
CandidateRecord :=
  Sigma {
    candidate_id      : CID
    parent_ids        : List CID
    source_digest     : CID
    harness_spec      : HarnessSpec
    admission_receipt : AdmissionReceipt
  }
```

`source_digest` MUST bind all source artifacts required to reproduce the
candidate realizer. Parent links SHOULD identify the candidate lineage used by
the proposer.

### 3.2 Evaluation runs

```text
EvalRun :=
  Sigma {
    eval_id          : CID
    candidate_id     : CID
    eval_policy_id   : CID
    runtime_closure  : CID
    model_profile_id : CID
    task_split_id    : CID
    trace_bundle_id  : CID
    score_bundle_id  : CID
  }
```

An evaluation run is incomplete until it has a corresponding `EvalReceipt` or a
typed obstruction explaining why no receipt could be produced.

### 3.3 Trace bundles

```text
TraceBundle :=
  Sigma {
    trace_bundle_id : CID
    events          : List TraceEvent
  }
```

Minimum trace event families:

```text
prompt | model_output | tool_call | tool_result | memory_read
| memory_write | retrieval | state_update | stop_decision
| error | timeout
```

Trace events MUST be sufficient to reconstruct the checked interaction path
under the active `TraceCoherenceReceipt` profile.

## 4. Proposer authority

The proposer MAY inspect allowed Gamma-material and emit proposed intents.

The proposer MUST NOT mutate authority state, admit its own proposal, overwrite
candidate records, alter evaluation policy, or update improvement claims.

The proposer emits:

```text
HarnessMutationIntent :=
  Sigma {
    base_candidate  : CID
    patch           : Patch
    rationale       : Text
    expected_effect : ImprovementHypothesis
    cited_evidence  : List CID
  }
```

An intent is admissible only after a kernel-controlled admission decision:

```text
AdmissionDecision :=
  accepted
| rejected(type_error)
| rejected(policy_violation)
| rejected(leakage_risk)
| rejected(non_reproducible)
| needs_witness
| needs_smaller_eval
```

The agent or LLM is therefore a hypothesis generator, not an authority source.

## 5. Evaluation as a Pi-operator

Evaluation is a universal operation over admitted harnesses, evaluation
policies, and runtime closures:

```text
EvalPi :
  Pi (H : HarnessSpec).
  Pi (E : EvalPolicy).
  Pi (C : RuntimeClosure).
  EvalReceipt(H, E, C)
```

An implementation MAY fail to realize the evaluation, but it MUST then produce
a typed obstruction rather than silently omitting the run.

The score is not the authoritative fact. The receipt is the authoritative fact.

```text
EvalReceipt :=
  Sigma {
    candidate_id      : CID
    eval_policy_id    : CID
    runtime_closure   : CID
    task_split_digest : CID
    trace_digest      : CID
    score_digest      : CID
    result_nf         : ResultNF
    verifier_digest   : CID
  }
```

`result_nf` MUST carry the normalized result class needed by the active
evaluation policy. Scalar scores MAY be included in score bundles, but they
MUST NOT be interpreted without the receipt, split scope, verifier digest, and
runtime closure.

## 6. Improvement claims

Improvement is an admitted, scoped claim. It is not a global adjective.

```text
ImprovementClaim :=
  Sigma {
    candidate    : CID
    baseline     : CID
    metric_order : MetricOrder
    evidence     : List EvalReceipt
    split_scope  : search | dev | test | heldout
    confidence   : ConfidenceProfile
  }
```

A conforming improvement judgment MUST name its scope:

```text
Gamma |- claim : Improves(H_new, H_base, E, metric_order)
```

The unqualified claim "better harness" MUST NOT be emitted unless the
evaluation policy defines a global scope and the cited receipts satisfy it.

Recommended scoped claim names include:

- `ImprovesOnSearchSet`
- `ImprovesOnDevSet`
- `ImprovesOnHeldoutModels`
- `ImprovesOnTaskRegime`

## 7. Visibility and leakage laws

A conforming optimization policy MUST make leakage impossible by construction
where the runtime can enforce it.

```text
EvalPolicy :=
  Sigma {
    search_set     : SealedTaskSet
    dev_set        : Optional SealedTaskSet
    test_set       : SealedTaskSet
    visibility_law : VisibilityLaw
    leakage_law    : LeakageLaw
    selection_law  : SelectionLaw
  }
```

The `VisibilityLaw` MUST classify which artifacts the proposer may inspect.
For example, a policy may allow search traces and search scores while
forbidding test labels, test traces, test scores, private task text, and hidden
verifier logic.

Every proposer read of archive material MUST be receipt-bearing:

```text
ReadAccessReceipt :=
  Sigma {
    actor        : ProposerID
    path         : CIDPath
    artifact     : CID
    allowed_by   : VisibilityLaw
    logical_time : LogicalTime
  }
```

Conforming systems SHOULD be able to derive:

```text
NoForbiddenRead(proposer, sealed_scope)
```

from read receipts and the visibility law, not from after-the-fact prose audit.

## 8. Obstruction objects

A failed candidate is not merely a low score. It is a typed obstruction.

```text
Obstruction :=
  Sigma {
    candidate_id     : CID
    obstruction_type : ObstructionType
    evidence         : List CID
    local_chart      : TraceWindow
    proposed_repair  : Optional HarnessMutationIntent
  }
```

Minimum obstruction types:

```text
TypeError
| RuntimeError
| Regression
| LeakageRisk
| NonDeterminism
| BudgetViolation
| BadStopPolicy
| TraceIncoherence
```

Confounded edits SHOULD be represented explicitly:

```text
ConfoundedEditObstruction :=
  Sigma {
    candidate_a         : CID
    candidate_b         : CID
    shared_delta        : PatchComponent
    diverging_delta     : PatchComponent
    regression_receipts : List EvalReceipt
  }
```

This permits the archive to preserve reusable negative knowledge such as:

```text
prompt-template edits and stop-policy edits were not separable in this run
```

without laundering the obstruction into a scalar score.

## 9. Trace coherence

Trace bundles SHOULD be treated as patches of local charts, not only as linear
logs.

Examples of local trace charts include:

- prompt chart,
- tool-call chart,
- memory chart,
- retrieval chart,
- score chart,
- state-update chart,
- stop-policy chart.

A conforming trace coherence receipt has the shape:

```text
TraceCoherenceReceipt :=
  Sigma {
    trace_bundle_id : CID
    charts          : List LocalTraceChart
    overlaps        : List OverlapCheck
    obstruction     : Optional Obstruction
  }
```

The checker MUST report either:

```text
Gamma |- trace_bundle coherent
```

or:

```text
Gamma |- obstruction(trace_bundle)
```

Representative descent failures include:

- a memory write records fact `X`, but later prompt construction omits `X`
  despite policy requiring it;
- a retriever selects document `Y`, but prompt construction uses document `Z`;
- the stop policy records completion while verifier traces show required tests
  did not run;
- score normalization cites a trace window that does not exist in the trace
  bundle.

Implementations MAY project this structure into a nerve-style local-chart
cover. This spec requires explicit charts, overlaps, and obstruction evidence;
it does not require one concrete nerve encoding.

## 10. Runtime closure and realization

Evaluation realization MUST bind the runtime closure used for execution.

```text
RuntimeClosure :=
  Sigma {
    closure_ref     : CID
    lock_ref        : CID
    container_ref   : Optional CID
    model_profile   : ModelProfile
    tool_versions   : ToolVersionMap
    env_policy      : EnvPolicy
    network_policy  : NetworkPolicy
    secret_policy   : SecretPolicy
  }
```

Nix flakes, container digests, model profiles, and tool-version maps are
possible realizers of these fields. No particular carrier is required by this
spec.

The realization judgment is:

```text
realize(H, E, C) -> EvalReceipt | Obstruction
```

where `H` is an admitted harness candidate, `E` is the evaluation policy, and
`C` is the runtime closure.

## 11. Transition spine

The admitted optimization loop is:

```text
Gamma_n |- intent_n : HarnessMutationIntent
Gamma_n |- admit(intent_n) : AdmittedPatch
Gamma_n |- realize(AdmittedPatch) => rho_n : EvalReceipt | Obstruction
Gamma_n, rho_n |- Gamma_{n+1} : WitnessedContext
```

Where `rho_n` is either a receipt-bearing evaluation result or a typed
obstruction. Silent failure and untyped mutation are non-conforming.

The update into `Gamma_{n+1}` MUST:

- append candidate, run, trace, receipt, obstruction, and improvement rows
  without overwriting prior authority rows;
- preserve content-addressed references;
- preserve visibility/leakage derivability;
- make scoped improvement claims only from cited receipts;
- expose unresolved structure as frontier or obstruction.

## 12. Minimal admission laws

A conforming implementation MUST enforce at least these laws:

1. A candidate MUST declare its `HarnessSpec`.
2. A mutation intent MUST identify a base candidate and cite evidence.
3. A proposer MUST NOT mutate authority state directly.
4. A candidate MUST pass interface validation before evaluation.
5. A candidate MUST execute under a declared `RuntimeClosure`.
6. A proposer MUST NOT access sealed artifacts forbidden by `VisibilityLaw`.
7. A candidate evaluation MUST emit canonical trace events or a typed
   obstruction.
8. A successful evaluation MUST emit an `EvalReceipt`.
9. A trace bundle MUST emit a `TraceCoherenceReceipt` or a trace obstruction.
10. An improvement claim MUST cite receipts and MUST declare its scope.
11. A failed candidate SHOULD produce an `Obstruction`.
12. The next witnessed context MUST be derived from append-only typed archive
   rows.

## 13. Realization boundaries

This spec intentionally leaves the first concrete carrier open.

Expected realizers include:

- a Tusk filesystem-backed meta-harness loop,
- a Kurma carrier for reusable validation/normalization,
- an LLM proposal adapter,
- a Neovim or editor harness candidate,
- a KB learning harness candidate,
- sealed task-set and runtime-closure providers.

All such realizers MUST preserve the authority boundary in this spec: proposal
is not admission, score is not receipt, failure is not absence, and improvement
is not global unless globally scoped by typed evidence.

## 14. Informative source

The Meta-Harness paper is an informative source for the empirical loop shape:

- `https://arxiv.org/pdf/2603.28052v1`

This specification does not import the paper's implementation as authority. It
uses the paper's experience-driven harness-search insight as input to the
Premath admitted experience calculus.
