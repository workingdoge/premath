use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const CHECK_KIND: &str = "ci.capability_stub_invariance_check.v1";
const FAILURE_CLASS_VIOLATION: &str = "capability_stub_invariance_violation";

fn load_json(path: &Path, errors: &mut Vec<String>) -> Option<Value> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            errors.push(format!("missing file: {}", path.display()));
            return None;
        }
        Err(err) => {
            errors.push(format!("failed to read {}: {err}", path.display()));
            return None;
        }
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(value) if value.is_object() => Some(value),
        Ok(_) => {
            errors.push(format!("json root must be object: {}", path.display()));
            None
        }
        Err(err) => {
            errors.push(format!("invalid json: {} ({err})", path.display()));
            None
        }
    }
}

fn discover_case_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|err| format!("failed reading directory {}: {err}", dir.display()))?;
    for row in entries {
        let entry = row.map_err(|err| format!("failed reading directory entry: {err}"))?;
        let path = entry.path();
        if path.is_dir() {
            discover_case_files(&path, out)?;
        } else if path.file_name().is_some_and(|name| name == "case.json") {
            out.push(path);
        }
    }
    Ok(())
}

fn discover_vector_dirs(capability_dir: &Path) -> Result<Vec<String>, String> {
    let mut case_files = Vec::new();
    discover_case_files(capability_dir, &mut case_files)?;
    let mut out = Vec::new();
    for case_file in case_files {
        let rel = case_file
            .strip_prefix(capability_dir)
            .map_err(|err| {
                format!(
                    "failed deriving vector path under {}: {err}",
                    capability_dir.display()
                )
            })?
            .to_string_lossy()
            .replace('\\', "/");
        if let Some(vector_dir) = rel.strip_suffix("/case.json") {
            out.push(vector_dir.to_string());
        }
    }
    out.sort();
    Ok(out)
}

fn validate_capability_dir(
    capability_dir: &Path,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> Result<usize, String> {
    let manifest_path = capability_dir.join("manifest.json");
    let Some(manifest) = load_json(&manifest_path, errors) else {
        return Ok(0);
    };
    let Some(manifest_obj) = manifest.as_object() else {
        return Ok(0);
    };

    let capability_id = manifest_obj
        .get("capabilityId")
        .and_then(Value::as_str)
        .map(str::to_string);
    let Some(capability_id) = capability_id else {
        errors.push(format!(
            "{}: capabilityId must be non-empty string",
            manifest_path.display()
        ));
        return Ok(0);
    };
    if capability_id.is_empty() {
        errors.push(format!(
            "{}: capabilityId must be non-empty string",
            manifest_path.display()
        ));
        return Ok(0);
    }
    if capability_id
        != capability_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    {
        errors.push(format!(
            "{}: capabilityId '{}' must match directory name '{}'",
            manifest_path.display(),
            capability_id,
            capability_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ));
    }

    let vectors = manifest_obj.get("vectors").and_then(Value::as_array);
    let Some(vectors) = vectors else {
        errors.push(format!(
            "{}: vectors must be non-empty list",
            manifest_path.display()
        ));
        return Ok(0);
    };
    if vectors.is_empty() {
        errors.push(format!(
            "{}: vectors must be non-empty list",
            manifest_path.display()
        ));
        return Ok(0);
    }

    let mut manifest_vectors = Vec::new();
    for (idx, row) in vectors.iter().enumerate() {
        let Some(value) = row.as_str() else {
            errors.push(format!(
                "{}: vectors[{}] must be non-empty string",
                manifest_path.display(),
                idx
            ));
            continue;
        };
        if value.is_empty() {
            errors.push(format!(
                "{}: vectors[{}] must be non-empty string",
                manifest_path.display(),
                idx
            ));
            continue;
        }
        manifest_vectors.push(value.to_string());
    }

    let mut unique = manifest_vectors.clone();
    unique.sort();
    unique.dedup();
    if unique.len() != manifest_vectors.len() {
        errors.push(format!(
            "{}: duplicate entries in vectors",
            manifest_path.display()
        ));
    }

    let discovered_vectors = discover_vector_dirs(capability_dir)?;
    let missing_in_manifest = discovered_vectors
        .iter()
        .filter(|row| !manifest_vectors.contains(row))
        .cloned()
        .collect::<Vec<_>>();
    let missing_on_disk = manifest_vectors
        .iter()
        .filter(|row| !discovered_vectors.contains(row))
        .cloned()
        .collect::<Vec<_>>();
    for row in missing_in_manifest {
        errors.push(format!(
            "{}: case exists on disk but not in vectors: {row}",
            manifest_path.display()
        ));
    }
    for row in missing_on_disk {
        errors.push(format!(
            "{}: vector declared but missing case.json: {row}",
            manifest_path.display()
        ));
    }

    let mut invariance_groups: BTreeMap<String, Vec<(String, Option<String>)>> = BTreeMap::new();
    let mut checked = 0usize;

    for vector in &manifest_vectors {
        let case_path = capability_dir.join(vector).join("case.json");
        let expect_path = capability_dir.join(vector).join("expect.json");
        let Some(case) = load_json(&case_path, errors) else {
            continue;
        };
        let Some(expect) = load_json(&expect_path, errors) else {
            continue;
        };
        checked += 1;

        let case_capability_id = case.get("capabilityId").and_then(Value::as_str);
        if case_capability_id != Some(capability_id.as_str()) {
            errors.push(format!(
                "{}: capabilityId '{:?}' != manifest capabilityId '{}'",
                case_path.display(),
                case_capability_id,
                capability_id
            ));
        }
        let case_vector_id = case.get("vectorId").and_then(Value::as_str);
        if case_vector_id != Some(vector.as_str()) {
            errors.push(format!(
                "{}: vectorId '{:?}' != manifest vector '{}'",
                case_path.display(),
                case_vector_id,
                vector
            ));
        }

        if case.get("schema").and_then(Value::as_i64) != Some(1) {
            warnings.push(format!("{}: schema is not 1", case_path.display()));
        }
        if expect.get("schema").and_then(Value::as_i64) != Some(1) {
            warnings.push(format!("{}: schema is not 1", expect_path.display()));
        }

        if vector.starts_with("invariance/") {
            let semantic_scenario_id = case
                .get("semanticScenarioId")
                .and_then(Value::as_str)
                .map(str::to_string);
            let profile = case
                .get("profile")
                .and_then(Value::as_str)
                .map(str::to_string);
            match semantic_scenario_id {
                Some(ref sid) if !sid.is_empty() => {
                    invariance_groups
                        .entry(sid.clone())
                        .or_default()
                        .push((vector.to_string(), profile));
                }
                _ => errors.push(format!(
                    "{}: invariance case requires non-empty semanticScenarioId",
                    case_path.display()
                )),
            }

            let assertions = expect.get("assertions").and_then(Value::as_array);
            let Some(assertions) = assertions else {
                errors.push(format!(
                    "{}: invariance expect requires non-empty assertions list",
                    expect_path.display()
                ));
                continue;
            };
            if assertions.is_empty() {
                errors.push(format!(
                    "{}: invariance expect requires non-empty assertions list",
                    expect_path.display()
                ));
                continue;
            }
            let text = assertions
                .iter()
                .map(|row| row.to_string())
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            if !text.contains("kernel verdict") {
                errors.push(format!(
                    "{}: invariance assertions must mention kernel verdict",
                    expect_path.display()
                ));
            }
            if !text.contains("gate failure") {
                errors.push(format!(
                    "{}: invariance assertions must mention Gate failure classes",
                    expect_path.display()
                ));
            }
        }
    }

    if invariance_groups.is_empty() {
        warnings.push(format!(
            "{}: no invariance cases found",
            manifest_path.display()
        ));
    } else {
        for (scenario_id, rows) in &invariance_groups {
            if rows.len() != 2 {
                errors.push(format!(
                    "{}: invariance scenario '{}' must have exactly 2 vectors, found {}",
                    manifest_path.display(),
                    scenario_id,
                    rows.len()
                ));
                continue;
            }
            let profiles = rows
                .iter()
                .filter_map(|(_, profile)| profile.clone())
                .collect::<Vec<_>>();
            let distinct = profiles.iter().collect::<std::collections::BTreeSet<_>>();
            if distinct.len() < 2 {
                errors.push(format!(
                    "{}: invariance scenario '{}' should have two distinct profiles; got {:?}",
                    manifest_path.display(),
                    scenario_id,
                    profiles
                ));
            }
        }
    }

    Ok(checked)
}

pub fn run(fixtures: String, json_output: bool) {
    let fixtures_root = PathBuf::from(fixtures);
    if !fixtures_root.exists() {
        eprintln!(
            "[error] fixtures path does not exist: {}",
            fixtures_root.display()
        );
        std::process::exit(2);
    }
    if !fixtures_root.is_dir() {
        eprintln!(
            "[error] fixtures path is not a directory: {}",
            fixtures_root.display()
        );
        std::process::exit(2);
    }

    let entries = fs::read_dir(&fixtures_root).unwrap_or_else(|err| {
        eprintln!(
            "[error] failed to read fixtures path {}: {err}",
            fixtures_root.display()
        );
        std::process::exit(2);
    });
    let mut capability_dirs = entries
        .map(|row| row.map_err(|err| format!("failed reading fixtures dir entry: {err}")))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|err| {
            eprintln!("[error] {err}");
            std::process::exit(2);
        })
        .into_iter()
        .filter(|row| row.path().is_dir())
        .filter(|row| !row.file_name().to_string_lossy().starts_with('.'))
        .map(|row| row.path())
        .collect::<Vec<_>>();
    capability_dirs.sort();

    if capability_dirs.is_empty() {
        eprintln!(
            "[error] no capability directories found under: {}",
            fixtures_root.display()
        );
        std::process::exit(2);
    }

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut checked_vectors = 0usize;
    for capability_dir in &capability_dirs {
        checked_vectors += validate_capability_dir(capability_dir, &mut errors, &mut warnings)
            .unwrap_or_else(|err| {
                eprintln!(
                    "[error] failed validating capability directory {}: {err}",
                    capability_dir.display()
                );
                std::process::exit(2);
            });
    }

    let result = if errors.is_empty() {
        "accepted"
    } else {
        "rejected"
    };
    let failure_classes: Vec<&str> = if errors.is_empty() {
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
            "capabilities": capability_dirs.len(),
            "vectors": checked_vectors,
            "errors": errors,
            "warnings": warnings,
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render capability-stub-invariance payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else if errors.is_empty() {
        println!(
            "[conformance-check] OK (capabilities={}, vectors={}, warnings={})",
            capability_dirs.len(),
            checked_vectors,
            warnings.len()
        );
        for warning in &warnings {
            println!("  [warn] {warning}");
        }
    } else {
        println!(
            "[conformance-check] FAIL ({} errors, {} warnings)",
            errors.len(),
            warnings.len()
        );
        for error in &errors {
            println!("  - {error}");
        }
        if !warnings.is_empty() {
            println!("[warnings]");
            for warning in &warnings {
                println!("  - {warning}");
            }
        }
    }

    if !errors.is_empty() {
        std::process::exit(1);
    }
}
