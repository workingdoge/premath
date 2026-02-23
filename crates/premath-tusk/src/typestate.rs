use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

pub const TYPESTATE_NORMALIZED_SCHEMA: u64 = 1;
pub const TYPESTATE_NORMALIZED_KIND: &str = "premath.harness.typestate_normalized.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CallSpecInput {
    pub call_id: String,
    pub model_ref: String,
    pub action_mode: String,
    pub execution_pattern: String,
    pub normalizer_id: String,
    pub mutation_policy_digest: String,
    pub governance_policy_digest: String,
    pub tool_render_protocol_digest: String,
    pub reminder_queue_policy_digest: String,
    pub state_view_policy_digest: String,
    pub decomposition_policy_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequestInput {
    pub tool_call_id: String,
    pub tool_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caller_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_ref_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultInput {
    pub tool_call_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retryable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_error: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolUseInput {
    pub tool_call_id: String,
    pub disposition: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolStateInput {
    pub stop_reason: String,
    #[serde(default)]
    pub continuation_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HandoffObservationInput {
    pub target: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_artifact_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_path_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolRenderObservationInput {
    pub tool_call_id: String,
    pub operator_payload_digest: String,
    pub reminder_render_digest: String,
    pub injection_point: String,
    pub policy_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReminderQueueReductionInput {
    pub queue_id: String,
    pub reduced_digest: String,
    pub policy_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StateViewObservationInput {
    pub view_id: String,
    pub view_digest: String,
    pub policy_digest: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TypestateEvidenceInput {
    pub call_spec: CallSpecInput,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_requests: Vec<ToolRequestInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_results: Vec<ToolResultInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_use: Vec<ToolUseInput>,
    pub protocol_state: ProtocolStateInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff: Option<HandoffObservationInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_render: Vec<ToolRenderObservationInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reminder_queue: Vec<ReminderQueueReductionInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_views: Vec<StateViewObservationInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub session_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trajectory_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedCallSpec {
    pub call_id: String,
    pub model_ref: String,
    pub action_mode: String,
    pub execution_pattern: String,
    pub normalizer_id: String,
    pub mutation_policy_digest: String,
    pub governance_policy_digest: String,
    pub tool_render_protocol_digest: String,
    pub reminder_queue_policy_digest: String,
    pub state_view_policy_digest: String,
    pub decomposition_policy_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedToolRequest {
    pub tool_call_id: String,
    pub tool_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caller_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_ref_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedToolErrorEnvelope {
    pub error_code: String,
    pub retryable: bool,
    pub error_message_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedToolResult {
    pub tool_call_id: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<NormalizedToolErrorEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedToolUse {
    pub tool_call_id: String,
    pub disposition: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedProtocolState {
    pub stop_reason: String,
    pub continuation_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedHandoffObservation {
    pub target: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_artifact_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_path_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedToolRenderObservation {
    pub tool_call_id: String,
    pub operator_payload_digest: String,
    pub reminder_render_digest: String,
    pub injection_point: String,
    pub policy_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedReminderQueueReduction {
    pub queue_id: String,
    pub reduced_digest: String,
    pub policy_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedStateViewObservation {
    pub view_id: String,
    pub view_digest: String,
    pub policy_digest: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TypestateDigestBundle {
    pub call_spec_digest: String,
    pub request_set_digest: String,
    pub result_set_digest: String,
    pub use_set_digest: String,
    pub tool_render_set_digest: String,
    pub reminder_queue_set_digest: String,
    pub state_view_set_digest: String,
    pub protocol_state_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff_digest: Option<String>,
    pub join_set_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedJoinState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub result_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub used_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_result_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orphan_result_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_use_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknown_use_call_ids: Vec<String>,
    pub join_closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedContextState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub render_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub queue_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_view_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_render_call_ids: Vec<String>,
    pub queue_reduction_present: bool,
    pub state_view_present: bool,
    pub render_policy_valid: bool,
    pub queue_policy_valid: bool,
    pub state_view_policy_valid: bool,
    pub continuation_context_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedTypestateEvidence {
    pub schema: u64,
    pub kind: String,
    pub call_spec: NormalizedCallSpec,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_requests: Vec<NormalizedToolRequest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_results: Vec<NormalizedToolResult>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_use: Vec<NormalizedToolUse>,
    pub protocol_state: NormalizedProtocolState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff: Option<NormalizedHandoffObservation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_render: Vec<NormalizedToolRenderObservation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reminder_queue: Vec<NormalizedReminderQueueReduction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_views: Vec<NormalizedStateViewObservation>,
    pub digests: TypestateDigestBundle,
    pub join_state: NormalizedJoinState,
    pub context_state: NormalizedContextState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub session_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trajectory_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JoinClosedInput {
    pub request_set_digest: String,
    pub result_set_digest: String,
    pub use_set_digest: String,
    pub tool_render_set_digest: String,
    pub reminder_queue_set_digest: String,
    pub state_view_set_digest: String,
    pub protocol_state_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff_digest: Option<String>,
    pub join_set_digest: String,
    pub reminder_queue_policy_digest: String,
    pub tool_render_protocol_digest: String,
    pub state_view_policy_digest: String,
    pub decomposition_policy_digest: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_result_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orphan_result_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_use_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknown_use_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_render_call_ids: Vec<String>,
    pub queue_reduction_present: bool,
    pub state_view_present: bool,
    pub render_policy_valid: bool,
    pub queue_policy_valid: bool,
    pub state_view_policy_valid: bool,
    pub continuation_context_ready: bool,
    pub join_closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MutationReadyInput {
    pub join_closed: bool,
    pub continuation_allowed: bool,
    pub continuation_context_ready: bool,
    pub queue_reduction_present: bool,
    pub state_view_present: bool,
    pub render_policy_valid: bool,
    pub queue_policy_valid: bool,
    pub state_view_policy_valid: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_render_call_ids: Vec<String>,
    pub mutation_policy_digest: String,
    pub governance_policy_digest: String,
    pub normalizer_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub session_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trajectory_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_result_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orphan_result_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_use_call_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknown_use_call_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypestateNormalizationError {
    MissingField(&'static str),
    InvalidField {
        field: &'static str,
        detail: String,
    },
    DuplicateToolCallId {
        surface: &'static str,
        tool_call_id: String,
    },
}

impl Display for TypestateNormalizationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "{field} must be non-empty"),
            Self::InvalidField { field, detail } => write!(f, "{field} invalid: {detail}"),
            Self::DuplicateToolCallId {
                surface,
                tool_call_id,
            } => {
                write!(
                    f,
                    "{surface} contains conflicting duplicate toolCallId: {tool_call_id}"
                )
            }
        }
    }
}

impl Error for TypestateNormalizationError {}

impl NormalizedTypestateEvidence {
    pub fn join_closed_input(&self) -> JoinClosedInput {
        JoinClosedInput {
            request_set_digest: self.digests.request_set_digest.clone(),
            result_set_digest: self.digests.result_set_digest.clone(),
            use_set_digest: self.digests.use_set_digest.clone(),
            tool_render_set_digest: self.digests.tool_render_set_digest.clone(),
            reminder_queue_set_digest: self.digests.reminder_queue_set_digest.clone(),
            state_view_set_digest: self.digests.state_view_set_digest.clone(),
            protocol_state_digest: self.digests.protocol_state_digest.clone(),
            handoff_digest: self.digests.handoff_digest.clone(),
            join_set_digest: self.digests.join_set_digest.clone(),
            reminder_queue_policy_digest: self.call_spec.reminder_queue_policy_digest.clone(),
            tool_render_protocol_digest: self.call_spec.tool_render_protocol_digest.clone(),
            state_view_policy_digest: self.call_spec.state_view_policy_digest.clone(),
            decomposition_policy_digest: self.call_spec.decomposition_policy_digest.clone(),
            missing_result_call_ids: self.join_state.missing_result_call_ids.clone(),
            orphan_result_call_ids: self.join_state.orphan_result_call_ids.clone(),
            missing_use_call_ids: self.join_state.missing_use_call_ids.clone(),
            unknown_use_call_ids: self.join_state.unknown_use_call_ids.clone(),
            missing_render_call_ids: self.context_state.missing_render_call_ids.clone(),
            queue_reduction_present: self.context_state.queue_reduction_present,
            state_view_present: self.context_state.state_view_present,
            render_policy_valid: self.context_state.render_policy_valid,
            queue_policy_valid: self.context_state.queue_policy_valid,
            state_view_policy_valid: self.context_state.state_view_policy_valid,
            continuation_context_ready: self.context_state.continuation_context_ready,
            join_closed: self.join_state.join_closed,
        }
    }

    pub fn mutation_ready_input(&self) -> MutationReadyInput {
        MutationReadyInput {
            join_closed: self.join_state.join_closed,
            continuation_allowed: self.protocol_state.continuation_allowed,
            continuation_context_ready: self.context_state.continuation_context_ready,
            queue_reduction_present: self.context_state.queue_reduction_present,
            state_view_present: self.context_state.state_view_present,
            render_policy_valid: self.context_state.render_policy_valid,
            queue_policy_valid: self.context_state.queue_policy_valid,
            state_view_policy_valid: self.context_state.state_view_policy_valid,
            missing_render_call_ids: self.context_state.missing_render_call_ids.clone(),
            mutation_policy_digest: self.call_spec.mutation_policy_digest.clone(),
            governance_policy_digest: self.call_spec.governance_policy_digest.clone(),
            normalizer_id: self.call_spec.normalizer_id.clone(),
            session_refs: self.session_refs.clone(),
            trajectory_refs: self.trajectory_refs.clone(),
            missing_result_call_ids: self.join_state.missing_result_call_ids.clone(),
            orphan_result_call_ids: self.join_state.orphan_result_call_ids.clone(),
            missing_use_call_ids: self.join_state.missing_use_call_ids.clone(),
            unknown_use_call_ids: self.join_state.unknown_use_call_ids.clone(),
        }
    }
}

pub fn normalize_typestate_evidence(
    input: TypestateEvidenceInput,
) -> Result<NormalizedTypestateEvidence, TypestateNormalizationError> {
    let call_spec = normalize_call_spec(input.call_spec)?;

    let mut request_map: BTreeMap<String, NormalizedToolRequest> = BTreeMap::new();
    for request in input.tool_requests {
        let normalized = normalize_tool_request(request)?;
        insert_unique(
            &mut request_map,
            normalized.tool_call_id.clone(),
            normalized,
            "toolRequests",
        )?;
    }
    let tool_requests: Vec<NormalizedToolRequest> = request_map.into_values().collect();

    let mut result_map: BTreeMap<String, NormalizedToolResult> = BTreeMap::new();
    for result in input.tool_results {
        let normalized = normalize_tool_result(result)?;
        insert_unique(
            &mut result_map,
            normalized.tool_call_id.clone(),
            normalized,
            "toolResults",
        )?;
    }
    let tool_results: Vec<NormalizedToolResult> = result_map.into_values().collect();

    let mut use_map: BTreeMap<String, NormalizedToolUse> = BTreeMap::new();
    for tool_use in input.tool_use {
        let normalized = normalize_tool_use(tool_use)?;
        insert_unique(
            &mut use_map,
            normalized.tool_call_id.clone(),
            normalized,
            "toolUse",
        )?;
    }
    let tool_use: Vec<NormalizedToolUse> = use_map.into_values().collect();

    let mut render_map: BTreeMap<String, NormalizedToolRenderObservation> = BTreeMap::new();
    for row in input.tool_render {
        let normalized = normalize_tool_render(row)?;
        insert_unique(
            &mut render_map,
            normalized.tool_call_id.clone(),
            normalized,
            "toolRender",
        )?;
    }
    let tool_render: Vec<NormalizedToolRenderObservation> = render_map.into_values().collect();

    let mut queue_map: BTreeMap<String, NormalizedReminderQueueReduction> = BTreeMap::new();
    for row in input.reminder_queue {
        let normalized = normalize_reminder_queue(row)?;
        insert_unique(
            &mut queue_map,
            normalized.queue_id.clone(),
            normalized,
            "reminderQueue",
        )?;
    }
    let reminder_queue: Vec<NormalizedReminderQueueReduction> = queue_map.into_values().collect();

    let mut state_view_map: BTreeMap<String, NormalizedStateViewObservation> = BTreeMap::new();
    for row in input.state_views {
        let normalized = normalize_state_view(row)?;
        insert_unique(
            &mut state_view_map,
            normalized.view_id.clone(),
            normalized,
            "stateViews",
        )?;
    }
    let state_views: Vec<NormalizedStateViewObservation> = state_view_map.into_values().collect();

    let protocol_state = normalize_protocol_state(input.protocol_state)?;
    let handoff = input.handoff.map(normalize_handoff).transpose()?;

    let requested_call_ids: Vec<String> = tool_requests
        .iter()
        .map(|item| item.tool_call_id.clone())
        .collect();
    let result_call_ids: Vec<String> = tool_results
        .iter()
        .map(|item| item.tool_call_id.clone())
        .collect();
    let used_call_ids: Vec<String> = tool_use
        .iter()
        .map(|item| item.tool_call_id.clone())
        .collect();
    let render_call_ids: Vec<String> = tool_render
        .iter()
        .map(|item| item.tool_call_id.clone())
        .collect();
    let queue_ids: Vec<String> = reminder_queue
        .iter()
        .map(|item| item.queue_id.clone())
        .collect();
    let state_view_ids: Vec<String> = state_views
        .iter()
        .map(|item| item.view_id.clone())
        .collect();

    let requested_set: BTreeSet<String> = requested_call_ids.iter().cloned().collect();
    let result_set: BTreeSet<String> = result_call_ids.iter().cloned().collect();
    let used_set: BTreeSet<String> = used_call_ids.iter().cloned().collect();
    let render_set: BTreeSet<String> = render_call_ids.iter().cloned().collect();

    let missing_result_call_ids: Vec<String> =
        requested_set.difference(&result_set).cloned().collect();
    let orphan_result_call_ids: Vec<String> =
        result_set.difference(&requested_set).cloned().collect();
    let missing_use_call_ids: Vec<String> = result_set.difference(&used_set).cloned().collect();
    let unknown_use_call_ids: Vec<String> = used_set.difference(&result_set).cloned().collect();
    let missing_render_call_ids: Vec<String> =
        result_set.difference(&render_set).cloned().collect();

    let join_closed = missing_result_call_ids.is_empty()
        && orphan_result_call_ids.is_empty()
        && missing_use_call_ids.is_empty()
        && unknown_use_call_ids.is_empty();
    let render_policy_valid = tool_render
        .iter()
        .all(|row| row.policy_digest == call_spec.tool_render_protocol_digest);
    let queue_policy_valid = reminder_queue
        .iter()
        .all(|row| row.policy_digest == call_spec.reminder_queue_policy_digest);
    let state_view_policy_valid = state_views
        .iter()
        .all(|row| row.policy_digest == call_spec.state_view_policy_digest);
    let queue_reduction_present = !reminder_queue.is_empty();
    let state_view_present = !state_views.is_empty();
    let continuation_context_ready = !protocol_state.continuation_allowed
        || (missing_render_call_ids.is_empty()
            && queue_reduction_present
            && state_view_present
            && render_policy_valid
            && queue_policy_valid
            && state_view_policy_valid);

    let call_spec_digest = digest_serializable(&call_spec);
    let request_set_digest = digest_serializable(&tool_requests);
    let result_set_digest = digest_serializable(&tool_results);
    let use_set_digest = digest_serializable(&tool_use);
    let tool_render_set_digest = digest_serializable(&tool_render);
    let reminder_queue_set_digest = digest_serializable(&reminder_queue);
    let state_view_set_digest = digest_serializable(&state_views);
    let protocol_state_digest = digest_serializable(&protocol_state);
    let handoff_digest = handoff.as_ref().map(digest_serializable);
    let join_set_digest = digest_serializable(&serde_json::json!({
        "requestSetDigest": request_set_digest,
        "resultSetDigest": result_set_digest,
        "useSetDigest": use_set_digest,
        "toolRenderSetDigest": tool_render_set_digest,
        "reminderQueueSetDigest": reminder_queue_set_digest,
        "stateViewSetDigest": state_view_set_digest,
        "protocolStateDigest": protocol_state_digest,
        "handoffDigest": handoff_digest,
        "requestedCallIds": requested_call_ids,
        "resultCallIds": result_call_ids,
        "usedCallIds": used_call_ids,
        "renderCallIds": render_call_ids,
        "queueIds": queue_ids,
        "stateViewIds": state_view_ids,
        "missingResultCallIds": missing_result_call_ids,
        "orphanResultCallIds": orphan_result_call_ids,
        "missingUseCallIds": missing_use_call_ids,
        "unknownUseCallIds": unknown_use_call_ids,
        "missingRenderCallIds": missing_render_call_ids,
        "queueReductionPresent": queue_reduction_present,
        "stateViewPresent": state_view_present,
        "renderPolicyValid": render_policy_valid,
        "queuePolicyValid": queue_policy_valid,
        "stateViewPolicyValid": state_view_policy_valid,
        "continuationContextReady": continuation_context_ready
    }));

    let digests = TypestateDigestBundle {
        call_spec_digest,
        request_set_digest,
        result_set_digest,
        use_set_digest,
        tool_render_set_digest,
        reminder_queue_set_digest,
        state_view_set_digest,
        protocol_state_digest,
        handoff_digest,
        join_set_digest,
    };

    let join_state = NormalizedJoinState {
        requested_call_ids,
        result_call_ids,
        used_call_ids,
        missing_result_call_ids,
        orphan_result_call_ids,
        missing_use_call_ids,
        unknown_use_call_ids,
        join_closed,
    };

    let context_state = NormalizedContextState {
        render_call_ids,
        queue_ids,
        state_view_ids,
        missing_render_call_ids,
        queue_reduction_present,
        state_view_present,
        render_policy_valid,
        queue_policy_valid,
        state_view_policy_valid,
        continuation_context_ready,
    };

    Ok(NormalizedTypestateEvidence {
        schema: TYPESTATE_NORMALIZED_SCHEMA,
        kind: TYPESTATE_NORMALIZED_KIND.to_string(),
        call_spec,
        tool_requests,
        tool_results,
        tool_use,
        protocol_state,
        handoff,
        tool_render,
        reminder_queue,
        state_views,
        digests,
        join_state,
        context_state,
        session_refs: normalize_refs(input.session_refs),
        trajectory_refs: normalize_refs(input.trajectory_refs),
    })
}

fn normalize_call_spec(
    call_spec: CallSpecInput,
) -> Result<NormalizedCallSpec, TypestateNormalizationError> {
    Ok(NormalizedCallSpec {
        call_id: clean_required(call_spec.call_id, "callSpec.callId")?,
        model_ref: clean_required(call_spec.model_ref, "callSpec.modelRef")?,
        action_mode: clean_required(call_spec.action_mode, "callSpec.actionMode")?
            .to_ascii_lowercase(),
        execution_pattern: clean_required(
            call_spec.execution_pattern,
            "callSpec.executionPattern",
        )?
        .to_ascii_lowercase(),
        normalizer_id: clean_required(call_spec.normalizer_id, "callSpec.normalizerId")?,
        mutation_policy_digest: clean_required(
            call_spec.mutation_policy_digest,
            "callSpec.mutationPolicyDigest",
        )?,
        governance_policy_digest: clean_required(
            call_spec.governance_policy_digest,
            "callSpec.governancePolicyDigest",
        )?,
        tool_render_protocol_digest: clean_required(
            call_spec.tool_render_protocol_digest,
            "callSpec.toolRenderProtocolDigest",
        )?,
        reminder_queue_policy_digest: clean_required(
            call_spec.reminder_queue_policy_digest,
            "callSpec.reminderQueuePolicyDigest",
        )?,
        state_view_policy_digest: clean_required(
            call_spec.state_view_policy_digest,
            "callSpec.stateViewPolicyDigest",
        )?,
        decomposition_policy_digest: clean_required(
            call_spec.decomposition_policy_digest,
            "callSpec.decompositionPolicyDigest",
        )?,
    })
}

fn normalize_tool_request(
    request: ToolRequestInput,
) -> Result<NormalizedToolRequest, TypestateNormalizationError> {
    Ok(NormalizedToolRequest {
        tool_call_id: clean_required(request.tool_call_id, "toolRequests[].toolCallId")?,
        tool_name: clean_required(request.tool_name, "toolRequests[].toolName")?,
        schema_digest: clean_optional(request.schema_digest),
        caller_id: clean_optional(request.caller_id),
        search_ref_digest: clean_optional(request.search_ref_digest),
    })
}

fn normalize_tool_result(
    result: ToolResultInput,
) -> Result<NormalizedToolResult, TypestateNormalizationError> {
    let raw_code = clean_optional(result.error_code);
    let raw_retryable = result.retryable;
    let raw_message = clean_optional(result.error_message);
    let (raw_object_code, raw_object_retryable, raw_object_message) =
        read_error_fields(result.raw_error.as_ref());
    let error_code = raw_code.or(raw_object_code);
    let retryable = raw_retryable.or(raw_object_retryable);
    let error_message = raw_message.or(raw_object_message);
    let has_error_material = error_code.is_some() || retryable.is_some() || error_message.is_some();

    let status = normalize_result_status(result.status, has_error_material)?;
    let result_digest = normalize_result_digest(result.result_digest, result.payload);

    let error = if status == "error" {
        let code = error_code.unwrap_or_else(|| "unknown_error".to_string());
        let retryable = retryable.unwrap_or(false);
        let message_digest = digest_string(error_message.as_deref().unwrap_or(""));
        Some(NormalizedToolErrorEnvelope {
            error_code: code,
            retryable,
            error_message_digest: message_digest,
        })
    } else {
        None
    };

    Ok(NormalizedToolResult {
        tool_call_id: clean_required(result.tool_call_id, "toolResults[].toolCallId")?,
        status,
        result_digest,
        error,
    })
}

fn normalize_result_status(
    raw_status: Option<String>,
    has_error_material: bool,
) -> Result<String, TypestateNormalizationError> {
    let normalized = match clean_optional(raw_status) {
        Some(value) => value.to_ascii_lowercase(),
        None if has_error_material => "error".to_string(),
        None => "ok".to_string(),
    };
    match normalized.as_str() {
        "ok" | "error" => Ok(normalized),
        _ => Err(TypestateNormalizationError::InvalidField {
            field: "toolResults[].status",
            detail: format!("unsupported value `{normalized}` (expected `ok` or `error`)"),
        }),
    }
}

fn normalize_result_digest(
    result_digest: Option<String>,
    payload: Option<Value>,
) -> Option<String> {
    match clean_optional(result_digest) {
        Some(value) => Some(value),
        None => payload.map(|value| digest_serializable(&value)),
    }
}

fn normalize_tool_use(
    tool_use: ToolUseInput,
) -> Result<NormalizedToolUse, TypestateNormalizationError> {
    let disposition =
        clean_required(tool_use.disposition, "toolUse[].disposition")?.to_ascii_lowercase();
    match disposition.as_str() {
        "consumed" | "observed" | "discarded" => {}
        _ => {
            return Err(TypestateNormalizationError::InvalidField {
                field: "toolUse[].disposition",
                detail: format!(
                    "unsupported value `{disposition}` (expected consumed|observed|discarded)"
                ),
            });
        }
    }

    Ok(NormalizedToolUse {
        tool_call_id: clean_required(tool_use.tool_call_id, "toolUse[].toolCallId")?,
        disposition,
        result_digest: clean_optional(tool_use.result_digest),
    })
}

fn normalize_protocol_state(
    protocol_state: ProtocolStateInput,
) -> Result<NormalizedProtocolState, TypestateNormalizationError> {
    Ok(NormalizedProtocolState {
        stop_reason: clean_required(protocol_state.stop_reason, "protocolState.stopReason")?
            .to_ascii_lowercase(),
        continuation_allowed: protocol_state.continuation_allowed,
    })
}

fn normalize_handoff(
    handoff: HandoffObservationInput,
) -> Result<NormalizedHandoffObservation, TypestateNormalizationError> {
    Ok(NormalizedHandoffObservation {
        target: clean_required(handoff.target, "handoff.target")?,
        required_artifact_refs: normalize_refs(handoff.required_artifact_refs),
        return_path_ref: clean_optional(handoff.return_path_ref),
    })
}

fn normalize_tool_render(
    row: ToolRenderObservationInput,
) -> Result<NormalizedToolRenderObservation, TypestateNormalizationError> {
    Ok(NormalizedToolRenderObservation {
        tool_call_id: clean_required(row.tool_call_id, "toolRender[].toolCallId")?,
        operator_payload_digest: clean_required(
            row.operator_payload_digest,
            "toolRender[].operatorPayloadDigest",
        )?,
        reminder_render_digest: clean_required(
            row.reminder_render_digest,
            "toolRender[].reminderRenderDigest",
        )?,
        injection_point: clean_required(row.injection_point, "toolRender[].injectionPoint")?
            .to_ascii_lowercase(),
        policy_digest: clean_required(row.policy_digest, "toolRender[].policyDigest")?,
    })
}

fn normalize_reminder_queue(
    row: ReminderQueueReductionInput,
) -> Result<NormalizedReminderQueueReduction, TypestateNormalizationError> {
    Ok(NormalizedReminderQueueReduction {
        queue_id: clean_required(row.queue_id, "reminderQueue[].queueId")?,
        reduced_digest: clean_required(row.reduced_digest, "reminderQueue[].reducedDigest")?,
        policy_digest: clean_required(row.policy_digest, "reminderQueue[].policyDigest")?,
    })
}

fn normalize_state_view(
    row: StateViewObservationInput,
) -> Result<NormalizedStateViewObservation, TypestateNormalizationError> {
    Ok(NormalizedStateViewObservation {
        view_id: clean_required(row.view_id, "stateViews[].viewId")?,
        view_digest: clean_required(row.view_digest, "stateViews[].viewDigest")?,
        policy_digest: clean_required(row.policy_digest, "stateViews[].policyDigest")?,
        source_refs: normalize_refs(row.source_refs),
    })
}

fn insert_unique<T: Clone + PartialEq>(
    map: &mut BTreeMap<String, T>,
    key: String,
    value: T,
    surface: &'static str,
) -> Result<(), TypestateNormalizationError> {
    if let Some(existing) = map.get(&key) {
        if existing != &value {
            return Err(TypestateNormalizationError::DuplicateToolCallId {
                surface,
                tool_call_id: key,
            });
        }
        return Ok(());
    }
    map.insert(key, value);
    Ok(())
}

fn clean_required(
    value: String,
    field: &'static str,
) -> Result<String, TypestateNormalizationError> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(TypestateNormalizationError::MissingField(field));
    }
    Ok(trimmed)
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn normalize_refs(values: Vec<String>) -> Vec<String> {
    let mut refs = BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            refs.insert(trimmed.to_string());
        }
    }
    refs.into_iter().collect()
}

fn read_error_fields(raw_error: Option<&Value>) -> (Option<String>, Option<bool>, Option<String>) {
    let mut code = None;
    let mut retryable = None;
    let mut message = None;
    if let Some(Value::Object(map)) = raw_error {
        code = map
            .get("errorCode")
            .or_else(|| map.get("code"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned);
        retryable = map.get("retryable").and_then(Value::as_bool);
        message = map
            .get("errorMessage")
            .or_else(|| map.get("message"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned);
    }
    (code, retryable, message)
}

fn digest_serializable<T: Serialize>(value: &T) -> String {
    let payload = serde_json::to_value(value).expect("serializable input should convert to value");
    let canonical = canonical_json_bytes(&payload);
    let digest = Sha256::digest(&canonical);
    format!("sha256:{}", hex_lower(&digest))
}

fn digest_string(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("sha256:{}", hex_lower(&digest))
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    match value {
        Value::Null => b"null".to_vec(),
        Value::Bool(true) => b"true".to_vec(),
        Value::Bool(false) => b"false".to_vec(),
        Value::Number(number) => number.to_string().into_bytes(),
        Value::String(_) => serde_json::to_vec(value).expect("string serialization should succeed"),
        Value::Array(items) => {
            let mut out = Vec::new();
            out.push(b'[');
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(b',');
                }
                out.extend(canonical_json_bytes(item));
            }
            out.push(b']');
            out
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();

            let mut out = Vec::new();
            out.push(b'{');
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(b',');
                }
                out.extend(
                    serde_json::to_vec(&Value::String((*key).clone()))
                        .expect("object key serialization should succeed"),
                );
                out.push(b':');
                out.extend(canonical_json_bytes(
                    map.get(*key).expect("sorted object key must exist in map"),
                ));
            }
            out.push(b'}');
            out
        }
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture_input() -> TypestateEvidenceInput {
        TypestateEvidenceInput {
            call_spec: CallSpecInput {
                call_id: "call-1".to_string(),
                model_ref: "gpt-5".to_string(),
                action_mode: "code".to_string(),
                execution_pattern: "parallel".to_string(),
                normalizer_id: "nf.v1".to_string(),
                mutation_policy_digest: "mut.pol.v1".to_string(),
                governance_policy_digest: "gov.pol.v1".to_string(),
                tool_render_protocol_digest: "render.pol.v1".to_string(),
                reminder_queue_policy_digest: "queue.pol.v1".to_string(),
                state_view_policy_digest: "state.pol.v1".to_string(),
                decomposition_policy_digest: "decomp.pol.v1".to_string(),
            },
            tool_requests: vec![
                ToolRequestInput {
                    tool_call_id: "tc-2".to_string(),
                    tool_name: "web.search".to_string(),
                    schema_digest: Some("schema.b".to_string()),
                    caller_id: Some("planner".to_string()),
                    search_ref_digest: None,
                },
                ToolRequestInput {
                    tool_call_id: "tc-1".to_string(),
                    tool_name: "fs.read".to_string(),
                    schema_digest: Some("schema.a".to_string()),
                    caller_id: Some("planner".to_string()),
                    search_ref_digest: None,
                },
            ],
            tool_results: vec![
                ToolResultInput {
                    tool_call_id: "tc-2".to_string(),
                    status: Some("ok".to_string()),
                    result_digest: None,
                    payload: Some(json!({"status": "ok", "count": 2})),
                    error_code: None,
                    retryable: None,
                    error_message: None,
                    raw_error: None,
                },
                ToolResultInput {
                    tool_call_id: "tc-1".to_string(),
                    status: Some("ok".to_string()),
                    result_digest: Some("sha256:tool-result-a".to_string()),
                    payload: None,
                    error_code: None,
                    retryable: None,
                    error_message: None,
                    raw_error: None,
                },
            ],
            tool_use: vec![
                ToolUseInput {
                    tool_call_id: "tc-2".to_string(),
                    disposition: "observed".to_string(),
                    result_digest: None,
                },
                ToolUseInput {
                    tool_call_id: "tc-1".to_string(),
                    disposition: "consumed".to_string(),
                    result_digest: Some("sha256:tool-result-a".to_string()),
                },
            ],
            tool_render: vec![
                ToolRenderObservationInput {
                    tool_call_id: "tc-2".to_string(),
                    operator_payload_digest: "sha256:operator-result-b".to_string(),
                    reminder_render_digest: "sha256:reminder-result-b".to_string(),
                    injection_point: "tool_response".to_string(),
                    policy_digest: "render.pol.v1".to_string(),
                },
                ToolRenderObservationInput {
                    tool_call_id: "tc-1".to_string(),
                    operator_payload_digest: "sha256:operator-result-a".to_string(),
                    reminder_render_digest: "sha256:reminder-result-a".to_string(),
                    injection_point: "tool_response".to_string(),
                    policy_digest: "render.pol.v1".to_string(),
                },
            ],
            reminder_queue: vec![ReminderQueueReductionInput {
                queue_id: "reminder/default".to_string(),
                reduced_digest: "sha256:queue-reduced".to_string(),
                policy_digest: "queue.pol.v1".to_string(),
            }],
            state_views: vec![StateViewObservationInput {
                view_id: "state/latest".to_string(),
                view_digest: "sha256:state-latest".to_string(),
                policy_digest: "state.pol.v1".to_string(),
                source_refs: vec![
                    "handoff://return/main".to_string(),
                    "handoff://return/main".to_string(),
                ],
            }],
            protocol_state: ProtocolStateInput {
                stop_reason: "tool_use".to_string(),
                continuation_allowed: true,
            },
            handoff: Some(HandoffObservationInput {
                target: "worker.backend".to_string(),
                required_artifact_refs: vec![
                    "artifact://spec/design".to_string(),
                    "artifact://spec/design".to_string(),
                ],
                return_path_ref: Some("handoff://return/main".to_string()),
            }),
            session_refs: vec![
                "session://call-1".to_string(),
                "session://call-1".to_string(),
                " ".to_string(),
            ],
            trajectory_refs: vec![
                "trajectory://step/1".to_string(),
                "trajectory://step/1".to_string(),
            ],
        }
    }

    #[test]
    fn normalization_is_order_invariant_for_tool_sets() {
        let input_a = fixture_input();
        let mut input_b = fixture_input();
        input_b.tool_requests.reverse();
        input_b.tool_results.reverse();
        input_b.tool_use.reverse();
        input_b.tool_render.reverse();

        let normalized_a = normalize_typestate_evidence(input_a).expect("normalize a");
        let normalized_b = normalize_typestate_evidence(input_b).expect("normalize b");

        assert_eq!(normalized_a.digests, normalized_b.digests);
        assert_eq!(normalized_a.join_state, normalized_b.join_state);
        assert_eq!(
            normalized_a.session_refs,
            vec!["session://call-1".to_string()]
        );
        assert_eq!(
            normalized_a.trajectory_refs,
            vec!["trajectory://step/1".to_string()]
        );
    }

    #[test]
    fn error_envelope_normalization_is_machine_readable() {
        let input = TypestateEvidenceInput {
            call_spec: fixture_input().call_spec,
            tool_requests: vec![ToolRequestInput {
                tool_call_id: "tc-1".to_string(),
                tool_name: "http.fetch".to_string(),
                schema_digest: None,
                caller_id: None,
                search_ref_digest: None,
            }],
            tool_results: vec![ToolResultInput {
                tool_call_id: "tc-1".to_string(),
                status: None,
                result_digest: None,
                payload: None,
                error_code: None,
                retryable: None,
                error_message: None,
                raw_error: Some(json!({
                    "code": "timeout",
                    "retryable": true,
                    "message": "network timeout"
                })),
            }],
            tool_use: vec![],
            tool_render: vec![],
            reminder_queue: vec![],
            state_views: vec![],
            protocol_state: ProtocolStateInput {
                stop_reason: "tool_use".to_string(),
                continuation_allowed: false,
            },
            handoff: None,
            session_refs: vec![],
            trajectory_refs: vec![],
        };

        let normalized = normalize_typestate_evidence(input).expect("normalize should succeed");
        assert_eq!(normalized.tool_results.len(), 1);
        assert_eq!(normalized.tool_results[0].status, "error");
        let error = normalized.tool_results[0]
            .error
            .clone()
            .expect("error envelope required");
        assert_eq!(error.error_code, "timeout");
        assert!(error.retryable);
        assert_eq!(error.error_message_digest, digest_string("network timeout"));
        assert_eq!(
            normalized.join_state.missing_use_call_ids,
            vec!["tc-1".to_string()]
        );
        assert!(!normalized.join_state.join_closed);
    }

    #[test]
    fn duplicate_tool_call_id_with_conflict_is_rejected() {
        let mut input = fixture_input();
        input.tool_requests = vec![
            ToolRequestInput {
                tool_call_id: "tc-1".to_string(),
                tool_name: "fs.read".to_string(),
                schema_digest: None,
                caller_id: None,
                search_ref_digest: None,
            },
            ToolRequestInput {
                tool_call_id: "tc-1".to_string(),
                tool_name: "web.search".to_string(),
                schema_digest: None,
                caller_id: None,
                search_ref_digest: None,
            },
        ];

        let err = normalize_typestate_evidence(input).expect_err("duplicate should fail");
        assert!(matches!(
            err,
            TypestateNormalizationError::DuplicateToolCallId { surface, .. } if surface == "toolRequests"
        ));
    }

    #[test]
    fn join_and_mutation_gate_inputs_carry_policy_digests() {
        let normalized =
            normalize_typestate_evidence(fixture_input()).expect("normalize should succeed");
        let join = normalized.join_closed_input();
        let mutation = normalized.mutation_ready_input();

        assert_eq!(join.reminder_queue_policy_digest, "queue.pol.v1");
        assert_eq!(join.tool_render_protocol_digest, "render.pol.v1");
        assert_eq!(join.state_view_policy_digest, "state.pol.v1");
        assert_eq!(join.decomposition_policy_digest, "decomp.pol.v1");
        assert!(join.missing_render_call_ids.is_empty());
        assert!(join.queue_reduction_present);
        assert!(join.state_view_present);
        assert!(join.render_policy_valid);
        assert!(join.queue_policy_valid);
        assert!(join.state_view_policy_valid);
        assert!(join.continuation_context_ready);
        assert!(join.join_closed);

        assert!(mutation.continuation_allowed);
        assert!(mutation.continuation_context_ready);
        assert!(mutation.queue_reduction_present);
        assert!(mutation.state_view_present);
        assert!(mutation.render_policy_valid);
        assert!(mutation.queue_policy_valid);
        assert!(mutation.state_view_policy_valid);
        assert!(mutation.missing_render_call_ids.is_empty());
        assert_eq!(mutation.mutation_policy_digest, "mut.pol.v1");
        assert_eq!(mutation.governance_policy_digest, "gov.pol.v1");
        assert_eq!(mutation.normalizer_id, "nf.v1");
        assert!(mutation.join_closed);
    }

    #[test]
    fn continuation_context_requires_render_queue_and_state_views() {
        let mut input = fixture_input();
        input.tool_render.clear();
        input.reminder_queue.clear();
        input.state_views.clear();

        let normalized =
            normalize_typestate_evidence(input).expect("normalization should still succeed");
        let join = normalized.join_closed_input();
        assert_eq!(
            join.missing_render_call_ids,
            vec!["tc-1".to_string(), "tc-2".to_string()]
        );
        assert!(!join.queue_reduction_present);
        assert!(!join.state_view_present);
        assert!(!join.continuation_context_ready);
    }

    #[test]
    fn continuation_context_fails_when_policy_digests_mismatch() {
        let mut input = fixture_input();
        input.reminder_queue[0].policy_digest = "queue.pol.v2".to_string();

        let normalized =
            normalize_typestate_evidence(input).expect("normalization should still succeed");
        let join = normalized.join_closed_input();
        assert!(join.queue_reduction_present);
        assert!(!join.queue_policy_valid);
        assert!(!join.continuation_context_ready);
    }
}
