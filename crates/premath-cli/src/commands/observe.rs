use crate::cli::ObserveModeArg;
use premath_ux::{SurrealObservationBackend, UxService};
use std::process;

pub struct Args {
    pub surface: String,
    pub mode: ObserveModeArg,
    pub instruction_id: Option<String>,
    pub projection_digest: Option<String>,
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
                if let Some(proj) = latest.summary.latest_projection_digest {
                    println!("  projection: {proj}");
                }
                if let Some(inst) = latest.summary.latest_instruction_id {
                    println!("  latest instruction: {inst}");
                }
                if let Some(reason) = latest.summary.top_failure_class {
                    println!("  top failure: {reason}");
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
                if let Some(reason) = needs.top_failure_class {
                    println!("  top failure: {reason}");
                }
                if let Some(proj) = needs.latest_projection_digest {
                    println!("  projection: {proj}");
                }
                if let Some(inst) = needs.latest_instruction_id {
                    println!("  latest instruction: {inst}");
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
            let view = service.projection(&projection_digest).unwrap_or_else(|| {
                eprintln!("error: projection not found in latest observation: {projection_digest}");
                process::exit(1);
            });

            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&view).expect("json serialization")
                );
            } else {
                println!("premath observe projection {projection_digest}");
                println!("  required witness: {}", yes_no(view.required.is_some()));
                println!("  delta snapshot: {}", yes_no(view.delta.is_some()));
                println!("  decision: {}", yes_no(view.decision.is_some()));
            }
        }
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
