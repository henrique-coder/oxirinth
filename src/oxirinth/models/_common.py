"""Shared internal base classes and public metadata/tag models."""

from __future__ import annotations

from datetime import datetime
from typing import Any

from pydantic import AliasChoices, BaseModel, ConfigDict, Field, PrivateAttr

from oxirinth._core import CacheStats
from oxirinth.exceptions import OxirinthUnboundModelError


__all__: list[str] = [
    "CacheStats",
    "CategoryTag",
    "Dependency",
    "FileHashes",
    "GalleryImage",
    "GameVersionTag",
    "License",
    "LicenseTag",
    "LinkUrl",
    "LoaderTag",
    "SearchDependency",
]


class _OxirinthModel(BaseModel):
    """Base immutable model ignoring unrequested Modrinth payload fields."""

    model_config = ConfigDict(extra="ignore", frozen=True, populate_by_name=True)


class _BoundClientMixin(_OxirinthModel):
    """Internal mixin storing the originating client on bound models."""

    _client: Any = PrivateAttr(default=None)

    def _require_client(self) -> Any:
        if self._client is None:
            msg = (
                f"{type(self).__name__} is not bound to a client; "
                "retrieve it from Oxirinth or AsyncOxirinth to call remote helper methods."
            )
            raise OxirinthUnboundModelError(msg)
        return self._client


class License(_OxirinthModel):
    """SPDX or custom license metadata attached to a project."""

    id: str
    name: str = ""
    url: str | None = None


class LinkUrl(_OxirinthModel):
    """External or donation URL entry on a project."""

    platform: str
    donation: bool = False
    url: str


class GalleryImage(_OxirinthModel):
    """Image entry from a project gallery."""

    id: int | None = None
    url: str
    raw_url: str | None = None
    featured: bool = False
    name: str | None = None
    description: str | None = None
    created_at: datetime | None = Field(default=None, validation_alias=AliasChoices("created", "created_at"))
    ordering: int = 0


class FileHashes(_OxirinthModel):
    """Cryptographic digests for a version file."""

    sha1: str | None = None
    sha512: str | None = None


class Dependency(_OxirinthModel):
    """Version or project dependency declared on a release."""

    version_id: str | None = None
    project_id: str | None = None
    file_name: str | None = None
    dependency_type: str


class SearchDependency(_OxirinthModel):
    """Inline project dependency summary returned in search results."""

    project_id: str
    dependency_type: str
    name: str | None = None
    slug: str | None = None
    icon_url: str | None = None


class LoaderTag(_OxirinthModel):
    """Loader tag metadata returned by ``GET /v3/tag/loader``."""

    icon: str = ""
    name: str
    supported_project_types: list[str] = Field(default_factory=list)
    supported_games: list[str] = Field(default_factory=list)
    supported_fields: list[str] = Field(default_factory=list)


class CategoryTag(_OxirinthModel):
    """Category tag metadata returned by ``GET /v3/tag/category``."""

    icon: str = ""
    name: str
    project_type: str
    header: str = ""


class GameVersionTag(_OxirinthModel):
    """Minecraft game version metadata from ``GET /v3/loader_field?loader_field=game_versions``."""

    version: str = Field(validation_alias=AliasChoices("value", "version"))
    version_type: str = Field(validation_alias=AliasChoices("type", "version_type"))
    released_at: datetime = Field(validation_alias=AliasChoices("created", "date", "released_at"))
    major: bool = False


class LicenseTag(_OxirinthModel):
    """SPDX license tag metadata returned by ``GET /v3/license``."""

    short: str
    name: str
