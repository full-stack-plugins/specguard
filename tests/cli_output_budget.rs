mod common;
mod producer_support;
use common::*;
use serde_json::json;
#[test]
fn small_duplicate_source_cannot_expand_unbounded_byte_array_cli_output() {
    let root = repo();
    let mut path = root.path().join("specs");
    for _ in 0..13 {
        path = path.join("a".repeat(230));
    }
    std::fs::create_dir_all(&path).unwrap();
    path = path.join("requirements.md");
    let text = format!(
        "---\nformat: markdown-explicit/v1\nnamespace: demo\n---\n{}",
        "## Requirement: R1\n".repeat(300)
    );
    std::fs::write(&path, &text).unwrap();
    let p = policy();
    let s = specguard::source::freeze(
        root.path(),
        &specguard::source::discover(root.path(), &p).unwrap(),
        binding(root.path()),
    )
    .unwrap();
    let request = json!({"apiVersion":"specguard.cli-check/v1alpha1","sourcePolicy":p,"binding":s.binding,"required":[key("R1")],"invocation":producer_support::invocation(&s),"mapping":producer_support::policy(),"finishedAt":"2026-10-09T10:00:01Z"});
    let bytes = serde_json::to_vec(&request).unwrap();
    std::fs::write(root.path().join("request.json"), &bytes).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_specguard"))
        .current_dir(root.path())
        .args(["check", ".", "request.json"])
        .output()
        .unwrap();
    eprintln!(
        "source={} request={} path={} stdout={} status={:?} stderr={}",
        text.len(),
        bytes.len(),
        path.to_str().unwrap().len(),
        out.stdout.len(),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    eprintln!(
        "runStatus={} decision={}",
        v["envelope"]["runStatus"], v["envelope"]["decision"]
    );
    assert_eq!(out.status.code(), Some(4));
    assert_eq!(v["envelope"]["runStatus"], "error");
    assert!(v["envelope"]["decision"].is_null());
    assert_eq!(v["envelope"]["diagnostics"][0]["code"], "cli.output_budget");
    assert_eq!(
        v["envelope"]["binding"]["candidateOid"],
        request["binding"]["candidateOid"]
    );
    let output = specguard::integration::producer::ProducedRun {
        envelope: serde_json::from_value(v["envelope"].clone()).unwrap(),
        contract: serde_json::from_value(v["contract"].clone()).unwrap(),
        facts: serde_json::from_value(v["facts"].clone()).unwrap(),
        report: serde_json::from_value(v["report"].clone()).unwrap(),
        domain: serde_json::from_value(v["domain"].clone()).unwrap(),
    };
    assert!(output.contract.is_none() && output.facts.is_none() && output.report.is_none());
    output.verify().unwrap();
    assert!(
        out.stdout.len() <= guardengine::integration::MAX_ARTIFACT_BYTES,
        "CLI bytes wrapper exceeds artifact budget"
    );
}
