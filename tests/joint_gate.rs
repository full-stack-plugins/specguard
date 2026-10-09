#[path = "../examples/joint_gate.rs"]
mod example;
#[test]
fn real_four_group_peer_captures_feed_current_baseline_lifecycle() {
    let report = example::run();
    assert_eq!(report["groups"], 4);
    for field in [
        "wrong_requirement_rejected",
        "expired_rejected",
        "revoked_rejected",
        "baseline_drift_rejected",
        "late_m1_rejected",
        "retained_bytes_unchanged",
        "rollback_required_unavailable",
    ] {
        assert_eq!(report[field], true, "{field}");
    }
    assert_eq!(report["execution_authorized"], false);
}
