//! Which account this tab currently belongs to, and what happens when that
//! changes.
//!
//! A browser tab outlives a sign-in. Sign out of A and into B in the same tab
//! and, without this, B inherits A's document (the shell keeps whatever was
//! last applied), A's provider credentials (same-origin storage key), and A's
//! sync baseline — and B's own document, starting at a LOWER daemon version,
//! cannot displace it, because the sync client only accepts a version higher
//! than the one it has applied. So the leak is not transient; it is what the
//! tab shows until it is reloaded.
//!
//! The fix is an epoch. Every `/api/auth/status` answer carries a subject; a
//! change of subject — including signed-in → signed-out → signed-in-as-someone
//! -else — bumps the epoch, and everything keyed to an account is dropped.
//!
//! ## Why the subject and not the display name
//!
//! `username` is the account handle the daemon reports for the session.
//! `display_name` is user-editable and `primary_email` can be absent, so
//! neither identifies an account across a switch.

use std::cell::RefCell;

/// Storage partition used before anyone signs in.
///
/// A real subject can never collide with it: the partition key is prefixed,
/// and this value is not a legal account handle.
pub const ANONYMOUS_SUBJECT: &str = "anon";

thread_local! {
    /// The subject this tab is currently showing.
    ///
    /// Three states, and the distinction matters: the outer `None` means no
    /// status answer has arrived yet, `Some(None)` means observed and signed
    /// out, and `Some(Some(subject))` means signed in. Collapsing the first
    /// two would make a sign-in after a sign-out look like a tab's very first
    /// answer — and that is exactly the A → out → B switch this exists to
    /// catch.
    static SUBJECT: RefCell<Option<Option<String>>> = const { RefCell::new(None) };
    /// Bumped on every observed identity change.
    static EPOCH: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The account partition in force, for storage keys.
pub fn current_subject() -> String {
    SUBJECT.with(|slot| {
        slot.borrow()
            .clone()
            .flatten()
            .unwrap_or_else(|| ANONYMOUS_SUBJECT.to_string())
    })
}

/// How many identity changes this tab has seen.
pub fn epoch() -> u64 {
    EPOCH.with(std::cell::Cell::get)
}

/// Record the subject an `/api/auth/status` answer reported.
///
/// Returns `true` when the identity CHANGED and the caller must therefore
/// drop everything keyed to the previous account. The first observation is
/// not a change: the tab has shown nothing yet.
pub fn observe_subject(subject: Option<&str>) -> bool {
    let next = subject
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    SUBJECT.with(|slot| {
        let mut slot = slot.borrow_mut();
        let previous = slot.replace(next.clone());
        match previous {
            // The tab's first status answer. Whatever it says is what this
            // tab has always been showing, so there is nothing to discard —
            // but the storage partition is now known, so the epoch moves.
            None => {
                EPOCH.with(|epoch| epoch.set(epoch.get().saturating_add(1)));
                false
            }
            // A repeat of the same answer: the common case, and it must not
            // churn state or every poll would reset the tab.
            Some(before) if before == next => false,
            // A genuine change — sign-in, sign-out, or a switch between two
            // accounts. All three must drop the previous account's state.
            Some(_) => {
                EPOCH.with(|epoch| epoch.set(epoch.get().saturating_add(1)));
                true
            }
        }
    })
}

/// Read the account subject out of an `/api/auth/status` body.
///
/// `None` for a signed-out or unparseable answer, which is the anonymous
/// partition — the safe direction, since it shares nothing with a real one.
pub fn subject_from_status(body: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    if !parsed["signed_in"].as_bool().unwrap_or(false) {
        return None;
    }
    parsed["username"]
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// Reset the epoch state. Tests only — a tab never goes back to "no answer".
#[cfg(test)]
pub fn reset_for_test() {
    SUBJECT.with(|slot| *slot.borrow_mut() = None);
    EPOCH.with(|epoch| epoch.set(0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_anonymous_observation_is_not_a_change() {
        reset_for_test();
        assert!(!observe_subject(None));
        assert_eq!(current_subject(), ANONYMOUS_SUBJECT);
    }

    #[test]
    fn the_first_answer_of_a_tab_is_never_a_reset() {
        reset_for_test();
        // Whatever the first answer says is what this tab has always shown.
        assert!(!observe_subject(Some("alice")));
        assert_eq!(current_subject(), "alice");
        assert!(epoch() > 0);
    }

    #[test]
    fn signing_in_after_an_observed_sign_out_is_a_reset() {
        // Distinct from the case above: the tab HAS shown the anonymous
        // state, so signing in replaces what was on screen.
        reset_for_test();
        assert!(!observe_subject(None));
        assert!(observe_subject(Some("alice")));
        assert_eq!(current_subject(), "alice");
    }

    #[test]
    fn switching_accounts_reports_a_change() {
        reset_for_test();
        observe_subject(Some("alice"));
        let before = epoch();
        assert!(
            observe_subject(Some("bob")),
            "a different account must reset the tab"
        );
        assert_eq!(current_subject(), "bob");
        assert!(epoch() > before);
    }

    #[test]
    fn signing_out_reports_a_change() {
        reset_for_test();
        observe_subject(Some("alice"));
        assert!(observe_subject(None));
        assert_eq!(current_subject(), ANONYMOUS_SUBJECT);
    }

    #[test]
    fn the_sign_out_then_in_path_is_a_change_at_each_step() {
        // The leak this exists for: A → anonymous → B in one tab.
        reset_for_test();
        observe_subject(Some("alice"));
        assert!(observe_subject(None));
        assert!(observe_subject(Some("bob")));
        assert_eq!(current_subject(), "bob");
    }

    #[test]
    fn the_same_account_repeated_is_never_a_change() {
        reset_for_test();
        observe_subject(Some("alice"));
        let epoch_after_sign_in = epoch();
        for _ in 0..5 {
            assert!(!observe_subject(Some("alice")));
        }
        assert_eq!(epoch(), epoch_after_sign_in, "a poll must not churn state");
    }

    #[test]
    fn a_subject_is_read_out_of_a_signed_in_status_body() {
        assert_eq!(
            subject_from_status(r#"{"signed_in":true,"username":"alice"}"#).as_deref(),
            Some("alice")
        );
    }

    #[test]
    fn a_signed_out_or_unusable_status_body_has_no_subject() {
        for body in [
            r#"{"signed_in":false,"username":"alice"}"#,
            r#"{"signed_in":true}"#,
            r#"{"signed_in":true,"username":"  "}"#,
            "not json",
            "{}",
        ] {
            assert_eq!(subject_from_status(body), None, "{body:?}");
        }
    }

    #[test]
    fn whitespace_around_a_subject_does_not_create_a_second_partition() {
        reset_for_test();
        observe_subject(Some("alice"));
        assert!(!observe_subject(Some("  alice  ")));
    }
}

// The storage partitions only exist in the build that has a settings store.
#[cfg(all(test, feature = "canvaskit"))]
mod partition_tests {
    use super::*;

    #[test]
    fn two_accounts_get_two_storage_partitions() {
        reset_for_test();
        observe_subject(Some("alice"));
        let alice_settings = crate::web_settings::settings_storage_key();
        let alice_credentials = crate::web_settings::credential_storage_key();

        observe_subject(Some("bob"));
        let bob_settings = crate::web_settings::settings_storage_key();
        let bob_credentials = crate::web_settings::credential_storage_key();

        assert_ne!(
            alice_settings, bob_settings,
            "two accounts sharing a browser must not share a settings blob"
        );
        assert_ne!(
            alice_credentials, bob_credentials,
            "one account's provider API keys must not be readable by the next"
        );
    }

    #[test]
    fn signing_out_returns_to_the_anonymous_partition() {
        reset_for_test();
        observe_subject(Some("alice"));
        let signed_in = crate::web_settings::credential_storage_key();
        observe_subject(None);
        let anonymous = crate::web_settings::credential_storage_key();
        assert_ne!(signed_in, anonymous);
        assert!(anonymous.ends_with(ANONYMOUS_SUBJECT), "{anonymous}");
    }

    #[test]
    fn a_partition_key_is_never_the_bare_legacy_key() {
        // The unpartitioned keys may hold a different account's credentials
        // than the one now signed in, so they are never read.
        reset_for_test();
        for subject in [None, Some("alice")] {
            observe_subject(subject);
            assert!(crate::web_settings::settings_storage_key().contains("::"));
            assert!(crate::web_settings::credential_storage_key().contains("::"));
        }
    }
}
