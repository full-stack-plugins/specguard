use crate::{baseline::*, model::*};
use std::collections::BTreeSet;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Profile {
    Fixture,
    Production,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalError {
    Unauthorized(String),
    Unavailable(String),
}
#[derive(Debug, Clone)]
pub struct Authentication {
    pub issuer: String,
    pub purpose: String,
    pub repository: String,
    pub scope: BTreeSet<Identity>,
    pub baseline_digest: String,
    pub policy_digest: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub revoked: bool,
}
pub trait ApprovalValidationPort {
    fn profile(&self) -> Profile;
    fn validate(&self, baseline: &ApprovedBaseline) -> Result<Authentication, ApprovalError>;
}
#[derive(Debug, Clone)]
pub struct ValidatedBaseline {
    pub(crate) record: ApprovedBaseline,
    pub(crate) profile: Profile,
}
impl ValidatedBaseline {
    pub fn record(&self) -> &ApprovedBaseline {
        &self.record
    }
    pub fn profile(&self) -> &Profile {
        &self.profile
    }
}
pub fn authenticate(
    b: &ApprovedBaseline,
    port: &dyn ApprovalValidationPort,
    profile: Profile,
    now: i64,
) -> Result<ValidatedBaseline, ApprovalError> {
    // Production is deliberately unavailable until a reviewed GE-TRUST adapter exists.
    if profile == Profile::Production || port.profile() != profile {
        return Err(ApprovalError::Unavailable(
            "production approval adapter not configured".into(),
        ));
    }
    validate_baseline(b).map_err(ApprovalError::Unauthorized)?;
    if b.state != BaselineState::Approved || now < b.effective_from || now >= b.expires_at {
        return Err(ApprovalError::Unauthorized("baseline inactive".into()));
    }
    let auth = port.validate(b)?;
    if auth.issuer.trim().is_empty()
        || auth.purpose != "specification-baseline"
        || auth.repository != b.repository
        || auth.scope != b.scope
        || auth.baseline_digest != digest(b)
        || auth.policy_digest != b.policy_digest
        || auth.issued_at > now
        || auth.expires_at <= now
        || auth.issued_at >= auth.expires_at
        || auth.revoked
    {
        return Err(ApprovalError::Unauthorized(
            "approval identity, binding or freshness mismatch".into(),
        ));
    }
    Ok(ValidatedBaseline {
        record: b.clone(),
        profile,
    })
}
