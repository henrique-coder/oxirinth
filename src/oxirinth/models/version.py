"""Modrinth v3 version and file metadata models."""

from __future__ import annotations

from datetime import datetime

from pydantic import Field

from oxirinth.models.common import Dependency, FileHashes, OxirinthModel


__all__: list[str] = ["Version", "VersionFile"]


class VersionFile(OxirinthModel):
    """Metadata and hashes for an individual file attached to a Modrinth version.

    Binary downloads are intentionally out of scope for Oxirinth; inspect
    ``url`` and ``hashes`` to integrate your preferred HTTP downloader.
    """

    id: str | None = None
    hashes: FileHashes
    url: str
    filename: str
    primary: bool = False
    size: int
    file_type: str | None = None


class Version(OxirinthModel):
    """Modrinth v3 project version release metadata."""

    id: str
    project_id: str
    author_id: str | None = None
    featured: bool = False
    name: str
    version_number: str
    project_types: list[str] = Field(default_factory=list)
    games: list[str] = Field(default_factory=list)
    changelog: str | None = None
    date_published: datetime
    downloads: int = 0
    version_type: str
    status: str = ""
    requested_status: str | None = None
    files: list[VersionFile] = Field(default_factory=list)
    dependencies: list[Dependency] = Field(default_factory=list)
    loaders: list[str] = Field(default_factory=list)
    game_versions: list[str] = Field(default_factory=list)
    environment: str | None = None

    @property
    def primary_file(self) -> VersionFile | None:
        """Return the primary version file, or the first file if none is marked primary."""
        for file in self.files:
            if file.primary:
                return file
        return self.files[0] if self.files else None
