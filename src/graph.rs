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
