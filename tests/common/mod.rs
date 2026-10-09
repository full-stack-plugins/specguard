#![allow(dead_code)]
use specguard::{model::*, source::*};
use std::{path::Path, process::Command};
pub fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
pub fn repo() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "-q"]);
    git(
        d.path(),
        &[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--allow-empty",
            "-qm",
            "fixture",
        ],
    );
    d
}
pub fn binding(root: &Path) -> CandidateBinding {
    CandidateBinding {
        candidate_oid: git(root, &["rev-parse", "HEAD"]),
        base_oid: git(root, &["rev-parse", "HEAD"]),
        object_format: git(root, &["rev-parse", "--show-object-format"]),
    }
}
pub fn policy() -> SourcePolicy {
    SourcePolicy {
        api_version: Version::V1,
        roots: vec![SourceRoot {
            path: "specs".into(),
            format: "markdown-explicit/v1".into(),
            namespace: "demo".into(),
            authority: "primary".into(),
        }],
        limits: Limits::default(),
    }
}
pub fn document() -> &'static str {
    include_str!("../../fixtures/source-versions/markdown-v1.md")
}
pub fn snapshot(text: &[u8]) -> (tempfile::TempDir, SourceSnapshot) {
    let d = repo();
    std::fs::create_dir(d.path().join("specs")).unwrap();
    std::fs::write(d.path().join("specs/a.md"), text).unwrap();
    let i = discover(d.path(), &policy()).unwrap();
    let s = freeze(d.path(), &i, binding(d.path())).unwrap();
    (d, s)
}
pub fn key(id: &str) -> Identity {
    Identity {
        namespace: "demo".into(),
        id: id.into(),
    }
}
pub fn graph() -> specguard::graph::SpecificationGraph {
    let (_, s) = snapshot(document().as_bytes());
    specguard::graph::build_graph(specguard::parser::parse(&s))
}
pub fn baseline() -> specguard::baseline::ApprovedBaseline {
    use specguard::baseline::*;
    let g = graph();
    ApprovedBaseline {
        api_version: Version::V1,
        repository: "fixture-repo".into(),
        scope: std::collections::BTreeSet::from([key("R1")]),
        source_revision: g.parsed.candidate_oid.clone(),
        source_digest: g.parsed.snapshot_digest.clone(),
        graph_digest: digest(&g),
        policy_digest: format!("sha256:{}", "a".repeat(64)),
        approval_ref: "fixture:approval-1".into(),
        effective_from: 10,
        expires_at: 100,
        state: BaselineState::Approved,
        graph: g,
    }
}
pub struct FixtureApproval(
    pub  Result<
        specguard::integration::approval::Authentication,
        specguard::integration::approval::ApprovalError,
    >,
);
impl specguard::integration::approval::ApprovalValidationPort for FixtureApproval {
    fn profile(&self) -> specguard::integration::approval::Profile {
        specguard::integration::approval::Profile::Fixture
    }
    fn validate(
        &self,
        _: &specguard::baseline::ApprovedBaseline,
    ) -> Result<
        specguard::integration::approval::Authentication,
        specguard::integration::approval::ApprovalError,
    > {
        self.0.clone()
    }
}
pub fn fixture_approval(b: &specguard::baseline::ApprovedBaseline) -> FixtureApproval {
    use specguard::integration::approval::*;
    FixtureApproval(Ok(Authentication {
        issuer: "fixture-controller".into(),
        purpose: "specification-baseline".into(),
        repository: b.repository.clone(),
        scope: b.scope.clone(),
        baseline_digest: digest(b),
        policy_digest: b.policy_digest.clone(),
        issued_at: 10,
        expires_at: 100,
        revoked: false,
    }))
}
