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

/// View projection for dependency semantics.
///
/// Keep one canonical edge encoding (`DepType`) and project it into
/// task-specific interpretations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyView {
    Execution,
    Gtd,
    Groupoid,
}

impl DependencyView {
    pub fn as_str(&self) -> &'static str {
        match self {
            DependencyView::Execution => "execution",
            DependencyView::Gtd => "gtd",
            DependencyView::Groupoid => "groupoid",
        }
    }
}

/// Projected dependency row used for deterministic CLI/API views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyProjection {
    pub issue_id: String,
    pub depends_on_id: String,
    #[serde(rename = "type")]
    pub dep_type: DepType,
    pub view: DependencyView,
    pub role: String,
    pub blocking: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub created_by: String,
}

impl Dependency {
    pub fn project(&self, view: DependencyView) -> DependencyProjection {
        DependencyProjection {
            issue_id: self.issue_id.clone(),
            depends_on_id: self.depends_on_id.clone(),
            dep_type: self.dep_type.clone(),
            view,
            role: self.dep_type.role_for_view(view).to_string(),
            blocking: self.dep_type.is_blocking(),
            created_by: self.created_by.clone(),
        }
    }
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

    /// Role label in a selected projection view.
    pub fn role_for_view(&self, view: DependencyView) -> &'static str {
        match view {
            DependencyView::Execution => {
                if self.is_blocking() {
                    "blocking"
                } else {
                    "informational"
                }
            }

            DependencyView::Gtd => match self {
                DepType::Blocks | DepType::ConditionalBlocks | DepType::WaitsFor => "next-action",
                DepType::ParentChild => "project-structure",
                DepType::DiscoveredFrom | DepType::RepliesTo => "captured-work",
                DepType::Related | DepType::RelatesTo => "reference",
                DepType::Duplicates | DepType::Supersedes => "dedupe",
            },

            DependencyView::Groupoid => match self {
                DepType::Blocks | DepType::ConditionalBlocks | DepType::WaitsFor => "constraint",
                DepType::ParentChild => "generator",
                DepType::DiscoveredFrom
                | DepType::RepliesTo
                | DepType::Related
                | DepType::RelatesTo => "provenance",
                DepType::Duplicates | DepType::Supersedes => "equivalence",
            },
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

    #[test]
    fn projection_views_map_to_expected_roles() {
        assert_eq!(
            DepType::Blocks.role_for_view(DependencyView::Execution),
            "blocking"
        );
        assert_eq!(
            DepType::Related.role_for_view(DependencyView::Execution),
            "informational"
        );
        assert_eq!(
            DepType::ParentChild.role_for_view(DependencyView::Gtd),
            "project-structure"
        );
        assert_eq!(
            DepType::DiscoveredFrom.role_for_view(DependencyView::Gtd),
            "captured-work"
        );
        assert_eq!(
            DepType::Supersedes.role_for_view(DependencyView::Groupoid),
            "equivalence"
        );
        assert_eq!(
            DepType::WaitsFor.role_for_view(DependencyView::Groupoid),
            "constraint"
        );
    }
}
