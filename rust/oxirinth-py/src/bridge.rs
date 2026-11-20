use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use oxirinth_engine::Engine;
use oxirinth_engine::cache::CacheStats as EngineCacheStats;
use oxirinth_engine::config::{CacheConfig, EngineConfig, SecretString, default_user_agent};
use oxirinth_engine::domain::{DomainKind, SearchFilters, SortIndex};
use oxirinth_engine::error::EngineError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use url::Url;

use crate::errors::{OxirinthConfigError, engine_error_to_py};
use crate::runtime::{catch_unwind_py, get_runtime};

/// Runtime cache usage statistics exposed from `oxirinth-engine`.
#[pyclass(name = "CacheStats", module = "oxirinth._core", frozen, get_all)]
#[derive(Debug, Clone)]
pub struct PyCacheStats {
    pub backend: String,
    pub entries: u64,
    pub hits: u64,
    pub misses: u64,
    pub revalidations: u64,
    pub size_bytes: u64,
}

#[pymethods]
impl PyCacheStats {
    fn __repr__(&self) -> String {
        format!(
            "CacheStats(backend={:?}, entries={}, hits={}, misses={}, revalidations={}, size_bytes={})",
            self.backend, self.entries, self.hits, self.misses, self.revalidations, self.size_bytes
        )
    }
}

impl From<EngineCacheStats> for PyCacheStats {
    fn from(stats: EngineCacheStats) -> Self {
        Self {
            backend: stats.backend.to_owned(),
            entries: stats.entries,
            hits: stats.hits,
            misses: stats.misses,
            revalidations: stats.revalidations,
            size_bytes: stats.size_bytes,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_engine(
    user_agent: Option<String>,
    api_key: Option<String>,
    cache_enabled: bool,
    cache_dir: Option<PathBuf>,
    ram_fallback_max_bytes: usize,
    zstd_min_bytes: usize,
    zstd_level: i32,
    search_ttl_secs: u32,
    project_ttl_secs: u32,
    version_list_ttl_secs: u32,
    version_ttl_secs: u32,
    tag_ttl_secs: u32,
    timeout_secs: f64,
    max_retries: u32,
    max_rate_limit_wait_secs: Option<f64>,
    rate_limit_safety_margin: u32,
    base_url: Option<String>,
) -> PyResult<Arc<Engine>> {
    if !timeout_secs.is_finite() || timeout_secs <= 0.0 {
        return Err(OxirinthConfigError::new_err(
            "timeout_secs must be a finite positive number",
        ));
    }

    let max_rate_limit_wait = match max_rate_limit_wait_secs {
        Some(secs) if !secs.is_finite() || secs < 0.0 => {
            return Err(OxirinthConfigError::new_err(
                "max_rate_limit_wait_secs must be finite and non-negative",
            ));
        }
        Some(secs) => Some(Duration::from_secs_f64(secs)),
        None => None,
    };

    let parsed_base_url = match base_url {
        Some(raw) => Url::parse(&raw).map_err(|err| {
            OxirinthConfigError::new_err(format!("invalid base_url '{raw}': {err}"))
        })?,
        None => EngineConfig::default().base_url,
    };

    let config = EngineConfig {
        base_url: parsed_base_url,
        user_agent: user_agent.unwrap_or_else(default_user_agent),
        api_key: api_key.map(SecretString::new),
        timeout: Duration::from_secs_f64(timeout_secs),
        max_retries,
        max_rate_limit_wait,
        rate_limit_safety_margin,
        cache: CacheConfig {
            enabled: cache_enabled,
            cache_dir,
            ram_fallback_max_bytes,
            zstd_min_bytes,
            zstd_level,
            search_ttl_secs,
            project_ttl_secs,
            version_list_ttl_secs,
            version_ttl_secs,
            tag_ttl_secs,
        },
    };

    let engine = Engine::new(config).map_err(engine_error_to_py)?;
    Ok(Arc::new(engine))
}

#[allow(clippy::too_many_arguments)]
fn build_search_args(
    domain: &str,
    query: String,
    loaders: Option<Vec<String>>,
    game_versions: Option<Vec<String>>,
    categories: Option<Vec<String>>,
    environment: Option<String>,
    sort: &str,
    limit: u32,
    offset: u32,
) -> PyResult<(DomainKind, SearchFilters)> {
    let kind = DomainKind::from_name(domain).map_err(engine_error_to_py)?;
    let sort_index = SortIndex::from_name(sort).map_err(engine_error_to_py)?;
    let filters = SearchFilters {
        query,
        loaders: loaders.unwrap_or_default(),
        game_versions: game_versions.unwrap_or_default(),
        categories: categories.unwrap_or_default(),
        environment,
        sort: sort_index,
        limit,
        offset,
    };
    Ok((kind, filters))
}

fn bytes_to_py(py: Python<'_>, data: &Bytes) -> Py<PyBytes> {
    PyBytes::new(py, data).unbind()
}

fn bytes_vec_to_py(py: Python<'_>, items: &[Bytes]) -> Vec<Py<PyBytes>> {
    items.iter().map(|b| PyBytes::new(py, b).unbind()).collect()
}

fn run_blocking_bytes<Fut>(py: Python<'_>, fut: Fut) -> PyResult<Py<PyBytes>>
where
    Fut: Future<Output = Result<Bytes, EngineError>> + Send,
{
    let rt = get_runtime()?;
    let bytes = py.detach(|| rt.block_on(fut)).map_err(engine_error_to_py)?;
    Ok(bytes_to_py(py, &bytes))
}

fn run_blocking_bytes_vec<Fut>(py: Python<'_>, fut: Fut) -> PyResult<Vec<Py<PyBytes>>>
where
    Fut: Future<Output = Result<Vec<Bytes>, EngineError>> + Send,
{
    let rt = get_runtime()?;
    let items = py.detach(|| rt.block_on(fut)).map_err(engine_error_to_py)?;
    Ok(bytes_vec_to_py(py, &items))
}

fn run_async_bytes<'py, Fut>(py: Python<'py>, fut: Fut) -> PyResult<Bound<'py, PyAny>>
where
    Fut: Future<Output = Result<Bytes, EngineError>> + Send + 'static,
{
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let bytes = fut.await.map_err(engine_error_to_py)?;
        Ok(Python::attach(|py| bytes_to_py(py, &bytes)))
    })
}

fn run_async_bytes_vec<'py, Fut>(py: Python<'py>, fut: Fut) -> PyResult<Bound<'py, PyAny>>
where
    Fut: Future<Output = Result<Vec<Bytes>, EngineError>> + Send + 'static,
{
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let items = fut.await.map_err(engine_error_to_py)?;
        Ok(Python::attach(|py| bytes_vec_to_py(py, &items)))
    })
}

/// Synchronous GIL-releasing FFI bridge to `oxirinth-engine`.
#[pyclass(name = "SyncCore", module = "oxirinth._core", frozen)]
pub struct SyncCore {
    engine: Arc<Engine>,
}

#[pymethods]
impl SyncCore {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        *,
        user_agent = None,
        api_key = None,
        cache_enabled = true,
        cache_dir = None,
        ram_fallback_max_bytes = 67_108_864,
        zstd_min_bytes = 512,
        zstd_level = 3,
        search_ttl_secs = 120,
        project_ttl_secs = 900,
        version_list_ttl_secs = 300,
        version_ttl_secs = 86_400,
        tag_ttl_secs = 21_600,
        timeout_secs = 30.0,
        max_retries = 3,
        max_rate_limit_wait_secs = None,
        rate_limit_safety_margin = 2,
        base_url = None,
    ))]
    fn new(
        user_agent: Option<String>,
        api_key: Option<String>,
        cache_enabled: bool,
        cache_dir: Option<PathBuf>,
        ram_fallback_max_bytes: usize,
        zstd_min_bytes: usize,
        zstd_level: i32,
        search_ttl_secs: u32,
        project_ttl_secs: u32,
        version_list_ttl_secs: u32,
        version_ttl_secs: u32,
        tag_ttl_secs: u32,
        timeout_secs: f64,
        max_retries: u32,
        max_rate_limit_wait_secs: Option<f64>,
        rate_limit_safety_margin: u32,
        base_url: Option<String>,
    ) -> PyResult<Self> {
        catch_unwind_py(|| {
            let engine = build_engine(
                user_agent,
                api_key,
                cache_enabled,
                cache_dir,
                ram_fallback_max_bytes,
                zstd_min_bytes,
                zstd_level,
                search_ttl_secs,
                project_ttl_secs,
                version_list_ttl_secs,
                version_ttl_secs,
                tag_ttl_secs,
                timeout_secs,
                max_retries,
                max_rate_limit_wait_secs,
                rate_limit_safety_margin,
                base_url,
            )?;
            Ok(Self { engine })
        })
    }

    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        domain,
        query = "",
        *,
        loaders = None,
        game_versions = None,
        categories = None,
        environment = None,
        sort = "relevance",
        limit = 10,
        offset = 0,
    ))]
    fn search_domain(
        &self,
        py: Python<'_>,
        domain: &str,
        query: &str,
        loaders: Option<Vec<String>>,
        game_versions: Option<Vec<String>>,
        categories: Option<Vec<String>>,
        environment: Option<String>,
        sort: &str,
        limit: u32,
        offset: u32,
    ) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| {
            let (kind, filters) = build_search_args(
                domain,
                query.to_owned(),
                loaders,
                game_versions,
                categories,
                environment,
                sort,
                limit,
                offset,
            )?;
            run_blocking_bytes(py, self.engine.search_domain(kind, &filters))
        })
    }

    fn get_domain(&self, py: Python<'_>, domain: &str, id_or_slug: &str) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| {
            let kind = DomainKind::from_name(domain).map_err(engine_error_to_py)?;
            run_blocking_bytes(py, self.engine.get_domain(kind, id_or_slug))
        })
    }

    fn get_domains(
        &self,
        py: Python<'_>,
        domain: &str,
        ids: Vec<String>,
    ) -> PyResult<Vec<Py<PyBytes>>> {
        catch_unwind_py(|| {
            let kind = DomainKind::from_name(domain).map_err(engine_error_to_py)?;
            run_blocking_bytes_vec(py, self.engine.get_domains(kind, &ids))
        })
    }

    #[pyo3(signature = (id_or_slug, *, loaders = None, game_versions = None, featured = None))]
    fn get_project_versions(
        &self,
        py: Python<'_>,
        id_or_slug: &str,
        loaders: Option<Vec<String>>,
        game_versions: Option<Vec<String>>,
        featured: Option<bool>,
    ) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| {
            let loaders_vec = loaders.unwrap_or_default();
            let game_versions_vec = game_versions.unwrap_or_default();
            run_blocking_bytes(
                py,
                self.engine.get_project_versions(
                    id_or_slug,
                    &loaders_vec,
                    &game_versions_vec,
                    featured,
                ),
            )
        })
    }

    fn get_version(&self, py: Python<'_>, version_id: &str) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| run_blocking_bytes(py, self.engine.get_version(version_id)))
    }

    fn get_versions(&self, py: Python<'_>, version_ids: Vec<String>) -> PyResult<Vec<Py<PyBytes>>> {
        catch_unwind_py(|| run_blocking_bytes_vec(py, self.engine.get_versions(&version_ids)))
    }

    #[pyo3(signature = (hash_value, *, algorithm = "sha512"))]
    fn get_version_by_hash(
        &self,
        py: Python<'_>,
        hash_value: &str,
        algorithm: &str,
    ) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| {
            run_blocking_bytes(py, self.engine.get_version_by_hash(hash_value, algorithm))
        })
    }

    fn get_project_dependencies(&self, py: Python<'_>, id_or_slug: &str) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| run_blocking_bytes(py, self.engine.get_project_dependencies(id_or_slug)))
    }

    fn get_project_members(&self, py: Python<'_>, id_or_slug: &str) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| run_blocking_bytes(py, self.engine.get_project_members(id_or_slug)))
    }

    fn get_team_members(&self, py: Python<'_>, team_id: &str) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| run_blocking_bytes(py, self.engine.get_team_members(team_id)))
    }

    fn get_user(&self, py: Python<'_>, id_or_username: &str) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| run_blocking_bytes(py, self.engine.get_user(id_or_username)))
    }

    fn get_user_projects(&self, py: Python<'_>, id_or_username: &str) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| run_blocking_bytes(py, self.engine.get_user_projects(id_or_username)))
    }

    fn get_organization(&self, py: Python<'_>, id_or_slug: &str) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| run_blocking_bytes(py, self.engine.get_organization(id_or_slug)))
    }

    fn get_organization_projects(&self, py: Python<'_>, id_or_slug: &str) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| {
            run_blocking_bytes(py, self.engine.get_organization_projects(id_or_slug))
        })
    }

    fn get_tag_loaders(&self, py: Python<'_>) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| run_blocking_bytes(py, self.engine.get_tag_loaders()))
    }

    fn get_tag_categories(&self, py: Python<'_>) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| run_blocking_bytes(py, self.engine.get_tag_categories()))
    }

    fn get_tag_game_versions(&self, py: Python<'_>) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| run_blocking_bytes(py, self.engine.get_tag_game_versions()))
    }

    fn get_tag_licenses(&self, py: Python<'_>) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| run_blocking_bytes(py, self.engine.get_tag_licenses()))
    }

    fn get_tag_project_types(&self, py: Python<'_>) -> PyResult<Py<PyBytes>> {
        catch_unwind_py(|| run_blocking_bytes(py, self.engine.get_tag_project_types()))
    }

    fn cache_stats(&self) -> PyResult<PyCacheStats> {
        catch_unwind_py(|| Ok(self.engine.cache_stats().into()))
    }

    fn clear_cache(&self) -> PyResult<()> {
        catch_unwind_py(|| self.engine.clear_cache().map_err(engine_error_to_py))
    }
}

/// Asynchronous `asyncio`-bridged FFI wrapper over `oxirinth-engine`.
#[pyclass(name = "AsyncCore", module = "oxirinth._core", frozen)]
pub struct AsyncCore {
    engine: Arc<Engine>,
}

#[pymethods]
impl AsyncCore {
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        *,
        user_agent = None,
        api_key = None,
        cache_enabled = true,
        cache_dir = None,
        ram_fallback_max_bytes = 67_108_864,
        zstd_min_bytes = 512,
        zstd_level = 3,
        search_ttl_secs = 120,
        project_ttl_secs = 900,
        version_list_ttl_secs = 300,
        version_ttl_secs = 86_400,
        tag_ttl_secs = 21_600,
        timeout_secs = 30.0,
        max_retries = 3,
        max_rate_limit_wait_secs = None,
        rate_limit_safety_margin = 2,
        base_url = None,
    ))]
    fn new(
        user_agent: Option<String>,
        api_key: Option<String>,
        cache_enabled: bool,
        cache_dir: Option<PathBuf>,
        ram_fallback_max_bytes: usize,
        zstd_min_bytes: usize,
        zstd_level: i32,
        search_ttl_secs: u32,
        project_ttl_secs: u32,
        version_list_ttl_secs: u32,
        version_ttl_secs: u32,
        tag_ttl_secs: u32,
        timeout_secs: f64,
        max_retries: u32,
        max_rate_limit_wait_secs: Option<f64>,
        rate_limit_safety_margin: u32,
        base_url: Option<String>,
    ) -> PyResult<Self> {
        catch_unwind_py(|| {
            let engine = build_engine(
                user_agent,
                api_key,
                cache_enabled,
                cache_dir,
                ram_fallback_max_bytes,
                zstd_min_bytes,
                zstd_level,
                search_ttl_secs,
                project_ttl_secs,
                version_list_ttl_secs,
                version_ttl_secs,
                tag_ttl_secs,
                timeout_secs,
                max_retries,
                max_rate_limit_wait_secs,
                rate_limit_safety_margin,
                base_url,
            )?;
            Ok(Self { engine })
        })
    }

    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        domain,
        query = "",
        *,
        loaders = None,
        game_versions = None,
        categories = None,
        environment = None,
        sort = "relevance",
        limit = 10,
        offset = 0,
    ))]
    fn search_domain<'py>(
        &self,
        py: Python<'py>,
        domain: &str,
        query: &str,
        loaders: Option<Vec<String>>,
        game_versions: Option<Vec<String>>,
        categories: Option<Vec<String>>,
        environment: Option<String>,
        sort: &str,
        limit: u32,
        offset: u32,
    ) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let (kind, filters) = build_search_args(
                domain,
                query.to_owned(),
                loaders,
                game_versions,
                categories,
                environment,
                sort,
                limit,
                offset,
            )?;
            let engine = Arc::clone(&self.engine);
            run_async_bytes(
                py,
                async move { engine.search_domain(kind, &filters).await },
            )
        })
    }

    fn get_domain<'py>(
        &self,
        py: Python<'py>,
        domain: &str,
        id_or_slug: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let kind = DomainKind::from_name(domain).map_err(engine_error_to_py)?;
            let id = id_or_slug.to_owned();
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move { engine.get_domain(kind, &id).await })
        })
    }

    fn get_domains<'py>(
        &self,
        py: Python<'py>,
        domain: &str,
        ids: Vec<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let kind = DomainKind::from_name(domain).map_err(engine_error_to_py)?;
            let engine = Arc::clone(&self.engine);
            run_async_bytes_vec(py, async move { engine.get_domains(kind, &ids).await })
        })
    }

    #[pyo3(signature = (id_or_slug, *, loaders = None, game_versions = None, featured = None))]
    fn get_project_versions<'py>(
        &self,
        py: Python<'py>,
        id_or_slug: &str,
        loaders: Option<Vec<String>>,
        game_versions: Option<Vec<String>>,
        featured: Option<bool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let id = id_or_slug.to_owned();
            let loaders_vec = loaders.unwrap_or_default();
            let game_versions_vec = game_versions.unwrap_or_default();
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move {
                engine
                    .get_project_versions(&id, &loaders_vec, &game_versions_vec, featured)
                    .await
            })
        })
    }

    fn get_version<'py>(&self, py: Python<'py>, version_id: &str) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let id = version_id.to_owned();
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move { engine.get_version(&id).await })
        })
    }

    fn get_versions<'py>(
        &self,
        py: Python<'py>,
        version_ids: Vec<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let engine = Arc::clone(&self.engine);
            run_async_bytes_vec(py, async move { engine.get_versions(&version_ids).await })
        })
    }

    #[pyo3(signature = (hash_value, *, algorithm = "sha512"))]
    fn get_version_by_hash<'py>(
        &self,
        py: Python<'py>,
        hash_value: &str,
        algorithm: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let hash = hash_value.to_owned();
            let algo = algorithm.to_owned();
            let engine = Arc::clone(&self.engine);
            run_async_bytes(
                py,
                async move { engine.get_version_by_hash(&hash, &algo).await },
            )
        })
    }

    fn get_project_dependencies<'py>(
        &self,
        py: Python<'py>,
        id_or_slug: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let id = id_or_slug.to_owned();
            let engine = Arc::clone(&self.engine);
            run_async_bytes(
                py,
                async move { engine.get_project_dependencies(&id).await },
            )
        })
    }

    fn get_project_members<'py>(
        &self,
        py: Python<'py>,
        id_or_slug: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let id = id_or_slug.to_owned();
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move { engine.get_project_members(&id).await })
        })
    }

    fn get_team_members<'py>(&self, py: Python<'py>, team_id: &str) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let id = team_id.to_owned();
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move { engine.get_team_members(&id).await })
        })
    }

    fn get_user<'py>(&self, py: Python<'py>, id_or_username: &str) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let id = id_or_username.to_owned();
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move { engine.get_user(&id).await })
        })
    }

    fn get_user_projects<'py>(
        &self,
        py: Python<'py>,
        id_or_username: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let id = id_or_username.to_owned();
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move { engine.get_user_projects(&id).await })
        })
    }

    fn get_organization<'py>(
        &self,
        py: Python<'py>,
        id_or_slug: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let id = id_or_slug.to_owned();
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move { engine.get_organization(&id).await })
        })
    }

    fn get_organization_projects<'py>(
        &self,
        py: Python<'py>,
        id_or_slug: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let id = id_or_slug.to_owned();
            let engine = Arc::clone(&self.engine);
            run_async_bytes(
                py,
                async move { engine.get_organization_projects(&id).await },
            )
        })
    }

    fn get_tag_loaders<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move { engine.get_tag_loaders().await })
        })
    }

    fn get_tag_categories<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move { engine.get_tag_categories().await })
        })
    }

    fn get_tag_game_versions<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move { engine.get_tag_game_versions().await })
        })
    }

    fn get_tag_licenses<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move { engine.get_tag_licenses().await })
        })
    }

    fn get_tag_project_types<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        catch_unwind_py(|| {
            let engine = Arc::clone(&self.engine);
            run_async_bytes(py, async move { engine.get_tag_project_types().await })
        })
    }

    fn cache_stats(&self) -> PyResult<PyCacheStats> {
        catch_unwind_py(|| Ok(self.engine.cache_stats().into()))
    }

    fn clear_cache(&self) -> PyResult<()> {
        catch_unwind_py(|| self.engine.clear_cache().map_err(engine_error_to_py))
    }
}
