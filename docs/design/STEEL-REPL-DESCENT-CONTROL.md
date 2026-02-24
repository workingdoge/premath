# Steel REPL Descent Control Surface

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Define how a Scheme/Steel REPL becomes the primary agent-facing control surface
without creating a parallel authority lane.

Target outcome:

- fewer transport-level tool calls per worker step,
- richer local planning/state inside one evaluator session,
- unchanged semantic and mutation authority boundaries.

## 2. Boundary Rule

REPL is orchestration and local reasoning, not authority.

Authority remains:

1. semantic admissibility: kernel/Gate/descent contracts,
2. mutation admissibility: instruction-linked, capability-scoped host calls,
3. evidence authority: typed witnesses and deterministic failure classes.

Any REPL design that bypasses host gating is out of scope.

## 3. Layer Placement

Runtime placement:

- `Harness` supervises session lifecycle (`boot/step/stop`),
- `Steel VM` executes bounded worker programs,
- `Host API` (Rust) is the only effect boundary,
- `premath-*` crates remain semantic/control implementation authority.

Transport placement:

- internal workers MAY call REPL directly,
- external clients MAY use MCP as a thin transport wrapper (for example one
  `scheme_eval` tool).

## 4. Sheaf/Descent Shape

Each solution run is represented as a descent object.

Base:

- global context `Γ`: `{issue, repo_ref, policy_digest, required_checks}`.

Cover:

- decomposition `{Γ_i -> Γ}` from claimed issues/workers/worktrees.

Local sections:

- `s_i`: bounded outputs per cover leg:
  `{patch_ref, witness_refs, verification_refs, lineage_refs}`.

Overlap restrictions:

- compatibility checks on pullbacks:
  - `harness-join-check`,
  - `dep diagnostics --graph-scope active`,
  - required/coherence witness verification.

Glue:

- parent transition (`issue.update --status closed` / promote) only when all
  local sections are
  mutually compatible and accepted.

Obstruction:

- typed failures instead of prose-only status:
  `tool.join_incomplete`, policy/capability mismatch, lease contention/staleness,
  unmet governance/coherence gates.

## 5. Host API v0 (Capability-Scoped)

REPL calls host functions; host functions call existing premath authority
surfaces.

Read/query family:

- `issue.ready`, `issue.blocked`, `issue.list`, `issue.check`,
- `issue.backend_status`,
- `dep.diagnostics`,
- `observe.latest`, `observe.needs_attention`, `observe.instruction`,
  `observe.projection`.

Mutation family (instruction-linked):

- `issue.claim`, `issue.lease_renew`, `issue.lease_release`,
- `issue.update`, `issue.discover`,
- `dep.add`, `dep.remove`, `dep.replace`.

Control/doctrine family:

- `instruction.check`, `instruction.run`,
- `coherence.check`,
- `required.projection`, `required.delta`, `required.witness`,
  `required.witness_verify`, `required.witness_decide`,
  `required.decision_verify`, `required.gate_ref`.

Harness durability family:

- `harness.session.read`, `harness.session.write`, `harness.session.bootstrap`,
- `harness.feature.read`, `harness.feature.write`, `harness.feature.check`,
  `harness.feature.next`,
- `harness.trajectory.append`, `harness.trajectory.query`.

### 5.1 Exact command/tool mapping (host id -> CLI/MCP)

| Host function id | Canonical CLI surface | MCP tool |
|---|---|---|
| `issue.ready` | `premath issue ready --issues <path> --json` | `issue_ready` |
| `issue.list` | `premath issue list --issues <path> --json` | `issue_list` |
| `issue.blocked` | `premath issue blocked --issues <path> --json` | `issue_blocked` |
| `issue.check` | `premath issue check --issues <path> --json` | `issue_check` |
| `issue.backend_status` | `premath issue backend-status --issues <path> --repo <repo> --projection <path> --json` | `issue_backend_status` |
| `issue.claim` | `premath issue claim <issue-id> --assignee <name> --issues <path> --json` | `issue_claim` |
| `issue.lease_renew` | n/a (CLI surface pending) | `issue_lease_renew` |
| `issue.lease_release` | n/a (CLI surface pending) | `issue_lease_release` |
| `issue.update` | `premath issue update <issue-id> --status <status> --issues <path> --json` | `issue_update` |
| `issue.discover` | `premath issue discover <parent-issue-id> <title> --issues <path> --json` | `issue_discover` |
| `dep.add` | `premath dep add <issue-id> <depends-on-id> --type <dep-type> --issues <path> --json` | `dep_add` |
| `dep.remove` | `premath dep remove <issue-id> <depends-on-id> --type <dep-type> --issues <path> --json` | `dep_remove` |
| `dep.replace` | `premath dep replace <issue-id> <depends-on-id> --from-type <dep-type> --to-type <dep-type> --issues <path> --json` | `dep_replace` |
| `dep.diagnostics` | `premath dep diagnostics --issues <path> --graph-scope active|full --json` | `dep_diagnostics` |
| `observe.latest` | `premath observe --surface <path> --mode latest --json` | `observe_latest` |
| `observe.needs_attention` | `premath observe --surface <path> --mode needs_attention --json` | `observe_needs_attention` |
| `observe.instruction` | `premath observe --surface <path> --mode instruction --instruction-id <id> --json` | `observe_instruction` |
| `observe.projection` | `premath observe --surface <path> --mode projection --projection-digest <digest> --json` | `observe_projection` |
| `instruction.check` | `premath instruction-check --instruction <path> --repo-root <repo> --json` | `instruction_check` |
| `instruction.run` | `mise run ci-pipeline-instruction` or `premath mcp-serve` instruction runner path | `instruction_run` |
| `coherence.check` | `premath coherence-check --contract <path> --repo-root <repo> --json` | n/a |
| `required.projection` | `premath required-projection --input <path> --json` | n/a |
| `required.delta` | `premath required-delta --input <path> --json` | n/a |
| `required.gate_ref` | `premath required-gate-ref --input <path> --json` | n/a |
| `required.witness` | `premath required-witness --runtime <path> --json` | n/a |
| `required.witness_verify` | `premath required-witness-verify --input <path> --json` | n/a |
| `required.witness_decide` | `premath required-witness-decide --input <path> --json` | n/a |
| `required.decision_verify` | `premath required-decision-verify --input <path> --json` | n/a |
| `harness.session.read` | `premath harness-session read --path <path> --json` | n/a |
| `harness.session.write` | `premath harness-session write --path <path> ... --json` | n/a |
| `harness.session.bootstrap` | `premath harness-session bootstrap --path <path> --feature-ledger <path> --json` | n/a |
| `harness.feature.read` | `premath harness-feature read --path <path> --json` | n/a |
| `harness.feature.write` | `premath harness-feature write --path <path> ... --json` | n/a |
| `harness.feature.check` | `premath harness-feature check --path <path> [--require-closure] --json` | n/a |
| `harness.feature.next` | `premath harness-feature next --path <path> --json` | n/a |
| `harness.trajectory.append` | `premath harness-trajectory append --path <path> ... --json` | n/a |
| `harness.trajectory.query` | `premath harness-trajectory query --path <path> --mode latest|failed|retry-needed --limit <n> --json` | n/a |

## 6. Deterministic Effect Row Contract

Every host call from REPL emits one host-effect envelope and binds it to one
canonical trajectory row.

Canonical host-effect envelope shape:

- `schema: "premath.host_effect.v0"`
- `action`
- `argsDigest`
- `resultClass`
- `payload` (action-specific typed JSON)
- `failureClasses[]`
- `witnessRefs[]`
- `policyDigest` (required for mutation-capable actions)
- `instructionRef` (required for mutation-capable actions)

Storage:

- bind each envelope to canonical harness trajectory storage only:
  `.premath/harness_trajectory.jsonl` rows of kind `premath.harness.step.v1`,
- never treated as semantic authority by itself.

## 7. Harness Integration (Ralph-Compatible)

Ralph loop alignment:

1. fresh context,
2. one task per loop,
3. backpressure before progression,
4. explicit update/handoff.

Mapped worker step:

1. `boot`: `dep diagnostics(active)` + `issue_ready` + `harness-session bootstrap`.
2. `claim`: obtain one bounded issue lease.
3. `eval`: run one bounded Steel program against host API.
4. `verify`: run required checks and witness verification.
5. `record`: append trajectory/evidence refs.
6. `transition`: `issue.update --status closed` or `issue.discover`, then lease
   renew/release.
7. `stop`: write compact handoff artifact.

Continuity rule:

- default continuity is descent artifacts (`session`, `trajectory`, witness refs),
- transcript compaction is optional fallback only.

## 8. Runtime Guardrails

Default REPL profile MUST:

1. deny direct shell/network effects by default,
2. enforce step budgets (time/fuel/memory),
3. require host API for all persistent mutation,
4. fail closed on missing instruction/policy/capability evidence,
5. produce replay-stable output envelopes and failure classes.

## 9. Transport Options

Option A: REPL-first local harness

- worker loop invokes Steel directly,
- no MCP in internal execution path.

Option B: thin MCP wrapper (recommended for interoperability)

- expose one high-bandwidth evaluator tool (`scheme_eval`) plus optional
  bootstrap/read tools,
- keep doctrine/operation mapping explicit and parity-checked.

Selection rule:

- use Option A for tightly controlled internal worker fleets,
- use Option B where external agent client compatibility is required.

## 10. Migration Plan

Phase 0: read-only evaluator

- ship read/query host API only,
- validate deterministic effect-row emission.

Phase 1: mutation parity

- add instruction-linked mutation host calls,
- prove parity with existing CLI/MCP mutation witnesses/failure classes.

Phase 2: harness adoption

- wire worker `step` to evaluator path,
- preserve existing `verify`/coherence closure gates.

Phase 3: transport simplification

- collapse multi-tool transport surfaces to thin compatibility wrappers where
  needed,
- keep one authority lane and deterministic evidence lineage.

## 11. Acceptance Signals

Direction is considered stable when:

1. coherence remains accepted under new control surface,
2. mutation witness/failure-class parity vectors pass,
3. multithread loop KPIs are not worse than baseline for equivalent work,
4. restartability works from persisted session/trajectory artifacts alone.
