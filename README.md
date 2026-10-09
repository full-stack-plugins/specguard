# SpecGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**Deterministic requirements, specification and acceptance governance for AI-native software engineering.**

> **Local implementation, limited trust profiles.** This branch contains Rust source, a runnable read-only CLI, schemas and tests. Independently accepted work is tracked in [tasks](openspec/changes/add-specification-baseline-analysis/tasks.md) and [implementation evidence](docs/implementation-progress.md). Historical `main` commit `01137804aa465c1b931c73c9d211ff0d29b46c7a` was documentation-only; that inventory does not describe this implementation branch. Production approval authentication, hosted enforcement and release remain unavailable.

## Why SpecGuard

AI can generate requirements and tasks quickly without proving that the intended scope is correct. SpecGuard is designed to check stable identities, traceability and changes to approved acceptance obligations. Ambiguous business intent remains a human review concern; natural-language scores cannot supply enforcement evidence.

~~~text
Original request / OpenSpec / Spec Kit / Superpowers / declared Markdown
                              ↓
                Read-only adapters + declared coverage
                              ↓
 Requirement → Acceptance → Design / Task → Test obligation
                              ↓
              Structural validation + approved baseline diff
                              ↓
             Domain findings → GuardFacts → GuardEngine
                              ↓
             Scoped decision + evidence → trusted CI / FlowGuard
~~~

## Scope and useful scenarios

- Find duplicate requirement IDs, broken references and missing mandatory acceptance links before implementation.
- Compare a candidate with an immutable approved baseline; detect removal or structured weakening of an acceptance obligation.
- Identify architecture, task and test obligations affected by specification changes, and invalidate stale evidence.
- Explain exactly what was analyzed, which sources remain unreadable and which questions require review.

Inputs are explicitly selected specification sources, a candidate snapshot, protected policies and, when required, a baseline plus externally authenticated approval records. Current local outputs include a specification graph, located findings, baseline differences, coverage and engine-compatible facts/report references. A test obligation is a requirement to obtain evidence, not proof that a behavioral test passed.

SpecGuard owns specification parsing and domain checks. ArchGuard owns architecture checks; CodeGuard owns code checks; TestGuard owns test evidence; GitGuard owns Git checks; FlowGuard coordinates gates and trusted approval verification. The six guards independently use [GuardEngine](https://github.com/full-stack-plugins/guardengine), which owns general contract validation, neutral rule evaluation and deterministic evidence computation. Neither SpecGuard nor the engine grants product approval, merge authority or release authority.

## Decisions, coverage and compatibility

Deterministic violations may be mapped to `enforce`; semantic uncertainty requires `review`; suggestions use `advise`. Engine decisions are `ALLOW`, `BLOCK` and `REQUIRE_APPROVAL`. Partial facts mean `BLOCK` with `INDETERMINATE` evaluation, never a successful complete check. “Complete” describes the declared analyzer scope, not full business understanding. Approval cannot override failed or incomplete analysis.

The current shared `guard.partme.ai/v1alpha1` protocol supports only GuardContract YAML, GuardFacts JSON, GuardReport JSON and exact `forbid_relation` assertions. Unknown fields are rejected. Quantified graph checks belong in SpecGuard's domain validator; source locations, approvals and orchestration metadata must not be invented as extra engine fields. Reports are unsigned; verification recomputes results and does not establish trust or authorization. The local producer generates and verifies actual GE artifacts; see [frozen compatibility capabilities](docs/frozen-integration-capabilities.md). This does not establish producer authority.

The [integration contract](docs/integration-contract.md) uses a separate `GuardRunEnvelope` (`guard.integration/v1alpha1`), implemented locally without adding fields to the engine contract/facts/report objects. Results must bind to the precise candidate/base, task and baseline; changed bindings invalidate evidence. Trusted CI must recheck the exact merge-queue candidate.

## Current local interfaces

Build with `cargo build --locked --bin specguard`; inspect `cargo run --locked -- --help`.

~~~sh
specguard doctor ROOT POLICY.json
specguard scan ROOT POLICY.json BINDING.json REQUIRED.json
specguard trace ROOT POLICY.json BINDING.json REQUIRED.json
specguard diff ROOT POLICY.json BINDING.json BASELINE.json --unverified-baseline
specguard check ROOT REQUEST.json [--cancel]
~~~

See the exact [CLI contract](docs/cli.md) and executable [CLI tests](tests/cli_contract.rs). Legacy trace-check/trace-export remain supported. Current check produces actual0 ALLOW,2 BLOCK and4 error/cancelled, with JSON stdout and diagnostics stderr. The Enforce-only structural producer cannot produce REQUIRE_APPROVAL; exit3 remains reserved, not demonstrated. Diff explicitly lacks approval authentication. `--report` is rejected and no path is modified. Prior output files do not represent a new successful run.

Explicit Markdown and pinned OpenSpec source versions are implemented and tested; see [compatibility evidence](docs/implementation-progress.md). Spec Kit, Superpowers and other external plugin adapters are unverified targets. The flag-based command examples in the original technical design remain proposals, not current syntax. MCP, authenticated gate usage and released installation remain future work. Reading sources does not install tools, fetch dependencies, execute document commands or issue approvals.

## Delivery and documentation

The sequence is read-only discovery (S0), graph/identity checks (S1), protected baseline comparison (S2), cross-guard evidence (S3), then stabilized CLI/MCP/CI and cache compatibility (S4). Each enforced rule needs valid, violation, tool-failure and incomplete-coverage fixtures. Current local tests include native parsing, snapshots, domain/GE artifacts, runtime cancellation and CLI execution; evidence is recorded in [implementation progress](docs/implementation-progress.md).

- [Architecture and ADRs](docs/architecture.md): boundaries, graph, baseline and run states, trust, integration and scenarios.
- [Technical design](docs/technical-design.md): proposed modules, DTOs, algorithms, interfaces, diagnostics and measurable acceptance.
- [Draft shared integration contract](docs/integration-contract.md): orchestration bindings separate from the engine wire protocol.

Remaining work includes a real approval provider, authenticated candidate/gate integration, complete CLI decision/publication support, further resource qualification, MCP and release. The local binary does not provide hosted service or signed attestation. See [the Guard repositories](https://github.com/orgs/full-stack-plugins/repositories).


## OpenSpec implementation backlog

The incremental [proposal](openspec/changes/add-specification-baseline-analysis/proposal.md), [design](openspec/changes/add-specification-baseline-analysis/design.md), [requirements](openspec/changes/add-specification-baseline-analysis/specs/) and [tasks](openspec/changes/add-specification-baseline-analysis/tasks.md) translate the architecture into pending implementation work. See the [cross-repository dependency roadmap](openspec/guard-roadmap.md) and [structural validation record](openspec/validation-2026-10-09.md). Acceptance is recorded per task after independent review; the current checkpoint is18/30. Newly implemented handoff/CLI slices remain unchecked until reviewed, and incomplete capabilities stay partial. Historical inventory and validation records remain linked as historical evidence rather than current capability claims.

Local Git-backed SDK: [exact committed-source binding](docs/git-source-binding.md). The current workspace now explicitly requires Rust1.90 and the reviewed Unix GitGuard dependency; Rust1.90 library/Git-path tests were actually run. Existing CLI remains the local advisory profile.
