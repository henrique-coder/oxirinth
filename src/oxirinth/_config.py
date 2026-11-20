"""Configuration models, User-Agent helper, and typed literal aliases."""

from __future__ import annotations

from pathlib import Path
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field

from oxirinth._core import DEFAULT_USER_AGENT, OxirinthConfigError


__all__: list[str] = [
    "DEFAULT_USER_AGENT",
    "CacheConfig",
    "Environment",
    "HashAlgorithm",
    "SortIndex",
    "build_user_agent",
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


def _validate_ua_part(value: str, label: str) -> str:
    cleaned = value.strip()
    if not cleaned or any(ord(ch) < 32 or ord(ch) == 127 for ch in cleaned):
        msg = f"{label} must be non-empty and contain no control characters"
        raise OxirinthConfigError(msg)
    return cleaned


def build_user_agent(
    project: str,
    version: str,
    *,
    author: str | None = None,
    contact: str | None = None,
) -> str:
    """Format a Modrinth-recommended ``User-Agent`` header value.

    Args:
        project: Application or library name.
        version: Application or library version string.
        author: Optional GitHub username, organization, or vendor prefix.
        contact: Optional contact email or repository URL placed in parentheses.

    Returns:
        Formatted ``User-Agent`` string validated for HTTP header safety.
    """
    clean_project = _validate_ua_part(project, "project")
    clean_version = _validate_ua_part(version, "version")

    prefix = (
        f"{_validate_ua_part(author, 'author')}/{clean_project}/{clean_version}"
        if author is not None
        else f"{clean_project}/{clean_version}"
    )

    if contact is not None:
        return f"{prefix} ({_validate_ua_part(contact, 'contact')})"
    return prefix


class CacheConfig(BaseModel):
    """Settings for local ``redb`` + ``zstd`` caching and RAM LRU fallback.

    Attributes:
        enabled: Enable response caching (defaults to ``True``).
        cache_dir: Custom directory for ``http_cache.redb``; defaults to the OS cache directory.
        ram_fallback_max_bytes: Byte budget for the in-memory LRU fallback cache.
        zstd_min_bytes: Minimum payload size in bytes before ``zstd`` compression applies.
        zstd_level: Zstandard compression level (1 through 22).
        search_ttl_secs: TTL in seconds for ``/v3/search`` responses.
        project_ttl_secs: TTL in seconds for project, user, and organization responses.
        version_list_ttl_secs: TTL in seconds for ``/v3/project/*/version`` responses.
        version_ttl_secs: TTL in seconds for individual version and file-hash responses.
        tag_ttl_secs: TTL in seconds for loader, category, game version, and license tags.
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
