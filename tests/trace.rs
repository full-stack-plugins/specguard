use serde_json::{Value, json};
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
        let f = Self(std::env::temp_dir().join(format!(
            "specguard-trace-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )));
        f.write("openspec/specs/auth/spec.md", "### Requirement: Login\nUsers MUST log in.\n#### Scenario: OK\n- **WHEN** valid\n- **THEN** succeeds\n");
        f.write("src/lib.rs", "pub fn login() -> bool { true }\n");
        f.write("manifest.json", &json!({"schema_version":"specguard.trace/v1","source":"openspec","execution_plan_digest":format!("sha256:{}", "a".repeat(64)),"mappings":[{"artifact":"openspec/specs/auth/spec.md","requirement_id":"Login","code":[{"path":"src/lib.rs","marker":"pub fn login"}],"tests":["login_test"]}]}).to_string());
        f
    }
    fn write(&self, path: &str, data: &str) {
        let p = self.0.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, data).unwrap();
    }
    fn run(&self, extra: &[&str], code: i32) -> Value {
        let out = Command::new(env!("CARGO_BIN_EXE_specguard"))
            .args(["trace", "check", "--project"])
            .arg(&self.0)
            .arg("--manifest")
            .arg(self.0.join("manifest.json"))
            .args(extra)
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(code),
            "{} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
    fn alter(&self, edit: impl FnOnce(&mut Value)) {
        let p = self.0.join("manifest.json");
        let mut v = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
        edit(&mut v);
        fs::write(p, v.to_string()).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn declaration_without_execution_never_passes() {
    let f = Fixture::new();
    assert_eq!(f.run(&[], 3)["status"], "INDETERMINATE");
}
#[test]
fn missing_requirement_mapping_blocks() {
    let f = Fixture::new();
    f.alter(|v| v["mappings"] = json!([]));
    assert_eq!(f.run(&[], 2)["status"], "FAIL");
}
#[test]
fn invalid_requirement_reference_blocks() {
    let f = Fixture::new();
    f.alter(|v| v["mappings"][0]["requirement_id"] = json!("Invented"));
    f.run(&[], 2);
}
#[test]
fn missing_code_marker_blocks_and_repair_rechecks() {
    let f = Fixture::new();
    f.write("src/lib.rs", "// empty implementation\n");
    f.run(&[], 2);
    f.write("src/lib.rs", "pub fn login() -> bool { true }\n");
    f.run(&[], 3);
}
#[test]
fn empty_test_obligation_blocks() {
    let f = Fixture::new();
    f.alter(|v| v["mappings"][0]["tests"] = json!([]));
    f.run(&[], 2);
}
#[test]
fn traversal_code_reference_blocks() {
    let f = Fixture::new();
    f.alter(|v| v["mappings"][0]["code"][0]["path"] = json!("../outside.rs"));
    f.run(&[], 2);
}

// 此处仅隔离验证消费者协议；真实runner端到端执行由统一集成测试覆盖。
struct Signed {
    f: Fixture,
    outside: PathBuf,
    public: PathBuf,
    evidence: PathBuf,
}
impl Signed {
    fn new(tests: &[&str]) -> Self {
        let f = Fixture::new();
        let outside = f.0.with_extension("evidence");
        fs::create_dir_all(&outside).unwrap();
        let public = outside.join("public.hex");
        let private = outside.join("private.pk8");
        guard_evidence::keygen(&private, &public).unwrap();
        let program = std::env::current_exe().unwrap();
        let artifact = outside.join("execution.log");
        fs::write(&artifact, "isolated consumer fixture").unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let payload = json!({"schema_version":"testguard.execution/v1","producer":"testguard","project":f.0.canonicalize().unwrap().to_str().unwrap(),"snapshot":guard_evidence::snapshot_project(&f.0).unwrap(),"plan_digest":format!("sha256:{}","a".repeat(64)),"runner":"cargo","executable":{"path":program,"sha256":guard_evidence::hash_file(&program).unwrap()},"argv":["test"],"started_at_unix_ms":now-1,"finished_at_unix_ms":now,"exit_code":0,"status":"PASS","required_tests":tests,"observed_tests":tests,"artifacts":[{"path":artifact,"sha256":guard_evidence::hash_file(&artifact).unwrap()}],"execution_verified":true});
        let envelope = guard_evidence::sign_receipt(&payload, &private).unwrap();
        let evidence = outside.join("receipt.json");
        fs::write(&evidence, envelope.to_string()).unwrap();
        Self {
            f,
            outside,
            public,
            evidence,
        }
    }
    fn run(&self, verify: bool, expected: i32) -> Value {
        let mut c = Command::new(env!("CARGO_BIN_EXE_specguard"));
        c.args([
            "trace",
            if verify { "verify" } else { "check" },
            "--project",
        ])
        .arg(&self.f.0)
        .arg("--manifest")
        .arg(self.f.0.join("manifest.json"))
        .arg("--evidence")
        .arg(&self.evidence)
        .arg("--public-key")
        .arg(&self.public);
        if verify {
            c.arg("--report").arg(self.outside.join("report.json"));
        }
        let out = c.output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(expected),
            "{} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
impl Drop for Signed {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.outside);
    }
}
#[test]
fn signed_current_linkage_and_report_recomputation() {
    let s = Signed::new(&["login_test"]);
    let report = s.run(false, 0);
    assert_eq!(report["semantic_validation"], "NOT_EVALUATED");
    fs::write(s.outside.join("report.json"), report.to_string()).unwrap();
    assert_eq!(s.run(true, 0), report);
    let mut fake = report;
    fake["semantic_validation"] = json!("PASS");
    fs::write(s.outside.join("report.json"), fake.to_string()).unwrap();
    s.run(true, 2);
}
#[test]
fn signed_other_test_does_not_cover_mapping() {
    Signed::new(&["other_test"]).run(false, 2);
}
#[test]
fn candidate_edit_invalidates_execution() {
    let s = Signed::new(&["login_test"]);
    s.run(false, 0);
    s.f.write("src/lib.rs", "pub fn login() -> bool { false }\n");
    s.run(false, 2);
}
#[test]
fn fake_report_and_tampered_execution_rejected() {
    let s = Signed::new(&["login_test"]);
    let mut e: Value = serde_json::from_slice(&fs::read(&s.evidence).unwrap()).unwrap();
    e["payload"]["status"] = json!("FAIL");
    fs::write(&s.evidence, e.to_string()).unwrap();
    s.run(false, 2);
}
#[test]
fn artifact_edit_invalidates_execution() {
    let s = Signed::new(&["login_test"]);
    fs::write(s.outside.join("execution.log"), "forged").unwrap();
    s.run(false, 2);
}

#[test]
fn trace_mcp_matches_cli_and_rejects_mutation() {
    use std::io::Write;
    use std::process::Stdio;
    let f = Fixture::new();
    let expected = f.run(&[], 3);
    let arguments = json!({"project":f.0,"manifest":f.0.join("manifest.json")});
    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"specguard_check_trace","arguments":arguments}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"specguard_check_trace","arguments":{"project":f.0,"manifest":f.0.join("manifest.json"),"execute":true}}}),
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
            (requests
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
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    let result: Value = serde_json::from_str(
        responses.iter().find(|r| r["id"] == 2).unwrap()["result"]["content"][0]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(result, expected);
    assert_eq!(
        responses.iter().find(|r| r["id"] == 3).unwrap()["result"]["isError"],
        true
    );
}
