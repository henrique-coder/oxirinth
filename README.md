# Oxirinth

**High-performance, strictly typed Python SDK for the Modrinth API v3, powered by Rust and Pydantic v2.**

> [!NOTE]
> Oxirinth is currently under active `0.1.0` development as a read-only Modrinth API v3 metadata SDK.

## Architecture

- **Internal Rust Engine (`oxirinth-engine` + `oxirinth._core`):** Handles HTTP/2 transport (`reqwest` with `rustls` + `ring`), exponential retries (`reqwest-retry`), proactive `X-Ratelimit-*` rate-limit pacing, singleflight request deduplication, and `redb` + `zstd` local caching with a bounded RAM LRU fallback.
- **Zero-Dict Byte Handoff:** Rust streams raw UTF-8 JSON bytes directly into Pydantic v2 `TypeAdapter.validate_json()` (`pydantic-core`), bypassing intermediate Python dictionary allocations.
- **Domain-First API:** Explicit `Oxirinth` (sync) and `AsyncOxirinth` (async) clients with human-readable methods (`search_mods`, `get_plugin`, `search_servers`, etc.) and strict domain models (`Mod`, `Plugin`, `ResourcePack`, `DataPack`, `Shader`, `Modpack`, `Server`).

## Development Setup

Requirements:

- Python `>=3.11, <3.16` (`.python-version` selects `3.15` for local development)
- Rust toolchain (`rustc` / `cargo`)
- `uv`, `pnpm`, and `just`

```bash
uv sync --all-groups
pnpm install
just lint
just test
```

## Documentation

- [Modrinth API v3 Reference](docs/api/v3-endpoints.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Security Policy](SECURITY.md)

## License

[MIT](LICENSE)
