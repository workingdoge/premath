use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const CHECK_KIND: &str = "ci.pipeline_wiring_check.v1";
const DEFAULT_FAILURE_CLASS_UNBOUND: &str = "control_plane_pipeline_wrapper_unbound";
const DEFAULT_FAILURE_CLASS_PARITY_DRIFT: &str = "control_plane_pipeline_wrapper_parity_drift";
const DEFAULT_FAILURE_CLASS_GOVERNANCE_MISSING: &str =
    "control_plane_pipeline_governance_gate_missing";
const DEFAULT_FAILURE_CLASS_KCIR_MAPPING_MISSING: &str =
    "control_plane_pipeline_kcir_mapping_gate_missing";

const DEFAULT_REQUIRED_ENTRYPOINT: &[&str] = &["python3", "tools/ci/pipeline_required.py"];
const DEFAULT_INSTRUCTION_ENTRYPOINT: &[&str] = &[
    "python3",
    "tools/ci/pipeline_instruction.py",
    "--instruction",
    "$INSTRUCTION_PATH",
];
const DEFAULT_REQUIRED_GOVERNANCE_HOOK: &str = "governance_failure_classes";
const DEFAULT_REQUIRED_KCIR_HOOK: &str = "evaluate_required_mapping";
const DEFAULT_INSTRUCTION_GOVERNANCE_HOOK: &str = "governance_failure_classes";
const DEFAULT_INSTRUCTION_KCIR_HOOK: &str = "evaluate_instruction_mapping";

const FORBIDDEN_PATTERNS: [(&str, &str); 9] = [
    (
        "legacy required gate task call",
        r"^\s*run:\s*mise run ci-required-attested\s*$",
    ),
    (
        "legacy required gate split call",
        r"^\s*run:\s*mise run ci-required\s*$",
    ),
    (
        "legacy strict verify call",
        r"^\s*run:\s*mise run ci-verify-required-strict\s*$",
    ),
    (
        "legacy decision call",
        r"^\s*run:\s*mise run ci-decide-required\s*$",
    ),
    (
        "legacy decision verify call",
        r"^\s*run:\s*mise run ci-verify-decision\s*$",
    ),
    (
        "legacy provider env export call",
        r"^\s*run:\s*python3 tools/ci/providers/export_github_env.py",
    ),
    (
        "legacy instruction check call",
        r"^\s*run:\s*INSTRUCTION=.*mise run ci-instruction-check\s*$",
    ),
    (
        "legacy run_instruction shell call",
        r"tools/ci/run_instruction.sh",
    ),
    ("inline summary script block", r"python3 - <<'PY'"),
];

#[derive(Debug)]
struct Config {
    required_entrypoint: Vec<String>,
    instruction_entrypoint: Vec<String>,
    required_governance_hook: String,
    required_kcir_hook: String,
    instruction_governance_hook: String,
    instruction_kcir_hook: String,
    failure_unbound: String,
    failure_parity_drift: String,
    failure_governance_missing: String,
    failure_kcir_missing: String,
}

fn parse_string_tokens(value: Option<&Value>, fallback: &[&str]) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|tokens| {
            tokens
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|token| !token.is_empty())
                .map(|token| token.to_string())
                .collect::<Vec<_>>()
        })
        .filter(|tokens| !tokens.is_empty())
        .unwrap_or_else(|| fallback.iter().map(|item| item.to_string()).collect())
}

fn parse_string(value: Option<&Value>, fallback: &str) -> String {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn load_config(contract_path: &Path) -> Config {
    let bytes = fs::read(contract_path).ok();
    let payload: Option<Value> = bytes
        .as_deref()
        .and_then(|raw| serde_json::from_slice::<Value>(raw).ok());

    let pipeline = payload
        .as_ref()
        .and_then(|value| value.get("pipelineWrapperSurface"));
    let hooks_required = pipeline
        .and_then(|value| value.get("requiredGateHooks"))
        .and_then(Value::as_object);
    let hooks_instruction = pipeline
        .and_then(|value| value.get("instructionGateHooks"))
        .and_then(Value::as_object);
    let failure_classes = pipeline
        .and_then(|value| value.get("failureClasses"))
        .and_then(Value::as_object);

    Config {
        required_entrypoint: parse_string_tokens(
            pipeline.and_then(|value| value.get("requiredPipelineEntrypoint")),
            DEFAULT_REQUIRED_ENTRYPOINT,
        ),
        instruction_entrypoint: parse_string_tokens(
            pipeline.and_then(|value| value.get("instructionPipelineEntrypoint")),
            DEFAULT_INSTRUCTION_ENTRYPOINT,
        ),
        required_governance_hook: parse_string(
            hooks_required.and_then(|row| row.get("governance")),
            DEFAULT_REQUIRED_GOVERNANCE_HOOK,
        ),
        required_kcir_hook: parse_string(
            hooks_required.and_then(|row| row.get("kcirMapping")),
            DEFAULT_REQUIRED_KCIR_HOOK,
        ),
        instruction_governance_hook: parse_string(
            hooks_instruction.and_then(|row| row.get("governance")),
            DEFAULT_INSTRUCTION_GOVERNANCE_HOOK,
        ),
        instruction_kcir_hook: parse_string(
            hooks_instruction.and_then(|row| row.get("kcirMapping")),
            DEFAULT_INSTRUCTION_KCIR_HOOK,
        ),
        failure_unbound: parse_string(
            failure_classes.and_then(|row| row.get("unbound")),
            DEFAULT_FAILURE_CLASS_UNBOUND,
        ),
        failure_parity_drift: parse_string(
            failure_classes.and_then(|row| row.get("parityDrift")),
            DEFAULT_FAILURE_CLASS_PARITY_DRIFT,
        ),
        failure_governance_missing: parse_string(
            failure_classes.and_then(|row| row.get("governanceGateMissing")),
            DEFAULT_FAILURE_CLASS_GOVERNANCE_MISSING,
        ),
        failure_kcir_missing: parse_string(
            failure_classes.and_then(|row| row.get("kcirMappingGateMissing")),
            DEFAULT_FAILURE_CLASS_KCIR_MAPPING_MISSING,
        ),
    }
}

fn token_pattern(token: &str) -> String {
    if token.starts_with('$') {
        format!("\"?{}\"?", regex::escape(token))
    } else {
        regex::escape(token)
    }
}

fn entrypoint_pattern(tokens: &[String]) -> Result<Regex, String> {
    let rendered = tokens
        .iter()
        .map(|token| token_pattern(token))
        .collect::<Vec<_>>()
        .join(r"\s+");
    RegexBuilder::new(&format!(r"^\s*run:\s*{}\s*$", rendered))
        .multi_line(true)
        .build()
        .map_err(|err| format!("failed compiling pipeline entrypoint regex: {err}"))
}

fn render_entrypoint(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|token| {
            if token.starts_with('$') {
                format!("\"{}\"", token)
            } else {
                token.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_hook_call(text: &str, hook: &str) -> bool {
    if hook.trim().is_empty() {
        return false;
    }
    let pattern = format!(r"\b{}\s*\(", regex::escape(hook));
    Regex::new(&pattern)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

pub fn run(repo_root: String, control_plane_contract: String, json_output: bool) {
    let repo_root = PathBuf::from(repo_root);
    let contract_path = {
        let path = PathBuf::from(control_plane_contract);
        if path.is_absolute() {
            path
        } else {
            repo_root.join(path)
        }
    };
    let config = load_config(&contract_path);

    let baseline = repo_root.join(".github/workflows/baseline.yml");
    let instruction = repo_root.join(".github/workflows/instruction.yml");
    let required_pipeline_script = repo_root.join("tools/ci/pipeline_required.py");
    let instruction_pipeline_script = repo_root.join("tools/ci/pipeline_instruction.py");

    let mut errors: Vec<String> = Vec::new();
    let mut failure_classes: BTreeSet<String> = BTreeSet::new();

    if !baseline.exists() {
        failure_classes.insert(config.failure_unbound.clone());
        errors.push(format!("missing workflow: {}", baseline.display()));
    }
    if !instruction.exists() {
        failure_classes.insert(config.failure_unbound.clone());
        errors.push(format!("missing workflow: {}", instruction.display()));
    }

    if baseline.exists() {
        let text = fs::read_to_string(&baseline).unwrap_or_else(|err| {
            eprintln!("error: failed reading {}: {err}", baseline.display());
            std::process::exit(2);
        });
        let pattern = entrypoint_pattern(&config.required_entrypoint).unwrap_or_else(|err| {
            eprintln!("error: {err}");
            std::process::exit(2);
        });
        let count = pattern.find_iter(&text).count();
        let rendered = render_entrypoint(&config.required_entrypoint);
        if count == 0 {
            failure_classes.insert(config.failure_unbound.clone());
            errors.push(format!(
                "baseline.yml: missing required pipeline entrypoint `{rendered}`"
            ));
        } else if count > 1 {
            failure_classes.insert(config.failure_parity_drift.clone());
            errors.push(format!(
                "baseline.yml: expected exactly one `{rendered}`, found {count}"
            ));
        }

        for (reason, pattern) in FORBIDDEN_PATTERNS {
            let re = RegexBuilder::new(pattern)
                .multi_line(true)
                .build()
                .unwrap_or_else(|err| {
                    eprintln!("error: failed compiling forbidden pattern `{reason}`: {err}");
                    std::process::exit(2);
                });
            if re.is_match(&text) {
                failure_classes.insert(config.failure_parity_drift.clone());
                errors.push(format!("baseline.yml: forbidden {reason}"));
            }
        }
    }

    if instruction.exists() {
        let text = fs::read_to_string(&instruction).unwrap_or_else(|err| {
            eprintln!("error: failed reading {}: {err}", instruction.display());
            std::process::exit(2);
        });
        let pattern = entrypoint_pattern(&config.instruction_entrypoint).unwrap_or_else(|err| {
            eprintln!("error: {err}");
            std::process::exit(2);
        });
        let count = pattern.find_iter(&text).count();
        let rendered = render_entrypoint(&config.instruction_entrypoint);
        if count == 0 {
            failure_classes.insert(config.failure_unbound.clone());
            errors.push(format!(
                "instruction.yml: missing required pipeline entrypoint `{rendered}`"
            ));
        } else if count > 1 {
            failure_classes.insert(config.failure_parity_drift.clone());
            errors.push(format!(
                "instruction.yml: expected exactly one `{rendered}`, found {count}"
            ));
        }

        for (reason, pattern) in FORBIDDEN_PATTERNS {
            let re = RegexBuilder::new(pattern)
                .multi_line(true)
                .build()
                .unwrap_or_else(|err| {
                    eprintln!("error: failed compiling forbidden pattern `{reason}`: {err}");
                    std::process::exit(2);
                });
            if re.is_match(&text) {
                failure_classes.insert(config.failure_parity_drift.clone());
                errors.push(format!("instruction.yml: forbidden {reason}"));
            }
        }
    }

    let required_text = fs::read_to_string(&required_pipeline_script).unwrap_or_else(|err| {
        eprintln!(
            "error: failed reading required pipeline script {}: {err}",
            required_pipeline_script.display()
        );
        std::process::exit(2);
    });
    if !contains_hook_call(&required_text, &config.required_governance_hook) {
        failure_classes.insert(config.failure_governance_missing.clone());
        errors.push(format!(
            "pipeline_required.py: missing governance gate hook `{}`",
            config.required_governance_hook
        ));
    }
    if !contains_hook_call(&required_text, &config.required_kcir_hook) {
        failure_classes.insert(config.failure_kcir_missing.clone());
        errors.push(format!(
            "pipeline_required.py: missing kcir mapping gate hook `{}`",
            config.required_kcir_hook
        ));
    }

    let instruction_text = fs::read_to_string(&instruction_pipeline_script).unwrap_or_else(|err| {
        eprintln!(
            "error: failed reading instruction pipeline script {}: {err}",
            instruction_pipeline_script.display()
        );
        std::process::exit(2);
    });
    if !contains_hook_call(&instruction_text, &config.instruction_governance_hook) {
        failure_classes.insert(config.failure_governance_missing.clone());
        errors.push(format!(
            "pipeline_instruction.py: missing governance gate hook `{}`",
            config.instruction_governance_hook
        ));
    }
    if !contains_hook_call(&instruction_text, &config.instruction_kcir_hook) {
        failure_classes.insert(config.failure_kcir_missing.clone());
        errors.push(format!(
            "pipeline_instruction.py: missing kcir mapping gate hook `{}`",
            config.instruction_kcir_hook
        ));
    }

    let failure_classes = failure_classes.into_iter().collect::<Vec<_>>();
    let result = if errors.is_empty() {
        "accepted"
    } else {
        "rejected"
    };

    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": CHECK_KIND,
            "result": result,
            "failureClasses": failure_classes,
            "errors": errors,
            "requiredEntrypoint": config.required_entrypoint,
            "instructionEntrypoint": config.instruction_entrypoint,
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
            eprintln!("error: failed to render pipeline-wiring-check payload: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else if errors.is_empty() {
        println!(
            "[pipeline-wiring] OK (baseline={}, instruction={})",
            render_entrypoint(&config.required_entrypoint),
            render_entrypoint(&config.instruction_entrypoint),
        );
    } else {
        println!(
            "[pipeline-wiring] FAIL (failureClasses={:?})",
            failure_classes
        );
        for err in &errors {
            println!("  - {err}");
        }
    }

    if !errors.is_empty() {
        std::process::exit(1);
    }
}
