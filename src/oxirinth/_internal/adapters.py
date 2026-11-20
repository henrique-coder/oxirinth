"""Cached Pydantic v2 TypeAdapter validation over raw UTF-8 JSON bytes from Rust."""

from __future__ import annotations

from functools import lru_cache
from typing import TYPE_CHECKING, Any, TypeVar

from pydantic import TypeAdapter


if TYPE_CHECKING:
    from collections.abc import Sequence

    from oxirinth.models.domains import _BoundClientMixin
    from oxirinth.models.search import BaseHit, SearchResult


T = TypeVar("T")
HitT = TypeVar("HitT", bound="BaseHit")
BoundT = TypeVar("BoundT", bound="_BoundClientMixin")


@lru_cache(maxsize=64)
def get_adapter(target_type: Any) -> TypeAdapter[Any]:
    """Return a cached ``TypeAdapter`` for ``target_type``."""
    return TypeAdapter(target_type)


def validate_json_model(target_type: type[T], raw: bytes) -> T:
    """Validate raw UTF-8 JSON ``bytes`` directly into a Pydantic model in Rust."""
    return get_adapter(target_type).validate_json(raw)


def validate_search_result(hit_cls: type[HitT], raw: bytes) -> SearchResult[HitT]:
    """Validate a raw ``/v3/search`` JSON payload into ``SearchResult[hit_cls]``."""
    from oxirinth.models.search import SearchResult  # noqa: PLC0415

    generic_cls: Any = SearchResult
    return get_adapter(generic_cls[hit_cls]).validate_json(raw)


def validate_json_list(item_type: type[T], raw: bytes) -> list[T]:
    """Validate a raw JSON array payload directly into ``list[item_type]``."""
    list_cls: Any = list
    return get_adapter(list_cls[item_type]).validate_json(raw)


def validate_json_batch(model_cls: type[T], raw_items: Sequence[bytes]) -> list[T]:
    """Validate a sequence of raw JSON byte items directly into ``list[model_cls]``."""
    adapter = get_adapter(model_cls)
    return [adapter.validate_json(item) for item in raw_items]


def bind_client(model: BoundT, client: Any) -> BoundT:
    """Attach the originating client instance to a bound model via ``_client``."""
    model._client = client
    return model


def bind_client_many(models: list[BoundT], client: Any) -> list[BoundT]:
    """Attach the originating client instance to every bound model in ``models``."""
    for model in models:
        model._client = client
    return models
