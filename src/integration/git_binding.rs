//! Exact committed source bytes via GitGuard. Still local advisory, not queue admission.
use super::{producer::*, runtime::CancellationToken};
use crate::{model::*, source::*};
use gitguard::{Repository, candidate::CandidateSnapshot};
use guardengine::integration::{RunBinding, RunStatus};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Component};
const VERSION: &str = "specguard.git-source/v1alpha1";
#[derive(Clone, Serialize)]
pub struct GitSourcePolicy {
    source: SourcePolicy,
    required: BTreeSet<Identity>,
    mapping: ProtectedMapping,
}
impl GitSourcePolicy {
    pub fn freeze(
        source: SourcePolicy,
        required: BTreeSet<Identity>,
        mapping: ProtectedMapping,
    ) -> Result<Self, String> {
        preflight(&(&source, &required, &mapping))?;
        mapping.validate(&required)?;
        if required.is_empty() || required.len() > 64 {
            return Err("Git source scope invalid".into());
        }
        Ok(Self {
            source,
            required,
            mapping,
        })
    }
    pub fn digest(&self) -> String {
        digest(&(VERSION, self))
    }
}
pub struct Attempt {
    pub run_id: String,
    pub started_at: String,
}
fn check(
    repo: &Repository,
    candidate: &CandidateSnapshot,
    policy: &GitSourcePolicy,
) -> Result<(), String> {
    preflight(&(candidate, policy))?;
    struct ContextCounter(usize);
    impl std::io::Write for ContextCounter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            if self.0 > 65536 {
                return Err(std::io::Error::other("candidate context budget"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(ContextCounter(0), candidate).map_err(|_| "candidate context budget")?;
    if candidate.members().len() > 64
        || candidate.allowed_paths().len() > 1024
        || candidate.requirement_ids().len() > 64
    {
        return Err("candidate context count budget".into());
    }

    candidate
        .validate(repo)
        .map_err(|_| "invalid GitGuard candidate")?;
    let mut expected_ids: Vec<_> = policy
        .required
        .iter()
        .map(|k| format!("{}:{}", k.namespace, k.id))
        .collect();
    expected_ids.sort();
    if !candidate.advisory()
        || policy.digest().strip_prefix("sha256:") != Some(candidate.policy_digest())
        || candidate.requirement_ids() != expected_ids
    {
        return Err("Git candidate scope/policy mismatch".into());
    }
    Ok(())
}
fn snapshot(
    repo: &Repository,
    candidate: &CandidateSnapshot,
    policy: &GitSourcePolicy,
) -> Result<(SourceSnapshot, String), String> {
    check(repo, candidate, policy)?;
    let files = repo
        .read_commit_files(candidate.candidate_oid())
        .map_err(|_| "Git committed files unavailable")?;
    let borrowed: Vec<_> = files
        .iter()
        .map(|f| (f.path(), f.mode(), f.object_oid(), f.contents()))
        .collect();
    preflight(&borrowed)?;
    let files_digest = digest(&borrowed);
    let temp = tempfile::tempdir().map_err(|_| "private source unavailable")?;
    for file in &files {
        let path =
            std::str::from_utf8(file.path()).map_err(|_| "unsupported non-UTF8 candidate path")?;
        if path.split('/').count() > 64
            || path.contains(['\\', ':'])
            || std::path::Path::new(path)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || path.split('/').any(|c| c == ".git")
        {
            return Err("unsupported candidate path".into());
        }
        let destination = temp.path().join(path);
        std::fs::create_dir_all(destination.parent().ok_or("invalid path")?)
            .map_err(|_| "private source directory error")?;
        std::fs::write(&destination, file.contents()).map_err(|_| "private source write error")?;
    }
    let inventory = discover(temp.path(), &policy.source)?;
    let binding = CandidateBinding {
        candidate_oid: candidate.candidate_oid().into(),
        base_oid: candidate.base_oid().into(),
        object_format: repo.object_format().into(),
    };
    let snapshot = freeze_checked(temp.path(), &inventory, binding, |_| Ok(()))?;
    // Full candidate tree read above is authoritative; every selected source must match it.
    for (path, bytes) in &snapshot.contents {
        if !files
            .iter()
            .any(|f| f.path() == path.as_bytes() && f.contents() == bytes)
        {
            return Err("selected source differs from committed bytes".into());
        }
    }
    Ok((snapshot, files_digest))
}
pub struct GitPreparedRun {
    prepared: PreparedRun,
    candidate: CandidateSnapshot,
    files_digest: String,
    policy_digest: String,
    object_format: String,
}
impl GitPreparedRun {
    /// Additionally require GG's observed worktree/index to match the exact candidate tree.
    /// This is a local cleanliness observation, not authenticated queue admission.
    pub fn prepare_clean(
        repo: &Repository,
        candidate: &CandidateSnapshot,
        policy: &GitSourcePolicy,
        attempt: Attempt,
    ) -> Result<Self, String> {
        if !candidate.clean() {
            return Err("candidate source worktree is dirty".into());
        }
        Self::prepare(repo, candidate, policy, attempt)
    }

    pub fn binding(&self) -> &RunBinding {
        self.prepared.binding()
    }
    pub fn work_key(&self) -> &super::freshness::WorkKey {
        self.prepared.work_key()
    }
    pub fn prepare(
        repo: &Repository,
        candidate: &CandidateSnapshot,
        policy: &GitSourcePolicy,
        attempt: Attempt,
    ) -> Result<Self, String> {
        preflight(&(&attempt.run_id, &attempt.started_at))?;
        let (snapshot, files_digest) = snapshot(repo, candidate, policy)?;
        let invocation = Invocation {
            run_id: attempt.run_id,
            started_at: attempt.started_at,
            repo_id: candidate.repo_id().into(),
            task_id: candidate.task_id().into(),
            worktree_id: candidate.worktree_id().into(),
            candidate_oid: Some(candidate.candidate_oid().into()),
            base_oid: Some(candidate.base_oid().into()),
            merge_group_id: candidate.merge_group_id().map(str::to_owned),
            baseline_digest: candidate.baseline_digest().map(|d| format!("sha256:{d}")),
        };
        let profile = format!("{VERSION}:{}", candidate.binding_digest());
        let prepared = prepare_checked(
            &snapshot,
            invocation,
            &policy.required,
            &policy.mapping,
            &profile,
            |_| {
                candidate
                    .validate(repo)
                    .map_err(|_| "invalid committed binding".into())
            },
        )
        .map_err(|e| format!("{}: {}", e.code, e.message))?;
        Ok(Self {
            prepared,
            candidate: candidate.clone(),
            files_digest,
            policy_digest: policy.digest(),
            object_format: repo.object_format().into(),
        })
    }
    pub fn execute(
        self,
        finished: &str,
        token: &CancellationToken,
    ) -> Result<GitEvidenceBundle, String> {
        self.execute_with_cache(finished, token, &mut crate::cache::ParseCache::disabled())
    }
    pub fn execute_with_cache(
        self,
        finished: &str,
        token: &CancellationToken,
        cache: &mut crate::cache::ParseCache,
    ) -> Result<GitEvidenceBundle, String> {
        let output = self
            .prepared
            .execute_with_cache(finished, token, |_| {}, cache)
            .map_err(|e| format!("{}: {}", e.code, e.message))?;
        Ok(GitEvidenceBundle {
            api_version: VERSION.into(),
            candidate: self.candidate,
            files_digest: self.files_digest,
            policy_digest: self.policy_digest,
            object_format: self.object_format,
            output,
        })
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitEvidenceBundle {
    api_version: String,
    candidate: CandidateSnapshot,
    files_digest: String,
    policy_digest: String,
    object_format: String,
    output: ProducedRun,
}
impl GitEvidenceBundle {
    pub fn output(&self) -> &ProducedRun {
        &self.output
    }
    pub fn candidate(&self) -> &CandidateSnapshot {
        &self.candidate
    }
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        preflight(self)?;
        serde_json::to_vec(self).map_err(|e| e.to_string())
    }
    pub fn load(
        bytes: &[u8],
        repo: &Repository,
        expected: &CandidateSnapshot,
        policy: &GitSourcePolicy,
        expected_binding: &RunBinding,
    ) -> Result<Self, String> {
        if bytes.len() > guardengine::integration::MAX_ARTIFACT_BYTES {
            return Err("Git source bundle byte budget".into());
        }
        let result: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        result.verify(repo, expected, policy, expected_binding)?;
        Ok(result)
    }
    pub fn verify(
        &self,
        repo: &Repository,
        expected: &CandidateSnapshot,
        policy: &GitSourcePolicy,
        expected_binding: &RunBinding,
    ) -> Result<(), String> {
        preflight(self)?;
        check(repo, expected, policy)?;
        if self.api_version != VERSION
            || self.object_format != repo.object_format()
            || self.policy_digest != policy.digest()
            || self.candidate.binding_digest() != expected.binding_digest()
            || &self.output.envelope.binding != expected_binding
        {
            return Err("Git source expected binding mismatch".into());
        }
        self.output.verify()?;
        let prepared = GitPreparedRun::prepare(
            repo,
            expected,
            policy,
            Attempt {
                run_id: self.output.envelope.run_id.clone(),
                started_at: self.output.envelope.started_at.clone(),
            },
        )?;
        if prepared.binding() != expected_binding || prepared.files_digest != self.files_digest {
            return Err("actual candidate source binding mismatch".into());
        }
        if self.output.envelope.run_status == RunStatus::Completed {
            let actual =
                prepared.execute(&self.output.envelope.finished_at, &CancellationToken::new())?;
            if actual.output.contract != self.output.contract
                || actual.output.facts != self.output.facts
                || actual.output.report != self.output.report
                || actual.output.domain != self.output.domain
            {
                return Err("candidate analysis differs from committed source".into());
            }
        }
        Ok(())
    }
}
