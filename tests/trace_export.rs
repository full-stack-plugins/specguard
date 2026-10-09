mod common;
use common::*;
use specguard::{source::*, trace::*};
use std::collections::BTreeSet;
mod trace_support;
use trace_support::{ADR, fixture};
#[test]
fn actual_files_export_stable_typed_targets_and_located_relations() {
    let (_, snapshot, _) = fixture(ADR);
    let artifact = scan(&snapshot, &BTreeSet::from([key("R1")])).unwrap();
    assert!(artifact.graph.complete());
    assert!(artifact.findings.is_empty());
    assert_eq!(artifact.graph.targets.nodes.len(), 2);
    assert_eq!(artifact.graph.targets.nodes[0].source.path, "adr.md");
    assert_eq!(artifact.graph.targets.nodes[0].source.line, 5);
    assert_eq!(artifact.graph.graph.parsed.edges.len(), 2);
    let bytes = export(&artifact).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["apiVersion"],
        "specguard.trace/v1alpha1"
    );
    let mut permuted = snapshot.clone();
    permuted.inventory.entries.reverse();
    permuted.inventory.sources.reverse();
    permuted.digest =
        specguard::model::digest(&(&permuted.inventory, &permuted.binding, &permuted.contents));
    let reordered = scan(&permuted, &BTreeSet::from([key("R1")])).unwrap();
    assert_eq!(artifact.graph.targets, reordered.graph.targets);
}
#[test]
fn duplicate_targets_keep_both_locations_and_fenced_headings_are_not_targets() {
    let (_, snapshot, _) = fixture(&format!(
        "{ADR}\n```md\n## ADR: invisible\n```\n## ADR: ADR1\nDuplicate\n"
    ));
    let artifact = scan(&snapshot, &BTreeSet::from([key("R1")])).unwrap();
    assert_eq!(artifact.graph.targets.nodes.len(), 3); // two ADRs plus one task
    let finding = artifact
        .findings
        .iter()
        .find(|f| f.kind == specguard::rules::FindingKind::Duplicate)
        .unwrap();
    assert_eq!(finding.sources.len(), 2);
}
#[test]
fn malformed_target_never_turns_unknown_scope_into_definite_missing() {
    let (_, snapshot, _) =
        fixture("---\nformat: markdown-adr/v2\nnamespace: demo\n---\n## ADR: ADR1\n");
    let artifact = scan(&snapshot, &BTreeSet::from([key("R1")])).unwrap();
    assert!(!artifact.graph.complete());
    assert!(
        artifact
            .findings
            .iter()
            .all(|f| f.kind != specguard::rules::FindingKind::BrokenReference)
    );
}
#[test]
fn real_cli_check_and_export_read_sources_without_mutation() {
    let (root, snapshot, p) = fixture(ADR);
    for (name, value) in [
        ("policy.json", serde_json::to_vec(&p).unwrap()),
        (
            "binding.json",
            serde_json::to_vec(&snapshot.binding).unwrap(),
        ),
        (
            "required.json",
            serde_json::to_vec(&vec![key("R1")]).unwrap(),
        ),
    ] {
        std::fs::write(root.path().join(name), value).unwrap();
    }
    let before = std::fs::read(root.path().join("adr.md")).unwrap();
    for command in ["trace-check", "trace-export"] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_specguard"))
            .arg(command)
            .arg(root.path())
            .arg(root.path().join("policy.json"))
            .arg(root.path().join("binding.json"))
            .arg(root.path().join("required.json"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["apiVersion"],
            "specguard.trace/v1alpha1"
        );
    }
    assert_eq!(before, std::fs::read(root.path().join("adr.md")).unwrap());
}

#[test]
fn fenced_trace_directive_in_list_is_data_not_a_relation() {
    let (root, _, p) = fixture(ADR);
    std::fs::write(
        root.path().join("specs/a.md"),
        format!(
            "{}\n- ```\n  traces_to_adr: demo:nonexistent\n  ```\n",
            document()
        ),
    )
    .unwrap();
    let snapshot = freeze(
        root.path(),
        &discover(root.path(), &p).unwrap(),
        binding(root.path()),
    )
    .unwrap();
    let artifact = scan(&snapshot, &BTreeSet::from([key("R1")])).unwrap();
    assert!(artifact.graph.graph.parsed.edges.is_empty());
    assert!(artifact.findings.is_empty());
}
#[test]
fn export_refuses_empty_required_scope_even_when_findings_are_recomputed() {
    let (_, snapshot, _) = fixture(ADR);
    let mut artifact = scan(&snapshot, &BTreeSet::from([key("R1")])).unwrap();
    artifact.required.clear();
    artifact.findings = artifact.graph.validate(&artifact.required);
    assert!(export(&artifact).is_err());
}

#[test]
fn unsupported_trace_relation_does_not_become_zero_edge_success() {
    let (root, _, p) = fixture(ADR);
    std::fs::write(
        root.path().join("specs/a.md"),
        format!("{}\n- traces_to_unknown: demo:T1\n", document()),
    )
    .unwrap();
    let snapshot = freeze(
        root.path(),
        &discover(root.path(), &p).unwrap(),
        binding(root.path()),
    )
    .unwrap();
    let artifact = scan(&snapshot, &BTreeSet::from([key("R1")])).unwrap();
    assert!(!artifact.graph.complete());
}

#[test]
fn real_cli_returns_block_for_wrong_kind_or_unknown_target_scope() {
    for text in [
        ADR.replace("ADR1", "different"),
        ADR.replace("markdown-adr/v1", "markdown-adr/v2"),
    ] {
        let (root, snapshot, p) = fixture(&text);
        for (name, value) in [
            ("policy.json", serde_json::to_vec(&p).unwrap()),
            (
                "binding.json",
                serde_json::to_vec(&snapshot.binding).unwrap(),
            ),
            (
                "required.json",
                serde_json::to_vec(&vec![key("R1")]).unwrap(),
            ),
        ] {
            std::fs::write(root.path().join(name), value).unwrap();
        }
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_specguard"))
            .arg("trace-check")
            .arg(root.path())
            .arg(root.path().join("policy.json"))
            .arg(root.path().join("binding.json"))
            .arg(root.path().join("required.json"))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(serde_json::from_slice::<TraceArtifact>(&output.stdout).is_ok());
    }
}
#[test]
fn strict_trace_artifact_rejects_unknown_version_fields_and_golden_recomputes() {
    let bytes = include_bytes!("../fixtures/traces/trace.json");
    let artifact: TraceArtifact = serde_json::from_slice(bytes).unwrap();
    export(&artifact).unwrap();
    for field in ["apiVersion", "extra"] {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        value[field] = serde_json::json!("unsupported");
        assert!(serde_json::from_value::<TraceArtifact>(value).is_err());
    }
}
