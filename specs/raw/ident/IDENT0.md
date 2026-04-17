# PREMATH.IDENT0

## Intent

`IDENT0` is the first concrete profile over `ASP0`.

It uses the ordered master role simplex:

```text
[issuer, subject, credential, scope, epoch]
```

Every valid identity shape is a face of that simplex.

The profile separates:

- raw presentations
- normalization-based equivalence
- witnesses for trust, binding, grants, freshness, revocation
- admitted full identities

## Role colors

```text
module PREMATH.IDENT0
import PREMATH.ASP0

universe U U1

data IColor : U where
  issuer
  subject
  credential
  scope
  epoch

role5 : Fin 5 -> IColor
role5 0 = issuer
role5 1 = subject
role5 2 = credential
role5 3 = scope
role5 4 = epoch
```

## Semantic environment

The profile is parameterized by a backend environment.

```text
record IEnv : U1 where
  IssuerP SubjectP CredentialP ScopeP EpochP : U

  IssuerNF SubjectNF CredentialNF ScopeNF EpochNF : U

  nfIssuer     : IssuerP     -> IssuerNF
  nfSubject    : SubjectP    -> SubjectNF
  nfCredential : CredentialP -> CredentialNF
  nfScope      : ScopeP      -> ScopeNF
  nfEpoch      : EpochP      -> EpochNF

  IssuerTrusted   : IssuerP     -> U
  SubjectValid    : SubjectP    -> U
  CredentialValid : CredentialP -> U
  NotRevoked      : CredentialP -> U
  ScopeValid      : ScopeP      -> U
  EpochValid      : EpochP      -> U

  SignedBy : CredentialP -> IssuerP  -> U
  Names    : CredentialP -> SubjectP -> U
  Grants   : CredentialP -> ScopeP   -> U
  FreshAt  : CredentialP -> EpochP   -> U

  IssuerTrusted_cong :
    {x y : IssuerP} ->
    nfIssuer x = nfIssuer y ->
    IssuerTrusted x -> IssuerTrusted y

  SubjectValid_cong :
    {x y : SubjectP} ->
    nfSubject x = nfSubject y ->
    SubjectValid x -> SubjectValid y

  CredentialValid_cong :
    {x y : CredentialP} ->
    nfCredential x = nfCredential y ->
    CredentialValid x -> CredentialValid y

  NotRevoked_cong :
    {x y : CredentialP} ->
    nfCredential x = nfCredential y ->
    NotRevoked x -> NotRevoked y

  ScopeValid_cong :
    {x y : ScopeP} ->
    nfScope x = nfScope y ->
    ScopeValid x -> ScopeValid y

  EpochValid_cong :
    {x y : EpochP} ->
    nfEpoch x = nfEpoch y ->
    EpochValid x -> EpochValid y

  SignedBy_cong :
    {c c' : CredentialP} ->
    {i i' : IssuerP} ->
    nfCredential c = nfCredential c' ->
    nfIssuer i = nfIssuer i' ->
    SignedBy c i -> SignedBy c' i'

  Names_cong :
    {c c' : CredentialP} ->
    {s s' : SubjectP} ->
    nfCredential c = nfCredential c' ->
    nfSubject s = nfSubject s' ->
    Names c s -> Names c' s'

  Grants_cong :
    {c c' : CredentialP} ->
    {r r' : ScopeP} ->
    nfCredential c = nfCredential c' ->
    nfScope r = nfScope r' ->
    Grants c r -> Grants c' r'

  FreshAt_cong :
    {c c' : CredentialP} ->
    {t t' : EpochP} ->
    nfCredential c = nfCredential c' ->
    nfEpoch t = nfEpoch t' ->
    FreshAt c t -> FreshAt c' t'
```

## ASP profile

```text
def IDENT0 (Γ : IEnv) : ASP0 where

  ------------------------------------------------------------
  -- 1. Colors and schemas
  ------------------------------------------------------------

  Color := IColor

  -- Every valid identity shape is a face of role5.
  Schema :
    (n : Nat) -> (Fin (n+1) -> Color) -> U
  Schema n chi :=
    Σ (u : Inj (n+1) 5), chi = (role5 ∘ u.map)

  pullSchema :
    {m n : Nat} ->
    (v : Inj (m+1) (n+1)) ->
    {chi : Fin (n+1) -> Color} ->
    Schema n chi ->
    Schema m (chi ∘ v.map)
  pullSchema v (u, e) :=
    (compInj v u, by rewrite [e])

  pullSchema_id   := by routine
  pullSchema_comp := by routine
```

`IDENT0` inherits `Shape` from `ASP0`.

## Role presence

Because every allowed shape is a face of `role5`, each role occurs at most once.

```text
Has : Shape -> Color -> U
Has σ c := Σ (i : Fin (dim σ + 1)), colors σ i = c
```

For any fixed `σ` and `c`, `Has σ c` is empty or contractible.

## Presentations

A presentation assigns values only to roles present in the shape.

```text
record PresI (σ : Shape) : U where
  issuerAt     : Has σ issuer     -> Γ.IssuerP
  subjectAt    : Has σ subject    -> Γ.SubjectP
  credentialAt : Has σ credential -> Γ.CredentialP
  scopeAt      : Has σ scope      -> Γ.ScopeP
  epochAt      : Has σ epoch      -> Γ.EpochP

Pres := PresI
```

Restriction along a face is defined by reindexing `Has`.

```text
liftHas :
  {σ : Shape} ->
  {m : Nat} ->
  (u : Inj (m+1) (dim σ + 1)) ->
  (c : Color) ->
  Has (pullShape u) c ->
  Has σ c

res :
  {σ : Shape} ->
  {m : Nat} ->
  (u : Inj (m+1) (dim σ + 1)) ->
  Pres σ ->
  Pres (pullShape u)

res_id   := by routine
res_comp := by routine
```

## Presentation equivalence

Two presentations are equivalent when all present components agree after normalization.

```text
record EqvI (σ : Shape) (p q : Pres σ) : U where
  issuer_eq :
    (h : Has σ issuer) ->
    Γ.nfIssuer (p.issuerAt h) = Γ.nfIssuer (q.issuerAt h)

  subject_eq :
    (h : Has σ subject) ->
    Γ.nfSubject (p.subjectAt h) = Γ.nfSubject (q.subjectAt h)

  credential_eq :
    (h : Has σ credential) ->
    Γ.nfCredential (p.credentialAt h) = Γ.nfCredential (q.credentialAt h)

  scope_eq :
    (h : Has σ scope) ->
    Γ.nfScope (p.scopeAt h) = Γ.nfScope (q.scopeAt h)

  epoch_eq :
    (h : Has σ epoch) ->
    Γ.nfEpoch (p.epochAt h) = Γ.nfEpoch (q.epochAt h)

Eqv := EqvI

eqv_refl  := by componentwise refl
eqv_sym   := by componentwise sym
eqv_trans := by componentwise trans
res_eqv   := by restriction of component equalities
```

## Witnesses

Witnesses are active only when the required roles are present.

```text
record WitI (σ : Shape) (p : Pres σ) : U where
  issuer_trusted :
    (hi : Has σ issuer) ->
    Γ.IssuerTrusted (p.issuerAt hi)

  subject_valid :
    (hs : Has σ subject) ->
    Γ.SubjectValid (p.subjectAt hs)

  credential_valid :
    (hc : Has σ credential) ->
    Γ.CredentialValid (p.credentialAt hc)

  not_revoked :
    (hc : Has σ credential) ->
    Γ.NotRevoked (p.credentialAt hc)

  scope_valid :
    (hr : Has σ scope) ->
    Γ.ScopeValid (p.scopeAt hr)

  epoch_valid :
    (ht : Has σ epoch) ->
    Γ.EpochValid (p.epochAt ht)

  signed_by :
    (hc : Has σ credential) ->
    (hi : Has σ issuer) ->
    Γ.SignedBy (p.credentialAt hc) (p.issuerAt hi)

  names_subject :
    (hc : Has σ credential) ->
    (hs : Has σ subject) ->
    Γ.Names (p.credentialAt hc) (p.subjectAt hs)

  grants_scope :
    (hc : Has σ credential) ->
    (hr : Has σ scope) ->
    Γ.Grants (p.credentialAt hc) (p.scopeAt hr)

  fresh_at :
    (hc : Has σ credential) ->
    (ht : Has σ epoch) ->
    Γ.FreshAt (p.credentialAt hc) (p.epochAt ht)

Wit := WitI
```

`WitEq`, `wit_res`, and `wit_eqv` are inherited in the obvious way from the environment congruence fields.

## Admitted simplices

```text
Adm := fun σ => Σ (p : Pres σ), Wit σ p
adm_res u (p, w) := (res u p, wit_res u w)
```

## Full identity simplex

```text
σ_ident := (4, role5, (idInj, refl))
```

The type of operational principals is:

```text
Principal_Γ := Adm σ_ident
```

An inhabitant is a full admitted 4-simplex containing issuer, subject, credential,
scope, and epoch.

## Common faces

Typical face readings:

- `[issuer, credential]` — issuance / signature face
- `[subject, credential]` — subject-binding face
- `[credential, scope]` — grant face
- `[credential, epoch]` — freshness face
- `[issuer, subject, credential]` — named credential triangle
- `[subject, credential, scope, epoch]` — live scoped credential

## `IDENT0` in one line

```text
identity = a role-simplex profile over ASP0
```
