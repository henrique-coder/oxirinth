# Oxirinth

**Oxirinth** is a high-performance, strictly typed read-only Python SDK for the **Modrinth API v3** (`https://api.modrinth.com/v3`), backed by a Rust transport and cache engine (`oxirinth._core`) and Pydantic v2 domain models.

## Why Oxirinth?

- **Zero-Dict FFI Fast Path:** Rust handles HTTP/2 transport (`reqwest` with `rustls` + `ring`), exponential retries (`reqwest-retry`), proactive `X-Ratelimit-*` pacing, in-flight singleflight coalescing, and persistent `redb` + `zstd` caching, then streams raw UTF-8 JSON `bytes` directly to `pydantic-core` (`TypeAdapter.validate_json()`).
- **Domain-First Developer Experience:** Explicit domain methods (`search_mods`, `get_plugin`, `search_servers`, etc.) inject Modrinth v3 facets automatically and return focused, immutable domain models (`Mod`, `Plugin`, `ResourcePack`, `DataPack`, `Shader`, `Modpack`, `Server`).
- **100% Sync & Async Parity:** Choose `Oxirinth` (GIL-releasing blocking calls) or `AsyncOxirinth` (native `asyncio` futures via `pyo3-async-runtimes`) with identical method names, keyword-only options, and strict `ty` / Pyright type hints.
- **Metadata Only by Design:** Oxirinth focuses exclusively on fast, reliable API metadata queries. Inspect `VersionFile.url` and `VersionFile.hashes` (`sha512`, `sha1`) and pair them with whichever HTTP downloader your application prefers.

## Quick Example

=== "Sync (`Oxirinth`)"

    ```python
    from oxirinth import Oxirinth

    with Oxirinth() as client:
        mods = client.search_mods("sodium", loaders=["fabric"], game_versions=["1.21.1"])
        for hit in mods.hits:
            print(hit.name, hit.downloads)

        plugin = client.get_plugin("luckperms")
        latest = plugin.get_latest_version(loaders=["paper"])
        if latest and latest.primary_file:
            print(latest.primary_file.url, latest.primary_file.hashes.sha512)
    ```

=== "Async (`AsyncOxirinth`)"

    ```python
    import asyncio
    from oxirinth import AsyncOxirinth


    async def main() -> None:
        async with AsyncOxirinth() as client:
            servers = await client.search_servers(limit=5, sort="follows")
            for srv in servers.hits:
                print(srv.name, srv.address, srv.players_online)


    asyncio.run(main())
    ```

## Next Steps

- [Getting Started](getting-started.md)
- [Domains & Models](guide/domains.md)
- [Caching & Rate Limits](guide/caching-and-ratelimits.md)
- [Exceptions](guide/exceptions.md)
- [Modrinth API v3 Reference](api/v3-endpoints.md)
