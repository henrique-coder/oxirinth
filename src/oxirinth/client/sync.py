"""Synchronous Modrinth API v3 client powered by Rust and Pydantic v2."""

from __future__ import annotations

from typing import TYPE_CHECKING, Self, TypeVar

from oxirinth._core import CacheStats, SyncCore
from oxirinth._internal.adapters import (
    bind_client,
    bind_client_many,
    validate_json_batch,
    validate_json_list,
    validate_json_model,
    validate_search_result,
)
from oxirinth.client._shared import build_core_kwargs
from oxirinth.models.common import CategoryTag, GameVersionTag, LicenseTag, LoaderTag
from oxirinth.models.domains import DataPack, Mod, Modpack, Plugin, ResourcePack, Shader
from oxirinth.models.search import (
    BaseHit,
    DataPackHit,
    ModHit,
    ModpackHit,
    PluginHit,
    ResourcePackHit,
    SearchResult,
    ServerHit,
    ShaderHit,
)
from oxirinth.models.server import Server
from oxirinth.models.social import Organization, ProjectDependencies, TeamMember, User
from oxirinth.models.version import Version


if TYPE_CHECKING:
    from collections.abc import Sequence
    from types import TracebackType

    from oxirinth.config import CacheConfig, Environment, HashAlgorithm, SortIndex
    from oxirinth.models.domains import _BoundClientMixin


__all__: list[str] = ["Oxirinth"]

ModelT = TypeVar("ModelT", bound="_BoundClientMixin")
HitT = TypeVar("HitT", bound=BaseHit)


class Oxirinth:
    """Synchronous, strictly typed client for the Modrinth API v3.

    Uses an internal Rust HTTP/2 + ``redb`` + ``zstd`` engine while releasing the GIL
    during blocking I/O and returning immutable Pydantic v2 domain models.
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
        """Initialize the synchronous Modrinth client."""
        kwargs = build_core_kwargs(
            user_agent=user_agent,
            api_key=api_key,
            cache=cache,
            timeout=timeout,
            max_retries=max_retries,
            max_rate_limit_wait=max_rate_limit_wait,
            base_url=base_url,
        )
        self._core = SyncCore(**kwargs)

    def __enter__(self) -> Self:
        """Enter the synchronous context manager."""
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        """Exit the synchronous context manager and close resources."""
        self.close()

    def close(self) -> None:
        """Close the client session (no-op hook retained for parity with ``AsyncOxirinth``)."""

    # ------------------------------------------------------------------
    # Domain search
    # ------------------------------------------------------------------

    def search_mods(
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
    ) -> SearchResult[ModHit]:
        """Search Modrinth mods with automatic ``project_types:mod`` facet injection."""
        return self._search(
            "mod",
            ModHit,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    def search_plugins(
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
    ) -> SearchResult[PluginHit]:
        """Search Modrinth server plugins with automatic ``project_types:plugin`` facet injection."""
        return self._search(
            "plugin",
            PluginHit,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    def search_resourcepacks(
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
    ) -> SearchResult[ResourcePackHit]:
        """Search Modrinth resource packs with automatic ``project_types:resourcepack`` facet injection."""
        return self._search(
            "resourcepack",
            ResourcePackHit,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    def search_datapacks(
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
    ) -> SearchResult[DataPackHit]:
        """Search Modrinth data packs with automatic ``project_types:datapack`` facet injection."""
        return self._search(
            "datapack",
            DataPackHit,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    def search_shaders(
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
    ) -> SearchResult[ShaderHit]:
        """Search Modrinth shaders with automatic ``project_types:shader`` facet injection."""
        return self._search(
            "shader",
            ShaderHit,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    def search_modpacks(
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
    ) -> SearchResult[ModpackHit]:
        """Search Modrinth modpacks with automatic ``project_types:modpack`` facet injection."""
        return self._search(
            "modpack",
            ModpackHit,
            query,
            loaders=loaders,
            game_versions=game_versions,
            categories=categories,
            environment=environment,
            sort=sort,
            limit=limit,
            offset=offset,
        )

    def search_servers(
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
    ) -> SearchResult[ServerHit]:
        """Search Modrinth servers with automatic ``project_types:minecraft_java_server`` facet injection."""
        return self._search(
            "server",
            ServerHit,
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

    def get_mod(self, id_or_slug: str) -> Mod:
        """Fetch a single Modrinth mod by ID or slug."""
        return self._get_one("mod", Mod, id_or_slug)

    def get_mods(self, ids: Sequence[str]) -> list[Mod]:
        """Fetch multiple Modrinth mods using cache-aware partial batching."""
        return self._get_many("mod", Mod, ids)

    def get_plugin(self, id_or_slug: str) -> Plugin:
        """Fetch a single Modrinth plugin by ID or slug."""
        return self._get_one("plugin", Plugin, id_or_slug)

    def get_plugins(self, ids: Sequence[str]) -> list[Plugin]:
        """Fetch multiple Modrinth plugins using cache-aware partial batching."""
        return self._get_many("plugin", Plugin, ids)

    def get_resourcepack(self, id_or_slug: str) -> ResourcePack:
        """Fetch a single Modrinth resource pack by ID or slug."""
        return self._get_one("resourcepack", ResourcePack, id_or_slug)

    def get_resourcepacks(self, ids: Sequence[str]) -> list[ResourcePack]:
        """Fetch multiple Modrinth resource packs using cache-aware partial batching."""
        return self._get_many("resourcepack", ResourcePack, ids)

    def get_datapack(self, id_or_slug: str) -> DataPack:
        """Fetch a single Modrinth data pack by ID or slug."""
        return self._get_one("datapack", DataPack, id_or_slug)

    def get_datapacks(self, ids: Sequence[str]) -> list[DataPack]:
        """Fetch multiple Modrinth data packs using cache-aware partial batching."""
        return self._get_many("datapack", DataPack, ids)

    def get_shader(self, id_or_slug: str) -> Shader:
        """Fetch a single Modrinth shader by ID or slug."""
        return self._get_one("shader", Shader, id_or_slug)

    def get_shaders(self, ids: Sequence[str]) -> list[Shader]:
        """Fetch multiple Modrinth shaders using cache-aware partial batching."""
        return self._get_many("shader", Shader, ids)

    def get_modpack(self, id_or_slug: str) -> Modpack:
        """Fetch a single Modrinth modpack by ID or slug."""
        return self._get_one("modpack", Modpack, id_or_slug)

    def get_modpacks(self, ids: Sequence[str]) -> list[Modpack]:
        """Fetch multiple Modrinth modpacks using cache-aware partial batching."""
        return self._get_many("modpack", Modpack, ids)

    def get_server(self, id_or_slug: str) -> Server:
        """Fetch a single Modrinth server listing by ID or slug."""
        return self._get_one("server", Server, id_or_slug)

    def get_servers(self, ids: Sequence[str]) -> list[Server]:
        """Fetch multiple Modrinth servers using cache-aware partial batching."""
        return self._get_many("server", Server, ids)

    # ------------------------------------------------------------------
    # Versions & file hashes
    # ------------------------------------------------------------------

    def get_version(self, version_id: str) -> Version:
        """Fetch a single Modrinth version by version ID."""
        raw = self._core.get_version(version_id)
        return validate_json_model(Version, raw)

    def get_versions(self, version_ids: Sequence[str]) -> list[Version]:
        """Fetch multiple Modrinth versions using cache-aware partial batching."""
        raw_items = self._core.get_versions(version_ids)
        return validate_json_batch(Version, raw_items)

    def get_version_by_hash(self, hash_value: str, *, algorithm: HashAlgorithm = "sha512") -> Version:
        """Look up a version matching a file SHA-512 or SHA-1 hash."""
        raw = self._core.get_version_by_hash(hash_value, algorithm=algorithm)
        return validate_json_model(Version, raw)

    # ------------------------------------------------------------------
    # Social, organizations & tags
    # ------------------------------------------------------------------

    def get_user(self, id_or_username: str) -> User:
        """Fetch a Modrinth user profile by ID or username."""
        raw = self._core.get_user(id_or_username)
        return validate_json_model(User, raw)

    def get_organization(self, id_or_slug: str) -> Organization:
        """Fetch a Modrinth organization profile by ID or slug."""
        raw = self._core.get_organization(id_or_slug)
        return validate_json_model(Organization, raw)

    def get_loaders(self) -> list[LoaderTag]:
        """Fetch supported loader tags from ``/v3/tag/loader``."""
        raw = self._core.get_tag_loaders()
        return validate_json_list(LoaderTag, raw)

    def get_categories(self) -> list[CategoryTag]:
        """Fetch supported category tags from ``/v3/tag/category``."""
        raw = self._core.get_tag_categories()
        return validate_json_list(CategoryTag, raw)

    def get_game_versions(self) -> list[GameVersionTag]:
        """Fetch supported Minecraft versions from ``/v2/tag/game_version``."""
        raw = self._core.get_tag_game_versions()
        return validate_json_list(GameVersionTag, raw)

    def get_licenses(self) -> list[LicenseTag]:
        """Fetch supported SPDX license tags from ``/v2/tag/license``."""
        raw = self._core.get_tag_licenses()
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

    def _search(
        self,
        domain: str,
        hit_cls: type[HitT],
        query: str,
        *,
        loaders: Sequence[str] | None,
        game_versions: Sequence[str] | None,
        categories: Sequence[str] | None,
        environment: Environment | None,
        sort: SortIndex,
        limit: int,
        offset: int,
    ) -> SearchResult[HitT]:
        raw = self._core.search_domain(
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
        return validate_search_result(hit_cls, raw)

    def _get_one(self, domain: str, model_cls: type[ModelT], id_or_slug: str) -> ModelT:
        raw = self._core.get_domain(domain, id_or_slug)
        model = validate_json_model(model_cls, raw)
        return bind_client(model, self)

    def _get_many(self, domain: str, model_cls: type[ModelT], ids: Sequence[str]) -> list[ModelT]:
        raw_items = self._core.get_domains(domain, ids)
        models = validate_json_batch(model_cls, raw_items)
        return bind_client_many(models, self)

    def _get_project_versions(
        self,
        id_or_slug: str,
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        featured: bool | None = None,
    ) -> list[Version]:
        raw = self._core.get_project_versions(
            id_or_slug,
            loaders=loaders,
            game_versions=game_versions,
            featured=featured,
        )
        return validate_json_list(Version, raw)

    def _get_project_members(self, id_or_slug: str) -> list[TeamMember]:
        raw = self._core.get_project_members(id_or_slug)
        return validate_json_list(TeamMember, raw)

    def _get_project_dependencies(self, id_or_slug: str) -> ProjectDependencies:
        raw = self._core.get_project_dependencies(id_or_slug)
        return validate_json_model(ProjectDependencies, raw)
