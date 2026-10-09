//! Local trusted-process CI exercise; no hosted identity or domain executor.
use guardengine::integration::{eligibility::*, *};
use sha2::{Digest, Sha256};
use specguard::{
    integration::{git_binding::*, runtime::CancellationToken},
    model::Version,
    source::*,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    process::Command,
};
pub const ADAPTER: &str = "specguard.example.local-ci/v1";
const NOW: i64 = 1791540002;
const PRINCIPAL: &str = "fixture-controller://specguard/ci";
fn digest(b: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(b))
}
fn key(id: &str) -> specguard::model::Identity {
    specguard::model::Identity {
        namespace: "demo".into(),
        id: id.into(),
    }
}
use guardengine::{
    ContractMetadata, ContractSpec, Enforcement, GuardAssertion, GuardContract, GuardRule,
};
use specguard::integration::producer::*;
use specguard::rules::FindingKind;
fn mapping() -> ProtectedMapping {
    let kinds = [
        FindingKind::Duplicate,
        FindingKind::MissingRequirement,
        FindingKind::MissingAcceptance,
        FindingKind::BrokenReference,
    ];
    let mut entries = vec![];
    let mut rules = vec![];
    for (index, kind) in kinds.into_iter().enumerate() {
        let rule_id = format!("rule-{index}");
        let object = format!("finding-{index}");
        entries.push(MappingEntry {
            key: key("R1"),
            kind,
            rule_id: rule_id.clone(),
            subject: "demo:R1".into(),
            predicate: "spec_violation".into(),
            object: object.clone(),
        });
        rules.push(GuardRule {
            id: rule_id,
            description: "fixture protected mapping".into(),
            enforcement: Enforcement::Enforce,
            assertion: GuardAssertion::ForbidRelation {
                subject: "demo:R1".into(),
                predicate: "spec_violation".into(),
                object,
            },
        });
    }
    ProtectedMapping {
        contract: GuardContract {
            api_version: guardengine::API_VERSION.into(),
            kind: "GuardContract".into(),
            metadata: ContractMetadata {
                id: "protected".into(),
                revision: "1".into(),
            },
            spec: ContractSpec { rules },
        },
        entries,
    }
}

#[derive(Clone, Copy)]
pub enum Scenario {
    Good,
    CandidateWeakensRules,
    Partial,
}
pub struct Exercise {
    pub bundle: GitEvidenceBundle,
    pub expected: EligibilityPolicy,
    issued: Option<ProducerRecord>,
    pub revoked: bool,
    pub unavailable: bool,
    pub unchanged: bool,
    pub repository_before: String,
    pub repository_after: String,
}
impl Exercise {
    pub fn evaluate(&self, envelope: &GuardRunEnvelope) -> EligibilityResult {
        let output = self.bundle.output();
        evaluate_eligibility(
            envelope,
            ArtifactBytes {
                contract: output.contract.as_deref().unwrap(),
                facts: output.facts.as_deref().unwrap(),
                report: output.report.as_deref().unwrap(),
            },
            &self.expected,
            self,
            NOW,
            None,
        )
    }
}
impl AuthorityProvider for Exercise {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        d: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        if self.unavailable {
            return Err(AuthorityError::Unavailable);
        }
        let mut r = self
            .issued
            .as_ref()
            .filter(|r| r.envelope_digest == d)
            .cloned()
            .ok_or(AuthorityError::Untrusted)?;
        r.validity.revoked = self.revoked;
        Ok(r)
    }
    fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
        Err(AuthorityError::Untrusted)
    }
}
// This creates its own small repository; there is no input path/command/config API.
// Git writes only fixture setup. The measured SG/GE job below is read-only.
pub fn exercise(scenario: Scenario) -> Exercise {
    let root = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| -> Vec<u8> {
        let out = Command::new("/usr/bin/git")
            .arg("-C")
            .arg(root.path())
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", root.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_AUTHOR_NAME", "LocalFixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "LocalFixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .env("GIT_AUTHOR_DATE", "2026-10-09T10:00:00Z")
            .env("GIT_COMMITTER_DATE", "2026-10-09T10:00:00Z")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stdout.len() < 1024 * 1024);
        out.stdout
    };
    let text = |args: &[&str]| String::from_utf8(git(args)).unwrap().trim().to_owned();
    git(&["init", "-q"]);
    std::fs::create_dir(root.path().join("specs")).unwrap();
    let good = include_str!("../../fixtures/source-versions/markdown-v1.md");
    std::fs::write(root.path().join("specs/a.md"), good).unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "base"]);
    let base = text(&["rev-parse", "HEAD"]);
    let doc = match scenario {
        Scenario::Good => good,
        Scenario::CandidateWeakensRules => good.split("### Acceptance").next().unwrap(),
        Scenario::Partial => "unsupported source",
    };
    std::fs::write(root.path().join("specs/a.md"), doc).unwrap();
    std::fs::write(
        root.path().join(".guard-policy.json"),
        br#"{"spec":{"rules":[]},"allowEverything":true}"#,
    )
    .unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "candidate"]);
    let head = text(&["rev-parse", "HEAD"]);
    // These are controller-owned constants, not read from the candidate rule file.
    let mapping = mapping();
    let contract_digest = digest(&serde_json::to_vec(&mapping.contract).unwrap());
    let policy = GitSourcePolicy::freeze(
        SourcePolicy {
            api_version: Version::V1,
            roots: vec![SourceRoot {
                path: "specs".into(),
                format: "markdown-explicit/v1".into(),
                namespace: "demo".into(),
                authority: "primary".into(),
            }],
            limits: Limits::default(),
        },
        BTreeSet::from([key("R1")]),
        mapping,
    )
    .unwrap();
    let repo = gitguard::Repository::discover(root.path(), "fixture-ci-repository").unwrap();
    let scope = gitguard::scope::TaskScope::advisory(
        "fixture-ci",
        vec!["demo:R1".into()],
        vec![b"specs".to_vec()],
        policy.digest().trim_start_matches("sha256:"),
        None,
    )
    .unwrap();
    let candidate = repo
        .prepare_candidate(
            &repo
                .resolve_subject(gitguard::subject::SubjectRequest::Commit(head))
                .unwrap(),
            &scope,
            &gitguard::candidate::CandidateRequest {
                worktree_id: "fixture-worktree".into(),
                base_oid: base,
                merge_group_id: None,
                members: vec![],
            },
        )
        .unwrap();
    let prepared = GitPreparedRun::prepare_clean(
        &repo,
        &candidate,
        &policy,
        Attempt {
            run_id: "local-ci-run".into(),
            started_at: "2026-10-09T10:00:00Z".into(),
        },
    )
    .unwrap();
    let expected = EligibilityPolicy {
        binding: prepared.binding().clone(),
        producer: Producer {
            guard: "SpecGuard".into(),
            version: "0.1.0".into(),
            analyzer_id: "specguard.structural".into(),
            analyzer_version: "1".into(),
        },
        required_scopes: vec![
            "profile:specguard.structural/v1".into(),
            "requirement:demo:R1".into(),
            "source:specs/a.md".into(),
        ],
        contract_digest,
        action: "fixture.ci-observe".into(),
        producer_principals: BTreeSet::from([PRINCIPAL.into()]),
        approval_principals: BTreeMap::new(),
    };
    let snapshot = || {
        (
            git(&["for-each-ref", "--format=%(refname) %(objectname)"]),
            git(&["rev-parse", "HEAD"]),
            std::fs::read(root.path().join(".git/index")).unwrap(),
            std::fs::read(root.path().join("specs/a.md")).unwrap(),
            std::fs::read(root.path().join(".guard-policy.json")).unwrap(),
            git(&["status", "--porcelain=v1", "--untracked-files=all"]),
        )
    };
    let before = snapshot();
    let bundle = prepared
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new())
        .unwrap();
    bundle
        .verify(&repo, &candidate, &policy, &expected.binding)
        .unwrap();
    let after = snapshot();
    let unchanged = before == after;
    let repository_before = digest(&serde_json::to_vec(&before).unwrap());
    let repository_after = digest(&serde_json::to_vec(&after).unwrap());
    // Only the owned producer run above may issue an identity record; no arbitrary
    // caller digest registration, signing, host authentication or cached eligibility.
    let issued = Some(ProducerRecord {
        principal: PRINCIPAL.into(),
        producer: expected.producer.clone(),
        envelope_digest: digest(&serde_json::to_vec(&bundle.output().envelope).unwrap()),
        validity: Validity {
            issued_at: NOW - 2,
            expires_at: NOW + 60,
            revoked: false,
        },
    });
    Exercise {
        bundle,
        expected,
        issued,
        revoked: false,
        unavailable: false,
        unchanged,
        repository_before,
        repository_after,
    }
}
