# Ralph Playbook in Premath

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Adopt the useful execution loop from Ralph while preserving Premath control and
authority boundaries.

Source:

- `https://claytonfarr.github.io/ralph-playbook/`

Premath interpretation:

- keep Ralph's fresh-context, one-task, backpressure loop,
- replace file-only planning authority with typed issue/dependency authority,
- require witness-linked mutation gating before state changes.
- prefer descent decomposition over transcript compaction in default operation.

## 2. Direct Crosswalk

Ralph concept -> Premath surface:

1. `IMPLEMENTATION_PLAN.md` task queue
   -> `.premath/issues.jsonl` + `issue_ready`/dependency graph.
2. "one task per loop"
   -> one claimed issue per worker session (`issue_claim`, lease discipline).
3. "fresh context each iteration"
   -> `harness-session bootstrap` (`attach|resume`) + bounded step loops.
4. "backpressure via tests/build"
   -> issue verification commands + `mise` gates (`baseline`, conformance,
      coherence, docs coherence).
5. "update plan + commit each loop"
   -> update issue status/notes + append trajectory/witness refs, then commit.
6. "subagents for focused work"
   -> typed orchestration/handoff contracts (`executionPattern`,
      `handoffContractDigest`, role-scoped context).

## 3. What Changes vs Vanilla Ralph

### 3.1 Authority

- Ralph default: plan markdown is operational authority.
- Premath: issue graph is authority; projection docs are views only.

### 3.2 Safety

- Ralph often assumes skip-permissions operation in an isolated sandbox.
- Premath defaults to mutation policy `instruction-linked` and fail-closed
  mutation gates.

### 3.3 Closure Criteria

- Ralph loop ends when one task passes backpressure and commits.
- Premath loop closes when acceptance + verification + witness-linked mutation
  criteria are satisfied and dependency graph remains coherent.

### 3.4 Flow Shape (linear vs descent)

- Many coding-agent loops are transcript-linear and rely on periodic context
  compaction to continue.
- Premath defaults to descent:
  - split work into bounded local slices (one issue/session at a time),
  - carry typed artifacts/witness refs across handoff boundaries,
  - glue via deterministic closure checks.
- Compaction is compatibility/fallback policy, not the primary continuity path.

## 4. Canonical Premath Ralph Loop

1. Preflight:
   - `dep_diagnostics(graph_scope=active)`,
   - select via `issue_ready` and priority/dependencies.
2. Claim one issue:
   - `issue_claim` (or `claim-next` path in worker loop).
3. Execute bounded change:
   - implementation within one issue scope.
4. Run backpressure gates:
   - run issue-specific verification commands,
   - run required closure checks for touched surfaces.
5. Record evidence:
   - issue notes with concise refs,
   - harness trajectory/session artifacts as projections,
   - witness refs for mutation lineage.
6. Mutate state deterministically:
   - close/update issue,
   - discover/link follow-up work when needed,
   - release/renew lease based on outcome.

## 5. Recommended Command Surface

- coordinator/worker loop:
  - `mise run harness-coordinator-loop`
  - `mise run harness-worker-loop`
- issue graph integrity:
  - `cargo run --package premath-cli -- issue-graph-check --repo-root . --issues .premath/issues.jsonl --note-warn-threshold 2000`
  - `cargo run --package premath-cli -- dep diagnostics --graph-scope active --json`
- baseline backpressure:
  - `mise run baseline`

## 6. Operational Rule

If a Ralph-style optimization conflicts with authority boundaries, keep:

1. semantic authority in checker/Gate contracts,
2. mutation authority in instruction-linked issue memory,
3. projections as non-authoritative views.

## 7. Operational Descent (Compaction-Avoiding by Default)

Descent reading of the loop:

1. base context: issue frontier + policy state.
2. cover: worker/session decomposition (`issue_ready -> claim`).
3. local sections: bounded implementation + verification outputs.
4. compatibility checks: typestate join closure + dependency integrity +
   policy-digest parity.
5. glue: deterministic close/update/discover transitions.
6. obstructions: fail-closed classes (join incomplete, handoff artifact missing,
   policy mismatch, stale/contended lease, unmet governance gates).

Default continuity strategy:

- restart from typed handoff/session artifacts (`attach|resume`) and witness
  refs,
- avoid linear transcript growth and avoid treating compaction as required for
  progress.

Fallback strategy:

- if compaction is explicitly enabled, compaction outputs must be typed and
  checked before resuming mutation-capable turns.

REPL-oriented execution companion:

- `docs/design/STEEL-REPL-DESCENT-CONTROL.md`

Conformance target:

- add paired golden/adversarial vectors that assert descent-first continuity
  (handoff/session artifacts present) and reject compaction-only continuity when
  required handoff evidence is missing.
