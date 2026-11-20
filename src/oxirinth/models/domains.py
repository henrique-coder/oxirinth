"""Domain-specific Modrinth v3 project models with strict sync/async parity."""

from __future__ import annotations

from collections.abc import Sequence
from datetime import datetime
from typing import Any

from pydantic import Field, PrivateAttr

from oxirinth.exceptions import OxirinthUnboundModelError
from oxirinth.models.common import GalleryImage, License, LinkUrl, OxirinthModel
from oxirinth.models.social import ProjectDependencies, TeamMember
from oxirinth.models.version import Version


__all__: list[str] = [
    "AsyncDataPack",
    "AsyncMod",
    "AsyncModpack",
    "AsyncPlugin",
    "AsyncResourcePack",
    "AsyncShader",
    "BaseDataPack",
    "BaseDomainMeta",
    "BaseMod",
    "BaseModpack",
    "BasePlugin",
    "BaseResourcePack",
    "BaseShader",
    "DataPack",
    "Mod",
    "Modpack",
    "Plugin",
    "ResourcePack",
    "Shader",
]


class BaseDomainMeta(OxirinthModel):
    """Shared Modrinth v3 metadata common across all project domains."""

    id: str
    slug: str
    name: str
    summary: str = ""
    description: str = ""
    project_types: list[str] = Field(default_factory=list)
    games: list[str] = Field(default_factory=list)
    categories: list[str] = Field(default_factory=list)
    additional_categories: list[str] = Field(default_factory=list)
    followers: int = 0
    icon_url: str | None = None
    raw_icon_url: str | None = None
    color: int | None = None
    published: datetime | None = None
    updated: datetime | None = None
    approved: datetime | None = None
    status: str = ""
    license: License | None = None
    link_urls: dict[str, LinkUrl] = Field(default_factory=dict)
    gallery: list[GalleryImage] = Field(default_factory=list)
    team_id: str | None = None
    organization: str | None = None


class _BoundClientMixin(OxirinthModel):
    """Internal mixin holding a private reference to the originating client."""

    _client: Any = PrivateAttr(default=None)

    def _require_client(self) -> Any:
        if self._client is None:
            msg = (
                f"{type(self).__name__} is not bound to an active client; "
                "fetch it through Oxirinth / AsyncOxirinth to call helper methods."
            )
            raise OxirinthUnboundModelError(msg)
        return self._client


class _BoundDomainBase(BaseDomainMeta, _BoundClientMixin):
    """Base for bound domain models sharing core project metadata and client reference."""


class _SyncMemberMixin(_BoundDomainBase):
    """Synchronous team member helper exposed on bound project and server models."""

    def get_members(self) -> list[TeamMember]:
        """Fetch team members associated with this project."""
        return self._require_client()._get_project_members(self.id)


class _AsyncMemberMixin(_BoundDomainBase):
    """Asynchronous team member helper exposed on bound project and server models."""

    async def get_members(self) -> list[TeamMember]:
        """Fetch team members associated with this project."""
        return await self._require_client()._get_project_members(self.id)


class _SyncProjectActionsMixin(_SyncMemberMixin):
    """Synchronous lazy metadata helpers for downloadable Modrinth project domains."""

    def get_versions(
        self,
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        featured: bool | None = None,
    ) -> list[Version]:
        """Fetch release versions for this project with optional loader/game version filters."""
        return self._require_client()._get_project_versions(
            self.id,
            loaders=loaders,
            game_versions=game_versions,
            featured=featured,
        )

    def get_latest_version(
        self,
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        featured: bool | None = None,
    ) -> Version | None:
        """Return the newest matching version for this project, or ``None`` if none match."""
        versions = self.get_versions(loaders=loaders, game_versions=game_versions, featured=featured)
        return versions[0] if versions else None

    def get_dependencies(self) -> ProjectDependencies:
        """Fetch resolved project and version dependencies for this project."""
        return self._require_client()._get_project_dependencies(self.id)


class _AsyncProjectActionsMixin(_AsyncMemberMixin):
    """Asynchronous lazy metadata helpers for downloadable Modrinth project domains."""

    async def get_versions(
        self,
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        featured: bool | None = None,
    ) -> list[Version]:
        """Fetch release versions for this project with optional loader/game version filters."""
        return await self._require_client()._get_project_versions(
            self.id,
            loaders=loaders,
            game_versions=game_versions,
            featured=featured,
        )

    async def get_latest_version(
        self,
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        featured: bool | None = None,
    ) -> Version | None:
        """Return the newest matching version for this project, or ``None`` if none match."""
        versions = await self.get_versions(loaders=loaders, game_versions=game_versions, featured=featured)
        return versions[0] if versions else None

    async def get_dependencies(self) -> ProjectDependencies:
        """Fetch resolved project and version dependencies for this project."""
        return await self._require_client()._get_project_dependencies(self.id)


class BaseMod(BaseDomainMeta):
    """Pure-data Modrinth v3 mod schema."""

    loaders: list[str] = Field(default_factory=list)
    environment: list[str] = Field(default_factory=list)
    game_versions: list[str] = Field(default_factory=list)
    versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class Mod(BaseMod, _SyncProjectActionsMixin):
    """Modrinth mod model bound to a synchronous ``Oxirinth`` client."""


class AsyncMod(BaseMod, _AsyncProjectActionsMixin):
    """Modrinth mod model bound to an asynchronous ``AsyncOxirinth`` client."""


class BasePlugin(BaseDomainMeta):
    """Pure-data Modrinth v3 server plugin schema."""

    loaders: list[str] = Field(default_factory=list)
    environment: list[str] = Field(default_factory=list)
    game_versions: list[str] = Field(default_factory=list)
    versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class Plugin(BasePlugin, _SyncProjectActionsMixin):
    """Modrinth plugin model bound to a synchronous ``Oxirinth`` client."""


class AsyncPlugin(BasePlugin, _AsyncProjectActionsMixin):
    """Modrinth plugin model bound to an asynchronous ``AsyncOxirinth`` client."""


class BaseResourcePack(BaseDomainMeta):
    """Pure-data Modrinth v3 resource pack schema."""

    game_versions: list[str] = Field(default_factory=list)
    versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class ResourcePack(BaseResourcePack, _SyncProjectActionsMixin):
    """Modrinth resource pack model bound to a synchronous ``Oxirinth`` client."""


class AsyncResourcePack(BaseResourcePack, _AsyncProjectActionsMixin):
    """Modrinth resource pack model bound to an asynchronous ``AsyncOxirinth`` client."""


class BaseDataPack(BaseDomainMeta):
    """Pure-data Modrinth v3 data pack schema."""

    game_versions: list[str] = Field(default_factory=list)
    versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class DataPack(BaseDataPack, _SyncProjectActionsMixin):
    """Modrinth data pack model bound to a synchronous ``Oxirinth`` client."""


class AsyncDataPack(BaseDataPack, _AsyncProjectActionsMixin):
    """Modrinth data pack model bound to an asynchronous ``AsyncOxirinth`` client."""


class BaseShader(BaseDomainMeta):
    """Pure-data Modrinth v3 shader schema."""

    loaders: list[str] = Field(default_factory=list)
    game_versions: list[str] = Field(default_factory=list)
    versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class Shader(BaseShader, _SyncProjectActionsMixin):
    """Modrinth shader model bound to a synchronous ``Oxirinth`` client."""


class AsyncShader(BaseShader, _AsyncProjectActionsMixin):
    """Modrinth shader model bound to an asynchronous ``AsyncOxirinth`` client."""


class BaseModpack(BaseDomainMeta):
    """Pure-data Modrinth v3 modpack schema."""

    mod_loaders: list[str] = Field(default_factory=list, alias="mrpack_loaders")
    environment: list[str] = Field(default_factory=list)
    game_versions: list[str] = Field(default_factory=list)
    versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class Modpack(BaseModpack, _SyncProjectActionsMixin):
    """Modrinth modpack model bound to a synchronous ``Oxirinth`` client."""


class AsyncModpack(BaseModpack, _AsyncProjectActionsMixin):
    """Modrinth modpack model bound to an asynchronous ``AsyncOxirinth`` client."""
