use crate::markdown_document::MarkdownDocument;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, io::Read, path::Path};

const MAX_BYTES: usize = 1024 * 1024;
const MAX_FILES: usize = 512;
const MAX_TOTAL: usize = 16 * MAX_BYTES;

fn issue(code: &str, path: &str, line: usize, message: &str) -> Value {
    json!({"code":code,"path":path,"line":line,"message":message})
}

fn bounded_read(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("source exceeds 1 MiB limit".into());
    }
    Ok(bytes)
}

fn linked(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_type().is_symlink() || meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

// 每个目录段都拒绝链接；有界读取避免目录逃逸、无限递归和资源耗尽。
fn collect(
    root: &Path,
    relative: &str,
    depth: usize,
    paths: &mut Vec<String>,
    visited: &mut usize,
) -> Result<(), String> {
    *visited += 1;
    if depth > 12 || *visited > MAX_FILES {
        return Err("source inventory limit exceeded".into());
    }
    let path = root.join(relative);
    let meta = fs::symlink_metadata(&path).map_err(|e| format!("{relative}: {e}"))?;
    if linked(&meta) {
        return Err(format!("linked source unsupported: {relative}"));
    }
    if depth == 0 && !meta.is_dir() {
        return Err(format!(
            "native source root must be a directory: {relative}"
        ));
    }
    if meta.is_file() {
        paths.push(relative.to_owned());
    } else if meta.is_dir() {
        let mut entries = fs::read_dir(&path)
            .map_err(|e| e.to_string())?
            .take(MAX_FILES + 1)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        if entries.len() > MAX_FILES {
            return Err("directory entry limit exceeded".into());
        }
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "non-UTF8 source path")?;
            // 归档不属于候选规格；仍先检查链接，不能利用 archive 链接绕过边界。
            if relative == "openspec/changes" && name == "archive" {
                if linked(&fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?) {
                    return Err("linked archive unsupported".into());
                }
                continue;
            }
            collect(
                root,
                &format!("{relative}/{name}"),
                depth + 1,
                paths,
                visited,
            )?;
        }
    } else {
        return Err(format!("non-regular source unsupported: {relative}"));
    }
    Ok(())
}

fn headings(text: &str, document: &MarkdownDocument) -> Vec<(usize, HeadingLevel, String)> {
    let mut result = Vec::new();
    let offsets: Vec<_> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    for (event, range) in Parser::new(text).into_offset_iter() {
        if let Event::Start(Tag::Heading { level, .. }) = event {
            let line = offsets.partition_point(|offset| *offset <= range.start);
            if let Some(title) = document.lines.get(&line) {
                result.push((line, level, title.clone()));
            }
        }
    }
    result
}

fn openspec(path: &str, text: &str, requirements: &mut Vec<Value>, findings: &mut Vec<Value>) {
    let document = MarkdownDocument::parse(text);
    let headings = headings(text, &document);
    let mut names = BTreeSet::new();
    let mut removed = false;
    for (index, (line, level, title)) in headings.iter().enumerate() {
        if *level == HeadingLevel::H2 {
            removed = title == "REMOVED Requirements";
        }
        if *level != HeadingLevel::H3 {
            continue;
        }
        let Some(name) = title
            .strip_prefix("Requirement:")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        requirements.push(json!({"id":name,"path":path,"line":line,"operation":if removed {"removed"} else {"defined"}}));
        if !names.insert(name) {
            findings.push(issue(
                "duplicate_requirement",
                path,
                *line,
                "Requirement name repeats within the same capability artifact",
            ));
        }
        if removed {
            continue;
        }
        let end = headings[index + 1..]
            .iter()
            .find(|(_, level, _)| {
                matches!(
                    level,
                    HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3
                )
            })
            .map(|(line, _, _)| *line)
            .unwrap_or(usize::MAX);
        let scenarios: Vec<_> = headings[index + 1..]
            .iter()
            .take_while(|(line, _, _)| *line < end)
            .filter(|(_, level, title)| {
                *level == HeadingLevel::H4
                    && title
                        .strip_prefix("Scenario:")
                        .is_some_and(|s| !s.trim().is_empty())
            })
            .collect();
        if scenarios.is_empty() {
            findings.push(issue(
                "scenario_missing",
                path,
                *line,
                "OpenSpec requirement needs at least one #### Scenario",
            ));
        }
        for (i, (start, _, _)) in scenarios.iter().enumerate() {
            let stop = scenarios
                .get(i + 1)
                .map(|(line, _, _)| *line)
                .unwrap_or(end);
            let body: Vec<_> = document
                .lines
                .range((*start + 1)..stop)
                .map(|(_, text)| text.trim())
                .collect();
            if !body.iter().any(|s| s.starts_with("WHEN "))
                || !body.iter().any(|s| s.starts_with("THEN "))
            {
                findings.push(issue(
                    "scenario_incomplete",
                    path,
                    *start,
                    "Scenario needs WHEN and THEN clauses; business correctness is not evaluated",
                ));
            }
        }
    }
}

fn speckit(path: &str, text: &str, requirements: &mut Vec<Value>, findings: &mut Vec<Value>) {
    let document = MarkdownDocument::parse(text);
    let mut ids = BTreeSet::new();
    for (line, text) in &document.lines {
        let Some((id, statement)) = text.trim().split_once(':') else {
            continue;
        };
        if !id
            .strip_prefix("FR-")
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        {
            continue;
        }
        requirements.push(json!({"id":id,"path":path,"line":line,"operation":"defined"}));
        if !ids.insert(id) {
            findings.push(issue(
                "duplicate_requirement",
                path,
                *line,
                "Functional requirement ID repeats in the feature spec",
            ));
        }
        if statement.trim().is_empty() {
            findings.push(issue(
                "requirement_empty",
                path,
                *line,
                "Functional requirement has no statement",
            ));
        }
    }
}

/// 检查指定原生规格来源；返回结构报告和退出码，不执行任务或颁发审批。
pub(crate) fn check(
    root: &Path,
    source: &str,
    baseline: Option<&Path>,
) -> Result<(Value, u8), String> {
    let mut paths = Vec::new();
    let mut diagnostics = Vec::new();
    let mut visited = 0;
    let roots: &[&str] = if source == "openspec" {
        &["openspec"]
    } else {
        &[".specify", "specs"]
    };
    for relative in roots {
        if let Err(error) = collect(root, relative, 0, &mut paths, &mut visited) {
            diagnostics.push(error);
        }
    }
    paths.sort();
    let mut sources = Vec::new();
    let mut requirements = Vec::new();
    let mut findings = Vec::new();
    let mut total = 0usize;
    for path in &paths {
        let bytes = match bounded_read(&root.join(path)) {
            Ok(bytes) => bytes,
            Err(error) => {
                diagnostics.push(format!("{path}: {error}"));
                continue;
            }
        };
        total += bytes.len();
        if total > MAX_TOTAL {
            diagnostics.push("source bytes exceed 16 MiB total".into());
            break;
        }
        sources.push(json!({"path":path,"sha256":format!("{:x}",Sha256::digest(&bytes))}));
        if !path.ends_with("/spec.md") {
            continue;
        }
        let text = match std::str::from_utf8(&bytes) {
            Ok(text) => text.trim_start_matches('\u{feff}'),
            Err(error) => {
                diagnostics.push(format!("{path}: {error}"));
                continue;
            }
        };
        let before = requirements.len();
        if source == "openspec" {
            let parts: Vec<_> = path.split('/').collect();
            let native = (parts.len() == 4 && parts[1] == "specs")
                || (parts.len() == 6 && parts[1] == "changes" && parts[3] == "specs");
            if !native {
                continue;
            }
            openspec(path, text, &mut requirements, &mut findings);
        } else if path.starts_with("specs/") {
            speckit(path, text, &mut requirements, &mut findings);
        } else {
            continue;
        }
        if before == requirements.len() {
            diagnostics.push(format!(
                "{path}: no supported native requirements; coverage is incomplete"
            ));
        }
    }
    if requirements.is_empty() {
        diagnostics.push("no supported native requirements observed".into());
    }
    let mut baseline_status = "not_requested";
    if let Some(path) = baseline {
        let bytes = bounded_read(path).map_err(|e| format!("baseline: {e}"))?;
        let previous: Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("invalid baseline: {e}"))?;
        let valid_sources = previous["sources"].as_array().is_some_and(|s| {
            !s.is_empty()
                && s.iter().all(|v| {
                    v["path"].as_str().is_some()
                        && v["sha256"].as_str().is_some_and(|h| {
                            h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())
                        })
                })
        });
        if previous["schema"] != "specguard-native/v1"
            || previous["project"] != root.to_string_lossy().as_ref()
            || previous["source"] != source
            || previous["decision"] != "ALLOW"
            || !valid_sources
        {
            return Err("baseline must be a successful specguard-native/v1 check report for the same project and source; it is not approval evidence".into());
        }
        if previous["sources"] != json!(sources) {
            baseline_status = "drifted";
            let old = previous["sources"].as_array().unwrap();
            let old_map: std::collections::BTreeMap<_, _> = old
                .iter()
                .map(|v| (v["path"].as_str().unwrap(), &v["sha256"]))
                .collect();
            let new_map: std::collections::BTreeMap<_, _> = sources
                .iter()
                .map(|v| (v["path"].as_str().unwrap(), &v["sha256"]))
                .collect();
            let all: BTreeSet<_> = old_map.keys().chain(new_map.keys()).copied().collect();
            for path in all {
                if old_map.get(path) != new_map.get(path) {
                    findings.push(issue("baseline_drift", path, 0, "Artifact added, removed or changed since observation; review the diff and recheck. This report cannot authorize a new baseline."));
                }
            }
        } else {
            baseline_status = "unchanged";
        }
    }
    let code = if !diagnostics.is_empty() {
        3
    } else if !findings.is_empty() {
        2
    } else {
        0
    };
    Ok((
        json!({"schema":"specguard-native/v1","analyzer":"native-structure/1","project":root.to_string_lossy(),"source":source,"decision":match code {0=>"ALLOW",2=>"BLOCK",_=>"INDETERMINATE"},"approval_status":"not_evaluated","execution_status":"not_evaluated","baseline_status":baseline_status,"baseline_trust":"untrusted_observation","sources":sources,"requirements":requirements,"findings":findings,"diagnostics":diagnostics,"coverage":{"inventory_complete":diagnostics.is_empty(),"scope":roots,"excluded":["openspec/changes/archive"],"semantic_validation":"not_evaluated","traceability":"not_evaluated"}}),
        code,
    ))
}
