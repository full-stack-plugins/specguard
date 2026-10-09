//! Native OpenSpec 1.14.1 canonical main/ADDED parsing. IDs are externally explicit,
//! and the registry is part of the same immutable source snapshot as the Markdown.
use super::{ParsedSource, identity};
use crate::{
    model::*,
    source::{Limits, SourceEntry, SourceSnapshot},
};
use pulldown_cmark::{Event, Parser, Tag};
use serde::Deserialize;
use std::{collections::BTreeSet, ops::Range, time::Instant};
type ParseError = (Terminal, String);
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct IdRegistry {
    api_version: String,
    documents: Vec<DocumentIds>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DocumentIds {
    path: String,
    requirements: Vec<RequirementIds>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequirementIds {
    title: String,
    id: String,
    scenarios: Vec<ScenarioId>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioId {
    title: String,
    id: String,
}
pub(super) type Registry = Vec<(String, DocumentIds)>;
fn malformed(message: impl Into<String>) -> ParseError {
    (Terminal::Malformed, message.into())
}
fn deadline(start: Instant, limits: &Limits) -> Result<(), ParseError> {
    if start.elapsed().as_millis() >= limits.max_millis as u128 {
        Err((Terminal::Limit, "native parse time budget".into()))
    } else {
        Ok(())
    }
}
pub(super) fn load_registry(
    snapshot: &SourceSnapshot,
    start: Instant,
) -> Result<Registry, ParseError> {
    let mut registry = vec![];
    let mut paths = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for entry in snapshot
        .inventory
        .entries
        .iter()
        .filter(|e| e.format == "openspec-identities/v1")
    {
        deadline(start, &snapshot.inventory.limits)?;
        let bytes = snapshot
            .contents
            .get(&entry.path)
            .ok_or_else(|| malformed("missing identity registry bytes"))?;
        if bytes.len() > snapshot.inventory.limits.max_bytes {
            return Err((Terminal::Limit, "identity registry byte budget".into()));
        }
        if digest(bytes) != entry.digest {
            return Err(malformed("identity registry byte digest mismatch"));
        }
        let data: IdRegistry = serde_json::from_slice(bytes)
            .map_err(|e| malformed(format!("identity registry: {e}")))?;
        if data.api_version != "specguard.openspec-ids/v1" {
            return Err((
                Terminal::Unsupported,
                "unknown native identity registry version".into(),
            ));
        }
        for doc in data.documents {
            if !paths.insert((entry.namespace.clone(), doc.path.clone())) {
                return Err((
                    Terminal::Conflict,
                    "duplicate native document registry".into(),
                ));
            }
            if !snapshot.inventory.entries.iter().any(|source| {
                source.path == doc.path
                    && source.namespace == entry.namespace
                    && source.format.starts_with("openspec/1.14.1-")
            }) {
                return Err(malformed(
                    "identity registry names an undiscovered native source",
                ));
            }
            let mut titles = BTreeSet::new();
            for req in &doc.requirements {
                let key = identity(&entry.namespace, &req.id)?;
                if req.title.trim().is_empty() || !titles.insert(&req.title) || !ids.insert(key) {
                    return Err((
                        Terminal::Conflict,
                        "duplicate or empty native requirement identity/title".into(),
                    ));
                }
                let mut scenarios = BTreeSet::new();
                for scenario in &req.scenarios {
                    let key = identity(&entry.namespace, &scenario.id)?;
                    if scenario.title.trim().is_empty()
                        || !scenarios.insert(&scenario.title)
                        || !ids.insert(key)
                    {
                        return Err((
                            Terminal::Conflict,
                            "duplicate or empty native scenario identity/title".into(),
                        ));
                    }
                }
            }
            registry.push((entry.namespace.clone(), doc));
        }
    }
    Ok(registry)
}
struct Heading {
    level: usize,
    title: String,
    line: usize,
    start: usize,
    body: usize,
}
struct Structure {
    headings: Vec<Heading>,
    code: Vec<Range<usize>>,
}
fn structure(text: &str, limits: &Limits, start: Instant) -> Result<Structure, ParseError> {
    let mut headings = vec![];
    let mut code = vec![];
    let mut depth = 0usize;
    for (nodes, (event, range)) in Parser::new(text).into_offset_iter().enumerate() {
        deadline(start, limits)?;
        if nodes >= limits.max_lines.saturating_mul(32) {
            return Err((Terminal::Limit, "Markdown AST node budget".into()));
        }
        match event {
            Event::Start(tag) => {
                depth += 1;
                if depth > limits.max_depth {
                    return Err((Terminal::Limit, "Markdown AST depth budget".into()));
                }
                match tag {
                    Tag::Heading { level, .. } if depth == 1 => {
                        let end = text[range.start..]
                            .find('\n')
                            .map_or(text.len(), |n| range.start + n);
                        let raw = &text[range.start..end];
                        // Canonical native headers are ATX, at column zero. Setext and nested
                        // headings are not silently promoted to canonical specification nodes.
                        if !raw.starts_with('#') {
                            return Err((
                                Terminal::Unsupported,
                                "native profile requires canonical ATX headings".into(),
                            ));
                        }
                        let title = normalize_title(raw.trim_start_matches('#').trim());
                        headings.push(Heading {
                            level: level as usize,
                            title,
                            line: text[..range.start].bytes().filter(|c| *c == b'\n').count() + 1,
                            start: range.start,
                            body: if end < text.len() { end + 1 } else { end },
                        });
                    }
                    Tag::CodeBlock(_) => code.push(range),
                    _ => {}
                }
            }
            Event::Html(_) | Event::InlineHtml(_) => {
                return Err((
                    Terminal::Unsupported,
                    "HTML blocks/inline HTML require an explicitly verified native capability"
                        .into(),
                ));
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(Structure { headings, code })
}
fn normalize_title(title: &str) -> String {
    let trimmed = title.trim_end_matches([' ', '\t']);
    let stripped = trimmed.trim_end_matches('#');
    if stripped.len() < trimmed.len() && stripped.ends_with([' ', '\t']) {
        stripped.trim().into()
    } else {
        trimmed.trim().into()
    }
}
fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map(|_| text[prefix.len()..].trim())
}
fn requirement_body(text: &str, range: Range<usize>, code: &[Range<usize>]) -> String {
    let mut body = vec![];
    let mut metadata = vec![];
    let mut position = range.start;
    for line in text[range.clone()].split_inclusive('\n') {
        let inside_code = code.iter().any(|block| block.contains(&position));
        position += line.len();
        if inside_code {
            continue;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("**") && line[2..].contains("**:") {
            metadata.push(line)
        } else {
            body.push(line)
        }
    }
    if body.is_empty() {
        metadata.join("\n")
    } else {
        body.join("\n")
    }
}
pub(super) fn parse_native(
    entry: &SourceEntry,
    text: &str,
    registry: &Registry,
    limits: &Limits,
    start: Instant,
) -> ParsedSource {
    let normalized = text
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let text = normalized.as_str();
    if text.lines().count() > limits.max_lines {
        return Err((Terminal::Limit, "normalized native line budget".into()));
    }
    let structure = structure(text, limits, start)?;
    let headings = &structure.headings;
    let doc = registry
        .iter()
        .find(|(namespace, doc)| namespace == &entry.namespace && doc.path == entry.path)
        .map(|(_, doc)| doc)
        .ok_or_else(|| malformed("missing explicit native identity registry"))?;
    let required_section = match entry.format.as_str() {
        "openspec/1.14.1-main" => "Requirements",
        "openspec/1.14.1-added" => "ADDED Requirements",
        _ => {
            return Err((
                Terminal::Unsupported,
                "unknown native OpenSpec profile".into(),
            ));
        }
    };
    for header in headings {
        if header.level == 2
            && [
                "MODIFIED Requirements",
                "REMOVED Requirements",
                "RENAMED Requirements",
            ]
            .iter()
            .any(|v| header.title.eq_ignore_ascii_case(v))
        {
            return Err((
                Terminal::Unsupported,
                "native delta operation requires baseline-aware application".into(),
            ));
        }
    }
    let sections: Vec<_> = headings
        .iter()
        .enumerate()
        .filter(|(_, h)| h.level == 2 && h.title.eq_ignore_ascii_case(required_section))
        .collect();
    if sections.len() != 1 {
        return Err(malformed(
            "native spec requires exactly one requirements section",
        ));
    }
    let (section_index, section) = sections[0];
    let end = headings[section_index + 1..]
        .iter()
        .find(|h| h.level <= 2)
        .map_or(text.len(), |h| h.start);
    if entry.format == "openspec/1.14.1-main" {
        if headings
            .iter()
            .any(|h| h.level == 2 && h.title.eq_ignore_ascii_case("ADDED Requirements"))
        {
            return Err((
                Terminal::Unsupported,
                "delta section in native main spec".into(),
            ));
        }
        let purpose = headings
            .iter()
            .enumerate()
            .find(|(_, h)| h.level == 2 && h.title.eq_ignore_ascii_case("Purpose"))
            .ok_or_else(|| malformed("native main spec requires Purpose"))?;
        let purpose_end = headings[purpose.0 + 1..]
            .iter()
            .find(|h| h.level <= 2)
            .map_or(text.len(), |h| h.start);
        if text[purpose.1.body..purpose_end].trim().is_empty() {
            return Err(malformed("empty native purpose"));
        }
    }
    if headings.iter().any(|h| {
        h.level == 3
            && strip_prefix_ci(&h.title, "Requirement:").is_some()
            && (h.start < section.body || h.start >= end)
    }) {
        return Err(malformed("native requirement outside declared scope"));
    }
    let mut reqs = vec![];
    let mut accs = vec![];
    let mut used_requirements = BTreeSet::new();
    let requirements: Vec<_> = headings
        .iter()
        .enumerate()
        .filter(|(_, h)| h.level == 3 && h.start >= section.body && h.start < end)
        .collect();
    for (index, header) in requirements {
        deadline(start, limits)?;
        let title = strip_prefix_ci(&header.title, "Requirement:")
            .ok_or_else(|| malformed("noncanonical native requirement heading"))?;
        let mapped = doc
            .requirements
            .iter()
            .find(|r| r.title == title)
            .ok_or_else(|| malformed(format!("unmapped native requirement: {title}")))?;
        if !used_requirements.insert(title) {
            return Err((
                Terminal::Conflict,
                "duplicate native requirement title".into(),
            ));
        }
        let req_end = headings[index + 1..]
            .iter()
            .find(|h| h.level <= 3)
            .map_or(end, |h| h.start.min(end));
        let body_end = headings[index + 1..]
            .first()
            .map_or(req_end, |h| h.start.min(req_end));
        let body = requirement_body(text, header.body..body_end, &structure.code);
        if !body
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .any(|word| matches!(word, "SHALL" | "MUST"))
        {
            return Err(malformed("native requirement body needs SHALL or MUST"));
        }
        let key = identity(&entry.namespace, &mapped.id)?;
        let mut used_scenarios = BTreeSet::new();
        for (scenario_index, scenario) in headings
            .iter()
            .enumerate()
            .filter(|(_, h)| h.level == 4 && h.start >= header.body && h.start < req_end)
        {
            let name = strip_prefix_ci(&scenario.title, "Scenario:").unwrap_or(&scenario.title);
            let id = mapped
                .scenarios
                .iter()
                .find(|s| s.title == name)
                .ok_or_else(|| malformed(format!("unmapped native scenario: {name}")))?;
            if !used_scenarios.insert(name) {
                return Err((Terminal::Conflict, "duplicate native scenario title".into()));
            }
            let scenario_end = headings[scenario_index + 1..]
                .iter()
                .find(|h| h.level <= 4)
                .map_or(req_end, |h| h.start.min(req_end));
            let body = text[scenario.body..scenario_end].trim();
            if body.is_empty() {
                return Err(malformed("empty native scenario"));
            }
            accs.push(Acceptance {
                key: identity(&entry.namespace, &id.id)?,
                requirement: key.clone(),
                text: body.into(),
                source: SourceRef {
                    path: entry.path.clone(),
                    line: scenario.line,
                },
            });
        }
        if used_scenarios.is_empty() || used_scenarios.len() != mapped.scenarios.len() {
            return Err(malformed(
                "missing native scenarios or stale scenario identities",
            ));
        }
        reqs.push(Requirement {
            key,
            text: body,
            source: SourceRef {
                path: entry.path.clone(),
                line: header.line,
            },
        });
    }
    if reqs.is_empty() || used_requirements.len() != doc.requirements.len() {
        return Err(malformed(
            "missing native requirements or stale requirement identities",
        ));
    }
    Ok((reqs, accs, vec![]))
}
