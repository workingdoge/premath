//! Admissibility Gate per GATE.md.
//!
//! The Gate is the verifier: given a constructor K = (C, Cov, Def, ~, Reindex),
//! it checks whether a judgment Γ ⊢ A satisfies all admissibility laws:
//!
//! - §3.1 Stability (functorial reindexing)
//! - §3.2 Locality (cover restriction)
//! - §3.3 Descent (gluing existence)
//! - §3.4 Uniqueness (contractible glue space)
//! - §3.5 Adjoint triple coherence (Sigma/Pi, optional)
//!
//! The Gate is parameterized by a `World` trait that provides the constructor
//! interface. This allows the same Gate logic to work with the toy bit-worlds
//! (for testing) and with real bd/JJ backends.

use crate::witness::{GateFailure, GateResult, failure_class, law_ref};
use serde_json::Value;

/// The constructor interface K = (C, Cov, Def, ~, Reindex).
///
/// A World provides all the operations the Gate needs to check admissibility.
/// This is deliberately abstract — the toy bit-worlds implement it with
/// bitmasks and lookup tables; the real bd backend implements it with
/// JJ repos and SurrealDB.
///
/// All operations are parameterized by bitmask-style context identifiers
/// (u64) for simplicity. Real implementations may use richer context types.
pub trait World {
    /// Name of this world (for diagnostics).
    fn name(&self) -> &str;

    /// Check if element `a` is a valid definable in context `gamma`.
    fn is_definable(&self, gamma: u64, a: &Value) -> bool;

    /// Reindex/restrict: pull definable `a` from context `src` to context `tgt`.
    ///
    /// Returns None if restriction is undefined (locality failure).
    /// `tgt` must be a sub-context of `src` (tgt ⊆ src in bitmask terms).
    fn restrict(&self, a: &Value, src: u64, tgt: u64) -> Option<Value>;

    /// Sameness check: are `a` and `b` the same definable in context `gamma`?
    fn same(&self, gamma: u64, a: &Value, b: &Value) -> bool;

    /// Check if a cover (set of legs) is valid for context `gamma`.
    fn is_cover(&self, gamma: u64, legs: &[u64]) -> bool;

    /// Compute the overlap context for two cover legs.
    /// In bitmask terms: leg_i & leg_j.
    fn overlap(&self, leg_i: u64, leg_j: u64) -> u64;

    /// Enumerate all definables in context `gamma`.
    ///
    /// Only feasible for toy/small worlds. Used for GATE-3.3/3.4
    /// descent existence + contractibility checking (global candidate
    /// enumeration). Real backends may implement this as a query.
    ///
    /// Returns None if enumeration is not supported (the gate will
    /// reject non-certified descent checks because it cannot prove
    /// existence/contractibility.
    fn enumerate(&self, gamma: u64) -> Option<Vec<Value>> {
        let _ = gamma;
        None
    }

    /// Whether this world advertises adjoint-triple structure (Sigma/f*/Pi).
    ///
    /// If false, adjoint-triple checks are out of scope for this world/profile.
    fn advertises_adjoint_triple(&self) -> bool {
        false
    }

    /// Verify adjoint-triple coherence for this world/profile.
    ///
    /// Implementations that advertise adjoint triples should return:
    /// - `Ok(())` when coherent
    /// - `Err(message)` when incoherent
    ///
    /// The default error is used if a caller asks for adjoint checks against
    /// a world that does not implement them.
    fn check_adjoint_triple(&self) -> Result<(), String> {
        Err("adjoint-triple checking is not implemented for this world".to_string())
    }
}

/// Gate check kinds matching the toy fixture format.
#[derive(Debug, Clone)]
pub enum GateCheck {
    /// GATE-3.1: Stability check.
    /// Verify (f ∘ g)* A ~ g*(f* A) for morphisms f, g and element a.
    Stability {
        gamma_mask: u64,
        a: Value,
        f_src: u64,
        f_tgt: u64,
        g_src: u64,
        g_tgt: u64,
        token_path: Option<String>,
    },

    /// GATE-3.2: Locality check.
    /// Verify that restriction along each cover leg is defined.
    Locality {
        gamma_mask: u64,
        a: Value,
        legs: Vec<u64>,
        token_path: Option<String>,
    },

    /// GATE-3.3 + 3.4: Descent check.
    /// Verify overlap compatibility, cocycle coherence, and gluing existence/uniqueness.
    Descent {
        base_mask: u64,
        legs: Vec<u64>,
        locals: Vec<Value>,
        token_path: Option<String>,
        /// Optional glue witness (v6: proof-carrying descent trace).
        /// When provided, the gate validates it matches the unique glue candidate.
        glue: Option<Value>,
        /// v7: When true, skip pairwise overlap compatibility check
        /// (already certified by O_ASSERT_OVERLAP certificates).
        overlap_certified: bool,
        /// v7: When true, skip triple-overlap cocycle coherence check
        /// (already certified by O_ASSERT_TRIPLE certificates).
        cocycle_certified: bool,
        /// v8: When true, skip brute-force enumeration for uniqueness.
        /// The KCIR layer has already verified contractibility via
        /// O_ASSERT_CONTRACTIBLE. The gate validates the glue witness
        /// restricts correctly to locals (existence sanity) instead.
        contractible_certified: bool,
        /// v10: Optional proof-scheme label for certified contractibility.
        /// Required when `contractible_certified` is true.
        contractible_scheme_id: Option<String>,
        /// v10: Opaque proof payload for the selected scheme.
        /// Required when `contractible_certified` is true.
        contractible_proof: Option<String>,
    },

    /// GATE-3.5: Adjoint-triple coherence (optional capability).
    ///
    /// This check is only meaningful when the world/profile advertises
    /// Sigma/f*/Pi support.
    AdjointTriple { token_path: Option<String> },
}

impl GateCheck {
    /// Parse a gate check from the toy fixture JSON format.
    pub fn from_fixture(check: &Value) -> Option<Self> {
        let kind = check.get("kind")?.as_str()?;
        let token_path = check
            .get("tokenPath")
            .and_then(|v| v.as_str())
            .map(String::from);

        match kind {
            "stability" => {
                let gamma_mask = check.get("gammaMask")?.as_u64()?;
                let a = check.get("a")?.clone();
                let f = check.get("f")?;
                let g = check.get("g")?;
                Some(GateCheck::Stability {
                    gamma_mask,
                    a,
                    f_src: f.get("src")?.as_u64()?,
                    f_tgt: f.get("tgt")?.as_u64()?,
                    g_src: g.get("src")?.as_u64()?,
                    g_tgt: g.get("tgt")?.as_u64()?,
                    token_path,
                })
            }
            "locality" => {
                let gamma_mask = check.get("gammaMask")?.as_u64()?;
                let a = check.get("a")?.clone();
                let legs = check
                    .get("legs")?
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_u64())
                    .collect();
                Some(GateCheck::Locality {
                    gamma_mask,
                    a,
                    legs,
                    token_path,
                })
            }
            "descent" => {
                let base_mask = check.get("baseMask")?.as_u64()?;
                let legs = check
                    .get("legs")?
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_u64())
                    .collect();
                let locals = check.get("locals")?.as_array()?.clone();
                // v6: optional glue witness
                let glue = check.get("glue").cloned().filter(|v| !v.is_null());
                // v7: certified overlap/cocycle flags
                let overlap_certified = check
                    .get("overlapCertified")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let cocycle_certified = check
                    .get("cocycleCertified")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                // v8: certified contractibility flag
                let contractible_certified = check
                    .get("contractibleCertified")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let contractible_scheme_id = check
                    .get("contractibleSchemeId")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let contractible_proof = check
                    .get("contractibleProof")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(GateCheck::Descent {
                    base_mask,
                    legs,
                    locals,
                    token_path,
                    glue,
                    overlap_certified,
                    cocycle_certified,
                    contractible_certified,
                    contractible_scheme_id,
                    contractible_proof,
                })
            }
            "adjoint_triple" => Some(GateCheck::AdjointTriple { token_path }),
            _ => None,
        }
    }
}

/// Run a Gate check against a World.
///
/// Returns a GateResult conforming to GATE.md §4.
pub fn run_gate_check(world: &dyn World, check: &GateCheck, profile: &str) -> GateResult {
    match check {
        GateCheck::Stability {
            gamma_mask,
            a,
            f_src,
            f_tgt,
            g_src,
            g_tgt,
            token_path,
        } => check_stability(
            world,
            profile,
            *gamma_mask,
            a,
            *f_src,
            *f_tgt,
            *g_src,
            *g_tgt,
            token_path.as_deref(),
        ),

        GateCheck::Locality {
            gamma_mask,
            a,
            legs,
            token_path,
        } => check_locality(world, profile, *gamma_mask, a, legs, token_path.as_deref()),

        GateCheck::Descent {
            base_mask,
            legs,
            locals,
            token_path,
            glue,
            overlap_certified,
            cocycle_certified,
            contractible_certified,
            contractible_scheme_id,
            contractible_proof,
        } => check_descent(
            world,
            profile,
            *base_mask,
            legs,
            locals,
            token_path.as_deref(),
            glue.as_ref(),
            *overlap_certified,
            *cocycle_certified,
            *contractible_certified,
            contractible_scheme_id.as_deref(),
            contractible_proof.as_deref(),
        ),
        GateCheck::AdjointTriple { token_path } => {
            check_adjoint_triple(world, profile, token_path.as_deref())
        }
    }
}

/// GATE §3.1: Stability (functorial reindexing).
///
/// Check identity law: (id_Γ)* A ~ A
/// Check composition law: (f ∘ g)* A ~ g*(f* A)
#[allow(clippy::too_many_arguments)]
fn check_stability(
    world: &dyn World,
    profile: &str,
    gamma_mask: u64,
    a: &Value,
    f_src: u64,
    f_tgt: u64,
    g_src: u64,
    g_tgt: u64,
    token_path: Option<&str>,
) -> GateResult {
    let mut failures = Vec::new();

    if !world.is_definable(gamma_mask, a) {
        let ctx = serde_json::json!({"mask": gamma_mask});
        failures.push(GateFailure::new(
            failure_class::STABILITY_FAILURE,
            law_ref::STABILITY,
            "input is not a definable in the claimed context",
            token_path.map(String::from),
            Some(ctx),
        ));
        return GateResult::rejected(profile, failures);
    }

    if f_tgt != gamma_mask || g_tgt != f_src {
        let ctx = serde_json::json!({
            "gammaMask": gamma_mask,
            "f": {"src": f_src, "tgt": f_tgt},
            "g": {"src": g_src, "tgt": g_tgt}
        });
        failures.push(GateFailure::new(
            failure_class::STABILITY_FAILURE,
            law_ref::STABILITY,
            "invalid morphism chain for composition check",
            token_path.map(String::from),
            Some(ctx),
        ));
        return GateResult::rejected(profile, failures);
    }

    // Identity law: (id_Γ)* A ~ A
    match world.restrict(a, gamma_mask, gamma_mask) {
        Some(ref id_restricted) => {
            if !world.same(gamma_mask, id_restricted, a) {
                let ctx = serde_json::json!({"mask": gamma_mask});
                failures.push(GateFailure::new(
                    failure_class::STABILITY_FAILURE,
                    law_ref::STABILITY,
                    "identity law failed: (id)* != original",
                    token_path.map(String::from),
                    Some(ctx),
                ));
            }
        }
        None => {
            let ctx = serde_json::json!({"mask": gamma_mask});
            failures.push(GateFailure::new(
                failure_class::STABILITY_FAILURE,
                law_ref::STABILITY,
                "restriction undefined for identity morphism",
                token_path.map(String::from),
                Some(ctx),
            ));
        }
    }

    if !failures.is_empty() {
        return GateResult::rejected(profile, failures);
    }

    // Composition law: (f ∘ g)* A ~ g*(f* A)
    // f: f_src → f_tgt, g: g_src → g_tgt
    // f ∘ g: g_src → f_tgt (first g, then f)
    //
    // Direct path: restrict a from gamma_mask to g_src directly
    // (this represents (f∘g)* since g_src is the ultimate source)
    let direct = world.restrict(a, gamma_mask, g_src);

    // Staged path: first restrict to f_src, then restrict to g_src
    // This represents g*(f* A)
    let staged = world
        .restrict(a, gamma_mask, f_src)
        .and_then(|intermediate| world.restrict(&intermediate, f_src, g_src));

    match (direct, staged) {
        (Some(ref d), Some(ref s)) => {
            if !world.same(g_src, d, s) {
                let ctx = serde_json::json!({"mask": g_src});
                failures.push(GateFailure::new(
                    failure_class::STABILITY_FAILURE,
                    law_ref::STABILITY,
                    "composition law failed: (f o g)* != g*(f*)",
                    token_path.map(String::from),
                    Some(ctx),
                ));
            }
        }
        (None, _) | (_, None) => {
            // If restriction is undefined, that's also a stability failure
            // (the functor isn't even defined)
            let ctx = serde_json::json!({"mask": g_src});
            failures.push(GateFailure::new(
                failure_class::STABILITY_FAILURE,
                law_ref::STABILITY,
                "restriction undefined in composition path",
                token_path.map(String::from),
                Some(ctx),
            ));
        }
    }

    if failures.is_empty() {
        GateResult::accepted(profile)
    } else {
        GateResult::rejected(profile, failures)
    }
}

/// GATE §3.2: Locality (cover restriction).
///
/// For every cover leg u_i, restriction u_i* A MUST exist.
fn check_locality(
    world: &dyn World,
    profile: &str,
    gamma_mask: u64,
    a: &Value,
    legs: &[u64],
    token_path: Option<&str>,
) -> GateResult {
    let mut failures = Vec::new();

    if !world.is_definable(gamma_mask, a) {
        let ctx = serde_json::json!({"mask": gamma_mask});
        failures.push(GateFailure::new(
            failure_class::LOCALITY_FAILURE,
            law_ref::LOCALITY,
            "input is not a definable in the claimed context",
            token_path.map(String::from),
            Some(ctx),
        ));
        return GateResult::rejected(profile, failures);
    }

    if !world.is_cover(gamma_mask, legs) {
        let ctx = serde_json::json!({"mask": gamma_mask});
        failures.push(GateFailure::new(
            failure_class::LOCALITY_FAILURE,
            law_ref::LOCALITY,
            "legs do not form a declared cover for the context",
            token_path.map(String::from),
            Some(ctx),
        ));
        return GateResult::rejected(profile, failures);
    }

    for &leg in legs {
        let restricted = world.restrict(a, gamma_mask, leg);
        if restricted.is_none() {
            let ctx = serde_json::json!({"mask": leg});
            failures.push(GateFailure::new(
                failure_class::LOCALITY_FAILURE,
                law_ref::LOCALITY,
                "restriction along a cover leg is undefined or ill-typed",
                token_path.map(String::from),
                Some(ctx),
            ));
            // Match Python: report first failure only
            break;
        }
    }

    if failures.is_empty() {
        GateResult::accepted(profile)
    } else {
        GateResult::rejected(profile, failures)
    }
}

/// GATE §3.3 + 3.4: Descent (gluing existence + contractible uniqueness).
///
/// Given local definables A_i on cover legs, check:
/// 1. Overlap compatibility: on each pairwise overlap, restrictions agree
/// 2. Cocycle coherence: on each triple overlap, all three restrictions agree
/// 3. Gluing existence: a global A exists
/// 4. Uniqueness: the glue space is contractible
///
/// Steps 1 and 2 can be skipped if `overlap_certified` / `cocycle_certified`
/// flags are set (certificates already verified by the KCIR layer).
#[allow(clippy::too_many_arguments)]
fn check_descent(
    world: &dyn World,
    profile: &str,
    base_mask: u64,
    legs: &[u64],
    locals: &[Value],
    token_path: Option<&str>,
    glue: Option<&Value>,
    overlap_certified: bool,
    cocycle_certified: bool,
    contractible_certified: bool,
    contractible_scheme_id: Option<&str>,
    contractible_proof: Option<&str>,
) -> GateResult {
    let mut failures = Vec::new();
    let base_ctx = serde_json::json!({"mask": base_mask});

    if !world.is_cover(base_mask, legs) {
        failures.push(GateFailure::new(
            failure_class::DESCENT_FAILURE,
            law_ref::DESCENT,
            "legs do not form a declared cover for the base context",
            token_path.map(String::from),
            Some(base_ctx.clone()),
        ));
        return GateResult::rejected(profile, failures);
    }

    if legs.len() != locals.len() {
        failures.push(GateFailure::new(
            failure_class::DESCENT_FAILURE,
            law_ref::DESCENT,
            "legs/locals length mismatch",
            token_path.map(String::from),
            None,
        ));
        return GateResult::rejected(profile, failures);
    }

    for (leg, local) in legs.iter().zip(locals.iter()) {
        if !world.is_definable(*leg, local) {
            let ctx = serde_json::json!({"mask": *leg});
            failures.push(GateFailure::new(
                failure_class::DESCENT_FAILURE,
                law_ref::DESCENT,
                "local value is not definable in its leg context",
                token_path.map(String::from),
                Some(ctx),
            ));
            return GateResult::rejected(profile, failures);
        }
    }

    // Check pairwise overlap compatibility (skip if certified)
    if !overlap_certified {
        'overlap: for i in 0..legs.len() {
            for j in (i + 1)..legs.len() {
                let overlap_ctx = world.overlap(legs[i], legs[j]);

                let ri = world.restrict(&locals[i], legs[i], overlap_ctx);
                let rj = world.restrict(&locals[j], legs[j], overlap_ctx);

                let compatible = match (&ri, &rj) {
                    (Some(a), Some(b)) => world.same(overlap_ctx, a, b),
                    _ => false,
                };

                if !compatible {
                    let ctx = serde_json::json!({"mask": overlap_ctx});
                    failures.push(GateFailure::new(
                        failure_class::DESCENT_FAILURE,
                        law_ref::DESCENT,
                        "overlap compatibility failed",
                        token_path.map(String::from),
                        Some(ctx),
                    ));
                    break 'overlap;
                }
            }
        }
    }

    // If overlap failed, return early
    if !failures.is_empty() {
        return GateResult::rejected(profile, failures);
    }

    // v7: Triple-overlap cocycle coherence check (skip if certified)
    if !cocycle_certified {
        'cocycle: for i in 0..legs.len() {
            for j in (i + 1)..legs.len() {
                for k in (j + 1)..legs.len() {
                    let tri_mask = legs[i] & legs[j] & legs[k];

                    let r1 = world.restrict(&locals[i], legs[i], tri_mask);
                    let r2 = world.restrict(&locals[j], legs[j], tri_mask);
                    let r3 = world.restrict(&locals[k], legs[k], tri_mask);

                    match (&r1, &r2, &r3) {
                        (Some(a), Some(b), Some(c)) => {
                            if !(world.same(tri_mask, a, b) && world.same(tri_mask, b, c)) {
                                let ctx = serde_json::json!({"mask": tri_mask});
                                failures.push(GateFailure::new(
                                    failure_class::DESCENT_FAILURE,
                                    law_ref::DESCENT,
                                    "cocycle coherence failed on triple overlap",
                                    token_path.map(String::from),
                                    Some(ctx),
                                ));
                                break 'cocycle;
                            }
                        }
                        _ => {
                            let ctx = serde_json::json!({"mask": tri_mask});
                            failures.push(GateFailure::new(
                                failure_class::DESCENT_FAILURE,
                                law_ref::DESCENT,
                                "cocycle coherence failed: restriction undefined on triple overlap",
                                token_path.map(String::from),
                                Some(ctx),
                            ));
                            break 'cocycle;
                        }
                    }
                }
            }
        }
    }

    // If cocycle failed, return early
    if !failures.is_empty() {
        return GateResult::rejected(profile, failures);
    }

    // v8: Two paths for existence + uniqueness checking.
    if contractible_certified {
        // Proof-carrying path: contractibility certified by O_ASSERT_CONTRACTIBLE.
        // Validate scheme + proof and then witness existence sanity.
        match glue {
            None => {
                failures.push(GateFailure::new(
                    failure_class::DESCENT_FAILURE,
                    law_ref::DESCENT,
                    "contractibleCertified is set but no glue witness was provided",
                    token_path.map(String::from),
                    Some(base_ctx),
                ));
            }
            Some(witness) => {
                if !world.is_definable(base_mask, witness) {
                    failures.push(GateFailure::new(
                        failure_class::DESCENT_FAILURE,
                        law_ref::DESCENT,
                        "provided glue witness is not definable in the base context",
                        token_path.map(String::from),
                        Some(base_ctx.clone()),
                    ));
                    return GateResult::rejected(profile, failures);
                }

                let scheme_id = match contractible_scheme_id {
                    Some(s) => s,
                    None => {
                        failures.push(GateFailure::new(
                            failure_class::DESCENT_FAILURE,
                            law_ref::DESCENT,
                            "contractibleCertified is set but contractibleSchemeId is missing",
                            token_path.map(String::from),
                            Some(base_ctx.clone()),
                        ));
                        return GateResult::rejected(profile, failures);
                    }
                };

                let proof = match contractible_proof {
                    Some(p) => p,
                    None => {
                        failures.push(GateFailure::new(
                            failure_class::DESCENT_FAILURE,
                            law_ref::DESCENT,
                            "contractibleCertified is set but contractibleProof is missing",
                            token_path.map(String::from),
                            Some(base_ctx.clone()),
                        ));
                        return GateResult::rejected(profile, failures);
                    }
                };

                if !verify_contractible_certificate(
                    world, scheme_id, proof, base_mask, legs, locals, witness,
                ) {
                    failures.push(GateFailure::new(
                        failure_class::GLUE_NON_CONTRACTIBLE,
                        law_ref::UNIQUENESS,
                        "contractibility certificate rejected for the provided scheme",
                        token_path.map(String::from),
                        Some(base_ctx.clone()),
                    ));
                    return GateResult::rejected(profile, failures);
                }

                // Existence sanity: glue witness must restrict to the claimed locals.
                for (leg, local) in legs.iter().zip(locals.iter()) {
                    let r = world.restrict(witness, base_mask, *leg);
                    let ok = match r {
                        Some(ref rv) => world.same(*leg, rv, local),
                        None => false,
                    };
                    if !ok {
                        let leg_ctx = serde_json::json!({"mask": *leg});
                        failures.push(GateFailure::new(
                            failure_class::DESCENT_FAILURE,
                            law_ref::DESCENT,
                            "provided glue witness does not restrict to the local data",
                            token_path.map(String::from),
                            Some(leg_ctx),
                        ));
                        break;
                    }
                }
            }
        }
    } else {
        // Non-certified path: the gate MUST still prove existence + uniqueness.
        let cands = match enumerate_glue_candidates(world, base_mask, legs, locals) {
            Some(cands) => cands,
            None => {
                failures.push(GateFailure::new(
                    failure_class::DESCENT_FAILURE,
                    law_ref::DESCENT,
                    "world cannot enumerate global candidates; contractibility proof is required",
                    token_path.map(String::from),
                    Some(base_ctx.clone()),
                ));
                return GateResult::rejected(profile, failures);
            }
        };

        // GATE-3.3: No global glue exists
        if cands.is_empty() {
            failures.push(GateFailure::new(
                failure_class::DESCENT_FAILURE,
                law_ref::DESCENT,
                "no global glue exists for compatible local data",
                token_path.map(String::from),
                Some(base_ctx),
            ));
        }
        // GATE-3.4: Multiple global glues — glue space is not contractible
        else if cands.len() > 1 {
            failures.push(GateFailure::new(
                failure_class::GLUE_NON_CONTRACTIBLE,
                law_ref::UNIQUENESS,
                "multiple global glues exist for the same descent datum",
                token_path.map(String::from),
                Some(base_ctx),
            ));
        }
        // Optional glue witness validation (when exactly 1 candidate)
        else if cands.len() == 1
            && let Some(witness) = glue
            && !world.same(base_mask, witness, &cands[0])
        {
            failures.push(GateFailure::new(
                failure_class::DESCENT_FAILURE,
                law_ref::DESCENT,
                "glue witness does not match unique candidate",
                token_path.map(String::from),
                Some(base_ctx),
            ));
        }
    }

    if failures.is_empty() {
        GateResult::accepted(profile)
    } else {
        GateResult::rejected(profile, failures)
    }
}

fn check_adjoint_triple(world: &dyn World, profile: &str, token_path: Option<&str>) -> GateResult {
    if !world.advertises_adjoint_triple() {
        // Explicit deterministic rejection: this optional capability is not advertised.
        let failure = GateFailure::new(
            failure_class::ADJOINT_TRIPLE_COHERENCE_FAILURE,
            law_ref::ADJOINT_TRIPLE,
            "adjoint-triple support is not advertised by this world/profile",
            token_path.map(String::from),
            None,
        );
        return GateResult::rejected(profile, vec![failure]);
    }

    match world.check_adjoint_triple() {
        Ok(()) => GateResult::accepted(profile),
        Err(message) => {
            let failure = GateFailure::new(
                failure_class::ADJOINT_TRIPLE_COHERENCE_FAILURE,
                law_ref::ADJOINT_TRIPLE,
                message,
                token_path.map(String::from),
                None,
            );
            GateResult::rejected(profile, vec![failure])
        }
    }
}

fn enumerate_glue_candidates(
    world: &dyn World,
    base_mask: u64,
    legs: &[u64],
    locals: &[Value],
) -> Option<Vec<Value>> {
    let candidates_pool = world.enumerate(base_mask)?;
    let mut candidates = Vec::new();

    for a in &candidates_pool {
        let mut ok = true;
        for (leg, local) in legs.iter().zip(locals.iter()) {
            let r = world.restrict(a, base_mask, *leg);
            match r {
                Some(ref rv) => {
                    if !world.same(*leg, rv, local) {
                        ok = false;
                        break;
                    }
                }
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            candidates.push(a.clone());
        }
    }

    Some(candidates)
}

fn verify_contractible_certificate(
    world: &dyn World,
    scheme_id: &str,
    _proof: &str,
    base_mask: u64,
    legs: &[u64],
    locals: &[Value],
    witness: &Value,
) -> bool {
    match scheme_id {
        // Deterministic toy proof scheme:
        // validate by explicit enumeration and uniqueness checking.
        "toy.enumerate.v1" => {
            let candidates = match enumerate_glue_candidates(world, base_mask, legs, locals) {
                Some(v) => v,
                None => return false,
            };
            if candidates.len() != 1 {
                return false;
            }
            world.same(base_mask, witness, &candidates[0])
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trivial world where everything works: Def(mask) = {0, 1}, restrict = identity.
    struct TrivialWorld;

    impl World for TrivialWorld {
        fn name(&self) -> &str {
            "trivial"
        }

        fn is_definable(&self, _gamma: u64, _a: &Value) -> bool {
            true
        }

        fn restrict(&self, a: &Value, _src: u64, _tgt: u64) -> Option<Value> {
            Some(a.clone())
        }

        fn same(&self, _gamma: u64, a: &Value, b: &Value) -> bool {
            a == b
        }

        fn is_cover(&self, _gamma: u64, _legs: &[u64]) -> bool {
            true
        }

        fn overlap(&self, leg_i: u64, leg_j: u64) -> u64 {
            leg_i & leg_j
        }

        fn enumerate(&self, _gamma: u64) -> Option<Vec<Value>> {
            Some(vec![Value::Number(0.into()), Value::Number(1.into())])
        }
    }

    #[test]
    fn trivial_stability() {
        let world = TrivialWorld;
        let check = GateCheck::Stability {
            gamma_mask: 7,
            a: Value::Number(1.into()),
            f_src: 3,
            f_tgt: 7,
            g_src: 1,
            g_tgt: 3,
            token_path: None,
        };
        let result = run_gate_check(&world, &check, "test");
        assert!(result.is_accepted());
    }

    #[test]
    fn trivial_locality() {
        let world = TrivialWorld;
        let check = GateCheck::Locality {
            gamma_mask: 3,
            a: Value::Number(1.into()),
            legs: vec![1, 2],
            token_path: None,
        };
        let result = run_gate_check(&world, &check, "test");
        assert!(result.is_accepted());
    }

    #[test]
    fn trivial_descent() {
        let world = TrivialWorld;
        let check = GateCheck::Descent {
            base_mask: 3,
            legs: vec![1, 2],
            locals: vec![Value::Number(0.into()), Value::Number(0.into())],
            token_path: None,
            glue: None,
            overlap_certified: false,
            cocycle_certified: false,
            contractible_certified: false,
            contractible_scheme_id: None,
            contractible_proof: None,
        };
        let result = run_gate_check(&world, &check, "test");
        assert!(result.is_accepted());
    }

    #[test]
    fn descent_rejects_without_enumeration_or_certificate() {
        struct NoEnumWorld;
        impl World for NoEnumWorld {
            fn name(&self) -> &str {
                "no-enum"
            }
            fn is_definable(&self, _gamma: u64, _a: &Value) -> bool {
                true
            }
            fn restrict(&self, a: &Value, _src: u64, _tgt: u64) -> Option<Value> {
                Some(a.clone())
            }
            fn same(&self, _gamma: u64, a: &Value, b: &Value) -> bool {
                a == b
            }
            fn is_cover(&self, _gamma: u64, _legs: &[u64]) -> bool {
                true
            }
            fn overlap(&self, leg_i: u64, leg_j: u64) -> u64 {
                leg_i & leg_j
            }
        }

        let world = NoEnumWorld;
        let check = GateCheck::Descent {
            base_mask: 3,
            legs: vec![1, 2],
            locals: vec![Value::Number(0.into()), Value::Number(0.into())],
            token_path: None,
            glue: None,
            overlap_certified: false,
            cocycle_certified: false,
            contractible_certified: false,
            contractible_scheme_id: None,
            contractible_proof: None,
        };
        let result = run_gate_check(&world, &check, "test");
        assert!(!result.is_accepted());
        assert_eq!(result.failures[0].class, failure_class::DESCENT_FAILURE);
    }

    #[test]
    fn adjoint_triple_rejects_when_not_advertised() {
        let world = TrivialWorld;
        let check = GateCheck::AdjointTriple { token_path: None };
        let result = run_gate_check(&world, &check, "test");
        assert!(!result.is_accepted());
        assert_eq!(
            result.failures[0].class,
            failure_class::ADJOINT_TRIPLE_COHERENCE_FAILURE
        );
    }

    #[test]
    fn locality_rejects_invalid_cover() {
        struct InvalidCoverWorld;
        impl World for InvalidCoverWorld {
            fn name(&self) -> &str {
                "invalid-cover"
            }
            fn is_definable(&self, _gamma: u64, _a: &Value) -> bool {
                true
            }
            fn restrict(&self, a: &Value, _src: u64, _tgt: u64) -> Option<Value> {
                Some(a.clone())
            }
            fn same(&self, _gamma: u64, a: &Value, b: &Value) -> bool {
                a == b
            }
            fn is_cover(&self, _gamma: u64, _legs: &[u64]) -> bool {
                false
            }
            fn overlap(&self, leg_i: u64, leg_j: u64) -> u64 {
                leg_i & leg_j
            }
        }

        let world = InvalidCoverWorld;
        let check = GateCheck::Locality {
            gamma_mask: 3,
            a: Value::Number(1.into()),
            legs: vec![1, 2],
            token_path: None,
        };
        let result = run_gate_check(&world, &check, "test");
        assert!(!result.is_accepted());
        assert_eq!(result.failures[0].class, failure_class::LOCALITY_FAILURE);
    }

    #[test]
    fn stability_rejects_non_definable_input() {
        struct NonDefWorld;
        impl World for NonDefWorld {
            fn name(&self) -> &str {
                "non-def"
            }
            fn is_definable(&self, _gamma: u64, _a: &Value) -> bool {
                false
            }
            fn restrict(&self, a: &Value, _src: u64, _tgt: u64) -> Option<Value> {
                Some(a.clone())
            }
            fn same(&self, _gamma: u64, a: &Value, b: &Value) -> bool {
                a == b
            }
            fn is_cover(&self, _gamma: u64, _legs: &[u64]) -> bool {
                true
            }
            fn overlap(&self, leg_i: u64, leg_j: u64) -> u64 {
                leg_i & leg_j
            }
        }

        let world = NonDefWorld;
        let check = GateCheck::Stability {
            gamma_mask: 7,
            a: Value::Number(1.into()),
            f_src: 3,
            f_tgt: 7,
            g_src: 1,
            g_tgt: 3,
            token_path: None,
        };
        let result = run_gate_check(&world, &check, "test");
        assert!(!result.is_accepted());
        assert_eq!(result.failures[0].class, failure_class::STABILITY_FAILURE);
    }
}
