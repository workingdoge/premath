# PREMATH.ASP0

## Intent

`ASP0` is the universal substrate.

It specifies **Admissible Simplex Presentations**:
colored simplex shapes, presentations over them, equivalences between presentations,
witnesses over presentations, admitted simplices, and derived boundary / horn / filling
notions.

The key implementation choice is:

> use **restriction along all injective vertex maps**

instead of only elementary face maps.

This turns semisimplicial coherence into functoriality.

## Kernel-neutral signature

```text
module PREMATH.ASP0

universe U U1

-- finite ordinals and injective monotone maps
record Inj (m n : Nat) : U where
  map  : Fin m -> Fin n
  mono : StrictMono map

idInj   : Inj n n
compInj : Inj l m -> Inj m n -> Inj l n

-- skip the i-th vertex: [n] ↪ [n+1]
skip : (n : Nat) -> Fin (n+1) -> Inj n (n+1)


record ASP0 : U1 where

  ------------------------------------------------------------
  -- 1. Chromatic / doctrinal layer
  ------------------------------------------------------------

  Color : U

  -- proof-relevant if desired; mere Prop if desired
  Schema : (n : Nat) -> (Fin (n+1) -> Color) -> U

  pullSchema :
    {m n : Nat} ->
    (u : Inj (m+1) (n+1)) ->
    {chi : Fin (n+1) -> Color} ->
    Schema n chi ->
    Schema m (chi ∘ u.map)

  pullSchema_id :
    {n : Nat} ->
    {chi : Fin (n+1) -> Color} ->
    (s : Schema n chi) ->
    pullSchema idInj s = s

  pullSchema_comp :
    {l m n : Nat} ->
    (v : Inj (l+1) (m+1)) ->
    (u : Inj (m+1) (n+1)) ->
    {chi : Fin (n+1) -> Color} ->
    (s : Schema n chi) ->
    pullSchema (compInj v u) s
      =
    pullSchema v (pullSchema u s)


  ------------------------------------------------------------
  -- 2. Colored simplex shapes
  ------------------------------------------------------------

  Shape : U
  Shape := Σ (n : Nat), Σ (chi : Fin (n+1) -> Color), Schema n chi

  dim : Shape -> Nat
  dim (n, chi, s) := n

  colors : (σ : Shape) -> Fin (dim σ + 1) -> Color
  colors (n, chi, s) := chi

  pullShape :
    {σ : Shape} ->
    {m : Nat} ->
    (u : Inj (m+1) (dim σ + 1)) ->
    Shape

  pullShape {σ = (n, chi, s)} u := (m, chi ∘ u.map, pullSchema u s)


  ------------------------------------------------------------
  -- 3. Presentation layer
  ------------------------------------------------------------

  Pres : Shape -> U

  res :
    {σ : Shape} ->
    {m : Nat} ->
    (u : Inj (m+1) (dim σ + 1)) ->
    Pres σ ->
    Pres (pullShape u)

  res_id :
    {σ : Shape} ->
    (p : Pres σ) ->
    res idInj p = p

  res_comp :
    {σ : Shape} ->
    {l m : Nat} ->
    (v : Inj (l+1) (m+1)) ->
    (u : Inj (m+1) (dim σ + 1)) ->
    (p : Pres σ) ->
    res (compInj v u) p
      =
    res v (res u p)


  ------------------------------------------------------------
  -- 4. Groupoid-ish equivalence layer on presentations
  ------------------------------------------------------------

  Eqv : (σ : Shape) -> Pres σ -> Pres σ -> U

  eqv_refl  : {σ : Shape} -> (p : Pres σ) -> Eqv σ p p
  eqv_sym   : {σ : Shape} -> {p q : Pres σ} -> Eqv σ p q -> Eqv σ q p
  eqv_trans :
    {σ : Shape} ->
    {p q r : Pres σ} ->
    Eqv σ p q -> Eqv σ q r -> Eqv σ p r

  res_eqv :
    {σ : Shape} ->
    {m : Nat} ->
    (u : Inj (m+1) (dim σ + 1)) ->
    {p q : Pres σ} ->
    Eqv σ p q ->
    Eqv (pullShape u) (res u p) (res u q)


  ------------------------------------------------------------
  -- 5. Witness / admission layer
  ------------------------------------------------------------

  Wit : (σ : Shape) -> Pres σ -> U

  WitEq :
    {σ : Shape} ->
    {p : Pres σ} ->
    Wit σ p -> Wit σ p -> U

  witeq_refl  :
    {σ : Shape} ->
    {p : Pres σ} ->
    (w : Wit σ p) ->
    WitEq w w

  witeq_sym   :
    {σ : Shape} ->
    {p : Pres σ} ->
    {w z : Wit σ p} ->
    WitEq w z -> WitEq z w

  witeq_trans :
    {σ : Shape} ->
    {p : Pres σ} ->
    {w x z : Wit σ p} ->
    WitEq w x -> WitEq x z -> WitEq w z

  wit_res :
    {σ : Shape} ->
    {m : Nat} ->
    (u : Inj (m+1) (dim σ + 1)) ->
    {p : Pres σ} ->
    Wit σ p ->
    Wit (pullShape u) (res u p)

  wit_eqv :
    {σ : Shape} ->
    {p q : Pres σ} ->
    (e : Eqv σ p q) ->
    Wit σ p ->
    Wit σ q

  wit_eqv_refl :
    {σ : Shape} ->
    {p : Pres σ} ->
    (w : Wit σ p) ->
    WitEq (wit_eqv (eqv_refl p) w) w

  wit_eqv_comp :
    {σ : Shape} ->
    {p q r : Pres σ} ->
    (e1 : Eqv σ p q) ->
    (e2 : Eqv σ q r) ->
    (w : Wit σ p) ->
    WitEq
      (wit_eqv (eqv_trans e1 e2) w)
      (wit_eqv e2 (wit_eqv e1 w))

  wit_res_comp :
    {σ : Shape} ->
    {l m : Nat} ->
    (v : Inj (l+1) (m+1)) ->
    (u : Inj (m+1) (dim σ + 1)) ->
    {p : Pres σ} ->
    (w : Wit σ p) ->
    WitEq
      (wit_res (compInj v u) w)
      (wit_res v (wit_res u w))

  wit_res_eqv :
    {σ : Shape} ->
    {m : Nat} ->
    (u : Inj (m+1) (dim σ + 1)) ->
    {p q : Pres σ} ->
    (e : Eqv σ p q) ->
    (w : Wit σ p) ->
    WitEq
      (wit_res u (wit_eqv e w))
      (wit_eqv (res_eqv u e) (wit_res u w))


  ------------------------------------------------------------
  -- 6. Admitted simplices
  ------------------------------------------------------------

  Adm : Shape -> U
  Adm σ := Σ (p : Pres σ), Wit σ p

  adm_res :
    {σ : Shape} ->
    {m : Nat} ->
    (u : Inj (m+1) (dim σ + 1)) ->
    Adm σ ->
    Adm (pullShape u)

  adm_res u (p, w) := (res u p, wit_res u w)

  AdmEq :
    {σ : Shape} ->
    Adm σ -> Adm σ -> U

  AdmEq (p, w) (q, z) := Σ (e : Eqv σ p q), WitEq (wit_eqv e w) z
```

## Derived face language

For an `n`-shape `σ`, define the `i`-th face inclusion:

```text
δ_i := skip(n, i) : Inj n (n+1)
```

Then:

```text
∂_i σ := pullShape δ_i
∂_i p := res δ_i p
∂_i a := adm_res δ_i a
```

The usual semisimplicial identities are **derived** from restriction functoriality:

```text
∂_i ∂_j = ∂_{j-1} ∂_i      for i < j
```

## Boundaries, horns, and fillers

Let `σ` have dimension `n`.

A full boundary is:

```text
Boundary(σ)
:=
Σ ((b_i : Adm(∂_i σ)) for i : Fin(n+1)),
  Π (i<j),
    AdmEq( ∂_{j-1} b_i , ∂_i b_j )
```

A `k`-horn is:

```text
Horn_k(σ)
:=
Σ ((h_i : Adm(∂_i σ)) for i ≠ k),
  Π (i<j with i,j ≠ k),
    AdmEq( ∂_{j-1} h_i , ∂_i h_j )
```

A filler for a horn `H` is:

```text
Fill(H)
:=
Σ (a : Adm σ),
  Π (i ≠ k), AdmEq(∂_i a, h_i)
```

## Obstruction interface

The kernel should not hard-code a single obstruction theory.
Instead it should admit profiles:

```text
record ObsTheory (A : ASP0) : U1 where
  Obs :
    {σ : A.Shape} ->
    (k : Fin (A.dim σ + 1)) ->
    Horn_k σ ->
    U

  sound :
    {σ : A.Shape} ->
    {k : Fin (A.dim σ + 1)} ->
    (H : Horn_k σ) ->
    Obs k H ->
    Empty (Fill H)

  complete :    -- optional
    {σ : A.Shape} ->
    {k : Fin (A.dim σ + 1)} ->
    (H : Horn_k σ) ->
    Empty (Fill H) ->
    Obs k H
```

## Optional enrichment to a literal groupoid

`ASP0` uses `Eqv` rather than an explicit morphism type.
If desired, a profile can enrich this with:

```text
Mor : (σ : Shape) -> Pres σ -> Pres σ -> U
id  : ...
comp : ...
inv : ...
resMor : ...
```

and either identify `Eqv` with `Mor` or derive `Eqv` from `Mor`.

## Informal slogan

```text
ASP0 = colored shapes + presentations + witnesses + fillers
```
