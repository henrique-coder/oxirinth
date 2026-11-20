use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use oxirinth_engine::Engine;
use oxirinth_engine::config::{CacheConfig, EngineConfig, SecretString};
use oxirinth_engine::domain::{DomainKind, SearchFilters, SortIndex};
use oxirinth_engine::error::EngineError;
use url::Url;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "oxirinth-integ-{label}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn build_test_engine(server: &MockServer, cache: CacheConfig, api_key: Option<&str>) -> Engine {
    let config = EngineConfig {
        base_url: Url::parse(&server.uri()).unwrap(),
        api_key: api_key.map(SecretString::new),
        cache,
        ..EngineConfig::default()
    };
    Engine::new(config).unwrap()
}

fn disabled_cache() -> CacheConfig {
    CacheConfig {
        enabled: false,
        ..CacheConfig::default()
    }
}

#[tokio::test]
async fn injects_domain_facets_for_servers_and_plugins() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v3/search"))
        .and(query_param(
            "facets",
            r#"[["project_types:minecraft_java_server"]]"#,
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"hits":[{"slug":"aero-server"}],"page":1,"hits_per_page":10,"total_hits":1}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v3/search"))
        .and(query_param(
            "facets",
            r#"[["project_types:plugin"],["loaders:paper"],["game_versions:1.21.1"]]"#,
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"hits":[{"slug":"luckperms"}],"page":1,"hits_per_page":10,"total_hits":1}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let engine = build_test_engine(&server, disabled_cache(), None);

    let srv_res = engine
        .search_domain(DomainKind::Server, &SearchFilters::default())
        .await
        .unwrap();
    assert!(
        std::str::from_utf8(&srv_res)
            .unwrap()
            .contains("aero-server")
    );

    let plugin_res = engine
        .search_domain(
            DomainKind::Plugin,
            &SearchFilters {
                loaders: vec!["paper".to_owned()],
                game_versions: vec!["1.21.1".to_owned()],
                sort: SortIndex::Downloads,
                ..SearchFilters::default()
            },
        )
        .await
        .unwrap();
    assert!(
        std::str::from_utf8(&plugin_res)
            .unwrap()
            .contains("luckperms")
    );
}

#[tokio::test]
async fn validates_domain_types_for_multi_type_and_mismatched_projects() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v3/project/luckperms"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"id":"Vebnzrzj","slug":"luckperms","project_types":["mod","plugin"]}"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v3/project/sodium"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"id":"AANobbMI","slug":"sodium","project_types":["mod"]}"#),
        )
        .mount(&server)
        .await;

    let engine = build_test_engine(&server, disabled_cache(), None);

    assert!(
        engine
            .get_domain(DomainKind::Plugin, "luckperms")
            .await
            .is_ok()
    );
    assert!(
        engine
            .get_domain(DomainKind::Mod, "luckperms")
            .await
            .is_ok()
    );

    let err = engine
        .get_domain(DomainKind::Plugin, "sodium")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        EngineError::TypeMismatch {
            expected: "plugin",
            ..
        }
    ));
}

#[tokio::test]
async fn caches_with_etag_304_revalidation_and_isolates_auth_keys() {
    let server = MockServer::start().await;
    let dir = TempDir::new("etag");

    Mock::given(method("GET"))
        .and(path("/v3/project/sodium"))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304).insert_header("etag", "\"v2\""))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v3/project/sodium"))
        .and(header("authorization", "mrp_secret_123"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"id":"AANobbMI","slug":"sodium","project_types":["mod"],"auth":true}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v3/project/sodium"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"v1\"")
                .set_body_string(
                    r#"{"id":"AANobbMI","slug":"sodium","project_types":["mod"],"auth":false}"#,
                ),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Fresh cache hit (ttl=10s) serves locally; stale cache (ttl=0s) triggers If-None-Match -> 304.
    let cache_dir = dir.0.clone();
    let fresh_engine = build_test_engine(
        &server,
        CacheConfig {
            cache_dir: Some(cache_dir.clone()),
            project_ttl_secs: 600,
            ..CacheConfig::default()
        },
        None,
    );
    let first = fresh_engine
        .get_domain(DomainKind::Mod, "sodium")
        .await
        .unwrap();
    let second = fresh_engine
        .get_domain(DomainKind::Mod, "sodium")
        .await
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(fresh_engine.cache_stats().hits, 1);
    assert_eq!(fresh_engine.cache_stats().misses, 1);
    drop(fresh_engine);

    // Reopen same cache with project_ttl_secs = 0 so entry is stale and triggers 304 revalidation.
    let stale_engine = build_test_engine(
        &server,
        CacheConfig {
            cache_dir: Some(cache_dir.clone()),
            project_ttl_secs: 0,
            ..CacheConfig::default()
        },
        None,
    );
    let revalidated = stale_engine
        .get_domain(DomainKind::Mod, "sodium")
        .await
        .unwrap();
    assert_eq!(revalidated, first);
    assert_eq!(stale_engine.cache_stats().revalidations, 1);
    drop(stale_engine);

    // Authenticated engine on the same cache directory MUST NOT reuse anonymous cache entry.
    let auth_engine = build_test_engine(
        &server,
        CacheConfig {
            cache_dir: Some(cache_dir),
            project_ttl_secs: 600,
            ..CacheConfig::default()
        },
        Some("mrp_secret_123"),
    );
    let auth_body = auth_engine
        .get_domain(DomainKind::Mod, "sodium")
        .await
        .unwrap();
    assert!(
        std::str::from_utf8(&auth_body)
            .unwrap()
            .contains(r#""auth":true"#)
    );
}

#[tokio::test]
async fn proactively_paces_on_low_ratelimit_remaining_and_respects_max_wait() {
    let server = MockServer::start().await;
    let call_count = AtomicUsize::new(0);

    Mock::given(method("GET"))
        .and(path("/v3/tag/loader"))
        .respond_with(move |_req: &Request| {
            let n = call_count.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                ResponseTemplate::new(200)
                    .insert_header("x-ratelimit-limit", "300")
                    .insert_header("x-ratelimit-remaining", "0")
                    .insert_header("x-ratelimit-reset", "1")
                    .set_body_string(r#"[{"name":"fabric"}]"#)
            } else {
                ResponseTemplate::new(200)
                    .insert_header("x-ratelimit-limit", "300")
                    .insert_header("x-ratelimit-remaining", "299")
                    .insert_header("x-ratelimit-reset", "60")
                    .set_body_string(r#"[{"name":"fabric"}]"#)
            }
        })
        .expect(2)
        .mount(&server)
        .await;

    // Disabled cache so second call actually reaches the rate limiter.
    let smooth_engine = build_test_engine(&server, disabled_cache(), None);
    let _ = smooth_engine.get_tag_loaders().await.unwrap();
    let start = Instant::now();
    let _ = smooth_engine.get_tag_loaders().await.unwrap();
    assert!(start.elapsed() >= Duration::from_millis(900));

    // Now test max_rate_limit_wait fast failure.
    let fast_fail_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v3/tag/loader"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-ratelimit-limit", "300")
                .insert_header("x-ratelimit-remaining", "0")
                .insert_header("x-ratelimit-reset", "5")
                .set_body_string(r#"[{"name":"fabric"}]"#),
        )
        .expect(1)
        .mount(&fast_fail_server)
        .await;

    let fail_engine = Engine::new(EngineConfig {
        base_url: Url::parse(&fast_fail_server.uri()).unwrap(),
        max_rate_limit_wait: Some(Duration::from_millis(100)),
        cache: disabled_cache(),
        ..EngineConfig::default()
    })
    .unwrap();
    let _ = fail_engine.get_tag_loaders().await.unwrap();
    let err = fail_engine.get_tag_loaders().await.unwrap_err();
    assert!(matches!(err, EngineError::RateLimited { .. }));
}

struct SequenceResponder {
    attempts: AtomicUsize,
    first_status: u16,
    retry_after: Option<&'static str>,
    success_body: &'static str,
}

impl Respond for SequenceResponder {
    fn respond(&self, _req: &Request) -> ResponseTemplate {
        let n = self.attempts.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            let mut tpl = ResponseTemplate::new(self.first_status);
            if let Some(ra) = self.retry_after {
                tpl = tpl.insert_header("retry-after", ra);
            }
            tpl.set_body_string(r#"{"error":"temporary","description":"retry me"}"#)
        } else {
            ResponseTemplate::new(200).set_body_string(self.success_body)
        }
    }
}

#[tokio::test]
async fn retries_transient_503_and_recovers_from_429() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v3/tag/category"))
        .respond_with(SequenceResponder {
            attempts: AtomicUsize::new(0),
            first_status: 503,
            retry_after: None,
            success_body: r#"[{"name":"optimization"}]"#,
        })
        .expect(2)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v2/tag/license"))
        .respond_with(SequenceResponder {
            attempts: AtomicUsize::new(0),
            first_status: 429,
            retry_after: Some("1"),
            success_body: r#"[{"short":"MIT","name":"MIT License"}]"#,
        })
        .expect(2)
        .mount(&server)
        .await;

    let engine = build_test_engine(&server, disabled_cache(), None);
    let cats = engine.get_tag_categories().await.unwrap();
    assert!(std::str::from_utf8(&cats).unwrap().contains("optimization"));

    let lics = engine.get_tag_licenses().await.unwrap();
    assert!(std::str::from_utf8(&lics).unwrap().contains("MIT"));
}

#[tokio::test]
async fn cache_aware_partial_batching_for_domains_and_versions() {
    let server = MockServer::start().await;
    let dir = TempDir::new("batch");

    // Warm individual project 'sodium'.
    Mock::given(method("GET"))
        .and(path("/v3/project/sodium"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"id":"AANobbMI","slug":"sodium","project_types":["mod"]}"#),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Batch projects endpoint must ONLY receive the two uncached items ('iris', 'lithium')!
    Mock::given(method("GET"))
        .and(path("/v3/projects"))
        .and(query_param("ids", r#"["iris","lithium"]"#))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"[{"id":"YL57xq9U","slug":"iris","project_types":["mod"]},{"id":"gvQqBUqZ","slug":"lithium","project_types":["mod"]}]"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    // Warm individual version 'v1'.
    Mock::given(method("GET"))
        .and(path("/v3/version/v1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"{"id":"v1","version_number":"1.0.0"}"#),
        )
        .expect(1)
        .mount(&server)
        .await;

    // Batch versions endpoint must ONLY receive 'v2' and 'v3'.
    Mock::given(method("GET"))
        .and(path("/v3/versions"))
        .and(query_param("ids", r#"["v2","v3"]"#))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"[{"id":"v2","version_number":"2.0.0"},{"id":"v3","version_number":"3.0.0"}]"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let engine = build_test_engine(
        &server,
        CacheConfig {
            cache_dir: Some(dir.0.clone()),
            ..CacheConfig::default()
        },
        None,
    );

    // Warm one project and one version.
    let _ = engine.get_domain(DomainKind::Mod, "sodium").await.unwrap();
    let _ = engine.get_version("v1").await.unwrap();

    // Request [sodium, lithium, iris] -> preserves caller order and caches individual misses.
    let batch_mods = engine
        .get_domains(
            DomainKind::Mod,
            &["sodium".to_owned(), "lithium".to_owned(), "iris".to_owned()],
        )
        .await
        .unwrap();
    assert_eq!(batch_mods.len(), 3);
    assert!(
        std::str::from_utf8(&batch_mods[0])
            .unwrap()
            .contains("sodium")
    );
    assert!(
        std::str::from_utf8(&batch_mods[1])
            .unwrap()
            .contains("lithium")
    );
    assert!(
        std::str::from_utf8(&batch_mods[2])
            .unwrap()
            .contains("iris")
    );

    // Repeat batch call (even querying by ID 'YL57xq9U') -> 0 HTTP requests!
    let repeat_mods = engine
        .get_domains(
            DomainKind::Mod,
            &["YL57xq9U".to_owned(), "sodium".to_owned()],
        )
        .await
        .unwrap();
    assert_eq!(repeat_mods.len(), 2);
    assert!(
        std::str::from_utf8(&repeat_mods[0])
            .unwrap()
            .contains("iris")
    );
    assert!(
        std::str::from_utf8(&repeat_mods[1])
            .unwrap()
            .contains("sodium")
    );

    // Partial batch versions -> preserves order [v3, v1, v2] and caches individual misses.
    let batch_versions = engine
        .get_versions(&["v3".to_owned(), "v1".to_owned(), "v2".to_owned()])
        .await
        .unwrap();
    assert_eq!(batch_versions.len(), 3);
    assert!(
        std::str::from_utf8(&batch_versions[0])
            .unwrap()
            .contains("3.0.0")
    );
    assert!(
        std::str::from_utf8(&batch_versions[1])
            .unwrap()
            .contains("1.0.0")
    );
    assert!(
        std::str::from_utf8(&batch_versions[2])
            .unwrap()
            .contains("2.0.0")
    );

    // Repeat version call -> 0 HTTP requests!
    let repeat_versions = engine
        .get_versions(&["v2".to_owned(), "v3".to_owned()])
        .await
        .unwrap();
    assert_eq!(repeat_versions.len(), 2);
}
