# PREMATH.IDENT.JWT1.JWKS0

## Intent

`JWT1-JWKS0` gives a concrete snapshot-based trust model for `JWT0`.

The checker remains pure:

> it reads a snapshot; it never fetches.

HTTP discovery, JWKS revalidation, and revocation refresh are modeled as **snapshot transitions**.

One key repair from raw JWT intuition is explicit here:

> `alg` and `kid` live in the **header**, not the claim body.

The snapshot backend therefore instantiates `JWT0.JwtClaims` with a parsed token view
that contains both header and claims.

## Core runtime objects

```text
module PREMATH.IDENT.JWT1.JWKS0
import PREMATH.IDENT.JWT0.SEARCH

universe U U1

data KeyUse : U where
  sig | enc

data MetaMode : U where
  static_jwks    : URI -> MetaMode
  oidc_discovery : URI -> MetaMode


record ProviderMeta : U where
  issuer   : URI
  jwks_uri : URI


record HttpFreshness : U where
  etag               : Option String
  max_age_sec        : Nat
  stale_if_error_sec : Nat


record CacheEntry (A : U) : U where
  value       : A
  fetched_at  : Int64
  fresh_until : Int64
  stale_until : Int64
  etag        : Option String


def Fresh {A : U} (now : Int64) (e : CacheEntry A) : U :=
  now <= e.fresh_until

def Usable {A : U} (now : Int64) (e : CacheEntry A) : U :=
  now <= e.stale_until


record IssuerCfg : U where
  mode        : MetaMode
  alg_allowed : Alg -> U
```

## Base JWT/JWKS interface

```text
record JWKSBase : U1 where
  Header Claims PubKey : U

  record Jwk : U where
    kid      : Option String
    use      : Option KeyUse
    alg_hint : Option Alg
    pub      : PubKey

  record JwksDoc : U where
    keys : List Jwk

  parse : JwtToken -> Maybe (Header × Claims)

  header_alg : Header -> Alg
  header_kid : Header -> Option String

  claims_iss   : Claims -> URI
  claims_sub   : Claims -> String
  claims_aud   : Claims -> FinSet URI
  claims_scope : Claims -> FinSet String
  claims_iat   : Claims -> Option Int64
  claims_nbf   : Claims -> Option Int64
  claims_exp   : Claims -> Option Int64
  claims_jti   : Claims -> Option String

  normalize_uri    : URI -> URI
  normalize_sub    : String -> String
  normalize_auds   : FinSet URI -> FinSet URI
  normalize_scopes : FinSet String -> FinSet String

  global_alg_allowed : Alg -> U
  key_compatible_alg : Jwk -> Alg -> U
  key_verify         : JwtToken -> Jwk -> U

  aud_covers   : FinSet URI -> FinSet URI -> U
  scope_covers : FinSet String -> FinSet String -> U

  default_skew : Nat

  nbf_ok : Int64 -> Option Int64 -> U
  exp_ok : Int64 -> Option Int64 -> U
```

## Snapshot

```text
record Snapshot (B : JWKSBase) : U1 where
  now : Int64

  issuer_cfg :
    URI -> Option IssuerCfg

  meta_cache :
    URI -> Option (CacheEntry ProviderMeta)

  jwks_cache :
    URI -> Option (CacheEntry B.JwksDoc)

  revocation_cache :
    URI -> Option (CacheEntry (FinSet String))
```

Trusted issuers are exactly those named in `issuer_cfg`.

## Discovery and key selection

```text
def KeyUseSig {B : JWKSBase} (k : B.Jwk) : U :=
  match k.use with
  | none      => Unit
  | some sig  => Unit
  | some enc  => Empty

def KidMatches {B : JWKSBase} (kid : Option String) (k : B.Jwk) : U :=
  match kid with
  | none    => Unit
  | some j  => k.kid = some j

def Candidate
  (B : JWKSBase)
  (cfg : IssuerCfg)
  (alg : Alg)
  (kid : Option String)
  (k : B.Jwk)
  : U
:=
  KeyUseSig k
  × cfg.alg_allowed alg
  × B.key_compatible_alg k alg
  × KidMatches kid k
```

Issuer-resolution obstructions:

```text
data ResolveJwksObs
  (B : JWKSBase)
  (Σ : Snapshot B)
  (iss : URI)
  : U
where
  issuer_unknown :
    Σ.issuer_cfg iss = none ->
    ResolveJwksObs B Σ iss

  provider_metadata_unavailable :
    (disc : URI) ->
    ResolveJwksObs B Σ iss

  provider_metadata_mismatch :
    (disc : URI) ->
    (e : CacheEntry ProviderMeta) ->
    Σ.meta_cache disc = some e ->
    Usable Σ.now e ->
    Not (B.normalize_uri e.value.issuer = iss) ->
    ResolveJwksObs B Σ iss

  jwks_unavailable :
    (jwks_uri : URI) ->
    ResolveJwksObs B Σ iss
```

Resolved issuer target:

```text
record ResolvedIssuer
  (B : JWKSBase)
  (Σ : Snapshot B)
  (iss : URI)
  : U
where
  cfg : IssuerCfg
  cfg_hit : Σ.issuer_cfg iss = some cfg
  jwks_uri : URI
```

Resolver:

```text
def resolveIssuer
  (B : JWKSBase)
  (Σ : Snapshot B)
  (iss_raw : URI)
  : Either (ResolveJwksObs B Σ (B.normalize_uri iss_raw))
           (ResolvedIssuer B Σ (B.normalize_uri iss_raw))
```

Key-resolution obstructions:

```text
data ResolveKeyObs
  (B : JWKSBase)
  (Σ : Snapshot B)
  (iss : URI)
  (alg : Alg)
  (kid : Option String)
  : U
where
  issuer_resolution_failed :
    ResolveJwksObs B Σ iss ->
    ResolveKeyObs B Σ iss alg kid

  jwks_unavailable :
    (ri : ResolvedIssuer B Σ iss) ->
    ResolveKeyObs B Σ iss alg kid

  no_compatible_key :
    (ri : ResolvedIssuer B Σ iss) ->
    ResolveKeyObs B Σ iss alg kid

  ambiguous_keys :
    (ri : ResolvedIssuer B Σ iss) ->
    ResolveKeyObs B Σ iss alg kid
```

Selected-key witness:

```text
record SelectedKey
  (B : JWKSBase)
  (Σ : Snapshot B)
  (iss : URI)
  (alg : Alg)
  (kid : Option String)
  : U
where
  issuer : ResolvedIssuer B Σ iss

  entry : CacheEntry B.JwksDoc
  cache_hit : Σ.jwks_cache issuer.jwks_uri = some entry
  usable : Usable Σ.now entry

  key : B.Jwk
  member : key ∈ entry.value.keys
  candidate : Candidate B issuer.cfg alg kid key

  unique :
    (k' : B.Jwk) ->
    k' ∈ entry.value.keys ->
    Candidate B issuer.cfg alg kid k' ->
    k' = key
```

Selector:

```text
def resolveKey
  (B : JWKSBase)
  (Σ : Snapshot B)
  (iss : URI)
  (alg : Alg)
  (kid : Option String)
  : Either (ResolveKeyObs B Σ iss alg kid)
           (SelectedKey B Σ iss alg kid)
```

## Snapshot to `JWTInfra`

The runtime snapshot induces the abstract interface expected by `JWT0`.

```text
def JWTInfra_of_snapshot
  (B : JWKSBase)
  (Σ : Snapshot B)
  : JWTInfra
where
  TokenNF   := ByteString
  JwtClaims := B.Header × B.Claims

  nfToken t := t.compact
  parse := B.parse

  claims_iss   := fun p => B.claims_iss p.2
  claims_sub   := fun p => B.claims_sub p.2
  claims_aud   := fun p => B.claims_aud p.2
  claims_scope := fun p => B.claims_scope p.2
  claims_iat   := fun p => B.claims_iat p.2
  claims_nbf   := fun p => B.claims_nbf p.2
  claims_exp   := fun p => B.claims_exp p.2
  claims_jti   := fun p => B.claims_jti p.2

  claims_alg   := fun p => B.header_alg p.1
  claims_kid   := fun p => B.header_kid p.1

  normalize_uri    := B.normalize_uri
  normalize_sub    := B.normalize_sub
  normalize_auds   := B.normalize_auds
  normalize_scopes := B.normalize_scopes

  trusted_issuer :=
    fun u => Σ.issuer_cfg (B.normalize_uri u) ≠ none

  alg_allowed :=
    B.global_alg_allowed

  verify_sig :=
    fun tok iss kid =>
      Σ (p : B.Header × B.Claims),
        B.parse tok = some p
        ×
        Σ (sel : SelectedKey B Σ (B.normalize_uri iss) (B.header_alg p.1) kid),
          B.key_verify tok sel.key

  revoked_jti :=
    fun iss j =>
      Σ (e : CacheEntry (FinSet String)),
        Σ.revocation_cache (B.normalize_uri iss) = some e
        × Usable Σ.now e
        × j ∈ e.value

  aud_covers   := B.aud_covers
  scope_covers := B.scope_covers

  skew_sec := B.default_skew
  nbf_ok   := B.nbf_ok
  exp_ok   := B.exp_ok
```

## Decider package

```text
record JWKSDec (B : JWKSBase) : U1 where
  dec_uri_eq    : (x y : URI) -> Dec (x = y)
  dec_string_eq : (x y : String) -> Dec (x = y)

  dec_global_alg_allowed :
    (a : Alg) -> Dec (B.global_alg_allowed a)

  dec_cfg_alg_allowed :
    (cfg : IssuerCfg) ->
    (a : Alg) ->
    Dec (cfg.alg_allowed a)

  dec_key_compatible_alg :
    (k : B.Jwk) ->
    (a : Alg) ->
    Dec (B.key_compatible_alg k a)

  dec_key_verify :
    (tok : JwtToken) ->
    (k : B.Jwk) ->
    Dec (B.key_verify tok k)

  dec_aud_covers :
    (have need : FinSet URI) ->
    Dec (B.aud_covers have need)

  dec_scope_covers :
    (have need : FinSet String) ->
    Dec (B.scope_covers have need)

  dec_rev_member :
    (S : FinSet String) ->
    (j : String) ->
    Dec (j ∈ S)
```

From this, derive:

```text
def JWTDec_of_snapshot
  (B : JWKSBase)
  (D : JWKSDec B)
  (Σ : Snapshot B)
  : JWTDec (JWTInfra_of_snapshot B Σ)
```

so all of `JWT0`, `JWT0-OBS`, and `JWT0-SEARCH` run unchanged on snapshots.

## Refresh semantics

```text
data Revalidate (A : U) : U where
  fetched      : A -> HttpFreshness -> Revalidate A
  not_modified : HttpFreshness -> Revalidate A
  failed       : Revalidate A
```

Entry installation:

```text
def installEntry
  {A : U}
  (now : Int64)
  (x : A)
  (h : HttpFreshness)
  : CacheEntry A
```

Revalidation:

```text
def revalidateEntry
  {A : U}
  (now : Int64)
  (old : Option (CacheEntry A))
  (r : Revalidate A)
  : Option (CacheEntry A)
```

Key rule:
- `fetched x h` installs fresh content
- `not_modified h` preserves cached value and extends freshness
- `failed` preserves the old cache entry

Snapshot steps:

```text
def stepDiscovery
  (B : JWKSBase)
  (Σ : Snapshot B)
  (disc : URI)
  (r : Revalidate ProviderMeta)
  : Snapshot B

def stepJWKS
  (B : JWKSBase)
  (Σ : Snapshot B)
  (u : URI)
  (r : Revalidate B.JwksDoc)
  : Snapshot B

def stepRevocation
  (B : JWKSBase)
  (Σ : Snapshot B)
  (iss : URI)
  (r : Revalidate (FinSet String))
  : Snapshot B
```

## Reachability by refresh

Let `ReachableByRefresh Σ Σ'` be the reflexive-transitive closure of
`stepDiscovery`, `stepJWKS`, and `stepRevocation`.

A compact pseudo-definition is:

```text
data ReachableByRefresh (B : JWKSBase) : Snapshot B -> Snapshot B -> U where
  refl  : ReachableByRefresh B Σ Σ
  disc  : ReachableByRefresh B Σ Σ0 ->
          ReachableByRefresh B Σ0 (stepDiscovery B Σ0 disc_uri r) ->
          ReachableByRefresh B Σ (stepDiscovery B Σ0 disc_uri r)
  jwks  : ...
  rev   : ...
```

Any equivalent reflexive-transitive closure presentation is acceptable.

## Rotation discipline

Stability across refresh is not automatic.
It requires an issuer-side doctrine:

> a signing key must remain available until every token signed by that key
> has expired, plus skew.

```text
record RotationDiscipline
  (B : JWKSBase)
  : U1
where
  retains_verified_keys :
    (Σ Σ' : Snapshot B) ->
    (tok : JwtToken) ->
    (p : B.Header × B.Claims) ->
    (iss : URI) ->
    (sel : SelectedKey B Σ iss (B.header_alg p.1) (B.header_kid p.1)) ->
    B.parse tok = some p ->
    B.key_verify tok sel.key ->
    (exp : Int64) ->
    B.claims_exp p.2 = some exp ->
    Σ'.now <= exp + B.default_skew ->
    ReachableByRefresh B Σ Σ' ->
    Σ (sel' : SelectedKey B Σ' iss (B.header_alg p.1) (B.header_kid p.1)),
      Unit
```

Derived stability theorem:

```text
theorem verify_sig_stable_under_refresh :
  (B : JWKSBase) ->
  (R : RotationDiscipline B) ->
  (Σ Σ' : Snapshot B) ->
  ReachableByRefresh B Σ Σ' ->
  (tok : JwtToken) ->
  (iss : URI) ->
  (kid : Option String) ->
  (p : B.Header × B.Claims) ->
  B.parse tok = some p ->
  (match B.claims_exp p.2 with
   | none    => Unit
   | some e  => Σ'.now <= e + B.default_skew) ->
  (JWTInfra_of_snapshot B Σ).verify_sig tok iss kid ->
  (JWTInfra_of_snapshot B Σ').verify_sig tok iss kid
```

## Summary

`JWT1-JWKS0` supplies:
- discovery and JWKS resolution
- cache-indexed key selection
- revocation cache semantics
- snapshot-relative execution
- refresh transitions
- an explicit doctrine for rotation stability

This is the point where the identity profile becomes a real trust-runtime spec
without putting network effects inside the checker itself.
