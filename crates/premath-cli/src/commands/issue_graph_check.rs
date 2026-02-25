use crate::commands::issue_graph_compactness::{
    evaluate_compactness_findings, print_compactness_findings,
};
use premath_bd::{IssueGraphCheckReport, MemoryStore};
use serde_json::json;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const CHECK_KIND: &str = "ci.issue_graph_check.v1";

fn resolve_issues_path(repo_root: &Path, issues: &str) -> PathBuf {
    let candidate = PathBuf::from(issues);
    if candidate.is_absolute() {
        candidate
    } else {
        repo_root.join(candidate)
    }
}

fn load_store_required(path: &Path) -> MemoryStore {
    if !path.exists() {
        eprintln!("error: issues file not found: {}", path.display());
        std::process::exit(1);
    }
    MemoryStore::load_jsonl(path).unwrap_or_else(|error| {
        eprintln!("error: failed to load {}: {error}", path.display());
        std::process::exit(1);
    })
}

fn print_core_report(report: &IssueGraphCheckReport) {
    println!(
        "[issue-graph] {} (issues={}, errors={}, warnings={})",
        if report.accepted() { "OK" } else { "FAIL" },
        report.summary.issue_count,
        report.summary.error_count,
        report.summary.warning_count
    );
    for finding in &report.errors {
        println!(
            "  - {} {} ({})",
            finding.issue_id, finding.class, finding.message
        );
    }
    for finding in &report.warnings {
        println!(
            "  - WARN {} {} ({})",
            finding.issue_id, finding.class, finding.message
        );
    }
}

pub fn run(repo_root: String, issues: String, note_warn_threshold: usize, json_output: bool) {
    let repo_root = PathBuf::from(repo_root);
    let issues_path = resolve_issues_path(&repo_root, &issues);
    let store = load_store_required(&issues_path);
    let core_report = store.check_issue_graph(note_warn_threshold);

    if !core_report.accepted() {
        if json_output {
            let payload = json!({
                "schema": 1,
                "checkKind": CHECK_KIND,
                "issuesPath": issues_path.display().to_string(),
                "result": "rejected",
                "failureClasses": core_report.failure_classes,
                "warningClasses": core_report.warning_classes,
                "core": {
                    "checkKind": core_report.check_kind,
                    "result": core_report.result,
                    "failureClasses": core_report.failure_classes,
                    "warningClasses": core_report.warning_classes,
                    "errors": core_report.errors,
                    "warnings": core_report.warnings,
                    "summary": core_report.summary
                },
                "compactness": {
                    "evaluated": false,
                    "findingCount": 0,
                    "findings": []
                }
            });
            let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|error| {
                eprintln!("error: failed to render issue-graph-check payload: {error}");
                std::process::exit(2);
            });
            println!("{rendered}");
        } else {
            print_core_report(&core_report);
        }
        std::process::exit(1);
    }

    let findings = evaluate_compactness_findings(&store);
    let result = if findings.is_empty() {
        "accepted"
    } else {
        "rejected"
    };
    let mut failure_classes: BTreeSet<String> =
        core_report.failure_classes.iter().cloned().collect();
    failure_classes.extend(findings.iter().map(|finding| finding.class_name.clone()));

    if json_output {
        let payload = json!({
            "schema": 1,
            "checkKind": CHECK_KIND,
            "issuesPath": issues_path.display().to_string(),
            "result": result,
            "failureClasses": failure_classes.into_iter().collect::<Vec<_>>(),
            "warningClasses": core_report.warning_classes,
            "core": {
                "checkKind": core_report.check_kind,
                "result": core_report.result,
                "failureClasses": core_report.failure_classes,
                "warningClasses": core_report.warning_classes,
                "errors": core_report.errors,
                "warnings": core_report.warnings,
                "summary": core_report.summary
            },
            "compactness": {
                "evaluated": true,
                "findingCount": findings.len(),
                "findings": findings
            }
        });
        let rendered = serde_json::to_string_pretty(&payload).unwrap_or_else(|error| {
            eprintln!("error: failed to render issue-graph-check payload: {error}");
            std::process::exit(2);
        });
        println!("{rendered}");
    } else {
        print_core_report(&core_report);
        if !findings.is_empty() {
            print_compactness_findings(&findings, Some(&repo_root), Some(&issues_path));
        }
    }

    if !findings.is_empty() {
        std::process::exit(1);
    }
}
