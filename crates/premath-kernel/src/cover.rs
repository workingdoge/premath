//! Covers and the Grothendieck pretopology.
//!
//! A cover of Γ is a family U = {u_i: Γ_i → Γ} ▷ Γ. Covers represent
//! decompositions of a context into pieces that can be worked on in
//! parallel.
//!
//! A backend may realize covers in many ways (dependency fronts, branch
//! families, semantic partitions, etc.). The kernel does not prescribe a
//! specific decomposition algorithm.

use crate::context::{ContextId, Morphism};
use serde::{Deserialize, Serialize};

/// A cover of a context: a family of morphisms into Γ.
///
/// U = {u_i: Γ_i → Γ} ▷ Γ
///
/// The cover decomposes Γ into a collection of "patches" Γ_i.
/// Each patch can be worked on independently, provided the
/// overlap compatibilities (descent data) are satisfied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cover {
    /// The context being covered.
    pub base: ContextId,

    /// Identifier for this cover (e.g., "wave-0", "wave-1").
    pub id: String,

    /// The covering morphisms: u_i: Γ_i → Γ.
    pub patches: Vec<Patch>,
}

/// A single patch in a cover: one morphism u_i: Γ_i → Γ.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    /// The local context Γ_i.
    pub context: ContextId,

    /// The morphism u_i: Γ_i → Γ.
    pub morphism: Morphism,
}

/// An overlap between two patches: Γ_i ×_Γ Γ_j.
///
/// The overlap is where two patches share structure. Descent data
/// must be compatible on overlaps for gluing to succeed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overlap {
    /// First patch index.
    pub patch_i: usize,

    /// Second patch index.
    pub patch_j: usize,

    /// The shared dependencies (issues both patches depend on).
    pub shared: Vec<ContextId>,

    /// Projection morphisms.
    ///
    /// p1: Γ_i ×_Γ Γ_j → Γ_i
    /// p2: Γ_i ×_Γ Γ_j → Γ_j
    pub proj_i: Morphism,
    pub proj_j: Morphism,
}

impl Cover {
    /// Create a new cover.
    pub fn new(base: ContextId, id: impl Into<String>, patches: Vec<Patch>) -> Self {
        Self {
            base,
            id: id.into(),
            patches,
        }
    }

    /// Number of patches in this cover.
    pub fn len(&self) -> usize {
        self.patches.len()
    }

    /// Whether the cover is empty (no patches).
    pub fn is_empty(&self) -> bool {
        self.patches.is_empty()
    }

    /// Compute all pairwise overlaps between patches.
    ///
    /// An overlap exists between patches i and j when they share
    /// at least one dependency (their contexts have a non-trivial
    /// fiber product over the base).
    pub fn overlaps(
        &self,
        shared_deps: &dyn Fn(&ContextId, &ContextId) -> Vec<ContextId>,
    ) -> Vec<Overlap> {
        let mut overlaps = Vec::new();

        for i in 0..self.patches.len() {
            for j in (i + 1)..self.patches.len() {
                let pi = &self.patches[i];
                let pj = &self.patches[j];

                let shared = shared_deps(&pi.context, &pj.context);
                if !shared.is_empty() {
                    overlaps.push(Overlap {
                        patch_i: i,
                        patch_j: j,
                        shared,
                        // Projections: the "restriction" morphisms from the
                        // overlap back to each patch context.
                        proj_i: Morphism::new(
                            pi.context.clone(),
                            pi.context.clone(),
                            crate::context::MorphismKind::Identity,
                        ),
                        proj_j: Morphism::new(
                            pj.context.clone(),
                            pj.context.clone(),
                            crate::context::MorphismKind::Identity,
                        ),
                    });
                }
            }
        }

        overlaps
    }
}

/// A refinement V ▷ Γ of a cover U ▷ Γ.
///
/// Premath requires that descent is invariant under refinement:
/// if V refines U, then Desc_V(Γ) ≈ Desc_U(Γ).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refinement {
    /// The original cover.
    pub original: String,

    /// The refined cover.
    pub refined: String,

    /// How each refined patch maps to original patches.
    pub mapping: Vec<(usize, usize)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_cover() {
        let cover = Cover::new(ContextId::new("base"), "empty", vec![]);
        assert!(cover.is_empty());
        assert_eq!(cover.len(), 0);
    }
}
