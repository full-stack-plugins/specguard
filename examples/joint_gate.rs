//! Reuse immutable actual peer captures; new SG runs are explicitly separate.
#![allow(dead_code)]
use guardengine::integration::{eligibility::*, *};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use specguard::{
    integration::{
        approval::*, baseline_review::BaselineReview, freshness::*, producer::*,
        runtime::CancellationToken,
    },
    source::SourceSnapshot,
};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
const NOW: i64 = 2_000_000_002;
fn sha(b: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(b))
}
fn load<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}
struct Clock(Cell<i64>);
impl ConsumptionClock for Clock {
    fn now(&self) -> Result<i64, String> {
        Ok(self.0.get())
    }
}
struct BaselineAuthority(Authentication);
impl ApprovalValidationPort for BaselineAuthority {
    fn profile(&self) -> Profile {
        Profile::Fixture
    }
    fn validate(
        &self,
        _: &specguard::baseline::ApprovedBaseline,
    ) -> Result<Authentication, ApprovalError> {
        Ok(self.0.clone())
    }
}
struct Authority {
    policy: EligibilityPolicy,
    digest: String,
    revoked: Cell<bool>,
    expires: Cell<i64>,
    enabled: Cell<bool>,
}
impl AuthorityProvider for Authority {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        digest: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        if !self.enabled.get() {
            return Err(AuthorityError::Unavailable);
        }
        if digest != self.digest {
            return Err(AuthorityError::Untrusted);
        }
        Ok(ProducerRecord {
            principal: "fixture:joint-producer".into(),
            producer: self.policy.producer.clone(),
            envelope_digest: self.digest.clone(),
            validity: Validity {
                issued_at: NOW - 1,
                expires_at: NOW + 500,
                revoked: false,
            },
        })
    }
    fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
        if reference != "fixture:joint-review" {
            return Err(AuthorityError::Untrusted);
        }
        Ok(ApprovalRecord {
            principal: "fixture:joint-reviewer".into(),
            purpose: "review".into(),
            action: self.policy.action.clone(),
            binding: self.policy.binding.clone(),
            contract_digest: self.policy.contract_digest.clone(),
            validity: Validity {
                issued_at: NOW - 1,
                expires_at: self.expires.get(),
                revoked: self.revoked.get(),
            },
        })
    }
}
struct Case {
    root: PathBuf,
    snapshot: SourceSnapshot,
    review: BaselineReview,
    mapping: ProtectedMapping,
    original: ProducedRun,
    policy: EligibilityPolicy,
}
impl Case {
    fn prepared(&self, name: &str) -> PreparedRun {
        let b = &self.original.envelope.binding;
        let invocation = Invocation {
            run_id: format!("current-sg-joint-{name}"),
            repo_id: b.repo_id.clone(),
            task_id: b.task_id.clone(),
            worktree_id: b.worktree_id.clone(),
            candidate_oid: Some(b.candidate_oid.clone()),
            base_oid: Some(b.base_oid.clone()),
            merge_group_id: b.merge_group_id.clone(),
            baseline_digest: b.baseline_digest.clone(),
            started_at: self.original.envelope.started_at.clone(),
        };
        prepare_baseline_review(
            &self.root,
            &self.snapshot,
            invocation,
            &self.review.baseline.scope,
            &self.mapping,
            &self.review,
        )
        .unwrap()
    }
    fn baseline(&self) -> BaselineAuthority {
        let b = &self.review.baseline;
        BaselineAuthority(Authentication {
            issuer: "fixture:joint-baseline".into(),
            purpose: "specification-baseline".into(),
            repository: b.repository.clone(),
            scope: b.scope.clone(),
            baseline_digest: specguard::model::digest(b),
            policy_digest: b.policy_digest.clone(),
            issued_at: NOW - 1,
            expires_at: NOW + 500,
            revoked: false,
        })
    }
}
fn verify_retained(root: &Path) -> BTreeMap<String, String> {
    let manifest: BTreeMap<String, String> = serde_json::from_str(include_str!(
        "../fixtures/end-to-end/retained-capture-sha256.json"
    ))
    .unwrap();
    for (path, hash) in &manifest {
        assert_eq!(
            &sha(&std::fs::read(root.join(path)).unwrap()),
            hash,
            "retained {path}"
        );
    }
    manifest
}
fn cases(root: &Path) -> BTreeMap<String, Case> {
    let provenance: Value = load(root.join("producer-manifest.json"));
    let mut cases = BTreeMap::new();
    for name in ["A", "B", "M1", "M2"] {
        let dir = root.join(name).join("specguard");
        let original: ProducedRun = load(dir.join("original.json"));
        original.verify().unwrap();
        assert_eq!(
            original.envelope.decision,
            Some(guardengine::Decision::RequireApproval)
        );
        let expected: Value = load(dir.join("expected.json"));
        let policy = EligibilityPolicy {
            binding: serde_json::from_value(expected["binding"].clone()).unwrap(),
            producer: serde_json::from_value(expected["producer"].clone()).unwrap(),
            contract_digest: expected["contract_digest"].as_str().unwrap().into(),
            required_scopes: serde_json::from_value(expected["required_scopes"].clone()).unwrap(),
            action: "fixture:joint-consume".into(),
            producer_principals: BTreeSet::from(["fixture:joint-producer".into()]),
            approval_principals: BTreeMap::from([(
                "review".into(),
                BTreeSet::from(["fixture:joint-reviewer".into()]),
            )]),
        };
        assert_eq!(policy.binding, original.envelope.binding);
        assert!(policy.binding.baseline_digest.is_some());
        let worktree = root.join(format!("worktree-{name}"));
        let observed = std::process::Command::new("git")
            .arg("-C")
            .arg(&worktree)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        assert!(observed.status.success());
        assert_eq!(
            String::from_utf8(observed.stdout).unwrap().trim(),
            provenance["profiles"][name]["candidate"].as_str().unwrap()
        );
        assert_eq!(
            serde_json::to_value(&policy.binding.requirement_ids).unwrap(),
            provenance["profiles"][name]["requirements"]
        );
        assert_eq!(
            serde_json::to_value(&policy.binding.merge_group_id).unwrap(),
            provenance["profiles"][name]["group"]
        );
        assert_eq!(
            policy.binding.base_oid,
            provenance["profiles"][name]["base"].as_str().unwrap()
        );
        if name.starts_with('M') {
            let parents = std::process::Command::new("git")
                .arg("-C")
                .arg(&worktree)
                .args(["rev-list", "--parents", "-n", "1", "HEAD"])
                .output()
                .unwrap();
            assert!(parents.status.success());
            assert!(
                String::from_utf8(parents.stdout)
                    .unwrap()
                    .split_whitespace()
                    .count()
                    >= 3
            );
            for member in provenance["profiles"][name]["members"].as_array().unwrap() {
                assert_ne!(policy.binding.candidate_oid, member.as_str().unwrap());
            }
        }
        let snapshot: SourceSnapshot = load(dir.join("snapshot.json"));
        let fresh =
            specguard::source::freeze(&worktree, &snapshot.inventory, snapshot.binding.clone())
                .unwrap();
        assert_eq!(
            serde_json::to_value(&fresh).unwrap(),
            serde_json::to_value(&snapshot).unwrap()
        );
        // Actual retained FG ALLOW is prior workflow evidence, not SG execution authority.
        let fg: Value = load(root.join(format!("{name}.stdout.json")));
        assert_eq!(fg["envelope"]["decision"], "ALLOW");
        assert_eq!(fg["execution_authorized"], false);
        cases.insert(
            name.into(),
            Case {
                root: worktree,
                snapshot,
                review: load(dir.join("review.json")),
                mapping: load(dir.join("mapping.json")),
                original,
                policy,
            },
        );
    }
    cases
}
pub fn run() -> Value {
    let root = std::env::var_os("SPECGUARD_JOINT_CAPTURE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../guard-implementation-ledger/flowguard-joint-independent-capture")
        });
    let root = root.canonicalize().unwrap();
    let retained = verify_retained(&root);
    let cases = cases(&root);
    let mut history = RunHistory::default();
    let clock = Clock(Cell::new(NOW));
    let mut pending = vec![];
    let mut expected = BTreeMap::new();
    // Two requirement runs are both registered before either completes.
    for name in ["A", "B", "M1"] {
        let case = &cases[name];
        let prepared = case.prepared(name);
        expected.insert(
            name,
            CurrentExpectation::freeze(&prepared, 1, &case.policy, Profile::Fixture).unwrap(),
        );
        pending.push((name, history.register(prepared, 0).unwrap()));
    }
    let mut new_runs = vec![];
    for (name, registered) in pending.into_iter().rev() {
        let complete = registered
            .execute(
                &cases[name].original.envelope.finished_at,
                &CancellationToken::new(),
                |_| {},
            )
            .unwrap();
        same_expert_output(complete.output(), &cases[name].original);
        new_runs.push(serde_json::to_value(complete.output()).unwrap());
        history.append(&complete).unwrap();
        history.publish(&complete).unwrap();
    }
    for name in ["A", "B", "M1"] {
        assert_current(&history, &expected[name], &cases[name], &clock);
    }
    let a_baseline = cases["A"].baseline();
    let b_attachment = history
        .attach_current(&expected["B"], &["fixture:joint-review".into()])
        .unwrap();
    let b_authority = Authority {
        policy: cases["B"].policy.clone(),
        digest: specguard::model::digest(b_attachment.envelope()),
        revoked: Cell::new(false),
        expires: Cell::new(NOW + 100),
        enabled: Cell::new(true),
    };
    assert!(matches!(
        history.consume_attached(
            &expected["A"],
            &b_attachment,
            &b_authority,
            &clock,
            Some(BaselineCheck {
                baseline: &cases["A"].review.baseline,
                authority: &a_baseline
            })
        ),
        Err(ConsumptionError::InvalidEvidence)
    ));
    // M1 retry overlaps M2. Its late completion remains history, never current M2.
    let late = history
        .register(cases["M1"].prepared("M1-late"), 1)
        .unwrap();
    let prepared = cases["M2"].prepared("M2");
    let target = prepared.work_key().target().clone();
    let m2_expected =
        CurrentExpectation::freeze(&prepared, 3, &cases["M2"].policy, Profile::Fixture).unwrap();
    let m2 = history
        .register(prepared, 2)
        .unwrap()
        .execute(
            &cases["M2"].original.envelope.finished_at,
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();
    same_expert_output(m2.output(), &cases["M2"].original);
    history.append(&m2).unwrap();
    history.publish(&m2).unwrap();
    new_runs.push(serde_json::to_value(m2.output()).unwrap());
    let late = late
        .execute(
            &cases["M1"].original.envelope.finished_at,
            &CancellationToken::new(),
            |_| {},
        )
        .unwrap();
    new_runs.push(serde_json::to_value(late.output()).unwrap());
    history.append(&late).unwrap();
    assert_eq!(history.publish(&late), Err(HistoryError::Stale));
    assert_eq!(
        history.current(&target).unwrap().run_id(),
        "current-sg-joint-M2"
    );
    assert!(
        history
            .attach_current(&expected["M1"], &["fixture:joint-review".into()])
            .is_err()
    );
    assert_current(&history, &m2_expected, &cases["M2"], &clock);
    let case = &cases["M2"];
    let baseline = case.baseline();
    let attachment = history
        .attach_current(&m2_expected, &["fixture:joint-review".into()])
        .unwrap();
    let authority = Authority {
        policy: case.policy.clone(),
        digest: specguard::model::digest(attachment.envelope()),
        revoked: Cell::new(false),
        expires: Cell::new(NOW + 100),
        enabled: Cell::new(true),
    };
    let check = || {
        Some(BaselineCheck {
            baseline: &case.review.baseline,
            authority: &baseline,
        })
    };
    let consume = || {
        history
            .consume_attached(&m2_expected, &attachment, &authority, &clock, check())
            .unwrap()
            .assessment()
            .eligible
    };
    assert!(consume());
    clock.0.set(NOW + 101);
    assert!(!consume());
    authority.expires.set(NOW + 300);
    assert!(consume());
    authority.revoked.set(true);
    assert!(!consume());
    authority.revoked.set(false);
    // The queue candidates intentionally share one approved baseline. Change only
    // its validity bound in a private controller input; never edit captured bytes.
    let mut drifted = case.review.baseline.clone();
    drifted.expires_at += 1;
    assert_eq!(drifted.scope, case.review.baseline.scope);
    let drift = history.consume_attached(
        &m2_expected,
        &attachment,
        &authority,
        &clock,
        Some(BaselineCheck {
            baseline: &drifted,
            authority: &baseline,
        }),
    );
    assert!(matches!(drift, Err(ConsumptionError::Baseline)));
    let before = serde_json::to_vec(m2.output()).unwrap();
    authority.enabled.set(false);
    assert!(!consume());
    assert_eq!(
        before,
        serde_json::to_vec(history.current(&target).unwrap().output()).unwrap()
    );
    assert_eq!(retained, verify_retained(&root));
    let mut new_artifacts = BTreeMap::new();
    let output = std::env::var_os("SPECGUARD_JOINT_OUTPUT").map(PathBuf::from);
    if let Some(dir) = &output {
        std::fs::create_dir(dir).unwrap();
    }
    for bundle in &new_runs {
        let id = bundle["envelope"]["runId"].as_str().unwrap();
        let bytes = serde_json::to_vec(bundle).unwrap();
        new_artifacts.insert(id.to_owned(), sha(&bytes));
        if let Some(dir) = &output {
            std::fs::write(dir.join(format!("{id}.json")), bytes).unwrap();
        }
    }

    json!({"profile":"explicit-local-fixture","groups":4,"new_artifact_sha256":new_artifacts,"new_sg_runs":new_runs.iter().map(|v|v["envelope"]["runId"].clone()).collect::<Vec<_>>(),"wrong_requirement_rejected":true,"expired_rejected":true,"revoked_rejected":true,"baseline_drift_rejected":true,"late_m1_rejected":true,"retained_bytes_unchanged":true,"retained_files":retained.len(),"rollback_required_unavailable":true,"execution_authorized":false})
}
fn same_expert_output(current: &ProducedRun, original: &ProducedRun) {
    assert_eq!(current.contract, original.contract);
    assert_eq!(current.domain, original.domain);
    let mut actual: Value = serde_json::from_slice(current.facts.as_ref().unwrap()).unwrap();
    let retained: Value = serde_json::from_slice(original.facts.as_ref().unwrap()).unwrap();
    assert_eq!(actual["subject"]["id"], current.envelope.run_id);
    // Facts subject.id is explicitly the new attempt ID; compare every other field.
    actual["subject"]["id"] = retained["subject"]["id"].clone();
    assert_eq!(actual, retained);
    assert_eq!(current.envelope.decision, original.envelope.decision);
}
fn assert_current(history: &RunHistory, expected: &CurrentExpectation, case: &Case, clock: &Clock) {
    let baseline = case.baseline();
    let attachment = history
        .attach_current(expected, &["fixture:joint-review".into()])
        .unwrap();
    let authority = Authority {
        policy: case.policy.clone(),
        digest: specguard::model::digest(attachment.envelope()),
        revoked: Cell::new(false),
        expires: Cell::new(NOW + 100),
        enabled: Cell::new(true),
    };
    let result = history
        .consume_attached(
            expected,
            &attachment,
            &authority,
            clock,
            Some(BaselineCheck {
                baseline: &case.review.baseline,
                authority: &baseline,
            }),
        )
        .unwrap();
    assert!(result.assessment().eligible, "{:?}", result.assessment());
    assert_eq!(
        result.assessment().technical_decision,
        Some(guardengine::Decision::RequireApproval)
    );
}
fn main() {
    println!("{}", run());
}
