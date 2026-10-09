//! Controller-frozen MCP 2025-11-25 local read-only profile, using existing Serde pins.
//! Access keys authorize this local session only; they do not authenticate evidence or approvals.
use crate::{
    integration::{
        producer::{Invocation, ProtectedMapping, prepare},
        runtime::CancellationToken,
    },
    model::Identity,
    source::{CandidateBinding, SourcePolicy, discover, freeze},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, Write},
    path::{Path, PathBuf},
};
pub const PROTOCOL_VERSION: &str = "2025-11-25";
const FRAME_LIMIT: usize = 8192;
const RESPONSE_LIMIT: usize = 16 * 1024 * 1024;
const HISTORY_LIMIT: usize = 32 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SessionConfig {
    pub request_key: String,
    pub source_policy: SourcePolicy,
    pub binding: CandidateBinding,
    pub required: BTreeSet<Identity>,
    pub invocation: Invocation,
    pub mapping: ProtectedMapping,
    pub finished_at: String,
}
pub struct ReadOnlySession {
    root: PathBuf,
    config: SessionConfig,
    attempt: u64,
    artifacts: BTreeMap<String, (String, Vec<u8>)>,
    retained: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Arguments {
    request_key: String,
    #[serde(default)]
    uri: Option<String>,
}
fn failed(code: &str) -> Value {
    json!({"exitCode":4,"diagnostic":code,"executionAuthorized":false,"authenticationProfile":"local-unverified"})
}
fn within(value: &impl serde::Serialize, limit: usize) -> bool {
    struct Count(usize);
    impl Write for Count {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(b.len())
                .ok_or_else(|| std::io::Error::other("budget"))?;
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Count(limit), value).is_ok()
}
impl ReadOnlySession {
    pub fn freeze(root: &Path, config: SessionConfig) -> Result<Self, String> {
        if config.request_key.len() < 16
            || config.request_key.len() > 256
            || config.request_key.chars().any(char::is_control)
        {
            return Err("invalid local access configuration".into());
        }
        crate::integration::producer::preflight(&(
            &config.source_policy,
            &config.binding,
            &config.required,
            &config.invocation,
            &config.mapping,
            &config.finished_at,
        ))?;
        if std::fs::symlink_metadata(root)
            .map_err(|_| "source root unavailable")?
            .file_type()
            .is_symlink()
        {
            return Err("source root must not be a symlink".into());
        }
        let root = root.canonicalize().map_err(|_| "source root unavailable")?;
        if !root.is_dir() {
            return Err("source root unavailable".into());
        }
        Ok(Self {
            root,
            config,
            attempt: 0,
            artifacts: BTreeMap::new(),
            retained: 0,
        })
    }
    fn call(&mut self, name: &str, args: Arguments, cancel: &CancellationToken) -> Value {
        if args.request_key != self.config.request_key {
            return failed("access_denied");
        }
        if (name == "specguard_evidence") != args.uri.is_some() {
            return failed("invalid_arguments");
        }
        match name {
            "specguard_discover" => {
                if cancel.is_cancelled() {
                    return failed("cancelled");
                }
                match discover(&self.root, &self.config.source_policy) {
                    Ok(inventory) => {
                        json!({"exitCode":0,"inventory":inventory,"executionAuthorized":false,"authenticationProfile":"local-unverified"})
                    }
                    Err(_) => failed("discovery_unavailable"),
                }
            }
            "specguard_check" => self
                .check(cancel)
                .unwrap_or_else(|_| failed("check_unavailable")),
            "specguard_evidence" => {
                if cancel.is_cancelled() {
                    return failed("cancelled");
                }
                let uri = args.uri.unwrap();
                if uri.len() > 4096 {
                    return failed("invalid_uri");
                }
                match self.artifacts.get(&uri) {
                    Some((digest, bytes)) => match std::str::from_utf8(bytes) {
                        Ok(text) => {
                            json!({"exitCode":0,"uri":uri,"digest":digest,"text":text,"executionAuthorized":false,"authenticationProfile":"local-unverified"})
                        }
                        Err(_) => failed("evidence_encoding"),
                    },
                    None => failed("evidence_unavailable"),
                }
            }
            _ => failed("unsupported_tool"),
        }
    }
    fn check(&mut self, cancel: &CancellationToken) -> Result<Value, String> {
        if self.attempt >= 64 {
            return Err("session capacity".into());
        }
        self.attempt += 1;
        let c = &self.config;
        let inventory = discover(&self.root, &c.source_policy)?;
        let snapshot = freeze(&self.root, &inventory, c.binding.clone())?;
        let original = &c.invocation;
        let invocation = Invocation {
            run_id: format!("{}-mcp-{}", original.run_id, self.attempt),
            repo_id: original.repo_id.clone(),
            task_id: original.task_id.clone(),
            worktree_id: original.worktree_id.clone(),
            candidate_oid: original.candidate_oid.clone(),
            base_oid: original.base_oid.clone(),
            merge_group_id: original.merge_group_id.clone(),
            baseline_digest: original.baseline_digest.clone(),
            started_at: original.started_at.clone(),
        };
        let prepared = prepare(&self.root, &snapshot, invocation, &c.required, &c.mapping)
            .map_err(|_| "check prebinding failed")?;
        let output = prepared
            .execute(&c.finished_at, cancel, |_| {})
            .map_err(|_| "check failed")?;
        output.verify()?;
        let envelope = &output.envelope;
        let mut additions = vec![];
        let mut cost = 0usize;
        let refs = [
            envelope.artifacts.contract.as_ref(),
            envelope.artifacts.facts.as_ref(),
            envelope.artifacts.report.as_ref(),
            envelope.artifacts.domain.first(),
        ];
        for (reference, bytes) in refs.into_iter().zip([
            output.contract.as_ref(),
            output.facts.as_ref(),
            output.report.as_ref(),
            output.domain.as_ref(),
        ]) {
            if let (Some(reference), Some(bytes)) = (reference, bytes) {
                if let Some((old_digest, old_bytes)) = self.artifacts.get(&reference.uri) {
                    if old_digest != &reference.digest || old_bytes != bytes {
                        return Err("artifact identity collision".into());
                    }
                } else {
                    cost = cost
                        .saturating_add(bytes.len())
                        .saturating_add(reference.uri.len())
                        .saturating_add(reference.digest.len());
                    additions.push((
                        reference.uri.clone(),
                        (reference.digest.clone(), bytes.clone()),
                    ));
                }
            }
        }
        if self.retained.saturating_add(cost) > HISTORY_LIMIT {
            return Err("evidence capacity".into());
        }
        let code = match envelope.decision {
            Some(guardengine::Decision::Allow) => 0,
            Some(guardengine::Decision::Block) => 2,
            Some(guardengine::Decision::RequireApproval) => 3,
            None => 4,
        };
        let response = json!({"exitCode":code,"envelope":envelope,"executionAuthorized":false,"authenticationProfile":"local-unverified"});
        if !within(&response, RESPONSE_LIMIT / 4) {
            return Err("response budget".into());
        }
        self.artifacts.extend(additions);
        self.retained += cost;
        Ok(response)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Message {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Params>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Params {
    Initialize(Initialize),
    Call(Call),
    Empty(Empty),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Initialize {
    protocol_version: String,
    capabilities: Value,
    client_info: ClientInfo,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientInfo {
    name: String,
    version: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Call {
    name: String,
    arguments: Arguments,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
pub struct Protocol {
    session: ReadOnlySession,
    initialized: bool,
    ready: bool,
}
fn failure(id: Value, code: i32) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":"request unavailable"}})
}
impl Protocol {
    pub fn new(session: ReadOnlySession) -> Self {
        Self {
            session,
            initialized: false,
            ready: false,
        }
    }
    pub fn handle(&mut self, bytes: &[u8], cancel: &CancellationToken) -> Option<Value> {
        if bytes.len() > FRAME_LIMIT {
            return Some(failure(Value::Null, -32600));
        }
        let msg: Message = match serde_json::from_slice(bytes) {
            Ok(v) => v,
            Err(_) => return Some(failure(Value::Null, -32600)),
        };
        if msg.jsonrpc != "2.0"
            || msg
                .id
                .as_ref()
                .is_some_and(|v| !v.is_string() && !v.is_i64() && !v.is_u64())
        {
            return Some(failure(Value::Null, -32600));
        }
        let Some(id) = msg.id else {
            if msg.method == "notifications/initialized"
                && self.initialized
                && matches!(msg.params, None | Some(Params::Empty(_)))
            {
                self.ready = true;
            }
            return None;
        };
        let result = match (msg.method.as_str(), msg.params) {
            ("initialize", Some(Params::Initialize(p))) if !self.initialized => {
                if p.protocol_version.is_empty()
                    || !p.capabilities.is_object()
                    || p.client_info.name.is_empty()
                    || p.client_info.version.is_empty()
                {
                    return Some(failure(id, -32602));
                }
                self.initialized = true;
                json!({"protocolVersion":PROTOCOL_VERSION,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"specguard-local-readonly","version":env!("CARGO_PKG_VERSION")},"instructions":"Controller-selected local source only. Results do not authenticate evidence or grant execution. Cancellation is cooperative through the host token; no asynchronous tasks."})
            }
            ("ping", None | Some(Params::Empty(_))) => json!({}),
            _ if !self.ready => return Some(failure(id, -32002)),
            ("tools/list", None | Some(Params::Empty(_))) => {
                let tools:[Value;3]=["specguard_discover","specguard_check","specguard_evidence"].map(|name|{let mut required=vec!["requestKey"];let mut properties=json!({"requestKey":{"type":"string"}});if name=="specguard_evidence"{properties["uri"]=json!({"type":"string"});required.push("uri");}json!({"name":name,"description":"Read controller-selected local specification evidence; no approval or execution authority.","inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"annotations":{"readOnlyHint":true,"destructiveHint":false,"openWorldHint":false},"execution":{"taskSupport":"forbidden"}})});
                json!({"tools":tools})
            }
            ("tools/call", Some(Params::Call(p))) => {
                if ![
                    "specguard_discover",
                    "specguard_check",
                    "specguard_evidence",
                ]
                .contains(&p.name.as_str())
                {
                    return Some(failure(id, -32602));
                }
                let response = self.session.call(&p.name, p.arguments, cancel);
                if !within(&response, RESPONSE_LIMIT / 4) {
                    return Some(failure(id, -32000));
                }
                json!({"content":[{"type":"text","text":response.to_string()}],"structuredContent":response,"isError":response["exitCode"]!=0})
            }
            _ => return Some(failure(id, -32601)),
        };
        Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }
}
/// Sequential bounded stdio. Host token cancellation is checked inside the existing producer.
/// No out-of-order requests or asynchronous MCP tasks are advertised.
pub fn serve(
    session: ReadOnlySession,
    mut input: impl BufRead,
    mut output: impl Write,
    cancel: &CancellationToken,
) -> std::io::Result<()> {
    let mut protocol = Protocol::new(session);
    loop {
        let mut frame = vec![];
        let mut oversized = false;
        let mut seen = false;
        loop {
            let bytes = input.fill_buf()?;
            if bytes.is_empty() {
                break;
            }
            seen = true;
            let newline = bytes.iter().position(|b| *b == b'\n');
            let n = newline.map_or(bytes.len(), |i| i + 1);
            if frame.len() + n > FRAME_LIMIT {
                oversized = true;
            } else if !oversized {
                frame.extend_from_slice(&bytes[..n]);
            }
            input.consume(n);
            if newline.is_some() {
                break;
            }
        }
        if !seen {
            return Ok(());
        }
        let response = if oversized {
            Some(failure(Value::Null, -32600))
        } else {
            protocol.handle(&frame, cancel)
        };
        if let Some(value) = response {
            serde_json::to_writer(&mut output, &value)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
}
