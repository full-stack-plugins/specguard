use crate::{common::*, producer_support};
use serde_json::{Value, json};
pub fn review_request(
    root: &std::path::Path,
    s: &specguard::source::SourceSnapshot,
) -> std::path::PathBuf {
    let p = root.join("request.json");
    std::fs::write(&p,serde_json::to_vec(&json!({"apiVersion":"specguard.cli-check/v1alpha1","sourcePolicy":policy(),"binding":s.binding,"required":[key("R1")],"invocation":producer_support::invocation(s),"mapping":producer_support::policy(),"finishedAt":"2026-10-09T10:00:01Z"})).unwrap()).unwrap();
    let mut b = baseline();
    b.repository = "fixture-repository".into();
    let mut v: Value = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    v["apiVersion"] = "specguard.cli-check-baseline-review/v1alpha1".into();
    v["invocation"]["baseline_digest"] = specguard::model::digest(&b).into();
    v["review"] = json!({"apiVersion":"specguard.baseline-review/v1", "baseline":b,
    "rules":[
        {"key":key("R1"),"kind":"text_review","ruleId":"review-text","subject":"demo:R1","predicate":"baseline_change","object":"text_review"},
        {"key":key("R1"),"kind":"acceptance_review","ruleId":"review-acceptance","subject":"demo:R1","predicate":"baseline_change","object":"acceptance_review"}
    ],
    "contract":null});
    // Use real engine serialization rather than inventing its wire enum spelling.
    let mut contract = producer_support::policy().contract;
    contract.spec.rules.truncate(2);
    for (rule, (id, obj)) in contract.spec.rules.iter_mut().zip([
        ("review-text", "text_review"),
        ("review-acceptance", "acceptance_review"),
    ]) {
        rule.id = id.into();
        rule.enforcement = guardengine::Enforcement::Review;
        rule.assertion = guardengine::GuardAssertion::ForbidRelation {
            subject: "demo:R1".into(),
            predicate: "baseline_change".into(),
            object: obj.into(),
        };
    }
    v["review"]["contract"] = serde_json::to_value(contract).unwrap();
    std::fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();
    p
}
