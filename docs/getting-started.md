# Getting Started

## Installation

Oxirinth supports **Python 3.11 through 3.15** (including free-threaded `t` builds) with pre-built binary wheels across Linux (`manylinux` and `musllinux`), macOS, and Windows:

=== "uv"

    ```bash
    uv add oxirinth
    ```

=== "pip"

    ```bash
    pip install oxirinth
    ```

## Client Initialization

Both `Oxirinth` (sync) and `AsyncOxirinth` (async) accept the exact same initialization options:

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
    cache=CacheConfig(),  # True by default (redb + zstd + RAM LRU fallback)
    timeout=30.0,  # Per-attempt timeout in seconds
    max_retries=3,  # Automatic retries on 502/503/504 & transient errors
    max_rate_limit_wait=None,  # None waits smoothly; float fails fast above cap
    base_url="https://api.modrinth.com",
)
```

### User-Agent Format

By default, Oxirinth sends a Modrinth-compliant `User-Agent` header exposed as `DEFAULT_USER_AGENT`:

```text
henrique-coder/oxirinth/0.1.0 (https://github.com/henrique-coder/oxirinth)
```

For production applications or launch tools, customize it with `build_user_agent(project, version, *, author=..., contact=...)` so it identifies your own application and contact info per Modrinth's API guidelines.

### Optional Authentication (`api_key`)

Public `GET` endpoints do not require authentication. Passing a Modrinth Personal Access Token via `api_key="..."` attaches an `Authorization` header to outbound requests:

- Secrets are wrapped inside a redacted Rust `SecretString` and never written to logs or disk cache files.
- Cache keys isolate sessions using a SHA-256 `auth_fingerprint`, ensuring authenticated responses never mix with anonymous cache entries.
