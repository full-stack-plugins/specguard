mod common;
#[allow(dead_code)]
mod producer_support;
use common::*;
use gitguard::{
    Repository,
    candidate::{CandidateRequest, CandidateSnapshot},
    scope::TaskScope,
    subject::SubjectRequest,
};
use specguard::integration::{git_binding::*, runtime::CancellationToken};
use std::collections::BTreeSet;
fn policy() -> GitSourcePolicy {
    GitSourcePolicy::freeze(
        common::policy(),
        BTreeSet::from([key("R1")]),
        producer_support::policy(),
    )
    .unwrap()
}
fn fixture(format: &str) -> (tempfile::TempDir, String, String) {
    let root = tempfile::tempdir().unwrap();
    git(
        root.path(),
        &["init", "-q", &format!("--object-format={format}")],
    );
    std::fs::create_dir(root.path().join("specs")).unwrap();
    std::fs::write(root.path().join("specs/a.md"), document()).unwrap();
    git(root.path(), &["add", "."]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "base",
        ],
    );
    let base = git(root.path(), &["rev-parse", "HEAD"]);
    std::fs::write(
        root.path().join("specs/a.md"),
        document().replace("sign in", "log in"),
    )
    .unwrap();
    git(root.path(), &["add", "."]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "candidate",
        ],
    );
    let candidate = git(root.path(), &["rev-parse", "HEAD"]);
    (root, base, candidate)
}
fn candidate(
    repo: &Repository,
    base: &str,
    head: &str,
    policy: &GitSourcePolicy,
) -> CandidateSnapshot {
    let scope = TaskScope::advisory(
        "task",
        vec!["demo:R1".into()],
        vec![b"specs".to_vec()],
        policy.digest().trim_start_matches("sha256:"),
        None,
    )
    .unwrap();
    repo.prepare_candidate(
        &repo
            .resolve_subject(SubjectRequest::Commit(head.into()))
            .unwrap(),
        &scope,
        &CandidateRequest {
            worktree_id: "worktree".into(),
            base_oid: base.into(),
            merge_group_id: Some("queue-1".into()),
            members: vec![base.into(), head.into()],
        },
    )
    .unwrap()
}
fn attempt() -> Attempt {
    Attempt {
        run_id: "git-source-run".into(),
        started_at: "2026-10-09T10:00:00Z".into(),
    }
}
#[test]
fn actual_sha1_and_sha256_candidate_tree_is_used_despite_dirty_old_head() {
    for format in ["sha1", "sha256"] {
        let (root, base, head) = fixture(format);
        git(root.path(), &["checkout", "-q", &base]);
        std::fs::write(
            root.path().join("specs/a.md"),
            b"dirty unsupported document",
        )
        .unwrap();
        let repo = Repository::discover(root.path(), "repo").unwrap();
        let policy = policy();
        let candidate = candidate(&repo, &base, &head, &policy);
        assert!(!candidate.clean());
        assert!(GitPreparedRun::prepare_clean(&repo, &candidate, &policy, attempt()).is_err());
        let prepared = GitPreparedRun::prepare(&repo, &candidate, &policy, attempt()).unwrap();
        let expected = prepared.binding().clone();
        assert_eq!(expected.candidate_oid, head);
        assert_eq!(expected.base_oid, base);
        assert_ne!(
            expected.source_snapshot_digest,
            candidate.source_snapshot_digest()
        );
        std::fs::write(root.path().join("specs/a.md"), b"later mutation").unwrap();
        let output = prepared
            .execute("2026-10-09T10:00:01Z", &CancellationToken::new())
            .unwrap();
        assert_eq!(output.output().envelope.binding, expected);
        assert_eq!(
            output.output().envelope.decision,
            Some(guardengine::Decision::Allow)
        );
        output
            .verify(&repo, &candidate, &policy, &expected)
            .unwrap();
        let bytes = output.encode().unwrap();
        GitEvidenceBundle::load(&bytes, &repo, &candidate, &policy, &expected).unwrap();
    }
}
#[test]
fn policy_scope_queue_binding_and_tampered_bytes_fail_closed() {
    let (root, base, head) = fixture("sha1");
    let repo = Repository::discover(root.path(), "repo").unwrap();
    let policy = policy();
    let candidate = candidate(&repo, &base, &head, &policy);
    let prepared = GitPreparedRun::prepare(&repo, &candidate, &policy, attempt()).unwrap();
    let expected = prepared.binding().clone();
    let output = prepared
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new())
        .unwrap();
    let mut wrong = expected.clone();
    wrong.merge_group_id = Some("other".into());
    assert!(output.verify(&repo, &candidate, &policy, &wrong).is_err());
    let mut mapping = producer_support::policy();
    mapping.contract.metadata.revision = "other".into();
    let changed =
        GitSourcePolicy::freeze(common::policy(), BTreeSet::from([key("R1")]), mapping).unwrap();
    assert!(GitPreparedRun::prepare(&repo, &candidate, &changed, attempt()).is_err());
    let mut value: serde_json::Value = serde_json::from_slice(&output.encode().unwrap()).unwrap();
    value["filesDigest"] = "sha256:fake".into();
    assert!(
        GitEvidenceBundle::load(
            &serde_json::to_vec(&value).unwrap(),
            &repo,
            &candidate,
            &policy,
            &expected
        )
        .is_err()
    );
    let tree = git(root.path(), &["rev-parse", "HEAD^{tree}"]);
    assert!(repo.read_commit_files(&tree).is_err());
    let subject = repo.resolve_subject(SubjectRequest::Worktree).unwrap();
    assert!(subject.candidate_oid().is_none());
}

#[test]
fn actual_synthetic_merge_candidate_and_context_change_are_distinct_from_branch_head() {
    let (root, base, head) = fixture("sha1");
    let repo = Repository::discover(root.path(), "repo").unwrap();
    let policy = policy();
    let preview = repo.preview_merge(&base, &head).unwrap();
    let synthetic = preview.candidate_oid().unwrap().to_owned();
    assert_ne!(synthetic, head);
    let scope = TaskScope::advisory(
        "task",
        vec!["demo:R1".into()],
        vec![b"specs".to_vec()],
        policy.digest().trim_start_matches("sha256:"),
        None,
    )
    .unwrap();
    let request = CandidateRequest {
        worktree_id: "worktree".into(),
        base_oid: base.clone(),
        merge_group_id: Some("merge-group-1".into()),
        members: vec![base.clone(), head.clone()],
    };
    let candidate = repo.prepare_candidate(&preview, &scope, &request).unwrap();
    assert!(candidate.clean());
    let prepared = GitPreparedRun::prepare_clean(&repo, &candidate, &policy, attempt()).unwrap();
    let expected = prepared.binding().clone();
    let first_key = prepared.work_key().clone();
    assert_eq!(expected.candidate_oid, synthetic);
    let mut other = request;
    other.merge_group_id = Some("merge-group-2".into());
    other.base_oid = head;
    other.members.reverse();
    let changed = repo.prepare_candidate(&preview, &scope, &other).unwrap();
    assert_ne!(
        &first_key,
        GitPreparedRun::prepare_clean(&repo, &changed, &policy, attempt())
            .unwrap()
            .work_key()
    );
    let output = prepared
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new())
        .unwrap();
    output
        .verify(&repo, &candidate, &policy, &expected)
        .unwrap();
    assert!(output.verify(&repo, &changed, &policy, &expected).is_err());
    std::fs::write(root.path().join("specs/a.md"), "dirty after clean capture").unwrap();
    assert!(GitPreparedRun::prepare_clean(&repo, &candidate, &policy, attempt()).is_err());
}
#[cfg(unix)]
#[test]
fn committed_symlink_is_rejected_by_actual_git_reader_without_following_it() {
    let (root, base, _) = fixture("sha1");
    std::os::unix::fs::symlink("/etc/passwd", root.path().join("link")).unwrap();
    git(root.path(), &["add", "link"]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "symlink",
        ],
    );
    let head = git(root.path(), &["rev-parse", "HEAD"]);
    let repo = Repository::discover(root.path(), "repo").unwrap();
    let policy = policy();
    let candidate = candidate(&repo, &base, &head, &policy);
    assert!(GitPreparedRun::prepare(&repo, &candidate, &policy, attempt()).is_err());
}

#[test]
fn even_self_consistent_domain_bytes_must_match_real_committed_source() {
    use sha2::{Digest, Sha256};
    let (root, base, head) = fixture("sha1");
    let repo = Repository::discover(root.path(), "repo").unwrap();
    let policy = policy();
    let candidate = candidate(&repo, &base, &head, &policy);
    let prepared = GitPreparedRun::prepare_clean(&repo, &candidate, &policy, attempt()).unwrap();
    let expected = prepared.binding().clone();
    let output = prepared
        .execute("2026-10-09T10:00:01Z", &CancellationToken::new())
        .unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&output.encode().unwrap()).unwrap();
    let bytes: Vec<u8> = serde_json::from_value(v["output"]["domain"].clone()).unwrap();
    let mut domain: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    domain["graph"]["parsed"]["requirements"][0]["text"] = "fabricated source text".into();
    let bytes = serde_json::to_vec(&domain).unwrap();
    v["output"]["domain"] = serde_json::json!(bytes);
    v["output"]["envelope"]["artifacts"]["domain"][0]["digest"] =
        format!("sha256:{:x}", Sha256::digest(&bytes)).into();
    let generic: specguard::integration::producer::ProducedRun =
        serde_json::from_value(v["output"].clone()).unwrap();
    generic.verify().unwrap();
    assert!(
        GitEvidenceBundle::load(
            &serde_json::to_vec(&v).unwrap(),
            &repo,
            &candidate,
            &policy,
            &expected
        )
        .is_err()
    );
}

#[test]
fn actual_git_preparation_is_not_skipped_by_cached_parsing() {
    let (root, base, head) = fixture("sha1");
    let repo = Repository::discover(root.path(), "repo").unwrap();
    let policy = policy();
    let candidate = candidate(&repo, &base, &head, &policy);
    let mut cache = specguard::cache::ParseCache::enabled();
    for run in ["cold", "warm"] {
        let mut a = attempt();
        a.run_id = run.into();
        let p = GitPreparedRun::prepare(&repo, &candidate, &policy, a).unwrap();
        let expected = p.binding().clone();
        let out = p
            .execute_with_cache(
                "2026-10-09T10:00:01Z",
                &CancellationToken::new(),
                &mut cache,
            )
            .unwrap();
        assert_eq!(out.output().envelope.run_id, run);
        out.verify(&repo, &candidate, &policy, &expected).unwrap();
    }
    assert_eq!(cache.stats().hits, 1);
    let mut forged = serde_json::to_value(&candidate).unwrap();
    forged["source_snapshot_digest"] = "f".repeat(64).into();
    let forged: CandidateSnapshot = serde_json::from_value(forged).unwrap();
    assert!(GitPreparedRun::prepare(&repo, &forged, &policy, attempt()).is_err());
}
