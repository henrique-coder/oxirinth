"""Internal Rust extension module powered by oxirinth-engine."""

from __future__ import annotations

__all__ = ["__version__", "engine_version"]
__version__: str

def engine_version() -> str:
    """Return the compiled oxirinth-engine semantic version."""
