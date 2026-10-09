use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SpecificationGraph {
    pub parsed: ParseResult,
}
impl SpecificationGraph {
    pub fn complete(&self) -> bool {
        !self.parsed.sources.is_empty()
            && self
                .parsed
                .sources
                .iter()
                .all(|s| s.status == Terminal::Complete)
            && self
                .parsed
                .edges
                .iter()
                .all(|edge| edge.relation == Relation::DependsOn)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ImpactSet {
    pub paths: Vec<Vec<Identity>>,
    pub complete: bool,
}
pub fn build_graph(mut parsed: ParseResult) -> SpecificationGraph {
    parsed.requirements.sort();
    parsed.acceptances.sort();
    parsed.edges.sort();
    parsed.sources.sort();
    SpecificationGraph { parsed }
}
pub fn validate_mappings(mappings: &BTreeMap<Identity, Identity>) -> Result<(), String> {
    let mut targets = BTreeSet::new();
    for target in mappings.values() {
        if !targets.insert(target) {
            return Err("mapping is not one-to-one".into());
        }
    }
    for start in mappings.keys() {
        let mut seen = BTreeSet::new();
        let mut current = start;
        while let Some(next) = mappings.get(current) {
            if !seen.insert(current) {
                return Err("mapping cycle".into());
            }
            current = next;
        }
    }
    Ok(())
}
pub fn impact(
    changed: &BTreeSet<Identity>,
    graph: &SpecificationGraph,
    budget: usize,
) -> ImpactSet {
    use std::collections::VecDeque;
    let mut reverse: BTreeMap<Identity, BTreeSet<Identity>> = BTreeMap::new();
    for e in &graph.parsed.edges {
        if e.relation == Relation::DependsOn {
            reverse
                .entry(e.to.clone())
                .or_default()
                .insert(e.from.clone());
        }
    }
    let mut seen = BTreeSet::new();
    let mut queue: VecDeque<Vec<Identity>> = changed.iter().cloned().map(|id| vec![id]).collect();
    let mut paths = vec![];
    let mut complete = graph.complete();
    while let Some(path) = queue.pop_front() {
        let last = path.last().unwrap();
        if seen.contains(last) {
            continue;
        }
        if paths.len() >= budget {
            complete = false;
            break;
        }
        seen.insert(last.clone());
        if let Some(next) = reverse.get(last) {
            for node in next {
                if !seen.contains(node) {
                    let mut child = path.clone();
                    child.push(node.clone());
                    queue.push_back(child);
                }
            }
        }
        paths.push(path);
    }
    paths.sort_by(|a, b| a.last().cmp(&b.last()));
    ImpactSet { paths, complete }
}

/// Explicit target inventory for local typed graph evaluation. Kept outside the
/// established ParseResult wire schema until a versioned source adapter exists.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TargetKind {
    Adr,
    Task,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TraceTarget {
    pub key: Identity,
    pub kind: TargetKind,
    pub source: SourceRef,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceTargets {
    pub nodes: Vec<TraceTarget>,
    pub sources: Vec<SourceStatus>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedSpecificationGraph {
    pub graph: SpecificationGraph,
    pub targets: TraceTargets,
}
pub fn build_typed_graph(
    parsed: ParseResult,
    mut targets: TraceTargets,
) -> TypedSpecificationGraph {
    targets.nodes.sort();
    targets.sources.sort();
    TypedSpecificationGraph {
        graph: build_graph(parsed),
        targets,
    }
}
impl TypedSpecificationGraph {
    pub fn complete(&self) -> bool {
        let parsed = &self.graph.parsed;
        let needs_targets = !self.targets.nodes.is_empty()
            || parsed
                .edges
                .iter()
                .any(|e| e.relation != Relation::DependsOn);
        !parsed.sources.is_empty()
            && parsed
                .sources
                .iter()
                .all(|s| s.status == Terminal::Complete)
            && (!needs_targets || !self.targets.sources.is_empty())
            && self
                .targets
                .sources
                .iter()
                .all(|s| s.status == Terminal::Complete)
            && self.targets.nodes.iter().all(|node| {
                self.targets.sources.iter().any(|source| {
                    source.path == node.source.path && source.status == Terminal::Complete
                })
            })
    }
    pub fn validate(&self, frozen: &BTreeSet<Identity>) -> Vec<crate::rules::Finding> {
        use crate::rules::{Finding, FindingKind, validate_graph};
        if !self.complete() {
            let tool_error = self
                .graph
                .parsed
                .sources
                .iter()
                .chain(&self.targets.sources)
                .any(|s| s.status == Terminal::IoError);
            return frozen
                .iter()
                .map(|key| Finding {
                    kind: if tool_error {
                        FindingKind::ToolError
                    } else {
                        FindingKind::Incomplete
                    },
                    key: key.clone(),
                    sources: self
                        .graph
                        .parsed
                        .requirements
                        .iter()
                        .filter(|r| &r.key == key)
                        .map(|r| r.source.clone())
                        .collect(),
                })
                .collect();
        }
        // Reuse the established requirement/acceptance rules, then evaluate the
        // additional relations against their own target types below.
        let mut dependency_graph = self.graph.clone();
        dependency_graph
            .parsed
            .edges
            .retain(|e| e.relation == Relation::DependsOn);
        let mut findings = validate_graph(&dependency_graph, frozen);
        findings.retain(|f| f.kind != FindingKind::Duplicate);
        let mut identities: BTreeMap<&Identity, Vec<SourceRef>> = BTreeMap::new();
        for r in &self.graph.parsed.requirements {
            identities.entry(&r.key).or_default().push(r.source.clone());
        }
        for a in &self.graph.parsed.acceptances {
            identities.entry(&a.key).or_default().push(a.source.clone());
        }
        for target in &self.targets.nodes {
            identities
                .entry(&target.key)
                .or_default()
                .push(target.source.clone());
        }
        for (key, mut sources) in identities {
            if sources.len() > 1 {
                sources.sort();
                findings.push(Finding {
                    kind: FindingKind::Duplicate,
                    key: key.clone(),
                    sources,
                });
            }
        }
        let reqs: BTreeSet<_> = self
            .graph
            .parsed
            .requirements
            .iter()
            .map(|r| &r.key)
            .collect();
        for edge in &self.graph.parsed.edges {
            let expected = match edge.relation {
                Relation::DependsOn => continue,
                Relation::TracesToAdr => TargetKind::Adr,
                Relation::TracesToTask => TargetKind::Task,
            };
            let valid_target = self
                .targets
                .nodes
                .iter()
                .any(|node| node.key == edge.to && node.kind == expected);
            if !reqs.contains(&edge.from) || !valid_target {
                findings.push(Finding {
                    kind: FindingKind::BrokenReference,
                    key: if !reqs.contains(&edge.from) {
                        edge.from.clone()
                    } else {
                        edge.to.clone()
                    },
                    sources: vec![edge.source.clone()],
                });
            }
        }
        findings.sort();
        findings
    }
}
