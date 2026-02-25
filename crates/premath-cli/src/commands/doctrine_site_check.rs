use regex::Regex;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

const CHECK_KIND: &str = "ci.doctrine_site_check.v1";
const FAILURE_CLASS_VIOLATION: &str = "doctrine_site_violation";

#[derive(Debug, Clone)]
struct DoctrineSiteSummary {
    nodes: usize,
    edges: usize,
    covers: usize,
    operations: usize,
    site_digest: String,
    registry_digest: String,
}

fn parse_summary(stdout: &str) -> Option<DoctrineSiteSummary> {
    let re = Regex::new(
        r"^\[ok\] doctrine site check passed \(nodes=(\d+), edges=(\d+), covers=(\d+), operations=(\d+), siteDigest=([0-9a-f]+), registryDigest=([0-9a-f]+)\)$",
    )
    .ok()?;
    for line in stdout.lines().rev() {
        let trimmed = line.trim();
        let Some(captures) = re.captures(trimmed) else {
            continue;
        };
        let nodes = captures.get(1)?.as_str().parse::<usize>().ok()?;
        let edges = captures.get(2)?.as_str().parse::<usize>().ok()?;
        let covers = captures.get(3)?.as_str().parse::<usize>().ok()?;
        let operations = captures.get(4)?.as_str().parse::<usize>().ok()?;
        let site_digest = captures.get(5)?.as_str().to_string();
        let registry_digest = captures.get(6)?.as_str().to_string();
        return Some(DoctrineSiteSummary {
            nodes,
            edges,
            covers,
            operations,
            site_digest,
            registry_digest,
        });
    }
    None
}

fn collect_error_lines(stdout: &str, stderr: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("[error] ") {
            out.push(value.to_string());
        }
    }
    if out.is_empty() {
        for line in stderr.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    }
    if out.is_empty() {
        for line in stdout.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

fn normalize_nonempty_lines(input: &str) -> Vec<String> {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
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
    let script_path = resolve_path(
        &PathBuf::from("tools/conformance/check_doctrine_site.py"),
        &repo_root,
    );
    ensure_path_exists(&script_path, "checker script");
    let packages_root = resolve_path(&PathBuf::from(packages_root), &repo_root);
    let site_map = resolve_path(&PathBuf::from(site_map), &repo_root);
    let input_map = resolve_path(&PathBuf::from(input_map), &repo_root);
    let operation_registry = resolve_path(&PathBuf::from(operation_registry), &repo_root);
    let digest_contract = resolve_path(&PathBuf::from(digest_contract), &repo_root);
    let cutover_contract = resolve_path(&PathBuf::from(cutover_contract), &repo_root);
    ensure_path_exists(&packages_root, "packages root");
    ensure_path_exists(&site_map, "site map");
    ensure_path_exists(&input_map, "input map");
    ensure_path_exists(&operation_registry, "operation registry");
    ensure_path_exists(&digest_contract, "digest contract");
    ensure_path_exists(&cutover_contract, "cutover contract");
    let operation_registry_override =
        operation_registry_override.map(|path| resolve_path(&PathBuf::from(path), &repo_root));
    if let Some(path) = &operation_registry_override {
        ensure_path_exists(path, "operation registry override");
    }

    let mut command = Command::new("python3");
    command.arg(&script_path);
    command.arg("--packages-root").arg(&packages_root);
    command.arg("--site-map").arg(&site_map);
    command.arg("--input-map").arg(&input_map);
    command.arg("--operation-registry").arg(&operation_registry);
    command.arg("--digest-contract").arg(&digest_contract);
    command.arg("--cutover-contract").arg(&cutover_contract);
    if let Some(path) = &operation_registry_override {
        command.arg("--operation-registry-override").arg(path);
    }
    let output = command.output().unwrap_or_else(|err| {
        eprintln!("[doctrine-site-check] ERROR: failed to execute checker script: {err}");
        std::process::exit(2);
    });

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let accepted = output.status.success();
    let result = if accepted { "accepted" } else { "rejected" };
    let failure_classes: Vec<&str> = if accepted {
        Vec::new()
    } else {
        vec![FAILURE_CLASS_VIOLATION]
    };
    let errors = collect_error_lines(&stdout, &stderr);
    let summary = parse_summary(&stdout);

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
            "stdoutLines": normalize_nonempty_lines(&stdout),
            "stderrLines": normalize_nonempty_lines(&stderr),
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render doctrine-site-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else {
        if !stdout.is_empty() {
            print!("{stdout}");
            if !stdout.ends_with('\n') {
                println!();
            }
        }
        if !stderr.is_empty() {
            eprint!("{stderr}");
            if !stderr.ends_with('\n') {
                eprintln!();
            }
        }
    }

    if !accepted {
        std::process::exit(1);
    }
}
