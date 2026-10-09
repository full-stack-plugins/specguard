# SG local handoff fixtures

Version: specguard.domain/v1alpha1. Consumer simulation: `cargo test --test baseline_handoff`. Golden producer regeneration: `cargo run --example generate_fixture`. Regeneration writes only these explicit fixture files. This fixture deliberately uses synthetic digest/OID values and an in-memory fixture approval controller; it is not a real repository candidate or a production approval. No production approval provider is enabled.

`baseline.json` binds repository, stable requirement scope, source revision/digest, graph digest, policy digest, approval reference, effective interval and immutable graph. `obligations.json` requires apiVersion, kind, baselineDigest, sourceDigest, candidateOid, scope, sources, complete, authenticationProfile and obligations. Every obligation carries stable ID, requirement/acceptance identities, source, text digest and typed links. `complete: false` cannot satisfy a complete plan; `fixture-only` cannot satisfy production authentication. No field claims that tests passed.

TestGuard and ArchGuard owners received these fixture paths and types. Actual consumer tests must be recorded by their owners with concrete versions and commands; this repository's golden test is explicitly a simulation. ADR/task relation tags are reserved in the DTO but the current source parser only produces depends_on.
