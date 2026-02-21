//! Fibre space of definables.
//!
//! Premath assigns to each context Γ an object of definables Def(Γ) ∈ V,
//! and to each morphism f: Γ' → Γ a reindexing map f*: Def(Γ) → Def(Γ').
//!
//! Def: C^op → V is a pseudo/∞-functor appropriate to V.
//!
//! In the bd realization: definables are content-hashed issue states.
//! Reindexing is pulling an issue into a different branch/context.
//! The content hash is the identity criterion.

use crate::coherence::CoherenceLevel;
use crate::context::ContextId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// A content-addressed hash identifying a definable.
///
/// Two definables with the same ContentHash are "the same" at V = Set.
/// At higher coherence levels, different hashes may still be equivalent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(pub String);

impl ContentHash {
    /// Compute a content hash from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Self {
        let hash = Sha256::digest(data);
        Self(format!("{hash:x}"))
    }

    /// Compute a content hash from a string.
    pub fn from_str_content(s: &str) -> Self {
        Self::from_bytes(s.as_bytes())
    }

    /// A builder for incrementally computing content hashes.
    pub fn builder() -> ContentHashBuilder {
        ContentHashBuilder {
            hasher: Sha256::new(),
        }
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Incremental content hash builder.
///
/// Feeds fields in a stable order to produce a deterministic hash.
/// Excludes volatile fields (timestamps, IDs) — only substantive content.
pub struct ContentHashBuilder {
    hasher: Sha256,
}

impl ContentHashBuilder {
    /// Feed a string field into the hash.
    pub fn field(mut self, name: &str, value: &str) -> Self {
        self.hasher.update(name.as_bytes());
        self.hasher.update(b":");
        self.hasher.update(value.as_bytes());
        self.hasher.update(b"\n");
        self
    }

    /// Feed an integer field into the hash.
    pub fn field_int(self, name: &str, value: i64) -> Self {
        self.field(name, &value.to_string())
    }

    /// Feed a boolean field into the hash.
    pub fn field_bool(self, name: &str, value: bool) -> Self {
        self.field(name, if value { "true" } else { "false" })
    }

    /// Feed an optional field (skipped if None).
    pub fn field_opt(self, name: &str, value: Option<&str>) -> Self {
        match value {
            Some(v) => self.field(name, v),
            None => self,
        }
    }

    /// Finalize and produce the content hash.
    pub fn finish(self) -> ContentHash {
        let hash = self.hasher.finalize();
        ContentHash(format!("{hash:x}"))
    }
}

/// The core trait: a definable in a Premath world.
///
/// A definable is an element of Def(Γ) — something that is well-defined
/// in context Γ. It must be content-addressable and carry enough structure
/// for reindexing and descent checking.
///
/// In the bd realization: an Issue with its dependencies is a definable.
pub trait Definable: Send + Sync {
    /// The context in which this definable lives.
    fn context(&self) -> &ContextId;

    /// The content hash of this definable's substantive content.
    ///
    /// Excludes volatile fields (timestamps, assignee, agent ID).
    /// Two definables with the same content hash are "the same" at V = Set.
    fn content_hash(&self) -> ContentHash;

    /// The structure hash: content hash + dependency graph shape.
    ///
    /// Captures not just what this definable says, but how it relates
    /// to other definables in the same context.
    fn structure_hash(&self) -> ContentHash;

    /// The fiber signature: full identity for descent checking.
    fn fiber_signature(&self) -> FiberSignature;
}

/// Complete identity of a definable in a context, for descent checking.
///
/// This is the data that gets compared on overlaps to determine
/// whether descent is effective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiberSignature {
    /// Which definable this is.
    pub id: String,

    /// In which context.
    pub context: ContextId,

    /// Content hash (substantive content only).
    pub content_hash: ContentHash,

    /// Structure hash (content + dependency shape).
    pub structure_hash: ContentHash,

    /// Typed dependency edges, sorted canonically.
    pub edges: Vec<Edge>,

    /// Phase/lifecycle state.
    pub phase: Phase,

    /// Which agent produced this (informational, not part of identity).
    pub agent_id: Option<String>,
}

/// A typed edge from a definable to another.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Edge {
    /// Target definable ID.
    pub target: String,

    /// Edge type (blocks, relates_to, parent_child, etc.).
    pub kind: EdgeKind,
}

/// Classification of dependency edges.
///
/// These correspond to beads dependency types but are expressed
/// in terms of their Premath semantics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// Blocking: target must complete before source can start.
    /// Affects ready-work computation. Part of the cover structure.
    Blocks,

    /// Hierarchical containment: source is a child of target.
    /// Blocking (parent must be open for child to exist).
    ParentChild,

    /// Conditional blocking: source runs only if target fails.
    ConditionalBlocks,

    /// Association: informational link, does not affect scheduling.
    RelatesTo,

    /// Duplication: source is a duplicate of target.
    Duplicates,

    /// Supersession: source replaces target.
    Supersedes,
}

impl EdgeKind {
    /// Whether this edge type affects ready-work computation.
    ///
    /// Blocking edges form the cover structure. Non-blocking edges
    /// are informational and don't participate in descent checking
    /// at V = Set (but may at higher coherence levels).
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            EdgeKind::Blocks | EdgeKind::ParentChild | EdgeKind::ConditionalBlocks
        )
    }
}

/// Phase/lifecycle state of a definable.
///
/// Phase must be consistent across parallel fibers — you cannot merge
/// an ephemeral wisp with a persistent molecule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Phase {
    /// The kind of definable (task, epic, molecule, message, etc.).
    pub kind: String,

    /// Whether this is ephemeral (wisp) or persistent.
    pub ephemeral: bool,

    /// Current status (open, in_progress, closed, etc.).
    pub status: String,

    /// Molecule type, if applicable (swarm, patrol, work).
    pub mol_type: Option<String>,
}

/// Check whether two fiber signatures are compatible at a given
/// coherence level.
///
/// This is the core comparison used in overlap checking:
/// φ_ij: p1*A_i ≈ p2*A_j
pub fn signatures_compatible(
    a: &FiberSignature,
    b: &FiberSignature,
    level: CoherenceLevel,
) -> Result<(), Vec<String>> {
    let mut conflicts = Vec::new();

    // Phase must always match (regardless of coherence level)
    if a.phase != b.phase {
        conflicts.push(format!("phase mismatch: {:?} vs {:?}", a.phase, b.phase));
    }

    // Find shared edges (the overlap)
    let a_targets: std::collections::HashMap<&str, &EdgeKind> = a
        .edges
        .iter()
        .map(|e| (e.target.as_str(), &e.kind))
        .collect();

    for edge_b in &b.edges {
        if let Some(kind_a) = a_targets.get(edge_b.target.as_str()) {
            match level {
                CoherenceLevel::Set => {
                    // Strict: edge kinds must match exactly
                    if **kind_a != edge_b.kind {
                        conflicts.push(format!(
                            "edge type mismatch on {}: {:?} vs {:?}",
                            edge_b.target, kind_a, edge_b.kind
                        ));
                    }
                }
                CoherenceLevel::Gpd => {
                    // Isomorphism: blocking class must match
                    if kind_a.is_blocking() != edge_b.kind.is_blocking() {
                        conflicts.push(format!(
                            "blocking class mismatch on {}: {:?} (blocking={}) vs {:?} (blocking={})",
                            edge_b.target,
                            kind_a, kind_a.is_blocking(),
                            edge_b.kind, edge_b.kind.is_blocking()
                        ));
                    }
                }
                CoherenceLevel::SInf => {
                    // Higher equivalence: all edge types are compatible
                    // (a full S∞ implementation would require explicit
                    // equivalence witnesses)
                }
            }
        }
    }

    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(conflicts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_determinism() {
        let h1 = ContentHash::builder()
            .field("title", "hello")
            .field_int("priority", 1)
            .finish();

        let h2 = ContentHash::builder()
            .field("title", "hello")
            .field_int("priority", 1)
            .finish();

        assert_eq!(h1, h2);
    }

    #[test]
    fn content_hash_sensitivity() {
        let h1 = ContentHash::builder().field("title", "hello").finish();

        let h2 = ContentHash::builder().field("title", "world").finish();

        assert_ne!(h1, h2);
    }

    #[test]
    fn edge_kind_blocking() {
        assert!(EdgeKind::Blocks.is_blocking());
        assert!(EdgeKind::ParentChild.is_blocking());
        assert!(EdgeKind::ConditionalBlocks.is_blocking());
        assert!(!EdgeKind::RelatesTo.is_blocking());
        assert!(!EdgeKind::Duplicates.is_blocking());
    }

    #[test]
    fn signature_compatibility_set() {
        let phase = Phase {
            kind: "task".into(),
            ephemeral: false,
            status: "open".into(),
            mol_type: None,
        };

        let a = FiberSignature {
            id: "a".into(),
            context: ContextId::new("ctx"),
            content_hash: ContentHash("ha".into()),
            structure_hash: ContentHash("sa".into()),
            edges: vec![Edge {
                target: "shared".into(),
                kind: EdgeKind::Blocks,
            }],
            phase: phase.clone(),
            agent_id: None,
        };

        let b = FiberSignature {
            id: "b".into(),
            context: ContextId::new("ctx"),
            content_hash: ContentHash("hb".into()),
            structure_hash: ContentHash("sb".into()),
            edges: vec![Edge {
                target: "shared".into(),
                kind: EdgeKind::Blocks,
            }],
            phase,
            agent_id: None,
        };

        assert!(signatures_compatible(&a, &b, CoherenceLevel::Set).is_ok());
    }

    #[test]
    fn signature_incompatibility_set() {
        let phase = Phase {
            kind: "task".into(),
            ephemeral: false,
            status: "open".into(),
            mol_type: None,
        };

        let a = FiberSignature {
            id: "a".into(),
            context: ContextId::new("ctx"),
            content_hash: ContentHash("ha".into()),
            structure_hash: ContentHash("sa".into()),
            edges: vec![Edge {
                target: "shared".into(),
                kind: EdgeKind::Blocks,
            }],
            phase: phase.clone(),
            agent_id: None,
        };

        let b = FiberSignature {
            id: "b".into(),
            context: ContextId::new("ctx"),
            content_hash: ContentHash("hb".into()),
            structure_hash: ContentHash("sb".into()),
            edges: vec![Edge {
                target: "shared".into(),
                kind: EdgeKind::RelatesTo,
            }],
            phase,
            agent_id: None,
        };

        // Incompatible at Set (different edge kinds)
        assert!(signatures_compatible(&a, &b, CoherenceLevel::Set).is_err());

        // Also incompatible at Gpd (blocking vs non-blocking)
        assert!(signatures_compatible(&a, &b, CoherenceLevel::Gpd).is_err());

        // Compatible at S∞
        assert!(signatures_compatible(&a, &b, CoherenceLevel::SInf).is_ok());
    }
}
