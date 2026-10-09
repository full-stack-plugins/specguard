mod common;
mod producer_support;
#[path = "../examples/rollout/mod.rs"]
mod rollout;
#[test]
fn actual_adapters_roll_out_and_independently_roll_back_without_losing_evidence() {
    let observed = rollout::exercise(
        env!("CARGO_BIN_EXE_specguard"),
        env!("CARGO_BIN_EXE_specguard-mcp"),
    );
    assert_eq!(observed["profile"], "local-unverified-controller-demo");
    assert_eq!(observed["advisoryWrites"], false);
    assert_eq!(observed["unresolvedDifferenceBlocksEnable"], true);
    assert_eq!(observed["explicitOptInRequired"], true);
    assert_eq!(observed["requiredConsumptionBeforeRollback"], true);
    assert_eq!(observed["cliDisabledRejectsRequired"], true);
    assert_eq!(observed["mcpDisabledRejectsRequired"], true);
    assert_eq!(observed["otherAdapterStillReads"], true);
    assert_eq!(observed["historyPreserved"], true);
    assert_eq!(observed["sourcePreserved"], true);
    assert_eq!(observed["executionAuthorized"], false);
}
