# SpecGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**Deterministic requirements, specification and acceptance governance for AI-native software engineering.**

> **Documentation-only design.** Inspected `main` commit: `01137804aa465c1b931c73c9d211ff0d29b46c7a` (2026-10-09 review). The tracked tree contains two READMEs and two design documents; it has no source, package manifest, tests, schemas, CI configuration or OpenSpec workspace. All SpecGuard capabilities and commands below are proposals, not implemented features. The documents define a direction, not a completed or validated implementation.

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

Inputs are explicitly selected specification sources, a candidate snapshot, protected policies and, when required, a baseline plus externally authenticated approval records. Planned outputs are a specification graph, located findings, baseline differences, coverage and engine-compatible facts/report references. A test obligation is a requirement to obtain evidence, not proof that a behavioral test passed.

SpecGuard owns specification parsing and domain checks. ArchGuard owns architecture checks; CodeGuard owns code checks; TestGuard owns test evidence; GitGuard owns Git checks; FlowGuard coordinates gates and trusted approval verification. The six guards independently use [GuardEngine](https://github.com/full-stack-plugins/guardengine), which owns general contract validation, neutral rule evaluation and deterministic evidence computation. Neither SpecGuard nor the engine grants product approval, merge authority or release authority.

## Decisions, coverage and compatibility

Deterministic violations may be mapped to `enforce`; semantic uncertainty requires `review`; suggestions use `advise`. Engine decisions are `ALLOW`, `BLOCK` and `REQUIRE_APPROVAL`. Partial facts mean `BLOCK` with `INDETERMINATE` evaluation, never a successful complete check. “Complete” describes the declared analyzer scope, not full business understanding. Approval cannot override failed or incomplete analysis.

The current shared `guard.partme.ai/v1alpha1` protocol supports only GuardContract YAML, GuardFacts JSON, GuardReport JSON and exact `forbid_relation` assertions. Unknown fields are rejected. Quantified graph checks belong in SpecGuard's proposed domain validator; source locations, approvals and orchestration metadata must not be invented as extra engine fields. Reports are unsigned; verification recomputes results and does not establish trust or authorization. This is the shared engine compatibility baseline, not a locally verified SpecGuard integration.

The proposed [integration contract](docs/integration-contract.md) is a separate `GuardRunEnvelope` (`guard.integration/v1alpha1`, **DRAFT**), not an extension accepted by the current engine. Results must bind to the precise candidate/base, task and baseline; changed bindings invalidate evidence. Trusted CI must recheck the exact merge-queue candidate.

## Planned interfaces — not executable

~~~sh
specguard doctor --project .
specguard scan --project . --source openspec --format json
specguard trace --requirement REQ-017 --format json
specguard diff --base <approved-ref> --head HEAD
specguard check --project . --format json
~~~

These commands have no binary or installer today. `check` targets exit codes `0` ALLOW, `2` BLOCK, `3` REQUIRE_APPROVAL, `4` invalid input/runtime/verification error; other commands' exact exit contracts remain to be specified. Proposed machine-readable output goes to stdout and diagnostics to stderr. No implemented `--report` flag exists. MCP and CI are later interfaces, also unimplemented.

Adapters for OpenSpec, Spec Kit, Superpowers and any historical spec workflow plugin are **unverified compatibility targets**. No external plugin installation or successful interoperability test is implied. Reading a project must not initialize tools, fetch dependencies, issue approvals or modify its specifications.

## Delivery and documentation

The sequence is read-only discovery (S0), graph/identity checks (S1), protected baseline comparison (S2), cross-guard evidence (S3), then stabilized CLI/MCP/CI and cache compatibility (S4). Each enforced rule needs valid, violation, tool-failure and incomplete-coverage fixtures. No runtime tests or OpenSpec validation were run: neither exists in the inspected tree.

- [Architecture and ADRs](docs/architecture.md): boundaries, graph, baseline and run states, trust, integration and scenarios.
- [Technical design](docs/technical-design.md): proposed modules, DTOs, algorithms, interfaces, diagnostics and measurable acceptance.
- [Draft shared integration contract](docs/integration-contract.md): orchestration bindings separate from the engine wire protocol.

Outstanding choices include supported source-format versions, stable-ID migration policy, approval provider, digest/schema implementation and measured resource budgets. No binary, hosted service or signed attestation is claimed. See [the Guard repositories](https://github.com/orgs/full-stack-plugins/repositories).
