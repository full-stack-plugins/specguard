#[path = "../tests/common/mod.rs"]
mod common;
#[path = "../tests/trace_support/mod.rs"]
mod trace_support;
fn main() {
    let destination = std::env::args().nth(1).expect("fresh output directory");
    let destination = std::path::Path::new(&destination);
    std::fs::create_dir_all(destination).unwrap();
    let (root, snapshot, policy) = trace_support::fixture(trace_support::ADR);
    let required = std::collections::BTreeSet::from([common::key("R1")]);
    let artifact = specguard::trace::scan(&snapshot, &required).unwrap();
    std::fs::write(
        destination.join("trace.json"),
        specguard::trace::export(&artifact).unwrap(),
    )
    .unwrap();
    for (name, value) in [
        (
            "snapshot.json",
            serde_json::to_vec_pretty(&snapshot).unwrap(),
        ),
        ("policy.json", serde_json::to_vec_pretty(&policy).unwrap()),
        (
            "binding.json",
            serde_json::to_vec_pretty(&snapshot.binding).unwrap(),
        ),
        (
            "required.json",
            serde_json::to_vec_pretty(&required).unwrap(),
        ),
    ] {
        std::fs::write(destination.join(name), value).unwrap();
    }
    for (path, bytes) in &snapshot.contents {
        let path = destination.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    let provenance = serde_json::json!({"profile":"local-fixture-not-authenticated","candidateOid":snapshot.binding.candidate_oid,"commitObject":common::git(root.path(),&["cat-file","-p",&snapshot.binding.candidate_oid]),"generator":"cargo run --locked --example generate_trace_fixture -- fixtures/traces"});
    std::fs::write(
        destination.join("provenance.json"),
        serde_json::to_vec_pretty(&provenance).unwrap(),
    )
    .unwrap();
}
