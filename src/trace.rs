//! Explicit-version local ADR/task source adapters and strict trace artifacts.
//! Stable IDs are written by the source owner, never derived from titles/lines.
use crate::{
    graph::*,
    integration::producer::preflight,
    model::*,
    parser::parse,
    rules::Finding,
    source::{SourceEntry, SourceSnapshot},
};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TraceVersion {
    #[serde(rename = "specguard.trace/v1alpha1")]
    V1,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TraceArtifact {
    pub api_version: TraceVersion,
    pub graph: TypedSpecificationGraph,
    pub required: BTreeSet<Identity>,
    pub findings: Vec<Finding>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    format: String,
    namespace: String,
}
fn identity(namespace: &str, id: &str) -> Result<Identity, String> {
    if [namespace, id].iter().any(|s| {
        s.is_empty()
            || s.len() > 1024
            || !s
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    }) {
        return Err("invalid explicit identity".into());
    }
    Ok(Identity {
        namespace: namespace.into(),
        id: id.into(),
    })
}
fn target_format(format: &str) -> bool {
    matches!(format, "markdown-adr/v1" | "markdown-task/v1")
}
fn line(text: &str, offset: usize) -> usize {
    text[..offset].bytes().filter(|b| *b == b'\n').count() + 1
}
fn targets(entry: &SourceEntry, text: &str) -> Result<Vec<TraceTarget>, String> {
    let rest = text
        .strip_prefix("---\n")
        .ok_or("required trace frontmatter")?;
    let end = rest.find("\n---\n").ok_or("unclosed trace frontmatter")?;
    let header: Header =
        serde_yaml::from_str(&rest[..end]).map_err(|_| "malformed trace header")?;
    if header.format != entry.format || header.namespace != entry.namespace {
        return Err("unsupported trace format/namespace".into());
    }
    let offset = 4 + end + 5;
    let body = &text[offset..];
    let (prefix, kind) = if entry.format == "markdown-adr/v1" {
        ("ADR: ", TargetKind::Adr)
    } else {
        ("Task: ", TargetKind::Task)
    };
    let mut result = vec![];
    let mut heading = None;
    let mut quote = 0usize;
    for (event, range) in Parser::new(body).into_offset_iter() {
        match event {
            Event::Start(Tag::BlockQuote(_)) => quote += 1,
            Event::End(TagEnd::BlockQuote(_)) => quote = quote.saturating_sub(1),
            Event::Start(Tag::Heading { level, .. }) if quote == 0 => {
                if level != HeadingLevel::H2 {
                    return Err("unsupported trace heading hierarchy".into());
                }
                heading = Some((String::new(), line(text, offset + range.start)));
            }
            Event::Text(text) if heading.is_some() => heading.as_mut().unwrap().0.push_str(&text),
            Event::End(TagEnd::Heading(_)) if quote == 0 => {
                let (text, line) = heading.take().ok_or("trace heading missing")?;
                let id = text
                    .strip_prefix(prefix)
                    .ok_or("unsupported trace heading")?;
                result.push(TraceTarget {
                    key: identity(&entry.namespace, id)?,
                    kind: kind.clone(),
                    source: SourceRef {
                        path: entry.path.clone(),
                        line,
                    },
                });
            }
            _ => {}
        }
    }
    if result.is_empty() {
        return Err("no explicit trace targets".into());
    }
    Ok(result)
}
fn links(path: &str, text: &str, requirements: &[Requirement]) -> Result<Vec<TraceEdge>, String> {
    let mut items: Vec<(usize, String)> = vec![];
    let mut edges = vec![];
    let mut quote = 0usize;
    let mut code = 0usize;
    for (event, range) in Parser::new(text).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code += 1,
            Event::End(TagEnd::CodeBlock) => code = code.saturating_sub(1),
            Event::Start(Tag::BlockQuote(_)) => quote += 1,
            Event::End(TagEnd::BlockQuote(_)) => quote = quote.saturating_sub(1),
            Event::Start(Tag::Item) if quote == 0 => {
                items.push((line(text, range.start), String::new()))
            }
            Event::Text(text) if quote == 0 && code == 0 => {
                if let Some((_, item)) = items.last_mut() {
                    item.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak if quote == 0 => {
                if let Some((_, item)) = items.last_mut() {
                    item.push('\n');
                }
            }
            Event::End(TagEnd::Item) if quote == 0 => {
                let Some((line, text)) = items.pop() else {
                    continue;
                };
                let (relation, target) = if let Some(t) = text.strip_prefix("traces_to_adr: ") {
                    (Relation::TracesToAdr, t)
                } else if let Some(t) = text.strip_prefix("traces_to_task: ") {
                    (Relation::TracesToTask, t)
                } else if text.starts_with("traces_to_") {
                    return Err("unsupported trace relation or directive syntax".into());
                } else {
                    continue;
                };
                let (namespace, id) = target
                    .trim()
                    .split_once(':')
                    .ok_or("trace requires namespace:id")?;
                let from = requirements
                    .iter()
                    .filter(|r| r.source.path == path && r.source.line <= line)
                    .max_by_key(|r| r.source.line)
                    .ok_or("trace before requirement")?;
                edges.push(TraceEdge {
                    from: from.key.clone(),
                    to: identity(namespace, id)?,
                    relation,
                    source: SourceRef {
                        path: path.into(),
                        line,
                    },
                });
            }
            _ => {}
        }
    }
    Ok(edges)
}
pub fn scan(
    snapshot: &SourceSnapshot,
    required: &BTreeSet<Identity>,
) -> Result<TraceArtifact, String> {
    preflight(&(snapshot, required))?;
    crate::integration::producer::parser_preflight(snapshot)?;
    if required.is_empty() || required.len() > 256 || snapshot.inventory.entries.len() > 4096 {
        return Err("trace scope budget".into());
    }
    for key in required {
        identity(&key.namespace, &key.id)?;
    }
    if digest(&(&snapshot.inventory, &snapshot.binding, &snapshot.contents)) != snapshot.digest {
        return Err("snapshot digest mismatch".into());
    }
    // Parse supported requirement files with the established parser. The target
    // adapters supply their own coverage; they are never labelled parsed by it.
    let target_paths: BTreeSet<_> = snapshot
        .inventory
        .entries
        .iter()
        .filter(|e| target_format(&e.format))
        .map(|e| e.path.clone())
        .collect();
    let mut subset = snapshot.clone();
    subset
        .inventory
        .entries
        .retain(|e| !target_paths.contains(&e.path));
    subset
        .inventory
        .sources
        .retain(|s| !target_paths.contains(&s.path));
    subset
        .contents
        .retain(|path, _| !target_paths.contains(path));
    subset.digest = digest(&(&subset.inventory, &subset.binding, &subset.contents));
    let mut parsed = parse(&subset);
    parsed.snapshot_digest = snapshot.digest.clone();
    let mut inventory = TraceTargets {
        nodes: vec![],
        sources: vec![],
    };
    for entry in &snapshot.inventory.entries {
        let outcome = (|| -> Result<(), String> {
            let bytes = snapshot
                .contents
                .get(&entry.path)
                .ok_or("missing frozen bytes")?;
            if digest(bytes) != entry.digest {
                return Err("entry byte digest mismatch".into());
            }
            let text = std::str::from_utf8(bytes).map_err(|_| "non UTF-8 trace source")?;
            if bytes.len() > snapshot.inventory.limits.max_bytes
                || text.lines().count() > snapshot.inventory.limits.max_lines
            {
                return Err("trace source budget".into());
            }
            if target_format(&entry.format) {
                inventory.nodes.extend(targets(entry, text)?);
            } else if parsed
                .sources
                .iter()
                .any(|s| s.path == entry.path && s.status == Terminal::Complete)
            {
                parsed
                    .edges
                    .extend(links(&entry.path, text, &parsed.requirements)?);
            }
            Ok(())
        })();
        if target_format(&entry.format) {
            // Existing conflict/error status is never discarded by successful parsing.
            inventory.sources.extend(
                snapshot
                    .inventory
                    .sources
                    .iter()
                    .filter(|s| s.path == entry.path && s.status != Terminal::Complete)
                    .cloned(),
            );
            inventory.sources.push(SourceStatus {
                path: entry.path.clone(),
                status: if outcome.is_ok() {
                    Terminal::Complete
                } else {
                    Terminal::Malformed
                },
                reason: outcome
                    .err()
                    .unwrap_or_else(|| "parsed explicit trace target".into()),
            });
        } else if let Err(reason) = outcome {
            parsed
                .sources
                .retain(|s| s.path != entry.path || s.status != Terminal::Complete);
            parsed.sources.push(SourceStatus {
                path: entry.path.clone(),
                status: Terminal::Malformed,
                reason,
            });
        }
    }
    if inventory
        .nodes
        .len()
        .saturating_add(parsed.edges.len())
        .saturating_add(parsed.requirements.len())
        .saturating_add(parsed.acceptances.len())
        > 4096
    {
        return Err("trace graph budget".into());
    }
    let graph = build_typed_graph(parsed, inventory);
    let findings = graph.validate(required);
    Ok(TraceArtifact {
        api_version: TraceVersion::V1,
        graph,
        required: required.clone(),
        findings,
    })
}
pub fn export(artifact: &TraceArtifact) -> Result<Vec<u8>, String> {
    preflight(artifact)?;
    if artifact.required.is_empty() || artifact.required.len() > 256 {
        return Err("trace required scope missing or oversized".into());
    }
    for key in &artifact.required {
        identity(&key.namespace, &key.id)?;
    }
    if artifact.graph.validate(&artifact.required) != artifact.findings {
        return Err("trace findings mismatch".into());
    }
    serde_json::to_vec(artifact).map_err(|e| e.to_string())
}
