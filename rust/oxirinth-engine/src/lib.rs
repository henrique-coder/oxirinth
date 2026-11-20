#![forbid(unsafe_code)]

use std::sync::Once;

pub mod cache;
pub mod config;
pub mod domain;
pub mod error;
pub mod http;
pub mod routes;

/// Semantic version of the internal `oxirinth-engine` crate.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

static CRYPTO_PROVIDER_INIT: Once = Once::new();

/// Install the `ring` crypto provider for `rustls` once per process.
///
/// If another component already installed a default provider, the error is
/// ignored so initialization remains idempotent across embedded Python hosts.
pub fn init_crypto_provider() {
    CRYPTO_PROVIDER_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_version_matches_crate_version() {
        assert_eq!(ENGINE_VERSION, "0.1.0");
    }

    #[test]
    fn init_crypto_provider_is_idempotent() {
        init_crypto_provider();
        init_crypto_provider();
        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }
}
