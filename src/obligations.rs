use crate::{graph::*, integration::approval::*, model::*};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TestObligation {
    pub id: String,
    pub requirement: Identity,
    pub acceptance: Identity,
    pub source: SourceRef,
    pub text_digest: String,
    pub links: Vec<TraceEdge>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ObligationSet {
    pub api_version: Version,
    pub kind: ObligationKind,
    pub baseline_digest: String,
    pub source_digest: String,
    pub candidate_oid: String,
    pub scope: BTreeSet<Identity>,
    pub sources: Vec<SourceStatus>,
    pub complete: bool,
    pub authentication_profile: String,
    pub obligations: Vec<TestObligation>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ObligationKind {
    TestObligationSet,
}
pub fn export_obligations(
    graph: &SpecificationGraph,
    approved: &ValidatedBaseline,
    scope: &BTreeSet<Identity>,
    expected_snapshot: &str,
) -> Result<ObligationSet, String> {
    let b = approved.record();
    if graph.parsed.snapshot_digest != expected_snapshot {
        return Err("candidate source digest drift".into());
    }
    if scope.is_empty() || !scope.is_subset(&b.scope) {
        return Err("scope is not covered by approved baseline".into());
    }
    let frozen_acceptances_unchanged = b
        .graph
        .parsed
        .acceptances
        .iter()
        .filter(|a| scope.contains(&a.requirement))
        .all(|a| {
            graph.parsed.acceptances.iter().any(|candidate| {
                candidate.key == a.key
                    && candidate.requirement == a.requirement
                    && candidate.text == a.text
            })
        });
    let complete = graph.complete()
        && frozen_acceptances_unchanged
        && crate::rules::validate_graph(graph, scope).is_empty();
    let mut obligations = vec![];
    for a in &graph.parsed.acceptances {
        if scope.contains(&a.requirement) {
            let mut links: Vec<_> = graph
                .parsed
                .edges
                .iter()
                .filter(|e| e.from == a.requirement)
                .cloned()
                .collect();
            links.sort();
            obligations.push(TestObligation {
                id: format!("obligation:{}", digest(&(&a.requirement, &a.key))),
                requirement: a.requirement.clone(),
                acceptance: a.key.clone(),
                source: a.source.clone(),
                text_digest: digest(&a.text),
                links,
            });
        }
    }
    obligations.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(ObligationSet {
        api_version: Version::V1,
        kind: ObligationKind::TestObligationSet,
        baseline_digest: digest(b),
        source_digest: graph.parsed.snapshot_digest.clone(),
        candidate_oid: graph.parsed.candidate_oid.clone(),
        scope: scope.clone(),
        sources: graph.parsed.sources.clone(),
        complete,
        authentication_profile: match approved.profile() {
            Profile::Fixture => "fixture-only",
            Profile::Production => "production",
        }
        .into(),
        obligations,
    })
}
