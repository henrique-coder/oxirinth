# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0).

## [0.1.0] - Unreleased

### Added

- Internal 2-crate Cargo workspace (`oxirinth-engine` + PyO3 `oxirinth._core`) built with Maturin.
- Verified Modrinth API v3 endpoint reference and `/v2/tag/*` fallback tracker (`docs/api/v3-endpoints.md`).
- Repository tooling, quality gates (`ruff`, `ty`, `stubtest`, `clippy`, `prettier`, `taplo`), and Zensical documentation setup.
