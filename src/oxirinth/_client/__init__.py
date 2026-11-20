"""Internal client package exposing ``Oxirinth`` and ``AsyncOxirinth``."""

from __future__ import annotations

from oxirinth._client._async import AsyncOxirinth
from oxirinth._client._sync import Oxirinth


__all__: list[str] = ["AsyncOxirinth", "Oxirinth"]
