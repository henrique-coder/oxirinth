"""Domain-specific Modrinth v3 server metadata models."""

from __future__ import annotations

from datetime import datetime
from typing import Any

from pydantic import Field

from oxirinth.models.common import OxirinthModel
from oxirinth.models.domains import BaseDomainMeta, _AsyncMemberMixin, _SyncMemberMixin


__all__: list[str] = [
    "AsyncServer",
    "BaseServer",
    "BedrockServerInfo",
    "JavaServerInfo",
    "Server",
    "ServerContent",
    "ServerLatency",
    "ServerMeta",
    "ServerPing",
    "ServerPingData",
]


class ServerLatency(OxirinthModel):
    """Duration structure reported by Modrinth server status pings."""

    secs: int = 0
    nanos: int = 0

    @property
    def milliseconds(self) -> float:
        """Return total latency in milliseconds."""
        return self.secs * 1_000.0 + self.nanos / 1_000_000.0


class ServerPingData(OxirinthModel):
    """Live Minecraft status ping snapshot embedded in ``minecraft_java_server.ping.data``."""

    latency: ServerLatency | None = None
    version_name: str | None = None
    version_protocol: int | None = None
    description: Any | None = None
    players_online: int | None = None
    players_max: int | None = None


class ServerPing(OxirinthModel):
    """Timestamped Minecraft server ping wrapper."""

    when: datetime
    address: str
    data: ServerPingData


class ServerContent(OxirinthModel):
    """Linked modpack or content reference associated with a Modrinth server."""

    kind: str
    version_id: str | None = None
    project_id: str | None = None
    project_name: str | None = None
    project_icon: str | None = None


class ServerMeta(OxirinthModel):
    """General server metadata from ``minecraft_server``."""

    max_players: int | None = None
    country: str | None = None
    region: str | None = None
    languages: list[str] = Field(default_factory=list)
    active_version: str | None = None


class JavaServerInfo(OxirinthModel):
    """Java Edition server metadata from ``minecraft_java_server``."""

    address: str = ""
    content: ServerContent | None = None
    ping: ServerPing | None = None
    verified_plays_2w: int = 0
    verified_plays_4w: int = 0


class BedrockServerInfo(OxirinthModel):
    """Bedrock Edition server metadata from ``minecraft_bedrock_server``."""

    address: str = ""


class BaseServer(BaseDomainMeta):
    """Pure-data Modrinth v3 server schema (``minecraft_java_server``)."""

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
    def bedrock_address(self) -> str | None:
        """Return the Bedrock Edition server address if configured."""
        if self.bedrock and self.bedrock.address:
            return self.bedrock.address
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

    @property
    def region(self) -> str | None:
        """Return the configured server region if available."""
        return self.server_meta.region if self.server_meta else None

    @property
    def content(self) -> ServerContent | None:
        """Return linked server modpack/content metadata if present."""
        return self.java.content if self.java else None

    @property
    def ping(self) -> ServerPing | None:
        """Return the latest timestamped Java ping payload if present."""
        return self.java.ping if self.java else None


class Server(BaseServer, _SyncMemberMixin):
    """Modrinth server model bound to a synchronous ``Oxirinth`` client."""


class AsyncServer(BaseServer, _AsyncMemberMixin):
    """Modrinth server model bound to an asynchronous ``AsyncOxirinth`` client."""
