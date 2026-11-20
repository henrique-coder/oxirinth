"""Domain-specific Modrinth project models with sync and async helpers."""

from __future__ import annotations

from collections.abc import Sequence
from datetime import datetime

from pydantic import AliasChoices, Field

from oxirinth.models._common import GalleryImage, License, LinkUrl, _BoundClientMixin, _OxirinthModel
from oxirinth.models._social import ProjectDependencies, TeamMember
from oxirinth.models._version import Version


__all__: list[str] = [
    "AsyncDataPack",
    "AsyncMod",
    "AsyncModpack",
    "AsyncPlugin",
    "AsyncResourcePack",
    "AsyncShader",
    "DataPack",
    "Mod",
    "Modpack",
    "Plugin",
    "ResourcePack",
    "Shader",
]


class _DomainMeta(_OxirinthModel):
    """Shared metadata fields common across all Modrinth project domains."""

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
    published_at: datetime | None = Field(default=None, validation_alias=AliasChoices("published", "published_at"))
    updated_at: datetime | None = Field(default=None, validation_alias=AliasChoices("updated", "updated_at"))
    approved_at: datetime | None = Field(default=None, validation_alias=AliasChoices("approved", "approved_at"))
    status: str = ""
    license: License | None = None
    link_urls: dict[str, LinkUrl] = Field(default_factory=dict)
    gallery: list[GalleryImage] = Field(default_factory=list)
    team_id: str | None = None
    organization: str | None = None


class _BoundDomainBase(_DomainMeta, _BoundClientMixin):
    """Base for bound domain models sharing core metadata and client binding."""


class _SyncMemberMixin(_BoundDomainBase):
    """Synchronous team member lookup helper."""

    def get_members(self) -> list[TeamMember]:
        """Fetch team members for this project."""
        return self._require_client()._get_project_members(self.id)


class _AsyncMemberMixin(_BoundDomainBase):
    """Asynchronous team member lookup helper."""

    async def get_members(self) -> list[TeamMember]:
        """Fetch team members for this project."""
        return await self._require_client()._get_project_members(self.id)


class _SyncProjectActionsMixin(_SyncMemberMixin):
    """Synchronous lazy metadata helpers for release-bearing project domains."""

    def get_versions(
        self,
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        featured: bool | None = None,
    ) -> list[Version]:
        """Fetch release versions for this project."""
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
        """Return the newest matching version for this project, or ``None``."""
        versions = self.get_versions(loaders=loaders, game_versions=game_versions, featured=featured)
        return versions[0] if versions else None

    def get_dependencies(self) -> ProjectDependencies:
        """Fetch resolved project and version dependencies."""
        return self._require_client()._get_project_dependencies(self.id)


class _AsyncProjectActionsMixin(_AsyncMemberMixin):
    """Asynchronous lazy metadata helpers for release-bearing project domains."""

    async def get_versions(
        self,
        *,
        loaders: Sequence[str] | None = None,
        game_versions: Sequence[str] | None = None,
        featured: bool | None = None,
    ) -> list[Version]:
        """Fetch release versions for this project."""
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
        """Return the newest matching version for this project, or ``None``."""
        versions = await self.get_versions(loaders=loaders, game_versions=game_versions, featured=featured)
        return versions[0] if versions else None

    async def get_dependencies(self) -> ProjectDependencies:
        """Fetch resolved project and version dependencies."""
        return await self._require_client()._get_project_dependencies(self.id)


class _PackFieldsMixin(_DomainMeta):
    """Shared release stats and game versions for pack-type domains."""

    game_versions: list[str] = Field(default_factory=list)
    versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class _LoaderEnvFieldsMixin(_PackFieldsMixin):
    """Shared loader and environment fields for mod and plugin domains."""

    loaders: list[str] = Field(default_factory=list)
    environment: list[str] = Field(default_factory=list)


class _BaseMod(_LoaderEnvFieldsMixin):
    """Internal mod schema."""


class Mod(_BaseMod, _SyncProjectActionsMixin):
    """Modrinth mod bound to a synchronous ``Oxirinth`` client."""


class AsyncMod(_BaseMod, _AsyncProjectActionsMixin):
    """Modrinth mod bound to an asynchronous ``AsyncOxirinth`` client."""


class _BasePlugin(_LoaderEnvFieldsMixin):
    """Internal plugin schema."""


class Plugin(_BasePlugin, _SyncProjectActionsMixin):
    """Modrinth server plugin bound to a synchronous ``Oxirinth`` client."""


class AsyncPlugin(_BasePlugin, _AsyncProjectActionsMixin):
    """Modrinth server plugin bound to an asynchronous ``AsyncOxirinth`` client."""


class _BaseResourcePack(_PackFieldsMixin):
    """Internal resource pack schema."""


class ResourcePack(_BaseResourcePack, _SyncProjectActionsMixin):
    """Modrinth resource pack bound to a synchronous ``Oxirinth`` client."""


class AsyncResourcePack(_BaseResourcePack, _AsyncProjectActionsMixin):
    """Modrinth resource pack bound to an asynchronous ``AsyncOxirinth`` client."""


class _BaseDataPack(_PackFieldsMixin):
    """Internal data pack schema."""


class DataPack(_BaseDataPack, _SyncProjectActionsMixin):
    """Modrinth data pack bound to a synchronous ``Oxirinth`` client."""


class AsyncDataPack(_BaseDataPack, _AsyncProjectActionsMixin):
    """Modrinth data pack bound to an asynchronous ``AsyncOxirinth`` client."""


class _BaseShader(_PackFieldsMixin):
    """Internal shader schema."""

    loaders: list[str] = Field(default_factory=list)


class Shader(_BaseShader, _SyncProjectActionsMixin):
    """Modrinth shader bound to a synchronous ``Oxirinth`` client."""


class AsyncShader(_BaseShader, _AsyncProjectActionsMixin):
    """Modrinth shader bound to an asynchronous ``AsyncOxirinth`` client."""


class _BaseModpack(_PackFieldsMixin):
    """Internal modpack schema."""

    mod_loaders: list[str] = Field(default_factory=list, validation_alias=AliasChoices("mrpack_loaders", "mod_loaders"))
    environment: list[str] = Field(default_factory=list)


class Modpack(_BaseModpack, _SyncProjectActionsMixin):
    """Modrinth modpack bound to a synchronous ``Oxirinth`` client."""


class AsyncModpack(_BaseModpack, _AsyncProjectActionsMixin):
    """Modrinth modpack bound to an asynchronous ``AsyncOxirinth`` client."""
