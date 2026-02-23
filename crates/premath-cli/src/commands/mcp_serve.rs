use super::init::{InitOutcome, init_layout};
use crate::support::{
    ISSUE_QUERY_PROJECTION_KIND, ISSUE_QUERY_PROJECTION_SCHEMA, IssueQueryProjectionPayload,
    analyze_issue_query_projection, backend_status_payload, collect_backend_status,
    paths_equivalent,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use premath_bd::DEFAULT_NOTE_WARN_THRESHOLD;
use premath_bd::issue::{issue_type_variants, parse_issue_type};
use premath_bd::{
    AtomicStoreMutationError, DepType, DependencyGraphScope, Issue, IssueLease, IssueLeaseState,
    MemoryStore, mutate_store_jsonl, store_snapshot_ref,
};
use premath_coherence::validate_instruction_envelope_payload;
use premath_jj::JjClient;
use premath_surreal::{ProjectionMatchMode, QueryCache};
use premath_ux::{ObserveQuery, SurrealObservationBackend, UxService};
use rust_mcp_sdk::{
    McpServer, StdioTransport, ToMcpServerHandler, TransportOptions,
    macros::{JsonSchema, mcp_tool},
    mcp_server::{McpServerOptions, ServerHandler, ServerRuntime, server_runtime},
    schema::{
        CallToolRequestParams, CallToolResult, Implementation, InitializeResult, ListToolsResult,
        PaginatedRequestParams, ProtocolVersion, RpcError, ServerCapabilities,
        ServerCapabilitiesTools, TextContent, schema_utils::CallToolError,
    },
    tool_box,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Args {
    pub issues: String,
    pub issue_query_backend: String,
    pub issue_query_projection: String,
    pub mutation_policy: String,
    pub surface: String,
    pub repo_root: String,
    pub server_name: String,
    pub server_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IssueQueryBackend {
    Jsonl,
    Surreal,
}

impl IssueQueryBackend {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim() {
            "jsonl" => Ok(Self::Jsonl),
            "surreal" => Ok(Self::Surreal),
            other => Err(format!(
                "invalid issue_query_backend `{other}` (expected `jsonl` or `surreal`)"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Jsonl => "jsonl",
            Self::Surreal => "surreal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MutationPolicy {
    Open,
    InstructionLinked,
}

impl MutationPolicy {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim() {
            "open" => Ok(Self::Open),
            "instruction-linked" => Ok(Self::InstructionLinked),
            other => Err(format!(
                "invalid mutation_policy `{other}` (expected `open` or `instruction-linked`)"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InstructionLinked => "instruction-linked",
        }
    }
}

#[derive(Debug, Clone)]
struct PremathMcpConfig {
    issues_path: String,
    issue_query_backend: IssueQueryBackend,
    issue_query_projection: String,
    mutation_policy: MutationPolicy,
    surface_path: String,
    repo_root: String,
}

#[derive(Debug, Clone)]
struct PremathMcpHandler {
    config: PremathMcpConfig,
}

pub fn run(args: Args) {
    eprintln!("premath mcp-serve");
    eprintln!("  transport: stdio");
    eprintln!("  server: {} {}", args.server_name, args.server_version);
    eprintln!("  default issues path: {}", args.issues);
    eprintln!("  issue query backend: {}", args.issue_query_backend);
    eprintln!("  issue query projection: {}", args.issue_query_projection);
    eprintln!("  mutation policy: {}", args.mutation_policy);
    eprintln!("  default surface path: {}", args.surface);
    eprintln!("  repo root: {}", args.repo_root);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| {
            eprintln!("error: failed to create tokio runtime: {e}");
            process::exit(1);
        });

    runtime.block_on(async move {
        if let Err(e) = run_async(args).await {
            eprintln!("error: mcp server failed: {e}");
            process::exit(1);
        }
    });
}

async fn run_async(args: Args) -> Result<(), String> {
    let issue_query_backend = IssueQueryBackend::parse(&args.issue_query_backend)?;
    let mutation_policy = MutationPolicy::parse(&args.mutation_policy)?;

    let config = PremathMcpConfig {
        issues_path: args.issues,
        issue_query_backend,
        issue_query_projection: args.issue_query_projection,
        mutation_policy,
        surface_path: args.surface,
        repo_root: args.repo_root,
    };

    let server_details = InitializeResult {
        server_info: Implementation {
            name: args.server_name,
            version: args.server_version,
            title: Some("Premath MCP Server".into()),
            description: Some("MCP tool surface for premath JSONL issue memory and observations".into()),
            icons: vec![],
            website_url: Some("https://github.com/premath/premath".into()),
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        protocol_version: ProtocolVersion::V2025_11_25.into(),
        instructions: Some(
            "Use init_tool to initialize canonical .premath/issues.jsonl memory. issue/dep mutation tools (including issue_claim, issue_lease_renew, issue_lease_release, and issue_discover) are instruction-mediated by policy (default: instruction-linked) and reads may use jsonl or surreal projection backends. Use issue_backend_status for backend integration state, issue_lease_projection for deterministic stale/contended lease views, and dep_diagnostics for scoped dependency-cycle diagnostics. Use instruction_* tools for doctrine-gated runs (instruction envelopes require normalizerId + policyDigest, requestedChecks must be policy-allowlisted, optional capabilityClaims are carried to witness for mutation gating, and optional proposal ingestion uses proposal/llmProposal) and observe_* tools for observation queries."
                .into(),
        ),
        meta: None,
    };

    let transport = StdioTransport::new(TransportOptions::default()).map_err(|e| e.to_string())?;
    let handler = PremathMcpHandler { config };

    let server: Arc<ServerRuntime> = server_runtime::create_server(McpServerOptions {
        server_details,
        transport,
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
    });

    server.start().await.map_err(|e| {
        e.rpc_error_message()
            .cloned()
            .unwrap_or_else(|| e.to_string())
    })
}

#[async_trait]
impl ServerHandler for PremathMcpHandler {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools: PremathTools::tools(),
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool_params: PremathTools =
            PremathTools::try_from(params).map_err(CallToolError::new)?;

        match tool_params {
            PremathTools::IssueReadyTool(tool) => call_issue_ready(&self.config, tool),
            PremathTools::IssueListTool(tool) => call_issue_list(&self.config, tool),
            PremathTools::IssueCheckTool(tool) => call_issue_check(&self.config, tool),
            PremathTools::IssueBackendStatusTool(tool) => {
                call_issue_backend_status(&self.config, tool)
            }
            PremathTools::IssueBlockedTool(tool) => call_issue_blocked(&self.config, tool),
            PremathTools::IssueAddTool(tool) => call_issue_add(&self.config, tool),
            PremathTools::IssueClaimTool(tool) => call_issue_claim(&self.config, tool),
            PremathTools::IssueLeaseRenewTool(tool) => call_issue_lease_renew(&self.config, tool),
            PremathTools::IssueLeaseReleaseTool(tool) => {
                call_issue_lease_release(&self.config, tool)
            }
            PremathTools::IssueDiscoverTool(tool) => call_issue_discover(&self.config, tool),
            PremathTools::IssueUpdateTool(tool) => call_issue_update(&self.config, tool),
            PremathTools::DepAddTool(tool) => call_dep_add(&self.config, tool),
            PremathTools::DepRemoveTool(tool) => call_dep_remove(&self.config, tool),
            PremathTools::DepReplaceTool(tool) => call_dep_replace(&self.config, tool),
            PremathTools::DepDiagnosticsTool(tool) => call_dep_diagnostics(&self.config, tool),
            PremathTools::InitTool(tool) => call_init_tool(&self.config, tool),
            PremathTools::IssueLeaseProjectionTool(tool) => {
                call_issue_lease_projection(&self.config, tool)
            }
            PremathTools::ObserveLatestTool(tool) => call_observe_latest(&self.config, tool),
            PremathTools::ObserveNeedsAttentionTool(tool) => {
                call_observe_needs_attention(&self.config, tool)
            }
            PremathTools::ObserveInstructionTool(tool) => {
                call_observe_instruction(&self.config, tool)
            }
            PremathTools::ObserveProjectionTool(tool) => {
                call_observe_projection(&self.config, tool)
            }
            PremathTools::InstructionCheckTool(tool) => call_instruction_check(&self.config, tool),
            PremathTools::InstructionRunTool(tool) => call_instruction_run(&self.config, tool),
        }
    }
}

#[mcp_tool(
    name = "issue_ready",
    description = "List open issues that have no unresolved blocking dependencies",
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
struct IssueReadyTool {
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "issue_list",
    description = "List issues with optional status/assignee filtering",
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
struct IssueListTool {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "issue_check",
    description = "Run deterministic issue-graph contract checks",
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
struct IssueCheckTool {
    #[serde(default)]
    issues_path: Option<String>,
    #[serde(default)]
    note_warn_threshold: Option<u64>,
}

#[mcp_tool(
    name = "issue_backend_status",
    description = "Report issue backend integration state (jsonl authority, surreal projection, jj)",
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
struct IssueBackendStatusTool {
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "issue_blocked",
    description = "List non-closed issues that currently have unresolved blocking dependencies",
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
struct IssueBlockedTool {
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "issue_add",
    description = "Add an issue to the JSONL substrate",
    read_only_hint = false,
    idempotent_hint = false
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct IssueAddTool {
    title: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    priority: Option<i32>,
    #[serde(default)]
    issue_type: Option<String>,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    instruction_id: Option<String>,
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "issue_claim",
    description = "Atomically claim an issue for work (sets assignee + in_progress)",
    read_only_hint = false,
    idempotent_hint = false
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct IssueClaimTool {
    id: String,
    assignee: String,
    #[serde(default)]
    lease_id: Option<String>,
    #[serde(default)]
    lease_ttl_seconds: Option<i64>,
    #[serde(default)]
    lease_expires_at: Option<String>,
    #[serde(default)]
    instruction_id: Option<String>,
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "issue_lease_renew",
    description = "Renew an active issue lease by id/assignee/lease_id",
    read_only_hint = false,
    idempotent_hint = false
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct IssueLeaseRenewTool {
    id: String,
    assignee: String,
    lease_id: String,
    #[serde(default)]
    lease_ttl_seconds: Option<i64>,
    #[serde(default)]
    lease_expires_at: Option<String>,
    #[serde(default)]
    instruction_id: Option<String>,
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "issue_lease_release",
    description = "Release an issue lease by id (optionally validating assignee/lease_id)",
    read_only_hint = false,
    idempotent_hint = false
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct IssueLeaseReleaseTool {
    id: String,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    lease_id: Option<String>,
    #[serde(default)]
    instruction_id: Option<String>,
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "issue_discover",
    description = "Create a follow-up issue and link it with discovered-from dependency",
    read_only_hint = false,
    idempotent_hint = false
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct IssueDiscoverTool {
    parent_issue_id: String,
    title: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    priority: Option<i32>,
    #[serde(default)]
    issue_type: Option<String>,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    instruction_id: Option<String>,
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "issue_update",
    description = "Update mutable issue fields by id",
    read_only_hint = false,
    idempotent_hint = false
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct IssueUpdateTool {
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    priority: Option<i32>,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    instruction_id: Option<String>,
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "dep_add",
    description = "Add a dependency edge between issues",
    read_only_hint = false,
    idempotent_hint = false
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DepAddTool {
    issue_id: String,
    depends_on_id: String,
    #[serde(default)]
    dep_type: Option<String>,
    #[serde(default)]
    created_by: Option<String>,
    #[serde(default)]
    instruction_id: Option<String>,
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "dep_remove",
    description = "Remove a dependency edge between issues",
    read_only_hint = false,
    idempotent_hint = false
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DepRemoveTool {
    issue_id: String,
    depends_on_id: String,
    #[serde(default)]
    dep_type: Option<String>,
    #[serde(default)]
    instruction_id: Option<String>,
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "dep_replace",
    description = "Replace one dependency edge type with another",
    read_only_hint = false,
    idempotent_hint = false
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DepReplaceTool {
    issue_id: String,
    depends_on_id: String,
    #[serde(default)]
    from_dep_type: Option<String>,
    to_dep_type: String,
    #[serde(default)]
    created_by: Option<String>,
    #[serde(default)]
    instruction_id: Option<String>,
    #[serde(default)]
    issues_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
enum DepGraphScopeToolArg {
    #[default]
    Active,
    Full,
}

impl From<DepGraphScopeToolArg> for DependencyGraphScope {
    fn from(value: DepGraphScopeToolArg) -> Self {
        match value {
            DepGraphScopeToolArg::Active => DependencyGraphScope::Active,
            DepGraphScopeToolArg::Full => DependencyGraphScope::Full,
        }
    }
}

#[mcp_tool(
    name = "dep_diagnostics",
    description = "Report dependency graph cycle diagnostics for active or full graph scope",
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
struct DepDiagnosticsTool {
    #[serde(default)]
    issues_path: Option<String>,
    #[serde(default)]
    graph_scope: DepGraphScopeToolArg,
}

#[mcp_tool(
    name = "init_tool",
    description = "Initialize premath local substrate (.premath/issues.jsonl), migrating legacy .beads store when present",
    read_only_hint = false,
    idempotent_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
struct InitTool {
    #[serde(default)]
    root_path: Option<String>,
}

#[mcp_tool(
    name = "observe_latest",
    description = "Return the latest projected observation view",
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
struct ObserveLatestTool {
    #[serde(default)]
    surface_path: Option<String>,
}

#[mcp_tool(
    name = "observe_needs_attention",
    description = "Return observation attention summary",
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
struct ObserveNeedsAttentionTool {
    #[serde(default)]
    surface_path: Option<String>,
}

#[mcp_tool(
    name = "observe_instruction",
    description = "Return one instruction row from observation surface",
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ObserveInstructionTool {
    instruction_id: String,
    #[serde(default)]
    surface_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
enum ObserveProjectionMatchArg {
    #[default]
    Typed,
    CompatibilityAlias,
}

impl From<ObserveProjectionMatchArg> for ProjectionMatchMode {
    fn from(value: ObserveProjectionMatchArg) -> Self {
        match value {
            ObserveProjectionMatchArg::Typed => ProjectionMatchMode::Typed,
            ObserveProjectionMatchArg::CompatibilityAlias => {
                ProjectionMatchMode::CompatibilityAlias
            }
        }
    }
}

#[mcp_tool(
    name = "observe_projection",
    description = "Return one projection view from observation surface",
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ObserveProjectionTool {
    projection_digest: String,
    #[serde(default)]
    projection_match: ObserveProjectionMatchArg,
    #[serde(default)]
    surface_path: Option<String>,
}

#[mcp_tool(
    name = "issue_lease_projection",
    description = "Return deterministic lease projection for stale/contended claims",
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
struct IssueLeaseProjectionTool {
    #[serde(default)]
    issues_path: Option<String>,
}

#[mcp_tool(
    name = "instruction_check",
    description = "Validate one instruction envelope against doctrine shape checks",
    read_only_hint = true
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct InstructionCheckTool {
    instruction_path: String,
}

#[mcp_tool(
    name = "instruction_run",
    description = "Run instruction pipeline and emit CI instruction witness",
    read_only_hint = false,
    idempotent_hint = false
)]
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct InstructionRunTool {
    instruction_path: String,
    #[serde(default)]
    allow_failure: Option<bool>,
}

tool_box!(
    PremathTools,
    [
        IssueReadyTool,
        IssueListTool,
        IssueCheckTool,
        IssueBackendStatusTool,
        IssueBlockedTool,
        IssueAddTool,
        IssueClaimTool,
        IssueLeaseRenewTool,
        IssueLeaseReleaseTool,
        IssueDiscoverTool,
        IssueUpdateTool,
        DepAddTool,
        DepRemoveTool,
        DepReplaceTool,
        DepDiagnosticsTool,
        InitTool,
        IssueLeaseProjectionTool,
        ObserveLatestTool,
        ObserveNeedsAttentionTool,
        ObserveInstructionTool,
        ObserveProjectionTool,
        InstructionCheckTool,
        InstructionRunTool
    ]
);

#[derive(Debug, Clone)]
struct InstructionWitnessLink {
    instruction_id: String,
    witness_path: String,
    instruction_digest: Option<String>,
    policy_digest: Option<String>,
    capability_claims: Vec<String>,
    required_checks: Vec<String>,
    executed_checks: Vec<String>,
}

impl InstructionWitnessLink {
    fn to_json(&self) -> Value {
        json!({
            "instructionId": self.instruction_id,
            "witnessPath": self.witness_path,
            "instructionDigest": self.instruction_digest,
            "policyDigest": self.policy_digest,
            "capabilityClaims": self.capability_claims,
            "requiredChecks": self.required_checks,
            "executedChecks": self.executed_checks,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum MutationAction {
    IssueAdd,
    IssueClaim,
    IssueLeaseRenew,
    IssueLeaseRelease,
    IssueDiscover,
    IssueUpdate,
    DepAdd,
    DepRemove,
    DepReplace,
}

impl MutationAction {
    fn action_id(self) -> &'static str {
        match self {
            Self::IssueAdd => "issue.add",
            Self::IssueClaim => "issue.claim",
            Self::IssueLeaseRenew => "issue.lease_renew",
            Self::IssueLeaseRelease => "issue.lease_release",
            Self::IssueDiscover => "issue.discover",
            Self::IssueUpdate => "issue.update",
            Self::DepAdd => "dep.add",
            Self::DepRemove => "dep.remove",
            Self::DepReplace => "dep.replace",
        }
    }

    fn required_capability_claim(self) -> &'static str {
        match self {
            Self::IssueAdd => "capabilities.change_morphisms.issue_add",
            Self::IssueClaim => "capabilities.change_morphisms.issue_claim",
            Self::IssueLeaseRenew => "capabilities.change_morphisms.issue_lease_renew",
            Self::IssueLeaseRelease => "capabilities.change_morphisms.issue_lease_release",
            Self::IssueDiscover => "capabilities.change_morphisms.issue_discover",
            Self::IssueUpdate => "capabilities.change_morphisms.issue_update",
            Self::DepAdd => "capabilities.change_morphisms.dep_add",
            Self::DepRemove => "capabilities.change_morphisms.dep_remove",
            Self::DepReplace => "capabilities.change_morphisms.dep_replace",
        }
    }
}

const CHANGE_MORPHISMS_BASE_CAPABILITY: &str = "capabilities.change_morphisms";
const CHANGE_MORPHISMS_ALL_CAPABILITY: &str = "capabilities.change_morphisms.all";
const POLICY_DIGEST_CI_V1: &str =
    "pol1_4ba916ce38da5c5607eb7f41d963294b34b644deb1fa6d55e133b072ca001b39";
const POLICY_DIGEST_TEST_V1: &str =
    "pol1_1ab3e7f398a472c2cf0f3fbd7ead7ece7bd74e836cbde924f1e33f02895d18ab";
const DEFAULT_LEASE_TTL_SECONDS: i64 = 3600;
const MIN_LEASE_TTL_SECONDS: i64 = 30;
const MAX_LEASE_TTL_SECONDS: i64 = 86_400;

const FAILURE_LEASE_INVALID_ASSIGNEE: &str = "lease_invalid_assignee";
const FAILURE_LEASE_INVALID_TTL: &str = "lease_invalid_ttl";
const FAILURE_LEASE_BINDING_AMBIGUOUS: &str = "lease_binding_ambiguous";
const FAILURE_LEASE_INVALID_EXPIRES_AT: &str = "lease_invalid_expires_at";
const FAILURE_LEASE_NOT_FOUND: &str = "lease_not_found";
const FAILURE_LEASE_CLOSED: &str = "lease_issue_closed";
const FAILURE_LEASE_CONTENTION_ACTIVE: &str = "lease_contention_active";
const FAILURE_LEASE_MISSING: &str = "lease_missing";
const FAILURE_LEASE_STALE: &str = "lease_stale";
const FAILURE_LEASE_OWNER_MISMATCH: &str = "lease_owner_mismatch";
const FAILURE_LEASE_ID_MISMATCH: &str = "lease_id_mismatch";
const FAILURE_LEASE_MUTATION_LOCK_BUSY: &str = "lease_mutation_lock_busy";
const FAILURE_LEASE_MUTATION_LOCK_IO: &str = "lease_mutation_lock_io";
const FAILURE_LEASE_MUTATION_STORE_IO: &str = "lease_mutation_store_io";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LeaseProjection {
    checked_at: String,
    stale_count: usize,
    stale_issue_ids: Vec<String>,
    contended_count: usize,
    contended_issue_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct IssueGraphView {
    store: MemoryStore,
    cache: QueryCache,
    query_source: &'static str,
}

struct ProjectionLoad {
    store: MemoryStore,
    source_path_matches_authority: bool,
    source_snapshot_ref_present: bool,
}

fn lease_state_label(issue: &Issue, now: DateTime<Utc>) -> &'static str {
    match issue.lease_state_at(now) {
        IssueLeaseState::Unleased => "unleased",
        IssueLeaseState::Active => "active",
        IssueLeaseState::Stale => "stale",
    }
}

fn issue_is_lease_contended(issue: &Issue, now: DateTime<Utc>) -> bool {
    let lease = match issue.lease.as_ref() {
        Some(lease) => lease,
        None => return false,
    };
    if lease.expires_at <= now {
        return false;
    }
    issue.status != "in_progress" || issue.assignee != lease.owner
}

fn lease_json(issue: &Issue, now: DateTime<Utc>) -> Value {
    let Some(lease) = issue.lease.as_ref() else {
        return Value::Null;
    };
    json!({
        "leaseId": lease.lease_id,
        "owner": lease.owner,
        "acquiredAt": lease.acquired_at.to_rfc3339(),
        "expiresAt": lease.expires_at.to_rfc3339(),
        "renewedAt": lease.renewed_at.map(|item| item.to_rfc3339()),
        "state": lease_state_label(issue, now)
    })
}

fn issue_summary_json(issue: &Issue, now: DateTime<Utc>) -> Value {
    json!({
        "id": issue.id,
        "title": issue.title,
        "status": issue.status,
        "priority": issue.priority,
        "issueType": issue.issue_type,
        "assignee": issue.assignee,
        "owner": issue.owner,
        "lease": lease_json(issue, now)
    })
}

fn compute_lease_projection(store: &MemoryStore, now: DateTime<Utc>) -> LeaseProjection {
    let mut stale_issue_ids = Vec::new();
    let mut contended_issue_ids = Vec::new();

    for issue in store.issues() {
        match issue.lease_state_at(now) {
            IssueLeaseState::Stale => stale_issue_ids.push(issue.id.clone()),
            IssueLeaseState::Active if issue_is_lease_contended(issue, now) => {
                contended_issue_ids.push(issue.id.clone())
            }
            IssueLeaseState::Unleased | IssueLeaseState::Active => {}
        }
    }

    stale_issue_ids.sort();
    contended_issue_ids.sort();

    LeaseProjection {
        checked_at: now.to_rfc3339(),
        stale_count: stale_issue_ids.len(),
        stale_issue_ids,
        contended_count: contended_issue_ids.len(),
        contended_issue_ids,
    }
}

fn lease_error(failure_class: &str, detail: impl Into<String>) -> CallToolError {
    call_tool_error(format!("[failureClass={failure_class}] {}", detail.into()))
}

fn map_atomic_store_mutation_error(err: AtomicStoreMutationError<CallToolError>) -> CallToolError {
    match err {
        AtomicStoreMutationError::Mutation(inner) => inner,
        AtomicStoreMutationError::LockBusy { lock_path } => lease_error(
            FAILURE_LEASE_MUTATION_LOCK_BUSY,
            format!("issue-memory lock busy: {lock_path}"),
        ),
        AtomicStoreMutationError::LockIo { lock_path, message } => lease_error(
            FAILURE_LEASE_MUTATION_LOCK_IO,
            format!("failed to acquire issue-memory lock {lock_path}: {message}"),
        ),
        AtomicStoreMutationError::Store(source) => {
            lease_error(FAILURE_LEASE_MUTATION_STORE_IO, source.to_string())
        }
    }
}

fn parse_lease_ttl_seconds(ttl_seconds: Option<i64>) -> std::result::Result<i64, CallToolError> {
    let ttl = ttl_seconds.unwrap_or(DEFAULT_LEASE_TTL_SECONDS);
    if !(MIN_LEASE_TTL_SECONDS..=MAX_LEASE_TTL_SECONDS).contains(&ttl) {
        return Err(lease_error(
            FAILURE_LEASE_INVALID_TTL,
            format!(
                "lease_ttl_seconds must be in range [{MIN_LEASE_TTL_SECONDS}, {MAX_LEASE_TTL_SECONDS}]"
            ),
        ));
    }
    Ok(ttl)
}

fn parse_lease_expiry(
    lease_ttl_seconds: Option<i64>,
    lease_expires_at: Option<String>,
    now: DateTime<Utc>,
) -> std::result::Result<DateTime<Utc>, CallToolError> {
    let expires_at_raw = non_empty(lease_expires_at);
    if lease_ttl_seconds.is_some() && expires_at_raw.is_some() {
        return Err(lease_error(
            FAILURE_LEASE_BINDING_AMBIGUOUS,
            "provide only one of leaseTtlSeconds or leaseExpiresAt",
        ));
    }

    if let Some(raw) = expires_at_raw {
        let parsed = DateTime::parse_from_rfc3339(&raw)
            .map_err(|_| {
                lease_error(
                    FAILURE_LEASE_INVALID_EXPIRES_AT,
                    "lease_expires_at must be RFC3339",
                )
            })?
            .with_timezone(&Utc);
        if parsed <= now {
            return Err(lease_error(
                FAILURE_LEASE_INVALID_EXPIRES_AT,
                "lease_expires_at must be in the future",
            ));
        }
        return Ok(parsed);
    }

    let ttl = parse_lease_ttl_seconds(lease_ttl_seconds)?;
    now.checked_add_signed(Duration::seconds(ttl))
        .ok_or_else(|| {
            lease_error(
                FAILURE_LEASE_INVALID_TTL,
                "lease_ttl_seconds overflowed timestamp range",
            )
        })
}

fn lease_token(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "anon".to_string()
    } else {
        trimmed.to_string()
    }
}

fn resolve_lease_id(raw_lease_id: Option<String>, issue_id: &str, assignee: &str) -> String {
    non_empty(raw_lease_id)
        .unwrap_or_else(|| format!("lease1_{}_{}", lease_token(issue_id), lease_token(assignee)))
}

fn call_issue_ready(
    config: &PremathMcpConfig,
    tool: IssueReadyTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let graph = load_issue_graph(config, &path)?;
    let now = Utc::now();
    let ids = graph.cache.ready_open_issue_ids();

    let items = ids
        .iter()
        .filter_map(|id| graph.cache.issue(id))
        .map(|issue| issue_summary_json(issue, now))
        .collect::<Vec<_>>();
    let lease_projection = compute_lease_projection(&graph.store, now);

    json_result(json!({
        "action": "issue.ready",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "querySource": graph.query_source,
        "count": items.len(),
        "items": items,
        "leaseProjection": lease_projection
    }))
}

fn call_issue_list(
    config: &PremathMcpConfig,
    tool: IssueListTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let graph = load_issue_graph(config, &path)?;
    let status = non_empty(tool.status);
    let assignee = non_empty(tool.assignee);
    let now = Utc::now();

    let items = graph
        .store
        .issues()
        .filter(|issue| status.as_ref().is_none_or(|s| issue.status == *s))
        .filter(|issue| assignee.as_ref().is_none_or(|a| issue.assignee == *a))
        .map(|issue| issue_summary_json(issue, now))
        .collect::<Vec<_>>();

    json_result(json!({
        "action": "issue.list",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "querySource": graph.query_source,
        "count": items.len(),
        "items": items
    }))
}

fn call_issue_check(
    config: &PremathMcpConfig,
    tool: IssueCheckTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let store = load_store_existing(&path)?;
    let note_warn_threshold = tool
        .note_warn_threshold
        .map(|value| {
            usize::try_from(value).map_err(|_| {
                call_tool_error(format!(
                    "note_warn_threshold exceeds platform usize: {}",
                    value
                ))
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_NOTE_WARN_THRESHOLD);
    let report = store.check_issue_graph(note_warn_threshold);

    json_result(json!({
        "action": "issue.check",
        "issuesPath": path.display().to_string(),
        "checkKind": report.check_kind,
        "result": report.result,
        "failureClasses": report.failure_classes,
        "warningClasses": report.warning_classes,
        "errors": report.errors,
        "warnings": report.warnings,
        "summary": report.summary
    }))
}

fn call_issue_backend_status(
    config: &PremathMcpConfig,
    tool: IssueBackendStatusTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let issues_path = resolve_path(tool.issues_path, &config.issues_path);
    let repo_root = resolve_repo_root(&config.repo_root);
    let projection_path = resolve_issue_query_projection_path(config);
    let status = collect_backend_status(&issues_path, &repo_root, &projection_path);
    let payload = backend_status_payload(
        "issue.backend-status",
        &status,
        Some(config.issue_query_backend.as_str()),
    );
    json_result(payload)
}

fn call_issue_blocked(
    config: &PremathMcpConfig,
    tool: IssueBlockedTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let graph = load_issue_graph(config, &path)?;

    let items = graph
        .store
        .issues()
        .filter(|issue| issue.status != "closed")
        .filter_map(|issue| {
            let blockers = graph
                .store
                .blocking_dependencies_of(&issue.id)
                .into_iter()
                .filter_map(|dep| {
                    let blocker = graph.cache.issue(&dep.depends_on_id);
                    let unresolved = blocker.is_none_or(|b| b.status != "closed");
                    if !unresolved {
                        return None;
                    }
                    Some(json!({
                        "issueId": dep.issue_id,
                        "dependsOnId": dep.depends_on_id,
                        "type": dep.dep_type.as_str(),
                        "createdBy": dep.created_by,
                        "blockerStatus": blocker.map(|b| b.status.clone()),
                        "blockerMissing": blocker.is_none()
                    }))
                })
                .collect::<Vec<_>>();

            if blockers.is_empty() {
                return None;
            }

            Some(json!({
                "id": issue.id,
                "title": issue.title,
                "status": issue.status,
                "priority": issue.priority,
                "blockers": blockers
            }))
        })
        .collect::<Vec<_>>();

    json_result(json!({
        "action": "issue.blocked",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "querySource": graph.query_source,
        "count": items.len(),
        "items": items
    }))
}

fn call_issue_add(
    config: &PremathMcpConfig,
    tool: IssueAddTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let mut store = load_store_or_empty(&path)?;
    let issue_id = non_empty(tool.id).unwrap_or_else(|| next_issue_id(&store));
    let instruction =
        resolve_instruction_link(config, tool.instruction_id, MutationAction::IssueAdd)?;

    if store.issue(&issue_id).is_some() {
        return Err(call_tool_error(format!("issue already exists: {issue_id}")));
    }

    let mut issue = Issue::new(issue_id.clone(), tool.title);
    issue.description = tool.description.unwrap_or_default();
    issue.priority = tool.priority.unwrap_or(2);
    let raw_issue_type = non_empty(tool.issue_type).unwrap_or_else(|| "task".to_string());
    issue.issue_type = parse_issue_type(&raw_issue_type).ok_or_else(|| {
        call_tool_error(format!(
            "invalid issue_type `{}` (expected one of: {})",
            raw_issue_type,
            issue_type_variants().join(", ")
        ))
    })?;
    issue.assignee = tool.assignee.unwrap_or_default();
    issue.owner = tool.owner.unwrap_or_default();
    issue.set_status(non_empty(tool.status).unwrap_or_else(|| "open".to_string()));
    let write_witness =
        build_write_witness(config, "issue.add", &issue_id, &path, instruction.as_ref());
    issue_attach_write_witness(&mut issue, write_witness.clone());
    let persisted = issue.clone();

    store.upsert_issue(issue);
    save_store(&store, &path)?;
    refresh_issue_query_projection(config, &path, &store)?;
    let now = Utc::now();

    json_result(json!({
        "action": "issue.add",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "issue": issue_summary_json(&persisted, now),
        "instruction": instruction.map(|link| link.to_json()),
        "writeWitness": write_witness
    }))
}

fn call_issue_claim(
    config: &PremathMcpConfig,
    tool: IssueClaimTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let instruction =
        resolve_instruction_link(config, tool.instruction_id, MutationAction::IssueClaim)?;
    let assignee = tool.assignee.trim().to_string();
    if assignee.is_empty() {
        return Err(lease_error(
            FAILURE_LEASE_INVALID_ASSIGNEE,
            "assignee is required",
        ));
    }
    let now = Utc::now();
    let lease_id = resolve_lease_id(tool.lease_id, &tool.id, &assignee);
    let lease_expires_at = parse_lease_expiry(tool.lease_ttl_seconds, tool.lease_expires_at, now)?;

    let write_witness =
        build_write_witness(config, "issue.claim", &tool.id, &path, instruction.as_ref());

    let (updated, changed, store) = mutate_store_jsonl(&path, |store| {
        let issue = store
            .issue_mut(&tool.id)
            .ok_or_else(|| call_tool_error(format!("issue not found: {}", tool.id)))?;

        if issue.status == "closed" {
            return Err(lease_error(
                FAILURE_LEASE_CLOSED,
                format!("cannot claim closed issue: {}", tool.id),
            ));
        }

        let mut changed = false;
        let mut status_changed = false;

        if issue.lease_state_at(now) == IssueLeaseState::Stale {
            issue.lease = None;
            changed = true;

            if issue.status == "in_progress" {
                issue.set_status("open".to_string());
                status_changed = true;
            }

            if !issue.assignee.is_empty() && issue.assignee != assignee {
                issue.assignee.clear();
                changed = true;
            }
        }

        if let Some(active_lease) = issue.lease.as_ref().filter(|lease| lease.expires_at > now)
            && active_lease.owner != assignee
        {
            return Err(lease_error(
                FAILURE_LEASE_CONTENTION_ACTIVE,
                format!(
                    "issue already leased: {} (owner={}, lease_id={})",
                    tool.id, active_lease.owner, active_lease.lease_id
                ),
            ));
        }

        if issue.lease.is_none() && !issue.assignee.is_empty() && issue.assignee != assignee {
            return Err(lease_error(
                FAILURE_LEASE_CONTENTION_ACTIVE,
                format!(
                    "issue already claimed: {} (assignee={})",
                    tool.id, issue.assignee
                ),
            ));
        }

        if issue.assignee != assignee {
            issue.assignee = assignee.clone();
            changed = true;
        }
        if issue.status != "in_progress" {
            issue.set_status("in_progress".to_string());
            changed = true;
            status_changed = true;
        }

        let next_lease = match issue.lease.as_ref() {
            Some(existing) if existing.owner == assignee && existing.lease_id == lease_id => {
                IssueLease {
                    lease_id: lease_id.clone(),
                    owner: assignee.clone(),
                    acquired_at: existing.acquired_at,
                    expires_at: lease_expires_at,
                    renewed_at: Some(now),
                }
            }
            _ => IssueLease {
                lease_id: lease_id.clone(),
                owner: assignee.clone(),
                acquired_at: now,
                expires_at: lease_expires_at,
                renewed_at: None,
            },
        };

        if issue.lease.as_ref() != Some(&next_lease) {
            issue.lease = Some(next_lease);
            changed = true;
        }

        if changed && !status_changed {
            issue.touch_updated_at();
        }

        if changed {
            issue_attach_write_witness(issue, write_witness.clone());
        }
        let updated = issue.clone();

        Ok(((updated, changed, store.clone()), changed))
    })
    .map_err(map_atomic_store_mutation_error)?;

    if changed {
        refresh_issue_query_projection(config, &path, &store)?;
    }
    let lease_projection = compute_lease_projection(&store, now);

    json_result(json!({
        "action": "issue.claim",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "issue": issue_summary_json(&updated, now),
        "changed": changed,
        "instruction": instruction.map(|link| link.to_json()),
        "writeWitness": if changed { Some(write_witness) } else { None },
        "leaseProjection": lease_projection
    }))
}

fn call_issue_lease_renew(
    config: &PremathMcpConfig,
    tool: IssueLeaseRenewTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let instruction =
        resolve_instruction_link(config, tool.instruction_id, MutationAction::IssueLeaseRenew)?;
    let assignee = tool.assignee.trim().to_string();
    if assignee.is_empty() {
        return Err(lease_error(
            FAILURE_LEASE_INVALID_ASSIGNEE,
            "assignee is required",
        ));
    }
    let lease_id = tool.lease_id.trim().to_string();
    if lease_id.is_empty() {
        return Err(lease_error(
            FAILURE_LEASE_ID_MISMATCH,
            "lease_id is required",
        ));
    }
    let now = Utc::now();
    let lease_expires_at = parse_lease_expiry(tool.lease_ttl_seconds, tool.lease_expires_at, now)?;
    let write_witness = build_write_witness(
        config,
        "issue.lease_renew",
        &tool.id,
        &path,
        instruction.as_ref(),
    );

    let (updated, changed, store) = mutate_store_jsonl(&path, |store| {
        let issue = store.issue_mut(&tool.id).ok_or_else(|| {
            lease_error(
                FAILURE_LEASE_NOT_FOUND,
                format!("issue not found: {}", tool.id),
            )
        })?;

        if issue.status == "closed" {
            return Err(lease_error(
                FAILURE_LEASE_CLOSED,
                format!("cannot renew lease on closed issue: {}", tool.id),
            ));
        }

        let current = issue.lease.clone().ok_or_else(|| {
            lease_error(
                FAILURE_LEASE_MISSING,
                format!("issue has no lease: {}", tool.id),
            )
        })?;

        if current.expires_at <= now {
            return Err(lease_error(
                FAILURE_LEASE_STALE,
                format!("lease is stale and must be reclaimed: {}", tool.id),
            ));
        }
        if current.owner != assignee {
            return Err(lease_error(
                FAILURE_LEASE_OWNER_MISMATCH,
                format!(
                    "lease owner mismatch for {} (expected={}, got={})",
                    tool.id, current.owner, assignee
                ),
            ));
        }
        if current.lease_id != lease_id {
            return Err(lease_error(
                FAILURE_LEASE_ID_MISMATCH,
                format!(
                    "lease_id mismatch for {} (expected={}, got={})",
                    tool.id, current.lease_id, lease_id
                ),
            ));
        }

        let mut changed = false;
        let mut status_changed = false;
        if issue.assignee != assignee {
            issue.assignee = assignee.clone();
            changed = true;
        }
        if issue.status != "in_progress" {
            issue.set_status("in_progress".to_string());
            changed = true;
            status_changed = true;
        }

        let renewed = IssueLease {
            lease_id,
            owner: assignee,
            acquired_at: current.acquired_at,
            expires_at: lease_expires_at,
            renewed_at: Some(now),
        };
        if issue.lease.as_ref() != Some(&renewed) {
            issue.lease = Some(renewed);
            changed = true;
        }

        if changed && !status_changed {
            issue.touch_updated_at();
        }
        if changed {
            issue_attach_write_witness(issue, write_witness.clone());
        }
        let updated = issue.clone();

        Ok(((updated, changed, store.clone()), changed))
    })
    .map_err(map_atomic_store_mutation_error)?;

    if changed {
        refresh_issue_query_projection(config, &path, &store)?;
    }
    let lease_projection = compute_lease_projection(&store, now);

    json_result(json!({
        "action": "issue.lease_renew",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "issue": issue_summary_json(&updated, now),
        "changed": changed,
        "instruction": instruction.map(|link| link.to_json()),
        "writeWitness": if changed { Some(write_witness) } else { None },
        "leaseProjection": lease_projection
    }))
}

fn call_issue_lease_release(
    config: &PremathMcpConfig,
    tool: IssueLeaseReleaseTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let instruction = resolve_instruction_link(
        config,
        tool.instruction_id,
        MutationAction::IssueLeaseRelease,
    )?;
    let expected_assignee = non_empty(tool.assignee);
    let expected_lease_id = non_empty(tool.lease_id);
    let now = Utc::now();
    let write_witness = build_write_witness(
        config,
        "issue.lease_release",
        &tool.id,
        &path,
        instruction.as_ref(),
    );

    let (updated, changed, store) = mutate_store_jsonl(&path, |store| {
        let issue = store.issue_mut(&tool.id).ok_or_else(|| {
            lease_error(
                FAILURE_LEASE_NOT_FOUND,
                format!("issue not found: {}", tool.id),
            )
        })?;

        let mut changed = false;
        let mut status_changed = false;

        match issue.lease.as_ref() {
            None => {
                if expected_assignee.is_some() || expected_lease_id.is_some() {
                    return Err(lease_error(
                        FAILURE_LEASE_MISSING,
                        format!("issue has no lease: {}", tool.id),
                    ));
                }
            }
            Some(current) => {
                if let Some(expected) = expected_assignee.as_ref()
                    && current.owner != *expected
                {
                    return Err(lease_error(
                        FAILURE_LEASE_OWNER_MISMATCH,
                        format!(
                            "lease owner mismatch for {} (expected={}, got={})",
                            tool.id, current.owner, expected
                        ),
                    ));
                }
                if let Some(expected) = expected_lease_id.as_ref()
                    && current.lease_id != *expected
                {
                    return Err(lease_error(
                        FAILURE_LEASE_ID_MISMATCH,
                        format!(
                            "lease_id mismatch for {} (expected={}, got={})",
                            tool.id, current.lease_id, expected
                        ),
                    ));
                }
                issue.lease = None;
                changed = true;
            }
        }

        if changed {
            if !issue.assignee.is_empty() {
                issue.assignee.clear();
                changed = true;
            }
            if issue.status == "in_progress" {
                issue.set_status("open".to_string());
                status_changed = true;
            }
            if !status_changed {
                issue.touch_updated_at();
            }
            issue_attach_write_witness(issue, write_witness.clone());
        }
        let updated = issue.clone();

        Ok(((updated, changed, store.clone()), changed))
    })
    .map_err(map_atomic_store_mutation_error)?;

    if changed {
        refresh_issue_query_projection(config, &path, &store)?;
    }
    let lease_projection = compute_lease_projection(&store, now);

    json_result(json!({
        "action": "issue.lease_release",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "issue": issue_summary_json(&updated, now),
        "changed": changed,
        "instruction": instruction.map(|link| link.to_json()),
        "writeWitness": if changed { Some(write_witness) } else { None },
        "leaseProjection": lease_projection
    }))
}

fn call_issue_lease_projection(
    config: &PremathMcpConfig,
    tool: IssueLeaseProjectionTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let graph = load_issue_graph(config, &path)?;
    let now = Utc::now();
    let projection = compute_lease_projection(&graph.store, now);
    let items = graph
        .store
        .issues()
        .filter(|issue| issue.lease.is_some())
        .map(|issue| {
            json!({
                "id": issue.id,
                "status": issue.status,
                "assignee": issue.assignee,
                "lease": lease_json(issue, now),
                "contended": issue_is_lease_contended(issue, now),
            })
        })
        .collect::<Vec<_>>();

    json_result(json!({
        "action": "issue.lease_projection",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "querySource": graph.query_source,
        "projection": projection,
        "count": items.len(),
        "items": items
    }))
}

fn call_issue_discover(
    config: &PremathMcpConfig,
    tool: IssueDiscoverTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let mut store = load_store_existing(&path)?;
    let instruction =
        resolve_instruction_link(config, tool.instruction_id, MutationAction::IssueDiscover)?;
    if store.issue(&tool.parent_issue_id).is_none() {
        return Err(call_tool_error(format!(
            "parent issue not found: {}",
            tool.parent_issue_id
        )));
    }

    let issue_id = non_empty(tool.id).unwrap_or_else(|| next_issue_id(&store));
    if store.issue(&issue_id).is_some() {
        return Err(call_tool_error(format!("issue already exists: {issue_id}")));
    }

    let mut issue = Issue::new(issue_id.clone(), tool.title);
    issue.description = tool.description.unwrap_or_default();
    issue.priority = tool.priority.unwrap_or(2);
    let raw_issue_type = non_empty(tool.issue_type).unwrap_or_else(|| "task".to_string());
    issue.issue_type = parse_issue_type(&raw_issue_type).ok_or_else(|| {
        call_tool_error(format!(
            "invalid issue_type `{}` (expected one of: {})",
            raw_issue_type,
            issue_type_variants().join(", ")
        ))
    })?;
    issue.assignee = tool.assignee.unwrap_or_default();
    issue.owner = tool.owner.unwrap_or_default();
    issue.set_status("open".to_string());

    store.upsert_issue(issue);
    store
        .add_dependency(
            &issue_id,
            &tool.parent_issue_id,
            DepType::DiscoveredFrom,
            String::new(),
        )
        .map_err(|e| call_tool_error(format!("failed to add discovered-from dependency: {e}")))?;

    let write_witness = build_write_witness(
        config,
        "issue.discover",
        &issue_id,
        &path,
        instruction.as_ref(),
    );

    let discovered = {
        let issue = store
            .issue_mut(&issue_id)
            .ok_or_else(|| call_tool_error(format!("issue not found: {}", issue_id)))?;
        issue_attach_write_witness(issue, write_witness.clone());
        issue.clone()
    };

    save_store(&store, &path)?;
    refresh_issue_query_projection(config, &path, &store)?;
    let now = Utc::now();

    json_result(json!({
        "action": "issue.discover",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "issue": issue_summary_json(&discovered, now),
        "dependency": {
            "issueId": issue_id,
            "dependsOnId": tool.parent_issue_id,
            "type": "discovered-from"
        },
        "instruction": instruction.map(|link| link.to_json()),
        "writeWitness": write_witness
    }))
}

fn call_issue_update(
    config: &PremathMcpConfig,
    tool: IssueUpdateTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let mut store = load_store_existing(&path)?;
    let instruction =
        resolve_instruction_link(config, tool.instruction_id, MutationAction::IssueUpdate)?;
    let write_witness = build_write_witness(
        config,
        "issue.update",
        &tool.id,
        &path,
        instruction.as_ref(),
    );

    let updated = {
        let issue = store
            .issue_mut(&tool.id)
            .ok_or_else(|| call_tool_error(format!("issue not found: {}", tool.id)))?;

        let mut changed = false;
        let mut status_changed = false;

        if let Some(next) = tool.title {
            issue.title = next;
            changed = true;
        }
        if let Some(next) = tool.description {
            issue.description = next;
            changed = true;
        }
        if let Some(next) = tool.notes {
            issue.notes = next;
            changed = true;
        }
        if let Some(next) = tool.priority {
            issue.priority = next;
            changed = true;
        }
        if let Some(next) = tool.assignee {
            issue.assignee = next;
            changed = true;
        }
        if let Some(next) = tool.owner {
            issue.owner = next;
            changed = true;
        }
        if let Some(next) = non_empty(tool.status) {
            issue.set_status(next);
            changed = true;
            status_changed = true;
        }

        if !changed {
            return Err(call_tool_error("no update fields provided"));
        }

        if !status_changed {
            issue.touch_updated_at();
        }
        issue_attach_write_witness(issue, write_witness.clone());

        issue.clone()
    };

    save_store(&store, &path)?;
    refresh_issue_query_projection(config, &path, &store)?;
    let now = Utc::now();

    json_result(json!({
        "action": "issue.update",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "issue": issue_summary_json(&updated, now),
        "instruction": instruction.map(|link| link.to_json()),
        "writeWitness": write_witness
    }))
}

fn call_dep_add(
    config: &PremathMcpConfig,
    tool: DepAddTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let mut store = load_store_existing(&path)?;
    let dep_type = parse_dep_type(tool.dep_type)?;
    let created_by = tool.created_by.unwrap_or_default();
    let instruction =
        resolve_instruction_link(config, tool.instruction_id, MutationAction::DepAdd)?;
    let write_witness = build_write_witness(
        config,
        "dep.add",
        &tool.issue_id,
        &path,
        instruction.as_ref(),
    );

    store
        .add_dependency(
            &tool.issue_id,
            &tool.depends_on_id,
            dep_type.clone(),
            created_by.clone(),
        )
        .map_err(|e| call_tool_error(format!("failed to add dependency: {e}")))?;
    let issue = store.issue_mut(&tool.issue_id).ok_or_else(|| {
        call_tool_error(format!("issue not found after dep add: {}", tool.issue_id))
    })?;
    issue_attach_write_witness(issue, write_witness.clone());
    save_store(&store, &path)?;
    refresh_issue_query_projection(config, &path, &store)?;

    json_result(json!({
        "action": "dep.add",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "dependency": {
            "issueId": tool.issue_id,
            "dependsOnId": tool.depends_on_id,
            "type": dep_type.as_str(),
            "createdBy": created_by
        },
        "instruction": instruction.map(|link| link.to_json()),
        "writeWitness": write_witness
    }))
}

fn call_dep_remove(
    config: &PremathMcpConfig,
    tool: DepRemoveTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let mut store = load_store_existing(&path)?;
    let dep_type = parse_dep_type(tool.dep_type)?;
    let instruction =
        resolve_instruction_link(config, tool.instruction_id, MutationAction::DepRemove)?;
    let write_witness = build_write_witness(
        config,
        "dep.remove",
        &tool.issue_id,
        &path,
        instruction.as_ref(),
    );

    store
        .remove_dependency(&tool.issue_id, &tool.depends_on_id, dep_type.clone())
        .map_err(|e| call_tool_error(format!("failed to remove dependency: {e}")))?;
    let issue = store.issue_mut(&tool.issue_id).ok_or_else(|| {
        call_tool_error(format!(
            "issue not found after dep remove: {}",
            tool.issue_id
        ))
    })?;
    issue_attach_write_witness(issue, write_witness.clone());
    save_store(&store, &path)?;
    refresh_issue_query_projection(config, &path, &store)?;

    json_result(json!({
        "action": "dep.remove",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "dependency": {
            "issueId": tool.issue_id,
            "dependsOnId": tool.depends_on_id,
            "type": dep_type.as_str()
        },
        "instruction": instruction.map(|link| link.to_json()),
        "writeWitness": write_witness
    }))
}

fn call_dep_replace(
    config: &PremathMcpConfig,
    tool: DepReplaceTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let mut store = load_store_existing(&path)?;
    let from_dep_type = parse_dep_type(tool.from_dep_type)?;
    let to_dep_type = parse_dep_type(Some(tool.to_dep_type.clone()))?;
    let created_by = tool.created_by.unwrap_or_default();
    let instruction =
        resolve_instruction_link(config, tool.instruction_id, MutationAction::DepReplace)?;
    let write_witness = build_write_witness(
        config,
        "dep.replace",
        &tool.issue_id,
        &path,
        instruction.as_ref(),
    );

    store
        .replace_dependency(
            &tool.issue_id,
            &tool.depends_on_id,
            from_dep_type.clone(),
            to_dep_type.clone(),
            created_by.clone(),
        )
        .map_err(|e| call_tool_error(format!("failed to replace dependency: {e}")))?;
    let issue = store.issue_mut(&tool.issue_id).ok_or_else(|| {
        call_tool_error(format!(
            "issue not found after dep replace: {}",
            tool.issue_id
        ))
    })?;
    issue_attach_write_witness(issue, write_witness.clone());
    save_store(&store, &path)?;
    refresh_issue_query_projection(config, &path, &store)?;

    json_result(json!({
        "action": "dep.replace",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "dependency": {
            "issueId": tool.issue_id,
            "dependsOnId": tool.depends_on_id,
            "fromType": from_dep_type.as_str(),
            "toType": to_dep_type.as_str(),
            "createdBy": created_by
        },
        "instruction": instruction.map(|link| link.to_json()),
        "writeWitness": write_witness
    }))
}

fn call_dep_diagnostics(
    config: &PremathMcpConfig,
    tool: DepDiagnosticsTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let store = load_store_existing(&path)?;
    let graph_scope: DependencyGraphScope = tool.graph_scope.into();
    let cycle = store.find_any_dependency_cycle_in_scope(graph_scope);

    json_result(json!({
        "action": "dep.diagnostics",
        "issuesPath": path.display().to_string(),
        "queryBackend": config.issue_query_backend.as_str(),
        "graphScope": graph_scope.as_str(),
        "integrity": {
            "hasCycle": cycle.is_some(),
            "cyclePath": cycle
        }
    }))
}

fn call_init_tool(
    config: &PremathMcpConfig,
    tool: InitTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let configured_root = resolve_repo_root(&config.repo_root);
    let root = resolve_root_path(tool.root_path, &configured_root);
    let outcome = init_layout(&root).map_err(call_tool_error)?;
    init_tool_result(outcome)
}

fn init_tool_result(outcome: InitOutcome) -> std::result::Result<CallToolResult, CallToolError> {
    json_result(json!({
        "action": "init.tool",
        "repoRoot": outcome.repo_root.display().to_string(),
        "premathDir": outcome.premath_dir.display().to_string(),
        "issuesPath": outcome.issues_path.display().to_string(),
        "createdRepoRoot": outcome.created_repo_root,
        "createdPremathDir": outcome.created_premath_dir,
        "createdIssuesFile": outcome.created_issues_file,
        "migratedFromLegacy": outcome
            .migrated_from_legacy
            .map(|path| path.display().to_string())
    }))
}

fn call_observe_latest(
    config: &PremathMcpConfig,
    tool: ObserveLatestTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.surface_path, &config.surface_path);
    let service = load_observe_service(&path)?;
    let value = service
        .query_json(ObserveQuery::Latest)
        .map_err(|e| call_tool_error(format!("failed to query latest: {e}")))?;

    json_result(json!({
        "action": "observe.latest",
        "surfacePath": path.display().to_string(),
        "view": value
    }))
}

fn call_observe_needs_attention(
    config: &PremathMcpConfig,
    tool: ObserveNeedsAttentionTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.surface_path, &config.surface_path);
    let service = load_observe_service(&path)?;
    let value = service
        .query_json(ObserveQuery::NeedsAttention)
        .map_err(|e| call_tool_error(format!("failed to query needs-attention: {e}")))?;

    json_result(json!({
        "action": "observe.needs_attention",
        "surfacePath": path.display().to_string(),
        "view": value
    }))
}

fn call_observe_instruction(
    config: &PremathMcpConfig,
    tool: ObserveInstructionTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.surface_path, &config.surface_path);
    let service = load_observe_service(&path)?;
    let value = service
        .query_json(ObserveQuery::Instruction {
            instruction_id: tool.instruction_id,
        })
        .map_err(|e| call_tool_error(format!("failed to query instruction: {e}")))?;

    json_result(json!({
        "action": "observe.instruction",
        "surfacePath": path.display().to_string(),
        "view": value
    }))
}

fn call_observe_projection(
    config: &PremathMcpConfig,
    tool: ObserveProjectionTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.surface_path, &config.surface_path);
    let service = load_observe_service(&path)?;
    let value = service
        .query_json(ObserveQuery::Projection {
            projection_digest: tool.projection_digest,
            projection_match: tool.projection_match.into(),
        })
        .map_err(|e| call_tool_error(format!("failed to query projection: {e}")))?;

    json_result(json!({
        "action": "observe.projection",
        "surfacePath": path.display().to_string(),
        "view": value
    }))
}

fn call_instruction_check(
    config: &PremathMcpConfig,
    tool: InstructionCheckTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let repo_root = resolve_repo_root(&config.repo_root);
    let instruction_path = resolve_instruction_path(&repo_root, &tool.instruction_path);

    let result = fs::read(&instruction_path)
        .map_err(|err| {
            call_tool_error(format!(
                "instruction_envelope_invalid: failed to read instruction file {}: {err}",
                instruction_path.display()
            ))
        })
        .and_then(|bytes| {
            serde_json::from_slice::<Value>(&bytes).map_err(|err| {
                call_tool_error(format!(
                    "instruction_envelope_invalid_json: failed to parse instruction json {}: {err}",
                    instruction_path.display()
                ))
            })
        })
        .and_then(|raw| {
            validate_instruction_envelope_payload(&raw, &instruction_path, &repo_root)
                .map_err(|err| call_tool_error(err.to_string()))
        });

    let (ok, exit_code, stdout, stderr) = match result {
        Ok(checked) => (
            true,
            Some(0),
            truncate_for_payload(
                &serde_json::to_string_pretty(&checked).unwrap_or_else(|_| "{}".to_string()),
                16_000,
            ),
            String::new(),
        ),
        Err(err) => (
            false,
            Some(2),
            String::new(),
            truncate_for_payload(&err.to_string(), 16_000),
        ),
    };

    let payload = json!({
        "action": "instruction.check",
        "repoRoot": repo_root.display().to_string(),
        "instructionPath": instruction_path.display().to_string(),
        "ok": ok,
        "exitCode": exit_code,
        "stdout": stdout,
        "stderr": stderr
    });
    json_result(payload)
}

fn call_instruction_run(
    config: &PremathMcpConfig,
    tool: InstructionRunTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let repo_root = resolve_repo_root(&config.repo_root);
    let instruction_path = resolve_instruction_path(&repo_root, &tool.instruction_path);
    let allow_failure = tool.allow_failure.unwrap_or(false);
    let instruction_id = instruction_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut cmd = Command::new("python3");
    cmd.arg("tools/ci/pipeline_instruction.py")
        .arg("--instruction")
        .arg(&instruction_path)
        .arg("--repo-root")
        .arg(&repo_root)
        .current_dir(&repo_root);
    if allow_failure {
        cmd.arg("--allow-failure");
    }

    let output = cmd
        .output()
        .map_err(|e| call_tool_error(format!("failed to execute instruction pipeline: {e}")))?;

    let witness_path = repo_root
        .join("artifacts")
        .join("ciwitness")
        .join(format!("{instruction_id}.json"));
    let witness_exists = witness_path.exists();

    let payload = json!({
        "action": "instruction.run",
        "repoRoot": repo_root.display().to_string(),
        "instructionPath": instruction_path.display().to_string(),
        "allowFailure": allow_failure,
        "ok": output.status.success(),
        "exitCode": output.status.code(),
        "witnessPath": witness_path.display().to_string(),
        "witnessExists": witness_exists,
        "stdout": truncate_for_payload(&String::from_utf8_lossy(&output.stdout), 16_000),
        "stderr": truncate_for_payload(&String::from_utf8_lossy(&output.stderr), 16_000)
    });
    json_result(payload)
}

fn load_observe_service(
    path: &Path,
) -> std::result::Result<UxService<SurrealObservationBackend>, CallToolError> {
    let backend = SurrealObservationBackend::load_json(path).map_err(|e| {
        call_tool_error(format!(
            "failed to load observation surface at {}: {e}",
            path.display()
        ))
    })?;
    Ok(UxService::new(backend))
}

fn resolve_path(input: Option<String>, default_path: &str) -> PathBuf {
    let candidate = input.unwrap_or_else(|| default_path.to_string());
    let candidate = candidate.trim();
    if candidate.is_empty() {
        PathBuf::from(default_path)
    } else {
        PathBuf::from(candidate)
    }
}

fn resolve_repo_root(configured: &str) -> PathBuf {
    let root = PathBuf::from(configured);
    if root.is_absolute() {
        root
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(root)
    }
}

fn resolve_root_path(input: Option<String>, default_root: &Path) -> PathBuf {
    let candidate = input.unwrap_or_else(|| default_root.display().to_string());
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return default_root.to_path_buf();
    }

    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        path
    } else {
        default_root.join(path)
    }
}

fn resolve_instruction_path(repo_root: &Path, instruction_path: &str) -> PathBuf {
    let path = PathBuf::from(instruction_path);
    if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|v| if v.trim().is_empty() { None } else { Some(v) })
}

fn parse_dep_type(value: Option<String>) -> std::result::Result<DepType, CallToolError> {
    let raw = non_empty(value).unwrap_or_else(|| "blocks".to_string());
    let dep_type = match raw.as_str() {
        "blocks" => DepType::Blocks,
        "parent-child" => DepType::ParentChild,
        "conditional-blocks" => DepType::ConditionalBlocks,
        "related" => DepType::Related,
        "discovered-from" => DepType::DiscoveredFrom,
        "relates-to" => DepType::RelatesTo,
        "duplicates" => DepType::Duplicates,
        "supersedes" => DepType::Supersedes,
        "waits-for" => DepType::WaitsFor,
        "replies-to" => DepType::RepliesTo,
        _ => {
            return Err(call_tool_error(format!(
                "invalid dep_type `{raw}` (expected one of: blocks, parent-child, conditional-blocks, related, discovered-from, relates-to, duplicates, supersedes, waits-for, replies-to)"
            )));
        }
    };
    Ok(dep_type)
}

fn resolve_instruction_link(
    config: &PremathMcpConfig,
    instruction_id: Option<String>,
    action: MutationAction,
) -> std::result::Result<Option<InstructionWitnessLink>, CallToolError> {
    let instruction_id = non_empty(instruction_id);
    if config.mutation_policy == MutationPolicy::InstructionLinked && instruction_id.is_none() {
        return Err(call_tool_error(
            "mutation policy `instruction-linked` requires `instruction_id` on mutation tools",
        ));
    }
    match instruction_id {
        None => Ok(None),
        Some(id) => {
            let link = load_instruction_witness_link(config, &id)?;
            enforce_instruction_mutation_scope(config, &link, action)?;
            Ok(Some(link))
        }
    }
}

fn policy_allows_instruction_mutation(policy_digest: &str) -> bool {
    matches!(policy_digest, POLICY_DIGEST_CI_V1 | POLICY_DIGEST_TEST_V1)
}

fn enforce_instruction_mutation_scope(
    config: &PremathMcpConfig,
    instruction: &InstructionWitnessLink,
    action: MutationAction,
) -> std::result::Result<(), CallToolError> {
    if config.mutation_policy != MutationPolicy::InstructionLinked {
        return Ok(());
    }

    let policy_digest = instruction.policy_digest.as_deref().ok_or_else(|| {
        call_tool_error(format!(
            "instruction witness `{}` missing `policyDigest` required for mutation policy scope",
            instruction.instruction_id
        ))
    })?;
    if !policy_allows_instruction_mutation(policy_digest) {
        return Err(call_tool_error(format!(
            "instruction policyDigest `{policy_digest}` is not scoped for mutation action `{}`",
            action.action_id()
        )));
    }

    let claims = &instruction.capability_claims;
    if !claims
        .iter()
        .any(|item| item == CHANGE_MORPHISMS_BASE_CAPABILITY)
    {
        return Err(call_tool_error(format!(
            "instruction `{}` missing required capability claim `{}` for mutation action `{}`",
            instruction.instruction_id,
            CHANGE_MORPHISMS_BASE_CAPABILITY,
            action.action_id()
        )));
    }

    let action_claim = action.required_capability_claim();
    let action_allowed = claims
        .iter()
        .any(|item| item == action_claim || item == CHANGE_MORPHISMS_ALL_CAPABILITY);
    if !action_allowed {
        return Err(call_tool_error(format!(
            "instruction `{}` missing required action capability claim `{}` for mutation action `{}`",
            instruction.instruction_id,
            action_claim,
            action.action_id()
        )));
    }

    Ok(())
}

fn load_instruction_witness_link(
    config: &PremathMcpConfig,
    instruction_id: &str,
) -> std::result::Result<InstructionWitnessLink, CallToolError> {
    let repo_root = resolve_repo_root(&config.repo_root);
    let witness_path = repo_root
        .join("artifacts")
        .join("ciwitness")
        .join(format!("{instruction_id}.json"));
    if !witness_path.exists() {
        return Err(call_tool_error(format!(
            "instruction witness not found: {}",
            witness_path.display()
        )));
    }

    let bytes = fs::read(&witness_path)
        .map_err(|e| call_tool_error(format!("failed to read {}: {e}", witness_path.display())))?;
    let payload = serde_json::from_slice::<Value>(&bytes).map_err(|e| {
        call_tool_error(format!(
            "failed to parse instruction witness {}: {e}",
            witness_path.display()
        ))
    })?;

    let id_in_witness = payload
        .get("instructionId")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            call_tool_error(format!(
                "instruction witness missing `instructionId`: {}",
                witness_path.display()
            ))
        })?;
    if id_in_witness != instruction_id {
        return Err(call_tool_error(format!(
            "instruction witness id mismatch (expected `{instruction_id}`, got `{id_in_witness}`)"
        )));
    }

    let verdict = payload.get("verdictClass").and_then(Value::as_str);
    if verdict != Some("accepted") {
        return Err(call_tool_error(format!(
            "instruction witness is not accepted for `{instruction_id}` (verdictClass={})",
            verdict.unwrap_or("missing")
        )));
    }

    Ok(InstructionWitnessLink {
        instruction_id: instruction_id.to_string(),
        witness_path: witness_path.display().to_string(),
        instruction_digest: payload
            .get("instructionDigest")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        policy_digest: payload
            .get("policyDigest")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        capability_claims: json_string_array(payload.get("capabilityClaims")),
        required_checks: json_string_array(payload.get("requiredChecks")),
        executed_checks: json_string_array(payload.get("executedChecks")),
    })
}

fn json_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn build_write_witness(
    config: &PremathMcpConfig,
    action: &str,
    issue_id: &str,
    issues_path: &Path,
    instruction: Option<&InstructionWitnessLink>,
) -> Value {
    let repo_root = resolve_repo_root(&config.repo_root);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let witness_id = format!("bdw1_{}", now.as_nanos());
    let jj_snapshot = maybe_jj_snapshot_for_root(&repo_root);

    json!({
        "schema": 1,
        "witnessKind": "bd.issue.write.v1",
        "witnessId": witness_id,
        "action": action,
        "issueId": issue_id,
        "issuesPath": issues_path.display().to_string(),
        "recordedAtUnixMs": now.as_millis(),
        "repoRoot": repo_root.display().to_string(),
        "mutationPolicy": config.mutation_policy.as_str(),
        "queryBackend": config.issue_query_backend.as_str(),
        "instruction": instruction.map(|link| link.to_json()),
        "jjSnapshot": jj_snapshot
    })
}

fn maybe_jj_snapshot_for_root(repo_root: &Path) -> Option<Value> {
    let client = JjClient::discover(repo_root).ok()?;
    let snapshot = client.snapshot().ok()?;
    Some(json!({
        "repoRoot": snapshot.repo_root.display().to_string(),
        "changeId": snapshot.change_id,
        "status": snapshot.status,
    }))
}

fn issue_attach_write_witness(issue: &mut Issue, witness: Value) {
    let mut metadata = match issue.metadata.take() {
        Some(Value::Object(map)) => map,
        Some(other) => {
            let mut map = serde_json::Map::new();
            map.insert("legacyMetadata".to_string(), other);
            map
        }
        None => serde_json::Map::new(),
    };
    metadata.insert("premathWriteWitness".to_string(), witness);
    issue.metadata = Some(Value::Object(metadata));
}

fn load_issue_graph(
    config: &PremathMcpConfig,
    issues_path: &Path,
) -> std::result::Result<IssueGraphView, CallToolError> {
    match config.issue_query_backend {
        IssueQueryBackend::Jsonl => {
            let store = load_store_existing(issues_path)?;
            let cache = QueryCache::hydrate(&store);
            Ok(IssueGraphView {
                store,
                cache,
                query_source: "jsonl",
            })
        }
        IssueQueryBackend::Surreal => load_issue_graph_from_projection(config, issues_path),
    }
}

fn load_issue_graph_from_projection(
    config: &PremathMcpConfig,
    issues_path: &Path,
) -> std::result::Result<IssueGraphView, CallToolError> {
    let projection_path = resolve_issue_query_projection_path(config);

    let projection_is_stale = projection_needs_refresh(issues_path, &projection_path);
    let store = if projection_path.exists() && !projection_is_stale {
        match load_store_from_projection(&projection_path, issues_path) {
            Ok(load) if load.source_path_matches_authority && load.source_snapshot_ref_present => {
                load.store
            }
            Ok(_) | Err(_) => {
                let refreshed = load_store_existing(issues_path)?;
                write_issue_query_projection(&projection_path, issues_path, &refreshed)?;
                refreshed
            }
        }
    } else {
        let refreshed = load_store_existing(issues_path)?;
        write_issue_query_projection(&projection_path, issues_path, &refreshed)?;
        refreshed
    };

    let cache = QueryCache::hydrate(&store);
    Ok(IssueGraphView {
        store,
        cache,
        query_source: "surreal-projection",
    })
}

fn refresh_issue_query_projection(
    config: &PremathMcpConfig,
    issues_path: &Path,
    store: &MemoryStore,
) -> std::result::Result<(), CallToolError> {
    if config.issue_query_backend == IssueQueryBackend::Surreal {
        let projection_path = resolve_issue_query_projection_path(config);
        write_issue_query_projection(&projection_path, issues_path, store)?;
    }
    Ok(())
}

fn resolve_issue_query_projection_path(config: &PremathMcpConfig) -> PathBuf {
    let path = PathBuf::from(&config.issue_query_projection);
    if path.is_absolute() {
        path
    } else {
        resolve_repo_root(&config.repo_root).join(path)
    }
}

fn projection_needs_refresh(issues_path: &Path, projection_path: &Path) -> bool {
    if !projection_path.exists() {
        return true;
    }
    let issues_mtime = file_modified(issues_path);
    let projection_mtime = file_modified(projection_path);
    match (issues_mtime, projection_mtime) {
        (Some(issues_mtime), Some(projection_mtime)) => projection_mtime < issues_mtime,
        _ => true,
    }
}

fn file_modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn load_store_from_projection(
    path: &Path,
    authority_issues_path: &Path,
) -> std::result::Result<ProjectionLoad, CallToolError> {
    let analysis = analyze_issue_query_projection(path);
    if let Some(err) = analysis.error {
        return Err(call_tool_error(format!(
            "invalid projection {}: {err}",
            path.display()
        )));
    }

    let source_snapshot_ref_present = analysis.source_snapshot_ref.is_some();
    let source_issues_path = analysis.source_issues_path.ok_or_else(|| {
        call_tool_error(format!(
            "invalid projection {}: missing source issues path",
            path.display()
        ))
    })?;
    let store = analysis.store.ok_or_else(|| {
        call_tool_error(format!(
            "invalid projection {}: missing hydrated issue store",
            path.display()
        ))
    })?;
    let source_path_matches_authority =
        paths_equivalent(Path::new(&source_issues_path), authority_issues_path);
    Ok(ProjectionLoad {
        store,
        source_path_matches_authority,
        source_snapshot_ref_present,
    })
}

fn write_issue_query_projection(
    projection_path: &Path,
    issues_path: &Path,
    store: &MemoryStore,
) -> std::result::Result<(), CallToolError> {
    if let Some(parent) = projection_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| {
            call_tool_error(format!(
                "failed to create projection directory {}: {e}",
                parent.display()
            ))
        })?;
    }
    let generated_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let issues: Vec<Issue> = store.issues().cloned().collect();
    let payload = IssueQueryProjectionPayload {
        schema: ISSUE_QUERY_PROJECTION_SCHEMA,
        kind: ISSUE_QUERY_PROJECTION_KIND.to_string(),
        source_issues_path: issues_path.display().to_string(),
        source_snapshot_ref: Some(store_snapshot_ref(store)),
        generated_at_unix_ms,
        issue_count: issues.len(),
        issues,
    };
    let bytes = serde_json::to_vec_pretty(&payload)
        .map_err(|e| call_tool_error(format!("failed to encode projection payload: {e}")))?;
    fs::write(projection_path, bytes).map_err(|e| {
        call_tool_error(format!(
            "failed to write projection {}: {e}",
            projection_path.display()
        ))
    })
}

fn load_store_existing(path: &Path) -> std::result::Result<MemoryStore, CallToolError> {
    if !path.exists() {
        return Err(call_tool_error(format!(
            "issues file not found: {}",
            path.display()
        )));
    }
    MemoryStore::load_jsonl(path)
        .map_err(|e| call_tool_error(format!("failed to load {}: {e}", path.display())))
}

fn load_store_or_empty(path: &Path) -> std::result::Result<MemoryStore, CallToolError> {
    if path.exists() {
        return MemoryStore::load_jsonl(path)
            .map_err(|e| call_tool_error(format!("failed to load {}: {e}", path.display())));
    }
    Ok(MemoryStore::default())
}

fn save_store(store: &MemoryStore, path: &Path) -> std::result::Result<(), CallToolError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| {
            call_tool_error(format!(
                "failed to create issues directory {}: {e}",
                parent.display()
            ))
        })?;
    }
    store
        .save_jsonl(path)
        .map_err(|e| call_tool_error(format!("failed to save {}: {e}", path.display())))
}

fn next_issue_id(store: &MemoryStore) -> String {
    let mut seq = 1usize;
    loop {
        let candidate = format!("bd-{seq}");
        if store.issue(&candidate).is_none() {
            return candidate;
        }
        seq += 1;
    }
}

fn json_result(value: Value) -> std::result::Result<CallToolResult, CallToolError> {
    let text = serde_json::to_string_pretty(&value).map_err(CallToolError::new)?;
    Ok(CallToolResult::text_content(vec![TextContent::from(text)]))
}

fn truncate_for_payload(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    format!(
        "{}...[truncated {} bytes]",
        &text[..max_len],
        text.len() - max_len
    )
}

fn call_tool_error(message: impl Into<String>) -> CallToolError {
    CallToolError::from_message(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "premath-cli-mcp-{prefix}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp dir should exist");
        path
    }

    fn parse_tool_json(result: CallToolResult) -> Value {
        let text = result
            .content
            .first()
            .expect("result should contain content")
            .as_text_content()
            .expect("content should be text")
            .text
            .clone();
        serde_json::from_str(&text).expect("tool response should be valid json")
    }

    fn test_config(root: &Path, issues_path: &Path, surface_path: &Path) -> PremathMcpConfig {
        PremathMcpConfig {
            issues_path: issues_path.display().to_string(),
            issue_query_backend: IssueQueryBackend::Jsonl,
            issue_query_projection: root
                .join(".premath/surreal_issue_cache.json")
                .display()
                .to_string(),
            mutation_policy: MutationPolicy::Open,
            surface_path: surface_path.display().to_string(),
            repo_root: root.display().to_string(),
        }
    }

    #[test]
    fn issue_tools_roundtrip_over_jsonl() {
        let root = temp_dir("issue-roundtrip");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Root".to_string(),
                id: Some("bd-root".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("first issue should be added");

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Child".to_string(),
                id: Some("bd-child".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("second issue should be added");

        let _ = call_dep_add(
            &config,
            DepAddTool {
                issue_id: "bd-child".to_string(),
                depends_on_id: "bd-root".to_string(),
                dep_type: Some("blocks".to_string()),
                created_by: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("dependency should be added");

        let ready = call_issue_ready(&config, IssueReadyTool { issues_path: None })
            .expect("ready query should succeed");
        let payload = parse_tool_json(ready);
        assert_eq!(payload["count"], 1);
        assert_eq!(payload["items"][0]["id"], "bd-root");

        let blocked = call_issue_blocked(&config, IssueBlockedTool { issues_path: None })
            .expect("blocked query should succeed");
        let blocked_payload = parse_tool_json(blocked);
        assert_eq!(blocked_payload["count"], 1);
        assert_eq!(blocked_payload["items"][0]["id"], "bd-child");
        assert_eq!(
            blocked_payload["items"][0]["blockers"][0]["dependsOnId"],
            "bd-root"
        );
    }

    #[test]
    fn issue_check_reports_core_invariants() {
        let root = temp_dir("issue-check");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "[EPIC] Missing epic type".to_string(),
                id: Some("bd-epic".to_string()),
                description: Some(
                    "Acceptance:\n- done\n\nVerification commands:\n- `mise run baseline`\n"
                        .to_string(),
                ),
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("issue add should succeed");

        let check = call_issue_check(
            &config,
            IssueCheckTool {
                issues_path: None,
                note_warn_threshold: None,
            },
        )
        .expect("issue check should execute");
        let payload = parse_tool_json(check);
        assert_eq!(payload["action"], "issue.check");
        assert_eq!(payload["checkKind"], "premath.issue_graph.check.v1");
        assert_eq!(payload["result"], "rejected");
        assert_eq!(
            payload["failureClasses"],
            serde_json::json!(["issue_graph.issue_type.epic_mismatch"])
        );
    }

    #[test]
    fn dep_replace_and_remove_update_ready_status() {
        let root = temp_dir("dep-replace-remove-ready");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Root".to_string(),
                id: Some("bd-root".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("first issue should be added");

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Child".to_string(),
                id: Some("bd-child".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("second issue should be added");

        let _ = call_dep_add(
            &config,
            DepAddTool {
                issue_id: "bd-child".to_string(),
                depends_on_id: "bd-root".to_string(),
                dep_type: Some("blocks".to_string()),
                created_by: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("dependency should be added");

        let _ = call_dep_replace(
            &config,
            DepReplaceTool {
                issue_id: "bd-child".to_string(),
                depends_on_id: "bd-root".to_string(),
                from_dep_type: Some("blocks".to_string()),
                to_dep_type: "related".to_string(),
                created_by: Some("codex".to_string()),
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("dependency type should be replaced");

        let ready = call_issue_ready(&config, IssueReadyTool { issues_path: None })
            .expect("ready query should succeed");
        let payload = parse_tool_json(ready);
        assert_eq!(payload["count"], 2);

        let _ = call_dep_remove(
            &config,
            DepRemoveTool {
                issue_id: "bd-child".to_string(),
                depends_on_id: "bd-root".to_string(),
                dep_type: Some("related".to_string()),
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("dependency should be removed");

        let blocked = call_issue_blocked(&config, IssueBlockedTool { issues_path: None })
            .expect("blocked query should succeed");
        let blocked_payload = parse_tool_json(blocked);
        assert_eq!(blocked_payload["count"], 0);
    }

    #[test]
    fn dep_add_rejects_cycle_edges() {
        let root = temp_dir("dep-add-cycle-reject");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "A".to_string(),
                id: Some("bd-a".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("issue A should add");
        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "B".to_string(),
                id: Some("bd-b".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("issue B should add");

        let _ = call_dep_add(
            &config,
            DepAddTool {
                issue_id: "bd-a".to_string(),
                depends_on_id: "bd-b".to_string(),
                dep_type: Some("blocks".to_string()),
                created_by: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("first edge should add");

        let err = call_dep_add(
            &config,
            DepAddTool {
                issue_id: "bd-b".to_string(),
                depends_on_id: "bd-a".to_string(),
                dep_type: Some("blocks".to_string()),
                created_by: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect_err("cycle edge should reject");
        assert!(
            err.to_string().contains("dependency cycle detected"),
            "expected cycle diagnostic, got: {err}"
        );
    }

    #[test]
    fn dep_diagnostics_reports_scoped_cycle_integrity() {
        let root = temp_dir("dep-diagnostics-scope");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));
        fs::write(
            &issues,
            concat!(
                r#"{"id":"bd-a","title":"A","status":"closed","dependencies":[{"issue_id":"bd-a","depends_on_id":"bd-b","type":"blocks"}]}"#,
                "\n",
                r#"{"id":"bd-b","title":"B","status":"closed","dependencies":[{"issue_id":"bd-b","depends_on_id":"bd-a","type":"blocks"}]}"#,
                "\n",
                r#"{"id":"bd-c","title":"C","status":"open"}"#,
                "\n"
            ),
        )
        .expect("issues fixture should write");

        let active = call_dep_diagnostics(
            &config,
            DepDiagnosticsTool {
                issues_path: None,
                graph_scope: DepGraphScopeToolArg::Active,
            },
        )
        .expect("active diagnostics should succeed");
        let active_payload = parse_tool_json(active);
        assert_eq!(active_payload["action"], "dep.diagnostics");
        assert_eq!(active_payload["graphScope"], "active");
        assert_eq!(active_payload["integrity"]["hasCycle"], false);
        assert_eq!(active_payload["integrity"]["cyclePath"], Value::Null);

        let full = call_dep_diagnostics(
            &config,
            DepDiagnosticsTool {
                issues_path: None,
                graph_scope: DepGraphScopeToolArg::Full,
            },
        )
        .expect("full diagnostics should succeed");
        let full_payload = parse_tool_json(full);
        assert_eq!(full_payload["graphScope"], "full");
        assert_eq!(full_payload["integrity"]["hasCycle"], true);
        assert_eq!(
            full_payload["integrity"]["cyclePath"],
            json!(["bd-a", "bd-b", "bd-a"])
        );
    }

    #[test]
    fn issue_backend_status_reports_integration_state() {
        let root = temp_dir("issue-backend-status");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Root".to_string(),
                id: Some("bd-root".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("seed issue should add");

        let projection_path = root.join(".premath").join("surreal_issue_cache.json");
        let store = load_store_existing(&issues).expect("issues should load");
        write_issue_query_projection(&projection_path, &issues, &store)
            .expect("projection should write");

        let result =
            call_issue_backend_status(&config, IssueBackendStatusTool { issues_path: None })
                .expect("backend status should succeed");
        let payload = parse_tool_json(result);
        assert_eq!(payload["action"], "issue.backend-status");
        assert_eq!(payload["queryBackend"], "jsonl");
        assert_eq!(payload["canonicalMemory"]["kind"], "jsonl");
        assert_eq!(payload["canonicalMemory"]["exists"], true);
        assert_eq!(
            payload["queryProjection"]["kind"],
            "premath.surreal.issue_projection.v0"
        );
        assert_eq!(payload["queryProjection"]["exists"], true);
        assert_eq!(payload["queryProjection"]["state"], "fresh");
        assert_eq!(
            payload["queryProjection"]["sourcePathMatchesAuthority"],
            true
        );
        assert_eq!(
            payload["queryProjection"]["snapshotRefMatchesAuthority"],
            true
        );
        assert_eq!(
            payload["queryProjection"]["snapshotRefMatchesProjection"],
            true
        );
        assert!(payload["canonicalMemory"]["snapshotRef"].is_string());
        assert!(payload["queryProjection"]["sourceSnapshotRef"].is_string());
        assert!(payload["jj"]["available"].is_boolean());
        let jj_state = payload["jj"]["state"]
            .as_str()
            .expect("jj.state should be a string");
        assert!(jj_state == "ready" || jj_state == "error" || jj_state == "unavailable");
    }

    #[test]
    fn issue_backend_status_reports_stale_projection_on_authority_change() {
        let root = temp_dir("issue-backend-status-stale");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Root".to_string(),
                id: Some("bd-root".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("seed issue should add");

        let projection_path = root.join(".premath").join("surreal_issue_cache.json");
        let store = load_store_existing(&issues).expect("issues should load");
        write_issue_query_projection(&projection_path, &issues, &store)
            .expect("projection should write");

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Second".to_string(),
                id: Some("bd-second".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("second issue should add");

        let result =
            call_issue_backend_status(&config, IssueBackendStatusTool { issues_path: None })
                .expect("backend status should succeed");
        let payload = parse_tool_json(result);
        assert_eq!(payload["queryProjection"]["state"], "stale");
        assert_eq!(
            payload["queryProjection"]["sourcePathMatchesAuthority"],
            true
        );
        assert_eq!(
            payload["queryProjection"]["snapshotRefMatchesAuthority"],
            false
        );
        assert_eq!(
            payload["queryProjection"]["snapshotRefMatchesProjection"],
            true
        );
    }

    #[test]
    fn issue_backend_status_marks_invalid_when_projection_payload_snapshot_mismatches() {
        let root = temp_dir("issue-backend-status-payload-mismatch");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Root".to_string(),
                id: Some("bd-root".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("seed issue should add");

        let projection_path = root.join(".premath").join("surreal_issue_cache.json");
        let store = load_store_existing(&issues).expect("issues should load");
        let projection_issues: Vec<Issue> = store.issues().cloned().collect();
        let invalid_payload = IssueQueryProjectionPayload {
            schema: ISSUE_QUERY_PROJECTION_SCHEMA,
            kind: ISSUE_QUERY_PROJECTION_KIND.to_string(),
            source_issues_path: issues.display().to_string(),
            source_snapshot_ref: Some("iss1_invalid_projection_ref".to_string()),
            generated_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            issue_count: projection_issues.len(),
            issues: projection_issues,
        };
        if let Some(parent) = projection_path.parent() {
            fs::create_dir_all(parent).expect("projection parent should exist");
        }
        fs::write(
            &projection_path,
            serde_json::to_vec_pretty(&invalid_payload).expect("projection should serialize"),
        )
        .expect("invalid projection should write");

        let result =
            call_issue_backend_status(&config, IssueBackendStatusTool { issues_path: None })
                .expect("backend status should succeed");
        let payload = parse_tool_json(result);
        assert_eq!(payload["queryProjection"]["state"], "invalid");
        assert_eq!(
            payload["queryProjection"]["snapshotRefMatchesProjection"],
            false
        );
        assert!(
            payload["queryProjection"]["error"]
                .as_str()
                .expect("projection error should be present")
                .contains("payload snapshot mismatch")
        );
    }

    #[test]
    fn init_tool_creates_and_is_idempotent() {
        let root = temp_dir("init-tool");
        let config = test_config(
            &root,
            &root.join(".premath/issues.jsonl"),
            &root.join("surface.json"),
        );

        let first = call_init_tool(&config, InitTool { root_path: None })
            .expect("first init should succeed");
        let first_payload = parse_tool_json(first);
        assert_eq!(first_payload["action"], "init.tool");
        assert_eq!(first_payload["createdPremathDir"], true);
        assert_eq!(first_payload["createdIssuesFile"], true);

        let second = call_init_tool(&config, InitTool { root_path: None })
            .expect("second init should succeed");
        let second_payload = parse_tool_json(second);
        assert_eq!(second_payload["createdPremathDir"], false);
        assert_eq!(second_payload["createdIssuesFile"], false);
    }

    #[test]
    fn observe_latest_tool_reads_surface() {
        let root = temp_dir("observe-latest");
        let surface = root.join("surface.json");
        let payload = json!({
            "schema": 1,
            "surfaceKind": "ci.observation.surface.v0",
            "summary": {
                "state": "accepted",
                "needsAttention": false,
                "topFailureClass": null,
                "latestProjectionDigest": "proj1",
                "latestInstructionId": "instr1",
                "requiredCheckCount": 1,
                "executedCheckCount": 1,
                "changedPathCount": 1
            },
            "latest": {
                "delta": null,
                "required": null,
                "decision": null
            },
            "instructions": []
        });
        fs::write(
            &surface,
            serde_json::to_vec_pretty(&payload).expect("surface should serialize"),
        )
        .expect("surface should write");

        let config = test_config(&root, &root.join("issues.jsonl"), &surface);

        let result = call_observe_latest(&config, ObserveLatestTool { surface_path: None })
            .expect("observe latest should succeed");
        let value = parse_tool_json(result);
        assert_eq!(value["action"], "observe.latest");
        assert_eq!(value["view"]["summary"]["state"], "accepted");
    }

    #[test]
    fn mutation_policy_instruction_linked_requires_instruction_id() {
        let root = temp_dir("mutation-policy-requires-id");
        let issues = root.join("issues.jsonl");
        let mut config = test_config(&root, &issues, &root.join("surface.json"));
        config.mutation_policy = MutationPolicy::InstructionLinked;

        let err = call_issue_add(
            &config,
            IssueAddTool {
                title: "Needs instruction".to_string(),
                id: None,
                description: None,
                status: None,
                priority: None,
                issue_type: None,
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect_err("mutation without instruction id should fail");
        assert!(
            err.to_string().contains("requires `instruction_id`"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn mutation_policy_instruction_linked_accepts_valid_witness() {
        let root = temp_dir("mutation-policy-valid");
        let issues = root.join("issues.jsonl");
        let witness_dir = root.join("artifacts").join("ciwitness");
        fs::create_dir_all(&witness_dir).expect("witness dir should exist");
        let instruction_id = "20260221T235959Z-test";
        let witness_path = witness_dir.join(format!("{instruction_id}.json"));
        let witness = json!({
            "instructionId": instruction_id,
            "instructionDigest": "instr1_test_digest",
            "policyDigest": "pol1_1ab3e7f398a472c2cf0f3fbd7ead7ece7bd74e836cbde924f1e33f02895d18ab",
            "capabilityClaims": [
                "capabilities.change_morphisms",
                "capabilities.change_morphisms.issue_add"
            ],
            "requiredChecks": ["hk-check"],
            "executedChecks": ["hk-check"],
            "verdictClass": "accepted"
        });
        fs::write(
            &witness_path,
            serde_json::to_vec_pretty(&witness).expect("witness json should serialize"),
        )
        .expect("witness should write");

        let mut config = test_config(&root, &issues, &root.join("surface.json"));
        config.mutation_policy = MutationPolicy::InstructionLinked;

        let result = call_issue_add(
            &config,
            IssueAddTool {
                title: "Instruction-linked write".to_string(),
                id: Some("bd-1".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: Some(instruction_id.to_string()),
                issues_path: None,
            },
        )
        .expect("instruction-linked mutation should succeed");
        let payload = parse_tool_json(result);
        assert_eq!(payload["instruction"]["instructionId"], instruction_id);
        assert_eq!(payload["writeWitness"]["action"], "issue.add");
    }

    #[test]
    fn mutation_policy_instruction_linked_rejects_unscoped_policy_digest() {
        let root = temp_dir("mutation-policy-unscoped-policy");
        let issues = root.join("issues.jsonl");
        let witness_dir = root.join("artifacts").join("ciwitness");
        fs::create_dir_all(&witness_dir).expect("witness dir should exist");
        let instruction_id = "20260222T000100Z-unscoped";
        let witness_path = witness_dir.join(format!("{instruction_id}.json"));
        let witness = json!({
            "instructionId": instruction_id,
            "instructionDigest": "instr1_test_digest",
            "policyDigest": "pol1_23a57a68a45e0c428868cce4b657206fc0bf100f4fd5b303eb0034ff29d92c9f",
            "capabilityClaims": [
                "capabilities.change_morphisms",
                "capabilities.change_morphisms.issue_add"
            ],
            "requiredChecks": ["ci-wiring-check"],
            "executedChecks": ["ci-wiring-check"],
            "verdictClass": "accepted"
        });
        fs::write(
            &witness_path,
            serde_json::to_vec_pretty(&witness).expect("witness json should serialize"),
        )
        .expect("witness should write");

        let mut config = test_config(&root, &issues, &root.join("surface.json"));
        config.mutation_policy = MutationPolicy::InstructionLinked;

        let err = call_issue_add(
            &config,
            IssueAddTool {
                title: "Should fail policy scope".to_string(),
                id: Some("bd-1".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: Some(instruction_id.to_string()),
                issues_path: None,
            },
        )
        .expect_err("mutation should fail for unscoped policy digest");
        assert!(
            err.to_string().contains("not scoped for mutation action"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn mutation_policy_instruction_linked_rejects_missing_action_capability_claim() {
        let root = temp_dir("mutation-policy-missing-action-claim");
        let issues = root.join("issues.jsonl");
        let witness_dir = root.join("artifacts").join("ciwitness");
        fs::create_dir_all(&witness_dir).expect("witness dir should exist");
        let instruction_id = "20260222T000200Z-missing-claim";
        let witness_path = witness_dir.join(format!("{instruction_id}.json"));
        let witness = json!({
            "instructionId": instruction_id,
            "instructionDigest": "instr1_test_digest",
            "policyDigest": "pol1_4ba916ce38da5c5607eb7f41d963294b34b644deb1fa6d55e133b072ca001b39",
            "capabilityClaims": [
                "capabilities.change_morphisms"
            ],
            "requiredChecks": ["hk-check"],
            "executedChecks": ["hk-check"],
            "verdictClass": "accepted"
        });
        fs::write(
            &witness_path,
            serde_json::to_vec_pretty(&witness).expect("witness json should serialize"),
        )
        .expect("witness should write");

        let mut config = test_config(&root, &issues, &root.join("surface.json"));
        config.mutation_policy = MutationPolicy::InstructionLinked;

        let err = call_issue_add(
            &config,
            IssueAddTool {
                title: "Should fail missing action claim".to_string(),
                id: Some("bd-1".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: Some(instruction_id.to_string()),
                issues_path: None,
            },
        )
        .expect_err("mutation should fail for missing action capability claim");
        assert!(
            err.to_string()
                .contains("missing required action capability claim"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn mutation_policy_instruction_linked_dep_add_requires_dep_action_claim() {
        let root = temp_dir("mutation-policy-dep-add-action-claim");
        let issues = root.join("issues.jsonl");
        let witness_dir = root.join("artifacts").join("ciwitness");
        fs::create_dir_all(&witness_dir).expect("witness dir should exist");
        let instruction_id = "20260222T000300Z-dep-action-claim";
        let witness_path = witness_dir.join(format!("{instruction_id}.json"));
        let witness = json!({
            "instructionId": instruction_id,
            "instructionDigest": "instr1_test_digest",
            "policyDigest": "pol1_4ba916ce38da5c5607eb7f41d963294b34b644deb1fa6d55e133b072ca001b39",
            "capabilityClaims": [
                "capabilities.change_morphisms",
                "capabilities.change_morphisms.issue_add"
            ],
            "requiredChecks": ["hk-check"],
            "executedChecks": ["hk-check"],
            "verdictClass": "accepted"
        });
        fs::write(
            &witness_path,
            serde_json::to_vec_pretty(&witness).expect("witness json should serialize"),
        )
        .expect("witness should write");

        let mut config = test_config(&root, &issues, &root.join("surface.json"));
        config.mutation_policy = MutationPolicy::InstructionLinked;

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Left".to_string(),
                id: Some("bd-left".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: Some(instruction_id.to_string()),
                issues_path: None,
            },
        )
        .expect("issue add should be allowed with issue_add action claim");

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Right".to_string(),
                id: Some("bd-right".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: Some(instruction_id.to_string()),
                issues_path: None,
            },
        )
        .expect("issue add should be allowed with issue_add action claim");

        let err = call_dep_add(
            &config,
            DepAddTool {
                issue_id: "bd-right".to_string(),
                depends_on_id: "bd-left".to_string(),
                dep_type: Some("blocks".to_string()),
                created_by: None,
                instruction_id: Some(instruction_id.to_string()),
                issues_path: None,
            },
        )
        .expect_err("dep add should require dep_add action claim");
        assert!(
            err.to_string()
                .contains("missing required action capability claim"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn surreal_issue_backend_uses_projection_source() {
        let root = temp_dir("surreal-projection");
        let issues = root.join("issues.jsonl");
        let mut config = test_config(&root, &issues, &root.join("surface.json"));
        config.issue_query_backend = IssueQueryBackend::Surreal;

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Root".to_string(),
                id: Some("bd-root".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(1),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("root issue should add");

        let ready = call_issue_ready(&config, IssueReadyTool { issues_path: None })
            .expect("ready query should succeed");
        let payload = parse_tool_json(ready);
        assert_eq!(payload["queryBackend"], "surreal");
        assert_eq!(payload["querySource"], "surreal-projection");
    }

    #[test]
    fn surreal_issue_backend_refreshes_when_projection_source_path_mismatches() {
        let root = temp_dir("surreal-projection-source-mismatch");
        let issues = root.join("issues.jsonl");
        let mut config = test_config(&root, &issues, &root.join("surface.json"));
        config.issue_query_backend = IssueQueryBackend::Surreal;

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Root".to_string(),
                id: Some("bd-root".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(1),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("root issue should add");

        let projection_path = resolve_issue_query_projection_path(&config);
        if let Some(parent) = projection_path.parent() {
            fs::create_dir_all(parent).expect("projection parent should exist");
        }
        let stale_payload = IssueQueryProjectionPayload {
            schema: ISSUE_QUERY_PROJECTION_SCHEMA,
            kind: ISSUE_QUERY_PROJECTION_KIND.to_string(),
            source_issues_path: root.join("other-issues.jsonl").display().to_string(),
            source_snapshot_ref: None,
            generated_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            issue_count: 0,
            issues: Vec::new(),
        };
        fs::write(
            &projection_path,
            serde_json::to_vec_pretty(&stale_payload).expect("projection should serialize"),
        )
        .expect("stale projection should write");

        let ready = call_issue_ready(&config, IssueReadyTool { issues_path: None })
            .expect("ready query should succeed");
        let payload = parse_tool_json(ready);
        assert_eq!(payload["queryBackend"], "surreal");
        assert_eq!(payload["querySource"], "surreal-projection");
        assert_eq!(payload["count"], 1);
        assert_eq!(payload["items"][0]["id"], "bd-root");

        let refreshed_bytes = fs::read(&projection_path).expect("projection should be refreshed");
        let refreshed: IssueQueryProjectionPayload =
            serde_json::from_slice(&refreshed_bytes).expect("projection should parse");
        assert_eq!(refreshed.source_issues_path, issues.display().to_string());
        assert_eq!(refreshed.issue_count, 1);
    }

    #[test]
    fn surreal_issue_backend_refreshes_when_projection_missing_source_snapshot_ref() {
        let root = temp_dir("surreal-projection-missing-source-snapshot-ref");
        let issues = root.join("issues.jsonl");
        let mut config = test_config(&root, &issues, &root.join("surface.json"));
        config.issue_query_backend = IssueQueryBackend::Surreal;

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Root".to_string(),
                id: Some("bd-root".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(1),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("root issue should add");

        let projection_path = resolve_issue_query_projection_path(&config);
        if let Some(parent) = projection_path.parent() {
            fs::create_dir_all(parent).expect("projection parent should exist");
        }
        let authority = load_store_existing(&issues).expect("authority should load");
        let projection_issues: Vec<Issue> = authority.issues().cloned().collect();
        let legacy_payload = IssueQueryProjectionPayload {
            schema: ISSUE_QUERY_PROJECTION_SCHEMA,
            kind: ISSUE_QUERY_PROJECTION_KIND.to_string(),
            source_issues_path: issues.display().to_string(),
            source_snapshot_ref: None,
            generated_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            issue_count: projection_issues.len(),
            issues: projection_issues,
        };
        fs::write(
            &projection_path,
            serde_json::to_vec_pretty(&legacy_payload).expect("projection should serialize"),
        )
        .expect("legacy projection should write");

        let ready = call_issue_ready(&config, IssueReadyTool { issues_path: None })
            .expect("ready query should succeed");
        let payload = parse_tool_json(ready);
        assert_eq!(payload["queryBackend"], "surreal");
        assert_eq!(payload["querySource"], "surreal-projection");
        assert_eq!(payload["count"], 1);
        assert_eq!(payload["items"][0]["id"], "bd-root");

        let refreshed_bytes = fs::read(&projection_path).expect("projection should be refreshed");
        let refreshed: IssueQueryProjectionPayload =
            serde_json::from_slice(&refreshed_bytes).expect("projection should parse");
        assert_eq!(refreshed.source_issues_path, issues.display().to_string());
        assert_eq!(refreshed.issue_count, 1);
        assert!(refreshed.source_snapshot_ref.is_some());
    }

    #[test]
    fn surreal_issue_backend_refreshes_when_projection_snapshot_ref_mismatches() {
        let root = temp_dir("surreal-projection-snapshot-mismatch");
        let issues = root.join("issues.jsonl");
        let mut config = test_config(&root, &issues, &root.join("surface.json"));
        config.issue_query_backend = IssueQueryBackend::Surreal;

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Root".to_string(),
                id: Some("bd-root".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(1),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("root issue should add");

        let projection_path = resolve_issue_query_projection_path(&config);
        if let Some(parent) = projection_path.parent() {
            fs::create_dir_all(parent).expect("projection parent should exist");
        }

        let authority = load_store_existing(&issues).expect("authority should load");
        let projection_issues: Vec<Issue> = authority.issues().cloned().collect();
        let stale_payload = IssueQueryProjectionPayload {
            schema: ISSUE_QUERY_PROJECTION_SCHEMA,
            kind: ISSUE_QUERY_PROJECTION_KIND.to_string(),
            source_issues_path: issues.display().to_string(),
            source_snapshot_ref: Some("iss1_invalid_projection_ref".to_string()),
            generated_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            issue_count: projection_issues.len(),
            issues: projection_issues,
        };
        fs::write(
            &projection_path,
            serde_json::to_vec_pretty(&stale_payload).expect("projection should serialize"),
        )
        .expect("stale projection should write");

        let ready = call_issue_ready(&config, IssueReadyTool { issues_path: None })
            .expect("ready query should succeed");
        let payload = parse_tool_json(ready);
        assert_eq!(payload["queryBackend"], "surreal");
        assert_eq!(payload["querySource"], "surreal-projection");
        assert_eq!(payload["count"], 1);
        assert_eq!(payload["items"][0]["id"], "bd-root");

        let refreshed_bytes = fs::read(&projection_path).expect("projection should be refreshed");
        let refreshed: IssueQueryProjectionPayload =
            serde_json::from_slice(&refreshed_bytes).expect("projection should parse");
        let expected_ref = store_snapshot_ref(&authority);
        assert_eq!(refreshed.source_issues_path, issues.display().to_string());
        assert_eq!(refreshed.issue_count, 1);
        assert_eq!(refreshed.source_snapshot_ref, Some(expected_ref));
    }

    #[test]
    fn issue_claim_rejects_conflicting_assignee() {
        let root = temp_dir("issue-claim-conflict");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Claim target".to_string(),
                id: Some("bd-claim".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: Some("alice".to_string()),
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("issue should add");

        let err = call_issue_claim(
            &config,
            IssueClaimTool {
                id: "bd-claim".to_string(),
                assignee: "bob".to_string(),
                lease_id: None,
                lease_ttl_seconds: None,
                lease_expires_at: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect_err("conflicting claim should fail");
        assert!(
            err.to_string().contains("already claimed"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn issue_claim_requires_instruction_id_when_policy_linked() {
        let root = temp_dir("issue-claim-policy-linked");
        let issues = root.join("issues.jsonl");
        let mut config = test_config(&root, &issues, &root.join("surface.json"));
        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Claim target".to_string(),
                id: Some("bd-claim-policy".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("seed issue should add in open mode");
        config.mutation_policy = MutationPolicy::InstructionLinked;

        let err = call_issue_claim(
            &config,
            IssueClaimTool {
                id: "bd-claim-policy".to_string(),
                assignee: "alice".to_string(),
                lease_id: None,
                lease_ttl_seconds: None,
                lease_expires_at: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect_err("claim without instruction id should fail");
        assert!(
            err.to_string().contains("requires `instruction_id`"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn issue_claim_assigns_active_lease() {
        let root = temp_dir("issue-claim-lease");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Lease target".to_string(),
                id: Some("bd-lease".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("seed issue should add");

        let claim = call_issue_claim(
            &config,
            IssueClaimTool {
                id: "bd-lease".to_string(),
                assignee: "alice".to_string(),
                lease_id: None,
                lease_ttl_seconds: Some(120),
                lease_expires_at: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("claim should succeed");

        let payload = parse_tool_json(claim);
        assert_eq!(payload["issue"]["status"], "in_progress");
        assert_eq!(payload["issue"]["assignee"], "alice");
        assert_eq!(payload["issue"]["lease"]["owner"], "alice");
        assert_eq!(payload["issue"]["lease"]["state"], "active");
        assert_eq!(payload["leaseProjection"]["staleCount"], 0);
        assert_eq!(payload["leaseProjection"]["contendedCount"], 0);
    }

    #[test]
    fn issue_claim_reclaims_stale_lease_and_rebinds_assignee() {
        let root = temp_dir("issue-claim-reclaim-stale");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Stale claim target".to_string(),
                id: Some("bd-stale-claim".to_string()),
                description: None,
                status: Some("in_progress".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: Some("alice".to_string()),
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("seed issue should add");

        let mut store = MemoryStore::load_jsonl(&issues).expect("store should load");
        {
            let issue = store
                .issue_mut("bd-stale-claim")
                .expect("stale claim issue must exist");
            issue.lease = Some(IssueLease {
                lease_id: "lease1_bd-stale-claim_alice".to_string(),
                owner: "alice".to_string(),
                acquired_at: Utc::now() - Duration::seconds(120),
                expires_at: Utc::now() - Duration::seconds(30),
                renewed_at: None,
            });
            issue.touch_updated_at();
        }
        store.save_jsonl(&issues).expect("store should save");

        let claim = call_issue_claim(
            &config,
            IssueClaimTool {
                id: "bd-stale-claim".to_string(),
                assignee: "bob".to_string(),
                lease_id: Some("lease1_bd-stale-claim_bob".to_string()),
                lease_ttl_seconds: Some(120),
                lease_expires_at: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("claim should reclaim stale lease");

        let payload = parse_tool_json(claim);
        assert_eq!(payload["issue"]["status"], "in_progress");
        assert_eq!(payload["issue"]["assignee"], "bob");
        assert_eq!(payload["issue"]["lease"]["owner"], "bob");
        assert_eq!(payload["issue"]["lease"]["state"], "active");
        assert_eq!(payload["leaseProjection"]["staleCount"], 0);
        assert_eq!(payload["leaseProjection"]["contendedCount"], 0);
    }

    #[test]
    fn issue_claim_rejects_invalid_lease_ttl() {
        let root = temp_dir("issue-claim-invalid-ttl");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Invalid ttl target".to_string(),
                id: Some("bd-invalid-ttl".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("seed issue should add");

        let err = call_issue_claim(
            &config,
            IssueClaimTool {
                id: "bd-invalid-ttl".to_string(),
                assignee: "alice".to_string(),
                lease_id: None,
                lease_ttl_seconds: Some(10),
                lease_expires_at: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect_err("invalid ttl should fail");
        assert!(
            err.to_string().contains("[failureClass=lease_invalid_ttl]"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn issue_claim_rejects_non_future_expiry() {
        let root = temp_dir("issue-claim-invalid-expiry");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Invalid expiry target".to_string(),
                id: Some("bd-invalid-expiry".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("seed issue should add");

        let err = call_issue_claim(
            &config,
            IssueClaimTool {
                id: "bd-invalid-expiry".to_string(),
                assignee: "alice".to_string(),
                lease_id: None,
                lease_ttl_seconds: None,
                lease_expires_at: Some((Utc::now() - Duration::seconds(5)).to_rfc3339()),
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect_err("non-future expiry should fail");
        assert!(
            err.to_string()
                .contains("[failureClass=lease_invalid_expires_at]"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn issue_lease_renew_rejects_stale_lease() {
        let root = temp_dir("issue-lease-renew-stale");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Renew target".to_string(),
                id: Some("bd-renew".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("seed issue should add");

        let claim = call_issue_claim(
            &config,
            IssueClaimTool {
                id: "bd-renew".to_string(),
                assignee: "alice".to_string(),
                lease_id: Some("lease1_bd-renew_alice".to_string()),
                lease_ttl_seconds: Some(300),
                lease_expires_at: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("claim should succeed");
        let claimed_payload = parse_tool_json(claim);
        let lease_id = claimed_payload["issue"]["lease"]["leaseId"]
            .as_str()
            .expect("lease id must be present")
            .to_string();

        let mut store = MemoryStore::load_jsonl(&issues).expect("store should load");
        {
            let issue = store
                .issue_mut("bd-renew")
                .expect("renew issue must exist in store");
            let mut lease = issue.lease.clone().expect("lease must be present");
            lease.expires_at = Utc::now() - Duration::seconds(30);
            issue.lease = Some(lease);
            issue.touch_updated_at();
        }
        store.save_jsonl(&issues).expect("store should save");

        let err = call_issue_lease_renew(
            &config,
            IssueLeaseRenewTool {
                id: "bd-renew".to_string(),
                assignee: "alice".to_string(),
                lease_id,
                lease_ttl_seconds: Some(120),
                lease_expires_at: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect_err("stale lease renew should fail");
        assert!(
            err.to_string().contains("[failureClass=lease_stale]"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn issue_lease_release_reopens_and_clears_assignee() {
        let root = temp_dir("issue-lease-release");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Release target".to_string(),
                id: Some("bd-release".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("seed issue should add");

        let claim = call_issue_claim(
            &config,
            IssueClaimTool {
                id: "bd-release".to_string(),
                assignee: "alice".to_string(),
                lease_id: Some("lease1_bd-release_alice".to_string()),
                lease_ttl_seconds: Some(120),
                lease_expires_at: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("claim should succeed");
        let claimed_payload = parse_tool_json(claim);
        let lease_id = claimed_payload["issue"]["lease"]["leaseId"]
            .as_str()
            .expect("lease id must be present")
            .to_string();

        let release = call_issue_lease_release(
            &config,
            IssueLeaseReleaseTool {
                id: "bd-release".to_string(),
                assignee: Some("alice".to_string()),
                lease_id: Some(lease_id),
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("release should succeed");

        let payload = parse_tool_json(release);
        assert_eq!(payload["issue"]["status"], "open");
        assert_eq!(payload["issue"]["assignee"], "");
        assert_eq!(payload["issue"]["lease"], Value::Null);
    }

    #[test]
    fn issue_lease_projection_reports_stale_and_contended() {
        let root = temp_dir("issue-lease-projection");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Stale".to_string(),
                id: Some("bd-stale".to_string()),
                description: None,
                status: Some("in_progress".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: Some("alice".to_string()),
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("stale issue should add");

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Contended".to_string(),
                id: Some("bd-contended".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: Some("alice".to_string()),
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("contended issue should add");

        let mut store = MemoryStore::load_jsonl(&issues).expect("store should load");
        {
            let stale_issue = store
                .issue_mut("bd-stale")
                .expect("stale issue must exist in store");
            stale_issue.lease = Some(IssueLease {
                lease_id: "lease1_bd-stale_alice".to_string(),
                owner: "alice".to_string(),
                acquired_at: Utc::now() - Duration::seconds(90),
                expires_at: Utc::now() - Duration::seconds(30),
                renewed_at: None,
            });
            stale_issue.touch_updated_at();
        }
        {
            let contended_issue = store
                .issue_mut("bd-contended")
                .expect("contended issue must exist in store");
            contended_issue.lease = Some(IssueLease {
                lease_id: "lease1_bd-contended_bob".to_string(),
                owner: "bob".to_string(),
                acquired_at: Utc::now() - Duration::seconds(30),
                expires_at: Utc::now() + Duration::seconds(300),
                renewed_at: None,
            });
            contended_issue.touch_updated_at();
        }
        store.save_jsonl(&issues).expect("store should save");

        let projection =
            call_issue_lease_projection(&config, IssueLeaseProjectionTool { issues_path: None })
                .expect("lease projection should succeed");
        let payload = parse_tool_json(projection);
        assert_eq!(payload["projection"]["staleCount"], 1);
        assert_eq!(payload["projection"]["contendedCount"], 1);
    }

    #[test]
    fn issue_discover_creates_issue_and_discovered_from_dependency() {
        let root = temp_dir("issue-discover");
        let issues = root.join("issues.jsonl");
        let config = test_config(&root, &issues, &root.join("surface.json"));

        let _ = call_issue_add(
            &config,
            IssueAddTool {
                title: "Parent".to_string(),
                id: Some("bd-parent".to_string()),
                description: None,
                status: Some("open".to_string()),
                priority: Some(1),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("parent should add");

        let discover = call_issue_discover(
            &config,
            IssueDiscoverTool {
                parent_issue_id: "bd-parent".to_string(),
                title: "Found while working".to_string(),
                id: Some("bd-found".to_string()),
                description: Some("new follow-up".to_string()),
                priority: Some(2),
                issue_type: Some("task".to_string()),
                assignee: None,
                owner: None,
                instruction_id: None,
                issues_path: None,
            },
        )
        .expect("discover should succeed");

        let payload = parse_tool_json(discover);
        assert_eq!(payload["action"], "issue.discover");
        assert_eq!(payload["issue"]["id"], "bd-found");
        assert_eq!(payload["dependency"]["type"], "discovered-from");
        assert_eq!(payload["dependency"]["dependsOnId"], "bd-parent");

        let list = call_issue_list(
            &config,
            IssueListTool {
                status: None,
                assignee: None,
                issues_path: None,
            },
        )
        .expect("list should succeed");
        let list_payload = parse_tool_json(list);
        assert_eq!(list_payload["count"], 2);
    }

    #[test]
    fn instruction_check_tool_validates_fixture() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("repo root should resolve")
            .to_path_buf();
        let config = test_config(
            &repo_root,
            &repo_root.join(".premath/issues.jsonl"),
            &repo_root.join("artifacts/observation/latest.json"),
        );

        let result = call_instruction_check(
            &config,
            InstructionCheckTool {
                instruction_path:
                    "tests/ci/fixtures/instructions/20260221T010000Z-ci-wiring-golden.json"
                        .to_string(),
            },
        )
        .expect("instruction check tool should execute");

        let value = parse_tool_json(result);
        assert_eq!(value["action"], "instruction.check");
        assert_eq!(value["ok"], true);
    }
}
