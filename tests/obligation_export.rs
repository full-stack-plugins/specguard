mod common;
use common::*;
use specguard::{integration::approval::*, model::*, obligations::*};
#[test]
fn export_binds_source_baseline_candidate_and_never_asserts_test_success() {
    let b = baseline();
    let verified = authenticate(&b, &fixture_approval(&b), Profile::Fixture, 50).unwrap();
    let e = export_obligations(&b.graph, &verified, &b.scope, &b.source_digest).unwrap();
    assert!(e.complete);
    assert_eq!(e.obligations.len(), 1);
    assert_eq!(e.obligations[0].acceptance, key("A1"));
    assert_eq!(e.candidate_oid, b.source_revision);
    assert!(!serde_json::to_string(&e).unwrap().contains("testPassed"));
    assert!(export_obligations(&b.graph, &verified, &b.scope, "drift").is_err());
    let mut g = b.graph.clone();
    g.parsed.sources[0].status = Terminal::Malformed;
    assert!(
        !export_obligations(&g, &verified, &b.scope, &b.source_digest)
            .unwrap()
            .complete
    );
    g.parsed.sources[0].status = Terminal::Complete;
    g.parsed.acceptances.clear();
    assert!(
        !export_obligations(&g, &verified, &b.scope, &b.source_digest)
            .unwrap()
            .complete
    );
}
#[test]
fn removing_frozen_acceptance_cannot_shrink_the_plan() {
    let b = baseline();
    let verified = authenticate(&b, &fixture_approval(&b), Profile::Fixture, 50).unwrap();
    let mut g = b.graph.clone();
    g.parsed.acceptances[0].key = key("new-acceptance");
    assert!(
        !export_obligations(&g, &verified, &b.scope, &b.source_digest)
            .unwrap()
            .complete
    );
}
