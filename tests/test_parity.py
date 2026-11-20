from __future__ import annotations

import inspect
from pathlib import Path

from pydantic import BaseModel
import pytest

import oxirinth
from oxirinth import (
    AsyncDataPack,
    AsyncDataPackSummary,
    AsyncMod,
    AsyncModpack,
    AsyncModpackSummary,
    AsyncModSummary,
    AsyncOxirinth,
    AsyncPlugin,
    AsyncPluginSummary,
    AsyncResourcePack,
    AsyncResourcePackSummary,
    AsyncServer,
    AsyncServerSummary,
    AsyncShader,
    AsyncShaderSummary,
    DataPack,
    DataPackSummary,
    Mod,
    Modpack,
    ModpackSummary,
    ModSummary,
    Oxirinth,
    Plugin,
    PluginSummary,
    ResourcePack,
    ResourcePackSummary,
    Server,
    ServerSummary,
    Shader,
    ShaderSummary,
    exceptions,
    models,
)


def _public_methods(cls: type[object]) -> dict[str, inspect.Signature]:
    methods: dict[str, inspect.Signature] = {}
    for name, member in inspect.getmembers(cls, predicate=inspect.isfunction):
        if name.startswith("_"):
            continue
        methods[name] = inspect.signature(member)
    return methods


def _normalize_params(sig: inspect.Signature) -> list[tuple[str, inspect._ParameterKind, object]]:
    return [(param.name, param.kind, param.default) for param in sig.parameters.values()]


def test_sync_and_async_client_signature_parity():
    assert _normalize_params(inspect.signature(Oxirinth.__init__)) == _normalize_params(
        inspect.signature(AsyncOxirinth.__init__),
    )

    sync_methods = _public_methods(Oxirinth)
    async_methods = _public_methods(AsyncOxirinth)

    assert "close" in sync_methods
    assert "aclose" in async_methods
    sync_comparable = {k: v for k, v in sync_methods.items() if k != "close"}
    async_comparable = {k: v for k, v in async_methods.items() if k != "aclose"}

    assert set(sync_comparable) == set(async_comparable)

    for method_name, sync_sig in sync_comparable.items():
        assert _normalize_params(sync_sig) == _normalize_params(async_comparable[method_name]), (
            f"Signature mismatch on client method '{method_name}'"
        )

    sync_only_helpers = {"cache_stats", "clear_cache"}
    for name in sync_comparable:
        async_fn = getattr(AsyncOxirinth, name)
        sync_fn = getattr(Oxirinth, name)
        assert not inspect.iscoroutinefunction(sync_fn)
        if name in sync_only_helpers:
            assert not inspect.iscoroutinefunction(async_fn)
        else:
            assert inspect.iscoroutinefunction(async_fn), f"AsyncOxirinth.{name} must be async"


def test_no_generic_project_or_download_methods_exist():
    forbidden = {
        "get_project",
        "get_projects",
        "search_projects",
        "download",
        "download_many",
        "download_latest",
        "download_modpack",
    }
    for cls in (Oxirinth, AsyncOxirinth, Mod, AsyncMod, Plugin, AsyncPlugin, Server, AsyncServer):
        public_names = {name for name in dir(cls) if not name.startswith("_")}
        assert public_names.isdisjoint(forbidden), f"Forbidden method found on {cls.__name__}"


@pytest.mark.parametrize(
    ("sync_cls", "async_cls"),
    [
        (Mod, AsyncMod),
        (Plugin, AsyncPlugin),
        (ResourcePack, AsyncResourcePack),
        (DataPack, AsyncDataPack),
        (Shader, AsyncShader),
        (Modpack, AsyncModpack),
        (Server, AsyncServer),
    ],
)
def test_sync_and_async_domain_model_parity(sync_cls: type[object], async_cls: type[object]):
    base_pydantic = set(dir(BaseModel))
    sync_own = {k: v for k, v in _public_methods(sync_cls).items() if k not in base_pydantic}
    async_own = {k: v for k, v in _public_methods(async_cls).items() if k not in base_pydantic}

    assert set(sync_own) == set(async_own)
    for name, sync_sig in sync_own.items():
        assert _normalize_params(sync_sig) == _normalize_params(async_own[name])
        assert not inspect.iscoroutinefunction(getattr(sync_cls, name))
        assert inspect.iscoroutinefunction(getattr(async_cls, name))


@pytest.mark.parametrize(
    ("sync_cls", "async_cls"),
    [
        (ModSummary, AsyncModSummary),
        (PluginSummary, AsyncPluginSummary),
        (ResourcePackSummary, AsyncResourcePackSummary),
        (DataPackSummary, AsyncDataPackSummary),
        (ShaderSummary, AsyncShaderSummary),
        (ModpackSummary, AsyncModpackSummary),
        (ServerSummary, AsyncServerSummary),
    ],
)
def test_sync_and_async_summary_model_parity(sync_cls: type[object], async_cls: type[object]):
    base_pydantic = set(dir(BaseModel))
    sync_own = {k: v for k, v in _public_methods(sync_cls).items() if k not in base_pydantic}
    async_own = {k: v for k, v in _public_methods(async_cls).items() if k not in base_pydantic}

    assert "fetch" in sync_own
    assert set(sync_own) == set(async_own)
    for name, sync_sig in sync_own.items():
        assert _normalize_params(sync_sig) == _normalize_params(async_own[name])
        assert not inspect.iscoroutinefunction(getattr(sync_cls, name))
        assert inspect.iscoroutinefunction(getattr(async_cls, name))


def test_strict_public_visibility_and_module_encapsulation():
    assert set(dir(oxirinth)) == set(oxirinth.__all__)
    assert set(dir(models)) == set(models.__all__)
    assert set(dir(exceptions)) == set(exceptions.__all__)

    forbidden_internals = {"OxirinthModel", "SyncCore", "AsyncCore", "_core", "BaseMod", "BaseHit"}
    for namespace_all in (oxirinth.__all__, models.__all__, exceptions.__all__):
        assert forbidden_internals.isdisjoint(namespace_all)
        assert not any(name.startswith("Base") for name in namespace_all)
        assert not any(name.startswith("_") and name != "__version__" for name in namespace_all)

    pkg_dir = Path(oxirinth.__file__).parent
    allowed_top_level = {"__init__.py", "exceptions.py", "models", "py.typed"}
    for entry in pkg_dir.iterdir():
        if entry.name.startswith("_"):
            continue
        assert entry.name in allowed_top_level, f"Unprefixed internal module exposed: {entry.name}"

    for entry in (pkg_dir / "models").iterdir():
        if entry.name.startswith("_"):
            continue
        assert entry.name == "__init__.py", f"Unprefixed internal model module exposed: models/{entry.name}"
