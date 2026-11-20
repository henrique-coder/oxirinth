"""Domain-specific Modrinth ``/v3/search`` hit and pagination envelope models."""

from __future__ import annotations

from datetime import datetime
from typing import Generic, TypeVar

from pydantic import Field

from oxirinth.models.common import OxirinthModel, SearchDependency
from oxirinth.models.server import BedrockServerInfo, JavaServerInfo, ServerMeta


__all__: list[str] = [
    "BaseHit",
    "DataPackHit",
    "HitT",
    "ModHit",
    "ModpackHit",
    "PluginHit",
    "ResourcePackHit",
    "SearchResult",
    "ServerHit",
    "ShaderHit",
]


class BaseHit(OxirinthModel):
    """Shared search hit fields returned by Modrinth ``/v3/search``."""

    project_id: str
    version_id: str | None = None
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
    follows: int = 0
    icon_url: str | None = None
    date_created: datetime | None = None
    date_modified: datetime | None = None
    license: str | None = None
    gallery: list[str] = Field(default_factory=list)
    featured_gallery: str | None = None
    color: int | None = None
    dependencies: list[SearchDependency] = Field(default_factory=list)


class ModHit(BaseHit):
    """Search result hit for a Modrinth mod."""

    loaders: list[str] = Field(default_factory=list)
    environment: list[str] = Field(default_factory=list)
    game_versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class PluginHit(BaseHit):
    """Search result hit for a Modrinth server plugin."""

    loaders: list[str] = Field(default_factory=list)
    environment: list[str] = Field(default_factory=list)
    game_versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class ResourcePackHit(BaseHit):
    """Search result hit for a Modrinth resource pack."""

    game_versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class DataPackHit(BaseHit):
    """Search result hit for a Modrinth data pack."""

    game_versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class ShaderHit(BaseHit):
    """Search result hit for a Modrinth shader."""

    loaders: list[str] = Field(default_factory=list)
    game_versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class ModpackHit(BaseHit):
    """Search result hit for a Modrinth modpack."""

    mod_loaders: list[str] = Field(default_factory=list, alias="mrpack_loaders")
    environment: list[str] = Field(default_factory=list)
    game_versions: list[str] = Field(default_factory=list)
    downloads: int = 0


class ServerHit(BaseHit):
    """Search result hit for a Modrinth server listing."""

    server_meta: ServerMeta | None = Field(default=None, alias="minecraft_server")
    java: JavaServerInfo | None = Field(default=None, alias="minecraft_java_server")
    bedrock: BedrockServerInfo | None = Field(default=None, alias="minecraft_bedrock_server")
    mod_loaders: list[str] = Field(default_factory=list, alias="mrpack_loaders")
    game_versions: list[str] = Field(default_factory=list)

    @property
    def address(self) -> str | None:
        """Return the primary Java Edition server address if non-empty."""
        if self.java and self.java.address:
            return self.java.address
        return None

    @property
    def players_online(self) -> int | None:
        """Return the latest observed online player count if available."""
        if self.java and self.java.ping:
            return self.java.ping.data.players_online
        return None

    @property
    def players_max(self) -> int | None:
        """Return the latest observed or configured maximum player slot count."""
        if self.java and self.java.ping and self.java.ping.data.players_max is not None:
            return self.java.ping.data.players_max
        if self.server_meta:
            return self.server_meta.max_players
        return None


HitT = TypeVar("HitT", bound=BaseHit)


class SearchResult(OxirinthModel, Generic[HitT]):
    """Paginated response envelope returned by Modrinth ``/v3/search``."""

    hits: list[HitT] = Field(default_factory=list)
    page: int = 1
    hits_per_page: int = 0
    total_hits: int = 0
