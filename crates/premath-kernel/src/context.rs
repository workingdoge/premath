//! World of contexts and morphisms.
//!
//! A Premath world consists of:
//! 1. **Contexts**: a category C with objects Γ and morphisms f: Γ' → Γ
//! 2. **Covers**: a coverage / Grothendieck pretopology
//!
//! Contexts represent the states in which definables live. A morphism
//! f: Γ' → Γ is a context change — resolving a dependency, claiming a
//! task, entering a subtask.
//!
//! Backend examples may instantiate:
//! - contexts as issue/branch snapshots,
//! - morphisms as state transitions,
//! - history from a VCS operation log.

use serde::{Deserialize, Serialize};

/// Opaque identifier for a context.
///
/// In practice this is a content-addressed hash (JJ change ID,
/// SurrealDB record ID, or a composite of both).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextId(pub String);

impl ContextId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for ContextId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A context in the Premath world.
///
/// Contexts are the objects of the category C. Each context represents
/// a state in which definables can be evaluated — a set of resolved
/// dependencies, a branch state, an agent's environment.
///
/// This trait is deliberately minimal: a context only needs to be
/// identifiable and comparable. The richness comes from the morphisms
/// between contexts and the definables that live over them.
pub trait Context: Send + Sync {
    /// The unique, content-addressed identifier for this context.
    fn id(&self) -> &ContextId;

    /// Parent contexts (contexts this one was derived from).
    ///
    /// In JJ terms: the predecessor changes.
    /// In bd terms: the dependency sources.
    fn parents(&self) -> Vec<ContextId>;

    /// Whether this context is a root (no parents).
    fn is_root(&self) -> bool {
        self.parents().is_empty()
    }
}

/// A morphism f: Γ' → Γ in the context category.
///
/// Morphisms represent context changes: resolving a dependency,
/// merging a branch, rebasing work onto a new base.
///
/// The key property: every morphism induces a reindexing map
/// f*: Def(Γ) → Def(Γ') that pulls definables back along the
/// context change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Morphism {
    /// Source context (Γ').
    pub source: ContextId,

    /// Target context (Γ).
    pub target: ContextId,

    /// What kind of context change this represents.
    pub kind: MorphismKind,
}

/// Classification of context changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MorphismKind {
    /// A dependency was resolved (issue closed).
    DependencyResolved,

    /// A branch was merged.
    BranchMerge,

    /// Work was rebased onto a new base.
    Rebase,

    /// An agent claimed a task (context narrowed).
    Claim,

    /// A cover was refined (finer decomposition).
    Refinement,

    /// Identity morphism (context unchanged).
    Identity,
}

impl Morphism {
    /// Create a new morphism from source to target.
    pub fn new(source: ContextId, target: ContextId, kind: MorphismKind) -> Self {
        Self {
            source,
            target,
            kind,
        }
    }

    /// The identity morphism on a context.
    pub fn identity(ctx: ContextId) -> Self {
        Self {
            source: ctx.clone(),
            target: ctx,
            kind: MorphismKind::Identity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_morphism() {
        let ctx = ContextId::new("ctx-1");
        let id = Morphism::identity(ctx.clone());
        assert_eq!(id.source, id.target);
        assert_eq!(id.kind, MorphismKind::Identity);
    }
}
