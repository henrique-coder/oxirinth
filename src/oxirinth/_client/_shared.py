"""Shared client configuration mapping for sync and async clients."""

from __future__ import annotations

from typing import Any

from oxirinth._config import CacheConfig


__all__: list[str] = ["build_core_kwargs"]


def build_core_kwargs(
    *,
    user_agent: str | None,
    api_key: str | None,
    cache: CacheConfig | bool,
    timeout: float,
    max_retries: int,
    max_rate_limit_wait: float | None,
    base_url: str,
) -> dict[str, Any]:
    """Translate Python client arguments into ``SyncCore`` / ``AsyncCore`` kwargs."""
    cache_cfg = CacheConfig(enabled=cache) if isinstance(cache, bool) else cache

    return {
        "user_agent": user_agent,
        "api_key": api_key,
        "cache_enabled": cache_cfg.enabled,
        "cache_dir": cache_cfg.cache_dir,
        "ram_fallback_max_bytes": cache_cfg.ram_fallback_max_bytes,
        "zstd_min_bytes": cache_cfg.zstd_min_bytes,
        "zstd_level": cache_cfg.zstd_level,
        "search_ttl_secs": cache_cfg.search_ttl_secs,
        "project_ttl_secs": cache_cfg.project_ttl_secs,
        "version_list_ttl_secs": cache_cfg.version_list_ttl_secs,
        "version_ttl_secs": cache_cfg.version_ttl_secs,
        "tag_ttl_secs": cache_cfg.tag_ttl_secs,
        "timeout_secs": timeout,
        "max_retries": max_retries,
        "max_rate_limit_wait_secs": max_rate_limit_wait,
        "base_url": base_url,
    }
