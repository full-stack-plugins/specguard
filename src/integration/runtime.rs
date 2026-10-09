//! Cooperative, panic=unwind library execution. No shell or detached process is
//! launched. Source checkpoints are bounded by the configured parser budgets.
use super::producer::{ProducedRun, raw_digest, reference};
use crate::model::SourceStatus;
use guardengine::integration::*;
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
#[derive(Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);
impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
#[derive(Clone)]
pub(crate) struct Recovery {
    run_id: String,
    producer: Option<Producer>,
    binding: Option<RunBinding>,
    coverage: Option<Coverage>,
    started_at: String,
}
#[derive(Default)]
pub(crate) struct History {
    records: Vec<SourceStatus>,
    omitted: usize,
    charged: usize,
    panic_payload: Option<String>,
    original_transport: Option<TransportDiagnostic>,
    verification_error: Option<String>,
}
impl History {
    pub(crate) fn record(&mut self, source: &SourceStatus) {
        let cost = source
            .path
            .len()
            .saturating_add(source.reason.len())
            .saturating_mul(6)
            .saturating_add(256);
        if self.records.len() >= 128 || self.charged.saturating_add(cost) > 65_536 {
            self.omitted += 1;
            return;
        }
        self.charged += cost;
        self.records.push(source.clone());
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RuntimeDiagnostic {
    api_version: String,
    run_id: String,
    binding_digest: String,
    required_scopes: Vec<String>,
    code: String,
    status: RunStatus,
    records: Vec<SourceStatus>,
    omitted: usize,
    panic_payload: Option<String>,
    original_transport: Option<TransportDiagnostic>,
    verification_error: Option<String>,
    finish_time_fallback: bool,
}
fn valid_finish(start: &str, finish: &str) -> bool {
    if finish.len() > 4096 {
        return false;
    }
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};
    match (
        OffsetDateTime::parse(start, &Rfc3339),
        OffsetDateTime::parse(finish, &Rfc3339),
    ) {
        (Ok(start), Ok(finish)) => finish.offset().is_utc() && finish >= start,
        _ => false,
    }
}
impl Recovery {
    pub(crate) fn from_draft(draft: &InvocationDraft) -> Self {
        Self {
            run_id: draft.run_id.clone(),
            producer: draft.producer.clone(),
            binding: draft.binding.clone(),
            coverage: draft.coverage.clone(),
            started_at: draft.started_at.clone(),
        }
    }
    pub(crate) fn failure(
        &self,
        status: RunStatus,
        code: &str,
        finished: &str,
        history: History,
    ) -> Result<ProducedRun, TransportDiagnostic> {
        let failure = || TransportDiagnostic {
            code: "runtime.recovery.invalid".into(),
            message: "frozen recovery context was invalid".into(),
        };
        let binding = self.binding.as_ref().ok_or_else(failure)?;
        let coverage = self.coverage.clone().ok_or_else(failure)?;
        let fallback = !valid_finish(&self.started_at, finished);
        let diagnostic = RuntimeDiagnostic {
            api_version: "specguard.runtime-diagnostic/v1alpha1".into(),
            run_id: self.run_id.clone(),
            binding_digest: raw_digest(&serde_json::to_vec(binding).map_err(|_| failure())?),
            required_scopes: coverage.required_scopes.clone(),
            code: code.into(),
            status: status.clone(),
            records: history.records,
            omitted: history.omitted,
            panic_payload: history.panic_payload,
            original_transport: history.original_transport,
            verification_error: history.verification_error,
            finish_time_fallback: fallback,
        };
        let domain = serde_json::to_vec(&diagnostic).map_err(|_| failure())?;
        let attempt = prepare_attempt(InvocationDraft {
            run_id: self.run_id.clone(),
            producer: self.producer.clone(),
            binding: self.binding.clone(),
            coverage: Some(coverage.clone()),
            profile: Some(EvidenceProfile::EngineBacked),
            started_at: self.started_at.clone(),
        })?;
        let envelope = attempt.finish(AttemptOutput {
            coverage,
            run_status: status,
            decision: None,
            artifacts: Artifacts {
                contract: None,
                facts: None,
                report: None,
                domain: vec![reference(&self.run_id, "diagnostic.json", &domain)],
            },
            approval_refs: vec![],
            diagnostics: vec![Diagnostic {
                code: code.into(),
                message: "Bound runtime did not complete; retained diagnostic artifact".into(),
                retryable: true,
                source: None,
            }],
            finished_at: if fallback {
                self.started_at.clone()
            } else {
                finished.into()
            },
            expires_at: None,
        })?;
        Ok(ProducedRun {
            envelope,
            contract: None,
            facts: None,
            report: None,
            domain: Some(domain),
        })
    }
}
pub(crate) fn finalize(
    result: std::thread::Result<Result<ProducedRun, TransportDiagnostic>>,
    recovery: &Recovery,
    cancellation: &CancellationToken,
    finished: &str,
    mut history: History,
) -> Result<ProducedRun, TransportDiagnostic> {
    match result {
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("non-string panic payload");
            let mut end = message.len().min(4096);
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            history.panic_payload = Some(message[..end].into());
            recovery.failure(RunStatus::Error, "runtime.panic", finished, history)
        }
        Ok(Err(error)) => {
            history.original_transport = Some(error);
            recovery.failure(RunStatus::Error, "runtime.finalization", finished, history)
        }
        Ok(Ok(output)) => {
            if cancellation.is_cancelled() {
                return recovery.failure(
                    RunStatus::Cancelled,
                    "runtime.cancelled",
                    finished,
                    history,
                );
            }
            if let Err(error) = output.verify() {
                let mut end = error.len().min(4096);
                while !error.is_char_boundary(end) {
                    end -= 1;
                }
                history.verification_error = Some(error[..end].into());
                return recovery.failure(
                    RunStatus::Error,
                    "runtime.verification",
                    finished,
                    history,
                );
            }
            if output.envelope.run_status != RunStatus::Completed {
                let code = output
                    .envelope
                    .diagnostics
                    .first()
                    .map(|d| d.code.as_str())
                    .unwrap_or("runtime.failed");
                return recovery.failure(
                    output.envelope.run_status.clone(),
                    code,
                    finished,
                    history,
                );
            }
            // Last cooperative checkpoint is the completion linearization point.
            if cancellation.is_cancelled() {
                return recovery.failure(
                    RunStatus::Cancelled,
                    "runtime.cancelled",
                    finished,
                    history,
                );
            }
            Ok(output)
        }
    }
}
pub(crate) fn verify_diagnostic(output: &ProducedRun) -> Result<(), String> {
    let Some(bytes) = output.domain.as_deref() else {
        return if output.envelope.artifacts.domain.is_empty() {
            Ok(())
        } else {
            Err("missing runtime diagnostic".into())
        };
    };
    if bytes.len() > MAX_ARTIFACT_BYTES
        || output.envelope.artifacts.domain.len() != 1
        || output.envelope.artifacts.domain[0].digest != raw_digest(bytes)
    {
        return Err("runtime diagnostic digest mismatch".into());
    }
    let diagnostic: RuntimeDiagnostic =
        serde_json::from_slice(bytes).map_err(|_| "invalid runtime diagnostic")?;
    if diagnostic.api_version != "specguard.runtime-diagnostic/v1alpha1"
        || diagnostic.run_id != output.envelope.run_id
        || diagnostic.status != output.envelope.run_status
        || diagnostic.binding_digest
            != raw_digest(
                &serde_json::to_vec(&output.envelope.binding)
                    .map_err(|_| "binding serialization")?,
            )
        || diagnostic.required_scopes != output.envelope.coverage.required_scopes
        || output.envelope.diagnostics.first().map(|d| d.code.as_str())
            != Some(diagnostic.code.as_str())
    {
        return Err("runtime diagnostic binding mismatch".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn verified_bundle() -> ProducedRun {
        ProducedRun {
            envelope: serde_json::from_slice(include_bytes!(
                "../../fixtures/engine-producer/allow/envelope.json"
            ))
            .unwrap(),
            contract: Some(
                include_bytes!("../../fixtures/engine-producer/allow/contract.json").to_vec(),
            ),
            facts: Some(include_bytes!("../../fixtures/engine-producer/allow/facts.json").to_vec()),
            report: Some(
                include_bytes!("../../fixtures/engine-producer/allow/report.json").to_vec(),
            ),
            domain: Some(
                include_bytes!("../../fixtures/engine-producer/allow/domain.json").to_vec(),
            ),
        }
    }
    #[test]
    fn real_verifier_failure_recovers_bound_error_and_diagnostic_artifact() {
        let mut output = verified_bundle();
        output.verify().unwrap();
        let original = output.envelope.binding.clone();
        let recovery = Recovery {
            run_id: output.envelope.run_id.clone(),
            producer: Some(output.envelope.producer.clone()),
            binding: Some(original.clone()),
            coverage: Some(Coverage {
                status: CoverageStatus::Partial,
                required_scopes: output.envelope.coverage.required_scopes.clone(),
                observed_scopes: vec![],
                missing_scopes: output.envelope.coverage.required_scopes.clone(),
            }),
            started_at: output.envelope.started_at.clone(),
        };
        // Actual bytes no longer match their real GE artifact digest.
        output.facts.as_mut().unwrap().push(b' ');
        assert!(output.verify().is_err());
        let mut history = History::default();
        history.record(&SourceStatus {
            path: "specs/a.md".into(),
            status: crate::model::Terminal::Complete,
            reason: "parsed".into(),
        });
        let recovered = finalize(
            Ok(Ok(output)),
            &recovery,
            &CancellationToken::new(),
            "2026-10-09T10:00:01Z",
            history,
        )
        .unwrap();
        assert_eq!(recovered.envelope.binding, original);
        assert_eq!(
            recovered.envelope.diagnostics[0].code,
            "runtime.verification"
        );
        assert_eq!(recovered.envelope.run_status, RunStatus::Error);
        assert!(
            recovered.report.is_none()
                && recovered.facts.is_none()
                && recovered.envelope.decision.is_none()
        );
        recovered.verify().unwrap();
        let mut corrupt = recovered.clone();
        corrupt.domain.as_mut().unwrap().push(b' ');
        assert!(corrupt.verify().is_err());
    }
}
