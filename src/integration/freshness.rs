//! Process-local, append-only history. Current means latest local result, never eligibility.
use super::{
    producer::{PreparedRun, ProducedRun},
    runtime::CancellationToken,
};
use crate::model::SourceStatus;
use guardengine::integration::{InvocationDraft, TransportDiagnostic};
use serde::Serialize;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct RunTarget {
    repo: String,
    task: String,
    worktree: String,
    requirements: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkKey {
    target: RunTarget,
    digest: String,
}
impl WorkKey {
    pub fn target(&self) -> &RunTarget {
        &self.target
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub(crate) fn freeze(
        draft: &InvocationDraft,
        source: &crate::source::SourceSnapshot,
        required: &std::collections::BTreeSet<crate::model::Identity>,
        mapping: &super::producer::ProtectedMapping,
        profile: &str,
    ) -> Self {
        let binding = draft
            .binding
            .as_ref()
            .expect("producer constructs full binding");
        // Retry run ID and observation timestamp are attempt identity, not reusable work identity.
        let identity = (
            &draft.binding,
            &draft.producer,
            &draft.coverage,
            "engine-backed",
            source,
            required,
            mapping,
            profile,
        );
        use sha2::{Digest, Sha256};
        struct HashWriter(Sha256);
        impl std::io::Write for HashWriter {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.update(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut writer = HashWriter(Sha256::new());
        serde_json::to_writer(&mut writer, &identity).expect("bounded serializable work inputs");
        Self {
            target: RunTarget {
                repo: binding.repo_id.clone(),
                task: binding.task_id.clone(),
                worktree: binding.worktree_id.clone(),
                requirements: binding.requirement_ids.clone(),
            },
            digest: format!("sha256:{:x}", writer.0.finalize()),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryError {
    Conflict,
    Stale,
    ForeignCompletion,
    UnknownRun,
    Capacity,
    InvalidCompletion,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendResult {
    Inserted,
    IdenticalReplay,
}
#[derive(Clone)]
struct Ticket {
    store: Arc<()>,
    run: String,
    key: WorkKey,
    generation: u64,
}
pub struct RegisteredRun {
    prepared: PreparedRun,
    ticket: Ticket,
}
impl RegisteredRun {
    pub fn work_key(&self) -> &WorkKey {
        &self.ticket.key
    }
    pub fn execute(
        self,
        finished: &str,
        token: &CancellationToken,
        observe: impl FnMut(&SourceStatus),
    ) -> Result<Completion, TransportDiagnostic> {
        let output = self.prepared.execute(finished, token, observe)?;
        Ok(Completion {
            ticket: self.ticket,
            output: Arc::new(output),
        })
    }
}
/// Can only be created by executing a registered frozen run. No raw-envelope import.
/// ```compile_fail
/// use specguard::integration::freshness::Completion;
/// let forged = Completion { ticket: todo!(), output: todo!() };
/// ```
/// ```compile_fail
/// use specguard::integration::{freshness::RunHistory, producer::ProducedRun};
/// fn import_raw(store: &mut RunHistory, raw: &ProducedRun) { store.append(raw); }
/// ```
pub struct Completion {
    ticket: Ticket,
    output: Arc<ProducedRun>,
}
impl Completion {
    pub fn output(&self) -> &ProducedRun {
        &self.output
    }
    pub fn work_key(&self) -> &WorkKey {
        &self.ticket.key
    }
}
pub struct StoredRun {
    ticket: Ticket,
    output: Arc<ProducedRun>,
}
impl StoredRun {
    pub fn run_id(&self) -> &str {
        &self.ticket.run
    }
    pub fn output(&self) -> &ProducedRun {
        &self.output
    }
    pub fn work_key(&self) -> &WorkKey {
        &self.ticket.key
    }
    pub fn generation(&self) -> u64 {
        self.ticket.generation
    }
}
#[derive(Default)]
pub struct RunHistory {
    identity: Arc<()>,
    registrations: BTreeMap<String, Ticket>,
    generations: BTreeMap<RunTarget, u64>,
    records: BTreeMap<String, StoredRun>,
    order: Vec<String>,
    current: BTreeMap<RunTarget, String>,
    charged: usize,
}
fn encoded_size(value: &impl Serialize) -> Result<usize, HistoryError> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            if self.0 > 16 * 1024 * 1024 {
                return Err(std::io::Error::other("history byte budget"));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value).map_err(|_| HistoryError::Capacity)?;
    Ok(counter.0)
}
impl RunHistory {
    pub fn register(
        &mut self,
        prepared: PreparedRun,
        expected_generation: u64,
    ) -> Result<RegisteredRun, HistoryError> {
        let run = prepared.attempt_id();
        let key = prepared.work_key();
        if self.registrations.contains_key(run) {
            return Err(HistoryError::Conflict);
        }
        let previous = self.generations.get(key.target()).copied().unwrap_or(0);
        if previous != expected_generation {
            return Err(HistoryError::Stale);
        }
        if self.registrations.len() >= 256 {
            return Err(HistoryError::Capacity);
        }
        // Reserve every copied key/target/run string before mutating the store.
        let cost = encoded_size(&(run, &key.target, &key.digest))?
            .saturating_mul(8)
            .saturating_add(4096);
        if self.charged.saturating_add(cost) > 64 * 1024 * 1024 {
            return Err(HistoryError::Capacity);
        }
        let generation = previous.checked_add(1).ok_or(HistoryError::Capacity)?;
        let ticket = Ticket {
            store: self.identity.clone(),
            run: run.into(),
            key: key.clone(),
            generation,
        };
        self.charged += cost;
        self.generations.insert(key.target.clone(), generation);
        self.current.remove(&key.target);
        self.registrations.insert(run.into(), ticket.clone());
        Ok(RegisteredRun { prepared, ticket })
    }
    fn validate(&self, ticket: &Ticket) -> Result<(), HistoryError> {
        if !Arc::ptr_eq(&self.identity, &ticket.store) {
            return Err(HistoryError::ForeignCompletion);
        }
        let registered = self
            .registrations
            .get(&ticket.run)
            .ok_or(HistoryError::UnknownRun)?;
        if registered.key != ticket.key || registered.generation != ticket.generation {
            return Err(HistoryError::Conflict);
        }
        Ok(())
    }
    pub fn append(&mut self, completion: &Completion) -> Result<AppendResult, HistoryError> {
        self.validate(&completion.ticket)?;
        if let Some(old) = self.records.get(&completion.ticket.run) {
            return if Arc::ptr_eq(&old.output, &completion.output) {
                Ok(AppendResult::IdenticalReplay)
            } else {
                Err(HistoryError::Conflict)
            };
        }
        completion
            .output
            .verify()
            .map_err(|_| HistoryError::InvalidCompletion)?;
        let size = encoded_size(completion.output.as_ref())?;
        let cost = size.saturating_add(4096);
        if self.charged.saturating_add(cost) > 64 * 1024 * 1024 {
            return Err(HistoryError::Capacity);
        }
        self.charged += cost;
        self.order.push(completion.ticket.run.clone());
        self.records.insert(
            completion.ticket.run.clone(),
            StoredRun {
                ticket: completion.ticket.clone(),
                output: completion.output.clone(),
            },
        );
        Ok(AppendResult::Inserted)
    }
    pub fn publish(&mut self, completion: &Completion) -> Result<(), HistoryError> {
        self.validate(&completion.ticket)?;
        if !self.records.contains_key(&completion.ticket.run) {
            return Err(HistoryError::UnknownRun);
        }
        if self.generations.get(completion.ticket.key.target())
            != Some(&completion.ticket.generation)
        {
            return Err(HistoryError::Stale);
        }
        self.current.insert(
            completion.ticket.key.target.clone(),
            completion.ticket.run.clone(),
        );
        Ok(())
    }
    pub fn generation(&self, target: &RunTarget) -> u64 {
        self.generations.get(target).copied().unwrap_or(0)
    }
    pub fn current(&self, target: &RunTarget) -> Option<&StoredRun> {
        self.current.get(target).and_then(|id| self.records.get(id))
    }
    pub fn history(&self, target: &RunTarget) -> Vec<&StoredRun> {
        self.order
            .iter()
            .filter_map(|id| self.records.get(id))
            .filter(|r| r.ticket.key.target() == target)
            .collect()
    }
}
