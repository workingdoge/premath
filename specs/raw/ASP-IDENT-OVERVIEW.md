# Overview

## Universal substrate

The base object is not “identity” as such. The universal layer is:

> **Admissible Simplex Presentations**
>
> a chromatic semi-simplicial presentation system with explicit witnesses.

The core decomposition is:

1. **shape theory**  
   colored simplex shapes and restriction along injective vertex maps

2. **presentation theory**  
   local presentations over shapes

3. **equivalence / transport**  
   invertible re-presentation of the same local content

4. **witness theory**  
   admissibility evidence over a presentation

5. **admitted simplex**  
   a presentation paired with witness

In symbols:

```text
Adm σ := Σ (p : Pres σ), Wit σ p
```

## Why the substrate is semisimplicial

The implementation form chosen here is:

> restrict along **all injective monotone maps**

rather than using only face maps plus separate simplicial identities.

That makes semisimplicial coherence a matter of ordinary functoriality.

## Identity as a profile

Identity is represented by a master role simplex:

```text
[issuer, subject, credential, scope, epoch]
```

Every valid identity shape is a face of that simplex.

The identity profile contributes:
- role colors
- a semantic environment `IEnv`
- normalized presentations
- witness clauses for trust, binding, grants, freshness, and revocation

## JWT specialization

The JWT profile interprets the role simplex as:

- `issuer`      — issuer handle
- `subject`     — issuer-scoped subject handle
- `credential`  — JWT bearer token
- `scope`       — requested audience/scope context
- `epoch`       — evaluation time

An admitted full simplex states:

> this token, at this time, from this issuer, names this subject, and covers this authorization context.

## Obstructions and fillers

The same object gives:
- validation of fully specified identities
- typed obstructions when validation fails
- horn-like fillers for missing roles

In this repo the main runtime fillers are:

- missing issuer — recover from token `iss`
- missing credential — search wallet / candidate list
- missing scope — narrow from policy list or clip against grants
- missing issuer + scope — recover and narrow in one pass

## Snapshot-relative JWKS layer

`JWT1-JWKS0` turns the abstract JWT interface into a concrete trust model:

- issuer configuration
- OIDC discovery metadata cache
- JWKS cache
- revocation cache
- key selection
- refresh semantics
- rotation discipline

The checker itself remains pure:

> **checking reads a snapshot; refresh builds a new snapshot**

## Dependency summary

```text
ASP0
  -> IDENT0
    -> JWT0
      -> JWT0-OBS
      -> JWT0-SEARCH
      -> JWT1-JWKS0
```

## Reading lens

You can read the specs from two directions.

From the top down:

```text
universal substrate -> identity profile -> JWT runtime profile
```

From the bottom up:

```text
snapshots -> key selection -> signature witness -> admitted principal
```

Both routes end at the same central object:

```text
admitted simplex = local presentation + admissibility witness
```
