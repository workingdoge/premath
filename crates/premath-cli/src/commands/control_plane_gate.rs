use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const GOVERNANCE_PROFILE_CLAIM_ID: &str = "profile.doctrine_inf_governance.v0";
const DEFAULT_PROMOTION_EVIDENCE_REL_PATH: &str = "artifacts/ciwitness/governance-promotion.json";
const KCIR_MAPPING_CONTRACT_VIOLATION: &str = "kcir_mapping_contract_violation";
const KCIR_MAPPING_LEGACY_AUTHORITY_VIOLATION: &str =
    "kcir_mapping_legacy_encoding_authority_violation";

const REQUIRED_GUARDRAIL_STAGES: [&str; 3] = ["pre_flight", "input", "output"];
const VALID_OBSERVABILITY_MODES: [&str; 3] = ["dashboard", "internal_processor", "disabled"];
const VALID_RISK_TIERS: [&str; 3] = ["low", "moderate", "high"];
const REQUIRED_EVAL_LINEAGE_FIELDS: [&str; 3] = [
    "datasetLineageRef",
    "graderConfigLineageRef",
    "metricThresholdsRef",
];
const PASS_GRADES: [&str; 3] = ["pass", "accepted", "ok"];

fn emit_error(failure_class: &str, message: impl Into<String>) -> ! {
    eprintln!("{failure_class}: {}", message.into());
    std::process::exit(2);
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GovernanceGateInput {
    #[serde(default)]
    repo_root: Option<String>,
    #[serde(default)]
    promotion_required: bool,
    #[serde(default)]
    promotion_evidence_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GovernanceGateOutput {
    action: &'static str,
    failure_classes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KcirMappingGateInput {
    #[serde(default)]
    repo_root: Option<String>,
    scope: String,
    #[serde(default)]
    instruction_path: Option<String>,
    #[serde(default)]
    instruction_id: Option<String>,
    #[serde(default)]
    strict: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct KcirMappingGateOutput {
    action: &'static str,
    scope: String,
    profile_id: String,
    declared_rows: Vec<String>,
    checked_rows: Vec<String>,
    failure_classes: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum MappingScope {
    Required,
    Instruction,
}

impl MappingScope {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "required" => Some(Self::Required),
            "instruction" => Some(Self::Instruction),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Instruction => "instruction",
        }
    }
}

#[derive(Debug, Clone)]
struct MappingContext {
    obligation_digest: Option<String>,
    coherence_normalizer_id: Option<String>,
    coherence_policy_digest: Option<String>,
    doctrine_site_digest: Option<String>,
    doctrine_operation_id: Option<String>,
}

#[derive(Debug, Clone)]
struct MappingSurface {
    profile_id: String,
    mapping_table: BTreeMap<String, Map<String, Value>>,
    legacy_authority_mode: Option<String>,
    legacy_failure_class: String,
    runtime_route_bindings: Map<String, Value>,
}

fn non_empty_string(value: Option<&Value>) -> Option<String> {
    let raw = value?.as_str()?.trim();
    if raw.is_empty() {
        return None;
    }
    Some(raw.to_string())
}

fn ordered_unique(values: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in values {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn sha256_file(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(sha256_bytes(&bytes))
}

fn load_json_object(path: &Path) -> Option<Map<String, Value>> {
    let bytes = fs::read(path).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    value.as_object().cloned()
}

fn resolve_repo_root(value: Option<&str>) -> PathBuf {
    let raw = value.unwrap_or(".");
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return PathBuf::from(".");
    }
    PathBuf::from(trimmed)
}

fn resolve_path(root: &Path, value: Option<&str>, default_rel: &str) -> PathBuf {
    let raw = value.unwrap_or(default_rel).trim();
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        return candidate;
    }
    root.join(candidate)
}

fn load_profile_overlay_claims(repo_root: &Path) -> Option<BTreeSet<String>> {
    let registry_path = repo_root.join("specs/premath/draft/CAPABILITY-REGISTRY.json");
    let payload = load_json_object(&registry_path)?;
    let claims_raw = payload.get("profileOverlayClaims")?.as_array()?;
    let mut claims = BTreeSet::new();
    for value in claims_raw {
        if let Some(claim) = non_empty_string(Some(value)) {
            claims.insert(claim);
        }
    }
    Some(claims)
}

fn evaluate_governance_profile(profile: &Map<String, Value>, failures: &mut BTreeSet<String>) {
    let claim_id = non_empty_string(profile.get("claimId"));
    let claimed = profile.get("claimed").and_then(Value::as_bool);
    match claim_id.as_deref() {
        Some(GOVERNANCE_PROFILE_CLAIM_ID) => {}
        _ => {
            failures.insert("governance.eval_lineage_missing".to_string());
        }
    }
    if claimed != Some(true) {
        failures.insert("governance.eval_lineage_missing".to_string());
    }

    let policy = profile.get("policyProvenance").and_then(Value::as_object);
    if let Some(policy) = policy {
        let pinned = policy.get("pinned").and_then(Value::as_bool) == Some(true);
        let package_ref = non_empty_string(policy.get("packageRef")).is_some();
        let expected_digest = non_empty_string(policy.get("expectedDigest"));
        let bound_digest = non_empty_string(policy.get("boundDigest"));
        if !pinned || !package_ref || expected_digest.is_none() || bound_digest.is_none() {
            failures.insert("governance.policy_package_unpinned".to_string());
        }
        if let (Some(expected), Some(bound)) = (expected_digest, bound_digest)
            && expected != bound
        {
            failures.insert("governance.policy_package_mismatch".to_string());
        }
    } else {
        failures.insert("governance.policy_package_unpinned".to_string());
    }

    let guardrail_stages = profile.get("guardrailStages").and_then(Value::as_array);
    if let Some(stages_raw) = guardrail_stages {
        let mut stages = Vec::new();
        for value in stages_raw {
            if let Some(stage) = non_empty_string(Some(value)) {
                stages.push(stage);
            } else {
                failures.insert("governance.guardrail_stage_missing".to_string());
                break;
            }
        }
        if failures.contains("governance.guardrail_stage_missing") {
            // no-op
        } else if REQUIRED_GUARDRAIL_STAGES
            .iter()
            .any(|required| !stages.iter().any(|stage| stage == required))
        {
            failures.insert("governance.guardrail_stage_missing".to_string());
        } else {
            let expected = REQUIRED_GUARDRAIL_STAGES
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>();
            if stages != expected {
                failures.insert("governance.guardrail_stage_order_invalid".to_string());
            }
        }
    } else {
        failures.insert("governance.guardrail_stage_missing".to_string());
    }

    let eval_gate = profile.get("evalGate").and_then(Value::as_object);
    match eval_gate
        .and_then(|gate| gate.get("passed"))
        .and_then(Value::as_bool)
    {
        Some(true) => {}
        _ => {
            failures.insert("governance.eval_gate_unmet".to_string());
        }
    }

    let eval_evidence = profile.get("evalEvidence").and_then(Value::as_object);
    if let Some(eval_evidence) = eval_evidence {
        for field in REQUIRED_EVAL_LINEAGE_FIELDS {
            if non_empty_string(eval_evidence.get(field)).is_none() {
                failures.insert("governance.eval_lineage_missing".to_string());
            }
        }
    } else {
        failures.insert("governance.eval_lineage_missing".to_string());
    }

    let observability_mode = non_empty_string(profile.get("observabilityMode"));
    if !observability_mode
        .as_deref()
        .map(|mode| VALID_OBSERVABILITY_MODES.contains(&mode))
        .unwrap_or(false)
    {
        failures.insert("governance.trace_mode_violation".to_string());
    }

    let risk_tier = profile.get("riskTier").and_then(Value::as_object);
    if let Some(risk_tier) = risk_tier {
        let tier = non_empty_string(risk_tier.get("tier"));
        let bound = risk_tier
            .get("controlProfileBound")
            .and_then(Value::as_bool)
            == Some(true);
        if !tier
            .as_deref()
            .map(|value| VALID_RISK_TIERS.contains(&value))
            .unwrap_or(false)
            || !bound
        {
            failures.insert("governance.risk_tier_profile_missing".to_string());
        }
    } else {
        failures.insert("governance.risk_tier_profile_missing".to_string());
    }

    let self_evolution = profile.get("selfEvolution").and_then(Value::as_object);
    if let Some(self_evolution) = self_evolution {
        let max_attempts = self_evolution.get("maxAttempts").and_then(Value::as_i64);
        if !max_attempts.map(|value| value >= 1).unwrap_or(false) {
            failures.insert("governance.self_evolution_retry_missing".to_string());
        }
        if non_empty_string(self_evolution.get("terminalEscalation")).is_none() {
            failures.insert("governance.self_evolution_escalation_missing".to_string());
        }
        if non_empty_string(self_evolution.get("rollbackRef")).is_none() {
            failures.insert("governance.self_evolution_rollback_missing".to_string());
        }
    } else {
        failures.insert("governance.self_evolution_retry_missing".to_string());
        failures.insert("governance.self_evolution_escalation_missing".to_string());
        failures.insert("governance.self_evolution_rollback_missing".to_string());
    }
}

fn evaluate_workflow_trace(trace_payload: Option<&Value>, failures: &mut BTreeSet<String>) {
    let Some(trace) = trace_payload.and_then(Value::as_object) else {
        failures.insert("governance.eval_lineage_missing".to_string());
        return;
    };

    if non_empty_string(trace.get("traceRef")).is_none() {
        failures.insert("governance.eval_lineage_missing".to_string());
    }

    let score = trace.get("score").and_then(Value::as_f64);
    let threshold = trace.get("threshold").and_then(Value::as_f64);
    match (score, threshold) {
        (Some(score), Some(threshold)) => {
            if score < threshold {
                failures.insert("governance.eval_gate_unmet".to_string());
            }
        }
        _ => {
            failures.insert("governance.eval_lineage_missing".to_string());
        }
    }

    if let Some(grade) = non_empty_string(trace.get("grade"))
        && !PASS_GRADES.contains(&grade.to_ascii_lowercase().as_str())
    {
        failures.insert("governance.eval_gate_unmet".to_string());
    }
}

fn evaluate_adversarial_gate(payload: Option<&Value>, failures: &mut BTreeSet<String>) {
    let Some(payload) = payload.and_then(Value::as_object) else {
        failures.insert("governance.eval_lineage_missing".to_string());
        return;
    };

    match payload.get("passed").and_then(Value::as_bool) {
        Some(true) => {}
        Some(false) => {
            failures.insert("governance.eval_gate_unmet".to_string());
        }
        None => {
            failures.insert("governance.eval_lineage_missing".to_string());
        }
    }

    if non_empty_string(payload.get("reportRef")).is_none() {
        failures.insert("governance.eval_lineage_missing".to_string());
    }
}

fn governance_failure_classes(input: &GovernanceGateInput) -> Vec<String> {
    let repo_root = resolve_repo_root(input.repo_root.as_deref());
    let claims = load_profile_overlay_claims(&repo_root);
    let required = input.promotion_required;

    let Some(claims) = claims else {
        if required {
            return vec!["governance.eval_lineage_missing".to_string()];
        }
        return Vec::new();
    };

    if !claims.contains(GOVERNANCE_PROFILE_CLAIM_ID) {
        return Vec::new();
    }

    let evidence_path = resolve_path(
        &repo_root,
        input.promotion_evidence_path.as_deref(),
        DEFAULT_PROMOTION_EVIDENCE_REL_PATH,
    );
    let payload = load_json_object(&evidence_path);
    let Some(payload) = payload else {
        if required {
            return vec!["governance.eval_lineage_missing".to_string()];
        }
        return Vec::new();
    };

    let promotion_intent = payload.get("promotionIntent").and_then(Value::as_bool);
    let Some(promotion_intent) = promotion_intent else {
        return vec!["governance.eval_lineage_missing".to_string()];
    };
    if !promotion_intent {
        return Vec::new();
    }

    let mut failures = BTreeSet::new();
    match payload.get("governanceProfile").and_then(Value::as_object) {
        Some(profile) => evaluate_governance_profile(profile, &mut failures),
        None => {
            failures.insert("governance.eval_lineage_missing".to_string());
        }
    }
    evaluate_workflow_trace(payload.get("workflowTrace"), &mut failures);
    evaluate_adversarial_gate(payload.get("adversarialGate"), &mut failures);
    failures.into_iter().collect()
}

fn expected_declared_rows() -> BTreeSet<String> {
    [
        "instructionEnvelope",
        "proposalPayload",
        "coherenceObligations",
        "coherenceCheckPayload",
        "doctrineRouteBinding",
        "requiredDecisionInput",
    ]
    .iter()
    .map(|item| item.to_string())
    .collect()
}

fn required_instruction_rows(include_proposal: bool) -> Vec<String> {
    let mut rows = vec![
        "instructionEnvelope".to_string(),
        "coherenceObligations".to_string(),
        "doctrineRouteBinding".to_string(),
    ];
    if include_proposal {
        rows.push("proposalPayload".to_string());
    }
    rows
}

fn required_required_rows() -> Vec<String> {
    vec![
        "coherenceCheckPayload".to_string(),
        "requiredDecisionInput".to_string(),
        "coherenceObligations".to_string(),
        "doctrineRouteBinding".to_string(),
    ]
}

fn load_mapping_surface(repo_root: &Path) -> Option<MappingSurface> {
    let contract_path = repo_root.join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");
    let contract = load_json_object(&contract_path)?;
    let control_plane_mappings = contract
        .get("controlPlaneKcirMappings")?
        .as_object()?
        .clone();

    let profile_id = non_empty_string(control_plane_mappings.get("profileId"))?;

    let mapping_table_raw = control_plane_mappings
        .get("mappingTable")?
        .as_object()?
        .clone();
    let mut mapping_table: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for (row_id, value) in mapping_table_raw {
        let row = value.as_object()?.clone();
        mapping_table.insert(row_id, row);
    }

    let legacy_non_kcir = control_plane_mappings
        .get("compatibilityPolicy")
        .and_then(Value::as_object)
        .and_then(|value| value.get("legacyNonKcirEncodings"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let legacy_authority_mode = non_empty_string(legacy_non_kcir.get("authorityMode"));
    let legacy_failure_class = non_empty_string(legacy_non_kcir.get("failureClass"))
        .unwrap_or_else(|| KCIR_MAPPING_LEGACY_AUTHORITY_VIOLATION.to_string());

    let runtime_route_bindings = contract
        .get("runtimeRouteBindings")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    Some(MappingSurface {
        profile_id,
        mapping_table,
        legacy_authority_mode,
        legacy_failure_class,
        runtime_route_bindings,
    })
}

fn load_mapping_context(repo_root: &Path, surface: &MappingSurface) -> MappingContext {
    let coherence_path = repo_root.join("specs/premath/draft/COHERENCE-CONTRACT.json");
    let coherence = load_json_object(&coherence_path);
    let doctrine_site_path = repo_root.join("specs/premath/draft/DOCTRINE-SITE.json");

    let mut coherence_normalizer_id = None;
    let mut coherence_policy_digest = None;
    let mut obligation_digest = None;

    if let Some(coherence) = coherence {
        if let Some(binding) = coherence.get("binding").and_then(Value::as_object) {
            coherence_normalizer_id = non_empty_string(binding.get("normalizerId"));
            coherence_policy_digest = non_empty_string(binding.get("policyDigest"));
        }
        if let Some(obligations) = coherence.get("obligations").and_then(Value::as_array) {
            let mut obligation_ids = BTreeSet::new();
            for row in obligations {
                if let Some(row_obj) = row.as_object()
                    && let Some(obligation_id) = non_empty_string(row_obj.get("id"))
                {
                    obligation_ids.insert(obligation_id);
                }
            }
            if !obligation_ids.is_empty() {
                let ordered = obligation_ids.into_iter().collect::<Vec<_>>();
                let encoded = serde_json::to_vec(&ordered).unwrap_or_else(|_| b"[]".to_vec());
                obligation_digest = Some(sha256_bytes(&encoded));
            }
        }
    }

    let doctrine_site_digest = sha256_file(&doctrine_site_path);
    let operation_ids = surface
        .runtime_route_bindings
        .values()
        .filter_map(Value::as_object)
        .filter_map(|value| non_empty_string(value.get("operationId")))
        .collect::<BTreeSet<_>>();
    let doctrine_operation_id = if operation_ids.is_empty() {
        None
    } else {
        Some(operation_ids.into_iter().collect::<Vec<_>>().join(","))
    };

    MappingContext {
        obligation_digest,
        coherence_normalizer_id,
        coherence_policy_digest,
        doctrine_site_digest,
        doctrine_operation_id,
    }
}

fn mapping_row_identities(
    row_id: &str,
    row: &Map<String, Value>,
    context: &MappingContext,
    witness: Option<&Map<String, Value>>,
    proposal_ingest: Option<&Map<String, Value>>,
    decision: Option<&Map<String, Value>>,
    decision_digest: Option<&str>,
) -> BTreeMap<String, Option<String>> {
    let identity_fields = row
        .get("identityFields")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| non_empty_string(Some(item)))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut values = BTreeMap::new();
    for field in identity_fields {
        let value = match row_id {
            "proposalPayload" => {
                let proposal_value =
                    proposal_ingest.and_then(|payload| non_empty_string(payload.get(&field)));
                proposal_value
                    .or_else(|| witness.and_then(|payload| non_empty_string(payload.get(&field))))
            }
            "requiredDecisionInput" => {
                if field == "requiredDigest" {
                    decision.and_then(|payload| non_empty_string(payload.get("witnessSha256")))
                } else if field == "decisionDigest" {
                    decision_digest.map(|value| value.to_string())
                } else {
                    decision.and_then(|payload| non_empty_string(payload.get(&field)))
                }
            }
            "coherenceObligations" => {
                if field == "obligationDigest" {
                    context.obligation_digest.clone()
                } else if field == "normalizerId" {
                    context.coherence_normalizer_id.clone().or_else(|| {
                        witness.and_then(|payload| non_empty_string(payload.get("normalizerId")))
                    })
                } else if field == "policyDigest" {
                    context.coherence_policy_digest.clone().or_else(|| {
                        witness.and_then(|payload| non_empty_string(payload.get("policyDigest")))
                    })
                } else {
                    witness.and_then(|payload| non_empty_string(payload.get(&field)))
                }
            }
            "doctrineRouteBinding" => {
                if field == "operationId" {
                    context.doctrine_operation_id.clone()
                } else if field == "siteDigest" {
                    context.doctrine_site_digest.clone()
                } else if field == "policyDigest" {
                    context.coherence_policy_digest.clone().or_else(|| {
                        witness.and_then(|payload| non_empty_string(payload.get("policyDigest")))
                    })
                } else {
                    witness.and_then(|payload| non_empty_string(payload.get(&field)))
                }
            }
            _ => witness.and_then(|payload| non_empty_string(payload.get(&field))),
        };
        values.insert(field, value);
    }

    values
}

fn kcir_mapping_report(input: &KcirMappingGateInput) -> KcirMappingGateOutput {
    let repo_root = resolve_repo_root(input.repo_root.as_deref());
    let scope = MappingScope::parse(&input.scope).unwrap_or(MappingScope::Required);
    let mut failures = Vec::new();

    let surface = load_mapping_surface(&repo_root);
    let Some(surface) = surface else {
        return KcirMappingGateOutput {
            action: "kcir-mapping-check",
            scope: scope.as_str().to_string(),
            profile_id: String::new(),
            declared_rows: Vec::new(),
            checked_rows: Vec::new(),
            failure_classes: vec![KCIR_MAPPING_CONTRACT_VIOLATION.to_string()],
        };
    };

    let context = load_mapping_context(&repo_root, &surface);
    let declared_rows = surface.mapping_table.keys().cloned().collect::<Vec<_>>();

    let strict = input.strict;
    let expected_rows = expected_declared_rows();

    let checked_rows = match scope {
        MappingScope::Instruction => {
            let Some(instruction_path_raw) = input.instruction_path.as_deref() else {
                return KcirMappingGateOutput {
                    action: "kcir-mapping-check",
                    scope: scope.as_str().to_string(),
                    profile_id: surface.profile_id.clone(),
                    declared_rows,
                    checked_rows: required_instruction_rows(true),
                    failure_classes: vec![KCIR_MAPPING_CONTRACT_VIOLATION.to_string()],
                };
            };
            let instruction_path = resolve_path(&repo_root, Some(instruction_path_raw), "");
            let envelope = load_json_object(&instruction_path);
            let instruction_id = input
                .instruction_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .or_else(|| {
                    instruction_path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .map(str::to_string)
                });
            let witness = instruction_id.as_ref().and_then(|instruction_id| {
                load_json_object(
                    &repo_root
                        .join("artifacts")
                        .join("ciwitness")
                        .join(format!("{instruction_id}.json")),
                )
            });
            let proposal_ingest = witness
                .as_ref()
                .and_then(|payload| payload.get("proposalIngest"))
                .and_then(Value::as_object);
            let include_proposal = envelope.as_ref().is_some_and(|payload| {
                payload.get("proposal").and_then(Value::as_object).is_some()
                    || payload
                        .get("llmProposal")
                        .and_then(Value::as_object)
                        .is_some()
            }) || witness
                .as_ref()
                .and_then(|payload| payload.get("proposalIngest"))
                .and_then(Value::as_object)
                .is_some();

            let checked_rows = required_instruction_rows(include_proposal);
            if strict {
                if declared_rows.iter().cloned().collect::<BTreeSet<_>>() != expected_rows {
                    failures.push(KCIR_MAPPING_CONTRACT_VIOLATION.to_string());
                }
                if witness.is_none() {
                    failures.push(KCIR_MAPPING_CONTRACT_VIOLATION.to_string());
                }
                for row_id in &checked_rows {
                    let Some(row) = surface.mapping_table.get(row_id) else {
                        failures.push(KCIR_MAPPING_CONTRACT_VIOLATION.to_string());
                        continue;
                    };
                    let identities = mapping_row_identities(
                        row_id,
                        row,
                        &context,
                        witness.as_ref(),
                        proposal_ingest,
                        None,
                        None,
                    );
                    if identities.is_empty() {
                        failures.push(KCIR_MAPPING_CONTRACT_VIOLATION.to_string());
                        continue;
                    }
                    if identities.values().any(|value| value.is_none()) {
                        failures.push(KCIR_MAPPING_CONTRACT_VIOLATION.to_string());
                    }
                }

                let legacy_forbidden =
                    surface.legacy_authority_mode.as_deref() == Some("forbidden");
                let has_legacy_alias = envelope.as_ref().is_some_and(|payload| {
                    payload.get("proposal").is_none()
                        && payload
                            .get("llmProposal")
                            .and_then(Value::as_object)
                            .is_some()
                });
                if legacy_forbidden && has_legacy_alias {
                    failures.push(surface.legacy_failure_class.clone());
                }
            }
            checked_rows
        }
        MappingScope::Required => {
            let witness_path = repo_root
                .join("artifacts")
                .join("ciwitness")
                .join("latest-required.json");
            let decision_path = repo_root
                .join("artifacts")
                .join("ciwitness")
                .join("latest-decision.json");
            let witness = load_json_object(&witness_path);
            let decision = load_json_object(&decision_path);
            let decision_digest = sha256_file(&decision_path);
            let checked_rows = required_required_rows();

            if strict {
                if declared_rows.iter().cloned().collect::<BTreeSet<_>>() != expected_rows {
                    failures.push(KCIR_MAPPING_CONTRACT_VIOLATION.to_string());
                }
                if witness.is_none() {
                    failures.push(KCIR_MAPPING_CONTRACT_VIOLATION.to_string());
                }
                if decision.is_none() {
                    failures.push(KCIR_MAPPING_CONTRACT_VIOLATION.to_string());
                }
                for row_id in &checked_rows {
                    let Some(row) = surface.mapping_table.get(row_id) else {
                        failures.push(KCIR_MAPPING_CONTRACT_VIOLATION.to_string());
                        continue;
                    };
                    let identities = mapping_row_identities(
                        row_id,
                        row,
                        &context,
                        witness.as_ref(),
                        None,
                        decision.as_ref(),
                        decision_digest.as_deref(),
                    );
                    if identities.is_empty() {
                        failures.push(KCIR_MAPPING_CONTRACT_VIOLATION.to_string());
                        continue;
                    }
                    if identities.values().any(|value| value.is_none()) {
                        failures.push(KCIR_MAPPING_CONTRACT_VIOLATION.to_string());
                    }
                }

                if surface.legacy_authority_mode.as_deref() == Some("forbidden") {
                    let mut typed_core_digest = witness.as_ref().and_then(|payload| {
                        non_empty_string(payload.get("typedCoreProjectionDigest"))
                    });
                    let mut authority_alias_digest = witness.as_ref().and_then(|payload| {
                        non_empty_string(payload.get("authorityPayloadDigest"))
                    });

                    if typed_core_digest.is_none() {
                        typed_core_digest = decision.as_ref().and_then(|payload| {
                            non_empty_string(payload.get("typedCoreProjectionDigest"))
                        });
                    }
                    if authority_alias_digest.is_none() {
                        authority_alias_digest = decision.as_ref().and_then(|payload| {
                            non_empty_string(payload.get("authorityPayloadDigest"))
                        });
                    }

                    if authority_alias_digest.is_some() && typed_core_digest.is_none() {
                        failures.push(surface.legacy_failure_class.clone());
                    }
                }
            }
            checked_rows
        }
    };

    KcirMappingGateOutput {
        action: "kcir-mapping-check",
        scope: scope.as_str().to_string(),
        profile_id: surface.profile_id,
        declared_rows,
        checked_rows,
        failure_classes: ordered_unique(&failures),
    }
}

pub fn run_governance(input: String, json_output: bool) {
    let input_path = PathBuf::from(&input);
    let bytes = fs::read(&input_path).unwrap_or_else(|err| {
        emit_error(
            "governance_gate_invalid",
            format!(
                "failed to read governance gate input {}: {err}",
                input_path.display()
            ),
        );
    });

    let request: GovernanceGateInput = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        emit_error(
            "governance_gate_invalid",
            format!(
                "failed to parse governance gate input json {}: {err}",
                input_path.display()
            ),
        );
    });

    let output = GovernanceGateOutput {
        action: "governance-promotion-check",
        failure_classes: governance_failure_classes(&request),
    };

    if json_output {
        let rendered = serde_json::to_string_pretty(&output).unwrap_or_else(|err| {
            emit_error(
                "governance_gate_invalid",
                format!("failed to render governance gate output: {err}"),
            )
        });
        println!("{rendered}");
        return;
    }

    if output.failure_classes.is_empty() {
        println!("premath governance-promotion-check: ACCEPT");
        return;
    }

    println!(
        "premath governance-promotion-check: REJECT (failureClasses={})",
        output.failure_classes.len()
    );
    for class in output.failure_classes {
        println!("  - {class}");
    }
}

pub fn run_kcir_mapping(input: String, json_output: bool) {
    let input_path = PathBuf::from(&input);
    let bytes = fs::read(&input_path).unwrap_or_else(|err| {
        emit_error(
            "kcir_mapping_gate_invalid",
            format!(
                "failed to read kcir mapping gate input {}: {err}",
                input_path.display()
            ),
        );
    });

    let request: KcirMappingGateInput = serde_json::from_slice(&bytes).unwrap_or_else(|err| {
        emit_error(
            "kcir_mapping_gate_invalid",
            format!(
                "failed to parse kcir mapping gate input json {}: {err}",
                input_path.display()
            ),
        );
    });

    if MappingScope::parse(&request.scope).is_none() {
        emit_error(
            "kcir_mapping_gate_invalid",
            format!(
                "scope must be `required` or `instruction` (got {})",
                request.scope
            ),
        );
    }

    if request.scope == "instruction" {
        let instruction_path = request
            .instruction_path
            .as_deref()
            .map(str::trim)
            .unwrap_or("");
        if instruction_path.is_empty() {
            emit_error(
                "kcir_mapping_gate_invalid",
                "instruction scope requires instructionPath",
            );
        }
    }

    let output = kcir_mapping_report(&request);

    if json_output {
        let rendered = serde_json::to_string_pretty(&output).unwrap_or_else(|err| {
            emit_error(
                "kcir_mapping_gate_invalid",
                format!("failed to render kcir mapping gate output: {err}"),
            )
        });
        println!("{rendered}");
        return;
    }

    if output.failure_classes.is_empty() {
        println!(
            "premath kcir-mapping-check: ACCEPT (scope={}, profile={})",
            output.scope, output.profile_id
        );
        return;
    }

    println!(
        "premath kcir-mapping-check: REJECT (scope={}, failures={})",
        output.scope,
        output.failure_classes.len()
    );
    for class in output.failure_classes {
        println!("  - {class}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root should resolve")
    }

    fn temp_repo(prefix: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "premath-cli-control-plane-gate-{prefix}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("specs/premath/draft")).expect("temp repo specs path");
        fs::create_dir_all(root.join("artifacts/ciwitness")).expect("temp repo witness path");

        let source_root = repo_root();
        for rel in [
            "specs/premath/draft/CAPABILITY-REGISTRY.json",
            "specs/premath/draft/CONTROL-PLANE-CONTRACT.json",
            "specs/premath/draft/COHERENCE-CONTRACT.json",
            "specs/premath/draft/DOCTRINE-SITE.json",
        ] {
            let src = source_root.join(rel);
            let dst = root.join(rel);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent).expect("dst parent should exist");
            }
            fs::copy(&src, &dst).unwrap_or_else(|err| {
                panic!(
                    "failed to copy fixture {} -> {}: {err}",
                    src.display(),
                    dst.display()
                )
            });
        }

        root
    }

    #[test]
    fn governance_gate_is_claim_gated() {
        let root = temp_repo("governance-claim-gated");
        let input = GovernanceGateInput {
            repo_root: Some(root.display().to_string()),
            promotion_required: false,
            promotion_evidence_path: None,
        };
        let failures = governance_failure_classes(&input);
        assert!(failures.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn governance_gate_detects_policy_digest_mismatch() {
        let root = temp_repo("governance-mismatch");
        let evidence = root
            .join("artifacts")
            .join("ciwitness")
            .join("governance-promotion.json");
        let payload = json!({
            "promotionIntent": true,
            "governanceProfile": {
                "claimId": GOVERNANCE_PROFILE_CLAIM_ID,
                "claimed": true,
                "policyProvenance": {
                    "pinned": true,
                    "packageRef": "policy/governance/v1",
                    "expectedDigest": "sha256:a",
                    "boundDigest": "sha256:b"
                },
                "guardrailStages": ["pre_flight", "input", "output"],
                "evalGate": {"passed": true},
                "evalEvidence": {
                    "datasetLineageRef": "dataset:v1",
                    "graderConfigLineageRef": "grader:v1",
                    "metricThresholdsRef": "metrics:v1"
                },
                "observabilityMode": "dashboard",
                "riskTier": {"tier": "moderate", "controlProfileBound": true},
                "selfEvolution": {
                    "maxAttempts": 2,
                    "terminalEscalation": "mark_blocked",
                    "rollbackRef": "rollback:v1"
                }
            },
            "workflowTrace": {
                "traceRef": "trace:ok",
                "score": 0.95,
                "threshold": 0.9,
                "grade": "pass"
            },
            "adversarialGate": {
                "passed": true,
                "reportRef": "adv:ok"
            }
        });
        fs::write(
            &evidence,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&payload).expect("json should render")
            ),
        )
        .expect("evidence should write");

        let mut capability_registry =
            load_json_object(&root.join("specs/premath/draft/CAPABILITY-REGISTRY.json"))
                .expect("capability registry should parse");
        capability_registry.insert(
            "profileOverlayClaims".to_string(),
            json!([GOVERNANCE_PROFILE_CLAIM_ID]),
        );
        fs::write(
            root.join("specs/premath/draft/CAPABILITY-REGISTRY.json"),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&Value::Object(capability_registry))
                    .expect("registry should render")
            ),
        )
        .expect("capability registry should write");

        let input = GovernanceGateInput {
            repo_root: Some(root.display().to_string()),
            promotion_required: true,
            promotion_evidence_path: None,
        };
        let failures = governance_failure_classes(&input);
        assert!(failures.contains(&"governance.policy_package_mismatch".to_string()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn kcir_mapping_required_detects_legacy_alias_violation() {
        let root = temp_repo("kcir-required-legacy");
        let ciwitness = root.join("artifacts/ciwitness");
        fs::write(
            ciwitness.join("latest-required.json"),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&json!({
                    "projectionDigest": "proj_demo",
                    "normalizerId": "normalizer.ci.required.v1",
                    "policyDigest": "ci-topos-v0",
                    "authorityPayloadDigest": "proj_demo"
                }))
                .expect("witness should render")
            ),
        )
        .expect("required witness should write");
        fs::write(
            ciwitness.join("latest-decision.json"),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&json!({
                    "decision": "accept",
                    "reasonClass": "verified_accept",
                    "policyDigest": "ci-topos-v0",
                    "authorityPayloadDigest": "proj_demo",
                    "witnessSha256": "witness_demo"
                }))
                .expect("decision should render")
            ),
        )
        .expect("required decision should write");

        let report = kcir_mapping_report(&KcirMappingGateInput {
            repo_root: Some(root.display().to_string()),
            scope: "required".to_string(),
            instruction_path: None,
            instruction_id: None,
            strict: true,
        });
        assert!(
            report
                .failure_classes
                .contains(&KCIR_MAPPING_LEGACY_AUTHORITY_VIOLATION.to_string())
        );
        let _ = fs::remove_dir_all(root);
    }
}
