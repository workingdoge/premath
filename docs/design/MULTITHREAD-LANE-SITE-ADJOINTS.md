# Multithread Lane/Site/Adjoint Contract

Status: draft
Scope: design-level, non-normative

## 1. Purpose

Define one canonical orchestration contract for concurrent workers that stays
aligned with lane ownership, site/descent semantics, and optional SigPi/Squeak
capability overlays.

Normative authority remains in:

- `specs/premath/draft/UNIFICATION-DOCTRINE.md` (§9),
- `specs/premath/raw/CTX-SITE.md`,
- `specs/premath/raw/SHEAF-STACK.md`,
- `specs/premath/profile/ADJOINTS-AND-SITES.md`,
- `specs/premath/draft/CHANGE-MORPHISMS.md`.

## 2. Lane Ownership (No Parallel Authority)

Worker orchestration MUST preserve this lane split:

1. semantic doctrine lane (`PREMATH-KERNEL`, `GATE`, `BIDIR-DESCENT`):
   admissibility authority only.
2. strict checker lane (`PREMATH-COHERENCE`): typed contract parity and
   fail-closed checker obligations only.
3. witness commutation lane (`SPAN-SQUARE-CHECKING`, CI witness projections):
   commutation/provenance evidence only.
4. runtime transport lane (`TUSK-CORE`, `SQUEAK-CORE`, `SQUEAK-SITE` when
   claimed): execution substrate and placement only.

Operational rule: a worker MAY propose in any lane, but acceptance MUST resolve
through the semantic/checker authority path, never by wrapper or harness logic.

## 3. Site-Based Worker Orchestration

Use site terms for decomposition:

- base object: work context `Gamma` (`Ctx` object),
- refinement morphism: `rho: Gamma' -> Gamma` (narrower task slice),
- cover family: `{rho_i: Gamma_i -> Gamma}` (parallelizable subtasks).

Concurrent execution policy:

1. coordinator selects admissible cover(s) from issue-ready frontier;
2. each worker takes one refinement context only;
3. workers emit local witnesses on `Gamma_i`;
4. coordinator performs glue-or-witness at overlaps (`Gamma_i x_Gamma Gamma_j`);
5. merge is accepted only when descent obligations are discharged or explicit
   obstruction witnesses are recorded.

This keeps concurrency as structured base-change, not ad hoc branch semantics.

## 4. Optional Capability Composition Boundaries

Capability overlays remain additive and explicit:

- `capabilities.change_morphisms`: issue-memory mutation discipline and typed
  morphism witnesses.
- `capabilities.adjoints_sites`: SigPi pullback/base-change obligations with
  admissible-map policy.
- `capabilities.squeak_site`: runtime location/site placement constraints.

Composed systems MUST route cross-lane pullback/base-change claims through typed
span/square witnesses and keep one authority artifact per boundary.

## 5. Execution Order (Architecture-First)

For multithread features, use this default order:

1. lane+site architecture contract update,
2. spec index/doctrine-site glue update,
3. control-plane typed contract + checker parity update,
4. implementation update,
5. conformance vectors and closure gates.

Issue graph discipline:

- encode the order with explicit dependency edges,
- keep one in-progress issue per worker session,
- record discovered work immediately with dependency linkage.

Operational entrypoint for this loop:

- `docs/design/TUSK-HARNESS-MULTITHREAD-RUNBOOK.md`
- `tools/harness/multithread_loop.py`
