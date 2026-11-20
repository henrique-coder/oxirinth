# Domains & Models

Oxirinth separates every Modrinth project type into dedicated client methods and immutable Pydantic v2 models (`frozen=True`, `extra="ignore"`), rather than returning generic untyped project payloads.

## Domain Matrix

| Domain         | Search Method          | Fetch Methods                           | Domain Models                        | Search Summary Models                              |
| :------------- | :--------------------- | :-------------------------------------- | :----------------------------------- | :------------------------------------------------- |
| Mods           | `search_mods`          | `get_mod`, `get_mods`                   | `Mod` / `AsyncMod`                   | `ModSummary` / `AsyncModSummary`                   |
| Plugins        | `search_plugins`       | `get_plugin`, `get_plugins`             | `Plugin` / `AsyncPlugin`             | `PluginSummary` / `AsyncPluginSummary`             |
| Resource Packs | `search_resourcepacks` | `get_resourcepack`, `get_resourcepacks` | `ResourcePack` / `AsyncResourcePack` | `ResourcePackSummary` / `AsyncResourcePackSummary` |
| Data Packs     | `search_datapacks`     | `get_datapack`, `get_datapacks`         | `DataPack` / `AsyncDataPack`         | `DataPackSummary` / `AsyncDataPackSummary`         |
| Shaders        | `search_shaders`       | `get_shader`, `get_shaders`             | `Shader` / `AsyncShader`             | `ShaderSummary` / `AsyncShaderSummary`             |
| Modpacks       | `search_modpacks`      | `get_modpack`, `get_modpacks`           | `Modpack` / `AsyncModpack`           | `ModpackSummary` / `AsyncModpackSummary`           |
| Servers        | `search_servers`       | `get_server`, `get_servers`             | `Server` / `AsyncServer`             | `ServerSummary` / `AsyncServerSummary`             |

## Search Filtering & `SearchPage`

All `search_*` methods accept uniform keyword filters while Rust translates domain-specific Modrinth v3 facets (`project_types:minecraft_java_server`, `mrpack_loaders:...`, `game_versions:...`). Results return a `SearchPage` that supports direct iteration, `len()`, indexing/slicing, and one-step hydration via `.fetch()`:

```python
page = client.search_plugins(
    "permissions",
    loaders=["paper", "folia"],
    game_versions=["1.21.1"],
    categories=["utility"],
    environment="server_only",
    sort="downloads",
    limit=10,
    offset=0,
)
print(f"{page.total} matches on page {page.page} (has_next={page.has_next})")

for summary in page:
    print(summary.id, summary.slug, summary.followers)

if page.first is not None:
    plugin = page.first.fetch()
    print(plugin.published_at)
```

Search summaries share unified field names (`.id`, `.followers`, `.created_at`, `.updated_at`, `.latest_version_id`) with full domain models so code transitions naturally between search results and full records.

## Multi-Type Projects & Domain Validation

Modrinth projects can carry multiple `project_types` (for example, `luckperms` has `["mod", "plugin"]` and `terralith` has `["datapack", "mod"]`):

- `get_plugin("luckperms")` and `get_mod("luckperms")` both succeed.
- `get_plugin("sodium")` (`["mod"]`) raises `OxirinthTypeMismatchError` in Rust before Pydantic deserialization.

## Bound Model Helpers

Models and search summaries returned by `Oxirinth` and `AsyncOxirinth` retain an internal client reference (excluded from `model_dump()`):

- `Mod`, `Plugin`, `ResourcePack`, `DataPack`, `Shader`, and `Modpack` (and their `Async*` twins) expose:
  - `get_versions(*, loaders=None, game_versions=None, featured=None) -> list[Version]`
  - `get_latest_version(*, loaders=None, game_versions=None, featured=None) -> Version | None`
  - `get_members() -> list[TeamMember]`
  - `get_dependencies() -> ProjectDependencies`
- `Server`, `AsyncServer`, `ServerSummary`, and `AsyncServerSummary` share clean server status properties:
  - `address` (`str | None`), `bedrock_address` (`str | None`)
  - `online_players` (`int | None`), `max_players` (`int | None`), `players` (`PlayerCount`)
  - `latency_ms` (`float | None`), `is_online` (`bool`)
  - `region` (`str | None`), `country` (`str | None`), `languages` (`list[str]`)
  - `content` (`ServerContent | None`), `ping` (`ServerPing | None`)
  - `Server` and `AsyncServer` also expose `get_members() -> list[TeamMember]`.
- Every summary model exposes `fetch()` (or `await summary.fetch()` for async) to load the complete domain object.

Calling any helper method on a manually constructed or unbound model raises `OxirinthUnboundModelError`.

## Versions, File Hashes & Tags

```python
version = client.get_version("CT6qQx01")
primary = version.primary_file
if primary is not None:
    print(primary.filename, primary.size, primary.url)
    print(primary.sha512)

by_hash = client.get_version_by_hash(primary.sha512, algorithm="sha512")
user = client.get_user("jellysquid3")
org = client.get_organization("caffeinemc")
loaders = client.get_loaders()
```
