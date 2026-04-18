# Premath Specs 0.1.0 Draft Promotion Review

Status: completed (`premath-2sg`)
Scope: review the absorbed `premath-specs` `0.1.0-draft` snapshot for adoption
into `specs/premath/draft/`
Authority: non-normative review artifact (no semantic delta)

## 1. Review Input

Absorbed raw snapshot under review:

- `specs/raw/ASP-IDENT-OVERVIEW.md`
- `specs/raw/asp/ASP0.md`
- `specs/raw/ident/IDENT0.md`
- `specs/raw/ident/JWT0.md`
- `specs/raw/ident/JWT0-OBS.md`
- `specs/raw/ident/JWT0-SEARCH.md`
- `specs/raw/ident/JWT1-JWKS0.md`
- `specs/raw/premath-specs.toml`

Promotion bar used for this review:

- `specs/premath/draft/README.md`
- `specs/premath/draft/SPEC-INDEX.md`
- `specs/premath/draft/CONFORMANCE.md`
- `specs/premath/draft/SPEC-TRACEABILITY.md`
- `specs/process/coss.md`
- `specs/process/HARNESS-SPEC-PROMOTION-MAP.md`
- `specs/process/decision-log.md` Decision 0038

## 2. Review Outcome

Decision: do not promote the `premath-specs` `0.1.0-draft` pack into
`specs/premath/draft/` in its current absorbed form.

Disposition:

- retain the absorbed pack under `specs/raw/`
- treat `specs/raw/premath-specs.toml` as absorbed snapshot metadata, not as an
  active in-repo package manifest
- use follow-up promotion-prep issues to integrate claim scope, traceability,
  and executable coverage before any draft adoption

This is a quality/integration hold, not a rejection of the ideas. The pack is
coherent and substantial, but it is not yet wired into Premath's existing draft
authority model.

## 3. Why Promotion Fails Closed Right Now

### 3.1 No draft claim or profile placement exists yet

Promoted draft specs in this repo are active, claim-bearing contracts. The
current draft surface routes normativity through `SPEC-INDEX`, `CONFORMANCE`,
optional capability claims, and optional profile overlays.

The absorbed ASP/IDENT/JWT pack currently has no such placement:

- no `SPEC-INDEX` claim or profile entry,
- no `CONFORMANCE` claim behavior,
- no `CAPABILITY-REGISTRY` or profile-overlay binding,
- no draft `README` or index integration.

Promotion without that placement would create a new authority subtree under
`draft/` without a declared claim surface.

### 3.2 No traceability row or deterministic executable surface is defined

Promoted draft docs in this repo are expected to land with an explicit
`SPEC-TRACEABILITY` row and a deterministic executable surface.

The absorbed pack currently has:

- no `SPEC-TRACEABILITY` rows,
- no canonical vector suites named from `CONFORMANCE`,
- no deterministic witness/failure mapping surface comparable to other promoted
  draft docs.

This misses the promotion bar documented in the harness promotion map and in
Decision 0038's raw-to-draft policy.

### 3.3 The absorbed tree and manifest are archival, not repo-native

The absorbed pack currently sits under top-level `specs/raw/`, while the active
Premath lifecycle tree uses `specs/premath/raw/` and `specs/premath/draft/`.
That makes the current location a visible snapshot/import surface, not an
already-integrated Premath raw lane.

`specs/raw/premath-specs.toml` still points at the source package layout:

- `entry = "specs/overview.md"`
- module paths under `specs/asp/...` and `specs/ident/...`

Those paths do not exist in the absorbed repo layout, where the files live under
top-level `specs/raw/`. That is acceptable for snapshot provenance, but it is
not a promotion-ready manifest.

### 3.4 The pack introduces a new semantic/domain subtree, not a mechanical draft lift

The draft corpus already has a tight authority spine:

- kernel law
- interop artifacts
- gate / bidirectional descent
- doctrine/coherence/control-plane overlays

The absorbed pack introduces a separate family:

`ASP0 -> IDENT0 -> JWT0 -> JWT0-OBS/JWT0-SEARCH/JWT1-JWKS0`

That family does not currently appear anywhere in the promoted draft authority
graph. Promotion therefore requires semantic placement work first, not just file
copying.

## 4. Module Review Matrix

| Module | Review result | Reason |
| --- | --- | --- |
| `ASP-IDENT-OVERVIEW` | hold in raw | useful orientation note, but not itself a claim-bearing draft surface |
| `ASP0` | hold in raw | introduces a universal admissible-simplex substrate not yet placed against `PREMATH-KERNEL` or current draft claims |
| `IDENT0` | hold in raw | identity profile depends on `ASP0` but has no capability/profile ownership in the current draft system |
| `JWT0` | hold in raw | concrete backend specialization, but no promoted claim, vectors, or failure mapping surface |
| `JWT0-OBS` | hold in raw | obstruction/checker layer needs deterministic failure-class mapping before draft promotion |
| `JWT0-SEARCH` | hold in raw | runtime filler/search behavior needs explicit capability/profile placement and executable vectors |
| `JWT1-JWKS0` | hold in raw | snapshot trust/revalidation model is substantial, but draft promotion needs a claim surface and deterministic refresh/check coverage |

## 5. Recommended Promotion Shape

If this family is promoted later, it should not be dropped into `draft/` as an
unclaimed parallel stack. The likely clean shape is:

1. decide whether ASP/IDENT/JWT is:
   - a new optional capability family, or
   - a new optional profile-overlay family, or
   - a separate retained raw research line that never becomes draft authority
2. define that placement in:
   - `specs/premath/draft/SPEC-INDEX.md`
   - `specs/premath/draft/CONFORMANCE.md`
   - `specs/premath/draft/CAPABILITY-REGISTRY.json` and/or `specs/premath/profile/`
3. choose the promoted target set and names rather than mechanically mirroring
   the absorbed package tree
4. add `SPEC-TRACEABILITY` rows with deterministic executable surfaces
5. only then promote selected documents into `specs/premath/draft/`

## 6. Promotion-Prep Checklist

Before re-review for draft adoption, the promotion-prep lane should close all of
the following:

- [ ] claim/profile placement chosen and recorded in `SPEC-INDEX`
- [ ] `CONFORMANCE` updated with deterministic behavior and vector ownership for
      the claimed surfaces
- [ ] `SPEC-TRACEABILITY` rows defined for every promoted target
- [ ] deterministic witness/failure mappings named for obstruction/search/trust
      layers
- [ ] target lifecycle location chosen (`specs/premath/raw/`, `specs/premath/draft/`,
      and/or `specs/premath/profile/`) instead of relying on the absorbed
      top-level snapshot tree
- [ ] repo-native target filenames and document boundaries chosen
- [ ] absorbed snapshot manifest either rewritten for repo-native use or
      explicitly retained as archival metadata only

## 7. Recommended Follow-Up Queue

Suggested issue order:

1. promotion-prep issue: decide claim/profile placement for the ASP/IDENT/JWT
   family
2. promotion-prep issue: define deterministic vector and failure-mapping plan
3. promotion issue: promote only the selected, integrated target docs with
   `SPEC-INDEX` and `SPEC-TRACEABILITY` updates in the same lane

## 8. Completion Statement

`premath-specs` `0.1.0-draft` is review-worthy but not promotion-ready. The
current absorbed raw snapshot should remain visible as source material while the
claim, traceability, and executable-coverage seams are made explicit.
