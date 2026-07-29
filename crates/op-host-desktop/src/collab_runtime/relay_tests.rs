#![cfg(test)]

use super::*;
use ed25519_dalek::{Signer, SigningKey};
use op_collab_relay_control_plane::SignedLocatorResponse;

struct SigningControlPlane(SigningKey);

impl RelayLocatorControlPlane for SigningControlPlane {
    fn publish_route(
        &self,
        draft: OwnerPublishDraft,
        _ticket: &OpaqueCollabTicket,
    ) -> Result<VerifiedRelayRoute, CollabRuntimeFailure> {
        let now = unix_time_ms().map_err(|_| CollabRuntimeFailure::RelayUnavailable)? / 1_000;
        let request = draft.request().clone();
        let key_id =
            LocatorKeyId::new("current").map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        let claims = UnsignedRelayLocatorV1::new(
            request.home_region(),
            *request.route_id(),
            request.generation(),
            *request.owner_noise_static(),
            request.expected_discovery_id().clone(),
            now,
            now + 60,
            key_id.clone(),
        )
        .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        let signature = self.0.sign(&claims.canonical_signing_bytes()).to_bytes();
        let locator = claims.attach_signature(
            LocatorSignature::new(signature).map_err(|_| CollabRuntimeFailure::RelayUnavailable)?,
        );
        let response = SignedLocatorResponse::decode(&locator.encode())
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        let verifier = Ed25519LocatorVerifier {
            keys: HashMap::from([(key_id.as_str().to_owned(), self.0.verifying_key())]),
        };
        let published = draft
            .complete(response, &verifier, now)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)?;
        published
            .invite()
            .verify(&verifier, now)
            .map_err(|_| CollabRuntimeFailure::RelayUnavailable)
    }
}

#[test]
fn development_unsigned_requires_all_three_gates() {
    for (debug, loopback, value, expected) in [
        (true, true, Some("1".to_owned()), true),
        (true, false, Some("1".to_owned()), false),
        (false, true, Some("1".to_owned()), false),
        (true, true, Some("true".to_owned()), false),
        (true, true, None, false),
    ] {
        assert_eq!(
            development_unsigned_opt_in(debug, loopback, value),
            expected
        );
    }
}

#[test]
fn locator_key_parser_is_bounded_and_verifies_ed25519() {
    let signing = SigningKey::from_bytes(&[7_u8; 32]);
    let encoded = URL_SAFE_NO_PAD.encode(signing.verifying_key().as_bytes());
    let verifier = parse_locator_keys(&format!("current={encoded}")).unwrap();
    let bytes = b"canonical locator bytes";
    let signature = signing.sign(bytes).to_bytes();
    assert!(verifier.verify(&LocatorKeyId::new("current").unwrap(), bytes, &signature));
    assert!(!verifier.verify(&LocatorKeyId::new("unknown").unwrap(), bytes, &signature));
    assert!(parse_locator_keys(&format!("a={encoded},a={encoded}")).is_err());
    assert!(parse_locator_keys("missing-separator").is_err());
}

#[test]
fn relay_x25519_pin_parser_is_canonical_bounded_and_rejects_duplicates() {
    let encoded = URL_SAFE_NO_PAD.encode([11_u8; 32]);
    assert!(parse_relay_x25519_keys(&format!("relay-cn={encoded}")).is_ok());
    assert!(parse_relay_x25519_keys(&format!("relay-cn={encoded},relay-cn={encoded}")).is_err());
    assert!(parse_relay_x25519_keys(&format!("relay-cn={encoded}=")).is_err());
    assert!(
        parse_relay_x25519_keys(&format!("relay-cn={}", URL_SAFE_NO_PAD.encode([0_u8; 32])))
            .is_err()
    );
}

#[test]
fn injected_control_plane_publishes_a_ticket_bound_owner_route() {
    let signing = SigningKey::from_bytes(&[9_u8; 32]);
    let verifying_key = signing.verifying_key();
    let draft = OwnerPublishDraft::generate(
        RelayRegion::Cn,
        OwnerNoiseStatic::new([2_u8; 32]).unwrap(),
        ExpectedDiscoveryId::new("stable-relay-prelude").unwrap(),
        RelayPublishLifetime::new(60).unwrap(),
    )
    .unwrap();
    let ticket = OpaqueCollabTicket::new(b"header.payload.signature".to_vec()).expect("ticket");
    let route = SigningControlPlane(signing)
        .publish_route(draft, &ticket)
        .expect("published route");
    assert_eq!(route.locator().claims().home_region(), RelayRegion::Cn);
    assert_eq!(
        route.locator().claims().owner_noise_static().as_bytes(),
        &[2_u8; 32]
    );
    let fragment = RelayInviteV1::new(&route).to_fragment();
    let invite = RelayInviteV1::from_fragment(&fragment).unwrap();
    let verifier = Ed25519LocatorVerifier {
        keys: HashMap::from([("current".to_owned(), verifying_key)]),
    };
    let now = unix_time_ms().unwrap() / 1_000;
    let verified = invite.verify(&verifier, now).expect("signed invite");
    assert_eq!(verified.locator().claims().home_region(), RelayRegion::Cn);
    assert!(endpoint_for_region(
        verified.locator().claims().home_region(),
        Some("wss://global-ingress.example.com/v1/tunnel"),
        Some("malformed-global-url"),
    )
    .is_ok());
}

#[test]
fn overseas_guest_follows_cn_home_region_and_never_falls_back() {
    // An overseas deployment may point the logical CN endpoint at a
    // Global L4/TLS-passthrough ingress whose upstream remains CN.
    let cn_url = "wss://global-ingress.example.com/v1/tunnel";
    let cn = RelayEndpoint::parse(cn_url).unwrap();
    assert_eq!(
        endpoint_for_region(RelayRegion::Cn, Some(cn_url), Some("malformed-global-url")).unwrap(),
        cn
    );
    assert_eq!(ui_region(RelayRegion::Cn), CollabRelayRegion::China);
    let error = endpoint_for_region(
        RelayRegion::Cn,
        None,
        Some("wss://global-home.example.com/v1/tunnel"),
    )
    .unwrap_err();
    assert_eq!(error.failure, CollabRuntimeFailure::RelayRegionUnavailable);
    let error = endpoint_for_region(
        RelayRegion::Cn,
        Some("malformed-cn-url"),
        Some("wss://global-home.example.com/v1/tunnel"),
    )
    .unwrap_err();
    assert_eq!(error.failure, CollabRuntimeFailure::RelayUnavailable);
}

#[test]
fn relay_setup_failures_use_connect_notices() {
    use op_editor_core::{CollabConnectErrorUi, CollabNoticeKind};

    for (failure, expected) in [
        (
            CollabRuntimeFailure::RelayInviteUnavailable,
            CollabConnectErrorUi::InviteUnavailable,
        ),
        (
            CollabRuntimeFailure::RelayUnavailable,
            CollabConnectErrorUi::RelayUnavailable,
        ),
        (
            CollabRuntimeFailure::RelayRegionUnavailable,
            CollabConnectErrorUi::RegionUnavailable,
        ),
    ] {
        assert_eq!(
            super::super::effects::disconnect_notice(failure),
            CollabNoticeKind::Connect(expected)
        );
    }
}
