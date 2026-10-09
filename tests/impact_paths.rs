mod common;
use common::*;
use specguard::{graph::*, model::*};
use std::collections::BTreeSet;
#[test]
fn reverse_impact_has_bounded_deterministic_paths_through_cycles() {
    let mut g = graph();
    let source = g.parsed.requirements[0].source.clone();
    for (a, b) in [
        ("R2", "R1"),
        ("R3", "R1"),
        ("R4", "R3"),
        ("R4", "R2"),
        ("R1", "R4"),
    ] {
        g.parsed.edges.push(TraceEdge {
            from: key(a),
            to: key(b),
            relation: Relation::DependsOn,
            source: source.clone(),
        });
    }
    let changed = BTreeSet::from([key("R1")]);
    let i = impact(&changed, &g, 20);
    assert!(i.complete);
    assert_eq!(i.paths.len(), 4);
    assert!(i.paths.contains(&vec![key("R1"), key("R2"), key("R4")]));
    g.parsed.edges.reverse();
    assert_eq!(i, impact(&changed, &g, 20));
    assert!(!impact(&changed, &g, 1).complete);
}
