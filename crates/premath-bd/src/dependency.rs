//! Dependency types: typed edges in the issue graph.

use premath_kernel::definable::EdgeKind;
use serde::{Deserialize, Serialize};

/// A dependency between two issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub issue_id: String,
    pub depends_on_id: String,
    /// JSONL compatibility:
    /// - Beads uses `type`
    /// - Legacy/internal tooling may use `dep_type`
    #[serde(rename = "type", alias = "dep_type")]
    pub dep_type: DepType,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub created_by: String,
}

/// Dependency type classification.
///
/// Maps to beads dependency types but expressed with Premath semantics.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DepType {
    Blocks,
    ParentChild,
    ConditionalBlocks,
    Related,
    DiscoveredFrom,
    RelatesTo,
    Duplicates,
    Supersedes,
    WaitsFor,
    RepliesTo,
}

impl DepType {
    /// Whether this dependency type affects ready-work computation.
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            DepType::Blocks | DepType::ParentChild | DepType::ConditionalBlocks | DepType::WaitsFor
        )
    }

    /// Convert to the kernel's EdgeKind.
    pub fn to_edge_kind(&self) -> EdgeKind {
        match self {
            DepType::Blocks | DepType::WaitsFor => EdgeKind::Blocks,
            DepType::ParentChild => EdgeKind::ParentChild,
            DepType::ConditionalBlocks => EdgeKind::ConditionalBlocks,
            DepType::Related
            | DepType::DiscoveredFrom
            | DepType::RelatesTo
            | DepType::RepliesTo => EdgeKind::RelatesTo,
            DepType::Duplicates => EdgeKind::Duplicates,
            DepType::Supersedes => EdgeKind::Supersedes,
        }
    }

    /// String representation for hashing.
    pub fn as_str(&self) -> &str {
        match self {
            DepType::Blocks => "blocks",
            DepType::ParentChild => "parent-child",
            DepType::ConditionalBlocks => "conditional-blocks",
            DepType::Related => "related",
            DepType::DiscoveredFrom => "discovered-from",
            DepType::RelatesTo => "relates-to",
            DepType::Duplicates => "duplicates",
            DepType::Supersedes => "supersedes",
            DepType::WaitsFor => "waits-for",
            DepType::RepliesTo => "replies-to",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_accepts_type_field() {
        let raw = r#"{
            "issue_id":"bd-a",
            "depends_on_id":"bd-b",
            "type":"discovered-from"
        }"#;

        let dep: Dependency = serde_json::from_str(raw).expect("must parse beads dependency");
        assert!(matches!(dep.dep_type, DepType::DiscoveredFrom));
    }

    #[test]
    fn dependency_accepts_dep_type_alias() {
        let raw = r#"{
            "issue_id":"bd-a",
            "depends_on_id":"bd-b",
            "dep_type":"blocks"
        }"#;

        let dep: Dependency = serde_json::from_str(raw).expect("must parse alias");
        assert!(matches!(dep.dep_type, DepType::Blocks));
    }
}
