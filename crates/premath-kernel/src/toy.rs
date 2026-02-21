//! Toy bit-worlds for Gate conformance testing.
//!
//! These implement the exact same semantics as the Python toy worlds
//! in the spec's `tools/toy/toy_worlds.py`, allowing us to run the
//! spec's test vectors and produce identical witness IDs.
//!
//! ## Argument conventions
//!
//! Our Rust `restrict(a, src, tgt)` means:
//!   "restrict element `a ∈ Def(src)` to `Def(tgt)` where `tgt ⊆ src`"
//!
//! This maps to the Python's `restrict(src_mask, tgt_mask, a)` as:
//!   Python's `tgt_mask` = our `src` (bigger, where a lives)
//!   Python's `src_mask` = our `tgt` (smaller, where we're restricting to)
//!
//! ## Worlds
//!
//! - **SheafBits**: The golden model. Full sheaf of bit-functions.
//!   `Def(mask)` = total functions `bits(mask) → {0,1}`.
//!   Restriction by projection (drop irrelevant bits).
//!
//! - **BadConstant**: `Def(mask≠∅) = {0,1}`, `Def(∅) = {"*"}`.
//!   Restriction to empty context returns `"*"` (the unique empty-context
//!   inhabitant). This means overlap compatibility passes on disjoint covers,
//!   but no global glue exists (GATE-3.3 failure).
//!
//! - **NonSeparated**: Like BadConstant but restriction always returns 0.
//!   Restriction to empty context returns `"*"`.
//!   Violates uniqueness (both 0 and 1 restrict to local 0).
//!   Overlap passes, but multiple glue candidates exist (GATE-3.4 failure).
//!
//! - **BadStability**: Like BadConstant but with a non-functorial override.
//!   Direct restriction 7→1 returns 0, but via 7→3→1 returns 1.
//!
//! - **PartialRestrict**: Like BadConstant but restriction to singletons
//!   is undefined. Tests locality failures.

use crate::gate::World;
use serde_json::Value;

/// The empty-context inhabitant for simple constant worlds.
///
/// v7: Uses `"*"` (JSON string) instead of `null` to disambiguate from
/// "restriction undefined" (which returns `None`/Option::None).
fn star() -> Value {
    Value::String("*".to_string())
}

/// Enumerate for simple constant worlds: Def(0) = {"*"}, Def(mask≠0) = {0, 1}.
fn simple_enumerate(gamma: u64) -> Option<Vec<Value>> {
    if gamma == 0 {
        Some(vec![star()])
    } else {
        Some(vec![Value::Number(0.into()), Value::Number(1.into())])
    }
}

/// Get a toy world by name (matching fixture "world" field).
pub fn get_world(name: &str) -> Option<Box<dyn World>> {
    match name {
        "sheaf_bits" => Some(Box::new(SheafBits)),
        "bad_constant" => Some(Box::new(BadConstant)),
        "non_separated" => Some(Box::new(NonSeparated)),
        "bad_stability" => Some(Box::new(BadStability)),
        "partial_restrict" => Some(Box::new(PartialRestrict)),
        _ => None,
    }
}

/// Extract the bit indices that are set in a mask.
fn bits_of(mask: u64) -> Vec<u64> {
    let mut bits = Vec::new();
    let mut m = mask;
    let mut i = 0;
    while m > 0 {
        if m & 1 == 1 {
            bits.push(i);
        }
        m >>= 1;
        i += 1;
    }
    bits
}

// ─── SheafBits ──────────────────────────────────────────────────────────────

/// The golden model: full sheaf of bit-functions.
///
/// `Def(mask)` = all total functions from `bits(mask) → {0,1}`.
/// Represented as JSON objects `{"0": 1, "1": 0, ...}`.
/// Restriction = projection (keep only bits present in target mask).
///
/// This is the unique model where every gate check passes.
pub struct SheafBits;

impl World for SheafBits {
    fn name(&self) -> &str {
        "sheaf_bits"
    }

    fn is_definable(&self, gamma: u64, a: &Value) -> bool {
        match a {
            Value::Object(map) => {
                let expected_bits = bits_of(gamma);
                if map.len() != expected_bits.len() {
                    return false;
                }
                for bit in &expected_bits {
                    let key = bit.to_string();
                    match map.get(&key) {
                        Some(Value::Number(n)) => {
                            if let Some(v) = n.as_u64() {
                                if v > 1 {
                                    return false;
                                }
                            } else {
                                return false;
                            }
                        }
                        _ => return false,
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn restrict(&self, a: &Value, _src: u64, tgt: u64) -> Option<Value> {
        // Project a onto the bits of tgt.
        // For tgt=0, bits_of(0) = [] → returns empty object {} which is Def(0).
        match a {
            Value::Object(map) => {
                let tgt_bits = bits_of(tgt);
                let mut result = serde_json::Map::new();
                for bit in &tgt_bits {
                    let key = bit.to_string();
                    match map.get(&key) {
                        Some(v) => {
                            result.insert(key, v.clone());
                        }
                        None => return None, // key not present → restriction undefined
                    }
                }
                Some(Value::Object(result))
            }
            _ => None,
        }
    }

    fn same(&self, _gamma: u64, a: &Value, b: &Value) -> bool {
        a == b
    }

    fn is_cover(&self, gamma: u64, legs: &[u64]) -> bool {
        let union: u64 = legs.iter().fold(0, |acc, &leg| acc | leg);
        (union & gamma) == gamma
    }

    fn overlap(&self, leg_i: u64, leg_j: u64) -> u64 {
        leg_i & leg_j
    }

    fn enumerate(&self, gamma: u64) -> Option<Vec<Value>> {
        let bits = bits_of(gamma);
        let n = bits.len();
        // All 2^n total functions from bits(gamma) → {0,1}
        let count = 1u64 << n;
        let mut result = Vec::with_capacity(count as usize);
        for x in 0..count {
            let mut map = serde_json::Map::new();
            for (i, bit) in bits.iter().enumerate() {
                let val = (x >> i) & 1;
                map.insert(bit.to_string(), Value::Number(val.into()));
            }
            result.push(Value::Object(map));
        }
        Some(result)
    }
}

// ─── BadConstant ────────────────────────────────────────────────────────────

/// `Def(mask≠0) = {0, 1}` as plain integers, `Def(0) = {"*"}`.
///
/// Restriction to empty context returns `"*"` (the unique inhabitant), so
/// overlap compatibility on disjoint covers passes (both sides restrict to `"*"`),
/// but no global candidate restricts to mismatched locals → GATE-3.3 failure.
pub struct BadConstant;

impl World for BadConstant {
    fn name(&self) -> &str {
        "bad_constant"
    }

    fn is_definable(&self, gamma: u64, a: &Value) -> bool {
        if gamma == 0 {
            a == &star()
        } else {
            a.is_number()
        }
    }

    fn restrict(&self, a: &Value, _src: u64, tgt: u64) -> Option<Value> {
        // Restriction to empty context returns the unique inhabitant "*"
        if tgt == 0 {
            return Some(star());
        }
        // Otherwise identity
        Some(a.clone())
    }

    fn same(&self, _gamma: u64, a: &Value, b: &Value) -> bool {
        a == b
    }

    fn is_cover(&self, gamma: u64, legs: &[u64]) -> bool {
        let union: u64 = legs.iter().fold(0, |acc, &leg| acc | leg);
        (union & gamma) == gamma
    }

    fn overlap(&self, leg_i: u64, leg_j: u64) -> u64 {
        leg_i & leg_j
    }

    fn enumerate(&self, gamma: u64) -> Option<Vec<Value>> {
        simple_enumerate(gamma)
    }
}

// ─── NonSeparated ───────────────────────────────────────────────────────────

/// Like BadConstant, but restriction to non-empty always returns 0.
/// Restriction to empty context returns `"*"`.
///
/// This world violates uniqueness (non-contractible glue space):
/// - Both globals 0 and 1 restrict to local 0 on any non-empty context
/// - Overlap compatibility passes (both restrict to `"*"` on empty overlap)
/// - But two candidates glue → GATE-3.4 glue_non_contractible failure
pub struct NonSeparated;

impl World for NonSeparated {
    fn name(&self) -> &str {
        "non_separated"
    }

    fn is_definable(&self, gamma: u64, a: &Value) -> bool {
        if gamma == 0 {
            a == &star()
        } else {
            a.is_number()
        }
    }

    fn restrict(&self, _a: &Value, _src: u64, tgt: u64) -> Option<Value> {
        // Restriction to empty context returns "*"
        if tgt == 0 {
            return Some(star());
        }
        // For non-empty targets, always return constant 0
        Some(Value::Number(0.into()))
    }

    fn same(&self, _gamma: u64, a: &Value, b: &Value) -> bool {
        a == b
    }

    fn is_cover(&self, gamma: u64, legs: &[u64]) -> bool {
        let union: u64 = legs.iter().fold(0, |acc, &leg| acc | leg);
        (union & gamma) == gamma
    }

    fn overlap(&self, leg_i: u64, leg_j: u64) -> u64 {
        leg_i & leg_j
    }

    fn enumerate(&self, gamma: u64) -> Option<Vec<Value>> {
        simple_enumerate(gamma)
    }
}

// ─── BadStability ───────────────────────────────────────────────────────────

/// Like BadConstant, but with a non-functorial override:
/// Direct restriction from context 7 to context 1 always returns 0,
/// even though composing via context 3 would give a different result.
///
/// This violates the composition law: (f∘g)* ≠ g*(f*).
pub struct BadStability;

impl World for BadStability {
    fn name(&self) -> &str {
        "bad_stability"
    }

    fn is_definable(&self, gamma: u64, a: &Value) -> bool {
        if gamma == 0 {
            a.is_null()
        } else {
            a.is_number()
        }
    }

    fn restrict(&self, a: &Value, src: u64, tgt: u64) -> Option<Value> {
        // Restriction to empty context is undefined
        if tgt == 0 {
            return None;
        }

        // Non-functorial override: src=7, tgt=1 always gives 0
        if src == 7 && tgt == 1 {
            return Some(Value::Number(0.into()));
        }

        // Otherwise identity
        Some(a.clone())
    }

    fn same(&self, _gamma: u64, a: &Value, b: &Value) -> bool {
        a == b
    }

    fn is_cover(&self, gamma: u64, legs: &[u64]) -> bool {
        let union: u64 = legs.iter().fold(0, |acc, &leg| acc | leg);
        (union & gamma) == gamma
    }

    fn overlap(&self, leg_i: u64, leg_j: u64) -> u64 {
        leg_i & leg_j
    }

    fn enumerate(&self, gamma: u64) -> Option<Vec<Value>> {
        simple_enumerate(gamma)
    }
}

// ─── PartialRestrict ────────────────────────────────────────────────────────

/// Like BadConstant, but restriction to singleton contexts (popcount=1)
/// is undefined. Tests locality failures.
pub struct PartialRestrict;

impl World for PartialRestrict {
    fn name(&self) -> &str {
        "partial_restrict"
    }

    fn is_definable(&self, gamma: u64, a: &Value) -> bool {
        if gamma == 0 {
            a.is_null()
        } else {
            a.is_number()
        }
    }

    fn restrict(&self, a: &Value, _src: u64, tgt: u64) -> Option<Value> {
        // Restriction to empty context is undefined
        if tgt == 0 {
            return None;
        }

        // Restriction to singleton (exactly one bit set) is undefined
        if tgt.count_ones() == 1 {
            return None;
        }

        // Otherwise identity
        Some(a.clone())
    }

    fn same(&self, _gamma: u64, a: &Value, b: &Value) -> bool {
        a == b
    }

    fn is_cover(&self, gamma: u64, legs: &[u64]) -> bool {
        let union: u64 = legs.iter().fold(0, |acc, &leg| acc | leg);
        (union & gamma) == gamma
    }

    fn overlap(&self, leg_i: u64, leg_j: u64) -> u64 {
        leg_i & leg_j
    }

    fn enumerate(&self, gamma: u64) -> Option<Vec<Value>> {
        simple_enumerate(gamma)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::{GateCheck, run_gate_check};

    // ─── Golden tests (should pass) ─────────────────────────────────────

    #[test]
    fn golden_stability_sheaf_bits() {
        let world = SheafBits;
        // From golden_stability_sheaf_bits/case.json
        let a = serde_json::json!({"0": 1, "1": 0, "2": 1});
        let check = GateCheck::Stability {
            gamma_mask: 7,
            a,
            f_src: 3,
            f_tgt: 7,
            g_src: 1,
            g_tgt: 3,
            token_path: None,
        };
        let result = run_gate_check(&world, &check, "toy");
        assert!(
            result.is_accepted(),
            "golden stability should pass: {:?}",
            result
        );
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::json!({
                "witnessSchema": 1,
                "profile": "toy",
                "result": "accepted",
                "failures": []
            })
        );
    }

    #[test]
    fn golden_descent_sheaf_bits() {
        let world = SheafBits;
        // From golden_descent_sheaf_bits/case.json
        let locals = vec![
            serde_json::json!({"0": 1, "1": 0}),
            serde_json::json!({"1": 0, "2": 1}),
        ];
        let check = GateCheck::Descent {
            base_mask: 7,
            legs: vec![3, 6],
            locals,
            token_path: None,
            glue: None,
            overlap_certified: false,
            cocycle_certified: false,
            contractible_certified: false,
            contractible_scheme_id: None,
            contractible_proof: None,
        };
        let result = run_gate_check(&world, &check, "toy");
        assert!(
            result.is_accepted(),
            "golden descent should pass: {:?}",
            result
        );
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::json!({
                "witnessSchema": 1,
                "profile": "toy",
                "result": "accepted",
                "failures": []
            })
        );
    }

    // ─── Adversarial tests (should fail with exact witness IDs) ──────────

    #[test]
    fn adversarial_stability_failure_bad_stability() {
        let world = BadStability;
        // From adversarial_stability_failure_bad_stability/case.json
        let check = GateCheck::Stability {
            gamma_mask: 7,
            a: Value::Number(1.into()),
            f_src: 3,
            f_tgt: 7,
            g_src: 1,
            g_tgt: 3,
            token_path: None,
        };
        let result = run_gate_check(&world, &check, "toy");
        assert!(!result.is_accepted());

        // Verify exact output matches spec fixture
        let expected = serde_json::json!({
            "witnessSchema": 1,
            "profile": "toy",
            "result": "rejected",
            "failures": [{
                "witnessId": "w1_l1v24u75j3sudbdnhflh4l6sb29ii02vosfj2cm5m3qh0rg8ab30",
                "class": "stability_failure",
                "lawRef": "GATE-3.1",
                "message": "composition law failed: (f o g)* != g*(f*)",
                "context": {"mask": 1},
                "tokenPath": null
            }]
        });
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            expected,
            "must match spec fixture exactly"
        );
    }

    #[test]
    fn adversarial_locality_failure_partial_restrict() {
        let world = PartialRestrict;
        // From adversarial_locality_failure_partial_restrict/case.json
        let check = GateCheck::Locality {
            gamma_mask: 3,
            a: Value::Number(1.into()),
            legs: vec![1, 2],
            token_path: None,
        };
        let result = run_gate_check(&world, &check, "toy");
        assert!(!result.is_accepted());

        let expected = serde_json::json!({
            "witnessSchema": 1,
            "profile": "toy",
            "result": "rejected",
            "failures": [{
                "witnessId": "w1_v9grre901frguq5nv7g8i8dtlf3uujt05i2lkmh8i5bdfdod4q90",
                "class": "locality_failure",
                "lawRef": "GATE-3.2",
                "message": "restriction along a cover leg is undefined or ill-typed",
                "context": {"mask": 1},
                "tokenPath": null
            }]
        });
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            expected,
            "must match spec fixture exactly"
        );
    }

    #[test]
    fn adversarial_descent_failure_bad_constant() {
        let world = BadConstant;
        // From adversarial_descent_failure_bad_constant/case.json
        // legs=[1,2] are disjoint, overlap = 1&2 = 0
        // v7: restriction to empty context returns "*" → overlap passes
        // But no global candidate (0 or 1) restricts to both locals (0 and 1) → GATE-3.3
        let check = GateCheck::Descent {
            base_mask: 3,
            legs: vec![1, 2],
            locals: vec![Value::Number(0.into()), Value::Number(1.into())],
            token_path: None,
            glue: None,
            overlap_certified: false,
            cocycle_certified: false,
            contractible_certified: false,
            contractible_scheme_id: None,
            contractible_proof: None,
        };
        let result = run_gate_check(&world, &check, "toy");
        assert!(!result.is_accepted());

        let expected = serde_json::json!({
            "witnessSchema": 1,
            "profile": "toy",
            "result": "rejected",
            "failures": [{
                "witnessId": "w1_sgjilie1aln83eq8oh0gbmois0f6gtbsppfplk8j26vps7qmu2fg",
                "class": "descent_failure",
                "lawRef": "GATE-3.3",
                "message": "no global glue exists for compatible local data",
                "context": {"mask": 3},
                "tokenPath": null
            }]
        });
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            expected,
            "must match spec fixture exactly"
        );
    }

    #[test]
    fn adversarial_glue_non_contractible_non_separated() {
        let world = NonSeparated;
        // From adversarial_glue_non_contractible_non_separated/case.json
        // legs=[1,2] are disjoint, overlap = 1&2 = 0
        // v7: restriction to empty context returns "*" → overlap passes
        // Both candidates (0, 1) restrict to local 0 on both legs → 2 candidates
        // → GATE-3.4 glue_non_contractible
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
        let result = run_gate_check(&world, &check, "toy");
        assert!(!result.is_accepted());

        let expected = serde_json::json!({
            "witnessSchema": 1,
            "profile": "toy",
            "result": "rejected",
            "failures": [{
                "witnessId": "w1_c0kubjophjuo3h5cr03elvc75606hoba3v4ksii3ktvpgr0enh9g",
                "class": "glue_non_contractible",
                "lawRef": "GATE-3.4",
                "message": "multiple global glues exist for the same descent datum",
                "context": {"mask": 3},
                "tokenPath": null
            }]
        });
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            expected,
            "must match spec fixture exactly"
        );
    }

    // ─── Fixture parser tests ───────────────────────────────────────────

    #[test]
    fn parse_stability_fixture() {
        let fixture = serde_json::json!({
            "schema": 1,
            "world": "sheaf_bits",
            "check": {
                "kind": "stability",
                "gammaMask": 7,
                "a": {"0": 1, "1": 0, "2": 1},
                "f": {"src": 3, "tgt": 7},
                "g": {"src": 1, "tgt": 3},
                "tokenPath": null
            }
        });
        let check = GateCheck::from_fixture(fixture.get("check").unwrap());
        assert!(check.is_some());
    }

    #[test]
    fn parse_descent_fixture() {
        let fixture = serde_json::json!({
            "schema": 1,
            "world": "sheaf_bits",
            "check": {
                "kind": "descent",
                "baseMask": 7,
                "legs": [3, 6],
                "locals": [{"0": 1, "1": 0}, {"1": 0, "2": 1}],
                "tokenPath": null
            }
        });
        let check = GateCheck::from_fixture(fixture.get("check").unwrap());
        assert!(check.is_some());
    }

    // ─── World unit tests ───────────────────────────────────────────────

    #[test]
    fn sheaf_bits_restrict_projection() {
        let world = SheafBits;
        let a = serde_json::json!({"0": 1, "1": 0, "2": 1});

        // Restrict from mask 7 (bits 0,1,2) to mask 3 (bits 0,1)
        let r = world.restrict(&a, 7, 3).unwrap();
        assert_eq!(r, serde_json::json!({"0": 1, "1": 0}));

        // Restrict from mask 7 to mask 1 (bit 0 only)
        let r = world.restrict(&a, 7, 1).unwrap();
        assert_eq!(r, serde_json::json!({"0": 1}));

        // Restrict to mask 0 → empty object
        let r = world.restrict(&a, 7, 0).unwrap();
        assert_eq!(r, serde_json::json!({}));
    }

    #[test]
    fn sheaf_bits_composition_law() {
        // (f∘g)* A = g*(f* A) should hold for sheaf_bits
        let world = SheafBits;
        let a = serde_json::json!({"0": 1, "1": 0, "2": 1});

        // Direct: restrict from 7 to 1
        let direct = world.restrict(&a, 7, 1).unwrap();

        // Staged: restrict from 7 to 3, then from 3 to 1
        let intermediate = world.restrict(&a, 7, 3).unwrap();
        let staged = world.restrict(&intermediate, 3, 1).unwrap();

        assert_eq!(direct, staged, "composition law must hold for sheaf_bits");
    }

    #[test]
    fn bad_stability_composition_fails() {
        let world = BadStability;
        let a = Value::Number(1.into());

        // Direct: restrict from 7 to 1 → forced to 0
        let direct = world.restrict(&a, 7, 1).unwrap();
        assert_eq!(direct, Value::Number(0.into()));

        // Staged: 7→3 (identity) then 3→1 (identity)
        let intermediate = world.restrict(&a, 7, 3).unwrap();
        assert_eq!(intermediate, Value::Number(1.into()));
        let staged = world.restrict(&intermediate, 3, 1).unwrap();
        assert_eq!(staged, Value::Number(1.into()));

        // Composition law fails: 0 ≠ 1
        assert_ne!(direct, staged);
    }

    #[test]
    fn partial_restrict_singletons_undefined() {
        let world = PartialRestrict;
        let a = Value::Number(1.into());

        // Singleton contexts (popcount=1) are undefined
        assert!(world.restrict(&a, 3, 1).is_none());
        assert!(world.restrict(&a, 3, 2).is_none());

        // Multi-bit contexts are fine
        assert!(world.restrict(&a, 7, 3).is_some());
    }

    #[test]
    fn non_separated_restriction_constant() {
        let world = NonSeparated;

        // Non-empty → always 0
        assert_eq!(
            world.restrict(&Value::Number(1.into()), 3, 1),
            Some(Value::Number(0.into()))
        );
        assert_eq!(
            world.restrict(&Value::Number(0.into()), 3, 1),
            Some(Value::Number(0.into()))
        );

        // Empty → "*"
        assert_eq!(
            world.restrict(&Value::Number(0.into()), 3, 0),
            Some(Value::String("*".to_string()))
        );
    }

    // ─── Golden cocycle test (3 legs, triple overlap) ───────────────

    #[test]
    fn golden_descent_sheaf_bits_cocycle() {
        let world = SheafBits;
        // From golden_descent_sheaf_bits_cocycle/case.json
        // baseMask=15, legs=[7,11,13], 3 locals with matching overlaps
        let locals = vec![
            serde_json::json!({"0": 1, "1": 0, "2": 1}),
            serde_json::json!({"0": 1, "1": 0, "3": 0}),
            serde_json::json!({"0": 1, "2": 1, "3": 0}),
        ];
        let check = GateCheck::Descent {
            base_mask: 15,
            legs: vec![7, 11, 13],
            locals,
            token_path: None,
            glue: None,
            overlap_certified: false,
            cocycle_certified: false,
            contractible_certified: false,
            contractible_scheme_id: None,
            contractible_proof: None,
        };
        let result = run_gate_check(&world, &check, "toy");
        assert!(
            result.is_accepted(),
            "golden cocycle descent should pass: {:?}",
            result
        );
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::json!({
                "witnessSchema": 1,
                "profile": "toy",
                "result": "accepted",
                "failures": []
            })
        );
    }
}
