mod common;
mod producer_support;
use common::*;
use serde_json::{Value, json};
use specguard::{
    integration::runtime::CancellationToken,
    mcp::{Protocol, ReadOnlySession, SessionConfig},
};
use std::collections::BTreeSet;
fn start() -> (tempfile::TempDir, Protocol) {
    let (root, snapshot) = snapshot(document().as_bytes());
    let config = SessionConfig {
        request_key: "fixture-controller-key".into(),
        source_policy: policy(),
        binding: snapshot.binding.clone(),
        required: BTreeSet::from([key("R1")]),
        invocation: producer_support::invocation(&snapshot),
        mapping: producer_support::policy(),
        finished_at: "2026-10-09T10:00:01Z".into(),
    };
    let mut protocol = Protocol::new(ReadOnlySession::freeze(root.path(), config).unwrap());
    let response=protocol.handle(&serde_json::to_vec(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}})).unwrap(),&CancellationToken::new()).unwrap();
    assert_eq!(response["result"]["protocolVersion"], "2025-11-25");
    protocol.handle(
        br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        &CancellationToken::new(),
    );
    (root, protocol)
}
fn call(p: &mut Protocol, name: &str, args: Value, cancel: &CancellationToken) -> Value {
    p.handle(&serde_json::to_vec(&json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":name,"arguments":args}})).unwrap(),cancel).unwrap()
}
#[test]
fn real_discovery_check_evidence_are_readonly_and_authorized() {
    let (_root, mut p) = start();
    let token = CancellationToken::new();
    let bad = call(
        &mut p,
        "specguard_check",
        json!({"requestKey":"wrong"}),
        &token,
    );
    assert_eq!(bad["result"]["isError"], true);
    let discovery = call(
        &mut p,
        "specguard_discover",
        json!({"requestKey":"fixture-controller-key"}),
        &token,
    );
    assert_eq!(discovery["result"]["isError"], false);
    let checked = call(
        &mut p,
        "specguard_check",
        json!({"requestKey":"fixture-controller-key"}),
        &token,
    );
    assert_eq!(
        checked["result"]["structuredContent"]["envelope"]["decision"],
        "ALLOW"
    );
    let uri =
        checked["result"]["structuredContent"]["envelope"]["artifacts"]["report"]["uri"].clone();
    let evidence = call(
        &mut p,
        "specguard_evidence",
        json!({"requestKey":"fixture-controller-key","uri":uri}),
        &token,
    );
    assert_eq!(evidence["result"]["isError"], false);
    assert_eq!(
        evidence["result"]["structuredContent"]["executionAuthorized"],
        false
    );
    for name in ["commit", "approve", "merge", "unknown"] {
        assert!(
            call(
                &mut p,
                name,
                json!({"requestKey":"fixture-controller-key"}),
                &token
            )["error"]
                .is_object()
        );
    }
}
#[test]
fn cancellation_has_null_decision_and_cannot_return_previous_allow() {
    let (_root, mut p) = start();
    let token = CancellationToken::new();
    let first = call(
        &mut p,
        "specguard_check",
        json!({"requestKey":"fixture-controller-key"}),
        &token,
    );
    token.cancel();
    let cancelled = call(
        &mut p,
        "specguard_check",
        json!({"requestKey":"fixture-controller-key"}),
        &token,
    );
    assert_eq!(
        cancelled["result"]["structuredContent"]["envelope"]["runStatus"],
        "cancelled"
    );
    assert!(cancelled["result"]["structuredContent"]["envelope"]["decision"].is_null());
    assert_ne!(
        first["result"]["structuredContent"]["envelope"]["runId"],
        cancelled["result"]["structuredContent"]["envelope"]["runId"]
    );
}

#[test]
fn malformed_arguments_and_unknown_evidence_fail_closed() {
    let (_root, mut p) = start();
    let token = CancellationToken::new();
    for raw in [
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"specguard_check","arguments":{"requestKey":"fixture-controller-key","path":"/etc/passwd"}}}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"specguard_check","arguments":{"requestKey":"wrong","requestKey":"fixture-controller-key"}}}"#,
    ] {
        assert!(p.handle(raw.as_bytes(), &token).unwrap()["error"].is_object());
    }
    let response = call(
        &mut p,
        "specguard_evidence",
        json!({"requestKey":"fixture-controller-key","uri":"file:///etc/passwd"}),
        &token,
    );
    assert_eq!(response["result"]["isError"], true);
}

#[test]
fn actual_stdio_launch_pins_configuration_and_recovers_after_oversize_frame() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let (root, snapshot) = snapshot(document().as_bytes());
    let config = json!({"requestKey":"fixture-controller-key","sourcePolicy":policy(),"binding":snapshot.binding,"required":[key("R1")],"invocation":producer_support::invocation(&snapshot),"mapping":producer_support::policy(),"finishedAt":"2026-10-09T10:00:01Z"});
    let bytes = serde_json::to_vec(&config).unwrap();
    let config_dir = tempfile::tempdir().unwrap();
    let path = config_dir.path().join("config.json");
    std::fs::write(&path, &bytes).unwrap();
    let bad = Command::new(env!("CARGO_BIN_EXE_specguard-mcp"))
        .args(["--local-readonly-v1"])
        .arg(root.path())
        .arg(&path)
        .arg("wrong")
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(4));
    assert!(bad.stdout.is_empty());
    let mut child = Command::new(env!("CARGO_BIN_EXE_specguard-mcp"))
        .arg("--local-readonly-v1")
        .arg(root.path())
        .arg(&path)
        .arg(specguard::model::digest(&bytes))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    writeln!(input, "{}", "x".repeat(9000)).unwrap();
    writeln!(input,"{}",json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}})).unwrap();
    writeln!(
        input,
        "{}",
        json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    writeln!(
        input,
        "{}",
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})
    )
    .unwrap();
    writeln!(input,"{}",json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"specguard_check","arguments":{"requestKey":"fixture-controller-key"}}})).unwrap();
    drop(input);
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    let responses: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(responses.len(), 4);
    assert!(responses[0]["error"].is_object());
    assert_eq!(responses[2]["result"]["tools"].as_array().unwrap().len(), 3);
    assert_eq!(
        responses[3]["result"]["structuredContent"]["envelope"]["decision"],
        "ALLOW"
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let sdk = Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/mcp_client.py"
        ))
        .arg(env!("CARGO_BIN_EXE_specguard-mcp"))
        .arg(root.path())
        .arg(&path)
        .arg(specguard::model::digest(&bytes))
        .output()
        .unwrap();
    assert!(
        sdk.status.success(),
        "{}",
        String::from_utf8_lossy(&sdk.stderr)
    );
    assert!(String::from_utf8_lossy(&sdk.stdout).contains("mcp-python/1.29.0"));
}

#[test]
fn history_survives_cancel_and_source_recheck_never_reuses_old_allow() {
    let (root, mut p) = start();
    let token = CancellationToken::new();
    let first = call(
        &mut p,
        "specguard_check",
        json!({"requestKey":"fixture-controller-key"}),
        &token,
    );
    let uri =
        first["result"]["structuredContent"]["envelope"]["artifacts"]["report"]["uri"].clone();
    let before = call(
        &mut p,
        "specguard_evidence",
        json!({"requestKey":"fixture-controller-key","uri":uri}),
        &token,
    );
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    call(
        &mut p,
        "specguard_check",
        json!({"requestKey":"fixture-controller-key"}),
        &cancelled,
    );
    let after = call(
        &mut p,
        "specguard_evidence",
        json!({"requestKey":"fixture-controller-key","uri":uri}),
        &token,
    );
    assert_eq!(before, after);
    std::fs::remove_dir_all(root.path().join("specs")).unwrap();
    let fresh = call(
        &mut p,
        "specguard_check",
        json!({"requestKey":"fixture-controller-key"}),
        &token,
    );
    assert_ne!(
        fresh["result"]["structuredContent"]["envelope"]["decision"],
        "ALLOW"
    );
}
