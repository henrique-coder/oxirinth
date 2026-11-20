"""Public exception hierarchy for the Oxirinth SDK."""

from __future__ import annotations

from oxirinth._core import (
    OxirinthAPIError,
    OxirinthAuthError,
    OxirinthCacheError,
    OxirinthConfigError,
    OxirinthError,
    OxirinthInternalError,
    OxirinthNetworkError,
    OxirinthNotFoundError,
    OxirinthRateLimitError,
    OxirinthTypeMismatchError,
    OxirinthUnboundModelError,
)


__all__: list[str] = [
    "OxirinthAPIError",
    "OxirinthAuthError",
    "OxirinthCacheError",
    "OxirinthConfigError",
    "OxirinthError",
    "OxirinthInternalError",
    "OxirinthNetworkError",
    "OxirinthNotFoundError",
    "OxirinthRateLimitError",
    "OxirinthTypeMismatchError",
    "OxirinthUnboundModelError",
]


def __dir__() -> list[str]:
    """Expose only public exception classes to IDEs and introspection."""
    return sorted(__all__)
