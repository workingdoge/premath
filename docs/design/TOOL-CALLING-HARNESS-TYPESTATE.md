# Tool-Calling Harness Typestate

Status: draft
Scope: design-level, non-normative

Authority note:

- this document is design guidance only,
- normative typestate authority is now
  `specs/premath/draft/HARNESS-TYPESTATE.md`,
- executable acceptance authority MUST come from promoted draft contracts and
  executable check surfaces, not from this document alone.

## 1. Why this document

This document defines a practical type contract for tool-calling in the
harness: one typed call spec per model turn, with deterministic closure before
any issue-memory mutation.

Design goal:

- keep mutation authority in existing premath lanes (`.premath/issues.jsonl`,
  instruction-linked policy, witnesses),
- make each model call replayable and auditable,
- fail closed when tool-call closure is incomplete or policy drifts.

## 2. Evidence baseline from `executable_code_actions.pdf`

The paper shows operational properties that matter for harness design:

1. Code-as-action provides native control/data flow (variables, loops,
   conditionals) and can compose multiple tool calls in one emitted action.
2. Multi-turn observation (execution output, error feedback) is required for
   dynamic correction of prior actions.
3. JSON/text action formats are simpler but typically constrain composition
   (appendix formatting examples explicitly enforce one tool invocation per
   JSON/text action).
4. Action quality improves when the agent can iterate on environment feedback
   instead of treating first output as terminal.

For harness policy, this implies the boundary is not just "tool call happened",
but "tool-call graph for this turn is closed under policy and observation".

## 3. Additional production guidance (Anthropic + OpenAI + Amp)

Two implementation constraints from production agent systems are directly
relevant to this contract:

1. Prefer the simplest architecture that works, and add agentic complexity only
   when it improves outcomes measurably (workflow first, autonomous loop when
   needed).
2. Treat tool design as part of prompt engineering (agent-computer interface):
   parameter names, formats, examples, and ambiguity reduction materially affect
   tool-call correctness.
3. Keep the inner agent loop minimal: one deterministic turn contract plus
   explicit state transitions, instead of implicit orchestration behavior.
4. Treat context as bounded state, not append-only transcript carryover:
   reconstruct model-visible context from typed state views, handoff packets,
   and required artifact refs.
5. Tool failures should remain machine-readable and policy-checkable; free-text
   error narration alone is insufficient for retry/stop/mutation gating.

For this harness, that means `CallSpec` should encode:

- execution pattern: `single | chain | route | parallel | orchestrator_workers | evaluator_optimizer`,
- stopping policy: max iterations and human checkpoint requirements,
- tool-interface quality inputs: schema digest plus usage-example digest.

Additional constraints from advanced tool-use production patterns:

- large tool catalogs should be modeled as discoverable inventory
  (`defer_loading`) rather than eagerly loaded context,
- programmatic execution must be caller-scoped (`allowed_callers`) with
  validated caller edges,
- schemas alone are insufficient for correctness; usage examples are a typed
  input to call validity, not just optional prompt text.
- token-intensive domains benefit from pre-context filtering via code execution,
  but cost effects are workload/model dependent and require local evaluation.
- tool protocol state (structured tool blocks + stop reasons) is part of the
  type contract, not UI-level metadata.
- long-running coding sessions should default to descent decomposition
  (bounded sessions + typed handoffs) instead of transcript-linear compaction;
  if compaction is enabled, it must remain typed and continuity-checked.
- tool transport policy matters: parallel call enablement, call/output ordering,
  and truncation policy should be explicit and auditable.
- coding-agent runtime boundaries should be typed: shell execution sandbox,
  mandatory `workdir`, and preferred code-edit surface (`apply_patch`).
- governance should be policy-as-code with package/version provenance, not
  ad-hoc per-agent configuration.
- guardrails are stage-typed (`pre_flight`, `input`, `output`) and should be
  represented as first-class turn evidence.
- policy changes should be gateable by measured eval performance (precision,
  recall, F1) and adversarial regression checks.
- observability mode is policy-bound (`dashboard | internal_processor |
  disabled`) to support Zero Data Retention constraints.
- controls should be risk-proportionate with explicit risk-tier policy
  selection.
- multi-agent orchestration should encode explicit handoff contracts with
  required artifact existence checks at each transfer boundary.
- context transfer across orchestrator/specialist boundaries should be
  packetized and typed (intent, constraints, artifact refs, return path), not
  inferred from implicit chat-history carryover
  (`https://michaellivs.com/blog/multi-agent-context-transfer/`).
- specialist agents should run with role-scoped context contracts to reduce
  cross-role drift and preserve repeatability.

## 4. Core dependent contract

Treat each model turn as a typed object whose admissible next operations depend
on current evidence:

`CallSpec -> ToolRequests -> ToolResults -> ToolUse -> JoinClosed -> MutationReady`

`MutationReady` must not be constructible unless all dependent obligations hold.

### 4.1 `CallSpec`

Minimum fields:

- `callId`: deterministic turn identity.
- `modelRef`: model/version identity used for the turn.
- `actionMode`: `code | json | text`.
- `executionPattern`: `single | chain | route | parallel | orchestrator_workers | evaluator_optimizer`.
- `toolChoice`: provider tool-choice policy (`auto | any | specific`).
- `toolPolicy`: allow-list + per-tool schema digest.
- `toolCatalogDigest`: digest over the full discoverable tool inventory.
- `deferPolicyDigest`: digest over eager-vs-deferred loading config.
- `toolExampleDigest`: digest over usage examples attached to tools.
- `allowedCallersDigest`: digest over per-tool caller allow-lists.
- `preContextFilterPolicy`: whether tools may execute filtering before loading
  content into model context.
- `evalProfileRef`: benchmark profile id for local accuracy/cost validation.
- `parallelPolicy`: whether parallel tool branches are admissible.
- `toolTransportPolicy`: transport flags and ordering contract (including
  `parallel_tool_calls` behavior).
- `toolResponseTruncationPolicy`: max budget + middle-truncation marker
  contract.
- `toolRenderProtocolDigest`: dual-rendering contract for tool outputs (operator
  payload + model-facing reminder rendering).
- `executionBoundaryPolicy`: allowed edit surfaces and shell/edit separation
  constraints.
- `runtimeSandboxRef`: shell/runtime sandbox profile binding.
- `compactionPolicyDigest`: optional compaction invocation and carry-forward
  item policy (fallback/compatibility path).
- `reminderQueuePolicyDigest`: queue/batching/dedup policy for reminder-bearing
  injections derived from tool outputs.
- `stateViewPolicyDigest`: deterministic conversation-state view definitions
  used for stop/enforcement decisions.
- `decompositionPolicyDigest`: policy for loop-vs-split worker decomposition and
  fan-out admissibility under `executionPattern`.
- `governancePolicyDigest`: pinned policy package/config provenance.
- `guardrailStagePolicyDigest`: stage-specific guardrail policy over
  `pre_flight`, `input`, `output`.
- `evalGatePolicyDigest`: performance gate policy for precision/recall/F1
  thresholds on labeled datasets.
- `redTeamPolicyDigest`: adversarial regression policy (plugins/strategies/min
  probes).
- `observabilityMode`: `dashboard | internal_processor | disabled`.
- `riskTier`: `low | moderate | high` with bound control profile.
- `orchestrationRole`: current role identity (`orchestrator | specialist`).
- `contextScopeDigest`: digest over role-allowed context/materialized inputs.
- `handoffContractDigest`: typed transfer contract (allowed targets, required
  artifacts, and return path).
- `mutationPolicyDigest`: expected instruction/mutation policy binding.
- `normalizerId`: output normalization contract.
- `stopPolicy`: max turns / checkpoint requirements.
- `protocolStatePolicy`: handled stop reasons (`tool_use`, `pause_turn`,
  `max_tokens`, `end_turn`) and continuation behavior.

### 4.2 `ToolRequests`

Derived from model output under `CallSpec`:

- each requested tool call has stable `toolCallId`,
- requests are extracted only from structured tool-use blocks,
- for `text` mode, a deterministic adapter MUST first normalize text records
  into structured tool-use blocks under an explicit parser profile; direct
  free-text extraction is invalid,
- each request is schema-validated against `toolPolicy`,
- request digest set is deterministic (order-insensitive multiset digest).
- if dynamically discovered, include `searchRef` (query + result-set digest),
  plus resolved tool identity bound to `toolCatalogDigest`.
- if parallel transport is enabled, preserve deterministic call-group ordering
  policy for later closure checks.
- pre-flight guardrail decisions are attached before tool execution begins.
- for orchestrated runs, the request set includes the active handoff contract
  projection (required incoming artifact refs and allowed transfer targets).

### 4.3 `ToolResults`

For each `toolCallId`:

- success/failure class is typed,
- failure rows include machine-readable envelope fields
  (`errorCode`, `retryable`, `errorMessageDigest`) with optional human-readable
  summary,
- output payload hash is recorded,
- provenance ref is attached (`command`, `api`, or runtime route).
- when programmatic execution is used, preserve caller-edge provenance
  (`code_execution` parent id -> concrete tool call id).
- caller-edge provenance must satisfy `allowedCallersDigest` for the invoked
  tool.
- record token-usage deltas for filtered vs unfiltered retrieval where
  applicable.
- if result payloads are truncated, emit truncation metadata conforming to
  `toolResponseTruncationPolicy`.
- shell-executed rows carry boundary evidence (`workdir`, sandbox profile,
  command digest).
- stage guardrail outcomes include guardrail id, trigger state, and
  threshold/model provenance when configured.
- tool rows may carry dual-render evidence: operator payload hash plus
  model-facing reminder rendering hash under `toolRenderProtocolDigest`.
- artifact-write rows include path + digest and are bound to role ownership
  constraints when present.

### 4.4 `ToolUse`

Required before join closure:

- every terminal `toolCallId` result is mapped to deterministic use disposition:
  `consumed | observed_only | discarded_with_reason | retry_scheduled`,
- `consumed` rows include provenance to the dependent synthesis/mutation intent
  (`summaryRef`, `handoffRef`, or mutation input ref),
- `discarded_with_reason` rows include a typed reason code,
- no `consumed` disposition may reference unknown or non-terminal result rows,
- use digest is stable over `{toolCallId, resultDigest, useDisposition,
  useReason?}`.

### 4.5 `JoinClosed`

Required before any synthesis or mutation:

- every requested `toolCallId` has exactly one terminal result row,
- no unknown tool ids or orphan result rows,
- branch cardinality matches `parallelPolicy`,
- join digest is stable over `{requestDigestSet, resultDigestSet,
  toolUseDigestSet}`.
- if `executionPattern=parallel`, require all branch joins closed before
  synthesis.
- stop reason must be admissible under `protocolStatePolicy`; otherwise closure
  fails.
- transport ordering must satisfy `toolTransportPolicy` when strict ordering is
  selected.
- required state views and queue reductions are present under
  `stateViewPolicyDigest` + `reminderQueuePolicyDigest` before continuing
  iterative tool loops.
- required guardrail stage decisions are present and ordered
  (`pre_flight -> input -> output`) under `guardrailStagePolicyDigest`.
- if handoff is attempted, required artifact set from `handoffContractDigest`
  must be satisfied before transfer closure.
- if synthesis or mutation is attempted, required `toolUse` evidence for
  consumed rows must be present and type-valid.

### 4.6 `MutationReady`

Construct only if:

- `JoinClosed` is true,
- `mutationPolicyDigest` matches active instruction-linked policy,
- required capability claims for intended mutation are present,
- all terminal tool-failure rows satisfy the typed `ToolResults` error envelope
  contract,
- no fail-closed class is active (`tool.unknown_or_disallowed`,
  `tool.join_incomplete`, `tool.use_missing`, `tool.use_unknown_result`,
  `tool.use_without_result`, `mutation.policy_digest_mismatch`,
  `mutation.capability_claim_missing`, `mutation.use_evidence_missing`).
- for governance-policy mutations: eval and red-team gates satisfy
  `evalGatePolicyDigest` and `redTeamPolicyDigest`.
- for orchestrated transfer mutations: caller role and transfer target satisfy
  `handoffContractDigest`.

## 5. Action-mode typing rules

### 5.1 `code` mode

- permit intra-action composition and control/data flow,
- allow multiple tool invocations in one action block,
- require execution-observation loop typing (`stdout/stderr/traceback` are part
  of turn evidence).
- preferred when intermediate data is large and should be processed before
  entering model context.

### 5.2 `json` mode

- one structured tool invocation per action object,
- no implicit control/data flow inside one invocation,
- composition must be represented as multi-turn sequencing.
- preferred when strict single-call determinism matters more than orchestration
  efficiency.

### 5.3 `text` mode

- minimal parser surface, highest ambiguity risk,
- direct tool invocation from raw text is unsupported by default,
- tool execution is permitted only after deterministic adapter normalization
  into structured tool-use blocks,
- strongest normalization requirement before request hashing.
- if no adapter/profile is configured, treat text actions as observe-only and
  fail closed for tool execution intents.

## 6. Harness mapping to current premath surfaces

Type state to concrete surfaces:

- `CallSpec`: instruction envelope + runtime call policy binding.
- `ToolRequests/ToolResults/ToolUse`: harness worker step traces and runtime
  outputs.
- `JoinClosed`: deterministic closure check over request/result/use sets before
  issue mutation.
- `MutationReady`: gate for `issue update`, `issue discover`, dependency edits.
- `Witness`: trajectory/session refs (`harness-trajectory`, `harness-session`)
  plus CI witness artifacts when applicable.

This keeps issue-memory authoritative while making per-turn tool behavior
typed and replayable.

### 6.1 Purity boundary by crate

Keep contract purity explicit:

- `premath-kernel`: pure normative semantics only (typed contracts,
  deterministic check/decision functions, fail-closed taxonomy, witness shape).
- `premath-tusk`: runtime evidence collection/normalization (tool events,
  artifact observations, handoff traces) into kernel-checkable inputs.
- `premath-cli`: command surface and wiring for local/CI execution.
- `premath-bd`: persistence of issue/mutation/witness rows.

No MCP/session/process/filesystem side effects are part of kernel semantics.

### 6.2 Placement and promotion policy

Default placement rule:

- new harness behavior starts in `premath-tusk` unless it is already a stable,
  reusable pure semantic rule.
- there is no automatic "move from tusk to kernel" step in this design.

Promotion to `premath-kernel` is optional and only valid when all criteria hold:

- rule can be expressed as pure input -> deterministic output with no runtime
  I/O assumptions.
- at least two call sites need the same normative decision semantics.
- fail-closed taxonomy and witness fields are stable enough for conformance
  vectors.
- a `premath-tusk` adapter can supply required evidence without embedding
  provider/runtime coupling into kernel types.

If any criterion is unmet, keep semantics in `premath-tusk` and defer
promotion.

### 6.3 External harness behavior coverage

Crosswalk against external harness/context-transfer patterns:

- conversation event stream + queryable views:
  `ToolRequests/ToolResults/ToolUse` + trajectory projections.
- deterministic safety before action:
  `ToolUse -> JoinClosed -> MutationReady`.
- explicit injection points:
  `toolRenderProtocolDigest` + `reminderQueuePolicyDigest`.
- stop/enforcement from state views:
  `stateViewPolicyDigest` + `protocolStatePolicy`.
- monolith vs parallel decomposition:
  `executionPattern` + `parallelPolicy` + `decompositionPolicyDigest`.
- multi-agent context-transfer packets:
  `handoffContractDigest` with required artifacts and return path.

Current executable status:

- queue/render/state-view/decomposition policies are emitted as typed witness
  rows and covered by adversarial vectors in
  `tests/conformance/fixtures/harness-typestate/`.
- mutation fail-closed enforcement is wired through instruction-linked mutation
  paths and join-gate witness checks.

## 7. Fail-closed classes (minimum set)

- `tool.schema_invalid`
- `tool.unknown_or_disallowed`
- `tool.discovery_mismatch`
- `tool.discovery_unresolved`
- `tool.example_digest_mismatch`
- `tool.caller_not_allowed`
- `tool.result_missing`
- `tool.result_orphan`
- `tool.join_incomplete`
- `tool.use_missing`
- `tool.use_unknown_result`
- `tool.use_without_result`
- `tool.parallel_policy_violation`
- `tool.programmatic_caller_edge_invalid`
- `tool.response_truncation_policy_violation`
- `protocol.stop_reason_unhandled`
- `protocol.parallel_transport_order_invalid`
- `runtime.shell_boundary_violation`
- `runtime.compaction_state_invalid`
- `context.queue_policy_violation`
- `context.injection_point_missing`
- `coordination.decomposition_policy_violation`
- `governance.policy_package_unpinned`
- `governance.policy_package_mismatch`
- `governance.guardrail_stage_missing`
- `governance.guardrail_stage_order_invalid`
- `governance.eval_gate_unmet`
- `governance.eval_lineage_missing`
- `governance.self_evolution_retry_missing`
- `governance.self_evolution_escalation_missing`
- `governance.self_evolution_rollback_missing`
- `governance.redteam_gate_unmet`
- `governance.trace_mode_violation`
- `governance.risk_tier_profile_missing`
- `handoff.required_artifact_missing`
- `handoff.target_not_allowed`
- `handoff.return_path_missing`
- `context.scope_violation`
- `mutation.policy_digest_mismatch`
- `mutation.capability_claim_missing`
- `mutation.use_evidence_missing`

If any class above is present, the harness may write projection artifacts but
MUST NOT perform issue/dependency mutation transitions (including completion
transitions).

## 8. Implementation slices (suggested)

1. Add typed `CallSpec`/`ToolUse`/`JoinClosed` witness rows in harness
   trajectory schema.
2. Add deterministic join checker command for local and CI execution.
3. Bind join checker output into instruction-linked mutation gate.
4. Add conformance vectors for mode-specific closure:
   - code multi-call success/failure,
   - json/text one-call constraint,
   - parallel branch closure mismatch.
5. Add dynamic discovery witness checks:
   - search query/result-set digest bound to loaded tool set.
6. Add programmatic-caller edge checks:
   - every tool call made from code execution has validated caller provenance.
7. Add execution-pattern guardrails:
   - enforce max-iteration stop policy and checkpoint requirements.
8. Add architecture escalation guardrails:
   - default to workflow patterns and require explicit measurable trigger for
     autonomous agent mode.
9. Add workload-bound eval reporting:
   - require representative accuracy + token-cost measurements before changing
     default tool-loading/filtering policy.
10. Add transport-conformance checks:
    - verify parallel call/output ordering and truncation marker policy.
11. Add runtime-boundary attestations:
    - require shell `workdir` + sandbox profile evidence on command results.
12. Add continuity checks for non-linear progression:
    - default: validate descent handoff artifacts before resuming turns.
    - fallback: if compaction is used, validate compaction artifacts before
      resuming turns.
13. Add staged-guardrail witness checks:
    - enforce pre-flight/input/output stage presence and ordering.
14. Add policy-provenance checks:
    - require pinned governance package/config digest on runs.
15. Add governance regression gates:
    - require eval thresholds and adversarial test gates before policy
      promotion.
16. Add observability-mode checks:
    - enforce ZDR-compatible tracing mode and processor requirements where
      configured.
17. Add handoff-contract conformance vectors:
    - orchestrator fan-out/fan-in with required artifact gating and blocked
      transfer when artifacts are missing.
18. Add role-scope conformance vectors:
    - specialist attempts to read/write outside allowed context scope fail
      closed.

## 9. Related docs

- `docs/design/TUSK-HARNESS-CONTRACT.md`
- `docs/design/TUSK-HARNESS-MULTITHREAD-RUNBOOK.md`
- `docs/design/MEMORY-LANES-CONTRACT.md`
- `specs/premath/draft/LLM-INSTRUCTION-DOCTRINE.md`
- `specs/premath/draft/UNIFICATION-DOCTRINE.md`
