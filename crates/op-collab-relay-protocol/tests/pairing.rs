//! Round-trip, corruption, and redaction coverage for short pairing codes.

use std::num::NonZeroU64;

use op_collab_relay_protocol::{
    ExpectedDiscoveryId, LocatorKeyId, LocatorSignature, OwnerNoiseStatic, PairingCode,
    RelayInviteV1, RelayLocatorVerifier, RelayProtocolError, RelayRegion, RouteCapability, RouteId,
    SealedPairingInvite, UnsignedRelayLocatorV1, VerifiedRelayRoute, MAX_SEALED_INVITE_BYTES,
    PAIRING_CODE_CHARS, PAIRING_CODE_ID_BYTES, SEALED_INVITE_NONCE_BYTES, SEALED_INVITE_TAG_BYTES,
};

const NOW: u64 = 1_754_000_000;
const NOT_BEFORE: u64 = NOW - 60;
const EXPIRES: u64 = NOW + 600;

struct AcceptVerifier;

impl RelayLocatorVerifier for AcceptVerifier {
    fn verify(
        &self,
        _key_id: &LocatorKeyId,
        _canonical_signing_bytes: &[u8],
        _signature: &[u8; 64],
    ) -> bool {
        true
    }
}

fn invite() -> RelayInviteV1 {
    let unsigned = UnsignedRelayLocatorV1::new(
        RelayRegion::Global,
        RouteId::new([0x11; 16]).unwrap(),
        NonZeroU64::new(7).unwrap(),
        OwnerNoiseStatic::new([0x22; 32]).unwrap(),
        ExpectedDiscoveryId::new("discover-owner-a").unwrap(),
        NOT_BEFORE,
        EXPIRES,
        LocatorKeyId::new("relay-key-2026").unwrap(),
    )
    .unwrap();
    let locator = unsigned.attach_signature(LocatorSignature::new([0x55; 64]).unwrap());
    let verified = locator.verify(&AcceptVerifier, NOW).unwrap();
    RelayInviteV1::new(&VerifiedRelayRoute::new(
        verified,
        RouteCapability::new([0x33; 32]).unwrap(),
    ))
}

fn code() -> PairingCode {
    PairingCode::parse("2A2C4E6G8J").unwrap()
}

#[test]
fn pairing_code_parses_confusables_case_and_grouping() {
    let canonical = code();
    for variant in ["2a2c4e6g8j", "2A2C-4E6G-8J", " 2A2C 4E6G 8J "] {
        let parsed = PairingCode::parse(variant).unwrap();
        assert_eq!(parsed.expose_str(), canonical.expose_str(), "{variant}");
        assert!(PairingCode::looks_like(variant), "{variant}");
    }
    // Crockford confusables: I/L → 1, O → 0.
    assert_eq!(
        PairingCode::parse("1ICLEOGHJX").unwrap().expose_str(),
        "11C1E0GHJX"
    );

    for rejected in [
        "",
        "2A2C4E6G8",   // short
        "2A2C4E6G8J0", // long
        "2A2C4E6G8U",  // U is out of the alphabet
        "2A2C4E6G8*",  // symbol
        "opc1_abcdef", // invite fragment shape
        "192.168.1.8:43120",
    ] {
        assert!(
            PairingCode::parse(rejected).is_err(),
            "must reject {rejected:?}"
        );
        assert!(!PairingCode::looks_like(rejected), "{rejected}");
    }
}

#[test]
fn generated_codes_are_canonical_distinct_and_region_tagged() {
    let first = PairingCode::generate_for(RelayRegion::Global).unwrap();
    let second = PairingCode::generate_for(RelayRegion::Global).unwrap();
    assert_eq!(first.expose_str().len(), PAIRING_CODE_CHARS);
    assert!(PairingCode::looks_like(first.expose_str()));
    assert_eq!(first.region(), Some(RelayRegion::Global));
    assert_eq!(
        PairingCode::generate_for(RelayRegion::Cn).unwrap().region(),
        Some(RelayRegion::Cn)
    );
    assert_ne!(
        first.expose_str(),
        second.expose_str(),
        "two generated codes colliding is a broken RNG"
    );
    assert_eq!(first.code_id().len(), PAIRING_CODE_ID_BYTES);
    assert_ne!(first.code_id(), second.code_id());
}

#[test]
fn sealing_the_same_invite_twice_uses_fresh_nonces() {
    let first = SealedPairingInvite::seal_random(&code(), &invite()).unwrap();
    let second = SealedPairingInvite::seal_random(&code(), &invite()).unwrap();
    assert_ne!(
        first.as_bytes(),
        second.as_bytes(),
        "identical sealed bytes mean nonce reuse"
    );
    assert_ne!(
        &first.as_bytes()[1..1 + SEALED_INVITE_NONCE_BYTES],
        &second.as_bytes()[1..1 + SEALED_INVITE_NONCE_BYTES],
        "nonces must differ per seal"
    );
}

#[test]
fn sealed_invite_round_trips_only_with_the_right_code() {
    let sealed = SealedPairingInvite::seal_random(&code(), &invite()).unwrap();
    assert!(sealed.as_bytes().len() <= MAX_SEALED_INVITE_BYTES);

    let reopened = SealedPairingInvite::from_bytes(sealed.as_bytes())
        .unwrap()
        .open(&code())
        .unwrap();
    assert_eq!(reopened, invite());

    let wrong = PairingCode::parse("ZZZZZZZZZZ").unwrap();
    assert!(matches!(
        sealed.open(&wrong),
        Err(RelayProtocolError::InvalidPairingCode)
    ));
}

#[test]
fn sealed_invite_rejects_truncation_trailing_and_bit_flips() {
    let sealed = SealedPairingInvite::seal_random(&code(), &invite()).unwrap();
    let raw = sealed.as_bytes().to_vec();

    let minimum = 1 + SEALED_INVITE_NONCE_BYTES + SEALED_INVITE_TAG_BYTES + 1;
    for length in [0, 1, minimum - 1] {
        assert!(SealedPairingInvite::from_bytes(&raw[..length]).is_err());
    }
    let mut oversized = raw.clone();
    oversized.resize(MAX_SEALED_INVITE_BYTES + 1, 0);
    assert!(SealedPairingInvite::from_bytes(&oversized).is_err());

    let mut wrong_version = raw.clone();
    wrong_version[0] ^= 0xFF;
    assert!(matches!(
        SealedPairingInvite::from_bytes(&wrong_version),
        Err(RelayProtocolError::UnsupportedVersion { .. })
    ));

    // Any single bit flip in nonce, ciphertext, or tag must fail the MAC.
    let ciphertext_mid = 1
        + SEALED_INVITE_NONCE_BYTES
        + (raw.len() - 1 - SEALED_INVITE_NONCE_BYTES - SEALED_INVITE_TAG_BYTES) / 2;
    let tag_mid = raw.len() - SEALED_INVITE_TAG_BYTES / 2;
    for index in [
        1,
        1 + SEALED_INVITE_NONCE_BYTES,
        ciphertext_mid,
        raw.len() - SEALED_INVITE_TAG_BYTES,
        tag_mid,
        raw.len() - 1,
    ] {
        let mut corrupted = raw.clone();
        corrupted[index] ^= 0x01;
        assert!(
            SealedPairingInvite::from_bytes(&corrupted)
                .unwrap()
                .open(&code())
                .is_err(),
            "flip at {index} must not open"
        );
    }
}

#[test]
fn code_id_does_not_reveal_the_sealing_key_derivation() {
    // Distinct contexts: the id and the subkeys must differ even over the
    // same code bytes, so the server-side handle cannot double as key
    // material.
    let sealed = SealedPairingInvite::seal_random(&code(), &invite()).unwrap();
    let id = code().code_id();
    assert!(
        !sealed
            .as_bytes()
            .windows(id.len())
            .any(|window| window == id),
        "code id must not appear inside the sealed blob"
    );
}

#[test]
fn pairing_debug_is_redacted() {
    let debug_code = format!("{:?}", code());
    assert_eq!(debug_code, "PairingCode([REDACTED])");
    let sealed = SealedPairingInvite::seal_random(&code(), &invite()).unwrap();
    assert_eq!(format!("{sealed:?}"), "SealedPairingInvite([REDACTED])");
}

#[test]
fn dispatch_shape_requires_a_region_tag() {
    // Parses as a code shape, but the first char names no region — join
    // dispatch must not route it to the pairing branch. This is what keeps
    // 10-char LAN hostnames out of the claim path.
    for hostname in ["renderfarm", "A2C4E6G8J0", "myhostname"] {
        assert!(PairingCode::parse(hostname).is_ok(), "{hostname}");
        assert!(!PairingCode::looks_like(hostname), "{hostname}");
        assert!(PairingCode::parse(hostname).unwrap().region().is_none());
    }
    assert_eq!(
        PairingCode::parse("1A2C4E6G8J").unwrap().region(),
        Some(RelayRegion::Cn)
    );
    assert_eq!(code().region(), Some(RelayRegion::Global));
}
