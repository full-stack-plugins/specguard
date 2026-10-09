mod common;
use common::*;
use specguard::{model::*, source::*};
#[test]
fn explicit_readonly_discovery_and_conflicts() {
    let d = repo();
    let p = policy();
    assert!(
        discover(d.path(), &p)
            .unwrap()
            .sources
            .iter()
            .any(|s| s.status != Terminal::Complete)
    );
    std::fs::create_dir(d.path().join("specs")).unwrap();
    std::fs::write(d.path().join("specs/a.md"), document()).unwrap();
    let before = std::fs::read(d.path().join("specs/a.md")).unwrap();
    let i = discover(d.path(), &p).unwrap();
    assert_eq!(i.entries.len(), 1);
    assert_eq!(std::fs::read(d.path().join("specs/a.md")).unwrap(), before);
    let mut conflict = p.clone();
    conflict.roots.push(SourceRoot {
        authority: "other".into(),
        ..p.roots[0].clone()
    });
    assert!(
        discover(d.path(), &conflict)
            .unwrap()
            .sources
            .iter()
            .any(|s| s.status == Terminal::Conflict)
    );
    let mut escape = p;
    escape.roots[0].path = "../".into();
    assert!(discover(d.path(), &escape).is_err());
}
#[test]
fn unknown_profile_is_terminal() {
    let d = repo();
    let mut p = policy();
    p.roots[0].format = "unknown/v8".into();
    assert!(
        discover(d.path(), &p)
            .unwrap()
            .sources
            .iter()
            .any(|s| s.status == Terminal::Unsupported)
    );
}
