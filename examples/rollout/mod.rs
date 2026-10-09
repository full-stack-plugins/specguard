//! Actual local CLI/MCP composition. This is demonstration-controller logic, not a grant service.
use crate::{common, producer_support};
use serde_json::{Value, json};
pub fn exercise(cli: &str, mcp: &str) -> Value {
    let (root, _) = common::snapshot(common::document().as_bytes());
    common::git(root.path(), &["add", "specs"]);
    common::git(
        root.path(),
        &[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "source",
        ],
    );
    let inventory = specguard::source::discover(root.path(), &common::policy()).unwrap();
    let snapshot =
        specguard::source::freeze(root.path(), &inventory, common::binding(root.path())).unwrap();
    let config = json!({"requestKey":"rollout-controller-key","sourcePolicy":common::policy(),"binding":snapshot.binding,"required":[common::key("R1")],"invocation":producer_support::invocation(&snapshot),"mapping":producer_support::policy(),"finishedAt":"2026-10-09T10:00:01Z"});
    let config_dir = tempfile::tempdir().unwrap();
    let path = config_dir.path().join("controller.json");
    std::fs::write(&path, serde_json::to_vec(&config).unwrap()).unwrap();
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/rollout/exercise.py"
        ))
        .arg(cli)
        .arg(mcp)
        .arg(root.path())
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut result: Value = serde_json::from_slice(&output.stdout).unwrap();
    let bundles = result.as_object_mut().unwrap().remove("bundles").unwrap();
    for bundle in bundles.as_array().unwrap() {
        let output = specguard::integration::producer::ProducedRun {
            envelope: serde_json::from_value(bundle["envelope"].clone()).unwrap(),
            contract: serde_json::from_value(bundle["contract"].clone()).unwrap(),
            facts: serde_json::from_value(bundle["facts"].clone()).unwrap(),
            report: serde_json::from_value(bundle["report"].clone()).unwrap(),
            domain: serde_json::from_value(bundle["domain"].clone()).unwrap(),
        };
        output.verify().unwrap();
    }
    result
}
