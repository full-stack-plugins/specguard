mod common;
use common::*;
use specguard::{model::*, rules::*};
use std::collections::BTreeSet;
#[test]
fn frozen_scope_distinguishes_violation_partial_and_tool_error() {
    let frozen = BTreeSet::from([key("R1"), key("R2")]);
    let mut g = graph();
    assert!(validate_graph(&g, &BTreeSet::from([key("R1")])).is_empty());
    g.parsed.acceptances.clear();
    let f = validate_graph(&g, &frozen);
    assert!(f.iter().any(|f| f.kind == FindingKind::MissingAcceptance));
    assert!(f.iter().any(|f| f.kind == FindingKind::MissingRequirement));
    g.parsed.sources[0].status = Terminal::Malformed;
    let f = validate_graph(&g, &frozen);
    assert_eq!(f.len(), 2);
    assert!(f.iter().all(|f| f.kind == FindingKind::Incomplete));
    g.parsed.sources[0].status = Terminal::IoError;
    assert!(
        validate_graph(&g, &frozen)
            .iter()
            .all(|f| f.kind == FindingKind::ToolError)
    );
}
#[test]
fn every_structural_rule_respects_partial_and_tool_failures() {
    let required = BTreeSet::from([key("R1")]);
    for mode in 0..3 {
        let mut g = graph();
        match mode {
            0 => g.parsed.requirements.push(g.parsed.requirements[0].clone()),
            1 => g.parsed.edges.push(TraceEdge {
                from: key("R1"),
                to: key("missing"),
                relation: Relation::DependsOn,
                source: g.parsed.requirements[0].source.clone(),
            }),
            _ => g.parsed.acceptances.clear(),
        };
        assert!(!validate_graph(&g, &required).is_empty());
        for (status, kind) in [
            (Terminal::Unsupported, FindingKind::Incomplete),
            (Terminal::IoError, FindingKind::ToolError),
        ] {
            g.parsed.sources[0].status = status;
            assert!(validate_graph(&g, &required).iter().all(|f| f.kind == kind));
        }
    }
}
#[test]
fn reserved_external_relation_cannot_claim_complete_analysis() {
    let mut g = graph();
    g.parsed.edges.push(TraceEdge {
        from: key("R1"),
        to: key("ADR-1"),
        relation: Relation::TracesToAdr,
        source: g.parsed.requirements[0].source.clone(),
    });
    assert!(
        validate_graph(&g, &BTreeSet::from([key("R1")]))
            .iter()
            .any(|f| f.kind == FindingKind::Incomplete)
    );
}
