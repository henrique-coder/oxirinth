"""Version and downloadable file metadata models."""

from __future__ import annotations

from datetime import datetime

from pydantic import AliasChoices, Field

from oxirinth.models._common import Dependency, FileHashes, _OxirinthModel


__all__: list[str] = ["Version", "VersionFile"]


class VersionFile(_OxirinthModel):
    """Metadata and cryptographic digests for an individual release file."""

    id: str | None = None
    hashes: FileHashes
    url: str
    filename: str
    primary: bool = False
    size: int
    file_type: str | None = None

    @property
    def sha512(self) -> str | None:
        """SHA-512 hex digest of the file, if provided."""
        return self.hashes.sha512

    @property
    def sha1(self) -> str | None:
        """SHA-1 hex digest of the file, if provided."""
        return self.hashes.sha1


class Version(_OxirinthModel):
    """Release version metadata for a Modrinth project."""

    id: str
    project_id: str
    author_id: str | None = None
    featured: bool = False
    name: str
    version_number: str
    project_types: list[str] = Field(default_factory=list)
    games: list[str] = Field(default_factory=list)
    changelog: str | None = None
    published_at: datetime = Field(validation_alias=AliasChoices("date_published", "published_at"))
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
        """Primary release file, or the first file if none is marked primary."""
        for file in self.files:
            if file.primary:
                return file
        return self.files[0] if self.files else None
