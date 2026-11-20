# Oxirinth

**High-performance, strictly typed Python SDK for the Modrinth API v3, powered by Rust and Pydantic v2.**

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Python 3.11–3.15](https://img.shields.io/badge/python-3.11%20%E2%80%93%203.15%20%7C%20free--threaded-blue.svg)](pyproject.toml)

Oxirinth separates network transport, rate limiting, and local persistence from Python Developer Experience:

- **Internal Rust Engine (`oxirinth._core`):** Built on `reqwest` HTTP/2 (`rustls` + `ring`), `reqwest-retry`, proactive `X-Ratelimit-*` smooth pacing, singleflight request coalescing, and persistent `redb` + `zstd` caching with a bounded RAM LRU fallback.
- **Zero-Dict JSON Byte Fast Path:** Rust streams raw UTF-8 JSON `bytes` directly to `pydantic-core`'s `TypeAdapter.validate_json()` without allocating intermediate Python dictionaries.
- **Domain-First Design:** Explicit domain methods (`search_mods`, `get_plugin`, `search_servers`, etc.) inject Modrinth v3 facets automatically and return focused Pydantic v2 models (`Mod`, `Plugin`, `ResourcePack`, `DataPack`, `Shader`, `Modpack`, `Server`).
- **100% Sync & Async Parity:** `Oxirinth` (GIL-releasing blocking client) and `AsyncOxirinth` (`asyncio`-bridged client) expose identical method names and signatures.
- **Read-Only Metadata Scope:** Oxirinth intentionally excludes file downloading. Inspect `VersionFile.url` and `VersionFile.hashes` (`sha512`, `sha1`) with your preferred downloader.

## Installation

```bash
uv add oxirinth
# or
pip install oxirinth
```

Pre-built wheels target **Python 3.11–3.15** (including free-threaded `3.13t`–`3.15t`) across Linux (`manylinux` and `musllinux` on `x86_64`, `aarch64`, `armv7`, `x86`), macOS (`arm64`, `x86_64`), and Windows (`x64`, `x86`, `arm64`).

## Quickstart

### Synchronous (`Oxirinth`)

```python
from oxirinth import Oxirinth, build_user_agent

ua = build_user_agent("my-app", "1.0.0", author="my-handle", contact="ops@example.com")

with Oxirinth(user_agent=ua) as client:
    results = client.search_mods("sodium", loaders=["fabric"], game_versions=["1.21.1"], limit=5)
    for hit in results.hits:
        print(f"{hit.name} ({hit.slug}): {hit.downloads:,} downloads")

    plugin = client.get_plugin("luckperms")
    latest = plugin.get_latest_version(loaders=["paper"])
    if latest and latest.primary_file:
        print(latest.primary_file.url)
        print(latest.primary_file.hashes.sha512)
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
        for srv in servers.hits:
            print(srv.name, srv.address, f"{srv.players_online}/{srv.players_max}")


asyncio.run(main())
```

## Supported Domains

| Domain         | Search                 | Get Single / Batch                       | Models (Sync / Async)                |
| :------------- | :--------------------- | :--------------------------------------- | :----------------------------------- |
| Mods           | `search_mods`          | `get_mod` / `get_mods`                   | `Mod` / `AsyncMod`                   |
| Plugins        | `search_plugins`       | `get_plugin` / `get_plugins`             | `Plugin` / `AsyncPlugin`             |
| Resource Packs | `search_resourcepacks` | `get_resourcepack` / `get_resourcepacks` | `ResourcePack` / `AsyncResourcePack` |
| Data Packs     | `search_datapacks`     | `get_datapack` / `get_datapacks`         | `DataPack` / `AsyncDataPack`         |
| Shaders        | `search_shaders`       | `get_shader` / `get_shaders`             | `Shader` / `AsyncShader`             |
| Modpacks       | `search_modpacks`      | `get_modpack` / `get_modpacks`           | `Modpack` / `AsyncModpack`           |
| Servers        | `search_servers`       | `get_server` / `get_servers`             | `Server` / `AsyncServer`             |

Plus versions & file hashes (`get_version`, `get_versions`, `get_version_by_hash`), users & organizations (`get_user`, `get_organization`), and tags (`get_loaders`, `get_categories`, `get_game_versions`, `get_licenses`).

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
