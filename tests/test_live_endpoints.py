from __future__ import annotations

import asyncio
from datetime import datetime
from os import getenv
from pathlib import Path
from typing import Any

import pytest

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
    CacheConfig,
    CacheStats,
    CategoryTag,
    DependencyProject,
    GameVersionTag,
    LicenseTag,
    LoaderTag,
    Mod,
    ModSummary,
    Organization,
    Oxirinth,
    OxirinthNotFoundError,
    OxirinthTypeMismatchError,
    PlayerCount,
    Plugin,
    ProjectDependencies,
    SearchPage,
    Server,
    ServerSummary,
    TeamMember,
    User,
    Version,
    VersionFile,
)
from oxirinth.models import JavaServerInfo


pytestmark = pytest.mark.live


def _load_modrinth_api_key() -> str | None:
    env_value = getenv("MODRINTH_API_KEY", "").strip()
    if env_value:
        return env_value

    dotenv_path = Path(".env")
    if not dotenv_path.is_file():
        return None

    for raw_line in dotenv_path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, val = line.partition("=")
        if key.strip() == "MODRINTH_API_KEY":
            cleaned = val.strip().strip("\"'")
            return cleaned or None
    return None


@pytest.fixture(scope="module")
def live_api_key() -> str | None:
    return _load_modrinth_api_key()


def _assert_search_page(
    result: SearchPage[Any],
    summary_cls: type[Any],
    expected_ptype: str,
    *,
    min_items: int = 2,
) -> None:
    assert isinstance(result, SearchPage)
    assert result.total > 0
    assert result.page >= 1
    assert result.per_page >= len(result)
    assert len(result) >= min_items
    assert result.first is not None

    for item in result:
        assert isinstance(item, summary_cls)
        assert item.id
        assert item.slug
        assert item.name
        assert expected_ptype in item.project_types
        assert isinstance(item.categories, list)
        assert item.followers >= 0


def _assert_project_meta(
    model: Any,
    model_cls: type[Any],
    expected_ptype: str,
    expected_slug: str,
) -> None:
    assert isinstance(model, model_cls)
    assert model.id
    assert model.slug == expected_slug
    assert model.name
    assert expected_ptype in model.project_types
    assert "minecraft-java" in model.games or bool(model.games)
    assert isinstance(model.categories, list)
    assert model.status
    assert isinstance(model.published_at, datetime)
    assert isinstance(model.updated_at, datetime)


def _assert_version_valid(ver: Version, expected_project_id: str | None = None) -> None:
    assert isinstance(ver, Version)
    assert ver.id
    assert ver.project_id
    if expected_project_id is not None:
        assert ver.project_id == expected_project_id
    assert ver.name
    assert ver.version_number
    assert ver.version_type
    assert isinstance(ver.published_at, datetime)
    assert ver.downloads >= 0
    assert ver.files
    primary = ver.primary_file
    assert isinstance(primary, VersionFile)
    assert primary.filename
    assert primary.size > 0
    assert primary.url.startswith("https://cdn.modrinth.com/")


def _assert_tag_lists(
    loaders: list[LoaderTag],
    categories: list[CategoryTag],
    game_versions: list[GameVersionTag],
    licenses: list[LicenseTag],
) -> None:
    assert loaders
    assert all(isinstance(tag, LoaderTag) and tag.name and tag.supported_project_types for tag in loaders)
    assert "fabric" in {tag.name for tag in loaders}
    assert "paper" in {tag.name for tag in loaders}

    assert categories
    assert all(isinstance(tag, CategoryTag) and tag.name and tag.project_type for tag in categories)
    assert "optimization" in {tag.name for tag in categories}

    assert game_versions
    assert all(
        isinstance(tag, GameVersionTag) and tag.version and tag.version_type and isinstance(tag.released_at, datetime)
        for tag in game_versions
    )
    assert "1.21.1" in {tag.version for tag in game_versions}

    assert licenses
    assert all(isinstance(tag, LicenseTag) and tag.short and tag.name for tag in licenses)
    assert "MIT" in {tag.short for tag in licenses}


async def test_live_async_all_search_endpoints_and_filters(live_api_key: str | None, tmp_path: Path):
    cache = CacheConfig(cache_dir=tmp_path)
    async with AsyncOxirinth(api_key=live_api_key, cache=cache) as client:
        async with asyncio.TaskGroup() as tg:
            t_mods = tg.create_task(client.search_mods(sort="downloads", limit=5))
            t_plugins = tg.create_task(client.search_plugins(sort="downloads", limit=5))
            t_rpacks = tg.create_task(client.search_resourcepacks(sort="downloads", limit=5))
            t_dpacks = tg.create_task(client.search_datapacks(sort="downloads", limit=5))
            t_shaders = tg.create_task(client.search_shaders(sort="downloads", limit=5))
            t_modpacks = tg.create_task(client.search_modpacks(sort="downloads", limit=5))
            t_servers = tg.create_task(client.search_servers(sort="follows", limit=5))
            t_filtered = tg.create_task(
                client.search_mods(
                    "sodium",
                    loaders=["fabric"],
                    game_versions=["1.21.1"],
                    categories=["optimization"],
                    environment="client_only",
                    sort="downloads",
                    limit=5,
                    offset=0,
                ),
            )
            t_page0 = tg.create_task(client.search_mods(sort="downloads", limit=3, offset=0))
            t_page1 = tg.create_task(client.search_mods(sort="downloads", limit=3, offset=3))

        mods = t_mods.result()
        plugins = t_plugins.result()
        rpacks = t_rpacks.result()
        dpacks = t_dpacks.result()
        shaders = t_shaders.result()
        modpacks = t_modpacks.result()
        servers = t_servers.result()
        filtered_mods = t_filtered.result()

        _assert_search_page(mods, AsyncModSummary, "mod")
        assert all(item.downloads >= 0 and isinstance(item.loaders, list) for item in mods)

        _assert_search_page(plugins, AsyncPluginSummary, "plugin")
        assert all(item.downloads >= 0 and isinstance(item.loaders, list) for item in plugins)

        _assert_search_page(rpacks, AsyncResourcePackSummary, "resourcepack")
        assert all(item.downloads >= 0 for item in rpacks)

        _assert_search_page(dpacks, AsyncDataPackSummary, "datapack")
        assert all(item.downloads >= 0 for item in dpacks)

        _assert_search_page(shaders, AsyncShaderSummary, "shader")
        assert all(item.downloads >= 0 and isinstance(item.loaders, list) for item in shaders)

        _assert_search_page(modpacks, AsyncModpackSummary, "modpack")
        assert all(item.downloads >= 0 and isinstance(item.mod_loaders, list) for item in modpacks)

        _assert_search_page(servers, AsyncServerSummary, "minecraft_java_server")
        assert any(item.address for item in servers)
        assert any(isinstance(item.players, PlayerCount) and item.max_players is not None for item in servers)

        _assert_search_page(filtered_mods, AsyncModSummary, "mod", min_items=1)
        assert "sodium" in {item.slug for item in filtered_mods}
        for item in filtered_mods:
            assert "fabric" in item.loaders
            assert "1.21.1" in item.game_versions
            assert "client_only" in item.environment

        sodium_summary = filtered_mods.first
        assert sodium_summary is not None
        hydrated = await sodium_summary.fetch()
        assert isinstance(hydrated, AsyncMod)
        assert hydrated.slug == sodium_summary.slug

    ids_page0 = {item.id for item in t_page0.result()}
    ids_page1 = {item.id for item in t_page1.result()}
    assert len(ids_page0) == 3
    assert len(ids_page1) == 3
    assert ids_page0.isdisjoint(ids_page1)


async def test_live_async_all_domain_gets_batches_and_bound_helpers(live_api_key: str | None, tmp_path: Path):
    cache = CacheConfig(cache_dir=tmp_path)
    async with AsyncOxirinth(api_key=live_api_key, cache=cache) as client:
        async with asyncio.TaskGroup() as tg_search:
            ts_mod = tg_search.create_task(client.search_mods(sort="downloads", limit=2))
            ts_plg = tg_search.create_task(client.search_plugins(sort="downloads", limit=2))
            ts_rp = tg_search.create_task(client.search_resourcepacks(sort="downloads", limit=2))
            ts_dp = tg_search.create_task(client.search_datapacks(sort="downloads", limit=2))
            ts_shd = tg_search.create_task(client.search_shaders(sort="downloads", limit=2))
            ts_mp = tg_search.create_task(client.search_modpacks(sort="downloads", limit=2))
            ts_srv = tg_search.create_task(client.search_servers(sort="follows", limit=2))

        mod_slugs = [h.slug for h in ts_mod.result()[:2]]
        plg_slugs = [h.slug for h in ts_plg.result()[:2]]
        rp_slugs = [h.slug for h in ts_rp.result()[:2]]
        dp_slugs = [h.slug for h in ts_dp.result()[:2]]
        shd_slugs = [h.slug for h in ts_shd.result()[:2]]
        mp_slugs = [h.slug for h in ts_mp.result()[:2]]
        srv_slugs = [h.slug for h in ts_srv.result()[:2]]

        async with asyncio.TaskGroup() as tg_single:
            t_mod = tg_single.create_task(client.get_mod(mod_slugs[0]))
            t_plg = tg_single.create_task(client.get_plugin(plg_slugs[0]))
            t_rp = tg_single.create_task(client.get_resourcepack(rp_slugs[0]))
            t_dp = tg_single.create_task(client.get_datapack(dp_slugs[0]))
            t_shd = tg_single.create_task(client.get_shader(shd_slugs[0]))
            t_mp = tg_single.create_task(client.get_modpack(mp_slugs[0]))
            t_srv = tg_single.create_task(client.get_server(srv_slugs[0]))
            t_sodium = tg_single.create_task(client.get_mod("sodium"))
            t_luckperms = tg_single.create_task(client.get_plugin("luckperms"))
            t_iris = tg_single.create_task(client.get_mod("iris"))

        mod = t_mod.result()
        plg = t_plg.result()
        rp = t_rp.result()
        dp = t_dp.result()
        shd = t_shd.result()
        mp = t_mp.result()
        srv = t_srv.result()
        sodium = t_sodium.result()
        luckperms = t_luckperms.result()
        iris = t_iris.result()

        _assert_project_meta(mod, AsyncMod, "mod", mod_slugs[0])
        _assert_project_meta(plg, AsyncPlugin, "plugin", plg_slugs[0])
        _assert_project_meta(rp, AsyncResourcePack, "resourcepack", rp_slugs[0])
        _assert_project_meta(dp, AsyncDataPack, "datapack", dp_slugs[0])
        _assert_project_meta(shd, AsyncShader, "shader", shd_slugs[0])
        _assert_project_meta(mp, AsyncModpack, "modpack", mp_slugs[0])
        _assert_project_meta(srv, AsyncServer, "minecraft_java_server", srv_slugs[0])
        assert mod.downloads >= 0
        assert isinstance(srv.java, JavaServerInfo)
        assert isinstance(srv.players, PlayerCount)

        async with asyncio.TaskGroup() as tg_batch:
            tb_mods = tg_batch.create_task(client.get_mods(mod_slugs))
            tb_plgs = tg_batch.create_task(client.get_plugins(plg_slugs))
            tb_rps = tg_batch.create_task(client.get_resourcepacks(rp_slugs))
            tb_dps = tg_batch.create_task(client.get_datapacks(dp_slugs))
            tb_shds = tg_batch.create_task(client.get_shaders(shd_slugs))
            tb_mps = tg_batch.create_task(client.get_modpacks(mp_slugs))
            tb_srvs = tg_batch.create_task(client.get_servers(srv_slugs))

        assert [x.slug for x in tb_mods.result()] == mod_slugs
        assert [x.slug for x in tb_plgs.result()] == plg_slugs
        assert [x.slug for x in tb_rps.result()] == rp_slugs
        assert [x.slug for x in tb_dps.result()] == dp_slugs
        assert [x.slug for x in tb_shds.result()] == shd_slugs
        assert [x.slug for x in tb_mps.result()] == mp_slugs
        assert [x.slug for x in tb_srvs.result()] == srv_slugs
        assert all(isinstance(x, AsyncModpack) for x in tb_mps.result())
        assert all(isinstance(x, AsyncServer) for x in tb_srvs.result())

        async with asyncio.TaskGroup() as tg_helpers:
            th_s_vers = tg_helpers.create_task(sodium.get_versions(loaders=["fabric"], game_versions=["1.21.1"]))
            th_s_lat = tg_helpers.create_task(sodium.get_latest_version(loaders=["fabric"]))
            th_s_mem = tg_helpers.create_task(sodium.get_members())
            th_i_dep = tg_helpers.create_task(iris.get_dependencies())
            th_lp_lat = tg_helpers.create_task(luckperms.get_latest_version(loaders=["paper"]))
            th_rp_lat = tg_helpers.create_task(rp.get_latest_version())
            th_dp_lat = tg_helpers.create_task(dp.get_latest_version())
            th_shd_lat = tg_helpers.create_task(shd.get_latest_version())
            th_mp_lat = tg_helpers.create_task(mp.get_latest_version())
            th_srv_mem = tg_helpers.create_task(srv.get_members())

    sodium_vers = th_s_vers.result()
    assert sodium_vers
    assert all("fabric" in v.loaders and "1.21.1" in v.game_versions for v in sodium_vers)
    sodium_latest = th_s_lat.result()
    assert sodium_latest is not None
    _assert_version_valid(sodium_latest, sodium.id)

    sodium_members = th_s_mem.result()
    assert sodium_members
    assert all(isinstance(m, TeamMember) and m.team_id and m.user.username for m in sodium_members)

    iris_deps = th_i_dep.result()
    assert isinstance(iris_deps, ProjectDependencies)
    assert iris_deps.projects
    assert all(isinstance(p, DependencyProject) and p.id and p.slug for p in iris_deps.projects)
    assert iris_deps.versions

    lp_latest = th_lp_lat.result()
    assert lp_latest is not None
    assert "paper" in lp_latest.loaders
    _assert_version_valid(lp_latest, luckperms.id)

    for item_model, item_latest in (
        (rp, th_rp_lat.result()),
        (dp, th_dp_lat.result()),
        (shd, th_shd_lat.result()),
        (mp, th_mp_lat.result()),
    ):
        assert item_latest is not None
        _assert_version_valid(item_latest, item_model.id)

    srv_members = th_srv_mem.result()
    assert isinstance(srv_members, list)
    assert all(isinstance(m, TeamMember) and m.user.username for m in srv_members)


async def test_live_async_versions_hashes_social_and_tags(live_api_key: str | None, tmp_path: Path):
    cache = CacheConfig(cache_dir=tmp_path)
    async with AsyncOxirinth(api_key=live_api_key, cache=cache) as client:
        sodium = await client.get_mod("sodium")
        async with asyncio.TaskGroup() as tg_seed:
            t_vers = tg_seed.create_task(sodium.get_versions(loaders=["fabric"]))
            t_mems = tg_seed.create_task(sodium.get_members())

        versions = t_vers.result()
        members = t_mems.result()
        assert len(versions) >= 2
        assert members

        ver_a, ver_b = versions[0], versions[1]
        hashed_file = next(
            (f for f in ver_a.files if f.sha512 and f.sha1),
            None,
        )
        assert hashed_file is not None
        sha512_val = hashed_file.sha512
        sha1_val = hashed_file.sha1
        assert sha512_val is not None
        assert len(sha512_val) == 128
        assert sha1_val is not None
        assert len(sha1_val) == 40

        target_member = members[0]
        org_slug_or_id = sodium.organization or "caffeinemc"

        async with asyncio.TaskGroup() as tg_all:
            t_single_ver = tg_all.create_task(client.get_version(ver_a.id))
            t_batch_vers = tg_all.create_task(client.get_versions([ver_a.id, ver_b.id]))
            t_by_sha512 = tg_all.create_task(client.get_version_by_hash(sha512_val, algorithm="sha512"))
            t_by_sha1 = tg_all.create_task(client.get_version_by_hash(sha1_val, algorithm="sha1"))
            t_user = tg_all.create_task(client.get_user(target_member.user.username))
            t_org = tg_all.create_task(client.get_organization(org_slug_or_id))
            t_loaders = tg_all.create_task(client.get_loaders())
            t_categories = tg_all.create_task(client.get_categories())
            t_gvs = tg_all.create_task(client.get_game_versions())
            t_licenses = tg_all.create_task(client.get_licenses())

    single_ver = t_single_ver.result()
    assert single_ver.id == ver_a.id
    _assert_version_valid(single_ver, sodium.id)

    assert [v.id for v in t_batch_vers.result()] == [ver_a.id, ver_b.id]
    assert t_by_sha512.result().id == ver_a.id
    assert t_by_sha1.result().id == ver_a.id

    user = t_user.result()
    assert isinstance(user, User)
    assert user.id == target_member.user.id
    assert user.username.lower() == target_member.user.username.lower()
    assert user.role

    org = t_org.result()
    assert isinstance(org, Organization)
    assert org.id
    assert org.slug
    assert org.name
    assert org.team_id
    assert org.members

    _assert_tag_lists(t_loaders.result(), t_categories.result(), t_gvs.result(), t_licenses.result())


async def test_live_errors_singleflight_and_warm_cache(live_api_key: str | None, tmp_path: Path):
    cache = CacheConfig(cache_dir=tmp_path)
    async with AsyncOxirinth(api_key=live_api_key, cache=cache) as client:
        with pytest.raises(OxirinthTypeMismatchError) as mismatch_exc:
            await client.get_plugin("sodium")
        assert mismatch_exc.value.expected == "plugin"
        assert "mod" in mismatch_exc.value.actual
        assert mismatch_exc.value.identifier == "sodium"

        with pytest.raises(OxirinthNotFoundError) as not_found_exc:
            await client.get_mod("oxirinth-nonexistent-project-slug-999999")
        assert not_found_exc.value.status_code == 404
        assert not_found_exc.value.error_code == "not_found"

        misses_before_burst = client.cache_stats().misses
        burst_mods = await asyncio.gather(*[client.get_mod("fabric-api") for _ in range(12)])
        assert len(burst_mods) == 12
        assert all(m.slug == "fabric-api" and m.id == burst_mods[0].id for m in burst_mods)
        assert client.cache_stats().misses == misses_before_burst + 1

        await client.get_loaders()
        cold_stats = client.cache_stats()

        async with asyncio.TaskGroup() as tg_warm:
            tw_mod = tg_warm.create_task(client.get_mod("fabric-api"))
            tw_loaders = tg_warm.create_task(client.get_loaders())

        assert tw_mod.result().slug == "fabric-api"
        assert tw_loaders.result()

        warm_stats = client.cache_stats()
        assert isinstance(warm_stats, CacheStats)
        assert warm_stats.backend == "redb"
        assert warm_stats.entries >= 2
        assert warm_stats.size_bytes > 0
        assert warm_stats.hits >= cold_stats.hits + 2
        assert warm_stats.misses == cold_stats.misses

        client.clear_cache()
        assert client.cache_stats().entries == 0


def test_live_sync_client_full_surface(live_api_key: str | None, tmp_path: Path):
    cache = CacheConfig(cache_dir=tmp_path)
    with Oxirinth(api_key=live_api_key, cache=cache) as client:
        mods_search = client.search_mods("sodium", sort="downloads", limit=2)
        srv_search = client.search_servers(sort="follows", limit=2)
        _assert_search_page(mods_search, ModSummary, "mod")
        _assert_search_page(srv_search, ServerSummary, "minecraft_java_server")

        first_mod = mods_search.first
        assert first_mod is not None
        assert first_mod.fetch().slug == first_mod.slug

        sodium = client.get_mod("sodium")
        assert isinstance(sodium, Mod)
        assert sodium.slug == "sodium"

        batch_mods = client.get_mods(["sodium", "fabric-api"])
        assert [m.slug for m in batch_mods] == ["sodium", "fabric-api"]

        plugin = client.get_plugin("luckperms")
        assert isinstance(plugin, Plugin)

        srv_summary = srv_search[0]
        assert isinstance(srv_summary.players, PlayerCount)
        server = client.get_server(srv_summary.slug)
        assert isinstance(server, Server)
        assert server.slug == srv_summary.slug
        assert isinstance(server.get_members(), list)

        latest = sodium.get_latest_version(loaders=["fabric"])
        assert latest is not None
        _assert_version_valid(latest, sodium.id)
        assert sodium.get_members()

        iris = client.get_mod("iris")
        assert iris.get_dependencies().projects

        ver = client.get_version(latest.id)
        assert ver.id == latest.id
        assert [v.id for v in client.get_versions([latest.id])] == [latest.id]

        primary = latest.primary_file
        assert primary is not None
        assert primary.sha512 is not None
        assert client.get_version_by_hash(primary.sha512, algorithm="sha512").id == latest.id

        assert client.get_user("jellysquid3").username.lower() == "jellysquid3"
        assert client.get_organization(sodium.organization or "caffeinemc").slug == "caffeinemc"

        _assert_tag_lists(
            client.get_loaders(),
            client.get_categories(),
            client.get_game_versions(),
            client.get_licenses(),
        )

        stats = client.cache_stats()
        assert stats.backend == "redb"
        assert stats.entries > 0
        client.clear_cache()
        assert client.cache_stats().entries == 0
