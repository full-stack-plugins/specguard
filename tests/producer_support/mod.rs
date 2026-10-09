use crate::common::key;
use guardengine::{
    ContractMetadata, ContractSpec, Enforcement, GuardAssertion, GuardContract, GuardRule,
};
use specguard::integration::producer::*;
use specguard::rules::FindingKind;
pub fn policy() -> ProtectedMapping {
    let kinds = [
        FindingKind::Duplicate,
        FindingKind::MissingRequirement,
        FindingKind::MissingAcceptance,
        FindingKind::BrokenReference,
    ];
    let mut entries = vec![];
    let mut rules = vec![];
    for (index, kind) in kinds.into_iter().enumerate() {
        let rule_id = format!("rule-{index}");
        let object = format!("finding-{index}");
        entries.push(MappingEntry {
            key: key("R1"),
            kind,
            rule_id: rule_id.clone(),
            subject: "demo:R1".into(),
            predicate: "spec_violation".into(),
            object: object.clone(),
        });
        rules.push(GuardRule {
            id: rule_id,
            description: "fixture protected mapping".into(),
            enforcement: Enforcement::Enforce,
            assertion: GuardAssertion::ForbidRelation {
                subject: "demo:R1".into(),
                predicate: "spec_violation".into(),
                object,
            },
        });
    }
    ProtectedMapping {
        contract: GuardContract {
            api_version: guardengine::API_VERSION.into(),
            kind: "GuardContract".into(),
            metadata: ContractMetadata {
                id: "protected".into(),
                revision: "1".into(),
            },
            spec: ContractSpec { rules },
        },
        entries,
    }
}
pub fn invocation(snapshot: &specguard::source::SourceSnapshot) -> Invocation {
    Invocation {
        run_id: "sg-attempt-1".into(),
        repo_id: "fixture-repository".into(),
        task_id: "task-1".into(),
        worktree_id: "worktree-1".into(),
        candidate_oid: Some(snapshot.binding.candidate_oid.clone()),
        base_oid: Some(snapshot.binding.base_oid.clone()),
        merge_group_id: None,
        baseline_digest: None,
        started_at: "2026-10-09T10:00:00Z".into(),
    }
}
