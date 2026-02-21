//! Error types for Premath kernel operations.

use crate::coherence::CoherenceLevel;

/// Errors arising from Premath axiom violations or invalid operations.
#[derive(Debug, thiserror::Error)]
pub enum PremathError {
    /// A definable is not stable under reindexing.
    #[error("stability violation: {description}")]
    Stability { description: String },

    /// A definable cannot be restricted along a cover.
    #[error("locality violation: {description}")]
    Locality { description: String },

    /// Compatible local definables do not produce a global.
    #[error("gluing violation: {description}")]
    Gluing { description: String },

    /// Gluing is not contractible — multiple globals fit the same locals.
    #[error("uniqueness violation: {description}")]
    Uniqueness { description: String },

    /// Meaning changes under refinement of covers.
    #[error("refinement violation: {description}")]
    Refinement { description: String },

    /// The requested coherence level cannot satisfy the operation.
    #[error("coherence mismatch: expected {expected:?}, got {actual:?}")]
    CoherenceMismatch {
        expected: CoherenceLevel,
        actual: CoherenceLevel,
    },

    /// A context or morphism is malformed.
    #[error("invalid context: {0}")]
    InvalidContext(String),

    /// Storage or I/O failure.
    #[error("storage error: {0}")]
    Storage(String),
}

/// Which Premath axiom was violated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axiom {
    Stability,
    Locality,
    Gluing,
    Uniqueness,
    Refinement,
}

/// Severity of a violation.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// A concrete violation of a Premath axiom.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub axiom: Axiom,
    pub severity: Severity,
    pub context_id: Option<String>,
    pub wave: Option<usize>,
    pub description: String,
}
