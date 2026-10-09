#[path = "../tests/common/mod.rs"]
mod common;
#[path = "../tests/producer_support/mod.rs"]
mod producer_support;
#[path = "rollout/mod.rs"]
mod rollout;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(
        args.len(),
        3,
        "usage: local_rollout ABSOLUTE_SG_BINARY ABSOLUTE_MCP_BINARY"
    );
    println!("{}", rollout::exercise(&args[1], &args[2]));
}
