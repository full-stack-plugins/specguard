# Local read-only source CLI

```
specguard doctor ROOT POLICY.json
specguard scan|trace|trace-check|trace-export ROOT POLICY.json BINDING.json REQUIRED.json
specguard diff ROOT POLICY.json BINDING.json BASELINE.json --unverified-baseline
specguard check ROOT REQUEST.json [--cancel] [--report-dir PRIVATE_DIR]
```

`doctor` discovers configured sources and emits inventory/capability diagnostics. `scan` freezes actual source bytes and emits the typed graph, inventory and binding. `trace` emits the validated versioned trace artifact. These queries exit 0 when they produce output, including partial coverage; that is not gate success. Input/runtime errors exit 4. Legacy trace-check/trace-export retain 0 for complete/no findings, 2 for incomplete/findings, and 4 for errors. `diff` requires the explicit unverified-baseline acknowledgement. Unknown and duplicate options reject.

Check emits bounded JSON stdout with `authenticationProfile: unverified`, a real GE envelope and exact contract/facts/report/domain bytes as nullable JSON byte arrays. Diagnostics use stderr. Check exits 0 ALLOW, 2 BLOCK, 3 REQUIRE_APPROVAL, or 4 error/cancelled. Prebinding failures emit no envelope/stdout. Bound errors carry null decision and diagnostic artifacts. `--cancel` produces a cancelled result; cooperative library cancellation remains available separately.

Existing `specguard.cli-check/v1alpha1` requests contain sourcePolicy, binding, required, invocation, mapping and finishedAt. Their structural policy remains Enforce-only. No existing finding is downgraded to manufacture exit 3. Incomplete/tool-error handling remains mandatory and cannot be repaired by Review.

Opt-in `specguard.cli-check-baseline-review/v1alpha1` requires an additional `review` object conforming to [the strict schema](../schemas/specguard-domain/cli-check-baseline-review.json). It contains version `specguard.baseline-review/v1`, full frozen baseline, a distinct exact Review contract and two mappings per required requirement: `text_review` and `acceptance_review`. Baseline repository/scope/digest must match invocation and required scope. Complete real baseline diffs generate Review facts; acceptance changes resolve the actual acceptance identity to its unchanged owning requirement. Structural failures still block. Other change kinds are unsupported by this narrow profile and produce a bound error, never silent ALLOW. Missing/unknown review inputs, ambiguous or overlapping mappings, and weakened structural rules reject. Baseline graphs are capped at 512 rows, review inputs at 1 MiB, combined contracts at 256 rules. Rust validates semantic bindings beyond JSON schema shape.

The review profile freezes the entire baseline, mapping and profile in private work identity and binds its distinct coverage scope. Domain artifacts include the baseline inputs and actual diff; verification reconstructs both Review and structural facts and the combined GE contract. Neither profile authenticates the controller, candidate, policy, baseline or an approval provider. A declared approval state or digest is not authentication. Production eligibility remains outside this CLI.

`--report-dir` is check-only and publishes **only an immutable envelope receipt**, using GE's reviewed local publisher. It requires an existing caller-protected private non-symlink directory on the qualified Unix platform. The destination is `attempt-<SHA256(runId)>.json`; it never replaces or deletes an existing path. The complete bundle remains stdout. Raw artifacts must be saved separately and resolved through authorized storage; a receipt alone is not a full bundle or new eligibility proof. Keep the directory inaccessible to untrusted writers throughout publication.

Publication failure returns exit 4 and bound error/null JSON. Existing files remain untouched and are not this invocation's result. A directory-sync failure can leave a newly linked receipt; the diagnostic requires reconciliation, not a claim of successful publication. Error/cancelled receipts preserve those statuses. Consumers must use the current process status and exact expected run/binding, never file existence. Arbitrary-file `--report` remains rejected before source reads. Source/configuration files are never modified.

Evidence: [actual binary tests](../tests/cli_contract.rs). Task 4.1 awaits independent review; this local interface does not complete production authentication or provider integration.
