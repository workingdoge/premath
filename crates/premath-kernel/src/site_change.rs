//! Site-change apply and compose semantics (CHANGE-SITE spec).
//!
//! Implements the category of doctrine-site changes: typed mutations on
//! SITE-PACKAGE.json with descent conditions. `g` is functorial —
//! `g(f2 ∘ f1) = g(f2) ∘ g(f1)`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

// ── Failure classes (CHANGE-SITE §7) ────────────────────────────────────────

pub mod failure_class {
    pub const DIGEST_MISMATCH: &str = "site_change_digest_mismatch";
    pub const NODE_NOT_FOUND: &str = "site_change_node_not_found";
    pub const NODE_ALREADY_EXISTS: &str = "site_change_node_already_exists";
    pub const EDGE_NOT_FOUND: &str = "site_change_edge_not_found";
    pub const EDGE_DANGLING: &str = "site_change_edge_dangling";
    pub const COVER_NOT_FOUND: &str = "site_change_cover_not_found";
    pub const COVER_PART_MISSING: &str = "site_change_cover_part_missing";
    pub const OPERATION_NOT_FOUND: &str = "site_change_operation_not_found";
    pub const OPERATION_ALREADY_EXISTS: &str = "site_change_operation_already_exists";
    pub const OPERATION_UNREACHABLE: &str = "site_change_operation_unreachable";
    pub const ROUTE_BINDING_INVALID: &str = "site_change_route_binding_invalid";
    pub const MORPHISM_ID_UNKNOWN: &str = "site_change_morphism_id_unknown";
    pub const COMPOSITION_DANGLING: &str = "site_change_composition_dangling";
    pub const COMPOSITION_AMBIGUOUS: &str = "site_change_composition_ambiguous";
    pub const GLUE_OBSTRUCTION: &str = "site_change_glue_obstruction";
    pub const INVALID_REQUEST: &str = "site_change_invalid_request";
}

// ── Types (CHANGE-SITE §2, §3) ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SiteChangeRequest {
    pub schema: u32,
    pub change_kind: String,
    #[serde(default)]
    pub change_id: String,
    pub concern_id: String,
    pub from_digest: String,
    #[serde(default)]
    pub to_digest: String,
    pub morphism_kind: String,
    pub mutations: Vec<SiteMutation>,
    #[serde(default)]
    pub preservation_claims: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SiteMutation {
    #[serde(rename_all = "camelCase")]
    AddNode {
        id: String,
        path: String,
        kind: String,
        requires_declaration: bool,
    },
    #[serde(rename_all = "camelCase")]
    RemoveNode { id: String },
    #[serde(rename_all = "camelCase")]
    AddEdge {
        id: String,
        from: String,
        to: String,
        morphisms: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    RemoveEdge { id: String },
    #[serde(rename_all = "camelCase")]
    AddCover {
        id: String,
        over: String,
        parts: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    RemoveCover { id: String },
    #[serde(rename_all = "camelCase")]
    UpdateCover { id: String, parts: Vec<String> },
    #[serde(rename_all = "camelCase")]
    AddOperation {
        id: String,
        edge_id: String,
        path: String,
        kind: String,
        morphisms: Vec<String>,
        operation_class: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        route_eligibility: Option<RouteEligibility>,
    },
    #[serde(rename_all = "camelCase")]
    RemoveOperation { id: String },
    #[serde(rename_all = "camelCase")]
    ReparentOperations { new_parent_node_id: String },
    #[serde(rename_all = "camelCase")]
    UpdateBaseCoverParts { parts: Vec<String> },
    #[serde(rename_all = "camelCase")]
    UpdateWorldRouteBinding {
        route_family_id: String,
        operation_ids: Vec<String>,
        world_id: String,
        morphism_row_id: String,
        required_morphisms: Vec<String>,
        #[serde(default = "default_failure_class_unbound")]
        failure_class_unbound: String,
    },
}

fn default_failure_class_unbound() -> String {
    "world_route_unbound".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RouteEligibility {
    pub resolver_eligible: bool,
    pub world_route_required: bool,
    pub route_family_id: String,
}

// ── Response types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteChangeResponse {
    pub result: String,
    pub change_id: String,
    pub from_digest: String,
    pub to_digest: String,
    pub commutation_check: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_digests: Option<ArtifactDigests>,
    #[serde(default)]
    pub witness_refs: Vec<String>,
    #[serde(default)]
    pub failure_classes: Vec<String>,
    #[serde(default)]
    pub diagnostics: Vec<SiteChangeDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDigests {
    pub site_package: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteChangeDiagnostic {
    pub class: String,
    pub message: String,
}

// ── Internal working state ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct SiteState {
    nodes: BTreeMap<String, NodeEntry>,
    edges: BTreeMap<String, EdgeEntry>,
    covers: BTreeMap<String, CoverEntry>,
    operations: BTreeMap<String, OperationEntry>,
    parent_node_id: String,
    base_cover_parts: Vec<String>,
    world_route_rows: Vec<WorldRouteRow>,
    // Preserve full package for fields we don't mutate
    raw: Value,
}

#[derive(Debug, Clone)]
struct NodeEntry {
    id: String,
    path: String,
    kind: String,
    requires_declaration: bool,
}

#[derive(Debug, Clone)]
struct EdgeEntry {
    id: String,
    from: String,
    to: String,
    morphisms: Vec<String>,
}

#[derive(Debug, Clone)]
struct CoverEntry {
    id: String,
    over: String,
    parts: Vec<String>,
}

#[derive(Debug, Clone)]
struct OperationEntry {
    id: String,
    edge_id: String,
    path: String,
    kind: String,
    morphisms: Vec<String>,
    operation_class: String,
    route_eligibility: Option<RouteEligibility>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorldRouteRow {
    route_family_id: String,
    operation_ids: Vec<String>,
    world_id: String,
    morphism_row_id: String,
    required_morphisms: Vec<String>,
    #[serde(default = "default_failure_class_unbound")]
    failure_class_unbound: String,
}

// ── Digest computation ──────────────────────────────────────────────────────

pub fn canonical_digest(value: &Value) -> String {
    let canonical = canonical_json(value);
    let hash = Sha256::digest(canonical.as_bytes());
    format!("{hash:x}")
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let entries: Vec<String> = keys
                .iter()
                .map(|k| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap(),
                        canonical_json(&map[*k])
                    )
                })
                .collect();
            format!("{{{}}}", entries.join(","))
        }
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(canonical_json).collect();
            format!("[{}]", items.join(","))
        }
        _ => serde_json::to_string(value).unwrap(),
    }
}

fn change_id_digest(request: &SiteChangeRequest) -> String {
    let id_content = serde_json::json!({
        "changeKind": request.change_kind,
        "concernId": request.concern_id,
        "fromDigest": request.from_digest,
        "mutations": request.mutations,
        "preservationClaims": request.preservation_claims,
    });
    canonical_digest(&id_content)
}

// ── Parse SITE-PACKAGE.json into working state ──────────────────────────────

fn parse_site_state(pkg: &Value) -> Result<SiteState, (String, String)> {
    let site = pkg.get("site").ok_or_else(|| {
        (
            failure_class::INVALID_REQUEST.to_string(),
            "missing 'site' field".to_string(),
        )
    })?;

    let mut nodes = BTreeMap::new();
    if let Some(arr) = site.get("nodes").and_then(Value::as_array) {
        for n in arr {
            let id = n
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            nodes.insert(
                id.clone(),
                NodeEntry {
                    id: id.clone(),
                    path: n
                        .get("path")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    kind: n
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    requires_declaration: n
                        .get("requiresDeclaration")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                },
            );
        }
    }

    let mut edges = BTreeMap::new();
    if let Some(arr) = site.get("edges").and_then(Value::as_array) {
        for e in arr {
            let id = e
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            edges.insert(
                id.clone(),
                EdgeEntry {
                    id: id.clone(),
                    from: e
                        .get("from")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    to: e
                        .get("to")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    morphisms: e
                        .get("morphisms")
                        .and_then(Value::as_array)
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_str)
                                .map(String::from)
                                .collect()
                        })
                        .unwrap_or_default(),
                },
            );
        }
    }

    let mut covers = BTreeMap::new();
    if let Some(arr) = site.get("covers").and_then(Value::as_array) {
        for c in arr {
            let id = c
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            covers.insert(
                id.clone(),
                CoverEntry {
                    id: id.clone(),
                    over: c
                        .get("over")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    parts: c
                        .get("parts")
                        .and_then(Value::as_array)
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_str)
                                .map(String::from)
                                .collect()
                        })
                        .unwrap_or_default(),
                },
            );
        }
    }

    let op_registry = pkg.get("operationRegistry").ok_or_else(|| {
        (
            failure_class::INVALID_REQUEST.to_string(),
            "missing 'operationRegistry' field".to_string(),
        )
    })?;

    let mut operations = BTreeMap::new();
    if let Some(arr) = op_registry.get("operations").and_then(Value::as_array) {
        for op in arr {
            let id = op
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let route_elig = op
                .get("routeEligibility")
                .and_then(|v| serde_json::from_value::<RouteEligibility>(v.clone()).ok());
            operations.insert(
                id.clone(),
                OperationEntry {
                    id: id.clone(),
                    edge_id: op
                        .get("edgeId")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    path: op
                        .get("path")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    kind: op
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    morphisms: op
                        .get("morphisms")
                        .and_then(Value::as_array)
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_str)
                                .map(String::from)
                                .collect()
                        })
                        .unwrap_or_default(),
                    operation_class: op
                        .get("operationClass")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    route_eligibility: route_elig,
                },
            );
        }
    }

    let parent_node_id = op_registry
        .get("parentNodeId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let base_cover_parts = op_registry
        .get("baseCoverParts")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let world_route_rows = pkg
        .get("worldRouteBindings")
        .and_then(|wrb| wrb.get("rows"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value::<WorldRouteRow>(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    Ok(SiteState {
        nodes,
        edges,
        covers,
        operations,
        parent_node_id,
        base_cover_parts,
        world_route_rows,
        raw: pkg.clone(),
    })
}

// ── Serialize state back to SITE-PACKAGE.json ───────────────────────────────

fn state_to_package(state: &SiteState) -> Value {
    let mut pkg = state.raw.clone();

    // Rebuild site
    if let Some(site) = pkg.get_mut("site") {
        // nodes
        let nodes: Vec<Value> = state
            .nodes
            .values()
            .map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "path": n.path,
                    "kind": n.kind,
                    "requiresDeclaration": n.requires_declaration,
                })
            })
            .collect();
        site["nodes"] = Value::Array(nodes);

        // edges
        let edges: Vec<Value> = state
            .edges
            .values()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "from": e.from,
                    "to": e.to,
                    "morphisms": e.morphisms,
                })
            })
            .collect();
        site["edges"] = Value::Array(edges);

        // covers
        let covers: Vec<Value> = state
            .covers
            .values()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "over": c.over,
                    "parts": c.parts,
                })
            })
            .collect();
        site["covers"] = Value::Array(covers);
    }

    // Rebuild operationRegistry
    if let Some(registry) = pkg.get_mut("operationRegistry") {
        registry["parentNodeId"] = Value::String(state.parent_node_id.clone());
        registry["baseCoverParts"] = serde_json::json!(state.base_cover_parts);

        let ops: Vec<Value> = state
            .operations
            .values()
            .map(|op| {
                let mut v = serde_json::json!({
                    "id": op.id,
                    "edgeId": op.edge_id,
                    "path": op.path,
                    "kind": op.kind,
                    "morphisms": op.morphisms,
                    "operationClass": op.operation_class,
                });
                if let Some(re) = &op.route_eligibility {
                    v["routeEligibility"] = serde_json::to_value(re).unwrap();
                }
                v
            })
            .collect();
        registry["operations"] = Value::Array(ops);
    }

    // Rebuild worldRouteBindings
    if let Some(wrb) = pkg.get_mut("worldRouteBindings") {
        let rows: Vec<Value> = state
            .world_route_rows
            .iter()
            .map(|r| serde_json::to_value(r).unwrap())
            .collect();
        wrb["rows"] = Value::Array(rows);
    }

    pkg
}

// ── Mutation application (CHANGE-SITE §4.2) ─────────────────────────────────

fn apply_mutation(state: &mut SiteState, mutation: &SiteMutation) -> Option<(String, String)> {
    match mutation {
        SiteMutation::AddNode {
            id,
            path,
            kind,
            requires_declaration,
        } => {
            if state.nodes.contains_key(id) {
                return Some((
                    failure_class::NODE_ALREADY_EXISTS.to_string(),
                    format!("node '{}' already exists", id),
                ));
            }
            state.nodes.insert(
                id.clone(),
                NodeEntry {
                    id: id.clone(),
                    path: path.clone(),
                    kind: kind.clone(),
                    requires_declaration: *requires_declaration,
                },
            );
            None
        }
        SiteMutation::RemoveNode { id } => {
            if !state.nodes.contains_key(id) {
                return Some((
                    failure_class::NODE_NOT_FOUND.to_string(),
                    format!("node '{}' not found", id),
                ));
            }
            // Check no edge references this node
            for e in state.edges.values() {
                if e.from == *id || e.to == *id {
                    return Some((
                        failure_class::EDGE_DANGLING.to_string(),
                        format!("edge '{}' references node '{}'", e.id, id),
                    ));
                }
            }
            // Check no cover references this node
            for c in state.covers.values() {
                if c.over == *id || c.parts.contains(id) {
                    return Some((
                        failure_class::COVER_PART_MISSING.to_string(),
                        format!("cover '{}' references node '{}'", c.id, id),
                    ));
                }
            }
            state.nodes.remove(id);
            None
        }
        SiteMutation::AddEdge {
            id,
            from,
            to,
            morphisms,
        } => {
            if state.edges.contains_key(id) {
                return Some((
                    failure_class::EDGE_DANGLING.to_string(),
                    format!("edge '{}' already exists", id),
                ));
            }
            if !state.nodes.contains_key(from) {
                return Some((
                    failure_class::EDGE_DANGLING.to_string(),
                    format!("edge '{}' from-node '{}' not found", id, from),
                ));
            }
            if !state.nodes.contains_key(to) {
                return Some((
                    failure_class::EDGE_DANGLING.to_string(),
                    format!("edge '{}' to-node '{}' not found", id, to),
                ));
            }
            state.edges.insert(
                id.clone(),
                EdgeEntry {
                    id: id.clone(),
                    from: from.clone(),
                    to: to.clone(),
                    morphisms: morphisms.clone(),
                },
            );
            None
        }
        SiteMutation::RemoveEdge { id } => {
            if !state.edges.contains_key(id) {
                return Some((
                    failure_class::EDGE_NOT_FOUND.to_string(),
                    format!("edge '{}' not found", id),
                ));
            }
            // Check no operation references this edge
            for op in state.operations.values() {
                if op.edge_id == *id {
                    return Some((
                        failure_class::OPERATION_UNREACHABLE.to_string(),
                        format!("operation '{}' references edge '{}'", op.id, id),
                    ));
                }
            }
            state.edges.remove(id);
            None
        }
        SiteMutation::AddCover { id, over, parts } => {
            if state.covers.contains_key(id) {
                return Some((
                    failure_class::COVER_PART_MISSING.to_string(),
                    format!("cover '{}' already exists", id),
                ));
            }
            if !state.nodes.contains_key(over) {
                return Some((
                    failure_class::COVER_PART_MISSING.to_string(),
                    format!("cover '{}' over-node '{}' not found", id, over),
                ));
            }
            for p in parts {
                if !state.nodes.contains_key(p) {
                    return Some((
                        failure_class::COVER_PART_MISSING.to_string(),
                        format!("cover '{}' part '{}' not found", id, p),
                    ));
                }
            }
            state.covers.insert(
                id.clone(),
                CoverEntry {
                    id: id.clone(),
                    over: over.clone(),
                    parts: parts.clone(),
                },
            );
            None
        }
        SiteMutation::RemoveCover { id } => {
            if !state.covers.contains_key(id) {
                return Some((
                    failure_class::COVER_NOT_FOUND.to_string(),
                    format!("cover '{}' not found", id),
                ));
            }
            state.covers.remove(id);
            None
        }
        SiteMutation::UpdateCover { id, parts } => {
            let cover = match state.covers.get_mut(id) {
                Some(c) => c,
                None => {
                    return Some((
                        failure_class::COVER_NOT_FOUND.to_string(),
                        format!("cover '{}' not found", id),
                    ));
                }
            };
            for p in parts {
                if !state.nodes.contains_key(p) {
                    return Some((
                        failure_class::COVER_PART_MISSING.to_string(),
                        format!("cover '{}' part '{}' not found", id, p),
                    ));
                }
            }
            cover.parts = parts.clone();
            None
        }
        SiteMutation::AddOperation {
            id,
            edge_id,
            path,
            kind,
            morphisms,
            operation_class,
            route_eligibility,
        } => {
            if state.operations.contains_key(id) {
                return Some((
                    failure_class::OPERATION_ALREADY_EXISTS.to_string(),
                    format!("operation '{}' already exists", id),
                ));
            }
            if *operation_class == "route_bound" && route_eligibility.is_none() {
                return Some((
                    failure_class::ROUTE_BINDING_INVALID.to_string(),
                    format!(
                        "operation '{}' is route_bound but missing routeEligibility",
                        id
                    ),
                ));
            }
            state.operations.insert(
                id.clone(),
                OperationEntry {
                    id: id.clone(),
                    edge_id: edge_id.clone(),
                    path: path.clone(),
                    kind: kind.clone(),
                    morphisms: morphisms.clone(),
                    operation_class: operation_class.clone(),
                    route_eligibility: route_eligibility.clone(),
                },
            );
            None
        }
        SiteMutation::RemoveOperation { id } => {
            if !state.operations.contains_key(id) {
                return Some((
                    failure_class::OPERATION_NOT_FOUND.to_string(),
                    format!("operation '{}' not found", id),
                ));
            }
            state.operations.remove(id);
            None
        }
        SiteMutation::ReparentOperations { new_parent_node_id } => {
            if !state.nodes.contains_key(new_parent_node_id) {
                return Some((
                    failure_class::NODE_NOT_FOUND.to_string(),
                    format!("parent node '{}' not found", new_parent_node_id),
                ));
            }
            state.parent_node_id = new_parent_node_id.clone();
            None
        }
        SiteMutation::UpdateBaseCoverParts { parts } => {
            for p in parts {
                if !state.nodes.contains_key(p) {
                    return Some((
                        failure_class::COVER_PART_MISSING.to_string(),
                        format!("base cover part '{}' not found", p),
                    ));
                }
            }
            state.base_cover_parts = parts.clone();
            None
        }
        SiteMutation::UpdateWorldRouteBinding {
            route_family_id,
            operation_ids,
            world_id,
            morphism_row_id,
            required_morphisms,
            failure_class_unbound,
        } => {
            for oid in operation_ids {
                match state.operations.get(oid) {
                    None => {
                        return Some((
                            failure_class::ROUTE_BINDING_INVALID.to_string(),
                            format!("world route binding references unknown operation '{}'", oid),
                        ));
                    }
                    Some(op) if op.operation_class != "route_bound" => {
                        return Some((
                            failure_class::ROUTE_BINDING_INVALID.to_string(),
                            format!(
                                "operation '{}' is '{}', not route_bound",
                                oid, op.operation_class
                            ),
                        ));
                    }
                    _ => {}
                }
            }
            // Replace or append
            if let Some(row) = state
                .world_route_rows
                .iter_mut()
                .find(|r| r.route_family_id == *route_family_id)
            {
                row.operation_ids = operation_ids.clone();
                row.world_id = world_id.clone();
                row.morphism_row_id = morphism_row_id.clone();
                row.required_morphisms = required_morphisms.clone();
                row.failure_class_unbound = failure_class_unbound.clone();
            } else {
                state.world_route_rows.push(WorldRouteRow {
                    route_family_id: route_family_id.clone(),
                    operation_ids: operation_ids.clone(),
                    world_id: world_id.clone(),
                    morphism_row_id: morphism_row_id.clone(),
                    required_morphisms: required_morphisms.clone(),
                    failure_class_unbound: failure_class_unbound.clone(),
                });
            }
            None
        }
    }
}

// ── GC1 post-condition check ────────────────────────────────────────────────

fn check_gc1(state: &SiteState) -> Option<(String, String)> {
    let bound_ops: BTreeSet<String> = state
        .world_route_rows
        .iter()
        .flat_map(|r| r.operation_ids.iter().cloned())
        .collect();

    for op in state.operations.values() {
        if op.operation_class == "route_bound"
            && let Some(re) = &op.route_eligibility
            && re.world_route_required
            && !bound_ops.contains(&op.id)
        {
            return Some((
                failure_class::ROUTE_BINDING_INVALID.to_string(),
                format!(
                    "route_bound operation '{}' with worldRouteRequired=true not in any worldRouteBindings row",
                    op.id
                ),
            ));
        }
    }
    None
}

// ── Apply (CHANGE-SITE §4) ─────────────────────────────────────────────────

pub fn apply_site_change(
    package: &Value,
    request: &SiteChangeRequest,
) -> (SiteChangeResponse, Option<Value>) {
    // §4.1 Digest validation
    let current_digest = canonical_digest(package);
    if current_digest != request.from_digest {
        return (
            SiteChangeResponse {
                result: "rejected".to_string(),
                change_id: String::new(),
                from_digest: request.from_digest.clone(),
                to_digest: String::new(),
                commutation_check: "rejected".to_string(),
                artifact_digests: None,
                witness_refs: vec![],
                failure_classes: vec![failure_class::DIGEST_MISMATCH.to_string()],
                diagnostics: vec![SiteChangeDiagnostic {
                    class: failure_class::DIGEST_MISMATCH.to_string(),
                    message: format!("expected {}, got {}", request.from_digest, current_digest),
                }],
            },
            None,
        );
    }

    // Parse into working state
    let mut state = match parse_site_state(package) {
        Ok(s) => s,
        Err((cls, msg)) => {
            return (
                SiteChangeResponse {
                    result: "rejected".to_string(),
                    change_id: String::new(),
                    from_digest: request.from_digest.clone(),
                    to_digest: String::new(),
                    commutation_check: "rejected".to_string(),
                    artifact_digests: None,
                    witness_refs: vec![],
                    failure_classes: vec![cls.clone()],
                    diagnostics: vec![SiteChangeDiagnostic {
                        class: cls,
                        message: msg,
                    }],
                },
                None,
            );
        }
    };

    // §4.2 Sequential mutation application
    for (i, mutation) in request.mutations.iter().enumerate() {
        if let Some((cls, msg)) = apply_mutation(&mut state, mutation) {
            return (
                SiteChangeResponse {
                    result: "rejected".to_string(),
                    change_id: String::new(),
                    from_digest: request.from_digest.clone(),
                    to_digest: String::new(),
                    commutation_check: "rejected".to_string(),
                    artifact_digests: None,
                    witness_refs: vec![],
                    failure_classes: vec![cls.clone()],
                    diagnostics: vec![SiteChangeDiagnostic {
                        class: cls,
                        message: format!("mutation[{}]: {}", i, msg),
                    }],
                },
                None,
            );
        }
    }

    // §4.3 Post-condition: GC1
    if let Some((cls, msg)) = check_gc1(&state) {
        return (
            SiteChangeResponse {
                result: "rejected".to_string(),
                change_id: String::new(),
                from_digest: request.from_digest.clone(),
                to_digest: String::new(),
                commutation_check: "rejected".to_string(),
                artifact_digests: None,
                witness_refs: vec![],
                failure_classes: vec![cls.clone()],
                diagnostics: vec![SiteChangeDiagnostic {
                    class: cls,
                    message: msg,
                }],
            },
            None,
        );
    }

    // §4.4 Digest computation
    let result_package = state_to_package(&state);
    let to_digest = canonical_digest(&result_package);

    // §4.5 Commutation: for vertical changes the base is unchanged,
    // so the square commutes trivially. For horizontal/mixed, we accept
    // when mutations applied cleanly (preconditions validated above).
    let commutation = "accepted".to_string();

    let cid = change_id_digest(request);

    (
        SiteChangeResponse {
            result: "accepted".to_string(),
            change_id: cid,
            from_digest: request.from_digest.clone(),
            to_digest: to_digest.clone(),
            commutation_check: commutation,
            artifact_digests: Some(ArtifactDigests {
                site_package: to_digest,
            }),
            witness_refs: vec![],
            failure_classes: vec![],
            diagnostics: vec![],
        },
        Some(result_package),
    )
}

// ── Compose (CHANGE-SITE §5) ───────────────────────────────────────────────

pub fn compose_site_changes(
    r1: &SiteChangeRequest,
    r2: &SiteChangeRequest,
) -> Result<SiteChangeRequest, String> {
    if r1.to_digest != r2.from_digest {
        return Err(format!(
            "not composable: r1.toDigest ({}) != r2.fromDigest ({})",
            r1.to_digest, r2.from_digest
        ));
    }

    let mut mutations = r1.mutations.clone();
    mutations.extend(r2.mutations.iter().cloned());

    let claims_1: BTreeSet<&str> = r1.preservation_claims.iter().map(|s| s.as_str()).collect();
    let claims_2: BTreeSet<&str> = r2.preservation_claims.iter().map(|s| s.as_str()).collect();
    let claims: Vec<String> = claims_1
        .intersection(&claims_2)
        .map(|s| s.to_string())
        .collect();

    let morphism_kind = if r1.morphism_kind == r2.morphism_kind {
        r1.morphism_kind.clone()
    } else {
        "mixed".to_string()
    };

    let mut composed = SiteChangeRequest {
        schema: 1,
        change_kind: "premath.site_change.v1".to_string(),
        change_id: String::new(),
        concern_id: "doctrine_site_topology".to_string(),
        from_digest: r1.from_digest.clone(),
        to_digest: r2.to_digest.clone(),
        morphism_kind,
        mutations,
        preservation_claims: claims,
    };
    composed.change_id = change_id_digest(&composed);

    Ok(composed)
}

// ── Current digest projection (CHANGE-SITE §12) ────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteDigestSummary {
    pub digest: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub operation_count: usize,
    pub cover_count: usize,
    pub world_route_binding_row_count: usize,
}

pub fn current_site_digest(package: &Value) -> Result<SiteDigestSummary, String> {
    let state = parse_site_state(package).map_err(|(cls, msg)| format!("{cls}: {msg}"))?;
    let digest = canonical_digest(package);
    Ok(SiteDigestSummary {
        digest,
        node_count: state.nodes.len(),
        edge_count: state.edges.len(),
        operation_count: state.operations.len(),
        cover_count: state.covers.len(),
        world_route_binding_row_count: state.world_route_rows.len(),
    })
}

pub fn current_site_digest_json(package_json: &str) -> String {
    let package: Value = match serde_json::from_str(package_json) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "result": "rejected",
                "failureClasses": [failure_class::INVALID_REQUEST],
                "diagnostic": format!("failed to parse package JSON: {e}"),
            })
            .to_string();
        }
    };
    match current_site_digest(&package) {
        Ok(summary) => serde_json::json!({
            "result": "accepted",
            "digest": summary.digest,
            "summary": {
                "nodeCount": summary.node_count,
                "edgeCount": summary.edge_count,
                "operationCount": summary.operation_count,
                "coverCount": summary.cover_count,
                "worldRouteBindingRowCount": summary.world_route_binding_row_count,
            }
        })
        .to_string(),
        Err(msg) => serde_json::json!({
            "result": "rejected",
            "failureClasses": [failure_class::INVALID_REQUEST],
            "diagnostic": msg,
        })
        .to_string(),
    }
}

// ── MorphismKind auto-classification (CHANGE-SITE §13.2) ───────────────────

fn classify_morphism_kind(mutations: &[SiteMutation]) -> String {
    let mut has_node = false;
    let mut has_op_edge = false;
    for m in mutations {
        match m {
            SiteMutation::AddNode { .. }
            | SiteMutation::RemoveNode { .. }
            | SiteMutation::AddCover { .. }
            | SiteMutation::RemoveCover { .. }
            | SiteMutation::UpdateCover { .. }
            | SiteMutation::UpdateBaseCoverParts { .. }
            | SiteMutation::ReparentOperations { .. } => {
                has_node = true;
            }
            SiteMutation::AddEdge { .. }
            | SiteMutation::RemoveEdge { .. }
            | SiteMutation::AddOperation { .. }
            | SiteMutation::RemoveOperation { .. }
            | SiteMutation::UpdateWorldRouteBinding { .. } => {
                has_op_edge = true;
            }
        }
    }
    match (has_node, has_op_edge) {
        (false, _) => "vertical".to_string(),
        (true, false) => "horizontal".to_string(),
        (true, true) => "mixed".to_string(),
    }
}

// ── Build change (CHANGE-SITE §13) ─────────────────────────────────────────

#[allow(clippy::result_large_err)]
pub fn build_site_change(
    package: &Value,
    mutations: Vec<SiteMutation>,
    preservation_claims: Vec<String>,
) -> Result<SiteChangeRequest, SiteChangeResponse> {
    let from_digest = canonical_digest(package);
    let morphism_kind = classify_morphism_kind(&mutations);

    // Tentative apply on clone to validate and compute toDigest
    let mut request = SiteChangeRequest {
        schema: 1,
        change_kind: "premath.site_change.v1".to_string(),
        change_id: String::new(),
        concern_id: "doctrine_site_topology".to_string(),
        from_digest: from_digest.clone(),
        to_digest: String::new(),
        morphism_kind,
        mutations,
        preservation_claims,
    };

    let (resp, _pkg) = apply_site_change(package, &request);
    if resp.result != "accepted" {
        return Err(resp);
    }

    request.to_digest = resp.to_digest;
    request.change_id = change_id_digest(&request);
    Ok(request)
}

pub fn build_site_change_json(package_json: &str, mutations_json: &str) -> String {
    let package: Value = match serde_json::from_str(package_json) {
        Ok(v) => v,
        Err(e) => {
            let resp = SiteChangeResponse {
                result: "rejected".to_string(),
                change_id: String::new(),
                from_digest: String::new(),
                to_digest: String::new(),
                commutation_check: "rejected".to_string(),
                artifact_digests: None,
                witness_refs: vec![],
                failure_classes: vec![failure_class::INVALID_REQUEST.to_string()],
                diagnostics: vec![SiteChangeDiagnostic {
                    class: failure_class::INVALID_REQUEST.to_string(),
                    message: format!("failed to parse package JSON: {e}"),
                }],
            };
            return serde_json::to_string(&resp).unwrap();
        }
    };

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct BuildInput {
        mutations: Vec<SiteMutation>,
        #[serde(default)]
        preservation_claims: Vec<String>,
    }

    let input: BuildInput = match serde_json::from_str(mutations_json) {
        Ok(v) => v,
        Err(e) => {
            let resp = SiteChangeResponse {
                result: "rejected".to_string(),
                change_id: String::new(),
                from_digest: String::new(),
                to_digest: String::new(),
                commutation_check: "rejected".to_string(),
                artifact_digests: None,
                witness_refs: vec![],
                failure_classes: vec![failure_class::INVALID_REQUEST.to_string()],
                diagnostics: vec![SiteChangeDiagnostic {
                    class: failure_class::INVALID_REQUEST.to_string(),
                    message: format!("failed to parse mutations JSON: {e}"),
                }],
            };
            return serde_json::to_string(&resp).unwrap();
        }
    };

    match build_site_change(&package, input.mutations, input.preservation_claims) {
        Ok(request) => serde_json::json!({
            "result": "accepted",
            "changeRequest": serde_json::to_value(&request).unwrap(),
            "fromDigest": request.from_digest,
            "toDigest": request.to_digest,
            "morphismKind": request.morphism_kind,
            "mutationCount": request.mutations.len(),
        })
        .to_string(),
        Err(resp) => serde_json::to_string(&resp).unwrap(),
    }
}

// ── Compose JSON entry point (CHANGE-SITE §14) ─────────────────────────────

pub fn compose_site_changes_json(request1_json: &str, request2_json: &str) -> String {
    let r1: SiteChangeRequest = match serde_json::from_str(request1_json) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "result": "rejected",
                "failureClasses": [failure_class::INVALID_REQUEST],
                "diagnostic": format!("failed to parse request1 JSON: {e}"),
            })
            .to_string();
        }
    };
    let r2: SiteChangeRequest = match serde_json::from_str(request2_json) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "result": "rejected",
                "failureClasses": [failure_class::INVALID_REQUEST],
                "diagnostic": format!("failed to parse request2 JSON: {e}"),
            })
            .to_string();
        }
    };
    match compose_site_changes(&r1, &r2) {
        Ok(composed) => serde_json::json!({
            "result": "accepted",
            "composedRequest": serde_json::to_value(&composed).unwrap(),
            "fromDigest": composed.from_digest,
            "toDigest": composed.to_digest,
            "morphismKind": composed.morphism_kind,
            "mutationCount": composed.mutations.len(),
        })
        .to_string(),
        Err(msg) => serde_json::json!({
            "result": "rejected",
            "failureClasses": [failure_class::COMPOSITION_DANGLING],
            "diagnostic": msg,
        })
        .to_string(),
    }
}

// ── JSON entry point (for transport dispatch) ───────────────────────────────

pub fn apply_site_change_json(package_json: &str, request_json: &str) -> String {
    let package: Value = match serde_json::from_str(package_json) {
        Ok(v) => v,
        Err(e) => {
            let resp = SiteChangeResponse {
                result: "rejected".to_string(),
                change_id: String::new(),
                from_digest: String::new(),
                to_digest: String::new(),
                commutation_check: "rejected".to_string(),
                artifact_digests: None,
                witness_refs: vec![],
                failure_classes: vec![failure_class::INVALID_REQUEST.to_string()],
                diagnostics: vec![SiteChangeDiagnostic {
                    class: failure_class::INVALID_REQUEST.to_string(),
                    message: format!("failed to parse package JSON: {}", e),
                }],
            };
            return serde_json::to_string(&resp).unwrap();
        }
    };

    let request: SiteChangeRequest = match serde_json::from_str(request_json) {
        Ok(v) => v,
        Err(e) => {
            let resp = SiteChangeResponse {
                result: "rejected".to_string(),
                change_id: String::new(),
                from_digest: String::new(),
                to_digest: String::new(),
                commutation_check: "rejected".to_string(),
                artifact_digests: None,
                witness_refs: vec![],
                failure_classes: vec![failure_class::INVALID_REQUEST.to_string()],
                diagnostics: vec![SiteChangeDiagnostic {
                    class: failure_class::INVALID_REQUEST.to_string(),
                    message: format!("failed to parse change request JSON: {}", e),
                }],
            };
            return serde_json::to_string(&resp).unwrap();
        }
    };

    let (response, _pkg) = apply_site_change(&package, &request);
    serde_json::to_string(&response).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_package() -> Value {
        serde_json::json!({
            "schema": 1,
            "packageKind": "premath.site_package.v1",
            "packageId": "sp.test",
            "site": {
                "schema": 1,
                "sourceKind": "test",
                "siteId": "test.v0",
                "version": "v0",
                "doctrineSpecPath": "test.md",
                "nodes": [
                    {"id": "n1", "path": "n1.md", "kind": "doctrine", "requiresDeclaration": false},
                    {"id": "n2", "path": "n2.md", "kind": "kernel", "requiresDeclaration": true}
                ],
                "covers": [
                    {"id": "c1", "over": "n1", "parts": ["n2"]}
                ],
                "edges": [
                    {"id": "e1", "from": "n1", "to": "n2", "morphisms": ["dm.identity"]}
                ]
            },
            "operationRegistry": {
                "schema": 1,
                "registryKind": "premath.doctrine_operation_registry.v1",
                "parentNodeId": "n2",
                "coverId": "cover.ci",
                "baseCoverParts": [],
                "operations": [],
                "operationClassPolicy": {
                    "schema": 1,
                    "policyKind": "premath.doctrine_operation_class_policy.v1",
                    "classes": {}
                }
            },
            "worldRouteBindings": {
                "schema": 1,
                "bindingKind": "premath.world_route_bindings.v1",
                "rows": []
            }
        })
    }

    #[test]
    fn identity_change_is_noop() {
        let pkg = sample_package();
        let digest = canonical_digest(&pkg);
        let req = SiteChangeRequest {
            schema: 1,
            change_kind: "premath.site_change.v1".to_string(),
            change_id: String::new(),
            concern_id: "doctrine_site_topology".to_string(),
            from_digest: digest.clone(),
            to_digest: String::new(),
            morphism_kind: "vertical".to_string(),
            mutations: vec![],
            preservation_claims: vec![],
        };
        let (resp, result_pkg) = apply_site_change(&pkg, &req);
        assert_eq!(resp.result, "accepted");
        assert_eq!(resp.to_digest, digest);
        assert!(result_pkg.is_some());
    }

    #[test]
    fn add_node_then_edge() {
        let pkg = sample_package();
        let digest = canonical_digest(&pkg);
        let req = SiteChangeRequest {
            schema: 1,
            change_kind: "premath.site_change.v1".to_string(),
            change_id: String::new(),
            concern_id: "doctrine_site_topology".to_string(),
            from_digest: digest.clone(),
            to_digest: String::new(),
            morphism_kind: "horizontal".to_string(),
            mutations: vec![
                SiteMutation::AddNode {
                    id: "n3".to_string(),
                    path: "n3.md".to_string(),
                    kind: "conformance".to_string(),
                    requires_declaration: true,
                },
                SiteMutation::AddEdge {
                    id: "e2".to_string(),
                    from: "n1".to_string(),
                    to: "n3".to_string(),
                    morphisms: vec!["dm.identity".to_string()],
                },
            ],
            preservation_claims: vec!["kernel_verdict_invariant".to_string()],
        };
        let (resp, _) = apply_site_change(&pkg, &req);
        assert_eq!(resp.result, "accepted");
        assert_ne!(resp.to_digest, digest);
    }

    #[test]
    fn digest_mismatch_rejects() {
        let pkg = sample_package();
        let req = SiteChangeRequest {
            schema: 1,
            change_kind: "premath.site_change.v1".to_string(),
            change_id: String::new(),
            concern_id: "doctrine_site_topology".to_string(),
            from_digest: "wrong".to_string(),
            to_digest: String::new(),
            morphism_kind: "vertical".to_string(),
            mutations: vec![],
            preservation_claims: vec![],
        };
        let (resp, _) = apply_site_change(&pkg, &req);
        assert_eq!(resp.result, "rejected");
        assert!(
            resp.failure_classes
                .contains(&failure_class::DIGEST_MISMATCH.to_string())
        );
    }

    #[test]
    fn duplicate_node_rejects() {
        let pkg = sample_package();
        let digest = canonical_digest(&pkg);
        let req = SiteChangeRequest {
            schema: 1,
            change_kind: "premath.site_change.v1".to_string(),
            change_id: String::new(),
            concern_id: "doctrine_site_topology".to_string(),
            from_digest: digest,
            to_digest: String::new(),
            morphism_kind: "horizontal".to_string(),
            mutations: vec![SiteMutation::AddNode {
                id: "n1".to_string(),
                path: "dup.md".to_string(),
                kind: "doctrine".to_string(),
                requires_declaration: false,
            }],
            preservation_claims: vec![],
        };
        let (resp, _) = apply_site_change(&pkg, &req);
        assert_eq!(resp.result, "rejected");
        assert!(
            resp.failure_classes
                .contains(&failure_class::NODE_ALREADY_EXISTS.to_string())
        );
    }

    #[test]
    fn compose_identity_left_unit() {
        let pkg = sample_package();
        let digest = canonical_digest(&pkg);

        let identity = SiteChangeRequest {
            schema: 1,
            change_kind: "premath.site_change.v1".to_string(),
            change_id: String::new(),
            concern_id: "doctrine_site_topology".to_string(),
            from_digest: digest.clone(),
            to_digest: digest.clone(),
            morphism_kind: "vertical".to_string(),
            mutations: vec![],
            preservation_claims: vec!["kernel_verdict_invariant".to_string()],
        };

        let f = SiteChangeRequest {
            schema: 1,
            change_kind: "premath.site_change.v1".to_string(),
            change_id: String::new(),
            concern_id: "doctrine_site_topology".to_string(),
            from_digest: digest.clone(),
            to_digest: String::new(),
            morphism_kind: "vertical".to_string(),
            mutations: vec![SiteMutation::AddNode {
                id: "n3".to_string(),
                path: "n3.md".to_string(),
                kind: "conformance".to_string(),
                requires_declaration: true,
            }],
            preservation_claims: vec!["kernel_verdict_invariant".to_string()],
        };

        // Apply f to get toDigest
        let (f_resp, _) = apply_site_change(&pkg, &f);
        assert_eq!(f_resp.result, "accepted");

        let mut f_with_digest = f.clone();
        f_with_digest.to_digest = f_resp.to_digest.clone();

        // compose(id, f) should equal f
        let composed = compose_site_changes(&identity, &f_with_digest).unwrap();
        let (composed_resp, _) = apply_site_change(&pkg, &composed);
        assert_eq!(composed_resp.result, "accepted");
        assert_eq!(composed_resp.to_digest, f_resp.to_digest);
    }

    #[test]
    fn current_site_digest_summary() {
        let pkg = sample_package();
        let summary = current_site_digest(&pkg).unwrap();
        assert_eq!(summary.digest, canonical_digest(&pkg));
        assert_eq!(summary.node_count, 2);
        assert_eq!(summary.edge_count, 1);
        assert_eq!(summary.operation_count, 0);
        assert_eq!(summary.cover_count, 1);
        assert_eq!(summary.world_route_binding_row_count, 0);
    }

    #[test]
    fn current_site_digest_json_roundtrip() {
        let pkg = sample_package();
        let pkg_json = serde_json::to_string(&pkg).unwrap();
        let result_json = current_site_digest_json(&pkg_json);
        let result: Value = serde_json::from_str(&result_json).unwrap();
        assert_eq!(result["result"], "accepted");
        assert_eq!(result["digest"], canonical_digest(&pkg));
        assert_eq!(result["summary"]["nodeCount"], 2);
    }

    #[test]
    fn build_site_change_roundtrip() {
        let pkg = sample_package();
        let mutations = vec![SiteMutation::AddNode {
            id: "n3".to_string(),
            path: "n3.md".to_string(),
            kind: "conformance".to_string(),
            requires_declaration: true,
        }];
        let claims = vec!["kernel_verdict_invariant".to_string()];
        let req = build_site_change(&pkg, mutations, claims).unwrap();
        assert_eq!(req.from_digest, canonical_digest(&pkg));
        assert!(!req.to_digest.is_empty());
        assert!(!req.change_id.is_empty());
        // Apply the built request — should succeed
        let (resp, _) = apply_site_change(&pkg, &req);
        assert_eq!(resp.result, "accepted");
        assert_eq!(resp.to_digest, req.to_digest);
    }

    #[test]
    fn build_site_change_invalid_mutation_rejects() {
        let pkg = sample_package();
        // Add duplicate node
        let mutations = vec![SiteMutation::AddNode {
            id: "n1".to_string(),
            path: "dup.md".to_string(),
            kind: "doctrine".to_string(),
            requires_declaration: false,
        }];
        let result = build_site_change(&pkg, mutations, vec![]);
        assert!(result.is_err());
        let resp = result.unwrap_err();
        assert_eq!(resp.result, "rejected");
    }

    #[test]
    fn build_site_change_morphism_kind_vertical() {
        let pkg = sample_package();
        let mutations = vec![SiteMutation::AddOperation {
            id: "op/test".to_string(),
            edge_id: "e1".to_string(),
            path: "test.rs".to_string(),
            kind: "operation".to_string(),
            morphisms: vec!["dm.identity".to_string()],
            operation_class: "tooling_only".to_string(),
            route_eligibility: None,
        }];
        let req = build_site_change(&pkg, mutations, vec![]).unwrap();
        assert_eq!(req.morphism_kind, "vertical");
    }

    #[test]
    fn build_site_change_morphism_kind_horizontal() {
        let pkg = sample_package();
        let mutations = vec![SiteMutation::AddNode {
            id: "n3".to_string(),
            path: "n3.md".to_string(),
            kind: "conformance".to_string(),
            requires_declaration: false,
        }];
        let req = build_site_change(&pkg, mutations, vec![]).unwrap();
        assert_eq!(req.morphism_kind, "horizontal");
    }

    #[test]
    fn compose_site_changes_json_roundtrip() {
        let pkg = sample_package();
        let digest = canonical_digest(&pkg);

        let r1 = build_site_change(
            &pkg,
            vec![SiteMutation::AddNode {
                id: "n3".to_string(),
                path: "n3.md".to_string(),
                kind: "conformance".to_string(),
                requires_declaration: false,
            }],
            vec!["kernel_verdict_invariant".to_string()],
        )
        .unwrap();

        // Apply r1 to get intermediate package
        let (_, pkg2) = apply_site_change(&pkg, &r1);
        let pkg2 = pkg2.unwrap();

        let r2 = build_site_change(
            &pkg2,
            vec![SiteMutation::AddNode {
                id: "n4".to_string(),
                path: "n4.md".to_string(),
                kind: "conformance".to_string(),
                requires_declaration: false,
            }],
            vec!["kernel_verdict_invariant".to_string()],
        )
        .unwrap();

        let r1_json = serde_json::to_string(&r1).unwrap();
        let r2_json = serde_json::to_string(&r2).unwrap();
        let result_json = compose_site_changes_json(&r1_json, &r2_json);
        let result: Value = serde_json::from_str(&result_json).unwrap();
        assert_eq!(result["result"], "accepted");
        assert_eq!(result["composedRequest"]["fromDigest"], digest);
        assert_eq!(result["composedRequest"]["toDigest"], r2.to_digest);
    }
}
