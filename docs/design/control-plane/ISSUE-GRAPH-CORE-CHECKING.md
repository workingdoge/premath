# Issue Graph Core Checking (Design)

Status: implemented
Scope: design-level, non-normative

## 1. Why

Issue-graph checks now run through native `premath-bd` semantics exposed by the
Premath CLI. CI wrappers may call the CLI, but they do not define independent
issue-graph meaning.

Target principle:

- core owns issue-memory semantics,
- CLI/MCP expose core semantics,
- CI executes core semantics as a wrapper (no parallel checker meaning).

## 2. Goal

Move issue-graph invariants to `premath-bd` with one canonical check surface.

Minimum first-class scope:

- typed `issue_type` with initial enum: `epic | task`,
- deterministic invariant checks:
  - `[EPIC]` title implies `issue_type=epic`,
  - active issues (`open | in_progress`) include acceptance criteria,
  - active issues include at least one verification command.

Warning-only scope:

- oversized `notes` payloads (churn/drift budget signal, not admission failure).

## 3. Authority Split

Core (`premath-bd`, `premath-cli`):

- parse + validate issue graph invariants,
- emit deterministic check result + classes + warnings.

Wrappers (`tools/ci/*`):

- call core check command through `sh tools/ci/run_task.sh ci-hygiene-check`,
- present output in CI logs,
- must not add independent issue-graph semantics.

## 4. Proposed Surfaces

Core/CLI command:

- `premath issue check --issues <path> --json`

Output shape (proposed):

- `checkKind`: `premath.issue_graph.check.v1`
- `result`: `accepted | rejected`
- `failureClasses`: deterministic list (empty if accepted)
- `warnings`: deterministic list
- `summary`: counts for scanned issues/errors/warnings

MCP surface (optional but preferred):

- tool: `issue_check`
- same core-backed result contract as CLI.

## 5. Failure Class Vocabulary (v0)

Hard-fail classes:

- `issue_graph.issue_type.epic_mismatch`
- `issue_graph.acceptance.missing`
- `issue_graph.verification_command.missing`
- `issue_graph.compactness.closed_block_edge`
- `issue_graph.compactness.transitive_block_edge`

Warning classes:

- `issue_graph.notes.large`

## 6. Migration Result

1. Core checker lives in `premath-bd` with deterministic tests.
2. CLI command is `premath issue check --issues <path> --json`.
3. MCP wrapper (`issue_check`) exposes the same core-backed result contract.
4. Python issue-graph wrappers were removed.
5. CI gate remains `sh tools/ci/run_task.sh ci-hygiene-check`, backed by native
   repository hygiene and issue-graph checks.

## 7. Non-Goals (for this slice)

- broad workflow taxonomy expansion beyond `epic | task`,
- rewriting historical issue notes immediately,
- changing dependency semantics (`blocks`, `discovered-from`, etc.).

## 8. Follow-on

- note compaction policy for oversized closed-issue notes (`bd-129`),
- optional richer issue types once required by executable workflows.
