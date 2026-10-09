mod common;
mod producer_support;
use common::*;
use specguard::integration::{freshness::*, producer::*, runtime::CancellationToken};
use std::collections::BTreeSet;
const FINISH: &str = "2026-10-09T10:00:01Z";
fn prepared(
    root: &std::path::Path,
    s: &specguard::source::SourceSnapshot,
    run: &str,
    policy: &ProtectedMapping,
    task: &str,
) -> PreparedRun {
    let mut invocation = producer_support::invocation(s);
    invocation.run_id = run.into();
    invocation.task_id = task.into();
    prepare(root, s, invocation, &BTreeSet::from([key("R1")]), policy).unwrap()
}
#[test]
fn full_work_key_changes_with_policy_baseline_scope_source_and_invocation_but_retry_is_same_work() {
    let (root, s) = snapshot(document().as_bytes());
    let policy = producer_support::policy();
    let first = prepared(root.path(), &s, "first", &policy, "task");
    let retry = prepared(root.path(), &s, "retry", &policy, "task");
    assert_eq!(first.work_key(), retry.work_key());
    let mut changed = policy.clone();
    changed.contract.metadata.revision = "2".into();
    assert_ne!(
        first.work_key(),
        prepared(root.path(), &s, "policy", &changed, "task").work_key()
    );
    assert_ne!(
        first.work_key(),
        prepared(root.path(), &s, "task", &policy, "other").work_key()
    );
    let mut invocation = producer_support::invocation(&s);
    invocation.task_id = "task".into();
    invocation.baseline_digest = Some(format!("sha256:{}", "a".repeat(64)));
    let baseline = prepare(
        root.path(),
        &s,
        invocation,
        &BTreeSet::from([key("R1")]),
        &policy,
    )
    .unwrap();
    assert_ne!(first.work_key(), baseline.work_key());
}
#[test]
fn retries_require_new_run_id_and_cas_and_late_completion_is_history_only() {
    let (root, s) = snapshot(document().as_bytes());
    let p = producer_support::policy();
    let mut history = RunHistory::default();
    let old = history
        .register(prepared(root.path(), &s, "old", &p, "task"), 0)
        .unwrap();
    let target = old.work_key().target().clone();
    assert!(matches!(
        history.register(prepared(root.path(), &s, "old", &p, "task"), 1),
        Err(HistoryError::Conflict)
    ));
    assert!(matches!(
        history.register(prepared(root.path(), &s, "bad-cas", &p, "task"), 0),
        Err(HistoryError::Stale)
    ));
    let retry = history
        .register(prepared(root.path(), &s, "retry", &p, "task"), 1)
        .unwrap();
    let newer = retry
        .execute(FINISH, &CancellationToken::new(), |_| {})
        .unwrap();
    assert_eq!(history.append(&newer).unwrap(), AppendResult::Inserted);
    history.publish(&newer).unwrap();
    assert_eq!(history.current(&target).unwrap().run_id(), "retry");
    let older = old
        .execute(FINISH, &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&older).unwrap();
    assert_eq!(history.publish(&older), Err(HistoryError::Stale));
    assert_eq!(history.current(&target).unwrap().run_id(), "retry");
    assert_eq!(history.history(&target).len(), 2);
    assert_eq!(
        history.append(&newer).unwrap(),
        AppendResult::IdenticalReplay
    );
}
#[test]
fn private_completion_cannot_be_relabelled_or_imported_into_another_store() {
    let (root, s) = snapshot(document().as_bytes());
    let p = producer_support::policy();
    let mut a = RunHistory::default();
    let mut b = RunHistory::default();
    let old = a
        .register(prepared(root.path(), &s, "same", &p, "task"), 0)
        .unwrap();
    let _new = b
        .register(prepared(root.path(), &s, "same", &p, "task"), 0)
        .unwrap();
    let completion = old
        .execute(FINISH, &CancellationToken::new(), |_| {})
        .unwrap();
    assert_eq!(b.append(&completion), Err(HistoryError::ForeignCompletion));
    let mut raw = completion.output().clone();
    raw.envelope.run_id = "relabel".into();
    assert_ne!(raw.envelope.run_id, completion.output().envelope.run_id);
    a.append(&completion).unwrap();
}

#[test]
fn actual_parallel_requirements_and_late_old_policy_do_not_cross_current_pointers() {
    let (root, s) = snapshot(
        format!(
            "{}\n## Requirement: R2\nSecond requirement.\n### Acceptance: A2\nSecond acceptance.\n",
            document()
        )
        .as_bytes(),
    );
    let policy = producer_support::policy();
    let mut history = RunHistory::default();
    let old = history
        .register(prepared(root.path(), &s, "old-policy", &policy, "task"), 0)
        .unwrap();
    let first_target = old.work_key().target().clone();
    let (ready_send, ready_recv) = std::sync::mpsc::channel();
    let (release_send, release_recv) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        old.execute(FINISH, &CancellationToken::new(), |_| {
            ready_send.send(()).unwrap();
            release_recv.recv().unwrap();
        })
        .unwrap()
    });
    ready_recv
        .recv_timeout(std::time::Duration::from_secs(3))
        .unwrap();
    let mut policy2 = policy.clone();
    for e in &mut policy2.entries {
        e.key = key("R2");
        e.subject = "demo:R2".into();
    }
    for r in &mut policy2.contract.spec.rules {
        let guardengine::GuardAssertion::ForbidRelation { subject, .. } = &mut r.assertion;
        *subject = "demo:R2".into();
    }
    let mut inv = producer_support::invocation(&s);
    inv.run_id = "requirement-2".into();
    inv.task_id = "task".into();
    let run2 = prepare(root.path(), &s, inv, &BTreeSet::from([key("R2")]), &policy2).unwrap();
    let second_target = run2.work_key().target().clone();
    assert_ne!(first_target, second_target);
    let registered2 = history.register(run2, 0).unwrap();
    let worker2 = std::thread::spawn(move || {
        registered2
            .execute(FINISH, &CancellationToken::new(), |_| {})
            .unwrap()
    });
    let result2 = worker2.join().unwrap();
    history.append(&result2).unwrap();
    history.publish(&result2).unwrap();
    let mut changed = policy;
    changed.contract.metadata.revision = "policy-2".into();
    let latest = history
        .register(prepared(root.path(), &s, "new-policy", &changed, "task"), 1)
        .unwrap();
    assert!(history.current(&first_target).is_none());
    let latest = latest
        .execute(FINISH, &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&latest).unwrap();
    history.publish(&latest).unwrap();
    release_send.send(()).unwrap();
    let late = worker.join().unwrap();
    assert_ne!(late.work_key(), latest.work_key());
    history.append(&late).unwrap();
    assert_eq!(history.publish(&late), Err(HistoryError::Stale));
    assert_eq!(
        history.current(&first_target).unwrap().run_id(),
        "new-policy"
    );
    assert_eq!(
        history.current(&second_target).unwrap().run_id(),
        "requirement-2"
    );
    assert_eq!(history.history(&first_target).len(), 2);
    assert_eq!(history.history(&second_target).len(), 1);
}
#[test]
fn advancing_clears_old_success_and_cancelled_retry_is_current_local_failure() {
    let (root, s) = snapshot(document().as_bytes());
    let p = producer_support::policy();
    let mut history = RunHistory::default();
    let run = history
        .register(prepared(root.path(), &s, "first", &p, "task"), 0)
        .unwrap();
    let target = run.work_key().target().clone();
    let done = run
        .execute(FINISH, &CancellationToken::new(), |_| {})
        .unwrap();
    history.append(&done).unwrap();
    history.publish(&done).unwrap();
    let retry = history
        .register(prepared(root.path(), &s, "cancelled", &p, "task"), 1)
        .unwrap();
    assert!(history.current(&target).is_none());
    let token = CancellationToken::new();
    token.cancel();
    let cancelled = retry.execute(FINISH, &token, |_| {}).unwrap();
    history.append(&cancelled).unwrap();
    history.publish(&cancelled).unwrap();
    assert!(
        history
            .current(&target)
            .unwrap()
            .output()
            .envelope
            .decision
            .is_none()
    );
    assert_eq!(
        history
            .current(&target)
            .unwrap()
            .output()
            .envelope
            .run_status,
        guardengine::integration::RunStatus::Cancelled
    );
}

#[test]
fn all_mutable_source_and_binding_dimensions_change_work_identity() {
    let (root, s) = snapshot(document().as_bytes());
    let p = producer_support::policy();
    let first = prepared(root.path(), &s, "first", &p, "task");
    for field in 0..3 {
        let mut inv = producer_support::invocation(&s);
        inv.task_id = "task".into();
        match field {
            0 => inv.worktree_id = "different-worktree".into(),
            1 => inv.merge_group_id = Some("queue-2".into()),
            _ => inv.repo_id = "other-repository".into(),
        };
        let changed = prepare(root.path(), &s, inv, &BTreeSet::from([key("R1")]), &p).unwrap();
        assert_ne!(first.work_key(), changed.work_key());
    }
    let mut inv = producer_support::invocation(&s);
    inv.task_id = "task".into();
    inv.run_id = "retry".into();
    inv.started_at = "2026-10-09T10:00:01Z".into();
    assert_eq!(
        first.work_key(),
        prepare(root.path(), &s, inv, &BTreeSet::from([key("R1")]), &p)
            .unwrap()
            .work_key()
    );
    let mut budget = s.clone();
    budget.inventory.limits.max_lines += 1;
    budget.digest =
        specguard::model::digest(&(&budget.inventory, &budget.binding, &budget.contents));
    assert_ne!(
        first.work_key(),
        prepared(root.path(), &budget, "budget", &p, "task").work_key()
    );
    std::fs::write(
        root.path().join("specs/a.md"),
        document().replace("sign in", "log in"),
    )
    .unwrap();
    let changed = specguard::source::freeze(
        root.path(),
        &specguard::source::discover(root.path(), &common::policy()).unwrap(),
        binding(root.path()),
    )
    .unwrap();
    assert_ne!(
        first.work_key(),
        prepared(root.path(), &changed, "source", &p, "task").work_key()
    );
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
            "next candidate",
        ],
    );
    let changed = specguard::source::freeze(
        root.path(),
        &specguard::source::discover(root.path(), &common::policy()).unwrap(),
        binding(root.path()),
    )
    .unwrap();
    assert_ne!(
        first.work_key(),
        prepared(root.path(), &changed, "candidate", &p, "task").work_key()
    );
}
#[test]
fn bounded_history_refuses_more_attempts_without_advancing_generation() {
    let (root, s) = snapshot(document().as_bytes());
    let p = producer_support::policy();
    let mut history = RunHistory::default();
    let target = prepared(root.path(), &s, "template", &p, "task")
        .work_key()
        .target()
        .clone();
    for generation in 0..256 {
        history
            .register(
                prepared(root.path(), &s, &format!("run-{generation}"), &p, "task"),
                generation,
            )
            .unwrap();
    }
    assert!(matches!(
        history.register(prepared(root.path(), &s, "over-capacity", &p, "task"), 256),
        Err(HistoryError::Capacity)
    ));
    assert_eq!(history.generation(&target), 256);
    assert!(history.current(&target).is_none());
}
