use premath_bd::{DepType, MemoryStore, store_snapshot_ref};
use premath_jj::JjClient;
use premath_kernel::{CoherenceLevel, ContextId, FiberSignature};
use premath_surreal::QueryCache;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFLICT_SAMPLE_LIMIT: usize = 25;
pub const DEFAULT_ISSUES_PATH: &str = ".premath/issues.jsonl";
pub const ISSUE_QUERY_PROJECTION_SCHEMA: u64 = 1;
pub const ISSUE_QUERY_PROJECTION_KIND: &str = "premath.surreal.issue_projection.v0";
const DEFAULT_ISSUES_SAMPLE_PATH: &str = ".premath/issues.jsonl.new";
const LEGACY_ISSUES_PATH: &str = ".beads/issues.jsonl";
const LEGACY_ISSUES_SAMPLE_PATH: &str = ".beads/issues.jsonl.new";

#[derive(Debug, Clone)]
pub struct BackendStatus {
    pub issues_path: PathBuf,
    pub repo_root: PathBuf,
    pub projection_path: PathBuf,
    pub issues_exists: bool,
    pub authority_snapshot_ref: Option<String>,
    pub authority_error: Option<String>,
    pub projection_exists: bool,
    pub projection_state: &'static str,
    pub projection_schema: Option<u64>,
    pub projection_kind: Option<String>,
    pub projection_source_issues_path: Option<String>,
    pub projection_source_path_matches_authority: Option<bool>,
    pub projection_source_snapshot_ref: Option<String>,
    pub projection_snapshot_matches_authority: Option<bool>,
    pub projection_error: Option<String>,
    pub jj_state: &'static str,
    pub jj_available: bool,
    pub jj_repo_root: Option<String>,
    pub jj_head_change_id: Option<String>,
    pub jj_error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectionHeader {
    #[serde(default)]
    schema: Option<u64>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    source_issues_path: Option<String>,
    #[serde(default)]
    source_snapshot_ref: Option<String>,
}

pub fn parse_level_or_exit(level: &str) -> CoherenceLevel {
    level.parse().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    })
}

pub fn load_store_or_exit(issues_arg: &str) -> (MemoryStore, PathBuf) {
    let path = resolve_issues_path_or_exit(issues_arg);
    let store = MemoryStore::load_jsonl(&path).unwrap_or_else(|e| {
        eprintln!("error: failed to load {}: {e}", path.display());
        std::process::exit(1);
    });
    (store, path)
}

fn resolve_issues_path_or_exit(issues_arg: &str) -> PathBuf {
    let requested = PathBuf::from(issues_arg);
    if requested.exists() {
        return requested;
    }

    // Convenience fallback for sample or legacy issue stores.
    let fallbacks: Vec<&str> = match issues_arg {
        DEFAULT_ISSUES_PATH => vec![
            DEFAULT_ISSUES_SAMPLE_PATH,
            LEGACY_ISSUES_PATH,
            LEGACY_ISSUES_SAMPLE_PATH,
        ],
        LEGACY_ISSUES_PATH => vec![LEGACY_ISSUES_SAMPLE_PATH],
        _ => Vec::new(),
    };

    for fallback in fallbacks {
        let path = PathBuf::from(fallback);
        if path.exists() {
            return path;
        }
    }

    eprintln!("error: issues file not found: {}", requested.display());
    std::process::exit(1);
}

pub fn scope_ids_or_exit(cache: &QueryCache, scope: &str) -> Vec<String> {
    select_scope_ids(cache, scope).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    })
}

fn select_scope_ids(cache: &QueryCache, scope: &str) -> Result<Vec<String>, String> {
    if scope == "all" {
        return Ok(cache.issue_ids());
    }
    if cache.issue(scope).is_none() {
        return Err(format!(
            "scope root `{scope}` not found; use an issue ID or `all`"
        ));
    }

    // Scope selection strategy: root + all descendants via parent-child edges.
    let mut selected = BTreeSet::new();
    let mut queue = VecDeque::new();
    selected.insert(scope.to_string());
    queue.push_back(scope.to_string());

    while let Some(current) = queue.pop_front() {
        for dep in cache.dependents_of(&current) {
            if dep.dep_type == DepType::ParentChild && dep.depends_on_id == current {
                let child = dep.issue_id.clone();
                if selected.insert(child.clone()) {
                    queue.push_back(child);
                }
            }
        }
    }

    Ok(selected.into_iter().collect())
}

pub fn fibers_or_exit(
    cache: &QueryCache,
    ids: &[String],
    context: &ContextId,
) -> Vec<FiberSignature> {
    build_fibers(cache, ids, context).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    })
}

fn build_fibers(
    cache: &QueryCache,
    ids: &[String],
    context: &ContextId,
) -> Result<Vec<FiberSignature>, String> {
    let mut fibers = Vec::with_capacity(ids.len());
    for id in ids {
        let issue = cache
            .issue(id)
            .ok_or_else(|| format!("issue missing from cache while building fibers: {id}"))?;
        fibers.push(issue.fiber_signature(context));
    }
    Ok(fibers)
}

pub fn maybe_jj_snapshot(repo: &str) -> Option<serde_json::Value> {
    let repo_path = Path::new(repo);
    let client = JjClient::discover(repo_path).ok()?;
    let snapshot = client.snapshot().ok()?;
    Some(json!({
        "repo_root": snapshot.repo_root.display().to_string(),
        "change_id": snapshot.change_id,
        "status": snapshot.status,
    }))
}

pub fn collect_backend_status(
    issues_path: impl AsRef<Path>,
    repo_root: impl AsRef<Path>,
    projection_path: impl AsRef<Path>,
) -> BackendStatus {
    let issues_path = issues_path.as_ref().to_path_buf();
    let repo_root = repo_root.as_ref().to_path_buf();
    let projection_path = projection_path.as_ref().to_path_buf();

    let issues_exists = issues_path.exists();
    let projection_exists = projection_path.exists();
    let mut authority_snapshot_ref: Option<String> = None;
    let mut authority_error: Option<String> = None;
    if issues_exists {
        match MemoryStore::load_jsonl(&issues_path) {
            Ok(store) => authority_snapshot_ref = Some(store_snapshot_ref(&store)),
            Err(err) => authority_error = Some(err.to_string()),
        }
    }

    let mut projection_state: &'static str = if projection_exists {
        "unknown"
    } else {
        "missing"
    };
    let mut projection_schema: Option<u64> = None;
    let mut projection_kind: Option<String> = None;
    let mut projection_source_issues_path: Option<String> = None;
    let mut projection_source_path_matches_authority: Option<bool> = None;
    let mut projection_source_snapshot_ref: Option<String> = None;
    let mut projection_snapshot_matches_authority: Option<bool> = None;
    let mut projection_error: Option<String> = None;

    if projection_exists {
        match read_projection_header(&projection_path) {
            Ok(header) => {
                projection_schema = header.schema;
                projection_kind = header.kind.clone();
                projection_source_issues_path = header.source_issues_path.clone();
                projection_source_snapshot_ref = header.source_snapshot_ref.clone();

                let schema_ok = projection_schema == Some(ISSUE_QUERY_PROJECTION_SCHEMA);
                let kind_ok = projection_kind.as_deref() == Some(ISSUE_QUERY_PROJECTION_KIND);
                if !schema_ok || !kind_ok {
                    projection_state = "invalid";
                    projection_error = Some(format!(
                        "projection metadata mismatch (expected schema={} kind={}, actual schema={:?} kind={:?})",
                        ISSUE_QUERY_PROJECTION_SCHEMA,
                        ISSUE_QUERY_PROJECTION_KIND,
                        projection_schema,
                        projection_kind
                    ));
                } else {
                    if let Some(source_path) = projection_source_issues_path.as_deref() {
                        projection_source_path_matches_authority =
                            Some(paths_match(source_path, &issues_path));
                    }
                    if let (Some(source_ref), Some(authority_ref)) = (
                        projection_source_snapshot_ref.as_deref(),
                        authority_snapshot_ref.as_deref(),
                    ) {
                        projection_snapshot_matches_authority = Some(source_ref == authority_ref);
                    }

                    projection_state = match (
                        projection_source_path_matches_authority,
                        projection_snapshot_matches_authority,
                    ) {
                        (Some(false), _) | (_, Some(false)) => "stale",
                        (Some(true), Some(true)) => "fresh",
                        _ => "unknown",
                    };
                }
            }
            Err(err) => {
                projection_state = "invalid";
                projection_error = Some(err);
            }
        }
    }

    let jj_available = JjClient::is_available();
    let mut jj_repo_root: Option<String> = None;
    let mut jj_head_change_id: Option<String> = None;
    let mut jj_error: Option<String> = None;

    if jj_available {
        match JjClient::discover(&repo_root) {
            Ok(client) => {
                jj_repo_root = Some(client.repo_root().display().to_string());
                match client.current_change_id() {
                    Ok(change_id) => {
                        jj_head_change_id = Some(change_id);
                    }
                    Err(err) => {
                        jj_error = Some(err.to_string());
                    }
                }
            }
            Err(err) => {
                jj_error = Some(err.to_string());
            }
        }
    }

    let jj_state = if !jj_available {
        "unavailable"
    } else if jj_error.is_none() {
        "ready"
    } else {
        "error"
    };

    BackendStatus {
        issues_path,
        repo_root,
        projection_path,
        issues_exists,
        authority_snapshot_ref,
        authority_error,
        projection_exists,
        projection_state,
        projection_schema,
        projection_kind,
        projection_source_issues_path,
        projection_source_path_matches_authority,
        projection_source_snapshot_ref,
        projection_snapshot_matches_authority,
        projection_error,
        jj_state,
        jj_available,
        jj_repo_root,
        jj_head_change_id,
        jj_error,
    }
}

pub fn backend_status_payload(
    action: &str,
    status: &BackendStatus,
    query_backend: Option<&str>,
) -> Value {
    let mut payload = json!({
        "action": action,
        "issuesPath": status.issues_path.display().to_string(),
        "repoRoot": status.repo_root.display().to_string(),
        "canonicalMemory": {
            "kind": "jsonl",
            "path": status.issues_path.display().to_string(),
            "exists": status.issues_exists,
            "snapshotRef": status.authority_snapshot_ref.clone(),
            "error": status.authority_error.clone()
        },
        "queryProjection": {
            "kind": ISSUE_QUERY_PROJECTION_KIND,
            "path": status.projection_path.display().to_string(),
            "exists": status.projection_exists,
            "state": status.projection_state,
            "schema": status.projection_schema,
            "projectionKind": status.projection_kind.clone(),
            "sourceIssuesPath": status.projection_source_issues_path.clone(),
            "sourcePathMatchesAuthority": status.projection_source_path_matches_authority,
            "sourceSnapshotRef": status.projection_source_snapshot_ref.clone(),
            "snapshotRefMatchesAuthority": status.projection_snapshot_matches_authority,
            "error": status.projection_error.clone()
        },
        "jj": {
            "state": status.jj_state,
            "available": status.jj_available,
            "repoRoot": status.jj_repo_root.clone(),
            "headChangeId": status.jj_head_change_id.clone(),
            "error": status.jj_error.clone()
        }
    });

    if let Some(query_backend) = query_backend
        && let Value::Object(ref mut map) = payload
    {
        map.insert(
            "queryBackend".to_string(),
            Value::String(query_backend.to_string()),
        );
    }

    payload
}

fn read_projection_header(path: &Path) -> Result<ProjectionHeader, String> {
    let bytes = fs::read(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    serde_json::from_slice::<ProjectionHeader>(&bytes)
        .map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

fn paths_match(source_path: &str, authority_path: &Path) -> bool {
    let source = PathBuf::from(source_path);
    if source == authority_path {
        return true;
    }
    normalize_path(&source) == normalize_path(authority_path)
}

fn normalize_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    }
    let joined = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(path);
    fs::canonicalize(&joined).unwrap_or(joined)
}

pub fn read_json_file_or_exit<T>(path: &str, label: &str) -> T
where
    T: serde::de::DeserializeOwned,
{
    let bytes = fs::read(path).unwrap_or_else(|e| {
        eprintln!("error: failed to read {label} at {}: {e}", path);
        std::process::exit(1);
    });
    serde_json::from_slice::<T>(&bytes).unwrap_or_else(|e| {
        eprintln!("error: failed to parse {label} JSON at {}: {e}", path);
        std::process::exit(1);
    })
}

pub fn sample_with_truncation<T>(items: Vec<T>, limit: usize) -> (Vec<T>, usize) {
    let total = items.len();
    let sample: Vec<T> = items.into_iter().take(limit).collect();
    let truncated = total.saturating_sub(sample.len());
    (sample, truncated)
}

pub fn print_sample_block(header: &str, items: &[String], truncated: usize) {
    if items.is_empty() {
        return;
    }

    println!("  {header} (showing up to {}):", items.len());
    for item in items {
        println!("    - {item}");
    }
    if truncated > 0 {
        println!("    - ... and {truncated} more");
    }
}

pub fn yes_no(ok: bool) -> &'static str {
    if ok { "yes" } else { "no" }
}
