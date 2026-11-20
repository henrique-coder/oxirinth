"""Typed configuration models and literal aliases for Oxirinth."""

from __future__ import annotations

from pathlib import Path
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field


__all__: list[str] = [
    "CacheConfig",
    "Environment",
    "HashAlgorithm",
    "SortIndex",
]

SortIndex = Literal["relevance", "downloads", "follows", "newest", "updated"]
Environment = Literal[
    "client_only",
    "server_only",
    "client_and_server",
    "singleplayer_only",
    "dedicated_server_only",
    "unknown",
]
HashAlgorithm = Literal["sha512", "sha1"]


class CacheConfig(BaseModel):
    """Configuration for Oxirinth's persistent `redb` + `zstd` HTTP response cache.

    Attributes:
        enabled: Enable local response caching (defaults to ``True``).
        cache_dir: Custom directory for ``http_cache.redb``; defaults to the OS cache directory.
        ram_fallback_max_bytes: Byte budget for the in-memory LRU fallback cache.
        zstd_min_bytes: Minimum body size in bytes before ``zstd`` compression is applied.
        zstd_level: Zstandard compression level (1 through 22).
        search_ttl_secs: Fresh TTL in seconds for ``/v3/search`` responses.
        project_ttl_secs: Fresh TTL in seconds for project, user, and organization responses.
        version_list_ttl_secs: Fresh TTL in seconds for ``/v3/project/*/version`` responses.
        version_ttl_secs: Fresh TTL in seconds for individual version and file-hash responses.
        tag_ttl_secs: Fresh TTL in seconds for loader, category, game version, and license tags.
    """

    model_config = ConfigDict(extra="forbid", frozen=True)

    enabled: bool = True
    cache_dir: Path | None = None
    ram_fallback_max_bytes: int = Field(default=67_108_864, ge=1)
    zstd_min_bytes: int = Field(default=512, ge=0)
    zstd_level: int = Field(default=3, ge=1, le=22)
    search_ttl_secs: int = Field(default=120, ge=0)
    project_ttl_secs: int = Field(default=900, ge=0)
    version_list_ttl_secs: int = Field(default=300, ge=0)
    version_ttl_secs: int = Field(default=86_400, ge=0)
    tag_ttl_secs: int = Field(default=21_600, ge=0)
