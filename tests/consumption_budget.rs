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
fn oversized_controller_policy_is_rejected_before_owning_clone() {
    use guardengine::integration::{Producer, eligibility::EligibilityPolicy};
    use specguard::integration::{
        approval::Profile, freshness::CurrentExpectation, producer::prepare,
    };
    let (root, s) = common::snapshot(common::document().as_bytes());
    let prepared = prepare(
        root.path(),
        &s,
        producer_support::invocation(&s),
        &BTreeSet::from([common::key("R1")]),
        &producer_support::policy(),
    )
    .unwrap();
    let policy = EligibilityPolicy {
        binding: prepared.binding().clone(),
        producer: Producer {
            guard: "SpecGuard".into(),
            version: "0.1.0".into(),
            analyzer_id: "specguard.structural".into(),
            analyzer_version: "1".into(),
        },
        required_scopes: vec![],
        contract_digest: format!("sha256:{}", "a".repeat(64)),
        action: "x".repeat(17 * 1024 * 1024),
        producer_principals: BTreeSet::new(),
        approval_principals: std::collections::BTreeMap::new(),
    };
    let before = LIVE.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);
    let result = CurrentExpectation::freeze(&prepared, 1, &policy, Profile::Fixture);
    let extra = PEAK.load(Ordering::Relaxed).saturating_sub(before);
    assert!(result.is_err());
    println!("17MiB action admission extra_peak_bytes={extra}");
    assert!(extra < 4096);
}
