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

## GE producer slice plan (tasks 3.1–3.3)

1. Add protected exact-relation mappings with a borrowed-size preflight, complete deterministic-kind coverage for frozen requirement IDs, and GE FactBudget source-fragment construction. Reject unknown/unmapped/duplicate/unused mappings and enforcement mismatches before reporting success.
2. Prepare a GE BoundAttempt only after validating snapshot integrity, actual Git commit objects, immutable invocation IDs and frozen source/requirement scope. Copy caller-owned source/policy only after bounded serialization preflight. Pre-binding failures remain transport diagnostics.
3. Run the existing parser and graph rules into real evaluate_bounded results. Return completed ALLOW/BLOCK, completed partial BLOCK, or error/cancelled null-decision with no report. Produce actual contract/facts/report/domain bytes and raw-byte SHA-256 references, verify with GE verifier, and test tamper/status/digest failures. This local library producer does not authenticate a controller or publish filesystem outputs.
4. Full native suite, strict checks and real consumer regressions; new 3.1–3.3 remain unchecked until independent review. No parser reduction or legacy wire changes.


### GE producer implementation checkpoint

Root independently accepted 1.3 at 0b81763 for Unix frozen-inventory capture, without an OS-atomic claim: **12/30 accepted**. New tasks 3.1–3.3 remain unchecked pending independent review.

`integration::producer::prepare` now freezes bounded protected mappings, required IDs/source/profile scopes, actual Git commit identities and a digest-checked source snapshot before analysis. A consuming PreparedRun runs the existing parser/structural validator, uses GE FactBudget with source fragments before concatenation, evaluates through evaluate_bounded, and returns an actual envelope plus contract/facts/report/domain bytes. Missing mappings are post-binding errors; malformed/unsupported coverage produces a completed partial BLOCK; I/O failure, explicit fail and cancel produce null decisions without old report payloads. Policy has a separate controller-owned input, never read from candidate content. No approval is manufactured. Complete scope means the required checks ran, including a definite missing requirement; it does not mean those checks passed.

Preflight limits: 16 MiB total borrowed serialized invocation/source/mapping before copies; at most 64 required identities, 256 exact enforced mappings/rules, 4096 graph nodes/edges and source statuses. Mapping tuples are unique, one-to-one with contract rules, and cover every deterministic finding kind for every required identity. Extra findings without exact mappings fail closed. Known partials use GE partial facts and cannot ALLOW. Artifact verification checks GE recomputation, raw-byte digests, domain version/profile/binding, frozen scope, recomputed findings and exact projected facts. Contract bytes are JSON (a valid YAML representation), preserving strict GE contract shape.

This is a real local library producer, not hosted trust, a CLI publisher, cancellation of an already executing thread, CAS storage or clean queue-tree proof. Caller-supplied finish timestamps must satisfy GE's UTC chronology contract; malformed completion metadata returns GE transport diagnostics. Actual Git object existence is verified, while dirty snapshot bytes remain separately bound. Public failure/cancellation entrypoints represent controller execution outcomes and never claim execution succeeded. URI references identify output bytes for an authorized storage adapter; this library performs no storage writes.

### Producer review: parser allocation admission

Independent review reproduced path-to-node allocation amplification before the former post-parse graph check: a 130,943-byte source at a 2,896-byte real filesystem path with 6,000 requirement headings allocated an extra peak 18,905,096 bytes before returning an error. The producer now performs a borrowed-byte admission estimate before invoking the existing parser: aggregate line count (all lines conservatively treated as potential nodes), path/namespace/typed-node sizes with vector/copy headroom, and text/registry/AST byte headroom. Over-budget input returns a bound `parser.budget` error with no decision/report. This intentionally rejects some otherwise valid large inputs; it is an admission estimate, not an OS/global allocator cap. The accepted native parser and source formats are unchanged. The independent allocation probe is preserved as `tests/producer_budget.rs` and must pass before committing the correction.

## Task 1.5 trace source/export plan

Add real file adapters `markdown-adr/v1` and `markdown-task/v1` with explicit namespace/ID headers and AST-based source positions, plus typed trace list directives in already-supported requirement documents. `trace::scan` composes the existing parser with target parsing, preserving full-source coverage; no native parser replacement. Export a strict separate `specguard.trace/v1alpha1` artifact. Add read-only stdout-only `trace-check` and `trace-export` commands using explicit root/policy/binding/required inputs; incomplete sources or invalid relations cannot return successful check status. Test real temp repositories, duplicates, dangling/wrong-kind references, frozen source identity, code fences, unsupported versions, deterministic order and actual command invocation.

### Trace delivery checkpoint / current acceptance

Root accepted producer 3.1/3.2 at the independently reviewed pins, with the allocation correction 10fbe7b isolated: **14/30 accepted**. Task 3.3 remains partial for real crash/mid-run cancellation and invalid post-binding completion metadata; its checkbox is unchanged.

Task 1.5 now has actual versioned Markdown ADR/task source adapters, typed relation extraction, strict separate trace artifact/schema, generated real-source golden, and runnable read-only trace-check/trace-export CLI with 0/2/4 exits. Fenced/quoted examples are not relations; unsupported reserved trace directives fail closed. Existing native OpenSpec parser and baseline/obligation schemas remain unchanged. This is the documented explicit-ID file convention only; no arbitrary ADR/issue tracker compatibility or authenticated gate claim. Task 1.5 stays unchecked until independent review. Budget admission is shared with the reviewed producer correction and runs before copies/parser allocation.

Trace final guard: a behavioral regression exposed deeply nested target prose bypassing max_depth. Both new target/trace-list AST walks now enforce configured depth, event count and elapsed-time budgets; limit coverage remains incomplete. Final native+producer+trace suite: 77/77, warning-free Clippy, fmt, all four schema fixture checks and strict OpenSpec pass. Trace source schema/CLI changes remain pending independent review.

## Task 3.3 supervised library execution plan

Add a consuming cooperative runtime API around prepared work: cancel token checked between real source parses and before finalization, a bounded source-progress observer, panic-unwind recovery and a frozen recovery receipt. Preserve existing parse()/complete() signatures. Post-binding invalid finish metadata and actual final verification failures must recover to valid error/null envelopes with bounded diagnostic artifacts and no old report. Test cancellation after a source really parsed, actual panic in a worker's observer callback, original source diagnostics in recovery artifacts, and real verifier rejection of corrupted bytes. This is a panic=unwind library profile, not containment of process abort/OOM/SIGKILL; no arbitrary shell or new provider. File publication remains task 4.1, not part of this slice.

Runtime checkpoint: root independently accepted 1.5 at b1c7e2b; current acceptance is **15/30**, with 3.3 still unchecked pending review. The supervised library profile uses a shared cancellation token at source and pipeline checkpoints, catches real panic unwinds (including bounded trusted observer callbacks), and preserves a private frozen recovery receipt. Invalid final metadata and actual verifier rejection now yield legal error/null envelopes with no contract/facts/report payloads. A separate digest-bound runtime diagnostic artifact retains bounded original source records, original transport/verifier errors and a truncated panic payload; overflow records are explicitly counted. Invalid finish time uses the validated invocation timestamp as a marked recovery sentinel (`finishTimeFallback`), never as a claim of actual completion time or fresh successful evidence. Older failure bundles without a diagnostic artifact remain verifiable.

The cancellation test runs a real worker thread, waits until source a.md has actually parsed, cancels from another thread, and proves b.md was not parsed. The panic test raises an actual observer panic after an actual malformed source diagnostic; the worker returns a bound error and retains the diagnostic. The actual GE-verification rejection test corrupts real golden facts bytes and confirms recovery rather than a completed result. These are not claims of process-abort/OOM/SIGKILL recovery or forced interruption inside a source parser/observer; that requires an external host. The native parser changes only add private checkpoints around existing source parsing, preserving its public parse signature and grammar. No shell, publication or provider is introduced.

## Architecture handoff checkpoint

Root independently accepted runtime 3.3 at fdca1aa for the cooperative panic=unwind library profile: **16/30 accepted**. Task 2.7 remains unchecked. The new architecture handoff exports actual frozen requirement/ADR source references with immutable baseline and exact candidate/source/scope, re-parses embedded bytes on decode and requires independently supplied caller pins. The only authentication profile is fixture-only; production approval is unavailable. ArchGuard owns its separate reader and protected architecture mappings. Actual consumer execution, golden provenance and remaining trust limits are recorded in the external handoff report.

## Read-only CLI checkpoint (4.1 partial)

Actual doctor/scan/trace/diff/check positional commands now use existing source, graph and runtime APIs. Legacy trace-check/trace-export and their exit semantics are preserved. Check has real0/2/4 (including cancelled4 and completed partial BLOCK2); Enforce-only structural policy cannot generate REQUIRE_APPROVAL and remains unchanged. Diff requires explicit unverified baseline mode; no approval authority is claimed. All commands are stdout-only, and --report explicitly rejects without modifying old output or source paths. Current process/binding must determine result, never old file existence. See docs/cli.md and tests/cli_contract.rs. Tasks4.1/4.2 remain unchecked; full decision3/authenticated diff/publication and further resource qualification are outstanding. Both READMEs now distinguish this implementation from the historical documentation-only main inventory.

CLI verification:95/95 full Rust tests, warning-free Clippy, fmt, all five schema fixture checks and strict OpenSpec pass. Tests spawn actual binary processes, reconstruct stdout raw-byte bundles and verify through the real GE/domain verifier. Coverage includes0/2/4, partial BLOCK, cancel/null, invalid finish bound error, pre-binding bad candidate/version/field, oversized/symlink configuration, old report preservation and legacy trace regression. No actual exit3, SIGINT adaptation or atomic report publication is claimed.

## Actual Git source binding checkpoint

Root independently accepted task3.6 at271aa0b for bounded process-local typed-completion history: **17/30 accepted**. New3.5 remains unchecked pending review. A separate GitPreparedRun reuses reviewed GitGuard Repository/read_commit_files and the existing native SG parser/producer over privately materialized actual committed bytes. It exposes exact SG binding before execution for independent controller expectations, preserves SG source digest separately from GG/file digests, and reanalyzes real candidate bytes during bundle verification. Standard preparation allows explicit advisory dirty-worktree observations; prepare_clean requires GG-observed candidate/worktree/index equality, not queue authority. Baseline authentication remains unavailable and GG profile currently supports no baseline. The workspace MSRV is explicitly1.90, with real1.90 tests; no old-MSRV claim. See docs/git-source-binding.md.

## Accepted exact Git source binding:18/30

Independent e0c75597 review accepts task3.5 for the declared local exact-candidate profile:109 tests plus an independent cancel/seven-dimensional binding mutation probe. Actual SHA1/SHA256, synthetic queue candidate, dirty old HEAD isolation and complete source reanalysis reject self-consistent forged domain artifacts. Clean gate consumers must explicitly use prepare_clean; ordinary prepare accepts clean=false only as advisory. Baseline comparison does not establish baseline authority, and GitGuard currently rejects non-null baseline in this source path. No production identity or hosted queue qualification. Evidence: cloud ledger specguard-git-binding-independent-review.md.


## Accepted actual architecture handoff: 19/30

Root accepted task 2.7 after independent review of ArchGuard `1d6d1e304e7cf403c0df311098c461429279c6e4` consuming SpecGuard `e0c75597b81f5c8045902683b71d3bec0b87731d`. Actual committed Cargo/Markdown input is frozen/exported by SG, decoded by AG and checked against GitGuard object bytes before real GuardEngine execution. External evidence: `archguard-sg-independent-review.md`, `archguard-sg-slice-report.md`, and `archguard-sg-golden/` in the implementation ledger. Shared historical baseline obligation IDs are preserved. This accepts the local fixture authentication profile only, not production authorization, historical Git provenance, or candidate TestGuard execution. Task 4.2 is currently under implementation and remains unchecked.


## Source/parser resource limits (4.2 pending review)

The [source safety profile](security-limits.md) now applies hard policy/metadata/path and aggregate byte admission, routes public parser calls through the established construction preflight, checks freeze/last-file/explicit-parser elapsed time, and reuses GitGuard's bounded Git object verifier. This intentionally rejects previously oversized configurations and unsupported Git storage rather than weakening source protection. Native grammar, trace identities, and exact candidate-tree binding remain intact. Independent acceptance is still pending; 19/30 accepted is unchanged.


## Reviewed resource-limit acceptance

Task 4.2 accepted at `bedcbf7a6d413ae9dc75638892902a92e91988a3` for the documented bounded/cooperative local profile. Independent review closed the repeated-root diagnostic-expansion P2: the unchanged 300-root probe falls from 90,001 statuses/25,195,115 bytes to 1,996 statuses/553,755 bytes with explicit Limit/unknown remaining scope. One global traversal counter and conservative 16MiB metadata accounting cover borrowed identities, rejected sources and pending paths before allocation. All 14 focused security tests passed on Rust1.90, including broad/deep pending-path admission; prior independent review passed 118 maintained tests and two doc tests. No global OS RSS/deadline guarantee, hostile-source sandbox or production authority is claimed. Evidence: cloud ledger `specguard-global-independent-review.md` and original `specguard-security-independent-review.md`.


## Reviewed optional parse cache

Task 4.3 accepted at `66498f2a849f4ef56208d5d253288a5e604e1ca7`. Independent fixed-source Rust1.90 regression passed134 committed tests plus a separate cross-controller-scope/hot-panic probe. The disabled-by-default cache retains only private complete parse results with full source/policy/coverage/baseline/version keys and bounded count/charge. Failed/cancelled/partial finalization cannot insert new entries; every run rebuilds graph, facts, GE report and envelope. Real Git preparation and separate fresh approval checks remain required. No eligibility/envelope/authority result is cached. Evidence: cloud ledger `specguard-cache-independent-review.md`; no production cache, RSS or timing guarantee is claimed.
