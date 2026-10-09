# SpecGuard implementation ledger

Approved scope: add-specification-baseline-analysis groups 1–2. No task boxes change before root review. Base c1137ba878f03a4df739803e450a5dac186d01b5. Baseline: documentation-only, no source, manifest or executable tests.

## Executable slices

1. Tasks 1.1–1.4: strict versioned DTOs and JSON schemas; read-only explicit roots/files; byte-digest freeze with real Git object validation; Markdown explicit-ID and OpenSpec explicit-ID profiles; bounded parsing and per-source terminal coverage. Tests: model_contract, source_discovery, source_snapshot, parser_coverage. Prerequisite: Rust, serde, serde_json, sha2, serde_yaml.
2. Tasks 1.5–1.7 and 2.5: deterministic graph retaining duplicates, typed edges, validated migrations, frozen structural checks, bounded reverse impact. Tests: graph_identity, graph_migration, structural_rules, impact_paths. No engine dependency.
3. Tasks 2.1–2.4: immutable baseline binding, read-only approval port with explicit fixture profile, conservative diff and typed integer bounds. Tests: baseline_contract, approval_validation, baseline_diff, condition_comparison. Production approval provider deferred to GE-TRUST.
4. Tasks 2.6–2.7: versioned obligation export and golden local handoff; tests check strict consumer deserialization, IDs, digests and incomplete coverage. Actual TestGuard/ArchGuard consumption requires their owners; do not label simulated consumer as real interoperability.

Choices: local domain version specguard.domain/v1alpha1 (not engine canonicalization); fixed Serde struct field order, BTreeMap/ordered vectors and SHA-256. Source profiles markdown-explicit/v1 and openspec-explicit/v1 (narrow explicit-ID extension, no general OpenSpec compatibility claim). No network or document execution. No persistent cache. Budgets are explicit configuration. Baseline effective times are UTC epoch seconds supplied by caller/controller. Only integer min/max comparisons with identical units have ordering; free text requires review.

Approval provider, trust policy and storage remain external decisions. Groups 3–4 are outside this slice and wait for frozen engine interfaces where required.

## Slice outcome (review pending)

Local Rust library and fixtures implemented, 28 tests pass; strict Clippy and schema-fixture checks pass. All OpenSpec task boxes remain unchecked. Task-level evidence and limits are in /workspace/guard-implementation-ledger/specguard-slice1-report.md.

Local acceptance implemented pending review: 1.1, 1.2, 1.6, 1.7 (supported dependency domain), 2.1, 2.2 (explicit fixture port), 2.3, 2.4, 2.5, 2.6. Partial tasks: 1.3 lacks OS-atomic hostile-mutation protection; 1.4 uses an explicit-ID line grammar rather than full Markdown AST/native upstream OpenSpec; 1.5 reserves ADR/task tags but rejects their analysis as incomplete; 2.7 has golden producer/local simulation, actual consumer verification being coordinated with TestGuard. All production approval use remains unavailable.

Brief pause for parent scope synchronization was followed by explicit instruction to continue implementation; no work was discarded. No engine integration or release claims are made.
