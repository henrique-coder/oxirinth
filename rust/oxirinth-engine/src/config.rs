use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use sha2::{Digest, Sha256};
use url::Url;

use crate::error::EngineError;

/// Default Modrinth production base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.modrinth.com";

/// Canonical project repository URL included in the default `User-Agent`.
pub const REPOSITORY_URL: &str = "https://github.com/henrique-coder/oxirinth";

/// Default `User-Agent` header formatted per Modrinth's API guidelines:
/// `github_username/project_name/version (contact_or_repo_url)`.
pub fn default_user_agent() -> String {
    format!(
        "henrique-coder/oxirinth/{} ({REPOSITORY_URL})",
        crate::ENGINE_VERSION
    )
}

/// Redacted wrapper for Modrinth API keys so secrets never appear in logs or disk caches.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    /// Wrap a raw API key string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Expose the raw secret strictly for outbound HTTP header construction.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Compute a stable SHA-256 hex digest for cache-key isolation without storing the secret.
    pub fn auth_fingerprint(&self) -> String {
        let digest = Sha256::digest(self.0.as_bytes());
        encode_lower_hex(&digest)
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString([REDACTED])")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// Configuration for the persistent `redb` + `zstd` HTTP response cache and RAM LRU fallback.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Whether response caching is enabled.
    pub enabled: bool,
    /// Optional explicit directory for `http_cache.redb`; defaults to the OS cache directory.
    pub cache_dir: Option<PathBuf>,
    /// Maximum memory budget in bytes for the LRU fallback cache.
    pub ram_fallback_max_bytes: usize,
    /// Minimum payload size in bytes before `zstd` compression is applied.
    pub zstd_min_bytes: usize,
    /// Zstandard compression level (`1..=22`).
    pub zstd_level: i32,
    /// Fresh TTL in seconds for `/v3/search` responses.
    pub search_ttl_secs: u32,
    /// Fresh TTL in seconds for `/v3/project/*`, `/v3/user/*`, and `/v3/organization/*` responses.
    pub project_ttl_secs: u32,
    /// Fresh TTL in seconds for `/v3/project/*/version` list responses.
    pub version_list_ttl_secs: u32,
    /// Fresh TTL in seconds for `/v3/version/*` and `/v3/version_file/*` responses.
    pub version_ttl_secs: u32,
    /// Fresh TTL in seconds for tag and metadata routes (`/v3/tag/*`, `/v3/license`, `/v3/loader_field`).
    pub tag_ttl_secs: u32,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_dir: None,
            ram_fallback_max_bytes: 64 * 1024 * 1024,
            zstd_min_bytes: 512,
            zstd_level: 3,
            search_ttl_secs: 120,
            project_ttl_secs: 900,
            version_list_ttl_secs: 300,
            version_ttl_secs: 86_400,
            tag_ttl_secs: 21_600,
        }
    }
}

impl CacheConfig {
    /// Resolve the cache directory, falling back to `<os_cache_dir>/oxirinth` when unset.
    pub fn resolved_cache_dir(&self) -> Option<PathBuf> {
        self.cache_dir
            .clone()
            .or_else(|| dirs::cache_dir().map(|dir| dir.join("oxirinth")))
    }
}

/// Top-level runtime configuration for `oxirinth-engine`.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Base URL for Modrinth API requests.
    pub base_url: Url,
    /// Descriptive `User-Agent` header value.
    pub user_agent: String,
    /// Optional Modrinth Personal Access Token (`Authorization` header).
    pub api_key: Option<SecretString>,
    /// Per-attempt HTTP request timeout.
    pub timeout: Duration,
    /// Maximum automatic retry attempts on transient network or 5xx errors.
    pub max_retries: u32,
    /// Optional upper bound for rate-limit waiting; `None` waits smoothly as needed.
    pub max_rate_limit_wait: Option<Duration>,
    /// Proactive pacing reserve threshold (`X-Ratelimit-Remaining <= safety_margin`).
    pub rate_limit_safety_margin: u32,
    /// Local response cache settings.
    pub cache: CacheConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            base_url: Url::parse(DEFAULT_BASE_URL).expect("default Modrinth URL must be valid"),
            user_agent: default_user_agent(),
            api_key: None,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            max_rate_limit_wait: None,
            rate_limit_safety_margin: 2,
            cache: CacheConfig::default(),
        }
    }
}

impl EngineConfig {
    /// Validate configuration invariants before building HTTP or cache state.
    pub fn validate(&self) -> Result<(), EngineError> {
        let scheme = self.base_url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(EngineError::Config(format!(
                "base_url must use http or https scheme, got '{scheme}'"
            )));
        }

        if self.base_url.host_str().is_none() {
            return Err(EngineError::Config(
                "base_url must include a host".to_owned(),
            ));
        }

        let ua = self.user_agent.trim();
        if ua.is_empty() || ua.chars().any(char::is_control) {
            return Err(EngineError::Config(
                "user_agent must be non-empty and contain no control characters".to_owned(),
            ));
        }

        if let Some(key) = &self.api_key {
            let raw = key.expose();
            if raw.trim().is_empty() || raw.chars().any(char::is_control) {
                return Err(EngineError::Config(
                    "api_key must be non-empty and contain no control characters".to_owned(),
                ));
            }
        }

        if self.timeout.is_zero() {
            return Err(EngineError::Config(
                "timeout must be greater than zero".to_owned(),
            ));
        }

        if self.cache.enabled {
            if !(1..=22).contains(&self.cache.zstd_level) {
                return Err(EngineError::Config(format!(
                    "zstd_level must be between 1 and 22, got {}",
                    self.cache.zstd_level
                )));
            }
            if self.cache.ram_fallback_max_bytes == 0 {
                return Err(EngineError::Config(
                    "ram_fallback_max_bytes must be greater than zero when cache is enabled"
                        .to_owned(),
                ));
            }
        }

        Ok(())
    }

    /// Return `"anon"` or a SHA-256 fingerprint of `api_key` for cache-key isolation.
    pub fn auth_fingerprint(&self) -> String {
        match &self.api_key {
            Some(key) => key.auth_fingerprint(),
            None => "anon".to_owned(),
        }
    }
}

pub(crate) fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_user_agent_follows_modrinth_recommended_format() {
        assert_eq!(
            default_user_agent(),
            "henrique-coder/oxirinth/0.1.0 (https://github.com/henrique-coder/oxirinth)"
        );
    }

    #[test]
    fn secret_string_is_redacted_in_debug_and_display() {
        let secret = SecretString::new("mrp_super_secret_token_123");
        assert_eq!(format!("{secret}"), "[REDACTED]");
        assert_eq!(format!("{secret:?}"), "SecretString([REDACTED])");
        assert!(!format!("{secret:?}").contains("mrp_"));
        assert_eq!(secret.expose(), "mrp_super_secret_token_123");
    }

    #[test]
    fn auth_fingerprint_isolates_anonymous_and_authenticated_sessions() {
        let anon = EngineConfig::default();
        let auth_a = EngineConfig {
            api_key: Some(SecretString::new("mrp_token_a")),
            ..EngineConfig::default()
        };
        let auth_b = EngineConfig {
            api_key: Some(SecretString::new("mrp_token_b")),
            ..EngineConfig::default()
        };

        assert_eq!(anon.auth_fingerprint(), "anon");
        assert_ne!(auth_a.auth_fingerprint(), "anon");
        assert_ne!(auth_a.auth_fingerprint(), auth_b.auth_fingerprint());
        assert_eq!(auth_a.auth_fingerprint().len(), 64);
    }

    #[test]
    fn validates_config_boundaries() {
        assert!(EngineConfig::default().validate().is_ok());

        let bad_scheme = EngineConfig {
            base_url: Url::parse("ftp://api.modrinth.com").unwrap(),
            ..EngineConfig::default()
        };
        assert!(matches!(bad_scheme.validate(), Err(EngineError::Config(_))));

        let empty_ua = EngineConfig {
            user_agent: "   ".to_owned(),
            ..EngineConfig::default()
        };
        assert!(matches!(empty_ua.validate(), Err(EngineError::Config(_))));

        let bad_key = EngineConfig {
            api_key: Some(SecretString::new("bad\nkey")),
            ..EngineConfig::default()
        };
        assert!(matches!(bad_key.validate(), Err(EngineError::Config(_))));

        let zero_timeout = EngineConfig {
            timeout: Duration::ZERO,
            ..EngineConfig::default()
        };
        assert!(matches!(
            zero_timeout.validate(),
            Err(EngineError::Config(_))
        ));

        let bad_zstd = EngineConfig {
            cache: CacheConfig {
                zstd_level: 99,
                ..CacheConfig::default()
            },
            ..EngineConfig::default()
        };
        assert!(matches!(bad_zstd.validate(), Err(EngineError::Config(_))));
    }
}
