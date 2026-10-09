//! Authorized, auditable reads of immutable local evidence. No production trust profile.
use super::{
    approval::Profile,
    freshness::{ConsumptionClock, StoredRun, WorkKey},
    producer::PreparedRun,
};
use guardengine::integration::{
    Producer,
    eligibility::{AuthorityProvider, Validity},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
const MAX_FIELD: usize = 1024;
const MAX_METADATA: usize = 64 * 1024;
const MAX_RECORDS: usize = 256;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditError {
    Capacity,
    Budget,
    Profile,
    Clock,
    Binding,
    Uri,
    Integrity,
    Access,
    Producer,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Contract,
    Facts,
    Report,
    Domain,
}
impl ArtifactKind {
    fn name(self) -> &'static str {
        match self {
            Self::Contract => "contract",
            Self::Facts => "facts",
            Self::Report => "report",
            Self::Domain => "domain",
        }
    }
    fn bytes(self, run: &StoredRun) -> Option<&[u8]> {
        let p = run.output();
        match self {
            Self::Contract => p.contract.as_deref(),
            Self::Facts => p.facts.as_deref(),
            Self::Report => p.report.as_deref(),
            Self::Domain => p.domain.as_deref(),
        }
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn fields<'a>(values: impl IntoIterator<Item = &'a str>) -> Result<(), AuditError> {
    let mut total = 0usize;
    for v in values {
        total = total.saturating_add(v.len());
        if v.len() > MAX_FIELD || total > MAX_METADATA {
            return Err(AuditError::Budget);
        }
    }
    Ok(())
}
fn producer_budget(p: &Producer) -> Result<(), AuditError> {
    fields([
        p.guard.as_str(),
        p.version.as_str(),
        p.analyzer_id.as_str(),
        p.analyzer_version.as_str(),
    ])
}
fn size(value: &impl Serialize) -> Result<(), AuditError> {
    struct Count(usize);
    impl std::io::Write for Count {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(b.len());
            if self.0 > MAX_METADATA {
                return Err(std::io::Error::other("audit metadata budget"));
            }
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Count(0), value).map_err(|_| AuditError::Budget)
}
fn digest(value: &impl Serialize) -> String {
    hash(&serde_json::to_vec(value).expect("admitted serializable metadata"))
}
fn valid(v: &Validity, now: i64) -> bool {
    !v.revoked && v.issued_at >= 0 && v.issued_at <= now && now < v.expires_at
}
/// Independent controller expectations, never deserialized from returned evidence.
pub struct AuditExpectation {
    key: WorkKey,
    generation: u64,
    producer: Producer,
    principals: BTreeSet<String>,
}
impl AuditExpectation {
    pub fn freeze(
        prepared: &PreparedRun,
        generation: u64,
        producer: &Producer,
        principals: &BTreeSet<String>,
        profile: Profile,
    ) -> Result<Self, AuditError> {
        if profile != Profile::Fixture {
            return Err(AuditError::Profile);
        }
        if principals.is_empty() || principals.len() > 32 {
            return Err(AuditError::Budget);
        }
        producer_budget(producer)?;
        fields(principals.iter().map(String::as_str))?;
        if principals.iter().any(String::is_empty) {
            return Err(AuditError::Producer);
        }
        size(prepared.work_key().target())?;
        Ok(Self {
            key: prepared.work_key().clone(),
            generation,
            producer: producer.clone(),
            principals: principals.clone(),
        })
    }
    pub fn freeze_git(
        prepared: &super::git_binding::GitPreparedRun,
        generation: u64,
        producer: &Producer,
        principals: &BTreeSet<String>,
        profile: Profile,
    ) -> Result<Self, AuditError> {
        Self::freeze(
            prepared.domain_run(),
            generation,
            producer,
            principals,
            profile,
        )
    }
}
/// Opaque local reference: never interpreted as a path, URL, or network destination.
pub fn evidence_uri(run: &StoredRun, kind: ArtifactKind) -> Result<String, AuditError> {
    fields([run.run_id()])?;
    Ok(format!(
        "specguard-evidence:v1:{}:{}",
        &hash(run.run_id().as_bytes())[7..],
        kind.name()
    ))
}
pub struct AccessRequest<'a> {
    pub uri: &'a str,
    pub work_digest: &'a str,
    pub generation: u64,
    pub envelope_digest: &'a str,
    pub artifact_digest: &'a str,
}
pub struct AccessRecord {
    pub actor: String,
    pub uri: String,
    pub work_digest: String,
    pub generation: u64,
    pub envelope_digest: String,
    pub artifact_digest: String,
    pub purpose: String,
    pub validity: Validity,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessError {
    Denied,
    Unavailable,
}
/// Protected controller port. Provider authenticates its own session; no request-supplied actor or clock.
pub trait EvidenceAccessPort {
    fn profile(&self) -> Profile;
    fn authorize(&self, request: &AccessRequest<'_>) -> Result<AccessRecord, AccessError>;
}
#[derive(Debug, Serialize)]
pub struct AuditRecord {
    pub sequence: u64,
    pub timestamp: i64,
    pub predecessor_digest: Option<String>,
    pub result: Result<(), AuditError>,
    pub work_digest: Option<String>,
    pub envelope_digest: Option<String>,
    pub artifact_digest: Option<String>,
    pub actor_digest: Option<String>,
    pub producer_digest: Option<String>,
    pub references_digest: Option<String>,
    pub record_digest: String,
}
/// Read receipt only, not approval or merge eligibility. No public construction or deserialization.
/// ```compile_fail
/// use specguard::integration::audit::AuthorizedEvidence;
/// let forged = AuthorizedEvidence { bytes: b"untrusted" };
/// ```
pub struct AuthorizedEvidence<'a> {
    bytes: &'a [u8],
}
impl AuthorizedEvidence<'_> {
    pub fn bytes(&self) -> &[u8] {
        self.bytes
    }
}
#[derive(Default)]
pub struct AuditTrail {
    records: Vec<AuditRecord>,
    watermark: i64,
}
impl AuditTrail {
    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }
    fn now(&mut self, clock: &dyn ConsumptionClock) -> Result<i64, AuditError> {
        let now = clock.now().map_err(|_| AuditError::Clock)?;
        if now < 0 || now < self.watermark {
            return Err(AuditError::Clock);
        }
        self.watermark = now;
        Ok(now)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn resolve<'a>(
        &mut self,
        run: &'a StoredRun,
        expected: &AuditExpectation,
        uri: &str,
        access: &dyn EvidenceAccessPort,
        authority: &dyn AuthorityProvider,
        clock: &dyn ConsumptionClock,
    ) -> Result<AuthorizedEvidence<'a>, AuditError> {
        if self.records.len() >= MAX_RECORDS {
            return Err(AuditError::Capacity);
        }
        let mut record = AuditRecord {
            sequence: self.records.len() as u64,
            timestamp: self.watermark,
            predecessor_digest: self.records.last().map(|r| r.record_digest.clone()),
            result: Ok(()),
            work_digest: None,
            envelope_digest: None,
            artifact_digest: None,
            actor_digest: None,
            producer_digest: None,
            references_digest: None,
            record_digest: String::new(),
        };
        let result = (|| {
            let now = self.now(clock)?;
            record.timestamp = now;
            if access.profile() != Profile::Fixture {
                return Err(AuditError::Profile);
            }
            if uri.len() > 128 {
                return Err(AuditError::Budget);
            }
            if run.work_key() != &expected.key || run.generation() != expected.generation {
                return Err(AuditError::Binding);
            }
            let kind = [
                ArtifactKind::Contract,
                ArtifactKind::Facts,
                ArtifactKind::Report,
                ArtifactKind::Domain,
            ]
            .into_iter()
            .find(|kind| evidence_uri(run, *kind).as_deref() == Ok(uri))
            .ok_or(AuditError::Uri)?;
            let output = run.output();
            size(&output.envelope)?;
            for bytes in [
                &output.contract,
                &output.facts,
                &output.report,
                &output.domain,
            ]
            .into_iter()
            .flatten()
            {
                if bytes.len() > guardengine::integration::MAX_ARTIFACT_BYTES {
                    return Err(AuditError::Budget);
                }
            }
            output.verify().map_err(|_| AuditError::Integrity)?;
            let bytes = kind.bytes(run).ok_or(AuditError::Integrity)?;
            let envelope_digest = digest(&output.envelope);
            let artifact_digest = hash(bytes);
            record.work_digest = Some(run.work_key().digest().into());
            record.envelope_digest = Some(envelope_digest.clone());
            record.artifact_digest = Some(artifact_digest.clone());
            record.references_digest = Some(digest(&(
                &output.envelope.artifacts.contract,
                &output.envelope.approval_refs,
            )));
            let granted = access
                .authorize(&AccessRequest {
                    uri,
                    work_digest: run.work_key().digest(),
                    generation: run.generation(),
                    envelope_digest: &envelope_digest,
                    artifact_digest: &artifact_digest,
                })
                .map_err(|_| AuditError::Access)?;
            fields([
                granted.actor.as_str(),
                &granted.uri,
                &granted.work_digest,
                &granted.envelope_digest,
                &granted.artifact_digest,
                &granted.purpose,
            ])?;
            if granted.actor.is_empty()
                || granted.uri != uri
                || granted.work_digest != run.work_key().digest()
                || granted.generation != run.generation()
                || granted.envelope_digest != envelope_digest
                || granted.artifact_digest != artifact_digest
                || granted.purpose != "read-evidence"
                || !valid(&granted.validity, now)
            {
                return Err(AuditError::Access);
            }
            record.actor_digest = Some(hash(granted.actor.as_bytes()));
            let producer = authority
                .verify_producer(&output.envelope, &envelope_digest)
                .map_err(|_| AuditError::Producer)?;
            fields([
                producer.principal.as_str(),
                producer.envelope_digest.as_str(),
            ])?;
            producer_budget(&producer.producer)?;
            if producer.producer != expected.producer
                || output.envelope.producer != expected.producer
                || !expected.principals.contains(&producer.principal)
                || producer.envelope_digest != envelope_digest
                || !valid(&producer.validity, now)
            {
                return Err(AuditError::Producer);
            }
            record.producer_digest = Some(hash(producer.principal.as_bytes()));
            let final_now = self.now(clock)?;
            record.timestamp = final_now;
            if !valid(&granted.validity, final_now) {
                return Err(AuditError::Access);
            }
            if !valid(&producer.validity, final_now) {
                return Err(AuditError::Producer);
            }
            Ok(AuthorizedEvidence { bytes })
        })();
        record.result = result.as_ref().map(|_| ()).map_err(|e| *e);
        record.record_digest = digest(&record);
        self.records.push(record);
        result
    }
}
