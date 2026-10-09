use crate::model::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceRoot {
    pub path: String,
    pub format: String,
    pub namespace: String,
    pub authority: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Limits {
    pub max_files: usize,
    pub max_bytes: usize,
    pub max_lines: usize,
    pub max_depth: usize,
    pub max_millis: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_files: 1000,
            max_bytes: 1_048_576,
            max_lines: 10_000,
            max_depth: 16,
            max_millis: 5000,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourcePolicy {
    pub api_version: Version,
    pub roots: Vec<SourceRoot>,
    pub limits: Limits,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceEntry {
    pub path: String,
    pub format: String,
    pub namespace: String,
    pub digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceInventory {
    pub api_version: Version,
    pub entries: Vec<SourceEntry>,
    pub sources: Vec<SourceStatus>,
    pub limits: Limits,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CandidateBinding {
    pub candidate_oid: String,
    pub base_oid: String,
    pub object_format: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceSnapshot {
    pub api_version: Version,
    pub inventory: SourceInventory,
    pub binding: CandidateBinding,
    pub contents: BTreeMap<String, Vec<u8>>,
    pub digest: String,
}
fn supported(format: &str) -> bool {
    matches!(
        format,
        "markdown-explicit/v1"
            | "openspec-explicit/v1"
            | "openspec/1.14.1-main"
            | "openspec/1.14.1-added"
            | "openspec-identities/v1"
    )
}
fn safe(root: &Path, relative: &str) -> Result<std::path::PathBuf, String> {
    use std::path::Component;
    let rel = Path::new(relative);
    if rel.as_os_str().is_empty() || rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(format!("unsafe path: {relative}"));
    }
    let mut path = root.to_path_buf();
    for c in rel.components() {
        path.push(c);
        match std::fs::symlink_metadata(&path) {
            Ok(m) if m.file_type().is_symlink() => return Err(format!("symlink: {relative}")),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(path)
}
fn read_bounded(root: &Path, path: &str, max: usize) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let path = safe(root, path)?;
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if !f.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("not a regular file".into());
    }
    let mut bytes = Vec::new();
    f.take(max.saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > max {
        return Err("byte limit".into());
    }
    Ok(bytes)
}
pub fn discover(root: &Path, policy: &SourcePolicy) -> Result<SourceInventory, String> {
    use std::{collections::BTreeSet, time::Instant};
    let start = Instant::now();
    let mut entries = Vec::new();
    let mut sources = Vec::new();
    let mut seen = BTreeSet::new();
    let mut authorities = BTreeMap::new();
    for config in &policy.roots {
        safe(root, &config.path)?;
        if !supported(&config.format) {
            sources.push(SourceStatus {
                path: config.path.clone(),
                status: Terminal::Unsupported,
                reason: "unsupported profile".into(),
            });
            continue;
        }
        if config.namespace.is_empty() || config.authority.is_empty() {
            return Err("empty namespace/authority".into());
        }
        if let Some(previous) =
            authorities.insert(config.namespace.clone(), config.authority.clone())
            && previous != config.authority
        {
            sources.push(SourceStatus {
                path: config.path.clone(),
                status: Terminal::Conflict,
                reason: "multiple authorities in namespace".into(),
            });
        }
        let mut pending = vec![(config.path.clone(), 0)];
        let initial = entries.len();
        let mut visited = 0usize;
        while let Some((relative, depth)) = pending.pop() {
            visited += 1;
            if entries.len() >= policy.limits.max_files
                || visited > policy.limits.max_files.saturating_mul(2).saturating_add(1)
                || depth > policy.limits.max_depth
                || start.elapsed().as_millis() >= policy.limits.max_millis as u128
            {
                sources.push(SourceStatus {
                    path: relative,
                    status: Terminal::Limit,
                    reason: "discovery budget exceeded; remaining scope unknown".into(),
                });
                break;
            }
            let path = safe(root, &relative)?;
            let metadata = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(e) => {
                    sources.push(SourceStatus {
                        path: relative,
                        status: Terminal::IoError,
                        reason: e.to_string(),
                    });
                    continue;
                }
            };
            if metadata.is_dir() {
                let list = std::fs::read_dir(path).map_err(|e| e.to_string())?;
                let mut children = Vec::new();
                for child in list.take(policy.limits.max_files.saturating_add(1)) {
                    let child = child.map_err(|e| e.to_string())?;
                    let name = child
                        .file_name()
                        .into_string()
                        .map_err(|_| "non UTF-8 path")?;
                    children.push(format!("{relative}/{name}"));
                }
                children.sort();
                if children.len() > policy.limits.max_files {
                    sources.push(SourceStatus {
                        path: relative,
                        status: Terminal::Limit,
                        reason: "directory entry budget".into(),
                    });
                    continue;
                }
                pending.extend(children.into_iter().rev().map(|p| (p, depth + 1)));
                continue;
            }
            if !seen.insert(relative.clone()) {
                sources.push(SourceStatus {
                    path: relative,
                    status: Terminal::Conflict,
                    reason: "overlapping source roots".into(),
                });
                continue;
            }
            match read_bounded(root, &relative, policy.limits.max_bytes) {
                Ok(bytes) => {
                    entries.push(SourceEntry {
                        path: relative.clone(),
                        format: config.format.clone(),
                        namespace: config.namespace.clone(),
                        digest: digest(&bytes),
                    });
                    sources.push(SourceStatus {
                        path: relative,
                        status: Terminal::Complete,
                        reason: "discovered".into(),
                    });
                }
                Err(e) => sources.push(SourceStatus {
                    path: relative,
                    status: if e == "byte limit" {
                        Terminal::Limit
                    } else {
                        Terminal::IoError
                    },
                    reason: e,
                }),
            }
        }
        if entries.len() == initial && !sources.iter().any(|s| s.path == config.path) {
            sources.push(SourceStatus {
                path: config.path.clone(),
                status: Terminal::IoError,
                reason: "no sources".into(),
            });
        }
    }
    if policy.roots.is_empty() {
        sources.push(SourceStatus {
            path: "<scope>".into(),
            status: Terminal::IoError,
            reason: "no configured roots".into(),
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    sources.sort();
    Ok(SourceInventory {
        api_version: Version::V1,
        entries,
        sources,
        limits: policy.limits.clone(),
    })
}
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("Git object verification failed".into());
    }
    Ok(String::from_utf8(out.stdout)
        .map_err(|e| e.to_string())?
        .trim()
        .into())
}
pub fn freeze(
    root: &Path,
    inventory: &SourceInventory,
    binding: CandidateBinding,
) -> Result<SourceSnapshot, String> {
    let actual = git(root, &["rev-parse", "--show-object-format"])?;
    if actual != binding.object_format {
        return Err("Git object format mismatch".into());
    }
    let len = match actual.as_str() {
        "sha1" => 40,
        "sha256" => 64,
        _ => return Err("unsupported Git object format".into()),
    };
    for oid in [&binding.candidate_oid, &binding.base_oid] {
        if oid.len() != len
            || !oid
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err("invalid full Git OID".into());
        }
        if git(root, &["cat-file", "-t", oid])? != "commit" {
            return Err("OID is not a commit".into());
        }
    }
    let mut contents = BTreeMap::new();
    for entry in &inventory.entries {
        let bytes = read_bounded(root, &entry.path, inventory.limits.max_bytes)?;
        if digest(&bytes) != entry.digest {
            return Err(format!("source drift: {}", entry.path));
        }
        contents.insert(entry.path.clone(), bytes);
    }
    // A second pass detects changes while the scope was read; parsed bytes are owned and immutable.
    for entry in &inventory.entries {
        if digest(&read_bounded(
            root,
            &entry.path,
            inventory.limits.max_bytes,
        )?) != entry.digest
        {
            return Err(format!("source drift: {}", entry.path));
        }
    }
    let snapshot_digest = digest(&(inventory, &binding, &contents));
    Ok(SourceSnapshot {
        api_version: Version::V1,
        inventory: inventory.clone(),
        binding,
        contents,
        digest: snapshot_digest,
    })
}
