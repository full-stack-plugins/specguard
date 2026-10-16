# Read-only native specification sources

This implements the source-adapter portion of **EG-S01 / EG-SPEC-001** in the existing [add-engineering-guard-core](../../codeguard/openspec/changes/add-engineering-guard-core/) change. It does not initialize another specification system or mark the cross-repository task complete.

```sh
specguard specs sources --project /absolute/project
```

MCP: `specguard_spec_sources({"project":"/absolute/project"})`. Both return the same `specguard-sources/v1` observation. Only `project` is accepted; neither interface accepts a source override, an approval flag, installation options or commands from documents. Nothing is written to the inspected project.

| Source marker | Parsed requirements | Native source state |
|---|---|---|
| `openspec/` | Existing CommonMark `### Requirement: NAME` parser, in `openspec/specs/CAPABILITY/spec.md` and `openspec/changes/CHANGE/specs/CAPABILITY/spec.md` | `baseline` or `candidate` is the **file location**, not approval |
| `.specify/` plus `specs/` | Existing `FR-NNN: statement` parser in feature `spec.md` files; bold/list syntax is supported | `approval_unknown`; task checkboxes do not grant approval |
| `docs/superpowers/specs/` | Explicit project convention `REQ-ID: statement` or `FR-ID: statement`, optionally bold/list/heading text; nonempty ASCII alphanumeric, hyphen or underscore suffix | `approval_unknown` |

Superpowers does not impose a universal requirement-ID format. This adapter recognizes the explicit project convention above; a freeform spec with no recognized ID remains **incomplete and requires review**. It does not assign IDs from line numbers, document summaries or an LLM. Fenced/indented code, quotes, images and comments cannot supply requirements. `docs/superpowers/plans/` is inventoried when a Superpowers spec source exists, but its tasks are never requirements. Plans alone do not establish a second specification authority.

Each requirement contains `native_id`, `stable_id`, `system`, `scope`, original `path`/`line`, `operation`, `source_state` and exact artifact SHA256. The report copies no requirement body. A stable ID is:

```text
urn:specguard:requirement:v1:<system>/<scope>/<native-key>
```

Each segment is UTF-8 percent-encoded independently; case and Unicode bytes are preserved. Scope is the OpenSpec capability, Spec Kit feature directory below `specs/`, or Superpowers spec path below its specs root. OpenSpec baseline and delta occurrences of the same capability/native key have the same logical ID and separate provenance. Different features may legitimately reuse `FR-001`. Moving lines or changing body text preserves the ID while changing the artifact digest. Renaming a native requirement key or its logical scope requires an explicit future migration mapping; the adapter never guesses equivalence. OpenSpec uses the complete native requirement heading name as its key, rather than guessing that a word inside the name is an independent ID.

`authority_status` reports `single_source_observed`, `unknown`, `unresolved` or `conflict`. Multiple frameworks are conservatively conflicting until ownership/priority is established externally; this version does not infer that two differently named changes are unrelated. A malformed or unreadable source does not discard observations from independent sources. `selected_source` is always null. A single source only establishes an observation, not authority or approval. Archived OpenSpec changes and templates do not supply active requirements. Removed native requirements retain `operation=removed` and candidate provenance.

Exit 0 means the supported source inventory and native structure were observed without a finding; exit 2 means deterministic structural findings; exit 3 means unknown/conflicting sources or incomplete analysis; exit 4 is invalid CLI/MCP input. Findings remain in the report even when exit 3 takes precedence. Callers must inspect the report rather than treating exit 0 as delivery authorization.

`approved_baseline_eligible=false`, `approval_status=not_evaluated`, `execution_status=not_evaluated` and `trust=local_unverified` are fixed. The adapter cannot issue an approved baseline, sign itself into trust, prove requirement completeness or verify semantic implementation. Requirement/design/task/test graph governance and protected baseline approval remain separate EG-S02/S03/S06 work.

The adapter reuses existing native checkers, the CommonMark projection and GuardEngine's bounded project-file reader. Each source inventory rejects symlinks/reparse points and nonregular files, caps recursion at 12 and directory/file visits at 512, and reads at most 1 MiB per file / 16 MiB per framework. Superpowers ancestor directories are checked before traversal. Static link rejection does not provide an atomic filesystem snapshot against concurrent replacement: use an immutable checkout when a protected controller consumes observations. Reports are deterministic for identical bytes; there is no cache or subprocess execution.

The existing `specs check/verify` (`specguard-native/v1`) and `trace check/verify` APIs are unchanged. An explicit old `--source` is an independent structural observation, never resolution of a conflict in this new report.

## Verification

`tests/spec_sources.rs` executes the real CLI/MCP in isolated filesystem fixtures: three source adapters; line-stable and feature-scoped IDs; escaping/case/Unicode; baseline/delta/removal provenance; ambiguous/unknown/malformed sources; freeform Superpowers; preservation of other independent observations; archive/template exclusion; invalid UTF-8, byte/traversal budgets; Windows ancestor junction rejection; strict MCP input; and CLI/MCP equivalence. Existing native/trace/AGENTS tests continue to exercise their original contracts. Rust 1.85 is used locally; Ubuntu execution is reserved for the repository CI.
