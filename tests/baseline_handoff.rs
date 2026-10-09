mod common;
use common::*;
use specguard::{baseline::*, integration::approval::*, model::*, obligations::*};
#[test]
fn fixture_consumer_reads_frozen_golden_handoff() {
    let b = decode_baseline(
        serde_json::from_str(include_str!("../fixtures/handoffs/baseline.json")).unwrap(),
    )
    .unwrap();
    let expected: ObligationSet =
        serde_json::from_str(include_str!("../fixtures/handoffs/obligations.json")).unwrap();
    let verified = authenticate(&b, &fixture_approval(&b), Profile::Fixture, 50).unwrap();
    let actual = export_obligations(&b.graph, &verified, &b.scope, &b.source_digest).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual.baseline_digest, digest(&b));
    assert_eq!(actual.authentication_profile, "fixture-only");
    // Consumer simulation, not actual TestGuard or ArchGuard execution.
    assert_eq!(actual.obligations[0].requirement, key("R1"));
    assert_eq!(actual.obligations[0].acceptance, key("A1"));
}

#[test]
fn generated_architecture_handoff_shares_immutable_baseline_with_testguard_obligations() {
    use specguard::architecture::*;
    let bytes = include_bytes!("../fixtures/handoffs/architecture/handoff.json");
    let provenance: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/handoffs/architecture/provenance.json"
    ))
    .unwrap();
    let baseline = decode_baseline(
        serde_json::from_str(include_str!("../fixtures/handoffs/baseline.json")).unwrap(),
    )
    .unwrap();
    let obligations: ObligationSet =
        serde_json::from_str(include_str!("../fixtures/handoffs/obligations.json")).unwrap();
    let shape: ArchitectureHandoff = serde_json::from_slice(bytes).unwrap();
    // This is a committed fixture consumer, not an authenticated controller.
    let expected = ExpectedHandoff {
        artifact_digest: provenance["artifactDigest"].as_str().unwrap().into(),
        repository: baseline.repository.clone(),
        candidate_binding: specguard::source::CandidateBinding {
            candidate_oid: provenance["candidateOid"].as_str().unwrap().into(),
            base_oid: provenance["candidateOid"].as_str().unwrap().into(),
            object_format: "sha1".into(),
        },
        baseline_digest: digest(&baseline),
        source_digest: shape.source_digest.clone(), // explicitly fixture-local, not a trust claim
        scope: baseline.scope.clone(),
        authentication_profile: AuthenticationProfile::FixtureOnly,
    };
    let h = decode(bytes, &expected).unwrap();
    assert_eq!(h.baseline, baseline);
    assert_eq!(h.baseline_digest, obligations.baseline_digest);
    assert_eq!(
        h.architecture_references[0].requirement,
        obligations.obligations[0].requirement
    );
    assert_eq!(h.architecture_references[0].adr, key("ADR1"));
}
