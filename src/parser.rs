use crate::{model::*, source::SourceSnapshot};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    format: String,
    namespace: String,
}
type ParsedSource = Result<(Vec<Requirement>, Vec<Acceptance>, Vec<TraceEdge>), (Terminal, String)>;
pub fn parse(snapshot: &SourceSnapshot) -> ParseResult {
    let mut result = ParseResult {
        api_version: Version::V1,
        snapshot_digest: snapshot.digest.clone(),
        candidate_oid: snapshot.binding.candidate_oid.clone(),
        sources: snapshot.inventory.sources.clone(),
        requirements: vec![],
        acceptances: vec![],
        edges: vec![],
    };
    let actual_digest = digest(&(&snapshot.inventory, &snapshot.binding, &snapshot.contents));
    if actual_digest != snapshot.digest {
        let mut paths: std::collections::BTreeSet<_> = snapshot
            .inventory
            .sources
            .iter()
            .map(|source| source.path.clone())
            .chain(
                snapshot
                    .inventory
                    .entries
                    .iter()
                    .map(|entry| entry.path.clone()),
            )
            .collect();
        if paths.is_empty() {
            paths.insert("<snapshot>".into());
        }
        result.sources = paths
            .into_iter()
            .map(|path| SourceStatus {
                path,
                status: Terminal::Malformed,
                reason: "aggregate snapshot digest mismatch".into(),
            })
            .collect();
        return result;
    }
    let start = std::time::Instant::now();
    for entry in &snapshot.inventory.entries {
        let parsed = (|| -> ParsedSource {
            let bytes = snapshot
                .contents
                .get(&entry.path)
                .ok_or((Terminal::IoError, "missing frozen bytes".into()))?;
            if digest(bytes) != entry.digest {
                return Err((Terminal::Malformed, "snapshot byte digest mismatch".into()));
            }
            let text = std::str::from_utf8(bytes)
                .map_err(|_| (Terminal::Malformed, "non UTF-8 source".into()))?;
            let limits = &snapshot.inventory.limits;
            if bytes.len() > limits.max_bytes
                || text.lines().count() > limits.max_lines
                || start.elapsed().as_millis() >= limits.max_millis as u128
            {
                return Err((Terminal::Limit, "parse budget exceeded".into()));
            }
            let mut lines = text.lines().enumerate();
            if lines.next().map(|(_, s)| s) != Some("---") {
                return Err((Terminal::Malformed, "required YAML frontmatter".into()));
            }
            let mut header = String::new();
            let mut closed = false;
            for (_, line) in lines.by_ref() {
                if line == "---" {
                    closed = true;
                    break;
                }
                header.push_str(line);
                header.push('\n');
            }
            if !closed {
                return Err((Terminal::Malformed, "unclosed YAML frontmatter".into()));
            }
            let h: Header =
                serde_yaml::from_str(&header).map_err(|e| (Terminal::Malformed, e.to_string()))?;
            if h.format != entry.format || h.namespace != entry.namespace {
                return Err((
                    Terminal::Unsupported,
                    "profile or namespace mismatch".into(),
                ));
            }
            let (rh, ah) = match h.format.as_str() {
                "markdown-explicit/v1" => ("## Requirement: ", "### Acceptance: "),
                "openspec-explicit/v1" => ("### Requirement: ", "#### Scenario: "),
                _ => return Err((Terminal::Unsupported, "unsupported profile".into())),
            };
            let mut reqs: Vec<Requirement> = vec![];
            let mut accs: Vec<Acceptance> = vec![];
            let mut edges = vec![];
            let mut active_acceptance = false;
            for (n, line) in lines {
                if line.chars().take_while(|c| *c == '#').count() > limits.max_depth {
                    return Err((Terminal::Limit, "heading depth budget".into()));
                }
                let source = SourceRef {
                    path: entry.path.clone(),
                    line: n + 1,
                };
                if let Some(id) = line.strip_prefix(rh) {
                    let key = identity(&h.namespace, id)?;
                    reqs.push(Requirement {
                        key,
                        text: String::new(),
                        source,
                    });
                    active_acceptance = false;
                } else if let Some(id) = line.strip_prefix(ah) {
                    let key = identity(&h.namespace, id)?;
                    let requirement = reqs
                        .last()
                        .ok_or((Terminal::Malformed, "acceptance before requirement".into()))?
                        .key
                        .clone();
                    accs.push(Acceptance {
                        key,
                        requirement,
                        text: String::new(),
                        source,
                    });
                    active_acceptance = true;
                } else if let Some(link) = line.strip_prefix("- depends_on: ") {
                    let from = reqs
                        .last()
                        .ok_or((Terminal::Malformed, "edge before requirement".into()))?
                        .key
                        .clone();
                    let (namespace, id) = link
                        .split_once(':')
                        .ok_or((Terminal::Malformed, "edge requires namespace:id".into()))?;
                    edges.push(TraceEdge {
                        from,
                        to: identity(namespace, id)?,
                        relation: Relation::DependsOn,
                        source,
                    });
                } else if line == "## ADDED Requirements" {
                    continue;
                } else if line.starts_with('#') {
                    return Err((Terminal::Malformed, "unsupported heading".into()));
                } else if active_acceptance {
                    let a = accs.last_mut().unwrap();
                    a.text.push_str(line);
                    a.text.push('\n');
                } else if let Some(r) = reqs.last_mut() {
                    r.text.push_str(line);
                    r.text.push('\n');
                } else if !line.trim().is_empty() {
                    return Err((Terminal::Malformed, "content outside requirement".into()));
                }
            }
            if reqs.is_empty() {
                return Err((Terminal::Malformed, "no explicit requirements".into()));
            }
            for r in &mut reqs {
                r.text = r.text.trim().into();
            }
            for a in &mut accs {
                a.text = a.text.trim().into();
            }
            Ok((reqs, accs, edges))
        })();
        result
            .sources
            .retain(|s| s.path != entry.path || s.status != Terminal::Complete);
        match parsed {
            Ok((r, a, e)) => {
                result.requirements.extend(r);
                result.acceptances.extend(a);
                result.edges.extend(e);
                result.sources.push(SourceStatus {
                    path: entry.path.clone(),
                    status: Terminal::Complete,
                    reason: "parsed".into(),
                });
            }
            Err((status, reason)) => result.sources.push(SourceStatus {
                path: entry.path.clone(),
                status,
                reason,
            }),
        }
    }
    result.sources.sort();
    result.requirements.sort();
    result.acceptances.sort();
    result.edges.sort();
    result
}
fn identity(namespace: &str, id: &str) -> Result<Identity, (Terminal, String)> {
    if [namespace, id].iter().any(|s| {
        s.is_empty()
            || !s
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    }) {
        return Err((Terminal::Malformed, "invalid explicit identity".into()));
    }
    Ok(Identity {
        namespace: namespace.into(),
        id: id.into(),
    })
}
