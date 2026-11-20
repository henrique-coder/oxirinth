# Contributing

Thank you for considering a contribution to `oxirinth`.

This project wraps the Modrinth API v3 (`api.modrinth.com/v3`) using an internal Rust engine (`oxirinth-engine` + PyO3 `oxirinth._core`) and typed Pydantic v2 models. Changes should stay modular, strictly typed, and verified with offline tests before submission.

## Before Opening an Issue

- Search existing issues first.
- Make sure you are using the latest released version.
- Check the documentation at <https://oxirinth.readthedocs.io> and [docs/api/v3-endpoints.md](docs/api/v3-endpoints.md).
- Do not include Modrinth Personal Access Tokens (`api_key`), `Authorization` headers, or private project data in public issues.

## Development Setup

Requirements:

- Python `>=3.11, <3.16` (`.python-version` selects Python `3.15` for local development)
- Rust toolchain (`rustc` >= 1.88, `cargo`)
- `uv`
- `pnpm`
- `just`

Install all development dependencies and build the extension module:

```bash
uv sync --all-groups
pnpm install
```

Useful commands:

| Command           | Purpose                                                                              |
| :---------------- | :----------------------------------------------------------------------------------- |
| `just format`     | Format Rust, Python, Markdown, YAML, and TOML files.                                 |
| `just lint`       | Run Cargo fmt/clippy, Ruff, ty, mypy stubtest, Prettier, Taplo, and workflow checks. |
| `just test`       | Run Rust unit/integration tests and Python `pytest` tests.                           |
| `just docs-build` | Build Zensical documentation with `--clean --strict`.                                |
| `just build`      | Build source distribution and wheel via Maturin.                                     |

Install the Git hook once per checkout:

```bash
uv run --only-group hooks prek install --hook-type pre-push
```

The `just lint` check runs at `git push`, not at `git add` or `git commit`. CI remains authoritative.

## Pull Request Checklist

- Keep changes focused on one concern.
- Respect the strict FFI boundary: HTTP transport, retries, rate limiting, and `redb`+`zstd` caching stay inside `rust/oxirinth-engine`; Python stays focused on Pydantic v2 models and explicit `Oxirinth` / `AsyncOxirinth` client ergonomics.
- Never hardcode Modrinth URLs outside `rust/oxirinth-engine/src/routes.rs`, and keep `docs/api/v3-endpoints.md` synchronized whenever checking `/v2` fallback routes.
- Maintain 100% signature and method parity between `Oxirinth` and `AsyncOxirinth`.
- Add or update offline unit/integration tests for any behavior change.
- Run `just lint`, `just test`, and `just docs-build` before requesting review.

## Code Style

### Rust

- `rust/oxirinth-engine` forbids `unsafe` code (`#![forbid(unsafe_code)]`) and has zero PyO3 dependency.
- All PyO3 entry points in `rust/oxirinth-py` must guard against panics and release the GIL during blocking I/O.
- Keep `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Python

- Keep Python imports at module scope using `from ... import ...`.
- Prefer immutable Pydantic v2 models (`frozen=True`, `extra="ignore"`) validated from raw UTF-8 JSON bytes via `TypeAdapter.validate_json()`.
- Keep public APIs strictly typed and documented with Google-style docstrings.
- Use blank lines to separate logical phases inside functions (setup, validation, core execution, final return).
- Ruff and `ty` enforce formatting, import ordering, annotations, and strict static types.

## Commit Style

Use short, conventional commit messages:

```text
fix(cache): handle redb lock contention fallback
feat(engine): implement proactive rate-limit pacer
docs: update Modrinth v3 endpoint reference
ci: add free-threaded Python wheel targets
```

## Security

Do not report security-sensitive details in public issues. See `SECURITY.md` for the supported reporting process.
