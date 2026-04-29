//! Provider pipeline workflow/wrapper wiring checker.

use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const COMMAND_SURFACE_MARKERS: &[(&str, &str)] = &[
    ("requiredDecision", "REQUIRED_DECISION_CANONICAL_ENTRYPOINT"),
    (
        "instructionEnvelopeCheck",
        "INSTRUCTION_ENVELOPE_CHECK_CANONICAL_ENTRYPOINT",
    ),
    (
        "instructionDecision",
        "INSTRUCTION_DECISION_CANONICAL_ENTRYPOINT",
    ),
];

const GATE_MARKERS: &[(&str, &str, &str)] = &[
    (
        "governance",
        "requiredPipeline",
        "governance_failure_classes",
    ),
    (
        "governance",
        "instructionPipeline",
        "governance_failure_classes",
    ),
    (
        "kcirMapping",
        "requiredPipeline",
        "evaluate_required_mapping",
    ),
    (
        "kcirMapping",
        "instructionPipeline",
        "evaluate_instruction_mapping",
    ),
];

#[derive(Serialize)]
struct Finding {
    failure_class: String,
    message: String,
}

#[derive(Serialize)]
struct Outcome<'a> {
    schema: u8,
    check_kind: &'a str,
    result: &'a str,
    commands: Vec<String>,
    findings: &'a [Finding],
}

pub fn run(repo_root: String, contract: String, json: bool) {
    match evaluate(Path::new(&repo_root), Path::new(&contract)) {
        Ok((commands, findings)) => emit(commands, findings, json),
        Err(message) => {
            let findings = vec![Finding {
                failure_class: "provider_pipeline_workflow_drift".to_owned(),
                message,
            }];
            emit(Vec::new(), findings, json);
            std::process::exit(1);
        }
    }
}

fn emit(commands: Vec<String>, findings: Vec<Finding>, json: bool) {
    let accepted = findings.is_empty();
    if json {
        let outcome = Outcome {
            schema: 1,
            check_kind: "ci.pipeline_wiring.v1",
            result: if accepted { "accepted" } else { "rejected" },
            commands,
            findings: &findings,
        };
        println!("{}", serde_json::to_string_pretty(&outcome).unwrap());
    } else if accepted {
        println!("[pipeline-wiring] OK ({})", commands.join(", "));
    } else {
        println!("[pipeline-wiring] FAIL");
        for finding in &findings {
            println!("  - {}: {}", finding.failure_class, finding.message);
        }
    }
    if !accepted {
        std::process::exit(1);
    }
}

fn evaluate(repo_root: &Path, contract: &Path) -> Result<(Vec<String>, Vec<Finding>), String> {
    let root = repo_root
        .canonicalize()
        .map_err(|err| format!("repo root not found: {} ({err})", repo_root.display()))?;
    let contract_path = absolutize(&root, contract);
    let contract_text = fs::read_to_string(&contract_path)
        .map_err(|err| format!("failed to read {}: {err}", contract_path.display()))?;
    let contract_value: Value = serde_json::from_str(&contract_text)
        .map_err(|err| format!("failed to parse {}: {err}", contract_path.display()))?;
    let wrappers = contract_value
        .get("providerPipelineWrappers")
        .and_then(Value::as_object)
        .ok_or_else(|| "providerPipelineWrappers must be an object".to_owned())?;

    let failure_classes = wrappers
        .get("failureClasses")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut wrapper_ids = wrappers
        .keys()
        .filter(|key| key.as_str() != "failureClasses")
        .cloned()
        .collect::<Vec<_>>();
    wrapper_ids.sort();

    let mut commands = Vec::new();
    let mut findings = Vec::new();

    for wrapper_id in wrapper_ids {
        let Some(row) = wrappers.get(&wrapper_id).and_then(Value::as_object) else {
            findings.push(Finding {
                failure_class: failure_class(&failure_classes, "workflowDrift"),
                message: format!("{wrapper_id}: contract row must be an object"),
            });
            continue;
        };

        let workflow_rel = row
            .get("workflowPath")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let wrapper_rel = row
            .get("wrapperPath")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let command = row
            .get("workflowEntrypoint")
            .and_then(Value::as_array)
            .map(|tokens| render_shell_command(tokens))
            .unwrap_or_default();
        if !command.is_empty() {
            commands.push(format!("{wrapper_id}={command}"));
        }

        let workflow_path = root.join(workflow_rel);
        let wrapper_path = root.join(wrapper_rel);

        match fs::read_to_string(&workflow_path) {
            Ok(_) if command.is_empty() => findings.push(Finding {
                failure_class: failure_class(&failure_classes, "workflowDrift"),
                message: format!("{workflow_rel}: workflow entrypoint is unbound"),
            }),
            Ok(text) => {
                check_workflow_entrypoint(
                    &mut findings,
                    &failure_classes,
                    workflow_rel,
                    &text,
                    &command,
                );
                check_forbidden(&mut findings, &failure_classes, workflow_rel, &text);
            }
            Err(_) => findings.push(Finding {
                failure_class: failure_class(&failure_classes, "workflowDrift"),
                message: format!("{workflow_rel}: workflow file missing"),
            }),
        }

        match fs::read_to_string(&wrapper_path) {
            Ok(text) => {
                check_wrapper_source(
                    &mut findings,
                    &failure_classes,
                    &wrapper_id,
                    wrapper_rel,
                    &text,
                    value_string_array(row.get("boundCommandSurfaces")),
                    value_string_array(row.get("enforcedGates")),
                );
            }
            Err(_) => findings.push(Finding {
                failure_class: failure_class(&failure_classes, "canonicalEntrypointDrift"),
                message: format!("{wrapper_rel}: wrapper file missing"),
            }),
        }
    }

    Ok((commands, findings))
}

fn absolutize(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn failure_class(map: &serde_json::Map<String, Value>, key: &str) -> String {
    map.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("provider_pipeline_{key}"))
}

fn render_shell_command(tokens: &[Value]) -> String {
    tokens
        .iter()
        .filter_map(Value::as_str)
        .map(|token| {
            if token.starts_with('$') {
                format!("\"{token}\"")
            } else {
                token.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn check_workflow_entrypoint(
    findings: &mut Vec<Finding>,
    failure_classes: &serde_json::Map<String, Value>,
    label: &str,
    text: &str,
    command: &str,
) {
    let expected = format!("run: {command}");
    let count = text.lines().filter(|line| line.trim() == expected).count();
    if count == 0 {
        findings.push(Finding {
            failure_class: failure_class(failure_classes, "workflowDrift"),
            message: format!("{label}: missing provider pipeline entrypoint `{command}`"),
        });
    } else if count > 1 {
        findings.push(Finding {
            failure_class: failure_class(failure_classes, "workflowDrift"),
            message: format!("{label}: expected exactly one `{command}`, found {count}"),
        });
    }
}

fn check_forbidden(
    findings: &mut Vec<Finding>,
    failure_classes: &serde_json::Map<String, Value>,
    label: &str,
    text: &str,
) {
    for (reason, found) in forbidden_hits(text) {
        if found {
            findings.push(Finding {
                failure_class: failure_class(failure_classes, "workflowDrift"),
                message: format!("{label}: forbidden {reason}"),
            });
        }
    }
}

fn forbidden_hits(text: &str) -> Vec<(&'static str, bool)> {
    let lines = text.lines().map(str::trim).collect::<Vec<_>>();
    vec![
        (
            "direct required attestation call",
            lines.contains(&"run: python3 tools/ci/run_required_attested.py"),
        ),
        (
            "split required gate call",
            lines.contains(&"run: python3 tools/ci/run_required_checks.py"),
        ),
        (
            "split strict verify call",
            lines
                .iter()
                .any(|line| line.starts_with("run: python3 tools/ci/verify_required_witness.py")),
        ),
        (
            "split decision call",
            lines
                .iter()
                .any(|line| line.starts_with("run: python3 tools/ci/decide_required.py")),
        ),
        (
            "split decision verify call",
            lines
                .iter()
                .any(|line| line.starts_with("run: python3 tools/ci/verify_decision.py")),
        ),
        (
            "legacy instruction check call",
            lines.contains(&"run: sh tools/ci/run_task.sh ci-instruction-check"),
        ),
        (
            "legacy run_instruction shell call",
            text.contains("tools/ci/run_instruction.sh"),
        ),
        (
            "inline summary script block",
            text.contains("python3 - <<'PY'"),
        ),
    ]
}

fn check_wrapper_source(
    findings: &mut Vec<Finding>,
    failure_classes: &serde_json::Map<String, Value>,
    wrapper_id: &str,
    label: &str,
    text: &str,
    bound_surfaces: Vec<String>,
    enforced_gates: Vec<String>,
) {
    for surface_id in bound_surfaces {
        let marker = COMMAND_SURFACE_MARKERS
            .iter()
            .find_map(|(id, marker)| (*id == surface_id).then_some(*marker));
        if marker.is_none_or(|marker| !text.contains(marker)) {
            findings.push(Finding {
                failure_class: failure_class(failure_classes, "canonicalEntrypointDrift"),
                message: format!("{label}: missing contract-bound command surface `{surface_id}`"),
            });
        }
    }

    for gate_id in enforced_gates {
        let marker = GATE_MARKERS.iter().find_map(|(id, wrapper, marker)| {
            (*id == gate_id && *wrapper == wrapper_id).then_some(*marker)
        });
        if marker.is_none_or(|marker| !text.contains(marker)) {
            findings.push(Finding {
                failure_class: failure_class(failure_classes, "gateDrift"),
                message: format!("{label}: missing enforced gate `{gate_id}`"),
            });
        }
    }
}

fn value_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
