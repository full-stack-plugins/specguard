use guardengine::integration::{EvidenceProfile, RunStatus, load_envelope_json};
use specguard::{integration::producer::ProducedRun, source::SourceSnapshot};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
#[test]
fn generated_golden_bytes_recompute_and_bind_actual_git_commit_objects() {
    for case in ["allow", "block", "partial", "error", "cancelled"] {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/engine-producer")
            .join(case);
        let envelope = load_envelope_json(
            &std::fs::read(root.join("envelope.json")).unwrap(),
            EvidenceProfile::EngineBacked,
        )
        .unwrap();
        let bytes = |name| std::fs::read(root.join(name)).ok();
        let output = ProducedRun {
            envelope,
            contract: bytes("contract.json"),
            facts: bytes("facts.json"),
            report: bytes("report.json"),
            domain: bytes("domain.json"),
        };
        output.verify().unwrap();
        let snapshot: SourceSnapshot =
            serde_json::from_slice(&std::fs::read(root.join("snapshot.json")).unwrap()).unwrap();
        assert_eq!(
            snapshot.digest,
            output.envelope.binding.source_snapshot_digest
        );
        assert_eq!(
            specguard::model::digest(&(&snapshot.inventory, &snapshot.binding, &snapshot.contents)),
            snapshot.digest
        );
        let provenance: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("provenance.json")).unwrap()).unwrap();
        assert_eq!(
            provenance["implementationCommit"],
            "082e8641ddbdfb1453c82ceeba26f12f45d22fa1"
        );
        assert_eq!(
            snapshot.contents["specs/a.md"],
            provenance["sourceBytes"].as_str().unwrap().as_bytes()
        );
        let repository = tempfile::tempdir().unwrap();
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(repository.path())
                .args([
                    "init",
                    "-q",
                    &format!(
                        "--object-format={}",
                        provenance["objectFormat"].as_str().unwrap()
                    )
                ])
                .status()
                .unwrap()
                .success()
        );
        let mut child = Command::new("git")
            .arg("-C")
            .arg(repository.path())
            .args(["hash-object", "-t", "commit", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(
                provenance["candidateCommitObject"]
                    .as_str()
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success());
        assert_eq!(
            String::from_utf8(result.stdout).unwrap().trim(),
            output.envelope.binding.candidate_oid
        );
        if matches!(case, "error" | "cancelled") {
            assert!(output.report.is_none());
            assert_ne!(output.envelope.run_status, RunStatus::Completed);
        } else {
            assert!(output.report.is_some());
        }
    }
}
