mod common;
use common::*;
use serde_json::json;
use specguard::baseline::*;
#[test]
fn immutable_baseline_requires_every_binding() {
    let b = baseline();
    let v = serde_json::to_value(&b).unwrap();
    assert!(decode_baseline(v.clone()).is_ok());
    for field in [
        "sourceRevision",
        "sourceDigest",
        "policyDigest",
        "scope",
        "effectiveFrom",
        "approvalRef",
    ] {
        let mut missing = v.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(decode_baseline(missing).is_err());
    }
    let mut forged = v;
    forged["approved"] = json!(true);
    assert!(decode_baseline(forged).is_err());
    let mut drift = b.clone();
    drift.graph.parsed.requirements[0].text = "tampered".into();
    assert!(validate_baseline(&drift).is_err());
    assert_eq!(b, baseline_from_value(&b));
}
fn baseline_from_value(b: &ApprovedBaseline) -> ApprovedBaseline {
    decode_baseline(serde_json::to_value(b).unwrap()).unwrap()
}
