from __future__ import annotations

import json

import pytest

from oxirinth import (
    DEFAULT_USER_AGENT,
    AsyncMod,
    AsyncModSummary,
    AsyncPlugin,
    AsyncServer,
    DataPack,
    GameVersionTag,
    Mod,
    Modpack,
    ModSummary,
    OxirinthConfigError,
    OxirinthUnboundModelError,
    PlayerCount,
    Plugin,
    ResourcePack,
    SearchPage,
    Server,
    ServerSummary,
    Shader,
    Version,
    build_user_agent,
)
from oxirinth._internal.adapters import validate_json_model, validate_search_page


def _project_payload(
    *,
    pid: str = "AANobbMI",
    slug: str = "sodium",
    name: str = "Sodium",
    project_types: list[str] | None = None,
    loaders: list[str] | None = None,
    mrpack_loaders: list[str] | None = None,
) -> bytes:
    payload = {
        "id": pid,
        "slug": slug,
        "project_types": project_types or ["mod"],
        "games": ["minecraft-java"],
        "team_id": "4reLOAKe",
        "organization": "LjcZDkRW",
        "name": name,
        "summary": "Summary",
        "description": "Description",
        "published": "2021-01-03T00:53:34.185936Z",
        "updated": "2026-11-12T01:12:22.856079Z",
        "approved": "2021-01-03T01:00:00Z",
        "status": "approved",
        "license": {"id": "MIT", "name": "MIT License", "url": None},
        "downloads": 12345,
        "followers": 678,
        "categories": ["optimization"],
        "additional_categories": [],
        "loaders": loaders or ["fabric"],
        "versions": ["v1", "v2"],
        "game_versions": ["1.21.1"],
        "environment": ["client_only"],
        "mrpack_loaders": mrpack_loaders or [],
        "minecraft_server": {
            "max_players": 300,
            "country": "DE",
            "region": "europe",
            "languages": ["en"],
            "active_version": None,
        },
        "minecraft_java_server": {
            "address": "play.aerosmp.com",
            "content": {
                "kind": "modpack",
                "version_id": "GDELFL7Y",
                "project_id": "OgcNPs6n",
                "project_name": "Aero SMP",
                "project_icon": None,
            },
            "ping": {
                "when": "2026-11-20T03:00:38.011606624Z",
                "address": "play.aerosmp.com",
                "data": {
                    "latency": {"secs": 0, "nanos": 519_025_823},
                    "version_name": "Velocity 1.21.1",
                    "version_protocol": 767,
                    "description": "Welcome",
                    "players_online": 27,
                    "players_max": 300,
                },
            },
            "verified_plays_2w": 4021,
            "verified_plays_4w": 7946,
        },
        "minecraft_bedrock_server": {"address": "bedrock.aerosmp.com"},
        "link_urls": {"source": {"platform": "source", "donation": False, "url": "https://github.com/test"}},
        "gallery": [],
        "color": 8703084,
    }
    return json.dumps(payload).encode("utf-8")


def test_domain_field_isolation_and_bytes_validation():
    mod = validate_json_model(Mod, _project_payload(project_types=["mod"]))
    plugin = validate_json_model(Plugin, _project_payload(pid="Vebnzrzj", slug="luckperms", project_types=["plugin"]))
    rp = validate_json_model(ResourcePack, _project_payload(project_types=["resourcepack"]))
    dp = validate_json_model(DataPack, _project_payload(project_types=["datapack"]))
    shader = validate_json_model(Shader, _project_payload(project_types=["shader"], loaders=["iris"]))
    modpack = validate_json_model(
        Modpack,
        _project_payload(project_types=["modpack"], mrpack_loaders=["fabric"]),
    )
    server = validate_json_model(
        Server,
        _project_payload(
            pid="OgcNPs6n",
            slug="aero-server",
            name="Aero SMP",
            project_types=["minecraft_java_server"],
            mrpack_loaders=["neoforge"],
        ),
    )

    assert mod.loaders == ["fabric"]
    assert mod.published_at is not None
    assert plugin.downloads == 12345
    assert rp.game_versions == ["1.21.1"]
    assert dp.downloads == 12345
    assert shader.loaders == ["iris"]
    assert modpack.mod_loaders == ["fabric"]

    # Irrelevant cross-domain fields must not leak onto specialized models.
    assert "minecraft_java_server" not in Mod.model_fields
    assert "minecraft_java_server" not in Plugin.model_fields
    assert "mrpack_loaders" not in Mod.model_fields
    assert "loaders" not in ResourcePack.model_fields
    assert "environment" not in ResourcePack.model_fields
    assert "downloads" not in Server.model_fields

    # Server convenience properties & nested telemetry.
    assert server.address == "play.aerosmp.com"
    assert server.bedrock_address == "bedrock.aerosmp.com"
    assert server.online_players == 27
    assert server.max_players == 300
    assert server.players == PlayerCount(online=27, max=300)
    assert str(server.players) == "27/300"
    assert server.latency_ms == pytest.approx(519.025823)
    assert server.is_online is True
    assert server.region == "europe"
    assert server.country == "DE"
    assert server.languages == ["en"]
    assert server.mod_loaders == ["neoforge"]
    assert server.ping is not None
    assert server.ping.checked_at is not None
    assert server.content is not None
    assert server.content.kind == "modpack"

    # Private client attribute must not pollute serialization.
    dumped = mod.model_dump()
    assert "_client" not in dumped


def test_unbound_models_and_summaries_raise_clean_error():
    mod = validate_json_model(Mod, _project_payload())
    server = validate_json_model(Server, _project_payload(project_types=["minecraft_java_server"]))
    async_mod = validate_json_model(AsyncMod, _project_payload())
    async_server = validate_json_model(AsyncServer, _project_payload(project_types=["minecraft_java_server"]))
    mod_summary = validate_json_model(ModSummary, b'{"project_id":"AANobbMI","slug":"sodium","name":"Sodium"}')

    assert isinstance(async_mod, AsyncMod)
    assert isinstance(async_server, AsyncServer)

    with pytest.raises(OxirinthUnboundModelError):
        mod.get_versions()

    with pytest.raises(OxirinthUnboundModelError):
        server.get_members()

    with pytest.raises(OxirinthUnboundModelError):
        mod_summary.fetch()


async def test_unbound_async_models_raise_clean_error():
    async_plugin = validate_json_model(AsyncPlugin, _project_payload(project_types=["plugin"]))
    async_summary = validate_json_model(AsyncModSummary, b'{"project_id":"AANobbMI","slug":"sodium","name":"Sodium"}')

    with pytest.raises(OxirinthUnboundModelError):
        await async_plugin.get_dependencies()

    with pytest.raises(OxirinthUnboundModelError):
        await async_summary.fetch()


def test_search_page_iteration_and_version_file_helpers():
    search_bytes = json.dumps(
        {
            "hits": [
                {
                    "project_id": "AANobbMI",
                    "version_id": "v1",
                    "slug": "sodium",
                    "name": "Sodium",
                    "summary": "Fast rendering",
                    "author": "jellysquid3",
                    "loaders": ["fabric"],
                    "environment": ["client_only"],
                    "game_versions": ["1.21.1"],
                    "downloads": 1000,
                    "follows": 50,
                }
            ],
            "page": 1,
            "hits_per_page": 10,
            "total_hits": 25,
        }
    ).encode("utf-8")

    result = validate_search_page(ModSummary, search_bytes)
    assert isinstance(result, SearchPage)
    assert result.total == 25
    assert result.per_page == 10
    assert result.page == 1
    assert result.has_next is True
    assert len(result) == 1
    assert bool(result) is True
    assert isinstance(result[0], ModSummary)
    assert result.first == result[0]
    assert list(result) == [result[0]]
    assert result[:1] == [result[0]]
    assert result[0].id == "AANobbMI"
    assert result[0].latest_version_id == "v1"
    assert result[0].followers == 50
    assert result[0].loaders == ["fabric"]

    server_search_bytes = json.dumps(
        {
            "hits": [
                {
                    "project_id": "OgcNPs6n",
                    "slug": "aero-server",
                    "name": "Aero SMP",
                    "mrpack_loaders": ["neoforge"],
                    "minecraft_server": {"max_players": 300, "region": "europe", "languages": ["en"]},
                    "minecraft_java_server": {
                        "address": "play.aerosmp.com",
                        "ping": {
                            "when": "2026-11-20T03:00:00Z",
                            "address": "play.aerosmp.com",
                            "data": {"players_online": 27, "players_max": 300},
                        },
                    },
                }
            ],
            "page": 1,
            "hits_per_page": 1,
            "total_hits": 1,
        }
    ).encode("utf-8")
    srv_page = validate_search_page(ServerSummary, server_search_bytes)
    assert srv_page.has_next is False
    assert isinstance(srv_page[0], ServerSummary)
    assert srv_page[0].id == "OgcNPs6n"
    assert srv_page[0].address == "play.aerosmp.com"
    assert srv_page[0].mod_loaders == ["neoforge"]
    assert srv_page[0].online_players == 27
    assert srv_page[0].max_players == 300
    assert srv_page[0].players == PlayerCount(online=27, max=300)
    assert srv_page[0].region == "europe"
    assert srv_page[0].is_online is True

    version = validate_json_model(
        Version,
        json.dumps(
            {
                "id": "V1",
                "project_id": "AANobbMI",
                "name": "0.6.0",
                "version_number": "0.6.0",
                "version_type": "release",
                "date_published": "2026-11-12T01:12:30.162066Z",
                "files": [
                    {
                        "hashes": {"sha1": "abc", "sha512": "def"},
                        "url": "https://cdn.modrinth.com/a.jar",
                        "filename": "a-sources.jar",
                        "primary": False,
                        "size": 10,
                    },
                    {
                        "hashes": {"sha1": "111", "sha512": "222"},
                        "url": "https://cdn.modrinth.com/b.jar",
                        "filename": "b.jar",
                        "primary": True,
                        "size": 20,
                    },
                ],
            }
        ).encode("utf-8"),
    )
    assert version.published_at is not None
    assert version.primary_file is not None
    assert version.primary_file.filename == "b.jar"
    assert version.primary_file.sha512 == "222"
    assert version.primary_file.sha1 == "111"


def test_user_agent_defaults_and_builder():
    assert DEFAULT_USER_AGENT == "henrique-coder/oxirinth/0.1.0 (https://github.com/henrique-coder/oxirinth)"
    assert (
        build_user_agent("my-launcher", "2.3.1", author="acme", contact="ops@example.com")
        == "acme/my-launcher/2.3.1 (ops@example.com)"
    )
    assert build_user_agent("my-cli", "1.0.0", author="acme") == "acme/my-cli/1.0.0"
    assert build_user_agent("my-cli", "1.0.0", contact="https://example.com") == "my-cli/1.0.0 (https://example.com)"
    assert build_user_agent("my-cli", "1.0.0") == "my-cli/1.0.0"

    with pytest.raises(OxirinthConfigError):
        build_user_agent("  ", "1.0.0")

    with pytest.raises(OxirinthConfigError):
        build_user_agent("my-cli", "1.0\n0")


def test_v3_loader_field_game_version_tag_parsing():
    tag = validate_json_model(
        GameVersionTag,
        b'{"value": "1.21.1", "type": "release", "created": "2024-08-08T00:00:00Z", "major": true}',
    )
    assert tag.version == "1.21.1"
    assert tag.version_type == "release"
    assert tag.released_at is not None
    assert tag.major is True
