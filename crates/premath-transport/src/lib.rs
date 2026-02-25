use chrono::{DateTime, Duration, Utc};
use premath_bd::{
    AtomicStoreMutationError, ClaimNextError, ClaimNextRequest, DEFAULT_LEASE_TTL_SECONDS, Issue,
    IssueLease, IssueLeaseState, MAX_LEASE_TTL_SECONDS, MIN_LEASE_TTL_SECONDS, MemoryStore,
    claim_next_issue_jsonl, mutate_store_jsonl,
};
use premath_kernel::{
    RequiredRouteBinding, SiteResolveRequest, SiteResolveWitness, parse_operation_route_rows,
    resolve_site_request, validate_world_route_bindings_with_requirements, world_failure_class,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;

const DEFAULT_ISSUES_PATH: &str = ".premath/issues.jsonl";
const WORLD_ID_LEASE: &str = "world.lease.v1";
const ROUTE_FAMILY_LEASE: &str = "route.issue_claim_lease";
const MORPHISM_ROW_LEASE: &str = "wm.control.lease.mutation";
const WORLD_ID_TRANSPORT: &str = "world.transport.v1";
const ROUTE_FAMILY_TRANSPORT: &str = "route.transport.dispatch";
const MORPHISM_ROW_TRANSPORT: &str = "wm.control.transport.dispatch";
const WORLD_ID_FIBER: &str = "world.fiber.v1";
const ROUTE_FAMILY_FIBER: &str = "route.fiber.lifecycle";
const MORPHISM_ROW_FIBER: &str = "wm.control.fiber.lifecycle";
const DOCTRINE_SITE_INPUT_JSON: &str =
    include_str!("../../../specs/premath/draft/DOCTRINE-SITE-INPUT.json");
const DOCTRINE_SITE_JSON: &str = include_str!("../../../specs/premath/draft/DOCTRINE-SITE.json");
const DOCTRINE_OP_REGISTRY_JSON: &str =
    include_str!("../../../specs/premath/draft/DOCTRINE-OP-REGISTRY.json");
const CONTROL_PLANE_CONTRACT_JSON: &str =
    include_str!("../../../specs/premath/draft/CONTROL-PLANE-CONTRACT.json");
const CAPABILITY_REGISTRY_JSON: &str =
    include_str!("../../../specs/premath/draft/CAPABILITY-REGISTRY.json");

const TRANSPORT_DISPATCH_KIND: &str = "premath.transport_dispatch.v1";
const TRANSPORT_ACTION_REGISTRY_KIND: &str = "premath.transport_action_registry.v1";
const TRANSPORT_CHECK_KIND: &str = "premath.transport_check.v1";
const TRANSPORT_PROFILE_ID: &str = "transport.issue_lease.v1";
const TRANSPORT_SEMANTIC_DIGEST_PREFIX: &str = "ts1_";
const ACTION_ID_TRANSPORT_INVALID_REQUEST: &str = "transport.action.invalid_request";
const ACTION_ID_TRANSPORT_UNKNOWN: &str = "transport.action.unknown";

const FAILURE_LEASE_INVALID_ASSIGNEE: &str = "lease_invalid_assignee";
const FAILURE_LEASE_INVALID_TTL: &str = "lease_invalid_ttl";
const FAILURE_LEASE_BINDING_AMBIGUOUS: &str = "lease_binding_ambiguous";
const FAILURE_LEASE_INVALID_EXPIRES_AT: &str = "lease_invalid_expires_at";
const FAILURE_LEASE_NOT_FOUND: &str = "lease_not_found";
const FAILURE_LEASE_CLOSED: &str = "lease_issue_closed";
const FAILURE_LEASE_CONTENTION_ACTIVE: &str = "lease_contention_active";
const FAILURE_LEASE_MISSING: &str = "lease_missing";
const FAILURE_LEASE_STALE: &str = "lease_stale";
const FAILURE_LEASE_OWNER_MISMATCH: &str = "lease_owner_mismatch";
const FAILURE_LEASE_ID_MISMATCH: &str = "lease_id_mismatch";
const FAILURE_LEASE_MUTATION_LOCK_BUSY: &str = "lease_mutation_lock_busy";
const FAILURE_LEASE_MUTATION_LOCK_IO: &str = "lease_mutation_lock_io";
const FAILURE_LEASE_MUTATION_STORE_IO: &str = "lease_mutation_store_io";
const FAILURE_LEASE_INVALID_PAYLOAD: &str = "lease_invalid_payload";
const FAILURE_LEASE_UNKNOWN_ACTION: &str = "lease_unknown_action";
const FAILURE_TRANSPORT_INVALID_REQUEST: &str = "transport_invalid_request";
const FAILURE_TRANSPORT_UNKNOWN_ACTION: &str = "transport_unknown_action";
const FAILURE_TRANSPORT_REGISTRY_EMPTY_FIELD: &str = "transport_registry_empty_field";
const FAILURE_TRANSPORT_REGISTRY_DUPLICATE_ACTION: &str = "transport_registry_duplicate_action";
const FAILURE_TRANSPORT_REGISTRY_DUPLICATE_ACTION_ID: &str =
    "transport_registry_duplicate_action_id";
const FAILURE_TRANSPORT_REGISTRY_MISSING_ACTION: &str = "transport_registry_missing_action";
const FAILURE_TRANSPORT_REGISTRY_DIGEST_MISMATCH: &str = "transport_registry_digest_mismatch";
const FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE: &str = "transport_kernel_contract_unavailable";
const FAILURE_FIBER_INVALID_PAYLOAD: &str = "fiber_invalid_payload";
const FAILURE_FIBER_MISSING_FIELD: &str = "fiber_missing_field";

#[derive(Debug, Clone, Copy)]
enum LeaseActionKind {
    Claim,
    ClaimNext,
    Renew,
    Release,
}

impl LeaseActionKind {
    fn action(self) -> &'static str {
        match self {
            Self::Claim => "issue.claim",
            Self::ClaimNext => "issue.claim_next",
            Self::Renew => "issue.lease_renew",
            Self::Release => "issue.lease_release",
        }
    }

    fn transport_action_id(self) -> TransportActionId {
        match self {
            Self::Claim => TransportActionId::IssueClaim,
            Self::ClaimNext => TransportActionId::IssueClaimNext,
            Self::Renew => TransportActionId::IssueLeaseRenew,
            Self::Release => TransportActionId::IssueLeaseRelease,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportActionId {
    IssueClaim,
    IssueClaimNext,
    IssueLeaseRenew,
    IssueLeaseRelease,
    WorldRouteBinding,
    FiberSpawn,
    FiberJoin,
    FiberCancel,
}

impl TransportActionId {
    fn from_action(value: &str) -> Option<Self> {
        let action = value.trim();
        TRANSPORT_ACTION_SPECS
            .iter()
            .find(|spec| spec.action == action)
            .map(|spec| spec.action_id)
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::IssueClaim => "transport.action.issue_claim",
            Self::IssueClaimNext => "transport.action.issue_claim_next",
            Self::IssueLeaseRenew => "transport.action.issue_lease_renew",
            Self::IssueLeaseRelease => "transport.action.issue_lease_release",
            Self::WorldRouteBinding => "transport.action.world_route_binding",
            Self::FiberSpawn => "transport.action.fiber_spawn",
            Self::FiberJoin => "transport.action.fiber_join",
            Self::FiberCancel => "transport.action.fiber_cancel",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TransportActionSpec {
    action_id: TransportActionId,
    action: &'static str,
    operation_id: &'static str,
    route_family_id: &'static str,
    world_id: &'static str,
    morphism_row_id: &'static str,
    required_morphisms: &'static [&'static str],
}

const REQUIRED_MORPHISMS_LEASE: &[&str] = &[
    "dm.identity",
    "dm.profile.execution",
    "dm.commitment.attest",
];
const REQUIRED_MORPHISMS_TRANSPORT: &[&str] = &["dm.identity", "dm.transport.world"];
const REQUIRED_MORPHISMS_FIBER: &[&str] =
    &["dm.identity", "dm.profile.execution", "dm.transport.world"];

const TRANSPORT_ACTION_SPECS: [TransportActionSpec; 8] = [
    TransportActionSpec {
        action_id: TransportActionId::IssueClaim,
        action: "issue.claim",
        operation_id: "op/mcp.issue_claim",
        route_family_id: ROUTE_FAMILY_LEASE,
        world_id: WORLD_ID_LEASE,
        morphism_row_id: MORPHISM_ROW_LEASE,
        required_morphisms: REQUIRED_MORPHISMS_LEASE,
    },
    TransportActionSpec {
        action_id: TransportActionId::IssueClaimNext,
        action: "issue.claim_next",
        operation_id: "op/transport.issue_claim_next",
        route_family_id: ROUTE_FAMILY_LEASE,
        world_id: WORLD_ID_LEASE,
        morphism_row_id: MORPHISM_ROW_LEASE,
        required_morphisms: REQUIRED_MORPHISMS_LEASE,
    },
    TransportActionSpec {
        action_id: TransportActionId::IssueLeaseRenew,
        action: "issue.lease_renew",
        operation_id: "op/mcp.issue_lease_renew",
        route_family_id: ROUTE_FAMILY_LEASE,
        world_id: WORLD_ID_LEASE,
        morphism_row_id: MORPHISM_ROW_LEASE,
        required_morphisms: REQUIRED_MORPHISMS_LEASE,
    },
    TransportActionSpec {
        action_id: TransportActionId::IssueLeaseRelease,
        action: "issue.lease_release",
        operation_id: "op/mcp.issue_lease_release",
        route_family_id: ROUTE_FAMILY_LEASE,
        world_id: WORLD_ID_LEASE,
        morphism_row_id: MORPHISM_ROW_LEASE,
        required_morphisms: REQUIRED_MORPHISMS_LEASE,
    },
    TransportActionSpec {
        action_id: TransportActionId::WorldRouteBinding,
        action: "world.route_binding",
        operation_id: "op/transport.world_route_binding",
        route_family_id: ROUTE_FAMILY_TRANSPORT,
        world_id: WORLD_ID_TRANSPORT,
        morphism_row_id: MORPHISM_ROW_TRANSPORT,
        required_morphisms: REQUIRED_MORPHISMS_TRANSPORT,
    },
    TransportActionSpec {
        action_id: TransportActionId::FiberSpawn,
        action: "fiber.spawn",
        operation_id: "op/transport.fiber_spawn",
        route_family_id: ROUTE_FAMILY_FIBER,
        world_id: WORLD_ID_FIBER,
        morphism_row_id: MORPHISM_ROW_FIBER,
        required_morphisms: REQUIRED_MORPHISMS_FIBER,
    },
    TransportActionSpec {
        action_id: TransportActionId::FiberJoin,
        action: "fiber.join",
        operation_id: "op/transport.fiber_join",
        route_family_id: ROUTE_FAMILY_FIBER,
        world_id: WORLD_ID_FIBER,
        morphism_row_id: MORPHISM_ROW_FIBER,
        required_morphisms: REQUIRED_MORPHISMS_FIBER,
    },
    TransportActionSpec {
        action_id: TransportActionId::FiberCancel,
        action: "fiber.cancel",
        operation_id: "op/transport.fiber_cancel",
        route_family_id: ROUTE_FAMILY_FIBER,
        world_id: WORLD_ID_FIBER,
        morphism_row_id: MORPHISM_ROW_FIBER,
        required_morphisms: REQUIRED_MORPHISMS_FIBER,
    },
];

fn transport_action_spec(action_id: TransportActionId) -> &'static TransportActionSpec {
    TRANSPORT_ACTION_SPECS
        .iter()
        .find(|spec| spec.action_id == action_id)
        .expect("transport action spec must exist for every action id")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldRouteBinding {
    pub operation_id: String,
    pub route_family_id: String,
    pub world_id: String,
    pub morphism_row_id: String,
}

fn world_binding(kind: LeaseActionKind) -> WorldRouteBinding {
    world_binding_for_action(kind.transport_action_id())
}

fn world_binding_for_action(action_id: TransportActionId) -> WorldRouteBinding {
    let spec = transport_action_spec(action_id);
    WorldRouteBinding {
        operation_id: spec.operation_id.to_string(),
        route_family_id: spec.route_family_id.to_string(),
        world_id: spec.world_id.to_string(),
        morphism_row_id: spec.morphism_row_id.to_string(),
    }
}

fn resolver_witness_ref(witness: &SiteResolveWitness) -> String {
    format!(
        "resolver://site-resolve/{}/{}",
        witness.operation_id.replace('/', "_"),
        witness.semantic_digest
    )
}

fn resolve_witness_for_action(action_id: TransportActionId) -> Option<SiteResolveWitness> {
    let spec = transport_action_spec(action_id);
    let response = resolve_site_for_spec(spec).ok()?;
    Some(response.witness)
}

fn resolver_fields_for_action(
    action_id: TransportActionId,
) -> (Option<String>, Option<SiteResolveWitness>) {
    let witness = resolve_witness_for_action(action_id);
    let witness_ref = witness.as_ref().map(resolver_witness_ref);
    (witness_ref, witness)
}

fn semantic_digest(material: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in material {
        hasher.update(part.as_bytes());
        hasher.update([0u8]);
    }
    format!("{TRANSPORT_SEMANTIC_DIGEST_PREFIX}{:x}", hasher.finalize())
}

fn transport_dispatch_digest(action: &str, action_id: &str) -> String {
    semantic_digest(&[
        TRANSPORT_PROFILE_ID,
        TRANSPORT_DISPATCH_KIND,
        action,
        action_id,
    ])
}

fn transport_action_row_digest(spec: &TransportActionSpec) -> String {
    let mut material: Vec<&str> = vec![
        TRANSPORT_PROFILE_ID,
        spec.action_id.as_str(),
        spec.action,
        spec.operation_id,
        spec.route_family_id,
        spec.world_id,
        spec.morphism_row_id,
    ];
    material.extend(spec.required_morphisms.iter().copied());
    semantic_digest(&material)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportActionRegistryRow {
    pub action: String,
    pub action_id: String,
    pub operation_id: String,
    pub route_family_id: String,
    pub world_id: String,
    pub morphism_row_id: String,
    pub semantic_digest: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportCheckIssue {
    pub failure_class: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportCheckReport {
    pub schema: u32,
    pub check_kind: String,
    pub registry_kind: String,
    pub profile_id: String,
    pub result: String,
    pub failure_classes: Vec<String>,
    pub issues: Vec<TransportCheckIssue>,
    pub action_count: usize,
    pub actions: Vec<TransportActionRegistryRow>,
    pub semantic_digest: String,
}

fn transport_action_row(spec: &TransportActionSpec) -> TransportActionRegistryRow {
    TransportActionRegistryRow {
        action: spec.action.to_string(),
        action_id: spec.action_id.as_str().to_string(),
        operation_id: spec.operation_id.to_string(),
        route_family_id: spec.route_family_id.to_string(),
        world_id: spec.world_id.to_string(),
        morphism_row_id: spec.morphism_row_id.to_string(),
        semantic_digest: transport_action_row_digest(spec),
    }
}

pub fn transport_action_registry_rows() -> Vec<TransportActionRegistryRow> {
    TRANSPORT_ACTION_SPECS
        .iter()
        .map(transport_action_row)
        .collect()
}

fn transport_check_digest(
    result: &str,
    failure_classes: &[String],
    actions: &[TransportActionRegistryRow],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(TRANSPORT_CHECK_KIND.as_bytes());
    hasher.update([0u8]);
    hasher.update(TRANSPORT_PROFILE_ID.as_bytes());
    hasher.update([0u8]);
    hasher.update(result.as_bytes());
    hasher.update([0u8]);
    for class in failure_classes {
        hasher.update(class.as_bytes());
        hasher.update([0u8]);
    }
    for action in actions {
        hasher.update(action.action.as_bytes());
        hasher.update([0u8]);
        hasher.update(action.action_id.as_bytes());
        hasher.update([0u8]);
        hasher.update(action.semantic_digest.as_bytes());
        hasher.update([0u8]);
    }
    format!("{TRANSPORT_SEMANTIC_DIGEST_PREFIX}{:x}", hasher.finalize())
}

fn validate_transport_registry(actions: &[TransportActionRegistryRow]) -> Vec<TransportCheckIssue> {
    let mut issues: Vec<TransportCheckIssue> = Vec::new();
    let mut seen_actions: BTreeSet<String> = BTreeSet::new();
    let mut seen_action_ids: BTreeSet<String> = BTreeSet::new();
    let expected_actions: BTreeSet<String> = TRANSPORT_ACTION_SPECS
        .iter()
        .map(|spec| spec.action.to_string())
        .collect();
    let mut present_actions: BTreeSet<String> = BTreeSet::new();
    let spec_rows: BTreeMap<String, TransportActionRegistryRow> = TRANSPORT_ACTION_SPECS
        .iter()
        .map(|spec| (spec.action.to_string(), transport_action_row(spec)))
        .collect();

    for row in actions {
        present_actions.insert(row.action.clone());

        if row.action.trim().is_empty()
            || row.action_id.trim().is_empty()
            || row.operation_id.trim().is_empty()
            || row.route_family_id.trim().is_empty()
            || row.world_id.trim().is_empty()
            || row.morphism_row_id.trim().is_empty()
            || row.semantic_digest.trim().is_empty()
        {
            issues.push(TransportCheckIssue {
                failure_class: FAILURE_TRANSPORT_REGISTRY_EMPTY_FIELD.to_string(),
                path: format!("actions/{}", row.action),
                message: "action row must provide non-empty typed fields".to_string(),
            });
        }

        if !seen_actions.insert(row.action.clone()) {
            issues.push(TransportCheckIssue {
                failure_class: FAILURE_TRANSPORT_REGISTRY_DUPLICATE_ACTION.to_string(),
                path: format!("actions/{}", row.action),
                message: "duplicate action row".to_string(),
            });
        }
        if !seen_action_ids.insert(row.action_id.clone()) {
            issues.push(TransportCheckIssue {
                failure_class: FAILURE_TRANSPORT_REGISTRY_DUPLICATE_ACTION_ID.to_string(),
                path: format!("actions/{}", row.action_id),
                message: "duplicate actionId row".to_string(),
            });
        }

        if let Some(expected) = spec_rows.get(&row.action)
            && row.semantic_digest != expected.semantic_digest
        {
            issues.push(TransportCheckIssue {
                failure_class: FAILURE_TRANSPORT_REGISTRY_DIGEST_MISMATCH.to_string(),
                path: format!("actions/{}/semanticDigest", row.action),
                message: format!(
                    "semanticDigest mismatch (expected={}, got={})",
                    expected.semantic_digest, row.semantic_digest
                ),
            });
        }
    }

    for action in expected_actions {
        if !present_actions.contains(&action) {
            issues.push(TransportCheckIssue {
                failure_class: FAILURE_TRANSPORT_REGISTRY_MISSING_ACTION.to_string(),
                path: "actions".to_string(),
                message: format!("missing required action row: {action}"),
            });
        }
    }

    issues.sort_by(|a, b| {
        (&a.failure_class, &a.path, &a.message).cmp(&(&b.failure_class, &b.path, &b.message))
    });
    issues
}

#[derive(Debug, Clone)]
struct TransportKernelBindingError {
    failure_class: String,
    diagnostic: String,
}

fn resolve_site_for_spec(
    spec: &TransportActionSpec,
) -> Result<premath_kernel::SiteResolveResponse, TransportKernelBindingError> {
    let request = SiteResolveRequest {
        schema: 1,
        request_kind: "premath.site_resolve.request.v1".to_string(),
        operation_id: spec.operation_id.to_string(),
        route_family_hint: Some(spec.route_family_id.to_string()),
        claimed_capabilities: Vec::new(),
        policy_digest: "pol1_transport_dispatch".to_string(),
        profile_id: "cp.bundle.v0".to_string(),
        context_ref: "transport.dispatch".to_string(),
    };
    let site_input: Value = serde_json::from_str(DOCTRINE_SITE_INPUT_JSON).map_err(|source| {
        TransportKernelBindingError {
            failure_class: FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE.to_string(),
            diagnostic: format!("failed to parse DOCTRINE-SITE-INPUT: {source}"),
        }
    })?;
    let site: Value =
        serde_json::from_str(DOCTRINE_SITE_JSON).map_err(|source| TransportKernelBindingError {
            failure_class: FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE.to_string(),
            diagnostic: format!("failed to parse DOCTRINE-SITE: {source}"),
        })?;
    let operation_registry: Value =
        serde_json::from_str(DOCTRINE_OP_REGISTRY_JSON).map_err(|source| {
            TransportKernelBindingError {
                failure_class: FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE.to_string(),
                diagnostic: format!("failed to parse DOCTRINE-OP-REGISTRY: {source}"),
            }
        })?;
    let control_plane_contract: Value =
        serde_json::from_str(CONTROL_PLANE_CONTRACT_JSON).map_err(|source| {
            TransportKernelBindingError {
                failure_class: FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE.to_string(),
                diagnostic: format!("failed to parse CONTROL-PLANE-CONTRACT: {source}"),
            }
        })?;
    let capability_registry: Value =
        serde_json::from_str(CAPABILITY_REGISTRY_JSON).map_err(|source| {
            TransportKernelBindingError {
                failure_class: FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE.to_string(),
                diagnostic: format!("failed to parse CAPABILITY-REGISTRY: {source}"),
            }
        })?;
    Ok(resolve_site_request(
        &request,
        &site_input,
        &site,
        &operation_registry,
        &control_plane_contract,
        &capability_registry,
    ))
}

fn validate_transport_action_binding_with_kernel(
    spec: &TransportActionSpec,
) -> Result<(), TransportKernelBindingError> {
    let site_input: Value = serde_json::from_str(DOCTRINE_SITE_INPUT_JSON).map_err(|source| {
        TransportKernelBindingError {
            failure_class: FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE.to_string(),
            diagnostic: format!("failed to parse DOCTRINE-SITE-INPUT: {source}"),
        }
    })?;
    let operation_registry: Value =
        serde_json::from_str(DOCTRINE_OP_REGISTRY_JSON).map_err(|source| {
            TransportKernelBindingError {
                failure_class: FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE.to_string(),
                diagnostic: format!("failed to parse DOCTRINE-OP-REGISTRY: {source}"),
            }
        })?;
    let operations = parse_operation_route_rows(&operation_registry).map_err(|source| {
        TransportKernelBindingError {
            failure_class: FAILURE_TRANSPORT_KERNEL_CONTRACT_UNAVAILABLE.to_string(),
            diagnostic: format!("failed to parse operation rows: {source}"),
        }
    })?;
    let required_families = vec![spec.route_family_id.to_string()];
    let required_bindings = vec![RequiredRouteBinding {
        route_family_id: spec.route_family_id.to_string(),
        operation_ids: vec![spec.operation_id.to_string()],
    }];
    let report = validate_world_route_bindings_with_requirements(
        &site_input,
        &operations,
        &required_families,
        &required_bindings,
    );
    if report.result == "accepted" {
        let resolver = resolve_site_for_spec(spec)?;
        if resolver.result != "accepted" {
            return Err(TransportKernelBindingError {
                failure_class: resolver
                    .failure_classes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| world_failure_class::WORLD_ROUTE_UNBOUND.to_string()),
                diagnostic: format!(
                    "site-resolve rejected action={} operation={} failures={:?}",
                    spec.action, spec.operation_id, resolver.failure_classes
                ),
            });
        }
        let Some(selected) = resolver.selected else {
            return Err(TransportKernelBindingError {
                failure_class: world_failure_class::WORLD_ROUTE_UNBOUND.to_string(),
                diagnostic: format!(
                    "site-resolve missing selected binding for action={} operation={}",
                    spec.action, spec.operation_id
                ),
            });
        };
        let binding_matches = selected.operation_id == spec.operation_id
            && selected.route_family_id == spec.route_family_id
            && selected.world_id == spec.world_id
            && selected.morphism_row_id == spec.morphism_row_id;
        if !binding_matches {
            return Err(TransportKernelBindingError {
                failure_class: world_failure_class::WORLD_ROUTE_UNBOUND.to_string(),
                diagnostic: format!(
                    "site-resolve binding drift for action={} expected=({}, {}, {}, {}) got=({}, {}, {}, {})",
                    spec.action,
                    spec.operation_id,
                    spec.route_family_id,
                    spec.world_id,
                    spec.morphism_row_id,
                    selected.operation_id,
                    selected.route_family_id,
                    selected.world_id,
                    selected.morphism_row_id
                ),
            });
        }
        return Ok(());
    }
    let failure_class = report
        .failure_classes
        .first()
        .cloned()
        .unwrap_or_else(|| world_failure_class::WORLD_ROUTE_UNBOUND.to_string());
    let diagnostic = report.issues.first().map_or_else(
        || {
            format!(
                "kernel route validation rejected action={} operation={} route={}",
                spec.action, spec.operation_id, spec.route_family_id
            )
        },
        |issue| {
            format!(
                "kernel route validation rejected action={} at {}: {}",
                spec.action, issue.path, issue.message
            )
        },
    );
    Err(TransportKernelBindingError {
        failure_class,
        diagnostic,
    })
}

pub fn transport_check() -> TransportCheckReport {
    let actions = transport_action_registry_rows();
    let mut issues = validate_transport_registry(&actions);
    for spec in TRANSPORT_ACTION_SPECS {
        if let Err(err) = validate_transport_action_binding_with_kernel(&spec) {
            issues.push(TransportCheckIssue {
                failure_class: err.failure_class,
                path: format!("actions/{}/kernelBinding", spec.action),
                message: err.diagnostic,
            });
        }
    }
    issues.sort_by(|a, b| {
        (&a.failure_class, &a.path, &a.message).cmp(&(&b.failure_class, &b.path, &b.message))
    });
    let mut failure_classes: Vec<String> = issues
        .iter()
        .map(|issue| issue.failure_class.clone())
        .collect();
    failure_classes.sort();
    failure_classes.dedup();
    let result = if issues.is_empty() {
        "accepted".to_string()
    } else {
        "rejected".to_string()
    };
    let semantic_digest = transport_check_digest(&result, &failure_classes, &actions);

    TransportCheckReport {
        schema: 1,
        check_kind: TRANSPORT_CHECK_KIND.to_string(),
        registry_kind: TRANSPORT_ACTION_REGISTRY_KIND.to_string(),
        profile_id: TRANSPORT_PROFILE_ID.to_string(),
        result,
        failure_classes,
        issues,
        action_count: actions.len(),
        actions,
        semantic_digest,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaseInfo {
    pub lease_id: String,
    pub owner: String,
    pub acquired_at: String,
    pub expires_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewed_at: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: i32,
    pub issue_type: String,
    pub assignee: String,
    pub owner: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease: Option<LeaseInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaseProjection {
    pub checked_at: String,
    pub stale_count: usize,
    pub stale_issue_ids: Vec<String>,
    pub contended_count: usize,
    pub contended_issue_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaseActionEnvelope {
    pub schema: u32,
    pub action: String,
    pub result: String,
    pub failure_classes: Vec<String>,
    pub issues_path: String,
    pub world_binding: WorldRouteBinding,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolver_witness_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolver_witness: Option<SiteResolveWitness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<IssueSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_projection: Option<LeaseProjection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone)]
struct LeaseMutationError {
    failure_class: String,
    diagnostic: String,
}

impl LeaseMutationError {
    fn new(failure_class: &str, diagnostic: impl Into<String>) -> Self {
        Self {
            failure_class: failure_class.to_string(),
            diagnostic: diagnostic.into(),
        }
    }
}

fn accepted_envelope(
    kind: LeaseActionKind,
    issues_path: String,
    issue: IssueSummary,
    changed: bool,
    lease_projection: LeaseProjection,
) -> LeaseActionEnvelope {
    accepted_envelope_optional(kind, issues_path, Some(issue), changed, lease_projection)
}

fn accepted_envelope_optional(
    kind: LeaseActionKind,
    issues_path: String,
    issue: Option<IssueSummary>,
    changed: bool,
    lease_projection: LeaseProjection,
) -> LeaseActionEnvelope {
    let (resolver_witness_ref, resolver_witness) =
        resolver_fields_for_action(kind.transport_action_id());
    LeaseActionEnvelope {
        schema: 1,
        action: kind.action().to_string(),
        result: "accepted".to_string(),
        failure_classes: Vec::new(),
        issues_path,
        world_binding: world_binding(kind),
        resolver_witness_ref,
        resolver_witness,
        changed: Some(changed),
        issue,
        lease_projection: Some(lease_projection),
        diagnostic: None,
    }
}

fn rejected_envelope(
    kind: LeaseActionKind,
    issues_path: String,
    failure_class: impl Into<String>,
    diagnostic: impl Into<String>,
) -> LeaseActionEnvelope {
    let (resolver_witness_ref, resolver_witness) =
        resolver_fields_for_action(kind.transport_action_id());
    LeaseActionEnvelope {
        schema: 1,
        action: kind.action().to_string(),
        result: "rejected".to_string(),
        failure_classes: vec![failure_class.into()],
        issues_path,
        world_binding: world_binding(kind),
        resolver_witness_ref,
        resolver_witness,
        changed: None,
        issue: None,
        lease_projection: None,
        diagnostic: Some(diagnostic.into()),
    }
}

fn map_atomic_store_error(err: AtomicStoreMutationError<LeaseMutationError>) -> LeaseMutationError {
    match err {
        AtomicStoreMutationError::Mutation(inner) => inner,
        AtomicStoreMutationError::LockBusy { lock_path } => LeaseMutationError::new(
            FAILURE_LEASE_MUTATION_LOCK_BUSY,
            format!("issue-memory lock busy: {lock_path}"),
        ),
        AtomicStoreMutationError::LockIo { lock_path, message } => LeaseMutationError::new(
            FAILURE_LEASE_MUTATION_LOCK_IO,
            format!("failed to acquire issue-memory lock {lock_path}: {message}"),
        ),
        AtomicStoreMutationError::Store(source) => {
            LeaseMutationError::new(FAILURE_LEASE_MUTATION_STORE_IO, source.to_string())
        }
    }
}

fn map_claim_next_atomic_store_error(
    err: AtomicStoreMutationError<Infallible>,
) -> LeaseMutationError {
    match err {
        AtomicStoreMutationError::Mutation(inner) => match inner {},
        AtomicStoreMutationError::LockBusy { lock_path } => LeaseMutationError::new(
            FAILURE_LEASE_MUTATION_LOCK_BUSY,
            format!("issue-memory lock busy: {lock_path}"),
        ),
        AtomicStoreMutationError::LockIo { lock_path, message } => LeaseMutationError::new(
            FAILURE_LEASE_MUTATION_LOCK_IO,
            format!("failed to acquire issue-memory lock {lock_path}: {message}"),
        ),
        AtomicStoreMutationError::Store(source) => {
            LeaseMutationError::new(FAILURE_LEASE_MUTATION_STORE_IO, source.to_string())
        }
    }
}

fn map_claim_next_error(err: ClaimNextError) -> LeaseMutationError {
    match err {
        ClaimNextError::InvalidAssignee => {
            LeaseMutationError::new(FAILURE_LEASE_INVALID_ASSIGNEE, "assignee is required")
        }
        ClaimNextError::InvalidLeaseTtl { actual } => LeaseMutationError::new(
            FAILURE_LEASE_INVALID_TTL,
            format!(
                "lease_ttl_seconds must be in range [{MIN_LEASE_TTL_SECONDS}, {MAX_LEASE_TTL_SECONDS}] (got {actual})"
            ),
        ),
        ClaimNextError::LeaseTtlOverflow => LeaseMutationError::new(
            FAILURE_LEASE_INVALID_TTL,
            "lease_ttl_seconds overflowed timestamp range",
        ),
        ClaimNextError::Atomic(inner) => map_claim_next_atomic_store_error(inner),
    }
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

fn resolve_issues_path(path: Option<String>) -> String {
    non_empty(path).unwrap_or_else(|| DEFAULT_ISSUES_PATH.to_string())
}

fn parse_lease_ttl_seconds(ttl_seconds: Option<i64>) -> Result<i64, LeaseMutationError> {
    let ttl = ttl_seconds.unwrap_or(DEFAULT_LEASE_TTL_SECONDS);
    if !(MIN_LEASE_TTL_SECONDS..=MAX_LEASE_TTL_SECONDS).contains(&ttl) {
        return Err(LeaseMutationError::new(
            FAILURE_LEASE_INVALID_TTL,
            format!(
                "lease_ttl_seconds must be in range [{MIN_LEASE_TTL_SECONDS}, {MAX_LEASE_TTL_SECONDS}]"
            ),
        ));
    }
    Ok(ttl)
}

fn parse_lease_expiry(
    lease_ttl_seconds: Option<i64>,
    lease_expires_at: Option<String>,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>, LeaseMutationError> {
    let expires_at_raw = non_empty(lease_expires_at);
    if lease_ttl_seconds.is_some() && expires_at_raw.is_some() {
        return Err(LeaseMutationError::new(
            FAILURE_LEASE_BINDING_AMBIGUOUS,
            "provide only one of leaseTtlSeconds or leaseExpiresAt",
        ));
    }

    if let Some(raw) = expires_at_raw {
        let parsed = DateTime::parse_from_rfc3339(&raw)
            .map_err(|_| {
                LeaseMutationError::new(
                    FAILURE_LEASE_INVALID_EXPIRES_AT,
                    "lease_expires_at must be RFC3339",
                )
            })?
            .with_timezone(&Utc);
        if parsed <= now {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_INVALID_EXPIRES_AT,
                "lease_expires_at must be in the future",
            ));
        }
        return Ok(parsed);
    }

    let ttl = parse_lease_ttl_seconds(lease_ttl_seconds)?;
    now.checked_add_signed(Duration::seconds(ttl))
        .ok_or_else(|| {
            LeaseMutationError::new(
                FAILURE_LEASE_INVALID_TTL,
                "lease_ttl_seconds overflowed timestamp range",
            )
        })
}

fn lease_token(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "anon".to_string()
    } else {
        trimmed.to_string()
    }
}

fn resolve_lease_id(raw_lease_id: Option<String>, issue_id: &str, assignee: &str) -> String {
    non_empty(raw_lease_id)
        .unwrap_or_else(|| format!("lease1_{}_{}", lease_token(issue_id), lease_token(assignee)))
}

fn lease_state_label(issue: &Issue, now: DateTime<Utc>) -> &'static str {
    match issue.lease_state_at(now) {
        IssueLeaseState::Unleased => "unleased",
        IssueLeaseState::Active => "active",
        IssueLeaseState::Stale => "stale",
    }
}

fn issue_is_lease_contended(issue: &Issue, now: DateTime<Utc>) -> bool {
    let Some(lease) = issue.lease.as_ref() else {
        return false;
    };
    if lease.expires_at <= now {
        return false;
    }
    issue.status != "in_progress" || issue.assignee != lease.owner
}

fn issue_summary(issue: &Issue, now: DateTime<Utc>) -> IssueSummary {
    let lease = issue.lease.as_ref().map(|lease| LeaseInfo {
        lease_id: lease.lease_id.clone(),
        owner: lease.owner.clone(),
        acquired_at: lease.acquired_at.to_rfc3339(),
        expires_at: lease.expires_at.to_rfc3339(),
        renewed_at: lease.renewed_at.map(|item| item.to_rfc3339()),
        state: lease_state_label(issue, now).to_string(),
    });

    IssueSummary {
        id: issue.id.clone(),
        title: issue.title.clone(),
        status: issue.status.clone(),
        priority: issue.priority,
        issue_type: issue.issue_type.clone(),
        assignee: issue.assignee.clone(),
        owner: issue.owner.clone(),
        lease,
    }
}

fn compute_lease_projection(store: &MemoryStore, now: DateTime<Utc>) -> LeaseProjection {
    let mut stale_issue_ids = Vec::new();
    let mut contended_issue_ids = Vec::new();

    for issue in store.issues() {
        match issue.lease_state_at(now) {
            IssueLeaseState::Stale => stale_issue_ids.push(issue.id.clone()),
            IssueLeaseState::Active if issue_is_lease_contended(issue, now) => {
                contended_issue_ids.push(issue.id.clone())
            }
            IssueLeaseState::Unleased | IssueLeaseState::Active => {}
        }
    }

    stale_issue_ids.sort();
    contended_issue_ids.sort();

    LeaseProjection {
        checked_at: now.to_rfc3339(),
        stale_count: stale_issue_ids.len(),
        stale_issue_ids,
        contended_count: contended_issue_ids.len(),
        contended_issue_ids,
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueClaimRequest {
    pub id: String,
    pub assignee: String,
    #[serde(default)]
    pub lease_id: Option<String>,
    #[serde(default)]
    pub lease_ttl_seconds: Option<i64>,
    #[serde(default)]
    pub lease_expires_at: Option<String>,
    #[serde(default)]
    pub issues_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueClaimNextRequest {
    pub assignee: String,
    #[serde(default)]
    pub lease_id: Option<String>,
    #[serde(default)]
    pub lease_ttl_seconds: Option<i64>,
    #[serde(default)]
    pub issues_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueLeaseRenewRequest {
    pub id: String,
    pub assignee: String,
    pub lease_id: String,
    #[serde(default)]
    pub lease_ttl_seconds: Option<i64>,
    #[serde(default)]
    pub lease_expires_at: Option<String>,
    #[serde(default)]
    pub issues_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueLeaseReleaseRequest {
    pub id: String,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub lease_id: Option<String>,
    #[serde(default)]
    pub issues_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportDispatchRequest {
    pub action: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportWorldBindingRequest {
    #[serde(alias = "action")]
    pub operation_action: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiberSpawnRequest {
    #[serde(default)]
    pub fiber_id: Option<String>,
    pub task_ref: String,
    #[serde(default)]
    pub parent_fiber_id: Option<String>,
    #[serde(default)]
    pub scope_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiberJoinRequest {
    pub fiber_id: String,
    pub join_set: Vec<String>,
    #[serde(default)]
    pub result_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiberCancelRequest {
    pub fiber_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

fn fiber_token(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "fiber".to_string()
    } else {
        trimmed.to_string()
    }
}

fn derive_fiber_id(task_ref: &str, parent_fiber_id: Option<&str>) -> String {
    let digest = semantic_digest(&[
        TRANSPORT_PROFILE_ID,
        "fiber.spawn",
        task_ref,
        parent_fiber_id.unwrap_or(""),
    ]);
    let suffix = digest
        .strip_prefix(TRANSPORT_SEMANTIC_DIGEST_PREFIX)
        .unwrap_or("");
    format!("fib1_{}", &suffix[..16])
}

fn fiber_witness_ref(action: &str, fiber_id: &str) -> String {
    let digest = semantic_digest(&[TRANSPORT_PROFILE_ID, action, fiber_id]);
    format!(
        "fiber://dispatch/{action}/{}/{}",
        fiber_token(fiber_id),
        digest
    )
}

fn fiber_rejected(action_id: TransportActionId, failure_class: &str, diagnostic: &str) -> Value {
    transport_rejected(
        transport_action_spec(action_id).action,
        action_id.as_str(),
        failure_class,
        diagnostic.to_string(),
    )
}

fn fiber_spawn_response(payload: Value) -> Value {
    let action_id = TransportActionId::FiberSpawn;
    let parsed = match serde_json::from_value::<FiberSpawnRequest>(payload) {
        Ok(value) => value,
        Err(source) => {
            return fiber_rejected(
                action_id,
                FAILURE_FIBER_INVALID_PAYLOAD,
                &format!("invalid fiber.spawn payload: {source}"),
            );
        }
    };

    let task_ref = parsed.task_ref.trim().to_string();
    if task_ref.is_empty() {
        return fiber_rejected(
            action_id,
            FAILURE_FIBER_MISSING_FIELD,
            "fiber.spawn requires taskRef",
        );
    }
    let parent_fiber_id = non_empty(parsed.parent_fiber_id);
    let scope_ref = non_empty(parsed.scope_ref);
    let fiber_id = non_empty(parsed.fiber_id)
        .unwrap_or_else(|| derive_fiber_id(&task_ref, parent_fiber_id.as_deref()));
    let witness_ref = fiber_witness_ref("fiber.spawn", &fiber_id);

    serde_json::json!({
        "schema": 1,
        "action": transport_action_spec(action_id).action,
        "result": "accepted",
        "failureClasses": [],
        "worldBinding": world_binding_for_action(action_id),
        "fiberId": fiber_id,
        "taskRef": task_ref,
        "parentFiberId": parent_fiber_id,
        "scopeRef": scope_ref,
        "fiberWitnessRef": witness_ref
    })
}

fn fiber_join_response(payload: Value) -> Value {
    let action_id = TransportActionId::FiberJoin;
    let parsed = match serde_json::from_value::<FiberJoinRequest>(payload) {
        Ok(value) => value,
        Err(source) => {
            return fiber_rejected(
                action_id,
                FAILURE_FIBER_INVALID_PAYLOAD,
                &format!("invalid fiber.join payload: {source}"),
            );
        }
    };

    let fiber_id = parsed.fiber_id.trim().to_string();
    if fiber_id.is_empty() {
        return fiber_rejected(
            action_id,
            FAILURE_FIBER_MISSING_FIELD,
            "fiber.join requires fiberId",
        );
    }
    let join_set = parsed
        .join_set
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if join_set.is_empty() {
        return fiber_rejected(
            action_id,
            FAILURE_FIBER_MISSING_FIELD,
            "fiber.join requires non-empty joinSet",
        );
    }
    let witness_ref = fiber_witness_ref("fiber.join", &fiber_id);

    serde_json::json!({
        "schema": 1,
        "action": transport_action_spec(action_id).action,
        "result": "accepted",
        "failureClasses": [],
        "worldBinding": world_binding_for_action(action_id),
        "fiberId": fiber_id,
        "joinSet": join_set,
        "resultRef": non_empty(parsed.result_ref),
        "fiberWitnessRef": witness_ref
    })
}

fn fiber_cancel_response(payload: Value) -> Value {
    let action_id = TransportActionId::FiberCancel;
    let parsed = match serde_json::from_value::<FiberCancelRequest>(payload) {
        Ok(value) => value,
        Err(source) => {
            return fiber_rejected(
                action_id,
                FAILURE_FIBER_INVALID_PAYLOAD,
                &format!("invalid fiber.cancel payload: {source}"),
            );
        }
    };

    let fiber_id = parsed.fiber_id.trim().to_string();
    if fiber_id.is_empty() {
        return fiber_rejected(
            action_id,
            FAILURE_FIBER_MISSING_FIELD,
            "fiber.cancel requires fiberId",
        );
    }
    let witness_ref = fiber_witness_ref("fiber.cancel", &fiber_id);

    serde_json::json!({
        "schema": 1,
        "action": transport_action_spec(action_id).action,
        "result": "accepted",
        "failureClasses": [],
        "worldBinding": world_binding_for_action(action_id),
        "fiberId": fiber_id,
        "reason": non_empty(parsed.reason),
        "fiberWitnessRef": witness_ref
    })
}

pub fn issue_claim_next(request: IssueClaimNextRequest) -> LeaseActionEnvelope {
    let kind = LeaseActionKind::ClaimNext;
    let issues_path = resolve_issues_path(request.issues_path.clone());
    if let Err(err) = validate_transport_action_binding_with_kernel(transport_action_spec(
        kind.transport_action_id(),
    )) {
        return rejected_envelope(kind, issues_path, err.failure_class, err.diagnostic);
    }

    let assignee = request.assignee.trim().to_string();
    if assignee.is_empty() {
        return rejected_envelope(
            kind,
            issues_path,
            FAILURE_LEASE_INVALID_ASSIGNEE,
            "assignee is required",
        );
    }

    let now = Utc::now();
    let outcome = match claim_next_issue_jsonl(
        &issues_path,
        ClaimNextRequest {
            assignee,
            lease_id: request.lease_id.clone(),
            lease_ttl_seconds: request.lease_ttl_seconds,
            now,
        },
    ) {
        Ok(value) => value,
        Err(err) => {
            let mapped = map_claim_next_error(err);
            return rejected_envelope(kind, issues_path, mapped.failure_class, mapped.diagnostic);
        }
    };

    let store = match MemoryStore::load_jsonl(&issues_path) {
        Ok(store) => store,
        Err(source) => {
            return rejected_envelope(
                kind,
                issues_path,
                FAILURE_LEASE_MUTATION_STORE_IO,
                source.to_string(),
            );
        }
    };
    let issue = outcome.issue.as_ref().map(|item| issue_summary(item, now));
    accepted_envelope_optional(
        kind,
        issues_path,
        issue,
        outcome.issue.is_some(),
        compute_lease_projection(&store, now),
    )
}

pub fn issue_claim(request: IssueClaimRequest) -> LeaseActionEnvelope {
    let kind = LeaseActionKind::Claim;
    let issues_path = resolve_issues_path(request.issues_path.clone());
    if let Err(err) = validate_transport_action_binding_with_kernel(transport_action_spec(
        kind.transport_action_id(),
    )) {
        return rejected_envelope(kind, issues_path, err.failure_class, err.diagnostic);
    }
    let assignee = request.assignee.trim().to_string();
    if assignee.is_empty() {
        return rejected_envelope(
            kind,
            issues_path,
            FAILURE_LEASE_INVALID_ASSIGNEE,
            "assignee is required",
        );
    }

    let now = Utc::now();
    let lease_expires_at = match parse_lease_expiry(
        request.lease_ttl_seconds,
        request.lease_expires_at.clone(),
        now,
    ) {
        Ok(value) => value,
        Err(err) => {
            return rejected_envelope(kind, issues_path, err.failure_class, err.diagnostic);
        }
    };
    let requested_lease_id = request.lease_id.clone();

    let mutation = mutate_store_jsonl(&issues_path, |store| {
        let issue = store.issue_mut(&request.id).ok_or_else(|| {
            LeaseMutationError::new(
                FAILURE_LEASE_NOT_FOUND,
                format!("issue not found: {}", request.id),
            )
        })?;

        if issue.status == "closed" {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_CLOSED,
                format!("cannot claim closed issue: {}", request.id),
            ));
        }

        let mut changed = false;
        let mut status_changed = false;

        if issue.lease_state_at(now) == IssueLeaseState::Stale {
            issue.lease = None;
            changed = true;

            if issue.status == "in_progress" {
                issue.set_status("open".to_string());
                status_changed = true;
            }

            if !issue.assignee.is_empty() && issue.assignee != assignee {
                issue.assignee.clear();
                changed = true;
            }
        }

        if let Some(active_lease) = issue.lease.as_ref().filter(|lease| lease.expires_at > now)
            && active_lease.owner != assignee
        {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_CONTENTION_ACTIVE,
                format!(
                    "issue already leased: {} (owner={}, lease_id={})",
                    request.id, active_lease.owner, active_lease.lease_id
                ),
            ));
        }

        if issue.lease.is_none() && !issue.assignee.is_empty() && issue.assignee != assignee {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_CONTENTION_ACTIVE,
                format!(
                    "issue already claimed: {} (assignee={})",
                    request.id, issue.assignee
                ),
            ));
        }

        if issue.assignee != assignee {
            issue.assignee = assignee.clone();
            changed = true;
        }
        if issue.status != "in_progress" {
            issue.set_status("in_progress".to_string());
            changed = true;
            status_changed = true;
        }

        let lease_id = issue
            .lease
            .as_ref()
            .filter(|existing| existing.expires_at > now && existing.owner == assignee)
            .map(|existing| existing.lease_id.clone())
            .unwrap_or_else(|| {
                resolve_lease_id(requested_lease_id.clone(), &request.id, &assignee)
            });

        let next_lease = match issue.lease.as_ref() {
            Some(existing) if existing.owner == assignee && existing.lease_id == lease_id => {
                IssueLease {
                    lease_id: lease_id.clone(),
                    owner: assignee.clone(),
                    acquired_at: existing.acquired_at,
                    expires_at: lease_expires_at,
                    renewed_at: Some(now),
                }
            }
            _ => IssueLease {
                lease_id: lease_id.clone(),
                owner: assignee.clone(),
                acquired_at: now,
                expires_at: lease_expires_at,
                renewed_at: None,
            },
        };

        if issue.lease.as_ref() != Some(&next_lease) {
            issue.lease = Some(next_lease);
            changed = true;
        }

        if changed && !status_changed {
            issue.touch_updated_at();
        }

        let updated = issue.clone();
        Ok(((updated, changed, store.clone()), changed))
    });

    match mutation {
        Ok((updated, changed, store)) => accepted_envelope(
            kind,
            issues_path,
            issue_summary(&updated, now),
            changed,
            compute_lease_projection(&store, now),
        ),
        Err(err) => {
            let mapped = map_atomic_store_error(err);
            rejected_envelope(kind, issues_path, mapped.failure_class, mapped.diagnostic)
        }
    }
}

pub fn issue_lease_renew(request: IssueLeaseRenewRequest) -> LeaseActionEnvelope {
    let kind = LeaseActionKind::Renew;
    let issues_path = resolve_issues_path(request.issues_path.clone());
    if let Err(err) = validate_transport_action_binding_with_kernel(transport_action_spec(
        kind.transport_action_id(),
    )) {
        return rejected_envelope(kind, issues_path, err.failure_class, err.diagnostic);
    }
    let assignee = request.assignee.trim().to_string();
    if assignee.is_empty() {
        return rejected_envelope(
            kind,
            issues_path,
            FAILURE_LEASE_INVALID_ASSIGNEE,
            "assignee is required",
        );
    }
    let lease_id = request.lease_id.trim().to_string();
    if lease_id.is_empty() {
        return rejected_envelope(
            kind,
            issues_path,
            FAILURE_LEASE_ID_MISMATCH,
            "lease_id is required",
        );
    }

    let now = Utc::now();
    let lease_expires_at = match parse_lease_expiry(
        request.lease_ttl_seconds,
        request.lease_expires_at.clone(),
        now,
    ) {
        Ok(value) => value,
        Err(err) => {
            return rejected_envelope(kind, issues_path, err.failure_class, err.diagnostic);
        }
    };

    let mutation = mutate_store_jsonl(&issues_path, |store| {
        let issue = store.issue_mut(&request.id).ok_or_else(|| {
            LeaseMutationError::new(
                FAILURE_LEASE_NOT_FOUND,
                format!("issue not found: {}", request.id),
            )
        })?;

        if issue.status == "closed" {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_CLOSED,
                format!("cannot renew lease on closed issue: {}", request.id),
            ));
        }

        let current = issue.lease.clone().ok_or_else(|| {
            LeaseMutationError::new(
                FAILURE_LEASE_MISSING,
                format!("issue has no lease: {}", request.id),
            )
        })?;

        if current.expires_at <= now {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_STALE,
                format!("lease is stale and must be reclaimed: {}", request.id),
            ));
        }
        if current.owner != assignee {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_OWNER_MISMATCH,
                format!(
                    "lease owner mismatch for {} (expected={}, got={})",
                    request.id, current.owner, assignee
                ),
            ));
        }
        if current.lease_id != lease_id {
            return Err(LeaseMutationError::new(
                FAILURE_LEASE_ID_MISMATCH,
                format!(
                    "lease_id mismatch for {} (expected={}, got={})",
                    request.id, current.lease_id, lease_id
                ),
            ));
        }

        let mut changed = false;
        let mut status_changed = false;
        if issue.assignee != assignee {
            issue.assignee = assignee.clone();
            changed = true;
        }
        if issue.status != "in_progress" {
            issue.set_status("in_progress".to_string());
            changed = true;
            status_changed = true;
        }

        let renewed = IssueLease {
            lease_id,
            owner: assignee,
            acquired_at: current.acquired_at,
            expires_at: lease_expires_at,
            renewed_at: Some(now),
        };
        if issue.lease.as_ref() != Some(&renewed) {
            issue.lease = Some(renewed);
            changed = true;
        }

        if changed && !status_changed {
            issue.touch_updated_at();
        }

        let updated = issue.clone();
        Ok(((updated, changed, store.clone()), changed))
    });

    match mutation {
        Ok((updated, changed, store)) => accepted_envelope(
            kind,
            issues_path,
            issue_summary(&updated, now),
            changed,
            compute_lease_projection(&store, now),
        ),
        Err(err) => {
            let mapped = map_atomic_store_error(err);
            rejected_envelope(kind, issues_path, mapped.failure_class, mapped.diagnostic)
        }
    }
}

pub fn issue_lease_release(request: IssueLeaseReleaseRequest) -> LeaseActionEnvelope {
    let kind = LeaseActionKind::Release;
    let issues_path = resolve_issues_path(request.issues_path.clone());
    if let Err(err) = validate_transport_action_binding_with_kernel(transport_action_spec(
        kind.transport_action_id(),
    )) {
        return rejected_envelope(kind, issues_path, err.failure_class, err.diagnostic);
    }
    let expected_assignee = non_empty(request.assignee.clone());
    let expected_lease_id = non_empty(request.lease_id.clone());
    let now = Utc::now();

    let mutation = mutate_store_jsonl(&issues_path, |store| {
        let issue = store.issue_mut(&request.id).ok_or_else(|| {
            LeaseMutationError::new(
                FAILURE_LEASE_NOT_FOUND,
                format!("issue not found: {}", request.id),
            )
        })?;

        let mut changed = false;
        let mut status_changed = false;

        match issue.lease.as_ref() {
            None => {
                if expected_assignee.is_some() || expected_lease_id.is_some() {
                    return Err(LeaseMutationError::new(
                        FAILURE_LEASE_MISSING,
                        format!("issue has no lease: {}", request.id),
                    ));
                }
            }
            Some(current) => {
                if let Some(expected) = expected_assignee.as_ref()
                    && current.owner != *expected
                {
                    return Err(LeaseMutationError::new(
                        FAILURE_LEASE_OWNER_MISMATCH,
                        format!(
                            "lease owner mismatch for {} (expected={}, got={})",
                            request.id, current.owner, expected
                        ),
                    ));
                }
                if let Some(expected) = expected_lease_id.as_ref()
                    && current.lease_id != *expected
                {
                    return Err(LeaseMutationError::new(
                        FAILURE_LEASE_ID_MISMATCH,
                        format!(
                            "lease_id mismatch for {} (expected={}, got={})",
                            request.id, current.lease_id, expected
                        ),
                    ));
                }
                issue.lease = None;
                changed = true;
            }
        }

        if changed {
            if !issue.assignee.is_empty() {
                issue.assignee.clear();
            }
            if issue.status == "in_progress" {
                issue.set_status("open".to_string());
                status_changed = true;
            }
            if !status_changed {
                issue.touch_updated_at();
            }
        }

        let updated = issue.clone();
        Ok(((updated, changed, store.clone()), changed))
    });

    match mutation {
        Ok((updated, changed, store)) => accepted_envelope(
            kind,
            issues_path,
            issue_summary(&updated, now),
            changed,
            compute_lease_projection(&store, now),
        ),
        Err(err) => {
            let mapped = map_atomic_store_error(err);
            rejected_envelope(kind, issues_path, mapped.failure_class, mapped.diagnostic)
        }
    }
}

pub fn issue_claim_json(payload_json: &str) -> String {
    let parsed = serde_json::from_str::<IssueClaimRequest>(payload_json);
    let envelope = match parsed {
        Ok(request) => issue_claim(request),
        Err(source) => rejected_envelope(
            LeaseActionKind::Claim,
            DEFAULT_ISSUES_PATH.to_string(),
            FAILURE_LEASE_INVALID_PAYLOAD,
            format!("invalid claim payload: {source}"),
        ),
    };
    serde_json::to_string(&envelope).expect("lease claim envelope should serialize")
}

pub fn issue_claim_next_json(payload_json: &str) -> String {
    let parsed = serde_json::from_str::<IssueClaimNextRequest>(payload_json);
    let envelope = match parsed {
        Ok(request) => issue_claim_next(request),
        Err(source) => rejected_envelope(
            LeaseActionKind::ClaimNext,
            DEFAULT_ISSUES_PATH.to_string(),
            FAILURE_LEASE_INVALID_PAYLOAD,
            format!("invalid claim-next payload: {source}"),
        ),
    };
    serde_json::to_string(&envelope).expect("lease claim-next envelope should serialize")
}

pub fn issue_lease_renew_json(payload_json: &str) -> String {
    let parsed = serde_json::from_str::<IssueLeaseRenewRequest>(payload_json);
    let envelope = match parsed {
        Ok(request) => issue_lease_renew(request),
        Err(source) => rejected_envelope(
            LeaseActionKind::Renew,
            DEFAULT_ISSUES_PATH.to_string(),
            FAILURE_LEASE_INVALID_PAYLOAD,
            format!("invalid renew payload: {source}"),
        ),
    };
    serde_json::to_string(&envelope).expect("lease renew envelope should serialize")
}

pub fn issue_lease_release_json(payload_json: &str) -> String {
    let parsed = serde_json::from_str::<IssueLeaseReleaseRequest>(payload_json);
    let envelope = match parsed {
        Ok(request) => issue_lease_release(request),
        Err(source) => rejected_envelope(
            LeaseActionKind::Release,
            DEFAULT_ISSUES_PATH.to_string(),
            FAILURE_LEASE_INVALID_PAYLOAD,
            format!("invalid release payload: {source}"),
        ),
    };
    serde_json::to_string(&envelope).expect("lease release envelope should serialize")
}

fn transport_rejected(
    action: &str,
    action_id: &str,
    failure_class: &str,
    diagnostic: impl Into<String>,
) -> Value {
    serde_json::json!({
        "schema": 1,
        "dispatchKind": TRANSPORT_DISPATCH_KIND,
        "profileId": TRANSPORT_PROFILE_ID,
        "result": "rejected",
        "action": action,
        "actionId": action_id,
        "semanticDigest": transport_dispatch_digest(action, action_id),
        "failureClasses": [failure_class],
        "diagnostic": diagnostic.into(),
    })
}

fn annotate_transport_dispatch_fields(
    envelope: &mut Value,
    action: &str,
    action_id: TransportActionId,
) {
    let Some(obj) = envelope.as_object_mut() else {
        return;
    };
    let action_id_text = action_id.as_str();
    obj.insert(
        "dispatchKind".to_string(),
        Value::String(TRANSPORT_DISPATCH_KIND.to_string()),
    );
    obj.insert(
        "profileId".to_string(),
        Value::String(TRANSPORT_PROFILE_ID.to_string()),
    );
    obj.insert(
        "actionId".to_string(),
        Value::String(action_id_text.to_string()),
    );
    obj.insert(
        "semanticDigest".to_string(),
        Value::String(transport_dispatch_digest(action, action_id_text)),
    );
    if !obj.contains_key("resolverWitnessRef") || !obj.contains_key("resolverWitness") {
        let (resolver_witness_ref, resolver_witness) = resolver_fields_for_action(action_id);
        if let Some(value) = resolver_witness_ref {
            obj.insert("resolverWitnessRef".to_string(), Value::String(value));
        }
        if let Some(witness) = resolver_witness {
            obj.insert(
                "resolverWitness".to_string(),
                serde_json::to_value(witness).unwrap_or(Value::Null),
            );
        }
    }
}

pub fn world_route_binding_json(action: &str) -> String {
    let envelope = match TransportActionId::from_action(action) {
        Some(action_id) => {
            let spec = transport_action_spec(action_id);
            match validate_transport_action_binding_with_kernel(spec) {
                Ok(()) => serde_json::json!({
                    "schema": 1,
                    "result": "accepted",
                    "action": spec.action,
                    "binding": world_binding_for_action(action_id),
                }),
                Err(err) => serde_json::json!({
                    "schema": 1,
                    "result": "rejected",
                    "failureClasses": [err.failure_class],
                    "action": spec.action,
                    "diagnostic": err.diagnostic,
                }),
            }
        }
        None => serde_json::json!({
            "schema": 1,
            "result": "rejected",
            "failureClasses": [FAILURE_LEASE_UNKNOWN_ACTION],
            "action": action,
        }),
    };
    serde_json::to_string(&envelope).expect("world binding envelope should serialize")
}

pub fn transport_dispatch_json(request_json: &str) -> String {
    let parsed = serde_json::from_str::<TransportDispatchRequest>(request_json);
    let response = match parsed {
        Ok(request) => dispatch_transport_request(request),
        Err(source) => transport_rejected(
            "transport.dispatch",
            ACTION_ID_TRANSPORT_INVALID_REQUEST,
            FAILURE_TRANSPORT_INVALID_REQUEST,
            format!("invalid transport request: {source}"),
        ),
    };
    serde_json::to_string(&response).expect("transport dispatch response should serialize")
}

#[cfg(any(feature = "rustler_nif", test))]
fn nif_dispatch_json(request_json: &str) -> String {
    transport_dispatch_json(request_json)
}

fn dispatch_transport_request(request: TransportDispatchRequest) -> Value {
    let action = request.action.trim().to_string();
    let Some(action_id) = TransportActionId::from_action(&action) else {
        return transport_rejected(
            &action,
            ACTION_ID_TRANSPORT_UNKNOWN,
            FAILURE_TRANSPORT_UNKNOWN_ACTION,
            format!("unsupported transport action: {action}"),
        );
    };
    let spec = transport_action_spec(action_id);
    if let Err(err) = validate_transport_action_binding_with_kernel(spec) {
        return transport_rejected(
            &action,
            action_id.as_str(),
            &err.failure_class,
            err.diagnostic,
        );
    }

    let mut response = dispatch_transport_action(action_id, request.payload);
    annotate_transport_dispatch_fields(&mut response, &action, action_id);
    response
}

fn dispatch_transport_action(action_id: TransportActionId, payload: Value) -> Value {
    match action_id {
        TransportActionId::IssueClaim => match serde_json::from_value::<IssueClaimRequest>(payload)
        {
            Ok(parsed) => serde_json::to_value(issue_claim(parsed))
                .expect("issue claim envelope should serialize"),
            Err(source) => transport_rejected(
                transport_action_spec(action_id).action,
                action_id.as_str(),
                FAILURE_LEASE_INVALID_PAYLOAD,
                format!("invalid claim payload: {source}"),
            ),
        },
        TransportActionId::IssueClaimNext => {
            match serde_json::from_value::<IssueClaimNextRequest>(payload) {
                Ok(parsed) => serde_json::to_value(issue_claim_next(parsed))
                    .expect("issue claim-next envelope should serialize"),
                Err(source) => transport_rejected(
                    transport_action_spec(action_id).action,
                    action_id.as_str(),
                    FAILURE_LEASE_INVALID_PAYLOAD,
                    format!("invalid claim-next payload: {source}"),
                ),
            }
        }
        TransportActionId::IssueLeaseRenew => {
            match serde_json::from_value::<IssueLeaseRenewRequest>(payload) {
                Ok(parsed) => serde_json::to_value(issue_lease_renew(parsed))
                    .expect("issue lease renew envelope should serialize"),
                Err(source) => transport_rejected(
                    transport_action_spec(action_id).action,
                    action_id.as_str(),
                    FAILURE_LEASE_INVALID_PAYLOAD,
                    format!("invalid renew payload: {source}"),
                ),
            }
        }
        TransportActionId::IssueLeaseRelease => {
            match serde_json::from_value::<IssueLeaseReleaseRequest>(payload) {
                Ok(parsed) => serde_json::to_value(issue_lease_release(parsed))
                    .expect("issue lease release envelope should serialize"),
                Err(source) => transport_rejected(
                    transport_action_spec(action_id).action,
                    action_id.as_str(),
                    FAILURE_LEASE_INVALID_PAYLOAD,
                    format!("invalid release payload: {source}"),
                ),
            }
        }
        TransportActionId::WorldRouteBinding => {
            match serde_json::from_value::<TransportWorldBindingRequest>(payload) {
                Ok(parsed) => {
                    let payload = world_route_binding_json(&parsed.operation_action);
                    let mut value = serde_json::from_str::<Value>(&payload)
                        .expect("world route binding payload should parse");
                    if let Some(obj) = value.as_object_mut() {
                        obj.insert(
                            "action".to_string(),
                            Value::String(transport_action_spec(action_id).action.to_string()),
                        );
                        obj.insert(
                            "operationAction".to_string(),
                            Value::String(parsed.operation_action),
                        );
                    }
                    value
                }
                Err(source) => transport_rejected(
                    transport_action_spec(action_id).action,
                    action_id.as_str(),
                    FAILURE_LEASE_INVALID_PAYLOAD,
                    format!("invalid world binding payload: {source}"),
                ),
            }
        }
        TransportActionId::FiberSpawn => fiber_spawn_response(payload),
        TransportActionId::FiberJoin => fiber_join_response(payload),
        TransportActionId::FiberCancel => fiber_cancel_response(payload),
    }
}

#[cfg(feature = "rustler_nif")]
mod nif {
    use super::nif_dispatch_json;

    #[rustler::nif(schedule = "DirtyIo")]
    fn dispatch(request_json: String) -> String {
        nif_dispatch_json(&request_json)
    }

    rustler::init!("Elixir.Premath.TransportNif");
}

#[cfg(test)]
mod tests {
    use super::{
        IssueClaimNextRequest, IssueClaimRequest, IssueLeaseReleaseRequest, IssueLeaseRenewRequest,
        issue_claim, issue_claim_next, issue_lease_release, issue_lease_renew,
        issue_lease_renew_json, nif_dispatch_json, transport_check, transport_dispatch_json,
        world_route_binding_json,
    };
    use premath_bd::{Issue, MemoryStore};
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_issues_path(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("premath-transport-{prefix}-{unique}"));
        fs::create_dir_all(&root).expect("temp dir should be created");
        root.join("issues.jsonl")
    }

    fn seed_open_issue(path: &PathBuf, id: &str) {
        let mut issue = Issue::new(id.to_string(), format!("Issue {id}"));
        issue.set_status("open".to_string());
        let mut store = MemoryStore::default();
        store.upsert_issue(issue);
        store.save_jsonl(path).expect("store should save");
    }

    #[test]
    fn claim_renew_release_roundtrip_is_accepted() {
        let path = temp_issues_path("roundtrip");
        seed_open_issue(&path, "bd-1");

        let claim = issue_claim(IssueClaimRequest {
            id: "bd-1".to_string(),
            assignee: "worker-a".to_string(),
            lease_id: None,
            lease_ttl_seconds: Some(3600),
            lease_expires_at: None,
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(claim.result, "accepted");
        assert_eq!(claim.action, "issue.claim");
        assert_eq!(claim.world_binding.world_id, "world.lease.v1");

        let lease_id = claim
            .issue
            .as_ref()
            .and_then(|item| item.lease.as_ref())
            .map(|item| item.lease_id.clone())
            .expect("claim should return lease");

        let renew = issue_lease_renew(IssueLeaseRenewRequest {
            id: "bd-1".to_string(),
            assignee: "worker-a".to_string(),
            lease_id: lease_id.clone(),
            lease_ttl_seconds: Some(3600),
            lease_expires_at: None,
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(renew.result, "accepted");
        assert_eq!(renew.action, "issue.lease_renew");

        let release = issue_lease_release(IssueLeaseReleaseRequest {
            id: "bd-1".to_string(),
            assignee: Some("worker-a".to_string()),
            lease_id: Some(lease_id),
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(release.result, "accepted");
        assert_eq!(release.action, "issue.lease_release");
        let issue = release.issue.expect("release should project issue");
        assert_eq!(issue.status, "open");
        assert!(issue.assignee.is_empty());
        assert!(issue.lease.is_none());
    }

    #[test]
    fn claim_next_accepts_and_returns_none_when_no_ready_issue() {
        let path = temp_issues_path("claim-next");
        seed_open_issue(&path, "bd-11");

        let first = issue_claim_next(IssueClaimNextRequest {
            assignee: "worker-a".to_string(),
            lease_id: None,
            lease_ttl_seconds: Some(3600),
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(first.result, "accepted");
        assert_eq!(first.action, "issue.claim_next");
        assert_eq!(
            first.issue.as_ref().map(|row| row.id.as_str()),
            Some("bd-11")
        );
        assert_eq!(first.changed, Some(true));

        let second = issue_claim_next(IssueClaimNextRequest {
            assignee: "worker-a".to_string(),
            lease_id: None,
            lease_ttl_seconds: Some(3600),
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(second.result, "accepted");
        assert_eq!(second.action, "issue.claim_next");
        assert!(second.issue.is_none());
        assert_eq!(second.changed, Some(false));
    }

    #[test]
    fn claim_rejects_empty_assignee() {
        let path = temp_issues_path("invalid-assignee");
        seed_open_issue(&path, "bd-2");

        let claim = issue_claim(IssueClaimRequest {
            id: "bd-2".to_string(),
            assignee: "   ".to_string(),
            lease_id: None,
            lease_ttl_seconds: Some(3600),
            lease_expires_at: None,
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(claim.result, "rejected");
        assert_eq!(
            claim.failure_classes,
            vec!["lease_invalid_assignee".to_string()]
        );
    }

    #[test]
    fn release_rejects_owner_mismatch() {
        let path = temp_issues_path("owner-mismatch");
        seed_open_issue(&path, "bd-3");

        let claim = issue_claim(IssueClaimRequest {
            id: "bd-3".to_string(),
            assignee: "worker-a".to_string(),
            lease_id: None,
            lease_ttl_seconds: Some(3600),
            lease_expires_at: None,
            issues_path: Some(path.display().to_string()),
        });
        let lease_id = claim
            .issue
            .as_ref()
            .and_then(|item| item.lease.as_ref())
            .map(|item| item.lease_id.clone())
            .expect("claim should return lease");

        let release = issue_lease_release(IssueLeaseReleaseRequest {
            id: "bd-3".to_string(),
            assignee: Some("worker-b".to_string()),
            lease_id: Some(lease_id),
            issues_path: Some(path.display().to_string()),
        });
        assert_eq!(release.result, "rejected");
        assert_eq!(
            release.failure_classes,
            vec!["lease_owner_mismatch".to_string()]
        );
    }

    #[test]
    fn json_wrapper_rejects_invalid_payload() {
        let payload = issue_lease_renew_json("{\"id\":1}");
        let parsed: Value = serde_json::from_str(&payload).expect("payload should parse");
        assert_eq!(parsed["result"], "rejected");
        assert_eq!(
            parsed["failureClasses"],
            serde_json::json!(["lease_invalid_payload"])
        );
    }

    #[test]
    fn world_binding_json_reports_known_and_unknown_actions() {
        let known = world_route_binding_json("issue.lease_renew");
        let known_value: Value = serde_json::from_str(&known).expect("known payload should parse");
        assert_eq!(known_value["result"], "accepted");
        assert_eq!(known_value["binding"]["worldId"], "world.lease.v1");
        assert_eq!(
            known_value["binding"]["routeFamilyId"],
            "route.issue_claim_lease"
        );

        let unknown = world_route_binding_json("issue.not_real");
        let unknown_value: Value =
            serde_json::from_str(&unknown).expect("unknown payload should parse");
        assert_eq!(unknown_value["result"], "rejected");
        assert_eq!(
            unknown_value["failureClasses"],
            serde_json::json!(["lease_unknown_action"])
        );
    }

    #[test]
    fn transport_dispatch_claim_accepts() {
        let path = temp_issues_path("dispatch-claim");
        seed_open_issue(&path, "bd-4");
        let request = serde_json::json!({
            "action": "issue.claim",
            "payload": {
                "id": "bd-4",
                "assignee": "worker-x",
                "leaseTtlSeconds": 3600,
                "issuesPath": path.display().to_string()
            }
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "accepted");
        assert_eq!(value["action"], "issue.claim");
        assert_eq!(value["worldBinding"]["worldId"], "world.lease.v1");
        assert_eq!(
            value["dispatchKind"],
            serde_json::json!("premath.transport_dispatch.v1")
        );
        assert_eq!(
            value["profileId"],
            serde_json::json!("transport.issue_lease.v1")
        );
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.issue_claim")
        );
        assert!(
            value["semanticDigest"]
                .as_str()
                .map(|digest| digest.starts_with("ts1_"))
                .unwrap_or(false)
        );
    }

    #[test]
    fn transport_dispatch_claim_next_accepts() {
        let path = temp_issues_path("dispatch-claim-next");
        seed_open_issue(&path, "bd-12");
        let request = serde_json::json!({
            "action": "issue.claim_next",
            "payload": {
                "assignee": "worker-y",
                "leaseTtlSeconds": 3600,
                "issuesPath": path.display().to_string()
            }
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "accepted");
        assert_eq!(value["action"], "issue.claim_next");
        assert_eq!(value["worldBinding"]["worldId"], "world.lease.v1");
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.issue_claim_next")
        );
        assert_eq!(value["issue"]["id"], "bd-12");
    }

    #[test]
    fn transport_dispatch_rejects_unknown_action() {
        let request = serde_json::json!({
            "action": "issue.not_supported",
            "payload": {}
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "rejected");
        assert_eq!(
            value["failureClasses"],
            serde_json::json!(["transport_unknown_action"])
        );
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.unknown")
        );
    }

    #[test]
    fn transport_dispatch_world_route_binding_accepts() {
        let request = serde_json::json!({
            "action": "world.route_binding",
            "payload": {
                "action": "issue.claim"
            }
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "accepted");
        assert_eq!(value["action"], "world.route_binding");
        assert_eq!(value["operationAction"], "issue.claim");
        assert_eq!(value["binding"]["worldId"], "world.lease.v1");
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.world_route_binding")
        );
    }

    #[test]
    fn transport_dispatch_fiber_spawn_accepts() {
        let request = serde_json::json!({
            "action": "fiber.spawn",
            "payload": {
                "fiberId": "fib-alpha",
                "taskRef": "task/check-coherence",
                "scopeRef": "scope/worktree-a"
            }
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "accepted");
        assert_eq!(value["action"], "fiber.spawn");
        assert_eq!(value["worldBinding"]["worldId"], "world.fiber.v1");
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.fiber_spawn")
        );
        assert_eq!(value["fiberId"], "fib-alpha");
        assert_eq!(value["taskRef"], "task/check-coherence");
        assert!(
            value["fiberWitnessRef"]
                .as_str()
                .map(|item| item.starts_with("fiber://dispatch/fiber.spawn/"))
                .unwrap_or(false)
        );
    }

    #[test]
    fn transport_dispatch_fiber_join_rejects_empty_join_set() {
        let request = serde_json::json!({
            "action": "fiber.join",
            "payload": {
                "fiberId": "fib-alpha",
                "joinSet": []
            }
        });
        let response = transport_dispatch_json(&request.to_string());
        let value: Value = serde_json::from_str(&response).expect("dispatch response should parse");
        assert_eq!(value["result"], "rejected");
        assert_eq!(
            value["failureClasses"],
            serde_json::json!(["fiber_missing_field"])
        );
        assert_eq!(
            value["actionId"],
            serde_json::json!("transport.action.fiber_join")
        );
    }

    #[test]
    fn transport_check_reports_typed_registry() {
        let report = transport_check();
        assert_eq!(report.schema, 1);
        assert_eq!(report.check_kind, "premath.transport_check.v1");
        assert_eq!(report.registry_kind, "premath.transport_action_registry.v1");
        assert_eq!(report.profile_id, "transport.issue_lease.v1");
        assert_eq!(report.result, "accepted");
        assert!(report.failure_classes.is_empty());
        assert_eq!(report.action_count, 8);
        assert!(
            report
                .actions
                .iter()
                .any(|row| row.action == "issue.lease_renew"
                    && row.action_id == "transport.action.issue_lease_renew")
        );
        assert!(
            report
                .actions
                .iter()
                .any(|row| row.action == "issue.claim_next"
                    && row.action_id == "transport.action.issue_claim_next")
        );
        assert!(
            report.actions.iter().any(|row| row.action == "fiber.spawn"
                && row.action_id == "transport.action.fiber_spawn")
        );
        assert!(report.semantic_digest.starts_with("ts1_"));
    }

    #[test]
    fn nif_dispatch_claim_next_matches_transport_dispatch_envelope_semantics() {
        let path_transport = temp_issues_path("nif-transport-claim-next");
        let path_nif = temp_issues_path("nif-dispatch-claim-next");
        seed_open_issue(&path_transport, "bd-31");
        seed_open_issue(&path_nif, "bd-31");

        let transport_request = serde_json::json!({
            "action": "issue.claim_next",
            "payload": {
                "assignee": "worker-nif",
                "leaseTtlSeconds": 3600,
                "issuesPath": path_transport.display().to_string()
            }
        });
        let nif_request = serde_json::json!({
            "action": "issue.claim_next",
            "payload": {
                "assignee": "worker-nif",
                "leaseTtlSeconds": 3600,
                "issuesPath": path_nif.display().to_string()
            }
        });

        let transport_value: Value =
            serde_json::from_str(&transport_dispatch_json(&transport_request.to_string()))
                .expect("transport dispatch response should parse");
        let nif_value: Value = serde_json::from_str(&nif_dispatch_json(&nif_request.to_string()))
            .expect("nif dispatch response should parse");

        assert_eq!(transport_value["result"], nif_value["result"]);
        assert_eq!(transport_value["action"], nif_value["action"]);
        assert_eq!(transport_value["actionId"], nif_value["actionId"]);
        assert_eq!(transport_value["dispatchKind"], nif_value["dispatchKind"]);
        assert_eq!(transport_value["profileId"], nif_value["profileId"]);
        assert_eq!(
            transport_value["failureClasses"],
            nif_value["failureClasses"]
        );
        assert_eq!(transport_value["worldBinding"], nif_value["worldBinding"]);
        assert_eq!(transport_value["issue"]["id"], nif_value["issue"]["id"]);
        assert_eq!(
            transport_value["issue"]["lease"]["leaseId"],
            nif_value["issue"]["lease"]["leaseId"]
        );
    }
}
