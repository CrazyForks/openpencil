//! Safe, bounded file-backed key source for signed collaboration policies.

use std::{
    fmt,
    fs::{File, OpenOptions},
    io::{Read, Take},
    num::NonZeroU64,
    path::{Path, PathBuf},
};

use op_auth_bridge::{
    CollabJwksFetchError, CollabJwksFetchRequest, CollabJwksFetchResponse, CollabJwksFetcher,
    CollabVerifierConfig,
};

const FILE_ETAG_CONTEXT: &str = "openpencil/op-collab-policy-file/pinned-policy-file-etag/v1";

/// Reads one operator-pinned signed policy file without following a final
/// symlink.
///
/// The configured endpoint is compared byte-for-byte with each verifier
/// request. File metadata and reads are bounded by the verifier's requested
/// maximum. Unix opens additionally use `O_NOFOLLOW | O_CLOEXEC`.
pub struct PinnedPolicyFileFetcher {
    endpoint: String,
    path: PathBuf,
    max_age_seconds: NonZeroU64,
}

impl PinnedPolicyFileFetcher {
    pub fn new(
        verifier_config: &CollabVerifierConfig,
        path: impl Into<PathBuf>,
        max_age_seconds: NonZeroU64,
    ) -> Self {
        Self {
            endpoint: verifier_config.keyset_endpoint().to_owned(),
            path: path.into(),
            max_age_seconds,
        }
    }

    pub fn validate_source(&self, maximum_body_bytes: usize) -> Result<(), CollabJwksFetchError> {
        read_bounded_regular_file(&self.path, maximum_body_bytes).map(|_| ())
    }
}

impl CollabJwksFetcher for PinnedPolicyFileFetcher {
    fn fetch(
        &self,
        request: CollabJwksFetchRequest<'_>,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
        if request.endpoint != self.endpoint {
            return Err(CollabJwksFetchError::RejectedResponse);
        }
        let body = read_bounded_regular_file(&self.path, request.maximum_body_bytes)?;
        let etag = file_etag(&body);
        if request.etag == Some(etag.as_str()) {
            return Ok(CollabJwksFetchResponse::NotModified {
                etag: Some(etag),
                max_age_seconds: self.max_age_seconds.get(),
            });
        }
        Ok(CollabJwksFetchResponse::Modified {
            body,
            etag: Some(etag),
            max_age_seconds: self.max_age_seconds.get(),
        })
    }
}

impl fmt::Debug for PinnedPolicyFileFetcher {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedPolicyFileFetcher")
            .field("endpoint", &"[REDACTED]")
            .field("path", &"[REDACTED]")
            .field("max_age_seconds", &self.max_age_seconds)
            .finish()
    }
}

pub fn read_bounded_regular_file(
    path: &Path,
    maximum: usize,
) -> Result<Vec<u8>, CollabJwksFetchError> {
    let link_metadata =
        std::fs::symlink_metadata(path).map_err(|_| CollabJwksFetchError::Unavailable)?;
    if link_metadata.file_type().is_symlink() {
        return Err(CollabJwksFetchError::RejectedResponse);
    }

    let file = open_no_follow(path)?;
    let metadata = file
        .metadata()
        .map_err(|_| CollabJwksFetchError::Unavailable)?;
    if !metadata.file_type().is_file() {
        return Err(CollabJwksFetchError::RejectedResponse);
    }
    if metadata.len() > maximum as u64 {
        return Err(CollabJwksFetchError::ResponseTooLarge);
    }

    let mut reader = file.take((maximum as u64).saturating_add(1));
    let mut body = Vec::with_capacity(metadata.len() as usize);
    read_all(&mut reader, &mut body)?;
    if body.len() > maximum {
        return Err(CollabJwksFetchError::ResponseTooLarge);
    }
    Ok(body)
}

fn open_no_follow(path: &Path) -> Result<File, CollabJwksFetchError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    }
    options
        .open(path)
        .map_err(|_| CollabJwksFetchError::Unavailable)
}

fn read_all(reader: &mut Take<File>, body: &mut Vec<u8>) -> Result<(), CollabJwksFetchError> {
    reader
        .read_to_end(body)
        .map(|_| ())
        .map_err(|_| CollabJwksFetchError::Unavailable)
}

fn file_etag(body: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(FILE_ETAG_CONTEXT);
    hasher.update(body);
    format!("\"{}\"", hasher.finalize().to_hex())
}

#[cfg(test)]
mod tests;
