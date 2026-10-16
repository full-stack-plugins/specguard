use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "specguard-native-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn write(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn run(&self, action: &str, source: &str, expected: i32) -> Value {
        let mut command = Command::new(env!("CARGO_BIN_EXE_specguard"));
        command
            .args(["specs", action, "--project"])
            .arg(&self.0)
            .args(["--source", source]);
        if action == "verify" {
            command.arg("--baseline").arg(self.0.join("baseline.json"));
        }
        let out = command.output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(expected),
            "{} {}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
        if expected == 4 {
            json!({"error":String::from_utf8_lossy(&out.stderr)})
        } else {
            serde_json::from_slice(&out.stdout).unwrap()
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

const VALID: &str = "# Auth\n### Requirement: Login\nUsers MUST log in.\n#### Scenario: Valid credentials\n- **WHEN** credentials match\n- **THEN** session opens\n";

#[test]
fn openspec_check_repair_verify_rejects_drift_and_is_repeatable() {
    let f = Fixture::new();
    f.write(
        "openspec/specs/auth/spec.md",
        "### Requirement: Login\nUsers MUST log in.\n",
    );
    let failed = f.run("check", "openspec", 2);
    assert_eq!(failed["findings"][0]["code"], "scenario_missing");
    f.write("openspec/specs/auth/spec.md", VALID);
    let clean = f.run("check", "openspec", 0);
    assert_eq!(clean["approval_status"], "not_evaluated");
    assert_eq!(clean["execution_status"], "not_evaluated");
    f.write("baseline.json", &serde_json::to_string(&clean).unwrap());
    let verified = f.run("verify", "openspec", 0);
    assert_eq!(verified["baseline_status"], "unchanged");
    assert_eq!(f.run("verify", "openspec", 0), verified);
    f.write(
        "openspec/specs/auth/spec.md",
        &VALID.replace("session opens", "session stays closed"),
    );
    assert!(
        f.run("verify", "openspec", 2)["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["code"] == "baseline_drift")
    );
    f.write("openspec/specs/auth/spec.md", VALID);
    f.write(
        "openspec/changes/login/tasks.md",
        "- [x] Pretend completed\n",
    );
    assert_eq!(f.run("verify", "openspec", 2)["baseline_status"], "drifted");
}

#[test]
fn examples_duplicate_names_and_partial_analysis_never_allow() {
    let f = Fixture::new();
    f.run("check", "openspec", 3);
    f.write(
        "openspec/specs/auth/spec.md",
        &format!("```md\n{VALID}```\n"),
    );
    f.run("check", "openspec", 3);
    f.write("openspec/specs/auth/spec.md", &format!("{VALID}\n{VALID}"));
    assert!(
        f.run("check", "openspec", 2)["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["code"] == "duplicate_requirement")
    );
    f.write(
        "openspec/specs/auth/spec.md",
        &VALID.replace("- **THEN** session opens", "- **WHEN** credentials match"),
    );
    assert!(
        f.run("check", "openspec", 2)["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["code"] == "scenario_incomplete")
    );
    f.write("openspec/specs/auth/spec.md", VALID);
    f.write(
        "baseline.json",
        "{\"decision\":\"ALLOW\",\"approval_status\":\"approved\"}",
    );
    f.run("verify", "openspec", 4);
}

#[test]
fn speckit_native_ids_and_artifact_drift_are_checked_without_approving_tasks() {
    let f = Fixture::new();
    f.write(".specify/memory/constitution.md", "# Constitution\n");
    f.write(
        "specs/001-auth/spec.md",
        "# Auth\n- **FR-001**: System MUST authenticate.\n",
    );
    f.write("specs/001-auth/plan.md", "# Design\n");
    f.write("specs/001-auth/tasks.md", "- [x] T001 Implement auth\n");
    let initial = f.run("check", "speckit", 0);
    assert_eq!(initial["requirements"].as_array().unwrap().len(), 1);
    assert_eq!(initial["execution_status"], "not_evaluated");
    f.write("baseline.json", &serde_json::to_string(&initial).unwrap());
    f.write("specs/001-auth/plan.md", "# Changed design\n");
    f.run("verify", "speckit", 2);
    f.write(
        "specs/001-auth/spec.md",
        "- **FR-001**: First\n- **FR-001**: Second\n",
    );
    f.run("check", "speckit", 2);
}

#[test]
fn malformed_unicode_and_oversized_sources_report_incomplete() {
    let f = Fixture::new();
    f.write("openspec/specs/auth/spec.md", VALID);
    fs::write(f.0.join("openspec/specs/auth/spec.md"), [0xff]).unwrap();
    f.run("check", "openspec", 3);
    f.write("openspec/specs/auth/spec.md", &"x".repeat(1024 * 1024 + 1));
    f.run("check", "openspec", 3);
}

#[test]
fn native_mcp_matches_cli_and_rejects_mutation_arguments() {
    let f = Fixture::new();
    f.write("openspec/specs/auth/spec.md", VALID);
    let expected = f.run("check", "openspec", 0);
    f.write("baseline.json", &expected.to_string());
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"specguard_check_specs","arguments":{"project":f.0,"source":"openspec"}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"specguard_verify_specs","arguments":{"project":f.0,"source":"openspec","baseline":f.0.join("baseline.json")}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"specguard_check_specs","arguments":{"project":f.0,"source":"openspec","execute":true}}}),
    ];
    let mut child = Command::new(env!("CARGO_BIN_EXE_specguard"))
        .args(["serve", "--mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let input = messages
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let responses: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let tools = responses.iter().find(|v| v["id"] == 2).unwrap()["result"]["tools"]
        .as_array()
        .unwrap();
    assert!(tools.iter().any(|v| v["name"] == "specguard_check_specs"));
    let parse = |id| -> Value {
        serde_json::from_str(
            responses.iter().find(|v| v["id"] == id).unwrap()["result"]["content"][0]["text"]
                .as_str()
                .unwrap(),
        )
        .unwrap()
    };
    assert_eq!(parse(3), expected);
    assert_eq!(parse(4), f.run("verify", "openspec", 0));
    assert_eq!(
        responses.iter().find(|v| v["id"] == 5).unwrap()["result"]["isError"],
        true
    );
    assert_eq!(
        fs::read_to_string(f.0.join("openspec/specs/auth/spec.md")).unwrap(),
        VALID
    );
    assert_eq!(
        fs::read_to_string(f.0.join("baseline.json")).unwrap(),
        expected.to_string()
    );
}

#[test]
fn removed_delta_does_not_require_a_new_scenario() {
    let f = Fixture::new();
    f.write("openspec/changes/auth/specs/login/spec.md", "## REMOVED Requirements\n### Requirement: Old login\n**Reason**: Superseded\n**Migration**: Use SSO\n");
    assert_eq!(
        f.run("check", "openspec", 0)["requirements"][0]["operation"],
        "removed"
    );
}

#[test]
fn templates_do_not_count_as_native_specifications() {
    let f = Fixture::new();
    f.write("openspec/templates/spec.md", VALID);
    f.run("check", "openspec", 3);
    f.write("openspec/specs/auth/spec.md", VALID);
    f.write(
        "openspec/templates/spec.md",
        "# Template without obligations\n",
    );
    let report = f.run("check", "openspec", 0);
    assert_eq!(report["requirements"].as_array().unwrap().len(), 1);
}

#[cfg(windows)]
#[test]
fn junction_source_is_incomplete_without_reading_target() {
    let f = Fixture::new();
    f.write("outside/spec.md", VALID);
    fs::create_dir(f.0.join("openspec")).unwrap();
    let link = f.0.join("openspec/specs");
    let result = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link.to_string_lossy().replace('/', "\\"))
        .arg(f.0.join("outside").to_string_lossy().replace('/', "\\"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let report = f.run("check", "openspec", 3);
    fs::remove_dir(link).unwrap();
    assert!(report["sources"].as_array().unwrap().is_empty());
    assert!(f.0.join("outside/spec.md").is_file());
}

#[test]
fn inventory_budget_and_missing_speckit_marker_are_incomplete() {
    let f = Fixture::new();
    f.write("specs/001-auth/spec.md", "- **FR-001**: Authenticate\n");
    f.run("check", "speckit", 3);
    f.write("openspec/specs/auth/spec.md", VALID);
    for index in 0..513 {
        f.write(&format!("openspec/extra/{index}.md"), "extra");
    }
    f.run("check", "openspec", 3);
}

#[test]
fn a_file_named_specify_is_not_framework_initialization() {
    let f = Fixture::new();
    f.write(".specify", "not a framework directory");
    f.write("specs/001-auth/spec.md", "- **FR-001**: Authenticate\n");
    f.run("check", "speckit", 3);
}

#[test]
fn baseline_is_project_source_bound_and_failed_report_is_rejected() {
    let f = Fixture::new();
    f.write("openspec/specs/auth/spec.md", VALID);
    let initial = f.run("check", "openspec", 0);
    for (key, bad) in [
        ("source", "speckit"),
        ("project", "/other"),
        ("schema", "unknown"),
        ("decision", "BLOCK"),
    ] {
        let mut invalid = initial.clone();
        invalid[key] = json!(bad);
        f.write("baseline.json", &invalid.to_string());
        f.run("verify", "openspec", 4);
    }
}

#[test]
fn empty_directories_count_toward_global_traversal_budget() {
    let f = Fixture::new();
    f.write("openspec/specs/auth/spec.md", VALID);
    for group in 0..2 {
        for index in 0..260 {
            fs::create_dir_all(f.0.join(format!("openspec/extra-{group}/{index}"))).unwrap();
        }
    }
    f.run("check", "openspec", 3);
}
