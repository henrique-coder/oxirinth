"""Domain-specific search summary models and iterable pagination container."""

from __future__ import annotations

from collections.abc import Iterator
from datetime import datetime
from typing import TYPE_CHECKING, Generic, TypeVar, overload

from pydantic import AliasChoices, ConfigDict, Field
from pydantic.dataclasses import dataclass as pydantic_dataclass

from oxirinth.models._common import SearchDependency, _BoundClientMixin
from oxirinth.models._server import _ServerFieldsMixin


if TYPE_CHECKING:
    from oxirinth.models._domains import (
        AsyncDataPack,
        AsyncMod,
        AsyncModpack,
        AsyncPlugin,
        AsyncResourcePack,
        AsyncShader,
        DataPack,
        Mod,
        Modpack,
        Plugin,
        ResourcePack,
        Shader,
    )
    from oxirinth.models._server import AsyncServer, Server


__all__: list[str] = [
    "AsyncDataPackSummary",
    "AsyncModSummary",
    "AsyncModpackSummary",
    "AsyncPluginSummary",
    "AsyncResourcePackSummary",
    "AsyncServerSummary",
    "AsyncShaderSummary",
    "DataPackSummary",
    "ModSummary",
    "ModpackSummary",
    "PluginSummary",
    "ResourcePackSummary",
    "SearchPage",
    "ServerSummary",
    "ShaderSummary",
]


class _BaseSummary(_BoundClientMixin):
    """Internal base for project summaries returned by ``search_*``."""

    id: str = Field(validation_alias=AliasChoices("project_id", "id"))
    latest_version_id: str | None = Field(
        default=None,
        validation_alias=AliasChoices("version_id", "latest_version_id"),
    )
    slug: str
    name: str
    summary: str = ""
    author: str = ""
    author_id: str | None = None
    organization: str | None = None
    organization_id: str | None = None
    project_types: list[str] = Field(default_factory=list)
    categories: list[str] = Field(default_factory=list)
    display_categories: list[str] = Field(default_factory=list)
    followers: int = Field(default=0, validation_alias=AliasChoices("follows", "followers"))
    icon_url: str | None = None
    created_at: datetime | None = Field(default=None, validation_alias=AliasChoices("date_created", "created_at"))
    updated_at: datetime | None = Field(default=None, validation_alias=AliasChoices("date_modified", "updated_at"))
    license: str | None = None
    gallery: list[str] = Field(default_factory=list)
    featured_gallery: str | None = None
    color: int | None = None
    dependencies: list[SearchDependency] = Field(default_factory=list)


class _PackSummaryFields(_BaseSummary):
    """Shared version and download counts for non-server search summaries."""

    game_versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class _LoaderEnvSummaryFields(_PackSummaryFields):
    """Shared loaders and environments for mod and plugin search summaries."""

    loaders: list[str] = Field(default_factory=list)
    environment: list[str] = Field(default_factory=list)


class _BaseModSummary(_LoaderEnvSummaryFields):
    """Internal mod search summary schema."""


class ModSummary(_BaseModSummary):
    """Mod search summary returned by ``Oxirinth.search_mods``."""

    def fetch(self) -> Mod:
        """Fetch the full ``Mod`` record for this summary."""
        return self._require_client().get_mod(self.id)


class AsyncModSummary(_BaseModSummary):
    """Mod search summary returned by ``AsyncOxirinth.search_mods``."""

    async def fetch(self) -> AsyncMod:
        """Fetch the full ``AsyncMod`` record for this summary."""
        return await self._require_client().get_mod(self.id)


class _BasePluginSummary(_LoaderEnvSummaryFields):
    """Internal plugin search summary schema."""


class PluginSummary(_BasePluginSummary):
    """Plugin search summary returned by ``Oxirinth.search_plugins``."""

    def fetch(self) -> Plugin:
        """Fetch the full ``Plugin`` record for this summary."""
        return self._require_client().get_plugin(self.id)


class AsyncPluginSummary(_BasePluginSummary):
    """Plugin search summary returned by ``AsyncOxirinth.search_plugins``."""

    async def fetch(self) -> AsyncPlugin:
        """Fetch the full ``AsyncPlugin`` record for this summary."""
        return await self._require_client().get_plugin(self.id)


class _BaseResourcePackSummary(_PackSummaryFields):
    """Internal resource pack search summary schema."""


class ResourcePackSummary(_BaseResourcePackSummary):
    """Resource pack search summary returned by ``Oxirinth.search_resourcepacks``."""

    def fetch(self) -> ResourcePack:
        """Fetch the full ``ResourcePack`` record for this summary."""
        return self._require_client().get_resourcepack(self.id)


class AsyncResourcePackSummary(_BaseResourcePackSummary):
    """Resource pack search summary returned by ``AsyncOxirinth.search_resourcepacks``."""

    async def fetch(self) -> AsyncResourcePack:
        """Fetch the full ``AsyncResourcePack`` record for this summary."""
        return await self._require_client().get_resourcepack(self.id)


class _BaseDataPackSummary(_PackSummaryFields):
    """Internal data pack search summary schema."""


class DataPackSummary(_BaseDataPackSummary):
    """Data pack search summary returned by ``Oxirinth.search_datapacks``."""

    def fetch(self) -> DataPack:
        """Fetch the full ``DataPack`` record for this summary."""
        return self._require_client().get_datapack(self.id)


class AsyncDataPackSummary(_BaseDataPackSummary):
    """Data pack search summary returned by ``AsyncOxirinth.search_datapacks``."""

    async def fetch(self) -> AsyncDataPack:
        """Fetch the full ``AsyncDataPack`` record for this summary."""
        return await self._require_client().get_datapack(self.id)


class _BaseShaderSummary(_PackSummaryFields):
    """Internal shader search summary schema."""

    loaders: list[str] = Field(default_factory=list)


class ShaderSummary(_BaseShaderSummary):
    """Shader search summary returned by ``Oxirinth.search_shaders``."""

    def fetch(self) -> Shader:
        """Fetch the full ``Shader`` record for this summary."""
        return self._require_client().get_shader(self.id)


class AsyncShaderSummary(_BaseShaderSummary):
    """Shader search summary returned by ``AsyncOxirinth.search_shaders``."""

    async def fetch(self) -> AsyncShader:
        """Fetch the full ``AsyncShader`` record for this summary."""
        return await self._require_client().get_shader(self.id)


class _BaseModpackSummary(_PackSummaryFields):
    """Internal modpack search summary schema."""

    mod_loaders: list[str] = Field(default_factory=list, validation_alias=AliasChoices("mrpack_loaders", "mod_loaders"))
    environment: list[str] = Field(default_factory=list)


class ModpackSummary(_BaseModpackSummary):
    """Modpack search summary returned by ``Oxirinth.search_modpacks``."""

    def fetch(self) -> Modpack:
        """Fetch the full ``Modpack`` record for this summary."""
        return self._require_client().get_modpack(self.id)


class AsyncModpackSummary(_BaseModpackSummary):
    """Modpack search summary returned by ``AsyncOxirinth.search_modpacks``."""

    async def fetch(self) -> AsyncModpack:
        """Fetch the full ``AsyncModpack`` record for this summary."""
        return await self._require_client().get_modpack(self.id)


class _BaseServerSummary(_BaseSummary, _ServerFieldsMixin):
    """Internal server search summary schema."""


class ServerSummary(_BaseServerSummary):
    """Server search summary returned by ``Oxirinth.search_servers``."""

    def fetch(self) -> Server:
        """Fetch the full ``Server`` record for this summary."""
        return self._require_client().get_server(self.id)


class AsyncServerSummary(_BaseServerSummary):
    """Server search summary returned by ``AsyncOxirinth.search_servers``."""

    async def fetch(self) -> AsyncServer:
        """Fetch the full ``AsyncServer`` record for this summary."""
        return await self._require_client().get_server(self.id)


ItemT = TypeVar("ItemT", bound=_BaseSummary)


@pydantic_dataclass(frozen=True, config=ConfigDict(extra="ignore", populate_by_name=True))
class SearchPage(Generic[ItemT]):
    """Iterable paginated container returned by all ``search_*`` methods."""

    items: list[ItemT] = Field(default_factory=list, validation_alias=AliasChoices("hits", "items"))
    page: int = 1
    per_page: int = Field(default=0, validation_alias=AliasChoices("hits_per_page", "per_page"))
    total: int = Field(default=0, validation_alias=AliasChoices("total_hits", "total"))

    def __iter__(self) -> Iterator[ItemT]:
        """Iterate directly over items on this page."""
        return iter(self.items)

    def __len__(self) -> int:
        """Return the number of items on this page."""
        return len(self.items)

    def __bool__(self) -> bool:
        """Return ``True`` when the page contains at least one item."""
        return bool(self.items)

    @overload
    def __getitem__(self, index: int) -> ItemT: ...

    @overload
    def __getitem__(self, index: slice) -> list[ItemT]: ...

    def __getitem__(self, index: int | slice) -> ItemT | list[ItemT]:
        """Index or slice items on this page directly."""
        return self.items[index]

    @property
    def first(self) -> ItemT | None:
        """First result on this page, or ``None`` if empty."""
        return self.items[0] if self.items else None

    @property
    def has_next(self) -> bool:
        """Whether additional pages exist beyond this page."""
        return self.per_page > 0 and self.page * self.per_page < self.total
