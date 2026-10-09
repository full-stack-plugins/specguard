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


## Independent-review corrections

The independent review accepted local tasks 1.1, 1.2, 1.7, 2.1, 2.2 and 2.5, with checkboxes still deferred. Root accepted the reversible same-unit i64 min/max contract for 2.4; supported units are explicitly count/ms/s/bytes, all unknown units and free-text comparisons require review.

Regression fixes: changed same-ID acceptance text makes export incomplete; parsing verifies the aggregate snapshot digest before interpreting bytes; mappings reject a source ID retained in the candidate. Each regression failed before its fix. Parser fixtures now use a correctly frozen policy/profile; exercising the existing OpenSpec fixture also fixed optional ADDED Requirements section handling. Full suite: 33 passing tests; strict Clippy and schema checks pass. No new source-format/atomicity implementation claimed in this correction slice.

Actual TestGuard local-fixture consumer evidence received and independently rerun: repository head fe85e3d96759e323da8cd5f94afc3a29bdd6d661; cargo test --locked --offline --test specguard_fixture, 2 passing tests. This consumes the exact exported golden fixture, retains source metadata, freezes explicit case/environment mappings, and rejects partial/production/unknown-version/field/missing-scope/candidate-drift inputs. Production authentication and ArchGuard native consumption are still unverified.

## Independent review acceptance

Local full task acceptance: 1.1, 1.2, 1.6, 1.7, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6. Review findings were fixed with RED/GREEN regressions and independently rechecked; see cloud execution ledger specguard-review.md. Scope remains local and advisory/fixture-labelled where stated. Production authority, remaining capability gaps and hosted gates are not claimed.

## Native OpenSpec slice plan (base 2759b3b)

Task 1.4: add parser dispatch for actual installed OpenSpec 1.14.1 main specs and ADDED-only delta specs, with Markdown AST parsing and explicit per-source incomplete diagnostics. Keep the old explicit-ID parser and existing domain wire payload unchanged. Verify native fixture files with the installed official Validator/MarkdownParser, then test real discovery → frozen registry+documents → parsing, stable IDs, fence/nesting limits, malformed/missing identity coverage and unknown/native delta operations. Record official version and commands, local RED/GREEN, full Rust regression and TG export compatibility.

Ruling: natural-language native headings are lookup locators, not persistent IDs — native source IDs come from an explicit specguard.openspec-ids/v1 registry discovered and digested alongside documents; without complete unique mappings native parsing is incomplete. This preserves the stable-ID requirement without editing native source format. Cost if wrong: registry/profile migration, not hidden title-derived identity merges.

Pre-flight 1.4→1.5/2.6: unchanged Requirement/Acceptance/TraceEdge/ParseResult/ObligationSet shapes keep existing consumers and old fixture digests compatible. New source profile names are opt-in. 1.5 ADR/task target schema work and 3.1/3.2 GE adapter work remain subsequent slices, not silently advertised. 1.3 filesystem atomicity remains an explicit gap.

Ruling: official main-spec and ADDED-delta capability support is pinned to installed @fission-ai/openspec 1.14.1; MODIFIED/REMOVED/RENAMED deltas require baseline-aware application and will report Unsupported, not be treated as additions. No full-tool/N-1 compatibility claim.


Native slice outcome (review pending): 44 Rust tests pass (11 native tests), strict Clippy/schema checks pass, and official @fission-ai/openspec 1.14.1 strict validation passes main/ADDED fixtures plus BOM/CRLF/closed-heading/fenced-example parity. Native identity registry schema is separate; all existing domain DTOs, handoff golden bytes and fixture-only authentication behavior are unchanged. Two additional RED→GREEN regressions enforce normalized lone-CR line limits and fail closed on unsupported HTML structures. Task 1.4 native capability evidence is ready for independent review; task checkboxes unchanged. Main/ADDED-only support is explicit, not full delta-application support. The rulings above and remaining 1.3/1.5/GE work stand.

Final source-reader comparison added two RED→GREEN cases: bold prose containing a later `**:` is preserved rather than dropped as metadata; indented code (whose official reader treatment differs from the AST) is explicitly Unsupported. Full native feature commit d6f4e6c plus this correction remain one reviewable task-1.4 slice, not a broader capability claim.

## Native hierarchy review correction (base b64aeff)

Independent review found an upstream-valid H5 scenario directly below an H3 requirement was omitted while coverage stayed complete. TDD reproduces this plus the same omission for skipped H4/H5 requirement nodes. The native reader now validates heading-parent relationships before any nodes are emitted: section→H3 requirement→H4 scenario; skipped direct-child levels return Unsupported. Deeper scenario-body headings remain raw text, and fenced/quoted headings remain data. Missing IDs still make canonical extra H4 scenarios incomplete; supplying IDs does not enable unsupported hierarchy.

Validation: 47 Rust tests pass (14 native test functions including depth/placement/fence/quote matrices); strict Clippy and schema checks pass. Official OpenSpec 1.14.1 validates 26 hierarchy variants and demonstrates the extra scenarios/requirements in skipped-level cases. Original public-API probe now reports deeper-scenario complete=false/acceptances=0, while the quoted control remains complete=true/acceptances=2. Actual TestGuard fixture consumer 2/2 and FlowGuard baseline consumer 1/1 pass. Checkboxes unchanged pending re-review; other native/atomicity/production limitations remain.

## Snapshot/graph continuation (base 7deaf72)

Task 1.3: TDD root/ancestor symlinks, stable descriptor reads and same-byte file replacement, dirty owned bytes, invalid encoding and actual SHA-1/SHA-256 commits. Keep source DTOs and parser behavior compatible. Task 1.5 follows: typed ADR/task target identities and full duplicate/broken-source diagnostics without weakening the native OpenSpec parser or changing existing consumer wire fields. New work stays unchecked pending independent review; root authorized recording prior native parser 1.4 acceptance (11/30 total).


### Current continuation checkpoint

Root authorized recording task 1.4's independent acceptance at 7deaf72: **11/30 accepted**. Tasks 1.3 and 1.5 remain unchecked pending review; their new implementation does not re-grade the accepted parser.

Task 1.3 now uses Unix descriptor-relative component opens with O_NOFOLLOW/O_DIRECTORY and O_NONBLOCK; root and ancestor symlinks and special files are rejected. Captures compare device/inode/ctime/mtime/length across read and verification, plus bytes against inventory digests. Identical-byte inode replacement and mutate/restore are detected on this host. Non-Unix capture explicitly returns unsupported rather than using a weaker path walk. The libc pin is shared-compatible 0.2.177. Duplicate inventory paths and inventory file-budget overflow fail closed. Frozen invalid UTF-8 remains owned bytes and the unchanged parser emits malformed coverage. Real full SHA-1/SHA-256 commit validation remains mandatory; dirty bytes change the snapshot digest without fabricating a Git OID.

This provides detected-drift rejection and owned immutable parser input, **not an OS-atomic multi-file snapshot transaction**. Filesystem timestamp granularity and privileged mutation remain limits; an authenticated protected gate still needs an immutable checkout/snapshot and task 3.5 candidate-tree binding. Discovery traversal is not an OS sandbox. Additions outside the frozen inventory are not automatically new obligations. No full hostile-filesystem atomicity claim is made.

Task 1.5 adds a separate in-memory TypedSpecificationGraph/TraceTargets API for explicit stable ADR/task IDs, provenance, typed relation direction/targets, preserved duplicate locations and deterministic ordering. Incomplete target source coverage prevents definite missing-target claims. Existing ParseResult/SpecificationGraph serialization, source parser and TG/FG APIs remain unchanged. No native ADR/task source format, serialized target artifact or production trace provider is claimed; callers must supply the explicit inventory and per-source coverage. Legacy graphs continue to fail closed for unsupported trace relations.

Full verification: 57 Rust tests, warning-free Clippy; see external specguard-source-graph-report.md for actual commands, RED/GREEN limits and consumer checks.
