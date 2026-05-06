# MH Site Dependency

Status: raw

## Purpose

This note records the placement boundary for Meta-Harness work.

Meta-Harness is a downstream semantic site presented using Premath doctrine. It
is not Premath kernel law. The harness umbrella lives in:

```text
fish/sites/harness
```

The canonical MH object doctrine remains the concrete subline:

```text
fish/sites/mh
```

Premath supplies upstream doctrine for admissibility, descent, gates, receipt
lineage, and scoped claims. It does not own MH-specific objects such as
`HarnessSpec`, `RuntimeClosure`, `EvalPolicy`, `TraceEvent`,
`VisibilityReceipt`, `EvalReceipt`, `Obstruction`, `ImprovementClaim`,
`HarnessMutationIntent`, `AdmissionDecision`, or `UnreplayableReceipt`.

## Boundary

- `fish/sites/mh` owns Meta-Harness semantic/domain law.
- `fish/sites/harness` owns umbrella harness placement, namespace migration,
  and realization-boundary doctrine.
- `fish/sites/premath` owns generic definability, admissibility, descent, gate,
  and evidence doctrine used by MH.
- `kurma` should carry MH into schemas, content addressing, validators,
  mediated archive APIs, and trace coherence checks.
- `tusk` should operate around carried MH artifacts through receipt-first eval
  lanes, candidate registries, and archive projection.

## Provenance

The MH draft stack was first crystallized in Premath under the temporary
`PMH-*` name, then moved to `fish/sites/mh` to keep Premath generic. The later
`fish/sites/harness` umbrella records broader harness placement without
renaming the concrete `mh` line.
