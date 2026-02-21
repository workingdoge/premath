//! Issue type: the primary definable in premath-bd.

use chrono::{DateTime, Utc};
use premath_kernel::context::ContextId;
use premath_kernel::definable::{ContentHash, Edge, FiberSignature, Phase};
use serde::{Deserialize, Serialize};

use crate::dependency::Dependency;

/// An issue: a trackable work item and the primary definable.
///
/// Implements `Definable` from the Premath kernel, meaning it carries
/// content-addressed identity, structural hashing, and fiber signatures
/// for descent checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    // ── Core identification ──
    pub id: String,

    // ── Content ──
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub design: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub acceptance_criteria: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,

    // ── Status & workflow ──
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(
        default = "default_issue_type",
        skip_serializing_if = "String::is_empty"
    )]
    pub issue_type: String,

    // ── Assignment ──
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub assignee: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub owner: String,

    // ── Timestamps ──
    #[serde(default = "default_timestamp")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "default_timestamp")]
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,

    // ── Molecule fields ──
    #[serde(default)]
    pub ephemeral: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mol_type: String,

    // ── Labels ──
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,

    // ── Dependencies (populated from JSONL/DB) ──
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,

    // ── Custom metadata ──
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

fn default_status() -> String {
    "open".to_string()
}

fn default_priority() -> i32 {
    2
}

fn default_issue_type() -> String {
    "task".to_string()
}

fn default_timestamp() -> DateTime<Utc> {
    Utc::now()
}

impl Issue {
    /// Construct a minimal issue with deterministic defaults.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            design: String::new(),
            acceptance_criteria: String::new(),
            notes: String::new(),
            status: default_status(),
            priority: default_priority(),
            issue_type: default_issue_type(),
            assignee: String::new(),
            owner: String::new(),
            created_at: now,
            updated_at: now,
            closed_at: None,
            ephemeral: false,
            mol_type: String::new(),
            labels: Vec::new(),
            dependencies: Vec::new(),
            metadata: None,
        }
    }

    /// Apply status transition side effects and update timestamps.
    ///
    /// Closed status sets `closed_at` if missing. Non-closed statuses clear it.
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
        self.updated_at = Utc::now();
        if self.status == "closed" {
            if self.closed_at.is_none() {
                self.closed_at = Some(Utc::now());
            }
        } else {
            self.closed_at = None;
        }
    }

    /// Bump updated timestamp without changing semantic content fields.
    pub fn touch_updated_at(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Compute the content hash of substantive fields.
    ///
    /// Excludes: id, timestamps, assignee, owner, metadata.
    /// These are volatile fields that can change without changing
    /// the "meaning" of the issue.
    pub fn content_hash(&self) -> ContentHash {
        ContentHash::builder()
            .field("title", &self.title)
            .field("description", &self.description)
            .field("design", &self.design)
            .field("acceptance_criteria", &self.acceptance_criteria)
            .field("notes", &self.notes)
            .field("status", &self.status)
            .field_int("priority", self.priority as i64)
            .field("issue_type", &self.issue_type)
            .field_bool("ephemeral", self.ephemeral)
            .field("mol_type", &self.mol_type)
            .finish()
    }

    /// Compute the structure hash: content + dependency shape.
    pub fn structure_hash(&self) -> ContentHash {
        let content = self.content_hash();
        let mut builder = ContentHash::builder().field("content", &content.0);

        // Add sorted dependency edges
        let mut edges: Vec<(&str, &str)> = self
            .dependencies
            .iter()
            .map(|d| (d.depends_on_id.as_str(), d.dep_type.as_str()))
            .collect();
        edges.sort();

        for (target, kind) in edges {
            builder = builder
                .field("edge_target", target)
                .field("edge_kind", kind);
        }

        builder.finish()
    }

    /// Build the fiber signature for descent checking.
    pub fn fiber_signature(&self, context: &ContextId) -> FiberSignature {
        let mut edges: Vec<Edge> = self
            .dependencies
            .iter()
            .map(|d| Edge {
                target: d.depends_on_id.clone(),
                kind: d.dep_type.to_edge_kind(),
            })
            .collect();
        edges.sort();

        FiberSignature {
            id: self.id.clone(),
            context: context.clone(),
            content_hash: self.content_hash(),
            structure_hash: self.structure_hash(),
            edges,
            phase: Phase {
                kind: self.issue_type.clone(),
                ephemeral: self.ephemeral,
                status: self.status.clone(),
                mol_type: if self.mol_type.is_empty() {
                    None
                } else {
                    Some(self.mol_type.clone())
                },
            },
            agent_id: if self.assignee.is_empty() {
                None
            } else {
                Some(self.assignee.clone())
            },
        }
    }
}
