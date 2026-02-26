use regex::Regex;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const CHECK_KIND: &str = "ci.docs_coherence_check.v1";
const FAILURE_CLASS_VIOLATION: &str = "docs_coherence_violation";
const CAPABILITY_REGISTRY_KIND: &str = "premath.capability_registry.v1";
const README_DOCTRINE_MARKERS: [&str; 2] = [
    "doctrine-to-operation site coherence",
    "mise run doctrine-check",
];
const ARCHITECTURE_DOCTRINE_MARKERS: [&str; 3] = [
    "`premath doctrine-site-check`",
    "`premath runtime-orchestration-check`",
    "`premath doctrine-mcp-parity-check`",
];
const CI_CLOSURE_DOCTRINE_MARKERS: [&str; 2] = [
    "`doctrine-check` (site coherence + runtime orchestration route parity +",
    "doctrine-inf vectors)",
];
const EXPECTED_DOCTRINE_CHECK_COMMANDS: [&str; 4] = [
    "cargo run --package premath-cli -- doctrine-site-check --json",
    "cargo run --package premath-cli -- runtime-orchestration-check --control-plane-contract specs/premath/draft/CONTROL-PLANE-CONTRACT.json --doctrine-op-registry specs/premath/draft/DOCTRINE-OP-REGISTRY.json --harness-runtime specs/premath/draft/HARNESS-RUNTIME.md --doctrine-site-input specs/premath/draft/DOCTRINE-SITE-INPUT.json --json",
    "cargo run --package premath-cli -- doctrine-mcp-parity-check --mcp-source crates/premath-cli/src/commands/mcp_serve.rs --registry specs/premath/draft/DOCTRINE-OP-REGISTRY.json --json",
    "python3 tools/conformance/run_fixture_suites.py --suite doctrine-inf",
];

#[derive(Debug, Clone)]
struct DocsCoherenceSummary {
    capabilities: usize,
    baseline_tasks: usize,
    projection_checks: usize,
    doctrine_checks: usize,
}

#[derive(Debug, Clone)]
struct DocsCoherenceEvaluation {
    summary: DocsCoherenceSummary,
    errors: Vec<String>,
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
            "[docs-coherence-check] ERROR: {label} missing: {}",
            path.display()
        );
        std::process::exit(2);
    }
}

fn load_text(path: &Path, label: &str) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|err| format!("{label}: failed reading {}: {err}", path.display()))
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

fn parse_string_list(value: Option<&Value>, label: &str) -> Result<Vec<String>, String> {
    let Some(rows) = value.and_then(Value::as_array) else {
        return Err(format!("{label}: list required"));
    };
    let mut out = Vec::with_capacity(rows.len());
    for (idx, row) in rows.iter().enumerate() {
        let Some(parsed) = row
            .as_str()
            .map(str::trim)
            .filter(|token| !token.is_empty())
        else {
            return Err(format!("{label}[{idx}]: non-empty string required"));
        };
        out.push(parsed.to_string());
    }
    Ok(out)
}

fn parse_capability_registry(path: &Path) -> Result<Vec<String>, String> {
    let payload = load_json_object(path, "capability registry")?;
    if payload.get("schema").and_then(Value::as_u64) != Some(1) {
        return Err(format!("{}: schema must equal 1", path.display()));
    }
    let Some(kind) = payload
        .get("registryKind")
        .and_then(Value::as_str)
        .map(str::trim)
    else {
        return Err(format!("{}: registryKind string required", path.display()));
    };
    if kind != CAPABILITY_REGISTRY_KIND {
        return Err(format!(
            "{}: registryKind must equal {:?}",
            path.display(),
            CAPABILITY_REGISTRY_KIND
        ));
    }
    parse_string_list(
        payload.get("executableCapabilities"),
        "executableCapabilities",
    )
}

fn sorted_csv(values: &BTreeSet<String>) -> String {
    values.iter().cloned().collect::<Vec<_>>().join(", ")
}

fn backtick_capabilities(text: &str) -> BTreeSet<String> {
    let re = Regex::new(r"`(capabilities\.[a-z0-9_]+)`").expect("capability regex");
    re.captures_iter(text)
        .filter_map(|row| row.get(1).map(|value| value.as_str().to_string()))
        .collect()
}

fn parse_readme_workspace_crates(text: &str) -> BTreeSet<String> {
    let re = Regex::new(r"`(crates/premath-[a-z0-9_-]+)`").expect("workspace crate regex");
    re.captures_iter(text)
        .filter_map(|row| row.get(1).map(|value| value.as_str().to_string()))
        .collect()
}

fn parse_workspace_members(cargo_toml_text: &str) -> BTreeSet<String> {
    let re = Regex::new(r#""(crates/premath-[a-z0-9_-]+)""#).expect("workspace members regex");
    re.captures_iter(cargo_toml_text)
        .filter_map(|row| row.get(1).map(|value| value.as_str().to_string()))
        .collect()
}

fn extract_heading_section(text: &str, heading_prefix: &str) -> Result<String, String> {
    let heading_re = Regex::new(&format!(r"(?m)^### {}\b.*$", regex::escape(heading_prefix)))
        .map_err(|err| format!("failed compiling heading regex: {err}"))?;
    let Some(heading) = heading_re.find(text) else {
        return Err(format!("missing heading: {heading_prefix:?}"));
    };
    let section_start = heading.end();
    let tail = &text[section_start..];
    let next_heading_re =
        Regex::new(r"(?m)^### ").map_err(|err| format!("failed compiling section regex: {err}"))?;
    let section = if let Some(next_heading) = next_heading_re.find(tail) {
        &tail[..next_heading.start()]
    } else {
        tail
    };
    Ok(section.to_string())
}

fn extract_section_between(
    text: &str,
    start_marker: &str,
    end_marker: &str,
) -> Result<String, String> {
    let Some(start) = text.find(start_marker) else {
        return Err(format!("missing start marker: {start_marker:?}"));
    };
    let tail = &text[start + start_marker.len()..];
    let Some(end) = tail.find(end_marker) else {
        return Err(format!(
            "missing end marker after start marker {start_marker:?}: {end_marker:?}"
        ));
    };
    Ok(tail[..end].to_string())
}

fn parse_mise_task_commands(text: &str, task_name: &str) -> Result<Vec<String>, String> {
    let section_marker = format!("[tasks.{task_name}]");
    let Some(section_start) = text.find(&section_marker) else {
        return Err(format!("missing [tasks.{task_name}] section"));
    };
    let section_tail = &text[section_start + section_marker.len()..];
    let section_body = if let Some(next_task_offset) = section_tail.find("\n[tasks.") {
        &section_tail[..next_task_offset]
    } else {
        section_tail
    };

    let Some(run_start) = section_body.find("run") else {
        return Err(format!("[tasks.{task_name}] missing run list"));
    };
    let run_tail = &section_body[run_start..];
    let Some(list_start_rel) = run_tail.find('[') else {
        return Err(format!("[tasks.{task_name}] missing run list"));
    };
    let run_list_tail = &run_tail[list_start_rel + 1..];
    let Some(list_end_rel) = run_list_tail.find(']') else {
        return Err(format!(
            "[tasks.{task_name}] missing closing ] for run list"
        ));
    };
    let run_body = &run_list_tail[..list_end_rel];

    let command_re = Regex::new(r#""([^"]+)""#)
        .map_err(|err| format!("failed compiling command regex: {err}"))?;
    let commands = command_re
        .captures_iter(run_body)
        .filter_map(|row| row.get(1).map(|value| value.as_str().to_string()))
        .collect::<Vec<_>>();
    if commands.is_empty() {
        return Err(format!("[tasks.{task_name}] run list has no commands"));
    }
    Ok(commands)
}

fn parse_baseline_task_ids_from_commands(commands: &[String]) -> Vec<String> {
    commands
        .iter()
        .filter_map(|command| {
            let trimmed = command.trim();
            trimmed
                .strip_prefix("mise run ")
                .map(|tail| {
                    tail.split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_string()
                })
                .filter(|value| !value.is_empty())
        })
        .collect()
}

fn parse_task_tokens(text: &str) -> BTreeSet<String> {
    let re = Regex::new(r"`([a-z][a-z0-9-]*)`").expect("task token regex");
    re.captures_iter(text)
        .filter_map(|row| row.get(1).map(|value| value.as_str().to_string()))
        .collect()
}

fn parse_control_plane_projection_checks(path: &Path) -> Result<Vec<String>, String> {
    let payload = load_json_object(path, "control-plane contract")?;
    let Some(required_gate_projection) = payload
        .get("requiredGateProjection")
        .and_then(Value::as_object)
    else {
        return Err(format!(
            "{}: requiredGateProjection object missing",
            path.display()
        ));
    };
    parse_string_list(
        required_gate_projection.get("checkOrder"),
        "requiredGateProjection.checkOrder",
    )
}

fn find_missing_markers(text: &str, markers: &[&str]) -> Vec<String> {
    markers
        .iter()
        .filter(|marker| !text.contains(**marker))
        .map(|marker| marker.to_string())
        .collect()
}

fn evaluate(repo_root: &Path) -> Result<DocsCoherenceEvaluation, String> {
    let readme = repo_root.join("README.md");
    let cargo_toml = repo_root.join("Cargo.toml");
    let mise_toml = repo_root.join(".mise.toml");
    let ci_closure = repo_root.join("docs/design/CI-CLOSURE.md");
    let architecture_map = repo_root.join("docs/design/ARCHITECTURE-MAP.md");
    let conformance_readme = repo_root.join("tools/conformance/README.md");
    let spec_index = repo_root.join("specs/premath/draft/SPEC-INDEX.md");
    let capability_registry = repo_root.join("specs/premath/draft/CAPABILITY-REGISTRY.json");
    let control_plane_contract = repo_root.join("specs/premath/draft/CONTROL-PLANE-CONTRACT.json");

    for (path, label) in [
        (&readme, "README"),
        (&cargo_toml, "Cargo.toml"),
        (&mise_toml, ".mise.toml"),
        (&ci_closure, "CI-CLOSURE doc"),
        (&architecture_map, "ARCHITECTURE-MAP doc"),
        (&conformance_readme, "conformance README"),
        (&spec_index, "SPEC-INDEX"),
        (&capability_registry, "CAPABILITY-REGISTRY"),
        (&control_plane_contract, "CONTROL-PLANE-CONTRACT"),
    ] {
        if !path.exists() {
            return Err(format!("{label} missing: {}", path.display()));
        }
    }

    let executable_capabilities = parse_capability_registry(&capability_registry)?;
    let executable_capability_set = executable_capabilities
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    let readme_text = load_text(&readme, "README")?;
    let conformance_readme_text = load_text(&conformance_readme, "conformance README")?;
    let architecture_text = load_text(&architecture_map, "ARCHITECTURE-MAP")?;
    let spec_index_text = load_text(&spec_index, "SPEC-INDEX")?;
    let cargo_toml_text = load_text(&cargo_toml, "Cargo.toml")?;
    let mise_toml_text = load_text(&mise_toml, ".mise.toml")?;
    let ci_closure_text = load_text(&ci_closure, "CI-CLOSURE")?;

    let mut errors = Vec::new();

    let readme_caps = backtick_capabilities(&readme_text);
    if readme_caps != executable_capability_set {
        errors.push(format!(
            "README capability list mismatch with executable capabilities: expected=[{}], got=[{}]",
            sorted_csv(&executable_capability_set),
            sorted_csv(&readme_caps),
        ));
    }

    let conformance_caps = backtick_capabilities(&conformance_readme_text);
    if conformance_caps != executable_capability_set {
        errors.push(format!(
            "tools/conformance/README capability list mismatch with executable capabilities: expected=[{}], got=[{}]",
            sorted_csv(&executable_capability_set),
            sorted_csv(&conformance_caps),
        ));
    }

    let section_54 = extract_heading_section(&spec_index_text, "5.4")?;
    let spec_index_caps = backtick_capabilities(&section_54);
    if spec_index_caps != executable_capability_set {
        errors.push(format!(
            "SPEC-INDEX §5.4 capability list mismatch with executable capabilities: expected=[{}], got=[{}]",
            sorted_csv(&executable_capability_set),
            sorted_csv(&spec_index_caps),
        ));
    }

    let readme_workspace_crates = parse_readme_workspace_crates(&readme_text);
    let workspace_members = parse_workspace_members(&cargo_toml_text);
    if readme_workspace_crates != workspace_members {
        let missing = workspace_members
            .difference(&readme_workspace_crates)
            .cloned()
            .collect::<Vec<_>>();
        let extra = readme_workspace_crates
            .difference(&workspace_members)
            .cloned()
            .collect::<Vec<_>>();
        errors.push(format!(
            "README workspace layering crate list mismatch with Cargo workspace members: missing={missing:?}, extra={extra:?}"
        ));
    }

    for marker in find_missing_markers(&readme_text, &README_DOCTRINE_MARKERS) {
        errors.push(format!("README doctrine-check marker missing: {marker}"));
    }
    for marker in find_missing_markers(&architecture_text, &ARCHITECTURE_DOCTRINE_MARKERS) {
        errors.push(format!(
            "ARCHITECTURE-MAP doctrine marker missing: {marker}"
        ));
    }
    for marker in find_missing_markers(&ci_closure_text, &CI_CLOSURE_DOCTRINE_MARKERS) {
        errors.push(format!(
            "CI-CLOSURE doctrine-check semantics missing marker: {marker}"
        ));
    }

    let baseline_commands = parse_mise_task_commands(&mise_toml_text, "baseline")?;
    let baseline_task_ids = parse_baseline_task_ids_from_commands(&baseline_commands);
    let baseline_task_set = baseline_task_ids.iter().cloned().collect::<BTreeSet<_>>();
    let doctrine_check_commands = parse_mise_task_commands(&mise_toml_text, "doctrine-check")?;
    if doctrine_check_commands
        != EXPECTED_DOCTRINE_CHECK_COMMANDS
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
    {
        errors.push(format!(
            "doctrine-check command surface mismatch: expected={:?}, got={:?}",
            EXPECTED_DOCTRINE_CHECK_COMMANDS, doctrine_check_commands,
        ));
    }

    let ci_baseline_section = extract_section_between(
        &ci_closure_text,
        "Current full baseline gate (`mise run baseline`) includes:",
        "Local command:",
    )?;
    let ci_baseline_tasks = parse_task_tokens(&ci_baseline_section);
    if ci_baseline_tasks != baseline_task_set {
        errors.push(format!(
            "CI-CLOSURE baseline task list mismatch with .mise baseline: expected=[{}], got=[{}]",
            sorted_csv(&baseline_task_set),
            sorted_csv(&ci_baseline_tasks),
        ));
    }

    let projection_checks = parse_control_plane_projection_checks(&control_plane_contract)?;
    let projection_check_set = projection_checks.iter().cloned().collect::<BTreeSet<_>>();
    let ci_projection_section = extract_section_between(
        &ci_closure_text,
        "Current deterministic projected check IDs include:",
        "## 5. Variants and capability projection",
    )?;
    let ci_projection_checks = parse_task_tokens(&ci_projection_section);
    if ci_projection_checks != projection_check_set {
        errors.push(format!(
            "CI-CLOSURE projected check ID list mismatch with CONTROL-PLANE-CONTRACT checkOrder: expected=[{}], got=[{}]",
            sorted_csv(&projection_check_set),
            sorted_csv(&ci_projection_checks),
        ));
    }

    Ok(DocsCoherenceEvaluation {
        summary: DocsCoherenceSummary {
            capabilities: executable_capabilities.len(),
            baseline_tasks: baseline_task_ids.len(),
            projection_checks: projection_checks.len(),
            doctrine_checks: doctrine_check_commands.len(),
        },
        errors,
    })
}

pub fn run(repo_root: String, json_output: bool) {
    let workspace_root = workspace_root();
    let repo_root = resolve_path(&PathBuf::from(repo_root), &workspace_root);
    ensure_path_exists(&repo_root, "repo root");

    let evaluation = evaluate(&repo_root);
    let (summary, errors) = match evaluation {
        Ok(result) => (Some(result.summary), result.errors),
        Err(err) => (None, vec![err]),
    };

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
            "capabilities": summary.as_ref().map(|row| row.capabilities),
            "baselineTasks": summary.as_ref().map(|row| row.baseline_tasks),
            "projectionChecks": summary.as_ref().map(|row| row.projection_checks),
            "doctrineChecks": summary.as_ref().map(|row| row.doctrine_checks),
            "errors": errors,
            "stdoutLines": Vec::<String>::new(),
            "stderrLines": Vec::<String>::new(),
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render docs-coherence-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else if let Some(summary) = summary.as_ref() {
        if accepted {
            println!(
                "[docs-coherence-check] OK (capabilities={}, baselineTasks={}, projectionChecks={}, doctrineChecks={})",
                summary.capabilities,
                summary.baseline_tasks,
                summary.projection_checks,
                summary.doctrine_checks,
            );
        } else {
            println!("[docs-coherence-check] FAIL (errors={})", errors.len());
            for error in &errors {
                println!("  - {error}");
            }
        }
    } else {
        println!("[docs-coherence-check] FAIL (errors={})", errors.len());
        for error in &errors {
            println!("  - {error}");
        }
    }

    if !accepted {
        std::process::exit(1);
    }
}
