use chrono::{DateTime, Duration, Utc};
use premath_bd::{
    AtomicStoreMutationError, ClaimNextError, DEFAULT_LEASE_TTL_SECONDS, Issue, IssueLeaseState,
    MAX_LEASE_TTL_SECONDS, MIN_LEASE_TTL_SECONDS, MemoryStore,
};
use premath_kernel::{SiteResolveRequest, SiteResolveWitness, resolve_site_request};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::convert::Infallible;

use crate::*;

#[derive(Debug, Clone, Copy)]
pub(crate) enum LeaseActionKind {
    Claim,
    ClaimNext,
    Renew,
    Release,
}

impl LeaseActionKind {
    pub(crate) fn action(self) -> &'static str {
        match self {
            Self::Claim => "issue.claim",
            Self::ClaimNext => "issue.claim_next",
            Self::Renew => "issue.lease_renew",
            Self::Release => "issue.lease_release",
        }
    }

    pub(crate) fn transport_action_id(self) -> TransportActionId {
        match self {
            Self::Claim => TransportActionId::IssueClaim,
            Self::ClaimNext => TransportActionId::IssueClaimNext,
            Self::Renew => TransportActionId::IssueLeaseRenew,
            Self::Release => TransportActionId::IssueLeaseRelease,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransportActionId {
    IssueClaim,
    IssueClaimNext,
    IssueLeaseRenew,
    IssueLeaseRelease,
    WorldRouteBinding,
    FiberSpawn,
    FiberJoin,
    FiberCancel,
    InstructionRun,
    SiteApplyChange,
    SiteCurrentDigest,
    SiteBuildChange,
    SiteComposeChanges,
}

impl TransportActionId {
    pub(crate) fn from_action(value: &str) -> Option<Self> {
        let action = value.trim();
        TRANSPORT_ACTION_SPECS
            .iter()
            .find(|spec| spec.action == action)
            .map(|spec| spec.action_id)
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::IssueClaim => "transport.action.issue_claim",
            Self::IssueClaimNext => "transport.action.issue_claim_next",
            Self::IssueLeaseRenew => "transport.action.issue_lease_renew",
            Self::IssueLeaseRelease => "transport.action.issue_lease_release",
            Self::WorldRouteBinding => "transport.action.world_route_binding",
            Self::FiberSpawn => "transport.action.fiber_spawn",
            Self::FiberJoin => "transport.action.fiber_join",
            Self::FiberCancel => "transport.action.fiber_cancel",
            Self::InstructionRun => "transport.action.instruction_run",
            Self::SiteApplyChange => "transport.action.site_apply_change",
            Self::SiteCurrentDigest => "transport.action.site_current_digest",
            Self::SiteBuildChange => "transport.action.site_build_change",
            Self::SiteComposeChanges => "transport.action.site_compose_changes",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TransportActionSpec {
    pub(crate) action_id: TransportActionId,
    pub(crate) action: &'static str,
    pub(crate) operation_id: &'static str,
    pub(crate) route_family_id: &'static str,
    pub(crate) world_id: &'static str,
    pub(crate) morphism_row_id: &'static str,
    pub(crate) required_morphisms: &'static [&'static str],
    /// When false, this action is a read-only projection and does not require
    /// world route binding validation.
    pub(crate) route_bound: bool,
}

pub(crate) const REQUIRED_MORPHISMS_LEASE: &[&str] = &[
    "dm.identity",
    "dm.profile.execution",
    "dm.commitment.attest",
];
pub(crate) const REQUIRED_MORPHISMS_TRANSPORT: &[&str] = &["dm.identity", "dm.transport.world"];
pub(crate) const REQUIRED_MORPHISMS_FIBER: &[&str] =
    &["dm.identity", "dm.profile.execution", "dm.transport.world"];
pub(crate) const REQUIRED_MORPHISMS_INSTRUCTION: &[&str] = &[
    "dm.commitment.attest",
    "dm.identity",
    "dm.profile.execution",
];
pub(crate) const REQUIRED_MORPHISMS_SITE_CHANGE: &[&str] = &[
    "dm.identity",
    "dm.profile.execution",
    "dm.commitment.attest",
];

pub(crate) const TRANSPORT_ACTION_SPECS: [TransportActionSpec; 13] = [
    TransportActionSpec {
        action_id: TransportActionId::IssueClaim,
        action: "issue.claim",
        operation_id: "op/mcp.issue_claim",
        route_family_id: ROUTE_FAMILY_LEASE,
        world_id: WORLD_ID_LEASE,
        morphism_row_id: MORPHISM_ROW_LEASE,
        required_morphisms: REQUIRED_MORPHISMS_LEASE,
        route_bound: true,
    },
    TransportActionSpec {
        action_id: TransportActionId::IssueClaimNext,
        action: "issue.claim_next",
        operation_id: "op/transport.issue_claim_next",
        route_family_id: ROUTE_FAMILY_LEASE,
        world_id: WORLD_ID_LEASE,
        morphism_row_id: MORPHISM_ROW_LEASE,
        required_morphisms: REQUIRED_MORPHISMS_LEASE,
        route_bound: true,
    },
    TransportActionSpec {
        action_id: TransportActionId::IssueLeaseRenew,
        action: "issue.lease_renew",
        operation_id: "op/mcp.issue_lease_renew",
        route_family_id: ROUTE_FAMILY_LEASE,
        world_id: WORLD_ID_LEASE,
        morphism_row_id: MORPHISM_ROW_LEASE,
        required_morphisms: REQUIRED_MORPHISMS_LEASE,
        route_bound: true,
    },
    TransportActionSpec {
        action_id: TransportActionId::IssueLeaseRelease,
        action: "issue.lease_release",
        operation_id: "op/mcp.issue_lease_release",
        route_family_id: ROUTE_FAMILY_LEASE,
        world_id: WORLD_ID_LEASE,
        morphism_row_id: MORPHISM_ROW_LEASE,
        required_morphisms: REQUIRED_MORPHISMS_LEASE,
        route_bound: true,
    },
    TransportActionSpec {
        action_id: TransportActionId::WorldRouteBinding,
        action: "world.route_binding",
        operation_id: "op/transport.world_route_binding",
        route_family_id: ROUTE_FAMILY_TRANSPORT,
        world_id: WORLD_ID_TRANSPORT,
        morphism_row_id: MORPHISM_ROW_TRANSPORT,
        required_morphisms: REQUIRED_MORPHISMS_TRANSPORT,
        route_bound: true,
    },
    TransportActionSpec {
        action_id: TransportActionId::FiberSpawn,
        action: "fiber.spawn",
        operation_id: "op/transport.fiber_spawn",
        route_family_id: ROUTE_FAMILY_FIBER,
        world_id: WORLD_ID_FIBER,
        morphism_row_id: MORPHISM_ROW_FIBER,
        required_morphisms: REQUIRED_MORPHISMS_FIBER,
        route_bound: true,
    },
    TransportActionSpec {
        action_id: TransportActionId::FiberJoin,
        action: "fiber.join",
        operation_id: "op/transport.fiber_join",
        route_family_id: ROUTE_FAMILY_FIBER,
        world_id: WORLD_ID_FIBER,
        morphism_row_id: MORPHISM_ROW_FIBER,
        required_morphisms: REQUIRED_MORPHISMS_FIBER,
        route_bound: true,
    },
    TransportActionSpec {
        action_id: TransportActionId::FiberCancel,
        action: "fiber.cancel",
        operation_id: "op/transport.fiber_cancel",
        route_family_id: ROUTE_FAMILY_FIBER,
        world_id: WORLD_ID_FIBER,
        morphism_row_id: MORPHISM_ROW_FIBER,
        required_morphisms: REQUIRED_MORPHISMS_FIBER,
        route_bound: true,
    },
    TransportActionSpec {
        action_id: TransportActionId::InstructionRun,
        action: "instruction.run",
        operation_id: "op/mcp.instruction_run",
        route_family_id: ROUTE_FAMILY_INSTRUCTION,
        world_id: WORLD_ID_INSTRUCTION,
        morphism_row_id: MORPHISM_ROW_INSTRUCTION,
        required_morphisms: REQUIRED_MORPHISMS_INSTRUCTION,
        route_bound: true,
    },
    TransportActionSpec {
        action_id: TransportActionId::SiteApplyChange,
        action: "site.apply_change",
        operation_id: "op/site.apply_change",
        route_family_id: ROUTE_FAMILY_SITE_CHANGE,
        world_id: WORLD_ID_SITE_CHANGE,
        morphism_row_id: MORPHISM_ROW_SITE_CHANGE,
        required_morphisms: REQUIRED_MORPHISMS_SITE_CHANGE,
        route_bound: true,
    },
    TransportActionSpec {
        action_id: TransportActionId::SiteCurrentDigest,
        action: "site.current_digest",
        operation_id: "op/site.current_digest",
        route_family_id: ROUTE_FAMILY_SITE_CHANGE,
        world_id: WORLD_ID_SITE_CHANGE,
        morphism_row_id: MORPHISM_ROW_SITE_CHANGE,
        required_morphisms: REQUIRED_MORPHISMS_SITE_CHANGE,
        route_bound: false,
    },
    TransportActionSpec {
        action_id: TransportActionId::SiteBuildChange,
        action: "site.build_change",
        operation_id: "op/site.build_change",
        route_family_id: ROUTE_FAMILY_SITE_CHANGE,
        world_id: WORLD_ID_SITE_CHANGE,
        morphism_row_id: MORPHISM_ROW_SITE_CHANGE,
        required_morphisms: REQUIRED_MORPHISMS_SITE_CHANGE,
        route_bound: true,
    },
    TransportActionSpec {
        action_id: TransportActionId::SiteComposeChanges,
        action: "site.compose_changes",
        operation_id: "op/site.compose_changes",
        route_family_id: ROUTE_FAMILY_SITE_CHANGE,
        world_id: WORLD_ID_SITE_CHANGE,
        morphism_row_id: MORPHISM_ROW_SITE_CHANGE,
        required_morphisms: REQUIRED_MORPHISMS_SITE_CHANGE,
        route_bound: true,
    },
];

pub(crate) fn transport_action_spec(action_id: TransportActionId) -> &'static TransportActionSpec {
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

pub(crate) fn world_binding(kind: LeaseActionKind) -> WorldRouteBinding {
    world_binding_for_action(kind.transport_action_id())
}

pub(crate) fn world_binding_for_action(action_id: TransportActionId) -> WorldRouteBinding {
    let spec = transport_action_spec(action_id);
    WorldRouteBinding {
        operation_id: spec.operation_id.to_string(),
        route_family_id: spec.route_family_id.to_string(),
        world_id: spec.world_id.to_string(),
        morphism_row_id: spec.morphism_row_id.to_string(),
    }
}

pub(crate) fn resolver_witness_ref(witness: &SiteResolveWitness) -> String {
    format!(
        "resolver://site-resolve/{}/{}",
        witness.operation_id.replace('/', "_"),
        witness.semantic_digest
    )
}

pub(crate) fn resolve_witness_for_action(
    action_id: TransportActionId,
) -> Option<SiteResolveWitness> {
    let spec = transport_action_spec(action_id);
    let response = resolve_site_for_spec(spec).ok()?;
    Some(response.witness)
}

pub(crate) fn resolver_fields_for_action(
    action_id: TransportActionId,
) -> (Option<String>, Option<SiteResolveWitness>) {
    let witness = resolve_witness_for_action(action_id);
    let witness_ref = witness.as_ref().map(resolver_witness_ref);
    (witness_ref, witness)
}

pub(crate) fn semantic_digest(material: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in material {
        hasher.update(part.as_bytes());
        hasher.update([0u8]);
    }
    format!("{TRANSPORT_SEMANTIC_DIGEST_PREFIX}{:x}", hasher.finalize())
}

pub(crate) fn transport_dispatch_digest(action: &str, action_id: &str) -> String {
    semantic_digest(&[
        TRANSPORT_PROFILE_ID,
        TRANSPORT_DISPATCH_KIND,
        action,
        action_id,
    ])
}

pub(crate) fn transport_action_row_digest(spec: &TransportActionSpec) -> String {
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

pub(crate) fn transport_action_row(spec: &TransportActionSpec) -> TransportActionRegistryRow {
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

pub(crate) fn transport_check_digest(
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

#[derive(Debug, Clone)]
pub(crate) struct TransportKernelBindingError {
    pub(crate) failure_class: String,
    pub(crate) diagnostic: String,
}

pub(crate) fn resolve_site_for_spec(
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
pub(crate) struct LeaseMutationError {
    pub(crate) failure_class: String,
    pub(crate) diagnostic: String,
}

impl LeaseMutationError {
    pub(crate) fn new(failure_class: &str, diagnostic: impl Into<String>) -> Self {
        Self {
            failure_class: failure_class.to_string(),
            diagnostic: diagnostic.into(),
        }
    }
}

pub(crate) fn accepted_envelope(
    kind: LeaseActionKind,
    issues_path: String,
    issue: IssueSummary,
    changed: bool,
    lease_projection: LeaseProjection,
) -> LeaseActionEnvelope {
    accepted_envelope_optional(kind, issues_path, Some(issue), changed, lease_projection)
}

pub(crate) fn accepted_envelope_optional(
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

pub(crate) fn rejected_envelope(
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

pub(crate) fn map_atomic_store_error(
    err: AtomicStoreMutationError<LeaseMutationError>,
) -> LeaseMutationError {
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

pub(crate) fn map_claim_next_atomic_store_error(
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

pub(crate) fn map_claim_next_error(err: ClaimNextError) -> LeaseMutationError {
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

pub(crate) fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(crate) fn resolve_issues_path(path: Option<String>) -> String {
    non_empty(path).unwrap_or_else(|| DEFAULT_ISSUES_PATH.to_string())
}

pub(crate) fn parse_lease_ttl_seconds(ttl_seconds: Option<i64>) -> Result<i64, LeaseMutationError> {
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

pub(crate) fn parse_lease_expiry(
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

pub(crate) fn lease_token(value: &str) -> String {
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

pub(crate) fn resolve_lease_id(
    raw_lease_id: Option<String>,
    issue_id: &str,
    assignee: &str,
) -> String {
    non_empty(raw_lease_id)
        .unwrap_or_else(|| format!("lease1_{}_{}", lease_token(issue_id), lease_token(assignee)))
}

pub(crate) fn lease_state_label(issue: &Issue, now: DateTime<Utc>) -> &'static str {
    match issue.lease_state_at(now) {
        IssueLeaseState::Unleased => "unleased",
        IssueLeaseState::Active => "active",
        IssueLeaseState::Stale => "stale",
    }
}

pub(crate) fn issue_is_lease_contended(issue: &Issue, now: DateTime<Utc>) -> bool {
    let Some(lease) = issue.lease.as_ref() else {
        return false;
    };
    if lease.expires_at <= now {
        return false;
    }
    issue.status != "in_progress" || issue.assignee != lease.owner
}

pub(crate) fn issue_summary(issue: &Issue, now: DateTime<Utc>) -> IssueSummary {
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

pub(crate) fn compute_lease_projection(store: &MemoryStore, now: DateTime<Utc>) -> LeaseProjection {
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

pub(crate) fn transport_rejected(
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

pub(crate) fn annotate_transport_dispatch_fields(
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
pub struct InstructionRunRequest {
    pub instruction_path: String,
    #[serde(default)]
    pub allow_failure: Option<bool>,
    #[serde(default)]
    pub repo_root: Option<String>,
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
