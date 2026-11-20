use std::time::{Duration, Instant};

use http::HeaderMap;
use parking_lot::Mutex;

use crate::error::EngineError;

const JITTER_BUFFER: Duration = Duration::from_millis(50);

struct BucketState {
    limit: Option<u32>,
    remaining: Option<u32>,
    reset_at: Option<Instant>,
    safety_margin: u32,
    max_wait: Option<Duration>,
}

/// Proactive header-synchronized rate limiter honoring Modrinth `X-Ratelimit-*` headers.
pub struct RateLimiter {
    state: Mutex<BucketState>,
}

impl RateLimiter {
    /// Create a rate limiter reserving `safety_margin` requests before window reset.
    pub fn new(safety_margin: u32, max_wait: Option<Duration>) -> Self {
        Self {
            state: Mutex::new(BucketState {
                limit: None,
                remaining: None,
                reset_at: None,
                safety_margin,
                max_wait,
            }),
        }
    }

    /// Wait smoothly before sending an outbound request when remaining quota reaches the reserve.
    pub async fn acquire(&self, url: &str) -> Result<(), EngineError> {
        let wait_until = {
            let mut guard = self.state.lock();
            let now = Instant::now();

            match (guard.remaining, guard.reset_at) {
                (Some(rem), Some(reset)) if rem <= guard.safety_margin && reset > now => {
                    let wait = (reset - now) + JITTER_BUFFER;
                    if let Some(cap) = guard.max_wait
                        && wait > cap
                    {
                        return Err(EngineError::RateLimited {
                            retry_after_secs: wait.as_secs().max(1),
                            url: url.to_owned(),
                        });
                    }
                    Some((wait, reset))
                }
                (Some(_), Some(reset)) if reset <= now => {
                    guard.remaining = guard.limit;
                    guard.reset_at = None;
                    None
                }
                _ => None,
            }
        };

        if let Some((wait, reset)) = wait_until {
            tokio::time::sleep(wait).await;
            let mut guard = self.state.lock();
            if Instant::now() >= reset {
                guard.remaining = guard.limit;
                guard.reset_at = None;
            }
        }

        Ok(())
    }

    /// Update tracked quota state from Modrinth `X-Ratelimit-*` response headers.
    pub fn update_from_headers(&self, headers: &HeaderMap) {
        let limit = parse_header_u32(headers, "x-ratelimit-limit");
        let remaining = parse_header_u32(headers, "x-ratelimit-remaining");
        let reset_secs = parse_header_u64(headers, "x-ratelimit-reset");

        let mut guard = self.state.lock();
        if let Some(l) = limit {
            guard.limit = Some(l);
        }
        if let Some(rem) = remaining {
            guard.remaining = Some(rem);
        }
        if let Some(secs) = reset_secs {
            if secs > 0 {
                guard.reset_at = Some(Instant::now() + Duration::from_secs(secs));
            } else if remaining.is_some_and(|rem| rem > guard.safety_margin) {
                guard.reset_at = None;
            }
        }
    }

    /// Parse `Retry-After` or `X-Ratelimit-Reset` seconds from an HTTP response header map.
    pub fn parse_retry_after_secs(headers: &HeaderMap) -> Option<u64> {
        parse_header_u64(headers, "retry-after")
            .or_else(|| parse_header_u64(headers, "x-ratelimit-reset"))
    }

    /// Sleep smoothly after an unexpected HTTP 429 unless `max_wait` is exceeded.
    pub async fn wait_for_429(&self, retry_after_secs: u64, url: &str) -> Result<(), EngineError> {
        let wait = Duration::from_secs(retry_after_secs.max(1)) + JITTER_BUFFER;
        let reset_at = Instant::now() + wait;

        {
            let mut guard = self.state.lock();
            if let Some(cap) = guard.max_wait
                && wait > cap
            {
                return Err(EngineError::RateLimited {
                    retry_after_secs: retry_after_secs.max(1),
                    url: url.to_owned(),
                });
            }
            guard.remaining = Some(0);
            guard.reset_at = Some(reset_at);
        }

        tokio::time::sleep(wait).await;

        let mut guard = self.state.lock();
        if Instant::now() >= reset_at {
            guard.remaining = guard.limit;
            guard.reset_at = None;
        }
        Ok(())
    }
}

fn parse_header_u32(headers: &HeaderMap, name: &str) -> Option<u32> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u32>().ok())
}

fn parse_header_u64(headers: &HeaderMap, name: &str) -> Option<u64> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use http::{HeaderMap, HeaderValue};

    use super::*;

    #[tokio::test]
    async fn fails_fast_when_max_wait_is_exceeded() {
        let limiter = RateLimiter::new(2, Some(Duration::from_millis(100)));
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-limit", HeaderValue::from_static("300"));
        headers.insert("x-ratelimit-remaining", HeaderValue::from_static("1"));
        headers.insert("x-ratelimit-reset", HeaderValue::from_static("2"));
        limiter.update_from_headers(&headers);

        let err = limiter
            .acquire("https://api.modrinth.com/v3/search")
            .await
            .unwrap_err();
        assert!(matches!(err, EngineError::RateLimited { .. }));
    }

    #[test]
    fn extracts_retry_after_with_fallback_to_ratelimit_reset() {
        let mut h1 = HeaderMap::new();
        h1.insert("retry-after", HeaderValue::from_static("15"));
        h1.insert("x-ratelimit-reset", HeaderValue::from_static("5"));
        assert_eq!(RateLimiter::parse_retry_after_secs(&h1), Some(15));

        let mut h2 = HeaderMap::new();
        h2.insert("x-ratelimit-reset", HeaderValue::from_static("7"));
        assert_eq!(RateLimiter::parse_retry_after_secs(&h2), Some(7));
    }
}
