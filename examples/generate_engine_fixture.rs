//! Generates bytes by real Git capture, SG analysis, GE evaluation and verification.
#[path = "../tests/common/mod.rs"]
mod common;
#[path = "../tests/producer_support/mod.rs"]
mod producer_support;
use specguard::integration::producer::prepare;
use std::{collections::BTreeSet, path::Path};
fn main() {
    let destination = std::env::args().nth(1).expect("output directory required");
    let destination = Path::new(&destination);
    std::fs::create_dir_all(destination).unwrap();
    let impl_commit = common::git(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        &["rev-parse", "HEAD"],
    );
    let engine_commit = common::git(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../guardengine"),
        &["rev-parse", "HEAD"],
    );
    for (case, source) in [
        ("allow", common::document()),
        (
            "block",
            "---\nformat: markdown-explicit/v1\nnamespace: demo\n---\n## Requirement: R1\nRequired.\n",
        ),
        ("partial", "malformed"),
        ("cancelled", common::document()),
        ("error", common::document()),
    ] {
        let (root, snapshot) = common::snapshot(source.as_bytes());
        let mut invocation = producer_support::invocation(&snapshot);
        invocation.run_id = format!("fixture-{case}");
        let prepared = prepare(
            root.path(),
            &snapshot,
            invocation,
            &BTreeSet::from([common::key("R1")]),
            &producer_support::policy(),
        )
        .unwrap();
        let output = match case {
            "cancelled" => prepared.cancel("2026-10-09T10:00:01Z"),
            "error" => prepared.fail("2026-10-09T10:00:01Z"),
            _ => prepared.complete("2026-10-09T10:00:01Z"),
        }
        .unwrap();
        output.verify().unwrap();
        let directory = destination.join(case);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("envelope.json"),
            serde_json::to_vec_pretty(&output.envelope).unwrap(),
        )
        .unwrap();
        for (name, bytes) in [
            ("contract.json", output.contract),
            ("facts.json", output.facts),
            ("report.json", output.report),
            ("domain.json", output.domain),
        ] {
            if let Some(bytes) = bytes {
                std::fs::write(directory.join(name), bytes).unwrap();
            }
        }
        std::fs::write(
            directory.join("snapshot.json"),
            serde_json::to_vec_pretty(&snapshot).unwrap(),
        )
        .unwrap();
        let commit_object = common::git(
            root.path(),
            &["cat-file", "-p", &snapshot.binding.candidate_oid],
        );
        std::fs::write(directory.join("provenance.json"),serde_json::to_vec_pretty(&serde_json::json!({"profile":"local-fixture-not-authenticated","implementationCommit":impl_commit,"engineCommit":engine_commit,"objectFormat":snapshot.binding.object_format,"candidateOid":snapshot.binding.candidate_oid,"candidateCommitObject":format!("{commit_object}\n"),"sourcePath":"specs/a.md","sourceBytes":source,"generation":"cargo run --locked --example generate_engine_fixture -- fixtures/engine-producer"})).unwrap()).unwrap();
    }
}
