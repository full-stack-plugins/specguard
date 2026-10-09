mod common;
mod producer_support;
use common::*;
use guardengine::integration::{EvidenceProfile, RunStatus};
use producer_support::*;
use specguard::integration::producer::prepare;
use std::collections::BTreeSet;
#[test]
fn invalid_finish_after_binding_recovers_error_without_successful_payloads() {
    let (root, snapshot) = snapshot(document().as_bytes());
    let original = invocation(&snapshot);
    let output = prepare(
        root.path(),
        &snapshot,
        original,
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
    )
    .unwrap()
    .complete("not-a-time")
    .unwrap();
    assert_eq!(output.envelope.run_status, RunStatus::Error);
    assert!(output.envelope.decision.is_none() && output.report.is_none());
    assert_eq!(
        output.envelope.binding.candidate_oid,
        snapshot.binding.candidate_oid
    );
    assert_eq!(output.envelope.diagnostics[0].code, "runtime.finalization");
    output
        .envelope
        .validate(EvidenceProfile::EngineBacked)
        .unwrap();
    output.verify().unwrap();
}

#[test]
fn invalid_finish_on_explicit_fail_or_cancel_also_has_bound_recovery() {
    for cancel in [false, true] {
        let (root, snapshot) = snapshot(document().as_bytes());
        let prepared = prepare(
            root.path(),
            &snapshot,
            invocation(&snapshot),
            &BTreeSet::from([key("R1")]),
            &producer_support::policy(),
        )
        .unwrap();
        let output = if cancel {
            prepared.cancel("invalid")
        } else {
            prepared.fail("invalid")
        }
        .unwrap();
        assert!(output.envelope.decision.is_none() && output.report.is_none());
        assert!(output.domain.is_some());
        output.verify().unwrap();
    }
}

#[test]
fn original_finalization_diagnostic_is_preserved_with_fallback_time_marker() {
    let (root, snapshot) = snapshot(document().as_bytes());
    let output = prepare(
        root.path(),
        &snapshot,
        invocation(&snapshot),
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
    )
    .unwrap()
    .complete("invalid")
    .unwrap();
    let diagnostic: serde_json::Value =
        serde_json::from_slice(output.domain.as_ref().unwrap()).unwrap();
    assert_eq!(diagnostic["originalTransport"]["code"], "attempt.invalid");
    assert_eq!(diagnostic["finishTimeFallback"], true);
    assert_eq!(output.envelope.finished_at, output.envelope.started_at);
}

#[test]
fn oversized_finish_metadata_is_rejected_before_worker_execution() {
    let (root, snapshot) = snapshot(document().as_bytes());
    let prepared = prepare(
        root.path(),
        &snapshot,
        invocation(&snapshot),
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
    )
    .unwrap();
    let mut observed = false;
    let output = prepared
        .execute(
            &"x".repeat(5000),
            &specguard::integration::runtime::CancellationToken::new(),
            |_| observed = true,
        )
        .unwrap();
    assert!(!observed);
    let diagnostic: serde_json::Value =
        serde_json::from_slice(output.domain.as_ref().unwrap()).unwrap();
    assert_eq!(diagnostic["originalTransport"]["code"], "finish.budget");
}
