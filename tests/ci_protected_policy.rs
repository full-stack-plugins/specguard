#[path = "../examples/ci/mod.rs"]
mod ci;
use guardengine::{
    Decision,
    integration::{CoverageStatus, eligibility::EligibilityCode},
};
#[test]
fn protected_actual_git_job_can_qualify_but_candidate_rules_cannot_weaken_it() {
    let good = ci::exercise(ci::Scenario::Good);
    assert!(good.unchanged);
    assert_eq!(good.repository_before, good.repository_after);
    assert_eq!(
        good.evaluate(&good.bundle.output().envelope).code,
        EligibilityCode::Eligible
    );
    let bad = ci::exercise(ci::Scenario::CandidateWeakensRules);
    assert!(bad.unchanged);
    let r = bad.evaluate(&bad.bundle.output().envelope);
    assert_eq!(r.technical_decision, Some(Decision::Block));
    assert_eq!(r.code, EligibilityCode::TechnicalBlock);
    assert_eq!(good.expected.contract_digest, bad.expected.contract_digest);
}
#[test]
fn current_identity_exact_binding_and_full_coverage_are_required() {
    let mut run = ci::exercise(ci::Scenario::Good);
    let original = run.bundle.output().envelope.clone();
    assert_eq!(run.evaluate(&original).code, EligibilityCode::Eligible);
    let mut forged = original.clone();
    forged.run_id = "unissued-run".into();
    assert_eq!(
        run.evaluate(&forged).code,
        EligibilityCode::UntrustedProducer
    );
    let mut wrong_producer = original.clone();
    wrong_producer.producer.analyzer_version = "unapproved-version".into();
    assert!(!run.evaluate(&wrong_producer).eligible);
    let mut wrong_contract = original.clone();
    wrong_contract.artifacts.contract.as_mut().unwrap().digest =
        format!("sha256:{}", "f".repeat(64));
    assert!(!run.evaluate(&wrong_contract).eligible);
    run.revoked = true;
    assert!(!run.evaluate(&original).eligible);
    run.revoked = false;
    run.unavailable = true;
    assert_eq!(
        run.evaluate(&original).code,
        EligibilityCode::ProviderUnavailable
    );
    run.unavailable = false;
    for field in ["candidateOid", "baseOid", "sourceSnapshotDigest"] {
        let mut value = serde_json::to_value(&original).unwrap();
        value["binding"][field] = if field == "sourceSnapshotDigest" {
            format!("sha256:{}", "f".repeat(64)).into()
        } else {
            "f".repeat(40).into()
        };
        let altered = serde_json::from_value(value).unwrap();
        assert_eq!(run.evaluate(&altered).code, EligibilityCode::BindingChanged);
    }
    let mut missing = original.clone();
    missing.coverage.status = CoverageStatus::Partial;
    missing.coverage.missing_scopes = vec![missing.coverage.observed_scopes.pop().unwrap()];
    assert!(!run.evaluate(&missing).eligible);
    let mut weakened = missing.clone();
    weakened.coverage.required_scopes = weakened.coverage.observed_scopes.clone();
    weakened.coverage.missing_scopes.clear();
    weakened.coverage.status = CoverageStatus::Complete;
    assert!(!run.evaluate(&weakened).eligible);
    assert_eq!(run.evaluate(&original).code, EligibilityCode::Eligible);
    let partial = ci::exercise(ci::Scenario::Partial);
    assert!(!partial.evaluate(&partial.bundle.output().envelope).eligible);
    assert!(partial.unchanged);
    assert_eq!(ci::ADAPTER, "specguard.example.local-ci/v1");
}
