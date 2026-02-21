use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

pub const OBSERVATION_SCHEMA: u64 = 1;
pub const OBSERVATION_KIND: &str = "ci.observation.surface.v0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSummary {
    pub state: String,
    pub needs_attention: bool,
    pub top_failure_class: Option<String>,
    pub latest_projection_digest: Option<String>,
    pub latest_instruction_id: Option<String>,
    pub required_check_count: u64,
    pub executed_check_count: u64,
    pub changed_path_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeltaSummary {
    pub r#ref: String,
    pub projection_policy: Option<String>,
    pub projection_digest: Option<String>,
    pub delta_source: Option<String>,
    pub from_ref: Option<String>,
    pub to_ref: Option<String>,
    pub changed_paths: Vec<String>,
    pub changed_path_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredSummary {
    pub r#ref: String,
    pub witness_kind: Option<String>,
    pub projection_policy: Option<String>,
    pub projection_digest: Option<String>,
    pub verdict_class: Option<String>,
    pub required_checks: Vec<String>,
    pub executed_checks: Vec<String>,
    pub failure_classes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DecisionSummary {
    pub r#ref: String,
    pub decision_kind: Option<String>,
    pub projection_digest: Option<String>,
    pub decision: Option<String>,
    pub reason_class: Option<String>,
    pub witness_path: Option<String>,
    pub delta_snapshot_path: Option<String>,
    pub required_checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstructionSummary {
    pub r#ref: String,
    pub witness_kind: Option<String>,
    pub instruction_id: String,
    pub instruction_digest: Option<String>,
    pub instruction_classification: Option<serde_json::Value>,
    pub intent: Option<String>,
    pub scope: Option<serde_json::Value>,
    pub policy_digest: Option<String>,
    pub verdict_class: Option<String>,
    pub required_checks: Vec<String>,
    pub executed_checks: Vec<String>,
    pub failure_classes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LatestObservation {
    pub delta: Option<DeltaSummary>,
    pub required: Option<RequiredSummary>,
    pub decision: Option<DecisionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSurface {
    pub schema: u64,
    pub surface_kind: String,
    pub summary: ObservationSummary,
    pub latest: LatestObservation,
    pub instructions: Vec<InstructionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionView {
    pub projection_digest: String,
    pub required: Option<RequiredSummary>,
    pub delta: Option<DeltaSummary>,
    pub decision: Option<DecisionSummary>,
}

#[derive(Debug, Error)]
pub enum ObservationError {
    #[error("failed to read observation surface: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse observation surface JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid observation surface: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone)]
pub struct ObservationIndex {
    surface: ObservationSurface,
    instruction_lookup: BTreeMap<String, usize>,
}

impl ObservationIndex {
    pub fn from_surface(mut surface: ObservationSurface) -> Result<Self, ObservationError> {
        if surface.schema != OBSERVATION_SCHEMA {
            return Err(ObservationError::Invalid(format!(
                "schema mismatch (expected={OBSERVATION_SCHEMA}, actual={})",
                surface.schema
            )));
        }
        if surface.surface_kind != OBSERVATION_KIND {
            return Err(ObservationError::Invalid(format!(
                "surfaceKind mismatch (expected={OBSERVATION_KIND}, actual={})",
                surface.surface_kind
            )));
        }

        surface
            .instructions
            .sort_by(|a, b| a.instruction_id.cmp(&b.instruction_id));

        let mut instruction_lookup = BTreeMap::new();
        for (idx, row) in surface.instructions.iter().enumerate() {
            instruction_lookup.insert(row.instruction_id.clone(), idx);
        }

        Ok(Self {
            surface,
            instruction_lookup,
        })
    }

    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, ObservationError> {
        let bytes = fs::read(path)?;
        let surface = serde_json::from_slice::<ObservationSurface>(&bytes)?;
        Self::from_surface(surface)
    }

    pub fn summary(&self) -> &ObservationSummary {
        &self.surface.summary
    }

    pub fn latest(&self) -> &LatestObservation {
        &self.surface.latest
    }

    pub fn surface(&self) -> &ObservationSurface {
        &self.surface
    }

    pub fn instruction(&self, instruction_id: &str) -> Option<&InstructionSummary> {
        self.instruction_lookup
            .get(instruction_id)
            .and_then(|idx| self.surface.instructions.get(*idx))
    }

    pub fn projection(&self, projection_digest: &str) -> Option<ProjectionView> {
        let required = self
            .surface
            .latest
            .required
            .clone()
            .filter(|row| row.projection_digest.as_deref() == Some(projection_digest));
        let delta = self
            .surface
            .latest
            .delta
            .clone()
            .filter(|row| row.projection_digest.as_deref() == Some(projection_digest));
        let decision = self
            .surface
            .latest
            .decision
            .clone()
            .filter(|row| row.projection_digest.as_deref() == Some(projection_digest));

        if required.is_none() && delta.is_none() && decision.is_none() {
            return None;
        }

        Some(ProjectionView {
            projection_digest: projection_digest.to_string(),
            required,
            delta,
            decision,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_surface() -> ObservationSurface {
        ObservationSurface {
            schema: OBSERVATION_SCHEMA,
            surface_kind: OBSERVATION_KIND.to_string(),
            summary: ObservationSummary {
                state: "accepted".to_string(),
                needs_attention: false,
                top_failure_class: Some("verified_accept".to_string()),
                latest_projection_digest: Some("proj1_alpha".to_string()),
                latest_instruction_id: Some("i1".to_string()),
                required_check_count: 1,
                executed_check_count: 1,
                changed_path_count: 2,
            },
            latest: LatestObservation {
                delta: Some(DeltaSummary {
                    r#ref: "artifacts/ciwitness/latest-delta.json".to_string(),
                    projection_policy: Some("ci-topos-v0".to_string()),
                    projection_digest: Some("proj1_alpha".to_string()),
                    delta_source: Some("git_diff+workspace".to_string()),
                    from_ref: Some("origin/main".to_string()),
                    to_ref: Some("HEAD".to_string()),
                    changed_paths: vec!["README.md".to_string(), "tools/ci/README.md".to_string()],
                    changed_path_count: 2,
                }),
                required: Some(RequiredSummary {
                    r#ref: "artifacts/ciwitness/latest-required.json".to_string(),
                    witness_kind: Some("ci.required.v1".to_string()),
                    projection_policy: Some("ci-topos-v0".to_string()),
                    projection_digest: Some("proj1_alpha".to_string()),
                    verdict_class: Some("accepted".to_string()),
                    required_checks: vec!["baseline".to_string()],
                    executed_checks: vec!["baseline".to_string()],
                    failure_classes: vec![],
                }),
                decision: Some(DecisionSummary {
                    r#ref: "artifacts/ciwitness/latest-decision.json".to_string(),
                    decision_kind: Some("ci.required.decision.v1".to_string()),
                    projection_digest: Some("proj1_alpha".to_string()),
                    decision: Some("accept".to_string()),
                    reason_class: Some("verified_accept".to_string()),
                    witness_path: None,
                    delta_snapshot_path: None,
                    required_checks: vec!["baseline".to_string()],
                }),
            },
            instructions: vec![InstructionSummary {
                r#ref: "artifacts/ciwitness/20260221T010000Z-ci-wiring-golden.json".to_string(),
                witness_kind: Some("ci.instruction.v1".to_string()),
                instruction_id: "20260221T010000Z-ci-wiring-golden".to_string(),
                instruction_digest: Some("instr1_alpha".to_string()),
                instruction_classification: None,
                intent: Some("validate wiring".to_string()),
                scope: None,
                policy_digest: Some("policy.ci.v1".to_string()),
                verdict_class: Some("accepted".to_string()),
                required_checks: vec!["ci-wiring-check".to_string()],
                executed_checks: vec!["ci-wiring-check".to_string()],
                failure_classes: vec![],
            }],
        }
    }

    #[test]
    fn instruction_lookup_and_projection_query() {
        let surface = sample_surface();
        let index = ObservationIndex::from_surface(surface).expect("surface should be valid");
        assert_eq!(index.summary().state, "accepted");
        assert!(
            index
                .instruction("20260221T010000Z-ci-wiring-golden")
                .is_some()
        );
        assert!(index.projection("proj1_alpha").is_some());
        assert!(index.projection("proj1_missing").is_none());
    }

    #[test]
    fn invalid_surface_kind_rejected() {
        let mut surface = sample_surface();
        surface.surface_kind = "wrong.kind".to_string();
        let err = ObservationIndex::from_surface(surface).expect_err("surface should be invalid");
        assert!(matches!(err, ObservationError::Invalid(_)));
    }
}
