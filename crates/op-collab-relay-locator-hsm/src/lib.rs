pub mod config;
pub mod error;
pub mod pkcs11;
pub mod protocol;
// The signer daemon speaks over a permission-checked Unix domain socket and
// enforces unix file modes on its secrets; there is no Windows deployment
// target, so the socket server and secure-file layers are unix-only.
#[cfg(unix)]
pub mod secure_file;
#[cfg(unix)]
pub mod server;

pub use config::{KeyConfig, Region, SignerConfig};
pub use error::{SignerError, SignerResult};
pub use pkcs11::KeyStore;
