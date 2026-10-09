# Retained actual joint capture inputs

`retained-capture-sha256.json` pins original bytes from the independent local
FlowGuard capture in `guard-implementation-ledger/flowguard-joint-independent-capture`.
It covers 259 files: all JSON artifacts under the five producer directories for
A/B/M1/M2, four actual FG outputs, and original generator/provenance/result
records. The independent source comparison identified FG1fd70a11, SG8fc3f9b,
GEb0e4b445, GGf6c7368c and AGd6c8235d. The retained producer-manifest preserves
per-tool command/binary provenance; hashes are integrity pins, not identities.
These are existing native/cross-guard captures, not newly simulated envelopes.

The SG joint exercise will read these bytes unchanged, then separately prepare
new SG runs on the corresponding actual candidate worktrees through current
SG source. Original producer pins and new consumer/source pins must remain
separate. No other native producer is rerun. The capture path may be overridden
by `SPECGUARD_JOINT_CAPTURE`; an override must match every pinned byte. Missing
captures fail the explicit joint test rather than silently passing.

CodeGuard native parity reference: implementation ledger
`codeguard-native-parity-independent-review.md`, fixed CG25bac257. No native
command replacement, production authentication or hosted queue authority is
implied by these local fixtures. Task4.8 is independently accepted for this local composition at `74ce33a`;
see ledger `specguard-joint-independent-review.md`.

## Current SG lifecycle replay

Run `cargo test --locked --offline --test joint_gate`, or:

```sh
SPECGUARD_JOINT_OUTPUT=/absolute/new-output-directory \
  cargo run --locked --offline --example joint_gate
```

The output directory must not already exist. The example saves five new current
SG raw bundles separately from all retained originals: A, B, M1, M2 and a late
M1 retry. Its stdout names the new runs and artifact SHA-256 values. This is a
new SG execution, not a claim that FG or any other native producer ran again.
Original historical FG ALLOW and per-producer artifacts retain their original
source/binary provenance. Source pins of this current SG run belong in its run
record, not in the old producer manifest.

Both independent requirement jobs are registered before either completes.
The real candidate worktrees are checked against the frozen candidate/base,
source bytes and required IDs. Queue candidates are actual multi-parent commits
and differ from their PR members. Contracts and domain artifact bytes match
the corresponding retained original SG outputs; facts differ only in the
explicitly permitted `subject.id` (the new attempt ID). Every new bundle is
verified by SG and consumed by its actual `RunHistory`/GE-TRUST eligibility
port using explicit fixture authorities. REQUIRE_APPROVAL remains unchanged.
B's approval attachment cannot authorize A. A/B have scoped baselines; M1 and
M2 intentionally share the same queue-scope baseline, rather than fabricating
a difference between them.

The clock advances past review-approval expiry, revocation is rechecked, and a
same-scope baseline input with only its expiry bound changed is rejected against
the original frozen digest. This private negative input does not edit captured
baseline bytes or represent a newly approved baseline. Late M1 remains retained
but cannot publish over M2. Disabling the fixture producer-authority adapter
makes current M2 ineligible while retaining its exact bytes. The separately
accepted real CLI/MCP adapter rollout is exercised in `examples/rollout/`;
this joint step does not claim to deploy or disable a hosted service.

All 259 retained artifact hashes are checked again after the new runs. No
producer is relabeled, no expert decision is rewritten, and no reference/hash
confers execution authority. Production identity, actual hosted queue control,
and durable cross-process history are outside this explicit local profile.
