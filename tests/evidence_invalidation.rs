mod common;
mod producer_support;
use common::*;
use guardengine::integration::eligibility::*;
use guardengine::integration::*;
use specguard::integration::{
    approval::{ApprovalValidationPort, Profile},
    freshness::{BaselineCheck, ConsumptionClock, CurrentExpectation, RunHistory},
    producer::{PreparedRun, prepare},
    runtime::CancellationToken,
};
use std::{
    cell::Cell,
    collections::{BTreeMap, BTreeSet},
};
const NOW: i64 = 1_791_540_002;
struct Clock(Cell<i64>);
impl ConsumptionClock for Clock {
    fn now(&self) -> Result<i64, String> {
        Ok(self.0.get())
    }
}
struct Authority {
    producer: Producer,
    calls: Cell<usize>,
    revoked: Cell<bool>,
    expires: Cell<i64>,
    unavailable: Cell<bool>,
}
impl AuthorityProvider for Authority {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        digest: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        self.calls.set(self.calls.get() + 1);
        if self.unavailable.get() {
            return Err(AuthorityError::Unavailable);
        }
        Ok(ProducerRecord {
            principal: "fixture:sg".into(),
            producer: self.producer.clone(),
            envelope_digest: digest.into(),
            validity: Validity {
                issued_at: 0,
                expires_at: self.expires.get(),
                revoked: self.revoked.get(),
            },
        })
    }
    fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
        Err(AuthorityError::Untrusted)
    }
}
fn engine_policy(
    prepared: &PreparedRun,
    s: &specguard::source::SourceSnapshot,
) -> EligibilityPolicy {
    use sha2::{Digest, Sha256};
    let contract = serde_json::to_vec(&producer_support::policy().contract).unwrap();
    let mut scopes = vec![
        "profile:specguard.structural/v1".into(),
        "requirement:demo:R1".into(),
    ];
    for source in &s.inventory.sources {
        scopes.push(format!("source:{}", source.path));
    }
    scopes.sort();
    scopes.dedup();
    EligibilityPolicy {
        binding: prepared.binding().clone(),
        producer: Producer {
            guard: "SpecGuard".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            analyzer_id: "specguard.structural".into(),
            analyzer_version: "1".into(),
        },
        required_scopes: scopes,
        contract_digest: format!("sha256:{:x}", Sha256::digest(contract)),
        action: "fixture:consume".into(),
        producer_principals: BTreeSet::from(["fixture:sg".into()]),
        approval_principals: BTreeMap::new(),
    }
}
fn authority(policy: &EligibilityPolicy) -> Authority {
    Authority {
        producer: policy.producer.clone(),
        calls: Cell::new(0),
        revoked: Cell::new(false),
        expires: Cell::new(NOW + 100),
        unavailable: Cell::new(false),
    }
}
fn prepared(
    root: &std::path::Path,
    s: &specguard::source::SourceSnapshot,
    run: &str,
) -> PreparedRun {
    let mut inv = producer_support::invocation(s);
    inv.run_id = run.into();
    prepare(
        root,
        s,
        inv,
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
    )
    .unwrap()
}
#[test]
fn current_consumer_rechecks_authority_without_ttl_and_never_changes_history_bytes() {
    let (root, s) = snapshot(document().as_bytes());
    let prepared = prepared(root.path(), &s, "first");
    let policy = engine_policy(&prepared, &s);
    let authority = authority(&policy);
    let expectation = CurrentExpectation::freeze(&prepared, 1, &policy, Profile::Fixture).unwrap();
    let mut history = RunHistory::default();
    let complete = history
        .register(prepared, 0)
        .unwrap()
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&complete).unwrap();
    history.publish(&complete).unwrap();
    // A controller's effectively indefinite authority interval still requires
    // a fresh revocation check; the evidence also has no envelope TTL.
    authority.expires.set(i64::MAX);
    let original = serde_json::to_vec(complete.output()).unwrap();
    assert!(complete.output().envelope.expires_at.is_none());
    let clock = Clock(Cell::new(NOW));
    for _ in 0..2 {
        assert!(
            history
                .consume_current(&expectation, &authority, &clock, None)
                .unwrap()
                .eligible()
        );
    }
    assert_eq!(authority.calls.get(), 2);
    authority.revoked.set(true);
    assert!(
        !history
            .consume_current(&expectation, &authority, &clock, None)
            .unwrap()
            .eligible()
    );
    authority.revoked.set(false);
    authority.expires.set(NOW);
    assert!(
        !history
            .consume_current(&expectation, &authority, &clock, None)
            .unwrap()
            .eligible()
    );
    authority.expires.set(NOW + 100);
    authority.unavailable.set(true);
    assert!(
        !history
            .consume_current(&expectation, &authority, &clock, None)
            .unwrap()
            .eligible()
    );
    assert_eq!(serde_json::to_vec(complete.output()).unwrap(), original);
}
#[test]
fn independent_generation_and_current_policy_are_required() {
    let (root, s) = snapshot(document().as_bytes());
    let first = prepared(root.path(), &s, "one");
    let policy = engine_policy(&first, &s);
    let authority = authority(&policy);
    let clock = Clock(Cell::new(NOW));
    let expected = CurrentExpectation::freeze(&first, 1, &policy, Profile::Fixture).unwrap();
    assert!(CurrentExpectation::freeze(&first, 1, &policy, Profile::Production).is_err());
    let mut history = RunHistory::default();
    let old = history
        .register(first, 0)
        .unwrap()
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&old).unwrap();
    history.publish(&old).unwrap();
    let next = prepared(root.path(), &s, "two");
    let current = CurrentExpectation::freeze(&next, 2, &policy, Profile::Fixture).unwrap();
    let token = CancellationToken::new();
    token.cancel();
    let failure = history
        .register(next, 1)
        .unwrap()
        .execute("2026-10-09T10:00:01Z", &token, |_| {})
        .unwrap();
    history.append(&failure).unwrap();
    history.publish(&failure).unwrap();
    assert!(
        history
            .consume_current(&expected, &authority, &clock, None)
            .is_err()
    );
    assert!(
        !history
            .consume_current(&current, &authority, &clock, None)
            .unwrap()
            .eligible()
    );
    assert!(history.publish(&old).is_err());
}

#[test]
fn every_work_or_controller_dimension_invalidates_old_evidence() {
    let (root, s) = snapshot(document().as_bytes());
    let first = prepared(root.path(), &s, "first");
    let policy = engine_policy(&first, &s);
    let authority = authority(&policy);
    let clock = Clock(Cell::new(NOW));
    let mut history = RunHistory::default();
    let old = history
        .register(first, 0)
        .unwrap()
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&old).unwrap();
    history.publish(&old).unwrap();
    git(
        root.path(),
        &[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--allow-empty",
            "-qm",
            "next",
        ],
    );
    let next_oid = git(root.path(), &["rev-parse", "HEAD"]);
    for dimension in 0..12 {
        let mut source = s.clone();
        let mut inv = producer_support::invocation(&source);
        let mut mapping = producer_support::policy();
        let mut required = BTreeSet::from([key("R1")]);
        match dimension {
            0 => {
                source.binding.candidate_oid = next_oid.clone();
                inv.candidate_oid = Some(next_oid.clone());
            }
            1 => {
                source.binding.base_oid = next_oid.clone();
                inv.base_oid = Some(next_oid.clone());
            }
            2 => inv.merge_group_id = Some("queue-changed".into()),
            3 => mapping.contract.metadata.revision = "new-policy".into(),
            4 => source.inventory.entries[0].format = "future/v2".into(),
            5 => inv.baseline_digest = Some(format!("sha256:{}", "a".repeat(64))),
            6 => {
                source
                    .contents
                    .values_mut()
                    .next()
                    .unwrap()
                    .extend_from_slice(b"\nchanged bytes");
                source.inventory.entries[0].digest =
                    specguard::model::digest(source.contents.values().next().unwrap());
            }
            7 => {
                required.insert(key("R2"));
                let extra = mapping.entries.clone();
                let extra_rules = mapping.contract.spec.rules.clone();
                for mut e in extra {
                    e.key = key("R2");
                    e.subject = "demo:R2".into();
                    e.rule_id.push_str("-r2");
                    mapping.entries.push(e);
                }
                for mut r in extra_rules {
                    r.id.push_str("-r2");
                    let guardengine::GuardAssertion::ForbidRelation { subject, .. } =
                        &mut r.assertion;
                    *subject = "demo:R2".into();
                    mapping.contract.spec.rules.push(r);
                }
            }
            8 => source.inventory.limits.max_millis -= 1,
            9 => inv.repo_id = "other-repository".into(),
            10 => inv.task_id = "other-task".into(),
            _ => inv.worktree_id = "other-worktree".into(),
        }
        source.digest =
            specguard::model::digest(&(&source.inventory, &source.binding, &source.contents));
        let fresh = prepare(root.path(), &source, inv, &required, &mapping).unwrap();
        let current = engine_policy(&fresh, &source);
        let expected = CurrentExpectation::freeze(&fresh, 1, &current, Profile::Fixture).unwrap();
        assert!(
            history
                .consume_current(&expected, &authority, &clock, None)
                .is_err(),
            "dimension {dimension}"
        );
    }
    for dimension in 0..4 {
        let fresh = prepared(root.path(), &s, "expectation");
        let mut changed = policy.clone();
        match dimension {
            0 => changed.producer.analyzer_version = "2".into(),
            1 => changed.required_scopes.push("capability:new".into()),
            2 => changed.contract_digest = format!("sha256:{}", "f".repeat(64)),
            _ => changed.producer.analyzer_id = "different-analyzer".into(),
        }
        changed.required_scopes.sort();
        let expected = CurrentExpectation::freeze(&fresh, 1, &changed, Profile::Fixture).unwrap();
        assert!(
            !history
                .consume_current(&expected, &authority, &clock, None)
                .unwrap()
                .eligible()
        );
    }
}

struct BaselinePort {
    inner: FixtureApproval,
    calls: Cell<usize>,
    revoked: Cell<bool>,
    unavailable: Cell<bool>,
}
impl ApprovalValidationPort for BaselinePort {
    fn profile(&self) -> Profile {
        Profile::Fixture
    }
    fn validate(
        &self,
        b: &specguard::baseline::ApprovedBaseline,
    ) -> Result<
        specguard::integration::approval::Authentication,
        specguard::integration::approval::ApprovalError,
    > {
        self.calls.set(self.calls.get() + 1);
        if self.unavailable.get() {
            return Err(
                specguard::integration::approval::ApprovalError::Unavailable("offline".into()),
            );
        }
        let mut result = self.inner.validate(b)?;
        result.revoked = self.revoked.get();
        Ok(result)
    }
}
#[test]
fn baseline_is_required_and_authenticated_freshly_at_current_clock() {
    let (root, s) = snapshot(document().as_bytes());
    let mut b = baseline();
    b.repository = "fixture-repository".into();
    b.effective_from = NOW - 10;
    b.expires_at = NOW + 100;
    let mut inv = producer_support::invocation(&s);
    inv.baseline_digest = Some(specguard::model::digest(&b));
    let p = prepare(
        root.path(),
        &s,
        inv,
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
    )
    .unwrap();
    let policy = engine_policy(&p, &s);
    let expected = CurrentExpectation::freeze(&p, 1, &policy, Profile::Fixture).unwrap();
    let authority = authority(&policy);
    let mut auth = fixture_approval(&b);
    auth.0.as_mut().unwrap().issued_at = NOW - 1;
    auth.0.as_mut().unwrap().expires_at = NOW + 50;
    let port = BaselinePort {
        inner: auth,
        calls: Cell::new(0),
        revoked: Cell::new(false),
        unavailable: Cell::new(false),
    };
    let clock = Clock(Cell::new(NOW));
    let mut history = RunHistory::default();
    let complete = history
        .register(p, 0)
        .unwrap()
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&complete).unwrap();
    history.publish(&complete).unwrap();
    assert!(
        history
            .consume_current(&expected, &authority, &clock, None)
            .is_err()
    );
    for _ in 0..2 {
        assert!(
            history
                .consume_current(
                    &expected,
                    &authority,
                    &clock,
                    Some(BaselineCheck {
                        baseline: &b,
                        authority: &port
                    })
                )
                .unwrap()
                .eligible()
        );
    }
    assert_eq!(port.calls.get(), 2);
    port.revoked.set(true);
    assert!(
        history
            .consume_current(
                &expected,
                &authority,
                &clock,
                Some(BaselineCheck {
                    baseline: &b,
                    authority: &port
                })
            )
            .is_err()
    );
    port.revoked.set(false);
    port.unavailable.set(true);
    assert!(
        history
            .consume_current(
                &expected,
                &authority,
                &clock,
                Some(BaselineCheck {
                    baseline: &b,
                    authority: &port
                })
            )
            .is_err()
    );
    port.unavailable.set(false);
    clock.0.set(NOW + 50);
    let before_expiry = port.calls.get();
    assert!(
        history
            .consume_current(
                &expected,
                &authority,
                &clock,
                Some(BaselineCheck {
                    baseline: &b,
                    authority: &port
                })
            )
            .is_err()
    );
    assert_eq!(
        port.calls.get(),
        before_expiry + 1,
        "approval expired before baseline; port still queried"
    );
    clock.0.set(NOW + 100);
    assert!(
        history
            .consume_current(
                &expected,
                &authority,
                &clock,
                Some(BaselineCheck {
                    baseline: &b,
                    authority: &port
                })
            )
            .is_err()
    );
    clock.0.set(NOW);
    let mut changed = b.clone();
    changed.policy_digest = format!("sha256:{}", "f".repeat(64));
    assert!(
        history
            .consume_current(
                &expected,
                &authority,
                &clock,
                Some(BaselineCheck {
                    baseline: &changed,
                    authority: &port
                })
            )
            .is_err()
    );
}
#[test]
fn partial_block_error_and_bad_clock_cannot_be_consumed() {
    for (text, cancel) in [
        ("malformed", false),
        (
            "---\nformat: markdown-explicit/v1\nnamespace: demo\n---\n## Requirement: R1\nMissing acceptance\n",
            false,
        ),
        (document(), true),
    ] {
        let (root, s) = snapshot(text.as_bytes());
        let p = prepared(root.path(), &s, "run");
        let policy = engine_policy(&p, &s);
        let expected = CurrentExpectation::freeze(&p, 1, &policy, Profile::Fixture).unwrap();
        let authority = authority(&policy);
        let clock = Clock(Cell::new(NOW));
        let mut history = RunHistory::default();
        let token = CancellationToken::new();
        if cancel {
            token.cancel();
        }
        let complete = history
            .register(p, 0)
            .unwrap()
            .execute("2026-10-09T10:00:01Z", &token, |_| {})
            .unwrap();
        history.append(&complete).unwrap();
        history.publish(&complete).unwrap();
        assert!(
            !history
                .consume_current(&expected, &authority, &clock, None)
                .unwrap()
                .eligible()
        );
        clock.0.set(-1);
        assert!(
            history
                .consume_current(&expected, &authority, &clock, None)
                .is_err()
        );
    }
}

#[test]
fn independent_targets_and_current_clock_failure_do_not_reuse_another_run() {
    let (root, s) = snapshot(document().as_bytes());
    let mut history = RunHistory::default();
    let one = prepared(root.path(), &s, "one");
    let policy = engine_policy(&one, &s);
    let authority = authority(&policy);
    let e1 = CurrentExpectation::freeze(&one, 1, &policy, Profile::Fixture).unwrap();
    let mut inv = producer_support::invocation(&s);
    inv.run_id = "two".into();
    inv.task_id = "other-task".into();
    let two = prepare(
        root.path(),
        &s,
        inv,
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
    )
    .unwrap();
    let policy2 = engine_policy(&two, &s);
    let e2 = CurrentExpectation::freeze(&two, 1, &policy2, Profile::Fixture).unwrap();
    let one = history.register(one, 0).unwrap();
    let two = history.register(two, 0).unwrap();
    let c2 = two
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&c2).unwrap();
    history.publish(&c2).unwrap();
    assert!(
        history
            .consume_current(&e1, &authority, &Clock(Cell::new(NOW)), None)
            .is_err()
    );
    assert!(
        history
            .consume_current(&e2, &authority, &Clock(Cell::new(NOW)), None)
            .unwrap()
            .eligible()
    );
    let c1 = one
        .execute("invalid-finish", &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&c1).unwrap();
    history.publish(&c1).unwrap();
    assert!(
        !history
            .consume_current(&e1, &authority, &Clock(Cell::new(NOW)), None)
            .unwrap()
            .eligible()
    );
    assert!(
        history
            .consume_current(&e2, &authority, &Clock(Cell::new(NOW)), None)
            .unwrap()
            .eligible()
    );
    struct BadClock;
    impl ConsumptionClock for BadClock {
        fn now(&self) -> Result<i64, String> {
            Err("clock unavailable".into())
        }
    }
    let before = authority.calls.get();
    assert!(
        history
            .consume_current(&e2, &authority, &BadClock, None)
            .is_err()
    );
    assert_eq!(authority.calls.get(), before);
}

#[test]
fn independent_clock_rollback_does_not_revive_expired_current_eligibility() {
    let (root, s) = snapshot(document().as_bytes());
    let p = prepared(root.path(), &s, "rollback");
    let policy = engine_policy(&p, &s);
    let authority = authority(&policy);
    authority.expires.set(NOW + 5);
    let expected = CurrentExpectation::freeze(&p, 1, &policy, Profile::Fixture).unwrap();
    let mut history = RunHistory::default();
    let done = history
        .register(p, 0)
        .unwrap()
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&done).unwrap();
    history.publish(&done).unwrap();
    let clock = Clock(Cell::new(NOW));
    assert!(
        history
            .consume_current(&expected, &authority, &clock, None)
            .unwrap()
            .eligible()
    );
    clock.0.set(NOW + 6);
    assert!(
        !history
            .consume_current(&expected, &authority, &clock, None)
            .unwrap()
            .eligible()
    );
    clock.0.set(NOW + 1);
    let rolled = history.consume_current(&expected, &authority, &clock, None);
    assert!(
        !rolled.is_ok_and(|r| r.eligible()),
        "clock rollback revived previously expired current evidence"
    );
}
#[test]
fn slower_old_time_call_cannot_return_eligible_after_newer_failed_consumption() {
    use std::sync::{Mutex, mpsc};
    struct BlockingAuthority {
        producer: Producer,
        ready: mpsc::SyncSender<()>,
        resume: Mutex<mpsc::Receiver<()>>,
    }
    impl AuthorityProvider for BlockingAuthority {
        fn verify_producer(
            &self,
            _: &GuardRunEnvelope,
            digest: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            self.ready.send(()).unwrap();
            self.resume.lock().unwrap().recv().unwrap();
            Ok(ProducerRecord {
                principal: "fixture:sg".into(),
                producer: self.producer.clone(),
                envelope_digest: digest.into(),
                validity: Validity {
                    issued_at: 0,
                    expires_at: i64::MAX,
                    revoked: false,
                },
            })
        }
        fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
            Err(AuthorityError::Untrusted)
        }
    }
    let (root, s) = snapshot(document().as_bytes());
    let prepared = prepared(root.path(), &s, "slow");
    let policy = engine_policy(&prepared, &s);
    let expected = CurrentExpectation::freeze(&prepared, 1, &policy, Profile::Fixture).unwrap();
    let mut history = RunHistory::default();
    let completed = history
        .register(prepared, 0)
        .unwrap()
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&completed).unwrap();
    history.publish(&completed).unwrap();
    let original = serde_json::to_vec(completed.output()).unwrap();
    let (ready, observed) = mpsc::sync_channel(0);
    let (resume, release) = mpsc::sync_channel(0);
    let slow = BlockingAuthority {
        producer: policy.producer.clone(),
        ready,
        resume: Mutex::new(release),
    };
    std::thread::scope(|scope| {
        let worker =
            scope.spawn(|| history.consume_current(&expected, &slow, &Clock(Cell::new(NOW)), None));
        observed.recv().unwrap();
        let fast = authority(&policy);
        fast.expires.set(NOW + 5);
        assert!(
            !history
                .consume_current(&expected, &fast, &Clock(Cell::new(NOW + 6)), None)
                .unwrap()
                .eligible()
        );
        resume.send(()).unwrap();
        assert!(worker.join().unwrap().is_err());
        fast.expires.set(i64::MAX);
        assert!(
            history
                .consume_current(&expected, &fast, &Clock(Cell::new(NOW + 1)), None)
                .is_err()
        );
        assert!(
            history
                .consume_current(&expected, &fast, &Clock(Cell::new(NOW + 6)), None)
                .unwrap()
                .eligible()
        );
    });
    assert_eq!(serde_json::to_vec(completed.output()).unwrap(), original);
}
