mod common;
mod producer_support;
mod review_support;
use common::*;
use guardengine::integration::{eligibility::*, *};
use specguard::integration::{
    approval::Profile,
    baseline_review::BaselineReview,
    freshness::*,
    producer::{Invocation, prepare_baseline_review},
    runtime::CancellationToken,
};
use std::{
    cell::{Cell, RefCell},
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
    policy: EligibilityPolicy,
    digest: RefCell<String>,
    revoked: Cell<bool>,
    expires: Cell<i64>,
    calls: Cell<usize>,
    unavailable: Cell<bool>,
    purpose: RefCell<String>,
}
impl AuthorityProvider for Authority {
    fn verify_producer(
        &self,
        _: &GuardRunEnvelope,
        _: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        Ok(ProducerRecord {
            principal: "fixture:producer".into(),
            producer: self.policy.producer.clone(),
            envelope_digest: self.digest.borrow().clone(),
            validity: Validity {
                issued_at: NOW - 1,
                expires_at: NOW + 100,
                revoked: false,
            },
        })
    }
    fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
        self.calls.set(self.calls.get() + 1);
        if self.unavailable.get() {
            return Err(AuthorityError::Unavailable);
        }
        if reference != "fixture:review" {
            return Err(AuthorityError::Untrusted);
        }
        Ok(ApprovalRecord {
            principal: "fixture:reviewer".into(),
            purpose: self.purpose.borrow().clone(),
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
fn setup(
    text: &str,
) -> (
    tempfile::TempDir,
    RunHistory,
    CurrentExpectation,
    BaselineReview,
    EligibilityPolicy,
    Vec<u8>,
    RunTarget,
) {
    setup_state(text, false, "2026-10-09T10:00:01Z")
}
fn setup_state(
    text: &str,
    cancel: bool,
    finish: &str,
) -> (
    tempfile::TempDir,
    RunHistory,
    CurrentExpectation,
    BaselineReview,
    EligibilityPolicy,
    Vec<u8>,
    RunTarget,
) {
    let (root, s) = snapshot(text.as_bytes());
    let req = review_support::review_request(root.path(), &s);
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(req).unwrap()).unwrap();
    let mut review: BaselineReview = serde_json::from_value(v["review"].clone()).unwrap();
    review.baseline.effective_from = NOW - 10;
    review.baseline.expires_at = NOW + 100;
    let mut inv: Invocation = serde_json::from_value(v["invocation"].clone()).unwrap();
    inv.baseline_digest = Some(specguard::model::digest(&review.baseline));
    let prepared = prepare_baseline_review(
        root.path(),
        &s,
        inv,
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
        &review,
    )
    .unwrap();
    let mut contract = producer_support::policy().contract;
    contract
        .spec
        .rules
        .extend(review.contract.spec.rules.clone());
    let policy = EligibilityPolicy {
        binding: prepared.binding().clone(),
        producer: Producer {
            guard: "SpecGuard".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            analyzer_id: "specguard.structural".into(),
            analyzer_version: "1".into(),
        },
        required_scopes: vec![
            "profile:specguard.baseline-review/v1".into(),
            "profile:specguard.structural/v1".into(),
            "requirement:demo:R1".into(),
            "source:specs/a.md".into(),
        ],
        contract_digest: specguard::model::digest(&contract),
        action: "fixture:consume".into(),
        producer_principals: BTreeSet::from(["fixture:producer".into()]),
        approval_principals: BTreeMap::from([(
            "review".into(),
            BTreeSet::from(["fixture:reviewer".into()]),
        )]),
    };
    let target = prepared.work_key().target().clone();
    let expected = CurrentExpectation::freeze(&prepared, 1, &policy, Profile::Fixture).unwrap();
    let mut history = RunHistory::default();
    let token = CancellationToken::new();
    if cancel {
        token.cancel();
    }
    let complete = history
        .register(prepared, 0)
        .unwrap()
        .execute(finish, &token, |_| {})
        .unwrap();
    let bytes = serde_json::to_vec(complete.output()).unwrap();
    history.append(&complete).unwrap();
    history.publish(&complete).unwrap();
    (root, history, expected, review, policy, bytes, target)
}
fn baseline_port(review: &BaselineReview) -> FixtureApproval {
    let mut p = fixture_approval(&review.baseline);
    p.0.as_mut().unwrap().issued_at = NOW - 1;
    p.0.as_mut().unwrap().expires_at = NOW + 100;
    p
}
#[test]
fn private_attachment_reauthenticates_new_digest_and_fresh_review_approval() {
    let (_root, history, expected, review, policy, original, target) =
        setup(&document().replace("sign in", "sign out"));
    let port = baseline_port(&review);
    let old: specguard::integration::producer::ProducedRun =
        serde_json::from_slice(&original).unwrap();
    let authority = Authority {
        policy,
        digest: RefCell::new(specguard::model::digest(&old.envelope)),
        revoked: Cell::new(false),
        expires: Cell::new(NOW + 50),
        calls: Cell::new(0),
        unavailable: Cell::new(false),
        purpose: RefCell::new("review".into()),
    };
    let clock = Clock(Cell::new(NOW));
    let check = || {
        Some(BaselineCheck {
            baseline: &review.baseline,
            authority: &port,
        })
    };
    assert_eq!(
        history
            .consume_current(&expected, &authority, &clock, check())
            .unwrap()
            .assessment()
            .code,
        EligibilityCode::MissingApproval
    );
    assert_eq!(authority.calls.get(), 0);
    let mut refs = vec!["fixture:review".into()];
    let attached = history.attach_current(&expected, &refs).unwrap();
    refs[0] = "caller-mutated".into();
    assert_eq!(attached.envelope().approval_refs, ["fixture:review"]);
    assert!(
        !history
            .consume_attached(&expected, &attached, &authority, &clock, check())
            .unwrap()
            .eligible()
    );
    assert_eq!(
        authority.calls.get(),
        0,
        "old producer envelope digest cannot authenticate attachment"
    );
    *authority.digest.borrow_mut() = specguard::model::digest(attached.envelope());
    let result = history
        .consume_attached(&expected, &attached, &authority, &clock, check())
        .unwrap();
    assert!(result.eligible());
    assert_eq!(
        result.assessment().technical_decision,
        Some(guardengine::Decision::RequireApproval)
    );
    assert!(attached.envelope().expires_at.is_none());
    authority.expires.set(i64::MAX);
    authority.revoked.set(true);
    assert!(
        !history
            .consume_attached(&expected, &attached, &authority, &clock, check())
            .unwrap()
            .eligible()
    );
    authority.revoked.set(false);
    authority.unavailable.set(true);
    assert!(
        !history
            .consume_attached(&expected, &attached, &authority, &clock, check())
            .unwrap()
            .eligible()
    );
    authority.unavailable.set(false);
    let changed_refs = history
        .attach_current(&expected, &["fixture:another-review".into()])
        .unwrap();
    let before = authority.calls.get();
    assert!(
        !history
            .consume_attached(&expected, &changed_refs, &authority, &clock, check())
            .unwrap()
            .eligible()
    );
    assert_eq!(
        authority.calls.get(),
        before,
        "new references need their own new producer envelope digest"
    );
    authority.expires.set(NOW + 50);
    clock.0.set(NOW + 50);
    assert!(
        !history
            .consume_attached(&expected, &attached, &authority, &clock, check())
            .unwrap()
            .eligible()
    );
    clock.0.set(NOW + 1);
    assert!(
        history
            .consume_attached(&expected, &attached, &authority, &clock, check())
            .is_err()
    );
    assert_eq!(attached.report_bytes(), old.report.as_deref());
    assert_eq!(attached.domain_bytes(), old.domain.as_deref());
    assert!(
        RunHistory::default()
            .consume_attached(&expected, &attached, &authority, &clock, check())
            .is_err()
    );
    assert_eq!(
        serde_json::to_vec(history.current(&target).unwrap().output()).unwrap(),
        original
    );
}

#[test]
fn exact_approval_bindings_are_required_and_other_store_cannot_import_attachment() {
    let (root, history, expected, review, policy, _original, _target) =
        setup(&document().replace("sign in", "sign out"));
    let port = baseline_port(&review);
    let refs = vec!["fixture:review".into()];
    let attached = history.attach_current(&expected, &refs).unwrap();
    let clock = Clock(Cell::new(NOW));
    let mut authority = Authority {
        policy: policy.clone(),
        digest: RefCell::new(specguard::model::digest(attached.envelope())),
        revoked: Cell::new(false),
        expires: Cell::new(NOW + 50),
        calls: Cell::new(0),
        unavailable: Cell::new(false),
        purpose: RefCell::new("review".into()),
    };
    for dimension in 0..4 {
        authority.policy = policy.clone();
        *authority.purpose.borrow_mut() = "review".into();
        match dimension {
            0 => authority
                .policy
                .binding
                .requirement_ids
                .push("demo:foreign".into()),
            1 => authority.policy.contract_digest = format!("sha256:{}", "f".repeat(64)),
            2 => authority.policy.action = "other-action".into(),
            _ => *authority.purpose.borrow_mut() = "other-purpose".into(),
        }
        assert!(
            !history
                .consume_attached(
                    &expected,
                    &attached,
                    &authority,
                    &clock,
                    Some(BaselineCheck {
                        baseline: &review.baseline,
                        authority: &port
                    })
                )
                .unwrap()
                .eligible()
        );
    }
    authority.policy = policy.clone();
    *authority.purpose.borrow_mut() = "review".into();
    let s = specguard::source::freeze(
        root.path(),
        &specguard::source::discover(root.path(), &common::policy()).unwrap(),
        binding(root.path()),
    )
    .unwrap();
    let mut inv = producer_support::invocation(&s);
    inv.baseline_digest = Some(specguard::model::digest(&review.baseline));
    let prepared = prepare_baseline_review(
        root.path(),
        &s,
        inv,
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
        &review,
    )
    .unwrap();
    let changed_generation =
        CurrentExpectation::freeze(&prepared, 2, &policy, Profile::Fixture).unwrap();
    assert!(
        history
            .consume_attached(
                &changed_generation,
                &attached,
                &authority,
                &clock,
                Some(BaselineCheck {
                    baseline: &review.baseline,
                    authority: &port
                })
            )
            .is_err()
    );
    let mut other = RunHistory::default();
    let done = other
        .register(prepared, 0)
        .unwrap()
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new(), |_| {})
        .unwrap();
    other.append(&done).unwrap();
    other.publish(&done).unwrap();
    let other_attachment = other.attach_current(&expected, &refs).unwrap();
    assert_eq!(other_attachment.envelope(), attached.envelope());
    assert!(
        other
            .consume_attached(
                &expected,
                &attached,
                &authority,
                &clock,
                Some(BaselineCheck {
                    baseline: &review.baseline,
                    authority: &port
                })
            )
            .is_err()
    );
    assert!(
        history
            .consume_attached(
                &expected,
                &other_attachment,
                &authority,
                &clock,
                Some(BaselineCheck {
                    baseline: &review.baseline,
                    authority: &port
                })
            )
            .is_err()
    );
}
#[test]
fn references_are_bounded_and_nonreview_or_partial_evidence_cannot_be_attached() {
    let (_root, history, expected, _review, _policy, _original, _target) =
        setup(&document().replace("sign in", "sign out"));
    for refs in [
        vec![],
        vec!["fixture:review".into(); 65],
        vec!["".into()],
        vec!["bad\nref".into()],
        vec!["z".into(), "a".into()],
        vec!["same".into(), "same".into()],
        vec!["x".repeat(17 * 1024 * 1024)],
        (0..17)
            .map(|i| format!("{i:02}{}", "x".repeat(1022)))
            .collect(),
    ] {
        assert!(history.attach_current(&expected, &refs).is_err());
    }
    for text in [
        document(),
        "malformed",
        "---\nformat: markdown-explicit/v1\nnamespace: demo\n---\n## Requirement: R1\nMissing acceptance\n",
    ] {
        let (_root, history, expected, _review, _policy, _original, _target) = setup(text);
        assert!(
            history
                .attach_current(&expected, &["fixture:review".into()])
                .is_err()
        );
    }
}

#[test]
fn failed_and_cancelled_runs_cannot_acquire_controller_approval_references() {
    for (cancel, finish) in [(false, "invalid"), (true, "2026-10-09T10:00:01Z")] {
        let (_root, history, expected, _review, _policy, _original, _target) =
            setup_state(&document().replace("sign in", "sign out"), cancel, finish);
        assert!(
            history
                .attach_current(&expected, &["fixture:review".into()])
                .is_err()
        );
    }
}

#[test]
fn concurrent_attachment_uses_shared_time_high_water_mark() {
    use std::sync::{Mutex, mpsc};
    struct Blocking {
        policy: EligibilityPolicy,
        digest: String,
        ready: mpsc::SyncSender<()>,
        resume: Mutex<mpsc::Receiver<()>>,
    }
    impl AuthorityProvider for Blocking {
        fn verify_producer(
            &self,
            _: &GuardRunEnvelope,
            _: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            Ok(ProducerRecord {
                principal: "fixture:producer".into(),
                producer: self.policy.producer.clone(),
                envelope_digest: self.digest.clone(),
                validity: Validity {
                    issued_at: NOW - 1,
                    expires_at: NOW + 100,
                    revoked: false,
                },
            })
        }
        fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
            self.ready.send(()).unwrap();
            self.resume.lock().unwrap().recv().unwrap();
            Ok(ApprovalRecord {
                principal: "fixture:reviewer".into(),
                purpose: "review".into(),
                action: self.policy.action.clone(),
                binding: self.policy.binding.clone(),
                contract_digest: self.policy.contract_digest.clone(),
                validity: Validity {
                    issued_at: NOW - 1,
                    expires_at: NOW + 100,
                    revoked: false,
                },
            })
        }
    }
    let (_root, history, expected, review, policy, original, _target) =
        setup(&document().replace("sign in", "sign out"));
    let port = baseline_port(&review);
    let old: specguard::integration::producer::ProducedRun =
        serde_json::from_slice(&original).unwrap();
    let attached = history
        .attach_current(&expected, &["fixture:review".into()])
        .unwrap();
    let (ready, observed) = mpsc::sync_channel(0);
    let (resume, release) = mpsc::sync_channel(0);
    let slow = Blocking {
        policy: policy.clone(),
        digest: specguard::model::digest(attached.envelope()),
        ready,
        resume: Mutex::new(release),
    };
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            history.consume_attached(
                &expected,
                &attached,
                &slow,
                &Clock(Cell::new(NOW)),
                Some(BaselineCheck {
                    baseline: &review.baseline,
                    authority: &port,
                }),
            )
        });
        observed.recv().unwrap();
        let fast = Authority {
            policy,
            digest: RefCell::new(specguard::model::digest(&old.envelope)),
            revoked: Cell::new(false),
            expires: Cell::new(NOW + 100),
            calls: Cell::new(0),
            unavailable: Cell::new(false),
            purpose: RefCell::new("review".into()),
        };
        assert_eq!(
            history
                .consume_current(
                    &expected,
                    &fast,
                    &Clock(Cell::new(NOW + 6)),
                    Some(BaselineCheck {
                        baseline: &review.baseline,
                        authority: &port
                    })
                )
                .unwrap()
                .assessment()
                .code,
            EligibilityCode::MissingApproval
        );
        resume.send(()).unwrap();
        assert!(worker.join().unwrap().is_err());
    });
}

struct AllocationProbe;
thread_local! {static TRACK_ALLOC:Cell<bool>=const{Cell::new(false)};static MAX_ALLOC:Cell<usize>=const{Cell::new(0)};}
unsafe impl std::alloc::GlobalAlloc for AllocationProbe {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let p = unsafe { std::alloc::System.alloc(layout) };
        if !p.is_null() {
            TRACK_ALLOC.with(|flag| {
                if flag.get() {
                    MAX_ALLOC.with(|max| max.set(max.get().max(layout.size())))
                }
            });
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(p, layout) }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        let p = unsafe { std::alloc::System.realloc(p, layout, size) };
        if !p.is_null() {
            TRACK_ALLOC.with(|flag| {
                if flag.get() {
                    MAX_ALLOC.with(|max| max.set(max.get().max(size)))
                }
            });
        }
        p
    }
}
#[global_allocator]
static ALLOC: AllocationProbe = AllocationProbe;
#[test]
fn oversized_ref_is_rejected_before_allocating_an_attachment() {
    let (_root, history, expected, _review, _policy, _original, _target) =
        setup(&document().replace("sign in", "sign out"));
    let refs = vec!["x".repeat(17 * 1024 * 1024)];
    MAX_ALLOC.with(|max| max.set(0));
    TRACK_ALLOC.with(|flag| flag.set(true));
    let result = history.attach_current(&expected, &refs);
    TRACK_ALLOC.with(|flag| flag.set(false));
    let largest = MAX_ALLOC.with(Cell::get);
    assert!(result.is_err());
    println!("17MiB approval ref largest allocation during rejection={largest}");
    assert_eq!(largest, 0);
}
