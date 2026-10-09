mod common;
mod producer_support;
use common::*;
use specguard::model::digest;
use specguard::{
    cache::ParseCache,
    integration::{
        producer::{PreparedRun, prepare},
        runtime::CancellationToken,
    },
    source::SourceSnapshot,
};
use std::{collections::BTreeSet, path::Path};
const FINISH: &str = "2026-10-09T10:00:01Z";
fn prepared(root: &Path, snapshot: &SourceSnapshot, run: &str) -> PreparedRun {
    let mut invocation = producer_support::invocation(snapshot);
    invocation.run_id = run.into();
    prepare(
        root,
        snapshot,
        invocation,
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
    )
    .unwrap()
}
#[test]
fn disabled_cold_and_warm_recompute_identical_evidence() {
    let (root, snapshot) = snapshot(document().as_bytes());
    let mut disabled = ParseCache::disabled();
    let mut cache = ParseCache::enabled();
    let run = |cache: &mut ParseCache| {
        prepared(root.path(), &snapshot, "same-run")
            .execute_with_cache(FINISH, &CancellationToken::new(), |_| {}, cache)
            .unwrap()
    };
    let off = run(&mut disabled);
    let cold = run(&mut cache);
    let warm = run(&mut cache);
    assert_eq!(
        serde_json::to_vec(&off).unwrap(),
        serde_json::to_vec(&cold).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&cold).unwrap(),
        serde_json::to_vec(&warm).unwrap()
    );
    assert_eq!(cache.stats().hits, 1);
    assert_eq!(cache.stats().misses, 1);
    assert_eq!(cache.len(), 1);
    assert_eq!(disabled.len(), 0);
    warm.verify().unwrap();
}
#[test]
fn retry_hits_parse_cache_but_builds_a_new_bound_run() {
    let (root, snapshot) = snapshot(document().as_bytes());
    let mut cache = ParseCache::enabled();
    let first = prepared(root.path(), &snapshot, "first")
        .execute_with_cache(FINISH, &CancellationToken::new(), |_| {}, &mut cache)
        .unwrap();
    let next = prepared(root.path(), &snapshot, "next")
        .execute_with_cache(FINISH, &CancellationToken::new(), |_| {}, &mut cache)
        .unwrap();
    assert_eq!(cache.stats().hits, 1);
    assert_eq!(first.envelope.run_id, "first");
    assert_eq!(next.envelope.run_id, "next");
    assert_ne!(first.facts, next.facts);
    next.verify().unwrap();
}
#[test]
fn invalid_finish_does_not_publish_a_parsed_cache_entry() {
    let (root, snapshot) = snapshot(document().as_bytes());
    let mut cache = ParseCache::enabled();
    let output = prepared(root.path(), &snapshot, "bad-finish")
        .execute_with_cache("invalid", &CancellationToken::new(), |_| {}, &mut cache)
        .unwrap();
    assert_eq!(
        output.envelope.run_status,
        guardengine::integration::RunStatus::Error
    );
    assert_eq!(cache.len(), 0);
    output.verify().unwrap();
}

#[test]
fn every_frozen_work_dimension_misses_and_tampered_snapshot_is_rejected() {
    let (root, original) = snapshot(document().as_bytes());
    let mut cache = ParseCache::enabled();
    prepared(root.path(), &original, "seed")
        .execute_with_cache(FINISH, &CancellationToken::new(), |_| {}, &mut cache)
        .unwrap();
    for case in 0..5 {
        let mut s = original.clone();
        let mut i = producer_support::invocation(&s);
        let mut m = producer_support::policy();
        let mut required = BTreeSet::from([key("R1")]);
        match case {
            0 => {
                s.contents
                    .get_mut("specs/a.md")
                    .unwrap()
                    .extend_from_slice(b"\nextra source text\n");
                s.inventory.entries[0].digest = digest(&s.contents["specs/a.md"]);
            }
            1 => s.inventory.limits.max_depth = 15,
            2 => m.contract.metadata.revision = "new-policy".into(),
            3 => i.baseline_digest = Some(format!("sha256:{}", "b".repeat(64))),
            _ => {
                required = BTreeSet::from([key("R2")]);
                for entry in &mut m.entries {
                    entry.key = key("R2");
                    entry.subject = "demo:R2".into();
                }
                for rule in &mut m.contract.spec.rules {
                    let guardengine::GuardAssertion::ForbidRelation { subject, .. } =
                        &mut rule.assertion;
                    *subject = "demo:R2".into();
                }
            }
        }
        s.digest = digest(&(&s.inventory, &s.binding, &s.contents));
        let output = prepare(root.path(), &s, i, &required, &m)
            .unwrap()
            .execute_with_cache(FINISH, &CancellationToken::new(), |_| {}, &mut cache)
            .unwrap();
        output.verify().unwrap();
        assert_eq!(cache.stats().hits, 0, "case {case}");
    }
    let mut tampered = original.clone();
    tampered.contents.get_mut("specs/a.md").unwrap().push(b'x');
    assert!(
        prepare(
            root.path(),
            &tampered,
            producer_support::invocation(&tampered),
            &BTreeSet::from([key("R1")]),
            &producer_support::policy()
        )
        .is_err()
    );
    assert_eq!(cache.stats().misses, 6);
}

#[test]
fn changed_or_zero_time_profile_does_not_turn_limit_into_a_hit() {
    let (root, original) = snapshot(document().as_bytes());
    let mut cache = ParseCache::enabled();
    prepared(root.path(), &original, "seed")
        .execute_with_cache(FINISH, &CancellationToken::new(), |_| {}, &mut cache)
        .unwrap();
    for case in 0..2 {
        let mut s = original.clone();
        if case == 0 {
            s.inventory.limits.max_millis = 0;
        } else {
            s.inventory.entries[0].format = "future-parser/v99".into();
        }
        s.digest = digest(&(&s.inventory, &s.binding, &s.contents));
        for _ in 0..2 {
            let output = prepared(root.path(), &s, "limited")
                .execute_with_cache(FINISH, &CancellationToken::new(), |_| {}, &mut cache)
                .unwrap();
            assert_ne!(
                output.envelope.coverage.status,
                guardengine::integration::CoverageStatus::Complete
            );
            output.verify().unwrap();
            assert_eq!(cache.len(), 1);
            assert_eq!(cache.stats().hits, 0);
        }
    }
}

#[test]
fn bounded_fifo_and_oversized_entry_bypass_preserve_evidence() {
    assert!(ParseCache::with_limits(9, 1024).is_err());
    assert!(ParseCache::with_limits(1, 8 * 1024 * 1024 + 1).is_err());
    let (root, s) = snapshot(document().as_bytes());
    let mut tiny = ParseCache::with_limits(1, 4096).unwrap();
    let expected = prepared(root.path(), &s, "same").complete(FINISH).unwrap();
    for _ in 0..2 {
        let out = prepared(root.path(), &s, "same")
            .execute_with_cache(FINISH, &CancellationToken::new(), |_| {}, &mut tiny)
            .unwrap();
        assert_eq!(
            serde_json::to_vec(&expected).unwrap(),
            serde_json::to_vec(&out).unwrap()
        );
    }
    assert_eq!(tiny.len(), 0);
    assert_eq!(tiny.charged_bytes(), 0);
    assert_eq!(tiny.stats().hits, 0);
    let mut cache = ParseCache::with_limits(2, 8 * 1024 * 1024).unwrap();
    for revision in ["a", "b", "c", "a"] {
        let mut mapping = producer_support::policy();
        mapping.contract.metadata.revision = revision.into();
        prepare(
            root.path(),
            &s,
            producer_support::invocation(&s),
            &BTreeSet::from([key("R1")]),
            &mapping,
        )
        .unwrap()
        .execute_with_cache(FINISH, &CancellationToken::new(), |_| {}, &mut cache)
        .unwrap();
        assert!(cache.len() <= 2);
        assert!(cache.charged_bytes() <= 8 * 1024 * 1024);
    }
    assert_eq!(cache.stats().misses, 4);
    assert_eq!(cache.stats().hits, 0);
}

#[test]
fn cache_hit_replays_real_worker_progress_and_midrun_cancellation() {
    let (root, s) = snapshot(document().as_bytes());
    let mut cache = ParseCache::enabled();
    prepared(root.path(), &s, "seed")
        .execute_with_cache(FINISH, &CancellationToken::new(), |_| {}, &mut cache)
        .unwrap();
    let p = prepared(root.path(), &s, "cancelled-hit");
    let token = CancellationToken::new();
    let worker_token = token.clone();
    let (send, recv) = std::sync::mpsc::channel();
    let (resume, wait) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let out = p
            .execute_with_cache(
                FINISH,
                &worker_token,
                |source| {
                    send.send(source.clone()).unwrap();
                    wait.recv().unwrap();
                },
                &mut cache,
            )
            .unwrap();
        (out, cache)
    });
    assert_eq!(
        recv.recv_timeout(std::time::Duration::from_secs(3))
            .unwrap()
            .reason,
        "parsed"
    );
    token.cancel();
    resume.send(()).unwrap();
    let (out, cache) = worker.join().unwrap();
    assert_eq!(
        out.envelope.run_status,
        guardengine::integration::RunStatus::Cancelled
    );
    assert_eq!(cache.len(), 1);
    out.verify().unwrap();
    assert!(
        String::from_utf8(out.domain.unwrap())
            .unwrap()
            .contains("specs/a.md")
    );
}

#[test]
fn cold_observer_panic_and_cancellation_leave_no_entries() {
    let (root, s) = snapshot(document().as_bytes());
    for panic in [false, true] {
        let mut cache = ParseCache::enabled();
        let token = CancellationToken::new();
        let out = prepared(root.path(), &s, "failed")
            .execute_with_cache(
                FINISH,
                &token,
                |_| {
                    if panic {
                        panic!("cache observer panic");
                    } else {
                        token.cancel();
                    }
                },
                &mut cache,
            )
            .unwrap();
        assert_ne!(
            out.envelope.run_status,
            guardengine::integration::RunStatus::Completed
        );
        assert_eq!(cache.len(), 0);
        out.verify().unwrap();
    }
}

#[test]
fn authority_refresh_remains_required_after_warm_parse() {
    use specguard::integration::approval::*;
    struct Port {
        inner: common::FixtureApproval,
        calls: std::cell::Cell<usize>,
    }
    impl ApprovalValidationPort for Port {
        fn profile(&self) -> Profile {
            Profile::Fixture
        }
        fn validate(
            &self,
            b: &specguard::baseline::ApprovedBaseline,
        ) -> Result<Authentication, ApprovalError> {
            self.calls.set(self.calls.get() + 1);
            self.inner.validate(b)
        }
    }
    let (root, s) = snapshot(document().as_bytes());
    let baseline = common::baseline();
    let mut port = Port {
        inner: common::fixture_approval(&baseline),
        calls: std::cell::Cell::new(0),
    };
    let mut cache = ParseCache::enabled();
    for run in ["first", "second"] {
        authenticate(&baseline, &port, Profile::Fixture, 20).unwrap();
        prepared(root.path(), &s, run)
            .execute_with_cache(FINISH, &CancellationToken::new(), |_| {}, &mut cache)
            .unwrap();
    }
    assert_eq!(cache.stats().hits, 1);
    assert_eq!(port.calls.get(), 2);
    port.inner.0.as_mut().unwrap().revoked = true;
    assert!(authenticate(&baseline, &port, Profile::Fixture, 20).is_err());
    assert_eq!(port.calls.get(), 3);
    // Parsing is advisory. The consumer still invokes authentication per use;
    // no ValidatedBaseline or approval observation enters the parse cache.
}
