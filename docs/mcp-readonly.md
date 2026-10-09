# Local read-only MCP profile

`specguard-mcp --local-readonly-v1 ROOT CONFIG.json CONFIG_DIGEST` starts the
optional sequential stdio server. CONFIG_DIGEST is the existing SG
`model::digest` of the exact configuration bytes (SHA-256 with its normal
prefix). The controller supplies this digest independently; a matching digest
is integrity checking, not identity authentication. Configuration must be a
regular file of at most 256 KiB. Startup failures exit 4 with a generic stderr
diagnostic; stdout contains only JSON-RPC responses. The existing `cargo run`
default remains the ordinary SpecGuard CLI.

Configuration uses the strict camelCase `mcp::SessionConfig`: `requestKey`
(16–256 characters), `sourcePolicy`, `binding`, `required`, `invocation`,
`mapping`, and `finishedAt`. The latter structures are the existing SG source,
producer and protected mapping schemas; invocation retains its existing
snake_case field spelling. All required identities, candidate binding, policy,
source root, time bounds and output scope are controller-owned. The request key
only permits access to this local session. It is not a GE-TRUST credential,
producer attestation, approval reference or execution grant. Use private local
stdio and protect launch configuration. No HTTP or multi-tenant profile exists.

The profile follows the already selected FlowGuard MCP 2025-11-25 protocol.
Implementation uses existing locked serde 1.0.228 and serde_json 1.0.145, without
a new runtime SDK dependency. Actual interoperability is tested with installed
Python MCP SDK 1.29.0 (test dependency). No other SDK or hosted UI is qualified.
Initialize, initialized notification, ping, tools/list and tools/call are
supported. Clients negotiate the reported protocol version. No asynchronous
tasks are advertised.

Three tools are exposed: `specguard_discover`, `specguard_check`, and
`specguard_evidence`. Each requires `requestKey`; only evidence additionally
accepts an exact `uri` returned by this session. URI lookup never reads a caller
path or network location. Calls cannot change controller expectations or select
a command. Unknown fields/tools are rejected; there is no commit, approve,
merge or shell entry point. Tool results carry `executionAuthorized:false` and
`authenticationProfile:"local-unverified"`. ALLOW is a technical result only.
The structural producer profile reuses actual SG discovery/parsing/GE/report
verification; it does not establish Git authority or support baseline approval.
Authenticated baseline consumption remains the separate reviewed trust port.

Each check rereads source and creates a new run ID; an earlier ALLOW is never
returned as a new result. Controller time bounds are fixed for this bounded
session, not a new clock or freshness authority. Existing immutable report
bytes remain queryable after later runs. The session caps checks at 64 and
retained artifact bytes/identity strings at 32 MiB. Input frames are at most
8192 bytes; oversized frames are drained before the next request. Structured
responses are capped at 4 MiB before JSON-RPC wrapping (16 MiB total budget).
Existing producer resource limits also apply.

Cancellation uses the existing cooperative `CancellationToken` supplied to
`serve` or `Protocol::handle` by the embedding host. A cancelled bound check
returns a fresh cancelled envelope with null decision; historical evidence
remains intact. The sequential CLI launcher supplies an initially clear token;
it does not support in-flight protocol cancellation or translate process
signals into a report. Terminating the process loses its in-memory session,
never promotes old evidence, and performs no source write. This limit is the
same sequential stdio boundary used by the selected FG profile.

Run `cargo test --locked --offline --test mcp_readonly` with Python MCP 1.29.0
installed for actual SDK/stdio and local contract verification. Tests cover
access denial, unknown/write tools, arbitrary URI refusal, duplicate/unknown
arguments, digest mismatch, oversized-frame recovery, real source reads,
evidence preservation, source mutation and cooperative cancellation. Task 4.4 is independently accepted at `80e6ea2` for this local profile;
see implementation ledger `specguard-mcp-independent-review.md`.
