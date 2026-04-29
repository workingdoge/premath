//! Checker endpoint for the raw work-tracker profile.
//!
//! This module intentionally checks normalized claims only. It does not define
//! `work.semantic_state`, tracker storage, or Tusk instrument behavior.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const SCHEMA: u64 = 1;
pub const CHECK_KIND: &str = "premath.work_tracker_checker.raw.v1";
pub const CHECKER_PROFILE_REF: &str = "premath://raw/WORK-TRACKER-CHECKER-PROFILE";
pub const REQUIRED_COVER_REF: &str = "atlas://work-tracker.v0";

pub const FAILURE_MISSING_AUTHORITY: &str = "work_checker.missing_authority";
pub const FAILURE_INVALID_BOUNDARY: &str = "work_checker.invalid_boundary";
pub const FAILURE_STALE_INPUT: &str = "work_checker.stale_input";
pub const FAILURE_UNSUPPORTED_OPERATION: &str = "work_checker.unsupported_operation";
pub const FAILURE_CONFLICTING_TRANSITION: &str = "work_checker.conflicting_transition";
pub const FAILURE_PROJECTION_AS_AUTHORITY: &str = "work_checker.projection_as_authority";
pub const FAILURE_INVALID_HANDOFF: &str = "work_checker.invalid_handoff";
pub const FAILURE_PROFILE_MISMATCH: &str = "work_checker.profile_mismatch";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkTrackerCheckInput {
    #[serde(default)]
    pub work_claim: Option<WorkClaim>,
    #[serde(default)]
    pub projection_check: Option<WorkProjectionCheck>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkClaim {
    #[serde(default)]
    pub cover_ref: Option<String>,
    #[serde(default)]
    pub checker_profile_ref: Option<String>,
    #[serde(default)]
    pub semantic_profile_ref: Option<String>,
    #[serde(default)]
    pub simplex_substrate_refs: Vec<String>,
    #[serde(default)]
    pub work_subject_ref: Option<String>,
    #[serde(default)]
    pub operation_class: Option<String>,
    #[serde(default)]
    pub declared_operation_classes: Vec<String>,
    #[serde(default)]
    pub prior_state_refs: Vec<String>,
    #[serde(default)]
    pub claimed_output_state_refs: Vec<String>,
    #[serde(default)]
    pub boundary_refs: Vec<String>,
    #[serde(default)]
    pub accepted_boundary_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub actor_ref: Option<String>,
    #[serde(default)]
    pub authority_input_kind: Option<String>,
    #[serde(default)]
    pub stale_state_refs: Vec<String>,
    #[serde(default)]
    pub conflict_refs: Vec<String>,
    #[serde(default)]
    pub graph_derived_semantics: bool,
    #[serde(default)]
    pub required_recovery_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkProjectionCheck {
    #[serde(default)]
    pub projection_ref: Option<String>,
    #[serde(default)]
    pub authority_decision_ref: Option<String>,
    #[serde(default)]
    pub derives_from: Option<String>,
    #[serde(default)]
    pub used_as_authority: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkTrackerCheckOutput {
    pub schema: u64,
    pub check_kind: String,
    pub result: String,
    pub failure_classes: Vec<String>,
    pub summary: WorkTrackerCheckSummary,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkTrackerCheckSummary {
    pub checked_claims: u64,
    pub checked_projections: u64,
    pub errors: u64,
}

pub fn evaluate_work_tracker_checker(input: &WorkTrackerCheckInput) -> WorkTrackerCheckOutput {
    let mut failures = BTreeSet::new();
    let mut errors = Vec::new();
    let mut checked_claims = 0;
    let mut checked_projections = 0;

    if let Some(claim) = input.work_claim.as_ref() {
        checked_claims = 1;
        evaluate_claim(claim, &mut failures, &mut errors);
    }

    if let Some(projection_check) = input.projection_check.as_ref() {
        checked_projections = 1;
        evaluate_projection_check(projection_check, &mut failures, &mut errors);
    }

    if checked_claims == 0 && checked_projections == 0 {
        failures.insert(FAILURE_MISSING_AUTHORITY.to_string());
        errors.push("payload must include workClaim or projectionCheck".to_string());
    }

    let failure_classes = failures.into_iter().collect::<Vec<_>>();
    let result = if failure_classes.is_empty() {
        "accepted"
    } else {
        "rejected"
    };
    WorkTrackerCheckOutput {
        schema: SCHEMA,
        check_kind: CHECK_KIND.to_string(),
        result: result.to_string(),
        failure_classes,
        summary: WorkTrackerCheckSummary {
            checked_claims,
            checked_projections,
            errors: errors.len() as u64,
        },
        errors,
    }
}

fn evaluate_claim(claim: &WorkClaim, failures: &mut BTreeSet<String>, errors: &mut Vec<String>) {
    let missing = missing_claim_fields(claim);
    if !missing.is_empty() {
        failures.insert(FAILURE_MISSING_AUTHORITY.to_string());
        errors.push(format!(
            "claim missing required authority fields: {}",
            missing.join(", ")
        ));
    }

    if let Some(cover_ref) = clean_string(claim.cover_ref.as_deref())
        && cover_ref != REQUIRED_COVER_REF
    {
        failures.insert(FAILURE_PROFILE_MISMATCH.to_string());
        errors.push(format!("coverRef must be {REQUIRED_COVER_REF:?}"));
    }
    if let Some(checker_profile_ref) = clean_string(claim.checker_profile_ref.as_deref())
        && checker_profile_ref != CHECKER_PROFILE_REF
    {
        failures.insert(FAILURE_PROFILE_MISMATCH.to_string());
        errors.push(format!("checkerProfileRef must be {CHECKER_PROFILE_REF:?}"));
    }
    if let Some(semantic_profile_ref) = clean_string(claim.semantic_profile_ref.as_deref())
        && !semantic_profile_ref.starts_with("work://")
    {
        failures.insert(FAILURE_PROFILE_MISMATCH.to_string());
        errors.push("semanticProfileRef must be work-owned".to_string());
    }

    if let Some(operation_class) = clean_string(claim.operation_class.as_deref()) {
        let declared_operations = clean_string_set(&claim.declared_operation_classes);
        if !declared_operations.is_empty() && !declared_operations.contains(operation_class) {
            failures.insert(FAILURE_UNSUPPORTED_OPERATION.to_string());
            errors.push(format!(
                "operationClass {operation_class:?} not declared by semantic profile"
            ));
        }

        if operation_class == "handoff" {
            let required = clean_string_set(&claim.required_recovery_evidence_refs);
            let evidence = clean_string_set(&claim.evidence_refs);
            let missing_recovery = required
                .difference(&evidence)
                .cloned()
                .collect::<Vec<String>>();
            if !missing_recovery.is_empty() {
                failures.insert(FAILURE_INVALID_HANDOFF.to_string());
                errors.push(format!(
                    "handoff missing recovery evidence: {}",
                    missing_recovery.join(", ")
                ));
            }
        }
    }

    let accepted_boundary_refs = clean_string_set(&claim.accepted_boundary_refs);
    let missing_boundary_refs = clean_string_set(&claim.boundary_refs)
        .difference(&accepted_boundary_refs)
        .cloned()
        .collect::<Vec<String>>();
    if !missing_boundary_refs.is_empty() {
        failures.insert(FAILURE_INVALID_BOUNDARY.to_string());
        errors.push(format!(
            "boundary refs lack accepted evidence: {}",
            missing_boundary_refs.join(", ")
        ));
    }

    let stale_state_refs = clean_string_set(&claim.prior_state_refs)
        .intersection(&clean_string_set(&claim.stale_state_refs))
        .cloned()
        .collect::<Vec<String>>();
    if !stale_state_refs.is_empty() {
        failures.insert(FAILURE_STALE_INPUT.to_string());
        errors.push(format!(
            "prior state refs are stale: {}",
            stale_state_refs.join(", ")
        ));
    }

    let conflict_refs = clean_string_vec(&claim.conflict_refs);
    if !conflict_refs.is_empty() {
        failures.insert(FAILURE_CONFLICTING_TRANSITION.to_string());
        errors.push(format!(
            "conflicting transition refs present: {}",
            conflict_refs.join(", ")
        ));
    }

    let authority_input_kind = clean_string(claim.authority_input_kind.as_deref());
    if authority_input_kind == Some("projection")
        || has_projection_ref(&claim.prior_state_refs)
        || has_projection_ref(&claim.claimed_output_state_refs)
    {
        failures.insert(FAILURE_PROJECTION_AS_AUTHORITY.to_string());
        errors.push("projection input used as mutation authority".to_string());
    }

    if claim.graph_derived_semantics {
        failures.insert(FAILURE_MISSING_AUTHORITY.to_string());
        errors.push("checker input relies on graph shape as work semantics".to_string());
    }
}

fn evaluate_projection_check(
    projection_check: &WorkProjectionCheck,
    failures: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    if clean_string(projection_check.projection_ref.as_deref()).is_none() {
        failures.insert(FAILURE_MISSING_AUTHORITY.to_string());
        errors.push("projection check missing projectionRef".to_string());
    }
    if clean_string(projection_check.authority_decision_ref.as_deref()).is_none()
        && clean_string(projection_check.derives_from.as_deref()).is_none()
    {
        failures.insert(FAILURE_MISSING_AUTHORITY.to_string());
        errors.push("projection check missing authority decision derivation".to_string());
    }
    if projection_check.used_as_authority {
        failures.insert(FAILURE_PROJECTION_AS_AUTHORITY.to_string());
        errors.push("projection marked as mutation authority".to_string());
    }
}

fn missing_claim_fields(claim: &WorkClaim) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if clean_string(claim.cover_ref.as_deref()).is_none() {
        missing.push("coverRef");
    }
    if clean_string(claim.checker_profile_ref.as_deref()).is_none() {
        missing.push("checkerProfileRef");
    }
    if clean_string(claim.semantic_profile_ref.as_deref()).is_none() {
        missing.push("semanticProfileRef");
    }
    if clean_string_vec(&claim.simplex_substrate_refs).is_empty() {
        missing.push("simplexSubstrateRefs");
    }
    if clean_string(claim.work_subject_ref.as_deref()).is_none() {
        missing.push("workSubjectRef");
    }
    if clean_string(claim.operation_class.as_deref()).is_none() {
        missing.push("operationClass");
    }
    if clean_string_vec(&claim.prior_state_refs).is_empty() {
        missing.push("priorStateRefs");
    }
    if clean_string_vec(&claim.claimed_output_state_refs).is_empty() {
        missing.push("claimedOutputStateRefs");
    }
    if clean_string_vec(&claim.evidence_refs).is_empty() {
        missing.push("evidenceRefs");
    }
    missing
}

fn clean_string(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn clean_string_vec(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn clean_string_set(values: &[String]) -> BTreeSet<String> {
    clean_string_vec(values).into_iter().collect()
}

fn has_projection_ref(values: &[String]) -> bool {
    clean_string_vec(values)
        .iter()
        .any(|value| value.starts_with("projection://") || value.starts_with("bd://"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_claim() -> WorkClaim {
        WorkClaim {
            cover_ref: Some(REQUIRED_COVER_REF.to_string()),
            checker_profile_ref: Some(CHECKER_PROFILE_REF.to_string()),
            semantic_profile_ref: Some("work://candidate/work-tracker.v0".to_string()),
            simplex_substrate_refs: vec!["simplex://nerve-provisional/sigma/root".to_string()],
            work_subject_ref: Some("work://subject/bd-101".to_string()),
            operation_class: Some("claim".to_string()),
            declared_operation_classes: vec!["claim".to_string()],
            prior_state_refs: vec!["work-state://bd-101/open".to_string()],
            claimed_output_state_refs: vec!["work-state://bd-101/in-progress".to_string()],
            evidence_refs: vec!["witness://claim/bd-101/001".to_string()],
            ..Default::default()
        }
    }

    #[test]
    fn accepts_explicit_authority_claim() {
        let output = evaluate_work_tracker_checker(&WorkTrackerCheckInput {
            work_claim: Some(base_claim()),
            projection_check: None,
        });
        assert_eq!(output.result, "accepted");
        assert!(output.failure_classes.is_empty());
    }

    #[test]
    fn rejects_projection_as_authority() {
        let mut claim = base_claim();
        claim.prior_state_refs = vec!["projection://tusk/ready-list/001".to_string()];
        let output = evaluate_work_tracker_checker(&WorkTrackerCheckInput {
            work_claim: Some(claim),
            projection_check: None,
        });
        assert_eq!(output.result, "rejected");
        assert_eq!(
            output.failure_classes,
            vec![FAILURE_PROJECTION_AS_AUTHORITY.to_string()]
        );
    }
}
