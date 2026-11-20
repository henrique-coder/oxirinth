"""Oxirinth — High-performance Python SDK for the Modrinth API v3."""

from __future__ import annotations

from oxirinth._core import __version__, engine_version


__all__: list[str] = ["__version__", "engine_version"]
