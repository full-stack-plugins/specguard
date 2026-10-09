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
fn emit(value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    use std::io::Write;
    std::io::stdout()
        .lock()
        .write_all(&bytes)
        .map_err(|e| e.to_string())
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
            emit(
                &serde_json::json!({"apiVersion":"specguard.cli-query/v1alpha1","kind":"doctor","authenticationProfile":"unverified","inventory":inventory}),
            )?;
            Ok(0)
        }
        Some("scan" | "trace" | "trace-check" | "trace-export") if args.len() == 5 => {
            let snapshot = snapshot(&args[1], &args[2], &args[3])?;
            let required: BTreeSet<Identity> = read(&args[4])?;
            let artifact = trace::scan(&snapshot, &required)?;
            trace::export(&artifact)?;
            if args[0] == "scan" {
                emit(
                    &serde_json::json!({"apiVersion":"specguard.cli-query/v1alpha1","kind":"scan","authenticationProfile":"unverified","candidateBinding":snapshot.binding,"sourceDigest":snapshot.digest,"inventory":snapshot.inventory,"graph":artifact.graph}),
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
            emit(
                &serde_json::json!({"apiVersion":"specguard.cli-query/v1alpha1","kind":"diff","authenticationProfile":"unverified","baselineDigest":crate::model::digest(&baseline),"candidateOid":snapshot.binding.candidate_oid,"sourceDigest":snapshot.digest,"diff":diff}),
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
            let bundle = prepared
                .execute(&request.finished_at, &token, |_| {})
                .map_err(|e| format!("{}: {}", e.code, e.message))?;
            bundle.verify()?;
            let code = match bundle.envelope.decision {
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
            emit(&CheckOutput {
                api_version: "specguard.cli-check/v1alpha1",
                authentication_profile: "unverified",
                bundle,
            })?;
            Ok(code)
        }
        _ => Err(format!("invalid arguments; {HELP}")),
    }
}
