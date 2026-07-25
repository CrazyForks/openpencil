//! Bridge to the proprietary op-auth client library.
//!
//! The real device-login implementation lives in the private
//! `ZSeven-W/op-platform` repository and ships as a prebuilt C-ABI static
//! library committed under `prebuilt/<target>/libop_auth.a`. When no
//! artifact exists for the current target — or its ABI version does not
//! match [`REQUIRED_ABI_VERSION`] — this crate falls back to a stub whose
//! [`available`] returns false, and hosts keep the account UI hidden.
//!
//! Poll model: [`login_begin`] returns a handle the host polls each frame
//! via [`poll`]; handle [`SESSION_HANDLE`] reports the restored/signed-in
//! session. All strings cross the FFI as UTF-8 pointer + length and are
//! freed by the bridge immediately after copying.

mod status;

#[cfg(op_auth_prebuilt)]
#[path = "real.rs"]
mod backend;

#[cfg(not(op_auth_prebuilt))]
#[path = "stub.rs"]
mod backend;

use std::path::PathBuf;

pub use status::AuthStatus;

/// ABI revision this bridge understands; must match the library's
/// `op_auth_abi_version()`.
pub const REQUIRED_ABI_VERSION: u32 = 1;

/// `poll` handle that reports the signed-in session instead of a flow.
pub const SESSION_HANDLE: u64 = 0;

/// Everything the runtime needs at startup.
#[derive(Clone, Debug)]
pub struct AuthInitConfig {
    /// SSO origin, e.g. `https://sso.zseven.cn` (override via
    /// `OPENPENCIL_SSO_URL` for local development).
    pub base_url: String,
    /// Directory owning the persisted credential file.
    pub storage_dir: PathBuf,
    pub device_name: String,
    pub app_version: String,
}

/// Whether a working auth backend is linked into this build.
pub fn available() -> bool {
    backend::available()
}

/// Initialize the runtime once per process; returns false when the
/// backend is unavailable or already initialized.
pub fn init(config: &AuthInitConfig) -> bool {
    backend::init(config)
}

/// Load a persisted session, if any; the profile then arrives via
/// `poll(SESSION_HANDLE)`. Returns whether a credential was restored.
pub fn restore() -> bool {
    backend::restore()
}

/// Start a browser login and return the flow handle (0 when unavailable).
pub fn login_begin() -> u64 {
    backend::login_begin()
}

/// Poll a flow handle (or [`SESSION_HANDLE`]) for its current status.
pub fn poll(handle: u64) -> AuthStatus {
    backend::poll(handle)
}

/// Abort an in-flight login flow.
pub fn cancel(handle: u64) {
    backend::cancel(handle)
}

/// Drop the local session and revoke the device token server-side.
pub fn sign_out() {
    backend::sign_out()
}

/// The zseven-sso platform identifier for this build target.
pub fn platform_id() -> &'static str {
    if cfg!(target_os = "macos") {
        "desktop_macos"
    } else if cfg!(target_os = "windows") {
        "desktop_windows"
    } else {
        "desktop_linux"
    }
}
