use chrono::Utc;
use premath_kernel::canonical_digest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

const SITE_PACKAGE_REL: &str =
    "specs/premath/site-packages/premath.doctrine_operation_site.v0/SITE-PACKAGE.json";
const CHANGE_LOG_REL: &str = ".premath/site-change-log.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeLogEntry {
    pub change_id: String,
    pub from_digest: String,
    pub to_digest: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChainCheckReport {
    schema: u32,
    check_kind: String,
    result: String,
    failure_classes: Vec<String>,
    current_digest: String,
    last_log_digest: String,
    log_entry_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic: Option<String>,
}

pub fn run(json_output: bool, repo_root: String) {
    let root = PathBuf::from(&repo_root);
    let package_path = root.join(SITE_PACKAGE_REL);
    let log_path = root.join(CHANGE_LOG_REL);

    let package_json = match fs::read_to_string(&package_path) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("error: failed to read {}: {}", package_path.display(), err);
            std::process::exit(2);
        }
    };

    let package: Value = match serde_json::from_str(&package_json) {
        Ok(v) => v,
        Err(err) => {
            eprintln!("error: failed to parse SITE-PACKAGE.json: {}", err);
            std::process::exit(2);
        }
    };

    let current_digest = canonical_digest(&package);

    // Read log entries
    let entries = if log_path.exists() {
        let file = match fs::File::open(&log_path) {
            Ok(f) => f,
            Err(err) => {
                eprintln!(
                    "error: failed to open change log {}: {}",
                    log_path.display(),
                    err
                );
                std::process::exit(2);
            }
        };
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(err) => {
                    eprintln!("error: failed to read change log line: {}", err);
                    std::process::exit(2);
                }
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<ChangeLogEntry>(trimmed) {
                Ok(entry) => entries.push(entry),
                Err(err) => {
                    eprintln!("error: failed to parse change log entry: {}", err);
                    std::process::exit(2);
                }
            }
        }
        entries
    } else {
        Vec::new()
    };

    // Genesis: if no entries, seed from current state
    if entries.is_empty() {
        let genesis = ChangeLogEntry {
            change_id: "genesis".to_string(),
            from_digest: current_digest.clone(),
            to_digest: current_digest.clone(),
            timestamp: Utc::now().to_rfc3339(),
        };
        if let Some(parent) = log_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let line = serde_json::to_string(&genesis).expect("genesis entry should serialize");
        if let Err(err) = fs::write(&log_path, format!("{line}\n")) {
            eprintln!(
                "error: failed to write genesis entry to {}: {}",
                log_path.display(),
                err
            );
            std::process::exit(2);
        }

        let report = ChainCheckReport {
            schema: 1,
            check_kind: "premath.site_change_chain_check.v1".to_string(),
            result: "accepted".to_string(),
            failure_classes: vec![],
            current_digest: current_digest.clone(),
            last_log_digest: current_digest,
            log_entry_count: 1,
            diagnostic: Some("genesis entry created".to_string()),
        };

        emit_report(&report, json_output);
        std::process::exit(0);
    }

    // Check: current digest must match last toDigest
    let last_entry = entries.last().unwrap();
    let last_to_digest = &last_entry.to_digest;

    if current_digest == *last_to_digest {
        let report = ChainCheckReport {
            schema: 1,
            check_kind: "premath.site_change_chain_check.v1".to_string(),
            result: "accepted".to_string(),
            failure_classes: vec![],
            current_digest,
            last_log_digest: last_to_digest.clone(),
            log_entry_count: entries.len(),
            diagnostic: None,
        };
        emit_report(&report, json_output);
        std::process::exit(0);
    } else {
        let report = ChainCheckReport {
            schema: 1,
            check_kind: "premath.site_change_chain_check.v1".to_string(),
            result: "rejected".to_string(),
            failure_classes: vec!["site_change_chain_break".to_string()],
            current_digest: current_digest.clone(),
            last_log_digest: last_to_digest.clone(),
            log_entry_count: entries.len(),
            diagnostic: Some(format!(
                "current digest {} != last log toDigest {}",
                current_digest, last_to_digest
            )),
        };
        emit_report(&report, json_output);
        std::process::exit(1);
    }
}

fn emit_report(report: &ChainCheckReport, json_output: bool) {
    if json_output {
        let rendered =
            serde_json::to_string_pretty(report).expect("chain check report should serialize");
        println!("{rendered}");
    } else {
        println!("premath site-change-chain-check");
        println!("  Result:          {}", report.result);
        println!("  Current digest:  {}", report.current_digest);
        println!("  Last log digest: {}", report.last_log_digest);
        println!("  Log entries:     {}", report.log_entry_count);
        if !report.failure_classes.is_empty() {
            for fc in &report.failure_classes {
                eprintln!("  [{}]", fc);
            }
        }
        if let Some(ref diag) = report.diagnostic {
            println!("  {}", diag);
        }
    }
}
