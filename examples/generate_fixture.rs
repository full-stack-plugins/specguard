//! Explicit local simulation only. No production approval is issued.
use specguard::{baseline::*, graph::*, integration::approval::*, model::*, obligations::*};
use std::collections::BTreeSet;
struct Fixture;
impl ApprovalValidationPort for Fixture {
    fn profile(&self) -> Profile {
        Profile::Fixture
    }
    fn validate(&self, b: &ApprovedBaseline) -> Result<Authentication, ApprovalError> {
        Ok(Authentication {
            issuer: "fixture-controller".into(),
            purpose: "specification-baseline".into(),
            repository: b.repository.clone(),
            scope: b.scope.clone(),
            baseline_digest: digest(b),
            policy_digest: b.policy_digest.clone(),
            issued_at: 10,
            expires_at: 100,
            revoked: false,
        })
    }
}
fn main() {
    let key = |id: &str| Identity {
        namespace: "demo".into(),
        id: id.into(),
    };
    let source = SourceRef {
        path: "specs/a.md".into(),
        line: 5,
    };
    let g = build_graph(ParseResult {
        api_version: Version::V1,
        snapshot_digest: format!("sha256:{}", "1".repeat(64)),
        candidate_oid: "2".repeat(40),
        sources: vec![SourceStatus {
            path: "specs/a.md".into(),
            status: Terminal::Complete,
            reason: "parsed".into(),
        }],
        requirements: vec![Requirement {
            key: key("R1"),
            text: "A user can sign in.".into(),
            source: source.clone(),
        }],
        acceptances: vec![Acceptance {
            key: key("A1"),
            requirement: key("R1"),
            text: "A correct password opens a session.".into(),
            source: SourceRef { line: 7, ..source },
        }],
        edges: vec![],
    });
    let b = ApprovedBaseline {
        api_version: Version::V1,
        repository: "fixture-repo".into(),
        scope: BTreeSet::from([key("R1")]),
        source_revision: g.parsed.candidate_oid.clone(),
        source_digest: g.parsed.snapshot_digest.clone(),
        graph_digest: digest(&g),
        policy_digest: format!("sha256:{}", "a".repeat(64)),
        approval_ref: "fixture:approval-1".into(),
        effective_from: 10,
        expires_at: 100,
        state: BaselineState::Approved,
        graph: g,
    };
    let v = authenticate(&b, &Fixture, Profile::Fixture, 50).unwrap();
    let e = export_obligations(&b.graph, &v, &b.scope, &b.source_digest).unwrap();
    std::fs::write(
        "fixtures/handoffs/baseline.json",
        format!("{}\n", serde_json::to_string_pretty(&b).unwrap()),
    )
    .unwrap();
    std::fs::write(
        "fixtures/handoffs/obligations.json",
        format!("{}\n", serde_json::to_string_pretty(&e).unwrap()),
    )
    .unwrap();
}
