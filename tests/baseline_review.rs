mod common;
mod producer_support;
mod review_support;
use common::*;
use specguard::integration::{
    baseline_review::BaselineReview,
    producer::{Invocation, prepare_baseline_review},
};
use std::collections::BTreeSet;
fn inputs(
    root: &std::path::Path,
    s: &specguard::source::SourceSnapshot,
) -> (BaselineReview, Invocation) {
    let p = review_support::review_request(root, s);
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
    (
        serde_json::from_value(v["review"].clone()).unwrap(),
        serde_json::from_value(v["invocation"].clone()).unwrap(),
    )
}
#[test]
fn review_policy_and_baseline_are_frozen_before_execution_and_verification_recomputes_diff() {
    let (root, s) = snapshot(document().replace("sign in", "sign out").as_bytes());
    let (mut review, invocation) = inputs(root.path(), &s);
    let required = BTreeSet::from([key("R1")]);
    let policy = producer_support::policy();
    let prepared = prepare_baseline_review(
        root.path(),
        &s,
        serde_json::from_value(serde_json::to_value(&invocation).unwrap()).unwrap(),
        &required,
        &policy,
        &review,
    )
    .unwrap();
    let original = prepared.work_key().clone();
    review.contract.metadata.revision = "new-policy".into();
    // Reuse the same frozen baseline; a second helper call creates another Git
    // commit and can change the baseline digest across a wall-clock second.

    let changed =
        prepare_baseline_review(root.path(), &s, invocation, &required, &policy, &review).unwrap();
    assert_ne!(original, *changed.work_key());
    let mut output = prepared.complete("2026-10-09T10:00:01Z").unwrap();
    output.verify().unwrap();
    assert_eq!(
        output.envelope.decision,
        Some(guardengine::Decision::RequireApproval)
    );
    let mut domain: serde_json::Value =
        serde_json::from_slice(output.domain.as_ref().unwrap()).unwrap();
    domain["baselineDiff"]["changes"] = serde_json::json!([]);
    let bytes = serde_json::to_vec(&domain).unwrap();
    use sha2::{Digest, Sha256};
    output.envelope.artifacts.domain[0].digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    output.domain = Some(bytes);
    assert!(output.verify().is_err());
}
#[test]
fn oversized_review_and_unsupported_change_fail_closed() {
    let (root, s) = snapshot(document().replace("A1", "A2").as_bytes());
    let (mut review, invocation) = inputs(root.path(), &s);
    let required = BTreeSet::from([key("R1")]);
    let policy = producer_support::policy();
    let output = prepare_baseline_review(root.path(), &s, invocation, &required, &policy, &review)
        .unwrap()
        .complete("2026-10-09T10:00:01Z")
        .unwrap();
    assert_eq!(
        output.envelope.run_status,
        guardengine::integration::RunStatus::Error
    );
    assert!(output.envelope.decision.is_none());
    output.verify().unwrap();
    review.contract.metadata.revision = "x".repeat(1_048_577);
    let (_, invocation) = inputs(root.path(), &s);
    assert!(
        prepare_baseline_review(root.path(), &s, invocation, &required, &policy, &review).is_err()
    );
}
