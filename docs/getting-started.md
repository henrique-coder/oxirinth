# Getting Started

## Installation

Oxirinth supports **Python 3.11 through 3.15** (including free-threaded `t` builds) with pre-built wheels across Linux (`manylinux` and `musllinux`), macOS, and Windows:

```bash
uv add oxirinth
```

## Client Initialization

Both `Oxirinth` (sync) and `AsyncOxirinth` (async) share identical initialization parameters:

```python
from oxirinth import AsyncOxirinth, CacheConfig, Oxirinth, build_user_agent

ua = build_user_agent(
    "my-mod-manager",
    "1.2.0",
    author="my-github-user",
    contact="ops@example.com",
)

client = Oxirinth(
    user_agent=ua,
    api_key=None,  # Optional Modrinth Personal Access Token
    cache=CacheConfig(),  # Enabled by default (redb + zstd + RAM LRU fallback)
    timeout=30.0,  # Per-attempt timeout in seconds
    max_retries=3,  # Automatic retries on 502/503/504 and transient errors
    max_rate_limit_wait=None,  # None paces smoothly; float fails fast above cap
    base_url="https://api.modrinth.com",
)
```

### User-Agent Format

By default, Oxirinth sends a descriptive `User-Agent` header exposed as `DEFAULT_USER_AGENT`:

```text
henrique-coder/oxirinth/0.1.0 (https://github.com/henrique-coder/oxirinth)
```

For production applications or launch tools, pass a custom value built with `build_user_agent(project, version, *, author=..., contact=...)`.

### Optional Authentication (`api_key`)

Public `GET` endpoints work without authentication. Passing a Modrinth Personal Access Token via `api_key="..."` attaches an `Authorization` header to outbound requests:

- Secrets are held in a redacted Rust `SecretString` and never written to logs or disk caches.
- Cache keys isolate sessions using a SHA-256 `auth_fingerprint` so authenticated and anonymous cache entries never mix.

### Public Package Layout

- `oxirinth` — Clients, `CacheConfig`, `DEFAULT_USER_AGENT`, `build_user_agent`, domain models, `SearchPage`, summary models, `PlayerCount`, versions, social models, tags, and exceptions.
- `oxirinth.models` — Complete set of public data models, including nested server and metadata structures.
- `oxirinth.exceptions` — `OxirinthError` exception hierarchy.
