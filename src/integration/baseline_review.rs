//! Opt-in local baseline comparison. No approval or policy authentication is implied.
use crate::{
    baseline::{ApprovedBaseline, BaselineDiff, ChangeKind, compare_baseline, validate_baseline},
    graph::SpecificationGraph,
    model::Identity,
};
use guardengine::{Enforcement, GuardAssertion, GuardContract, integration::FactBudget};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const PROFILE: &str = "specguard.baseline-review/v1";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Version {
    #[serde(rename = "specguard.baseline-review/v1")]
    V1,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReviewRule {
    pub key: Identity,
    pub kind: ChangeKind,
    pub rule_id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BaselineReview {
    pub api_version: Version,
    pub baseline: ApprovedBaseline,
    pub rules: Vec<ReviewRule>,
    pub contract: GuardContract,
}
impl BaselineReview {
    pub(crate) fn validate(
        &self,
        required: &BTreeSet<Identity>,
        repo_id: &str,
        baseline_digest: Option<&str>,
        structural: &super::producer::ProtectedMapping,
    ) -> Result<(), String> {
        // Library callers get the same bounded review-input ceiling as CLI files.
        struct Count(usize);
        impl std::io::Write for Count {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if bytes.len() > 1_048_576usize.saturating_sub(self.0) {
                    return Err(std::io::ErrorKind::InvalidInput.into());
                }
                self.0 += bytes.len();
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        serde_json::to_writer(Count(0), self).map_err(|_| "review input byte budget")?;
        if structural
            .contract
            .spec
            .rules
            .len()
            .saturating_add(self.contract.spec.rules.len())
            > 256
        {
            return Err("combined contract budget".into());
        }
        super::producer::preflight(self)?;
        let parsed = &self.baseline.graph.parsed;
        if parsed
            .requirements
            .len()
            .saturating_add(parsed.acceptances.len())
            .saturating_add(parsed.edges.len())
            .saturating_add(parsed.sources.len())
            > 512
            || self.rules.len() != required.len() * 2
            || self.contract.spec.rules.len() != self.rules.len()
        {
            return Err("review budget/scope".into());
        }
        if self.baseline.scope != *required
            || self.baseline.repository != repo_id
            || baseline_digest != Some(crate::model::digest(&self.baseline).as_str())
        {
            return Err("review baseline binding".into());
        }
        validate_baseline(&self.baseline)?;
        self.contract.validate().map_err(|e| e.to_string())?;
        let mut used = BTreeSet::new();
        let mut pairs = BTreeSet::new();
        let mut tuples = BTreeSet::new();
        for rule in &self.rules {
            if !required.contains(&rule.key)
                || !matches!(
                    rule.kind,
                    ChangeKind::TextReview | ChangeKind::AcceptanceReview
                )
                || !used.insert(&rule.rule_id)
                || !pairs.insert((&rule.key, &rule.kind))
                || !tuples.insert((&rule.subject, &rule.predicate, &rule.object))
                || rule.subject != super::producer::qualified(&rule.key)?
            {
                return Err("review mapping ambiguity".into());
            }
            if structural
                .contract
                .spec
                .rules
                .iter()
                .any(|r| r.id == rule.rule_id)
                || structural.entries.iter().any(|r| {
                    (&r.subject, &r.predicate, &r.object)
                        == (&rule.subject, &rule.predicate, &rule.object)
                })
            {
                return Err("review overlaps structural mapping".into());
            }
            let actual = self
                .contract
                .spec
                .rules
                .iter()
                .find(|r| r.id == rule.rule_id)
                .ok_or("review rule missing")?;
            let GuardAssertion::ForbidRelation {
                subject,
                predicate,
                object,
            } = &actual.assertion;
            if actual.enforcement != Enforcement::Review
                || (subject, predicate, object) != (&rule.subject, &rule.predicate, &rule.object)
            {
                return Err("review rule not exact".into());
            }
        }
        for key in required {
            for kind in [ChangeKind::TextReview, ChangeKind::AcceptanceReview] {
                if !pairs.contains(&(key, &kind)) {
                    return Err("review coverage missing".into());
                }
            }
        }
        Ok(())
    }
    pub(crate) fn project(
        &self,
        graph: &SpecificationGraph,
    ) -> Result<(BaselineDiff, Vec<guardengine::GuardFact>), String> {
        let diff = compare_baseline(&self.baseline, graph, &BTreeMap::new())?;
        if !diff.complete {
            return Err("incomplete baseline comparison".into());
        }
        let mut budget = FactBudget::new();
        for change in &diff.changes {
            // This version supports exact text/acceptance review only. Other changes
            // cannot silently fall through to ALLOW.
            let key = if change.kind == ChangeKind::AcceptanceReview {
                let old = self
                    .baseline
                    .graph
                    .parsed
                    .acceptances
                    .iter()
                    .find(|a| a.key == change.key)
                    .ok_or("missing baseline acceptance")?;
                let current = graph
                    .parsed
                    .acceptances
                    .iter()
                    .find(|a| a.key == change.key)
                    .ok_or("missing candidate acceptance")?;
                if old.requirement != current.requirement {
                    return Err("acceptance scope changed".into());
                }
                &old.requirement
            } else {
                &change.key
            };
            let mapping = self
                .rules
                .iter()
                .find(|r| &r.key == key && r.kind == change.kind)
                .ok_or("unsupported baseline change")?;
            budget
                .push_relation(
                    &mapping.subject,
                    &mapping.predicate,
                    &mapping.object,
                    "frozen-baseline-comparison",
                )
                .map_err(|e| e.to_string())?;
        }
        Ok((diff, budget.finish().map_err(|e| e.to_string())?))
    }
}
