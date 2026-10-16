use crate::markdown_document::MarkdownDocument;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};

// 保留大小写和原始 UTF-8 字节；可逆的分段编码避免分隔符碰撞和跨功能 FR-001 混用。
fn component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn stable_id(system: &str, scope: &str, native_id: &str) -> String {
    format!(
        "urn:specguard:requirement:v1:{}/{}/{}",
        component(system),
        component(scope),
        component(native_id)
    )
}

// 逐段检查标识，不能先 follows-link 再检查末端；不存在和无法安全读取必须区分。
fn present(root: &Path, relative: &str) -> Result<bool, String> {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(format!("{relative}: {error}")),
        };
        let linked = metadata.file_type().is_symlink();
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            linked || metadata.file_attributes() & 0x400 != 0
        };
        if linked {
            return Err(format!("linked source unsupported: {relative}"));
        }
    }
    Ok(true)
}

fn decorate(system: &str, native: &Value, artifacts: &[Value]) -> Value {
    let path = native["path"].as_str().unwrap();
    let parts: Vec<_> = path.split('/').collect();
    let scope = match system {
        "openspec" => parts[parts.len() - 2].to_owned(),
        "speckit" => parts[1..parts.len() - 1].join("/"),
        _ => path
            .strip_prefix("docs/superpowers/specs/")
            .unwrap()
            .to_owned(),
    };
    let native_id = native["id"].as_str().unwrap();
    let source_state = match system {
        "openspec" if path.starts_with("openspec/specs/") => "baseline",
        "openspec" => "candidate",
        _ => "approval_unknown",
    };
    let digest = artifacts
        .iter()
        .find(|artifact| artifact["path"] == path)
        .map(|artifact| artifact["sha256"].clone())
        .unwrap_or(Value::Null);
    json!({"stable_id":stable_id(system, &scope, native_id),"native_id":native_id,"system":system,"scope":scope,"path":path,"line":native["line"],"operation":native["operation"],"source_state":source_state,"artifact_sha256":digest,"identity_basis":"native_key_with_scope","rename_policy":"explicit_mapping_required"})
}

fn superpowers(root: &Path) -> Value {
    let mut paths = Vec::new();
    let mut visited = 0;
    let mut diagnostics = Vec::new();
    let mut artifacts = Vec::new();
    let mut requirements = Vec::new();
    let mut findings = Vec::new();
    for relative in ["docs/superpowers/specs", "docs/superpowers/plans"] {
        match present(root, relative) {
            Ok(true) => {
                if let Err(error) =
                    crate::specs_check::collect(root, relative, 0, &mut paths, &mut visited)
                {
                    diagnostics.push(error);
                }
            }
            Ok(false) if relative.ends_with("/plans") => {}
            Ok(false) => diagnostics.push("missing Superpowers specs directory".into()),
            Err(error) => diagnostics.push(error),
        }
    }
    paths.sort();
    let mut total = 0;
    for path in paths {
        let bytes = match guard_install::read_project_file(root, &path) {
            Ok(bytes) => bytes,
            Err(error) => {
                diagnostics.push(format!("{path}: {error}"));
                continue;
            }
        };
        total += bytes.len();
        if total > 16 * 1024 * 1024 {
            diagnostics.push("source bytes exceed 16 MiB total".into());
            break;
        }
        artifacts.push(json!({"path":path,"sha256":format!("{:x}",Sha256::digest(&bytes))}));
        if !path.starts_with("docs/superpowers/specs/") || !path.ends_with(".md") {
            continue;
        }
        let text = match std::str::from_utf8(&bytes) {
            Ok(text) => text.trim_start_matches('\u{feff}'),
            Err(_) => {
                diagnostics.push(format!("{path}: specification must be UTF-8"));
                continue;
            }
        };
        let before = requirements.len();
        let mut ids = BTreeSet::new();
        for (line, text) in MarkdownDocument::parse(text).lines {
            let Some((id, statement)) = text.trim().split_once(':') else {
                continue;
            };
            // Superpowers 没有统一需求编号语法；仅接受项目显式声明的 REQ-/FR- 行约定。
            let suffix = id.strip_prefix("REQ-").or_else(|| id.strip_prefix("FR-"));
            if suffix.is_none_or(|suffix| suffix.is_empty())
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            {
                continue;
            }
            requirements.push(json!({"id":id,"path":path,"line":line,"operation":"defined"}));
            if !ids.insert(id.to_owned()) {
                findings.push(json!({"code":"duplicate_requirement","path":path,"line":line,"message":"Explicit requirement ID repeats in one Superpowers document"}));
            }
            if statement.trim().is_empty() {
                findings.push(json!({"code":"requirement_empty","path":path,"line":line,"message":"Explicit requirement has no statement"}));
            }
        }
        if before == requirements.len() {
            diagnostics.push(format!(
                "{path}: no explicit supported requirement IDs; freeform content requires review"
            ));
        }
    }
    if requirements.is_empty() {
        diagnostics.push("no supported native requirements observed".into());
    }
    json!({"sources":artifacts,"requirements":requirements,"findings":findings,"diagnostics":diagnostics})
}

/// 发现现有原生来源并只读提取作用域 ID；返回观察报告和退出码，绝不选择审批事实源。
pub(crate) fn check(root: &Path) -> Result<(Value, u8), String> {
    let mut sources = Vec::new();
    let mut requirements = Vec::new();
    let mut diagnostics = Vec::new();
    let mut findings = Vec::new();
    for (system, marker) in [
        ("openspec", "openspec"),
        ("speckit", ".specify"),
        ("superpowers", "docs/superpowers/specs"),
    ] {
        match present(root, marker) {
            Ok(false) => continue,
            Err(error) => {
                diagnostics.push(json!({"system":system,"message":error}));
                continue;
            }
            Ok(true) => {}
        }
        let native = if system == "superpowers" {
            superpowers(root)
        } else {
            crate::specs_check::check(root, system, None)?.0
        };
        let artifacts = native["sources"].as_array().unwrap();
        let source_diagnostics = native["diagnostics"].as_array().unwrap();
        let source_findings = native["findings"].as_array().unwrap();
        for requirement in native["requirements"].as_array().unwrap() {
            requirements.push(decorate(system, requirement, artifacts));
        }
        for finding in source_findings {
            let mut finding = finding.clone();
            finding["system"] = json!(system);
            findings.push(finding);
        }
        for message in source_diagnostics {
            diagnostics.push(json!({"system":system,"message":message}));
        }
        sources.push(json!({"system":system,"marker":marker,"status":if !source_diagnostics.is_empty(){"incomplete"}else if !source_findings.is_empty(){"invalid"}else{"observed"},"artifacts":artifacts}));
    }
    let authority = match sources.len() {
        0 => "unknown",
        1 if diagnostics.is_empty() => "single_source_observed",
        1 => "unresolved",
        _ => "conflict",
    };
    if sources.is_empty() {
        diagnostics.push(json!({"system":null,"message":"no supported native specification source; no initialization performed"}));
    }
    if authority == "conflict" {
        diagnostics.push(json!({"system":null,"message":"multiple possible requirement sources; priority/change ownership is not established, review required"}));
    }
    let code = if !diagnostics.is_empty() {
        3
    } else if !findings.is_empty() {
        2
    } else {
        0
    };
    Ok((
        json!({"schema":"specguard-sources/v1","analyzer":"native-source-adapters/1","project":root.to_string_lossy(),"authority_status":authority,"selected_source":null,"approved_baseline_eligible":false,"approval_status":"not_evaluated","execution_status":"not_evaluated","trust":"local_unverified","sources":sources,"requirements":requirements,"findings":findings,"diagnostics":diagnostics,"coverage":{"inventory_complete":code!=3,"semantic_validation":"not_evaluated","source_priority":"not_evaluated","excluded":["openspec/changes/archive"],"superpowers_ids":"project_explicit_REQ_or_FR_only"}}),
        code,
    ))
}
