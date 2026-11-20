use url::Url;

use crate::config::CacheConfig;
use crate::domain::{DomainKind, SearchFilters, build_facets_json};
use crate::error::EngineError;

/// Maximum identifiers permitted per Modrinth batch request (`/v3/projects` or `/v3/versions`).
pub const MAX_BATCH_IDS: usize = 100;

/// Cache TTL category associated with a Modrinth API route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheTier {
    Search,
    Project,
    VersionList,
    Version,
    Tag,
}

impl CacheTier {
    /// Resolve the fresh TTL in seconds from `CacheConfig`.
    pub const fn ttl_secs(self, cfg: &CacheConfig) -> u32 {
        match self {
            Self::Search => cfg.search_ttl_secs,
            Self::Project => cfg.project_ttl_secs,
            Self::VersionList => cfg.version_list_ttl_secs,
            Self::Version => cfg.version_ttl_secs,
            Self::Tag => cfg.tag_ttl_secs,
        }
    }
}

/// Canonical URL + cache policy tier produced by the centralized route builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRequest {
    pub url: Url,
    pub tier: CacheTier,
}

impl RouteRequest {
    /// Canonical string form used both as outbound HTTP URL and cache key input.
    pub fn canonical_url(&self) -> &str {
        self.url.as_str()
    }
}

/// Build `GET /v3/search` with domain facet injection and deterministic query ordering.
pub fn search(
    base: &Url,
    domain: DomainKind,
    filters: &SearchFilters,
) -> Result<RouteRequest, EngineError> {
    if !(1..=100).contains(&filters.limit) {
        return Err(EngineError::Config(format!(
            "search limit must be between 1 and 100, got {}",
            filters.limit
        )));
    }

    let facets_json = build_facets_json(domain, filters)?;
    let mut url = build_endpoint_url(base, &["v3", "search"])?;

    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("facets", &facets_json);
        pairs.append_pair("index", filters.sort.as_str());
        pairs.append_pair("limit", &filters.limit.to_string());
        pairs.append_pair("offset", &filters.offset.to_string());

        let query = filters.query.trim();
        if !query.is_empty() {
            pairs.append_pair("query", query);
        }
    }

    Ok(RouteRequest {
        url,
        tier: CacheTier::Search,
    })
}

/// Build `GET /v3/project/{id_or_slug}`.
pub fn project(base: &Url, id_or_slug: &str) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v3", "project", id_or_slug])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Project,
    })
}

/// Build `GET /v3/projects?ids=[...]` (up to 100 IDs/slugs per call).
pub fn projects(base: &Url, ids: &[String]) -> Result<RouteRequest, EngineError> {
    let ids_json = serialize_batch_ids(ids, "projects")?;
    let mut url = build_endpoint_url(base, &["v3", "projects"])?;
    url.query_pairs_mut().append_pair("ids", &ids_json);

    Ok(RouteRequest {
        url,
        tier: CacheTier::Project,
    })
}

/// Build `GET /v3/project/{id_or_slug}/check`.
pub fn project_check(base: &Url, id_or_slug: &str) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v3", "project", id_or_slug, "check"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Project,
    })
}

/// Build `GET /v3/project/{id_or_slug}/version` with optional loader, game version, and featured filters.
pub fn project_versions(
    base: &Url,
    id_or_slug: &str,
    loaders: &[String],
    game_versions: &[String],
    featured: Option<bool>,
) -> Result<RouteRequest, EngineError> {
    let mut url = build_endpoint_url(base, &["v3", "project", id_or_slug, "version"])?;

    let featured_str = featured.map(|val| if val { "true" } else { "false" });
    let game_versions_json = serialize_filter_array(game_versions)?;
    let loaders_json = serialize_filter_array(loaders)?;

    if featured_str.is_some() || game_versions_json.is_some() || loaders_json.is_some() {
        let mut pairs = url.query_pairs_mut();
        if let Some(val) = featured_str {
            pairs.append_pair("featured", val);
        }
        if let Some(val) = &game_versions_json {
            pairs.append_pair("game_versions", val);
        }
        if let Some(val) = &loaders_json {
            pairs.append_pair("loaders", val);
        }
    }

    Ok(RouteRequest {
        url,
        tier: CacheTier::VersionList,
    })
}

/// Build `GET /v3/version/{version_id}`.
pub fn version(base: &Url, version_id: &str) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v3", "version", version_id])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Version,
    })
}

/// Build `GET /v3/versions?ids=[...]` (up to 100 version IDs per call).
pub fn versions(base: &Url, ids: &[String]) -> Result<RouteRequest, EngineError> {
    let ids_json = serialize_batch_ids(ids, "versions")?;
    let mut url = build_endpoint_url(base, &["v3", "versions"])?;
    url.query_pairs_mut().append_pair("ids", &ids_json);

    Ok(RouteRequest {
        url,
        tier: CacheTier::Version,
    })
}

/// Build `GET /v3/version_file/{hash}?algorithm=sha512|sha1`.
pub fn version_file(base: &Url, hash: &str, algorithm: &str) -> Result<RouteRequest, EngineError> {
    let normalized_algo = algorithm.trim().to_ascii_lowercase();
    if normalized_algo != "sha512" && normalized_algo != "sha1" {
        return Err(EngineError::Config(format!(
            "hash algorithm must be 'sha512' or 'sha1', got '{algorithm}'"
        )));
    }

    let normalized_hash = hash.trim();
    if normalized_hash.is_empty() || !normalized_hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(EngineError::Config(
            "version_file hash must be a non-empty hexadecimal string".to_owned(),
        ));
    }

    let mut url = build_endpoint_url(base, &["v3", "version_file", normalized_hash])?;
    url.query_pairs_mut()
        .append_pair("algorithm", &normalized_algo);

    Ok(RouteRequest {
        url,
        tier: CacheTier::Version,
    })
}

/// Build `GET /v3/project/{id_or_slug}/dependencies`.
pub fn project_dependencies(base: &Url, id_or_slug: &str) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v3", "project", id_or_slug, "dependencies"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Project,
    })
}

/// Build `GET /v3/project/{id_or_slug}/members`.
pub fn project_members(base: &Url, id_or_slug: &str) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v3", "project", id_or_slug, "members"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Project,
    })
}

/// Build `GET /v3/team/{team_id}/members`.
pub fn team_members(base: &Url, team_id: &str) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v3", "team", team_id, "members"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Project,
    })
}

/// Build `GET /v3/user/{id_or_username}`.
pub fn user(base: &Url, id_or_username: &str) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v3", "user", id_or_username])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Project,
    })
}

/// Build `GET /v3/user/{id_or_username}/projects`.
pub fn user_projects(base: &Url, id_or_username: &str) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v3", "user", id_or_username, "projects"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Project,
    })
}

/// Build `GET /v3/organization/{id_or_slug}`.
pub fn organization(base: &Url, id_or_slug: &str) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v3", "organization", id_or_slug])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Project,
    })
}

/// Build `GET /v3/organization/{id_or_slug}/projects`.
pub fn organization_projects(base: &Url, id_or_slug: &str) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v3", "organization", id_or_slug, "projects"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Project,
    })
}

/// Build `GET /v3/tag/loader`.
pub fn tag_loaders(base: &Url) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v3", "tag", "loader"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Tag,
    })
}

/// Build `GET /v3/tag/category`.
pub fn tag_categories(base: &Url) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v3", "tag", "category"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Tag,
    })
}

// ---------------------------------------------------------------------------
// Documented `/v2` tag fallbacks (see `docs/api/v3-endpoints.md`).
// Re-verify against `/v3/tag/*` on every release check.
// ---------------------------------------------------------------------------

/// Build `GET /v2/tag/game_version` (documented v2 fallback; `/v3/tag/game_version` returns 404).
pub fn tag_game_versions(base: &Url) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v2", "tag", "game_version"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Tag,
    })
}

/// Build `GET /v2/tag/license` (documented v2 fallback; `/v3/tag/license` returns 404).
pub fn tag_licenses(base: &Url) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v2", "tag", "license"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Tag,
    })
}

/// Build `GET /v2/tag/project_type` (documented v2 fallback; `/v3/tag/project_type` returns 404).
pub fn tag_project_types(base: &Url) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v2", "tag", "project_type"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Tag,
    })
}

/// Build `GET /v2/tag/donation_platform` (documented v2 fallback; `/v3/tag/donation_platform` returns 404).
pub fn tag_donation_platforms(base: &Url) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v2", "tag", "donation_platform"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Tag,
    })
}

/// Build `GET /v2/tag/report_type` (documented v2 fallback; `/v3/tag/report_type` returns 404).
pub fn tag_report_types(base: &Url) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v2", "tag", "report_type"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Tag,
    })
}

/// Build `GET /v2/tag/side_type` (documented v2 fallback; `/v3/tag/side_type` returns 404).
pub fn tag_side_types(base: &Url) -> Result<RouteRequest, EngineError> {
    let url = build_endpoint_url(base, &["v2", "tag", "side_type"])?;
    Ok(RouteRequest {
        url,
        tier: CacheTier::Tag,
    })
}

fn build_endpoint_url(base: &Url, segments: &[&str]) -> Result<Url, EngineError> {
    let mut url = base.clone();
    url.set_query(None);
    url.set_fragment(None);

    let mut path_mut = url.path_segments_mut().map_err(|()| {
        EngineError::Config("base_url cannot be a cannot-be-a-base URL".to_owned())
    })?;
    path_mut.pop_if_empty();

    for raw_seg in segments {
        let seg = raw_seg.trim();
        if seg.is_empty() || seg == "." || seg == ".." || seg.contains('/') || seg.contains('\\') {
            return Err(EngineError::Config(format!(
                "invalid path segment '{raw_seg}' in endpoint URL"
            )));
        }
        path_mut.push(seg);
    }
    drop(path_mut);

    Ok(url)
}

fn serialize_batch_ids(ids: &[String], resource_label: &str) -> Result<String, EngineError> {
    let mut cleaned: Vec<&str> = ids
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    cleaned.sort_unstable();
    cleaned.dedup();

    if cleaned.is_empty() {
        return Err(EngineError::Config(format!(
            "{resource_label} batch requires at least one non-empty identifier"
        )));
    }

    if cleaned.len() > MAX_BATCH_IDS {
        return Err(EngineError::Config(format!(
            "{resource_label} batch accepts at most {MAX_BATCH_IDS} identifiers per request, got {}",
            cleaned.len()
        )));
    }

    serde_json::to_string(&cleaned).map_err(|err| EngineError::Serialization(err.to_string()))
}

fn serialize_filter_array(values: &[String]) -> Result<Option<String>, EngineError> {
    let mut cleaned: Vec<&str> = values
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if cleaned.is_empty() {
        return Ok(None);
    }

    cleaned.sort_unstable();
    cleaned.dedup();

    serde_json::to_string(&cleaned)
        .map(Some)
        .map_err(|err| EngineError::Serialization(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SortIndex;

    fn base() -> Url {
        Url::parse("https://api.modrinth.com").unwrap()
    }

    #[test]
    fn builds_v3_search_with_injected_facets_and_sorted_params() {
        let route = search(
            &base(),
            DomainKind::Server,
            &SearchFilters {
                query: "aero".to_owned(),
                sort: SortIndex::Downloads,
                limit: 20,
                offset: 40,
                ..SearchFilters::default()
            },
        )
        .unwrap();

        assert_eq!(route.tier, CacheTier::Search);
        assert_eq!(route.url.path(), "/v3/search");
        let pairs: Vec<(String, String)> = route.url.query_pairs().into_owned().collect();
        assert_eq!(
            pairs,
            vec![
                (
                    "facets".to_owned(),
                    r#"[["project_types:minecraft_java_server"]]"#.to_owned()
                ),
                ("index".to_owned(), "downloads".to_owned()),
                ("limit".to_owned(), "20".to_owned()),
                ("offset".to_owned(), "40".to_owned()),
                ("query".to_owned(), "aero".to_owned()),
            ]
        );
    }

    #[test]
    fn encodes_path_segments_safely_against_traversal() {
        assert!(matches!(
            project(&base(), "../admin"),
            Err(EngineError::Config(_))
        ));
        assert!(matches!(
            project(&base(), ".."),
            Err(EngineError::Config(_))
        ));
        assert!(matches!(
            project(&base(), "foo/bar"),
            Err(EngineError::Config(_))
        ));

        let encoded = project(&base(), "foo?x=1#frag").unwrap();
        assert_eq!(encoded.url.path(), "/v3/project/foo%3Fx=1%23frag");
        assert!(encoded.url.query().is_none());
        assert!(encoded.url.fragment().is_none());

        let with_prefix = Url::parse("https://proxy.example.com/modrinth/").unwrap();
        let req = project(&with_prefix, "sodium").unwrap();
        assert_eq!(
            req.canonical_url(),
            "https://proxy.example.com/modrinth/v3/project/sodium"
        );
    }

    #[test]
    fn builds_batch_and_version_file_and_v2_fallbacks() {
        let batch = projects(
            &base(),
            &[
                "sodium".to_owned(),
                "luckperms".to_owned(),
                "sodium".to_owned(),
            ],
        )
        .unwrap();
        assert_eq!(batch.tier, CacheTier::Project);
        assert_eq!(batch.url.path(), "/v3/projects");
        assert_eq!(
            batch.url.query_pairs().next().unwrap().1,
            r#"["luckperms","sodium"]"#
        );

        let hash_req = version_file(&base(), "abcdef0123456789", "SHA512").unwrap();
        assert_eq!(hash_req.tier, CacheTier::Version);
        assert_eq!(hash_req.url.path(), "/v3/version_file/abcdef0123456789");
        assert_eq!(hash_req.url.query(), Some("algorithm=sha512"));

        assert!(matches!(
            version_file(&base(), "not-hex!", "sha512"),
            Err(EngineError::Config(_))
        ));
        assert!(matches!(
            version_file(&base(), "abcd", "md5"),
            Err(EngineError::Config(_))
        ));

        assert_eq!(tag_loaders(&base()).unwrap().url.path(), "/v3/tag/loader");
        assert_eq!(
            tag_categories(&base()).unwrap().url.path(),
            "/v3/tag/category"
        );
        assert_eq!(
            tag_game_versions(&base()).unwrap().url.path(),
            "/v2/tag/game_version"
        );
        assert_eq!(tag_licenses(&base()).unwrap().url.path(), "/v2/tag/license");
        assert_eq!(
            tag_project_types(&base()).unwrap().url.path(),
            "/v2/tag/project_type"
        );
    }
}
