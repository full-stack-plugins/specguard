use serde_json::json;
use specguard::model::*;

#[test]
fn domain_contract_is_strict_versioned_and_deterministic() {
    let valid = json!({"apiVersion":"specguard.domain/v1alpha1","snapshotDigest":"sha256:x","candidateOid":"abc","sources":[],"requirements":[],"acceptances":[],"edges":[]});
    let parsed = decode_parse(valid.clone()).expect("valid DTO");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), valid);
    let mut missing = valid.clone();
    missing.as_object_mut().unwrap().remove("sources");
    assert!(decode_parse(missing).is_err());
    let mut unknown = valid.clone();
    unknown["apiVersion"] = json!("v2");
    assert!(decode_parse(unknown).is_err());
    let mut extra = valid.clone();
    extra["approved"] = json!(true);
    assert!(decode_parse(extra).is_err());
    assert_eq!(digest(&parsed), digest(&decode_parse(valid).unwrap()));
}
