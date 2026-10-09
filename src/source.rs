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
            | "markdown-adr/v1"
            | "markdown-task/v1"
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
#[cfg(unix)]
fn open_regular(path: &Path, directory: bool) -> Result<std::fs::File, String> {
    use std::os::{
        fd::{AsRawFd, FromRawFd},
        unix::ffi::OsStrExt,
    };
    use std::path::Component;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let parts: Vec<_> = absolute
        .components()
        .filter(|c| !matches!(c, Component::RootDir | Component::CurDir))
        .collect();
    let mut handle = std::fs::File::open("/").map_err(|e| e.to_string())?;
    for (index, part) in parts.iter().enumerate() {
        let Component::Normal(name) = part else {
            return Err("unsafe source path".into());
        };
        let name = std::ffi::CString::new(name.as_bytes()).map_err(|_| "NUL path")?;
        let is_dir = index + 1 < parts.len() || directory;
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if is_dir { libc::O_DIRECTORY } else { 0 };
        // SAFETY: the parent descriptor and NUL-terminated name live through openat;
        // ownership of the returned descriptor transfers once to File below.
        let fd = unsafe { libc::openat(handle.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(format!(
                "unsafe or unreadable path {}: {}",
                path.display(),
                std::io::Error::last_os_error()
            ));
        }
        handle = unsafe { std::fs::File::from_raw_fd(fd) };
    }
    let metadata = handle.metadata().map_err(|e| e.to_string())?;
    if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err("not a regular source file/directory".into());
    }
    Ok(handle)
}
#[cfg(not(unix))]
fn open_regular(_: &Path, _: bool) -> Result<std::fs::File, String> {
    Err("descriptor-safe source capture unsupported on this platform".into())
}

#[derive(PartialEq, Eq)]
struct FileStamp {
    length: u64,
    modified: std::time::SystemTime,
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
}
fn stamp(file: &std::fs::File) -> Result<FileStamp, String> {
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    Ok(FileStamp {
        length: metadata.len(),
        modified: metadata.modified().map_err(|e| e.to_string())?,
        #[cfg(unix)]
        identity: (
            metadata.dev(),
            metadata.ino(),
            metadata.ctime(),
            metadata.ctime_nsec(),
        ),
    })
}
struct CapturedFile {
    bytes: Vec<u8>,
    stamp: FileStamp,
}
impl CapturedFile {
    fn read(root: &Path, relative: &str, max: usize) -> Result<Self, String> {
        use std::io::Read;
        let path = safe(root, relative)?;
        let mut file = open_regular(&path, false)?;
        let before = stamp(&file)?;
        let mut bytes = Vec::new();
        (&mut file)
            .take(max.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > max {
            return Err("byte limit".into());
        }
        if before != stamp(&file)? {
            return Err(format!("source drift: {relative}"));
        }
        Ok(Self {
            bytes,
            stamp: before,
        })
    }
    fn verify(&self, root: &Path, relative: &str, max: usize) -> Result<(), String> {
        let current = Self::read(root, relative, max)?;
        if self.stamp != current.stamp || self.bytes != current.bytes {
            return Err(format!("source drift: {relative}"));
        }
        Ok(())
    }
}
fn read_bounded(root: &Path, path: &str, max: usize) -> Result<Vec<u8>, String> {
    Ok(CapturedFile::read(root, path, max)?.bytes)
}
pub fn discover(root: &Path, policy: &SourcePolicy) -> Result<SourceInventory, String> {
    use std::{collections::BTreeSet, time::Instant};
    let _root_handle = open_regular(root, true)?;
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
    freeze_checked(root, inventory, binding, |binding| {
        verify_candidate_objects(root, binding)
    })
}
pub(crate) fn freeze_checked(
    root: &Path,
    inventory: &SourceInventory,
    binding: CandidateBinding,
    verify: impl FnOnce(&CandidateBinding) -> Result<(), String>,
) -> Result<SourceSnapshot, String> {
    let root_handle = open_regular(root, true)?;
    let root_stamp = stamp(&root_handle)?;
    if inventory.entries.len() > inventory.limits.max_files {
        return Err("snapshot file limit".into());
    }
    verify(&binding)?;
    let mut captured = BTreeMap::new();
    for entry in &inventory.entries {
        if captured.contains_key(&entry.path) {
            return Err(format!("duplicate inventory path: {}", entry.path));
        }
        let file = CapturedFile::read(root, &entry.path, inventory.limits.max_bytes)?;
        if digest(&file.bytes) != entry.digest {
            return Err(format!("source drift: {}", entry.path));
        }
        captured.insert(entry.path.clone(), file);
    }
    // Validate all owned captures after the read phase. Identity and ctime catch
    // same-byte inode replacement and mutate/restore, in addition to digest drift.
    for (path, file) in &captured {
        file.verify(root, path, inventory.limits.max_bytes)?;
    }
    if stamp(&open_regular(root, true)?)? != root_stamp {
        return Err("source root drift".into());
    }
    let contents: BTreeMap<_, _> = captured
        .into_iter()
        .map(|(path, file)| (path, file.bytes))
        .collect();
    let snapshot_digest = digest(&(inventory, &binding, &contents));
    Ok(SourceSnapshot {
        api_version: Version::V1,
        inventory: inventory.clone(),
        binding,
        contents,
        digest: snapshot_digest,
    })
}

#[cfg(all(test, unix))]
mod snapshot_stability_tests {
    #[test]
    fn identical_bytes_replacement_and_restore_are_detected() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a.md");
        std::fs::write(&path, b"original").unwrap();
        let read = super::CapturedFile::read(root.path(), "a.md", 1024).unwrap();
        std::fs::write(root.path().join("replacement"), b"original").unwrap();
        std::fs::rename(root.path().join("replacement"), &path).unwrap();
        assert!(read.verify(root.path(), "a.md", 1024).is_err());
        let read = super::CapturedFile::read(root.path(), "a.md", 1024).unwrap();
        std::fs::write(&path, b"mutated").unwrap();
        std::fs::write(&path, b"original").unwrap();
        assert!(read.verify(root.path(), "a.md", 1024).is_err());
    }
}

/// Read-only Git object validation; not producer authentication or clean-tree proof.
pub fn verify_candidate_objects(root: &Path, binding: &CandidateBinding) -> Result<(), String> {
    let _root = open_regular(root, true)?;
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
    Ok(())
}
