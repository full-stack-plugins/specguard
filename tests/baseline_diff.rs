mod common;
use common::*;
use specguard::{baseline::*, model::*};
use std::collections::BTreeMap;
#[test]
fn partial_comparison_never_confirms_deletion() {
    let b = baseline();
    let original = b.clone();
    let mut g = b.graph.clone();
    g.parsed.requirements.clear();
    g.parsed.acceptances.clear();
    let diff = compare_baseline(&b, &g, &BTreeMap::new()).unwrap();
    assert!(diff.changes.iter().any(|c| c.kind == ChangeKind::Removed));
    g.parsed.sources[0].status = Terminal::Malformed;
    let partial = compare_baseline(&b, &g, &BTreeMap::new()).unwrap();
    assert!(!partial.complete);
    assert!(
        !partial
            .changes
            .iter()
            .any(|c| matches!(c.kind, ChangeKind::Removed | ChangeKind::AcceptanceRemoved))
    );
    assert_eq!(b, original);
}
#[test]
fn stable_moves_and_explicit_migrations_do_not_guess_identity() {
    let b = baseline();
    let mut g = b.graph.clone();
    g.parsed.requirements[0].source.path = "moved.md".into();
    assert!(
        compare_baseline(&b, &g, &BTreeMap::new())
            .unwrap()
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::Moved)
    );
    g.parsed.requirements[0].key = key("R2");
    g.parsed.acceptances[0].requirement = key("R2");
    let diff = compare_baseline(&b, &g, &BTreeMap::new()).unwrap();
    assert!(diff.changes.iter().any(|c| c.kind == ChangeKind::Removed));
    assert!(diff.changes.iter().any(|c| c.kind == ChangeKind::Added));
    let mapped = compare_baseline(&b, &g, &BTreeMap::from([(key("R1"), key("R2"))])).unwrap();
    assert!(!mapped.changes.iter().any(|c| c.kind == ChangeKind::Removed));
    g.parsed.requirements[0].text = "different meaning".into();
    assert!(
        compare_baseline(&b, &g, &BTreeMap::from([(key("R1"), key("R2"))]))
            .unwrap()
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::TextReview)
    );
}
#[test]
fn duplicate_acceptances_make_comparison_incomplete() {
    let b = baseline();
    let mut g = b.graph.clone();
    g.parsed.acceptances.push(g.parsed.acceptances[0].clone());
    assert!(!compare_baseline(&b, &g, &BTreeMap::new()).unwrap().complete);
}

#[test]
fn migration_rejects_retained_old_identity_in_candidate() {
    let b = baseline();
    let mut candidate = b.graph.clone();
    let mut migrated = candidate.parsed.requirements[0].clone();
    migrated.key = key("R2");
    candidate.parsed.requirements.push(migrated);
    candidate.parsed.requirements[0].text = "Changed retained R1".into();
    let mappings = BTreeMap::from([(key("R1"), key("R2"))]);
    assert!(compare_baseline(&b, &candidate, &mappings).is_err());
    let unmapped = compare_baseline(&b, &candidate, &BTreeMap::new()).unwrap();
    assert!(
        unmapped
            .changes
            .iter()
            .any(|c| c.key == key("R1") && c.kind == ChangeKind::TextReview)
    );
    assert!(
        unmapped
            .changes
            .iter()
            .any(|c| c.key == key("R2") && c.kind == ChangeKind::Added)
    );
}
