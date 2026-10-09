mod common;
use common::*;
use specguard::{model::*, parser::*};
#[test]
fn explicit_ids_and_acceptance_locations() {
    let (_, s) = snapshot(document().as_bytes());
    let p = parse(&s);
    assert_eq!(p.requirements[0].key, key("R1"));
    assert_eq!(p.acceptances[0].requirement, key("R1"));
    assert_eq!(p.requirements[0].source.line, 5);
    assert!(p.sources.iter().all(|s| s.status == Terminal::Complete));
}
#[test]
fn malformed_unknown_non_utf8_and_budget_are_incomplete() {
    for bytes in [
        b"---\nformat: [\n---\n".as_slice(),
        b"---\nformat: other/v2\nnamespace: demo\n---\n",
        &[255u8],
    ] {
        let (_, s) = snapshot(bytes);
        assert!(
            parse(&s)
                .sources
                .iter()
                .any(|s| s.status != Terminal::Complete)
        );
    }
    let (_, mut s) = snapshot(document().as_bytes());
    s.inventory.limits.max_lines = 1;
    assert!(
        parse(&s)
            .sources
            .iter()
            .any(|s| s.status == Terminal::Limit)
    );
}
#[test]
fn openspec_profile_and_commands_are_only_data() {
    let (d, mut s) = snapshot(
        document()
            .replace("markdown-explicit/v1", "openspec-explicit/v1")
            .replace("## Requirement:", "### Requirement:")
            .replace("### Acceptance:", "#### Scenario:")
            .as_bytes(),
    );
    s.inventory.entries[0].format = "openspec-explicit/v1".into();
    let p = parse(&s);
    assert_eq!(p.requirements.len(), 1);
    assert_eq!(p.acceptances.len(), 1);
    assert!(!d.path().join("executed").exists());
}
