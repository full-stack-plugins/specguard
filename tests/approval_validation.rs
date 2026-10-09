mod common;
use common::*;
use specguard::{baseline::*, integration::approval::*};
#[test]
fn external_authentication_distinguishes_unavailable_and_unauthorized() {
    let b = baseline();
    let port = fixture_approval(&b);
    assert!(authenticate(&b, &port, Profile::Fixture, 50).is_ok());
    assert!(matches!(
        authenticate(&b, &port, Profile::Production, 50),
        Err(ApprovalError::Unavailable(_))
    ));
    for n in 0..5 {
        let mut a = port.0.clone().unwrap();
        match n {
            0 => a.issuer = "".into(),
            1 => a.scope.clear(),
            2 => a.expires_at = 49,
            3 => a.revoked = true,
            _ => a.baseline_digest = "forged".into(),
        };
        assert!(matches!(
            authenticate(&b, &FixtureApproval(Ok(a)), Profile::Fixture, 50),
            Err(ApprovalError::Unauthorized(_))
        ));
    }
    assert!(matches!(
        authenticate(
            &b,
            &FixtureApproval(Err(ApprovalError::Unavailable("service down".into()))),
            Profile::Fixture,
            50
        ),
        Err(ApprovalError::Unavailable(_))
    ));
    for state in [
        BaselineState::Proposed,
        BaselineState::Superseded,
        BaselineState::Expired,
        BaselineState::Revoked,
    ] {
        let mut other = b.clone();
        other.state = state;
        assert!(authenticate(&other, &fixture_approval(&other), Profile::Fixture, 50).is_err());
    }
}
