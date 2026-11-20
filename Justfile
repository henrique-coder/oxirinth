default:
    @just --list

update:
    uv sync --upgrade --all-groups
    cargo update
    pnpm update

dev:
    uv run maturin develop

format:
    cargo fmt --all
    uv run --only-group lint ruff check --fix
    uv run --only-group lint ruff format
    pnpm prettier --write .
    pnpm taplo format Cargo.toml pyproject.toml zensical.toml rust/oxirinth-engine/Cargo.toml rust/oxirinth-py/Cargo.toml

lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    uv run --only-group lint ruff check
    uv run --only-group lint ruff format --check
    uv run --no-dev --group lint --group tests ty check
    uv run --no-dev --group lint python -m mypy.stubtest oxirinth._core
    pnpm prettier --check .
    pnpm taplo lint Cargo.toml pyproject.toml zensical.toml rust/oxirinth-engine/Cargo.toml rust/oxirinth-py/Cargo.toml
    if [ -d .github/workflows ]; then uv run --only-group workflow zizmor .github/workflows; fi

test:
    cargo test -p oxirinth-engine
    uv run --no-dev --group tests pytest

test-live:
    uv run --no-dev --group tests pytest -m live -v

docs:
    uv run --only-group docs zensical serve -a localhost:4000

docs-build:
    uv run --only-group docs zensical build --clean --strict

build:
    uv build
