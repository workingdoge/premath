use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CHECK_KIND: &str = "ci.branch_policy_check.v1";
const POLICY_KIND: &str = "premath.github.branch_policy.v1";

#[derive(Debug, Clone)]
struct BranchPolicy {
    policy_id: String,
    repository: String,
    branch: String,
    required_rule_types: Vec<String>,
    required_status_checks: Vec<String>,
    strict_status_checks: bool,
    require_pull_request: bool,
    forbid_bypass_actors: bool,
}

fn load_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    serde_json::from_str::<Value>(&text).map_err(|err| format!("{}: {err}", path.display()))
}

fn require_non_empty_string(
    payload: &Map<String, Value>,
    key: &str,
    path: &Path,
) -> Result<String, String> {
    let value = payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if value.is_empty() {
        return Err(format!(
            "{}: {} must be a non-empty string",
            path.display(),
            key
        ));
    }
    Ok(value.to_string())
}

fn require_string_list(
    payload: &Map<String, Value>,
    key: &str,
    path: &Path,
) -> Result<Vec<String>, String> {
    let rows = payload.get(key).and_then(Value::as_array).ok_or_else(|| {
        format!(
            "{}: {} must be a non-empty list of strings",
            path.display(),
            key
        )
    })?;
    if rows.is_empty() {
        return Err(format!(
            "{}: {} must be a non-empty list of strings",
            path.display(),
            key
        ));
    }
    let mut out = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        let value = row.as_str().map(str::trim).unwrap_or_default();
        if value.is_empty() {
            return Err(format!(
                "{}: {}[{}] must be a non-empty string",
                path.display(),
                key,
                idx
            ));
        }
        out.push(value.to_string());
    }
    let unique = out.iter().collect::<BTreeSet<_>>();
    if unique.len() != out.len() {
        return Err(format!(
            "{}: {} must not contain duplicates",
            path.display(),
            key
        ));
    }
    Ok(out)
}

fn parse_policy(path: &Path) -> Result<BranchPolicy, String> {
    let payload = load_json(path)?;
    let object = payload
        .as_object()
        .ok_or_else(|| format!("{}: policy root must be an object", path.display()))?;

    if object.get("schema").and_then(Value::as_i64) != Some(1) {
        return Err(format!("{}: schema must be 1", path.display()));
    }
    if object.get("policyKind").and_then(Value::as_str) != Some(POLICY_KIND) {
        return Err(format!(
            "{}: policyKind must be {:?}",
            path.display(),
            POLICY_KIND
        ));
    }

    let repository = require_non_empty_string(object, "repository", path)?;
    if !repository.contains('/') {
        return Err(format!("{}: repository must be owner/name", path.display()));
    }

    Ok(BranchPolicy {
        policy_id: require_non_empty_string(object, "policyId", path)?,
        repository,
        branch: require_non_empty_string(object, "branch", path)?,
        required_rule_types: require_string_list(object, "requiredRuleTypes", path)?,
        required_status_checks: require_string_list(object, "requiredStatusChecks", path)?,
        strict_status_checks: object
            .get("strictStatusChecks")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        require_pull_request: object
            .get("requirePullRequest")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        forbid_bypass_actors: object
            .get("forbidBypassActors")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn fetch_json_with_curl(url: &str, token: &str) -> Result<Value, String> {
    let auth_header = format!("Authorization: Bearer {token}");
    let output = Command::new("curl")
        .args([
            "-sS",
            "-L",
            "--fail",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            auth_header.as_str(),
            "-H",
            "X-GitHub-Api-Version: 2022-11-28",
            "-H",
            "User-Agent: premath-branch-policy-check",
            url,
        ])
        .output()
        .map_err(|err| format!("live rules fetch failed for {url}: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("curl exit status {}", output.status.code().unwrap_or(1))
        };
        return Err(format!("live rules fetch failed for {url}: {detail}"));
    }
    serde_json::from_slice::<Value>(&output.stdout)
        .map_err(|err| format!("live rules payload parse failed for {url}: {err}"))
}

fn fetch_live_rules(api_url: &str, repo: &str, branch: &str, token: &str) -> Result<Value, String> {
    let base = api_url.trim_end_matches('/');
    let rules_url = format!("{base}/repos/{repo}/rules/branches/{branch}");
    let payload = fetch_json_with_curl(&rules_url, token)?;
    if payload.as_array().is_some_and(|rows| rows.is_empty()) {
        let protection_url = format!("{base}/repos/{repo}/branches/{branch}/protection");
        return fetch_json_with_curl(&protection_url, token);
    }
    Ok(payload)
}

fn admin_bypass_enabled(payload: &Value) -> Option<bool> {
    let object = payload.as_object()?;
    let enforce_admins = object.get("enforce_admins")?;
    if let Some(value) = enforce_admins.as_bool() {
        return Some(!value);
    }
    if let Some(enabled) = enforce_admins
        .as_object()
        .and_then(|row| row.get("enabled"))
        .and_then(Value::as_bool)
    {
        return Some(!enabled);
    }
    None
}

fn normalize_actor(row: &Value) -> String {
    if let Some(value) = row.as_str() {
        return value.to_string();
    }
    if let Some(object) = row.as_object() {
        let actor_type = object
            .get("actor_type")
            .or_else(|| object.get("actorType"))
            .and_then(Value::as_str);
        let actor_id = object
            .get("actor_id")
            .or_else(|| object.get("actorId"))
            .or_else(|| object.get("id"));
        let actor_name = object
            .get("login")
            .or_else(|| object.get("slug"))
            .or_else(|| object.get("name"))
            .and_then(Value::as_str);
        if let (Some(actor_type), Some(actor_id)) = (actor_type, actor_id) {
            return format!("{actor_type}:{actor_id}");
        }
        if let (Some(actor_type), Some(actor_name)) = (actor_type, actor_name) {
            return format!("{actor_type}:{actor_name}");
        }
        if let Some(actor_name) = actor_name {
            return actor_name.to_string();
        }
    }
    serde_json::to_string(row).unwrap_or_else(|_| "<unrenderable-actor>".to_string())
}

fn collect_bypass_pull_request_allowances(allowances: &Map<String, Value>) -> Vec<String> {
    let mut out = Vec::new();
    for key in ["users", "teams", "apps"] {
        if let Some(rows) = allowances.get(key).and_then(Value::as_array) {
            for row in rows {
                out.push(format!("{key}:{}", normalize_actor(row)));
            }
        }
    }
    out
}

fn collect_bypass_actors(payload: &Value) -> Vec<String> {
    fn visit(node: &Value, out: &mut Vec<String>) {
        if let Some(object) = node.as_object() {
            for (key, value) in object {
                if key == "bypass_actors"
                    && let Some(rows) = value.as_array()
                {
                    for actor in rows {
                        out.push(normalize_actor(actor));
                    }
                }
                if key == "bypass_pull_request_allowances"
                    && let Some(allowances) = value.as_object()
                {
                    out.extend(collect_bypass_pull_request_allowances(allowances));
                }
                visit(value, out);
            }
            return;
        }
        if let Some(rows) = node.as_array() {
            for row in rows {
                visit(row, out);
            }
        }
    }

    let mut out = Vec::new();
    visit(payload, &mut out);
    let unique = out.into_iter().collect::<BTreeSet<_>>();
    unique.into_iter().collect()
}

fn as_rule_list(payload: &Value) -> Vec<Map<String, Value>> {
    if let Some(rows) = payload.as_array() {
        return rows
            .iter()
            .filter_map(|row| row.as_object().cloned())
            .collect();
    }

    let Some(object) = payload.as_object() else {
        return Vec::new();
    };
    if let Some(rules) = object.get("rules").and_then(Value::as_array) {
        return rules
            .iter()
            .filter_map(|row| row.as_object().cloned())
            .collect();
    }

    let mut synthetic = Vec::new();

    if let Some(required_status) = object
        .get("required_status_checks")
        .and_then(Value::as_object)
    {
        let mut checks = Vec::new();
        if let Some(contexts) = required_status.get("contexts").and_then(Value::as_array) {
            for row in contexts {
                if let Some(context) = row.as_str().map(str::trim)
                    && !context.is_empty()
                {
                    checks.push(json!({ "context": context }));
                }
            }
        }
        synthetic.push(json!({
            "type": "required_status_checks",
            "parameters": {
                "strict_required_status_checks_policy": required_status
                    .get("strict")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                "required_status_checks": checks,
            }
        }));
    }

    if object
        .get("required_pull_request_reviews")
        .and_then(Value::as_object)
        .is_some()
    {
        synthetic.push(json!({
            "type": "pull_request",
            "parameters": object.get("required_pull_request_reviews").cloned().unwrap_or(Value::Null),
        }));
    }

    if object
        .get("allow_force_pushes")
        .and_then(Value::as_object)
        .and_then(|row| row.get("enabled"))
        .and_then(Value::as_bool)
        == Some(false)
    {
        synthetic.push(json!({ "type": "non_fast_forward" }));
    }
    if object
        .get("allow_deletions")
        .and_then(Value::as_object)
        .and_then(|row| row.get("enabled"))
        .and_then(Value::as_bool)
        == Some(false)
    {
        synthetic.push(json!({ "type": "deletion" }));
    }

    synthetic
        .into_iter()
        .filter_map(|row| row.as_object().cloned())
        .collect()
}

fn extract_rule_types(rules: &[Map<String, Value>]) -> Vec<String> {
    let mut out = BTreeSet::new();
    for row in rules {
        if let Some(rule_type) = row.get("type").and_then(Value::as_str) {
            let trimmed = rule_type.trim();
            if !trimmed.is_empty() {
                out.insert(trimmed.to_string());
            }
        }
    }
    out.into_iter().collect()
}

fn extract_required_status_contexts(rules: &[Map<String, Value>]) -> Vec<String> {
    let mut out = BTreeSet::new();
    for row in rules {
        if row.get("type").and_then(Value::as_str) != Some("required_status_checks") {
            continue;
        }
        let Some(parameters) = row.get("parameters").and_then(Value::as_object) else {
            continue;
        };
        let Some(checks) = parameters
            .get("required_status_checks")
            .and_then(Value::as_array)
        else {
            continue;
        };
        for check in checks {
            if let Some(context) = check
                .as_object()
                .and_then(|object| object.get("context"))
                .and_then(Value::as_str)
            {
                let trimmed = context.trim();
                if !trimmed.is_empty() {
                    out.insert(trimmed.to_string());
                }
                continue;
            }
            if let Some(context) = check.as_str() {
                let trimmed = context.trim();
                if !trimmed.is_empty() {
                    out.insert(trimmed.to_string());
                }
            }
        }
    }
    out.into_iter().collect()
}

fn extract_strict_status_checks(rules: &[Map<String, Value>]) -> Option<bool> {
    let mut values = Vec::new();
    for row in rules {
        if row.get("type").and_then(Value::as_str) != Some("required_status_checks") {
            continue;
        }
        let Some(parameters) = row.get("parameters").and_then(Value::as_object) else {
            continue;
        };
        if let Some(value) = parameters
            .get("strict_required_status_checks_policy")
            .and_then(Value::as_bool)
        {
            values.push(value);
        }
    }
    if values.is_empty() {
        None
    } else {
        Some(values.into_iter().all(std::convert::identity))
    }
}

fn evaluate_policy(policy: &BranchPolicy, payload: &Value) -> (Vec<String>, Value) {
    let rules = as_rule_list(payload);
    let rule_types = extract_rule_types(&rules);
    let required_status_checks = extract_required_status_contexts(&rules);
    let strict_status_checks = extract_strict_status_checks(&rules);
    let bypass_actors = collect_bypass_actors(payload);
    let admin_bypass_enabled = admin_bypass_enabled(payload);

    let mut errors = Vec::new();
    if rules.is_empty() {
        errors.push("missing rules surface in payload".to_string());
    }

    for rule_type in &policy.required_rule_types {
        if !rule_types.contains(rule_type) {
            errors.push(format!("missing required rule type: {rule_type}"));
        }
    }

    if policy.require_pull_request && !rule_types.iter().any(|rule| rule == "pull_request") {
        errors.push("pull_request rule missing while requirePullRequest=true".to_string());
    }

    for check in &policy.required_status_checks {
        if !required_status_checks.contains(check) {
            errors.push(format!("missing required status check context: {check}"));
        }
    }

    if policy.strict_status_checks && strict_status_checks != Some(true) {
        errors.push("strict status checks policy is not enabled".to_string());
    }

    if policy.forbid_bypass_actors && !bypass_actors.is_empty() {
        errors.push(format!(
            "bypass actors present: {}",
            bypass_actors.join(", ")
        ));
    }
    if policy.forbid_bypass_actors && admin_bypass_enabled == Some(true) {
        errors.push("admin bypass path enabled: enforce_admins=false".to_string());
    }

    let details = json!({
        "ruleTypes": rule_types,
        "requiredStatusChecks": required_status_checks,
        "strictStatusChecks": strict_status_checks,
        "bypassActors": bypass_actors,
        "adminBypassEnabled": admin_bypass_enabled,
    });
    (errors, details)
}

fn print_json_result(
    result: &str,
    failure_classes: Vec<&str>,
    policy_id: Option<&str>,
    source: Option<&str>,
    errors: Vec<String>,
    details: Value,
) {
    let payload = json!({
        "schema": 1,
        "checkKind": CHECK_KIND,
        "result": result,
        "failureClasses": failure_classes,
        "policyId": policy_id,
        "source": source,
        "errors": errors,
        "details": details,
    });
    let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|err| {
        eprintln!("error: failed to render branch-policy-check payload: {err}");
        std::process::exit(2);
    });
    println!("{rendered}");
}

pub struct Args {
    pub policy: String,
    pub rules_json: Option<String>,
    pub fetch_live: bool,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub github_api_url: String,
    pub token_env: String,
    pub json_output: bool,
}

pub fn run(args: Args) {
    let policy_path = PathBuf::from(&args.policy);
    let policy = match parse_policy(&policy_path) {
        Ok(policy) => policy,
        Err(err) => {
            if args.json_output {
                print_json_result(
                    "rejected",
                    vec!["branch_policy_policy_invalid"],
                    None,
                    None,
                    vec![err],
                    json!({}),
                );
            } else {
                println!("[branch-policy-check] FAIL policy-invalid: {err}");
            }
            std::process::exit(1);
        }
    };

    if args.rules_json.is_some() == args.fetch_live {
        let error =
            "choose exactly one of --rules-json or --fetch-live (one is required)".to_string();
        if args.json_output {
            print_json_result(
                "rejected",
                vec!["branch_policy_invalid_args"],
                Some(&policy.policy_id),
                None,
                vec![error],
                json!({}),
            );
        } else {
            println!("[branch-policy-check] FAIL invalid-args: {error}");
        }
        std::process::exit(1);
    }

    let repo = args.repo.unwrap_or_else(|| policy.repository.clone());
    let branch = args.branch.unwrap_or_else(|| policy.branch.clone());

    let (payload, source) = if let Some(rules_json) = args.rules_json {
        let payload_path = PathBuf::from(rules_json);
        let payload = match load_json(&payload_path) {
            Ok(payload) => payload,
            Err(err) => {
                if args.json_output {
                    print_json_result(
                        "rejected",
                        vec!["branch_policy_input_invalid"],
                        Some(&policy.policy_id),
                        Some(payload_path.to_string_lossy().as_ref()),
                        vec![err.clone()],
                        json!({}),
                    );
                } else {
                    println!("[branch-policy-check] FAIL input-invalid: {err}");
                }
                std::process::exit(1);
            }
        };
        (payload, payload_path.to_string_lossy().to_string())
    } else {
        let token = std::env::var(&args.token_env).unwrap_or_default();
        let token = token.trim().to_string();
        if token.is_empty() {
            let error = format!("env {} is required for --fetch-live", args.token_env);
            if args.json_output {
                print_json_result(
                    "rejected",
                    vec!["branch_policy_missing_token"],
                    Some(&policy.policy_id),
                    None,
                    vec![error.clone()],
                    json!({}),
                );
            } else {
                println!("[branch-policy-check] FAIL missing-token: {error}");
            }
            std::process::exit(1);
        }
        let payload = match fetch_live_rules(&args.github_api_url, &repo, &branch, &token) {
            Ok(payload) => payload,
            Err(err) => {
                if args.json_output {
                    print_json_result(
                        "rejected",
                        vec!["branch_policy_live_fetch_error"],
                        Some(&policy.policy_id),
                        Some(&format!("live:{repo}:{branch}")),
                        vec![err.clone()],
                        json!({}),
                    );
                } else {
                    println!("[branch-policy-check] FAIL live-fetch: {err}");
                }
                std::process::exit(1);
            }
        };
        (payload, format!("live:{repo}:{branch}"))
    };

    let (errors, details) = evaluate_policy(&policy, &payload);
    if !errors.is_empty() {
        if args.json_output {
            print_json_result(
                "rejected",
                vec!["branch_policy_violation"],
                Some(&policy.policy_id),
                Some(&source),
                errors,
                details,
            );
        } else {
            println!(
                "[branch-policy-check] FAIL (policyId={}, source={}, errors={})",
                policy.policy_id,
                source,
                errors.len()
            );
            for error in &errors {
                println!("  - {error}");
            }
            println!(
                "[branch-policy-check] DETAILS {}",
                serde_json::to_string(&details).unwrap_or_else(|_| "{}".to_string())
            );
        }
        std::process::exit(1);
    }

    if args.json_output {
        print_json_result(
            "accepted",
            Vec::new(),
            Some(&policy.policy_id),
            Some(&source),
            Vec::new(),
            details.clone(),
        );
    } else {
        let rule_type_count = details
            .get("ruleTypes")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        let required_checks_count = details
            .get("requiredStatusChecks")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        println!(
            "[branch-policy-check] OK (policyId={}, source={}, ruleTypes={}, requiredChecks={})",
            policy.policy_id, source, rule_type_count, required_checks_count
        );
    }
}
