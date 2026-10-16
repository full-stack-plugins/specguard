# SpecGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**Deterministic requirements, specification and acceptance governance for AI-native software engineering.**

> **Status: 0.2.0 implements AGENTS.md organization-convention checks and native OpenSpec / Spec Kit structural checks, artifact SHA256 drift verification, CLI/MCP and plugin skills.** Declared native requirement/source/test linkage with signed current execution is implemented; business semantic correctness and protected approved-baseline checks remain outside this claim. See [trace evidence](docs/trace-evidence.md).

Use `specguard specs check --project ABSOLUTE_DIRECTORY --source openspec|speckit` and `specs verify ... --baseline ABSOLUTE_REPORT` for repeatable native-artifact checks. See [native scope, repair/recheck workflow and baseline trust](docs/native-specs.md). Successful snapshots are untrusted observations and never approval or implementation evidence.

Use `specguard agents check --project ABSOLUTE_DIRECTORY` to check six Guard instruction blocks and optional additive project conventions; `agents template` returns a review draft without writing files. See [scope and usage](docs/agents-context.md) and [plugin](plugin/README.md). Required blocks are organization conventions, not official AGENTS.md fields. Document ALLOW never proves agent execution or approval.

## Why SpecGuard

AI can generate requirements and coding tasks quickly but cannot independently prove that the requirement scope is correct. SpecGuard validates traceability, approved baselines and well-defined obligations, while sending ambiguous business intent to human review.

~~~text
Original request / OpenSpec / Spec Kit / Superpowers
                      ↓
                 Source adapters
                      ↓
     Requirement → Acceptance → Design / Task → Test obligation
                      ↓
             Structural rules + Baseline diff
                      ↓
             Findings / GuardFacts / Coverage
                      ↓
                  GuardEngine
                      ↓
          CLI feedback / Trusted CI / FlowGuard
~~~

## Responsibility boundaries

SpecGuard owns stable requirement IDs, specification references, completeness checks, acceptance obligations, and approved baseline diffs. It does not approve product decisions, design software architecture, run full behavioral tests, or merge Git changes.

**ENFORCE** applies only to deterministic constraints such as duplicate IDs, broken links and unauthorized removal of approved criteria. **REVIEW** handles ambiguous wording, missing business scenarios and design trade-offs. Missing source coverage is **INDETERMINATE**, not PASS.

The planned specification graph integrates with [GuardEngine](https://github.com/full-stack-plugins/guardengine) Guard Protocol. Implemented checkers return their own versioned reports; these are not delivery authorization reports. Exact-byte observation drift is implemented; approved-baseline enforcement and full traceability remain planned.

## Detailed documents

- [Architecture and ADRs](docs/architecture.md): system boundary, specification graph, baselines, provenance, trust model, cross-Guard interfaces and acceptance scenarios.
- [Technical design](docs/technical-design.md): Rust modules, schemas, adapter behavior, rule DSL, CLI/MCP/CI interfaces, security, Wave delivery and tests.

## Planned (not executable yet)

~~~sh
specguard doctor --project .
specguard scan --project . --source openspec --format json
specguard trace --requirement REQ-017
specguard diff --base <approved-ref> --head HEAD
~~~

Current native CLI/MCP checks cover supported structure, scoped IDs and exact artifact drift. Later stages will add complete trace graphs, protected baseline enforcement and cross-Guard evidence. Source files remain the project's existing truth; there is no implicit initialization, installation or approval issuance.

See [all Partme Guard repositories](https://github.com/orgs/full-stack-plugins/repositories). Source builds and managed installation are available; no hosted service or signed attestation is claimed.

## Agent installation and managed instructions

The shared installer provides `install`, `uninstall`, `context`, `instructions render|sync`, `doctor --integration`, and `serve --mcp` for Codex, Claude Code, and Gemini CLI. Each tool renders its own managed block.

```sh
cargo build --locked
specguard install --agents codex,claude,gemini --project /absolute/project --bin-dir /absolute/bin --dry-run
specguard context --project /absolute/project
```

After building, invoke the executable under `target/debug/` (with `.exe` on Windows); short commands above assume PATH is configured. `--bin-dir` copies the current executable; PATH is not changed automatically. Remove `--dry-run` to apply. A sibling `../guardengine/crates/guard-install` source checkout is required.

[Shared design, host mapping, update/uninstall and verification](https://github.com/full-stack-plugins/guardengine/blob/main/docs/agent-installation.md). Configuration success does not prove host loading or quality approval.
