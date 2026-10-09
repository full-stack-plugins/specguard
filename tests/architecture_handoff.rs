mod common;
mod trace_support;
use common::*;
use specguard::{architecture::*, integration::approval::*, model::*};
use std::collections::BTreeSet;
fn exported() -> ArchitectureHandoff {
    let (_, snapshot, _) = trace_support::fixture(trace_support::ADR);
    let baseline = baseline();
    let approved = authenticate(
        &baseline,
        &fixture_approval(&baseline),
        Profile::Fixture,
        50,
    )
    .unwrap();
    export(&snapshot, &approved, &BTreeSet::from([key("R1")])).unwrap()
}
fn context(h: &ArchitectureHandoff, bytes: &[u8]) -> ExpectedHandoff {
    ExpectedHandoff {
        artifact_digest: artifact_digest(bytes),
        repository: h.repository.clone(),
        candidate_binding: h.candidate_binding.clone(),
        baseline_digest: h.baseline_digest.clone(),
        source_digest: h.source_digest.clone(),
        scope: h.scope.clone(),
        authentication_profile: AuthenticationProfile::FixtureOnly,
    }
}
#[test]
fn real_source_export_round_trips_frozen_baseline_and_exact_adr_locations() {
    let h = exported();
    let bytes = encode(&h).unwrap();
    assert_eq!(decode(&bytes, &context(&h, &bytes)).unwrap(), h);
    assert_eq!(h.authentication_profile, AuthenticationProfile::FixtureOnly);
    assert_eq!(h.architecture_references.len(), 1);
    assert_eq!(h.architecture_references[0].requirement, key("R1"));
    assert_eq!(h.architecture_references[0].adr, key("ADR1"));
    assert_eq!(h.architecture_references[0].adr_source.path, "adr.md");
    assert_eq!(h.baseline_digest, digest(&h.baseline));
}
#[test]
fn rejects_partial_real_source_and_uncovered_scope() {
    let (_, snapshot, _) = trace_support::fixture("bad ADR");
    let b = baseline();
    let a = authenticate(&b, &fixture_approval(&b), Profile::Fixture, 50).unwrap();
    assert!(export(&snapshot, &a, &BTreeSet::from([key("R1")])).is_err());
    assert!(export(&snapshot, &a, &BTreeSet::from([key("R2")])).is_err());
}
#[test]
fn pins_external_context_and_recomputes_references_instead_of_trusting_serialized_claims() {
    let h = exported();
    let bytes = encode(&h).unwrap();
    for field in 0..6 {
        let mut expected = context(&h, &bytes);
        match field {
            0 => expected.repository.push('x'),
            1 => expected.candidate_binding.base_oid = "b".repeat(40),
            2 => expected.baseline_digest.push('x'),
            3 => expected.source_digest.push('x'),
            4 => {
                expected.scope.insert(key("R2"));
            }
            _ => expected.artifact_digest.push('x'),
        };
        assert!(decode(&bytes, &expected).is_err());
    }
    let mut forged = h.clone();
    forged.architecture_references[0].adr = key("forged");
    let forged_bytes = serde_json::to_vec(&forged).unwrap();
    assert!(decode(&forged_bytes, &context(&forged, &forged_bytes)).is_err());
    let mut forged = h;
    forged.baseline.graph.parsed.requirements[0].text.push('x');
    let forged_bytes = serde_json::to_vec(&forged).unwrap();
    assert!(decode(&forged_bytes, &context(&forged, &forged_bytes)).is_err());
}
#[test]
fn strict_version_profile_fields_and_budget() {
    let h = exported();
    let original = serde_json::to_value(&h).unwrap();
    for (field, value) in [
        ("apiVersion", serde_json::json!("unknown")),
        ("authenticationProfile", serde_json::json!("production")),
        ("trusted", serde_json::json!(true)),
    ] {
        let mut v = original.clone();
        v[field] = value;
        let bytes = serde_json::to_vec(&v).unwrap();
        assert!(decode(&bytes, &context(&h, &bytes)).is_err());
    }
    assert!(decode(&vec![b' '; MAX_HANDOFF_BYTES + 1], &context(&h, b"")).is_err());
}

#[test]
fn rejects_rehashed_invalid_candidate_identity_and_forged_source_locations() {
    let mut h = exported();
    h.snapshot.binding.candidate_oid = "not-a-git-object".into();
    h.candidate_binding = h.snapshot.binding.clone();
    h.snapshot.digest = digest(&(
        &h.snapshot.inventory,
        &h.snapshot.binding,
        &h.snapshot.contents,
    ));
    h.source_digest = h.snapshot.digest.clone();
    h.trace = specguard::trace::scan(&h.snapshot, &h.scope).unwrap();
    assert!(encode(&h).is_err());
    let mut h = exported();
    h.trace.graph.targets.nodes[0].source.line += 1;
    let bytes = serde_json::to_vec(&h).unwrap();
    assert!(decode(&bytes, &context(&h, &bytes)).is_err());
}
