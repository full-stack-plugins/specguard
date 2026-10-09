# SpecGuard

[English](README.md) · [简体中文](README.zh-CN.md)

**Deterministic requirements, specification and acceptance governance for AI-native software engineering.**

> **Status: architecture and technical design are complete as documents; runtime implementation has not started.** Planned commands in this README are not yet available.

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

SpecGuard reuses [GuardEngine](https://github.com/full-stack-plugins/guardengine) Guard Protocol. The current v1alpha1 engine supports only exact forbidden-relation assertions; planned collection and traceability checks require SpecGuard-specific validation or a versioned protocol extension.

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

The first implementation will deliver read-only source discovery and exact graph/ID checks. Later stages will add protected baseline enforcement, cross-Guard evidence and MCP/CI integration. Source files must remain the project's existing truth; there is no implicit initialization, installation or approval issuance.

See [all Partme Guard repositories](https://github.com/orgs/full-stack-plugins/repositories). No binary, installer, hosted service or signed attestation is claimed for SpecGuard at this point.
