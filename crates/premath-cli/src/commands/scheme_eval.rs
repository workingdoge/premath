use crate::support::{DEFAULT_ISSUES_PATH, read_json_file_or_exit};
use chrono::{SecondsFormat, Utc};
use premath_bd::MemoryStore;
use premath_kernel::{SiteResolveRequest, SiteResolveWitness, resolve_site_request};
use premath_surreal::{
    HARNESS_TRAJECTORY_KIND, HARNESS_TRAJECTORY_SCHEMA, HarnessTrajectoryRow, QueryCache,
    append_trajectory_row,
};
use premath_transport::transport_dispatch_json;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const HOST_EFFECT_SCHEMA: &str = "premath.host_effect.v0";
const SCHEME_EVAL_RESULT_KIND: &str = "premath.scheme_eval.result.v0";
#[cfg(feature = "rhai-frontend")]
const RHAI_EVAL_RESULT_KIND: &str = "premath.rhai_eval.result.v0";
const SCHEME_EVAL_PROFILE: &str = "steel.default.v0";
#[cfg(feature = "rhai-frontend")]
const RHAI_EVAL_PROFILE: &str = "rhai.default.v0";
const SCHEME_EVAL_ACTION_PREFIX: &str = "scheme_eval.";
#[cfg(feature = "rhai-frontend")]
const RHAI_EVAL_ACTION_PREFIX: &str = "rhai_eval.";
const SCHEME_EVAL_REQUEST_KIND: &str = "premath.scheme_eval.request.v0";
const RHAI_EVAL_REQUEST_KIND: &str = "premath.rhai_eval.request.v0";
const CHANGE_MORPHISMS_BASE_CAPABILITY: &str = "capabilities.change_morphisms";
const CHANGE_MORPHISMS_ALL_CAPABILITY: &str = "capabilities.change_morphisms.all";
const FAILURE_INVALID_PROGRAM: &str = "scheme_eval.invalid_program";
const FAILURE_CALL_BUDGET_EXCEEDED: &str = "scheme_eval.call_budget_exceeded";
const FAILURE_EFFECT_DENIED: &str = "scheme_eval.effect_denied";
const FAILURE_ACTION_UNALLOWLISTED: &str = "scheme_eval.host_action_unallowlisted";
const FAILURE_ACTION_UNIMPLEMENTED: &str = "scheme_eval.host_action_unimplemented";
const FAILURE_EXECUTION_ERROR: &str = "scheme_eval.host_action_execution_error";
const FAILURE_MUTATION_USE_EVIDENCE_MISSING: &str = "mutation.use_evidence_missing";
const FAILURE_MUTATION_CAPABILITY_CLAIM_MISSING: &str = "mutation.capability_claim_missing";
const FAILURE_MCP_TRANSPORT_REQUIRED: &str = "control_plane_host_action_mcp_transport_required";
const FAILURE_HOST_ACTION_BINDING_MISMATCH: &str = "control_plane_host_action_binding_mismatch";
const FAILURE_HOST_ACTION_CONTRACT_UNBOUND: &str = "control_plane_host_action_contract_unbound";

const DOCTRINE_SITE_INPUT_JSON: &str =
    include_str!("../../../../specs/premath/draft/DOCTRINE-SITE-INPUT.json");
const DOCTRINE_SITE_JSON: &str = include_str!("../../../../specs/premath/draft/DOCTRINE-SITE.json");
const DOCTRINE_OP_REGISTRY_JSON: &str =
    include_str!("../../../../specs/premath/draft/DOCTRINE-OP-REGISTRY.json");
const CAPABILITY_REGISTRY_JSON: &str =
    include_str!("../../../../specs/premath/draft/CAPABILITY-REGISTRY.json");

const HOST_ACTIONS_SUPPORTED: &[&str] = &[
    "issue.ready",
    "issue.list",
    "issue.blocked",
    "issue.check",
    "dep.diagnostics",
    "issue.claim",
    "issue.claim_next",
    "issue.lease_renew",
    "issue.lease_release",
    "fiber.spawn",
    "fiber.join",
    "fiber.cancel",
];

const HOST_ACTIONS_MUTATION: &[&str] = &[
    "issue.claim",
    "issue.claim_next",
    "issue.lease_renew",
    "issue.lease_release",
];

const HOST_ACTIONS_TRANSPORT_DISPATCH: &[&str] = &[
    "issue.claim",
    "issue.claim_next",
    "issue.lease_renew",
    "issue.lease_release",
    "fiber.spawn",
    "fiber.join",
    "fiber.cancel",
];

#[derive(Debug, Clone, Copy)]
pub struct FrontendConfig {
    pub command_name: &'static str,
    pub result_kind: &'static str,
    pub profile: &'static str,
    pub action_prefix: &'static str,
    pub request_kind: &'static str,
}

pub const FRONTEND_SCHEME: FrontendConfig = FrontendConfig {
    command_name: "scheme-eval",
    result_kind: SCHEME_EVAL_RESULT_KIND,
    profile: SCHEME_EVAL_PROFILE,
    action_prefix: SCHEME_EVAL_ACTION_PREFIX,
    request_kind: SCHEME_EVAL_REQUEST_KIND,
};

#[cfg(feature = "rhai-frontend")]
pub const FRONTEND_RHAI: FrontendConfig = FrontendConfig {
    command_name: "rhai-eval",
    result_kind: RHAI_EVAL_RESULT_KIND,
    profile: RHAI_EVAL_PROFILE,
    action_prefix: RHAI_EVAL_ACTION_PREFIX,
    request_kind: RHAI_EVAL_REQUEST_KIND,
};

#[derive(Debug, Clone)]
pub struct Args {
    pub program: String,
    pub control_plane_contract: String,
    pub trajectory_path: String,
    pub step_prefix: String,
    pub max_calls: usize,
    pub issue_id: Option<String>,
    pub policy_digest: Option<String>,
    pub instruction_ref: Option<String>,
    pub capability_claims: Vec<String>,
    pub json: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemeEvalProgram {
    #[serde(default)]
    schema: Option<u64>,
    #[serde(default)]
    program_kind: Option<String>,
    #[serde(default)]
    issue_id: Option<String>,
    #[serde(default)]
    policy_digest: Option<String>,
    #[serde(default)]
    instruction_ref: Option<String>,
    #[serde(default)]
    capability_claims: Vec<String>,
    #[serde(default)]
    calls: Vec<SchemeEvalCall>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemeEvalCall {
    #[serde(default, alias = "id")]
    call_id: Option<String>,
    action: String,
    #[serde(default)]
    args: Value,
    #[serde(default)]
    issue_id: Option<String>,
    #[serde(default)]
    policy_digest: Option<String>,
    #[serde(default)]
    instruction_ref: Option<String>,
    #[serde(default)]
    capability_claims: Vec<String>,
    #[serde(default)]
    witness_refs: Vec<String>,
    #[serde(default)]
    lineage_refs: Vec<String>,
}

#[derive(Debug, Clone)]
struct HostActionSpec {
    operation_id: Option<String>,
    route_family_hint: Option<String>,
}

#[derive(Debug, Clone)]
struct HostActionFailureClasses {
    binding_mismatch: String,
    contract_unbound: String,
}

#[derive(Debug, Clone)]
struct SiteResolveAdmissionConfig {
    profile_id: String,
    policy_digest_prefix: String,
    executable_capabilities: BTreeSet<String>,
    host_action_failures: HostActionFailureClasses,
}

#[derive(Debug, Clone)]
struct HostActionDispatchResult {
    payload: Value,
    failure_classes: Vec<String>,
    witness_refs: Vec<String>,
}

#[derive(Debug, Clone)]
struct HostActionDispatchError {
    failure_class: String,
    diagnostic: String,
}

impl HostActionDispatchError {
    fn new(failure_class: impl Into<String>, diagnostic: impl Into<String>) -> Self {
        Self {
            failure_class: failure_class.into(),
            diagnostic: diagnostic.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RejectionStage {
    RoutePreflight,
    TransportDispatch,
    PolicyCapability,
    InputValidation,
    Unknown,
}

impl RejectionStage {
    fn as_str(self) -> &'static str {
        match self {
            RejectionStage::RoutePreflight => "route preflight",
            RejectionStage::TransportDispatch => "transport dispatch",
            RejectionStage::PolicyCapability => "policy/capability",
            RejectionStage::InputValidation => "input validation",
            RejectionStage::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
struct NonJsonFailureSummary {
    failure_class: String,
    action: String,
    stage: RejectionStage,
    diagnostic: String,
}

pub fn run(args: Args) {
    let program_json: Value = read_json_file_or_exit(&args.program, "scheme-eval program");
    run_with_program_value(args, &FRONTEND_SCHEME, program_json);
}

pub fn run_with_program_value(args: Args, frontend: &FrontendConfig, program_json: Value) {
    let mut program: SchemeEvalProgram =
        serde_json::from_value(program_json).unwrap_or_else(|err| {
            exit_invalid_program(format!("invalid evaluator program shape: {err}"));
        });
    apply_cli_metadata_defaults(&mut program, &args);
    let control_plane_contract: Value =
        read_json_file_or_exit(&args.control_plane_contract, "control-plane contract");
    run_with_inputs(args, frontend, program, control_plane_contract);
}

fn run_with_inputs(
    args: Args,
    frontend: &FrontendConfig,
    program: SchemeEvalProgram,
    control_plane_contract: Value,
) {
    if program.calls.is_empty() {
        exit_invalid_program("program must include at least one call");
    }
    if args.max_calls == 0 {
        exit_invalid_program("--max-calls must be greater than zero");
    }
    if program.calls.len() > args.max_calls {
        exit_with_error(
            FAILURE_CALL_BUDGET_EXCEEDED,
            format!(
                "program call count {} exceeds max_calls {}",
                program.calls.len(),
                args.max_calls
            ),
        );
    }
    if let Some(schema) = program.schema
        && schema != 1
    {
        exit_invalid_program(format!("unsupported program schema: {schema}"));
    }
    if let Some(kind) = program.program_kind.as_deref() {
        let trimmed = kind.trim();
        if !trimmed.is_empty()
            && trimmed != SCHEME_EVAL_REQUEST_KIND
            && trimmed != RHAI_EVAL_REQUEST_KIND
            && trimmed != frontend.request_kind
        {
            exit_invalid_program(format!("unsupported program kind: {kind}"));
        }
    }

    let doctrine_site_input = parse_embedded_json(DOCTRINE_SITE_INPUT_JSON, "DOCTRINE-SITE-INPUT");
    let doctrine_site = parse_embedded_json(DOCTRINE_SITE_JSON, "DOCTRINE-SITE");
    let doctrine_op_registry =
        parse_embedded_json(DOCTRINE_OP_REGISTRY_JSON, "DOCTRINE-OP-REGISTRY");
    let capability_registry = parse_embedded_json(CAPABILITY_REGISTRY_JSON, "CAPABILITY-REGISTRY");
    let admission_config =
        build_site_resolve_admission_config(&control_plane_contract, &capability_registry);

    let host_actions = parse_host_action_surface(&control_plane_contract);
    let allowlisted_actions = derive_allowlisted_actions(&host_actions);
    let mcp_only_actions = parse_mcp_only_actions(&control_plane_contract);
    let trajectory_path = PathBuf::from(args.trajectory_path);
    let now = Utc::now();
    let mut executed_effects: Vec<Value> = Vec::new();
    let mut top_failure_classes: BTreeSet<String> = BTreeSet::new();
    let mut result = "accepted".to_string();

    for (index, call) in program.calls.iter().enumerate() {
        let started_at = Utc::now();
        let action = call.action.trim().to_string();
        let args_payload = normalize_payload(call.args.clone());
        let args_digest = args_digest(&args_payload);
        let call_id = call
            .call_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("call-{}", index + 1));
        let step_id = build_step_id(&args.step_prefix, &call_id, &action, &args_digest, index);
        let issue_id =
            non_empty(call.issue_id.clone()).or_else(|| non_empty(program.issue_id.clone()));
        let policy_digest = non_empty(call.policy_digest.clone())
            .or_else(|| non_empty(program.policy_digest.clone()));
        let instruction_ref = non_empty(call.instruction_ref.clone())
            .or_else(|| non_empty(program.instruction_ref.clone()));
        let capability_claims = merge_claims(&program.capability_claims, &call.capability_claims);
        let mut witness_refs = normalize_refs(call.witness_refs.clone());
        let lineage_refs = normalize_refs(call.lineage_refs.clone());
        let effect_ref = format!(
            "host-effect://scheme-eval/{}/{}/{}",
            sanitize_for_ref(&action),
            args_digest,
            index + 1
        );
        witness_refs.push(effect_ref.clone());
        witness_refs.sort();
        witness_refs.dedup();

        let host_action_spec = host_actions.get(&action);
        let mut effect_failure_classes: Vec<String> = Vec::new();
        let payload = if action.is_empty() {
            effect_failure_classes.push(FAILURE_INVALID_PROGRAM.to_string());
            json!({
                "schema": 1,
                "result": "rejected",
                "failureClasses": [FAILURE_INVALID_PROGRAM],
                "diagnostic": "action is required",
            })
        } else if denies_direct_effect(&action) {
            effect_failure_classes.push(FAILURE_EFFECT_DENIED.to_string());
            json!({
                "schema": 1,
                "result": "rejected",
                "failureClasses": [FAILURE_EFFECT_DENIED],
                "diagnostic": format!("direct effect denied in evaluator profile: {action}"),
            })
        } else if host_action_spec
            .and_then(|spec| spec.operation_id.as_ref())
            .is_none()
            && host_actions.contains_key(&action)
        {
            effect_failure_classes.push(
                admission_config
                    .host_action_failures
                    .contract_unbound
                    .clone(),
            );
            json!({
                "schema": 1,
                "result": "rejected",
                "failureClasses": [admission_config.host_action_failures.contract_unbound.as_str()],
                "diagnostic": format!("host action row is present but operationId is unbound: {action}"),
            })
        } else if !allowlisted_actions.contains(&action) {
            effect_failure_classes.push(FAILURE_ACTION_UNALLOWLISTED.to_string());
            json!({
                "schema": 1,
                "result": "rejected",
                "failureClasses": [FAILURE_ACTION_UNALLOWLISTED],
                "diagnostic": format!("host action is not allowlisted for scheme_eval: {action}"),
            })
        } else if mcp_only_actions.contains(&action) {
            effect_failure_classes.push(FAILURE_MCP_TRANSPORT_REQUIRED.to_string());
            json!({
                "schema": 1,
                "result": "rejected",
                "failureClasses": [FAILURE_MCP_TRANSPORT_REQUIRED],
                "diagnostic": format!("host action requires MCP transport profile: {action}"),
            })
        } else if HOST_ACTIONS_MUTATION.contains(&action.as_str()) {
            match validate_mutation_evidence(
                &action,
                policy_digest.as_deref(),
                instruction_ref.as_deref(),
                &capability_claims,
            ) {
                Ok(()) => dispatch_with_route_admission(
                    &action,
                    &args_payload,
                    host_action_spec,
                    &mut effect_failure_classes,
                    &mut witness_refs,
                    &capability_claims,
                    policy_digest.as_deref(),
                    frontend,
                    &control_plane_contract,
                    &doctrine_site_input,
                    &doctrine_site,
                    &doctrine_op_registry,
                    &capability_registry,
                    &admission_config,
                ),
                Err(err) => {
                    let failure_class = err.failure_class.clone();
                    effect_failure_classes.push(failure_class.clone());
                    json!({
                        "schema": 1,
                        "result": "rejected",
                        "failureClasses": [failure_class],
                        "diagnostic": err.diagnostic
                    })
                }
            }
        } else {
            dispatch_with_route_admission(
                &action,
                &args_payload,
                host_action_spec,
                &mut effect_failure_classes,
                &mut witness_refs,
                &capability_claims,
                policy_digest.as_deref(),
                frontend,
                &control_plane_contract,
                &doctrine_site_input,
                &doctrine_site,
                &doctrine_op_registry,
                &capability_registry,
                &admission_config,
            )
        };

        if effect_failure_classes.is_empty() {
            effect_failure_classes = payload_failure_classes(&payload);
        }
        let result_class = if effect_failure_classes.is_empty() {
            "accepted".to_string()
        } else {
            effect_failure_classes
                .first()
                .cloned()
                .unwrap_or_else(|| FAILURE_EXECUTION_ERROR.to_string())
        };

        let finished_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let row = HarnessTrajectoryRow {
            schema: HARNESS_TRAJECTORY_SCHEMA,
            step_kind: HARNESS_TRAJECTORY_KIND.to_string(),
            step_id: step_id.clone(),
            issue_id: issue_id.clone(),
            action: format!("{}{}", frontend.action_prefix, action),
            result_class: result_class.clone(),
            instruction_refs: instruction_ref.iter().cloned().collect(),
            witness_refs: witness_refs.clone(),
            lineage_refs: lineage_refs.clone(),
            started_at: Some(started_at.to_rfc3339_opts(SecondsFormat::Secs, true)),
            finished_at: finished_at.clone(),
        };
        let appended_row = append_trajectory_row(&trajectory_path, row).unwrap_or_else(|err| {
            exit_with_error(
                FAILURE_EXECUTION_ERROR,
                format!(
                    "failed to append harness trajectory row at {}: {err}",
                    trajectory_path.display()
                ),
            )
        });

        let effect = json!({
            "schema": HOST_EFFECT_SCHEMA,
            "callId": call_id,
            "action": action,
            "argsDigest": args_digest,
            "resultClass": result_class,
            "payload": payload,
            "failureClasses": effect_failure_classes,
            "witnessRefs": witness_refs,
            "policyDigest": policy_digest,
            "instructionRef": instruction_ref,
            "trajectoryRow": {
                "path": trajectory_path.display().to_string(),
                "stepId": appended_row.step_id,
                "resultClass": appended_row.result_class,
                "finishedAt": appended_row.finished_at
            }
        });
        executed_effects.push(effect);

        if let Some(latest) = executed_effects.last() {
            for class in json_string_array(latest.get("failureClasses")) {
                top_failure_classes.insert(class);
            }
            if latest
                .get("resultClass")
                .and_then(Value::as_str)
                .is_some_and(|class| class != "accepted")
            {
                result = "rejected".to_string();
                break;
            }
        }
    }

    let output = json!({
        "schema": 1,
        "kind": frontend.result_kind,
        "profile": frontend.profile,
        "programPath": args.program,
        "controlPlaneContract": args.control_plane_contract,
        "trajectoryPath": trajectory_path.display().to_string(),
        "allowlistedActions": allowlisted_actions.into_iter().collect::<Vec<_>>(),
        "executedCallCount": executed_effects.len(),
        "result": result,
        "failureClasses": top_failure_classes.into_iter().collect::<Vec<_>>(),
        "startedAt": now.to_rfc3339_opts(SecondsFormat::Secs, true),
        "finishedAt": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "effects": executed_effects
    });

    if args.json {
        print_json(&output);
    } else {
        println!("premath {}", frontend.command_name);
        println!("  Program: {}", args.program);
        println!(
            "  Result: {}",
            output["result"].as_str().unwrap_or("unknown")
        );
        println!(
            "  Executed calls: {}",
            output["executedCallCount"].as_u64().unwrap_or(0)
        );
        println!("  Trajectory path: {}", trajectory_path.display());
        if let Some(summary) = summarize_non_json_failure(&output) {
            println!("  Failure class: {}", summary.failure_class);
            println!("  Failure stage: {}", summary.stage.as_str());
            println!("  Failed action: {}", summary.action);
            println!("  Diagnostic: {}", summary.diagnostic);
            println!("  Hint: rerun with --json for full failure envelope");
        }
    }

    if output
        .get("result")
        .and_then(Value::as_str)
        .is_some_and(|result| result != "accepted")
    {
        std::process::exit(1);
    }
}

fn summarize_non_json_failure(output: &Value) -> Option<NonJsonFailureSummary> {
    if output
        .get("result")
        .and_then(Value::as_str)
        .is_none_or(|result| result != "rejected")
    {
        return None;
    }

    let rejected_effect = output
        .get("effects")
        .and_then(Value::as_array)
        .and_then(|effects| {
            effects.iter().find(|effect| {
                effect
                    .get("resultClass")
                    .and_then(Value::as_str)
                    .is_some_and(|result_class| result_class != "accepted")
            })
        })?;

    let failure_class = rejected_effect
        .get("resultClass")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            output
                .get("failureClasses")
                .and_then(Value::as_array)
                .and_then(|classes| classes.first())
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or(FAILURE_EXECUTION_ERROR)
        .to_string();

    let action = rejected_effect
        .get("action")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string();

    let diagnostic = rejected_effect
        .get("payload")
        .and_then(|payload| payload.get("diagnostic"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("no diagnostic provided")
        .to_string();

    Some(NonJsonFailureSummary {
        stage: classify_rejection_stage(failure_class.as_str(), diagnostic.as_str()),
        failure_class,
        action,
        diagnostic,
    })
}

fn classify_rejection_stage(failure_class: &str, diagnostic: &str) -> RejectionStage {
    let class = failure_class.trim();
    let diag = diagnostic.trim();

    if class.starts_with("site_resolve_")
        || class == FAILURE_HOST_ACTION_CONTRACT_UNBOUND
        || diag.contains("kernel route preflight rejected host action")
    {
        return RejectionStage::RoutePreflight;
    }

    if class.starts_with("transport.")
        || class == FAILURE_HOST_ACTION_BINDING_MISMATCH
        || diag.contains("transport resolver witness")
        || diag.contains("transport dispatch")
        || diag.contains("failed to serialize transport request")
        || diag.contains("failed to parse transport response")
    {
        return RejectionStage::TransportDispatch;
    }

    if class == FAILURE_MUTATION_USE_EVIDENCE_MISSING
        || class == FAILURE_MUTATION_CAPABILITY_CLAIM_MISSING
        || class == FAILURE_EFFECT_DENIED
        || class == FAILURE_ACTION_UNALLOWLISTED
        || class == FAILURE_MCP_TRANSPORT_REQUIRED
    {
        return RejectionStage::PolicyCapability;
    }

    if class == FAILURE_INVALID_PROGRAM || class == FAILURE_CALL_BUDGET_EXCEEDED {
        return RejectionStage::InputValidation;
    }

    RejectionStage::Unknown
}

fn parse_host_action_surface(contract: &Value) -> BTreeMap<String, HostActionSpec> {
    let mut map = BTreeMap::new();
    let Some(required_actions) = contract
        .get("hostActionSurface")
        .and_then(Value::as_object)
        .and_then(|surface| surface.get("requiredActions"))
        .and_then(Value::as_object)
    else {
        exit_invalid_program("control-plane contract missing hostActionSurface.requiredActions");
    };

    for (action_id, row) in required_actions {
        let operation_id = row
            .as_object()
            .and_then(|obj| obj.get("operationId"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        map.insert(
            action_id.to_string(),
            HostActionSpec {
                operation_id,
                route_family_hint: expected_route_family_for_host_action(action_id),
            },
        );
    }
    map
}

fn expected_route_family_for_host_action(action: &str) -> Option<String> {
    match action {
        "issue.claim" | "issue.claim_next" | "issue.lease_renew" | "issue.lease_release" => {
            Some("route.issue_claim_lease".to_string())
        }
        "fiber.spawn" | "fiber.join" | "fiber.cancel" => Some("route.fiber.lifecycle".to_string()),
        _ => None,
    }
}

fn parse_mcp_only_actions(contract: &Value) -> BTreeSet<String> {
    contract
        .get("hostActionSurface")
        .and_then(Value::as_object)
        .and_then(|surface| surface.get("mcpOnlyHostActions"))
        .map(|items| json_string_array(Some(items)))
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn derive_allowlisted_actions(host_actions: &BTreeMap<String, HostActionSpec>) -> BTreeSet<String> {
    HOST_ACTIONS_SUPPORTED
        .iter()
        .filter_map(|action| {
            host_actions
                .get(*action)
                .and_then(|spec| spec.operation_id.as_ref().map(|_| (*action).to_string()))
        })
        .collect()
}

fn parse_embedded_json(payload: &str, label: &str) -> Value {
    serde_json::from_str(payload).unwrap_or_else(|err| {
        exit_invalid_program(format!("failed to parse embedded {label}: {err}"))
    })
}

fn parse_executable_capabilities(capability_registry: &Value) -> BTreeSet<String> {
    capability_registry
        .get("executableCapabilities")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_host_action_failure_classes(contract: &Value) -> HostActionFailureClasses {
    let failure_classes = contract
        .get("hostActionSurface")
        .and_then(Value::as_object)
        .and_then(|surface| surface.get("failureClasses"))
        .and_then(Value::as_object);
    let binding_mismatch = failure_classes
        .and_then(|row| row.get("bindingMismatch"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(FAILURE_HOST_ACTION_BINDING_MISMATCH)
        .to_string();
    let contract_unbound = failure_classes
        .and_then(|row| row.get("contractUnbound"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(FAILURE_HOST_ACTION_CONTRACT_UNBOUND)
        .to_string();
    HostActionFailureClasses {
        binding_mismatch,
        contract_unbound,
    }
}

fn resolve_site_profile_id(contract: &Value) -> String {
    let profile = contract
        .get("controlPlaneKcirMappings")
        .and_then(Value::as_object)
        .and_then(|row| row.get("profileId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            contract
                .get("controlPlaneSite")
                .and_then(Value::as_object)
                .and_then(|row| row.get("profileId"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("cp.control.site.v0");
    profile.to_string()
}

fn resolve_policy_digest_prefix(contract: &Value) -> String {
    contract
        .get("ciInstructionPolicy")
        .and_then(Value::as_object)
        .and_then(|row| row.get("policyDigestPrefix"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("pol1_")
        .to_string()
}

fn build_site_resolve_admission_config(
    control_plane_contract: &Value,
    capability_registry: &Value,
) -> SiteResolveAdmissionConfig {
    SiteResolveAdmissionConfig {
        profile_id: resolve_site_profile_id(control_plane_contract),
        policy_digest_prefix: resolve_policy_digest_prefix(control_plane_contract),
        executable_capabilities: parse_executable_capabilities(capability_registry),
        host_action_failures: parse_host_action_failure_classes(control_plane_contract),
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_with_route_admission(
    action: &str,
    args: &Value,
    host_action_spec: Option<&HostActionSpec>,
    effect_failure_classes: &mut Vec<String>,
    witness_refs: &mut Vec<String>,
    capability_claims: &[String],
    policy_digest: Option<&str>,
    frontend: &FrontendConfig,
    control_plane_contract: &Value,
    doctrine_site_input: &Value,
    doctrine_site: &Value,
    doctrine_op_registry: &Value,
    capability_registry: &Value,
    admission_config: &SiteResolveAdmissionConfig,
) -> Value {
    let Some(spec) = host_action_spec else {
        effect_failure_classes.push(FAILURE_ACTION_UNALLOWLISTED.to_string());
        return json!({
            "schema": 1,
            "result": "rejected",
            "failureClasses": [FAILURE_ACTION_UNALLOWLISTED],
            "diagnostic": format!("host action is not allowlisted for scheme_eval: {action}"),
        });
    };

    let preflight = match preflight_route_bound_host_action(
        action,
        spec,
        capability_claims,
        policy_digest,
        frontend,
        control_plane_contract,
        doctrine_site_input,
        doctrine_site,
        doctrine_op_registry,
        capability_registry,
        admission_config,
    ) {
        Ok(value) => value,
        Err(err) => {
            let failure_class = err.failure_class.clone();
            effect_failure_classes.push(failure_class.clone());
            return json!({
                "schema": 1,
                "result": "rejected",
                "failureClasses": [failure_class],
                "diagnostic": err.diagnostic,
            });
        }
    };

    if let Some(witness) = preflight.as_ref() {
        witness_refs.push(resolver_witness_ref(witness));
        witness_refs.sort();
        witness_refs.dedup();
    }

    let mut payload = dispatch_or_rejected(action, args, effect_failure_classes, witness_refs);
    if let Some(preflight_witness) = preflight.as_ref()
        && let Err(err) = enforce_dispatch_resolver_witness_parity(
            &payload,
            preflight_witness,
            admission_config
                .host_action_failures
                .binding_mismatch
                .as_str(),
        )
    {
        effect_failure_classes.clear();
        let failure_class = err.failure_class.clone();
        effect_failure_classes.push(failure_class.clone());
        payload = json!({
            "schema": 1,
            "result": "rejected",
            "failureClasses": [failure_class],
            "diagnostic": err.diagnostic,
        });
    }
    payload
}

#[allow(clippy::too_many_arguments)]
fn preflight_route_bound_host_action(
    action: &str,
    host_action_spec: &HostActionSpec,
    capability_claims: &[String],
    policy_digest: Option<&str>,
    frontend: &FrontendConfig,
    control_plane_contract: &Value,
    doctrine_site_input: &Value,
    doctrine_site: &Value,
    doctrine_op_registry: &Value,
    capability_registry: &Value,
    admission_config: &SiteResolveAdmissionConfig,
) -> Result<Option<SiteResolveWitness>, HostActionDispatchError> {
    let Some(route_family_hint) = host_action_spec.route_family_hint.clone() else {
        return Ok(None);
    };
    let operation_id = host_action_spec.operation_id.clone().ok_or_else(|| {
        HostActionDispatchError::new(
            admission_config
                .host_action_failures
                .contract_unbound
                .as_str(),
            format!("host action `{action}` missing operationId binding"),
        )
    })?;
    let filtered_claims = capability_claims
        .iter()
        .map(|claim| claim.trim())
        .filter(|claim| !claim.is_empty())
        .filter(|claim| admission_config.executable_capabilities.contains(*claim))
        .map(str::to_string)
        .collect();
    let policy_digest = policy_digest
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "{}scheme_eval_route_preflight",
                admission_config.policy_digest_prefix
            )
        });
    let request = SiteResolveRequest {
        schema: 1,
        request_kind: "premath.site_resolve.request.v1".to_string(),
        operation_id: operation_id.clone(),
        route_family_hint: Some(route_family_hint.clone()),
        claimed_capabilities: filtered_claims,
        policy_digest,
        profile_id: admission_config.profile_id.clone(),
        context_ref: format!("{}.route-preflight.{action}", frontend.command_name),
    };
    let response = resolve_site_request(
        &request,
        doctrine_site_input,
        doctrine_site,
        doctrine_op_registry,
        control_plane_contract,
        capability_registry,
    );
    if response.result == "accepted" {
        return Ok(Some(response.witness));
    }
    let failure_class = response
        .failure_classes
        .first()
        .cloned()
        .unwrap_or_else(|| {
            admission_config
                .host_action_failures
                .contract_unbound
                .clone()
        });
    Err(HostActionDispatchError::new(
        failure_class,
        format!(
            "kernel route preflight rejected host action `{action}` operation={} route={} failures={:?}",
            operation_id, route_family_hint, response.failure_classes
        ),
    ))
}

fn resolver_witness_ref(witness: &SiteResolveWitness) -> String {
    format!(
        "resolver://site-resolve/{}/{}",
        witness.operation_id.replace('/', "_"),
        witness.semantic_digest
    )
}

fn payload_result(payload: &Value) -> Option<&str> {
    payload.get("result").and_then(Value::as_str).map(str::trim)
}

fn enforce_dispatch_resolver_witness_parity(
    payload: &Value,
    preflight_witness: &SiteResolveWitness,
    binding_mismatch_failure_class: &str,
) -> Result<(), HostActionDispatchError> {
    if payload_result(payload) != Some("accepted") {
        return Ok(());
    }
    let resolver_witness = payload
        .get("resolverWitness")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            HostActionDispatchError::new(
                binding_mismatch_failure_class,
                "transport dispatch accepted without resolverWitness payload",
            )
        })?;
    let dispatched_operation_id = resolver_witness
        .get("operationId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            HostActionDispatchError::new(
                binding_mismatch_failure_class,
                "transport resolverWitness.operationId missing",
            )
        })?;
    let dispatched_route_family_id = resolver_witness
        .get("routeFamilyId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let dispatched_world_id = resolver_witness
        .get("worldId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let dispatched_morphism_row_id = resolver_witness
        .get("morphismRowId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let binding_matches = dispatched_operation_id == preflight_witness.operation_id
        && dispatched_route_family_id == preflight_witness.route_family_id.as_deref()
        && dispatched_world_id == preflight_witness.world_id.as_deref()
        && dispatched_morphism_row_id == preflight_witness.morphism_row_id.as_deref();
    if binding_matches {
        return Ok(());
    }

    Err(HostActionDispatchError::new(
        binding_mismatch_failure_class,
        format!(
            "transport resolver witness drift detected: preflight=({}, {:?}, {:?}, {:?}) dispatch=({}, {:?}, {:?}, {:?})",
            preflight_witness.operation_id,
            preflight_witness.route_family_id,
            preflight_witness.world_id,
            preflight_witness.morphism_row_id,
            dispatched_operation_id,
            dispatched_route_family_id,
            dispatched_world_id,
            dispatched_morphism_row_id
        ),
    ))
}

fn dispatch_host_action(
    action: &str,
    args: &Value,
) -> Result<HostActionDispatchResult, HostActionDispatchError> {
    if HOST_ACTIONS_TRANSPORT_DISPATCH.contains(&action) {
        return dispatch_transport_action(action, args.clone());
    }
    match action {
        "issue.ready" => dispatch_issue_ready(args),
        "issue.list" => dispatch_issue_list(args),
        "issue.blocked" => dispatch_issue_blocked(args),
        "issue.check" => dispatch_issue_check(args),
        "dep.diagnostics" => dispatch_dep_diagnostics(args),
        _ => Err(HostActionDispatchError::new(
            FAILURE_ACTION_UNIMPLEMENTED,
            format!("unsupported host action in scheme_eval: {action}"),
        )),
    }
}

fn dispatch_transport_action(
    action: &str,
    payload: Value,
) -> Result<HostActionDispatchResult, HostActionDispatchError> {
    let request = json!({ "action": action, "payload": payload });
    let request_json = serde_json::to_string(&request).map_err(|err| {
        HostActionDispatchError::new(
            FAILURE_EXECUTION_ERROR,
            format!("failed to serialize transport request: {err}"),
        )
    })?;
    let response_json = transport_dispatch_json(&request_json);
    let response: Value = serde_json::from_str(&response_json).map_err(|err| {
        HostActionDispatchError::new(
            FAILURE_EXECUTION_ERROR,
            format!("failed to parse transport response: {err}"),
        )
    })?;
    let witness_refs = extract_witness_refs(&response);
    let failure_classes = payload_failure_classes(&response);
    Ok(HostActionDispatchResult {
        payload: response,
        failure_classes,
        witness_refs,
    })
}

fn dispatch_or_rejected(
    action: &str,
    args: &Value,
    effect_failure_classes: &mut Vec<String>,
    witness_refs: &mut Vec<String>,
) -> Value {
    match dispatch_host_action(action, args) {
        Ok(dispatch) => {
            *effect_failure_classes = dispatch.failure_classes.clone();
            witness_refs.extend(dispatch.witness_refs);
            witness_refs.sort();
            witness_refs.dedup();
            dispatch.payload
        }
        Err(err) => {
            let failure_class = err.failure_class.clone();
            effect_failure_classes.push(failure_class.clone());
            json!({
                "schema": 1,
                "result": "rejected",
                "failureClasses": [failure_class],
                "diagnostic": err.diagnostic
            })
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueReadyArgs {
    #[serde(default, alias = "issues_path")]
    issues_path: Option<String>,
}

fn dispatch_issue_ready(args: &Value) -> Result<HostActionDispatchResult, HostActionDispatchError> {
    let parsed: IssueReadyArgs = serde_json::from_value(args.clone()).map_err(|err| {
        HostActionDispatchError::new(
            FAILURE_INVALID_PROGRAM,
            format!("invalid issue.ready args: {err}"),
        )
    })?;
    let issues_path = resolve_issues_path(parsed.issues_path);
    let store = load_store(&issues_path)?;
    let cache = QueryCache::hydrate(&store);
    let ids = cache.ready_open_issue_ids();
    let items = ids
        .iter()
        .filter_map(|id| cache.issue(id))
        .map(|issue| {
            json!({
                "id": issue.id,
                "title": issue.title,
                "status": issue.status,
                "priority": issue.priority
            })
        })
        .collect::<Vec<_>>();
    Ok(HostActionDispatchResult {
        payload: json!({
            "action": "issue.ready",
            "issuesPath": issues_path,
            "count": items.len(),
            "items": items
        }),
        failure_classes: Vec::new(),
        witness_refs: Vec::new(),
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueListArgs {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default, alias = "issues_path")]
    issues_path: Option<String>,
}

fn dispatch_issue_list(args: &Value) -> Result<HostActionDispatchResult, HostActionDispatchError> {
    let parsed: IssueListArgs = serde_json::from_value(args.clone()).map_err(|err| {
        HostActionDispatchError::new(
            FAILURE_INVALID_PROGRAM,
            format!("invalid issue.list args: {err}"),
        )
    })?;
    let issues_path = resolve_issues_path(parsed.issues_path);
    let store = load_store(&issues_path)?;
    let status_filter = non_empty(parsed.status);
    let assignee_filter = non_empty(parsed.assignee);
    let items = store
        .issues()
        .filter(|issue| {
            status_filter
                .as_ref()
                .is_none_or(|value| issue.status == *value)
        })
        .filter(|issue| {
            assignee_filter
                .as_ref()
                .is_none_or(|value| issue.assignee == *value)
        })
        .map(|issue| {
            json!({
                "id": issue.id,
                "title": issue.title,
                "status": issue.status,
                "priority": issue.priority,
                "issueType": issue.issue_type,
                "assignee": issue.assignee
            })
        })
        .collect::<Vec<_>>();
    Ok(HostActionDispatchResult {
        payload: json!({
            "action": "issue.list",
            "issuesPath": issues_path,
            "count": items.len(),
            "items": items
        }),
        failure_classes: Vec::new(),
        witness_refs: Vec::new(),
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueBlockedArgs {
    #[serde(default, alias = "issues_path")]
    issues_path: Option<String>,
}

fn dispatch_issue_blocked(
    args: &Value,
) -> Result<HostActionDispatchResult, HostActionDispatchError> {
    let parsed: IssueBlockedArgs = serde_json::from_value(args.clone()).map_err(|err| {
        HostActionDispatchError::new(
            FAILURE_INVALID_PROGRAM,
            format!("invalid issue.blocked args: {err}"),
        )
    })?;
    let issues_path = resolve_issues_path(parsed.issues_path);
    let store = load_store(&issues_path)?;
    let cache = QueryCache::hydrate(&store);

    let items = store
        .issues()
        .filter(|issue| issue.status != "closed")
        .filter_map(|issue| {
            let manual_blocked = issue.status == "blocked";
            let blockers = store
                .blocking_dependencies_of(&issue.id)
                .into_iter()
                .filter_map(|dep| {
                    let blocker = cache.issue(&dep.depends_on_id);
                    let unresolved = blocker.is_none_or(|row| row.status != "closed");
                    if !unresolved {
                        return None;
                    }
                    Some(json!({
                        "issueId": dep.issue_id,
                        "dependsOnId": dep.depends_on_id,
                        "type": dep.dep_type.as_str(),
                        "createdBy": dep.created_by,
                        "blockerStatus": blocker.map(|row| row.status.clone()),
                        "blockerMissing": blocker.is_none()
                    }))
                })
                .collect::<Vec<_>>();
            if blockers.is_empty() && !manual_blocked {
                return None;
            }
            Some(json!({
                "id": issue.id,
                "title": issue.title,
                "status": issue.status,
                "priority": issue.priority,
                "manualBlocked": manual_blocked,
                "blockers": blockers
            }))
        })
        .collect::<Vec<_>>();

    Ok(HostActionDispatchResult {
        payload: json!({
            "action": "issue.blocked",
            "issuesPath": issues_path,
            "count": items.len(),
            "items": items
        }),
        failure_classes: Vec::new(),
        witness_refs: Vec::new(),
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueCheckArgs {
    #[serde(default, alias = "issues_path")]
    issues_path: Option<String>,
    #[serde(default, alias = "note_warn_threshold")]
    note_warn_threshold: Option<usize>,
}

fn dispatch_issue_check(args: &Value) -> Result<HostActionDispatchResult, HostActionDispatchError> {
    let parsed: IssueCheckArgs = serde_json::from_value(args.clone()).map_err(|err| {
        HostActionDispatchError::new(
            FAILURE_INVALID_PROGRAM,
            format!("invalid issue.check args: {err}"),
        )
    })?;
    let issues_path = resolve_issues_path(parsed.issues_path);
    let store = load_store(&issues_path)?;
    let report = store.check_issue_graph(parsed.note_warn_threshold.unwrap_or(2000));
    let failure_classes = if report.accepted() {
        Vec::new()
    } else {
        report.failure_classes.clone()
    };
    Ok(HostActionDispatchResult {
        payload: json!({
            "action": "issue.check",
            "issuesPath": issues_path,
            "checkKind": report.check_kind,
            "result": report.result,
            "failureClasses": report.failure_classes,
            "warningClasses": report.warning_classes,
            "errors": report.errors,
            "warnings": report.warnings,
            "summary": report.summary
        }),
        failure_classes,
        witness_refs: Vec::new(),
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DepDiagnosticsArgs {
    #[serde(default, alias = "issues_path")]
    issues_path: Option<String>,
    #[serde(default, alias = "graph_scope")]
    graph_scope: Option<String>,
}

fn dispatch_dep_diagnostics(
    args: &Value,
) -> Result<HostActionDispatchResult, HostActionDispatchError> {
    let parsed: DepDiagnosticsArgs = serde_json::from_value(args.clone()).map_err(|err| {
        HostActionDispatchError::new(
            FAILURE_INVALID_PROGRAM,
            format!("invalid dep.diagnostics args: {err}"),
        )
    })?;
    let issues_path = resolve_issues_path(parsed.issues_path);
    let store = load_store(&issues_path)?;
    let graph_scope = parsed
        .graph_scope
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("active");
    let scope = match graph_scope {
        "active" => premath_bd::DependencyGraphScope::Active,
        "full" => premath_bd::DependencyGraphScope::Full,
        _ => {
            return Err(HostActionDispatchError::new(
                FAILURE_INVALID_PROGRAM,
                format!(
                    "dep.diagnostics graphScope must be `active` or `full` (got `{graph_scope}`)"
                ),
            ));
        }
    };
    let cycle = store.find_any_dependency_cycle_in_scope(scope);
    Ok(HostActionDispatchResult {
        payload: json!({
            "action": "dep.diagnostics",
            "issuesPath": issues_path,
            "graphScope": graph_scope,
            "integrity": {
                "hasCycle": cycle.is_some(),
                "cyclePath": cycle
            }
        }),
        failure_classes: Vec::new(),
        witness_refs: Vec::new(),
    })
}

fn validate_mutation_evidence(
    action: &str,
    policy_digest: Option<&str>,
    instruction_ref: Option<&str>,
    capability_claims: &[String],
) -> Result<(), HostActionDispatchError> {
    if policy_digest.is_none_or(|value| value.trim().is_empty())
        || instruction_ref.is_none_or(|value| value.trim().is_empty())
    {
        return Err(HostActionDispatchError::new(
            FAILURE_MUTATION_USE_EVIDENCE_MISSING,
            format!(
                "mutation action `{action}` requires non-empty policyDigest and instructionRef"
            ),
        ));
    }
    if !capability_claims
        .iter()
        .any(|claim| claim == CHANGE_MORPHISMS_BASE_CAPABILITY)
    {
        return Err(HostActionDispatchError::new(
            FAILURE_MUTATION_CAPABILITY_CLAIM_MISSING,
            format!(
                "mutation action `{action}` requires capability claim `{CHANGE_MORPHISMS_BASE_CAPABILITY}`"
            ),
        ));
    }
    let required_action_claim = required_action_capability_claim(action).ok_or_else(|| {
        HostActionDispatchError::new(
            FAILURE_MUTATION_CAPABILITY_CLAIM_MISSING,
            format!("mutation action `{action}` has no capability mapping"),
        )
    })?;
    let action_allowed = capability_claims
        .iter()
        .any(|claim| claim == required_action_claim || claim == CHANGE_MORPHISMS_ALL_CAPABILITY);
    if !action_allowed {
        return Err(HostActionDispatchError::new(
            FAILURE_MUTATION_CAPABILITY_CLAIM_MISSING,
            format!(
                "mutation action `{action}` requires capability claim `{required_action_claim}` or `{CHANGE_MORPHISMS_ALL_CAPABILITY}`"
            ),
        ));
    }
    Ok(())
}

fn required_action_capability_claim(action: &str) -> Option<&'static str> {
    match action {
        "issue.claim" | "issue.claim_next" => Some("capabilities.change_morphisms.issue_claim"),
        "issue.lease_renew" => Some("capabilities.change_morphisms.issue_lease_renew"),
        "issue.lease_release" => Some("capabilities.change_morphisms.issue_lease_release"),
        _ => None,
    }
}

fn denies_direct_effect(action: &str) -> bool {
    let lowered = action.to_ascii_lowercase();
    lowered.starts_with("shell.")
        || lowered.starts_with("network.")
        || lowered.starts_with("http.")
        || lowered.starts_with("exec.")
}

fn resolve_issues_path(raw: Option<String>) -> String {
    non_empty(raw).unwrap_or_else(|| DEFAULT_ISSUES_PATH.to_string())
}

fn load_store(path: &str) -> Result<MemoryStore, HostActionDispatchError> {
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        return Err(HostActionDispatchError::new(
            FAILURE_EXECUTION_ERROR,
            format!("issues file not found: {}", path_buf.display()),
        ));
    }
    MemoryStore::load_jsonl(&path_buf).map_err(|err| {
        HostActionDispatchError::new(
            FAILURE_EXECUTION_ERROR,
            format!("failed to load {}: {err}", path_buf.display()),
        )
    })
}

fn payload_failure_classes(payload: &Value) -> Vec<String> {
    json_string_array(payload.get("failureClasses"))
}

fn extract_witness_refs(payload: &Value) -> Vec<String> {
    let mut refs = BTreeSet::new();
    if let Some(value) = payload.get("fiberWitnessRef").and_then(Value::as_str) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            refs.insert(trimmed.to_string());
        }
    }
    if let Some(value) = payload.get("witnessRef").and_then(Value::as_str) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            refs.insert(trimmed.to_string());
        }
    }
    if let Some(value) = payload.get("resolverWitnessRef").and_then(Value::as_str) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            refs.insert(trimmed.to_string());
        }
    }
    refs.into_iter().collect()
}

fn merge_claims(base: &[String], extra: &[String]) -> Vec<String> {
    let mut merged = BTreeSet::new();
    for claim in base.iter().chain(extra.iter()) {
        let claim = claim.trim();
        if !claim.is_empty() {
            merged.insert(claim.to_string());
        }
    }
    merged.into_iter().collect()
}

fn apply_cli_metadata_defaults(program: &mut SchemeEvalProgram, args: &Args) {
    if let Some(issue_id) = non_empty(args.issue_id.clone()) {
        program.issue_id = Some(issue_id);
    }
    if let Some(policy_digest) = non_empty(args.policy_digest.clone()) {
        program.policy_digest = Some(policy_digest);
    }
    if let Some(instruction_ref) = non_empty(args.instruction_ref.clone()) {
        program.instruction_ref = Some(instruction_ref);
    }
    program.capability_claims = merge_claims(&program.capability_claims, &args.capability_claims);
}

fn normalize_refs(values: Vec<String>) -> Vec<String> {
    let mut refs = BTreeSet::new();
    for value in values {
        let value = value.trim();
        if !value.is_empty() {
            refs.insert(value.to_string());
        }
    }
    refs.into_iter().collect()
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn build_step_id(
    step_prefix: &str,
    call_id: &str,
    action: &str,
    args_digest: &str,
    index: usize,
) -> String {
    let prefix = if step_prefix.trim().is_empty() {
        "scheme_eval"
    } else {
        step_prefix.trim()
    };
    format!(
        "{prefix}.{}.{}.{}",
        index + 1,
        sanitize_for_ref(call_id),
        sanitize_for_ref(&format!("{action}.{args_digest}"))
    )
}

fn sanitize_for_ref(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "x".to_string()
    } else {
        trimmed.to_string()
    }
}

fn args_digest(args: &Value) -> String {
    let canonical = canonicalize_json(args);
    let bytes = serde_json::to_vec(&canonical).expect("canonical args should serialize");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("he1_{:x}", hasher.finalize())
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = Map::new();
            let mut sorted = BTreeMap::new();
            for (key, item) in map {
                sorted.insert(key.clone(), canonicalize_json(item));
            }
            for (key, item) in sorted {
                out.insert(key, item);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json).collect()),
        _ => value.clone(),
    }
}

fn normalize_payload(value: Value) -> Value {
    match value {
        Value::Null => json!({}),
        other => other,
    }
}

fn json_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn print_json(payload: &Value) {
    let rendered = serde_json::to_string_pretty(payload).unwrap_or_else(|err| {
        eprintln!("error: failed to render scheme-eval payload: {err}");
        std::process::exit(2);
    });
    println!("{rendered}");
}

fn exit_invalid_program(message: impl Into<String>) -> ! {
    exit_with_error(FAILURE_INVALID_PROGRAM, message)
}

fn exit_with_error(failure_class: &str, message: impl Into<String>) -> ! {
    let diagnostic = message.into();
    eprintln!("error: {diagnostic} ({failure_class})");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_digest_is_stable_for_key_order() {
        let left = json!({"b": 2, "a": 1, "nested": {"z": true, "x": 1}});
        let right = json!({"nested": {"x": 1, "z": true}, "a": 1, "b": 2});
        assert_eq!(args_digest(&left), args_digest(&right));
    }

    #[test]
    fn detects_direct_effect_denied_prefixes() {
        assert!(denies_direct_effect("shell.exec"));
        assert!(denies_direct_effect("network.request"));
        assert!(denies_direct_effect("http.get"));
        assert!(!denies_direct_effect("issue.ready"));
    }
}
