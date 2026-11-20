# Domains & Models

Oxirinth intentionally avoids generic `get_project` or `search_projects` wrappers. Every Modrinth project category has dedicated methods on `Oxirinth` and `AsyncOxirinth` plus cleanly scoped Pydantic v2 models (`frozen=True`, `extra="ignore"`).

## Domain Matrix

| Domain         | Search Method          | Fetch Methods                           | Sync / Async Models                  | Search Hit        |
| :------------- | :--------------------- | :-------------------------------------- | :----------------------------------- | :---------------- |
| Mods           | `search_mods`          | `get_mod`, `get_mods`                   | `Mod` / `AsyncMod`                   | `ModHit`          |
| Plugins        | `search_plugins`       | `get_plugin`, `get_plugins`             | `Plugin` / `AsyncPlugin`             | `PluginHit`       |
| Resource Packs | `search_resourcepacks` | `get_resourcepack`, `get_resourcepacks` | `ResourcePack` / `AsyncResourcePack` | `ResourcePackHit` |
| Data Packs     | `search_datapacks`     | `get_datapack`, `get_datapacks`         | `DataPack` / `AsyncDataPack`         | `DataPackHit`     |
| Shaders        | `search_shaders`       | `get_shader`, `get_shaders`             | `Shader` / `AsyncShader`             | `ShaderHit`       |
| Modpacks       | `search_modpacks`      | `get_modpack`, `get_modpacks`           | `Modpack` / `AsyncModpack`           | `ModpackHit`      |
| Servers        | `search_servers`       | `get_server`, `get_servers`             | `Server` / `AsyncServer`             | `ServerHit`       |

## Search Filtering

All `search_*` methods accept uniform keyword arguments while the Rust engine translates domain-specific Modrinth v3 facets (such as `project_types:minecraft_java_server`, `mrpack_loaders:...`, and `game_versions:...`):

```python
results = client.search_plugins(
    "permissions",
    loaders=["paper", "folia"],
    game_versions=["1.21.1"],
    categories=["utility"],
    environment="server_only",
    sort="downloads",
    limit=10,
    offset=0,
)
print(f"{results.total_hits} matches on page {results.page}")
```

## Multi-Type Projects & Domain Validation

Modrinth v3 projects can belong to multiple domains (for instance, `luckperms` carries `["mod", "plugin"]` and `terralith` carries `["datapack", "mod"]`).

- `get_plugin("luckperms")` and `get_mod("luckperms")` both succeed.
- `get_plugin("sodium")` (`["mod"]`) raises `OxirinthTypeMismatchError` before Python deserializes the payload.

## Bound Model Helpers

Models returned by `Oxirinth` and `AsyncOxirinth` retain an internal reference to the client (`_client`, excluded from `model_dump()`):

- `Mod`, `Plugin`, `ResourcePack`, `DataPack`, `Shader`, and `Modpack` (and their `Async*` twins) expose:
  - `get_versions(*, loaders=None, game_versions=None, featured=None) -> list[Version]`
  - `get_latest_version(*, loaders=None, game_versions=None, featured=None) -> Version | None`
  - `get_members() -> list[TeamMember]`
  - `get_dependencies() -> ProjectDependencies`
- `Server` and `AsyncServer` expose `get_members() -> list[TeamMember]` along with structured properties (`address`, `bedrock_address`, `players_online`, `players_max`, `region`, `content`, `ping`).

Calling a bound helper on a manually deserialized model raises `OxirinthUnboundModelError`.

## Versions, File Hashes & Metadata

```python
version = client.get_version("CT6qQx01")
primary = version.primary_file
if primary is not None:
    print(primary.filename, primary.size, primary.url)
    print(primary.hashes.sha512)

by_hash = client.get_version_by_hash(primary.hashes.sha512, algorithm="sha512")
user = client.get_user("jellysquid3")
org = client.get_organization("caffeinemc")
loaders = client.get_loaders()
```
