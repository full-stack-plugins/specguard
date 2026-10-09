use crate::common::*;
use specguard::source::*;
pub fn fixture(adr: &str) -> (tempfile::TempDir, SourceSnapshot, SourcePolicy) {
    let root = repo();
    std::fs::create_dir(root.path().join("specs")).unwrap();
    std::fs::write(
        root.path().join("specs/a.md"),
        format!(
            "{}\n- traces_to_adr: demo:ADR1\n- traces_to_task: demo:T1\n",
            document()
        ),
    )
    .unwrap();
    std::fs::write(root.path().join("adr.md"), adr).unwrap();
    std::fs::write(root.path().join("tasks.md"),"---\nformat: markdown-task/v1\nnamespace: demo\n---\n## Task: T1\nImplement the requirement.\n").unwrap();
    let mut p = policy();
    for (path, format) in [
        ("adr.md", "markdown-adr/v1"),
        ("tasks.md", "markdown-task/v1"),
    ] {
        p.roots.push(SourceRoot {
            path: path.into(),
            format: format.into(),
            namespace: "demo".into(),
            authority: "primary".into(),
        });
    }
    let snapshot = freeze(
        root.path(),
        &discover(root.path(), &p).unwrap(),
        binding(root.path()),
    )
    .unwrap();
    (root, snapshot, p)
}
pub const ADR: &str =
    "---\nformat: markdown-adr/v1\nnamespace: demo\n---\n## ADR: ADR1\nUse the specified design.\n";
