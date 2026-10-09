//! Actual frozen source, baseline diff and verified GE output; never production authority.
#[path = "../tests/common/mod.rs"]
mod common;
#[path = "../tests/producer_support/mod.rs"]
mod producer_support;
#[path = "../tests/review_support/mod.rs"]
mod review_support;
fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("fixture destination"));
    std::fs::create_dir_all(&out).unwrap();
    let text = common::document().replace("A user can sign in.", "A user can sign out.");
    let (root, snapshot) = common::snapshot(text.as_bytes());
    let request = review_support::review_request(root.path(), &snapshot);
    let bytes = std::fs::read(request).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let review = serde_json::from_value(v["review"].clone()).unwrap();
    let prepared = specguard::integration::producer::prepare_baseline_review(
        root.path(),
        &snapshot,
        serde_json::from_value(v["invocation"].clone()).unwrap(),
        &std::collections::BTreeSet::from([common::key("R1")]),
        &producer_support::policy(),
        &review,
    )
    .unwrap();
    let output = prepared.complete("2026-10-09T10:00:01Z").unwrap();
    output.verify().unwrap();
    assert_eq!(
        output.envelope.decision,
        Some(guardengine::Decision::RequireApproval)
    );
    std::fs::write(out.join("request.json"), bytes).unwrap();
    std::fs::write(out.join("source.md"), text).unwrap();
    std::fs::write(
        out.join("bundle.json"),
        serde_json::to_vec_pretty(&output).unwrap(),
    )
    .unwrap();
}
