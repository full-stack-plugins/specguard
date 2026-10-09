# Measured compatibility and local source distribution

This is a Linux x86_64 development profile, tested with Rust 1.90.0. It does not publish a registry package, authenticate production identities, or qualify another platform. The original task's GE-RELEASE production boundary remains in force.

## Explicit matrix

`fixtures/compatibility/matrix.json` and `tests/compatibility_matrix.rs` exercise actual filesystem discovery, frozen snapshots, parsers, protected domain mappings, engine evaluation and artifact verification:

| Source capability | Measured result |
| --- | --- |
| `markdown-explicit/v1`, `openspec-explicit/v1` | Complete structural evidence / ALLOW |
| `openspec/1.14.1-main`, `openspec/1.14.1-added` plus identity registry | Complete structural evidence / ALLOW |
| `openspec/1.14.0-main`, `openspec/1.14.2-main`, `markdown-explicit/v2` | Unknown capability, incomplete evidence / BLOCK |
| Actual baseline-review golden | Verified REQUIRE_APPROVAL, no authorization grant |
| Unknown envelope version, absent contract, missing observed coverage | Rejected |
| Rehashed domain claiming production-authenticated profile | Rejected |

The native fixtures record OpenSpec 1.14.1 provenance in `fixtures/source-versions/`; the test does not execute an installed OpenSpec CLI. A neighboring version is not implicitly compatible. Existing CLI-review provenance remains advisory frozen filesystem input, not proof of committed Git source or production approval.

SDK source combinations measured by the independent consumer:

| SG source | GE source | GG source | Result |
| --- | --- | --- | --- |
| `1ebf6363a38859293a1753a5485e65363210116d` | `6527e2a67cb55690330c95dd12b496dd878ed39b` | `95eb3d9e45dab9122f2f025c2b7767a981c61a60` | Both matrix tests pass |
| Same SG | Actual local GE c80ec325 archive, SHA-256 `da5b30c22c28c9db444a0ca9c214b9441c3b7aebd14c223606298547452189bc` | Same GG | Both matrix tests pass |

The second row corrects an initial, untested expectation of incompatibility: the actual c80 artifact already contains the relevant publisher and eligibility interfaces. All 86 extracted files were compared with the immutable `.crate`. Neither the old artifact nor its lock was modified. This row qualifies these tests only; it does not assert all SDK behavior is equivalent. All three crates use 0.1.0, which is not a substitute for exact source pins.

## Independent development distribution

`scripts/source-bundle.py pack` reads exact local Git commits and selects Cargo manifests, locks, source, and the SG matrix's fixture/helper inputs. It produces a deterministic content-addressed tar with a strict manifest containing all three commits and every file's SHA-256. No `.git`, target, toolchain, or credential directory is packaged. The source archive recorded in `fixtures/compatibility/distribution-evidence.json` is `a5c2a8d06fc80f7246027011ae399a38e4c3f09c2a1d29c87cccc73f51f87ce2`.

`unpack` requires a new destination in a caller-owned parent. It checks the archive's content address, schema/profile, inventory and file hashes before writing. Nonregular entries, traversal, duplicate paths and unknown source namespaces are rejected. This is an integrity check, not an authenticated identity or an operating-system sandbox: callers must independently select the trusted archive digest and source pins. Packing assumes trusted local repositories; the subprocess's Git archive output is not a hostile-repository streaming resource boundary.

Example (substitute exact reviewed full commits):

```sh
python3 scripts/source-bundle.py pack ARTIFACTS \
  --specguard SG_REPO SG_COMMIT --guardengine GE_REPO GE_COMMIT --gitguard GG_REPO GG_COMMIT
python3 scripts/prepare-source-consumer.py ARTIFACTS/SHA256.tar NEW_PRIVATE_DIRECTORY
cargo +1.90.0 test --offline --manifest-path NEW_PRIVATE_DIRECTORY/consumer/Cargo.toml
cargo +1.90.0 test --locked --offline --manifest-path NEW_PRIVATE_DIRECTORY/consumer/Cargo.toml
cargo +1.90.0 metadata --locked --offline --format-version 1 \
  --manifest-path NEW_PRIVATE_DIRECTORY/consumer/Cargo.toml
```

The first consumer invocation resolves its new root package into a separate lock; the subsequent locked invocation qualifies that graph. The measured consumer was outside all implementation repositories. Metadata confirmed that SG, GE and GG manifests all resolve inside the extracted bundle. Third-party registry crates still require a populated offline Cargo cache; they are not vendored. The consumer reruns the packaged actual matrix; it does not trust a precomputed success flag.

The source-package commit intentionally precedes this evidence document, avoiding a self-referential package digest. No binary installer, public registry release, production authority provider, remote update channel, or blanket N/N-1 promise is supplied. Rollout and rollback policy are separate task 4.7.
