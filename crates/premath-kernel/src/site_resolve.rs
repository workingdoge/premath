//! Deterministic site-resolve semantics.
//!
//! This module implements the `draft/SITE-RESOLVE` contract over the canonical
//! doctrine artifacts.

use crate::parse_world_route_binding_rows;
use crate::world_registry::{WorldRouteBindingRow, failure_class as world_failure_class};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub const SITE_RESOLVE_SCHEMA: u32 = 1;
pub const SITE_RESOLVE_REQUEST_KIND: &str = "premath.site_resolve.request.v1";
pub const SITE_RESOLVE_RESPONSE_KIND: &str = "premath.site_resolve.response.v1";
pub const SITE_RESOLVE_PROJECTION_KIND: &str = "premath.site_resolve.projection.v1";
pub const SITE_RESOLVE_WITNESS_KIND: &str = "premath.site_resolve.witness.v1";
pub const SITE_PACKAGE_SCHEMA: u32 = 1;
pub const SITE_PACKAGE_KIND: &str = "premath.site_package.v1";
pub const DOCTRINE_ROUTE_BINDING_MAPPING_ROW_ID: &str = "doctrineRouteBinding";
const SITE_RESOLVE_SEMANTIC_DIGEST_PREFIX: &str = "sr1_";
pub const DEFAULT_DOCTRINE_SITE_INPUT_REF: &str = "specs/premath/draft/DOCTRINE-SITE-INPUT.json";
pub const DEFAULT_DOCTRINE_SITE_REF: &str = "specs/premath/draft/DOCTRINE-SITE.json";
pub const DEFAULT_DOCTRINE_OP_REGISTRY_REF: &str = "specs/premath/draft/DOCTRINE-OP-REGISTRY.json";
pub const DEFAULT_CONTROL_PLANE_CONTRACT_REF: &str =
    "specs/premath/draft/CONTROL-PLANE-CONTRACT.json";

pub mod failure_class {
    pub const SITE_RESOLVE_UNBOUND: &str = "site_resolve_unbound";
    pub const SITE_RESOLVE_AMBIGUOUS: &str = "site_resolve_ambiguous";
    pub const SITE_RESOLVE_CAPABILITY_MISSING: &str = "site_resolve_capability_missing";
    pub const SITE_RESOLVE_POLICY_DENIED: &str = "site_resolve_policy_denied";
    pub const SITE_OVERLAP_MISMATCH: &str = "site_overlap_mismatch";
    pub const SITE_GLUE_MISSING: &str = "site_glue_missing";
    pub const SITE_GLUE_NON_CONTRACTIBLE: &str = "site_glue_non_contractible";
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SiteResolveRequest {
    pub schema: u32,
    pub request_kind: String,
    pub operation_id: String,
    #[serde(default)]
    pub route_family_hint: Option<String>,
    #[serde(default)]
    pub claimed_capabilities: Vec<String>,
    pub policy_digest: String,
    pub profile_id: String,
    pub context_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SiteResolveResponse {
    pub schema: u32,
    pub response_kind: String,
    pub result: String,
    pub failure_classes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<SiteResolveSelectedBinding>,
    pub projection: SiteResolveProjection,
    pub witness: SiteResolveWitness,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SiteResolveSelectedBinding {
    pub operation_id: String,
    pub route_family_id: String,
    pub site_node_id: String,
    pub cover_id: String,
    pub world_id: String,
    pub morphism_row_id: String,
    pub required_morphisms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SiteResolveProjection {
    pub projection_kind: String,
    pub request_digest: String,
    pub site_package_digest: String,
    pub doctrine_site_digest: String,
    pub doctrine_op_registry_digest: String,
    pub world_route_digest: String,
    pub policy_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kcir_mapping_ref: Option<SiteResolveKcirMappingRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SiteResolveWitness {
    pub schema: u32,
    pub witness_kind: String,
    pub site_id: String,
    pub operation_id: String,
    pub route_family_id: Option<String>,
    pub world_id: Option<String>,
    pub morphism_row_id: Option<String>,
    pub semantic_digest: String,
    pub failure_classes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SiteResolveKcirMappingRef {
    pub source_kind: String,
    pub target_domain: String,
    pub target_kind: String,
    pub identity_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SitePackageProjection {
    pub schema: u32,
    pub package_kind: String,
    pub source_refs: SitePackageSourceRefs,
    pub site_topology: SitePackageTopology,
    pub operation_rows: Vec<SitePackageOperationRow>,
    pub world_route_rows: Vec<SitePackageWorldRouteRow>,
    pub kcir_mapping_rows: Vec<SitePackageKcirMappingRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SitePackageSourceRefs {
    pub doctrine_site_input: String,
    pub doctrine_site: String,
    pub doctrine_operation_registry: String,
    pub control_plane_contract: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SitePackageTopology {
    pub site_id: String,
    pub nodes: Vec<String>,
    pub covers: Vec<String>,
    pub edges: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SitePackageOperationRow {
    pub operation_id: String,
    pub operation_class: String,
    pub morphisms: Vec<String>,
    pub resolver_eligible: bool,
    pub world_route_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_family_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SitePackageWorldRouteRow {
    pub route_family_id: String,
    pub operation_ids: Vec<String>,
    pub world_id: String,
    pub morphism_row_id: String,
    pub required_morphisms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SitePackageKcirMappingRow {
    pub row_id: String,
    pub source_kind: String,
    pub target_domain: String,
    pub target_kind: String,
    pub identity_fields: Vec<String>,
}

impl SitePackageKcirMappingRow {
    fn to_projection_ref(&self) -> SiteResolveKcirMappingRef {
        SiteResolveKcirMappingRef {
            source_kind: self.source_kind.clone(),
            target_domain: self.target_domain.clone(),
            target_kind: self.target_kind.clone(),
            identity_fields: self.identity_fields.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperationRegistryInput {
    #[serde(default)]
    parent_node_id: String,
    #[serde(default)]
    cover_id: String,
    #[serde(default)]
    operations: Vec<OperationRowInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperationRowInput {
    #[serde(default)]
    id: String,
    #[serde(default)]
    edge_id: String,
    #[serde(default)]
    operation_class: String,
    #[serde(default)]
    morphisms: Vec<String>,
    #[serde(default)]
    route_eligibility: Option<RouteEligibilityInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RouteEligibilityInput {
    #[serde(default)]
    resolver_eligible: bool,
    #[serde(default)]
    world_route_required: bool,
    #[serde(default)]
    route_family_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctrineSiteInput {
    #[serde(default)]
    site_id: String,
    #[serde(default)]
    nodes: Vec<DoctrineSiteNodeRow>,
    #[serde(default)]
    covers: Vec<DoctrineSiteCoverRow>,
    #[serde(default)]
    edges: Vec<DoctrineSiteEdgeRow>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctrineSiteNodeRow {
    #[serde(default)]
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctrineSiteCoverRow {
    #[serde(default)]
    id: String,
    #[serde(default)]
    parts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctrineSiteEdgeRow {
    #[serde(default)]
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityRegistryInput {
    #[serde(default)]
    executable_capabilities: Vec<String>,
}

#[derive(Debug, Clone)]
struct ResolverOperationCandidate {
    operation_id: String,
    route_family_id: String,
    morphisms: BTreeSet<String>,
    edge_id: String,
}

#[derive(Debug, Clone)]
struct ResolvedCandidate {
    operation_id: String,
    route_family_id: String,
    site_node_id: String,
    cover_id: String,
    world_id: String,
    morphism_row_id: String,
    required_morphisms: Vec<String>,
    edge_id: String,
    cover_specificity: usize,
}

pub fn resolve_site_request(
    request: &SiteResolveRequest,
    doctrine_site_input: &Value,
    doctrine_site: &Value,
    doctrine_op_registry: &Value,
    control_plane_contract: &Value,
    capability_registry: &Value,
) -> SiteResolveResponse {
    let canonical_request = canonicalize_request(request);
    let doctrine_site_digest = digest_json_value(doctrine_site);
    let doctrine_op_registry_digest = digest_json_value(doctrine_op_registry);

    let world_route_rows = parse_world_route_binding_rows(doctrine_site_input).unwrap_or_default();
    let canonical_world_route_rows = canonicalize_world_route_rows(&world_route_rows);
    let kcir_mapping_rows = extract_kcir_mapping_rows(control_plane_contract);
    let kcir_mapping_ref = kcir_mapping_rows
        .iter()
        .find(|row| row.row_id == DOCTRINE_ROUTE_BINDING_MAPPING_ROW_ID)
        .map(SitePackageKcirMappingRow::to_projection_ref);

    let operation_registry = parse_operation_registry(doctrine_op_registry).ok();
    let doctrine_site_parsed = parse_doctrine_site(doctrine_site).ok();
    let site_package = build_site_package(
        operation_registry.as_ref(),
        doctrine_site_parsed.as_ref(),
        &canonical_world_route_rows,
        &kcir_mapping_rows,
    );
    let projection = build_projection(
        &canonical_request,
        &site_package,
        &doctrine_site_digest,
        &doctrine_op_registry_digest,
        kcir_mapping_ref.clone(),
    );

    if canonical_request.schema != SITE_RESOLVE_SCHEMA
        || canonical_request.request_kind != SITE_RESOLVE_REQUEST_KIND
    {
        return reject(
            projection,
            vec![failure_class::SITE_RESOLVE_UNBOUND.to_string()],
            &canonical_request,
            &site_package,
        );
    }
    if canonical_request.operation_id.is_empty() {
        return reject(
            projection,
            vec![failure_class::SITE_RESOLVE_UNBOUND.to_string()],
            &canonical_request,
            &site_package,
        );
    }
    if canonical_request.policy_digest.is_empty() || canonical_request.profile_id.is_empty() {
        return reject(
            projection,
            vec![failure_class::SITE_RESOLVE_POLICY_DENIED.to_string()],
            &canonical_request,
            &site_package,
        );
    }

    let Some(operation_registry) = operation_registry else {
        return reject(
            projection,
            vec![failure_class::SITE_RESOLVE_UNBOUND.to_string()],
            &canonical_request,
            &site_package,
        );
    };
    let Some(doctrine_site_parsed) = doctrine_site_parsed else {
        return reject(
            projection,
            vec![failure_class::SITE_GLUE_MISSING.to_string()],
            &canonical_request,
            &site_package,
        );
    };
    if world_route_rows.is_empty() {
        return reject(
            projection,
            vec![world_failure_class::WORLD_ROUTE_UNBOUND.to_string()],
            &canonical_request,
            &site_package,
        );
    }

    let gathered_candidates = gather_operation_candidates(&canonical_request, &operation_registry);
    if gathered_candidates.is_empty() {
        return reject(
            projection,
            vec![failure_class::SITE_RESOLVE_UNBOUND.to_string()],
            &canonical_request,
            &site_package,
        );
    }

    let executable_capabilities = extract_executable_capabilities(capability_registry);
    if !claims_allowed(
        &canonical_request.claimed_capabilities,
        &executable_capabilities,
    ) {
        return reject(
            projection,
            vec![failure_class::SITE_RESOLVE_CAPABILITY_MISSING.to_string()],
            &canonical_request,
            &site_package,
        );
    }
    if !profile_allowed(&canonical_request.profile_id, control_plane_contract)
        || !policy_allowed(&canonical_request.policy_digest, control_plane_contract)
    {
        return reject(
            projection,
            vec![failure_class::SITE_RESOLVE_POLICY_DENIED.to_string()],
            &canonical_request,
            &site_package,
        );
    }

    let mut world_rows_by_family: BTreeMap<String, SitePackageWorldRouteRow> = BTreeMap::new();
    for row in &canonical_world_route_rows {
        world_rows_by_family.insert(row.route_family_id.clone(), row.clone());
    }

    let mut world_validated: Vec<ResolvedCandidate> = Vec::new();
    let mut world_failures: BTreeSet<String> = BTreeSet::new();
    for candidate in gathered_candidates {
        let Some(world_row) = world_rows_by_family.get(&candidate.route_family_id) else {
            world_failures.insert(world_failure_class::WORLD_ROUTE_UNBOUND.to_string());
            continue;
        };
        if !world_row.operation_ids.contains(&candidate.operation_id) {
            world_failures.insert(world_failure_class::WORLD_ROUTE_UNBOUND.to_string());
            continue;
        }
        let missing: Vec<String> = world_row
            .required_morphisms
            .iter()
            .filter(|morphism| !candidate.morphisms.contains(*morphism))
            .cloned()
            .collect();
        if !missing.is_empty() {
            world_failures.insert(world_failure_class::WORLD_ROUTE_MORPHISM_DRIFT.to_string());
            continue;
        }

        world_validated.push(ResolvedCandidate {
            operation_id: candidate.operation_id,
            route_family_id: candidate.route_family_id,
            site_node_id: operation_registry.parent_node_id.trim().to_string(),
            cover_id: operation_registry.cover_id.trim().to_string(),
            world_id: world_row.world_id.clone(),
            morphism_row_id: world_row.morphism_row_id.clone(),
            required_morphisms: world_row.required_morphisms.clone(),
            edge_id: candidate.edge_id,
            cover_specificity: 0,
        });
    }
    if world_validated.is_empty() {
        if world_failures.is_empty() {
            return reject(
                projection,
                vec![failure_class::SITE_RESOLVE_UNBOUND.to_string()],
                &canonical_request,
                &site_package,
            );
        }
        return reject(
            projection,
            world_failures.into_iter().collect(),
            &canonical_request,
            &site_package,
        );
    }

    let site_node_ids: BTreeSet<String> = doctrine_site_parsed
        .nodes
        .iter()
        .filter_map(|row| canonicalize_string(row.id.as_str()))
        .collect();
    let cover_specificity: BTreeMap<String, usize> = doctrine_site_parsed
        .covers
        .iter()
        .filter_map(|row| {
            canonicalize_string(row.id.as_str())
                .map(|cover_id| (cover_id, canonicalize_string_vec(&row.parts).len()))
        })
        .collect();
    let site_edge_ids: BTreeSet<String> = doctrine_site_parsed
        .edges
        .iter()
        .filter_map(|row| canonicalize_string(row.id.as_str()))
        .collect();

    let mut overlap_validated: Vec<ResolvedCandidate> = Vec::new();
    let mut overlap_failures: BTreeSet<String> = BTreeSet::new();
    for mut candidate in world_validated {
        let has_node = site_node_ids.contains(&candidate.site_node_id);
        let specificity = cover_specificity.get(&candidate.cover_id).copied();
        if !has_node || specificity.is_none() {
            overlap_failures.insert(failure_class::SITE_GLUE_MISSING.to_string());
            continue;
        }
        if candidate.edge_id.is_empty() {
            overlap_failures.insert(failure_class::SITE_GLUE_MISSING.to_string());
            continue;
        }
        if !site_edge_ids.contains(&candidate.edge_id) {
            overlap_failures.insert(failure_class::SITE_OVERLAP_MISMATCH.to_string());
            continue;
        }
        candidate.cover_specificity = specificity.unwrap_or_default();
        overlap_validated.push(candidate);
    }
    if overlap_validated.is_empty() {
        if overlap_failures.is_empty() {
            return reject(
                projection,
                vec![failure_class::SITE_GLUE_NON_CONTRACTIBLE.to_string()],
                &canonical_request,
                &site_package,
            );
        }
        return reject(
            projection,
            overlap_failures.into_iter().collect(),
            &canonical_request,
            &site_package,
        );
    }

    let route_hint = canonical_request.route_family_hint.clone();
    overlap_validated.sort_by(|left, right| compare_candidates(left, right, route_hint.as_deref()));
    if overlap_validated.len() > 1
        && compare_candidates(
            &overlap_validated[0],
            &overlap_validated[1],
            route_hint.as_deref(),
        ) == Ordering::Equal
    {
        return reject(
            projection,
            vec![failure_class::SITE_RESOLVE_AMBIGUOUS.to_string()],
            &canonical_request,
            &site_package,
        );
    }

    let selected = overlap_validated.remove(0);
    accept(
        projection,
        &canonical_request,
        &site_package,
        SiteResolveSelectedBinding {
            operation_id: selected.operation_id,
            route_family_id: selected.route_family_id,
            site_node_id: selected.site_node_id,
            cover_id: selected.cover_id,
            world_id: selected.world_id,
            morphism_row_id: selected.morphism_row_id,
            required_morphisms: selected.required_morphisms,
        },
    )
}

fn compare_candidates(
    left: &ResolvedCandidate,
    right: &ResolvedCandidate,
    route_family_hint: Option<&str>,
) -> Ordering {
    let left_hint = route_family_hint.is_some_and(|hint| hint == left.route_family_id);
    let right_hint = route_family_hint.is_some_and(|hint| hint == right.route_family_id);
    right_hint
        .cmp(&left_hint)
        .then_with(|| right.cover_specificity.cmp(&left.cover_specificity))
        .then_with(|| {
            (
                left.route_family_id.as_str(),
                left.operation_id.as_str(),
                left.world_id.as_str(),
                left.morphism_row_id.as_str(),
                left.site_node_id.as_str(),
                left.cover_id.as_str(),
            )
                .cmp(&(
                    right.route_family_id.as_str(),
                    right.operation_id.as_str(),
                    right.world_id.as_str(),
                    right.morphism_row_id.as_str(),
                    right.site_node_id.as_str(),
                    right.cover_id.as_str(),
                ))
        })
}

fn gather_operation_candidates(
    request: &SiteResolveRequest,
    operation_registry: &OperationRegistryInput,
) -> Vec<ResolverOperationCandidate> {
    operation_registry
        .operations
        .iter()
        .filter_map(|operation| {
            let operation_id = canonicalize_string(operation.id.as_str())?;
            if operation_id != request.operation_id {
                return None;
            }
            let operation_class = canonicalize_string(operation.operation_class.as_str())?;
            if operation_class != "route_bound" {
                return None;
            }
            let route_eligibility = operation.route_eligibility.as_ref()?;
            if !route_eligibility.resolver_eligible || !route_eligibility.world_route_required {
                return None;
            }
            let route_family_id = route_eligibility
                .route_family_id
                .as_deref()
                .and_then(canonicalize_string)?;
            if let Some(route_hint) = request.route_family_hint.as_deref()
                && route_family_id != route_hint
            {
                return None;
            }
            Some(ResolverOperationCandidate {
                operation_id,
                route_family_id,
                morphisms: canonicalize_string_vec(&operation.morphisms)
                    .into_iter()
                    .collect(),
                edge_id: canonicalize_string(operation.edge_id.as_str()).unwrap_or_default(),
            })
        })
        .collect()
}

fn parse_operation_registry(value: &Value) -> Result<OperationRegistryInput, String> {
    serde_json::from_value(value.clone()).map_err(|err| err.to_string())
}

fn parse_doctrine_site(value: &Value) -> Result<DoctrineSiteInput, String> {
    serde_json::from_value(value.clone()).map_err(|err| err.to_string())
}

fn build_site_package(
    operation_registry: Option<&OperationRegistryInput>,
    doctrine_site: Option<&DoctrineSiteInput>,
    world_route_rows: &[SitePackageWorldRouteRow],
    kcir_mapping_rows: &[SitePackageKcirMappingRow],
) -> SitePackageProjection {
    let source_refs = SitePackageSourceRefs {
        doctrine_site_input: DEFAULT_DOCTRINE_SITE_INPUT_REF.to_string(),
        doctrine_site: DEFAULT_DOCTRINE_SITE_REF.to_string(),
        doctrine_operation_registry: DEFAULT_DOCTRINE_OP_REGISTRY_REF.to_string(),
        control_plane_contract: DEFAULT_CONTROL_PLANE_CONTRACT_REF.to_string(),
    };

    let topology = doctrine_site
        .map(|site| SitePackageTopology {
            site_id: canonicalize_string(site.site_id.as_str()).unwrap_or_default(),
            nodes: site
                .nodes
                .iter()
                .filter_map(|row| canonicalize_string(row.id.as_str()))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            covers: site
                .covers
                .iter()
                .filter_map(|row| canonicalize_string(row.id.as_str()))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            edges: site
                .edges
                .iter()
                .filter_map(|row| canonicalize_string(row.id.as_str()))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        })
        .unwrap_or_default();

    let operation_rows = operation_registry
        .map(|registry| canonicalize_operation_rows(&registry.operations))
        .unwrap_or_default();

    SitePackageProjection {
        schema: SITE_PACKAGE_SCHEMA,
        package_kind: SITE_PACKAGE_KIND.to_string(),
        source_refs,
        site_topology: topology,
        operation_rows,
        world_route_rows: world_route_rows.to_vec(),
        kcir_mapping_rows: kcir_mapping_rows.to_vec(),
    }
}

fn canonicalize_operation_rows(rows: &[OperationRowInput]) -> Vec<SitePackageOperationRow> {
    let mut out: Vec<SitePackageOperationRow> = rows
        .iter()
        .filter_map(|row| {
            let operation_id = canonicalize_string(row.id.as_str())?;
            let operation_class = canonicalize_string(row.operation_class.as_str())?;
            let morphisms = canonicalize_string_vec(&row.morphisms);
            let route_eligibility = row.route_eligibility.as_ref();
            let resolver_eligible = route_eligibility
                .map(|eligibility| eligibility.resolver_eligible)
                .unwrap_or(false);
            let world_route_required = route_eligibility
                .map(|eligibility| eligibility.world_route_required)
                .unwrap_or(false);
            let route_family_id = route_eligibility
                .and_then(|eligibility| eligibility.route_family_id.as_deref())
                .and_then(canonicalize_string);

            Some(SitePackageOperationRow {
                operation_id,
                operation_class,
                morphisms,
                resolver_eligible,
                world_route_required,
                route_family_id,
            })
        })
        .collect();
    out.sort_by(|left, right| {
        (
            left.operation_id.as_str(),
            left.operation_class.as_str(),
            left.route_family_id.as_deref().unwrap_or(""),
        )
            .cmp(&(
                right.operation_id.as_str(),
                right.operation_class.as_str(),
                right.route_family_id.as_deref().unwrap_or(""),
            ))
    });
    out.dedup();
    out
}

fn canonicalize_world_route_rows(rows: &[WorldRouteBindingRow]) -> Vec<SitePackageWorldRouteRow> {
    let mut out: Vec<SitePackageWorldRouteRow> = rows
        .iter()
        .filter_map(|row| {
            let route_family_id = canonicalize_string(row.route_family_id.as_str())?;
            let world_id = canonicalize_string(row.world_id.as_str())?;
            let morphism_row_id = canonicalize_string(row.morphism_row_id.as_str())?;
            Some(SitePackageWorldRouteRow {
                route_family_id,
                operation_ids: canonicalize_string_vec(&row.operation_ids),
                world_id,
                morphism_row_id,
                required_morphisms: canonicalize_string_vec(&row.required_morphisms),
            })
        })
        .collect();
    out.sort_by(|left, right| left.route_family_id.cmp(&right.route_family_id));
    out.dedup();
    out
}

fn extract_kcir_mapping_rows(control_plane_contract: &Value) -> Vec<SitePackageKcirMappingRow> {
    let Some(mapping_table) = control_plane_contract
        .get("controlPlaneKcirMappings")
        .and_then(Value::as_object)
        .and_then(|row| row.get("mappingTable"))
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };

    let mut out: Vec<SitePackageKcirMappingRow> = Vec::new();
    for (row_id, row) in mapping_table {
        let Some(row_obj) = row.as_object() else {
            continue;
        };
        let Some(source_kind) = row_obj
            .get("sourceKind")
            .and_then(Value::as_str)
            .and_then(canonicalize_string)
        else {
            continue;
        };
        let Some(target_domain) = row_obj
            .get("targetDomain")
            .and_then(Value::as_str)
            .and_then(canonicalize_string)
        else {
            continue;
        };
        let Some(target_kind) = row_obj
            .get("targetKind")
            .and_then(Value::as_str)
            .and_then(canonicalize_string)
        else {
            continue;
        };
        let identity_fields = row_obj
            .get("identityFields")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .filter_map(canonicalize_string)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        out.push(SitePackageKcirMappingRow {
            row_id: row_id.trim().to_string(),
            source_kind,
            target_domain,
            target_kind,
            identity_fields,
        });
    }
    out.sort_by(|left, right| left.row_id.cmp(&right.row_id));
    out
}

fn build_projection(
    request: &SiteResolveRequest,
    site_package: &SitePackageProjection,
    doctrine_site_digest: &str,
    doctrine_op_registry_digest: &str,
    kcir_mapping_ref: Option<SiteResolveKcirMappingRef>,
) -> SiteResolveProjection {
    SiteResolveProjection {
        projection_kind: SITE_RESOLVE_PROJECTION_KIND.to_string(),
        request_digest: digest_serializable(request),
        site_package_digest: digest_serializable(site_package),
        doctrine_site_digest: doctrine_site_digest.to_string(),
        doctrine_op_registry_digest: doctrine_op_registry_digest.to_string(),
        world_route_digest: digest_serializable(&site_package.world_route_rows),
        policy_digest: request.policy_digest.clone(),
        kcir_mapping_ref,
    }
}

fn accept(
    projection: SiteResolveProjection,
    request: &SiteResolveRequest,
    site_package: &SitePackageProjection,
    selected: SiteResolveSelectedBinding,
) -> SiteResolveResponse {
    let failure_classes: Vec<String> = Vec::new();
    SiteResolveResponse {
        schema: SITE_RESOLVE_SCHEMA,
        response_kind: SITE_RESOLVE_RESPONSE_KIND.to_string(),
        result: "accepted".to_string(),
        failure_classes: failure_classes.clone(),
        selected: Some(selected.clone()),
        projection,
        witness: build_witness(
            request,
            site_package,
            Some(&selected),
            &failure_classes,
            "accepted",
        ),
    }
}

fn reject(
    projection: SiteResolveProjection,
    failure_classes: Vec<String>,
    request: &SiteResolveRequest,
    site_package: &SitePackageProjection,
) -> SiteResolveResponse {
    let mut failure_classes: Vec<String> = failure_classes
        .into_iter()
        .filter_map(|value| canonicalize_string(value.as_str()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if failure_classes.is_empty() {
        failure_classes.push(failure_class::SITE_RESOLVE_UNBOUND.to_string());
    }
    SiteResolveResponse {
        schema: SITE_RESOLVE_SCHEMA,
        response_kind: SITE_RESOLVE_RESPONSE_KIND.to_string(),
        result: "rejected".to_string(),
        failure_classes: failure_classes.clone(),
        selected: None,
        projection,
        witness: build_witness(request, site_package, None, &failure_classes, "rejected"),
    }
}

fn build_witness(
    request: &SiteResolveRequest,
    site_package: &SitePackageProjection,
    selected: Option<&SiteResolveSelectedBinding>,
    failure_classes: &[String],
    result: &str,
) -> SiteResolveWitness {
    let site_id = site_package.site_topology.site_id.clone();
    let operation_id = request.operation_id.clone();
    let route_family_id = selected.map(|row| row.route_family_id.clone());
    let world_id = selected.map(|row| row.world_id.clone());
    let morphism_row_id = selected.map(|row| row.morphism_row_id.clone());
    let semantic_digest = site_resolve_semantic_digest(
        site_id.as_str(),
        operation_id.as_str(),
        route_family_id.as_deref(),
        world_id.as_deref(),
        morphism_row_id.as_deref(),
        result,
        failure_classes,
    );

    SiteResolveWitness {
        schema: SITE_RESOLVE_SCHEMA,
        witness_kind: SITE_RESOLVE_WITNESS_KIND.to_string(),
        site_id,
        operation_id,
        route_family_id,
        world_id,
        morphism_row_id,
        semantic_digest,
        failure_classes: failure_classes.to_vec(),
    }
}

fn site_resolve_semantic_digest(
    site_id: &str,
    operation_id: &str,
    route_family_id: Option<&str>,
    world_id: Option<&str>,
    morphism_row_id: Option<&str>,
    result: &str,
    failure_classes: &[String],
) -> String {
    let mut material = Vec::new();
    material.push(site_id.to_string());
    material.push(operation_id.to_string());
    material.push(route_family_id.unwrap_or("").to_string());
    material.push(world_id.unwrap_or("").to_string());
    material.push(morphism_row_id.unwrap_or("").to_string());
    material.push(result.to_string());
    material.extend(failure_classes.iter().cloned());
    let joined = material.join("\u{0000}");
    format!(
        "{SITE_RESOLVE_SEMANTIC_DIGEST_PREFIX}{}",
        digest_bytes(joined.as_bytes())
    )
}

fn canonicalize_request(request: &SiteResolveRequest) -> SiteResolveRequest {
    SiteResolveRequest {
        schema: request.schema,
        request_kind: canonicalize_string(request.request_kind.as_str()).unwrap_or_default(),
        operation_id: canonicalize_string(request.operation_id.as_str()).unwrap_or_default(),
        route_family_hint: request
            .route_family_hint
            .as_deref()
            .and_then(canonicalize_string),
        claimed_capabilities: canonicalize_string_vec(&request.claimed_capabilities),
        policy_digest: canonicalize_string(request.policy_digest.as_str()).unwrap_or_default(),
        profile_id: canonicalize_string(request.profile_id.as_str()).unwrap_or_default(),
        context_ref: canonicalize_string(request.context_ref.as_str()).unwrap_or_default(),
    }
}

fn extract_executable_capabilities(capability_registry: &Value) -> BTreeSet<String> {
    let parsed: CapabilityRegistryInput = serde_json::from_value(capability_registry.clone())
        .unwrap_or(CapabilityRegistryInput {
            executable_capabilities: Vec::new(),
        });
    canonicalize_string_vec(&parsed.executable_capabilities)
        .into_iter()
        .collect()
}

fn claims_allowed(
    claimed_capabilities: &[String],
    executable_capabilities: &BTreeSet<String>,
) -> bool {
    claimed_capabilities.is_empty()
        || claimed_capabilities
            .iter()
            .all(|claim| executable_capabilities.contains(claim))
}

fn profile_allowed(profile_id: &str, control_plane_contract: &Value) -> bool {
    let mut allowed: BTreeSet<String> = BTreeSet::new();
    if let Some(profile_id) = control_plane_contract
        .get("controlPlaneBundleProfile")
        .and_then(Value::as_object)
        .and_then(|row| row.get("profileId"))
        .and_then(Value::as_str)
        .and_then(canonicalize_string)
    {
        allowed.insert(profile_id);
    }
    if let Some(profile_id) = control_plane_contract
        .get("controlPlaneKcirMappings")
        .and_then(Value::as_object)
        .and_then(|row| row.get("profileId"))
        .and_then(Value::as_str)
        .and_then(canonicalize_string)
    {
        allowed.insert(profile_id);
    }
    allowed.is_empty() || allowed.contains(profile_id)
}

fn policy_allowed(policy_digest: &str, control_plane_contract: &Value) -> bool {
    let Some(prefix) = control_plane_contract
        .get("ciInstructionPolicy")
        .and_then(Value::as_object)
        .and_then(|row| row.get("policyDigestPrefix"))
        .and_then(Value::as_str)
        .and_then(canonicalize_string)
    else {
        return true;
    };
    policy_digest.starts_with(prefix.as_str())
}

fn canonicalize_string_vec(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| canonicalize_string(value.as_str()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn canonicalize_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn digest_json_value(value: &Value) -> String {
    digest_bytes(serde_json::to_vec(value).unwrap_or_default().as_slice())
}

fn digest_serializable<T: Serialize>(value: &T) -> String {
    digest_bytes(serde_json::to_vec(value).unwrap_or_default().as_slice())
}

fn digest_bytes(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    let output = digest.finalize();
    let mut rendered = String::with_capacity(output.len() * 2);
    for byte in output {
        rendered.push_str(format!("{byte:02x}").as_str());
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture_request(operation_id: &str) -> SiteResolveRequest {
        SiteResolveRequest {
            schema: SITE_RESOLVE_SCHEMA,
            request_kind: SITE_RESOLVE_REQUEST_KIND.to_string(),
            operation_id: operation_id.to_string(),
            route_family_hint: None,
            claimed_capabilities: vec!["capabilities.ci_witnesses".to_string()],
            policy_digest: "pol1_test".to_string(),
            profile_id: "cp.bundle.v0".to_string(),
            context_ref: "ctx.main".to_string(),
        }
    }

    fn fixture_site_input() -> Value {
        json!({
            "schema": 1,
            "inputKind": "premath.doctrine_operation_site.input.v1",
            "worldRouteBindings": {
                "schema": 1,
                "bindingKind": "premath.world_route_bindings.v1",
                "rows": [{
                    "routeFamilyId": "route.gate_execution",
                    "operationIds": ["op/ci.run_gate"],
                    "worldId": "world.kernel.semantic.v1",
                    "morphismRowId": "wm.kernel.semantic.runtime_gate",
                    "requiredMorphisms": ["dm.identity", "dm.profile.execution"],
                    "failureClassUnbound": "world_route_unbound"
                }]
            }
        })
    }

    fn fixture_site() -> Value {
        json!({
            "siteId": "premath.doctrine_operation_site.v0",
            "nodes": [{"id": "raw/PREMATH-CI"}],
            "covers": [{"id": "cover.ci", "parts": ["raw/CI-TOPOS"]}],
            "edges": [{"id": "e.ci.op.run_gate"}]
        })
    }

    fn fixture_op_registry() -> Value {
        json!({
            "schema": 1,
            "registryKind": "premath.doctrine_operation_registry.v1",
            "parentNodeId": "raw/PREMATH-CI",
            "coverId": "cover.ci",
            "operations": [{
                "id": "op/ci.run_gate",
                "edgeId": "e.ci.op.run_gate",
                "operationClass": "route_bound",
                "morphisms": ["dm.identity", "dm.profile.execution"],
                "routeEligibility": {
                    "resolverEligible": true,
                    "worldRouteRequired": true,
                    "routeFamilyId": "route.gate_execution"
                }
            }]
        })
    }

    fn fixture_control_plane_contract() -> Value {
        json!({
            "controlPlaneBundleProfile": {
                "profileId": "cp.bundle.v0"
            },
            "controlPlaneKcirMappings": {
                "profileId": "cp.kcir.mapping.v0",
                "mappingTable": {
                    "doctrineRouteBinding": {
                        "sourceKind": "doctrine.route.binding.v1",
                        "targetDomain": "kcir.node",
                        "targetKind": "doctrine.route.witness.v1",
                        "identityFields": ["operationId", "siteDigest", "policyDigest"]
                    }
                }
            },
            "ciInstructionPolicy": {
                "policyDigestPrefix": "pol1_"
            }
        })
    }

    fn fixture_capability_registry() -> Value {
        json!({
            "schema": 1,
            "registryKind": "premath.capability_registry.v1",
            "executableCapabilities": ["capabilities.ci_witnesses"]
        })
    }

    #[test]
    fn site_resolve_accepts_valid_route_bound_operation() {
        let response = resolve_site_request(
            &fixture_request("op/ci.run_gate"),
            &fixture_site_input(),
            &fixture_site(),
            &fixture_op_registry(),
            &fixture_control_plane_contract(),
            &fixture_capability_registry(),
        );
        assert_eq!(response.result, "accepted");
        assert!(response.failure_classes.is_empty());
        assert_eq!(
            response
                .selected
                .as_ref()
                .map(|row| row.route_family_id.as_str()),
            Some("route.gate_execution")
        );
        assert_eq!(
            response.projection.projection_kind,
            SITE_RESOLVE_PROJECTION_KIND
        );
    }

    #[test]
    fn site_resolve_rejects_unbound_operation() {
        let response = resolve_site_request(
            &fixture_request("op/ci.missing"),
            &fixture_site_input(),
            &fixture_site(),
            &fixture_op_registry(),
            &fixture_control_plane_contract(),
            &fixture_capability_registry(),
        );
        assert_eq!(response.result, "rejected");
        assert_eq!(
            response.failure_classes,
            vec![failure_class::SITE_RESOLVE_UNBOUND.to_string()]
        );
        assert!(response.selected.is_none());
    }

    #[test]
    fn site_resolve_rejects_duplicate_candidates_as_ambiguous() {
        let mut op_registry = fixture_op_registry();
        op_registry
            .get_mut("operations")
            .and_then(Value::as_array_mut)
            .expect("operations should exist")
            .push(json!({
                "id": "op/ci.run_gate",
                "edgeId": "e.ci.op.run_gate",
                "operationClass": "route_bound",
                "morphisms": ["dm.identity", "dm.profile.execution"],
                "routeEligibility": {
                    "resolverEligible": true,
                    "worldRouteRequired": true,
                    "routeFamilyId": "route.gate_execution"
                }
            }));

        let response = resolve_site_request(
            &fixture_request("op/ci.run_gate"),
            &fixture_site_input(),
            &fixture_site(),
            &op_registry,
            &fixture_control_plane_contract(),
            &fixture_capability_registry(),
        );
        assert_eq!(response.result, "rejected");
        assert_eq!(
            response.failure_classes,
            vec![failure_class::SITE_RESOLVE_AMBIGUOUS.to_string()]
        );
    }

    #[test]
    fn site_resolve_rejects_missing_capability_claim() {
        let mut request = fixture_request("op/ci.run_gate");
        request.claimed_capabilities = vec!["capabilities.missing".to_string()];
        let response = resolve_site_request(
            &request,
            &fixture_site_input(),
            &fixture_site(),
            &fixture_op_registry(),
            &fixture_control_plane_contract(),
            &fixture_capability_registry(),
        );
        assert_eq!(response.result, "rejected");
        assert_eq!(
            response.failure_classes,
            vec![failure_class::SITE_RESOLVE_CAPABILITY_MISSING.to_string()]
        );
    }

    #[test]
    fn site_resolve_rejects_policy_prefix_drift() {
        let mut request = fixture_request("op/ci.run_gate");
        request.policy_digest = "legacy_policy".to_string();
        let response = resolve_site_request(
            &request,
            &fixture_site_input(),
            &fixture_site(),
            &fixture_op_registry(),
            &fixture_control_plane_contract(),
            &fixture_capability_registry(),
        );
        assert_eq!(response.result, "rejected");
        assert_eq!(
            response.failure_classes,
            vec![failure_class::SITE_RESOLVE_POLICY_DENIED.to_string()]
        );
    }
}
