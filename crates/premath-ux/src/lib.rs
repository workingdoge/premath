//! UX composition layer.
//!
//! This crate defines user-observation query contracts over projection
//! backends. Backends (for example `premath-surreal`) remain adapters; this
//! crate owns the interaction shape used by frontends.

pub mod http;

use premath_surreal::{
    DecisionSummary, DeltaSummary, InstructionSummary, LatestObservation, ObservationError,
    ObservationIndex, ObservationSummary, ProjectionView, RequiredSummary,
};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use thiserror::Error;

pub trait ObservationBackend {
    fn summary(&self) -> ObservationSummary;
    fn latest_delta(&self) -> Option<DeltaSummary>;
    fn latest_required(&self) -> Option<RequiredSummary>;
    fn latest_decision(&self) -> Option<DecisionSummary>;
    fn instruction(&self, instruction_id: &str) -> Option<InstructionSummary>;
    fn projection(&self, projection_digest: &str) -> Option<ProjectionView>;
}

#[derive(Debug, Clone)]
pub struct SurrealObservationBackend {
    index: ObservationIndex,
}

impl SurrealObservationBackend {
    pub fn from_index(index: ObservationIndex) -> Self {
        Self { index }
    }

    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, ObservationError> {
        ObservationIndex::load_json(path).map(Self::from_index)
    }
}

impl ObservationBackend for SurrealObservationBackend {
    fn summary(&self) -> ObservationSummary {
        self.index.summary().clone()
    }

    fn latest_delta(&self) -> Option<DeltaSummary> {
        self.index.latest().delta.clone()
    }

    fn latest_required(&self) -> Option<RequiredSummary> {
        self.index.latest().required.clone()
    }

    fn latest_decision(&self) -> Option<DecisionSummary> {
        self.index.latest().decision.clone()
    }

    fn instruction(&self, instruction_id: &str) -> Option<InstructionSummary> {
        self.index.instruction(instruction_id).cloned()
    }

    fn projection(&self, projection_digest: &str) -> Option<ProjectionView> {
        self.index.projection(projection_digest)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestView {
    pub summary: ObservationSummary,
    pub latest: LatestObservation,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeedsAttentionView {
    pub needs_attention: bool,
    pub state: String,
    pub top_failure_class: Option<String>,
    pub latest_projection_digest: Option<String>,
    pub latest_instruction_id: Option<String>,
    pub coherence: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObserveQuery {
    Latest,
    NeedsAttention,
    Instruction { instruction_id: String },
    Projection { projection_digest: String },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ObserveQueryError {
    #[error("instruction not found: {0}")]
    InstructionNotFound(String),
    #[error("projection not found: {0}")]
    ProjectionNotFound(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone)]
pub struct UxService<B: ObservationBackend> {
    backend: B,
}

impl<B: ObservationBackend> UxService<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    pub fn latest(&self) -> LatestView {
        LatestView {
            summary: self.backend.summary(),
            latest: LatestObservation {
                delta: self.backend.latest_delta(),
                required: self.backend.latest_required(),
                decision: self.backend.latest_decision(),
            },
        }
    }

    pub fn needs_attention(&self) -> NeedsAttentionView {
        let summary = self.backend.summary();
        NeedsAttentionView {
            needs_attention: summary.needs_attention,
            state: summary.state,
            top_failure_class: summary.top_failure_class,
            latest_projection_digest: summary.latest_projection_digest,
            latest_instruction_id: summary.latest_instruction_id,
            coherence: summary.coherence,
        }
    }

    pub fn instruction(&self, instruction_id: &str) -> Option<InstructionSummary> {
        self.backend.instruction(instruction_id)
    }

    pub fn projection(&self, projection_digest: &str) -> Option<ProjectionView> {
        self.backend.projection(projection_digest)
    }

    pub fn query_json(&self, query: ObserveQuery) -> Result<Value, ObserveQueryError> {
        match query {
            ObserveQuery::Latest => serde_json::to_value(self.latest())
                .map_err(|e| ObserveQueryError::Serialization(e.to_string())),
            ObserveQuery::NeedsAttention => serde_json::to_value(self.needs_attention())
                .map_err(|e| ObserveQueryError::Serialization(e.to_string())),
            ObserveQuery::Instruction { instruction_id } => {
                let row = self.instruction(&instruction_id).ok_or_else(|| {
                    ObserveQueryError::InstructionNotFound(instruction_id.clone())
                })?;
                serde_json::to_value(row)
                    .map_err(|e| ObserveQueryError::Serialization(e.to_string()))
            }
            ObserveQuery::Projection { projection_digest } => {
                let row = self.projection(&projection_digest).ok_or_else(|| {
                    ObserveQueryError::ProjectionNotFound(projection_digest.clone())
                })?;
                serde_json::to_value(row)
                    .map_err(|e| ObserveQueryError::Serialization(e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use premath_surreal::{ObservationSummary, ProjectionView, RequiredSummary};

    #[derive(Clone)]
    struct MockBackend;

    impl ObservationBackend for MockBackend {
        fn summary(&self) -> ObservationSummary {
            ObservationSummary {
                state: "accepted".to_string(),
                needs_attention: false,
                top_failure_class: None,
                latest_projection_digest: Some("proj1_x".to_string()),
                latest_instruction_id: Some("i1".to_string()),
                required_check_count: 1,
                executed_check_count: 1,
                changed_path_count: 2,
                coherence: None,
            }
        }

        fn latest_delta(&self) -> Option<DeltaSummary> {
            None
        }

        fn latest_required(&self) -> Option<RequiredSummary> {
            Some(RequiredSummary {
                r#ref: "artifacts/ciwitness/latest-required.json".to_string(),
                witness_kind: Some("ci.required.v1".to_string()),
                projection_policy: Some("ci-topos-v0".to_string()),
                projection_digest: Some("proj1_x".to_string()),
                verdict_class: Some("accepted".to_string()),
                required_checks: vec!["baseline".to_string()],
                executed_checks: vec!["baseline".to_string()],
                failure_classes: vec![],
            })
        }

        fn latest_decision(&self) -> Option<DecisionSummary> {
            None
        }

        fn instruction(&self, _instruction_id: &str) -> Option<InstructionSummary> {
            None
        }

        fn projection(&self, projection_digest: &str) -> Option<ProjectionView> {
            Some(ProjectionView {
                projection_digest: projection_digest.to_string(),
                required: self.latest_required(),
                delta: None,
                decision: None,
            })
        }
    }

    #[test]
    fn latest_and_needs_attention_views_are_projected() {
        let service = UxService::new(MockBackend);
        let latest = service.latest();
        assert_eq!(latest.summary.state, "accepted");
        assert_eq!(
            latest
                .latest
                .required
                .expect("required should exist")
                .required_checks,
            vec!["baseline".to_string()]
        );

        let needs = service.needs_attention();
        assert!(!needs.needs_attention);
        assert_eq!(needs.latest_projection_digest.as_deref(), Some("proj1_x"));
    }

    #[test]
    fn projection_passthrough_works() {
        let service = UxService::new(MockBackend);
        let projection = service
            .projection("proj1_x")
            .expect("projection should exist");
        assert_eq!(projection.projection_digest, "proj1_x");
    }

    #[test]
    fn latest_view_serializes() {
        let service = UxService::new(MockBackend);
        let latest = service.latest();
        let serialized = serde_json::to_value(latest).expect("serialization should succeed");
        assert_eq!(serialized["summary"]["state"], "accepted");
    }

    #[test]
    fn query_json_reports_missing_rows() {
        let service = UxService::new(MockBackend);
        let err = service
            .query_json(ObserveQuery::Instruction {
                instruction_id: "missing".to_string(),
            })
            .expect_err("missing instruction should error");
        assert!(matches!(err, ObserveQueryError::InstructionNotFound(_)));
    }

    #[test]
    fn query_json_latest_roundtrip() {
        let service = UxService::new(MockBackend);
        let value = service
            .query_json(ObserveQuery::Latest)
            .expect("latest query should serialize");
        assert_eq!(value["summary"]["state"], "accepted");
    }
}
