#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::{Arc, Once};

use bytes::Bytes;
use serde::Deserialize;

pub mod cache;
pub mod config;
pub mod domain;
pub mod error;
pub mod http;
pub mod routes;

use self::cache::CacheStats;
use self::cache::codec::CacheEntry;
use self::config::EngineConfig;
use self::domain::{
    DomainKind, SearchFilters, extract_project_id_slug, split_json_array_raw, verify_project_domain,
};
use self::error::EngineError;
use self::http::HttpPipeline;
use self::routes::{CacheTier, MAX_BATCH_IDS, RouteRequest};

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

#[derive(Deserialize)]
struct VersionIdProbe<'a> {
    #[serde(borrow)]
    id: &'a str,
}

/// High-level read-only Modrinth API v3 metadata engine.
///
/// Cheaply cloneable via internal `Arc<HttpPipeline>` so sync and async Python
/// wrappers can share pooled HTTP/2 connections, rate-limit state, and cache locks.
#[derive(Clone)]
pub struct Engine {
    config: EngineConfig,
    pipeline: Arc<HttpPipeline>,
}

impl Engine {
    /// Construct a new [`Engine`] from a validated [`EngineConfig`].
    pub fn new(config: EngineConfig) -> Result<Self, EngineError> {
        let pipeline = Arc::new(HttpPipeline::new(&config)?);
        Ok(Self { config, pipeline })
    }

    /// Expose immutable engine configuration.
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Search Modrinth `/v3/search` with automatic domain facet injection.
    pub async fn search_domain(
        &self,
        domain: DomainKind,
        filters: &SearchFilters,
    ) -> Result<Bytes, EngineError> {
        let route = routes::search(&self.config.base_url, domain, filters)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch a single project from `/v3/project/{id_or_slug}` and verify `domain` membership.
    pub async fn get_domain(
        &self,
        domain: DomainKind,
        id_or_slug: &str,
    ) -> Result<Bytes, EngineError> {
        let route = routes::project(&self.config.base_url, id_or_slug)?;
        let raw = self.pipeline.get_json(&route).await?;
        verify_project_domain(&raw, domain, id_or_slug)?;
        Ok(raw)
    }

    /// Fetch multiple projects using cache-aware partial batching over `/v3/projects`.
    ///
    /// Cached entries are served immediately; only cache misses are queried in chunks
    /// of up to 100 identifiers, split, cached individually under both ID and slug keys,
    /// verified against `domain`, and reassembled in caller order.
    pub async fn get_domains(
        &self,
        domain: DomainKind,
        ids: &[String],
    ) -> Result<Vec<Bytes>, EngineError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let now = HttpPipeline::now_unix();
        let mut resolved: Vec<Option<Bytes>> = vec![None; ids.len()];
        let mut miss_indices_by_id: HashMap<String, Vec<usize>> = HashMap::new();
        let mut unique_misses: Vec<String> = Vec::new();

        for (idx, raw_id) in ids.iter().enumerate() {
            let route = routes::project(&self.config.base_url, raw_id)?;
            let key = self.pipeline.cache_key_for_url(route.canonical_url());
            let ttl_secs = route.tier.ttl_secs(self.pipeline.cache_config());
            if let Some(entry) = self.pipeline.cache().get(&key)
                && entry.is_fresh_within(now, ttl_secs)
            {
                self.pipeline.cache().record_hit();
                resolved[idx] = Some(entry.body);
                continue;
            }

            let trimmed = raw_id.trim().to_owned();
            miss_indices_by_id
                .entry(trimmed.clone())
                .and_modify(|list| list.push(idx))
                .or_insert_with(|| {
                    unique_misses.push(trimmed);
                    vec![idx]
                });
        }

        if !unique_misses.is_empty() {
            let mut fetched_lookup: HashMap<String, Bytes> = HashMap::new();

            for chunk in unique_misses.chunks(MAX_BATCH_IDS) {
                let batch_route = routes::projects(&self.config.base_url, chunk)?;
                let batch_raw = self.pipeline.get_json(&batch_route).await?;
                let items = split_json_array_raw(&batch_raw)?;

                for item in items {
                    let (proj_id, proj_slug) = extract_project_id_slug(&item)?;
                    self.store_individual_project(&proj_id, &proj_slug, &item)?;
                    fetched_lookup.insert(proj_id.clone(), item.clone());
                    fetched_lookup.insert(proj_slug.to_ascii_lowercase(), item);
                }
            }

            for (missed_id, indices) in &miss_indices_by_id {
                let found = fetched_lookup
                    .get(missed_id)
                    .or_else(|| fetched_lookup.get(&missed_id.to_ascii_lowercase()));

                if let Some(raw) = found {
                    for &idx in indices {
                        resolved[idx] = Some(raw.clone());
                    }
                }
            }
        }

        let mut output = Vec::with_capacity(ids.len());
        for (idx, item) in resolved.into_iter().enumerate() {
            if let Some(raw) = item {
                verify_project_domain(&raw, domain, &ids[idx])?;
                output.push(raw);
            }
        }

        Ok(output)
    }

    /// Fetch versions for a project from `/v3/project/{id_or_slug}/version`.
    pub async fn get_project_versions(
        &self,
        id_or_slug: &str,
        loaders: &[String],
        game_versions: &[String],
        featured: Option<bool>,
    ) -> Result<Bytes, EngineError> {
        let route = routes::project_versions(
            &self.config.base_url,
            id_or_slug,
            loaders,
            game_versions,
            featured,
        )?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch a single version from `/v3/version/{version_id}`.
    pub async fn get_version(&self, version_id: &str) -> Result<Bytes, EngineError> {
        let route = routes::version(&self.config.base_url, version_id)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch multiple versions using cache-aware partial batching over `/v3/versions`.
    pub async fn get_versions(&self, version_ids: &[String]) -> Result<Vec<Bytes>, EngineError> {
        if version_ids.is_empty() {
            return Ok(Vec::new());
        }

        let now = HttpPipeline::now_unix();
        let mut resolved: Vec<Option<Bytes>> = vec![None; version_ids.len()];
        let mut miss_indices_by_id: HashMap<String, Vec<usize>> = HashMap::new();
        let mut unique_misses: Vec<String> = Vec::new();

        for (idx, raw_id) in version_ids.iter().enumerate() {
            let route = routes::version(&self.config.base_url, raw_id)?;
            let key = self.pipeline.cache_key_for_url(route.canonical_url());
            let ttl_secs = route.tier.ttl_secs(self.pipeline.cache_config());
            if let Some(entry) = self.pipeline.cache().get(&key)
                && entry.is_fresh_within(now, ttl_secs)
            {
                self.pipeline.cache().record_hit();
                resolved[idx] = Some(entry.body);
                continue;
            }

            let trimmed = raw_id.trim().to_owned();
            miss_indices_by_id
                .entry(trimmed.clone())
                .and_modify(|list| list.push(idx))
                .or_insert_with(|| {
                    unique_misses.push(trimmed);
                    vec![idx]
                });
        }

        if !unique_misses.is_empty() {
            let mut fetched_lookup: HashMap<String, Bytes> = HashMap::new();

            for chunk in unique_misses.chunks(MAX_BATCH_IDS) {
                let batch_route = routes::versions(&self.config.base_url, chunk)?;
                let batch_raw = self.pipeline.get_json(&batch_route).await?;
                let items = split_json_array_raw(&batch_raw)?;

                for item in items {
                    let probe: VersionIdProbe<'_> =
                        serde_json::from_slice(&item).map_err(|err| {
                            EngineError::Serialization(format!(
                                "failed to extract version id: {err}"
                            ))
                        })?;
                    let route = routes::version(&self.config.base_url, probe.id)?;
                    self.store_individual_item(&route, &item);
                    fetched_lookup.insert(probe.id.to_owned(), item);
                }
            }

            for (missed_id, indices) in &miss_indices_by_id {
                if let Some(raw) = fetched_lookup.get(missed_id) {
                    for &idx in indices {
                        resolved[idx] = Some(raw.clone());
                    }
                }
            }
        }

        Ok(resolved.into_iter().flatten().collect())
    }

    /// Look up the version matching a file hash via `/v3/version_file/{hash}`.
    pub async fn get_version_by_hash(
        &self,
        hash: &str,
        algorithm: &str,
    ) -> Result<Bytes, EngineError> {
        let route = routes::version_file(&self.config.base_url, hash, algorithm)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch project dependencies via `/v3/project/{id_or_slug}/dependencies`.
    pub async fn get_project_dependencies(&self, id_or_slug: &str) -> Result<Bytes, EngineError> {
        let route = routes::project_dependencies(&self.config.base_url, id_or_slug)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch project team members via `/v3/project/{id_or_slug}/members`.
    pub async fn get_project_members(&self, id_or_slug: &str) -> Result<Bytes, EngineError> {
        let route = routes::project_members(&self.config.base_url, id_or_slug)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch team members via `/v3/team/{team_id}/members`.
    pub async fn get_team_members(&self, team_id: &str) -> Result<Bytes, EngineError> {
        let route = routes::team_members(&self.config.base_url, team_id)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch a user profile via `/v3/user/{id_or_username}`.
    pub async fn get_user(&self, id_or_username: &str) -> Result<Bytes, EngineError> {
        let route = routes::user(&self.config.base_url, id_or_username)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch projects owned/authored by a user via `/v3/user/{id_or_username}/projects`.
    pub async fn get_user_projects(&self, id_or_username: &str) -> Result<Bytes, EngineError> {
        let route = routes::user_projects(&self.config.base_url, id_or_username)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch an organization via `/v3/organization/{id_or_slug}`.
    pub async fn get_organization(&self, id_or_slug: &str) -> Result<Bytes, EngineError> {
        let route = routes::organization(&self.config.base_url, id_or_slug)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch an organization's projects via `/v3/organization/{id_or_slug}/projects`.
    pub async fn get_organization_projects(&self, id_or_slug: &str) -> Result<Bytes, EngineError> {
        let route = routes::organization_projects(&self.config.base_url, id_or_slug)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch loader tags via `/v3/tag/loader`.
    pub async fn get_tag_loaders(&self) -> Result<Bytes, EngineError> {
        let route = routes::tag_loaders(&self.config.base_url)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch category tags via `/v3/tag/category`.
    pub async fn get_tag_categories(&self) -> Result<Bytes, EngineError> {
        let route = routes::tag_categories(&self.config.base_url)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch game version tags via documented fallback `GET /v2/tag/game_version`.
    pub async fn get_tag_game_versions(&self) -> Result<Bytes, EngineError> {
        let route = routes::tag_game_versions(&self.config.base_url)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch license tags via documented fallback `GET /v2/tag/license`.
    pub async fn get_tag_licenses(&self) -> Result<Bytes, EngineError> {
        let route = routes::tag_licenses(&self.config.base_url)?;
        self.pipeline.get_json(&route).await
    }

    /// Fetch project type tags via documented fallback `GET /v2/tag/project_type`.
    pub async fn get_tag_project_types(&self) -> Result<Bytes, EngineError> {
        let route = routes::tag_project_types(&self.config.base_url)?;
        self.pipeline.get_json(&route).await
    }

    /// Return current cache counters and footprint statistics.
    pub fn cache_stats(&self) -> CacheStats {
        self.pipeline.cache().stats()
    }

    /// Clear all cached entries from the active cache backend.
    pub fn clear_cache(&self) -> Result<(), EngineError> {
        self.pipeline.cache().clear()
    }

    fn store_individual_project(
        &self,
        id: &str,
        slug: &str,
        body: &Bytes,
    ) -> Result<(), EngineError> {
        let id_route = routes::project(&self.config.base_url, id)?;
        self.store_individual_item(&id_route, body);

        if slug != id {
            let slug_route = routes::project(&self.config.base_url, slug)?;
            self.store_individual_item(&slug_route, body);
        }
        Ok(())
    }

    fn store_individual_item(&self, route: &RouteRequest, body: &Bytes) {
        let entry = CacheEntry {
            status: 200,
            stored_at_unix: HttpPipeline::now_unix(),
            ttl_secs: CacheTier::ttl_secs(route.tier, self.pipeline.cache_config()),
            etag: None,
            last_modified: None,
            body: body.clone(),
        };
        let key = self.pipeline.cache_key_for_url(route.canonical_url());
        self.pipeline.cache().put(&key, &entry);
    }
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
