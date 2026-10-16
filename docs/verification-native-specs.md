# Native specification verification evidence — 2026-10-16

Scope: [professional-guard-checks](../../guardengine/openspec/changes/professional-guard-checks/), extending existing SpecGuard technical design without initializing another specification system. Existing main branch, uncommitted README changes, untracked 0.2.0 source and plugin were preserved; no commit, branch switch, global installation or shared GuardEngine source edit was performed.

PowerShell environment used for every Cargo invocation:

```powershell
$env:CARGO_HOME='E:/workspaces/workspace-agent-skills/.guard-build/cargo'
$env:CARGO_TARGET_DIR='E:/workspaces/workspace-agent-skills/.guard-build/targets/specguard-agent'
$env:TEMP='E:/workspaces/workspace-agent-skills/.guard-build/tmp'
$env:TMP=$env:TEMP
```

## Red / green evidence

1. `cargo test --locked --test native_specs` initially failed four cases with exit 4 (`unsupported command`) instead of the expected native-check decisions. CLI was absent.
2. After native CLI implementation, native MCP test failed because `specguard_check_specs` was absent from `tools/list`; domain tools were then implemented using the same analyzer.
3. The template-source test exposed a false ALLOW from `openspec/templates/spec.md`; native parsing was restricted to actual baseline/change specification paths.
4. The `.specify` file test exposed a false ALLOW when the framework marker was a regular file; native roots now require directories.
5. The traversal-budget test exposed a false ALLOW for 520 empty directories split across two parent directories; a global visited-entry budget now includes directories.
6. The full suite initially exposed an existing fixed MCP tool-count assertion (expected four, now six); that assertion was updated for the two new read-only tools. Existing AGENTS behavior and CLI/MCP parity remain tested.

Final executed validation:

| Command | Observed result |
|---|---|
| `cargo test --locked` | 22 tests passed: 10 existing AGENTS/plugin tests and 12 native specification tests, zero failures |
| `cargo clippy --locked --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --check` | exit 0 |
| `git diff --check` | exit 0 |

Native tests cover check → original-file repair → recheck → retained observation → verify, repeated deterministic output, requirement/scenario drift, new task artifact drift, Spec Kit design/plan drift, duplicate IDs/names, missing scenario clauses, examples, removed deltas, unsupported source templates, invalid UTF-8, 1 MiB per-file limit, inventory/traversal budget, missing or invalid framework markers, Windows junction rejection, malformed or wrong-project/source baseline reports, CLI/MCP equivalence, rejected mutation arguments and unchanged input bytes.

## Real repository read-only audit

Executed:

```powershell
E:/workspaces/workspace-agent-skills/.guard-build/targets/specguard-agent/debug/specguard.exe specs check --project E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories/guardengine --source openspec
```

Observed at audit time: exit 3 / INDETERMINATE, 21 source files, 15 supported requirements, zero structural findings and one diagnostic. The historical `bootstrap-guard-protocol/specs/guard-protocol/spec.md` uses `### Strict versioning` and similar custom headings instead of native `### Requirement:` headings. This artifact remains unsupported and was not rewritten to obtain ALLOW. Later edits to GuardEngine may change these counts; rerun for current evidence.

## Remaining boundaries

No semantic correctness, task/test trace graph, signed evidence, protected approval baseline, external review authority, operating-system sandbox, host-loaded plugin, installed binary or production delivery authorization is claimed. Report baselines are caller-editable observations bound to a canonical local project path and source. Source reads are bounded but not an atomic checkout snapshot. Unknown native forms and resource/read errors remain INDETERMINATE. Plugin files and skill references are verified locally; actual host loading requires a host integration check.
