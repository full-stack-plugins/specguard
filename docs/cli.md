# Current read-only CLI

Build locally with `cargo build --locked --bin specguard`; `cargo run --locked -- --help` prints actual positional syntax. No released installer or production authorization is claimed.

```
specguard doctor ROOT POLICY.json
specguard scan ROOT POLICY.json BINDING.json REQUIRED.json
specguard trace ROOT POLICY.json BINDING.json REQUIRED.json
specguard diff ROOT POLICY.json BINDING.json BASELINE.json --unverified-baseline
specguard check ROOT REQUEST.json [--cancel]
specguard trace-check ROOT POLICY.json BINDING.json REQUIRED.json
specguard trace-export ROOT POLICY.json BINDING.json REQUIRED.json
```

`doctor` discovers configured sources and reports terminal inventory/capability diagnostics. `scan` freezes actual source bytes and emits the typed graph, inventory and binding. `trace` emits the validated versioned trace artifact with scoped findings. Query commands exit0 when output is produced, including partial coverage; this is not gate success. Input/runtime errors exit4. Existing trace-check/trace-export retain their previous 0 for complete/no findings,2 for incomplete/findings,4 for errors contract.

`diff` runs conservative structural comparison against a content-validated immutable baseline and never infers a rename without a mapping. Only explicit `--unverified-baseline` local mode is available: the output labels authentication unverified. It does not validate external approval identity, expiry or revocation, and cannot authorize a gate. Without that flag it rejects the request. Full authenticated diff remains a later adapter.

`check` reads a strict bounded JSON request with apiVersion=`specguard.cli-check/v1alpha1`, sourcePolicy (SourcePolicy), binding (CandidateBinding), required (Identity set), invocation (producer Invocation), mapping (ProtectedMapping), and finishedAt (UTC timestamp). See executable [CLI tests](../tests/cli_contract.rs) and [producer test builders](../tests/producer_support/mod.rs) for complete requests backed by real temporary Git repositories. Invocation retains its snake_case fields; the outer request uses camelCase. Unknown fields/version, incomplete mapping, source or Git-object/binding failures are rejected. Caller-supplied mapping must come from the host's protected policy; reading a JSON file does not authenticate it.

The check stdout object has apiVersion, authenticationProfile=`unverified`, a real GE envelope and exact raw contract/facts/report/domain bytes represented as optional JSON byte arrays. Raw bytes preserve artifact digests. It has no trusted flag. Actual outcomes are0 ALLOW,2 BLOCK (including completed partial evidence),4 error/cancelled. Exit3 is reserved by the general decision adapter but is unreachable in the current Enforce-only structural profile; no fake REQUIRE_APPROVAL fixture is claimed and structural findings are not weakened to manufacture one. Failed bound attempts retain null decision, no successful payloads and original bounded diagnostic bytes. Pre-binding failures emit only JSON stderr diagnostics and no stdout/envelope.

`--cancel` requests cancellation after preparation/binding and before analysis using the reviewed cooperative runtime token, producing cancelled/null and exit4. It is not proof of CLI signal handling or OS worker termination. Mid-run cancellation/panic-unwind is separately tested through the library API; this CLI does not install signal handlers.

All commands are stdout-only and read-only. `--report` and `--report=...` are explicitly rejected before input processing, with no path mutation. Prior success files remain untouched and are never loaded as this run's outcome. Consumers must use the current process status and current bound envelope; shell redirection and atomic file publication are host responsibilities. CLI task4.1 remains partial for full decision3/authenticated diff and atomic publication.

Configuration reads cap raw JSON at1MiB, require regular files and on Unix use no-follow/nonblocking opens. Existing source snapshot/path and parser budgets remain in force; full task4.2 is not implied by these checks. No tools are downloaded, source instructions executed, approvals issued or files repaired. Diagnostics go to stderr; stdout is a single JSON result (except human `--help`).
