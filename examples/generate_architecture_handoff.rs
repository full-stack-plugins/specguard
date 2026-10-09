#[path = "../tests/common/mod.rs"]
mod common;
#[path = "../tests/trace_support/mod.rs"]
mod trace_support;
use specguard::{architecture, baseline::decode_baseline, integration::approval::*, model::*};
fn main() {
    let destination = std::env::args().nth(1).expect("fresh output directory");
    let destination = std::path::Path::new(&destination);
    std::fs::create_dir_all(destination).unwrap();
    let baseline = decode_baseline(
        serde_json::from_str(include_str!("../fixtures/handoffs/baseline.json")).unwrap(),
    )
    .unwrap();
    let approved = authenticate(
        &baseline,
        &common::fixture_approval(&baseline),
        Profile::Fixture,
        50,
    )
    .unwrap();
    let (root, snapshot, _) = trace_support::fixture(trace_support::ADR);
    let h = architecture::export(&snapshot, &approved, &baseline.scope).unwrap();
    let bytes = architecture::encode(&h).unwrap();
    let provenance = serde_json::json!({
        "profile":"fixture-only-not-authenticated", "generatorCommit":common::git(std::path::Path::new(env!("CARGO_MANIFEST_DIR")),&["rev-parse","HEAD"]),
        "generator":"cargo run --locked --example generate_architecture_handoff -- fixtures/handoffs/architecture",
        "artifactDigest":architecture::artifact_digest(&bytes), "baselineDigest":digest(&baseline),
        "candidateOid":snapshot.binding.candidate_oid, "commitObject":common::git(root.path(),&["cat-file","-p",&snapshot.binding.candidate_oid]),
        "sourceProfile":"actual-temp-git-with-frozen-dirty-source-bytes", "authentication":"existing explicit fixture ApprovalValidationPort; production unavailable"
    });
    std::fs::write(destination.join("handoff.json"), bytes).unwrap();
    std::fs::write(
        destination.join("provenance.json"),
        serde_json::to_vec_pretty(&provenance).unwrap(),
    )
    .unwrap();
}
