//! Short pairing codes: a human-typeable handle that redeems to a full
//! [`RelayInviteV1`].
//!
//! A 503-char invite cannot be read over a shoulder or typed from a phone
//! call. The pairing code fixes that by moving the invite bytes to the
//! control plane, sealed under a key only holders of the code can derive:
//!
//! - The code is 10 chars of Crockford base32. The first char names the
//!   relay region (so a guest contacts exactly one control plane); the other
//!   nine are random → 45 bits of entropy.
//! - `code_id` (the storage key) and the sealing key are independent BLAKE3
//!   derivations of the canonical code, so the server can look a blob up
//!   without learning the key that opens it.
//! - The sealed blob is encrypt-then-MAC over the invite fragment with
//!   derived subkeys and a random 24-byte nonce; the MAC tag is compared
//!   through `blake3::Hash`'s constant-time equality.
//!
//! An online guesser must present a valid `code_id`, which requires the code
//! itself; the 2^45 space, the ≤1h server TTL, and the control plane's rate
//! limits make that infeasible. The control-plane operator holding a blob can
//! grind the 45-bit space offline — recovering (or forging) the sealed
//! invite. Session admission still requires an authenticated guest, and a
//! forged owner identity must survive the guest's owner-confirmation gate,
//! but unlike the long invite the short code is NOT confidential or
//! authentic against the storage operator; see the threat model.

use core::fmt;

use zeroize::Zeroizing;

use crate::{RelayInviteV1, RelayProtocolError, RelayRegion, MAX_INVITE_CHARS};

/// Exact canonical length of a pairing code.
pub const PAIRING_CODE_CHARS: usize = 10;
/// First-char region tags. The region rides in the code so a guest claims
/// from exactly one control plane instead of spraying the code id (and its
/// bearer ticket) across every region.
pub const PAIRING_REGION_CN_TAG: u8 = b'1';
pub const PAIRING_REGION_GLOBAL_TAG: u8 = b'2';
/// Crockford base32: no I, L, O (confusable) and no U (accidental words).
pub const PAIRING_CODE_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Storage lookup handle length.
pub const PAIRING_CODE_ID_BYTES: usize = 16;
/// Random nonce mixed into both sealing subkeys.
pub const SEALED_INVITE_NONCE_BYTES: usize = 24;
/// MAC tag length (full BLAKE3 output so `blake3::Hash` compares it).
pub const SEALED_INVITE_TAG_BYTES: usize = 32;
/// Version + nonce + longest allowed fragment + tag.
pub const MAX_SEALED_INVITE_BYTES: usize =
    1 + SEALED_INVITE_NONCE_BYTES + MAX_INVITE_CHARS + SEALED_INVITE_TAG_BYTES;

const CODE_ID_CONTEXT: &str = "openpencil/op-collab-relay-protocol/pairing-code-id/v1";
const ENC_KEY_CONTEXT: &str = "openpencil/op-collab-relay-protocol/pairing-code-enc-key/v1";
const MAC_KEY_CONTEXT: &str = "openpencil/op-collab-relay-protocol/pairing-code-mac-key/v1";

/// A canonical (uppercase, exact-length) short pairing code.
///
/// Treat the value as a secret: it derives the sealing key. `Debug` is
/// redacted and the buffer zeroizes on drop.
#[derive(Clone)]
pub struct PairingCode(Zeroizing<[u8; PAIRING_CODE_CHARS]>);

impl PairingCode {
    /// Parse user input into canonical form. Accepts lowercase and the
    /// Crockford confusables (`I`/`L` → `1`, `O` → `0`); everything else
    /// must be in the alphabet, and the length must be exact after
    /// whitespace/dash trimming.
    pub fn parse(input: &str) -> Result<Self, RelayProtocolError> {
        let mut canonical = [0_u8; PAIRING_CODE_CHARS];
        let mut length = 0_usize;
        for byte in input.bytes() {
            if byte.is_ascii_whitespace() || byte == b'-' {
                continue;
            }
            let Some(mapped) = canonical_code_byte(byte) else {
                return Err(RelayProtocolError::InvalidPairingCode);
            };
            if length == PAIRING_CODE_CHARS {
                return Err(RelayProtocolError::InvalidPairingCode);
            }
            canonical[length] = mapped;
            length += 1;
        }
        if length != PAIRING_CODE_CHARS {
            return Err(RelayProtocolError::InvalidPairingCode);
        }
        Ok(Self(Zeroizing::new(canonical)))
    }

    /// Whether user input is a claimable pairing code: canonical shape AND a
    /// valid region tag. The region requirement keeps 10-char LAN hostnames
    /// (`renderfarm`) out of the pairing branch of join dispatch.
    pub fn looks_like(input: &str) -> bool {
        Self::parse(input).is_ok_and(|code| code.region().is_some())
    }

    /// The relay region encoded in the first character, `None` when the tag
    /// is not a known region — such a code cannot be claimed anywhere.
    pub fn region(&self) -> Option<RelayRegion> {
        match self.0[0] {
            PAIRING_REGION_CN_TAG => Some(RelayRegion::Cn),
            PAIRING_REGION_GLOBAL_TAG => Some(RelayRegion::Global),
            _ => None,
        }
    }

    /// Generate a fresh random code claimable in `region`: one region tag
    /// plus nine random alphabet chars (45 bits).
    #[cfg(feature = "random")]
    pub fn generate_for(region: RelayRegion) -> Result<Self, RelayProtocolError> {
        let mut raw = [0_u8; PAIRING_CODE_CHARS - 1];
        getrandom::fill(&mut raw).map_err(|_| RelayProtocolError::RandomUnavailable)?;
        let mut canonical = [0_u8; PAIRING_CODE_CHARS];
        canonical[0] = match region {
            RelayRegion::Cn => PAIRING_REGION_CN_TAG,
            RelayRegion::Global => PAIRING_REGION_GLOBAL_TAG,
        };
        for (slot, byte) in canonical[1..].iter_mut().zip(raw) {
            *slot = PAIRING_CODE_ALPHABET[(byte % 32) as usize];
        }
        Ok(Self(Zeroizing::new(canonical)))
    }

    /// The canonical display form. Secret — show it only in the owner's
    /// share surface, never in logs.
    pub fn expose_str(&self) -> &str {
        core::str::from_utf8(self.0.as_slice()).expect("alphabet is ASCII")
    }

    /// Storage lookup handle. Independent from the sealing key, so handing
    /// it to the control plane does not hand over the blob's contents.
    pub fn code_id(&self) -> [u8; PAIRING_CODE_ID_BYTES] {
        let digest = blake3::Hasher::new_derive_key(CODE_ID_CONTEXT)
            .update(self.0.as_slice())
            .finalize();
        let mut id = [0_u8; PAIRING_CODE_ID_BYTES];
        id.copy_from_slice(&digest.as_bytes()[..PAIRING_CODE_ID_BYTES]);
        id
    }

    fn subkey(
        &self,
        context: &str,
        nonce: &[u8; SEALED_INVITE_NONCE_BYTES],
    ) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(
            *blake3::Hasher::new_derive_key(context)
                .update(self.0.as_slice())
                .update(nonce)
                .finalize()
                .as_bytes(),
        )
    }
}

impl fmt::Debug for PairingCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PairingCode([REDACTED])")
    }
}

fn canonical_code_byte(byte: u8) -> Option<u8> {
    let upper = byte.to_ascii_uppercase();
    let mapped = match upper {
        b'I' | b'L' => b'1',
        b'O' => b'0',
        other => other,
    };
    PAIRING_CODE_ALPHABET.contains(&mapped).then_some(mapped)
}

/// An invite fragment sealed under a pairing code.
///
/// Binary layout: `[version=1][nonce:24][ciphertext][tag:32]`, where the
/// ciphertext is the invite fragment XORed with a BLAKE3 XOF keystream and
/// the tag is a keyed BLAKE3 MAC over version, nonce, and ciphertext.
#[derive(Clone, PartialEq, Eq)]
pub struct SealedPairingInvite(Vec<u8>);

impl SealedPairingInvite {
    /// Seal an invite under `code` with a caller-supplied random nonce.
    pub fn seal(
        code: &PairingCode,
        invite: &RelayInviteV1,
        nonce: [u8; SEALED_INVITE_NONCE_BYTES],
    ) -> Self {
        let fragment = Zeroizing::new(invite.to_fragment());
        let mut body = fragment.as_bytes().to_vec();
        apply_keystream(&code.subkey(ENC_KEY_CONTEXT, &nonce), &mut body);
        let mut raw = Vec::with_capacity(
            1 + SEALED_INVITE_NONCE_BYTES + body.len() + SEALED_INVITE_TAG_BYTES,
        );
        raw.push(crate::RELAY_PROTOCOL_VERSION);
        raw.extend_from_slice(&nonce);
        raw.extend_from_slice(&body);
        let tag = mac_tag(&code.subkey(MAC_KEY_CONTEXT, &nonce), &raw);
        raw.extend_from_slice(tag.as_bytes());
        Self(raw)
    }

    /// Seal with a fresh random nonce.
    #[cfg(feature = "random")]
    pub fn seal_random(
        code: &PairingCode,
        invite: &RelayInviteV1,
    ) -> Result<Self, RelayProtocolError> {
        let mut nonce = [0_u8; SEALED_INVITE_NONCE_BYTES];
        getrandom::fill(&mut nonce).map_err(|_| RelayProtocolError::RandomUnavailable)?;
        Ok(Self::seal(code, invite, nonce))
    }

    /// Accept an untrusted blob from the wire, checking only bounds. The
    /// authenticity check happens in [`Self::open`].
    pub fn from_bytes(raw: &[u8]) -> Result<Self, RelayProtocolError> {
        let minimum = 1 + SEALED_INVITE_NONCE_BYTES + SEALED_INVITE_TAG_BYTES + 1;
        if raw.len() < minimum || raw.len() > MAX_SEALED_INVITE_BYTES {
            return Err(RelayProtocolError::InvalidSealedInvite);
        }
        if raw[0] != crate::RELAY_PROTOCOL_VERSION {
            return Err(RelayProtocolError::UnsupportedVersion {
                actual: raw[0],
                expected: crate::RELAY_PROTOCOL_VERSION,
            });
        }
        Ok(Self(raw.to_vec()))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Authenticate and decrypt with `code`, returning the parsed invite.
    /// A wrong code fails the constant-time tag comparison before any
    /// plaintext is interpreted.
    pub fn open(&self, code: &PairingCode) -> Result<RelayInviteV1, RelayProtocolError> {
        let raw = &self.0;
        let tag_offset = raw.len() - SEALED_INVITE_TAG_BYTES;
        let mut nonce = [0_u8; SEALED_INVITE_NONCE_BYTES];
        nonce.copy_from_slice(&raw[1..1 + SEALED_INVITE_NONCE_BYTES]);
        let expected = mac_tag(&code.subkey(MAC_KEY_CONTEXT, &nonce), &raw[..tag_offset]);
        let presented = blake3::Hash::from_bytes(
            raw[tag_offset..]
                .try_into()
                .expect("tag range is fixed width"),
        );
        // `blake3::Hash` equality is constant-time.
        if expected != presented {
            return Err(RelayProtocolError::InvalidPairingCode);
        }
        let mut body = Zeroizing::new(raw[1 + SEALED_INVITE_NONCE_BYTES..tag_offset].to_vec());
        apply_keystream(&code.subkey(ENC_KEY_CONTEXT, &nonce), &mut body);
        let fragment =
            core::str::from_utf8(&body).map_err(|_| RelayProtocolError::InvalidSealedInvite)?;
        RelayInviteV1::from_fragment(fragment)
    }
}

impl fmt::Debug for SealedPairingInvite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SealedPairingInvite([REDACTED])")
    }
}

fn apply_keystream(key: &[u8; 32], body: &mut [u8]) {
    let mut reader = blake3::Hasher::new_keyed(key).finalize_xof();
    let mut stream = Zeroizing::new(vec![0_u8; body.len()]);
    reader.fill(stream.as_mut_slice());
    for (byte, mask) in body.iter_mut().zip(stream.iter()) {
        *byte ^= mask;
    }
}

fn mac_tag(key: &[u8; 32], message: &[u8]) -> blake3::Hash {
    blake3::Hasher::new_keyed(key).update(message).finalize()
}
