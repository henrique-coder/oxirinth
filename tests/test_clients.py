from __future__ import annotations

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from threading import Thread
from typing import TYPE_CHECKING, Any
from urllib.parse import parse_qs, urlparse

import pytest

from oxirinth import (
    AsyncOxirinth,
    AsyncPlugin,
    AsyncServer,
    CacheConfig,
    DataPack,
    Mod,
    Modpack,
    Oxirinth,
    OxirinthNotFoundError,
    OxirinthTypeMismatchError,
    Plugin,
    ResourcePack,
    Server,
    Shader,
)


if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path


def _make_project(pid: str, slug: str, name: str, types: list[str], **extra: Any) -> dict[str, Any]:
    base: dict[str, Any] = {
        "id": pid,
        "slug": slug,
        "project_types": types,
        "games": ["minecraft-java"],
        "team_id": "TEAM1",
        "organization": "ORG1",
        "name": name,
        "summary": f"{name} summary",
        "description": f"{name} description",
        "published": "2024-01-01T00:00:00Z",
        "updated": "2026-11-01T00:00:00Z",
        "status": "approved",
        "license": {"id": "MIT", "name": "MIT License", "url": None},
        "downloads": 1000,
        "followers": 50,
        "categories": ["utility"],
        "loaders": ["fabric"],
        "versions": ["VER1"],
        "game_versions": ["1.21.1"],
        "environment": ["client_and_server"],
        "mrpack_loaders": [],
        "link_urls": {},
        "gallery": [],
    }
    base.update(extra)
    return base


PROJECTS: dict[str, dict[str, Any]] = {
    "sodium": _make_project("AANobbMI", "sodium", "Sodium", ["mod"]),
    "luckperms": _make_project("Vebnzrzj", "luckperms", "LuckPerms", ["mod", "plugin"], loaders=["paper"]),
    "fresh-animations": _make_project("1bokaNcj", "fresh-animations", "Fresh Animations", ["resourcepack"]),
    "terralith": _make_project("8oi3bsk5", "terralith", "Terralith", ["datapack", "mod"]),
    "complementary": _make_project("HVnmMxH1", "complementary", "Complementary", ["shader"], loaders=["iris"]),
    "fabulously-optimized": _make_project(
        "1KVo5zza",
        "fabulously-optimized",
        "Fabulously Optimized",
        ["modpack"],
        mrpack_loaders=["fabric"],
    ),
    "aero-server": _make_project(
        "OgcNPs6n",
        "aero-server",
        "Aero SMP",
        ["minecraft_java_server"],
        mrpack_loaders=["neoforge"],
        minecraft_server={"max_players": 300, "region": "europe", "languages": ["en"]},
        minecraft_java_server={
            "address": "play.aerosmp.com",
            "ping": {
                "when": "2026-11-20T03:00:00Z",
                "address": "play.aerosmp.com",
                "data": {"players_online": 27, "players_max": 300},
            },
        },
    ),
}

VERSION_PAYLOAD = {
    "id": "VER1",
    "project_id": "Vebnzrzj",
    "author_id": "USR1",
    "featured": True,
    "name": "LuckPerms v5.4",
    "version_number": "5.4.145",
    "project_types": ["plugin"],
    "games": ["minecraft-java"],
    "date_published": "2026-10-01T12:00:00Z",
    "downloads": 5000,
    "version_type": "release",
    "status": "listed",
    "files": [
        {
            "id": "FIL1",
            "hashes": {"sha1": "deadbeef", "sha512": "cafebabe" * 16},
            "url": "https://cdn.modrinth.com/luckperms.jar",
            "filename": "LuckPerms-Bukkit-5.4.145.jar",
            "primary": True,
            "size": 1500000,
        }
    ],
    "dependencies": [],
    "loaders": ["paper"],
    "game_versions": ["1.21.1"],
    "environment": "server_only",
}


STATIC_ROUTES: dict[str, Any] = {
    "/v3/version/VER1": VERSION_PAYLOAD,
    "/v3/versions": [VERSION_PAYLOAD],
    "/v3/user/lucko": {"id": "USR1", "username": "lucko", "role": "developer"},
    "/v3/organization/caffeinemc": {"id": "ORG1", "slug": "caffeinemc", "name": "CaffeineMC", "team_id": "T1"},
    "/v3/tag/loader": [{"name": "paper", "supported_project_types": ["plugin"]}],
    "/v3/tag/category": [{"name": "optimization", "project_type": "mod", "header": "categories"}],
    "/v2/tag/game_version": [{"version": "1.21.1", "version_type": "release", "date": "2024-08-08T00:00:00Z"}],
    "/v2/tag/license": [{"short": "MIT", "name": "MIT License"}],
}


def _resolve_mock_route(path: str, qs: dict[str, list[str]]) -> tuple[int, Any]:
    if path in STATIC_ROUTES:
        return 200, STATIC_ROUTES[path]

    parts = [seg for seg in path.split("/") if seg]
    result: tuple[int, Any] | None = None

    if path == "/v3/search":
        facets = json.loads(qs.get("facets", ["[]"])[0])
        ptype = facets[0][0].split(":", 1)[1] if facets and facets[0] else "mod"
        hits = [
            {
                "project_id": p["id"],
                "slug": p["slug"],
                "name": p["name"],
                "project_types": p["project_types"],
                "loaders": p["loaders"],
                "mrpack_loaders": p["mrpack_loaders"],
                "game_versions": p["game_versions"],
                "downloads": p["downloads"],
                "minecraft_server": p.get("minecraft_server"),
                "minecraft_java_server": p.get("minecraft_java_server"),
            }
            for p in PROJECTS.values()
            if ptype in p["project_types"]
        ]
        result = (200, {"hits": hits, "page": 1, "hits_per_page": 10, "total_hits": len(hits)})
    elif path == "/v3/projects":
        ids = json.loads(qs.get("ids", ["[]"])[0])
        result = (200, [p for p in PROJECTS.values() if p["slug"] in ids or p["id"] in ids])
    elif len(parts) == 3 and parts[:2] == ["v3", "project"]:
        proj = next((p for p in PROJECTS.values() if p["slug"] == parts[2] or p["id"] == parts[2]), None)
        if proj is not None:
            result = (200, proj)
    elif len(parts) == 4 and parts[:2] == ["v3", "project"]:
        subroute_payloads: dict[str, Any] = {
            "version": [VERSION_PAYLOAD],
            "members": [{"team_id": "TEAM1", "user": {"id": "USR1", "username": "lucko"}, "role": "Owner"}],
            "dependencies": {"projects": [], "versions": [VERSION_PAYLOAD]},
        }
        if parts[3] in subroute_payloads:
            result = (200, subroute_payloads[parts[3]])
    elif parts[:2] == ["v3", "version_file"] and len(parts) == 3:
        result = (200, VERSION_PAYLOAD)

    return result if result is not None else (404, {"error": "not_found", "description": "Not found"})


@pytest.fixture
def mock_api_url() -> Iterator[str]:
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, format: str, *args: object) -> None:
            return

        def do_GET(self) -> None:
            parsed = urlparse(self.path)
            qs = parse_qs(parsed.query)
            status, payload = _resolve_mock_route(parsed.path, qs)
            raw = json.dumps(payload).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(raw)))
            self.end_headers()
            self.wfile.write(raw)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}"
    finally:
        server.shutdown()
        server.server_close()


def test_sync_client_all_domains_and_helpers(mock_api_url: str, tmp_path: Path):
    with Oxirinth(base_url=mock_api_url, cache=CacheConfig(cache_dir=tmp_path)) as client:
        assert isinstance(client.get_mod("sodium"), Mod)
        assert isinstance(client.get_plugin("luckperms"), Plugin)
        assert isinstance(client.get_resourcepack("fresh-animations"), ResourcePack)
        assert isinstance(client.get_datapack("terralith"), DataPack)
        assert isinstance(client.get_shader("complementary"), Shader)
        assert isinstance(client.get_modpack("fabulously-optimized"), Modpack)

        server = client.get_server("aero-server")
        assert isinstance(server, Server)
        assert server.address == "play.aerosmp.com"
        assert server.players_online == 27
        assert server.get_members()[0].user.username == "lucko"

        assert [m.slug for m in client.get_mods(["sodium", "terralith"])] == ["sodium", "terralith"]
        assert client.get_plugins(["luckperms"])[0].slug == "luckperms"
        assert client.get_resourcepacks(["fresh-animations"])[0].slug == "fresh-animations"
        assert client.get_datapacks(["terralith"])[0].slug == "terralith"
        assert client.get_shaders(["complementary"])[0].slug == "complementary"
        assert client.get_modpacks(["fabulously-optimized"])[0].slug == "fabulously-optimized"
        assert client.get_servers(["aero-server"])[0].slug == "aero-server"

        assert client.search_mods().hits[0].slug == "sodium"
        assert client.search_plugins().hits[0].slug == "luckperms"
        assert client.search_resourcepacks().hits[0].slug == "fresh-animations"
        assert client.search_datapacks().hits[0].slug == "terralith"
        assert client.search_shaders().hits[0].slug == "complementary"
        assert client.search_modpacks().hits[0].mod_loaders == ["fabric"]
        assert client.search_servers().hits[0].address == "play.aerosmp.com"

        plugin = client.get_plugin("luckperms")
        latest = plugin.get_latest_version(loaders=["paper"])
        assert latest is not None
        assert latest.primary_file is not None
        assert latest.primary_file.filename == "LuckPerms-Bukkit-5.4.145.jar"
        assert plugin.get_dependencies().versions[0].id == "VER1"

        assert client.get_version("VER1").id == "VER1"
        assert client.get_versions(["VER1"])[0].version_number == "5.4.145"
        assert client.get_version_by_hash("deadbeef", algorithm="sha1").id == "VER1"

        assert client.get_user("lucko").username == "lucko"
        assert client.get_organization("caffeinemc").name == "CaffeineMC"
        assert client.get_loaders()[0].name == "paper"
        assert client.get_categories()[0].name == "optimization"
        assert client.get_game_versions()[0].version == "1.21.1"
        assert client.get_licenses()[0].short == "MIT"

        with pytest.raises(OxirinthTypeMismatchError):
            client.get_plugin("sodium")

        with pytest.raises(OxirinthNotFoundError):
            client.get_mod("nonexistent")

        assert client.cache_stats().hits >= 1
        client.clear_cache()
        assert client.cache_stats().entries == 0


async def test_async_client_all_domains_and_helpers(mock_api_url: str, tmp_path: Path):
    async with AsyncOxirinth(base_url=mock_api_url, cache=CacheConfig(cache_dir=tmp_path)) as client:
        plugin = await client.get_plugin("luckperms")
        assert isinstance(plugin, AsyncPlugin)

        latest = await plugin.get_latest_version(loaders=["paper"])
        assert latest is not None
        assert latest.id == "VER1"
        assert (await plugin.get_members())[0].user.username == "lucko"
        assert (await plugin.get_dependencies()).versions[0].id == "VER1"

        server = await client.get_server("aero-server")
        assert isinstance(server, AsyncServer)
        assert server.address == "play.aerosmp.com"
        assert (await server.get_members())[0].role == "Owner"

        assert (await client.search_mods()).hits[0].slug == "sodium"
        assert (await client.search_servers()).hits[0].players_online == 27
        assert [m.slug for m in await client.get_mods(["sodium", "luckperms"])] == ["sodium", "luckperms"]
        assert (await client.get_version("VER1")).id == "VER1"
        assert (await client.get_versions(["VER1"]))[0].id == "VER1"
        assert (await client.get_version_by_hash("deadbeef", algorithm="sha1")).id == "VER1"
        assert (await client.get_user("lucko")).id == "USR1"
        assert (await client.get_organization("caffeinemc")).slug == "caffeinemc"
        assert (await client.get_loaders())[0].name == "paper"
        assert (await client.get_categories())[0].name == "optimization"
        assert (await client.get_game_versions())[0].version == "1.21.1"
        assert (await client.get_licenses())[0].short == "MIT"

        assert client.cache_stats().backend == "redb"
        client.clear_cache()
        assert client.cache_stats().entries == 0
