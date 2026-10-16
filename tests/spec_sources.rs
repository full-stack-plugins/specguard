use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "specguard-sources-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn run(&self, expected: i32) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_specguard"))
            .args(["specs", "sources", "--project"])
            .arg(&self.0)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{} {}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
const OPEN: &str = "## ADDED Requirements\n### Requirement: EG-AUTH-001 Login\n#### Scenario: Login\n- **WHEN** credentials match\n- **THEN** authenticate\n";

#[test]
fn openspec_native_keys_are_capability_scoped_and_line_stable() {
    let f = Fixture::new();
    f.write("openspec/specs/auth/spec.md", OPEN);
    let first = f.run(0);
    assert_eq!(first["schema"], "specguard-sources/v1");
    assert_eq!(first["authority_status"], "single_source_observed");
    assert_eq!(first["approved_baseline_eligible"], false);
    assert_eq!(first["approval_status"], "not_evaluated");
    assert_eq!(first["requirements"][0]["source_state"], "baseline");
    let id = first["requirements"][0]["stable_id"].clone();
    f.write("openspec/specs/auth/spec.md", &format!("\n\n{OPEN}"));
    let next = f.run(0);
    assert_eq!(next["requirements"][0]["stable_id"], id);
    assert_ne!(
        next["requirements"][0]["artifact_sha256"],
        first["requirements"][0]["artifact_sha256"]
    );
    f.write("openspec/changes/login/specs/auth/spec.md", OPEN);
    f.write("openspec/specs/other/spec.md", OPEN);
    let next = f.run(0);
    let rows = next["requirements"].as_array().unwrap();
    assert_eq!(rows.iter().filter(|r| r["stable_id"] == id).count(), 2);
    assert_eq!(
        rows.iter()
            .filter(|r| r["source_state"] == "candidate")
            .count(),
        1
    );
    assert_eq!(rows.iter().filter(|r| r["stable_id"] != id).count(), 1);
    assert!(
        !next["requirements"]
            .to_string()
            .contains("credentials match")
    );
}

#[test]
fn speckit_feature_scope_preserves_repeated_native_fr_ids() {
    let f = Fixture::new();
    f.write(".specify/memory/constitution.md", "# Project rules\n");
    f.write("specs/001-login/spec.md", "- **FR-001**: Authenticate\n");
    f.write("specs/002-pay/spec.md", "- **FR-001**: Pay\n");
    let report = f.run(0);
    let rows = report["requirements"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["native_id"], "FR-001");
    assert_ne!(rows[0]["stable_id"], rows[1]["stable_id"]);
    assert_eq!(report["trust"], "local_unverified");
}

#[test]
fn two_sources_report_conflict_but_keep_both_independent_observations() {
    let f = Fixture::new();
    f.write("openspec/specs/auth/spec.md", OPEN);
    f.write(".specify/memory/constitution.md", "# Rules\n");
    f.write("specs/001-login/spec.md", "- **FR-001**: Authenticate\n");
    let report = f.run(3);
    assert_eq!(report["authority_status"], "conflict");
    assert_eq!(report["requirements"].as_array().unwrap().len(), 2);
    assert_eq!(report["selected_source"], Value::Null);
    assert_eq!(report["approved_baseline_eligible"], false);
    assert_eq!(report["sources"].as_array().unwrap().len(), 2);
}

#[test]
fn unknown_source_and_plans_only_do_not_initialize_or_choose() {
    let f = Fixture::new();
    f.write(
        "specs/custom.md",
        "# A specification\nFR-001: Unknown framework\n",
    );
    f.write(
        "docs/superpowers/plans/plan.md",
        "# Tasks\nREQ-001: Not a requirement source\n",
    );
    let report = f.run(3);
    assert_eq!(report["authority_status"], "unknown");
    assert!(report["requirements"].as_array().unwrap().is_empty());
    assert!(!f.0.join(".specify").exists());
    assert!(!f.0.join("openspec").exists());
    assert!(!f.0.join("docs/superpowers/specs").exists());
}

#[test]
fn superpowers_explicit_ids_are_read_only_and_freeform_remains_incomplete() {
    let f = Fixture::new();
    f.write("docs/superpowers/specs/login.md", "# Login\n## Requirements\n- **REQ-AUTH-001**: Authenticate\n```md\nREQ-FAKE: Example\n```\n> REQ-QUOTE: Example\n<!-- REQ-COMMENT: Example -->\n");
    f.write(
        "docs/superpowers/plans/login.md",
        "# Plan\nREQ-PLAN: Not a requirement\n",
    );
    let report = f.run(0);
    assert_eq!(report["requirements"].as_array().unwrap().len(), 1);
    assert_eq!(report["requirements"][0]["native_id"], "REQ-AUTH-001");
    assert_eq!(
        report["requirements"][0]["source_state"],
        "approval_unknown"
    );
    assert_eq!(report["sources"][0]["system"], "superpowers");
    assert_eq!(report["execution_status"], "not_evaluated");
    f.write(
        "docs/superpowers/specs/freeform.md",
        "# Spec\n## Requirements\nThe user can login.\n",
    );
    let report = f.run(3);
    assert_eq!(report["requirements"].as_array().unwrap().len(), 1);
    assert_eq!(report["sources"][0]["status"], "incomplete");
}

#[test]
fn superpowers_plans_do_not_conflict_with_openspec_facts() {
    let f = Fixture::new();
    f.write("openspec/specs/auth/spec.md", OPEN);
    f.write("docs/superpowers/plans/login.md", "# Implementation plan\n");
    assert_eq!(f.run(0)["authority_status"], "single_source_observed");
    f.write(
        "docs/superpowers/specs/login.md",
        "REQ-AUTH: Authenticate\n",
    );
    assert_eq!(f.run(3)["authority_status"], "conflict");
}

#[test]
fn duplicate_explicit_ids_remain_structural_findings_not_approval() {
    let f = Fixture::new();
    f.write(
        "docs/superpowers/specs/login.md",
        "REQ-AUTH: One\n\nREQ-AUTH: Two\n",
    );
    let report = f.run(2);
    assert_eq!(report["sources"][0]["status"], "invalid");
    assert_eq!(report["findings"][0]["code"], "duplicate_requirement");
    assert_eq!(report["approved_baseline_eligible"], false);
}

#[test]
fn templates_archives_and_removed_requirements_preserve_native_state() {
    let f = Fixture::new();
    f.write("openspec/templates/spec.md", OPEN);
    f.write("openspec/changes/archive/old/specs/auth/spec.md", OPEN);
    f.write(
        "openspec/changes/delete/specs/auth/spec.md",
        "## REMOVED Requirements\n### Requirement: Old login\n",
    );
    let report = f.run(0);
    assert_eq!(report["requirements"].as_array().unwrap().len(), 1);
    assert_eq!(report["requirements"][0]["operation"], "removed");
    assert_eq!(report["requirements"][0]["source_state"], "candidate");
}

#[test]
fn malformed_source_does_not_erase_other_source_observations() {
    let f = Fixture::new();
    f.write("openspec", "not a directory");
    f.write(".specify/memory/constitution.md", "# Rules\n");
    f.write("specs/001-login/spec.md", "FR-001: Login\n");
    let report = f.run(3);
    assert_eq!(report["requirements"].as_array().unwrap().len(), 1);
    assert!(!report["diagnostics"].as_array().unwrap().is_empty());
    assert_eq!(report["selected_source"], Value::Null);
}

#[test]
fn invalid_utf8_and_oversized_native_sources_are_incomplete() {
    let f = Fixture::new();
    f.write("docs/superpowers/specs/bad.md", "placeholder");
    fs::write(f.0.join("docs/superpowers/specs/bad.md"), [0xff]).unwrap();
    f.run(3);
    f.write(
        "docs/superpowers/specs/bad.md",
        &"x".repeat(1024 * 1024 + 1),
    );
    f.run(3);
}

#[test]
fn id_components_are_collision_free_case_preserving_and_not_content_digests() {
    let f = Fixture::new();
    f.write(
        "openspec/specs/auth/spec.md",
        &["A/B", "A%2FB", "Login", "login", "登录"]
            .map(|name| OPEN.replace("EG-AUTH-001 Login", name))
            .join("\n"),
    );
    let report = f.run(0);
    let rows = report["requirements"].as_array().unwrap();
    assert_eq!(rows.len(), 5);
    let ids: std::collections::BTreeSet<_> = rows
        .iter()
        .map(|row| row["stable_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 5);
    assert!(
        rows.iter()
            .any(|row| row["stable_id"].as_str().unwrap().ends_with("A%2FB"))
    );
    assert!(
        rows.iter()
            .any(|row| row["stable_id"].as_str().unwrap().ends_with("A%252FB"))
    );
    for row in rows {
        assert!(
            row["stable_id"]
                .as_str()
                .unwrap()
                .starts_with("urn:specguard:requirement:v1:")
        );
    }
}

#[cfg(windows)]
#[test]
fn linked_superpowers_ancestor_does_not_read_external_requirements() {
    let f = Fixture::new();
    let outside = Fixture::new();
    outside.write(
        "superpowers/specs/login.md",
        "REQ-SECRET: Never read this\n",
    );
    let link = f.0.join("docs");
    let out = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&outside.0)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report = f.run(3);
    fs::remove_dir(&link).unwrap();
    assert!(report["requirements"].as_array().unwrap().is_empty());
    assert!(!report.to_string().contains("REQ-SECRET"));
}

#[test]
fn superpowers_file_and_directory_budgets_remain_incomplete() {
    let f = Fixture::new();
    f.write("docs/superpowers/specs/auth.md", "REQ-AUTH: Login\n");
    for index in 0..513 {
        f.write(
            &format!("docs/superpowers/specs/extra/{index}.md"),
            "# Extra\n",
        );
    }
    f.run(3);
}

#[test]
fn empty_superpowers_id_prefix_is_not_a_requirement() {
    let f = Fixture::new();
    f.write(
        "docs/superpowers/specs/login.md",
        "REQ-: Not an identifier\n",
    );
    let report = f.run(3);
    assert!(report["requirements"].as_array().unwrap().is_empty());
}

#[test]
fn mcp_sources_matches_cli_and_rejects_mutation_or_source_override() {
    use std::{io::Write, process::Stdio};
    let f = Fixture::new();
    f.write("docs/superpowers/specs/login.md", "REQ-AUTH: Login\n");
    let expected = f.run(0);
    let messages = [
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"specguard_spec_sources","arguments":{"project":f.0}}}),
        serde_json::json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"specguard_spec_sources","arguments":{"project":f.0,"source":"openspec"}}}),
    ];
    let mut child = Command::new(env!("CARGO_BIN_EXE_specguard"))
        .args(["serve", "--mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            (messages
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n")
                .as_bytes(),
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let responses: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        responses.iter().find(|r| r["id"] == 2).unwrap()["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "specguard_spec_sources")
    );
    let projected: Value = serde_json::from_str(
        responses.iter().find(|r| r["id"] == 3).unwrap()["result"]["content"][0]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(projected, expected);
    assert_eq!(
        responses.iter().find(|r| r["id"] == 4).unwrap()["result"]["isError"],
        true
    );
}
