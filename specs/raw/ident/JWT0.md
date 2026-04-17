# PREMATH.IDENT.JWT0

## Intent

`JWT0` instantiates `IDENT0` with a JWT bearer-token backend.

One important choice is built in:

> `SubjectP` is issuer-scoped.

A naked `sub` string is not globally meaningful; it is interpreted relative to an issuer.

Another important choice:

> credential equivalence is **strict** in `JWT0`

so `CredentialNF` is a normalized token form, not a semantic quotient of “tokens with equivalent claims”.

## Concrete carriers

```text
record URI : U
record ByteString : U
record FinSet (A : U) : U

data Alg : U where
  RS256 | ES256 | EdDSA | HS256

record IssuerHandle : U where
  iss_uri : URI

record SubjectHandle : U where
  home_issuer : URI
  sub_id      : String

record ScopeReq : U where
  auds   : FinSet URI
  scopes : FinSet String

abbrev Epoch := Int64

record JwtToken : U where
  compact : ByteString
```

## Backend interface

In `JWT0`, the parsed view is abstract and may include both header-derived and claim-derived data.
So, despite the field names, `JwtClaims` should be read as:

> parsed JWT view

rather than “claim body only”.

```text
record JWTInfra : U1 where
  ------------------------------------------------------------
  -- Normal forms and parsed view
  ------------------------------------------------------------

  TokenNF   : U
  JwtClaims : U

  nfToken : JwtToken -> TokenNF
  parse   : JwtToken -> Maybe JwtClaims

  claims_iss   : JwtClaims -> URI
  claims_sub   : JwtClaims -> String
  claims_aud   : JwtClaims -> FinSet URI
  claims_scope : JwtClaims -> FinSet String
  claims_iat   : JwtClaims -> Option Int64
  claims_nbf   : JwtClaims -> Option Int64
  claims_exp   : JwtClaims -> Option Int64
  claims_jti   : JwtClaims -> Option String

  -- In a concrete implementation these are usually header-derived.
  claims_alg : JwtClaims -> Alg
  claims_kid : JwtClaims -> Option String

  normalize_uri    : URI -> URI
  normalize_sub    : String -> String
  normalize_auds   : FinSet URI -> FinSet URI
  normalize_scopes : FinSet String -> FinSet String

  ------------------------------------------------------------
  -- Trust / crypto / policy oracles
  ------------------------------------------------------------

  trusted_issuer : URI -> U
  alg_allowed    : Alg -> U

  verify_sig :
    JwtToken -> URI -> Option String -> U

  revoked_jti :
    URI -> String -> U

  aud_covers :
    FinSet URI -> FinSet URI -> U

  scope_covers :
    FinSet String -> FinSet String -> U

  ------------------------------------------------------------
  -- Time window predicates
  ------------------------------------------------------------

  skew_sec : Nat

  nbf_ok :
    (now : Int64) -> (nbf : Option Int64) -> U

  exp_ok :
    (now : Int64) -> (exp : Option Int64) -> U
```

## Concrete `IEnv`

```text
def JWTEnv (J : JWTInfra) : IEnv where

  ------------------------------------------------------------
  -- Carriers
  ------------------------------------------------------------

  IssuerP     := IssuerHandle
  SubjectP    := SubjectHandle
  CredentialP := JwtToken
  ScopeP      := ScopeReq
  EpochP      := Epoch

  ------------------------------------------------------------
  -- Normal forms
  ------------------------------------------------------------

  IssuerNF     := URI
  SubjectNF    := URI × String
  CredentialNF := J.TokenNF
  ScopeNF      := FinSet URI × FinSet String
  EpochNF      := Int64

  nfIssuer i := J.normalize_uri i.iss_uri

  nfSubject s :=
    ( J.normalize_uri s.home_issuer
    , J.normalize_sub s.sub_id )

  nfCredential c := J.nfToken c

  nfScope r :=
    ( J.normalize_auds r.auds
    , J.normalize_scopes r.scopes )

  nfEpoch t := t

  ------------------------------------------------------------
  -- Unary predicates
  ------------------------------------------------------------

  IssuerTrusted i :=
    J.trusted_issuer (nfIssuer i)

  SubjectValid s :=
    NonEmpty s.sub_id

  CredentialValid c :=
    Σ (cl : J.JwtClaims),
      J.parse c = some cl
      × J.alg_allowed (J.claims_alg cl)

  NotRevoked c :=
    Σ (cl : J.JwtClaims),
      J.parse c = some cl
      × match J.claims_jti cl with
        | none   => Unit
        | some j => Not (J.revoked_jti (J.normalize_uri (J.claims_iss cl)) j)

  ScopeValid r :=
    Unit

  EpochValid t :=
    Unit

  ------------------------------------------------------------
  -- Binary relations
  ------------------------------------------------------------

  SignedBy c i :=
    Σ (cl : J.JwtClaims),
      J.parse c = some cl
      × J.normalize_uri (J.claims_iss cl) = nfIssuer i
      × J.alg_allowed (J.claims_alg cl)
      × J.verify_sig c (nfIssuer i) (J.claims_kid cl)

  Names c s :=
    Σ (cl : J.JwtClaims),
      J.parse c = some cl
      × ( J.normalize_uri (J.claims_iss cl)
        , J.normalize_sub (J.claims_sub cl) )
        = nfSubject s

  Grants c r :=
    Σ (cl : J.JwtClaims),
      J.parse c = some cl
      × J.aud_covers
          (J.normalize_auds   (J.claims_aud cl))
          (fst (nfScope r))
      × J.scope_covers
          (J.normalize_scopes (J.claims_scope cl))
          (snd (nfScope r))

  FreshAt c t :=
    Σ (cl : J.JwtClaims),
      J.parse c = some cl
      × J.nbf_ok t (J.claims_nbf cl)
      × J.exp_ok t (J.claims_exp cl)

  ------------------------------------------------------------
  -- Congruence fields
  ------------------------------------------------------------

  IssuerTrusted_cong   := by rewrite
  SubjectValid_cong    := by rewrite
  CredentialValid_cong := by rewrite
  NotRevoked_cong      := by rewrite
  ScopeValid_cong      := by rewrite
  EpochValid_cong      := by rewrite

  SignedBy_cong := by rewrite
  Names_cong    := by rewrite
  Grants_cong   := by rewrite
  FreshAt_cong  := by rewrite
```

## The induced ASP profile

```text
def IDENT_JWT0 (J : JWTInfra) : ASP0 :=
  IDENT0 (JWTEnv J)
```

## Full operational principal

Let `A := IDENT_JWT0 J` and let `σ_ident` be the full role simplex from `IDENT0`.

```text
PrincipalJWT : U
PrincipalJWT := A.Adm σ_ident
```

An inhabitant is:

```text
(p, w)
```

where:

- `p` assigns issuer, subject, credential, scope, epoch
- `w` proves trust, signature validity, subject binding, grant coverage, freshness, and non-revocation

## Reading of the full simplex

For

```text
[issuer, subject, credential, scope, epoch]
```

the concrete reading is:

- `issuer`      — trusted JWT issuer handle
- `subject`     — issuer-scoped subject handle
- `credential`  — JWT bearer token
- `scope`       — requested audience/scope context
- `epoch`       — evaluation time

## Design notes

- `JWT0` is intentionally strict on credential equivalence.
- `ScopeValid` and `EpochValid` are trivial in `JWT0`; richer policy lands later.
- `JWT1-JWKS0` later realizes `claims_alg` and `claims_kid` using a parsed header component.
