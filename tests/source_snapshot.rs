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

#[cfg(unix)]
#[test]
fn repository_root_and_ancestor_symlinks_are_rejected() {
    let (d, s) = snapshot(document().as_bytes());
    let aliases = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(d.path(), aliases.path().join("linked")).unwrap();
    let root = aliases.path().join("linked");
    assert!(discover(&root, &policy()).is_err());
    assert!(freeze(&root, &s.inventory, s.binding.clone()).is_err());
    std::os::unix::fs::symlink(d.path().parent().unwrap(), aliases.path().join("parent")).unwrap();
    let root = aliases
        .path()
        .join("parent")
        .join(d.path().file_name().unwrap());
    assert!(discover(&root, &policy()).is_err());
}

#[test]
fn duplicate_inventory_paths_cannot_silently_share_one_frozen_payload() {
    let (d, mut s) = snapshot(document().as_bytes());
    s.inventory.entries.push(s.inventory.entries[0].clone());
    assert!(freeze(d.path(), &s.inventory, s.binding).is_err());
}

#[test]
fn dirty_bytes_and_invalid_encoding_are_owned_without_fabricating_commit_identity() {
    use specguard::{model::Terminal, parser::parse};
    let (d, frozen) = snapshot(&[0xff, 0xfe]);
    let oid = binding(d.path()).candidate_oid;
    assert_eq!(frozen.binding.candidate_oid, oid);
    std::fs::write(d.path().join("specs/a.md"), document()).unwrap();
    assert_eq!(frozen.contents["specs/a.md"], vec![0xff, 0xfe]);
    let parsed = parse(&frozen);
    assert!(parsed.requirements.is_empty());
    assert!(
        parsed
            .sources
            .iter()
            .any(|s| s.status == Terminal::Malformed && s.reason.contains("UTF-8"))
    );
    let changed = freeze(
        d.path(),
        &discover(d.path(), &policy()).unwrap(),
        binding(d.path()),
    )
    .unwrap();
    assert_ne!(frozen.digest, changed.digest);
    assert_eq!(frozen.binding.candidate_oid, changed.binding.candidate_oid);
}

#[test]
fn full_git_oid_validation_rejects_wrong_object_kind_and_format() {
    let (d, snapshot) = snapshot(document().as_bytes());
    let mut wrong = snapshot.binding.clone();
    wrong.candidate_oid = git(d.path(), &["rev-parse", "HEAD^{tree}"]);
    assert!(
        freeze(d.path(), &snapshot.inventory, wrong)
            .unwrap_err()
            .contains("not a commit")
    );
    let mut wrong = snapshot.binding.clone();
    wrong.object_format = "sha256".into();
    assert!(
        freeze(d.path(), &snapshot.inventory, wrong)
            .unwrap_err()
            .contains("format mismatch")
    );
    let mut wrong = snapshot.binding;
    wrong.candidate_oid.truncate(8);
    assert!(
        freeze(d.path(), &snapshot.inventory, wrong)
            .unwrap_err()
            .contains("full Git OID")
    );
}

#[cfg(unix)]
#[test]
fn fifo_source_is_rejected_without_open_blocking() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let d = repo();
    std::fs::create_dir(d.path().join("specs")).unwrap();
    let path = CString::new(d.path().join("specs/a.md").as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let start = std::time::Instant::now();
    let inventory = discover(d.path(), &policy()).unwrap();
    assert!(inventory.entries.is_empty());
    assert!(
        inventory
            .sources
            .iter()
            .any(|s| s.status == specguard::model::Terminal::IoError)
    );
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
}
