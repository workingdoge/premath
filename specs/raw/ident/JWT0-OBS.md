# PREMATH.IDENT.JWT0.OBS

## Intent

`JWT0-OBS` adds typed obstruction families and an executable checker interface
for the full identity simplex.

This is the lower obstruction layer:

> obstruction to building a witness over a **fixed full presentation**

rather than the general higher horn obstruction theory of `ASP0`.

## Full identity shape and canonical presentation

```text
module PREMATH.IDENT.JWT0.OBS
import PREMATH.IDENT.JWT0

universe U U1

module A (J : JWTInfra) := IDENT0 (JWTEnv J)

def σ_ident (J : JWTInfra) : (A J).Shape :=
  (4, role5, (idInj, refl))

def hI (J : JWTInfra) : (A J).Has (σ_ident J) issuer     := (0, refl)
def hS (J : JWTInfra) : (A J).Has (σ_ident J) subject    := (1, refl)
def hC (J : JWTInfra) : (A J).Has (σ_ident J) credential := (2, refl)
def hR (J : JWTInfra) : (A J).Has (σ_ident J) scope      := (3, refl)
def hT (J : JWTInfra) : (A J).Has (σ_ident J) epoch      := (4, refl)

def fullPres
  (J : JWTInfra)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (c : JwtToken)
  (r : ScopeReq)
  (t : Epoch)
  : (A J).Pres (σ_ident J)
:= {
  issuerAt     := fun _ => i
  subjectAt    := fun _ => s
  credentialAt := fun _ => c
  scopeAt      := fun _ => r
  epochAt      := fun _ => t
}

def FullWit
  (J : JWTInfra)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (c : JwtToken)
  (r : ScopeReq)
  (t : Epoch)
  : U
:= (A J).Wit (σ_ident J) (fullPres J i s c r t)
```

## Obstruction algebra

```text
data FullObs
  (J : JWTInfra)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (c : JwtToken)
  (r : ScopeReq)
  (t : Epoch)
  : U
where
  parse_error :
    J.parse c = none ->
    FullObs J i s c r t

  issuer_untrusted :
    Not ((JWTEnv J).IssuerTrusted i) ->
    FullObs J i s c r t

  subject_invalid :
    Not ((JWTEnv J).SubjectValid s) ->
    FullObs J i s c r t

  alg_disallowed :
    (cl : J.JwtClaims) ->
    J.parse c = some cl ->
    Not (J.alg_allowed (J.claims_alg cl)) ->
    FullObs J i s c r t

  issuer_claim_mismatch :
    (cl : J.JwtClaims) ->
    J.parse c = some cl ->
    Not (J.normalize_uri (J.claims_iss cl) = (JWTEnv J).nfIssuer i) ->
    FullObs J i s c r t

  signature_invalid :
    (cl : J.JwtClaims) ->
    J.parse c = some cl ->
    Not (J.verify_sig c ((JWTEnv J).nfIssuer i) (J.claims_kid cl)) ->
    FullObs J i s c r t

  subject_mismatch :
    (cl : J.JwtClaims) ->
    J.parse c = some cl ->
    Not (
      ( J.normalize_uri (J.claims_iss cl)
      , J.normalize_sub (J.claims_sub cl) )
      =
      (JWTEnv J).nfSubject s
    ) ->
    FullObs J i s c r t

  audience_insufficient :
    (cl : J.JwtClaims) ->
    J.parse c = some cl ->
    Not (
      J.aud_covers
        (J.normalize_auds (J.claims_aud cl))
        (fst ((JWTEnv J).nfScope r))
    ) ->
    FullObs J i s c r t

  scope_insufficient :
    (cl : J.JwtClaims) ->
    J.parse c = some cl ->
    Not (
      J.scope_covers
        (J.normalize_scopes (J.claims_scope cl))
        (snd ((JWTEnv J).nfScope r))
    ) ->
    FullObs J i s c r t

  token_not_yet_valid :
    (cl : J.JwtClaims) ->
    J.parse c = some cl ->
    Not (J.nbf_ok t (J.claims_nbf cl)) ->
    FullObs J i s c r t

  token_expired :
    (cl : J.JwtClaims) ->
    J.parse c = some cl ->
    Not (J.exp_ok t (J.claims_exp cl)) ->
    FullObs J i s c r t

  token_revoked :
    (cl : J.JwtClaims) ->
    J.parse c = some cl ->
    (j : String) ->
    J.claims_jti cl = some j ->
    J.revoked_jti (J.normalize_uri (J.claims_iss cl)) j ->
    FullObs J i s c r t
```

## Soundness theorem

```text
theorem fullObs_sound :
  (J : JWTInfra) ->
  (i : IssuerHandle) ->
  (s : SubjectHandle) ->
  (c : JwtToken) ->
  (r : ScopeReq) ->
  (t : Epoch) ->
  FullObs J i s c r t ->
  Not (FullWit J i s c r t)
```

Proof sketch:

- `parse_error` contradicts `credential_valid` at `hC`
- `issuer_untrusted` contradicts `issuer_trusted` at `hI`
- `subject_invalid` contradicts `subject_valid` at `hS`
- `alg_disallowed` contradicts `credential_valid`
- `issuer_claim_mismatch` or `signature_invalid` contradict `signed_by hC hI`
- `subject_mismatch` contradicts `names_subject hC hS`
- `audience_insufficient` or `scope_insufficient` contradict `grants_scope hC hR`
- `token_not_yet_valid` or `token_expired` contradict `fresh_at hC hT`
- `token_revoked` contradicts `not_revoked hC`

## Positive witness builder

```text
def buildFullWitness
  (J : JWTInfra)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (c : JwtToken)
  (r : ScopeReq)
  (t : Epoch)
  (cl : J.JwtClaims)
  (parse_ok : J.parse c = some cl)
  (issuer_ok : (JWTEnv J).IssuerTrusted i)
  (subject_ok : (JWTEnv J).SubjectValid s)
  (alg_ok : J.alg_allowed (J.claims_alg cl))
  (iss_ok : J.normalize_uri (J.claims_iss cl) = (JWTEnv J).nfIssuer i)
  (sig_ok : J.verify_sig c ((JWTEnv J).nfIssuer i) (J.claims_kid cl))
  (sub_ok :
    ( J.normalize_uri (J.claims_iss cl)
    , J.normalize_sub (J.claims_sub cl) )
    = (JWTEnv J).nfSubject s)
  (aud_ok :
    J.aud_covers
      (J.normalize_auds (J.claims_aud cl))
      (fst ((JWTEnv J).nfScope r)))
  (scope_ok :
    J.scope_covers
      (J.normalize_scopes (J.claims_scope cl))
      (snd ((JWTEnv J).nfScope r)))
  (nbf_ok : J.nbf_ok t (J.claims_nbf cl))
  (exp_ok : J.exp_ok t (J.claims_exp cl))
  (rev_ok :
    match J.claims_jti cl with
    | none   => Unit
    | some j => Not (J.revoked_jti (J.normalize_uri (J.claims_iss cl)) j))
  : FullWit J i s c r t
```

The resulting witness fills all fields of `Wit` on the full identity simplex.

## Decision package

```text
record JWTDec (J : JWTInfra) : U1 where
  dec_IssuerTrusted :
    (i : IssuerHandle) ->
    Dec ((JWTEnv J).IssuerTrusted i)

  dec_SubjectValid :
    (s : SubjectHandle) ->
    Dec ((JWTEnv J).SubjectValid s)

  dec_alg_allowed :
    (a : Alg) ->
    Dec (J.alg_allowed a)

  dec_verify_sig :
    (c : JwtToken) ->
    (u : URI) ->
    (k : Option String) ->
    Dec (J.verify_sig c u k)

  dec_uri_eq :
    (x y : URI) -> Dec (x = y)

  dec_subject_eq :
    (x y : URI × String) -> Dec (x = y)

  dec_aud_covers :
    (have need : FinSet URI) ->
    Dec (J.aud_covers have need)

  dec_scope_covers :
    (have need : FinSet String) ->
    Dec (J.scope_covers have need)

  dec_nbf_ok :
    (now : Int64) ->
    (nbf : Option Int64) ->
    Dec (J.nbf_ok now nbf)

  dec_exp_ok :
    (now : Int64) ->
    (exp : Option Int64) ->
    Dec (J.exp_ok now exp)

  dec_revoked_jti :
    (iss : URI) ->
    (j : String) ->
    Dec (J.revoked_jti iss j)
```

## Checker

```text
def checkFull
  (J : JWTInfra)
  (D : JWTDec J)
  (i : IssuerHandle)
  (s : SubjectHandle)
  (c : JwtToken)
  (r : ScopeReq)
  (t : Epoch)
  : Either (FullObs J i s c r t) ((A J).Adm (σ_ident J))
```

Operationally, `checkFull` performs the staged case split:

1. parse token
2. check allowed algorithm
3. check issuer trust
4. check subject well-formedness
5. compare normalized `iss`
6. verify signature
7. compare issuer-scoped subject
8. check audience coverage
9. check scope coverage
10. check `nbf`
11. check `exp`
12. check revocation if `jti` is present
13. on success, return `(fullPres, buildFullWitness ...)`

## Summary

`JWT0-OBS` is the point where generic identity presentation becomes a practical
checker with typed failure modes.
