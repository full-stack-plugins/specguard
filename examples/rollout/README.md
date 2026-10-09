# Local adapter rollout and independent rollback

Build the existing binaries and run the real local exercise:

```sh
cargo build --locked --offline --bins
cargo run --locked --offline --example local_rollout -- \
  "$PWD/target/debug/specguard" "$PWD/target/debug/specguard-mcp"
```

Python MCP SDK 1.29.0 is the selected test client (already used by the MCP
profile). No new SDK, external platform, shell-command endpoint or dependency
installer is added. The exercise creates a private temporary Git repository,
commits actual specification bytes, then executes the real CLI and MCP SDK
against the same frozen source, required identities and protected mapping.
The example only accepts binary paths from its local operator. It is not a
remotely exposed command service.

The advisory phase runs the original CLI without enabling required consumption.
Shadow runs MCP and a deliberately cancelled CLI check: their actual outcomes
differ and activation fails. A new successful CLI check resolves the difference
by comparing producer, complete candidate/source binding, coverage, technical
status/decision and contract digest. Even then explicit controller opt-in is
required. Both adapters remain required for the demonstration's local technical
consumption. There is no automatic shrinking of the required set.

Disabling CLI blocks required consumption while MCP produces a fresh result;
disabling MCP blocks required consumption while CLI produces a fresh result.
Old current evidence and the complete retained history are preserved for audit,
including exact report bytes queried from the retained MCP session. Such audit
queries do not reactivate the adapter for required consumption. Each returned
CLI/MCP bundle is reconstructed and verified through the existing Rust
`ProducedRun::verify`; hashes alone are not expert-result verification. The
exercise asserts source/configuration bytes are unchanged. Its JSON summary
includes all six new run-history digests and explicit `executionAuthorized:false`.

These are actual local adapters and results, with a small private demonstration
controller. Opt-in/enforce describes required local technical consumption, not
hosted branch protection, a production issuer, authorization, an execution
grant or native command replacement. Structural MCP does not authenticate
approved baselines; the separate reviewed trust port remains unchanged.
Temporary run history lasts for the exercise and is checked before teardown;
this is not a durable audit store or production rollback deployment.

CodeGuard is neither called nor modified by this exercise. Native command/exit
preservation reuses the independently accepted historical matrix at CodeGuard
`25bac257d4c7e8767c391402833468fdeb5d83c4`, documented in the implementation
ledger `codeguard-native-parity-independent-review.md`: 23 original captures,
10 compared old/new pairs and 18 independent native reruns, with the disclosed
old/new provenance and platform limits. That retained evidence is not presented
as a new native rerun here.

`tests/rollout_rollback.rs` executes this same composition. Task 4.7 stays
unchecked until independent acceptance.
