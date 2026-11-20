# Oxirinth

High-performance Python SDK for the Modrinth API v3 (`https://api.modrinth.com/v3`), powered by Rust.

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Python 3.11–3.15](https://img.shields.io/badge/python-3.11%20%E2%80%93%203.15%20%7C%20free--threaded-blue.svg)](pyproject.toml)

## Overview

- **Internal Rust engine (`oxirinth._core`):** Uses `reqwest` HTTP/2 (`rustls` + `ring`), exponential backoff retries (`reqwest-retry`), `X-Ratelimit-*` quota pacing, in-flight request coalescing, and `redb` + `zstd` disk caching with a bounded in-memory LRU fallback.
- **Zero-dict JSON validation:** Streams UTF-8 JSON bytes from Rust directly into `pydantic-core` `TypeAdapter.validate_json()`.
- **Domain-first API:** Exposes explicit methods per Modrinth domain (`search_mods`, `get_plugin`, `search_servers`, etc.) returning immutable Pydantic v2 models (`Mod`, `Plugin`, `ResourcePack`, `DataPack`, `Shader`, `Modpack`, `Server`).
- **Sync and async parity:** `Oxirinth` releases the GIL during blocking calls; `AsyncOxirinth` bridges futures natively to `asyncio` with identical method signatures.
- **Read-only metadata scope:** File downloads are excluded. Inspect `VersionFile.url` and `VersionFile.sha512` / `VersionFile.sha1` to integrate any HTTP client you prefer.

## Installation

Install with `uv`:

```bash
uv add oxirinth
```

Pre-built wheels target **Python 3.11–3.15** (including free-threaded `3.14t`–`3.15t`) across Linux (`manylinux` and `musllinux` on `x86_64`, `aarch64`, `armv7`, `x86`), macOS (`arm64`, `x86_64`), and Windows (`x64`, `x86`, `arm64`).

## Quickstart

### Synchronous (`Oxirinth`)

```python
from oxirinth import Oxirinth, build_user_agent

ua = build_user_agent("my-app", "1.0.0", author="my-handle", contact="ops@example.com")

with Oxirinth(user_agent=ua) as client:
    results = client.search_mods("sodium", loaders=["fabric"], game_versions=["1.21.1"], limit=5)
    print(f"{results.total} matches (page {results.page})")

    for mod in results:
        print(f"{mod.name} ({mod.slug}): {mod.downloads:,} downloads, {mod.followers:,} followers")

    if results.first is not None:
        full_mod = results.first.fetch()
        print(full_mod.published_at, full_mod.categories)

    plugin = client.get_plugin("luckperms")
    latest = plugin.get_latest_version(loaders=["paper"])
    if latest and latest.primary_file:
        print(latest.primary_file.url)
        print(latest.primary_file.sha512)
```

### Asynchronous (`AsyncOxirinth`)

```python
import asyncio
from oxirinth import AsyncOxirinth


async def main() -> None:
    async with AsyncOxirinth() as client:
        mods, servers = await asyncio.gather(
            client.get_mods(["sodium", "lithium", "iris"]),
            client.search_servers(limit=3, sort="follows"),
        )
        print([m.name for m in mods])
        for srv in servers:
            print(srv.name, srv.address, srv.players, srv.latency_ms)


asyncio.run(main())
```

## Public Namespaces

Import directly from `oxirinth` for everyday usage, or browse specific submodules when you need detailed nested types:

- `oxirinth`: `Oxirinth`, `AsyncOxirinth`, `CacheConfig`, `CacheStats`, `build_user_agent`, `DEFAULT_USER_AGENT`, domain models, `SearchPage`, summary models, `PlayerCount`, versions, users, organizations, tags, and exceptions.
- `oxirinth.models`: Full set of public models including nested server/metadata structures (`JavaServerInfo`, `BedrockServerInfo`, `ServerMeta`, `ServerContent`, `ServerPing`, `ServerPingData`, `ServerLatency`, `License`, `LinkUrl`, `GalleryImage`, `SearchDependency`).
- `oxirinth.exceptions`: All `OxirinthError` subclasses.

## Supported Domains

| Domain         | Search Method          | Fetch Methods                            | Domain Models                        | Search Summary Models                              |
| :------------- | :--------------------- | :--------------------------------------- | :----------------------------------- | :------------------------------------------------- |
| Mods           | `search_mods`          | `get_mod` / `get_mods`                   | `Mod` / `AsyncMod`                   | `ModSummary` / `AsyncModSummary`                   |
| Plugins        | `search_plugins`       | `get_plugin` / `get_plugins`             | `Plugin` / `AsyncPlugin`             | `PluginSummary` / `AsyncPluginSummary`             |
| Resource Packs | `search_resourcepacks` | `get_resourcepack` / `get_resourcepacks` | `ResourcePack` / `AsyncResourcePack` | `ResourcePackSummary` / `AsyncResourcePackSummary` |
| Data Packs     | `search_datapacks`     | `get_datapack` / `get_datapacks`         | `DataPack` / `AsyncDataPack`         | `DataPackSummary` / `AsyncDataPackSummary`         |
| Shaders        | `search_shaders`       | `get_shader` / `get_shaders`             | `Shader` / `AsyncShader`             | `ShaderSummary` / `AsyncShaderSummary`             |
| Modpacks       | `search_modpacks`      | `get_modpack` / `get_modpacks`           | `Modpack` / `AsyncModpack`           | `ModpackSummary` / `AsyncModpackSummary`           |
| Servers        | `search_servers`       | `get_server` / `get_servers`             | `Server` / `AsyncServer`             | `ServerSummary` / `AsyncServerSummary`             |

Additional client methods cover versions and file hashes (`get_version`, `get_versions`, `get_version_by_hash`), users and organizations (`get_user`, `get_organization`), and tags (`get_loaders`, `get_categories`, `get_game_versions`, `get_licenses`).

## Development Setup

Requirements:

- Python `>=3.11, <3.16` (`.python-version` selects `3.15` locally)
- Rust toolchain (`rustc >= 1.88`, `cargo`)
- `uv`, `pnpm`, and `just`

```bash
uv sync --all-groups
pnpm install
just lint
just test
just test-live
just docs-build
```

## Documentation

- [Getting Started](docs/getting-started.md)
- [Domains & Models](docs/guide/domains.md)
- [Caching & Rate Limits](docs/guide/caching-and-ratelimits.md)
- [Exceptions](docs/guide/exceptions.md)
- [Modrinth API v3 Endpoint Reference](docs/api/v3-endpoints.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Security Policy](SECURITY.md)

## License

[MIT](LICENSE)
