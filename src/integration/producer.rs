//! Opt-in local engine-backed producer. Controller policy is supplied separately
//! from candidate sources; neither this API nor recomputation authenticates it.
use crate::{
    graph::{SpecificationGraph, build_graph},
    model::{Identity, Terminal},
    parser::parse_controlled,
    rules::{Finding, FindingKind, validate_graph},
    source::{SourceSnapshot, verify_candidate_objects},
};
use guardengine::{integration::*, *};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

const ANALYZER: &str = "specguard.structural";
const PROFILE: &str = "specguard.structural/v1";
const KINDS: [FindingKind; 4] = [
    FindingKind::Duplicate,
    FindingKind::MissingRequirement,
    FindingKind::MissingAcceptance,
    FindingKind::BrokenReference,
];
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MappingEntry {
    pub key: Identity,
    pub kind: FindingKind,
    pub rule_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtectedMapping {
    pub contract: GuardContract,
    pub entries: Vec<MappingEntry>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    pub run_id: String,
    pub repo_id: String,
    pub task_id: String,
    pub worktree_id: String,
    pub candidate_oid: Option<String>,
    pub base_oid: Option<String>,
    pub merge_group_id: Option<String>,
    pub baseline_digest: Option<String>,
    pub started_at: String,
}

fn transport(code: &str) -> TransportDiagnostic {
    TransportDiagnostic {
        code: code.into(),
        message: format!("SpecGuard producer rejected {code}"),
    }
}
// Counts borrowed serialized inputs before any source/policy clones or GE's
// allocation-based envelope validation. Also bounds JSON escaping expansion.
pub(crate) fn preflight<T: Serialize>(value: &T) -> Result<(), String> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_ARTIFACT_BYTES.saturating_sub(self.0) {
                return Err(std::io::Error::other("producer byte budget"));
            }
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter(0), value).map_err(|_| "producer byte budget".into())
}
fn qualified(key: &Identity) -> Result<String, String> {
    if [&key.namespace, &key.id].iter().any(|v| {
        v.is_empty()
            || v.len() > 1024
            || !v
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    }) {
        return Err("invalid requirement identity".into());
    }
    Ok(format!("{}:{}", key.namespace, key.id))
}
impl ProtectedMapping {
    fn validate(&self, required: &BTreeSet<Identity>) -> Result<(), String> {
        preflight(self)?;
        if self.entries.is_empty()
            || self.entries.len() > 256
            || self.contract.spec.rules.len() > 256
        {
            return Err("mapping count budget".into());
        }
        self.contract.validate().map_err(|e| e.to_string())?;
        let mut mappings = BTreeSet::new();
        let mut used = BTreeSet::new();
        let mut tuples = BTreeSet::new();
        for entry in &self.entries {
            if !KINDS.contains(&entry.kind)
                || entry.subject != qualified(&entry.key)?
                || !mappings.insert((&entry.key, &entry.kind))
                || !used.insert(&entry.rule_id)
                || !tuples.insert((&entry.subject, &entry.predicate, &entry.object))
            {
                return Err("ambiguous or unsupported mapping".into());
            }
            let rule = self
                .contract
                .spec
                .rules
                .iter()
                .find(|r| r.id == entry.rule_id)
                .ok_or("unmapped rule")?;
            let GuardAssertion::ForbidRelation {
                subject,
                predicate,
                object,
            } = &rule.assertion;
            if rule.enforcement != Enforcement::Enforce
                || (subject, predicate, object) != (&entry.subject, &entry.predicate, &entry.object)
            {
                return Err("mapping not exact enforced rule".into());
            }
        }
        if used.len() != self.contract.spec.rules.len() {
            return Err("unused contract rule".into());
        }
        for key in required {
            for kind in &KINDS {
                if !mappings.contains(&(key, kind)) {
                    return Err("required finding mapping absent".into());
                }
            }
        }
        Ok(())
    }
}

pub struct PreparedRun {
    work_key: super::freshness::WorkKey,
    attempt: BoundAttempt,
    recovery: super::runtime::Recovery,
    snapshot: SourceSnapshot,
    mapping: ProtectedMapping,
    required: BTreeSet<Identity>,
    coverage: Coverage,
    run_id: String,
}

pub fn prepare(
    root: &Path,
    snapshot: &SourceSnapshot,
    invocation: Invocation,
    required: &BTreeSet<Identity>,
    mapping: &ProtectedMapping,
) -> Result<PreparedRun, TransportDiagnostic> {
    // All checks are borrowed until the immutable context is established.
    preflight(&(snapshot, &invocation, required, mapping))
        .map_err(|_| transport("input.budget"))?;
    if required.is_empty()
        || required.len() > 64
        || snapshot.inventory.entries.len() > 4096
        || snapshot.inventory.sources.len() > 4096
    {
        return Err(transport("scope.invalid"));
    }
    mapping
        .validate(required)
        .map_err(|_| transport("mapping.invalid"))?;
    if crate::model::digest(&(&snapshot.inventory, &snapshot.binding, &snapshot.contents))
        != snapshot.digest
    {
        return Err(transport("snapshot.invalid"));
    }
    if invocation.candidate_oid.as_ref() != Some(&snapshot.binding.candidate_oid)
        || invocation.base_oid.as_ref() != Some(&snapshot.binding.base_oid)
    {
        return Err(transport("binding.unresolved"));
    }
    verify_candidate_objects(root, &snapshot.binding).map_err(|_| transport("binding.objects"))?;
    let mut ids: Vec<String> = required
        .iter()
        .map(qualified)
        .collect::<Result<_, _>>()
        .map_err(|_| transport("scope.invalid"))?;
    ids.sort();
    let mut scopes: BTreeSet<String> = ids.iter().map(|id| format!("requirement:{id}")).collect();
    scopes.insert(format!("profile:{PROFILE}"));
    for source in &snapshot.inventory.sources {
        scopes.insert(format!("source:{}", source.path));
    }
    for source in &snapshot.inventory.entries {
        scopes.insert(format!("source:{}", source.path));
    }
    let required_scopes: Vec<_> = scopes.into_iter().collect();
    let coverage = Coverage {
        status: CoverageStatus::Partial,
        observed_scopes: vec![],
        missing_scopes: required_scopes.clone(),
        required_scopes,
    };
    let binding = RunBinding {
        repo_id: invocation.repo_id,
        task_id: invocation.task_id,
        worktree_id: invocation.worktree_id,
        requirement_ids: ids,
        candidate_oid: snapshot.binding.candidate_oid.clone(),
        base_oid: snapshot.binding.base_oid.clone(),
        merge_group_id: invocation.merge_group_id,
        source_snapshot_digest: snapshot.digest.clone(),
        baseline_digest: invocation.baseline_digest,
    };
    let draft = InvocationDraft {
        run_id: invocation.run_id.clone(),
        producer: Some(Producer {
            guard: "SpecGuard".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            analyzer_id: ANALYZER.into(),
            analyzer_version: "1".into(),
        }),
        binding: Some(binding),
        coverage: Some(coverage.clone()),
        profile: Some(EvidenceProfile::EngineBacked),
        started_at: invocation.started_at,
    };
    let work_key = super::freshness::WorkKey::freeze(&draft, snapshot, required, mapping, PROFILE);
    let recovery = super::runtime::Recovery::from_draft(&draft);
    let attempt = prepare_attempt(draft)?;
    Ok(PreparedRun {
        work_key,
        attempt,
        recovery,
        snapshot: snapshot.clone(),
        mapping: mapping.clone(),
        required: required.clone(),
        coverage,
        run_id: invocation.run_id,
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct ProducedRun {
    pub envelope: GuardRunEnvelope,
    pub contract: Option<Vec<u8>>,
    pub facts: Option<Vec<u8>>,
    pub report: Option<Vec<u8>>,
    pub domain: Option<Vec<u8>>,
}
pub(crate) fn raw_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}
pub(crate) fn reference(run: &str, name: &str, bytes: &[u8]) -> ArtifactRef {
    // Digest-derived storage namespace avoids treating opaque run IDs as paths.
    let run_hash = raw_digest(run.as_bytes());
    ArtifactRef {
        uri: format!("artifact://specguard/{}/{name}", &run_hash[7..]),
        digest: raw_digest(bytes),
        media_type: "application/json".into(),
    }
}
fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    preflight(value)?;
    serde_json::to_vec(value).map_err(|_| "serialization failure".into())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DomainEvidence<'a> {
    api_version: &'static str,
    profile: &'static str,
    graph: &'a SpecificationGraph,
    findings: &'a [Finding],
    mapping: &'a ProtectedMapping,
    required: &'a BTreeSet<Identity>,
}

// Admission estimate uses borrowed bytes only. Each source line is charged as
// a potential node, including source identity, vector growth, and domain/rule
// copies. Text/registry/AST work also receives a conservative byte multiplier.
// This intentionally rejects some large but otherwise valid documents before
// parser allocation; it is not a global allocator or OS memory limit.
pub(crate) fn parser_preflight(snapshot: &SourceSnapshot) -> Result<(), String> {
    let mut cost = 0usize;
    let mut lines = 0usize;
    for entry in &snapshot.inventory.entries {
        let bytes = snapshot
            .contents
            .get(&entry.path)
            .ok_or("missing source bytes")?;
        let source_lines = bytes
            .iter()
            .filter(|b| **b == b'\n')
            .count()
            .saturating_add(1);
        lines = lines.saturating_add(source_lines);
        let per_line = entry
            .path
            .len()
            .saturating_add(entry.namespace.len())
            .saturating_add(std::mem::size_of::<crate::model::Requirement>())
            .saturating_add(std::mem::size_of::<crate::model::Acceptance>())
            .saturating_add(std::mem::size_of::<crate::model::TraceEdge>())
            .saturating_add(128)
            .saturating_mul(2);
        cost = cost
            .saturating_add(source_lines.saturating_mul(per_line))
            .saturating_add(bytes.len().saturating_mul(16));
        if lines > 4096 || cost > MAX_ARTIFACT_BYTES {
            return Err("parser construction budget".into());
        }
    }
    Ok(())
}

fn graph_preflight(graph: &SpecificationGraph) -> Result<(), String> {
    preflight(graph)?;
    let p = &graph.parsed;
    if p.requirements
        .len()
        .saturating_add(p.acceptances.len())
        .saturating_add(p.edges.len())
        > 4096
        || p.sources.len() > 4096
    {
        return Err("graph count budget".into());
    }
    Ok(())
}
fn project_facts(
    findings: &[Finding],
    mapping: &ProtectedMapping,
    complete: bool,
    run_id: &str,
    snapshot_digest: &str,
) -> Result<GuardFacts, String> {
    let mut budget = FactBudget::new();
    if complete {
        for finding in findings {
            let entry = mapping
                .entries
                .iter()
                .find(|m| m.key == finding.key && m.kind == finding.kind)
                .ok_or("unmapped finding")?;
            if finding.sources.is_empty() {
                budget
                    .push_relation(
                        &entry.subject,
                        &entry.predicate,
                        &entry.object,
                        "frozen-required-scope",
                    )
                    .map_err(|e| e.to_string())?;
            }
            for source in &finding.sources {
                let line = source.line.to_string();
                budget
                    .push_relation_with_source_parts(
                        &entry.subject,
                        &entry.predicate,
                        &entry.object,
                        &[&source.path, ":", &line],
                    )
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    let facts = GuardFacts {
        api_version: API_VERSION.into(),
        kind: "GuardFacts".into(),
        analyzer: AnalyzerIdentity {
            id: ANALYZER.into(),
            version: "1".into(),
        },
        subject: GuardSubject {
            id: run_id.into(),
            snapshot_digest: snapshot_digest.into(),
        },
        completeness: if complete {
            Completeness::Complete
        } else {
            Completeness::Partial
        },
        facts: budget.finish().map_err(|e| e.to_string())?,
        diagnostics: if complete {
            vec![]
        } else {
            vec!["required source/profile coverage is incomplete".into()]
        },
    };
    Ok(facts)
}

impl PreparedRun {
    pub fn work_key(&self) -> &super::freshness::WorkKey {
        &self.work_key
    }
    pub(crate) fn attempt_id(&self) -> &str {
        &self.run_id
    }
    pub fn cancel(self, finished_at: &str) -> Result<ProducedRun, TransportDiagnostic> {
        self.retained_failure(RunStatus::Cancelled, "execution.cancelled", finished_at)
    }
    pub fn fail(self, finished_at: &str) -> Result<ProducedRun, TransportDiagnostic> {
        self.retained_failure(RunStatus::Error, "execution.failed", finished_at)
    }
    fn retained_failure(
        self,
        status: RunStatus,
        code: &str,
        finished_at: &str,
    ) -> Result<ProducedRun, TransportDiagnostic> {
        let mut history = super::runtime::History::default();
        for source in &self.snapshot.inventory.sources {
            history.record(source);
        }
        self.recovery.failure(status, code, finished_at, history)
    }
    fn terminal(
        self,
        status: RunStatus,
        code: &str,
        finished_at: &str,
    ) -> Result<ProducedRun, TransportDiagnostic> {
        let envelope = self.attempt.finish(AttemptOutput {
            coverage: self.coverage,
            run_status: status,
            decision: None,
            artifacts: Artifacts {
                contract: None,
                facts: None,
                report: None,
                domain: vec![],
            },
            approval_refs: vec![],
            diagnostics: vec![Diagnostic {
                code: code.into(),
                message: "SpecGuard did not complete this bound attempt".into(),
                retryable: true,
                source: None,
            }],
            finished_at: finished_at.into(),
            expires_at: None,
        })?;
        Ok(ProducedRun {
            envelope,
            contract: None,
            facts: None,
            report: None,
            domain: None,
        })
    }
    pub fn complete(self, finished_at: &str) -> Result<ProducedRun, TransportDiagnostic> {
        self.execute(
            finished_at,
            &super::runtime::CancellationToken::new(),
            |_| {},
        )
    }
    /// Execute with cooperative source-boundary cancellation and a trusted,
    /// bounded progress observer. Panics unwind into a bound error; process
    /// abort/OOM/signals and non-returning observers require an external host.
    pub fn execute(
        self,
        finished_at: &str,
        cancellation: &super::runtime::CancellationToken,
        mut observe: impl FnMut(&crate::model::SourceStatus),
    ) -> Result<ProducedRun, TransportDiagnostic> {
        let recovery = self.recovery.clone();
        let mut history = super::runtime::History::default();
        for source in &self.snapshot.inventory.sources {
            history.record(source);
        }
        if finished_at.len() > 4096 {
            return super::runtime::finalize(
                Ok(Err(transport("finish.budget"))),
                &recovery,
                cancellation,
                finished_at,
                history,
            );
        }
        if cfg!(panic = "abort") {
            return recovery.failure(
                RunStatus::Error,
                "runtime.unsupported",
                finished_at,
                history,
            );
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.complete_inner(finished_at, |source| {
                if cancellation.is_cancelled() {
                    return Err(());
                }
                if let Some(source) = source {
                    history.record(source);
                    observe(source);
                }
                if cancellation.is_cancelled() {
                    Err(())
                } else {
                    Ok(())
                }
            })
        }));
        super::runtime::finalize(result, &recovery, cancellation, finished_at, history)
    }
    fn complete_inner(
        self,
        finished_at: &str,
        mut control: impl FnMut(Option<&crate::model::SourceStatus>) -> Result<(), ()>,
    ) -> Result<ProducedRun, TransportDiagnostic> {
        if parser_preflight(&self.snapshot).is_err() {
            return self.terminal(RunStatus::Error, "parser.budget", finished_at);
        }
        let parsed = match parse_controlled(&self.snapshot, &mut control) {
            Ok(parsed) => parsed,
            Err(()) => {
                return self.terminal(RunStatus::Cancelled, "execution.cancelled", finished_at);
            }
        };
        if control(None).is_err() {
            return self.terminal(RunStatus::Cancelled, "execution.cancelled", finished_at);
        }
        let graph = build_graph(parsed);
        if graph
            .parsed
            .sources
            .iter()
            .any(|s| s.status == Terminal::IoError)
        {
            return self.terminal(RunStatus::Error, "source.io", finished_at);
        }
        if graph_preflight(&graph).is_err() {
            return self.terminal(RunStatus::Error, "graph.budget", finished_at);
        }
        let complete = graph.complete();
        let findings = validate_graph(&graph, &self.required);
        if control(None).is_err() {
            return self.terminal(RunStatus::Cancelled, "execution.cancelled", finished_at);
        }
        let projected = (|| -> Result<_, String> {
            preflight(&(&graph, &findings))?;
            let facts = project_facts(
                &findings,
                &self.mapping,
                complete,
                &self.run_id,
                &self.snapshot.digest,
            )?;
            let report =
                evaluate_bounded(&self.mapping.contract, &facts).map_err(|e| e.to_string())?;
            let domain = encode(&DomainEvidence {
                api_version: "specguard.producer/v1alpha1",
                profile: PROFILE,
                graph: &graph,
                findings: &findings,
                mapping: &self.mapping,
                required: &self.required,
            })?;
            Ok((
                encode(&self.mapping.contract)?,
                encode(&facts)?,
                encode(&report)?,
                domain,
                report.decision,
            ))
        })();
        let (contract, facts, report, domain, decision) = match projected {
            Ok(v) => v,
            Err(_) => return self.terminal(RunStatus::Error, "projection.failed", finished_at),
        };
        if control(None).is_err() {
            return self.terminal(RunStatus::Cancelled, "execution.cancelled", finished_at);
        }
        let mut coverage = self.coverage;
        if complete {
            coverage.status = CoverageStatus::Complete;
            coverage.observed_scopes = coverage.required_scopes.clone();
            coverage.missing_scopes.clear();
        }
        let artifacts = Artifacts {
            contract: Some(reference(&self.run_id, "contract.json", &contract)),
            facts: Some(reference(&self.run_id, "facts.json", &facts)),
            report: Some(reference(&self.run_id, "report.json", &report)),
            domain: vec![reference(&self.run_id, "domain.json", &domain)],
        };
        let envelope = self.attempt.finish(AttemptOutput {
            coverage,
            run_status: RunStatus::Completed,
            decision: Some(decision),
            artifacts,
            approval_refs: vec![],
            diagnostics: vec![],
            finished_at: finished_at.into(),
            expires_at: None,
        })?;
        let output = ProducedRun {
            envelope,
            contract: Some(contract),
            facts: Some(facts),
            report: Some(report),
            domain: Some(domain),
        };
        Ok(output)
    }
}
impl ProducedRun {
    /// Byte/report integrity only; does not authenticate producer, policy or storage.
    pub fn verify(&self) -> Result<(), String> {
        self.envelope
            .validate(EvidenceProfile::EngineBacked)
            .map_err(|e| e.to_string())?;
        if self.envelope.run_status != RunStatus::Completed {
            if self.contract.is_some() || self.facts.is_some() || self.report.is_some() {
                return Err("failed run must not retain old successful payloads".into());
            }
            return super::runtime::verify_diagnostic(self);
        }
        verify_engine_artifacts(
            &self.envelope,
            self.contract.as_deref().ok_or("missing contract")?,
            self.facts.as_deref().ok_or("missing facts")?,
            self.report.as_deref().ok_or("missing report")?,
        )
        .map_err(|e| e.to_string())?;
        let domain = self.domain.as_deref().ok_or("missing domain")?;
        if domain.len() > MAX_ARTIFACT_BYTES
            || self.envelope.artifacts.domain.len() != 1
            || self.envelope.artifacts.domain[0].digest != raw_digest(domain)
        {
            return Err("domain artifact mismatch".into());
        }
        let decoded: DomainArtifact =
            serde_json::from_slice(domain).map_err(|_| "invalid domain artifact")?;
        if decoded.api_version != "specguard.producer/v1alpha1"
            || decoded.profile != PROFILE
            || decoded.graph.parsed.snapshot_digest != self.envelope.binding.source_snapshot_digest
            || decoded.graph.parsed.candidate_oid != self.envelope.binding.candidate_oid
        {
            return Err("domain binding/profile mismatch".into());
        }
        if decoded.required.is_empty() || decoded.required.len() > 64 {
            return Err("domain scope budget".into());
        }
        graph_preflight(&decoded.graph)?;
        decoded.mapping.validate(&decoded.required)?;
        let mut ids: Vec<_> = decoded
            .required
            .iter()
            .map(qualified)
            .collect::<Result<_, _>>()?;
        ids.sort();
        if ids != self.envelope.binding.requirement_ids {
            return Err("domain requirements mismatch".into());
        }
        let mut expected_scopes: BTreeSet<String> =
            ids.iter().map(|id| format!("requirement:{id}")).collect();
        expected_scopes.insert(format!("profile:{PROFILE}"));
        expected_scopes.extend(
            decoded
                .graph
                .parsed
                .sources
                .iter()
                .map(|source| format!("source:{}", source.path)),
        );
        if expected_scopes.into_iter().collect::<Vec<_>>() != self.envelope.coverage.required_scopes
        {
            return Err("domain required scopes mismatch".into());
        }
        let expected_findings = validate_graph(&decoded.graph, &decoded.required);
        if expected_findings != decoded.findings {
            return Err("domain findings mismatch".into());
        }
        let actual_contract =
            load_contract_yaml(self.contract.as_deref().ok_or("missing contract")?)
                .map_err(|e| e.to_string())?;
        if actual_contract != decoded.mapping.contract {
            return Err("domain contract mismatch".into());
        }
        let actual_facts = load_facts_json(self.facts.as_deref().ok_or("missing facts")?)
            .map_err(|e| e.to_string())?;
        let expected = project_facts(
            &expected_findings,
            &decoded.mapping,
            decoded.graph.complete(),
            &self.envelope.run_id,
            &self.envelope.binding.source_snapshot_digest,
        )?;
        if expected != actual_facts {
            return Err("domain projection mismatch".into());
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DomainArtifact {
    api_version: String,
    profile: String,
    graph: SpecificationGraph,
    findings: Vec<Finding>,
    mapping: ProtectedMapping,
    required: BTreeSet<Identity>,
}
