"""Asynchronous Modrinth API v3 client."""

from __future__ import annotations

from typing import TYPE_CHECKING, Self, TypeVar

from oxirinth._client._shared import build_core_kwargs
from oxirinth._core import AsyncCore, CacheStats
from oxirinth._internal.adapters import (
    bind_client,
    bind_client_many,
    validate_json_batch,
    validate_json_list,
    validate_json_model,
    validate_search_page,
)
from oxirinth.models._common import CategoryTag, GameVersionTag, LicenseTag, LoaderTag
from oxirinth.models._domains import (
    AsyncDataPack,
    AsyncMod,
    AsyncModpack,
    AsyncPlugin,
    AsyncResourcePack,
    AsyncShader,
)
from oxirinth.models._search import (
    AsyncDataPackSummary,
    AsyncModpackSummary,
    AsyncModSummary,
    AsyncPluginSummary,
    AsyncResourcePackSummary,
    AsyncServerSummary,
    AsyncShaderSummary,
    SearchPage,
    _BaseSummary,
)
from oxirinth.models._server import AsyncServer
from oxirinth.models._social import Organization, ProjectDependencies, TeamMember, User
from oxirinth.models._version import Version


if TYPE_CHECKING:
    from collections.abc import Sequence
    from types import TracebackType

    from oxirinth._config import CacheConfig, Environment, HashAlgorithm, SortIndex
    from oxirinth.models._common import _BoundClientMixin


__all__: list[str] = ["AsyncOxirinth"]

ModelT = TypeVar("ModelT", bound="_BoundClientMixin")
SummaryT = TypeVar("SummaryT", bound="_BaseSummary")


class AsyncOxirinth:
    """Asynchronous client for the Modrinth API v3.

    Bridges Rust Tokio futures to ``asyncio`` via ``pyo3-async-runtimes`` and
    returns immutable Pydantic v2 models.
    """

    def __init__(
        self,
        *,
        user_agent: str | None = None,
        api_key: str | None = None,
        cache: CacheConfig | bool = True,
        timeout: float = 30.0,
        max_retries: int = 3,
        max_rate_limit_wait: float | None = None,
        base_url: str = "https://api.modrinth.com",
    ) -> None:
        """Initialize the asynchronous Modrinth client."""
        kwargs = build_core_kwargs(
            user_agent=user_agent,
            api_key=api_key,
            cache=cache,
            timeout=timeout,
            max_retries=max_retries,
            max_rate_limit_wait=max_rate_limit_wait,
            base_url=base_url,
        )
        self._core = AsyncCore(**kwargs)

    async def __aenter__(self) -> Self:
        """Enter the asynchronous context manager."""
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        """Exit the asynchronous context manager and close resources."""
        await self.aclose()

    async def aclose(self) -> None:
        """Close the async client session (retained for parity with ``Oxirinth``)."""

    # ------------------------------------------------------------------
    # Domain search
    # ------------------------------------------------------------------

    async def search_mods(
        self,
        query: str = "",
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        categories: Sequence[str] | None = None,
        environment: Environment | None = None,
        sort: SortIndex = "relevance",
        limit: int = 10,
        offset: int = 0,
    ) -> SearchPage[AsyncModSummary]:
        """Search Modrinth mods."""
        return await self._search(
            "mod",
            AsyncModSummary,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    async def search_plugins(
        self,
        query: str = "",
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        categories: Sequence[str] | None = None,
        environment: Environment | None = None,
        sort: SortIndex = "relevance",
        limit: int = 10,
        offset: int = 0,
    ) -> SearchPage[AsyncPluginSummary]:
        """Search Modrinth server plugins."""
        return await self._search(
            "plugin",
            AsyncPluginSummary,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    async def search_resourcepacks(
        self,
        query: str = "",
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        categories: Sequence[str] | None = None,
        environment: Environment | None = None,
        sort: SortIndex = "relevance",
        limit: int = 10,
        offset: int = 0,
    ) -> SearchPage[AsyncResourcePackSummary]:
        """Search Modrinth resource packs."""
        return await self._search(
            "resourcepack",
            AsyncResourcePackSummary,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    async def search_datapacks(
        self,
        query: str = "",
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        categories: Sequence[str] | None = None,
        environment: Environment | None = None,
        sort: SortIndex = "relevance",
        limit: int = 10,
        offset: int = 0,
    ) -> SearchPage[AsyncDataPackSummary]:
        """Search Modrinth data packs."""
        return await self._search(
            "datapack",
            AsyncDataPackSummary,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    async def search_shaders(
        self,
        query: str = "",
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        categories: Sequence[str] | None = None,
        environment: Environment | None = None,
        sort: SortIndex = "relevance",
        limit: int = 10,
        offset: int = 0,
    ) -> SearchPage[AsyncShaderSummary]:
        """Search Modrinth shaders."""
        return await self._search(
            "shader",
            AsyncShaderSummary,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    async def search_modpacks(
        self,
        query: str = "",
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        categories: Sequence[str] | None = None,
        environment: Environment | None = None,
        sort: SortIndex = "relevance",
        limit: int = 10,
        offset: int = 0,
    ) -> SearchPage[AsyncModpackSummary]:
        """Search Modrinth modpacks."""
        return await self._search(
            "modpack",
            AsyncModpackSummary,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    async def search_servers(
        self,
        query: str = "",
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        categories: Sequence[str] | None = None,
        environment: Environment | None = None,
        sort: SortIndex = "relevance",
        limit: int = 10,
        offset: int = 0,
    ) -> SearchPage[AsyncServerSummary]:
        """Search Modrinth Minecraft servers."""
        return await self._search(
            "server",
            AsyncServerSummary,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    # ------------------------------------------------------------------
    # Domain single & batch retrieval
    # ------------------------------------------------------------------

    async def get_mod(self, id_or_slug: str) -> AsyncMod:
        """Fetch a single Modrinth mod by ID or slug."""
        return await self._get_one("mod", AsyncMod, id_or_slug)

    async def get_mods(self, ids: Sequence[str]) -> list[AsyncMod]:
        """Fetch multiple Modrinth mods using cache-aware partial batching."""
        return await self._get_many("mod", AsyncMod, ids)

    async def get_plugin(self, id_or_slug: str) -> AsyncPlugin:
        """Fetch a single Modrinth plugin by ID or slug."""
        return await self._get_one("plugin", AsyncPlugin, id_or_slug)

    async def get_plugins(self, ids: Sequence[str]) -> list[AsyncPlugin]:
        """Fetch multiple Modrinth plugins using cache-aware partial batching."""
        return await self._get_many("plugin", AsyncPlugin, ids)

    async def get_resourcepack(self, id_or_slug: str) -> AsyncResourcePack:
        """Fetch a single Modrinth resource pack by ID or slug."""
        return await self._get_one("resourcepack", AsyncResourcePack, id_or_slug)

    async def get_resourcepacks(self, ids: Sequence[str]) -> list[AsyncResourcePack]:
        """Fetch multiple Modrinth resource packs using cache-aware partial batching."""
        return await self._get_many("resourcepack", AsyncResourcePack, ids)

    async def get_datapack(self, id_or_slug: str) -> AsyncDataPack:
        """Fetch a single Modrinth data pack by ID or slug."""
        return await self._get_one("datapack", AsyncDataPack, id_or_slug)

    async def get_datapacks(self, ids: Sequence[str]) -> list[AsyncDataPack]:
        """Fetch multiple Modrinth data packs using cache-aware partial batching."""
        return await self._get_many("datapack", AsyncDataPack, ids)

    async def get_shader(self, id_or_slug: str) -> AsyncShader:
        """Fetch a single Modrinth shader by ID or slug."""
        return await self._get_one("shader", AsyncShader, id_or_slug)

    async def get_shaders(self, ids: Sequence[str]) -> list[AsyncShader]:
        """Fetch multiple Modrinth shaders using cache-aware partial batching."""
        return await self._get_many("shader", AsyncShader, ids)

    async def get_modpack(self, id_or_slug: str) -> AsyncModpack:
        """Fetch a single Modrinth modpack by ID or slug."""
        return await self._get_one("modpack", AsyncModpack, id_or_slug)

    async def get_modpacks(self, ids: Sequence[str]) -> list[AsyncModpack]:
        """Fetch multiple Modrinth modpacks using cache-aware partial batching."""
        return await self._get_many("modpack", AsyncModpack, ids)

    async def get_server(self, id_or_slug: str) -> AsyncServer:
        """Fetch a single Modrinth server listing by ID or slug."""
        return await self._get_one("server", AsyncServer, id_or_slug)

    async def get_servers(self, ids: Sequence[str]) -> list[AsyncServer]:
        """Fetch multiple Modrinth servers using cache-aware partial batching."""
        return await self._get_many("server", AsyncServer, ids)

    # ------------------------------------------------------------------
    # Versions & file hashes
    # ------------------------------------------------------------------

    async def get_version(self, version_id: str) -> Version:
        """Fetch a single Modrinth version by version ID."""
        raw = await self._core.get_version(version_id)
        return validate_json_model(Version, raw)

    async def get_versions(self, version_ids: Sequence[str]) -> list[Version]:
        """Fetch multiple Modrinth versions using cache-aware partial batching."""
        raw_items = await self._core.get_versions(version_ids)
        return validate_json_batch(Version, raw_items)

    async def get_version_by_hash(self, hash_value: str, *, algorithm: HashAlgorithm = "sha512") -> Version:
        """Look up a version matching a file SHA-512 or SHA-1 hash."""
        raw = await self._core.get_version_by_hash(hash_value, algorithm=algorithm)
        return validate_json_model(Version, raw)

    # ------------------------------------------------------------------
    # Social, organizations & tags
    # ------------------------------------------------------------------

    async def get_user(self, id_or_username: str) -> User:
        """Fetch a Modrinth user profile by ID or username."""
        raw = await self._core.get_user(id_or_username)
        return validate_json_model(User, raw)

    async def get_organization(self, id_or_slug: str) -> Organization:
        """Fetch a Modrinth organization profile by ID or slug."""
        raw = await self._core.get_organization(id_or_slug)
        return validate_json_model(Organization, raw)

    async def get_loaders(self) -> list[LoaderTag]:
        """Fetch supported loader tags from ``/v3/tag/loader``."""
        raw = await self._core.get_tag_loaders()
        return validate_json_list(LoaderTag, raw)

    async def get_categories(self) -> list[CategoryTag]:
        """Fetch supported category tags from ``/v3/tag/category``."""
        raw = await self._core.get_tag_categories()
        return validate_json_list(CategoryTag, raw)

    async def get_game_versions(self) -> list[GameVersionTag]:
        """Fetch supported Minecraft versions from ``/v3/loader_field?loader_field=game_versions``."""
        raw = await self._core.get_tag_game_versions()
        return validate_json_list(GameVersionTag, raw)

    async def get_licenses(self) -> list[LicenseTag]:
        """Fetch supported SPDX license tags from ``/v3/license``."""
        raw = await self._core.get_tag_licenses()
        return validate_json_list(LicenseTag, raw)

    # ------------------------------------------------------------------
    # Cache management
    # ------------------------------------------------------------------

    def cache_stats(self) -> CacheStats:
        """Return live cache statistics and hit/miss counters."""
        return self._core.cache_stats()

    def clear_cache(self) -> None:
        """Remove all cached responses from the active backend."""
        self._core.clear_cache()

    # ------------------------------------------------------------------
    # Internal helpers (shared with bound domain models)
    # ------------------------------------------------------------------

    async def _search(
        self,
        domain: str,
        summary_cls: type[SummaryT],
        query: str,
        *,
        loaders: Sequence[str] | None,
        game_versions: Sequence[str] | None,
        categories: Sequence[str] | None,
        environment: Environment | None,
        sort: SortIndex,
        limit: int,
        offset: int,
    ) -> SearchPage[SummaryT]:
        raw = await self._core.search_domain(
            domain,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )
        page = validate_search_page(summary_cls, raw)
        bind_client_many(page.items, self)
        return page

    async def _get_one(self, domain: str, model_cls: type[ModelT], id_or_slug: str) -> ModelT:
        raw = await self._core.get_domain(domain, id_or_slug)
        model = validate_json_model(model_cls, raw)
        return bind_client(model, self)

    async def _get_many(self, domain: str, model_cls: type[ModelT], ids: Sequence[str]) -> list[ModelT]:
        raw_items = await self._core.get_domains(domain, ids)
        models = validate_json_batch(model_cls, raw_items)
        return bind_client_many(models, self)

    async def _get_project_versions(
        self,
        id_or_slug: str,
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        featured: bool | None = None,
    ) -> list[Version]:
        raw = await self._core.get_project_versions(
            id_or_slug,
            loaders=loaders,
            game_versions=game_versions,
            featured=featured,
        )
        return validate_json_list(Version, raw)

    async def _get_project_members(self, id_or_slug: str) -> list[TeamMember]:
        raw = await self._core.get_project_members(id_or_slug)
        return validate_json_list(TeamMember, raw)

    async def _get_project_dependencies(self, id_or_slug: str) -> ProjectDependencies:
        raw = await self._core.get_project_dependencies(id_or_slug)
        return validate_json_model(ProjectDependencies, raw)
