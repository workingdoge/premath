# Premath KCIR Profile Adapter

This directory records Premath's adapter boundary to the KCIR site.

KCIR substrate authority lives in:

- `fish/sites/kcir/specs/draft/KCIR-0001-SUBSTRATE.md`
- `fish/sites/kcir/specs/draft/KCIR-0002-PREMATH-INTEROP-PROFILE.md`

Premath does not own KCIR as an implementation surface. Premath owns only the
admissibility-facing acceptance rule:

```text
KCIR-carried artifact + declared Premath claim
  -> Premath checker/profile claim
  -> deterministic accepted/rejected Gate result + witness/replay receipt
```

## Authority Split

| Surface | Owner |
| --- | --- |
| Premath admissibility law, Gate classes, witness/replay sufficiency | `fish/sites/premath` |
| Generic artifact substrate, refs, rows, nodes, dependency vocabulary, carrier profiles | `fish/sites/kcir` |
| Executable lowering, builders, stores, codecs, normalizers, receipts | `/Users/arj/irai/kurma` |
| Operator workflow/projection over stable refs | `tusk` |
| Theory-specific `ObjNF`/`MorNF` meaning | source theory site, for example `fish/sites/nerve` |

## Transitional Premath Files

The following files currently remain under `specs/premath/draft/` for path
stability and existing checker tooling:

- `KCIR-CORE.md`
- `REF-BINDING.md`
- `NF.md`
- `NORMALIZER.md`
- `WIRE-FORMATS.md`
- `ERROR-CODES.md`

They are now interpreted as the Premath KCIR adapter/profile surface, not as a
claim that Premath owns the KCIR carrier site. Physical migration should happen
only after the KCIR site has equivalent draft coverage and traceability hooks.

## Claim Rule

An implementation claiming `premath.interop-core.v0` or
`premath.interop-full.v0` must satisfy:

1. the relevant KCIR site substrate/profile requirements;
2. the transitional Premath draft adapter requirements while they remain the
   active checker fixtures;
3. Premath Core verdict and Gate failure-class invariance for fixed semantic
   inputs.

KCIR compatibility alone is not Premath acceptance. KCIR can carry artifacts;
Premath decides whether a carried artifact is acceptable for a Premath claim.
