//! Premath CLI: the `premath` command.

mod cli;
mod commands;
mod support;

use clap::Parser;
use cli::{
    Cli, Commands, HarnessFeatureCommands, HarnessSessionCommands, HarnessTrajectoryCommands,
    RefCommands,
};

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

        Commands::Init { path, json } => commands::init::run(path, json),

        Commands::EvaluatorScaffold { path, json } => commands::evaluator_scaffold::run(path, json),

        Commands::Observe {
            surface,
            mode,
            instruction_id,
            projection_digest,
            projection_match,
            json,
        } => commands::observe::run(commands::observe::Args {
            surface,
            mode,
            instruction_id,
            projection_digest,
            projection_match,
            json,
        }),

        Commands::ObserveBuild {
            repo_root,
            ciwitness_dir,
            issues_path,
            out_json,
            out_jsonl,
            json,
        } => commands::observe_build::run(commands::observe_build::Args {
            repo_root,
            ciwitness_dir,
            issues_path,
            out_json,
            out_jsonl,
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
            pre_execution_failure_class,
            pre_execution_reason,
            repo_root,
            json,
        } => commands::instruction_witness::run(
            instruction,
            runtime,
            pre_execution_failure_class,
            pre_execution_reason,
            repo_root,
            json,
        ),

        Commands::RequiredWitness { runtime, json } => {
            commands::required_witness::run(runtime, json)
        }

        Commands::RequiredProjection { input, json } => {
            commands::required_projection::run(input, json)
        }

        Commands::RequiredDelta { input, json } => commands::required_delta::run(input, json),

        Commands::RequiredGateRef { input, json } => commands::required_gate_ref::run(input, json),

        Commands::RequiredWitnessVerify { input, json } => {
            commands::required_witness_verify::run(input, json)
        }

        Commands::RequiredWitnessDecide { input, json } => {
            commands::required_witness_decide::run(input, json)
        }

        Commands::RequiredDecisionVerify { input, json } => {
            commands::required_decision_verify::run(input, json)
        }

        Commands::GovernancePromotionCheck { input, json } => {
            commands::control_plane_gate::run_governance(input, json)
        }

        Commands::KcirMappingCheck { input, json } => {
            commands::control_plane_gate::run_kcir_mapping(input, json)
        }

        Commands::DoctrineInfCheck { input, json } => {
            commands::doctrine_inf_check::run(input, json)
        }

        Commands::ObligationRegistry { json } => commands::obligation_registry::run(json),

        Commands::TransportCheck { json } => commands::transport_check::run(json),

        Commands::TransportDispatch {
            action,
            payload,
            json,
        } => commands::transport_dispatch::run(action, payload, json),

        Commands::SchemeEval {
            program,
            control_plane_contract,
            trajectory_path,
            step_prefix,
            max_calls,
            issue_id,
            policy_digest,
            instruction_ref,
            capability_claims,
            json,
        } => commands::scheme_eval::run(commands::scheme_eval::Args {
            program,
            control_plane_contract,
            trajectory_path,
            step_prefix,
            max_calls,
            issue_id,
            policy_digest,
            instruction_ref,
            capability_claims,
            json,
        }),

        #[cfg(feature = "rhai-frontend")]
        Commands::RhaiEval {
            script,
            control_plane_contract,
            trajectory_path,
            step_prefix,
            max_calls,
            issue_id,
            policy_digest,
            instruction_ref,
            capability_claims,
            json,
        } => commands::rhai_eval::run(commands::rhai_eval::Args {
            script,
            control_plane_contract,
            trajectory_path,
            step_prefix,
            max_calls,
            issue_id,
            policy_digest,
            instruction_ref,
            capability_claims,
            json,
        }),

        Commands::WorldRegistryCheck {
            registry,
            site_input,
            operations,
            control_plane_contract,
            required_route_families,
            required_route_bindings,
            json,
        } => commands::world_registry_check::run(
            registry,
            site_input,
            operations,
            control_plane_contract,
            required_route_families,
            required_route_bindings,
            json,
        ),

        Commands::SiteResolve {
            request,
            doctrine_site_input,
            doctrine_site,
            doctrine_op_registry,
            control_plane_contract,
            capability_registry,
            json,
        } => commands::site_resolve::run(
            request,
            doctrine_site_input,
            doctrine_site,
            doctrine_op_registry,
            control_plane_contract,
            capability_registry,
            json,
        ),

        Commands::RuntimeOrchestrationCheck {
            control_plane_contract,
            doctrine_op_registry,
            harness_runtime,
            doctrine_site_input,
            json,
        } => commands::runtime_orchestration_check::run(
            control_plane_contract,
            doctrine_op_registry,
            harness_runtime,
            doctrine_site_input,
            json,
        ),

        Commands::WorldGateCheck {
            operations,
            check,
            profile,
            json,
        } => commands::world_gate_check::run(operations, check, profile, json),

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

        Commands::HarnessSession { command } => match command {
            HarnessSessionCommands::Read { path, json } => {
                commands::harness_session::run_read(path, json)
            }
            HarnessSessionCommands::Write {
                path,
                session_id,
                state,
                issue_id,
                summary,
                next_step,
                instruction_refs,
                witness_refs,
                lineage_refs,
                issues,
                json,
            } => commands::harness_session::run_write(commands::harness_session::WriteArgs {
                path,
                session_id,
                state,
                issue_id,
                summary,
                next_step,
                instruction_refs,
                witness_refs,
                lineage_refs,
                issues,
                json,
            }),
            HarnessSessionCommands::Bootstrap {
                path,
                feature_ledger,
                json,
            } => commands::harness_session::run_bootstrap(path, feature_ledger, json),
        },

        Commands::HarnessFeature { command } => match command {
            HarnessFeatureCommands::Read { path, json } => {
                commands::harness_feature::run_read(path, json)
            }
            HarnessFeatureCommands::Write {
                path,
                feature_id,
                status,
                issue_id,
                summary,
                session_ref,
                instruction_refs,
                verification_refs,
                json,
            } => commands::harness_feature::run_write(commands::harness_feature::WriteArgs {
                path,
                feature_id,
                status,
                issue_id,
                summary,
                session_ref,
                instruction_refs,
                verification_refs,
                json,
            }),
            HarnessFeatureCommands::Check {
                path,
                require_closure,
                json,
            } => commands::harness_feature::run_check(path, require_closure, json),
            HarnessFeatureCommands::Next { path, json } => {
                commands::harness_feature::run_next(path, json)
            }
        },

        Commands::HarnessTrajectory { command } => match command {
            HarnessTrajectoryCommands::Append {
                path,
                step_id,
                issue_id,
                action,
                result_class,
                instruction_refs,
                witness_refs,
                lineage_refs,
                started_at,
                finished_at,
                json,
            } => {
                commands::harness_trajectory::run_append(commands::harness_trajectory::AppendArgs {
                    path,
                    step_id,
                    issue_id,
                    action,
                    result_class,
                    instruction_refs,
                    witness_refs,
                    lineage_refs,
                    started_at,
                    finished_at,
                    json,
                })
            }
            HarnessTrajectoryCommands::Query {
                path,
                mode,
                limit,
                json,
            } => commands::harness_trajectory::run_query(path, mode, limit, json),
        },

        Commands::HarnessJoinCheck { input, json } => {
            commands::harness_join_check::run(input, json)
        }

        Commands::Dep { command } => commands::dep::run(command),
    }
}
