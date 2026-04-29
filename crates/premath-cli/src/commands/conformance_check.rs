//! Capability fixture stub and invariance checker.

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct Outcome<'a> {
    schema: u8,
    check_kind: &'a str,
    result: &'a str,
    capabilities: usize,
    vectors: usize,
    errors: &'a [String],
    warnings: &'a [String],
}

pub fn run(fixtures: String, json: bool) {
    match check(Path::new(&fixtures)) {
        Ok(report) => emit(report, json),
        Err(err) => {
            if json {
                let errors = vec![err];
                let warnings = Vec::new();
                let outcome = Outcome {
                    schema: 1,
                    check_kind: "premath.conformance.fixtures.v1",
                    result: "rejected",
                    capabilities: 0,
                    vectors: 0,
                    errors: &errors,
                    warnings: &warnings,
                };
                println!("{}", serde_json::to_string_pretty(&outcome).unwrap());
            } else {
                println!("[error] {err}");
            }
            std::process::exit(2);
        }
    }
}

fn emit(report: Report, json: bool) {
    let accepted = report.errors.is_empty();
    if json {
        let outcome = Outcome {
            schema: 1,
            check_kind: "premath.conformance.fixtures.v1",
            result: if accepted { "accepted" } else { "rejected" },
            capabilities: report.capabilities,
            vectors: report.checked_vectors,
            errors: &report.errors,
            warnings: &report.warnings,
        };
        println!("{}", serde_json::to_string_pretty(&outcome).unwrap());
    } else if accepted {
        println!(
            "[conformance-check] OK (capabilities={}, vectors={}, warnings={})",
            report.capabilities,
            report.checked_vectors,
            report.warnings.len()
        );
        for warning in &report.warnings {
            println!("  [warn] {warning}");
        }
    } else {
        println!(
            "[conformance-check] FAIL ({} errors, {} warnings)",
            report.errors.len(),
            report.warnings.len()
        );
        for error in &report.errors {
            println!("  - {error}");
        }
        if !report.warnings.is_empty() {
            println!("[warnings]");
            for warning in &report.warnings {
                println!("  - {warning}");
            }
        }
    }

    if !accepted {
        std::process::exit(1);
    }
}

struct Report {
    capabilities: usize,
    checked_vectors: usize,
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn check(fixtures_root: &Path) -> Result<Report, String> {
    if !fixtures_root.exists() {
        return Err(format!(
            "fixtures path does not exist: {}",
            fixtures_root.display()
        ));
    }
    if !fixtures_root.is_dir() {
        return Err(format!(
            "fixtures path is not a directory: {}",
            fixtures_root.display()
        ));
    }

    let mut capability_dirs = read_dirs(fixtures_root)
        .map_err(|err| format!("failed to read {}: {err}", fixtures_root.display()))?;
    capability_dirs.retain(|path| {
        path.is_dir()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| !name.starts_with('.'))
    });

    if capability_dirs.is_empty() {
        return Err(format!(
            "no capability directories found under: {}",
            fixtures_root.display()
        ));
    }

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut checked_vectors = 0usize;
    for capability_dir in &capability_dirs {
        checked_vectors += validate_capability_dir(capability_dir, &mut errors, &mut warnings);
    }

    Ok(Report {
        capabilities: capability_dirs.len(),
        checked_vectors,
        errors,
        warnings,
    })
}

fn validate_capability_dir(
    capability_dir: &Path,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> usize {
    let manifest_path = capability_dir.join("manifest.json");
    let Some(manifest) = load_json_object(&manifest_path, errors) else {
        return 0;
    };

    let capability_id = manifest.get("capabilityId").and_then(Value::as_str);
    let Some(capability_id) = capability_id.filter(|value| !value.is_empty()) else {
        errors.push(format!(
            "{}: capabilityId must be non-empty string",
            manifest_path.display()
        ));
        return 0;
    };

    let dir_name = capability_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if capability_id != dir_name {
        errors.push(format!(
            "{}: capabilityId '{capability_id}' must match directory name '{dir_name}'",
            manifest_path.display()
        ));
    }

    let Some(vectors_value) = manifest.get("vectors") else {
        errors.push(format!(
            "{}: vectors must be non-empty list",
            manifest_path.display()
        ));
        return 0;
    };
    let Some(vectors) = vectors_value.as_array().filter(|items| !items.is_empty()) else {
        errors.push(format!(
            "{}: vectors must be non-empty list",
            manifest_path.display()
        ));
        return 0;
    };

    let mut manifest_vectors = Vec::new();
    for (idx, vector) in vectors.iter().enumerate() {
        match vector.as_str().filter(|value| !value.is_empty()) {
            Some(value) => manifest_vectors.push(value.to_owned()),
            None => errors.push(format!(
                "{}: vectors[{idx}] must be non-empty string",
                manifest_path.display()
            )),
        }
    }

    let unique: BTreeSet<&String> = manifest_vectors.iter().collect();
    if unique.len() != manifest_vectors.len() {
        errors.push(format!(
            "{}: duplicate entries in vectors",
            manifest_path.display()
        ));
    }

    let discovered_vectors = discover_vector_dirs(capability_dir, errors);
    let discovered: BTreeSet<String> = discovered_vectors.into_iter().collect();
    let declared: BTreeSet<String> = manifest_vectors.iter().cloned().collect();

    for vector in discovered.difference(&declared) {
        errors.push(format!(
            "{}: case exists on disk but not in vectors: {vector}",
            manifest_path.display()
        ));
    }
    for vector in declared.difference(&discovered) {
        errors.push(format!(
            "{}: vector declared but missing case.json: {vector}",
            manifest_path.display()
        ));
    }

    let mut invariance_groups: BTreeMap<String, Vec<(String, Option<String>)>> = BTreeMap::new();
    let mut checked = 0usize;
    for vector in &manifest_vectors {
        let case_path = capability_dir.join(vector).join("case.json");
        let expect_path = capability_dir.join(vector).join("expect.json");
        let case = load_json_object(&case_path, errors);
        let expect = load_json_object(&expect_path, errors);
        let (Some(case), Some(expect)) = (case, expect) else {
            continue;
        };
        checked += 1;

        let case_cap = case.get("capabilityId").and_then(Value::as_str);
        if case_cap != Some(capability_id) {
            errors.push(format!(
                "{}: capabilityId '{}' != manifest capabilityId '{capability_id}'",
                case_path.display(),
                case_cap.unwrap_or("null")
            ));
        }

        let case_vec = case.get("vectorId").and_then(Value::as_str);
        if case_vec != Some(vector.as_str()) {
            errors.push(format!(
                "{}: vectorId '{}' != manifest vector '{vector}'",
                case_path.display(),
                case_vec.unwrap_or("null")
            ));
        }

        if case.get("schema").and_then(Value::as_i64) != Some(1) {
            warnings.push(format!("{}: schema is not 1", case_path.display()));
        }
        if expect.get("schema").and_then(Value::as_i64) != Some(1) {
            warnings.push(format!("{}: schema is not 1", expect_path.display()));
        }

        if vector.starts_with("invariance/") {
            let sid = case.get("semanticScenarioId").and_then(Value::as_str);
            if let Some(sid) = sid.filter(|value| !value.is_empty()) {
                let profile = case
                    .get("profile")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                invariance_groups
                    .entry(sid.to_owned())
                    .or_default()
                    .push((vector.to_owned(), profile));
            } else {
                errors.push(format!(
                    "{}: invariance case requires non-empty semanticScenarioId",
                    case_path.display()
                ));
            }

            let assertions = expect.get("assertions").and_then(Value::as_array);
            match assertions.filter(|items| !items.is_empty()) {
                Some(assertions) => {
                    let text = assertions
                        .iter()
                        .map(Value::to_string)
                        .collect::<Vec<_>>()
                        .join(" ")
                        .to_ascii_lowercase();
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
                None => errors.push(format!(
                    "{}: invariance expect requires non-empty assertions list",
                    expect_path.display()
                )),
            }
        }
    }

    if invariance_groups.is_empty() {
        warnings.push(format!(
            "{}: no invariance cases found",
            manifest_path.display()
        ));
    } else {
        for (sid, rows) in invariance_groups {
            if rows.len() != 2 {
                errors.push(format!(
                    "{}: invariance scenario '{sid}' must have exactly 2 vectors, found {}",
                    manifest_path.display(),
                    rows.len()
                ));
                continue;
            }
            let profiles: BTreeSet<String> = rows
                .iter()
                .filter_map(|(_, profile)| profile.clone())
                .collect();
            if profiles.len() < 2 {
                errors.push(format!(
                    "{}: invariance scenario '{sid}' should have two distinct profiles; got {:?}",
                    manifest_path.display(),
                    profiles
                ));
            }
        }
    }

    checked
}

fn load_json_object(path: &Path, errors: &mut Vec<String>) -> Option<Value> {
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
    let data: Value = match serde_json::from_str(&text) {
        Ok(data) => data,
        Err(err) => {
            errors.push(format!("invalid json: {} ({err})", path.display()));
            return None;
        }
    };
    if !data.is_object() {
        errors.push(format!("json root must be object: {}", path.display()));
        return None;
    }
    Some(data)
}

fn discover_vector_dirs(capability_dir: &Path, errors: &mut Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    discover_cases(capability_dir, capability_dir, &mut out, errors);
    out.sort();
    out
}

fn discover_cases(root: &Path, current: &Path, out: &mut Vec<String>, errors: &mut Vec<String>) {
    let Ok(entries) = read_dirs(current) else {
        errors.push(format!("failed to read directory: {}", current.display()));
        return;
    };
    for path in entries {
        if path.is_dir() {
            discover_cases(root, &path, out, errors);
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) != Some("case.json") {
            continue;
        }
        let Some(parent) = path.parent() else {
            continue;
        };
        if let Ok(rel) = parent.strip_prefix(root)
            && !rel.as_os_str().is_empty()
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn read_dirs(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    Ok(entries)
}
