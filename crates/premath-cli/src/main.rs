//! Premath CLI: the `premath` command.

mod cli;
mod commands;
mod support;

use clap::Parser;
use cli::{Cli, Commands, RefCommands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check {
            id,
            level,
            issues,
            repo,
            json,
        } => commands::check::run(id, level, issues, repo, json),

        Commands::Verify {
            id,
            level,
            issues,
            repo,
            json,
        } => commands::verify::run(id, level, issues, repo, json),

        Commands::MockGate {
            world_id,
            unit_id,
            parent_unit_id,
            context_id,
            cover_id,
            ctx_ref,
            data_head_ref,
            adapter_id,
            adapter_version,
            normalizer_id,
            policy_digest,
            cover_strategy_digest,
            intent_kind,
            target_scope,
            outcomes,
            failures,
            include_cover_strategy_in_run_id,
            json,
        } => commands::mock_gate::run(commands::mock_gate::Args {
            world_id,
            unit_id,
            parent_unit_id,
            context_id,
            cover_id,
            ctx_ref,
            data_head_ref,
            adapter_id,
            adapter_version,
            normalizer_id,
            policy_digest,
            cover_strategy_digest,
            intent_kind,
            target_scope,
            outcomes,
            failures,
            include_cover_strategy_in_run_id,
            json,
        }),

        Commands::TuskEval {
            identity,
            descent_pack,
            include_cover_strategy_in_run_id,
            json,
        } => commands::tusk_eval::run(
            identity,
            descent_pack,
            include_cover_strategy_in_run_id,
            json,
        ),

        Commands::Init { path } => commands::init::run(path),

        Commands::Observe {
            surface,
            mode,
            instruction_id,
            projection_digest,
            json,
        } => commands::observe::run(commands::observe::Args {
            surface,
            mode,
            instruction_id,
            projection_digest,
            json,
        }),

        Commands::ObserveServe { surface, bind } => commands::observe_serve::run(surface, bind),

        Commands::McpServe {
            issues,
            issue_query_backend,
            issue_query_projection,
            mutation_policy,
            surface,
            repo_root,
            server_name,
            server_version,
        } => commands::mcp_serve::run(commands::mcp_serve::Args {
            issues,
            issue_query_backend,
            issue_query_projection,
            mutation_policy,
            surface,
            repo_root,
            server_name,
            server_version,
        }),

        Commands::CoherenceCheck {
            contract,
            repo_root,
            json,
        } => commands::coherence_check::run(contract, repo_root, json),

        Commands::ProposalCheck { proposal, json } => commands::proposal_check::run(proposal, json),

        Commands::InstructionCheck {
            instruction,
            repo_root,
            json,
        } => commands::instruction_check::run(instruction, repo_root, json),

        Commands::InstructionWitness {
            instruction,
            runtime,
            repo_root,
            json,
        } => commands::instruction_witness::run(instruction, runtime, repo_root, json),

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

        Commands::Issue { command } => commands::issue::run(command),

        Commands::Dep { command } => commands::dep::run(command),
    }
}
