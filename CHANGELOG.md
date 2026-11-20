# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0).

## [0.1.0] - Unreleased

### Added

- **Internal Rust engine (`oxirinth-engine` + `oxirinth._core`):** Built on `reqwest` HTTP/2 (`rustls` + `ring`), `reqwest-retry` exponential backoff, proactive `X-Ratelimit-*` pacing, in-flight singleflight coalescing, and persistent `redb` + `zstd` caching with a bounded RAM LRU fallback and auth-session isolation.
- **Zero-dict FFI byte pipeline:** PyO3 bridge (`SyncCore`, `AsyncCore`, `CacheStats`) guarding boundaries with `catch_unwind`, releasing the GIL for blocking calls, bridging `asyncio` futures via `pyo3-async-runtimes`, and passing raw UTF-8 JSON `bytes` directly to `pydantic-core`.
- **Modrinth-compliant User-Agent handling:** Default `User-Agent` formatted as `henrique-coder/oxirinth/0.1.0 (https://github.com/henrique-coder/oxirinth)` alongside `DEFAULT_USER_AGENT` and `build_user_agent()`.
- **Domain-specific Pydantic v2 models & clients:** `Oxirinth` and `AsyncOxirinth` with 100% signature parity across all 7 Modrinth domains (`Mod`, `Plugin`, `ResourcePack`, `DataPack`, `Shader`, `Modpack`, `Server` + `Async*` twins), versions, file-hash lookups, users, organizations, tags, and typed exceptions.
- **Verification & CI/CD:** Offline `wiremock` integration tests, sync/async signature parity tests, `mypy.stubtest` verification, `zensical` documentation site, and GitHub Actions workflows for CI, CodeQL/zizmor security, and cross-platform wheel builds (`manylinux`, `musllinux`, macOS, Windows, Python `3.11`–`3.15` + free-threaded builds).
