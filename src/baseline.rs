use crate::{graph::*, model::*};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BaselineState {
    Proposed,
    Approved,
    Superseded,
    Expired,
    Revoked,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ApprovedBaseline {
    pub api_version: Version,
    pub repository: String,
    pub scope: BTreeSet<Identity>,
    pub source_revision: String,
    pub source_digest: String,
    pub graph_digest: String,
    pub policy_digest: String,
    pub approval_ref: String,
    pub effective_from: i64,
    pub expires_at: i64,
    pub state: BaselineState,
    pub graph: SpecificationGraph,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Removed,
    UncertainRemoval,
    Moved,
    ReferencesChanged,
    TextReview,
    AcceptanceAdded,
    AcceptanceRemoved,
    AcceptanceReview,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Change {
    pub key: Identity,
    pub kind: ChangeKind,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BaselineDiff {
    pub changes: Vec<Change>,
    pub complete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", deny_unknown_fields, rename_all = "snake_case")]
pub enum Condition {
    IntegerMinimum { value: i64, unit: String },
    IntegerMaximum { value: i64, unit: String },
    Text { text: String },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
    Equal,
    Tightened,
    Relaxed,
    Review,
}
pub fn decode_baseline(value: serde_json::Value) -> Result<ApprovedBaseline, String> {
    let b = serde_json::from_value(value).map_err(|e| e.to_string())?;
    validate_baseline(&b)?;
    Ok(b)
}
pub fn validate_baseline(b: &ApprovedBaseline) -> Result<(), String> {
    let is_digest = |s: &str| {
        s.strip_prefix("sha256:").is_some_and(|v| {
            v.len() == 64
                && v.bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        })
    };
    if b.repository.trim().is_empty()
        || b.scope.is_empty()
        || b.approval_ref.trim().is_empty()
        || b.effective_from >= b.expires_at
    {
        return Err("missing or invalid baseline binding".into());
    }
    if ![40, 64].contains(&b.source_revision.len())
        || !b
            .source_revision
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err("invalid revision".into());
    }
    if ![&b.source_digest, &b.graph_digest, &b.policy_digest]
        .iter()
        .all(|s| is_digest(s))
    {
        return Err("invalid digest".into());
    }
    if b.source_digest != b.graph.parsed.snapshot_digest
        || b.source_revision != b.graph.parsed.candidate_oid
        || b.graph_digest != digest(&b.graph)
    {
        return Err("baseline content binding mismatch".into());
    }
    if !crate::rules::validate_graph(&b.graph, &b.scope).is_empty() {
        return Err("baseline is incomplete or structurally invalid".into());
    }
    Ok(())
}
pub fn compare_baseline(
    b: &ApprovedBaseline,
    g: &SpecificationGraph,
    mappings: &BTreeMap<Identity, Identity>,
) -> Result<BaselineDiff, String> {
    validate_baseline(b)?;
    validate_mappings(mappings)?;
    let old = &b.graph.parsed;
    let new = &g.parsed;
    let mut changes = vec![];
    let mut used = BTreeSet::new();
    let oldkeys: BTreeSet<_> = old.requirements.iter().map(|r| r.key.clone()).collect();
    let newkeys: BTreeSet<_> = new.requirements.iter().map(|r| r.key.clone()).collect();
    // Reviewed mappings still must name real old/new identities and cannot displace an existing one.
    for (a, z) in mappings {
        if !oldkeys.contains(a)
            || !newkeys.contains(z)
            || (oldkeys.contains(z) && a != z)
            || (newkeys.contains(a) && a != z)
        {
            return Err("mapping does not describe a unique old/new identity".into());
        }
    }
    let mut complete = g.complete()
        && !crate::rules::validate_graph(g, &BTreeSet::new())
            .iter()
            .any(|f| f.kind == crate::rules::FindingKind::Duplicate);
    for before in &old.requirements {
        let target = mappings.get(&before.key).unwrap_or(&before.key);
        let candidates: Vec<_> = new
            .requirements
            .iter()
            .filter(|r| &r.key == target)
            .collect();
        if candidates.len() > 1 {
            complete = false;
            continue;
        }
        let Some(after) = candidates.first() else {
            changes.push(Change {
                key: before.key.clone(),
                kind: if g.complete() {
                    ChangeKind::Removed
                } else {
                    ChangeKind::UncertainRemoval
                },
            });
            continue;
        };
        used.insert(target.clone());
        if before.source.path != after.source.path || before.key != after.key {
            changes.push(Change {
                key: after.key.clone(),
                kind: ChangeKind::Moved,
            });
        }
        if before.text != after.text {
            changes.push(Change {
                key: after.key.clone(),
                kind: ChangeKind::TextReview,
            });
        }
        let oldrefs: BTreeSet<_> = old
            .edges
            .iter()
            .filter(|e| e.from == before.key)
            .map(|e| {
                (
                    e.relation.clone(),
                    mappings.get(&e.to).unwrap_or(&e.to).clone(),
                )
            })
            .collect();
        let newrefs: BTreeSet<_> = new
            .edges
            .iter()
            .filter(|e| e.from == after.key)
            .map(|e| (e.relation.clone(), e.to.clone()))
            .collect();
        if oldrefs != newrefs {
            changes.push(Change {
                key: after.key.clone(),
                kind: ChangeKind::ReferencesChanged,
            });
        }
        let oldacc: BTreeMap<_, _> = old
            .acceptances
            .iter()
            .filter(|a| a.requirement == before.key)
            .map(|a| (a.key.clone(), a))
            .collect();
        let newacc: BTreeMap<_, _> = new
            .acceptances
            .iter()
            .filter(|a| a.requirement == after.key)
            .map(|a| (a.key.clone(), a))
            .collect();
        for (key, a) in &oldacc {
            match newacc.get(key) {
                Some(z) if a.text != z.text => changes.push(Change {
                    key: key.clone(),
                    kind: ChangeKind::AcceptanceReview,
                }),
                None => changes.push(Change {
                    key: key.clone(),
                    kind: if g.complete() {
                        ChangeKind::AcceptanceRemoved
                    } else {
                        ChangeKind::UncertainRemoval
                    },
                }),
                _ => {}
            }
        }
        for key in newacc.keys() {
            if !oldacc.contains_key(key) {
                changes.push(Change {
                    key: key.clone(),
                    kind: ChangeKind::AcceptanceAdded,
                });
            }
        }
    }
    for after in &new.requirements {
        if !used.contains(&after.key) && !oldkeys.contains(&after.key) {
            changes.push(Change {
                key: after.key.clone(),
                kind: ChangeKind::Added,
            });
        }
    }
    changes.sort();
    changes.dedup();
    Ok(BaselineDiff { changes, complete })
}
pub fn compare_condition(before: &Condition, after: &Condition) -> Comparison {
    use Condition::*;
    let supported_unit = |unit: &str| matches!(unit, "count" | "ms" | "s" | "bytes");
    let order = match (before, after) {
        (IntegerMinimum { value: a, unit: u }, IntegerMinimum { value: b, unit: v })
            if u == v && supported_unit(u) =>
        {
            b.cmp(a)
        }
        (IntegerMaximum { value: a, unit: u }, IntegerMaximum { value: b, unit: v })
            if u == v && supported_unit(u) =>
        {
            a.cmp(b)
        }
        _ => return Comparison::Review,
    };
    match order {
        std::cmp::Ordering::Greater => Comparison::Tightened,
        std::cmp::Ordering::Less => Comparison::Relaxed,
        _ => Comparison::Equal,
    }
}
