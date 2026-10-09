mod common;
mod producer_support;
use common::*;
use serde_json::{Value, json};
use std::process::Command;
fn command(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_specguard"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
fn request(root: &std::path::Path, s: &specguard::source::SourceSnapshot) -> std::path::PathBuf {
    let p = root.join("request.json");
    std::fs::write(&p,serde_json::to_vec(&json!({"apiVersion":"specguard.cli-check/v1alpha1","sourcePolicy":policy(),"binding":s.binding,"required":[key("R1")],"invocation":producer_support::invocation(s),"mapping":producer_support::policy(),"finishedAt":"2026-10-09T10:00:01Z"})).unwrap()).unwrap();
    p
}
#[test]
fn actual_check_allow_block_partial_cancel_and_bound_error() {
    for (text, flag, expected, status) in [
        (document(), "", 0, "completed"),
        (
            "---\nformat: markdown-explicit/v1\nnamespace: demo\n---\n## Requirement: R1\nMissing acceptance\n",
            "",
            2,
            "completed",
        ),
        ("malformed", "", 2, "completed"),
        (document(), "--cancel", 4, "cancelled"),
    ] {
        let (root, s) = snapshot(text.as_bytes());
        let p = request(root.path(), &s);
        let mut args = vec!["check", root.path().to_str().unwrap(), p.to_str().unwrap()];
        if !flag.is_empty() {
            args.push(flag);
        }
        let out = command(root.path(), &args);
        assert_eq!(
            out.status.code(),
            Some(expected),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["envelope"]["runStatus"], status);
        assert_eq!(v["authenticationProfile"], "unverified");
        let reconstructed = specguard::integration::producer::ProducedRun {
            envelope: serde_json::from_value(v["envelope"].clone()).unwrap(),
            contract: serde_json::from_value(v["contract"].clone()).unwrap(),
            facts: serde_json::from_value(v["facts"].clone()).unwrap(),
            report: serde_json::from_value(v["report"].clone()).unwrap(),
            domain: serde_json::from_value(v["domain"].clone()).unwrap(),
        };
        reconstructed.verify().unwrap();
        if expected != 4 {
            assert!(out.stderr.is_empty());
        }

        if expected == 4 {
            assert!(v["envelope"]["decision"].is_null());
            assert!(v["report"].is_null());
        }
    }
    let (root, s) = snapshot(document().as_bytes());
    let p = request(root.path(), &s);
    let mut v: Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    v["finishedAt"] = "invalid".into();
    std::fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();
    let out = command(
        root.path(),
        &["check", root.path().to_str().unwrap(), p.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(4));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["envelope"]["runStatus"], "error");
    assert!(v["envelope"]["decision"].is_null());
}
#[test]
fn prebinding_failures_have_only_stderr_and_report_paths_are_never_modified() {
    let (root, s) = snapshot(document().as_bytes());
    let p = request(root.path(), &s);
    let report = root.path().join("old-report.json");
    std::fs::write(&report, b"old-success").unwrap();
    for args in [
        vec!["check", root.path().to_str().unwrap(), "missing.json"],
        vec![
            "check",
            root.path().to_str().unwrap(),
            p.to_str().unwrap(),
            "--report",
            report.to_str().unwrap(),
        ],
    ] {
        let out = command(root.path(), &args);
        assert_eq!(out.status.code(), Some(4));
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
        assert_eq!(std::fs::read(&report).unwrap(), b"old-success");
    }
    let out = command(root.path(), &["--help"]);
    assert!(out.status.success());
    let help = String::from_utf8(out.stdout).unwrap();
    for name in ["doctor", "scan", "trace", "diff", "check"] {
        assert!(help.contains(name));
    }
}
#[test]
fn real_query_commands_are_read_only_and_diff_explicitly_unverified() {
    let (root, s) = snapshot(document().as_bytes());
    for (name, value) in [
        ("policy", serde_json::to_value(policy()).unwrap()),
        ("binding", serde_json::to_value(&s.binding).unwrap()),
        ("required", json!([key("R1")])),
        ("baseline", serde_json::to_value(baseline()).unwrap()),
    ] {
        std::fs::write(
            root.path().join(format!("{name}.json")),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
    }
    let original = std::fs::read(root.path().join("specs/a.md")).unwrap();
    for args in [
        vec!["doctor", ".", "policy.json"],
        vec!["scan", ".", "policy.json", "binding.json", "required.json"],
        vec!["trace", ".", "policy.json", "binding.json", "required.json"],
        vec![
            "diff",
            ".",
            "policy.json",
            "binding.json",
            "baseline.json",
            "--unverified-baseline",
        ],
    ] {
        let out = command(root.path(), &args);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let _: Value = serde_json::from_slice(&out.stdout).unwrap();
    }
    assert_eq!(
        std::fs::read(root.path().join("specs/a.md")).unwrap(),
        original
    );
    let out = command(
        root.path(),
        &["diff", ".", "policy.json", "binding.json", "baseline.json"],
    );
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
}

#[test]
fn wrong_candidate_unknown_request_version_and_oversized_input_fail_before_envelope() {
    let (root, s) = snapshot(document().as_bytes());
    let p = request(root.path(), &s);
    let original: Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    for mode in 0..3 {
        let mut v = original.clone();
        match mode {
            0 => v["invocation"]["candidate_oid"] = "0".repeat(40).into(),
            1 => v["apiVersion"] = "future/v2".into(),
            _ => v["unexpected"] = true.into(),
        }
        std::fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();
        let out = command(root.path(), &["check", ".", p.to_str().unwrap()]);
        assert_eq!(out.status.code(), Some(4));
        assert!(out.stdout.is_empty());
        let _: Value = serde_json::from_slice(&out.stderr).unwrap();
    }
    std::fs::write(&p, vec![b' '; 1_048_577]).unwrap();
    let out = command(root.path(), &["check", ".", p.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
}

#[cfg(unix)]
#[test]
fn configuration_symlink_is_not_followed() {
    let (root, s) = snapshot(document().as_bytes());
    let p = request(root.path(), &s);
    let alias = root.path().join("link.json");
    std::os::unix::fs::symlink(&p, &alias).unwrap();
    let out = command(root.path(), &["check", ".", alias.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    assert!(p.exists());
}
