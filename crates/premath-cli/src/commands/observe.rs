use crate::cli::{ObserveModeArg, ProjectionMatchArg};
use premath_surreal::ProjectionMatchMode;
use premath_ux::{SurrealObservationBackend, UxService};
use serde_json::Value;
use std::process;

pub struct Args {
    pub surface: String,
    pub mode: ObserveModeArg,
    pub instruction_id: Option<String>,
    pub projection_digest: Option<String>,
    pub projection_match: ProjectionMatchArg,
    pub json: bool,
}

pub fn run(args: Args) {
    let backend = SurrealObservationBackend::load_json(&args.surface).unwrap_or_else(|e| {
        eprintln!(
            "error: failed to load observation surface at {}: {e}",
            args.surface
        );
        process::exit(1);
    });
    let service = UxService::new(backend);

    match args.mode {
        ObserveModeArg::Latest => {
            let latest = service.latest();
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&latest).expect("json serialization")
                );
            } else {
                println!("premath observe latest");
                println!("  state: {}", latest.summary.state);
                println!(
                    "  needs attention: {}",
                    yes_no(latest.summary.needs_attention)
                );
                if let Some(proj) = latest.summary.latest_projection_digest.as_deref() {
                    println!("  projection: {proj}");
                }
                if let Some(inst) = latest.summary.latest_instruction_id.as_deref() {
                    println!("  latest instruction: {inst}");
                }
                if let Some(reason) = latest.summary.top_failure_class.as_deref() {
                    println!("  top failure: {reason}");
                }
                if let Some(coherence) = latest.summary.coherence.as_ref() {
                    print_coherence_summary(coherence);
                }
            }
        }
        ObserveModeArg::NeedsAttention => {
            let needs = service.needs_attention();
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&needs).expect("json serialization")
                );
            } else {
                println!("premath observe needs_attention");
                println!("  state: {}", needs.state);
                println!("  needs attention: {}", yes_no(needs.needs_attention));
                if let Some(reason) = needs.top_failure_class.as_deref() {
                    println!("  top failure: {reason}");
                }
                if let Some(proj) = needs.latest_projection_digest.as_deref() {
                    println!("  projection: {proj}");
                }
                if let Some(inst) = needs.latest_instruction_id.as_deref() {
                    println!("  latest instruction: {inst}");
                }
                if let Some(coherence) = needs.coherence.as_ref() {
                    print_coherence_summary(coherence);
                }
            }
        }
        ObserveModeArg::Instruction => {
            let instruction_id = args.instruction_id.unwrap_or_else(|| {
                eprintln!("error: --instruction-id is required for --mode instruction");
                process::exit(1);
            });
            let row = service.instruction(&instruction_id).unwrap_or_else(|| {
                eprintln!("error: instruction not found: {instruction_id}");
                process::exit(1);
            });

            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&row).expect("json serialization")
                );
            } else {
                println!("premath observe instruction {instruction_id}");
                println!(
                    "  verdict: {}",
                    row.verdict_class.as_deref().unwrap_or("(missing)")
                );
                println!(
                    "  required checks: {}",
                    if row.required_checks.is_empty() {
                        "(none)".to_string()
                    } else {
                        row.required_checks.join(", ")
                    }
                );
                println!(
                    "  executed checks: {}",
                    if row.executed_checks.is_empty() {
                        "(none)".to_string()
                    } else {
                        row.executed_checks.join(", ")
                    }
                );
            }
        }
        ObserveModeArg::Projection => {
            let projection_digest = args.projection_digest.unwrap_or_else(|| {
                eprintln!("error: --projection-digest is required for --mode projection");
                process::exit(1);
            });
            let projection_match = projection_match_from_arg(&args.projection_match);
            let view = service
                .projection(&projection_digest, projection_match)
                .unwrap_or_else(|| {
                    eprintln!(
                        "error: projection not found in latest observation: {projection_digest}"
                    );
                    process::exit(1);
                });

            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&view).expect("json serialization")
                );
            } else {
                println!("premath observe projection {projection_digest}");
                println!(
                    "  match mode: {}",
                    match projection_match {
                        ProjectionMatchMode::Typed => "typed",
                        ProjectionMatchMode::CompatibilityAlias => "compatibility_alias",
                    }
                );
                println!("  required witness: {}", yes_no(view.required.is_some()));
                println!("  delta snapshot: {}", yes_no(view.delta.is_some()));
                println!("  decision: {}", yes_no(view.decision.is_some()));
            }
        }
    }
}

fn projection_match_from_arg(arg: &ProjectionMatchArg) -> ProjectionMatchMode {
    match arg {
        ProjectionMatchArg::Typed => ProjectionMatchMode::Typed,
        ProjectionMatchArg::CompatibilityAlias => ProjectionMatchMode::CompatibilityAlias,
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn print_coherence_summary(coherence: &Value) {
    if let Some(policy_drift) = json_bool(coherence, "/policyDrift/driftDetected") {
        println!("  policy drift: {}", yes_no(policy_drift));
    }
    if let Some(unknown_rate) = json_f64(coherence, "/instructionTyping/unknownRatePercent") {
        println!("  unknown classification rate: {:.2}%", unknown_rate);
    }
    if let Some(proposal_rejects) = json_i64(coherence, "/proposalRejectClasses/totalRejectCount") {
        println!("  proposal reject classes: {proposal_rejects}");
    }
    if let Some(partition_coherent) = json_bool(coherence, "/issuePartition/isCoherent") {
        println!(
            "  ready/blocked partition coherent: {}",
            yes_no(partition_coherent)
        );
    }
    let active_cycle =
        json_bool(coherence, "/dependencyIntegrity/active/hasCycle").unwrap_or(false);
    let full_cycle = json_bool(coherence, "/dependencyIntegrity/full/hasCycle").unwrap_or(false);
    println!(
        "  dependency cycles (active/full): {}/{}",
        yes_no(active_cycle),
        yes_no(full_cycle)
    );
    let stale = json_i64(coherence, "/leaseHealth/staleCount").unwrap_or(0);
    let contended = json_i64(coherence, "/leaseHealth/contendedCount").unwrap_or(0);
    if stale > 0 || contended > 0 {
        println!("  stale/contended claims: {stale}/{contended}");
    }
    let worker_count = json_i64(coherence, "/workerLaneThroughput/workerCount").unwrap_or(0);
    let in_progress = json_i64(coherence, "/workerLaneThroughput/inProgressCount").unwrap_or(0);
    let unassigned =
        json_i64(coherence, "/workerLaneThroughput/unassignedInProgressCount").unwrap_or(0);
    if worker_count > 0 || in_progress > 0 {
        println!(
            "  worker-lane WIP (workers/in-progress/unassigned): {worker_count}/{in_progress}/{unassigned}"
        );
    }
}

fn json_bool(root: &Value, pointer: &str) -> Option<bool> {
    root.pointer(pointer).and_then(Value::as_bool)
}

fn json_f64(root: &Value, pointer: &str) -> Option<f64> {
    root.pointer(pointer).and_then(Value::as_f64)
}

fn json_i64(root: &Value, pointer: &str) -> Option<i64> {
    root.pointer(pointer).and_then(Value::as_i64)
}
