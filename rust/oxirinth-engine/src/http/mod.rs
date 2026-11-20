pub mod ratelimit;
pub mod singleflight;

use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use http::header::{ACCEPT, AUTHORIZATION, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use http::{HeaderMap, HeaderValue, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::{
    RetryTransientMiddleware, Retryable, RetryableStrategy, default_on_request_failure,
};

use crate::cache::codec::CacheEntry;
use crate::cache::{CacheKey, CacheManager};
use crate::config::{CacheConfig, EngineConfig};
use crate::error::EngineError;
use crate::routes::RouteRequest;

use self::ratelimit::RateLimiter;
use self::singleflight::SingleflightGroup;

const MAX_429_RECOVERY_ATTEMPTS: usize = 3;

struct OxirinthRetryStrategy;

impl RetryableStrategy for OxirinthRetryStrategy {
    fn handle(
        &self,
        res: &Result<reqwest::Response, reqwest_middleware::Error>,
    ) -> Option<Retryable> {
        match res {
            Ok(resp) => match resp.status() {
                StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::GATEWAY_TIMEOUT => Some(Retryable::Transient),
                // 429 is handled explicitly by RateLimiter::wait_for_429 with exact header pacing.
                _ => None,
            },
            Err(err) => default_on_request_failure(err),
        }
    }
}

/// Core HTTP/2 transport pipeline integrating retries, rate limiting, singleflight, and caching.
pub struct HttpPipeline {
    client: ClientWithMiddleware,
    cache: CacheManager,
    cache_config: CacheConfig,
    rate_limiter: RateLimiter,
    singleflight: SingleflightGroup<Bytes>,
    auth_fingerprint: String,
}

impl HttpPipeline {
    /// Construct and validate the HTTP + cache pipeline from `EngineConfig`.
    pub fn new(config: &EngineConfig) -> Result<Self, EngineError> {
        config.validate()?;
        crate::init_crypto_provider();

        let mut default_headers = HeaderMap::new();
        default_headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        if let Some(api_key) = &config.api_key {
            let value = HeaderValue::from_str(api_key.expose()).map_err(|_| {
                EngineError::Config("api_key contains invalid HTTP header bytes".to_owned())
            })?;
            default_headers.insert(AUTHORIZATION, value);
        }

        let base_client = reqwest::Client::builder()
            .user_agent(&config.user_agent)
            .timeout(config.timeout)
            .default_headers(default_headers)
            .http2_adaptive_window(true)
            .build()
            .map_err(|err| {
                EngineError::Config(format!("failed to construct HTTP client: {err}"))
            })?;

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(config.max_retries);
        let retry_mw = RetryTransientMiddleware::new_with_policy_and_strategy(
            retry_policy,
            OxirinthRetryStrategy,
        );
        let client = ClientBuilder::new(base_client).with(retry_mw).build();

        Ok(Self {
            client,
            cache: CacheManager::open(&config.cache),
            cache_config: config.cache.clone(),
            rate_limiter: RateLimiter::new(
                config.rate_limit_safety_margin,
                config.max_rate_limit_wait,
            ),
            singleflight: SingleflightGroup::new(),
            auth_fingerprint: config.auth_fingerprint(),
        })
    }

    /// Execute a cached, rate-limited, singleflight-coalesced `GET` request returning raw UTF-8 JSON bytes.
    pub async fn get_json(&self, route: &RouteRequest) -> Result<Bytes, EngineError> {
        let key = self.cache_key_for_url(route.canonical_url());
        let now = current_unix_secs();
        let configured_ttl = route.tier.ttl_secs(&self.cache_config);

        if let Some(entry) = self.cache.get(&key)
            && entry.is_fresh_within(now, configured_ttl)
        {
            self.cache.record_hit();
            return Ok(entry.body);
        }

        self.singleflight
            .run(key, || self.fetch_with_revalidation(route, key))
            .await
            .map_err(|arc_err| match std::sync::Arc::try_unwrap(arc_err) {
                Ok(err) => err,
                Err(shared) => clone_shared_error(&shared),
            })
    }

    /// Access the underlying `CacheManager` for cache-aware batch seeding and statistics.
    pub fn cache(&self) -> &CacheManager {
        &self.cache
    }

    /// Active cache configuration.
    pub fn cache_config(&self) -> &CacheConfig {
        &self.cache_config
    }

    /// Compute the canonical GET cache key for `canonical_url` scoped to this engine's auth session.
    pub fn cache_key_for_url(&self, canonical_url: &str) -> CacheKey {
        CacheKey::from_request("GET", canonical_url, &self.auth_fingerprint)
    }

    /// Return current UNIX timestamp in seconds.
    pub fn now_unix() -> u64 {
        current_unix_secs()
    }

    async fn fetch_with_revalidation(
        &self,
        route: &RouteRequest,
        key: CacheKey,
    ) -> Result<Bytes, EngineError> {
        let now = current_unix_secs();
        let configured_ttl = route.tier.ttl_secs(&self.cache_config);
        let stale_entry = match self.cache.get(&key) {
            Some(entry) if entry.is_fresh_within(now, configured_ttl) => {
                self.cache.record_hit();
                return Ok(entry.body);
            }
            other => other,
        };

        self.cache.record_miss();
        let url_str = route.canonical_url();

        for attempt in 0..MAX_429_RECOVERY_ATTEMPTS {
            self.rate_limiter.acquire(url_str).await?;

            let mut req = self.client.get(route.url.clone());
            if let Some(stale) = &stale_entry {
                if let Some(etag) = &stale.etag {
                    req = req.header(IF_NONE_MATCH, etag);
                }
                if let Some(last_mod) = &stale.last_modified {
                    req = req.header(IF_MODIFIED_SINCE, last_mod);
                }
            }

            let resp = req.send().await.map_err(|err| EngineError::Network {
                message: err.to_string(),
                url: Some(url_str.to_owned()),
            })?;

            self.rate_limiter.update_from_headers(resp.headers());
            let status = resp.status();

            if status == StatusCode::NOT_MODIFIED
                && let Some(stale) = stale_entry.as_ref()
            {
                let new_etag = header_str(resp.headers(), ETAG);
                let new_last_mod = header_str(resp.headers(), LAST_MODIFIED);
                let ttl_secs = route.tier.ttl_secs(&self.cache_config);
                let refreshed = self.cache.refresh_metadata(
                    &key,
                    current_unix_secs(),
                    ttl_secs,
                    new_etag,
                    new_last_mod,
                );
                self.cache.record_revalidation();
                return Ok(refreshed.map_or_else(|| stale.body.clone(), |entry| entry.body));
            }

            if status == StatusCode::TOO_MANY_REQUESTS {
                let retry_after = RateLimiter::parse_retry_after_secs(resp.headers()).unwrap_or(1);
                if attempt + 1 >= MAX_429_RECOVERY_ATTEMPTS {
                    return Err(EngineError::RateLimited {
                        retry_after_secs: retry_after,
                        url: url_str.to_owned(),
                    });
                }
                self.rate_limiter.wait_for_429(retry_after, url_str).await?;
                continue;
            }

            let etag = header_str(resp.headers(), ETAG);
            let last_modified = header_str(resp.headers(), LAST_MODIFIED);
            let retry_after = RateLimiter::parse_retry_after_secs(resp.headers());

            let body = resp.bytes().await.map_err(|err| EngineError::Network {
                message: err.to_string(),
                url: Some(url_str.to_owned()),
            })?;

            if status.is_success() {
                let entry = CacheEntry {
                    status: status.as_u16(),
                    stored_at_unix: current_unix_secs(),
                    ttl_secs: route.tier.ttl_secs(&self.cache_config),
                    etag,
                    last_modified,
                    body: body.clone(),
                };
                self.cache.put(&key, &entry);
                return Ok(body);
            }

            return Err(EngineError::from_http_status(
                status.as_u16(),
                url_str,
                &body,
                retry_after,
            ));
        }

        Err(EngineError::RateLimited {
            retry_after_secs: 1,
            url: url_str.to_owned(),
        })
    }
}

fn header_str(headers: &HeaderMap, name: http::header::HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|val| val.to_str().ok())
        .map(str::to_owned)
}

fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |dur| dur.as_secs())
}

fn clone_shared_error(err: &EngineError) -> EngineError {
    match err {
        EngineError::Config(msg) => EngineError::Config(msg.clone()),
        EngineError::Network { message, url } => EngineError::Network {
            message: message.clone(),
            url: url.clone(),
        },
        EngineError::Api {
            status,
            error_code,
            description,
            url,
        } => EngineError::Api {
            status: *status,
            error_code: error_code.clone(),
            description: description.clone(),
            url: url.clone(),
        },
        EngineError::NotFound {
            identifier,
            description,
            url,
        } => EngineError::NotFound {
            identifier: identifier.clone(),
            description: description.clone(),
            url: url.clone(),
        },
        EngineError::Auth {
            status,
            description,
            url,
        } => EngineError::Auth {
            status: *status,
            description: description.clone(),
            url: url.clone(),
        },
        EngineError::RateLimited {
            retry_after_secs,
            url,
        } => EngineError::RateLimited {
            retry_after_secs: *retry_after_secs,
            url: url.clone(),
        },
        EngineError::TypeMismatch {
            expected,
            actual,
            identifier,
        } => EngineError::TypeMismatch {
            expected,
            actual: actual.clone(),
            identifier: identifier.clone(),
        },
        EngineError::Cache(msg) => EngineError::Cache(msg.clone()),
        EngineError::Serialization(msg) => EngineError::Serialization(msg.clone()),
        EngineError::Internal(msg) => EngineError::Internal(msg.clone()),
    }
}
