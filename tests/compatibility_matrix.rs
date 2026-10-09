mod common;
mod producer_support;
use common::*;
use specguard::{integration::producer::prepare, model::Terminal, source::*};
use std::collections::BTreeSet;
#[test]
fn frozen_source_versions_are_exercised_through_real_engine_projection() {
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/compatibility/matrix.json")).unwrap();
    for row in rows["sources"].as_array().unwrap() {
        let format = row["format"].as_str().unwrap();
        let text = match row["fixture"].as_str().unwrap() {
            "markdown" => document(),
            "explicit" => include_str!("../fixtures/source-versions/openspec-explicit-v1.md"),
            "native-main" => {
                include_str!("../fixtures/source-versions/openspec-1.14.1/main/spec.md")
            }
            "native-added" => include_str!(
                "../fixtures/source-versions/openspec-1.14.1/change/specs/session/spec.md"
            ),
            _ => panic!("unqualified fixture"),
        };
        let root = repo();
        std::fs::create_dir(root.path().join("specs")).unwrap();
        std::fs::write(root.path().join("specs/a.md"), text).unwrap();
        let mut config = policy();
        config.roots[0].format = format.into();
        if row["registry"].as_bool().unwrap() {
            std::fs::write(
                root.path().join("ids.json"),
                include_str!("../fixtures/source-versions/openspec-1.14.1/identities.json"),
            )
            .unwrap();
            config.roots.push(SourceRoot {
                path: "ids.json".into(),
                format: "openspec-identities/v1".into(),
                namespace: "demo".into(),
                authority: "primary".into(),
            });
        }
        let s = freeze(
            root.path(),
            &discover(root.path(), &config).unwrap(),
            binding(root.path()),
        )
        .unwrap();
        let parsed = specguard::parser::parse(&s);
        let complete = parsed
            .sources
            .iter()
            .all(|s| s.status == Terminal::Complete);
        assert_eq!(complete, row["complete"].as_bool().unwrap(), "{format}");
        let output = prepare(
            root.path(),
            &s,
            producer_support::invocation(&s),
            &BTreeSet::from([key("R1")]),
            &producer_support::policy(),
        )
        .unwrap()
        .complete("2026-10-09T10:00:01Z")
        .unwrap();
        output.verify().unwrap();
        assert_eq!(
            output.envelope.decision,
            Some(if complete {
                guardengine::Decision::Allow
            } else {
                guardengine::Decision::Block
            }),
            "{format}"
        );
    }
}
#[test]
fn actual_review_golden_and_unknown_or_stronger_profiles_fail_closed() {
    use guardengine::integration::{EvidenceProfile, load_envelope_json};
    let output: specguard::integration::producer::ProducedRun =
        serde_json::from_slice(include_bytes!("../fixtures/cli-review/bundle.json")).unwrap();
    output.verify().unwrap();
    assert_eq!(
        output.envelope.decision,
        Some(guardengine::Decision::RequireApproval)
    );
    let mut value = serde_json::to_value(&output.envelope).unwrap();
    value["apiVersion"] = "guard.integration/future".into();
    assert!(
        load_envelope_json(
            &serde_json::to_vec(&value).unwrap(),
            EvidenceProfile::EngineBacked
        )
        .is_err()
    );
    let mut value = serde_json::to_value(&output.envelope).unwrap();
    value["artifacts"]["contract"] = serde_json::Value::Null;
    assert!(
        load_envelope_json(
            &serde_json::to_vec(&value).unwrap(),
            EvidenceProfile::EngineBacked
        )
        .is_err()
    );
    let mut missing = output.clone();
    missing.envelope.coverage.observed_scopes.clear();
    assert!(missing.verify().is_err());
    let mut stronger = output;
    let mut domain: serde_json::Value =
        serde_json::from_slice(stronger.domain.as_ref().unwrap()).unwrap();
    domain["profile"] = "specguard.production-authenticated/v1".into();
    let bytes = serde_json::to_vec(&domain).unwrap();
    use sha2::{Digest, Sha256};
    stronger.envelope.artifacts.domain[0].digest = format!("sha256:{:x}", Sha256::digest(&bytes));
    stronger.domain = Some(bytes);
    assert!(stronger.verify().is_err());
}
