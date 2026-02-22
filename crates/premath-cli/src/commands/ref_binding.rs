use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;

const REF_PROFILE_KIND: &str = "premath.ref_profile.v1";
const EVIDENCE_POLICY_EMPTY_ONLY: &str = "empty_only";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RefProfile {
    pub schema: u32,
    pub profile_kind: String,
    pub profile_id: String,
    pub scheme_id: String,
    pub params_hash: String,
    pub supported_domains: Vec<String>,
    pub evidence_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BoundRef {
    pub scheme_id: String,
    pub params_hash: String,
    pub domain: String,
    pub digest: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectOutput {
    schema: u32,
    profile_id: String,
    #[serde(rename = "ref")]
    bound_ref: BoundRef,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VerifyOutput {
    schema: u32,
    result: String,
    failure_classes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    projected_ref: Option<BoundRef>,
}

fn ensure_non_empty(value: &str, label: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} must be a non-empty string"));
    }
    Ok(trimmed.to_string())
}

fn parse_hex_bytes(value: &str, label: &str) -> Result<Vec<u8>, String> {
    let normalized = value.trim();
    if normalized.len() % 2 != 0 {
        return Err(format!("{label} must be even-length hex"));
    }
    if normalized.is_empty() {
        return Ok(Vec::new());
    }
    if !normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!("{label} must be valid hex"));
    }
    let mut out = Vec::with_capacity(normalized.len() / 2);
    let bytes = normalized.as_bytes();
    for idx in (0..bytes.len()).step_by(2) {
        let hi = (bytes[idx] as char)
            .to_digit(16)
            .ok_or_else(|| format!("{label} must be valid hex"))?;
        let lo = (bytes[idx + 1] as char)
            .to_digit(16)
            .ok_or_else(|| format!("{label} must be valid hex"))?;
        out.push(((hi << 4) | lo) as u8);
    }
    Ok(out)
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(format!("{byte:02x}").as_str());
    }
    out
}

fn canonical_json(value: &Value) -> String {
    fn render(value: &Value, out: &mut String) {
        match value {
            Value::Null => out.push_str("null"),
            Value::Bool(flag) => {
                if *flag {
                    out.push_str("true");
                } else {
                    out.push_str("false");
                }
            }
            Value::Number(number) => out.push_str(number.to_string().as_str()),
            Value::String(text) => {
                out.push_str(
                    serde_json::to_string(text)
                        .expect("string serialization should always succeed")
                        .as_str(),
                );
            }
            Value::Array(items) => {
                out.push('[');
                for (idx, item) in items.iter().enumerate() {
                    if idx > 0 {
                        out.push(',');
                    }
                    render(item, out);
                }
                out.push(']');
            }
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort_unstable();
                out.push('{');
                for (idx, key) in keys.iter().enumerate() {
                    if idx > 0 {
                        out.push(',');
                    }
                    out.push_str(
                        serde_json::to_string(key)
                            .expect("object key serialization should always succeed")
                            .as_str(),
                    );
                    out.push(':');
                    let value = map
                        .get(*key)
                        .expect("iterating object keys should always resolve value");
                    render(value, out);
                }
                out.push('}');
            }
        }
    }

    let mut out = String::new();
    render(value, &mut out);
    out
}

fn stable_sha256_hex(value: &Value) -> String {
    let canonical = canonical_json(value);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn load_profile(profile_path: &str) -> Result<RefProfile, String> {
    let bytes = fs::read(profile_path)
        .map_err(|err| format!("failed to read profile {}: {err}", profile_path))?;
    let parsed: RefProfile = serde_json::from_slice(&bytes)
        .map_err(|err| format!("failed to parse profile {}: {err}", profile_path))?;

    if parsed.schema != 1 {
        return Err("profile schema must be 1".to_string());
    }
    if parsed.profile_kind != REF_PROFILE_KIND {
        return Err(format!(
            "profileKind must be {} (actual={})",
            REF_PROFILE_KIND, parsed.profile_kind
        ));
    }
    let _ = ensure_non_empty(parsed.profile_id.as_str(), "profileId")?;
    let _ = ensure_non_empty(parsed.scheme_id.as_str(), "schemeId")?;
    let _ = ensure_non_empty(parsed.params_hash.as_str(), "paramsHash")?;
    if parsed.evidence_policy != EVIDENCE_POLICY_EMPTY_ONLY {
        return Err(format!(
            "unsupported evidencePolicy '{}' (expected '{}')",
            parsed.evidence_policy, EVIDENCE_POLICY_EMPTY_ONLY
        ));
    }
    if parsed.supported_domains.is_empty() {
        return Err("supportedDomains must be non-empty".to_string());
    }
    let mut domain_set = BTreeSet::new();
    for (idx, domain) in parsed.supported_domains.iter().enumerate() {
        let text = ensure_non_empty(domain, format!("supportedDomains[{idx}]").as_str())?;
        if !domain_set.insert(text) {
            return Err(format!("supportedDomains contains duplicate '{domain}'"));
        }
    }

    Ok(parsed)
}

fn project_ref(profile: &RefProfile, domain: &str, payload_hex: &str) -> Result<BoundRef, String> {
    let normalized_domain = ensure_non_empty(domain, "domain")?;
    if !profile
        .supported_domains
        .iter()
        .any(|item| item == normalized_domain.as_str())
    {
        return Err("kcir_v2.domain_mismatch".to_string());
    }

    let payload = parse_hex_bytes(payload_hex, "payloadHex")
        .map_err(|_| "kcir_v2.parse_error".to_string())?;
    let projection_material = json!({
        "schemeId": profile.scheme_id,
        "paramsHash": profile.params_hash,
        "domain": normalized_domain,
        "payloadHex": hex_lower(&payload),
    });
    let digest = stable_sha256_hex(&projection_material);

    Ok(BoundRef {
        scheme_id: profile.scheme_id.clone(),
        params_hash: profile.params_hash.clone(),
        domain: normalized_domain,
        digest,
    })
}

fn verify_ref(
    profile: &RefProfile,
    domain: &str,
    payload_hex: &str,
    evidence_hex: &str,
    provided_ref: BoundRef,
) -> VerifyOutput {
    let payload = match parse_hex_bytes(payload_hex, "payloadHex") {
        Ok(value) => value,
        Err(_) => {
            return VerifyOutput {
                schema: 1,
                result: "rejected".to_string(),
                failure_classes: vec!["kcir_v2.parse_error".to_string()],
                projected_ref: None,
            };
        }
    };
    let evidence = match parse_hex_bytes(evidence_hex, "evidenceHex") {
        Ok(value) => value,
        Err(_) => {
            return VerifyOutput {
                schema: 1,
                result: "rejected".to_string(),
                failure_classes: vec!["kcir_v2.parse_error".to_string()],
                projected_ref: None,
            };
        }
    };

    let normalized_domain = match ensure_non_empty(domain, "domain") {
        Ok(value) => value,
        Err(_) => {
            return VerifyOutput {
                schema: 1,
                result: "rejected".to_string(),
                failure_classes: vec!["kcir_v2.domain_mismatch".to_string()],
                projected_ref: None,
            };
        }
    };

    let projected = {
        let projection_material = json!({
            "schemeId": profile.scheme_id,
            "paramsHash": profile.params_hash,
            "domain": normalized_domain,
            "payloadHex": hex_lower(&payload),
        });
        BoundRef {
            scheme_id: profile.scheme_id.clone(),
            params_hash: profile.params_hash.clone(),
            domain: normalized_domain.clone(),
            digest: stable_sha256_hex(&projection_material),
        }
    };

    let mut failures = Vec::new();
    if provided_ref.scheme_id != profile.scheme_id {
        failures.push("kcir_v2.profile_mismatch".to_string());
    } else if provided_ref.params_hash != profile.params_hash {
        failures.push("kcir_v2.params_hash_mismatch".to_string());
    } else if provided_ref.domain != normalized_domain
        || !profile
            .supported_domains
            .iter()
            .any(|item| item == normalized_domain.as_str())
        || !profile
            .supported_domains
            .iter()
            .any(|item| item == provided_ref.domain.as_str())
    {
        failures.push("kcir_v2.domain_mismatch".to_string());
    } else if provided_ref.digest != projected.digest {
        failures.push("kcir_v2.digest_mismatch".to_string());
    } else if profile.evidence_policy == EVIDENCE_POLICY_EMPTY_ONLY && !evidence.is_empty() {
        failures.push("kcir_v2.evidence_invalid".to_string());
    }

    VerifyOutput {
        schema: 1,
        result: if failures.is_empty() {
            "accepted".to_string()
        } else {
            "rejected".to_string()
        },
        failure_classes: failures,
        projected_ref: Some(projected),
    }
}

pub fn run_project(profile: String, domain: String, payload_hex: String, json_output: bool) {
    let profile_value = load_profile(profile.as_str()).unwrap_or_else(|err| {
        eprintln!("{err}");
        std::process::exit(2);
    });
    let projected = project_ref(&profile_value, domain.as_str(), payload_hex.as_str())
        .unwrap_or_else(|err| {
            eprintln!("{err}");
            std::process::exit(2);
        });

    let output = ProjectOutput {
        schema: 1,
        profile_id: profile_value.profile_id,
        bound_ref: projected,
    };

    if json_output {
        let rendered = serde_json::to_string_pretty(&output).unwrap_or_else(|err| {
            eprintln!("failed to render ref project output: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
        return;
    }

    println!("premath ref project");
    println!("  Profile: {}", output.profile_id);
    println!("  Domain: {}", output.bound_ref.domain);
    println!("  Digest: {}", output.bound_ref.digest);
}

pub struct VerifyInput {
    pub profile: String,
    pub domain: String,
    pub payload_hex: String,
    pub evidence_hex: String,
    pub ref_scheme_id: String,
    pub ref_params_hash: String,
    pub ref_domain: String,
    pub ref_digest: String,
    pub json_output: bool,
}

pub fn run_verify(input: VerifyInput) {
    let profile_value = load_profile(input.profile.as_str()).unwrap_or_else(|err| {
        eprintln!("{err}");
        std::process::exit(2);
    });
    let provided_ref = BoundRef {
        scheme_id: input.ref_scheme_id,
        params_hash: input.ref_params_hash,
        domain: input.ref_domain,
        digest: input.ref_digest,
    };
    let output = verify_ref(
        &profile_value,
        input.domain.as_str(),
        input.payload_hex.as_str(),
        input.evidence_hex.as_str(),
        provided_ref,
    );

    if input.json_output {
        let rendered = serde_json::to_string_pretty(&output).unwrap_or_else(|err| {
            eprintln!("failed to render ref verify output: {err}");
            std::process::exit(2);
        });
        println!("{rendered}");
        return;
    }

    println!("premath ref verify");
    println!("  Result: {}", output.result);
    if !output.failure_classes.is_empty() {
        println!("  Failure classes:");
        for class_name in &output.failure_classes {
            println!("    - {class_name}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile() -> RefProfile {
        RefProfile {
            schema: 1,
            profile_kind: REF_PROFILE_KIND.to_string(),
            profile_id: "ref.sha256.detached.v1".to_string(),
            scheme_id: "ref.sha256.detached.v1".to_string(),
            params_hash: "sha256.detached.params.v1".to_string(),
            supported_domains: vec![
                "kcir.node".to_string(),
                "kcir.obj_nf".to_string(),
                "kcir.mor_nf".to_string(),
            ],
            evidence_policy: EVIDENCE_POLICY_EMPTY_ONLY.to_string(),
        }
    }

    #[test]
    fn project_ref_is_deterministic() {
        let profile = sample_profile();
        let left =
            project_ref(&profile, "kcir.node", "DEADBEEF").expect("projection should succeed");
        let right =
            project_ref(&profile, "kcir.node", "deadbeef").expect("projection should succeed");
        assert_eq!(left, right);
        assert_eq!(left.scheme_id, profile.scheme_id);
        assert_eq!(left.params_hash, profile.params_hash);
        assert_eq!(left.domain, "kcir.node");
        assert_eq!(left.digest.len(), 64);
    }

    #[test]
    fn verify_ref_rejects_digest_mismatch() {
        let profile = sample_profile();
        let provided = BoundRef {
            scheme_id: profile.scheme_id.clone(),
            params_hash: profile.params_hash.clone(),
            domain: "kcir.node".to_string(),
            digest: "00".repeat(32),
        };
        let outcome = verify_ref(&profile, "kcir.node", "deadbeef", "", provided);
        assert_eq!(outcome.result, "rejected");
        assert_eq!(
            outcome.failure_classes,
            vec!["kcir_v2.digest_mismatch".to_string()]
        );
    }

    #[test]
    fn verify_ref_rejects_non_empty_evidence_when_empty_only_policy() {
        let profile = sample_profile();
        let projected =
            project_ref(&profile, "kcir.node", "deadbeef").expect("projection should succeed");
        let outcome = verify_ref(&profile, "kcir.node", "deadbeef", "aa", projected);
        assert_eq!(outcome.result, "rejected");
        assert_eq!(
            outcome.failure_classes,
            vec!["kcir_v2.evidence_invalid".to_string()]
        );
    }
}
