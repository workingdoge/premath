use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const VALID_STATUS: &[&str] = &["covered", "instrumented", "gap"];

pub struct Args {
    pub draft_dir: String,
    pub matrix: String,
    pub authority_map: String,
    pub json: bool,
}

#[derive(Debug, Clone)]
struct TraceabilityRow {
    spec_name: String,
    authority_class: String,
    status: String,
    target: String,
}

pub fn run(args: Args) {
    let outcome = evaluate(&args);
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&outcome.payload).unwrap_or_else(|err| {
                eprintln!("error: failed to render traceability-check json: {err}");
                std::process::exit(2);
            })
        );
    } else if outcome.errors.is_empty() {
        println!(
            "[traceability-check] OK (draftSpecs={}, matrixRows={})",
            outcome.draft_count, outcome.row_count
        );
    } else {
        println!(
            "[traceability-check] FAIL (draftSpecs={}, matrixRows={}, errors={})",
            outcome.draft_count,
            outcome.row_count,
            outcome.errors.len()
        );
        for error in &outcome.errors {
            println!("  - {error}");
        }
    }

    if !outcome.errors.is_empty() {
        std::process::exit(1);
    }
}

struct Outcome {
    draft_count: usize,
    row_count: usize,
    errors: Vec<String>,
    payload: Value,
}

fn evaluate(args: &Args) -> Outcome {
    let draft_dir = PathBuf::from(&args.draft_dir);
    let matrix_path = PathBuf::from(&args.matrix);
    let authority_map_path = PathBuf::from(&args.authority_map);
    let mut errors = Vec::new();

    if !draft_dir.is_dir() {
        errors.push(format!(
            "draft directory missing: {}",
            display_path(&draft_dir)
        ));
        return outcome(0, 0, errors);
    }
    if !matrix_path.is_file() {
        errors.push(format!(
            "matrix file missing: {}",
            display_path(&matrix_path)
        ));
        return outcome(0, 0, errors);
    }

    let draft_specs = match promoted_draft_specs(&draft_dir) {
        Ok(specs) => specs,
        Err(err) => {
            errors.push(err);
            return outcome(0, 0, errors);
        }
    };

    let matrix_text = match fs::read_to_string(&matrix_path) {
        Ok(text) => text,
        Err(err) => {
            errors.push(format!(
                "{}: failed to read: {err}",
                display_path(&matrix_path)
            ));
            return outcome(draft_specs.len(), 0, errors);
        }
    };
    let rows = match parse_table_rows(&matrix_text, &matrix_path) {
        Ok(rows) => rows,
        Err(err) => {
            errors.push(err);
            Vec::new()
        }
    };

    let authority_classes = match load_authority_classes(&authority_map_path) {
        Ok(classes) => classes,
        Err(mut errs) => {
            errors.append(&mut errs);
            BTreeMap::new()
        }
    };
    errors.extend(validate_matrix(&draft_specs, &rows, &authority_classes));
    outcome(draft_specs.len(), rows.len(), errors)
}

fn outcome(draft_count: usize, row_count: usize, errors: Vec<String>) -> Outcome {
    let result = if errors.is_empty() {
        "accepted"
    } else {
        "rejected"
    };
    let payload = json!({
        "schema": 1,
        "checkKind": "premath.traceability.v1",
        "result": result,
        "draftSpecs": draft_count,
        "matrixRows": row_count,
        "errors": errors,
    });
    let errors = payload
        .get("errors")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Outcome {
        draft_count,
        row_count,
        errors,
        payload,
    }
}

fn promoted_draft_specs(draft_dir: &Path) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let entries = fs::read_dir(draft_dir).map_err(|err| {
        format!(
            "{}: failed to read directory: {err}",
            display_path(draft_dir)
        )
    })?;
    for entry in entries {
        let entry = entry
            .map_err(|err| format!("{}: failed to read entry: {err}", display_path(draft_dir)))?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name == "README.md" {
            continue;
        }
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => out.push(file_name.to_string()),
            Some("md") if frontmatter_status(&path)? == Some("draft".to_string()) => {
                out.push(file_name.to_string());
            }
            _ => {}
        }
    }
    out.sort();
    Ok(out)
}

fn frontmatter_status(path: &Path) -> Result<Option<String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("{}: failed to read: {err}", display_path(path)))?;
    if !text.starts_with("---\n") {
        return Ok(None);
    }
    let Some(rest) = text.strip_prefix("---\n") else {
        return Ok(None);
    };
    let Some((frontmatter, _body)) = rest.split_once("---\n") else {
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

fn parse_table_rows(text: &str, matrix_path: &Path) -> Result<Vec<TraceabilityRow>, String> {
    let mut rows = Vec::new();
    let mut in_matrix = false;
    let mut in_table = false;

    for raw in text.lines() {
        let line = raw.trim_end();
        if line.starts_with("## 3. Traceability Matrix") {
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

        let parts: Vec<String> = stripped
            .trim_matches('|')
            .split('|')
            .map(|part| part.trim().trim_matches('\u{200b}').to_string())
            .collect();
        if is_separator_row(&parts) {
            continue;
        }
        if parts.len() != 5 {
            return Err(format!(
                "{}: malformed matrix row: {line}",
                display_path(matrix_path)
            ));
        }
        if parts[0] == "Draft spec" {
            continue;
        }
        let spec_name = first_code_ref(&parts[0]).ok_or_else(|| {
            format!(
                "{}: first column must contain backticked spec name: {line}",
                display_path(matrix_path)
            )
        })?;
        let authority_class =
            full_code_ref(&parts[1]).unwrap_or_else(|| parts[1].trim().to_string());
        rows.push(TraceabilityRow {
            spec_name,
            authority_class,
            status: parts[3].clone(),
            target: parts[4].clone(),
        });
    }

    Ok(rows)
}

fn is_separator_row(parts: &[String]) -> bool {
    !parts.is_empty()
        && parts.iter().all(|part| {
            let compact: String = part.chars().filter(|ch| !ch.is_whitespace()).collect();
            !compact.is_empty() && compact.chars().all(|ch| ch == '-')
        })
}

fn first_code_ref(value: &str) -> Option<String> {
    let start = value.find('`')?;
    let rest = &value[start + 1..];
    let end = rest.find('`')?;
    Some(rest[..end].trim().to_string())
}

fn full_code_ref(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('`') || !trimmed.ends_with('`') || trimmed.len() < 2 {
        return None;
    }
    Some(trimmed[1..trimmed.len() - 1].trim().to_string())
}

fn load_authority_classes(path: &Path) -> Result<BTreeMap<String, String>, Vec<String>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            return Err(vec![format!(
                "authority map missing: {}: {err}",
                display_path(path)
            )]);
        }
    };
    let payload: Value = match serde_json::from_str(&text) {
        Ok(payload) => payload,
        Err(err) => return Err(vec![format!("{}: invalid JSON: {err}", display_path(path))]),
    };

    let mut errors = Vec::new();
    if payload.get("schema").and_then(Value::as_u64) != Some(1) {
        errors.push(format!("{}: schema must be 1", display_path(path)));
    }
    if payload.get("mapKind").and_then(Value::as_str) != Some("premath.authority_map.v1") {
        errors.push(format!(
            "{}: mapKind must be 'premath.authority_map.v1'",
            display_path(path)
        ));
    }

    let Some(classes) = payload.get("classes").and_then(Value::as_object) else {
        errors.push(format!(
            "{}: classes must be a non-empty object",
            display_path(path)
        ));
        return Err(errors);
    };
    if classes.is_empty() {
        errors.push(format!(
            "{}: classes must be a non-empty object",
            display_path(path)
        ));
        return Err(errors);
    }

    let mut spec_to_class = BTreeMap::new();
    for (class_id, class_payload) in classes {
        if class_id.trim().is_empty() {
            errors.push(format!(
                "{}: class IDs must be non-empty strings",
                display_path(path)
            ));
            continue;
        }
        let Some(class_obj) = class_payload.as_object() else {
            errors.push(format!(
                "{}: class {class_id:?} must be an object",
                display_path(path)
            ));
            continue;
        };
        let Some(draft_specs_value) = class_obj.get("draftSpecs") else {
            continue;
        };
        let Some(draft_specs) = draft_specs_value.as_array() else {
            errors.push(format!(
                "{}: class {class_id:?}.draftSpecs must be a list",
                display_path(path)
            ));
            continue;
        };
        for (idx, spec_name) in draft_specs.iter().enumerate() {
            let Some(spec_name) = spec_name
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                errors.push(format!(
                    "{}: class {class_id:?}.draftSpecs[{idx}] must be a non-empty string",
                    display_path(path)
                ));
                continue;
            };
            if let Some(previous) =
                spec_to_class.insert(spec_name.to_string(), class_id.to_string())
            {
                errors.push(format!(
                    "{}: draft spec {spec_name:?} appears in both {previous:?} and {class_id:?}",
                    display_path(path)
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(spec_to_class)
    } else {
        Err(errors)
    }
}

fn validate_matrix(
    draft_specs: &[String],
    rows: &[TraceabilityRow],
    authority_classes: &BTreeMap<String, String>,
) -> Vec<String> {
    let draft_set: BTreeSet<String> = draft_specs.iter().cloned().collect();
    let mut row_map: BTreeMap<String, usize> = BTreeMap::new();
    let mut errors = Vec::new();

    for row in rows {
        *row_map.entry(row.spec_name.clone()).or_insert(0) += 1;
        match authority_classes.get(&row.spec_name) {
            Some(expected) if expected == &row.authority_class => {}
            Some(expected) => errors.push(format!(
                "authority class mismatch for {:?}: matrix={:?}, authorityMap={:?}",
                row.spec_name, row.authority_class, expected
            )),
            None => errors.push(format!(
                "authority map missing draft spec: {:?}",
                row.spec_name
            )),
        }
        if !VALID_STATUS.contains(&row.status.as_str()) {
            errors.push(format!(
                "invalid status for {:?}: {:?}",
                row.spec_name, row.status
            ));
        }
        if row.status == "gap" && !is_gap_target(&row.target) {
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
        match row_map.get(spec).copied().unwrap_or(0) {
            0 => errors.push(format!("promoted draft spec missing from matrix: {spec:?}")),
            1 => {}
            count => errors.push(format!(
                "promoted draft spec appears multiple times in matrix: {spec:?} ({count} rows)"
            )),
        }
        if !authority_classes.contains_key(spec) {
            errors.push(format!(
                "promoted draft spec missing from authority map: {spec:?}"
            ));
        }
    }

    for spec in authority_classes.keys() {
        if !draft_set.contains(spec) {
            errors.push(format!(
                "authority map references unknown draft spec: {spec:?}"
            ));
        }
    }

    errors
}

fn is_gap_target(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    parts.len() == 3
        && parts[0] == "T"
        && !parts[1].is_empty()
        && parts[1].chars().all(|ch| ch.is_ascii_uppercase())
        && !parts[2].is_empty()
        && parts[2].chars().all(|ch| ch.is_ascii_digit())
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}
