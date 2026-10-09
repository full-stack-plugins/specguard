mod review_support;
use review_support::review_request;
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

#[test]
#[cfg(unix)]
fn report_directory_publishes_only_immutable_receipt_and_collision_is_bound_error() {
    use std::os::unix::fs::PermissionsExt;
    let (root, s) = snapshot(document().as_bytes());
    let p = request(root.path(), &s);
    let reports = root.path().join("receipts");
    std::fs::create_dir(&reports).unwrap();
    std::fs::set_permissions(&reports, std::fs::Permissions::from_mode(0o700)).unwrap();
    let args = [
        "check",
        root.path().to_str().unwrap(),
        p.to_str().unwrap(),
        "--report-dir",
        reports.to_str().unwrap(),
    ];
    let out = command(root.path(), &args);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    let files: Vec<_> = std::fs::read_dir(&reports)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    let saved = std::fs::read(&files[0]).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&saved).unwrap(),
        body["envelope"]
    );
    assert!(!body["contract"].is_null());
    let repeat = command(root.path(), &args);
    assert_eq!(repeat.status.code(), Some(4));
    let body: Value = serde_json::from_slice(&repeat.stdout).unwrap();
    assert_eq!(body["envelope"]["runStatus"], "error");
    assert!(body["envelope"]["decision"].is_null());
    assert_eq!(std::fs::read(&files[0]).unwrap(), saved);
}

#[test]
fn actual_baseline_diff_produces_review_without_weakening_structure() {
    for (text, expected) in [
        (document().to_owned(), 0),
        (
            document().replace("A user can sign in.", "A user can sign out."),
            3,
        ),
        (
            document().replace(
                "A correct password opens a session.",
                "A password opens an audited session.",
            ),
            3,
        ),
        (
            document().replace(
                "### Acceptance: A1\nA correct password opens a session.\n",
                "",
            ),
            2,
        ),
    ] {
        let (root, s) = snapshot(text.as_bytes());
        let p = review_request(root.path(), &s);
        let out = command(
            root.path(),
            &["check", root.path().to_str().unwrap(), p.to_str().unwrap()],
        );
        assert_eq!(
            out.status.code(),
            Some(expected),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["authenticationProfile"], "unverified");
        let report: guardengine::GuardReport = serde_json::from_slice(
            &serde_json::from_value::<Vec<u8>>(v["report"].clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            report.decision,
            if expected == 3 {
                guardengine::Decision::RequireApproval
            } else if expected == 2 {
                guardengine::Decision::Block
            } else {
                guardengine::Decision::Allow
            }
        );
    }
}

#[test]
fn review_request_drift_or_weakened_structural_policy_never_falls_back() {
    for change in 0..6 {
        let (root, s) = snapshot(document().as_bytes());
        let p = review_request(root.path(), &s);
        let mut v: Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
        match change {
            0 => {
                v.as_object_mut().unwrap().remove("review");
            }
            1 => {
                v["invocation"]["baseline_digest"] =
                    "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into()
            }
            2 => v["review"]["baseline"]["scope"] = json!([key("other")]),
            3 => v["mapping"]["contract"]["spec"]["rules"][0]["enforcement"] = "review".into(),
            4 => {
                v["review"]["contract"]["spec"]["rules"][0]["assertion"]["object"] =
                    "foreign".into()
            }
            _ => v["review"]["unknown"] = true.into(),
        }
        std::fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();
        let out = command(
            root.path(),
            &["check", root.path().to_str().unwrap(), p.to_str().unwrap()],
        );
        assert_eq!(out.status.code(), Some(4));
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
}
#[test]
fn report_flag_shape_and_noncheck_usage_fail_before_binding() {
    let (root, s) = snapshot(document().as_bytes());
    let p = request(root.path(), &s);
    for suffix in [
        vec!["--report-dir"],
        vec!["--report-dir", "x", "--report-dir", "y"],
        vec!["--cancel", "--cancel"],
        vec!["--unknown"],
    ] {
        let mut args = vec!["check", root.path().to_str().unwrap(), p.to_str().unwrap()];
        args.extend(suffix);
        let out = command(root.path(), &args);
        assert_eq!(out.status.code(), Some(4));
        assert!(out.stdout.is_empty());
    }
    let out = command(
        root.path(),
        &[
            "doctor",
            root.path().to_str().unwrap(),
            p.to_str().unwrap(),
            "--report-dir",
            "x",
        ],
    );
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
}

#[test]
#[cfg(unix)]
fn unsafe_report_directories_preserve_source_and_emit_bound_error() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let (root, s) = snapshot(document().as_bytes());
    let p = request(root.path(), &s);
    let dir = root.path().join("reports");
    std::fs::create_dir(&dir).unwrap();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    let link = root.path().join("report-link");
    symlink(&dir, &link).unwrap();
    let source = root.path().join("specs/a.md");
    let original = std::fs::read(&source).unwrap();
    for path in [&dir, &link, &source] {
        let out = command(
            root.path(),
            &[
                "check",
                root.path().to_str().unwrap(),
                p.to_str().unwrap(),
                "--report-dir",
                path.to_str().unwrap(),
            ],
        );
        assert_eq!(out.status.code(), Some(4));
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["envelope"]["runStatus"], "error");
        assert!(v["envelope"]["decision"].is_null());
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none());
        assert_eq!(std::fs::read(&source).unwrap(), original);
    }
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let out = command(
        root.path(),
        &[
            "check",
            root.path().to_str().unwrap(),
            p.to_str().unwrap(),
            "--report-dir",
            dir.to_str().unwrap(),
            "--cancel",
        ],
    );
    assert_eq!(out.status.code(), Some(4));
    let saved = std::fs::read(
        std::fs::read_dir(&dir)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path(),
    )
    .unwrap();
    let receipt: Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(receipt["runStatus"], "cancelled");
    assert!(receipt["decision"].is_null());
    let repeat = command(
        root.path(),
        &[
            "check",
            root.path().to_str().unwrap(),
            p.to_str().unwrap(),
            "--report-dir",
            dir.to_str().unwrap(),
            "--cancel",
        ],
    );
    assert_eq!(repeat.status.code(), Some(4));
    assert_eq!(
        serde_json::from_slice::<Value>(&repeat.stdout).unwrap(),
        serde_json::from_slice::<Value>(&out.stdout).unwrap(),
        "publication error must preserve the original cancelled diagnostic artifact"
    );
}

#[test]
fn partial_baseline_review_is_still_block_and_cannot_publish_review_decision() {
    let (root, s) = snapshot(b"malformed");
    let p = review_request(root.path(), &s);
    let out = command(
        root.path(),
        &["check", root.path().to_str().unwrap(), p.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(2));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["envelope"]["coverage"]["status"], "partial");
    assert_eq!(v["envelope"]["decision"], "BLOCK");
}
