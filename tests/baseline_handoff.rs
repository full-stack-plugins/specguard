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
