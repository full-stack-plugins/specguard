mod common;
use common::*;
use specguard::source::*;
#[test]
fn frozen_bytes_and_real_oids_reject_drift() {
    let (d, s) = snapshot(document().as_bytes());
    assert!(!s.digest.is_empty());
    std::fs::write(d.path().join("specs/a.md"), "changed").unwrap();
    assert!(freeze(d.path(), &s.inventory, binding(d.path())).is_err());
    let mut bad = binding(d.path());
    bad.candidate_oid = "0".repeat(40);
    assert!(freeze(d.path(), &discover(d.path(), &policy()).unwrap(), bad).is_err());
}
#[cfg(unix)]
#[test]
fn symlink_escape_is_rejected() {
    let d = repo();
    std::os::unix::fs::symlink("/tmp", d.path().join("specs")).unwrap();
    assert!(discover(d.path(), &policy()).is_err());
}
#[test]
fn sha256_git_repository_is_supported() {
    let d = tempfile::tempdir().unwrap();
    git(d.path(), &["init", "-q", "--object-format=sha256"]);
    git(
        d.path(),
        &[
            "-c",
            "user.name=f",
            "-c",
            "user.email=f@invalid",
            "commit",
            "--allow-empty",
            "-qm",
            "fixture",
        ],
    );
    std::fs::create_dir(d.path().join("specs")).unwrap();
    std::fs::write(d.path().join("specs/a.md"), document()).unwrap();
    let s = freeze(
        d.path(),
        &discover(d.path(), &policy()).unwrap(),
        binding(d.path()),
    )
    .unwrap();
    assert_eq!(s.binding.candidate_oid.len(), 64);
}

#[test]
fn parser_rejects_changed_binding_under_old_aggregate_digest() {
    use specguard::{graph::build_graph, parser::parse};
    let (_, original) = snapshot(document().as_bytes());
    assert!(build_graph(parse(&original)).complete());
    for field in 0..3 {
        let mut altered = original.clone();
        match field {
            0 => altered.binding.candidate_oid = "f".repeat(40),
            1 => altered.binding.base_oid = "e".repeat(40),
            _ => altered.binding.object_format = "sha256".into(),
        }
        let parsed = parse(&altered);
        assert!(
            !build_graph(parsed.clone()).complete(),
            "binding field {field}"
        );
        assert!(parsed.requirements.is_empty());
        assert!(
            parsed
                .sources
                .iter()
                .any(|s| s.reason.contains("aggregate snapshot digest"))
        );
    }
}

#[test]
fn parser_rejects_changed_bytes_and_entry_digest_under_old_aggregate_digest() {
    use specguard::{graph::build_graph, model::digest, parser::parse};
    let (_, mut altered) = snapshot(document().as_bytes());
    let bytes = document().replace("R1", "R2").into_bytes();
    altered.inventory.entries[0].digest = digest(&bytes);
    altered.contents.insert("specs/a.md".into(), bytes);
    let parsed = parse(&altered);
    assert!(!build_graph(parsed.clone()).complete());
    assert!(parsed.requirements.is_empty());
    assert!(
        parsed
            .sources
            .iter()
            .any(|s| s.reason.contains("aggregate snapshot digest"))
    );
}
