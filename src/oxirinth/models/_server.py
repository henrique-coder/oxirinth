"""Minecraft server domain models and shared server status helpers."""

from __future__ import annotations

from datetime import datetime
from typing import Any

from pydantic import AliasChoices, Field

from oxirinth.models._common import _OxirinthModel
from oxirinth.models._domains import _AsyncMemberMixin, _DomainMeta, _SyncMemberMixin


__all__: list[str] = [
    "AsyncServer",
    "BedrockServerInfo",
    "JavaServerInfo",
    "PlayerCount",
    "Server",
    "ServerContent",
    "ServerLatency",
    "ServerMeta",
    "ServerPing",
    "ServerPingData",
]


class PlayerCount(_OxirinthModel):
    """Snapshot of online and maximum server player slots."""

    online: int | None = None
    max: int | None = None

    def __str__(self) -> str:
        """Render the count in compact ``online/max`` form."""
        online = "-" if self.online is None else str(self.online)
        maximum = "-" if self.max is None else str(self.max)
        return f"{online}/{maximum}"


class ServerLatency(_OxirinthModel):
    """Ping round-trip duration reported by Modrinth server monitors."""

    secs: int = 0
    nanos: int = 0

    @property
    def milliseconds(self) -> float:
        """Total latency in milliseconds."""
        return self.secs * 1_000.0 + self.nanos / 1_000_000.0


class ServerPingData(_OxirinthModel):
    """Live Minecraft status ping payload."""

    latency: ServerLatency | None = None
    version_name: str | None = None
    version_protocol: int | None = None
    description: Any | None = None
    online_players: int | None = Field(default=None, validation_alias=AliasChoices("players_online", "online_players"))
    max_players: int | None = Field(default=None, validation_alias=AliasChoices("players_max", "max_players"))


class ServerPing(_OxirinthModel):
    """Timestamped Minecraft server status snapshot."""

    checked_at: datetime = Field(validation_alias=AliasChoices("when", "checked_at"))
    address: str
    data: ServerPingData


class ServerContent(_OxirinthModel):
    """Modpack or content reference linked to a server listing."""

    kind: str
    version_id: str | None = None
    project_id: str | None = None
    project_name: str | None = None
    project_icon: str | None = None


class ServerMeta(_OxirinthModel):
    """General server configuration metadata."""

    max_players: int | None = None
    country: str | None = None
    region: str | None = None
    languages: list[str] = Field(default_factory=list)
    active_version: str | None = None


class JavaServerInfo(_OxirinthModel):
    """Java Edition server address, linked content, and ping telemetry."""

    address: str = ""
    content: ServerContent | None = None
    ping: ServerPing | None = None
    verified_plays_2w: int = 0
    verified_plays_4w: int = 0


class BedrockServerInfo(_OxirinthModel):
    """Bedrock Edition server connection details."""

    address: str = ""


class _ServerFieldsMixin(_OxirinthModel):
    """Shared server fields and convenience properties for ``Server`` and ``ServerSummary``."""

    meta: ServerMeta | None = Field(default=None, validation_alias=AliasChoices("minecraft_server", "meta"))
    java: JavaServerInfo | None = Field(default=None, validation_alias=AliasChoices("minecraft_java_server", "java"))
    bedrock: BedrockServerInfo | None = Field(
        default=None,
        validation_alias=AliasChoices("minecraft_bedrock_server", "bedrock"),
    )
    mod_loaders: list[str] = Field(default_factory=list, validation_alias=AliasChoices("mrpack_loaders", "mod_loaders"))
    game_versions: list[str] = Field(default_factory=list)

    @property
    def address(self) -> str | None:
        """Primary Java Edition connection address, if configured."""
        if self.java and self.java.address:
            return self.java.address
        return None

    @property
    def bedrock_address(self) -> str | None:
        """Bedrock Edition connection address, if configured."""
        if self.bedrock and self.bedrock.address:
            return self.bedrock.address
        return None

    @property
    def online_players(self) -> int | None:
        """Most recent observed online player count."""
        if self.java and self.java.ping:
            return self.java.ping.data.online_players
        return None

    @property
    def max_players(self) -> int | None:
        """Most recent observed or configured maximum player capacity."""
        if self.java and self.java.ping and self.java.ping.data.max_players is not None:
            return self.java.ping.data.max_players
        if self.meta:
            return self.meta.max_players
        return None

    @property
    def players(self) -> PlayerCount:
        """Combined online and maximum player count snapshot."""
        return PlayerCount(online=self.online_players, max=self.max_players)

    @property
    def latency_ms(self) -> float | None:
        """Most recent observed ping latency in milliseconds."""
        if self.java and self.java.ping and self.java.ping.data.latency is not None:
            return self.java.ping.data.latency.milliseconds
        return None

    @property
    def is_online(self) -> bool:
        """Whether a recent status ping is present for the Java endpoint."""
        return self.java is not None and self.java.ping is not None

    @property
    def region(self) -> str | None:
        """Configured hosting region."""
        return self.meta.region if self.meta else None

    @property
    def country(self) -> str | None:
        """Configured hosting country code."""
        return self.meta.country if self.meta else None

    @property
    def languages(self) -> list[str]:
        """Configured server languages."""
        return list(self.meta.languages) if self.meta else []

    @property
    def content(self) -> ServerContent | None:
        """Linked modpack or content reference."""
        return self.java.content if self.java else None

    @property
    def ping(self) -> ServerPing | None:
        """Most recent timestamped Java ping payload."""
        return self.java.ping if self.java else None


class _BaseServer(_DomainMeta, _ServerFieldsMixin):
    """Internal server listing schema."""


class Server(_BaseServer, _SyncMemberMixin):
    """Modrinth server bound to a synchronous ``Oxirinth`` client."""


class AsyncServer(_BaseServer, _AsyncMemberMixin):
    """Modrinth server bound to an asynchronous ``AsyncOxirinth`` client."""
