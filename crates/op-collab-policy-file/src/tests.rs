#![cfg(test)]

use std::num::NonZeroU64;

use op_auth_bridge::{
    CollabJwksCacheLimits, CollabTicketVerifier, CollabVerifierConfig, CollabVerifierConfigError,
};

use super::*;

#[test]
fn bounded_regular_file_reads_exact_bytes_and_redacts_path() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("policy.json");
    std::fs::write(&path, b"{\"version\":1}").expect("policy");
    assert_eq!(
        read_bounded_regular_file(&path, 64).expect("read"),
        b"{\"version\":1}"
    );
    assert!(matches!(
        read_bounded_regular_file(&path, 4),
        Err(CollabJwksFetchError::ResponseTooLarge)
    ));

    let config = test_config().expect("config");
    let fetcher = PinnedPolicyFileFetcher::new(&config, &path, NonZeroU64::MIN);
    let debug = format!("{fetcher:?}");
    assert!(!debug.contains(path.to_string_lossy().as_ref()));
}

#[cfg(unix)]
#[test]
fn final_symlink_is_rejected() {
    use std::os::unix::fs::symlink;

    let directory = tempfile::tempdir().expect("temp directory");
    let target = directory.path().join("target.json");
    let link = directory.path().join("policy.json");
    std::fs::write(&target, b"{}").expect("target");
    symlink(&target, &link).expect("symlink");
    assert!(matches!(
        read_bounded_regular_file(&link, 64),
        Err(CollabJwksFetchError::RejectedResponse)
    ));
}

#[test]
fn verifier_endpoint_is_pinned() {
    let directory = tempfile::tempdir().expect("temp directory");
    let path = directory.path().join("policy.json");
    std::fs::write(&path, b"{\"keys\":[]}").expect("policy");
    let config = test_config().expect("config");
    let fetcher = PinnedPolicyFileFetcher::new(&config, path, NonZeroU64::MIN);
    let verifier = CollabTicketVerifier::new(config, fetcher, CollabJwksCacheLimits::default());
    assert!(verifier.is_ok());
}

fn test_config() -> Result<CollabVerifierConfig, CollabVerifierConfigError> {
    CollabVerifierConfig::new_pinned(
        "https://issuer.test.invalid",
        "https://issuer.test.invalid/.well-known/collab-jwks.json",
    )
}
