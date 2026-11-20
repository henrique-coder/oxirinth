# Caching & Rate Limits

Oxirinth runs HTTP caching and quota pacing inside Rust so concurrent Python applications stay within Modrinth rate limits.

## Multi-Layer Caching (`redb` + `zstd` + RAM LRU)

Caching is **enabled by default** using `<os_cache_dir>/oxirinth/http_cache.redb`:

1. **Compact Binary Envelope & `zstd` Compression:** Bodies `>= 512` bytes are compressed with `zstd` (level `3`) only when compression strictly shrinks payload size.
2. **Tiered Default TTLs:**
   - `/v3/version/*` & `/v3/version_file/*`: **24 hours** (`86_400s`)
   - `/v3/project/*`, `/v3/user/*`, `/v3/organization/*`: **15 minutes** (`900s`)
   - `/v3/project/*/version`: **5 minutes** (`300s`)
   - `/v3/search`: **2 minutes** (`120s`)
   - `/v3/tag/*`, `/v3/license`, and `/v3/loader_field`: **6 hours** (`21_600s`)
3. **Conditional Revalidation (`304 Not Modified`):** Expired entries storing `ETag` or `Last-Modified` send `If-None-Match` / `If-Modified-Since`. A `304` refreshes freshness metadata in place without transferring or recompressing the JSON body.
4. **Auth Session Isolation:** Every cache key hashes `(method, canonical_url, auth_fingerprint)` using SHA-256 (`"anon"` or `SHA-256(api_key)`), so authenticated and anonymous responses never mix.
5. **Bounded RAM LRU Fallback:** If `redb` cannot open or acquire the exclusive file lock (for example, `DatabaseAlreadyOpen` or a read-only filesystem), Oxirinth logs a warning and degrades cleanly to a thread-safe in-memory LRU cache (`64 MiB` by default).

### Customizing & Inspecting Cache

```python
from pathlib import Path
from oxirinth import CacheConfig, Oxirinth

cfg = CacheConfig(
    enabled=True,
    cache_dir=Path("./.oxirinth-cache"),
    search_ttl_secs=60,
    project_ttl_secs=600,
)

with Oxirinth(cache=cfg) as client:
    client.get_mod("sodium")
    stats = client.cache_stats()
    print(stats.backend, stats.hits, stats.misses, stats.entries, stats.size_bytes)
    client.clear_cache()
```

Pass `cache=False` to disable caching completely for ephemeral runs.

## Singleflight Coalescing & Partial Batching

- **Singleflight Request Coalescing:** Concurrent async tasks or threads requesting the identical resource wait on one shared in-flight request instead of stampeding Modrinth.
- **Cache-Aware Partial Batching:** Calling `get_mods(["sodium", "iris", "lithium"])` or `get_versions([...])` checks local cache entries first, fetches only cache misses in chunks of up to `100` identifiers via `/v3/projects` or `/v3/versions`, caches each split result under both ID and slug keys, and returns results in your original order.

## Proactive Rate Limiting & Retries

- **Header-Synchronized Smooth Pacing:** Every Modrinth response updates `X-Ratelimit-Limit`, `X-Ratelimit-Remaining`, and `X-Ratelimit-Reset` inside Rust. When remaining quota drops to the safety reserve (`<= 2`), subsequent requests sleep smoothly until window reset (`+50ms` jitter) with the GIL released.
- **Transient Retries:** `reqwest-retry` automatically retries `502`, `503`, `504`, and transient network/TLS failures with exponential backoff and jitter (`max_retries=3` by default).
- **Optional Wait Cap:** Set `max_rate_limit_wait=5.0` on `Oxirinth` / `AsyncOxirinth` if you prefer failing fast with `OxirinthRateLimitError` instead of waiting across long reset windows.
