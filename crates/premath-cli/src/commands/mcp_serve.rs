use async_trait::async_trait;
use premath_bd::{DepType, Issue, MemoryStore};
use premath_surreal::QueryCache;
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

pub struct Args {
    pub issues: String,
    pub surface: String,
    pub repo_root: String,
    pub server_name: String,
    pub server_version: String,
}

#[derive(Debug, Clone)]
struct PremathMcpConfig {
    issues_path: String,
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
    let config = PremathMcpConfig {
        issues_path: args.issues,
        surface_path: args.surface,
        repo_root: args.repo_root,
    };

    let server_details = InitializeResult {
        server_info: Implementation {
            name: args.server_name.into(),
            version: args.server_version.into(),
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
            "Use instruction_* tools for doctrine-gated runs. Use issue/dep and observe_* tools for data/observation reads."
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
            PremathTools::IssueAddTool(tool) => call_issue_add(&self.config, tool),
            PremathTools::IssueUpdateTool(tool) => call_issue_update(&self.config, tool),
            PremathTools::DepAddTool(tool) => call_dep_add(&self.config, tool),
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
    issues_path: Option<String>,
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
    surface_path: Option<String>,
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
        IssueAddTool,
        IssueUpdateTool,
        DepAddTool,
        ObserveLatestTool,
        ObserveNeedsAttentionTool,
        ObserveInstructionTool,
        ObserveProjectionTool,
        InstructionCheckTool,
        InstructionRunTool
    ]
);

fn call_issue_ready(
    config: &PremathMcpConfig,
    tool: IssueReadyTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let store = load_store_existing(&path)?;
    let cache = QueryCache::hydrate(&store);
    let ids = cache.ready_open_issue_ids();

    let items = ids
        .iter()
        .filter_map(|id| cache.issue(id))
        .map(|issue| {
            json!({
                "id": issue.id,
                "title": issue.title,
                "status": issue.status,
                "priority": issue.priority
            })
        })
        .collect::<Vec<_>>();

    json_result(json!({
        "action": "issue.ready",
        "issuesPath": path.display().to_string(),
        "count": items.len(),
        "items": items
    }))
}

fn call_issue_list(
    config: &PremathMcpConfig,
    tool: IssueListTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let store = load_store_existing(&path)?;
    let status = non_empty(tool.status);
    let assignee = non_empty(tool.assignee);

    let items = store
        .issues()
        .filter(|issue| status.as_ref().is_none_or(|s| issue.status == *s))
        .filter(|issue| assignee.as_ref().is_none_or(|a| issue.assignee == *a))
        .map(|issue| {
            json!({
                "id": issue.id,
                "title": issue.title,
                "status": issue.status,
                "priority": issue.priority,
                "issueType": issue.issue_type,
                "assignee": issue.assignee,
                "owner": issue.owner
            })
        })
        .collect::<Vec<_>>();

    json_result(json!({
        "action": "issue.list",
        "issuesPath": path.display().to_string(),
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

    if store.issue(&issue_id).is_some() {
        return Err(call_tool_error(format!("issue already exists: {issue_id}")));
    }

    let mut issue = Issue::new(issue_id.clone(), tool.title);
    issue.description = tool.description.unwrap_or_default();
    issue.priority = tool.priority.unwrap_or(2);
    issue.issue_type = non_empty(tool.issue_type).unwrap_or_else(|| "task".to_string());
    issue.assignee = tool.assignee.unwrap_or_default();
    issue.owner = tool.owner.unwrap_or_default();
    issue.set_status(non_empty(tool.status).unwrap_or_else(|| "open".to_string()));
    let persisted = issue.clone();

    store.upsert_issue(issue);
    save_store(&store, &path)?;

    json_result(json!({
        "action": "issue.add",
        "issuesPath": path.display().to_string(),
        "issue": {
            "id": persisted.id,
            "title": persisted.title,
            "status": persisted.status,
            "priority": persisted.priority,
            "issueType": persisted.issue_type,
            "assignee": persisted.assignee,
            "owner": persisted.owner
        }
    }))
}

fn call_issue_update(
    config: &PremathMcpConfig,
    tool: IssueUpdateTool,
) -> std::result::Result<CallToolResult, CallToolError> {
    let path = resolve_path(tool.issues_path, &config.issues_path);
    let mut store = load_store_existing(&path)?;

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

        issue.clone()
    };

    save_store(&store, &path)?;

    json_result(json!({
        "action": "issue.update",
        "issuesPath": path.display().to_string(),
        "issue": {
            "id": updated.id,
            "title": updated.title,
            "status": updated.status,
            "priority": updated.priority,
            "issueType": updated.issue_type,
            "assignee": updated.assignee,
            "owner": updated.owner
        }
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

    store
        .add_dependency(
            &tool.issue_id,
            &tool.depends_on_id,
            dep_type.clone(),
            created_by.clone(),
        )
        .map_err(|e| call_tool_error(format!("failed to add dependency: {e}")))?;
    save_store(&store, &path)?;

    json_result(json!({
        "action": "dep.add",
        "issuesPath": path.display().to_string(),
        "dependency": {
            "issueId": tool.issue_id,
            "dependsOnId": tool.depends_on_id,
            "type": dep_type.as_str(),
            "createdBy": created_by
        }
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

    let output = Command::new("python3")
        .arg("tools/ci/check_instruction_envelope.py")
        .arg(&instruction_path)
        .arg("--repo-root")
        .arg(&repo_root)
        .current_dir(&repo_root)
        .output()
        .map_err(|e| call_tool_error(format!("failed to execute instruction check: {e}")))?;

    let payload = json!({
        "action": "instruction.check",
        "repoRoot": repo_root.display().to_string(),
        "instructionPath": instruction_path.display().to_string(),
        "ok": output.status.success(),
        "exitCode": output.status.code(),
        "stdout": truncate_for_payload(&String::from_utf8_lossy(&output.stdout), 16_000),
        "stderr": truncate_for_payload(&String::from_utf8_lossy(&output.stderr), 16_000)
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

    #[test]
    fn issue_tools_roundtrip_over_jsonl() {
        let root = temp_dir("issue-roundtrip");
        let issues = root.join("issues.jsonl");
        let config = PremathMcpConfig {
            issues_path: issues.display().to_string(),
            surface_path: root.join("surface.json").display().to_string(),
            repo_root: root.display().to_string(),
        };

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
                issues_path: None,
            },
        )
        .expect("dependency should be added");

        let ready = call_issue_ready(&config, IssueReadyTool { issues_path: None })
            .expect("ready query should succeed");
        let payload = parse_tool_json(ready);
        assert_eq!(payload["count"], 1);
        assert_eq!(payload["items"][0]["id"], "bd-root");
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

        let config = PremathMcpConfig {
            issues_path: root.join("issues.jsonl").display().to_string(),
            surface_path: surface.display().to_string(),
            repo_root: root.display().to_string(),
        };

        let result = call_observe_latest(&config, ObserveLatestTool { surface_path: None })
            .expect("observe latest should succeed");
        let value = parse_tool_json(result);
        assert_eq!(value["action"], "observe.latest");
        assert_eq!(value["view"]["summary"]["state"], "accepted");
    }

    #[test]
    fn instruction_check_tool_validates_fixture() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("repo root should resolve")
            .to_path_buf();
        let config = PremathMcpConfig {
            issues_path: repo_root.join(".beads/issues.jsonl").display().to_string(),
            surface_path: repo_root
                .join("artifacts/observation/latest.json")
                .display()
                .to_string(),
            repo_root: repo_root.display().to_string(),
        };

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
