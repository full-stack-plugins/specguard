# Official-format fixture provenance

`main/spec.md` uses the main-spec grammar of the installed official @fission-ai/openspec 1.14.1 MarkdownParser and strict Validator. `change/specs/session/spec.md` fills the native `schemas/spec-driven/templates/spec.md` template from that package with fixture-owned authentication text. Neither document contains SpecGuard headers, YAML or generated IDs. The registry is a separate SpecGuard file frozen in the same source inventory; it is not claimed as part of the native OpenSpec format.

Verified package source: https://github.com/Fission-AI/OpenSpec (installed package.json version 1.14.1). Relevant installed files: dist/core/parsers/markdown-parser.js, requirement-text.js, requirement-blocks.js, spec-structure.js, code-fence.js, dist/core/validation/validator.js and schemas/spec-driven/templates/spec.md. The source package is MIT licensed. The Rust adapter is an independent implementation, with golden extraction and actual official-tool execution, not a claim based only on similar headings.

`official-extraction.json` is captured output from the official validator/parser run. Reproduce and check with `node scripts/check-native-openspec.mjs`. Main and added-delta validation returned valid=true with zero errors/warnings/info; the main requirement has two scenarios. Additional generated variants (BOM, CRLF, closing ATX hashes and fenced fake headings) are validated and compared by that script. The Rust suite checks those same extraction strings and source identities.

The fixed 1.14.1 main/ADDED capability set is supported; all other delta operations remain unsupported pending explicit baseline-aware application. This is not universal OpenSpec compatibility or production authentication.
