#[path = "ci/mod.rs"]
mod ci;
fn main() {
    for scenario in [
        ci::Scenario::Good,
        ci::Scenario::CandidateWeakensRules,
        ci::Scenario::Partial,
    ] {
        let run = ci::exercise(scenario);
        let result = run.evaluate(&run.bundle.output().envelope);
        println!(
            "{}",
            serde_json::json!({"profile":ci::ADAPTER,"candidate":run.expected.binding.candidate_oid,"sourceDigest":run.expected.binding.source_snapshot_digest,"decision":result.technical_decision,"eligibility":result.code,"unchanged":run.unchanged,"repositoryBefore":run.repository_before,"repositoryAfter":run.repository_after,"contractDigest":run.expected.contract_digest})
        );
    }
}
