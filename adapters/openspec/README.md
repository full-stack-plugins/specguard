# openspec-explicit/v1

An explicit-ID subset using `### Requirement: ID` and `#### Scenario: ID`, optional `## ADDED Requirements`, and YAML frontmatter `{format: openspec-explicit/v1, namespace: demo}`. The parser otherwise follows markdown-explicit/v1. This is a SpecGuard profile, not a claim of compatibility with all upstream OpenSpec versions or existing natural-language requirement headings. No IDs are inferred from titles/line numbers. Other source tools/versions remain unsupported.

The checked fixture is fixtures/source-versions/openspec-explicit-v1.md, exercised by parser_coverage. A full native upstream OpenSpec adapter remains a separate task acceptance gap.

## Native OpenSpec 1.14.1

Two new opt-in source profiles parse native Markdown without injected YAML, IDs or SpecGuard headings:

- `openspec/1.14.1-main`: canonical `## Purpose`, `## Requirements`, `### Requirement: natural title`, `#### Scenario: natural title`.
- `openspec/1.14.1-added`: native `## ADDED Requirements` deltas with complete requirement/scenario bodies. This is an added-facts capability, not baseline-aware application of all delta operations.

The Rust adapter uses pulldown-cmark 0.13.0 AST events and a bounded native-section reader. The default per-file bounds are 1 MiB, 10,000 normalized lines, nesting depth 16, at most 32 × maxLines AST events, and the existing advisory time budget. Fenced code cannot manufacture requirements/scenarios; BOM/CRLF/lone-CR normalization and closing ATX hashes are handled. Requirement body and scenario text for the checked corpus match the installed official OpenSpec 1.14.1 parser. Existing explicit-ID profiles still use their original grammar.

Native title strings are lookup locators, never generated permanent identities. Include a separately configured source file with format `openspec-identities/v1`, same namespace/authority, and `specguard.openspec-ids/v1` JSON (example: fixtures/source-versions/openspec-1.14.1/identities.json). The registry has documents identified by exact frozen relative path, explicit stable requirement IDs and explicit scenario IDs. Discovery/freeze includes its bytes and format in the same snapshot digest. Missing, duplicate, stale or out-of-scope identity mappings prevent complete parsing. Changing headings or moving files requires updating the explicit locator mapping; preserving its stable IDs preserves identity. The registry itself is not an approval record.

Supported native syntax is the canonical main/ADDED subset tested here. MODIFIED/REMOVED/RENAMED delta application, HTML/inline HTML and noncanonical/setext requirement structures are explicitly unsupported. Unknown versions fail closed; there is no N/N-1 promise. Other tools are unsupported. Parsing native added facts does not claim a merged candidate baseline or execute OpenSpec archive/write operations.

Official compatibility evidence: `node scripts/check-native-openspec.mjs` imports the separately installed @fission-ai/openspec package, asserts version 1.14.1, invokes its strict Validator on both fixture formats, and compares MarkdownParser extraction for the main, BOM/closed-heading/CRLF and fenced-example variants. Set OPENSPEC_PACKAGE_ROOT to a separately installed official package if the default workspace path differs. Runtime SpecGuard parsing does not invoke Node, OpenSpec, network or source commands. `cargo test --test native_openspec` independently parses the same native source bytes with Rust.
