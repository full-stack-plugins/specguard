//! Frozen local architecture handoff. No approval or Cargo policy is issued here.
use crate::{
    baseline::*, graph::TargetKind, integration::approval::*, model::*, source::*, trace::*,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub const MAX_HANDOFF_BYTES: usize = 1_048_576;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HandoffVersion {
    #[serde(rename = "specguard.architecture-handoff/v1alpha1")]
    V1,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthenticationProfile {
    #[serde(rename = "fixture-only")]
    FixtureOnly,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchitectureReference {
    pub requirement: Identity,
    pub requirement_source: SourceRef,
    pub adr: Identity,
    pub adr_source: SourceRef,
    pub relation_source: SourceRef,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchitectureHandoff {
    pub api_version: HandoffVersion,
    pub authentication_profile: AuthenticationProfile,
    pub repository: String,
    pub baseline: ApprovedBaseline,
    pub baseline_digest: String,
    pub candidate_binding: CandidateBinding,
    pub source_digest: String,
    pub scope: BTreeSet<Identity>,
    pub snapshot: SourceSnapshot,
    pub trace: TraceArtifact,
    pub architecture_references: Vec<ArchitectureReference>,
}
/// Caller-owned pins, obtained outside the candidate's serialized artifact.
#[derive(Debug, Clone)]
pub struct ExpectedHandoff {
    pub artifact_digest: String,
    pub repository: String,
    pub candidate_binding: CandidateBinding,
    pub baseline_digest: String,
    pub source_digest: String,
    pub scope: BTreeSet<Identity>,
    pub authentication_profile: AuthenticationProfile,
}
pub fn artifact_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn references(trace: &TraceArtifact) -> Result<Vec<ArchitectureReference>, String> {
    if !trace.graph.complete() || !trace.findings.is_empty() {
        return Err("incomplete or invalid architecture trace".into());
    }
    let parsed = &trace.graph.graph.parsed;
    let mut refs = Vec::new();
    for edge in &parsed.edges {
        if edge.relation != Relation::TracesToAdr || !trace.required.contains(&edge.from) {
            continue;
        }
        let requirement = parsed
            .requirements
            .iter()
            .find(|r| r.key == edge.from)
            .ok_or("missing requirement")?;
        let adr = trace
            .graph
            .targets
            .nodes
            .iter()
            .find(|t| t.key == edge.to && t.kind == TargetKind::Adr)
            .ok_or("missing ADR")?;
        refs.push(ArchitectureReference {
            requirement: edge.from.clone(),
            requirement_source: requirement.source.clone(),
            adr: edge.to.clone(),
            adr_source: adr.source.clone(),
            relation_source: edge.source.clone(),
        });
    }
    if trace
        .required
        .iter()
        .any(|id| !refs.iter().any(|r| &r.requirement == id))
    {
        return Err("required architecture scope lacks explicit ADR reference".into());
    }
    refs.sort();
    Ok(refs)
}
fn bounded<T: Serialize>(value: &T) -> Result<(), String> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            if self.0 > MAX_HANDOFF_BYTES {
                return Err(std::io::Error::other("handoff budget"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter(0), value).map_err(|_| "architecture handoff byte budget".into())
}
fn validate(h: &ArchitectureHandoff) -> Result<(), String> {
    bounded(h)?;
    let oid_len = match h.candidate_binding.object_format.as_str() {
        "sha1" => 40,
        "sha256" => 64,
        _ => return Err("unsupported Git object format".into()),
    };
    for oid in [
        &h.candidate_binding.candidate_oid,
        &h.candidate_binding.base_oid,
    ] {
        if oid.len() != oid_len
            || !oid
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err("invalid Git object identity".into());
        }
    }
    let parsed = &h.baseline.graph.parsed;
    if parsed
        .requirements
        .len()
        .saturating_add(parsed.acceptances.len())
        .saturating_add(parsed.edges.len())
        .saturating_add(parsed.sources.len())
        > 4096
    {
        return Err("baseline graph budget".into());
    }
    if h.scope.is_empty()
        || h.scope.len() > 256
        || !h.scope.is_subset(&h.baseline.scope)
        || h.repository != h.baseline.repository
        || h.baseline.state != BaselineState::Approved
        || h.baseline_digest != digest(&h.baseline)
    {
        return Err("baseline or scope binding mismatch".into());
    }
    validate_baseline(&h.baseline)?;
    if h.candidate_binding != h.snapshot.binding
        || h.source_digest != h.snapshot.digest
        || h.trace.required != h.scope
    {
        return Err("candidate/source/scope binding mismatch".into());
    }
    let trace = crate::trace::scan(&h.snapshot, &h.scope)?;
    if trace != h.trace || references(&trace)? != h.architecture_references {
        return Err("trace or architecture references mismatch".into());
    }
    Ok(())
}
pub fn export(
    snapshot: &SourceSnapshot,
    approved: &ValidatedBaseline,
    scope: &BTreeSet<Identity>,
) -> Result<ArchitectureHandoff, String> {
    bounded(&(snapshot, approved.record(), scope))?;
    if approved.profile() != &Profile::Fixture {
        return Err("production architecture handoff unavailable".into());
    }
    let trace = crate::trace::scan(snapshot, scope)?;
    let h = ArchitectureHandoff {
        api_version: HandoffVersion::V1,
        authentication_profile: AuthenticationProfile::FixtureOnly,
        repository: approved.record().repository.clone(),
        baseline: approved.record().clone(),
        baseline_digest: digest(approved.record()),
        candidate_binding: snapshot.binding.clone(),
        source_digest: snapshot.digest.clone(),
        scope: scope.clone(),
        architecture_references: references(&trace)?,
        snapshot: snapshot.clone(),
        trace,
    };
    encode(&h)?;
    Ok(h)
}
pub fn encode(h: &ArchitectureHandoff) -> Result<Vec<u8>, String> {
    validate(h)?;
    let bytes = serde_json::to_vec(h).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_HANDOFF_BYTES {
        return Err("architecture handoff byte budget".into());
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8], expected: &ExpectedHandoff) -> Result<ArchitectureHandoff, String> {
    if bytes.len() > MAX_HANDOFF_BYTES || artifact_digest(bytes) != expected.artifact_digest {
        return Err("architecture handoff digest/budget mismatch".into());
    }
    let h: ArchitectureHandoff = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if h.repository != expected.repository
        || h.candidate_binding != expected.candidate_binding
        || h.baseline_digest != expected.baseline_digest
        || h.source_digest != expected.source_digest
        || h.scope != expected.scope
        || h.authentication_profile != expected.authentication_profile
    {
        return Err("external handoff context mismatch".into());
    }
    validate(&h)?;
    Ok(h)
}
