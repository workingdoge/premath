//! Contractible descent: the core Premath axiom.
//!
//! For every cover U ▷ Γ,
//!   res_U: Def(Γ) → Desc_U(Γ) is an equivalence in V.
//!
//! Equivalently: for every descent datum d ∈ Desc_U(Γ), the homotopy fiber
//!   Glue(d) := fib_d(res_U)
//! is contractible (in the sense of V).
//!
//! This single axiom is:
//! - the sheaf condition when V = Set
//! - the stack condition when V = Gpd
//! - the higher stack condition when V = S∞

use crate::coherence::CoherenceLevel;
use crate::context::ContextId;
use crate::definable::{ContentHash, FiberSignature, signatures_compatible};
use crate::error::{Axiom, PremathError, Severity, Violation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// A descent datum over a cover.
///
/// Given a cover U = {u_i: Γ_i → Γ} ▷ Γ, a descent datum consists of:
/// - local definables A_i ∈ Def(Γ_i)
/// - overlap compatibilities φ_ij: p1*A_i ≈ p2*A_j
/// - satisfying cocycle coherence on triple overlaps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescentDatum {
    /// Identifier for the cover this datum is over.
    pub cover_id: String,

    /// The global context being covered.
    pub context_id: ContextId,

    /// Local definables: one fiber signature per patch.
    /// Key is the definable/issue ID.
    pub fibers: BTreeMap<String, FiberSignature>,

    /// Overlap witnesses: compatibility checks between pairs.
    /// Key is "id_a:id_b" (canonically sorted).
    pub overlaps: BTreeMap<String, OverlapWitness>,

    /// Coherence level used for this datum.
    pub level: CoherenceLevel,
}

/// Witness of compatibility (or incompatibility) on an overlap.
///
/// This is φ_ij: p1*A_i ≈ p2*A_j on Γ_i ×_Γ Γ_j.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlapWitness {
    /// First fiber ID.
    pub fiber_a: String,

    /// Second fiber ID.
    pub fiber_b: String,

    /// Shared dependencies forming the overlap.
    pub shared: Vec<String>,

    /// Whether the fibers are compatible on the overlap.
    pub compatible: bool,

    /// Conflict descriptions if not compatible.
    pub conflicts: Vec<String>,
}

impl DescentDatum {
    /// Assemble a descent datum from a collection of fiber signatures.
    ///
    /// Computes all pairwise overlap witnesses.
    pub fn assemble(
        cover_id: impl Into<String>,
        context_id: ContextId,
        fibers: Vec<FiberSignature>,
        level: CoherenceLevel,
    ) -> Self {
        let cover_id = cover_id.into();
        let fiber_map: BTreeMap<String, FiberSignature> =
            fibers.into_iter().map(|f| (f.id.clone(), f)).collect();

        let mut overlaps = BTreeMap::new();

        let ids: Vec<&String> = fiber_map.keys().collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let a = &fiber_map[ids[i]];
                let b = &fiber_map[ids[j]];

                let witness = Self::compute_overlap(a, b, level);

                // Only record overlaps with shared dependencies or conflicts
                if !witness.shared.is_empty() || !witness.conflicts.is_empty() {
                    let key = overlap_key(&witness.fiber_a, &witness.fiber_b);
                    overlaps.insert(key, witness);
                }
            }
        }

        Self {
            cover_id,
            context_id,
            fibers: fiber_map,
            overlaps,
            level,
        }
    }

    /// Check whether this descent datum is effective.
    ///
    /// Effective = all overlaps are compatible = Glue(d) is contractible.
    pub fn is_effective(&self) -> bool {
        self.overlaps.values().all(|w| w.compatible)
    }

    /// Collect all conflicts across all overlaps.
    pub fn conflicts(&self) -> Vec<&str> {
        self.overlaps
            .values()
            .flat_map(|w| w.conflicts.iter().map(String::as_str))
            .collect()
    }

    /// Compute the glue hash: the deterministic fingerprint of the
    /// unique global merge.
    ///
    /// Returns None if descent is not effective (no unique merge exists).
    pub fn glue_hash(&self) -> Option<ContentHash> {
        if !self.is_effective() {
            return None;
        }

        let mut hasher = Sha256::new();
        hasher.update(b"context:");
        hasher.update(self.context_id.0.as_bytes());
        hasher.update(b"\n");

        // Include all fiber structure hashes in sorted order.
        // Intentionally excludes cover_id so semantically equivalent
        // refinements produce the same glue fingerprint.
        // (BTreeMap is already sorted)
        for (id, fiber) in &self.fibers {
            hasher.update(b"fiber:");
            hasher.update(id.as_bytes());
            hasher.update(b":");
            hasher.update(fiber.structure_hash.0.as_bytes());
            hasher.update(b"\n");
        }

        let hash = hasher.finalize();
        Some(ContentHash(format!("{hash:x}")))
    }

    /// Compute the overlap witness between two fibers.
    fn compute_overlap(
        a: &FiberSignature,
        b: &FiberSignature,
        level: CoherenceLevel,
    ) -> OverlapWitness {
        // Find shared edge targets (the overlap)
        let a_targets: std::collections::HashSet<&str> =
            a.edges.iter().map(|e| e.target.as_str()).collect();
        let b_targets: std::collections::HashSet<&str> =
            b.edges.iter().map(|e| e.target.as_str()).collect();

        let shared: Vec<String> = a_targets
            .intersection(&b_targets)
            .map(|s| s.to_string())
            .collect();

        // Check compatibility at the given coherence level
        let compat_result = signatures_compatible(a, b, level);

        match compat_result {
            Ok(()) => OverlapWitness {
                fiber_a: a.id.clone(),
                fiber_b: b.id.clone(),
                shared,
                compatible: true,
                conflicts: vec![],
            },
            Err(conflicts) => OverlapWitness {
                fiber_a: a.id.clone(),
                fiber_b: b.id.clone(),
                shared,
                compatible: false,
                conflicts,
            },
        }
    }
}

/// The result of a full contractibility check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractibilityResult {
    /// The global context being checked.
    pub context_id: ContextId,

    /// Coherence level used.
    pub level: CoherenceLevel,

    /// Per-cover results (`wave` is a conventional cover index label).
    pub waves: Vec<WaveResult>,

    /// Whether ALL covers are contractible.
    pub contractible: bool,

    /// The global glue hash, if contractible.
    pub glue_hash: Option<ContentHash>,

    /// All violations found.
    pub violations: Vec<Violation>,
}

/// Result for a single cover.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveResult {
    /// Cover index (historically named wave).
    pub wave: usize,

    /// The descent datum for this cover.
    pub datum: DescentDatum,

    /// Whether this cover is contractible.
    pub contractible: bool,

    /// Number of fibers in this cover.
    pub fiber_count: usize,

    /// Number of overlaps checked.
    pub overlap_count: usize,

    /// Number of conflicts found.
    pub conflict_count: usize,
}

impl ContractibilityResult {
    /// Build a contractibility result from per-cover results.
    pub fn from_waves(
        context_id: ContextId,
        level: CoherenceLevel,
        waves: Vec<WaveResult>,
    ) -> Self {
        let contractible = waves.iter().all(|w| w.contractible);

        let glue_hash = if contractible {
            // Combine per-cover glue hashes
            let mut hasher = Sha256::new();
            hasher.update(b"global:");
            hasher.update(context_id.0.as_bytes());
            hasher.update(b"\n");
            for w in &waves {
                if let Some(gh) = w.datum.glue_hash() {
                    hasher.update(b"wave:");
                    hasher.update(gh.0.as_bytes());
                    hasher.update(b"\n");
                }
            }
            let hash = hasher.finalize();
            Some(ContentHash(format!("{hash:x}")))
        } else {
            None
        };

        let mut violations = Vec::new();
        for w in &waves {
            for overlap in w.datum.overlaps.values() {
                if !overlap.compatible {
                    for conflict in &overlap.conflicts {
                        violations.push(Violation {
                            axiom: Axiom::Gluing,
                            severity: Severity::Error,
                            context_id: Some(format!("{}:{}", overlap.fiber_a, overlap.fiber_b)),
                            wave: Some(w.wave),
                            description: conflict.clone(),
                        });
                    }
                }
            }
        }

        Self {
            context_id,
            level,
            waves,
            contractible,
            glue_hash,
            violations,
        }
    }
}

/// Detect blocking dependencies internal to the same cover index.
///
/// Elements inside the same cover should not have blocking dependencies
/// on each other. If they do, the cover decomposition is invalid.
pub fn detect_locality_violations(
    wave: usize,
    issue_ids: &[String],
    blocking_deps: &dyn Fn(&str) -> Vec<String>,
) -> Vec<Violation> {
    let wave_set: std::collections::HashSet<&str> = issue_ids.iter().map(String::as_str).collect();

    let mut violations = Vec::new();

    for id in issue_ids {
        for dep in blocking_deps(id) {
            if wave_set.contains(dep.as_str()) {
                violations.push(Violation {
                    axiom: Axiom::Locality,
                    severity: Severity::Error,
                    context_id: Some(id.clone()),
                    wave: Some(wave),
                    description: format!(
                        "{id} has blocking dependency on {dep} within the same cover index"
                    ),
                });
            }
        }
    }

    violations
}

/// Check refinement invariance between two descent data over the same context.
///
/// Premath requires refinement closure: semantics must be stable under passing
/// to a finer cover. Operationally, this means:
/// - both covers must produce effective descent, and
/// - the induced global glue fingerprint must agree.
pub fn check_refinement_invariance(
    coarse: &DescentDatum,
    refined: &DescentDatum,
) -> Result<(), PremathError> {
    if coarse.context_id != refined.context_id {
        return Err(PremathError::Refinement {
            description: format!(
                "context mismatch: coarse={}, refined={}",
                coarse.context_id, refined.context_id
            ),
        });
    }

    if !coarse.is_effective() || !refined.is_effective() {
        return Err(PremathError::Refinement {
            description:
                "cannot establish refinement invariance: at least one cover is non-effective"
                    .to_string(),
        });
    }

    let coarse_glue = coarse.glue_hash();
    let refined_glue = refined.glue_hash();
    if coarse_glue != refined_glue {
        return Err(PremathError::Refinement {
            description: format!(
                "glue mismatch under refinement: coarse={:?}, refined={:?}",
                coarse_glue, refined_glue
            ),
        });
    }

    Ok(())
}

/// Canonical key for an overlap pair (sorted).
fn overlap_key(a: &str, b: &str) -> String {
    if a < b {
        format!("{a}:{b}")
    } else {
        format!("{b}:{a}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definable::{Edge, EdgeKind, Phase};

    fn test_phase() -> Phase {
        Phase {
            kind: "task".into(),
            ephemeral: false,
            status: "open".into(),
            mol_type: None,
        }
    }

    fn test_fiber(id: &str, edges: Vec<Edge>) -> FiberSignature {
        FiberSignature {
            id: id.into(),
            context: ContextId::new("ctx"),
            content_hash: ContentHash(format!("h-{id}")),
            structure_hash: ContentHash(format!("s-{id}")),
            edges,
            phase: test_phase(),
            agent_id: None,
        }
    }

    #[test]
    fn effective_descent() {
        let fibers = vec![
            test_fiber(
                "a",
                vec![Edge {
                    target: "shared".into(),
                    kind: EdgeKind::Blocks,
                }],
            ),
            test_fiber(
                "b",
                vec![Edge {
                    target: "shared".into(),
                    kind: EdgeKind::Blocks,
                }],
            ),
        ];

        let datum = DescentDatum::assemble(
            "wave-0",
            ContextId::new("epic"),
            fibers,
            CoherenceLevel::Set,
        );

        assert!(datum.is_effective());
        assert!(datum.glue_hash().is_some());
    }

    #[test]
    fn non_effective_descent() {
        let fibers = vec![
            test_fiber(
                "a",
                vec![Edge {
                    target: "shared".into(),
                    kind: EdgeKind::Blocks,
                }],
            ),
            test_fiber(
                "b",
                vec![Edge {
                    target: "shared".into(),
                    kind: EdgeKind::RelatesTo,
                }],
            ),
        ];

        let datum = DescentDatum::assemble(
            "wave-0",
            ContextId::new("epic"),
            fibers,
            CoherenceLevel::Set,
        );

        assert!(!datum.is_effective());
        assert!(datum.glue_hash().is_none());
        assert!(!datum.conflicts().is_empty());
    }

    #[test]
    fn glue_hash_determinism() {
        let fibers = vec![test_fiber("a", vec![]), test_fiber("b", vec![])];

        let d1 = DescentDatum::assemble(
            "w",
            ContextId::new("e"),
            fibers.clone(),
            CoherenceLevel::Set,
        );
        let d2 = DescentDatum::assemble("w", ContextId::new("e"), fibers, CoherenceLevel::Set);

        assert_eq!(d1.glue_hash(), d2.glue_hash());
    }

    #[test]
    fn glue_hash_is_cover_invariant() {
        let fibers = vec![test_fiber("a", vec![]), test_fiber("b", vec![])];
        let d1 = DescentDatum::assemble(
            "coarse",
            ContextId::new("e"),
            fibers.clone(),
            CoherenceLevel::Set,
        );
        let d2 =
            DescentDatum::assemble("refined", ContextId::new("e"), fibers, CoherenceLevel::Set);
        assert_eq!(d1.glue_hash(), d2.glue_hash());
    }

    #[test]
    fn overlap_key_canonical() {
        assert_eq!(overlap_key("a", "b"), overlap_key("b", "a"));
        assert_eq!(overlap_key("a", "b"), "a:b");
    }

    #[test]
    fn locality_violation_detection() {
        let issues = vec!["a".into(), "b".into(), "c".into()];

        let deps = |id: &str| -> Vec<String> {
            match id {
                "a" => vec!["b".into()], // a blocks on b — violation!
                _ => vec![],
            }
        };

        let violations = detect_locality_violations(0, &issues, &deps);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].axiom, Axiom::Locality);
    }

    #[test]
    fn contractibility_result() {
        let fibers = vec![test_fiber("a", vec![]), test_fiber("b", vec![])];

        let datum = DescentDatum::assemble(
            "wave-0",
            ContextId::new("epic"),
            fibers,
            CoherenceLevel::Set,
        );

        let wave = WaveResult {
            wave: 0,
            fiber_count: 2,
            overlap_count: datum.overlaps.len(),
            conflict_count: 0,
            contractible: datum.is_effective(),
            datum,
        };

        let result = ContractibilityResult::from_waves(
            ContextId::new("epic"),
            CoherenceLevel::Set,
            vec![wave],
        );

        assert!(result.contractible);
        assert!(result.glue_hash.is_some());
        assert!(result.violations.is_empty());
    }

    #[test]
    fn refinement_invariance_holds_for_equivalent_data() {
        let fibers = vec![
            test_fiber(
                "a",
                vec![Edge {
                    target: "shared".into(),
                    kind: EdgeKind::Blocks,
                }],
            ),
            test_fiber(
                "b",
                vec![Edge {
                    target: "shared".into(),
                    kind: EdgeKind::Blocks,
                }],
            ),
        ];
        let coarse = DescentDatum::assemble(
            "coarse-cover",
            ContextId::new("epic"),
            fibers.clone(),
            CoherenceLevel::Set,
        );
        let refined = DescentDatum::assemble(
            "refined-cover",
            ContextId::new("epic"),
            fibers,
            CoherenceLevel::Set,
        );

        assert!(check_refinement_invariance(&coarse, &refined).is_ok());
    }

    #[test]
    fn refinement_invariance_detects_non_effective_refinement() {
        let effective = DescentDatum::assemble(
            "coarse",
            ContextId::new("epic"),
            vec![
                test_fiber(
                    "a",
                    vec![Edge {
                        target: "shared".into(),
                        kind: EdgeKind::Blocks,
                    }],
                ),
                test_fiber(
                    "b",
                    vec![Edge {
                        target: "shared".into(),
                        kind: EdgeKind::Blocks,
                    }],
                ),
            ],
            CoherenceLevel::Set,
        );

        let non_effective = DescentDatum::assemble(
            "refined",
            ContextId::new("epic"),
            vec![
                test_fiber(
                    "a",
                    vec![Edge {
                        target: "shared".into(),
                        kind: EdgeKind::Blocks,
                    }],
                ),
                test_fiber(
                    "b",
                    vec![Edge {
                        target: "shared".into(),
                        kind: EdgeKind::RelatesTo,
                    }],
                ),
            ],
            CoherenceLevel::Set,
        );

        let err = check_refinement_invariance(&effective, &non_effective).unwrap_err();
        match err {
            PremathError::Refinement { .. } => {}
            other => panic!("expected refinement error, got {other:?}"),
        }
    }
}
