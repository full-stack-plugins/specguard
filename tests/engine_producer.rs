mod common;
use common::*;
use guardengine::integration::{CoverageStatus, RunStatus, verify_engine_artifacts};
use guardengine::{Decision, Enforcement};
use specguard::integration::producer::*;
use std::collections::BTreeSet;

mod producer_support;
use producer_support::{invocation, policy};
const FINISHED: &str = "2026-10-09T10:00:01Z";
#[test]
fn actual_snapshot_produces_engine_verified_allow_and_exact_mapped_block() {
    for (text, expected) in [(document().to_owned(),Decision::Allow),("---\nformat: markdown-explicit/v1\nnamespace: demo\n---\n## Requirement: R1\nMust work.\n".into(),Decision::Block)] {
        let (root,snapshot) = snapshot(text.as_bytes());
        let prepared = prepare(root.path(), &snapshot, invocation(&snapshot), &BTreeSet::from([key("R1")]), &policy()).unwrap();
        let result = prepared.complete(FINISHED).unwrap();
        assert_eq!(result.envelope.run_status,RunStatus::Completed);
        assert_eq!(result.envelope.decision,Some(expected));
        result.verify().unwrap();
        verify_engine_artifacts(&result.envelope,result.contract.as_ref().unwrap(),result.facts.as_ref().unwrap(),result.report.as_ref().unwrap()).unwrap();
        assert!(!result.domain.as_ref().unwrap().is_empty());
    }
}
#[test]
fn missing_binding_and_incomplete_or_mismatched_policy_never_get_an_envelope() {
    let (root, snapshot) = snapshot(document().as_bytes());
    let required = BTreeSet::from([key("R1")]);
    let mut request = invocation(&snapshot);
    request.candidate_oid = None;
    let error = prepare(root.path(), &snapshot, request, &required, &policy())
        .err()
        .unwrap();
    assert!(
        serde_json::to_value(error)
            .unwrap()
            .get("binding")
            .is_none()
    );
    for case in 0..4 {
        let mut mapping = policy();
        match case {
            0 => {
                mapping.entries.pop();
            }
            1 => mapping.entries[0].object = "mismatch".into(),
            2 => mapping.contract.spec.rules[0].enforcement = Enforcement::Advise,
            _ => {
                mapping.entries.push(mapping.entries[0].clone());
            }
        }
        assert!(
            prepare(
                root.path(),
                &snapshot,
                invocation(&snapshot),
                &required,
                &mapping
            )
            .is_err()
        );
    }
}
#[test]
fn malformed_source_is_completed_partial_block_but_io_failure_and_cancel_are_not_decisions() {
    let (root, snapshot) = snapshot(b"not valid format");
    let required = BTreeSet::from([key("R1")]);
    let result = prepare(
        root.path(),
        &snapshot,
        invocation(&snapshot),
        &required,
        &policy(),
    )
    .unwrap()
    .complete(FINISHED)
    .unwrap();
    assert_eq!(result.envelope.run_status, RunStatus::Completed);
    assert_eq!(result.envelope.coverage.status, CoverageStatus::Partial);
    assert_eq!(result.envelope.decision, Some(Decision::Block));
    result.verify().unwrap();
    let cancelled = prepare(
        root.path(),
        &snapshot,
        invocation(&snapshot),
        &required,
        &policy(),
    )
    .unwrap()
    .cancel(FINISHED)
    .unwrap();
    assert_eq!(cancelled.envelope.run_status, RunStatus::Cancelled);
    assert!(cancelled.envelope.decision.is_none() && cancelled.report.is_none());
    let failed = prepare(
        root.path(),
        &snapshot,
        invocation(&snapshot),
        &required,
        &policy(),
    )
    .unwrap()
    .fail(FINISHED)
    .unwrap();
    assert_eq!(failed.envelope.run_status, RunStatus::Error);
    assert!(failed.envelope.decision.is_none() && failed.report.is_none());
}
#[test]
fn frozen_policy_and_snapshot_cannot_be_weakened_after_preparation() {
    let (root, mut snapshot) = snapshot(
        b"---\nformat: markdown-explicit/v1\nnamespace: demo\n---\n## Requirement: R1\nrequired\n",
    );
    let mut required = BTreeSet::from([key("R1")]);
    let mut mapping = policy();
    let prepared = prepare(
        root.path(),
        &snapshot,
        invocation(&snapshot),
        &required,
        &mapping,
    )
    .unwrap();
    required.clear();
    mapping.entries.clear();
    snapshot.contents.clear();
    let output = prepared.complete(FINISHED).unwrap();
    assert_eq!(output.envelope.decision, Some(Decision::Block));
    assert_eq!(output.envelope.binding.requirement_ids, vec!["demo:R1"]);
    output.verify().unwrap();
}
#[test]
fn altered_artifact_decision_and_domain_digest_are_rejected() {
    let (root, snapshot) = snapshot(document().as_bytes());
    let output = prepare(
        root.path(),
        &snapshot,
        invocation(&snapshot),
        &BTreeSet::from([key("R1")]),
        &policy(),
    )
    .unwrap()
    .complete(FINISHED)
    .unwrap();
    for case in 0..4 {
        let mut bad = output.clone();
        match case {
            0 => bad.envelope.decision = Some(Decision::Block),
            1 => bad.facts.as_mut().unwrap().push(b' '),
            2 => bad.domain.as_mut().unwrap().push(b' '),
            _ => {
                bad.report = None;
            }
        }
        assert!(bad.verify().is_err());
    }
}

#[test]
fn domain_with_rehashed_but_different_snapshot_is_not_consistent_evidence() {
    use sha2::{Digest, Sha256};
    let (root, snapshot) = snapshot(document().as_bytes());
    let mut output = prepare(
        root.path(),
        &snapshot,
        invocation(&snapshot),
        &BTreeSet::from([key("R1")]),
        &policy(),
    )
    .unwrap()
    .complete(FINISHED)
    .unwrap();
    let mut domain: serde_json::Value =
        serde_json::from_slice(output.domain.as_ref().unwrap()).unwrap();
    domain["graph"]["parsed"]["snapshotDigest"] =
        serde_json::json!(format!("sha256:{}", "a".repeat(64)));
    let bytes = serde_json::to_vec(&domain).unwrap();
    output.envelope.artifacts.domain[0].digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    output.domain = Some(bytes);
    assert!(output.verify().is_err());
}

#[test]
fn actual_io_terminal_and_unmapped_finding_fail_after_binding_without_old_report() {
    let root = repo();
    let inventory = specguard::source::discover(root.path(), &common::policy()).unwrap();
    let frozen = specguard::source::freeze(root.path(), &inventory, binding(root.path())).unwrap();
    let output = prepare(
        root.path(),
        &frozen,
        invocation(&frozen),
        &BTreeSet::from([key("R1")]),
        &policy(),
    )
    .unwrap()
    .complete(FINISHED)
    .unwrap();
    assert_eq!(output.envelope.run_status, RunStatus::Error);
    assert!(output.envelope.decision.is_none() && output.report.is_none());
    let text = format!("{}\n### Acceptance: A1\nAnother acceptance\n", document());
    let (root, frozen) = snapshot(text.as_bytes());
    let output = prepare(
        root.path(),
        &frozen,
        invocation(&frozen),
        &BTreeSet::from([key("R1")]),
        &policy(),
    )
    .unwrap()
    .complete(FINISHED)
    .unwrap();
    assert_eq!(output.envelope.run_status, RunStatus::Error);
    assert_eq!(output.envelope.diagnostics[0].code, "projection.failed");
    assert!(output.report.is_none());
}

#[test]
fn strict_mapping_and_envelope_shapes_and_borrowed_oversize_input_fail_closed() {
    let mut value = serde_json::to_value(policy()).unwrap();
    value["approval"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ProtectedMapping>(value).is_err());
    let mut value = serde_json::to_value(policy()).unwrap();
    value["contract"]["spec"]["rules"][0]["assertion"]["type"] = serde_json::json!("execute");
    assert!(serde_json::from_value::<ProtectedMapping>(value).is_err());
    let (root, mut frozen) = snapshot(document().as_bytes());
    let output = prepare(
        root.path(),
        &frozen,
        invocation(&frozen),
        &BTreeSet::from([key("R1")]),
        &policy(),
    )
    .unwrap()
    .complete(FINISHED)
    .unwrap();
    for field in ["apiVersion", "unexpected"] {
        let mut value = serde_json::to_value(&output.envelope).unwrap();
        value[field] = serde_json::json!("unknown");
        assert!(
            guardengine::integration::load_envelope_json(
                &serde_json::to_vec(&value).unwrap(),
                guardengine::integration::EvidenceProfile::EngineBacked
            )
            .is_err()
        );
    }
    frozen.inventory.sources[0].reason = "x".repeat(guardengine::integration::MAX_ARTIFACT_BYTES);
    let result = prepare(
        root.path(),
        &frozen,
        invocation(&frozen),
        &BTreeSet::from([key("R1")]),
        &policy(),
    );
    assert_eq!(result.err().unwrap().code, "input.budget");
}
