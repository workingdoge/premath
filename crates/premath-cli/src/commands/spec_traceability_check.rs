use regex::Regex;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const CHECK_KIND: &str = "ci.spec_traceability_check.v1";
const FAILURE_CLASS_VIOLATION: &str = "spec_traceability_violation";
const MATRIX_SECTION_HEADING: &str = "## 3. Traceability Matrix";
const VALID_STATUS: [&str; 3] = ["covered", "instrumented", "gap"];

#[derive(Debug)]
struct MatrixRow {
    spec_name: String,
    status: String,
    target: String,
}

fn extract_frontmatter_status(path: &Path) -> Result<Option<String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed reading {}: {err}", path.display()))?;
    if !text.starts_with("---\n") {
        return Ok(None);
    }
    let Some((_, rest)) = text.split_once("---\n") else {
        return Ok(None);
    };
    let Some((frontmatter, _)) = rest.split_once("---\n") else {
        return Ok(None);
    };
    for raw in frontmatter.lines() {
        let line = raw.trim();
        if let Some(value) = line.strip_prefix("status:") {
            return Ok(Some(value.trim().to_string()));
        }
    }
    Ok(None)
}

fn promoted_draft_specs(draft_dir: &Path) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(draft_dir)
        .map_err(|err| format!("failed reading {}: {err}", draft_dir.display()))?;
    let mut files = entries
        .map(|entry| entry.map_err(|err| format!("failed reading directory entry: {err}")))
        .collect::<Result<Vec<_>, _>>()?;
    files.sort_by_key(|entry| entry.file_name());

    let mut promoted = Vec::new();
    for entry in files {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let Some(name_os) = path.file_name() else {
            continue;
        };
        let Some(name) = name_os.to_str() else {
            continue;
        };
        if name == "README.md" {
            continue;
        }
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("md") => {
                if extract_frontmatter_status(&path)?.as_deref() == Some("draft") {
                    promoted.push(name.to_string());
                }
            }
            Some("json") => promoted.push(name.to_string()),
            _ => {}
        }
    }
    Ok(promoted)
}

fn strip_cell(cell: &str) -> String {
    cell.trim().trim_matches('\u{200b}').to_string()
}

fn parse_matrix_rows(matrix_path: &Path) -> Result<Vec<MatrixRow>, String> {
    let text = fs::read_to_string(matrix_path)
        .map_err(|err| format!("failed reading {}: {err}", matrix_path.display()))?;
    let separator_re = Regex::new(r"^\|\s*-+\s*\|\s*-+\s*\|\s*-+\s*\|\s*-+\s*\|$")
        .map_err(|err| format!("failed compiling matrix separator regex: {err}"))?;
    let code_ref_re = Regex::new(r"`([^`]+)`")
        .map_err(|err| format!("failed compiling code-ref regex: {err}"))?;

    let mut rows = Vec::new();
    let mut in_matrix = false;
    let mut in_table = false;

    for raw in text.lines() {
        let line = raw.trim_end();
        if line.starts_with(MATRIX_SECTION_HEADING) {
            in_matrix = true;
            continue;
        }
        if in_matrix && line.starts_with("## ") {
            break;
        }
        if !in_matrix {
            continue;
        }
        if line.trim_start().starts_with('|') {
            in_table = true;
        }
        if !in_table {
            continue;
        }
        let stripped = line.trim();
        if stripped.is_empty() || !stripped.starts_with('|') {
            continue;
        }
        if separator_re.is_match(stripped) {
            continue;
        }
        let parts = stripped
            .trim_matches('|')
            .split('|')
            .map(strip_cell)
            .collect::<Vec<_>>();
        if parts.len() != 4 {
            return Err(format!(
                "{}: malformed matrix row: {line}",
                matrix_path.display()
            ));
        }
        let spec_cell = parts[0].as_str();
        if spec_cell == "Draft spec" {
            continue;
        }
        let Some(spec_match) = code_ref_re.captures(spec_cell) else {
            return Err(format!(
                "{}: first column must contain backticked spec name: {line}",
                matrix_path.display()
            ));
        };
        let Some(spec_ref) = spec_match.get(1) else {
            return Err(format!(
                "{}: missing spec name capture: {line}",
                matrix_path.display()
            ));
        };
        rows.push(MatrixRow {
            spec_name: spec_ref.as_str().trim().to_string(),
            status: parts[2].clone(),
            target: parts[3].clone(),
        });
    }
    Ok(rows)
}

fn validate_matrix(draft_specs: &[String], rows: &[MatrixRow]) -> Result<Vec<String>, String> {
    let gap_target_re = Regex::new(r"^T-[A-Z]+-\d+$")
        .map_err(|err| format!("failed compiling gap target regex: {err}"))?;

    let draft_set: BTreeSet<String> = draft_specs.iter().cloned().collect();
    let mut row_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut errors = Vec::new();

    for row in rows {
        *row_counts.entry(row.spec_name.clone()).or_insert(0) += 1;
        if !VALID_STATUS.contains(&row.status.as_str()) {
            errors.push(format!(
                "invalid status for {:?}: {:?}",
                row.spec_name, row.status
            ));
        }
        if row.status == "gap" && !gap_target_re.is_match(&row.target) {
            errors.push(format!(
                "gap row for {:?} must use target ID T-*-*: got {:?}",
                row.spec_name, row.target
            ));
        }
        if !draft_set.contains(&row.spec_name) {
            errors.push(format!(
                "matrix row references unknown draft spec: {:?}",
                row.spec_name
            ));
        }
    }

    for spec in &draft_set {
        let count = row_counts.get(spec).copied().unwrap_or(0);
        if count == 0 {
            errors.push(format!(
                "promoted draft spec missing from matrix: {:?}",
                spec
            ));
        } else if count > 1 {
            errors.push(format!(
                "promoted draft spec appears multiple times in matrix: {:?} ({} rows)",
                spec, count
            ));
        }
    }

    Ok(errors)
}

pub fn run(draft_dir: String, matrix: String, json_output: bool) {
    let draft_dir = PathBuf::from(draft_dir);
    let matrix_path = PathBuf::from(matrix);

    if !draft_dir.exists() || !draft_dir.is_dir() {
        eprintln!(
            "[traceability-check] ERROR: draft directory missing: {}",
            draft_dir.display()
        );
        std::process::exit(2);
    }
    if !matrix_path.exists() {
        eprintln!(
            "[traceability-check] ERROR: matrix file missing: {}",
            matrix_path.display()
        );
        std::process::exit(2);
    }

    let draft_specs = promoted_draft_specs(&draft_dir).unwrap_or_else(|err| {
        eprintln!("error: {err}");
        std::process::exit(2);
    });
    let rows = parse_matrix_rows(&matrix_path).unwrap_or_else(|err| {
        eprintln!("error: {err}");
        std::process::exit(2);
    });
    let errors = validate_matrix(&draft_specs, &rows).unwrap_or_else(|err| {
        eprintln!("error: {err}");
        std::process::exit(2);
    });

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
            "draftSpecs": draft_specs.len(),
            "matrixRows": rows.len(),
            "errors": errors,
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render spec-traceability-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else if errors.is_empty() {
        println!(
            "[traceability-check] OK (draftSpecs={}, matrixRows={})",
            draft_specs.len(),
            rows.len()
        );
    } else {
        println!(
            "[traceability-check] FAIL (draftSpecs={}, matrixRows={}, errors={})",
            draft_specs.len(),
            rows.len(),
            errors.len()
        );
        for err in &errors {
            println!("  - {err}");
        }
    }

    if !errors.is_empty() {
        std::process::exit(1);
    }
}
