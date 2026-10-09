use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ParseResult {
    pub api_version: Version,
    pub snapshot_digest: String,
    pub candidate_oid: String,
    pub sources: Vec<SourceStatus>,
    pub requirements: Vec<Requirement>,
    pub acceptances: Vec<Acceptance>,
    pub edges: Vec<TraceEdge>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Version {
    #[serde(rename = "specguard.domain/v1alpha1")]
    V1,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceRef {
    pub path: String,
    pub line: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Terminal {
    Complete,
    Unsupported,
    Malformed,
    Limit,
    IoError,
    Conflict,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceStatus {
    pub path: String,
    pub status: Terminal,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Identity {
    pub namespace: String,
    pub id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Requirement {
    pub key: Identity,
    pub text: String,
    pub source: SourceRef,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Acceptance {
    pub key: Identity,
    pub requirement: Identity,
    pub text: String,
    pub source: SourceRef,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    DependsOn,
    TracesToAdr,
    TracesToTask,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TraceEdge {
    pub from: Identity,
    pub to: Identity,
    pub relation: Relation,
    pub source: SourceRef,
}
pub fn decode_parse(value: serde_json::Value) -> Result<ParseResult, String> {
    serde_json::from_value(value).map_err(|e| e.to_string())
}
pub fn digest<T: Serialize>(value: &T) -> String {
    use sha2::{Digest, Sha256};
    format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("serializable domain value"))
    )
}
