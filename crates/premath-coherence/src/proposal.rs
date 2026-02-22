use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

const PROPOSAL_KINDS: &[&str] = &["value", "derivation", "refinementPlan"];

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{failure_class}: {message}")]
pub struct ProposalError {
    pub failure_class: String,
    pub message: String,
}

impl ProposalError {
    fn new(failure_class: &str, message: impl Into<String>) -> Self {
        Self {
            failure_class: failure_class.to_string(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalBinding {
    pub normalizer_id: String,
    pub policy_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalTargetJudgment {
    pub kind: String,
    pub shape: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalStep {
    pub rule_id: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub claim: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalProposal {
    pub proposal_kind: String,
    pub target_ctx_ref: String,
    pub target_judgment: ProposalTargetJudgment,
    pub candidate_refs: Vec<String>,
    pub binding: ProposalBinding,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<ProposalStep>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedProposal {
    pub canonical: CanonicalProposal,
    pub digest: String,
    pub kcir_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalObligationContext {
    pub r#ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalObligationSubject {
    pub kind: String,
    pub r#ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalObligationDetails {
    pub proposal_kind: String,
    pub candidate_count: usize,
    pub step_count: usize,
    pub obligation_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalObligation {
    pub obligation_id: String,
    pub kind: String,
    pub ctx: ProposalObligationContext,
    pub subject: ProposalObligationSubject,
    pub details: ProposalObligationDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalDischargeStep {
    pub obligation_id: String,
    pub kind: String,
    pub status: String,
    pub mode: String,
    pub binding: ProposalBinding,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub law_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalDischarge {
    pub mode: String,
    pub binding: ProposalBinding,
    pub outcome: String,
    pub steps: Vec<ProposalDischargeStep>,
    pub failure_classes: Vec<String>,
}

fn sort_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            let mut sorted = Map::new();
            for key in keys {
                if let Some(item) = map.get(key) {
                    sorted.insert(key.clone(), sort_json_value(item));
                }
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_json_value).collect()),
        _ => value.clone(),
    }
}

fn canonical_json(value: &Value) -> String {
    serde_json::to_string(&sort_json_value(value))
        .expect("canonical json serialization should succeed")
}

fn stable_hash(value: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_json(value).as_bytes());
    format!("{:x}", hasher.finalize())
}

fn ensure_non_empty_string(
    value: Option<&Value>,
    label: &str,
    failure_class: &str,
) -> Result<String, ProposalError> {
    let Some(raw) = value else {
        return Err(ProposalError::new(
            failure_class,
            format!("{label} must be a non-empty string"),
        ));
    };
    let Some(text) = raw.as_str() else {
        return Err(ProposalError::new(
            failure_class,
            format!("{label} must be a non-empty string"),
        ));
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(ProposalError::new(
            failure_class,
            format!("{label} must be a non-empty string"),
        ));
    }
    Ok(trimmed.to_string())
}

fn as_object<'a>(
    raw: &'a Value,
    failure_class: &str,
    message: &str,
) -> Result<&'a Map<String, Value>, ProposalError> {
    raw.as_object()
        .ok_or_else(|| ProposalError::new(failure_class, message.to_string()))
}

pub fn compute_proposal_digest(canonical: &CanonicalProposal) -> String {
    let payload = serde_json::to_value(canonical).expect("proposal should serialize");
    format!("prop1_{}", stable_hash(&payload))
}

pub fn compute_proposal_kcir_ref(canonical: &CanonicalProposal) -> String {
    let payload = json!({
        "kind": "kcir.proposal.v1",
        "canonicalProposal": canonical,
    });
    format!("kcir1_{}", stable_hash(&payload))
}

pub fn validate_proposal_payload(raw: &Value) -> Result<ValidatedProposal, ProposalError> {
    let root = as_object(raw, "proposal_invalid_shape", "proposal must be an object")?;

    let proposal_kind = ensure_non_empty_string(
        root.get("proposalKind"),
        "proposal.proposalKind",
        "proposal_invalid_kind",
    )?;
    if !PROPOSAL_KINDS.iter().any(|item| *item == proposal_kind) {
        return Err(ProposalError::new(
            "proposal_invalid_kind",
            format!("proposal.proposalKind must be one of {:?}", PROPOSAL_KINDS),
        ));
    }

    let target_ctx_ref = ensure_non_empty_string(
        root.get("targetCtxRef"),
        "proposal.targetCtxRef",
        "proposal_invalid_target",
    )?;

    let target_judgment_raw = root.get("targetJudgment").ok_or_else(|| {
        ProposalError::new(
            "proposal_invalid_target_judgment",
            "proposal.targetJudgment must be an object",
        )
    })?;
    let target_judgment = as_object(
        target_judgment_raw,
        "proposal_invalid_target_judgment",
        "proposal.targetJudgment must be an object",
    )?;

    let target_kind = ensure_non_empty_string(
        target_judgment.get("kind"),
        "proposal.targetJudgment.kind",
        "proposal_invalid_target_judgment",
    )?;
    if target_kind != "obj" && target_kind != "mor" {
        return Err(ProposalError::new(
            "proposal_invalid_target_judgment",
            "proposal.targetJudgment.kind must be 'obj' or 'mor'",
        ));
    }
    let target_shape = ensure_non_empty_string(
        target_judgment.get("shape"),
        "proposal.targetJudgment.shape",
        "proposal_invalid_target_judgment",
    )?;

    let binding_raw = root.get("binding").ok_or_else(|| {
        ProposalError::new(
            "proposal_unbound_policy",
            "proposal.binding must be an object",
        )
    })?;
    let binding = as_object(
        binding_raw,
        "proposal_unbound_policy",
        "proposal.binding must be an object",
    )?;
    let normalizer_id = ensure_non_empty_string(
        binding.get("normalizerId"),
        "proposal.binding.normalizerId",
        "proposal_unbound_policy",
    )?;
    let policy_digest = ensure_non_empty_string(
        binding.get("policyDigest"),
        "proposal.binding.policyDigest",
        "proposal_unbound_policy",
    )?;

    let mut candidate_refs = Vec::new();
    if let Some(candidate_refs_raw) = root.get("candidateRefs") {
        let candidate_refs_list = candidate_refs_raw.as_array().ok_or_else(|| {
            ProposalError::new(
                "proposal_invalid_step",
                "proposal.candidateRefs must be a list",
            )
        })?;
        for (idx, item) in candidate_refs_list.iter().enumerate() {
            candidate_refs.push(ensure_non_empty_string(
                Some(item),
                format!("proposal.candidateRefs[{idx}]").as_str(),
                "proposal_invalid_step",
            )?);
        }
    }
    candidate_refs.sort();
    candidate_refs.dedup();

    let steps_list = if let Some(steps_raw) = root.get("steps") {
        Some(steps_raw.as_array().ok_or_else(|| {
            ProposalError::new("proposal_invalid_step", "proposal.steps must be a list")
        })?)
    } else {
        None
    };
    let has_steps = steps_list.is_some_and(|items| !items.is_empty());

    if proposal_kind == "derivation" && !has_steps {
        return Err(ProposalError::new(
            "proposal_invalid_step",
            "proposal.steps must be non-empty for derivation proposals",
        ));
    }
    if proposal_kind != "derivation" && has_steps {
        return Err(ProposalError::new(
            "proposal_invalid_step",
            "proposal.steps is only valid for derivation proposals",
        ));
    }

    let mut steps = Vec::new();
    if let Some(step_rows) = steps_list {
        for (idx, step_raw) in step_rows.iter().enumerate() {
            let step = as_object(
                step_raw,
                "proposal_invalid_step",
                format!("proposal.steps[{idx}] must be an object").as_str(),
            )?;
            let rule_id = ensure_non_empty_string(
                step.get("ruleId"),
                format!("proposal.steps[{idx}].ruleId").as_str(),
                "proposal_invalid_step",
            )?;
            let claim = ensure_non_empty_string(
                step.get("claim"),
                format!("proposal.steps[{idx}].claim").as_str(),
                "proposal_invalid_step",
            )?;

            let mut inputs = Vec::new();
            if let Some(inputs_raw) = step.get("inputs") {
                let inputs_list = inputs_raw.as_array().ok_or_else(|| {
                    ProposalError::new(
                        "proposal_invalid_step",
                        format!("proposal.steps[{idx}].inputs/outputs must be lists"),
                    )
                })?;
                for (jdx, item) in inputs_list.iter().enumerate() {
                    inputs.push(ensure_non_empty_string(
                        Some(item),
                        format!("proposal.steps[{idx}].inputs[{jdx}]").as_str(),
                        "proposal_invalid_step",
                    )?);
                }
            }
            let mut outputs = Vec::new();
            if let Some(outputs_raw) = step.get("outputs") {
                let outputs_list = outputs_raw.as_array().ok_or_else(|| {
                    ProposalError::new(
                        "proposal_invalid_step",
                        format!("proposal.steps[{idx}].inputs/outputs must be lists"),
                    )
                })?;
                for (jdx, item) in outputs_list.iter().enumerate() {
                    outputs.push(ensure_non_empty_string(
                        Some(item),
                        format!("proposal.steps[{idx}].outputs[{jdx}]").as_str(),
                        "proposal_invalid_step",
                    )?);
                }
            }

            steps.push(ProposalStep {
                rule_id,
                inputs,
                outputs,
                claim,
            });
        }
    }

    let canonical = CanonicalProposal {
        proposal_kind: proposal_kind.clone(),
        target_ctx_ref,
        target_judgment: ProposalTargetJudgment {
            kind: target_kind,
            shape: target_shape,
        },
        candidate_refs,
        binding: ProposalBinding {
            normalizer_id,
            policy_digest,
        },
        steps: if steps.is_empty() { None } else { Some(steps) },
    };

    let digest = compute_proposal_digest(&canonical);
    let kcir_ref = compute_proposal_kcir_ref(&canonical);

    if let Some(raw_declared_digest) = root.get("proposalDigest") {
        let declared_digest = ensure_non_empty_string(
            Some(raw_declared_digest),
            "proposal.proposalDigest",
            "proposal_nondeterministic",
        )?;
        if declared_digest != digest {
            return Err(ProposalError::new(
                "proposal_nondeterministic",
                "proposal.proposalDigest does not match canonical payload digest",
            ));
        }
    }
    if let Some(raw_declared_kcir_ref) = root.get("proposalKcirRef") {
        let declared_kcir_ref = ensure_non_empty_string(
            Some(raw_declared_kcir_ref),
            "proposal.proposalKcirRef",
            "proposal_kcir_ref_mismatch",
        )?;
        if declared_kcir_ref != kcir_ref {
            return Err(ProposalError::new(
                "proposal_kcir_ref_mismatch",
                "proposal.proposalKcirRef does not match canonical KCIR ref",
            ));
        }
    }

    Ok(ValidatedProposal {
        canonical,
        digest,
        kcir_ref,
    })
}

fn proposal_has_discharge_candidate(canonical: &CanonicalProposal) -> bool {
    if !canonical.candidate_refs.is_empty() {
        return true;
    }
    canonical
        .steps
        .as_ref()
        .is_some_and(|steps| steps.iter().any(|step| !step.outputs.is_empty()))
}

fn proposal_subject_ref(canonical: &CanonicalProposal) -> String {
    if let Some(first) = canonical.candidate_refs.first() {
        return first.clone();
    }
    if let Some(steps) = &canonical.steps {
        for step in steps.iter().rev() {
            if let Some(first) = step.outputs.first() {
                return first.clone();
            }
        }
    }
    format!(
        "{}#{}",
        canonical.target_ctx_ref, canonical.target_judgment.kind
    )
}

pub fn compile_proposal_obligations(canonical: &CanonicalProposal) -> Vec<ProposalObligation> {
    let candidate_count = canonical.candidate_refs.len();
    let step_count = canonical.steps.as_ref().map_or(0, |steps| steps.len());

    let mut obligation_kinds = vec!["stability".to_string(), "locality".to_string()];
    if proposal_has_discharge_candidate(canonical) {
        obligation_kinds.push("descent_exists".to_string());
    } else {
        obligation_kinds.push("ext_gap".to_string());
    }
    if canonical.proposal_kind == "value" && candidate_count > 1 {
        obligation_kinds.push("ext_ambiguous".to_string());
    }
    if canonical.proposal_kind == "refinementPlan" {
        obligation_kinds.extend(
            [
                "adjoint_triple",
                "adjoint_triangle",
                "beck_chevalley_sigma",
                "beck_chevalley_pi",
                "refinement_invariance",
            ]
            .iter()
            .map(|item| item.to_string()),
        );
    }

    let subject_ref = proposal_subject_ref(canonical);
    obligation_kinds
        .into_iter()
        .enumerate()
        .map(|(idx, kind)| {
            let details = ProposalObligationDetails {
                proposal_kind: canonical.proposal_kind.clone(),
                candidate_count,
                step_count,
                obligation_index: idx,
            };
            let core = json!({
                "kind": kind,
                "ctx": { "ref": canonical.target_ctx_ref },
                "subject": {
                    "kind": canonical.target_judgment.kind,
                    "ref": subject_ref,
                },
                "details": details,
            });
            let obligation_id = format!("obl1_{}", stable_hash(&core));
            ProposalObligation {
                obligation_id,
                kind: core
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                ctx: ProposalObligationContext {
                    r#ref: canonical.target_ctx_ref.clone(),
                },
                subject: ProposalObligationSubject {
                    kind: canonical.target_judgment.kind.clone(),
                    r#ref: subject_ref.clone(),
                },
                details,
            }
        })
        .collect()
}

fn obligation_to_failure(kind: &str) -> &'static str {
    match kind {
        "stability" => "stability_failure",
        "locality" => "locality_failure",
        "descent_exists" => "descent_failure",
        "descent_contractible" => "glue_non_contractible",
        "adjoint_triangle" => "adjoint_triple_coherence_failure",
        "beck_chevalley_sigma" => "adjoint_triple_coherence_failure",
        "beck_chevalley_pi" => "adjoint_triple_coherence_failure",
        "refinement_invariance" => "stability_failure",
        "adjoint_triple" => "adjoint_triple_coherence_failure",
        "ext_gap" => "descent_failure",
        "ext_ambiguous" => "glue_non_contractible",
        _ => "descent_failure",
    }
}

fn failure_to_law_ref(failure_class: &str) -> &'static str {
    match failure_class {
        "stability_failure" => "GATE-3.1",
        "locality_failure" => "GATE-3.2",
        "descent_failure" => "GATE-3.3",
        "glue_non_contractible" => "GATE-3.4",
        "adjoint_triple_coherence_failure" => "GATE-3.5",
        _ => "GATE-3.3",
    }
}

fn refinement_obligation_hint(kind: &str) -> Option<&'static str> {
    match kind {
        "adjoint_triangle" => Some("hint:adjoint_triangle"),
        "beck_chevalley_sigma" => Some("hint:beck_chevalley_sigma"),
        "beck_chevalley_pi" => Some("hint:beck_chevalley_pi"),
        "refinement_invariance" => Some("hint:refinement_invariance"),
        _ => None,
    }
}

pub fn discharge_proposal_obligations(
    canonical: &CanonicalProposal,
    obligations: &[ProposalObligation],
) -> ProposalDischarge {
    let binding = canonical.binding.clone();
    let candidate_ref_set: BTreeSet<String> = canonical.candidate_refs.iter().cloned().collect();
    let mut failure_classes_set = BTreeSet::new();
    let mut steps = Vec::new();

    for obligation in obligations {
        let mut failed = obligation.kind == "ext_gap" || obligation.kind == "ext_ambiguous";
        let hint = refinement_obligation_hint(obligation.kind.as_str());
        if let Some(missing_hint) = hint
            && !candidate_ref_set.contains(missing_hint)
        {
            failed = true;
        }

        let mut step = ProposalDischargeStep {
            obligation_id: obligation.obligation_id.clone(),
            kind: obligation.kind.clone(),
            status: if failed { "failed" } else { "passed" }.to_string(),
            mode: "normalized".to_string(),
            binding: binding.clone(),
            failure_class: None,
            law_ref: None,
            missing_hint: None,
        };

        if failed {
            let failure_class = obligation_to_failure(obligation.kind.as_str()).to_string();
            let law_ref = failure_to_law_ref(&failure_class).to_string();
            step.failure_class = Some(failure_class.clone());
            step.law_ref = Some(law_ref);
            if let Some(missing_hint) = hint
                && !candidate_ref_set.contains(missing_hint)
            {
                step.missing_hint = Some(missing_hint.to_string());
            }
            failure_classes_set.insert(failure_class);
        }

        steps.push(step);
    }

    let failure_classes: Vec<String> = failure_classes_set.into_iter().collect();
    ProposalDischarge {
        mode: "normalized".to_string(),
        binding,
        outcome: if failure_classes.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        steps,
        failure_classes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_proposal() -> Value {
        json!({
            "proposalKind": "value",
            "targetCtxRef": "ctx:demo",
            "targetJudgment": {
                "kind": "obj",
                "shape": "ObjNF:site"
            },
            "candidateRefs": ["obj:alpha"],
            "binding": {
                "normalizerId": "normalizer.ci.v1",
                "policyDigest": "pol1_demo"
            }
        })
    }

    #[test]
    fn validate_proposal_accepts_matching_declared_refs() {
        let proposal = base_proposal();
        let validated = validate_proposal_payload(&proposal).expect("proposal should validate");
        let mut with_declared = proposal.clone();
        with_declared
            .as_object_mut()
            .expect("proposal should be object")
            .insert(
                "proposalDigest".to_string(),
                Value::String(validated.digest.clone()),
            );
        with_declared
            .as_object_mut()
            .expect("proposal should be object")
            .insert(
                "proposalKcirRef".to_string(),
                Value::String(validated.kcir_ref.clone()),
            );

        let validated_declared = validate_proposal_payload(&with_declared)
            .expect("proposal with declared refs should validate");
        assert_eq!(validated, validated_declared);
    }

    #[test]
    fn validate_proposal_rejects_digest_mismatch() {
        let mut proposal = base_proposal();
        proposal
            .as_object_mut()
            .expect("proposal should be object")
            .insert(
                "proposalDigest".to_string(),
                Value::String("prop1_deadbeef".to_string()),
            );
        let err = validate_proposal_payload(&proposal).expect_err("proposal should reject");
        assert_eq!(err.failure_class, "proposal_nondeterministic");
    }

    #[test]
    fn compile_and_discharge_ext_gap_rejects() {
        let proposal = json!({
            "proposalKind": "value",
            "targetCtxRef": "ctx:demo",
            "targetJudgment": {
                "kind": "obj",
                "shape": "ObjNF:site"
            },
            "candidateRefs": [],
            "binding": {
                "normalizerId": "normalizer.ci.v1",
                "policyDigest": "pol1_demo"
            }
        });
        let validated = validate_proposal_payload(&proposal).expect("proposal should validate");
        let obligations = compile_proposal_obligations(&validated.canonical);
        let discharge = discharge_proposal_obligations(&validated.canonical, &obligations);
        assert_eq!(discharge.outcome, "rejected");
        assert_eq!(
            discharge.failure_classes,
            vec!["descent_failure".to_string()]
        );
    }
}
