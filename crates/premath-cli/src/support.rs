use premath_bd::{DepType, MemoryStore};
use premath_jj::JjClient;
use premath_kernel::{CoherenceLevel, ContextId, FiberSignature};
use premath_surreal::QueryCache;
use serde_json::json;
use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFLICT_SAMPLE_LIMIT: usize = 25;
pub const DEFAULT_ISSUES_PATH: &str = ".premath/issues.jsonl";
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
    pub projection_exists: bool,
    pub jj_state: &'static str,
    pub jj_available: bool,
    pub jj_repo_root: Option<String>,
    pub jj_head_change_id: Option<String>,
    pub jj_error: Option<String>,
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
        projection_exists,
        jj_state,
        jj_available,
        jj_repo_root,
        jj_head_change_id,
        jj_error,
    }
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
