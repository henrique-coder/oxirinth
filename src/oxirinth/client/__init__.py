"""Synchronous and asynchronous Modrinth API v3 clients."""

from __future__ import annotations

from oxirinth.client.async_ import AsyncOxirinth
from oxirinth.client.sync import Oxirinth


__all__: list[str] = ["AsyncOxirinth", "Oxirinth"]
