mod common;
mod producer_support;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    collections::BTreeSet,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Observed;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Observed {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            let n = LIVE.fetch_add(l.size(), Ordering::Relaxed) + l.size();
            PEAK.fetch_max(n, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Ordering::Relaxed);
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let p = unsafe { System.realloc(p, l, n) };
        if !p.is_null() {
            let live = if n >= l.size() {
                LIVE.fetch_add(n - l.size(), Ordering::Relaxed) + n - l.size()
            } else {
                LIVE.fetch_sub(l.size() - n, Ordering::Relaxed) - (l.size() - n)
            };
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        p
    }
}
#[global_allocator]
static ALLOC: Observed = Observed;
#[test]
fn real_source_path_fanout_is_rejected_before_graph_allocation() {
    use specguard::{integration::producer::*, source::*};
    let root = common::repo();
    let directory = (0..12)
        .map(|_| "x".repeat(240))
        .collect::<Vec<_>>()
        .join("/");
    std::fs::create_dir_all(root.path().join(&directory)).unwrap();
    let relative = format!("{directory}/a.md");
    let mut text = "---\nformat: markdown-explicit/v1\nnamespace: demo\n---\n".to_owned();
    for i in 0..6000 {
        text.push_str(&format!("## Requirement: R{i}\n"));
    }
    std::fs::write(root.path().join(&relative), &text).unwrap();
    let mut p = common::policy();
    p.roots[0].path = relative.clone();
    let s = freeze(
        root.path(),
        &discover(root.path(), &p).unwrap(),
        common::binding(root.path()),
    )
    .unwrap();
    let prepared = prepare(
        root.path(),
        &s,
        producer_support::invocation(&s),
        &BTreeSet::from([common::key("R1")]),
        &producer_support::policy(),
    )
    .unwrap();
    let baseline = LIVE.load(Ordering::Relaxed);
    PEAK.store(baseline, Ordering::Relaxed);
    let result = prepared.complete("2026-10-09T10:00:01Z").unwrap();
    let extra = PEAK.load(Ordering::Relaxed).saturating_sub(baseline);
    println!(
        "source_bytes={} path_bytes={} nodes=6000 extra_peak_bytes={} status={:?}",
        text.len(),
        relative.len(),
        extra,
        result.envelope.run_status
    );
    assert_eq!(result.envelope.diagnostics[0].code, "parser.budget");
    assert!(result.report.is_none() && result.envelope.decision.is_none());
    assert!(
        extra <= guardengine::integration::MAX_ARTIFACT_BYTES,
        "domain graph fanout before budget: {extra}"
    );
}
