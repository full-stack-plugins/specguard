use crate::{graph::*, model::*};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    Duplicate,
    MissingRequirement,
    MissingAcceptance,
    BrokenReference,
    Incomplete,
    ToolError,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Finding {
    pub kind: FindingKind,
    pub key: Identity,
    pub sources: Vec<SourceRef>,
}
pub fn validate_graph(graph: &SpecificationGraph, frozen: &BTreeSet<Identity>) -> Vec<Finding> {
    use std::collections::BTreeMap;
    if !graph.complete() {
        let kind = if graph
            .parsed
            .sources
            .iter()
            .any(|s| s.status == Terminal::IoError)
        {
            FindingKind::ToolError
        } else {
            FindingKind::Incomplete
        };
        return frozen
            .iter()
            .map(|key| Finding {
                kind: kind.clone(),
                key: key.clone(),
                sources: graph
                    .parsed
                    .requirements
                    .iter()
                    .filter(|r| &r.key == key)
                    .map(|r| r.source.clone())
                    .collect(),
            })
            .collect();
    }
    let mut findings = Vec::new();
    let mut identities: BTreeMap<Identity, Vec<SourceRef>> = BTreeMap::new();
    let reqs: BTreeSet<_> = graph
        .parsed
        .requirements
        .iter()
        .map(|r| r.key.clone())
        .collect();
    for r in &graph.parsed.requirements {
        identities
            .entry(r.key.clone())
            .or_default()
            .push(r.source.clone());
    }
    for a in &graph.parsed.acceptances {
        identities
            .entry(a.key.clone())
            .or_default()
            .push(a.source.clone());
        if !reqs.contains(&a.requirement) {
            findings.push(Finding {
                kind: FindingKind::BrokenReference,
                key: a.requirement.clone(),
                sources: vec![a.source.clone()],
            });
        }
    }
    for (key, mut sources) in identities {
        if sources.len() > 1 {
            sources.sort();
            findings.push(Finding {
                kind: FindingKind::Duplicate,
                key,
                sources,
            });
        }
    }
    for key in frozen {
        let sources: Vec<_> = graph
            .parsed
            .requirements
            .iter()
            .filter(|r| &r.key == key)
            .map(|r| r.source.clone())
            .collect();
        if sources.is_empty() {
            findings.push(Finding {
                kind: FindingKind::MissingRequirement,
                key: key.clone(),
                sources,
            });
        } else if !graph
            .parsed
            .acceptances
            .iter()
            .any(|a| &a.requirement == key && !a.text.is_empty())
        {
            findings.push(Finding {
                kind: FindingKind::MissingAcceptance,
                key: key.clone(),
                sources,
            });
        }
    }
    for edge in &graph.parsed.edges {
        if !reqs.contains(&edge.from)
            || (edge.relation == Relation::DependsOn && !reqs.contains(&edge.to))
        {
            findings.push(Finding {
                kind: FindingKind::BrokenReference,
                key: edge.to.clone(),
                sources: vec![edge.source.clone()],
            });
        }
    }
    findings.sort();
    findings
}
