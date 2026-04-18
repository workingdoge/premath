# premath-specs

Draft, spec-only repository for the Premath line developed in this conversation.

The central move is to treat identity as a **profile** over a more universal substrate:
**Admissible Simplex Presentations (ASP)**.

This repo contains **only specs**:
- kernel-neutral dependent-type pseudocode
- data signatures
- derived definitions
- search / obstruction interfaces
- runtime snapshot semantics for JWT + JWKS

It does **not** contain:
- executable parser code
- crypto implementations
- HTTP clients
- proof scripts in a concrete prover
- production package wiring

## Reading order

1. `specs/raw/ASP-IDENT-OVERVIEW.md`
2. `specs/raw/COVER-TERMINALITY.md`
3. `specs/raw/COVER-TERMINALITY-FIXED-POINT.md`
4. `specs/raw/COVER-TERMINALITY-LIMIT.md`
5. `specs/raw/COVER-TERMINALITY-ATTRACTOR.md`
6. `specs/raw/COVER-TERMINALITY-BASIN-OF-ATTRACTION.md`
7. `specs/raw/asp/ASP0.md`
8. `specs/raw/ident/IDENT0.md`
9. `specs/raw/ident/JWT0.md`
10. `specs/raw/ident/JWT0-OBS.md`
11. `specs/raw/ident/JWT0-SEARCH.md`
12. `specs/raw/ident/JWT1-JWKS0.md`

## Module graph

```text
PREMATH.COVER.TERMINALITY
  └─ PREMATH.COVER.TERMINALITY.FIXED_POINT
       └─ PREMATH.COVER.TERMINALITY.LIMIT
            └─ PREMATH.COVER.TERMINALITY.ATTRACTOR
                 └─ PREMATH.COVER.TERMINALITY.BASIN_OF_ATTRACTION
PREMATH.ASP0
  └─ PREMATH.IDENT0
       └─ PREMATH.IDENT.JWT0
            ├─ PREMATH.IDENT.JWT0.OBS
            ├─ PREMATH.IDENT.JWT0.SEARCH
            └─ PREMATH.IDENT.JWT1.JWKS0
```

## Repo layout

```text
premath-specs/
├── README.md
├── premath-specs.toml
└── specs/raw/
    ├── ASP-IDENT-OVERVIEW.md
    ├── COVER-TERMINALITY.md
    ├── COVER-TERMINALITY-FIXED-POINT.md
    ├── COVER-TERMINALITY-LIMIT.md
    ├── COVER-TERMINALITY-ATTRACTOR.md
    ├── COVER-TERMINALITY-BASIN-OF-ATTRACTION.md
    ├── asp/
    │   └── ASP0.md
    └── ident/
        ├── IDENT0.md
        ├── JWT0.md
        ├── JWT0-OBS.md
        ├── JWT0-SEARCH.md
        └── JWT1-JWKS0.md
```

## Conventions

- `U`, `U1` are universes.
- `Σ` and `Π` are dependent sum and dependent product.
- `Dec P` is a decision procedure returning either a witness of `P` or a witness of `Not P`.
- Equalities and congruence proofs are written in compact pseudocode; routine proof terms are usually omitted.
- `JWT0` intentionally keeps credential equivalence **strict**.
- `JWT1-JWKS0` is **snapshot-relative**: checking never fetches; network refresh is modeled as snapshot transition.

## Main idea in one line

```text
admitted simplex = presentation + witness
```

In symbols:

```text
Adm σ := Σ (p : Pres σ), Wit σ p
```

## Scope of this draft

This is a coherent spec set for:
- the universal ASP substrate
- an identity profile over ASP
- a JWT backend
- typed failure / obstruction families
- horn-like fillers for issuer, credential, and scope
- a JWKS snapshot layer with cache and rotation doctrine

## Next natural extensions

- revocation doctrine beyond simple JTI sets
- DID / VC backend
- X.509 / SPIFFE backend
- concrete proof assistant transcription
- higher obstruction profiles for multi-domain coherence
