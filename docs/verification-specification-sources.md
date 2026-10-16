# Source adapter verification record

Scope: EG-S01 / EG-SPEC-001 of the existing `codeguard/openspec/changes/add-engineering-guard-core` change. This record covers the SpecGuard source-observation implementation, not completion of protected specification baselines, business semantics, full traceability or delivery authorization. No cross-repository task checkbox was changed.

Local environment: Windows, Rust/Cargo 1.85, isolated target `E:/workspaces/workspace-agent-skills/.guard-build/targets/goal-specguard-s01`; temporary files and Cargo cache are on E:. No SDK installation, specification initialization, Git branch action, commit or push was performed.

## Observed TDD transitions

1. Added ten real CLI filesystem tests in `tests/spec_sources.rs` before implementing the new entry point. `cargo +1.85 test --locked --offline --test spec_sources` failed all ten: the old CLI returned argument exit 4 because `specs sources` did not exist. Implemented the read-only adapters; the same ten tests passed.
2. Added the real stdio MCP test before registering `specguard_spec_sources`. The tool-list assertion failed because the tool was absent. Added the tool and strict single-project arguments; the same test passed and its response exactly matched CLI JSON.
3. Added `empty_superpowers_id_prefix_is_not_a_requirement`. It failed because `REQ-:` was wrongly treated as a requirement and returned exit 0. Required a nonempty ID suffix; the same test now returns incomplete with no inferred requirement.

## Executed checks

| Check | Observed result |
|---|---|
| `cargo +1.85 test --locked --offline` | 49 passed: 10 AGENTS, 12 existing native checks, 15 source adapters, 12 trace checks; 0 failed |
| `cargo +1.85 fmt --check` | Passed |
| `cargo +1.85 clippy --locked --offline --all-targets -- -D warnings` | Passed |
| `git diff --check` | Passed |
| GuardEngine `verify_guard_packages.py --repositories-root ABS --bin-dir TARGET/debug --tools specguard` | Passed: 3 skills, 9 MCP tools, stdio verified, `host_loaded=false` |

The full regression was rerun after the empty-ID correction and the existing AGENTS test's MCP-count update. Clippy also exposed two pre-existing `format_collect` warnings in that touched test; constructing the same fixture strings with `fold` preserves its assertions and removes those warnings.

The shared `verify_guard_installation.py` initially observed 6 passed, 3 scope skips and 1 failure: its exact MCP-name set omitted the new `specguard_spec_sources` tool. The root owner added that exact tool name to the shared expectation and independently reran the actual binary: 7 passed and 3 scope skips, zero failures (1.636 seconds). Installation/uninstallation, mutation protection and stdio tool-set checks all passed within the selected single-product scope; the three skipped cases require other products/providers and are not claimed as verified.

Independent review also ran all 49 Rust 1.85 tests successfully and observed the real ArchGuard repository specifications: 13 native requirements were retained, with exit 3 for an older freeform specification lacking native Requirement keys. This validates honest incomplete handling, not complete specification semantics. The additive CLI/MCP release candidate and bundled plugin are versioned 0.3.0; no release tag or package publication is claimed.

Ubuntu execution and future repository CI results are not inferred from these Windows runs. Existing CI runs `cargo test --locked` on both Ubuntu and Windows, so it will include the new tests when the parent publishes the candidate.

The implementation keeps `approval_status=not_evaluated`, `execution_status=not_evaluated`, `trust=local_unverified`, `selected_source=null` and `approved_baseline_eligible=false`. There is no new signature or protocol trust claim. Superpowers freeform specs without explicit supported IDs remain incomplete; renamed native keys need explicit migration mapping. Multiple sources conservatively remain conflicting without an independent ownership decision.
