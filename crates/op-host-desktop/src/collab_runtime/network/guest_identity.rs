//! Which accounts a guest may accept as the owner of a session it joins.

use op_collab_transport::PeerIdentityPolicy;

/// Deliberately asymmetric with the owner's side, which admits any issued
/// account because a human approves each guest against the verified identity.
/// A guest has no such prompt — whatever it accepts, it accepts silently — so
/// the account check may only be dropped when the owner's device key was
/// already pinned out of band, which an invite supplies through its signed
/// locator. An unpinned LAN join has no anchor at all: mDNS is spoofable and
/// nothing else names the peer, so there the subject *is* the authentication
/// and only this account's own devices are accepted.
pub(super) fn guest_identity_policy<'a>(
    pinned_remote_static: Option<&[u8; 32]>,
    local_subject: &'a str,
) -> PeerIdentityPolicy<'a> {
    if pinned_remote_static.is_some() {
        PeerIdentityPolicy::AnyIssuedAccount
    } else {
        PeerIdentityPolicy::SameAccount {
            subject: local_subject,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::guest_identity_policy;
    use op_collab_transport::PeerIdentityPolicy;

    const LOCAL_SUBJECT: &str = "00000000-0000-0000-0000-000000000001";

    #[test]
    fn a_pinned_owner_key_admits_any_account() {
        // An invite pins the owner's Noise static in its signed locator, and
        // that pin is checked before admission runs. The device is therefore
        // already authenticated and the account behind it is irrelevant, which
        // is what makes cross-account collaboration by invite safe.
        assert_eq!(
            guest_identity_policy(Some(&[9_u8; 32]), LOCAL_SUBJECT),
            PeerIdentityPolicy::AnyIssuedAccount
        );
    }

    #[test]
    fn an_unpinned_join_still_requires_this_account() {
        // Regression guard. A guest has no approval prompt, so with no pinned
        // key the subject is the only thing authenticating the owner. Relaxing
        // this would let anyone on the LAN holding any valid ticket pose as the
        // owner, silently.
        assert_eq!(
            guest_identity_policy(None, LOCAL_SUBJECT),
            PeerIdentityPolicy::SameAccount {
                subject: LOCAL_SUBJECT
            }
        );
    }
}
