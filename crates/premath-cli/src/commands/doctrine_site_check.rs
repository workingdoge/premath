use premath_kernel::{parse_operation_route_rows, validate_world_route_bindings};
use regex::Regex;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

const CHECK_KIND: &str = "ci.doctrine_site_check.v1";
const FAILURE_CLASS_VIOLATION: &str = "doctrine_site_violation";
const SITE_INPUT_KIND: &str = "premath.doctrine_operation_site.input.v1";
const SITE_SOURCE_KIND: &str = "premath.doctrine_operation_site.source.v1";
const SITE_PACKAGE_KIND: &str = "premath.site_package.v1";
const OP_REGISTRY_KIND: &str = "premath.doctrine_operation_registry.v1";
const WORLD_ROUTE_BINDINGS_KIND: &str = "premath.world_route_bindings.v1";
const DOCTRINE_SITE_CUTOVER_KIND: &str = "premath.doctrine_site_cutover.v1";
const DOCTRINE_SITE_GENERATION_DIGEST_KIND: &str = "premath.doctrine_site_generation_digest.v1";
const OPERATION_CLASS_POLICY_KIND: &str = "premath.doctrine_operation_class_policy.v1";
const OP_CLASS_ROUTE_BOUND: &str = "route_bound";
const OP_CLASS_READ_ONLY: &str = "read_only_projection";
const OP_CLASS_TOOLING_ONLY: &str = "tooling_only";
const REQUIRED_OPERATION_CLASSES: [&str; 3] = [
    OP_CLASS_ROUTE_BOUND,
    OP_CLASS_READ_ONLY,
    OP_CLASS_TOOLING_ONLY,
];

#[derive(Debug, Clone)]
struct DoctrineSiteSummary {
    nodes: usize,
    edges: usize,
    covers: usize,
    operations: usize,
    site_digest: String,
    registry_digest: String,
}

#[derive(Debug, Clone)]
struct CutoverPolicy {
    current_phase_id: String,
    allow_operation_registry_override: bool,
}

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|path| path.parent())
        .unwrap_or(crate_dir.as_path())
        .to_path_buf()
}

fn resolve_path(path: &Path, repo_root: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    if path.exists() {
        return path.to_path_buf();
    }
    repo_root.join(path)
}

fn ensure_path_exists(path: &Path, label: &str) {
    if !path.exists() {
        eprintln!(
            "[doctrine-site-check] ERROR: {label} missing: {}",
            path.display()
        );
        std::process::exit(2);
    }
}

fn ensure_string(value: Option<&Value>, label: &str) -> Result<String, String> {
    let Some(raw) = value.and_then(Value::as_str) else {
        return Err(format!("{label}: non-empty string required"));
    };
    let parsed = raw.trim();
    if parsed.is_empty() {
        return Err(format!("{label}: non-empty string required"));
    }
    Ok(parsed.to_string())
}

fn ensure_bool(value: Option<&Value>, label: &str) -> Result<bool, String> {
    value
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{label}: boolean required"))
}

fn ensure_object<'a>(
    value: Option<&'a Value>,
    label: &str,
) -> Result<&'a Map<String, Value>, String> {
    value
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{label}: object required"))
}

fn ensure_array<'a>(value: Option<&'a Value>, label: &str) -> Result<&'a Vec<Value>, String> {
    value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label}: list required"))
}

fn ensure_string_list(
    value: Option<&Value>,
    label: &str,
    require_non_empty: bool,
) -> Result<Vec<String>, String> {
    let rows = ensure_array(value, label)?;
    if require_non_empty && rows.is_empty() {
        return Err(format!("{label}: non-empty list required"));
    }
    let mut out = Vec::with_capacity(rows.len());
    let mut seen = BTreeSet::new();
    for (idx, row) in rows.iter().enumerate() {
        let parsed = ensure_string(Some(row), &format!("{label}[{idx}]"))?;
        if !seen.insert(parsed.clone()) {
            return Err(format!("{label}: duplicate entry {parsed:?}"));
        }
        out.push(parsed);
    }
    Ok(out)
}

fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn parse_node(
    raw: &Value,
    label: &str,
    include_declares: bool,
) -> Result<Map<String, Value>, String> {
    let obj = ensure_object(Some(raw), label)?;
    let id = ensure_string(obj.get("id"), &format!("{label}.id"))?;
    let path = ensure_string(obj.get("path"), &format!("{label}.path"))?;
    let kind = ensure_string(obj.get("kind"), &format!("{label}.kind"))?;
    let requires_declaration = obj
        .get("requiresDeclaration")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut out = Map::new();
    out.insert("id".to_string(), Value::String(id));
    out.insert("path".to_string(), Value::String(path));
    out.insert("kind".to_string(), Value::String(kind));
    out.insert(
        "requiresDeclaration".to_string(),
        Value::Bool(requires_declaration),
    );

    if include_declares && requires_declaration {
        let declares = ensure_object(obj.get("declares"), &format!("{label}.declares"))?;
        let preserved = sorted_unique(ensure_string_list(
            declares.get("preserved"),
            &format!("{label}.declares.preserved"),
            true,
        )?);
        let not_preserved = sorted_unique(ensure_string_list(
            declares.get("notPreserved"),
            &format!("{label}.declares.notPreserved"),
            true,
        )?);
        let mut declaration_row = Map::new();
        declaration_row.insert(
            "preserved".to_string(),
            Value::Array(preserved.into_iter().map(Value::String).collect()),
        );
        declaration_row.insert(
            "notPreserved".to_string(),
            Value::Array(not_preserved.into_iter().map(Value::String).collect()),
        );
        out.insert("declares".to_string(), Value::Object(declaration_row));
    }

    Ok(out)
}

fn parse_cover(raw: &Value, label: &str) -> Result<Map<String, Value>, String> {
    let obj = ensure_object(Some(raw), label)?;
    let id = ensure_string(obj.get("id"), &format!("{label}.id"))?;
    let over = ensure_string(obj.get("over"), &format!("{label}.over"))?;
    let parts = sorted_unique(ensure_string_list(
        obj.get("parts"),
        &format!("{label}.parts"),
        true,
    )?);
    let mut out = Map::new();
    out.insert("id".to_string(), Value::String(id));
    out.insert("over".to_string(), Value::String(over));
    out.insert(
        "parts".to_string(),
        Value::Array(parts.into_iter().map(Value::String).collect()),
    );
    Ok(out)
}

fn parse_edge(raw: &Value, label: &str) -> Result<Map<String, Value>, String> {
    let obj = ensure_object(Some(raw), label)?;
    let id = ensure_string(obj.get("id"), &format!("{label}.id"))?;
    let from = ensure_string(obj.get("from"), &format!("{label}.from"))?;
    let to = ensure_string(obj.get("to"), &format!("{label}.to"))?;
    let morphisms = sorted_unique(ensure_string_list(
        obj.get("morphisms"),
        &format!("{label}.morphisms"),
        true,
    )?);
    let mut out = Map::new();
    out.insert("id".to_string(), Value::String(id));
    out.insert("from".to_string(), Value::String(from));
    out.insert("to".to_string(), Value::String(to));
    out.insert(
        "morphisms".to_string(),
        Value::Array(morphisms.into_iter().map(Value::String).collect()),
    );
    Ok(out)
}

fn canonicalize_site_source(
    source_map: &Map<String, Value>,
    label: &str,
) -> Result<Map<String, Value>, String> {
    if let Some(schema_raw) = source_map.get("schema")
        && schema_raw != &json!(1)
    {
        return Err(format!("{label}.schema must equal 1"));
    }
    if let Some(source_kind) = source_map.get("sourceKind") {
        let parsed = ensure_string(Some(source_kind), &format!("{label}.sourceKind"))?;
        if parsed != SITE_SOURCE_KIND {
            return Err(format!(
                "{label}.sourceKind must equal {SITE_SOURCE_KIND:?}"
            ));
        }
    }

    let site_id = ensure_string(source_map.get("siteId"), &format!("{label}.siteId"))?;
    let version = ensure_string(source_map.get("version"), &format!("{label}.version"))?;
    let doctrine_spec_path = ensure_string(
        source_map.get("doctrineSpecPath"),
        &format!("{label}.doctrineSpecPath"),
    )?;

    let nodes_raw = ensure_array(source_map.get("nodes"), &format!("{label}.nodes"))?;
    if nodes_raw.is_empty() {
        return Err(format!("{label}.nodes must be a non-empty list"));
    }
    let mut nodes = Vec::with_capacity(nodes_raw.len());
    let mut node_ids = BTreeSet::new();
    for (idx, row) in nodes_raw.iter().enumerate() {
        let node = parse_node(row, &format!("{label}.nodes[{idx}]"), false)?;
        let node_id = ensure_string(node.get("id"), &format!("{label}.nodes[{idx}].id"))?;
        if !node_ids.insert(node_id.clone()) {
            return Err(format!("{label}.nodes duplicate id {node_id:?}"));
        }
        nodes.push(Value::Object(node));
    }
    nodes.sort_by(|a, b| {
        let a_id = a
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let b_id = b
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        a_id.cmp(b_id)
    });

    let covers_raw = ensure_array(source_map.get("covers"), &format!("{label}.covers"))?;
    if covers_raw.is_empty() {
        return Err(format!("{label}.covers must be a non-empty list"));
    }
    let mut covers = Vec::with_capacity(covers_raw.len());
    let mut cover_ids = BTreeSet::new();
    for (idx, row) in covers_raw.iter().enumerate() {
        let cover = parse_cover(row, &format!("{label}.covers[{idx}]"))?;
        let cover_id = ensure_string(cover.get("id"), &format!("{label}.covers[{idx}].id"))?;
        if !cover_ids.insert(cover_id.clone()) {
            return Err(format!("{label}.covers duplicate id {cover_id:?}"));
        }
        covers.push(Value::Object(cover));
    }
    covers.sort_by(|a, b| {
        let a_id = a
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let b_id = b
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        a_id.cmp(b_id)
    });

    let edges_raw = ensure_array(source_map.get("edges"), &format!("{label}.edges"))?;
    if edges_raw.is_empty() {
        return Err(format!("{label}.edges must be a non-empty list"));
    }
    let mut edges = Vec::with_capacity(edges_raw.len());
    let mut edge_ids = BTreeSet::new();
    for (idx, row) in edges_raw.iter().enumerate() {
        let edge = parse_edge(row, &format!("{label}.edges[{idx}]"))?;
        let edge_id = ensure_string(edge.get("id"), &format!("{label}.edges[{idx}].id"))?;
        if !edge_ids.insert(edge_id.clone()) {
            return Err(format!("{label}.edges duplicate id {edge_id:?}"));
        }
        edges.push(Value::Object(edge));
    }
    edges.sort_by(|a, b| {
        let a_id = a
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let b_id = b
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        a_id.cmp(b_id)
    });

    let mut canonical = Map::new();
    canonical.insert("schema".to_string(), json!(1));
    canonical.insert(
        "sourceKind".to_string(),
        Value::String(SITE_SOURCE_KIND.to_string()),
    );
    canonical.insert("siteId".to_string(), Value::String(site_id));
    canonical.insert("version".to_string(), Value::String(version));
    canonical.insert(
        "doctrineSpecPath".to_string(),
        Value::String(doctrine_spec_path),
    );
    canonical.insert("nodes".to_string(), Value::Array(nodes));
    canonical.insert("covers".to_string(), Value::Array(covers));
    canonical.insert("edges".to_string(), Value::Array(edges));
    Ok(canonical)
}

fn parse_operation_class_policy(
    policy_map: &Map<String, Value>,
    label: &str,
) -> Result<Map<String, Value>, String> {
    if policy_map.get("schema") != Some(&json!(1)) {
        return Err(format!("{label}.schema must equal 1"));
    }
    let policy_kind = ensure_string(policy_map.get("policyKind"), &format!("{label}.policyKind"))?;
    if policy_kind != OPERATION_CLASS_POLICY_KIND {
        return Err(format!(
            "{label}.policyKind must equal {OPERATION_CLASS_POLICY_KIND:?}"
        ));
    }
    let classes = ensure_object(policy_map.get("classes"), &format!("{label}.classes"))?;
    let mut class_rows: BTreeMap<String, Map<String, Value>> = BTreeMap::new();

    for (class_id, raw) in classes {
        let parsed_class_id = class_id.trim();
        if parsed_class_id.is_empty() {
            return Err(format!("{label}.classes key: non-empty string required"));
        }
        let row = ensure_object(Some(raw), &format!("{label}.classes.{parsed_class_id}"))?;
        let authority_mode = ensure_string(
            row.get("authorityMode"),
            &format!("{label}.classes.{parsed_class_id}.authorityMode"),
        )?;
        let resolver_eligible = ensure_bool(
            row.get("resolverEligible"),
            &format!("{label}.classes.{parsed_class_id}.resolverEligible"),
        )?;
        let mutation_allowed = ensure_bool(
            row.get("mutationAllowed"),
            &format!("{label}.classes.{parsed_class_id}.mutationAllowed"),
        )?;
        let mut class_row = Map::new();
        class_row.insert("authorityMode".to_string(), Value::String(authority_mode));
        class_row.insert(
            "resolverEligible".to_string(),
            Value::Bool(resolver_eligible),
        );
        class_row.insert("mutationAllowed".to_string(), Value::Bool(mutation_allowed));
        class_rows.insert(parsed_class_id.to_string(), class_row);
    }

    let missing = REQUIRED_OPERATION_CLASSES
        .iter()
        .filter(|class| !class_rows.contains_key(**class))
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "{label}.classes missing required classes: {missing:?}"
        ));
    }

    let route_bound = class_rows
        .get(OP_CLASS_ROUTE_BOUND)
        .and_then(|row| row.get("resolverEligible"))
        .and_then(Value::as_bool)
        == Some(true);
    if !route_bound {
        return Err(format!(
            "{label}.classes.{OP_CLASS_ROUTE_BOUND}.resolverEligible must be true"
        ));
    }
    let read_only_mutation = class_rows
        .get(OP_CLASS_READ_ONLY)
        .and_then(|row| row.get("mutationAllowed"))
        .and_then(Value::as_bool)
        == Some(false);
    if !read_only_mutation {
        return Err(format!(
            "{label}.classes.{OP_CLASS_READ_ONLY}.mutationAllowed must be false"
        ));
    }
    let read_only_resolver = class_rows
        .get(OP_CLASS_READ_ONLY)
        .and_then(|row| row.get("resolverEligible"))
        .and_then(Value::as_bool)
        == Some(false);
    if !read_only_resolver {
        return Err(format!(
            "{label}.classes.{OP_CLASS_READ_ONLY}.resolverEligible must be false"
        ));
    }
    let tooling_resolver = class_rows
        .get(OP_CLASS_TOOLING_ONLY)
        .and_then(|row| row.get("resolverEligible"))
        .and_then(Value::as_bool)
        == Some(false);
    if !tooling_resolver {
        return Err(format!(
            "{label}.classes.{OP_CLASS_TOOLING_ONLY}.resolverEligible must be false"
        ));
    }

    let mut classes_out = Map::new();
    for (class_id, row) in class_rows {
        classes_out.insert(class_id, Value::Object(row));
    }
    let mut out = Map::new();
    out.insert("schema".to_string(), json!(1));
    out.insert(
        "policyKind".to_string(),
        Value::String(OPERATION_CLASS_POLICY_KIND.to_string()),
    );
    out.insert("classes".to_string(), Value::Object(classes_out));
    Ok(out)
}

fn parse_route_eligibility(
    value: Option<&Value>,
    label: &str,
) -> Result<Option<Map<String, Value>>, String> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let row = ensure_object(Some(raw), label)?;
    let resolver_eligible = ensure_bool(
        row.get("resolverEligible"),
        &format!("{label}.resolverEligible"),
    )?;
    let world_route_required = ensure_bool(
        row.get("worldRouteRequired"),
        &format!("{label}.worldRouteRequired"),
    )?;
    let route_family_id =
        ensure_string(row.get("routeFamilyId"), &format!("{label}.routeFamilyId"))?;
    let mut out = Map::new();
    out.insert(
        "resolverEligible".to_string(),
        Value::Bool(resolver_eligible),
    );
    out.insert(
        "worldRouteRequired".to_string(),
        Value::Bool(world_route_required),
    );
    out.insert("routeFamilyId".to_string(), Value::String(route_family_id));
    Ok(Some(out))
}

fn parse_operation(raw: &Value, label: &str) -> Result<Map<String, Value>, String> {
    let obj = ensure_object(Some(raw), label)?;
    let id = ensure_string(obj.get("id"), &format!("{label}.id"))?;
    let edge_id = ensure_string(obj.get("edgeId"), &format!("{label}.edgeId"))?;
    let path = ensure_string(obj.get("path"), &format!("{label}.path"))?;
    let kind = ensure_string(obj.get("kind"), &format!("{label}.kind"))?;
    let operation_class = ensure_string(
        obj.get("operationClass"),
        &format!("{label}.operationClass"),
    )?;
    let morphisms = sorted_unique(ensure_string_list(
        obj.get("morphisms"),
        &format!("{label}.morphisms"),
        true,
    )?);

    let route_eligibility = parse_route_eligibility(
        obj.get("routeEligibility"),
        &format!("{label}.routeEligibility"),
    )?;

    let mut out = Map::new();
    out.insert("id".to_string(), Value::String(id));
    out.insert("edgeId".to_string(), Value::String(edge_id));
    out.insert("path".to_string(), Value::String(path));
    out.insert("kind".to_string(), Value::String(kind));
    out.insert("operationClass".to_string(), Value::String(operation_class));
    out.insert(
        "morphisms".to_string(),
        Value::Array(morphisms.into_iter().map(Value::String).collect()),
    );
    if let Some(route) = route_eligibility {
        out.insert("routeEligibility".to_string(), Value::Object(route));
    }
    Ok(out)
}

fn canonicalize_operation_registry(
    registry_map: &Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    let registry_kind = ensure_string(registry_map.get("registryKind"), "registryKind")?;
    if registry_kind != OP_REGISTRY_KIND {
        return Err(format!("registryKind must be {OP_REGISTRY_KIND:?}"));
    }
    let parent_node_id = ensure_string(registry_map.get("parentNodeId"), "parentNodeId")?;
    let cover_id = ensure_string(registry_map.get("coverId"), "coverId")?;
    let policy_raw = ensure_object(
        registry_map.get("operationClassPolicy"),
        "operationClassPolicy",
    )?;
    let operation_class_policy = parse_operation_class_policy(policy_raw, "operationClassPolicy")?;
    let base_cover_parts = sorted_unique(ensure_string_list(
        registry_map.get("baseCoverParts"),
        "baseCoverParts",
        true,
    )?);

    let operations_raw = ensure_array(registry_map.get("operations"), "operations")?;
    if operations_raw.is_empty() {
        return Err("operations: non-empty list required".to_string());
    }
    let classes = ensure_object(
        operation_class_policy.get("classes"),
        "operationClassPolicy.classes",
    )?;
    let mut operations = Vec::with_capacity(operations_raw.len());
    let mut op_ids = BTreeSet::new();
    let mut edge_ids = BTreeSet::new();
    for (idx, row) in operations_raw.iter().enumerate() {
        let operation = parse_operation(row, &format!("operations[{idx}]"))?;
        let op_id = ensure_string(operation.get("id"), &format!("operations[{idx}].id"))?;
        let edge_id = ensure_string(
            operation.get("edgeId"),
            &format!("operations[{idx}].edgeId"),
        )?;
        let operation_class = ensure_string(
            operation.get("operationClass"),
            &format!("operations[{idx}].operationClass"),
        )?;
        if !classes.contains_key(&operation_class) {
            return Err(format!(
                "operations[{idx}].operationClass {operation_class:?} is not declared in operationClassPolicy.classes"
            ));
        }
        if !op_ids.insert(op_id.clone()) {
            return Err(format!("duplicate operation id: {op_id}"));
        }
        if !edge_ids.insert(edge_id.clone()) {
            return Err(format!("duplicate operation edgeId: {edge_id}"));
        }
        let route_eligibility = operation.get("routeEligibility").and_then(Value::as_object);
        if operation_class == OP_CLASS_ROUTE_BOUND {
            let Some(route) = route_eligibility else {
                return Err(format!(
                    "operations[{idx}] ({op_id}) route_bound requires routeEligibility object"
                ));
            };
            if route.get("resolverEligible").and_then(Value::as_bool) != Some(true) {
                return Err(format!(
                    "operations[{idx}] ({op_id}) route_bound requires routeEligibility.resolverEligible=true"
                ));
            }
            if route.get("worldRouteRequired").and_then(Value::as_bool) != Some(true) {
                return Err(format!(
                    "operations[{idx}] ({op_id}) route_bound requires routeEligibility.worldRouteRequired=true"
                ));
            }
        } else if route_eligibility
            .and_then(|route| route.get("resolverEligible"))
            .and_then(Value::as_bool)
            == Some(true)
        {
            return Err(format!(
                "operations[{idx}] ({op_id}) non-route class {operation_class:?} must not be resolverEligible"
            ));
        }
        operations.push(Value::Object(operation));
    }
    operations.sort_by(|a, b| {
        let a_id = a
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let b_id = b
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        a_id.cmp(b_id)
    });

    let mut out = Map::new();
    out.insert("schema".to_string(), json!(1));
    out.insert("registryKind".to_string(), Value::String(registry_kind));
    out.insert("parentNodeId".to_string(), Value::String(parent_node_id));
    out.insert("coverId".to_string(), Value::String(cover_id));
    out.insert(
        "operationClassPolicy".to_string(),
        Value::Object(operation_class_policy),
    );
    out.insert(
        "baseCoverParts".to_string(),
        Value::Array(base_cover_parts.into_iter().map(Value::String).collect()),
    );
    out.insert("operations".to_string(), Value::Array(operations));
    Ok(out)
}

fn canonicalize_world_route_bindings(
    block: &Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    if let Some(schema_raw) = block.get("schema")
        && schema_raw != &json!(1)
    {
        return Err("worldRouteBindings.schema must equal 1".to_string());
    }
    let binding_kind = ensure_string(block.get("bindingKind"), "worldRouteBindings.bindingKind")?;
    if binding_kind != WORLD_ROUTE_BINDINGS_KIND {
        return Err(format!(
            "worldRouteBindings.bindingKind must equal {WORLD_ROUTE_BINDINGS_KIND:?}"
        ));
    }
    let rows_raw = ensure_array(block.get("rows"), "worldRouteBindings.rows")?;
    if rows_raw.is_empty() {
        return Err("worldRouteBindings.rows must be a non-empty list".to_string());
    }
    let mut rows = Vec::with_capacity(rows_raw.len());
    let mut family_ids = BTreeSet::new();
    let mut operation_membership: BTreeMap<String, String> = BTreeMap::new();
    for (idx, row) in rows_raw.iter().enumerate() {
        let item = ensure_object(Some(row), &format!("worldRouteBindings.rows[{idx}]"))?;
        let route_family_id = ensure_string(
            item.get("routeFamilyId"),
            &format!("worldRouteBindings.rows[{idx}].routeFamilyId"),
        )?;
        if !family_ids.insert(route_family_id.clone()) {
            return Err(format!(
                "worldRouteBindings.rows duplicate routeFamilyId {route_family_id:?}"
            ));
        }
        let operation_ids = sorted_unique(ensure_string_list(
            item.get("operationIds"),
            &format!("worldRouteBindings.rows[{idx}].operationIds"),
            true,
        )?);
        for operation_id in &operation_ids {
            if let Some(existing) = operation_membership.get(operation_id)
                && existing != &route_family_id
            {
                return Err(format!(
                    "worldRouteBindings.rows defines duplicate operation binding for {operation_id:?}: {existing:?} vs {route_family_id:?}"
                ));
            }
            operation_membership.insert(operation_id.to_string(), route_family_id.clone());
        }
        let world_id = ensure_string(
            item.get("worldId"),
            &format!("worldRouteBindings.rows[{idx}].worldId"),
        )?;
        let morphism_row_id = ensure_string(
            item.get("morphismRowId"),
            &format!("worldRouteBindings.rows[{idx}].morphismRowId"),
        )?;
        let required_morphisms = sorted_unique(ensure_string_list(
            item.get("requiredMorphisms"),
            &format!("worldRouteBindings.rows[{idx}].requiredMorphisms"),
            true,
        )?);
        let failure_class_unbound = ensure_string(
            item.get("failureClassUnbound"),
            &format!("worldRouteBindings.rows[{idx}].failureClassUnbound"),
        )?;
        let mut row_out = Map::new();
        row_out.insert("routeFamilyId".to_string(), Value::String(route_family_id));
        row_out.insert(
            "operationIds".to_string(),
            Value::Array(operation_ids.into_iter().map(Value::String).collect()),
        );
        row_out.insert("worldId".to_string(), Value::String(world_id));
        row_out.insert("morphismRowId".to_string(), Value::String(morphism_row_id));
        row_out.insert(
            "requiredMorphisms".to_string(),
            Value::Array(required_morphisms.into_iter().map(Value::String).collect()),
        );
        row_out.insert(
            "failureClassUnbound".to_string(),
            Value::String(failure_class_unbound),
        );
        rows.push(Value::Object(row_out));
    }
    rows.sort_by(|a, b| {
        let a_id = a
            .as_object()
            .and_then(|row| row.get("routeFamilyId"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let b_id = b
            .as_object()
            .and_then(|row| row.get("routeFamilyId"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        a_id.cmp(b_id)
    });

    let mut out = Map::new();
    out.insert("schema".to_string(), json!(1));
    out.insert(
        "bindingKind".to_string(),
        Value::String(WORLD_ROUTE_BINDINGS_KIND.to_string()),
    );
    out.insert("rows".to_string(), Value::Array(rows));
    Ok(out)
}

fn collect_world_route_membership(
    world_route_block: &Map<String, Value>,
) -> Result<BTreeMap<String, String>, String> {
    let rows = ensure_array(world_route_block.get("rows"), "worldRouteBindings.rows")?;
    let mut membership: BTreeMap<String, String> = BTreeMap::new();
    for (idx, row) in rows.iter().enumerate() {
        let item = ensure_object(Some(row), &format!("worldRouteBindings.rows[{idx}]"))?;
        let route_family_id = ensure_string(
            item.get("routeFamilyId"),
            &format!("worldRouteBindings.rows[{idx}].routeFamilyId"),
        )?;
        let operation_ids = ensure_string_list(
            item.get("operationIds"),
            &format!("worldRouteBindings.rows[{idx}].operationIds"),
            true,
        )?;
        for operation_id in operation_ids {
            if let Some(existing) = membership.get(&operation_id)
                && existing != &route_family_id
            {
                return Err(format!(
                    "worldRouteBindings.rows defines duplicate operation binding for {operation_id:?}: {existing:?} vs {route_family_id:?}"
                ));
            }
            membership.insert(operation_id, route_family_id.clone());
        }
    }
    Ok(membership)
}

fn validate_operation_classes_against_world_routes(
    registry: &Map<String, Value>,
    world_route_membership: &BTreeMap<String, String>,
    label: &str,
) -> Result<(), String> {
    let operations = ensure_array(registry.get("operations"), &format!("{label}.operations"))?;
    let mut by_id: BTreeMap<String, &Map<String, Value>> = BTreeMap::new();
    for (idx, row) in operations.iter().enumerate() {
        let op = ensure_object(Some(row), &format!("{label}.operations[{idx}]"))?;
        let operation_id = ensure_string(op.get("id"), &format!("{label}.operations[{idx}].id"))?;
        by_id.insert(operation_id, op);
    }

    for (operation_id, route_family_id) in world_route_membership {
        let Some(operation) = by_id.get(operation_id) else {
            return Err(format!(
                "{label}: worldRouteBindings references unknown operation {operation_id:?}"
            ));
        };
        let operation_class = ensure_string(
            operation.get("operationClass"),
            &format!("{label}.operations[{operation_id}].operationClass"),
        )?;
        if operation_class != OP_CLASS_ROUTE_BOUND {
            return Err(format!(
                "{label}: operation {operation_id:?} is world-route bound to {route_family_id:?} but operationClass is {operation_class:?} (expected {OP_CLASS_ROUTE_BOUND:?})"
            ));
        }
        let route = ensure_object(
            operation.get("routeEligibility"),
            &format!("{label}.operations[{operation_id}].routeEligibility"),
        )?;
        let bound_family = ensure_string(
            route.get("routeFamilyId"),
            &format!("{label}.operations[{operation_id}].routeEligibility.routeFamilyId"),
        )?;
        if &bound_family != route_family_id {
            return Err(format!(
                "{label}: operation {operation_id:?} routeEligibility.routeFamilyId {bound_family:?} does not match worldRouteBindings routeFamilyId {route_family_id:?}"
            ));
        }
    }

    for (operation_id, operation) in by_id {
        let operation_class = ensure_string(
            operation.get("operationClass"),
            &format!("{label}.operations[{operation_id}].operationClass"),
        )?;
        if operation_class == OP_CLASS_ROUTE_BOUND {
            if !world_route_membership.contains_key(&operation_id) {
                return Err(format!(
                    "{label}: route_bound operation {operation_id:?} is missing worldRouteBindings entry"
                ));
            }
        } else if world_route_membership.contains_key(&operation_id) {
            return Err(format!(
                "{label}: non-route operation {operation_id:?} must not appear in worldRouteBindings"
            ));
        }
    }
    Ok(())
}

fn canonicalize_site_input(site_input: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    if site_input.get("schema") != Some(&json!(1)) {
        return Err("DOCTRINE-SITE-INPUT.schema must equal 1".to_string());
    }
    let input_kind = ensure_string(site_input.get("inputKind"), "DOCTRINE-SITE-INPUT.inputKind")?;
    if input_kind != SITE_INPUT_KIND {
        return Err(format!(
            "DOCTRINE-SITE-INPUT.inputKind must equal {SITE_INPUT_KIND:?}"
        ));
    }
    let source_raw = ensure_object(site_input.get("site"), "DOCTRINE-SITE-INPUT.site")?;
    let registry_raw = ensure_object(
        site_input.get("operationRegistry"),
        "DOCTRINE-SITE-INPUT.operationRegistry",
    )?;
    let world_routes_raw = ensure_object(
        site_input.get("worldRouteBindings"),
        "DOCTRINE-SITE-INPUT.worldRouteBindings",
    )?;

    let source = canonicalize_site_source(source_raw, "DOCTRINE-SITE-INPUT.site")?;
    let registry = canonicalize_operation_registry(registry_raw)?;
    let world_routes = canonicalize_world_route_bindings(world_routes_raw)?;
    let membership = collect_world_route_membership(&world_routes)?;
    validate_operation_classes_against_world_routes(
        &registry,
        &membership,
        "DOCTRINE-SITE-INPUT.operationRegistry",
    )?;

    let mut out = Map::new();
    out.insert("schema".to_string(), json!(1));
    out.insert(
        "inputKind".to_string(),
        Value::String(SITE_INPUT_KIND.to_string()),
    );
    out.insert("site".to_string(), Value::Object(source));
    out.insert("operationRegistry".to_string(), Value::Object(registry));
    out.insert(
        "worldRouteBindings".to_string(),
        Value::Object(world_routes),
    );
    Ok(out)
}

fn canonicalize_site_package(
    package: &Map<String, Value>,
    label: &str,
) -> Result<Map<String, Value>, String> {
    if package.get("schema") != Some(&json!(1)) {
        return Err(format!("{label}.schema must equal 1"));
    }
    let package_kind = ensure_string(package.get("packageKind"), &format!("{label}.packageKind"))?;
    if package_kind != SITE_PACKAGE_KIND {
        return Err(format!(
            "{label}.packageKind must equal {SITE_PACKAGE_KIND:?}"
        ));
    }
    let package_id = ensure_string(package.get("packageId"), &format!("{label}.packageId"))?;
    let source_raw = ensure_object(package.get("site"), &format!("{label}.site"))?;
    let registry_raw = ensure_object(
        package.get("operationRegistry"),
        &format!("{label}.operationRegistry"),
    )?;
    let world_routes_raw = ensure_object(
        package.get("worldRouteBindings"),
        &format!("{label}.worldRouteBindings"),
    )?;

    let source = canonicalize_site_source(source_raw, &format!("{label}.site"))?;
    let registry = canonicalize_operation_registry(registry_raw)?;
    let world_routes = canonicalize_world_route_bindings(world_routes_raw)?;
    let membership = collect_world_route_membership(&world_routes)?;
    validate_operation_classes_against_world_routes(
        &registry,
        &membership,
        &format!("{label}.operationRegistry"),
    )?;

    let mut out = Map::new();
    out.insert("schema".to_string(), json!(1));
    out.insert(
        "packageKind".to_string(),
        Value::String(SITE_PACKAGE_KIND.to_string()),
    );
    out.insert("packageId".to_string(), Value::String(package_id));
    out.insert("site".to_string(), Value::Object(source));
    out.insert("operationRegistry".to_string(), Value::Object(registry));
    out.insert(
        "worldRouteBindings".to_string(),
        Value::Object(world_routes),
    );
    Ok(out)
}

fn parse_cutover_policy(
    cutover: &Map<String, Value>,
    label: &str,
) -> Result<CutoverPolicy, String> {
    if cutover.get("schema") != Some(&json!(1)) {
        return Err(format!("{label}.schema must equal 1"));
    }
    let cutover_kind = ensure_string(cutover.get("cutoverKind"), &format!("{label}.cutoverKind"))?;
    if cutover_kind != DOCTRINE_SITE_CUTOVER_KIND {
        return Err(format!(
            "{label}.cutoverKind must equal {DOCTRINE_SITE_CUTOVER_KIND:?}"
        ));
    }
    let current_phase_id = ensure_string(
        cutover.get("currentPhaseId"),
        &format!("{label}.currentPhaseId"),
    )?;
    let phases = ensure_array(cutover.get("phases"), &format!("{label}.phases"))?;
    if phases.is_empty() {
        return Err(format!("{label}.phases must be a non-empty list"));
    }

    let mut saw_legacy_enabled = false;
    let mut saw_cutover_phase = false;
    let mut seen_phase_ids = BTreeSet::new();
    let mut current_policy: Option<CutoverPolicy> = None;

    for (idx, row) in phases.iter().enumerate() {
        let phase = ensure_object(Some(row), &format!("{label}.phases[{idx}]"))?;
        let phase_id = ensure_string(
            phase.get("phaseId"),
            &format!("{label}.phases[{idx}].phaseId"),
        )?;
        if !seen_phase_ids.insert(phase_id.clone()) {
            return Err(format!("{label}.phases duplicate phaseId {phase_id:?}"));
        }
        let allow_legacy_source_kind = ensure_bool(
            phase.get("allowLegacySourceKind"),
            &format!("{label}.phases[{idx}].allowLegacySourceKind"),
        )?;
        let allow_operation_registry_override = ensure_bool(
            phase.get("allowOperationRegistryOverride"),
            &format!("{label}.phases[{idx}].allowOperationRegistryOverride"),
        )?;
        if allow_legacy_source_kind || allow_operation_registry_override {
            saw_legacy_enabled = true;
            ensure_string(
                phase.get("windowStartDate"),
                &format!("{label}.phases[{idx}].windowStartDate"),
            )?;
            ensure_string(
                phase.get("windowEndDate"),
                &format!("{label}.phases[{idx}].windowEndDate"),
            )?;
        } else {
            saw_cutover_phase = true;
            ensure_string(
                phase.get("effectiveFromDate"),
                &format!("{label}.phases[{idx}].effectiveFromDate"),
            )?;
        }
        if phase_id == current_phase_id {
            current_policy = Some(CutoverPolicy {
                current_phase_id: phase_id,
                allow_operation_registry_override,
            });
        }
    }

    if !saw_legacy_enabled {
        return Err(format!(
            "{label}.phases must include at least one bounded compatibility phase"
        ));
    }
    if !saw_cutover_phase {
        return Err(format!(
            "{label}.phases must include at least one cutover phase"
        ));
    }
    current_policy.ok_or_else(|| {
        format!("{label}.currentPhaseId {current_phase_id:?} must reference one phases[*].phaseId")
    })
}

fn load_json_object(path: &Path, label: &str) -> Result<Map<String, Value>, String> {
    let payload = fs::read_to_string(path)
        .map_err(|err| format!("{label}: failed reading {}: {err}", path.display()))?;
    let value: Value = serde_json::from_str(&payload)
        .map_err(|err| format!("{label}: failed parsing {}: {err}", path.display()))?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| format!("{label}: top-level object required"))
}

fn list_site_package_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn walk(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        let entries = fs::read_dir(path)
            .map_err(|err| format!("failed reading {}: {err}", path.display()))?;
        for entry in entries {
            let entry =
                entry.map_err(|err| format!("failed iterating {}: {err}", path.display()))?;
            let child = entry.path();
            if child.is_dir() {
                walk(&child, out)?;
            } else if child
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name == "SITE-PACKAGE.json")
                .unwrap_or(false)
            {
                out.push(child);
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    walk(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn generate_site_input_from_packages(
    repo_root: &Path,
    packages_root: &Path,
    cutover_policy: &CutoverPolicy,
    operation_registry_override: Option<&Path>,
) -> Result<Map<String, Value>, String> {
    if !packages_root.exists() || !packages_root.is_dir() {
        return Err(format!(
            "site package root missing: {}",
            packages_root.display()
        ));
    }
    let package_files = list_site_package_files(packages_root)?;
    if package_files.is_empty() {
        return Err(format!(
            "no site package files found under {}",
            packages_root.display()
        ));
    }
    if package_files.len() != 1 {
        let listed = package_files
            .iter()
            .map(|path| relative_display(path, repo_root))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "expected exactly one site package file for v0 source layout, found {}: {}",
            package_files.len(),
            listed
        ));
    }
    let package_path = &package_files[0];
    let package_raw = load_json_object(package_path, "site package")?;
    let canonical_package =
        canonicalize_site_package(&package_raw, &package_path.display().to_string())?;

    let mut input = Map::new();
    input.insert("schema".to_string(), json!(1));
    input.insert(
        "inputKind".to_string(),
        Value::String(SITE_INPUT_KIND.to_string()),
    );
    input.insert(
        "site".to_string(),
        canonical_package
            .get("site")
            .cloned()
            .ok_or_else(|| "site package missing `site`".to_string())?,
    );
    input.insert(
        "operationRegistry".to_string(),
        canonical_package
            .get("operationRegistry")
            .cloned()
            .ok_or_else(|| "site package missing `operationRegistry`".to_string())?,
    );
    input.insert(
        "worldRouteBindings".to_string(),
        canonical_package
            .get("worldRouteBindings")
            .cloned()
            .ok_or_else(|| "site package missing `worldRouteBindings`".to_string())?,
    );

    if let Some(override_path) = operation_registry_override {
        if !cutover_policy.allow_operation_registry_override {
            return Err(format!(
                "operation-registry override path is disabled by cutover phase {:?}; use generated `draft/DOCTRINE-SITE-INPUT.json` authority only",
                cutover_policy.current_phase_id
            ));
        }
        let override_raw = load_json_object(override_path, "operation registry override")?;
        let override_registry = canonicalize_operation_registry(&override_raw)?;
        let world_route_bindings = ensure_object(
            input.get("worldRouteBindings"),
            "DOCTRINE-SITE-INPUT.worldRouteBindings",
        )?;
        let membership = collect_world_route_membership(world_route_bindings)?;
        validate_operation_classes_against_world_routes(
            &override_registry,
            &membership,
            "DOCTRINE-SITE-INPUT.operationRegistry",
        )?;
        input.insert(
            "operationRegistry".to_string(),
            Value::Object(override_registry),
        );
    }

    canonicalize_site_input(&input)
}

fn morphism_regex() -> Regex {
    Regex::new(r"`(dm\.[a-z0-9_.-]+)`").expect("morphism regex should compile")
}

fn parse_registry(doctrine_spec_path: &Path) -> Result<BTreeSet<String>, String> {
    let text = fs::read_to_string(doctrine_spec_path)
        .map_err(|err| format!("{}: failed to read: {err}", doctrine_spec_path.display()))?;
    let heading_re =
        Regex::new(r"(?m)^##\s+.*Doctrine morphism registry \(v0\)\s*$").expect("regex");
    let Some(heading) = heading_re.find(&text) else {
        return Err(format!(
            "{}: cannot locate 'Doctrine morphism registry (v0)' section",
            doctrine_spec_path.display()
        ));
    };
    let tail = &text[heading.end()..];
    let next_heading_re = Regex::new(r"(?m)^##\s+").expect("regex");
    let section = if let Some(next) = next_heading_re.find(tail) {
        &tail[..next.start()]
    } else {
        tail
    };
    let ids = morphism_regex()
        .captures_iter(section)
        .filter_map(|captures| captures.get(1).map(|row| row.as_str().to_string()))
        .collect::<BTreeSet<_>>();
    if ids.is_empty() {
        return Err(format!(
            "{}: no doctrine morphism IDs found in registry",
            doctrine_spec_path.display()
        ));
    }
    Ok(ids)
}

fn parse_declaration(spec_path: &Path) -> Result<(BTreeSet<String>, BTreeSet<String>), String> {
    let text = fs::read_to_string(spec_path)
        .map_err(|err| format!("{}: failed to read: {err}", spec_path.display()))?;
    let heading_re =
        Regex::new(r"(?m)^##\s+.*Doctrine Preservation Declaration \(v0\)\s*$").expect("regex");
    let Some(heading) = heading_re.find(&text) else {
        return Err(format!(
            "{}: missing 'Doctrine Preservation Declaration (v0)' section",
            spec_path.display()
        ));
    };
    let tail = &text[heading.end()..];
    let section_split_re = Regex::new(r"(?m)^##\s+").expect("regex");
    let section = if let Some(next) = section_split_re.find(tail) {
        &tail[..next.start()]
    } else {
        tail
    };
    let mut preserved = BTreeSet::new();
    let mut not_preserved = BTreeSet::new();
    enum Mode {
        None,
        Preserved,
        NotPreserved,
    }
    let mut mode = Mode::None;
    let morphism_re = morphism_regex();
    for line in section.lines() {
        let trimmed = line.trim();
        let lowered = trimmed.to_lowercase();
        if lowered.starts_with("preserved morphisms") {
            mode = Mode::Preserved;
            continue;
        }
        if lowered.starts_with("not preserved") {
            mode = Mode::NotPreserved;
            continue;
        }
        if !trimmed.starts_with('-') {
            continue;
        }
        let ids = morphism_re
            .captures_iter(trimmed)
            .filter_map(|captures| captures.get(1).map(|row| row.as_str().to_string()))
            .collect::<Vec<_>>();
        if ids.is_empty() {
            continue;
        }
        match mode {
            Mode::Preserved => {
                for id in ids {
                    preserved.insert(id);
                }
            }
            Mode::NotPreserved => {
                for id in ids {
                    not_preserved.insert(id);
                }
            }
            Mode::None => {}
        }
    }
    if preserved.is_empty() && not_preserved.is_empty() {
        return Err(format!(
            "{}: declaration section present but morphism lists were not parsed",
            spec_path.display()
        ));
    }
    Ok((preserved, not_preserved))
}

fn canonicalize_site_map(site_map: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    let site_id = ensure_string(site_map.get("siteId"), "siteId")?;
    let version = ensure_string(site_map.get("version"), "version")?;
    let doctrine_spec_path = ensure_string(site_map.get("doctrineSpecPath"), "doctrineSpecPath")?;

    let nodes_raw = ensure_array(site_map.get("nodes"), "nodes")?;
    if nodes_raw.is_empty() {
        return Err("nodes: non-empty list required".to_string());
    }
    let covers_raw = ensure_array(site_map.get("covers"), "covers")?;
    if covers_raw.is_empty() {
        return Err("covers: non-empty list required".to_string());
    }
    let edges_raw = ensure_array(site_map.get("edges"), "edges")?;
    if edges_raw.is_empty() {
        return Err("edges: non-empty list required".to_string());
    }

    let mut nodes = Vec::with_capacity(nodes_raw.len());
    let mut node_ids = BTreeSet::new();
    for (idx, row) in nodes_raw.iter().enumerate() {
        let node = parse_node(row, &format!("nodes[{idx}]"), true)?;
        let node_id = ensure_string(node.get("id"), &format!("nodes[{idx}].id"))?;
        if !node_ids.insert(node_id.clone()) {
            return Err(format!("duplicate node id: {node_id}"));
        }
        nodes.push(Value::Object(node));
    }
    nodes.sort_by(|a, b| {
        let a_id = a
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let b_id = b
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        a_id.cmp(b_id)
    });

    let mut covers = Vec::with_capacity(covers_raw.len());
    let mut cover_ids = BTreeSet::new();
    for (idx, row) in covers_raw.iter().enumerate() {
        let cover = parse_cover(row, &format!("covers[{idx}]"))?;
        let cover_id = ensure_string(cover.get("id"), &format!("covers[{idx}].id"))?;
        if !cover_ids.insert(cover_id.clone()) {
            return Err(format!("duplicate cover id: {cover_id}"));
        }
        covers.push(Value::Object(cover));
    }
    covers.sort_by(|a, b| {
        let a_id = a
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let b_id = b
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        a_id.cmp(b_id)
    });

    let mut edges = Vec::with_capacity(edges_raw.len());
    let mut edge_ids = BTreeSet::new();
    for (idx, row) in edges_raw.iter().enumerate() {
        let edge = parse_edge(row, &format!("edges[{idx}]"))?;
        let edge_id = ensure_string(edge.get("id"), &format!("edges[{idx}].id"))?;
        if !edge_ids.insert(edge_id.clone()) {
            return Err(format!("duplicate edge id: {edge_id}"));
        }
        edges.push(Value::Object(edge));
    }
    edges.sort_by(|a, b| {
        let a_id = a
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let b_id = b
            .as_object()
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        a_id.cmp(b_id)
    });

    let mut out = Map::new();
    out.insert("siteId".to_string(), Value::String(site_id));
    out.insert("version".to_string(), Value::String(version));
    out.insert(
        "doctrineSpecPath".to_string(),
        Value::String(doctrine_spec_path),
    );
    out.insert("nodes".to_string(), Value::Array(nodes));
    out.insert("covers".to_string(), Value::Array(covers));
    out.insert("edges".to_string(), Value::Array(edges));
    Ok(out)
}

fn generate_site_map(
    repo_root: &Path,
    site_input: &Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    let source = ensure_object(site_input.get("site"), "DOCTRINE-SITE-INPUT.site")?;
    let registry = ensure_object(
        site_input.get("operationRegistry"),
        "DOCTRINE-SITE-INPUT.operationRegistry",
    )?;

    let nodes_raw = ensure_array(source.get("nodes"), "DOCTRINE-SITE-INPUT.site.nodes")?;
    let covers_raw = ensure_array(source.get("covers"), "DOCTRINE-SITE-INPUT.site.covers")?;
    let edges_raw = ensure_array(source.get("edges"), "DOCTRINE-SITE-INPUT.site.edges")?;
    let operations_raw = ensure_array(
        registry.get("operations"),
        "DOCTRINE-SITE-INPUT.operationRegistry.operations",
    )?;
    let doctrine_spec_path = ensure_string(source.get("doctrineSpecPath"), "doctrineSpecPath")?;
    let parent_node_id = ensure_string(registry.get("parentNodeId"), "parentNodeId")?;

    let mut node_ids = BTreeSet::new();
    let mut nodes = Vec::new();
    for (idx, row) in nodes_raw.iter().enumerate() {
        let mut node = parse_node(row, &format!("nodes[{idx}]"), false)?;
        let node_id = ensure_string(node.get("id"), &format!("nodes[{idx}].id"))?;
        if !node_ids.insert(node_id.clone()) {
            return Err(format!("duplicate node id {node_id:?}"));
        }
        let node_path = ensure_string(node.get("path"), &format!("nodes[{idx}].path"))?;
        let full_path = repo_root.join(&node_path);
        if !full_path.exists() {
            return Err(format!("node {node_id:?} path missing: {node_path}"));
        }
        let requires_declaration = node
            .get("requiresDeclaration")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if requires_declaration {
            let (preserved, not_preserved) = parse_declaration(&full_path)?;
            let mut declares = Map::new();
            declares.insert(
                "preserved".to_string(),
                Value::Array(preserved.into_iter().map(Value::String).collect()),
            );
            declares.insert(
                "notPreserved".to_string(),
                Value::Array(not_preserved.into_iter().map(Value::String).collect()),
            );
            node.insert("declares".to_string(), Value::Object(declares));
        }
        nodes.push(Value::Object(node));
    }
    if !node_ids.contains(&parent_node_id) {
        return Err(format!(
            "operationRegistry.parentNodeId {parent_node_id:?} must exist in source nodes"
        ));
    }

    let mut generated_operation_ids = Vec::new();
    let mut generated_operation_edges = Vec::new();
    for (idx, row) in operations_raw.iter().enumerate() {
        let operation = parse_operation(row, &format!("operations[{idx}]"))?;
        let op_id = ensure_string(operation.get("id"), &format!("operations[{idx}].id"))?;
        if !node_ids.insert(op_id.clone()) {
            return Err(format!("duplicate operation/node id {op_id:?}"));
        }
        generated_operation_ids.push(op_id.clone());
        let op_path = ensure_string(operation.get("path"), &format!("operations[{idx}].path"))?;
        let full_op_path = repo_root.join(&op_path);
        if !full_op_path.exists() {
            return Err(format!("operation {op_id:?} path missing: {op_path}"));
        }
        let op_kind = ensure_string(operation.get("kind"), &format!("operations[{idx}].kind"))?;
        let edge_id = ensure_string(
            operation.get("edgeId"),
            &format!("operations[{idx}].edgeId"),
        )?;
        let morphisms = sorted_unique(ensure_string_list(
            operation.get("morphisms"),
            &format!("operations[{idx}].morphisms"),
            true,
        )?);

        let mut op_node = Map::new();
        op_node.insert("id".to_string(), Value::String(op_id.clone()));
        op_node.insert("path".to_string(), Value::String(op_path));
        op_node.insert("kind".to_string(), Value::String(op_kind));
        op_node.insert("requiresDeclaration".to_string(), Value::Bool(false));
        nodes.push(Value::Object(op_node));

        let mut edge = Map::new();
        edge.insert("id".to_string(), Value::String(edge_id));
        edge.insert("from".to_string(), Value::String(parent_node_id.clone()));
        edge.insert("to".to_string(), Value::String(op_id));
        edge.insert(
            "morphisms".to_string(),
            Value::Array(morphisms.into_iter().map(Value::String).collect()),
        );
        generated_operation_edges.push(Value::Object(edge));
    }

    let mut covers = Vec::new();
    for (idx, row) in covers_raw.iter().enumerate() {
        covers.push(Value::Object(parse_cover(row, &format!("covers[{idx}]"))?));
    }
    let generated_cover_id = ensure_string(registry.get("coverId"), "coverId")?;
    let base_cover_parts =
        ensure_string_list(registry.get("baseCoverParts"), "baseCoverParts", true)?;
    let mut generated_cover_parts = base_cover_parts;
    generated_cover_parts.extend(generated_operation_ids.clone());
    generated_cover_parts = sorted_unique(generated_cover_parts);
    let mut generated_cover = Map::new();
    generated_cover.insert("id".to_string(), Value::String(generated_cover_id));
    generated_cover.insert("over".to_string(), Value::String(parent_node_id.clone()));
    generated_cover.insert(
        "parts".to_string(),
        Value::Array(
            generated_cover_parts
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    covers.push(Value::Object(generated_cover));

    let mut edges = Vec::new();
    for (idx, row) in edges_raw.iter().enumerate() {
        edges.push(Value::Object(parse_edge(row, &format!("edges[{idx}]"))?));
    }
    edges.extend(generated_operation_edges);

    let site_id = ensure_string(source.get("siteId"), "siteId")?;
    let version = ensure_string(source.get("version"), "version")?;
    let mut generated = Map::new();
    generated.insert("siteId".to_string(), Value::String(site_id));
    generated.insert("version".to_string(), Value::String(version));
    generated.insert(
        "doctrineSpecPath".to_string(),
        Value::String(doctrine_spec_path),
    );
    generated.insert("nodes".to_string(), Value::Array(nodes));
    generated.insert("covers".to_string(), Value::Array(covers));
    generated.insert("edges".to_string(), Value::Array(edges));
    canonicalize_site_map(&generated)
}

fn write_canonical_json(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        Value::Number(v) => out.push_str(&v.to_string()),
        Value::String(v) => {
            out.push_str(&serde_json::to_string(v).expect("string serialization should not fail"))
        }
        Value::Array(rows) => {
            out.push('[');
            for (idx, row) in rows.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_canonical_json(row, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(
                    &serde_json::to_string(key).expect("key serialization should not fail"),
                );
                out.push(':');
                if let Some(value) = map.get(*key) {
                    write_canonical_json(value, out);
                }
            }
            out.push('}');
        }
    }
}

fn canonical_json_string(value: &Value) -> String {
    let mut out = String::new();
    write_canonical_json(value, &mut out);
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn digest_value(value: &Value) -> String {
    sha256_hex(canonical_json_string(value).as_bytes())
}

fn site_input_digest(value: &Map<String, Value>) -> String {
    digest_value(&Value::Object(value.clone()))
}

fn site_map_digest(value: &Map<String, Value>) -> String {
    digest_value(&Value::Object(value.clone()))
}

fn operation_registry_digest(value: &Map<String, Value>) -> String {
    digest_value(&Value::Object(value.clone()))
}

fn site_input_equality_diff(
    expected: &Map<String, Value>,
    actual: &Map<String, Value>,
) -> Vec<String> {
    if expected == actual {
        return Vec::new();
    }
    vec![
        "roundtrip mismatch: tracked doctrine site input differs from generated output".to_string(),
        format!("  - expectedDigest={}", site_input_digest(expected)),
        format!("  - actualDigest={}", site_input_digest(actual)),
    ]
}

fn site_map_equality_diff(
    expected: &Map<String, Value>,
    actual: &Map<String, Value>,
) -> Vec<String> {
    if expected == actual {
        return Vec::new();
    }
    vec![
        "roundtrip mismatch: tracked doctrine site map differs from generated output".to_string(),
        format!("  - expectedDigest={}", site_map_digest(expected)),
        format!("  - actualDigest={}", site_map_digest(actual)),
    ]
}

fn operation_registry_equality_diff(
    expected: &Map<String, Value>,
    actual: &Map<String, Value>,
) -> Vec<String> {
    if expected == actual {
        return Vec::new();
    }
    vec![
        "roundtrip mismatch: tracked doctrine operation registry differs from generated output"
            .to_string(),
        format!("  - expectedDigest={}", operation_registry_digest(expected)),
        format!("  - actualDigest={}", operation_registry_digest(actual)),
    ]
}

fn summarize_site_map(
    site_map: &Map<String, Value>,
) -> Result<(usize, usize, usize, usize), String> {
    let nodes = ensure_array(site_map.get("nodes"), "nodes")?;
    let edges = ensure_array(site_map.get("edges"), "edges")?;
    let covers = ensure_array(site_map.get("covers"), "covers")?;
    let operations = nodes
        .iter()
        .filter(|row| {
            row.as_object()
                .and_then(|node| node.get("kind"))
                .and_then(Value::as_str)
                == Some("operation")
        })
        .count();
    Ok((nodes.len(), edges.len(), covers.len(), operations))
}

fn validate_site_map(repo_root: &Path, site_map: &Map<String, Value>) -> Vec<String> {
    let mut errors = Vec::new();
    let doctrine_spec_rel =
        match ensure_string(site_map.get("doctrineSpecPath"), "doctrineSpecPath") {
            Ok(value) => value,
            Err(err) => return vec![err],
        };
    let doctrine_spec_path = repo_root.join(&doctrine_spec_rel);
    let doctrine_registry = if !doctrine_spec_path.exists() {
        errors.push(format!("missing doctrine spec path: {doctrine_spec_rel}"));
        BTreeSet::new()
    } else {
        match parse_registry(&doctrine_spec_path) {
            Ok(ids) => ids,
            Err(err) => {
                errors.push(err);
                BTreeSet::new()
            }
        }
    };

    let nodes_raw = match ensure_array(site_map.get("nodes"), "nodes") {
        Ok(value) => value,
        Err(err) => return vec![err],
    };
    let edges_raw = match ensure_array(site_map.get("edges"), "edges") {
        Ok(value) => value,
        Err(err) => return vec![err],
    };
    let covers_raw = match ensure_array(site_map.get("covers"), "covers") {
        Ok(value) => value,
        Err(err) => return vec![err],
    };

    let mut nodes: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    let mut parsed_declarations: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> =
        BTreeMap::new();
    let mut doctrine_root_id = String::new();
    let mut operation_ids = Vec::new();

    for (idx, node_raw) in nodes_raw.iter().enumerate() {
        let Some(node) = node_raw.as_object() else {
            errors.push(format!("nodes[{idx}]: object required"));
            continue;
        };
        let node_id = match ensure_string(node.get("id"), &format!("nodes[{idx}].id")) {
            Ok(value) => value,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let node_path_rel = match ensure_string(node.get("path"), &format!("nodes[{idx}].path")) {
            Ok(value) => value,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let node_kind = match ensure_string(node.get("kind"), &format!("nodes[{idx}].kind")) {
            Ok(value) => value,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let requires_decl = node
            .get("requiresDeclaration")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let node_path = repo_root.join(&node_path_rel);
        if !node_path.exists() {
            errors.push(format!("{node_id}: missing path '{node_path_rel}'"));
        }

        nodes.insert(node_id.clone(), node.clone());

        if node_kind == "doctrine" {
            if !doctrine_root_id.is_empty() {
                errors.push(format!(
                    "multiple doctrine roots found: '{doctrine_root_id}' and '{node_id}'"
                ));
            }
            doctrine_root_id = node_id.clone();
        }
        if node_kind == "operation" {
            operation_ids.push(node_id.clone());
        }

        if !requires_decl || !node_path.exists() {
            continue;
        }

        let (preserved_actual, not_preserved_actual) = match parse_declaration(&node_path) {
            Ok(value) => value,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        parsed_declarations.insert(
            node_id.clone(),
            (preserved_actual.clone(), not_preserved_actual.clone()),
        );

        if !doctrine_registry.is_empty() {
            let unknown = preserved_actual
                .union(&not_preserved_actual)
                .filter(|id| !doctrine_registry.contains(*id))
                .cloned()
                .collect::<Vec<_>>();
            if !unknown.is_empty() {
                errors.push(format!(
                    "{node_id}: declaration uses unknown doctrine morphism IDs: {unknown:?}"
                ));
            }
        }

        let Some(declared_map) = node.get("declares").and_then(Value::as_object) else {
            errors.push(format!(
                "{node_id}: requiresDeclaration=true but 'declares' object missing"
            ));
            continue;
        };
        let preserved_expected = match ensure_string_list(
            declared_map.get("preserved"),
            &format!("{node_id}.declares.preserved"),
            true,
        ) {
            Ok(value) => value.into_iter().collect::<BTreeSet<_>>(),
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let not_preserved_expected = match ensure_string_list(
            declared_map.get("notPreserved"),
            &format!("{node_id}.declares.notPreserved"),
            true,
        ) {
            Ok(value) => value.into_iter().collect::<BTreeSet<_>>(),
            Err(err) => {
                errors.push(err);
                continue;
            }
        };

        if !doctrine_registry.is_empty() {
            let unknown = preserved_expected
                .union(&not_preserved_expected)
                .filter(|id| !doctrine_registry.contains(*id))
                .cloned()
                .collect::<Vec<_>>();
            if !unknown.is_empty() {
                errors.push(format!(
                    "{node_id}: map declares unknown doctrine IDs: {unknown:?}"
                ));
            }
        }
        if preserved_expected != preserved_actual {
            errors.push(format!(
                "{node_id}: preserved mismatch map={:?} spec={:?}",
                preserved_expected, preserved_actual
            ));
        }
        if not_preserved_expected != not_preserved_actual {
            errors.push(format!(
                "{node_id}: notPreserved mismatch map={:?} spec={:?}",
                not_preserved_expected, not_preserved_actual
            ));
        }
    }

    if doctrine_root_id.is_empty() {
        errors.push("exactly one doctrine root node (kind='doctrine') is required".to_string());
    }
    if operation_ids.is_empty() {
        errors.push("at least one operation node (kind='operation') is required".to_string());
    }

    let mut edge_ids = BTreeSet::new();
    let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (idx, edge_raw) in edges_raw.iter().enumerate() {
        let Some(edge) = edge_raw.as_object() else {
            errors.push(format!("edges[{idx}]: object required"));
            continue;
        };
        let edge_id = match ensure_string(edge.get("id"), &format!("edges[{idx}].id")) {
            Ok(value) => value,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let from_id = match ensure_string(edge.get("from"), &format!("edges[{idx}].from")) {
            Ok(value) => value,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let to_id = match ensure_string(edge.get("to"), &format!("edges[{idx}].to")) {
            Ok(value) => value,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        let morphisms = match ensure_string_list(
            edge.get("morphisms"),
            &format!("edges[{idx}].morphisms"),
            true,
        ) {
            Ok(value) => value,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        if !edge_ids.insert(edge_id.clone()) {
            errors.push(format!("duplicate edge id: {edge_id}"));
            continue;
        }
        if !nodes.contains_key(&from_id) {
            errors.push(format!("{edge_id}: from node '{from_id}' is missing"));
        }
        if !nodes.contains_key(&to_id) {
            errors.push(format!("{edge_id}: to node '{to_id}' is missing"));
        }
        if !doctrine_registry.is_empty() {
            let unknown = morphisms
                .iter()
                .filter(|id| !doctrine_registry.contains(*id))
                .cloned()
                .collect::<Vec<_>>();
            if !unknown.is_empty() {
                errors.push(format!("{edge_id}: unknown morphism IDs: {unknown:?}"));
            }
        }
        if let Some((preserved, _)) = parsed_declarations.get(&to_id) {
            let missing = morphisms
                .iter()
                .filter(|morphism| !preserved.contains(*morphism))
                .cloned()
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                errors.push(format!(
                    "{edge_id}: morphisms not preserved by destination '{to_id}': {missing:?}"
                ));
            }
        }
        adjacency.entry(from_id).or_default().push(to_id);
    }

    let mut cover_ids = BTreeSet::new();
    for (idx, cover_raw) in covers_raw.iter().enumerate() {
        let Some(cover) = cover_raw.as_object() else {
            errors.push(format!("covers[{idx}]: object required"));
            continue;
        };
        let cover_id = match ensure_string(cover.get("id"), &format!("covers[{idx}].id")) {
            Ok(value) => value,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        if !cover_ids.insert(cover_id.clone()) {
            errors.push(format!("duplicate cover id: {cover_id}"));
            continue;
        }
        let over = match ensure_string(cover.get("over"), &format!("covers[{idx}].over")) {
            Ok(value) => value,
            Err(err) => {
                errors.push(err);
                continue;
            }
        };
        if !nodes.contains_key(&over) {
            errors.push(format!("{cover_id}: over node '{over}' is missing"));
        }
        let parts =
            match ensure_string_list(cover.get("parts"), &format!("covers[{idx}].parts"), true) {
                Ok(value) => value,
                Err(err) => {
                    errors.push(err);
                    continue;
                }
            };
        for part in parts {
            if !nodes.contains_key(&part) {
                errors.push(format!("{cover_id}: part node '{part}' is missing"));
            }
        }
    }

    if !doctrine_root_id.is_empty() && !operation_ids.is_empty() {
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(doctrine_root_id.clone());
        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(next_rows) = adjacency.get(&current) {
                for next in next_rows {
                    if !visited.contains(next) {
                        queue.push_back(next.clone());
                    }
                }
            }
        }
        for operation_id in operation_ids {
            if !visited.contains(&operation_id) {
                errors.push(format!(
                    "unreachable operation node '{operation_id}' from doctrine root '{doctrine_root_id}'"
                ));
            }
        }
    }
    errors
}

fn relative_display(path: &Path, repo_root: &Path) -> String {
    path.strip_prefix(repo_root)
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn digest_payload(
    repo_root: &Path,
    packages_root: &Path,
    input_map: &Path,
    site_map: &Path,
    operation_registry: &Path,
    cutover_contract: &Path,
    generated_input: &Map<String, Value>,
    generated_site: &Map<String, Value>,
    generated_registry: &Map<String, Value>,
) -> Map<String, Value> {
    let mut source = Map::new();
    source.insert(
        "packagesRoot".to_string(),
        Value::String(relative_display(packages_root, repo_root)),
    );
    source.insert(
        "packageGlob".to_string(),
        Value::String("**/SITE-PACKAGE.json".to_string()),
    );
    source.insert(
        "generator".to_string(),
        Value::String("tools/conformance/generate_doctrine_site.py".to_string()),
    );
    source.insert(
        "cutoverContract".to_string(),
        Value::String(relative_display(cutover_contract, repo_root)),
    );

    let mut site_input_row = Map::new();
    site_input_row.insert(
        "path".to_string(),
        Value::String(relative_display(input_map, repo_root)),
    );
    site_input_row.insert(
        "sha256".to_string(),
        Value::String(site_input_digest(generated_input)),
    );

    let mut site_map_row = Map::new();
    site_map_row.insert(
        "path".to_string(),
        Value::String(relative_display(site_map, repo_root)),
    );
    site_map_row.insert(
        "sha256".to_string(),
        Value::String(site_map_digest(generated_site)),
    );

    let mut operation_registry_row = Map::new();
    operation_registry_row.insert(
        "path".to_string(),
        Value::String(relative_display(operation_registry, repo_root)),
    );
    operation_registry_row.insert(
        "sha256".to_string(),
        Value::String(operation_registry_digest(generated_registry)),
    );

    let mut artifacts = Map::new();
    artifacts.insert("siteInput".to_string(), Value::Object(site_input_row));
    artifacts.insert("siteMap".to_string(), Value::Object(site_map_row));
    artifacts.insert(
        "operationRegistry".to_string(),
        Value::Object(operation_registry_row),
    );

    let mut payload = Map::new();
    payload.insert("schema".to_string(), json!(1));
    payload.insert(
        "digestKind".to_string(),
        Value::String(DOCTRINE_SITE_GENERATION_DIGEST_KIND.to_string()),
    );
    payload.insert("source".to_string(), Value::Object(source));
    payload.insert("artifacts".to_string(), Value::Object(artifacts));
    payload
}

fn prefix12(digest: &str) -> String {
    digest.chars().take(12).collect()
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    packages_root: String,
    site_map: String,
    input_map: String,
    operation_registry: String,
    digest_contract: String,
    cutover_contract: String,
    operation_registry_override: Option<String>,
    json_output: bool,
) {
    let repo_root = workspace_root();
    let packages_root = resolve_path(&PathBuf::from(packages_root), &repo_root);
    let site_map = resolve_path(&PathBuf::from(site_map), &repo_root);
    let input_map = resolve_path(&PathBuf::from(input_map), &repo_root);
    let operation_registry = resolve_path(&PathBuf::from(operation_registry), &repo_root);
    let digest_contract = resolve_path(&PathBuf::from(digest_contract), &repo_root);
    let cutover_contract = resolve_path(&PathBuf::from(cutover_contract), &repo_root);
    let operation_registry_override =
        operation_registry_override.map(|path| resolve_path(&PathBuf::from(path), &repo_root));

    ensure_path_exists(&packages_root, "packages root");
    ensure_path_exists(&site_map, "site map");
    ensure_path_exists(&input_map, "input map");
    ensure_path_exists(&operation_registry, "operation registry");
    ensure_path_exists(&digest_contract, "digest contract");
    ensure_path_exists(&cutover_contract, "cutover contract");
    if let Some(path) = &operation_registry_override {
        ensure_path_exists(path, "operation registry override");
    }

    let mut errors: Vec<String> = Vec::new();
    let mut summary: Option<DoctrineSiteSummary> = None;

    let cutover = match load_json_object(&cutover_contract, "cutover contract") {
        Ok(value) => value,
        Err(err) => {
            errors.push(err);
            Map::new()
        }
    };
    let cutover_policy = if errors.is_empty() {
        match parse_cutover_policy(&cutover, &cutover_contract.display().to_string()) {
            Ok(policy) => Some(policy),
            Err(err) => {
                errors.push(err);
                None
            }
        }
    } else {
        None
    };

    let generated_input = if let Some(cutover_policy) = cutover_policy.as_ref() {
        match generate_site_input_from_packages(
            &repo_root,
            &packages_root,
            cutover_policy,
            operation_registry_override.as_deref(),
        ) {
            Ok(value) => Some(value),
            Err(err) => {
                errors.push(err);
                None
            }
        }
    } else {
        None
    };

    let tracked_input = match load_json_object(&input_map, "tracked doctrine site input") {
        Ok(value) => match canonicalize_site_input(&value) {
            Ok(canonical) => Some(canonical),
            Err(err) => {
                errors.push(err);
                None
            }
        },
        Err(err) => {
            errors.push(err);
            None
        }
    };

    let generated_site = if let Some(input) = generated_input.as_ref() {
        match generate_site_map(&repo_root, input) {
            Ok(value) => Some(value),
            Err(err) => {
                errors.push(err);
                None
            }
        }
    } else {
        None
    };

    let generated_registry = generated_input.as_ref().and_then(|input| {
        ensure_object(
            input.get("operationRegistry"),
            "DOCTRINE-SITE-INPUT.operationRegistry",
        )
        .ok()
        .cloned()
    });

    let tracked_site = match load_json_object(&site_map, "tracked doctrine site map") {
        Ok(value) => match canonicalize_site_map(&value) {
            Ok(canonical) => Some(canonical),
            Err(err) => {
                errors.push(err);
                None
            }
        },
        Err(err) => {
            errors.push(err);
            None
        }
    };

    let tracked_registry =
        match load_json_object(&operation_registry, "tracked doctrine operation registry") {
            Ok(value) => match canonicalize_operation_registry(&value) {
                Ok(canonical) => Some(canonical),
                Err(err) => {
                    errors.push(err);
                    None
                }
            },
            Err(err) => {
                errors.push(err);
                None
            }
        };

    let tracked_digest_contract =
        match load_json_object(&digest_contract, "tracked doctrine generation digest") {
            Ok(value) => Some(value),
            Err(err) => {
                errors.push(err);
                None
            }
        };

    if let (Some(generated), Some(tracked)) = (generated_input.as_ref(), tracked_input.as_ref()) {
        errors.extend(site_input_equality_diff(generated, tracked));
    }
    if let (Some(generated), Some(tracked)) = (generated_site.as_ref(), tracked_site.as_ref()) {
        errors.extend(site_map_equality_diff(generated, tracked));
    }
    if let (Some(generated), Some(tracked)) =
        (generated_registry.as_ref(), tracked_registry.as_ref())
    {
        errors.extend(operation_registry_equality_diff(generated, tracked));
    }

    if let (
        Some(generated_input),
        Some(generated_site),
        Some(generated_registry),
        Some(tracked_digest),
    ) = (
        generated_input.as_ref(),
        generated_site.as_ref(),
        generated_registry.as_ref(),
        tracked_digest_contract.as_ref(),
    ) {
        let expected_digest = digest_payload(
            &repo_root,
            &packages_root,
            &input_map,
            &site_map,
            &operation_registry,
            &cutover_contract,
            generated_input,
            generated_site,
            generated_registry,
        );
        if canonical_json_string(&Value::Object(expected_digest.clone()))
            != canonical_json_string(&Value::Object(tracked_digest.clone()))
        {
            errors.push(
                "tracked doctrine generation digest differs from generated output".to_string(),
            );
        }
    }

    if let Some(site) = tracked_site.as_ref() {
        errors.extend(validate_site_map(&repo_root, site));
    }

    if let (Some(input), Some(registry)) = (tracked_input.as_ref(), tracked_registry.as_ref()) {
        let operations_value = Value::Object(registry.clone());
        match parse_operation_route_rows(&operations_value) {
            Ok(operation_rows) => {
                let report =
                    validate_world_route_bindings(&Value::Object(input.clone()), &operation_rows);
                if report.result != "accepted" {
                    for issue in report.issues {
                        errors.push(format!(
                            "{}: {} ({})",
                            issue.path, issue.message, issue.failure_class
                        ));
                    }
                }
            }
            Err(err) => errors.push(format!("world route operation parsing failed: {err}")),
        }
    }

    if let (Some(site), Some(registry)) = (tracked_site.as_ref(), tracked_registry.as_ref()) {
        if let Ok((nodes, edges, covers, operations)) = summarize_site_map(site) {
            summary = Some(DoctrineSiteSummary {
                nodes,
                edges,
                covers,
                operations,
                site_digest: site_map_digest(site),
                registry_digest: operation_registry_digest(registry),
            });
        }
    }

    let accepted = errors.is_empty();
    let result = if accepted { "accepted" } else { "rejected" };
    let failure_classes: Vec<&str> = if accepted {
        Vec::new()
    } else {
        vec![FAILURE_CLASS_VIOLATION]
    };

    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": CHECK_KIND,
            "result": result,
            "failureClasses": failure_classes,
            "nodes": summary.as_ref().map(|row| row.nodes),
            "edges": summary.as_ref().map(|row| row.edges),
            "covers": summary.as_ref().map(|row| row.covers),
            "operations": summary.as_ref().map(|row| row.operations),
            "siteDigest": summary.as_ref().map(|row| row.site_digest.clone()),
            "registryDigest": summary.as_ref().map(|row| row.registry_digest.clone()),
            "errors": errors,
            "stdoutLines": Vec::<String>::new(),
            "stderrLines": Vec::<String>::new(),
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render doctrine-site-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else if accepted {
        if let Some(summary) = summary {
            println!(
                "[ok] doctrine site check passed (nodes={}, edges={}, covers={}, operations={}, siteDigest={}, registryDigest={})",
                summary.nodes,
                summary.edges,
                summary.covers,
                summary.operations,
                prefix12(&summary.site_digest),
                prefix12(&summary.registry_digest),
            );
        } else {
            println!("[ok] doctrine site check passed");
        }
    } else {
        for error in &errors {
            println!("[error] {error}");
        }
        println!(
            "[fail] doctrine site check failed (errors={})",
            errors.len()
        );
    }

    if !accepted {
        std::process::exit(1);
    }
}
