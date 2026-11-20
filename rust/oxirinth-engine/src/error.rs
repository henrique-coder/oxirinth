use serde::Deserialize;
use thiserror::Error;

const MAX_FALLBACK_BODY_CHARS: usize = 256;

#[derive(Debug, Deserialize)]
struct ModrinthErrorBody {
    error: Option<String>,
    description: Option<String>,
}

/// Structured error returned by the internal `oxirinth-engine` crate.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EngineError {
    /// Invalid client, cache, or query configuration.
    #[error("invalid configuration: {0}")]
    Config(String),

    /// HTTP transport, TLS, or connection failure.
    #[error("network error: {message}")]
    Network {
        message: String,
        url: Option<String>,
    },

    /// Non-success HTTP response returned by Modrinth API.
    #[error("modrinth API error ({status}): {description}")]
    Api {
        status: u16,
        error_code: Option<String>,
        description: String,
        url: String,
    },

    /// Requested Modrinth resource was not found (HTTP 404).
    #[error("resource not found: {description}")]
    NotFound {
        identifier: Option<String>,
        description: String,
        url: String,
    },

    /// Authentication or authorization failure (HTTP 401/403).
    #[error("authentication error ({status}): {description}")]
    Auth {
        status: u16,
        description: String,
        url: String,
    },

    /// Rate limit exceeded and max wait budget exhausted (HTTP 429).
    #[error("rate limit exceeded; retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64, url: String },

    /// Retrieved project does not belong to the requested domain type.
    #[error(
        "project '{identifier}' does not match expected domain '{expected}' (found: {})",
        actual.join(", ")
    )]
    TypeMismatch {
        expected: &'static str,
        actual: Vec<String>,
        identifier: String,
    },

    /// Persistent or in-memory cache codec/storage failure.
    #[error("cache error: {0}")]
    Cache(String),

    /// JSON payload inspection or construction failure.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Unexpected internal invariant violation.
    #[error("internal error: {0}")]
    Internal(String),
}

impl EngineError {
    /// Build a structured API error from an HTTP status code and response body.
    pub fn from_http_status(
        status: u16,
        url: impl Into<String>,
        body: &[u8],
        retry_after_secs: Option<u64>,
    ) -> Self {
        let url = url.into();
        let parsed = serde_json::from_slice::<ModrinthErrorBody>(body).ok();
        let error_code = parsed.as_ref().and_then(|b| b.error.clone());
        let description = parsed
            .and_then(|b| b.description)
            .filter(|desc| !desc.trim().is_empty())
            .unwrap_or_else(|| sanitize_fallback_body(status, body));

        match status {
            404 => Self::NotFound {
                identifier: None,
                description,
                url,
            },
            401 | 403 => Self::Auth {
                status,
                description,
                url,
            },
            429 => Self::RateLimited {
                retry_after_secs: retry_after_secs.unwrap_or(60),
                url,
            },
            _ => Self::Api {
                status,
                error_code,
                description,
                url,
            },
        }
    }
}

fn sanitize_fallback_body(status: u16, body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return format!("HTTP {status}");
    }

    // Truncate on UTF-8 scalar boundary to avoid slicing mid-codepoint or leaking huge bodies.
    match trimmed.char_indices().nth(MAX_FALLBACK_BODY_CHARS) {
        Some((idx, _)) => format!("{}...", &trimmed[..idx]),
        None => trimmed.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modrinth_404_json_error() {
        let body = br#"{"error":"not_found","description":"the requested route does not exist"}"#;
        let err = EngineError::from_http_status(
            404,
            "https://api.modrinth.com/v3/project/missing",
            body,
            None,
        );

        match err {
            EngineError::NotFound {
                identifier,
                description,
                url,
            } => {
                assert!(identifier.is_none());
                assert_eq!(description, "the requested route does not exist");
                assert_eq!(url, "https://api.modrinth.com/v3/project/missing");
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    #[test]
    fn parses_auth_and_rate_limit_statuses() {
        let auth = EngineError::from_http_status(
            401,
            "https://api.modrinth.com/v3/project/x",
            br#"{"error":"unauthorized","description":"Invalid authentication token"}"#,
            None,
        );
        assert!(matches!(auth, EngineError::Auth { status: 401, .. }));

        let rate =
            EngineError::from_http_status(429, "https://api.modrinth.com/v3/search", b"", Some(12));
        assert!(matches!(
            rate,
            EngineError::RateLimited {
                retry_after_secs: 12,
                ..
            }
        ));
    }

    #[test]
    fn truncates_large_non_json_fallback_body() {
        let oversized = "x".repeat(MAX_FALLBACK_BODY_CHARS + 50);
        let err = EngineError::from_http_status(
            502,
            "https://api.modrinth.com/v3/search",
            oversized.as_bytes(),
            None,
        );

        match err {
            EngineError::Api {
                status,
                error_code,
                description,
                ..
            } => {
                assert_eq!(status, 502);
                assert!(error_code.is_none());
                assert_eq!(description.len(), MAX_FALLBACK_BODY_CHARS + 3);
                assert!(description.ends_with("..."));
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }
}
