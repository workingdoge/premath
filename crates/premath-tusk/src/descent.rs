use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModeBinding {
    pub normalizer_id: String,
    pub policy_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompatWitness {
    pub part_i: String,
    pub part_j: String,
    pub overlap_id: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DescentCore {
    pub cover_id: String,
    pub locals: BTreeMap<String, Value>,
    pub compat: Vec<CompatWitness>,
    pub mode: ModeBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlueProposal {
    pub proposal_id: String,
    #[serde(default)]
    pub payload: Value,
}

pub type GlueProposalSet = Vec<GlueProposal>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DescentPack {
    pub core: DescentCore,
    pub glue_proposals: GlueProposalSet,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GlueMethod {
    NormalForm,
    EquivWitness,
    ExternalChecker,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContractibilityBasis {
    pub mode: ModeBinding,
    pub method: GlueMethod,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlueResult {
    pub selected: String,
    pub contractibility_basis: ContractibilityBasis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal_form_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GlueSelectionFailure {
    NoValidProposal,
    NonContractibleSelection,
    ModeComparisonUnavailable,
}
