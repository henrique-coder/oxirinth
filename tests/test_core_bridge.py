from __future__ import annotations

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from threading import Thread
from typing import TYPE_CHECKING
from urllib.parse import parse_qs, urlparse

import pytest

from oxirinth import _core


if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path


SODIUM_JSON = json.dumps({"id": "AANobbMI", "slug": "sodium", "project_types": ["mod"], "name": "Sodium"})
LUCKPERMS_JSON = json.dumps(
    {"id": "Vebnzrzj", "slug": "luckperms", "project_types": ["mod", "plugin"], "name": "LuckPerms"},
)
AERO_SERVER_JSON = json.dumps(
    {"id": "OgcNPs6n", "slug": "aero-server", "project_types": ["minecraft_java_server"], "name": "Aero SMP"},
)


@pytest.fixture
def mock_modrinth_url() -> Iterator[str]:
    calls: dict[str, int] = {"sodium": 0}

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, format: str, *args: object) -> None:
            return

        def do_GET(self) -> None:
            parsed = urlparse(self.path)
            qs = parse_qs(parsed.query)

            if parsed.path == "/v3/project/sodium":
                calls["sodium"] += 1
                if self.headers.get("If-None-Match") == '"v1"':
                    self.send_response(304)
                    self.send_header("ETag", '"v2"')
                    self.end_headers()
                    return

                self._send_json(200, SODIUM_JSON, headers={"ETag": '"v1"'})
                return

            if parsed.path == "/v3/project/luckperms":
                self._send_json(200, LUCKPERMS_JSON)
                return

            if parsed.path == "/v3/project/aero-server":
                self._send_json(200, AERO_SERVER_JSON)
                return

            if parsed.path == "/v3/projects":
                ids = json.loads(qs.get("ids", ["[]"])[0])
                mapping = {"sodium": json.loads(SODIUM_JSON), "luckperms": json.loads(LUCKPERMS_JSON)}
                items = [mapping[item] for item in ids if item in mapping]
                self._send_json(200, json.dumps(items))
                return

            if parsed.path == "/v3/search":
                facets = json.loads(qs.get("facets", ["[]"])[0])
                payload = {
                    "hits": [{"slug": "aero-server", "facets_echo": facets}],
                    "page": 1,
                    "hits_per_page": 10,
                    "total_hits": 1,
                }
                self._send_json(200, json.dumps(payload))
                return

            self._send_json(404, json.dumps({"error": "not_found", "description": "Requested item does not exist"}))

        def _send_json(self, status: int, body: str, headers: dict[str, str] | None = None) -> None:
            encoded = body.encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(encoded)))
            if headers:
                for k, v in headers.items():
                    self.send_header(k, v)
            self.end_headers()
            self.wfile.write(encoded)

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}"
    finally:
        server.shutdown()
        server.server_close()


def test_sync_core_bytes_handoff_cache_and_exceptions(mock_modrinth_url: str, tmp_path: Path):
    core = _core.SyncCore(base_url=mock_modrinth_url, cache_dir=tmp_path)

    raw_mod = core.get_domain("mod", "sodium")
    assert isinstance(raw_mod, bytes)
    assert json.loads(raw_mod)["id"] == "AANobbMI"

    # Second read hits local redb cache.
    cached_mod = core.get_domain("mod", "sodium")
    assert cached_mod == raw_mod
    stats = core.cache_stats()
    assert stats.backend == "redb"
    assert stats.hits == 1
    assert stats.misses == 1
    assert stats.entries >= 1

    # Batch fetch returns list[bytes] in caller order.
    batch = core.get_domains("mod", ["luckperms", "sodium"])
    assert len(batch) == 2
    assert [json.loads(item)["slug"] for item in batch] == ["luckperms", "sodium"]

    # Type mismatch raises typed OxirinthTypeMismatchError.
    with pytest.raises(_core.OxirinthTypeMismatchError) as exc_info:
        core.get_domain("plugin", "sodium")

    assert exc_info.value.expected == "plugin"
    assert exc_info.value.actual == ("mod",)
    assert exc_info.value.identifier == "sodium"

    # 404 raises OxirinthNotFoundError (subclass of OxirinthAPIError & OxirinthError).
    with pytest.raises(_core.OxirinthNotFoundError) as not_found:
        core.get_domain("mod", "does-not-exist")

    assert isinstance(not_found.value, _core.OxirinthAPIError)
    assert isinstance(not_found.value, _core.OxirinthError)
    assert not_found.value.status_code == 404
    assert not_found.value.description == "Requested item does not exist"

    core.clear_cache()
    assert core.cache_stats().entries == 0


async def test_async_core_bytes_handoff_and_parity(mock_modrinth_url: str, tmp_path: Path):
    core = _core.AsyncCore(base_url=mock_modrinth_url, cache_dir=tmp_path)

    raw_search = await core.search_domain("server", query="aero")
    assert isinstance(raw_search, bytes)
    parsed = json.loads(raw_search)
    assert parsed["hits"][0]["facets_echo"] == [["project_types:minecraft_java_server"]]

    raw_plugin = await core.get_domain("plugin", "luckperms")
    assert json.loads(raw_plugin)["slug"] == "luckperms"

    batch = await core.get_domains("mod", ["sodium", "luckperms"])
    assert [json.loads(item)["slug"] for item in batch] == ["sodium", "luckperms"]

    sync_methods = {name for name in dir(_core.SyncCore) if not name.startswith("_")}
    async_methods = {name for name in dir(_core.AsyncCore) if not name.startswith("_")}
    assert sync_methods == async_methods
