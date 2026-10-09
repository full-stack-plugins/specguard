mod common;
mod producer_support;
use common::*;
use guardengine::integration::RunStatus;
use producer_support::*;
use specguard::integration::{producer::prepare, runtime::CancellationToken};
use std::collections::BTreeSet;
const FINISH: &str = "2026-10-09T10:00:01Z";
#[test]
fn running_worker_cancels_after_real_source_progress_and_preserves_diagnostics() {
    let (root, _) = snapshot(document().as_bytes());
    std::fs::write(
        root.path().join("specs/b.md"),
        document().replace("R1", "R2").replace("A1", "A2"),
    )
    .unwrap();
    let snapshot = specguard::source::freeze(
        root.path(),
        &specguard::source::discover(root.path(), &common::policy()).unwrap(),
        binding(root.path()),
    )
    .unwrap();
    let prepared = prepare(
        root.path(),
        &snapshot,
        invocation(&snapshot),
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
    )
    .unwrap();
    let token = CancellationToken::new();
    let worker_token = token.clone();
    let (progress_send, progress_recv) = std::sync::mpsc::channel();
    let (resume_send, resume_recv) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        prepared
            .execute(FINISH, &worker_token, |source| {
                assert_eq!(source.reason, "parsed");
                progress_send.send(source.path.clone()).unwrap();
                resume_recv.recv().unwrap();
            })
            .unwrap()
    });
    assert_eq!(
        progress_recv
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap(),
        "specs/a.md"
    );
    token.cancel();
    resume_send.send(()).unwrap();
    let output = worker.join().unwrap();
    assert_eq!(output.envelope.run_status, RunStatus::Cancelled);
    assert!(output.report.is_none() && output.envelope.decision.is_none());
    let diagnostics: String = String::from_utf8(output.domain.clone().unwrap()).unwrap();
    assert!(diagnostics.contains("parsed") && diagnostics.contains("specs/a.md"));
    let artifact: serde_json::Value = serde_json::from_str(&diagnostics).unwrap();
    assert!(
        !artifact["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["path"] == "specs/b.md" && source["reason"] == "parsed")
    );
    assert!(progress_recv.try_recv().is_err());
    output.verify().unwrap();
}
#[test]
fn actual_worker_panic_is_bound_error_and_original_source_failure_is_retained() {
    let (root, snapshot) = snapshot(b"not-a-supported-document");
    let prepared = prepare(
        root.path(),
        &snapshot,
        invocation(&snapshot),
        &BTreeSet::from([key("R1")]),
        &producer_support::policy(),
    )
    .unwrap();
    let worker = std::thread::spawn(move || {
        prepared
            .execute(FINISH, &CancellationToken::new(), |_| {
                panic!("actual observer worker panic")
            })
            .unwrap()
    });
    let output = worker
        .join()
        .expect("panic must be recovered inside bound runtime");
    assert_eq!(output.envelope.run_status, RunStatus::Error);
    assert_eq!(output.envelope.diagnostics[0].code, "runtime.panic");
    assert!(output.report.is_none());
    let diagnostics = String::from_utf8(output.domain.clone().unwrap()).unwrap();
    assert!(diagnostics.contains("required YAML frontmatter"));
    assert!(diagnostics.contains("actual observer worker panic"));
    output.verify().unwrap();
}
