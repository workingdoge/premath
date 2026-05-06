//! Premath CLI: the `premath` command.

mod cli;
mod commands;

use clap::Parser;
use cli::{Cli, Commands, RefCommands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::CoherenceCheck {
            contract,
            repo_root,
            json,
        } => commands::coherence_check::run(contract, repo_root, json),

        Commands::TraceabilityCheck {
            draft_dir,
            matrix,
            authority_map,
            json,
        } => commands::traceability_check::run(commands::traceability_check::Args {
            draft_dir,
            matrix,
            authority_map,
            json,
        }),

        Commands::CommandSurfaceCheck { repo_root, json } => {
            commands::command_surface_check::run(repo_root, json)
        }

        Commands::RepoHygieneCheck {
            repo_root,
            paths,
            json,
        } => commands::repo_hygiene_check::run(repo_root, paths, json),

        Commands::DriftBudgetCheck {
            repo_root,
            coherence_json,
            topology_budget,
            json,
        } => commands::drift_budget_check::run(commands::drift_budget_check::Args {
            repo_root,
            coherence_json,
            topology_budget,
            json,
        }),

        Commands::ProposalCheck { proposal, json } => commands::proposal_check::run(proposal, json),

        Commands::RequiredProjection { input, json } => {
            commands::required_projection::run(input, json)
        }

        Commands::ObligationRegistry { json } => commands::obligation_registry::run(json),

        Commands::Ref { command } => match command {
            RefCommands::Project {
                profile,
                domain,
                payload_hex,
                json,
            } => commands::ref_binding::run_project(profile, domain, payload_hex, json),
            RefCommands::Verify {
                profile,
                domain,
                payload_hex,
                evidence_hex,
                ref_scheme_id,
                ref_params_hash,
                ref_domain,
                ref_digest,
                json,
            } => commands::ref_binding::run_verify(commands::ref_binding::VerifyInput {
                profile,
                domain,
                payload_hex,
                evidence_hex,
                ref_scheme_id,
                ref_params_hash,
                ref_domain,
                ref_digest,
                json_output: json,
            }),
        },

        Commands::WorkTrackerCheck { input, json } => {
            commands::work_tracker_check::run(input, json)
        }

        Commands::ToyGateCheck { input, json } => commands::toy_gate_check::run(input, json),
    }
}
