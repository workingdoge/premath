use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

const REQUIRED_WITNESS_KIND: &str = "ci.required.v1";

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{failure_class}: {message}")]
pub struct RequiredWitnessError {
    pub failure_class: String,
    pub message: String,
}

impl RequiredWitnessError {
    fn new(failure_class: &str, message: impl Into<String>) -> Self {
        Self {
            failure_class: failure_class.to_string(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutedRequiredCheck {
    pub check_id: String,
    pub status: String,
    pub exit_code: i64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredGateWitnessRef {
    pub check_id: String,
    pub artifact_rel_path: String,
    pub sha256: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(default)]
    pub failure_classes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredWitnessRuntime {
    pub projection_policy: String,
    pub projection_digest: String,
    pub changed_paths: Vec<String>,
    pub required_checks: Vec<String>,
    pub results: Vec<ExecutedRequiredCheck>,
    pub gate_witness_refs: Vec<RequiredGateWitnessRef>,
    pub docs_only: bool,
    pub reasons: Vec<String>,
    pub delta_source: String,
    pub from_ref: Option<String>,
    pub to_ref: Option<String>,
    pub normalizer_id: String,
    pub policy_digest: String,
    pub squeak_site_profile: String,
    pub run_started_at: String,
    pub run_finished_at: String,
    pub run_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequiredWitness {
    pub ci_schema: u32,
    pub witness_kind: String,
    pub projection_policy: String,
    pub projection_digest: String,
    pub changed_paths: Vec<String>,
    pub required_checks: Vec<String>,
    pub executed_checks: Vec<String>,
    pub results: Vec<ExecutedRequiredCheck>,
    pub gate_witness_refs: Vec<RequiredGateWitnessRef>,
    pub verdict_class: String,
    pub operational_failure_classes: Vec<String>,
    pub semantic_failure_classes: Vec<String>,
    pub failure_classes: Vec<String>,
    pub docs_only: bool,
    pub reasons: Vec<String>,
    pub delta_source: String,
    pub from_ref: Option<String>,
    pub to_ref: Option<String>,
    pub normalizer_id: String,
    pub policy_digest: String,
    pub typed_core_projection_digest: String,
    pub authority_payload_digest: String,
    pub squeak_site_profile: String,
    pub run_started_at: String,
    pub run_finished_at: String,
    pub run_duration_ms: u64,
}

fn ensure_non_empty(value: &str, label: &str) -> Result<String, RequiredWitnessError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(RequiredWitnessError::new(
            "required_witness_runtime_invalid",
            format!("{label} must be a non-empty string"),
        ));
    }
    Ok(trimmed.to_string())
}

fn ensure_non_empty_list(
    values: Vec<String>,
    label: &str,
) -> Result<Vec<String>, RequiredWitnessError> {
    let mut out = Vec::with_capacity(values.len());
    for (idx, value) in values.into_iter().enumerate() {
        out.push(ensure_non_empty(
            &value,
            format!("{label}[{idx}]").as_str(),
        )?);
    }
    Ok(out)
}

fn ensure_optional_non_empty(
    value: Option<String>,
    label: &str,
) -> Result<Option<String>, RequiredWitnessError> {
    match value {
        None => Ok(None),
        Some(raw) => Ok(Some(ensure_non_empty(&raw, label)?)),
    }
}

fn sorted_unique_non_empty(values: &[String]) -> Vec<String> {
    let mut out = BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            out.insert(trimmed.to_string());
        }
    }
    out.into_iter().collect()
}

pub fn compute_typed_core_projection_digest(
    authority_payload_digest: &str,
    normalizer_id: &str,
    policy_digest: &str,
) -> String {
    let mut hasher = Sha256::new();
    for part in [authority_payload_digest, normalizer_id, policy_digest] {
        hasher.update(part.as_bytes());
        hasher.update([0u8]);
    }
    format!("ev1_{:x}", hasher.finalize())
}

pub fn build_required_witness(
    runtime: RequiredWitnessRuntime,
) -> Result<RequiredWitness, RequiredWitnessError> {
    let projection_policy = ensure_non_empty(&runtime.projection_policy, "projectionPolicy")?;
    let projection_digest = ensure_non_empty(&runtime.projection_digest, "projectionDigest")?;
    let changed_paths = ensure_non_empty_list(runtime.changed_paths, "changedPaths")?;
    let required_checks = ensure_non_empty_list(runtime.required_checks, "requiredChecks")?;
    let reasons = ensure_non_empty_list(runtime.reasons, "reasons")?;
    let delta_source = ensure_non_empty(&runtime.delta_source, "deltaSource")?;
    let normalizer_id = ensure_non_empty(&runtime.normalizer_id, "normalizerId")?;
    let policy_digest = ensure_non_empty(&runtime.policy_digest, "policyDigest")?;
    if policy_digest != projection_policy {
        return Err(RequiredWitnessError::new(
            "required_witness_runtime_invalid",
            format!(
                "policyDigest must match projectionPolicy ({policy_digest:?} != {projection_policy:?})"
            ),
        ));
    }
    let squeak_site_profile = ensure_non_empty(&runtime.squeak_site_profile, "squeakSiteProfile")?;
    let run_started_at = ensure_non_empty(&runtime.run_started_at, "runStartedAt")?;
    let run_finished_at = ensure_non_empty(&runtime.run_finished_at, "runFinishedAt")?;
    let from_ref = ensure_optional_non_empty(runtime.from_ref, "fromRef")?;
    let to_ref = ensure_optional_non_empty(runtime.to_ref, "toRef")?;

    let mut results: Vec<ExecutedRequiredCheck> = Vec::with_capacity(runtime.results.len());
    for (idx, row) in runtime.results.into_iter().enumerate() {
        let check_id = ensure_non_empty(&row.check_id, format!("results[{idx}].checkId").as_str())?;
        let status = ensure_non_empty(&row.status, format!("results[{idx}].status").as_str())?;
        if status != "passed" && status != "failed" {
            return Err(RequiredWitnessError::new(
                "required_witness_runtime_invalid",
                format!("results[{idx}].status must be 'passed' or 'failed'"),
            ));
        }
        let expected_status = if row.exit_code == 0 {
            "passed"
        } else {
            "failed"
        };
        if status != expected_status {
            return Err(RequiredWitnessError::new(
                "required_witness_runtime_invalid",
                format!(
                    "results[{idx}] status/exitCode mismatch (status={status:?}, exitCode={})",
                    row.exit_code
                ),
            ));
        }
        results.push(ExecutedRequiredCheck {
            check_id,
            status,
            exit_code: row.exit_code,
            duration_ms: row.duration_ms,
        });
    }

    let executed_checks: Vec<String> = results.iter().map(|row| row.check_id.clone()).collect();

    let mut gate_witness_refs: Vec<RequiredGateWitnessRef> =
        Vec::with_capacity(runtime.gate_witness_refs.len());
    for (idx, row) in runtime.gate_witness_refs.into_iter().enumerate() {
        let check_id = ensure_non_empty(
            &row.check_id,
            format!("gateWitnessRefs[{idx}].checkId").as_str(),
        )?;
        let artifact_rel_path = ensure_non_empty(
            &row.artifact_rel_path,
            format!("gateWitnessRefs[{idx}].artifactRelPath").as_str(),
        )?;
        let sha256 = ensure_non_empty(
            &row.sha256,
            format!("gateWitnessRefs[{idx}].sha256").as_str(),
        )?;
        let source = ensure_non_empty(
            &row.source,
            format!("gateWitnessRefs[{idx}].source").as_str(),
        )?;
        if source != "native" && source != "fallback" {
            return Err(RequiredWitnessError::new(
                "required_witness_runtime_invalid",
                format!("gateWitnessRefs[{idx}].source must be 'native' or 'fallback'"),
            ));
        }
        gate_witness_refs.push(RequiredGateWitnessRef {
            check_id,
            artifact_rel_path,
            sha256,
            source,
            run_id: ensure_optional_non_empty(
                row.run_id,
                format!("gateWitnessRefs[{idx}].runId").as_str(),
            )?,
            witness_kind: ensure_optional_non_empty(
                row.witness_kind,
                format!("gateWitnessRefs[{idx}].witnessKind").as_str(),
            )?,
            result: ensure_optional_non_empty(
                row.result,
                format!("gateWitnessRefs[{idx}].result").as_str(),
            )?,
            failure_classes: sorted_unique_non_empty(&row.failure_classes),
        });
    }

    let failed = results.iter().any(|row| row.exit_code != 0);
    let operational_failure_classes = if failed {
        vec!["check_failed".to_string()]
    } else {
        Vec::new()
    };
    let semantic_failure_classes = sorted_unique_non_empty(
        &gate_witness_refs
            .iter()
            .flat_map(|row| row.failure_classes.iter().cloned())
            .collect::<Vec<String>>(),
    );
    let failure_classes = sorted_unique_non_empty(
        &operational_failure_classes
            .iter()
            .chain(semantic_failure_classes.iter())
            .cloned()
            .collect::<Vec<String>>(),
    );

    let authority_payload_digest = projection_digest.clone();
    let typed_core_projection_digest = compute_typed_core_projection_digest(
        &authority_payload_digest,
        &normalizer_id,
        &policy_digest,
    );

    Ok(RequiredWitness {
        ci_schema: 1,
        witness_kind: REQUIRED_WITNESS_KIND.to_string(),
        projection_policy,
        projection_digest,
        changed_paths,
        required_checks,
        executed_checks,
        results,
        gate_witness_refs,
        verdict_class: if failed {
            "rejected".to_string()
        } else {
            "accepted".to_string()
        },
        operational_failure_classes,
        semantic_failure_classes,
        failure_classes,
        docs_only: runtime.docs_only,
        reasons,
        delta_source,
        from_ref,
        to_ref,
        normalizer_id,
        policy_digest,
        typed_core_projection_digest,
        authority_payload_digest,
        squeak_site_profile,
        run_started_at,
        run_finished_at,
        run_duration_ms: runtime.run_duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime(failed: bool, semantic_classes: Vec<String>) -> RequiredWitnessRuntime {
        RequiredWitnessRuntime {
            projection_policy: "ci-topos-v0".to_string(),
            projection_digest: "proj1_demo".to_string(),
            changed_paths: vec!["README.md".to_string()],
            required_checks: vec!["baseline".to_string()],
            results: vec![ExecutedRequiredCheck {
                check_id: "baseline".to_string(),
                status: if failed {
                    "failed".to_string()
                } else {
                    "passed".to_string()
                },
                exit_code: if failed { 1 } else { 0 },
                duration_ms: 25,
            }],
            gate_witness_refs: vec![RequiredGateWitnessRef {
                check_id: "baseline".to_string(),
                artifact_rel_path: "gates/proj1_demo/01-baseline.json".to_string(),
                sha256: "abc123".to_string(),
                source: "native".to_string(),
                run_id: Some("run1_demo".to_string()),
                witness_kind: Some("gate".to_string()),
                result: Some(if failed {
                    "rejected".to_string()
                } else {
                    "accepted".to_string()
                }),
                failure_classes: semantic_classes,
            }],
            docs_only: false,
            reasons: vec!["kernel_or_ci_or_governance_change".to_string()],
            delta_source: "explicit".to_string(),
            from_ref: Some("origin/main".to_string()),
            to_ref: Some("HEAD".to_string()),
            normalizer_id: "normalizer.ci.required.v1".to_string(),
            policy_digest: "ci-topos-v0".to_string(),
            squeak_site_profile: "local".to_string(),
            run_started_at: "2026-02-22T00:00:00Z".to_string(),
            run_finished_at: "2026-02-22T00:00:01Z".to_string(),
            run_duration_ms: 1000,
        }
    }

    #[test]
    fn build_required_witness_accepts_passed_results() {
        let witness =
            build_required_witness(runtime(false, vec![])).expect("witness build should pass");
        assert_eq!(witness.verdict_class, "accepted");
        assert_eq!(witness.operational_failure_classes, Vec::<String>::new());
        assert_eq!(witness.semantic_failure_classes, Vec::<String>::new());
        assert_eq!(witness.failure_classes, Vec::<String>::new());
        assert_eq!(witness.executed_checks, vec!["baseline".to_string()]);
    }

    #[test]
    fn build_required_witness_unions_operational_and_semantic_failures() {
        let witness = build_required_witness(runtime(
            true,
            vec![
                "descent_failure".to_string(),
                "descent_failure".to_string(),
                "locality_failure".to_string(),
            ],
        ))
        .expect("witness build should pass");
        assert_eq!(witness.verdict_class, "rejected");
        assert_eq!(
            witness.semantic_failure_classes,
            vec![
                "descent_failure".to_string(),
                "locality_failure".to_string()
            ]
        );
        assert_eq!(
            witness.failure_classes,
            vec![
                "check_failed".to_string(),
                "descent_failure".to_string(),
                "locality_failure".to_string()
            ]
        );
    }

    #[test]
    fn build_required_witness_rejects_policy_mismatch() {
        let mut payload = runtime(false, vec![]);
        payload.policy_digest = "ci-topos-v1".to_string();
        let err = build_required_witness(payload).expect_err("policy mismatch should reject");
        assert_eq!(err.failure_class, "required_witness_runtime_invalid");
        assert!(
            err.message
                .contains("policyDigest must match projectionPolicy")
        );
    }
}
