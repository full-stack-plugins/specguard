use crate::{agents_policy::AgentsPolicy, markdown_document::MarkdownDocument};
use guard_install::{
    CONTEXT_GUARDS, agent_context_block, agent_context_requirements, read_project_file,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

fn finding(
    code: &str,
    origin: &str,
    path: &str,
    line: Option<usize>,
    rule: &str,
    message: &str,
    remediation: &str,
) -> Value {
    json!({"code":code,"origin":origin,"path":path,"line":line,"rule":rule,"message":message,"remediation":remediation})
}

// 先验证每个存在的路径段，避免把断链或符号链接伪装成“可选配置缺失”。
fn optional_file(root: &Path, relative: &str) -> Result<Option<Vec<u8>>, String> {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let linked = meta.file_type().is_symlink();
        #[cfg(windows)]
        let linked = {
            use std::os::windows::fs::MetadataExt;
            linked || meta.file_attributes() & 0x400 != 0
        };
        if linked {
            return Err(format!(
                "linked path unsupported by this analyzer: {relative}"
            ));
        }
    }
    read_project_file(root, relative).map(Some)
}

fn load(root: &Path, path: &str, sources: &mut Vec<Value>) -> Result<Option<Vec<u8>>, String> {
    let bytes = optional_file(root, path).map_err(|error| format!("{path}: {error}"))?;
    sources.push(json!({"path":path,"present":bytes.is_some(),"sha256":bytes.as_ref().map(|b|format!("{:x}",Sha256::digest(b)))}));
    Ok(bytes)
}

fn inspect(source: &str, policy: Option<AgentsPolicy>, findings: &mut Vec<Value>) {
    let document = MarkdownDocument::parse(source.trim_start_matches('\u{feff}'));
    let mut spans: BTreeMap<&str, Vec<(usize, usize)>> = BTreeMap::new();
    let mut active: Option<(&str, usize)> = None;
    for (line, marker) in &document.markers {
        for name in CONTEXT_GUARDS {
            if marker == &format!("<!-- {}_START -->", name.to_uppercase()) {
                if active.is_some() {
                    findings.push(finding(
                        "nested_block",
                        "organization",
                        "AGENTS.md",
                        Some(*line),
                        name,
                        "Guard blocks must not nest",
                        "修复标记嵌套后重新检查。",
                    ));
                }
                active = Some((name, *line));
            }
            if marker == &format!("<!-- {}_END -->", name.to_uppercase()) {
                match active.take() {
                    Some((opened, start)) if opened == name => {
                        spans.entry(name).or_default().push((start, *line))
                    }
                    _ => findings.push(finding(
                        "unbalanced_block",
                        "organization",
                        "AGENTS.md",
                        Some(*line),
                        name,
                        "Unmatched or out-of-order end marker",
                        "修复开始/结束标记配对后重新检查。",
                    )),
                }
            }
        }
    }
    if let Some((name, line)) = active {
        findings.push(finding(
            "unbalanced_block",
            "organization",
            "AGENTS.md",
            Some(line),
            name,
            "Missing end marker",
            "补齐受管块结束标记。",
        ));
    }
    for name in CONTEXT_GUARDS {
        let blocks = spans.get(name).map(Vec::as_slice).unwrap_or_default();
        if blocks.len() != 1 {
            findings.push(finding(if blocks.is_empty(){"block_missing"}else{"duplicate_block"}, "organization", "AGENTS.md", blocks.first().map(|b|b.0), name, "Require exactly one complete Guard block", &format!("查看 {name} instructions render --project PROJECT；缺失时使用已授权的 install，已有安装使用 instructions sync；先预览并保留人工内容。")));
            continue;
        }
        let (start, end) = blocks[0];
        for (index, expected) in agent_context_requirements(name).iter().enumerate() {
            if !document
                .lines
                .range((start + 1)..end)
                .any(|(_, line)| line.trim() == expected)
            {
                findings.push(finding("instruction_missing", "organization", "AGENTS.md", Some(start), &format!("{name}.{}", index + 1), expected, &format!("使用 {name} instructions sync --project PROJECT --dry-run 预览更新；模板仅用于审阅，不强制覆盖人工修改。")));
            }
        }
    }
    if let Some(policy) = policy {
        let visible = document
            .lines
            .values()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        for rule in policy.required_text {
            if !visible.contains(&rule.text) {
                findings.push(finding(
                    "project_text_missing",
                    "project",
                    "AGENTS.md",
                    None,
                    &rule.id,
                    &rule.text,
                    "按照项目附加约定补充正文，然后重新执行 agents check。",
                ));
            }
        }
    }
}

/// 检查指定项目根文档与附加策略；返回领域报告及退出码，不修改文件或执行文档命令。
pub(crate) fn check(root: &Path) -> (Value, u8) {
    let mut sources = vec![];
    let mut findings = vec![];
    let analysis = (|| -> Result<(), String> {
        let policy = load(root, ".specguard/agents-policy.json", &mut sources)?
            .map(|bytes| {
                AgentsPolicy::parse(&bytes)
                    .map_err(|error| format!(".specguard/agents-policy.json: {error}"))
            })
            .transpose()?;
        match load(root, "AGENTS.md", &mut sources)? {
            Some(bytes) => inspect(
                std::str::from_utf8(&bytes)
                    .map_err(|_| "AGENTS.md must be UTF-8 for this analyzer")?,
                policy,
                &mut findings,
            ),
            None => findings.push(finding(
                "agents_missing",
                "organization",
                "AGENTS.md",
                None,
                "root-document",
                "AGENTS.md is absent in the explicitly selected project directory",
                "先生成 agents template 审阅；在授权范围内使用各工具安装指令，保留项目人工约定。",
            )),
        }
        Ok(())
    })();
    let (decision, code) = match analysis {
        Err(message) => {
            findings.push(finding(
                "analysis_incomplete",
                "analyzer",
                ".",
                None,
                "input-coverage",
                &message,
                "修复输入或配置后重新检查；不可将未完成分析当作通过。",
            ));
            ("INDETERMINATE", 3)
        }
        Ok(()) if !findings.is_empty() => ("BLOCK", 2),
        Ok(()) => ("ALLOW", 0),
    };
    (
        json!({"schema_version":1,"tool":"specguard","version":env!("CARGO_PKG_VERSION"),"profile":"six-guard-context/v1","project":root,
        "decision":decision,"scope":"root-document-conformance","coverage":{"status":if code==3{"partial"}else{"complete"},"included":["AGENTS.md",".specguard/agents-policy.json"],"excluded":["parent and nested instruction inheritance","host overrides and truncation","natural-language contradiction detection","tool installation and execution","approval authority"]},
        "official_baseline":{"url":"https://agents.md/","mandatory_headings":false,"checked_on":"2026-10-16"},
        "sources":sources,"findings":findings,"execution_status":"not_evaluated","approval_status":"not_evaluated"}),
        code,
    )
}

/// 返回组织约定模板用于审阅；输出不代表安装、配置或运行已完成。
pub(crate) fn template() -> Value {
    json!({"profile":"six-guard-context/v1","content":CONTEXT_GUARDS.iter().map(|name|agent_context_block(name)).collect::<Vec<_>>().join("\n"),"written":false,"execution_status":"not_evaluated"})
}
