# PREMATH.IDENT.JWT0.SEARCH

## Intent

`JWT0-SEARCH` adds horn-like runtime fillers over `JWT0-OBS`.

The central rule is:

> a filler is not just “some admitted full simplex”;
> it must preserve the non-missing coordinates of the horn boundary.

This spec defines:

- missing credential search
- missing issuer recovery
- missing scope narrowing by finite menu
- missing scope narrowing by meet / clipping
- combined issuer recovery + scope clipping

## Full admitted simplex and projections

```text
module PREMATH.IDENT.JWT0.SEARCH
import PREMATH.IDENT.JWT0.OBS

universe U U1

module A (J : JWTInfra) := IDENT0 (JWTEnv J)

def Adm0 (J : JWTInfra) : U :=
  (A J).Adm (σ_ident J)

def presOf {J : JWTInfra} (a : Adm0 J) := fst a
def witOf  {J : JWTInfra} (a : Adm0 J) := snd a

def issuerOfAdm (J : JWTInfra) (a : Adm0 J) : IssuerHandle :=
  (presOf a).issuerAt (hI J)

def subjectOfAdm (J : JWTInfra) (a : Adm0 J) : SubjectHandle :=
  (presOf a).subjectAt (hS J)

def credentialOfAdm (J : JWTInfra) (a : Adm0 J) : JwtToken :=
  (presOf a).credentialAt (hC J)

def scopeOfAdm (J : JWTInfra) (a : Adm0 J) : ScopeReq :=
  (presOf a).scopeAt (hR J)

def epochOfAdm (J : JWTInfra) (a : Adm0 J) : Epoch :=
  (presOf a).epochAt (hT J)
```

## Fixed-coordinate predicates

```text
-- issuer missing; subject / credential / scope / epoch fixed
def FixSCRT
  (J : JWTInfra)
  (s : SubjectHandle)
  (c : JwtToken)
  (r : ScopeReq)
  (t : Epoch)
  (a : Adm0 J)
  : U
:=
  subjectOfAdm    J a = s
  × credentialOfAdm J a = c
  × scopeOfAdm      J a = r
  × epochOfAdm      J a = t

-- credential missing; issuer / subject / scope / epoch fixed
def FixISRT
  (J : JWTInfra)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (r : ScopeReq)
  (t : Epoch)
  (a : Adm0 J)
  : U
:=
  issuerOfAdm     J a = i
  × subjectOfAdm    J a = s
  × scopeOfAdm      J a = r
  × epochOfAdm      J a = t

-- scope missing; issuer / subject / credential / epoch fixed
def FixISCT
  (J : JWTInfra)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (c : JwtToken)
  (t : Epoch)
  (a : Adm0 J)
  : U
:=
  issuerOfAdm     J a = i
  × subjectOfAdm    J a = s
  × credentialOfAdm J a = c
  × epochOfAdm      J a = t

-- issuer and scope both missing; subject / credential / epoch fixed
def FixSCT
  (J : JWTInfra)
  (s : SubjectHandle)
  (c : JwtToken)
  (t : Epoch)
  (a : Adm0 J)
  : U
:=
  subjectOfAdm    J a = s
  × credentialOfAdm J a = c
  × epochOfAdm      J a = t
```

## Missing credential: wallet / candidate search

```text
def FillCredential
  (J : JWTInfra)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (r : ScopeReq)
  (t : Epoch)
  : U
:=
  Σ (a : Adm0 J), FixISRT J i s r t a


data CredCandidateFail
  (J : JWTInfra)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (r : ScopeReq)
  (t : Epoch)
  : JwtToken -> U
where
  blocked :
    {c' : JwtToken} ->
    FullObs J i s c' r t ->
    CredCandidateFail J i s r t c'
```

A generic “all candidates failed” certificate:

```text
data All {A : U} (P : A -> U) : List A -> U where
  nil  : All P []
  cons : {x : A} -> {xs : List A} -> P x -> All P xs -> All P (x :: xs)

def CredSearchObs
  (J : JWTInfra)
  (W : List JwtToken)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (r : ScopeReq)
  (t : Epoch)
  : U
:=
  All (CredCandidateFail J i s r t) W
```

Searcher:

```text
def fillCredentialFromWallet
  (J : JWTInfra)
  (D : JWTDec J)
  (W : List JwtToken)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (r : ScopeReq)
  (t : Epoch)
  : Either (CredSearchObs J W i s r t)
           (FillCredential J i s r t)
```

Operationally:
- try each candidate token in `W`
- run `checkFull J D i s c r t`
- return the first admitted full simplex
- otherwise return a typed failure for every candidate

## Missing issuer: deterministic recovery from `iss`

```text
def recoveredIssuer
  (J : JWTInfra)
  (cl : J.JwtClaims)
  : IssuerHandle
:= {
  iss_uri := J.normalize_uri (J.claims_iss cl)
}

def FillIssuer
  (J : JWTInfra)
  (s : SubjectHandle)
  (c : JwtToken)
  (r : ScopeReq)
  (t : Epoch)
  : U
:=
  Σ (a : Adm0 J), FixSCRT J s c r t a
```

Obstruction family:

```text
data IssuerHoleObs
  (J : JWTInfra)
  (s : SubjectHandle)
  (c : JwtToken)
  (r : ScopeReq)
  (t : Epoch)
  : U
where
  parse_error :
    J.parse c = none ->
    IssuerHoleObs J s c r t

  recovered_issuer_failed :
    (cl : J.JwtClaims) ->
    J.parse c = some cl ->
    FullObs J (recoveredIssuer J cl) s c r t ->
    IssuerHoleObs J s c r t
```

Filler:

```text
def fillIssuerFromToken
  (J : JWTInfra)
  (D : JWTDec J)
  (s : SubjectHandle)
  (c : JwtToken)
  (r : ScopeReq)
  (t : Epoch)
  : Either (IssuerHoleObs J s c r t)
           (FillIssuer J s c r t)
```

At `JWT0`, this is deterministic after parsing.

## Scope order

Scope narrowing is expressed as:

```text
def NarrowerThan
  (J : JWTInfra)
  (r' r : ScopeReq)
  : U
:=
  J.aud_covers
    (fst ((JWTEnv J).nfScope r))
    (fst ((JWTEnv J).nfScope r'))
  ×
  J.scope_covers
    (snd ((JWTEnv J).nfScope r))
    (snd ((JWTEnv J).nfScope r'))
```

So `r' ≤ r` means `r'` is no broader than `r`.

## Missing scope: finite candidate list

```text
def FillScopeUnder
  (J : JWTInfra)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (c : JwtToken)
  (r_req : ScopeReq)
  (t : Epoch)
  : U
:=
  Σ (a : Adm0 J),
    FixISCT J i s c t a
    ×
    NarrowerThan J (scopeOfAdm J a) r_req
```

Per-candidate failure:

```text
data ScopeCandidateFail
  (J : JWTInfra)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (c : JwtToken)
  (r_req : ScopeReq)
  (t : Epoch)
  : ScopeReq -> U
where
  too_broad :
    {r' : ScopeReq} ->
    Not (NarrowerThan J r' r_req) ->
    ScopeCandidateFail J i s c r_req t r'

  blocked :
    {r' : ScopeReq} ->
    NarrowerThan J r' r_req ->
    FullObs J i s c r' t ->
    ScopeCandidateFail J i s c r_req t r'
```

Aggregate obstruction:

```text
def ScopeListObs
  (J : JWTInfra)
  (R : List ScopeReq)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (c : JwtToken)
  (r_req : ScopeReq)
  (t : Epoch)
  : U
:=
  All (ScopeCandidateFail J i s c r_req t) R
```

Searcher:

```text
def dec_NarrowerThan
  (J : JWTInfra)
  (D : JWTDec J)
  (r' r : ScopeReq)
  : Dec (NarrowerThan J r' r)

def fillScopeFromList
  (J : JWTInfra)
  (D : JWTDec J)
  (R : List ScopeReq)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (c : JwtToken)
  (r_req : ScopeReq)
  (t : Epoch)
  : Either (ScopeListObs J R i s c r_req t)
           (FillScopeUnder J i s c r_req t)
```

## Missing scope: deterministic clipping by meet

```text
record JWTScopeMeet (J : JWTInfra) : U1 where
  aud_meet   : FinSet URI -> FinSet URI -> FinSet URI
  scope_meet : FinSet String -> FinSet String -> FinSet String

  aud_meet_left  :
    (x y : FinSet URI) ->
    J.aud_covers x (aud_meet x y)

  aud_meet_right :
    (x y : FinSet URI) ->
    J.aud_covers y (aud_meet x y)

  scope_meet_left  :
    (x y : FinSet String) ->
    J.scope_covers x (scope_meet x y)

  scope_meet_right :
    (x y : FinSet String) ->
    J.scope_covers y (scope_meet x y)

  aud_norm_idem :
    (x : FinSet URI) ->
    J.normalize_auds (J.normalize_auds x) = J.normalize_auds x

  scope_norm_idem :
    (x : FinSet String) ->
    J.normalize_scopes (J.normalize_scopes x) = J.normalize_scopes x
```

Scope clipping:

```text
def clipScope
  (J : JWTInfra)
  (M : JWTScopeMeet J)
  (cl : J.JwtClaims)
  (r_req : ScopeReq)
  : ScopeReq
:= {
  auds :=
    M.aud_meet
      (J.normalize_auds (J.claims_aud cl))
      (fst ((JWTEnv J).nfScope r_req))

  scopes :=
    M.scope_meet
      (J.normalize_scopes (J.claims_scope cl))
      (snd ((JWTEnv J).nfScope r_req))
}
```

The key properties are:

```text
theorem clipScope_narrow :
  NarrowerThan J (clipScope J M cl r_req) r_req

theorem token_covers_clip :
  J.aud_covers
    (J.normalize_auds (J.claims_aud cl))
    (fst ((JWTEnv J).nfScope (clipScope J M cl r_req)))
  ×
  J.scope_covers
    (J.normalize_scopes (J.claims_scope cl))
    (snd ((JWTEnv J).nfScope (clipScope J M cl r_req)))
```

Obstruction family:

```text
data ClipScopeObs
  (J : JWTInfra)
  (M : JWTScopeMeet J)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (c : JwtToken)
  (r_req : ScopeReq)
  (t : Epoch)
  : U
where
  parse_error :
    J.parse c = none ->
    ClipScopeObs J M i s c r_req t

  clipped_scope_failed :
    (cl : J.JwtClaims) ->
    J.parse c = some cl ->
    let r' := clipScope J M cl r_req
    FullObs J i s c r' t ->
    ClipScopeObs J M i s c r_req t
```

Filler:

```text
def fillScopeByMeet
  (J : JWTInfra)
  (D : JWTDec J)
  (M : JWTScopeMeet J)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (c : JwtToken)
  (r_req : ScopeReq)
  (t : Epoch)
  : Either (ClipScopeObs J M i s c r_req t)
           (FillScopeUnder J i s c r_req t)
```

Under the meet laws, failures from `fillScopeByMeet` are no longer “scope too large”
failures; the remaining blockers are the non-scope clauses such as signature, freshness,
subject mismatch, or revocation.

## Combined missing issuer + scope

```text
def FillIssuerAndScopeUnder
  (J : JWTInfra)
  (s : SubjectHandle)
  (c : JwtToken)
  (r_req : ScopeReq)
  (t : Epoch)
  : U
:=
  Σ (a : Adm0 J),
    FixSCT J s c t a
    ×
    NarrowerThan J (scopeOfAdm J a) r_req
```

Obstruction family:

```text
data IssuerScopeObs
  (J : JWTInfra)
  (M : JWTScopeMeet J)
  (s : SubjectHandle)
  (c : JwtToken)
  (r_req : ScopeReq)
  (t : Epoch)
  : U
where
  parse_error :
    J.parse c = none ->
    IssuerScopeObs J M s c r_req t

  recovered_pair_failed :
    (cl : J.JwtClaims) ->
    J.parse c = some cl ->
    let i  := recoveredIssuer J cl
    let r' := clipScope J M cl r_req
    FullObs J i s c r' t ->
    IssuerScopeObs J M s c r_req t
```

Filler:

```text
def fillIssuerAndNarrowScope
  (J : JWTInfra)
  (D : JWTDec J)
  (M : JWTScopeMeet J)
  (s : SubjectHandle)
  (c : JwtToken)
  (r_req : ScopeReq)
  (t : Epoch)
  : Either (IssuerScopeObs J M s c r_req t)
           (FillIssuerAndScopeUnder J s c r_req t)
```

## Summary

`JWT0-SEARCH` turns the identity profile into a real horn-filling API:

- pick a credential from a wallet
- recover an issuer from token claims
- narrow a requested scope
- combine recovery and narrowing in one elaboration step
