mod common;
mod producer_support;
use common::*;
use guardengine::integration::{Producer, eligibility::*};
use specguard::integration::{
    approval::Profile,
    audit::*,
    freshness::{ConsumptionClock, RunHistory},
    producer::prepare,
    runtime::CancellationToken,
};
use std::{cell::Cell, collections::BTreeSet};
const NOW: i64 = 1_791_540_002;
struct Clock(Cell<i64>);
impl ConsumptionClock for Clock {
    fn now(&self) -> Result<i64, String> {
        Ok(self.0.get())
    }
}
struct Access {
    denied: Cell<bool>,
}
impl EvidenceAccessPort for Access {
    fn profile(&self) -> Profile {
        Profile::Fixture
    }
    fn authorize(&self, request: &AccessRequest<'_>) -> Result<AccessRecord, AccessError> {
        if self.denied.get() {
            return Err(AccessError::Denied);
        }
        Ok(AccessRecord {
            actor: "secret:authenticated-session".into(),
            uri: request.uri.into(),
            work_digest: request.work_digest.into(),
            generation: request.generation,
            envelope_digest: request.envelope_digest.into(),
            artifact_digest: request.artifact_digest.into(),
            purpose: "read-evidence".into(),
            validity: Validity {
                issued_at: 0,
                expires_at: NOW + 10,
                revoked: false,
            },
        })
    }
}
struct Authority {
    producer: Producer,
    forged: Cell<bool>,
}
impl AuthorityProvider for Authority {
    fn verify_producer(
        &self,
        _: &guardengine::integration::GuardRunEnvelope,
        digest: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        Ok(ProducerRecord {
            principal: if self.forged.get() {
                "forged"
            } else {
                "fixture:sg"
            }
            .into(),
            producer: self.producer.clone(),
            envelope_digest: digest.into(),
            validity: Validity {
                issued_at: 0,
                expires_at: NOW + 10,
                revoked: false,
            },
        })
    }
    fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
        Err(AuthorityError::Untrusted)
    }
}
#[test]
fn audited_access_is_fresh_authorized_and_redacts_secrets_without_mutating_history() {
    let (root, s) = snapshot(document().as_bytes());
    let prepared = prepare(
        root.path(),
        &s,
        producer_support::invocation(&s),
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
    )
    .unwrap();
    let producer = Producer {
        guard: "SpecGuard".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        analyzer_id: "specguard.structural".into(),
        analyzer_version: "1".into(),
    };
    let expected = AuditExpectation::freeze(
        &prepared,
        1,
        &producer,
        &BTreeSet::from(["fixture:sg".into()]),
        Profile::Fixture,
    )
    .unwrap();
    let mut history = RunHistory::default();
    let completion = history
        .register(prepared, 0)
        .unwrap()
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&completion).unwrap();
    history.publish(&completion).unwrap();
    let stored = history.current(completion.work_key().target()).unwrap();
    let before = serde_json::to_vec(stored.output()).unwrap();
    let uri = evidence_uri(stored, ArtifactKind::Report).unwrap();
    let access = Access {
        denied: Cell::new(false),
    };
    let authority = Authority {
        producer,
        forged: Cell::new(false),
    };
    let clock = Clock(Cell::new(NOW));
    let mut trail = AuditTrail::default();
    let bytes = trail
        .resolve(stored, &expected, &uri, &access, &authority, &clock)
        .unwrap();
    assert_eq!(bytes.bytes(), stored.output().report.as_ref().unwrap());
    authority.forged.set(true);
    assert!(
        trail
            .resolve(stored, &expected, &uri, &access, &authority, &clock)
            .is_err()
    );
    authority.forged.set(false);
    access.denied.set(true);
    assert!(
        trail
            .resolve(stored, &expected, &uri, &access, &authority, &clock)
            .is_err()
    );
    access.denied.set(false);
    assert!(
        trail
            .resolve(
                stored,
                &expected,
                "file:///etc/passwd",
                &access,
                &authority,
                &clock
            )
            .is_err()
    );
    let json = serde_json::to_string(trail.records()).unwrap();
    assert!(!json.contains("secret:"));
    assert!(!json.contains("passwd"));
    assert_eq!(trail.records().len(), 4);
    assert_eq!(before, serde_json::to_vec(stored.output()).unwrap());
}
fn fixture() -> (
    RunHistory,
    specguard::integration::freshness::Completion,
    AuditExpectation,
    Authority,
) {
    fixture_for("task-1")
}
fn fixture_for(
    task: &str,
) -> (
    RunHistory,
    specguard::integration::freshness::Completion,
    AuditExpectation,
    Authority,
) {
    let (root, s) = snapshot(document().as_bytes());
    let mut invocation = producer_support::invocation(&s);
    invocation.task_id = task.into();
    let prepared = prepare(
        root.path(),
        &s,
        invocation,
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
    )
    .unwrap();
    let producer = Producer {
        guard: "SpecGuard".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        analyzer_id: "specguard.structural".into(),
        analyzer_version: "1".into(),
    };
    let expected = AuditExpectation::freeze(
        &prepared,
        1,
        &producer,
        &BTreeSet::from(["fixture:sg".into()]),
        Profile::Fixture,
    )
    .unwrap();
    assert!(matches!(
        AuditExpectation::freeze(
            &prepared,
            1,
            &producer,
            &BTreeSet::from(["fixture:sg".into()]),
            Profile::Production
        ),
        Err(AuditError::Profile)
    ));
    let mut history = RunHistory::default();
    let completion = history
        .register(prepared, 0)
        .unwrap()
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&completion).unwrap();
    history.publish(&completion).unwrap();
    (
        history,
        completion,
        expected,
        Authority {
            producer,
            forged: Cell::new(false),
        },
    )
}
struct MalformedAccess(&'static str);
impl EvidenceAccessPort for MalformedAccess {
    fn profile(&self) -> Profile {
        if self.0 == "production" {
            Profile::Production
        } else {
            Profile::Fixture
        }
    }
    fn authorize(&self, r: &AccessRequest<'_>) -> Result<AccessRecord, AccessError> {
        let mut v = Access {
            denied: Cell::new(false),
        }
        .authorize(r)?;
        match self.0 {
            "uri" => v.uri.push('x'),
            "work" => v.work_digest.push('x'),
            "generation" => v.generation += 1,
            "envelope" => v.envelope_digest.push('x'),
            "artifact" => v.artifact_digest.push('x'),
            "purpose" => v.purpose = "merge".into(),
            "revoked" => v.validity.revoked = true,
            "expired" => v.validity.expires_at = NOW,
            "future" => v.validity.issued_at = NOW + 1,
            "oversize" => v.actor = "x".repeat(17 * 1024 * 1024),
            "unavailable" => return Err(AccessError::Unavailable),
            _ => {}
        }
        Ok(v)
    }
}
#[test]
fn attachment_tamper_and_cross_resource_or_identity_claims_never_authorize() {
    let (history, completion, expected, authority) = fixture();
    let stored = history.current(completion.work_key().target()).unwrap();
    let uri = evidence_uri(stored, ArtifactKind::Report).unwrap();
    let clock = Clock(Cell::new(NOW));
    let mut trail = AuditTrail::default();
    // Raw recomputation demonstrates integrity only. It cannot import the clone into private history.
    let mut tampered = stored.output().clone();
    tampered.report.as_mut().unwrap().push(b'X');
    assert!(tampered.verify().is_err());
    for mode in [
        "uri",
        "work",
        "generation",
        "envelope",
        "artifact",
        "purpose",
        "revoked",
        "expired",
        "future",
        "oversize",
        "unavailable",
        "production",
    ] {
        assert!(
            trail
                .resolve(
                    stored,
                    &expected,
                    &uri,
                    &MalformedAccess(mode),
                    &authority,
                    &clock
                )
                .is_err(),
            "{mode}"
        );
    }
    let access = Access {
        denied: Cell::new(false),
    };
    for bad in [
        "https://host/report",
        "specguard-evidence:v1:../report",
        "specguard-evidence:v2:abc:report",
    ] {
        assert!(
            trail
                .resolve(stored, &expected, bad, &access, &authority, &clock)
                .is_err()
        )
    }
    let (other_history, other_completion, other_expected, _) = fixture_for("foreign-task");
    let other = other_history
        .current(other_completion.work_key().target())
        .unwrap();
    // Independent work keys include actual frozen source inventory, even for same run label.
    assert!(matches!(
        trail.resolve(other, &expected, &uri, &access, &authority, &clock),
        Err(AuditError::Binding)
    ));
    assert!(matches!(
        trail.resolve(stored, &other_expected, &uri, &access, &authority, &clock),
        Err(AuditError::Binding)
    ));
    for pair in trail.records().windows(2) {
        assert_eq!(
            pair[1].predecessor_digest.as_ref(),
            Some(&pair[0].record_digest)
        );
        assert_eq!(pair[1].sequence, pair[0].sequence + 1)
    }
}
#[test]
fn trusted_clock_expiry_rollback_and_audit_capacity_fail_closed() {
    let (history, completion, expected, authority) = fixture();
    let stored = history.current(completion.work_key().target()).unwrap();
    let uri = evidence_uri(stored, ArtifactKind::Report).unwrap();
    let clock = Clock(Cell::new(NOW));
    let access = Access {
        denied: Cell::new(false),
    };
    let mut trail = AuditTrail::default();
    assert!(
        trail
            .resolve(stored, &expected, &uri, &access, &authority, &clock)
            .is_ok()
    );
    clock.0.set(NOW + 11);
    assert!(matches!(
        trail.resolve(stored, &expected, &uri, &access, &authority, &clock),
        Err(AuditError::Access)
    ));
    clock.0.set(NOW + 1);
    assert!(matches!(
        trail.resolve(stored, &expected, &uri, &access, &authority, &clock),
        Err(AuditError::Clock)
    ));
    clock.0.set(NOW + 11);
    for _ in 3..256 {
        assert!(
            trail
                .resolve(stored, &expected, &uri, &access, &authority, &clock)
                .is_err()
        );
    }
    let before = serde_json::to_vec(trail.records()).unwrap();
    assert!(matches!(
        trail.resolve(stored, &expected, &uri, &access, &authority, &clock),
        Err(AuditError::Capacity)
    ));
    assert_eq!(before, serde_json::to_vec(trail.records()).unwrap());
}
#[test]
fn uri_inputs_are_bounded_and_audit_never_stores_caller_text() {
    let (history, completion, expected, authority) = fixture();
    let stored = history.current(completion.work_key().target()).unwrap();
    let secret = "secret-token".repeat(2 * 1024 * 1024);
    let mut trail = AuditTrail::default();
    assert!(matches!(
        trail.resolve(
            stored,
            &expected,
            &secret,
            &Access {
                denied: Cell::new(false)
            },
            &authority,
            &Clock(Cell::new(NOW))
        ),
        Err(AuditError::Budget)
    ));
    let output = serde_json::to_string(trail.records()).unwrap();
    assert!(output.len() < 1024);
    assert!(!output.contains("secret-token"));
}
thread_local! {static TRACK: Cell<bool> = const {Cell::new(false)};static LARGEST: Cell<usize> = const {Cell::new(0)};}
struct AllocationObserver;
unsafe impl std::alloc::GlobalAlloc for AllocationObserver {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        let _ = TRACK.try_with(|enabled| {
            if enabled.get() {
                let _ = LARGEST.try_with(|max| max.set(max.get().max(l.size())));
            }
        });
        unsafe { std::alloc::System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        unsafe { std::alloc::System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, n: usize) -> *mut u8 {
        let _ = TRACK.try_with(|enabled| {
            if enabled.get() {
                let _ = LARGEST.try_with(|max| max.set(max.get().max(n)));
            }
        });
        unsafe { std::alloc::System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOC: AllocationObserver = AllocationObserver;
struct OwnedAccess(std::cell::RefCell<Option<AccessRecord>>);
impl EvidenceAccessPort for OwnedAccess {
    fn profile(&self) -> Profile {
        Profile::Fixture
    }
    fn authorize(&self, _: &AccessRequest<'_>) -> Result<AccessRecord, AccessError> {
        Ok(self.0.borrow_mut().take().unwrap())
    }
}
#[test]
fn oversized_uri_and_provider_record_are_rejected_before_resolver_clone_or_hash() {
    let (history, completion, expected, authority) = fixture();
    let stored = history.current(completion.work_key().target()).unwrap();
    let mut trail = AuditTrail::default();
    let clock = Clock(Cell::new(NOW));
    let uri = "s".repeat(17 * 1024 * 1024);
    LARGEST.set(0);
    TRACK.set(true);
    let result = trail.resolve(
        stored,
        &expected,
        &uri,
        &Access {
            denied: Cell::new(false),
        },
        &authority,
        &clock,
    );
    TRACK.set(false);
    let max = LARGEST.get();
    assert!(matches!(result, Err(AuditError::Budget)));
    assert!(max < 4096, "URI maximum allocation {max}");
    let uri = evidence_uri(stored, ArtifactKind::Report).unwrap();
    let provider = OwnedAccess(std::cell::RefCell::new(Some(AccessRecord {
        actor: "s".repeat(17 * 1024 * 1024),
        uri: uri.clone(),
        work_digest: String::new(),
        generation: 1,
        envelope_digest: String::new(),
        artifact_digest: String::new(),
        purpose: "read-evidence".into(),
        validity: Validity {
            issued_at: 0,
            expires_at: NOW + 100,
            revoked: false,
        },
    })));
    LARGEST.set(0);
    TRACK.set(true);
    let result = trail.resolve(stored, &expected, &uri, &provider, &authority, &clock);
    TRACK.set(false);
    let max = LARGEST.get();
    assert!(matches!(result, Err(AuditError::Budget)));
    assert!(max < 65536, "provider maximum allocation {max}");
    println!(
        "oversized borrowed/provider input admitted without large clone; max provider allocation {max}"
    );
}
struct WrongProducer<'a>(&'a Authority, &'static str);
impl AuthorityProvider for WrongProducer<'_> {
    fn verify_producer(
        &self,
        e: &guardengine::integration::GuardRunEnvelope,
        d: &str,
    ) -> Result<ProducerRecord, AuthorityError> {
        if self.1 == "unavailable" {
            return Err(AuthorityError::Unavailable);
        }
        let mut p = self.0.verify_producer(e, d)?;
        match self.1 {
            "digest" => p.envelope_digest.push('x'),
            "identity" => p.producer.analyzer_id.push('x'),
            "revoked" => p.validity.revoked = true,
            "expired" => p.validity.expires_at = NOW,
            _ => {}
        }
        Ok(p)
    }
    fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
        Err(AuthorityError::Untrusted)
    }
}
struct AdvancingAccess<'a>(&'a Clock);
impl EvidenceAccessPort for AdvancingAccess<'_> {
    fn profile(&self) -> Profile {
        Profile::Fixture
    }
    fn authorize(&self, r: &AccessRequest<'_>) -> Result<AccessRecord, AccessError> {
        let record = Access {
            denied: Cell::new(false),
        }
        .authorize(r)?;
        self.0.0.set(NOW + 11);
        Ok(record)
    }
}
#[test]
fn actual_envelope_producer_authentication_and_end_of_read_expiry_are_required() {
    let (history, completion, expected, authority) = fixture();
    let stored = history.current(completion.work_key().target()).unwrap();
    let uri = evidence_uri(stored, ArtifactKind::Report).unwrap();
    let clock = Clock(Cell::new(NOW));
    let access = Access {
        denied: Cell::new(false),
    };
    let mut trail = AuditTrail::default();
    for mode in ["digest", "identity", "revoked", "expired", "unavailable"] {
        assert!(
            matches!(
                trail.resolve(
                    stored,
                    &expected,
                    &uri,
                    &access,
                    &WrongProducer(&authority, mode),
                    &clock
                ),
                Err(AuditError::Producer)
            ),
            "{mode}"
        );
    }
    assert!(matches!(
        trail.resolve(
            stored,
            &expected,
            &uri,
            &AdvancingAccess(&clock),
            &authority,
            &clock
        ),
        Err(AuditError::Access)
    ));
    clock.0.set(NOW + 1);
    assert!(matches!(
        trail.resolve(stored, &expected, &uri, &access, &authority, &clock),
        Err(AuditError::Clock)
    ));
}
