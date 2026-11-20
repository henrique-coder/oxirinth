"""Internal Rust extension module powered by oxirinth-engine."""

from collections.abc import Sequence
from pathlib import Path
from typing import final

__all__ = [
    "DEFAULT_USER_AGENT",
    "AsyncCore",
    "CacheStats",
    "OxirinthAPIError",
    "OxirinthAuthError",
    "OxirinthCacheError",
    "OxirinthConfigError",
    "OxirinthError",
    "OxirinthInternalError",
    "OxirinthNetworkError",
    "OxirinthNotFoundError",
    "OxirinthRateLimitError",
    "OxirinthTypeMismatchError",
    "OxirinthUnboundModelError",
    "SyncCore",
    "__version__",
    "engine_version",
]

__version__: str
DEFAULT_USER_AGENT: str

def engine_version() -> str:
    """Return the compiled oxirinth-engine semantic version."""

class OxirinthError(Exception):
    """Base exception for all Oxirinth SDK errors."""

class OxirinthConfigError(OxirinthError):
    """Invalid client, cache, or query configuration."""

class OxirinthNetworkError(OxirinthError):
    """HTTP transport, TLS, or timeout error."""

class OxirinthAPIError(OxirinthError):
    """Non-success HTTP response returned by Modrinth API."""

    status_code: int
    error_code: str | None
    description: str
    url: str

class OxirinthNotFoundError(OxirinthAPIError):
    """Requested Modrinth resource was not found (HTTP 404)."""

class OxirinthAuthError(OxirinthAPIError):
    """Authentication or authorization failure (HTTP 401/403)."""

class OxirinthRateLimitError(OxirinthAPIError):
    """Modrinth API rate limit exceeded and wait limit exhausted (HTTP 429)."""

    retry_after_secs: int

class OxirinthTypeMismatchError(OxirinthError):
    """Retrieved project does not belong to the requested Modrinth domain."""

    expected: str
    actual: tuple[str, ...]
    identifier: str

class OxirinthCacheError(OxirinthError):
    """Local redb/zstd or memory cache error."""

class OxirinthUnboundModelError(OxirinthError):
    """Helper method requiring a client was invoked on an unbound model."""

class OxirinthInternalError(OxirinthError):
    """Unexpected internal Rust panic or invariant violation."""

@final
class CacheStats:
    """Runtime cache usage statistics."""

    @property
    def backend(self) -> str: ...
    @property
    def entries(self) -> int: ...
    @property
    def hits(self) -> int: ...
    @property
    def misses(self) -> int: ...
    @property
    def revalidations(self) -> int: ...
    @property
    def size_bytes(self) -> int: ...

@final
class SyncCore:
    """Synchronous GIL-releasing FFI bridge to oxirinth-engine."""

    def __new__(
        cls,
        *,
        user_agent: str | None = None,
        api_key: str | None = None,
        cache_enabled: bool = True,
        cache_dir: str | Path | None = None,
        ram_fallback_max_bytes: int = 67_108_864,
        zstd_min_bytes: int = 512,
        zstd_level: int = 3,
        search_ttl_secs: int = 120,
        project_ttl_secs: int = 900,
        version_list_ttl_secs: int = 300,
        version_ttl_secs: int = 86_400,
        tag_ttl_secs: int = 21_600,
        timeout_secs: float = 30.0,
        max_retries: int = 3,
        max_rate_limit_wait_secs: float | None = None,
        rate_limit_safety_margin: int = 2,
        base_url: str | None = None,
    ) -> SyncCore: ...
    def search_domain(
        self,
        domain: str,
        query: str = "",
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        categories: Sequence[str] | None = None,
        environment: str | None = None,
        sort: str = "relevance",
        limit: int = 10,
        offset: int = 0,
    ) -> bytes: ...
    def get_domain(self, domain: str, id_or_slug: str) -> bytes: ...
    def get_domains(self, domain: str, ids: Sequence[str]) -> list[bytes]: ...
    def get_project_versions(
        self,
        id_or_slug: str,
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        featured: bool | None = None,
    ) -> bytes: ...
    def get_version(self, version_id: str) -> bytes: ...
    def get_versions(self, version_ids: Sequence[str]) -> list[bytes]: ...
    def get_version_by_hash(self, hash_value: str, *, algorithm: str = "sha512") -> bytes: ...
    def get_project_dependencies(self, id_or_slug: str) -> bytes: ...
    def get_project_members(self, id_or_slug: str) -> bytes: ...
    def get_team_members(self, team_id: str) -> bytes: ...
    def get_user(self, id_or_username: str) -> bytes: ...
    def get_user_projects(self, id_or_username: str) -> bytes: ...
    def get_organization(self, id_or_slug: str) -> bytes: ...
    def get_organization_projects(self, id_or_slug: str) -> bytes: ...
    def get_tag_loaders(self) -> bytes: ...
    def get_tag_categories(self) -> bytes: ...
    def get_tag_game_versions(self) -> bytes: ...
    def get_tag_licenses(self) -> bytes: ...
    def get_tag_project_types(self) -> bytes: ...
    def cache_stats(self) -> CacheStats: ...
    def clear_cache(self) -> None: ...

@final
class AsyncCore:
    """Asynchronous asyncio-bridged FFI wrapper over oxirinth-engine."""

    def __new__(
        cls,
        *,
        user_agent: str | None = None,
        api_key: str | None = None,
        cache_enabled: bool = True,
        cache_dir: str | Path | None = None,
        ram_fallback_max_bytes: int = 67_108_864,
        zstd_min_bytes: int = 512,
        zstd_level: int = 3,
        search_ttl_secs: int = 120,
        project_ttl_secs: int = 900,
        version_list_ttl_secs: int = 300,
        version_ttl_secs: int = 86_400,
        tag_ttl_secs: int = 21_600,
        timeout_secs: float = 30.0,
        max_retries: int = 3,
        max_rate_limit_wait_secs: float | None = None,
        rate_limit_safety_margin: int = 2,
        base_url: str | None = None,
    ) -> AsyncCore: ...
    async def search_domain(
        self,
        domain: str,
        query: str = "",
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        categories: Sequence[str] | None = None,
        environment: str | None = None,
        sort: str = "relevance",
        limit: int = 10,
        offset: int = 0,
    ) -> bytes: ...
    async def get_domain(self, domain: str, id_or_slug: str) -> bytes: ...
    async def get_domains(self, domain: str, ids: Sequence[str]) -> list[bytes]: ...
    async def get_project_versions(
        self,
        id_or_slug: str,
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        featured: bool | None = None,
    ) -> bytes: ...
    async def get_version(self, version_id: str) -> bytes: ...
    async def get_versions(self, version_ids: Sequence[str]) -> list[bytes]: ...
    async def get_version_by_hash(self, hash_value: str, *, algorithm: str = "sha512") -> bytes: ...
    async def get_project_dependencies(self, id_or_slug: str) -> bytes: ...
    async def get_project_members(self, id_or_slug: str) -> bytes: ...
    async def get_team_members(self, team_id: str) -> bytes: ...
    async def get_user(self, id_or_username: str) -> bytes: ...
    async def get_user_projects(self, id_or_username: str) -> bytes: ...
    async def get_organization(self, id_or_slug: str) -> bytes: ...
    async def get_organization_projects(self, id_or_slug: str) -> bytes: ...
    async def get_tag_loaders(self) -> bytes: ...
    async def get_tag_categories(self) -> bytes: ...
    async def get_tag_game_versions(self) -> bytes: ...
    async def get_tag_licenses(self) -> bytes: ...
    async def get_tag_project_types(self) -> bytes: ...
    def cache_stats(self) -> CacheStats: ...
    def clear_cache(self) -> None: ...
