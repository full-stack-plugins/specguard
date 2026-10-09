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
