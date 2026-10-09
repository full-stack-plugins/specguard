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

#[test]
fn typed_adr_and_task_links_resolve_by_identity_and_preserve_all_conflict_sources() {
    let mut parsed = graph().parsed;
    parsed.edges = vec![
        TraceEdge {
            from: key("R1"),
            to: key("ADR1"),
            relation: Relation::TracesToAdr,
            source: SourceRef {
                path: "specs/a.md".into(),
                line: 7,
            },
        },
        TraceEdge {
            from: key("R1"),
            to: key("T1"),
            relation: Relation::TracesToTask,
            source: SourceRef {
                path: "specs/a.md".into(),
                line: 8,
            },
        },
    ];
    let mut targets = TraceTargets {
        nodes: vec![
            TraceTarget {
                key: key("ADR1"),
                kind: TargetKind::Adr,
                source: SourceRef {
                    path: "adr/one.md".into(),
                    line: 1,
                },
            },
            TraceTarget {
                key: key("T1"),
                kind: TargetKind::Task,
                source: SourceRef {
                    path: "tasks/one.md".into(),
                    line: 2,
                },
            },
        ],
        sources: ["adr/one.md", "tasks/one.md"]
            .into_iter()
            .map(|path| SourceStatus {
                path: path.into(),
                status: Terminal::Complete,
                reason: "fixture inventory".into(),
            })
            .collect(),
    };
    let typed = build_typed_graph(parsed.clone(), targets.clone());
    assert!(typed.complete());
    assert!(typed.validate(&BTreeSet::from([key("R1")])).is_empty());
    let mut duplicate = targets.nodes[0].clone();
    duplicate.source.path = "adr/two.md".into();
    targets.nodes.push(duplicate);
    targets.sources.push(SourceStatus {
        path: "adr/two.md".into(),
        status: Terminal::Complete,
        reason: "fixture".into(),
    });
    let typed = build_typed_graph(parsed.clone(), targets.clone());
    let findings = typed.validate(&BTreeSet::from([key("R1")]));
    let duplicate = findings
        .iter()
        .find(|f| f.kind == FindingKind::Duplicate)
        .unwrap();
    assert_eq!(duplicate.sources.len(), 2);
    targets.nodes.reverse();
    parsed.edges.reverse();
    assert_eq!(typed, build_typed_graph(parsed, targets));
}

#[test]
fn typed_trace_wrong_kind_direction_and_missing_targets_are_located() {
    for (from, to, relation) in [
        (key("R1"), key("T1"), Relation::TracesToAdr),
        (key("T1"), key("R1"), Relation::TracesToTask),
        (key("R1"), key("absent"), Relation::TracesToTask),
    ] {
        let mut parsed = graph().parsed;
        parsed.edges = vec![TraceEdge {
            from,
            to,
            relation,
            source: SourceRef {
                path: "specs/a.md".into(),
                line: 19,
            },
        }];
        let targets = TraceTargets {
            nodes: vec![TraceTarget {
                key: key("T1"),
                kind: TargetKind::Task,
                source: SourceRef {
                    path: "tasks/one.md".into(),
                    line: 1,
                },
            }],
            sources: vec![SourceStatus {
                path: "tasks/one.md".into(),
                status: Terminal::Complete,
                reason: "fixture".into(),
            }],
        };
        let findings = build_typed_graph(parsed, targets).validate(&BTreeSet::from([key("R1")]));
        assert!(
            findings
                .iter()
                .any(|f| f.kind == FindingKind::BrokenReference
                    && f.sources.iter().any(|s| s.line == 19))
        );
    }
}

#[test]
fn incomplete_trace_target_scope_does_not_claim_a_missing_target() {
    let mut parsed = graph().parsed;
    parsed.edges.push(TraceEdge {
        from: key("R1"),
        to: key("missing"),
        relation: Relation::TracesToAdr,
        source: SourceRef {
            path: "specs/a.md".into(),
            line: 9,
        },
    });
    let typed = build_typed_graph(
        parsed,
        TraceTargets {
            nodes: vec![],
            sources: vec![],
        },
    );
    assert!(!typed.complete());
    let findings = typed.validate(&BTreeSet::from([key("R1")]));
    assert!(findings.iter().any(|f| f.kind == FindingKind::Incomplete));
    assert!(
        !findings
            .iter()
            .any(|f| f.kind == FindingKind::BrokenReference)
    );
}

#[test]
fn unrelated_complete_source_cannot_cover_a_trace_target() {
    let targets = TraceTargets {
        nodes: vec![TraceTarget {
            key: key("ADR1"),
            kind: TargetKind::Adr,
            source: SourceRef {
                path: "adr/one.md".into(),
                line: 3,
            },
        }],
        sources: vec![SourceStatus {
            path: "unrelated.md".into(),
            status: Terminal::Complete,
            reason: "parsed".into(),
        }],
    };
    assert!(!build_typed_graph(graph().parsed, targets).complete());
}
