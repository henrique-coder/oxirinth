# AGENTS.md — project field guide

Read this before changing code. It explains what `oxirinth` does, the architectural boundaries that protect performance and type safety, and how to verify changes cleanly.

## What this project is

`oxirinth` is a strictly typed, read-only Python SDK for the **Modrinth API v3** (`https://api.modrinth.com/v3`).

- **Internal Rust Engine (`rust/oxirinth-engine`, `publish = false`):** Pure Rust crate with zero PyO3 dependency. Owns URL routing, domain facet injection, zero-copy `project_types` validation, `reqwest` HTTP/2 transport (`rustls` + `ring`), `reqwest-retry` backoff, proactive `X-Ratelimit-*` pacing, in-flight singleflight coalescing, and `redb` + `zstd` persistent caching (with a bounded RAM LRU fallback).
- **Thin PyO3 Bridge (`rust/oxirinth-py`, `publish = false`):** Compiles via Maturin into `oxirinth._core` (`#[pymodule(gil_used = false)]`). Runs one shared global Tokio multi-threaded runtime, guards all FFI entry points with `catch_unwind`, releases the GIL for blocking calls (`SyncCore`), bridges futures via `pyo3-async-runtimes` (`AsyncCore`), and returns raw UTF-8 JSON `bytes` (`Py<PyBytes>`) to Python without constructing intermediate Python dicts.
- **Python DX Layer (`src/oxirinth/`):** Pydantic v2 models (`frozen=True`, `extra="ignore"`) validated directly from Rust `bytes` via `pydantic-core` `TypeAdapter.validate_json()`, wrapped by explicit `Oxirinth` (sync) and `AsyncOxirinth` (async) clients.

Binary file downloading is intentionally out of scope for this library. Users inspect `VersionFile.url` / `hashes` and bring their own downloader.

## Code map

- `rust/oxirinth-engine/src/`:
  - `error.rs`: `EngineError` (`#[non_exhaustive]`) and Modrinth JSON error parser.
  - `config.rs`: `EngineConfig`, `CacheConfig`, and redacted `SecretString` for `api_key`.
  - `domain.rs`: `DomainKind` (`Mod`, `Plugin`, `ResourcePack`, `DataPack`, `Shader`, `Modpack`, `Server`), facet builder, and zero-copy `project_types` probe.
  - `routes.rs`: Single source of truth for all Modrinth `/v3` endpoints and explicitly documented `/v2/tag/*` fallbacks.
  - `cache/`: Binary envelope codec (`zstd` >= 512B), `redb` persistent store, RAM LRU fallback, and `CacheManager`.
  - `http/`: `SingleflightGroup` request coalescer, proactive rate limiter, and `reqwest` pipeline.
- `rust/oxirinth-py/src/`: PyO3 `oxirinth._core` module, Tokio runtime, exception hierarchy, and sync/async bridges.
- `src/oxirinth/`: Public Python package (`_core.pyi`, `py.typed`, `models/`, `client/`, `_internal/`).
- `docs/`: Zensical documentation and `docs/api/v3-endpoints.md`.
- `tests/`: Offline unit/integration tests and parity checks (`live` marker is opt-in only).

## Contracts that must not change accidentally

1. **No standalone Rust crate publication:** Both `oxirinth-engine` and `oxirinth-py` remain `publish = false`.
2. **Zero hardcoded URLs or facets in Python:** All Modrinth endpoints live in `rust/oxirinth-engine/src/routes.rs` and domain facets live in `rust/oxirinth-engine/src/domain.rs`.
3. **v3-first with tracked v2 fallbacks:** Never add `/v2` endpoints when a `/v3` equivalent exists. Keep `docs/api/v3-endpoints.md` up to date on every release check.
4. **Domain-specific public interface:** Expose explicit domain methods (`search_mods`, `get_plugin`, `search_servers`, etc.), never generic `get_project` / `search_projects` wrappers on public clients.
5. **100% Sync/Async Parity:** `Oxirinth` and `AsyncOxirinth` must maintain identical method names, parameters, defaults, and keyword-only signatures (verified by tests), with zero magic dual-return functions.
6. **Secret & Cache Isolation:** Never log or write raw `api_key` values to disk. Cache keys must include the hashed `auth_fingerprint` so authenticated and anonymous responses never mix.
7. **Stub Parity:** `src/oxirinth/_core.pyi` must always match the compiled `oxirinth._core` module (`mypy.stubtest oxirinth._core` must pass with zero issues).

## Style & canonical commands

- Repository source, comments, docs, and commit messages are written in `en_US` English.
- Keep `just lint`, `just test`, and `just docs-build` green after changes:
  - `just format` — formats Rust (`cargo fmt`), Python (`ruff`), Markdown/YAML (`prettier`), and TOML (`taplo`).
  - `just lint` — checks `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `ruff`, `ty`, `mypy.stubtest`, `prettier`, `taplo`, and `zizmor` (when `.github/workflows` exists).
  - `just test` — runs `cargo test -p oxirinth-engine` and `pytest`.
  - `just docs-build` — runs `zensical build --clean --strict`.
- Never run `git commit` or `git push` without explicit user authorization.
