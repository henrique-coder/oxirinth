"""Shared Pydantic v2 models for Modrinth metadata, links, licenses, and tags."""

from __future__ import annotations

from datetime import datetime

from pydantic import BaseModel, ConfigDict, Field

from oxirinth._core import CacheStats


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
    "OxirinthModel",
    "SearchDependency",
]


class OxirinthModel(BaseModel):
    """Base immutable Pydantic v2 model ignoring unrequested cross-domain fields."""

    model_config = ConfigDict(extra="ignore", frozen=True, populate_by_name=True)


class License(OxirinthModel):
    """SPDX or custom license metadata attached to a Modrinth project."""

    id: str
    name: str = ""
    url: str | None = None


class LinkUrl(OxirinthModel):
    """External or donation URL entry from a Modrinth v3 project's ``link_urls`` map."""

    platform: str
    donation: bool = False
    url: str


class GalleryImage(OxirinthModel):
    """Image entry from a Modrinth project gallery."""

    id: int | None = None
    url: str
    raw_url: str | None = None
    featured: bool = False
    name: str | None = None
    description: str | None = None
    created: datetime | None = None
    ordering: int = 0


class FileHashes(OxirinthModel):
    """Cryptographic digests for a Modrinth version file."""

    sha1: str | None = None
    sha512: str | None = None


class Dependency(OxirinthModel):
    """Version or project dependency entry on a Modrinth version."""

    version_id: str | None = None
    project_id: str | None = None
    file_name: str | None = None
    dependency_type: str


class SearchDependency(OxirinthModel):
    """Inline project dependency summary returned inside ``/v3/search`` hits."""

    project_id: str
    dependency_type: str
    name: str | None = None
    slug: str | None = None
    icon_url: str | None = None


class LoaderTag(OxirinthModel):
    """Mod, plugin, shader, or modpack loader metadata from ``/v3/tag/loader``."""

    icon: str = ""
    name: str
    supported_project_types: list[str] = Field(default_factory=list)
    supported_games: list[str] = Field(default_factory=list)
    supported_fields: list[str] = Field(default_factory=list)


class CategoryTag(OxirinthModel):
    """Category tag metadata from ``/v3/tag/category``."""

    icon: str = ""
    name: str
    project_type: str
    header: str = ""


class GameVersionTag(OxirinthModel):
    """Game version tag metadata from ``/v2/tag/game_version``."""

    version: str
    version_type: str
    date: datetime
    major: bool = False


class LicenseTag(OxirinthModel):
    """SPDX license tag metadata from ``/v2/tag/license``."""

    short: str
    name: str
