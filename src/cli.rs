//! Read-only CLI. Output is local unverified evidence, never a gate authorization.
use crate::integration::{
    producer::{Invocation, ProducedRun, ProtectedMapping, prepare},
    runtime::CancellationToken,
};
use crate::{
    model::Identity,
    source::{CandidateBinding, SourcePolicy, SourceSnapshot, discover, freeze},
    trace,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, io::Read, path::Path};
const HELP:&str="specguard doctor ROOT POLICY.json
specguard scan|trace|trace-check|trace-export ROOT POLICY.json BINDING.json REQUIRED.json
specguard diff ROOT POLICY.json BINDING.json BASELINE.json --unverified-baseline
specguard check ROOT REQUEST.json [--cancel]
All commands are read-only, JSON stdout. --report is unsupported and modifies no path.
check: 0 ALLOW, 2 BLOCK, 4 error/cancelled. Structural profile cannot produce REQUIRE_APPROVAL (reserved exit 3).
Query commands: 0 on produced output, 4 on input/runtime errors. Legacy trace-check/trace-export retain 0/2/4.
Check and diff do not authenticate candidate/controller/approval.
";
fn read<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("configuration must be a regular file".into());
    }
    let mut bytes = vec![];
    file.take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1_048_576 {
        return Err("configuration byte limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CheckRequest {
    api_version: CheckVersion,
    source_policy: SourcePolicy,
    binding: CandidateBinding,
    required: BTreeSet<Identity>,
    invocation: Invocation,
    mapping: ProtectedMapping,
    finished_at: String,
}
#[derive(Deserialize)]
enum CheckVersion {
    #[serde(rename = "specguard.cli-check/v1alpha1")]
    V1,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckOutput {
    api_version: &'static str,
    authentication_profile: &'static str,
    #[serde(flatten)]
    bundle: ProducedRun,
}
fn encode(value: &impl Serialize) -> Result<Vec<u8>, String> {
    // Count borrowed encoded bytes, including JSON byte-array expansion, before allocation.
    crate::integration::producer::preflight(value)
        .map_err(|_| "CLI output byte budget".to_owned())?;
    struct Bounded(Vec<u8>);
    impl std::io::Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len()
                > guardengine::integration::MAX_ARTIFACT_BYTES.saturating_sub(self.0.len())
            {
                return Err(std::io::Error::other("CLI output byte budget"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Bounded(Vec::new());
    serde_json::to_writer(&mut output, value).map_err(|e| e.to_string())?;
    Ok(output.0)
}
fn write_output(bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    std::io::stdout()
        .lock()
        .write_all(bytes)
        .map_err(|e| e.to_string())
}
fn emit(value: &impl Serialize) -> Result<(), String> {
    write_output(&encode(value)?)
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Query<T: Serialize> {
    api_version: &'static str,
    kind: &'static str,
    authentication_profile: &'static str,
    #[serde(flatten)]
    payload: T,
}
fn query(kind: &'static str, payload: impl Serialize) -> Result<(), String> {
    emit(&Query {
        api_version: "specguard.cli-query/v1alpha1",
        kind,
        authentication_profile: "unverified",
        payload,
    })
}
#[derive(Serialize)]
struct Doctor<'a> {
    inventory: &'a crate::source::SourceInventory,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Scan<'a> {
    candidate_binding: &'a CandidateBinding,
    source_digest: &'a str,
    inventory: &'a crate::source::SourceInventory,
    graph: &'a crate::graph::TypedSpecificationGraph,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Diff<'a> {
    baseline_digest: String,
    candidate_oid: &'a str,
    source_digest: &'a str,
    diff: &'a crate::baseline::BaselineDiff,
}
fn snapshot(root: &str, policy: &str, binding: &str) -> Result<SourceSnapshot, String> {
    let policy: SourcePolicy = read(policy)?;
    let binding: CandidateBinding = read(binding)?;
    let root = Path::new(root);
    freeze(root, &discover(root, &policy)?, binding)
}
pub fn run(args: &[String]) -> Result<i32, String> {
    if args == ["--help"] || args == ["help"] {
        print!("{HELP}");
        return Ok(0);
    }
    if args
        .iter()
        .any(|a| a == "--report" || a.starts_with("--report="))
    {
        return Err("--report unsupported; no path was modified; old files are not this invocation's result".into());
    }
    match args.first().map(String::as_str) {
        Some("doctor") if args.len() == 3 => {
            let policy: SourcePolicy = read(&args[2])?;
            let inventory = discover(Path::new(&args[1]), &policy)?;
            query(
                "doctor",
                Doctor {
                    inventory: &inventory,
                },
            )?;
            Ok(0)
        }
        Some("scan" | "trace" | "trace-check" | "trace-export") if args.len() == 5 => {
            let snapshot = snapshot(&args[1], &args[2], &args[3])?;
            let required: BTreeSet<Identity> = read(&args[4])?;
            let artifact = trace::scan(&snapshot, &required)?;
            trace::export(&artifact)?;
            if args[0] == "scan" {
                query(
                    "scan",
                    Scan {
                        candidate_binding: &snapshot.binding,
                        source_digest: &snapshot.digest,
                        inventory: &snapshot.inventory,
                        graph: &artifact.graph,
                    },
                )?;
            } else {
                emit(&artifact)?;
            }
            Ok(
                if matches!(args[0].as_str(), "trace-check" | "trace-export")
                    && !(artifact.graph.complete() && artifact.findings.is_empty())
                {
                    2
                } else {
                    0
                },
            )
        }
        Some("diff") if args.len() == 6 && args[5] == "--unverified-baseline" => {
            let snapshot = snapshot(&args[1], &args[2], &args[3])?;
            crate::integration::producer::parser_preflight(&snapshot)?;
            let baseline: crate::baseline::ApprovedBaseline = read(&args[4])?;
            let parsed = &baseline.graph.parsed;
            if parsed
                .requirements
                .len()
                .saturating_add(parsed.acceptances.len())
                .saturating_add(parsed.edges.len())
                .saturating_add(parsed.sources.len())
                > 4096
            {
                return Err("baseline graph budget".into());
            }
            let graph = crate::graph::build_graph(crate::parser::parse(&snapshot));
            let diff = crate::baseline::compare_baseline(
                &baseline,
                &graph,
                &std::collections::BTreeMap::new(),
            )?;
            query(
                "diff",
                Diff {
                    baseline_digest: crate::model::digest(&baseline),
                    candidate_oid: &snapshot.binding.candidate_oid,
                    source_digest: &snapshot.digest,
                    diff: &diff,
                },
            )?;
            Ok(0)
        }
        Some("check") if args.len() == 3 || (args.len() == 4 && args[3] == "--cancel") => {
            let request: CheckRequest = read(&args[2])?;
            let _version = request.api_version;
            let root = Path::new(&args[1]);
            let inventory = discover(root, &request.source_policy)?;
            let snapshot = freeze(root, &inventory, request.binding)?;
            let prepared = prepare(
                root,
                &snapshot,
                request.invocation,
                &request.required,
                &request.mapping,
            )
            .map_err(|e| format!("{}: {}", e.code, e.message))?;
            let token = CancellationToken::new();
            if args.len() == 4 {
                token.cancel();
            }
            let recovery = prepared.recovery_receipt();
            let bundle = prepared
                .execute(&request.finished_at, &token, |_| {})
                .map_err(|e| format!("{}: {}", e.code, e.message))?;
            bundle.verify()?;
            let mut output = CheckOutput {
                api_version: "specguard.cli-check/v1alpha1",
                authentication_profile: "unverified",
                bundle,
            };
            let bytes = match encode(&output) {
                Ok(bytes) => bytes,
                Err(_) => {
                    let mut history = crate::integration::runtime::History::default();
                    for source in &snapshot.inventory.sources {
                        history.record(source);
                    }
                    output.bundle = recovery
                        .failure(
                            guardengine::integration::RunStatus::Error,
                            "cli.output_budget",
                            &request.finished_at,
                            history,
                        )
                        .map_err(|e| format!("{}: {}", e.code, e.message))?;
                    output.bundle.verify()?;
                    encode(&output)?
                }
            };
            let code = match output.bundle.envelope.decision {
                Some(guardengine::Decision::Allow) => 0,
                Some(guardengine::Decision::Block) => 2,
                Some(guardengine::Decision::RequireApproval) => 3,
                None => 4,
            };
            if code == 4 {
                eprintln!(
                    "{}",
                    serde_json::json!({"code":"cli.bound_failure","message":"check did not complete; use this invocation's envelope and diagnostics"})
                );
            }
            write_output(&bytes)?;
            Ok(code)
        }
        _ => Err(format!("invalid arguments; {HELP}")),
    }
}

#[cfg(test)]
mod output_budget_tests {
    use super::*;
    #[test]
    fn query_payload_admission_counts_byte_array_expansion() {
        #[derive(Serialize)]
        struct Payload<'a> {
            bytes: &'a [u8],
        }
        let bytes = vec![255; 5 * 1024 * 1024];
        let query = Query {
            api_version: "specguard.cli-query/v1alpha1",
            kind: "probe",
            authentication_profile: "unverified",
            payload: Payload { bytes: &bytes },
        };
        assert_eq!(encode(&query).unwrap_err(), "CLI output byte budget");
    }
}
