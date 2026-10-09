mod common;
use common::*;
use specguard::{graph::*, model::*, rules::*};
use std::collections::BTreeSet;
#[test]
fn duplicates_keep_all_locations_and_order_is_stable() {
    let mut p = graph().parsed;
    let mut duplicate = p.requirements[0].clone();
    duplicate.source.path = "specs/b.md".into();
    p.requirements.push(duplicate);
    let g = build_graph(p.clone());
    p.requirements.reverse();
    assert_eq!(g, build_graph(p));
    let f = validate_graph(&g, &BTreeSet::from([key("R1")]));
    let d = f.iter().find(|f| f.kind == FindingKind::Duplicate).unwrap();
    assert_eq!(d.sources.len(), 2);
}
#[test]
fn broken_reference_is_located() {
    let mut g = graph();
    g.parsed.edges.push(TraceEdge {
        from: key("R1"),
        to: key("missing"),
        relation: Relation::DependsOn,
        source: SourceRef {
            path: "specs/a.md".into(),
            line: 9,
        },
    });
    let f = validate_graph(&g, &BTreeSet::from([key("R1")]));
    assert!(
        f.iter()
            .any(|f| f.kind == FindingKind::BrokenReference && f.sources[0].line == 9)
    );
}
