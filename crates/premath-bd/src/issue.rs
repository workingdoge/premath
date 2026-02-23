//! Issue type: the primary definable in premath-bd.

use chrono::{DateTime, Utc};
use premath_kernel::context::ContextId;
use premath_kernel::definable::{ContentHash, Edge, FiberSignature, Phase};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser};

use crate::dependency::Dependency;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssueLease {
    pub lease_id: String,
    pub owner: String,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renewed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueLeaseState {
    Unleased,
    Active,
    Stale,
}

pub const ISSUE_TYPE_EPIC: &str = "epic";
pub const ISSUE_TYPE_TASK: &str = "task";

pub fn issue_type_variants() -> &'static [&'static str] {
    &[ISSUE_TYPE_EPIC, ISSUE_TYPE_TASK]
}

pub fn normalize_issue_type(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        ISSUE_TYPE_EPIC => Some(ISSUE_TYPE_EPIC),
        ISSUE_TYPE_TASK => Some(ISSUE_TYPE_TASK),
        _ => None,
    }
}

pub fn parse_issue_type(value: &str) -> Option<String> {
    normalize_issue_type(value).map(ToOwned::to_owned)
}

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
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_issue_type",
        serialize_with = "serialize_issue_type"
    )]
    pub issue_type: String,

    // ── Assignment ──
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub assignee: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub owner: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease: Option<IssueLease>,

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
    ISSUE_TYPE_TASK.to_string()
}

fn deserialize_issue_type<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    parse_issue_type(&raw).ok_or_else(|| {
        de::Error::custom(format!(
            "invalid issue_type `{}` (expected one of: {})",
            raw,
            issue_type_variants().join(", ")
        ))
    })
}

fn serialize_issue_type<S>(issue_type: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let normalized = parse_issue_type(issue_type).ok_or_else(|| {
        ser::Error::custom(format!(
            "invalid issue_type `{}` (expected one of: {})",
            issue_type,
            issue_type_variants().join(", ")
        ))
    })?;
    serializer.serialize_str(&normalized)
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
            lease: None,
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
    /// Any non-`in_progress` status clears active lease metadata.
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
        self.updated_at = Utc::now();
        if !self.status.trim().eq_ignore_ascii_case("in_progress") {
            self.lease = None;
        }
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

    pub fn set_issue_type(&mut self, issue_type: impl AsRef<str>) -> Result<(), String> {
        let raw = issue_type.as_ref();
        let normalized = parse_issue_type(raw).ok_or_else(|| {
            format!(
                "invalid issue_type `{}` (expected one of: {})",
                raw,
                issue_type_variants().join(", ")
            )
        })?;
        self.issue_type = normalized;
        Ok(())
    }

    pub fn issue_type(&self) -> &str {
        normalize_issue_type(&self.issue_type).unwrap_or(ISSUE_TYPE_TASK)
    }

    pub fn lease_state_at(&self, now: DateTime<Utc>) -> IssueLeaseState {
        match self.lease.as_ref() {
            None => IssueLeaseState::Unleased,
            Some(lease) if lease.expires_at > now => IssueLeaseState::Active,
            Some(_) => IssueLeaseState::Stale,
        }
    }

    pub fn lease_owner(&self) -> Option<&str> {
        self.lease.as_ref().map(|lease| lease.owner.as_str())
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
            .field("issue_type", self.issue_type())
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
                kind: self.issue_type().to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn fixture_with_active_lease() -> Issue {
        let now = Utc::now();
        let mut issue = Issue::new("bd-lease", "Lease fixture");
        issue.status = "in_progress".to_string();
        issue.assignee = "worker-a".to_string();
        issue.lease = Some(IssueLease {
            lease_id: "lease1_bd-lease_worker-a".to_string(),
            owner: "worker-a".to_string(),
            acquired_at: now,
            expires_at: now + Duration::minutes(30),
            renewed_at: None,
        });
        issue
    }

    #[test]
    fn set_status_clears_lease_for_open() {
        let mut issue = fixture_with_active_lease();
        issue.set_status("open");
        assert!(issue.lease.is_none());
        assert_eq!(issue.status, "open");
        assert!(issue.closed_at.is_none());
    }

    #[test]
    fn set_status_clears_lease_for_blocked() {
        let mut issue = fixture_with_active_lease();
        issue.set_status("blocked");
        assert!(issue.lease.is_none());
        assert_eq!(issue.status, "blocked");
    }

    #[test]
    fn set_status_preserves_lease_for_in_progress() {
        let mut issue = fixture_with_active_lease();
        let expected = issue.lease.clone();
        issue.set_status("in_progress");
        assert_eq!(issue.lease, expected);
    }
}
