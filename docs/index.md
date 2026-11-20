# Oxirinth

**Oxirinth** is a high-performance, strictly typed Python SDK for the **Modrinth API v3** (`api.modrinth.com/v3`), backed by a Rust transport/cache engine (`oxirinth._core`) and Pydantic v2 domain models.

## Architecture Overview

- **Internal Rust Engine (`oxirinth-engine` + PyO3 `oxirinth._core`):** Handles HTTP/2 transport via `reqwest` (`rustls` + `ring`), exponential retries via `reqwest-retry`, proactive `X-Ratelimit-*` pacing, in-flight singleflight coalescing, and persistent `redb` + `zstd` caching with a bounded RAM LRU fallback.
- **Zero-Dict Byte Handoff:** Rust returns raw UTF-8 JSON `bytes` directly to `pydantic-core` `TypeAdapter.validate_json()` without allocating intermediate Python dictionaries.
- **Domain-Specific Models & Sync/Async Parity:** Explicit `Oxirinth` and `AsyncOxirinth` clients expose typed domain methods (`search_mods`, `get_plugin`, `search_servers`, etc.) returning strict Pydantic v2 models.

## Reference

- [Modrinth API v3 Endpoints & Facets](api/v3-endpoints.md)
