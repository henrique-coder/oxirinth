# Oxirinth

**Oxirinth** is a strictly typed, read-only Python SDK for the **Modrinth API v3** (`https://api.modrinth.com/v3`). It pairs a Rust HTTP/2 + `redb` cache engine (`oxirinth._core`) with immutable Pydantic v2 domain models.

## Architecture

- **Zero-Dict FFI Fast Path:** Rust manages HTTP/2 connections (`reqwest` with `rustls` + `ring`), transient retries (`reqwest-retry`), proactive `X-Ratelimit-*` pacing, in-flight request coalescing, and persistent `redb` + `zstd` caching, then hands raw UTF-8 JSON `bytes` directly to `pydantic-core`.
- **Ergonomic Domain API:** Dedicated methods per Modrinth project type (`search_mods`, `get_plugin`, `search_servers`, etc.) inject Modrinth v3 facets automatically and return focused domain and search summary models (`Mod`, `ModSummary`, `Server`, `ServerSummary`, etc.).
- **Sync and Async Parity:** `Oxirinth` releases the GIL around blocking Rust calls and `AsyncOxirinth` bridges Tokio futures to `asyncio`, keeping identical parameter names, defaults, and strict type annotations.
- **Metadata Only:** File downloading is out of scope. Inspect `VersionFile.url`, `VersionFile.sha512`, and `VersionFile.sha1` to pair Oxirinth with any downloader.

## Quick Example

=== "Sync (`Oxirinth`)"

    ```python
    from oxirinth import Oxirinth

    with Oxirinth() as client:
        mods = client.search_mods("sodium", loaders=["fabric"], game_versions=["1.21.1"])
        for mod in mods:
            print(mod.name, mod.downloads, mod.followers)

        if mods.first is not None:
            full_mod = mods.first.fetch()
            print(full_mod.published_at)

        plugin = client.get_plugin("luckperms")
        latest = plugin.get_latest_version(loaders=["paper"])
        if latest and latest.primary_file:
            print(latest.primary_file.url, latest.primary_file.sha512)
    ```

=== "Async (`AsyncOxirinth`)"

    ```python
    import asyncio
    from oxirinth import AsyncOxirinth


    async def main() -> None:
        async with AsyncOxirinth() as client:
            servers = await client.search_servers(limit=5, sort="follows")
            for srv in servers:
                print(srv.name, srv.address, srv.players, srv.latency_ms)


    asyncio.run(main())
    ```

## Next Steps

- [Getting Started](getting-started.md)
- [Domains & Models](guide/domains.md)
- [Caching & Rate Limits](guide/caching-and-ratelimits.md)
- [Exceptions](guide/exceptions.md)
- [Modrinth API v3 Reference](api/v3-endpoints.md)
